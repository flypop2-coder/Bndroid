//! Checked Linux anonymous virtual-memory plans, without page-table side effects.
use alloc::vec::Vec;

pub const PAGE: u64 = 4096;
// Keep sparse application mappings outside the native image, stack, MMIO and
// shared kernel arenas, within the existing private level-one table.
pub const ARENA_START: u64 = 0x8_0000_0000;
pub const ARENA_END: u64 = 0x80_0000_0000;
pub const MAX_REGIONS: usize = 128;
pub const ENOMEM: i64 = 12;
pub const EACCES: i64 = 13;
pub const EEXIST: i64 = 17;
pub const EINVAL: i64 = 22;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Protection {
    None,
    Read,
    ReadWrite,
}
impl Protection {
    pub fn from_linux(raw: u64) -> Result<Self, i64> {
        match raw {
            0 => Ok(Self::None),
            1 => Ok(Self::Read),
            3 => Ok(Self::ReadWrite),
            _ => Err(EACCES),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Region {
    pub start: u64,
    pub end: u64,
    pub protection: Protection,
}

#[derive(Debug, Default)]
pub struct Layout {
    pub regions: Vec<Region>,
}

pub struct Change {
    pub start: u64,
    pub end: u64,
    pub protection: Option<Protection>,
    pub replace: bool,
    pub layout: Layout,
}

pub fn rounded_length(length: u64) -> Result<u64, i64> {
    if length == 0 {
        return Err(EINVAL);
    }
    length
        .checked_add(PAGE - 1)
        .map(|value| value & !(PAGE - 1))
        .ok_or(ENOMEM)
}

impl Layout {
    pub fn map(
        &self,
        hint: u64,
        length: u64,
        prot: u64,
        flags: u64,
        offset: u64,
    ) -> Result<Change, i64> {
        const PRIVATE_ANONYMOUS: u64 = 0x22;
        const FIXED: u64 = 0x10;
        const NOREPLACE: u64 = 0x100000;
        const HINTS: u64 = 0x4000 | 0x20000; // NORESERVE, STACK
        if flags & PRIVATE_ANONYMOUS != PRIVATE_ANONYMOUS
            || flags & !(PRIVATE_ANONYMOUS | FIXED | NOREPLACE | HINTS) != 0
            || !offset.is_multiple_of(PAGE)
        {
            return Err(EINVAL);
        }
        let protection = Protection::from_linux(prot)?;
        let size = rounded_length(length)?;
        let fixed = flags & (FIXED | NOREPLACE) != 0;
        let start = if fixed {
            if !hint.is_multiple_of(PAGE) {
                return Err(EINVAL);
            }
            checked_range(hint, size)?;
            if flags & NOREPLACE != 0 && !self.free(hint, size) {
                return Err(EEXIST);
            }
            hint
        } else {
            let hint = hint & !(PAGE - 1);
            if self.free(hint, size) {
                hint
            } else {
                self.first_fit(size)?
            }
        };
        self.change(start, size, Some(protection), true)
    }

    pub fn protect(&self, address: u64, length: u64, prot: u64) -> Result<Change, i64> {
        let protection = Protection::from_linux(prot)?;
        let size = rounded_length(length)?;
        let end = checked_range(address, size)?;
        let mut cursor = address;
        for region in &self.regions {
            if region.start > cursor {
                break;
            }
            if region.end > cursor {
                cursor = region.end;
            }
            if cursor >= end {
                break;
            }
        }
        if cursor < end {
            return Err(ENOMEM);
        }
        self.change(address, size, Some(protection), false)
    }

    pub fn unmap(&self, address: u64, length: u64) -> Result<Change, i64> {
        self.change(address, rounded_length(length)?, None, true)
    }

    fn free(&self, address: u64, size: u64) -> bool {
        checked_range(address, size).is_ok_and(|end| {
            self.regions
                .iter()
                .all(|r| r.end <= address || end <= r.start)
        })
    }

    fn first_fit(&self, size: u64) -> Result<u64, i64> {
        let mut cursor = ARENA_START;
        for r in &self.regions {
            if cursor.checked_add(size).is_some_and(|end| end <= r.start) {
                return Ok(cursor);
            }
            cursor = r.end;
        }
        checked_range(cursor, size).map(|_| cursor)
    }

    fn change(
        &self,
        start: u64,
        size: u64,
        protection: Option<Protection>,
        replace: bool,
    ) -> Result<Change, i64> {
        let end = checked_range(start, size)?;
        let mut regions = Vec::new();
        regions
            .try_reserve_exact(MAX_REGIONS + 3)
            .map_err(|_| ENOMEM)?;
        for &r in &self.regions {
            if r.end <= start || end <= r.start {
                regions.push(r);
                continue;
            }
            if r.start < start {
                regions.push(Region { end: start, ..r });
            }
            if end < r.end {
                regions.push(Region { start: end, ..r });
            }
        }
        if let Some(protection) = protection {
            regions.push(Region {
                start,
                end,
                protection,
            });
        }
        regions.sort_unstable_by_key(|r| r.start);
        let mut index = 1;
        while index < regions.len() {
            if regions[index - 1].end == regions[index].start
                && regions[index - 1].protection == regions[index].protection
            {
                regions[index - 1].end = regions[index].end;
                regions.remove(index);
            } else {
                index += 1;
            }
        }
        if regions.len() > MAX_REGIONS {
            return Err(ENOMEM);
        }
        Ok(Change {
            start,
            end,
            protection,
            replace,
            layout: Layout { regions },
        })
    }
}

pub fn checked_range(start: u64, size: u64) -> Result<u64, i64> {
    if !start.is_multiple_of(PAGE) || !size.is_multiple_of(PAGE) {
        return Err(EINVAL);
    }
    let end = start.checked_add(size).ok_or(ENOMEM)?;
    if start < ARENA_START || end > ARENA_END || start >= end {
        return Err(ENOMEM);
    }
    Ok(end)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sparse_reservation_and_protection_split_merge() {
        let layout = Layout::default()
            .map(0, 1 << 38, 0, 0x4022, 0)
            .unwrap()
            .layout;
        assert_eq!(
            layout.regions[0],
            Region {
                start: ARENA_START,
                end: ARENA_START + (1 << 38),
                protection: Protection::None
            }
        );
        let split = layout.protect(ARENA_START + PAGE, 7, 3).unwrap().layout;
        assert_eq!(split.regions.len(), 3);
        assert_eq!(split.regions[1].protection, Protection::ReadWrite);
        let merged = split.protect(ARENA_START + PAGE, PAGE, 0).unwrap().layout;
        assert_eq!(merged.regions, layout.regions);
    }
    #[test]
    fn fixed_replacement_and_hole_unmap_preserve_neighbors() {
        let layout = Layout::default()
            .map(0, PAGE * 5, 3, 0x22, 0)
            .unwrap()
            .layout;
        assert_eq!(
            layout.map(ARENA_START, PAGE, 1, 0x100022, 0).err(),
            Some(EEXIST)
        );
        let replacement = layout.map(ARENA_START + PAGE, PAGE, 1, 0x32, 0).unwrap();
        assert!(replacement.replace);
        assert_eq!(replacement.layout.regions.len(), 3);
        let hole = replacement
            .layout
            .unmap(ARENA_START + PAGE, PAGE * 2)
            .unwrap()
            .layout;
        assert_eq!(hole.regions.len(), 2);
        assert_eq!(hole.protect(ARENA_START, PAGE * 5, 1).err(), Some(ENOMEM));
        assert!(hole.unmap(ARENA_START + PAGE, PAGE * 2).is_ok());
    }
    #[test]
    fn rejects_overflows_kernel_addresses_and_unsupported_rights() {
        let layout = Layout::default();
        for (address, length, flags) in [
            (0x40000000, PAGE, 0x32),
            (ARENA_END, PAGE, 0x32),
            (ARENA_START, u64::MAX, 0x22),
            (ARENA_START + 1, PAGE, 0x32),
        ] {
            assert!(layout.map(address, length, 3, flags, 0).is_err());
        }
        assert_eq!(layout.map(0, PAGE, 7, 0x22, 0).err(), Some(EACCES));
        assert_eq!(layout.map(0, PAGE, 3, 0x21, 0).err(), Some(EINVAL));
        assert_eq!(layout.map(0, 0, 3, 0x22, 0).err(), Some(EINVAL));
        assert_eq!(layout.map(0, PAGE, 3, 0x22, 1).err(), Some(EINVAL));
    }
    #[test]
    fn hints_fall_back_and_released_holes_are_reused() {
        let initial = Layout::default()
            .map(0x40000000, PAGE * 2, 3, 0x22, 0)
            .unwrap();
        assert_eq!(initial.start, ARENA_START);
        let next = initial.layout.map(ARENA_START, PAGE, 3, 0x22, 0).unwrap();
        assert_eq!(next.start, ARENA_START + PAGE * 2);
        let hole = next.layout.unmap(ARENA_START + PAGE, PAGE).unwrap().layout;
        assert_eq!(
            hole.map(0, PAGE, 3, 0x22, 0).unwrap().start,
            ARENA_START + PAGE
        );
    }
}
