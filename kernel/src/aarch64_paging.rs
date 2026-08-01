pub const DESCRIPTOR_BLOCK: u64 = 0b01;
pub const DESCRIPTOR_TABLE_OR_PAGE: u64 = 0b11;
pub const TABLE_ADDRESS_MASK: u64 = 0x0000_ffff_ffff_f000;

const PAGE_SIZE: usize = 4096;
const ACCESS_FLAG: u64 = 1 << 10;
const NOT_GLOBAL: u64 = 1 << 11;
const INNER_SHAREABLE: u64 = 0b11 << 8;
const ATTR_NORMAL: u64 = 1 << 2;
const READ_WRITE_EL0: u64 = 0b01 << 6;
const READ_ONLY_EL0: u64 = 0b11 << 6;
const PRIVILEGED_EXECUTE_NEVER: u64 = 1 << 53;
const UNPRIVILEGED_EXECUTE_NEVER: u64 = 1 << 54;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageIndices {
    pub level_0: usize,
    pub level_1: usize,
    pub level_2: usize,
    pub level_3: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DescriptorError {
    UnalignedPhysicalAddress,
    PhysicalAddressTooLarge,
}

pub const fn lower_48_page_indices(virtual_address: usize) -> Option<PageIndices> {
    if virtual_address >= (1_usize << 48) || virtual_address & (PAGE_SIZE - 1) != 0 {
        return None;
    }
    Some(PageIndices {
        level_0: (virtual_address >> 39) & 0x1ff,
        level_1: (virtual_address >> 30) & 0x1ff,
        level_2: (virtual_address >> 21) & 0x1ff,
        level_3: (virtual_address >> 12) & 0x1ff,
    })
}

pub const fn kernel_rw_nx_page_descriptor(physical_address: usize) -> Result<u64, DescriptorError> {
    normal_page_descriptor(
        physical_address,
        PRIVILEGED_EXECUTE_NEVER | UNPRIVILEGED_EXECUTE_NEVER,
    )
}

pub const fn user_rw_nx_page_descriptor(physical_address: usize) -> Result<u64, DescriptorError> {
    normal_page_descriptor(
        physical_address,
        READ_WRITE_EL0 | NOT_GLOBAL | PRIVILEGED_EXECUTE_NEVER | UNPRIVILEGED_EXECUTE_NEVER,
    )
}

pub const fn user_ro_x_page_descriptor(physical_address: usize) -> Result<u64, DescriptorError> {
    normal_page_descriptor(
        physical_address,
        READ_ONLY_EL0 | NOT_GLOBAL | PRIVILEGED_EXECUTE_NEVER,
    )
}

pub const fn user_ro_nx_page_descriptor(physical_address: usize) -> Result<u64, DescriptorError> {
    normal_page_descriptor(
        physical_address,
        READ_ONLY_EL0 | NOT_GLOBAL | PRIVILEGED_EXECUTE_NEVER | UNPRIVILEGED_EXECUTE_NEVER,
    )
}

const fn normal_page_descriptor(
    physical_address: usize,
    access_and_execute: u64,
) -> Result<u64, DescriptorError> {
    if physical_address & (PAGE_SIZE - 1) != 0 {
        return Err(DescriptorError::UnalignedPhysicalAddress);
    }
    if physical_address >= (1_usize << 48) {
        return Err(DescriptorError::PhysicalAddressTooLarge);
    }
    Ok(physical_address as u64
        | DESCRIPTOR_TABLE_OR_PAGE
        | ACCESS_FLAG
        | INNER_SHAREABLE
        | ATTR_NORMAL
        | access_and_execute)
}

pub const fn descriptor_type(descriptor: u64) -> u64 {
    descriptor & 0b11
}

pub const fn descriptor_address(descriptor: u64) -> usize {
    (descriptor & TABLE_ADDRESS_MASK) as usize
}

#[cfg(test)]
mod tests {
    use super::{
        DESCRIPTOR_TABLE_OR_PAGE, DescriptorError, PageIndices, descriptor_address,
        descriptor_type, kernel_rw_nx_page_descriptor, lower_48_page_indices,
        user_ro_nx_page_descriptor, user_ro_x_page_descriptor, user_rw_nx_page_descriptor,
    };

    #[test]
    fn extracts_all_four_indices_for_the_dynamic_arena() {
        assert_eq!(
            lower_48_page_indices(0x0000_0001_0000_0000),
            Some(PageIndices {
                level_0: 0,
                level_1: 4,
                level_2: 0,
                level_3: 0,
            })
        );
        assert_eq!(
            lower_48_page_indices(0x0000_0001_4020_3000),
            Some(PageIndices {
                level_0: 0,
                level_1: 5,
                level_2: 1,
                level_3: 3,
            })
        );
    }

    #[test]
    fn rejects_unaligned_or_non_lower_48_virtual_pages() {
        assert_eq!(lower_48_page_indices(0x1001), None);
        assert_eq!(lower_48_page_indices(1_usize << 48), None);
    }

    #[test]
    fn encodes_kernel_rw_normal_nx_leaf_attributes() {
        let descriptor = kernel_rw_nx_page_descriptor(0x4000_3000).unwrap();
        assert_eq!(descriptor_type(descriptor), DESCRIPTOR_TABLE_OR_PAGE);
        assert_eq!(descriptor_address(descriptor), 0x4000_3000);
        assert_ne!(descriptor & (1 << 10), 0); // AF
        assert_eq!((descriptor >> 8) & 0b11, 0b11); // Inner-shareable
        assert_eq!((descriptor >> 2) & 0b111, 1); // MAIR AttrIndx 1
        assert_eq!((descriptor >> 6) & 0b11, 0); // EL1 read/write
        assert_ne!(descriptor & (1 << 53), 0); // PXN
        assert_ne!(descriptor & (1 << 54), 0); // UXN
    }

    #[test]
    fn rejects_invalid_physical_page_addresses() {
        assert_eq!(
            kernel_rw_nx_page_descriptor(0x4000_0001),
            Err(DescriptorError::UnalignedPhysicalAddress)
        );
        assert_eq!(
            kernel_rw_nx_page_descriptor(1_usize << 48),
            Err(DescriptorError::PhysicalAddressTooLarge)
        );
    }

    #[test]
    fn encodes_user_code_as_read_only_el0_executable_but_pxn() {
        let descriptor = user_ro_x_page_descriptor(0x4000_5000).unwrap();
        assert_eq!(descriptor_address(descriptor), 0x4000_5000);
        assert_eq!((descriptor >> 6) & 0b11, 0b11); // EL0/EL1 read-only
        assert_ne!(descriptor & (1 << 53), 0); // PXN
        assert_eq!(descriptor & (1 << 54), 0); // EL0 executable
        assert_ne!(descriptor & (1 << 11), 0); // non-global
    }

    #[test]
    fn encodes_user_data_as_read_write_and_never_executable() {
        let descriptor = user_rw_nx_page_descriptor(0x4000_6000).unwrap();
        assert_eq!(descriptor_address(descriptor), 0x4000_6000);
        assert_eq!((descriptor >> 6) & 0b11, 0b01); // EL0/EL1 read-write
        assert_ne!(descriptor & (1 << 53), 0); // PXN
        assert_ne!(descriptor & (1 << 54), 0); // UXN
        assert_ne!(descriptor & (1 << 11), 0); // non-global
    }

    #[test]
    fn encodes_user_rodata_as_read_only_and_never_executable() {
        let descriptor = user_ro_nx_page_descriptor(0x4000_7000).unwrap();
        assert_eq!(descriptor_address(descriptor), 0x4000_7000);
        assert_eq!((descriptor >> 6) & 0b11, 0b11); // EL0/EL1 read-only
        assert_ne!(descriptor & (1 << 53), 0); // PXN
        assert_ne!(descriptor & (1 << 54), 0); // UXN
        assert_ne!(descriptor & (1 << 11), 0); // non-global
    }
}
