//! Checked Linux private virtual-memory plans, without page-table side effects.
use alloc::vec::Vec;

pub const PAGE: u64 = 4096;
// Keep sparse application mappings outside the native image, stack, MMIO and
// shared kernel arenas, within the existing private level-one table.
pub const ARENA_START: u64 = 0x8_0000_0000;
pub const ARENA_END: u64 = 0x80_0000_0000;
// ART compressed references need low addresses. Only L1 slots 2/3 are free;
// slots 0/1 retain privileged MMIO/RAM and 4..8 retain kernel VM/user startup.
pub const LOW_START: u64 = 0x8000_0000;
pub const LOW_END: u64 = 0x1_0000_0000;
pub const MAX_REGIONS: usize = if cfg!(feature = "art-runtime") {
    1024
} else {
    128
};
pub const ENOMEM: i64 = 12;
pub const EACCES: i64 = 13;
pub const EEXIST: i64 = 17;
pub const EINVAL: i64 = 22;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Protection {
    None,
    Read,
    ReadWrite,
    ReadExecute,
}
impl Protection {
    pub fn from_linux(raw: u64) -> Result<Self, i64> {
        match raw {
            0 => Ok(Self::None),
            1 => Ok(Self::Read),
            3 => Ok(Self::ReadWrite),
            #[cfg(feature = "bionic-dynamic-runtime")]
            5 => Ok(Self::ReadExecute),
            _ => Err(EACCES),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileBacking {
    pub available: u64,
    pub offset: u64,
    pub inode: u64,
    pub path: &'static str,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Region {
    pub start: u64,
    pub end: u64,
    pub protection: Protection,
    pub file_backed: bool,
    pub file: Option<FileBacking>,
    pub file_origin: u64,
    pub shared: bool,
    pub file_page_end: u64,
    pub wipe_on_fork: bool,
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
        self.map_with_backing(hint, length, prot, flags, offset, None)
    }

    pub fn map_with_backing(
        &self,
        hint: u64,
        length: u64,
        prot: u64,
        flags: u64,
        offset: u64,
        file_bytes: Option<FileBacking>,
    ) -> Result<Change, i64> {
        const PRIVATE_ANONYMOUS: u64 = 0x22;
        const FIXED: u64 = 0x10;
        const NOREPLACE: u64 = 0x100000;
        const HINTS: u64 = 0x4000 | 0x20000; // NORESERVE, STACK
        let shared = cfg!(feature = "art-runtime") && flags & 3 == 1;
        let kind = if shared { 0x21 } else { PRIVATE_ANONYMOUS };
        if flags & kind != kind
            || flags & !(kind | FIXED | NOREPLACE | HINTS) != 0
            || !offset.is_multiple_of(PAGE)
        {
            return Err(EINVAL);
        }
        let protection = Protection::from_linux(prot)?;
        if shared && file_bytes.is_some() && protection == Protection::ReadWrite {
            return Err(EACCES);
        }
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
            } else if cfg!(feature = "art-runtime") && hint != 0 && hint < LOW_END {
                self.first_fit_in(size, LOW_START, LOW_END)
                    .or_else(|_| self.first_fit(size))?
            } else {
                self.first_fit(size)?
            }
        };
        self.change(start, size, Some(protection), true, file_bytes, shared)
    }

    pub fn protect(&self, address: u64, length: u64, prot: u64) -> Result<Change, i64> {
        let protection = Protection::from_linux(prot)?;
        let size = rounded_length(length)?;
        let end = checked_range(address, size)?;
        if protection == Protection::ReadWrite
            && self
                .regions
                .iter()
                .any(|r| r.start < end && address < r.end && r.shared && r.file_backed)
        {
            return Err(EACCES);
        }
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
        self.change(address, size, Some(protection), false, None, false)
    }

    pub fn unmap(&self, address: u64, length: u64) -> Result<Change, i64> {
        self.change(address, rounded_length(length)?, None, true, None, false)
    }

