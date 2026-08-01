//! Small, allocation-free sector-reader boundary shared by partition and
//! filesystem parsers.

pub const SECTOR_SIZE: usize = 512;
pub type Sector = [u8; SECTOR_SIZE];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockReadError {
    OutOfBounds,
    Device,
}

impl BlockReadError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OutOfBounds => "block read is outside the device",
            Self::Device => "block device read failed",
        }
    }
}

/// Synchronous facade over an underlying block transport.
///
/// Implementations may internally complete asynchronously. The boundary is
/// deliberately sector-sized so on-disk parsers neither know about virtio nor
/// retain DMA-backed buffers.
pub trait BlockReader {
    fn sector_count(&self) -> u64;

    fn read_sector(&mut self, lba: u64, output: &mut Sector) -> Result<(), BlockReadError>;
}

pub fn checked_sector_range(
    sector_count: u64,
    first_lba: u64,
    sectors: u64,
) -> Result<core::ops::Range<u64>, BlockReadError> {
    let end = first_lba
        .checked_add(sectors)
        .ok_or(BlockReadError::OutOfBounds)?;
    if sectors == 0 || first_lba >= sector_count || end > sector_count {
        return Err(BlockReadError::OutOfBounds);
    }
    Ok(first_lba..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_range_accepts_exact_tail() {
        assert_eq!(checked_sector_range(16, 12, 4), Ok(12..16));
    }

    #[test]
    fn checked_range_rejects_empty_overflow_and_past_end() {
        assert_eq!(
            checked_sector_range(16, 4, 0),
            Err(BlockReadError::OutOfBounds)
        );
        assert_eq!(
            checked_sector_range(16, u64::MAX, 2),
            Err(BlockReadError::OutOfBounds)
        );
        assert_eq!(
            checked_sector_range(16, 15, 2),
            Err(BlockReadError::OutOfBounds)
        );
    }
}