    pub fn advise_fork(&self, address: u64, length: u64, wipe: bool) -> Result<Self, i64> {
        let size = rounded_length(length)?;
        let end = checked_range(address, size)?;
        let mut regions = Vec::new();
        regions
            .try_reserve_exact(MAX_REGIONS + 3)
            .map_err(|_| ENOMEM)?;
        let mut cursor = address;
        for &r in &self.regions {
            if r.end <= address || end <= r.start {
                regions.push(r);
                continue;
            }
            if r.start > cursor {
                return Err(ENOMEM);
            }
            if wipe && (r.file_backed || r.shared) {
                return Err(EINVAL);
            }
            if r.start < address {
                regions.push(Region { end: address, ..r });
            }
            regions.push(Region {
                start: r.start.max(address),
                end: r.end.min(end),
                wipe_on_fork: wipe,
                ..r
            });
            if end < r.end {
                regions.push(Region { start: end, ..r });
            }
            cursor = r.end;
        }
        if cursor < end {
            return Err(ENOMEM);
        }
        if regions.len() > MAX_REGIONS {
            return Err(ENOMEM);
        }
        Ok(Self { regions })
    }

    fn free(&self, address: u64, size: u64) -> bool {
        checked_range(address, size).is_ok_and(|end| {
            self.regions
                .iter()
                .all(|r| r.end <= address || end <= r.start)
        })
    }

    fn first_fit(&self, size: u64) -> Result<u64, i64> {
        self.first_fit_in(size, ARENA_START, ARENA_END)
    }

    fn first_fit_in(&self, size: u64, start: u64, limit: u64) -> Result<u64, i64> {
        let mut cursor = start;
        for r in &self.regions {
            if r.end <= cursor {
                continue;
            }
            if r.start >= limit {
                break;
            }
            if cursor
                .checked_add(size)
                .is_some_and(|end| end <= r.start && end <= limit)
            {
                return Ok(cursor);
            }
            cursor = r.end;
        }
        if cursor.checked_add(size).is_some_and(|end| end <= limit) {
            Ok(cursor)
        } else {
            Err(ENOMEM)
        }
    }

    pub fn msync(&self, address: u64, length: u64, flags: u64) -> Result<u64, i64> {
        if !address.is_multiple_of(PAGE) || flags & !7 != 0 || flags & 5 == 5 {
            return Err(EINVAL);
        }
        if length == 0 {
            return Ok(0);
        }
        let end = address.checked_add(rounded_length(length)?).ok_or(ENOMEM)?;
        let mut cursor = address;
        for region in &self.regions {
            if region.start > cursor {
                break;
            }
            if region.end > cursor {
                cursor = region.end;
            }
            if cursor >= end {
                return Ok(0);
            }
        }
        Err(ENOMEM)
    }

    fn change(
        &self,
        start: u64,
        size: u64,
        protection: Option<Protection>,
        replace: bool,
        file_bytes: Option<FileBacking>,
        shared: bool,
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
            if !replace {
                regions.push(Region {
                    start: r.start.max(start),
                    end: r.end.min(end),
                    protection: protection.unwrap(),
                    ..r
                });
            }
        }
        if replace && let Some(protection) = protection {
            regions.push(Region {
                start,
                end,
                protection,
                file_backed: file_bytes.is_some(),
                file: file_bytes,
                file_origin: if file_bytes.is_some() { start } else { 0 },
                shared,
                file_page_end: match file_bytes.map(|file| file.available) {
                    Some(0) => start,
                    Some(bytes) => start.checked_add(rounded_length(bytes)?).ok_or(ENOMEM)?,
                    None => 0,
                },
                wipe_on_fork: false,
            });
        }
        regions.sort_unstable_by_key(|r| r.start);
        let mut index = 1;
        while index < regions.len() {
            if regions[index - 1].end == regions[index].start
                && regions[index - 1].protection == regions[index].protection
                && regions[index - 1].file_backed == regions[index].file_backed
                && regions[index - 1].file == regions[index].file
                && regions[index - 1].file_origin == regions[index].file_origin
                && regions[index - 1].shared == regions[index].shared
                && regions[index - 1].file_page_end == regions[index].file_page_end
                && regions[index - 1].wipe_on_fork == regions[index].wipe_on_fork
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
    if start >= end
        || !((start >= ARENA_START && end <= ARENA_END)
            || (cfg!(feature = "art-runtime") && start >= LOW_START && end <= LOW_END))
    {
        return Err(ENOMEM);
    }
    Ok(end)
}

pub fn is_arena_address(address: u64) -> bool {
    (ARENA_START..ARENA_END).contains(&address)
        || (cfg!(feature = "art-runtime") && (LOW_START..LOW_END).contains(&address))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_limit_failure_preserves_existing_mappings_and_recovers_after_unmap() {
        let mut layout = Layout::default();
        for n in 0..MAX_REGIONS {
            layout = layout
                .map(ARENA_START + n as u64 * PAGE * 2, PAGE, 3, 0x100022, 0)
                .unwrap()
                .layout;
        }
        let extra = ARENA_START + MAX_REGIONS as u64 * PAGE * 2;
        let before = layout.regions.clone();
        assert_eq!(layout.map(extra, PAGE, 3, 0x100022, 0).err(), Some(ENOMEM));
        assert_eq!(layout.regions, before);
        layout = layout.unmap(ARENA_START, PAGE).unwrap().layout;
        let restored = layout.map(extra, PAGE, 3, 0x100022, 0).unwrap().layout;
        assert_eq!(restored.regions.len(), MAX_REGIONS);
        assert_eq!(restored.regions.last().unwrap().start, extra);
    }
    #[test]
    fn split_file_mappings_keep_their_original_offset_identity() {
        let backing = FileBacking {
            available: PAGE * 9,
            offset: PAGE * 3,
            inode: 5,
            path: "system/data",
        };
        let mapped = Layout::default()
            .map_with_backing(0, PAGE * 3, 3, 0x22, PAGE * 3, Some(backing))
            .unwrap();
        let split = mapped
            .layout
            .protect(mapped.start + PAGE, PAGE, 1)
            .unwrap()
            .layout;
        let middle = split.regions[1];
        assert_eq!(
            middle.file.unwrap().offset + middle.start - middle.file_origin,
            PAGE * 4
        );
        let trimmed = split.unmap(mapped.start, PAGE).unwrap().layout;
        assert_eq!(trimmed.regions[0].file.unwrap().inode, 5);
        assert_eq!(trimmed.regions[0].file_origin, mapped.start);
        #[cfg(feature = "art-runtime")]
        {
            let shared = Layout::default()
                .map_with_backing(0, PAGE, 1, 0x21, 0, Some(backing))
                .unwrap();
            assert_eq!(
                shared.layout.protect(shared.start, PAGE, 3).err(),
                Some(EACCES)
            );
            assert_eq!(
                shared
                    .layout
                    .protect(shared.start, PAGE, 0)
                    .unwrap()
                    .layout
                    .protect(shared.start, PAGE, 3)
                    .err(),
                Some(EACCES)
            );
        }
    }
    fn file(bytes: u64) -> FileBacking {
        FileBacking {
            available: bytes,
            offset: 0,
            inode: 1,
            path: "system/test",
        }
    }
    #[test]
    #[cfg(feature = "art-runtime")]
    fn low_arena_never_selects_kernel_tables_or_changes_high_first_fit() {
        let low = Layout::default()
            .map(0x10000000, PAGE * 3, 3, 0x22, 0)
            .unwrap();
        assert_eq!(low.start, LOW_START);
        let high = low.layout.map(0, PAGE, 3, 0x22, 0).unwrap();
        assert_eq!(high.start, ARENA_START);
        assert_eq!(
            high.layout.map(0x10000000, PAGE, 3, 0x22, 0).unwrap().start,
            LOW_START + PAGE * 3
        );
        for address in [
            0,
            0x09000000,
            0x40080000,
            LOW_START - PAGE,
            LOW_END,
            0x200000000,
        ] {
            assert_eq!(
                high.layout.map(address, PAGE, 3, 0x32, 0).err(),
                Some(ENOMEM)
            );
        }
        assert_eq!(
            high.layout.map(LOW_END - PAGE, PAGE * 2, 3, 0x32, 0).err(),
            Some(ENOMEM)
        );
        let protected = high
            .layout
            .protect(LOW_START + PAGE, PAGE, 1)
            .unwrap()
            .layout;
        assert!(
            protected
                .regions
                .iter()
                .any(|r| r.start == LOW_START + PAGE && r.protection == Protection::Read)
        );
    }
    #[test]
    #[cfg(feature = "art-runtime")]
    fn shared_policy_survives_splits_and_msync_detects_holes() {
        let layout = Layout::default()
            .map(LOW_START, PAGE * 3, 3, 0x21, 0)
            .unwrap()
            .layout;
        assert!(layout.regions[0].shared);
        assert_eq!(
            layout.advise_fork(LOW_START, PAGE, true).err(),
            Some(EINVAL)
        );
        assert_eq!(layout.msync(LOW_START, PAGE * 3, 4), Ok(0));
        assert_eq!(layout.msync(LOW_START, PAGE, 5), Err(EINVAL));
        assert_eq!(layout.msync(LOW_START + 1, PAGE, 0), Err(EINVAL));
        assert_eq!(layout.msync(0x40080000, PAGE, 0), Err(ENOMEM));
        let split = layout.protect(LOW_START + PAGE, PAGE, 0).unwrap().layout;
        assert!(split.regions.iter().all(|r| r.shared));
        assert_eq!(split.msync(LOW_START, PAGE * 3, 0), Ok(0));
        let hole = split.unmap(LOW_START + PAGE, PAGE).unwrap().layout;
        assert_eq!(hole.msync(LOW_START, PAGE * 3, 0), Err(ENOMEM));
        let replaced = layout
            .map(LOW_START + PAGE, PAGE, 3, 0x32, 0)
            .unwrap()
            .layout;
        assert_eq!(replaced.regions.len(), 3);
        assert!(!replaced.regions[1].shared);
        assert_eq!(
            layout
                .map_with_backing(0, PAGE, 3, 0x21, 0, Some(file(PAGE)))
                .err(),
            Some(EACCES)
        );
    }
    #[test]
    fn fork_advice_preserves_backing_and_survives_protection_changes() {
        let layout = Layout::default()
            .map(0, PAGE * 3, 3, 0x22, 0)
            .unwrap()
            .layout;
        let advised = layout.advise_fork(ARENA_START + PAGE, PAGE, true).unwrap();
        assert_eq!(advised.regions.len(), 3);
        assert!(advised.regions[1].wipe_on_fork);
        let protected = advised.protect(ARENA_START, PAGE * 3, 1).unwrap().layout;
        assert_eq!(protected.regions.len(), 3);
        assert!(protected.regions[1].wipe_on_fork);
        assert!(!protected.regions[0].wipe_on_fork);
        let file = protected
            .map_with_backing(ARENA_START + PAGE, PAGE, 1, 0x32, 0, Some(file(PAGE)))
            .unwrap()
            .layout;
        assert!(file.regions[1].file_backed);
        assert!(!file.regions[1].wipe_on_fork);
        assert_eq!(
            file.advise_fork(ARENA_START + PAGE, PAGE, true).err(),
            Some(EINVAL)
        );
        assert_eq!(
            layout.advise_fork(ARENA_START, PAGE * 4, true).err(),
            Some(ENOMEM)
        );
    }
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
                protection: Protection::None,
                file_backed: false,
                file: None,
                file_origin: 0,
                shared: false,
                file_page_end: 0,
                wipe_on_fork: false,
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
        if !cfg!(feature = "art-runtime") {
            assert_eq!(layout.map(0, PAGE, 3, 0x21, 0).err(), Some(EINVAL));
        }
        assert_eq!(layout.map(0, 0, 3, 0x22, 0).err(), Some(EINVAL));
        assert_eq!(layout.map(0, PAGE, 3, 0x22, 1).err(), Some(EINVAL));
    }
    #[test]
    fn hints_fall_back_and_released_holes_are_reused() {
        let initial = Layout::default()
            .map(ARENA_END + PAGE, PAGE * 2, 3, 0x22, 0)
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
