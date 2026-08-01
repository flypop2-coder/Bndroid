use alloc::alloc::{Layout, alloc};
use alloc::boxed::Box;
use core::arch::asm;
use core::cmp::{max, min};
use core::ptr::{self, NonNull};
#[cfg(feature = "el0-fault-containment-self-test")]
use core::slice;

use bndr_abi::UserImageId;
#[cfg(not(feature = "el0-fault-containment-self-test"))]
use bndr_elf::{ElfError, ElfImage};
use bndroid_kernel::aarch64_paging::{
    user_ro_nx_page_descriptor, user_ro_x_page_descriptor, user_rw_nx_page_descriptor,
};
use bndroid_kernel::memory::{self, AllocateError, PAGE_SIZE};

use crate::arch::aarch64::mmu::{
    self, AddressSpaceError, OwnedUserMappings, StoppedAddressSpaceProof, UnmapError,
    UserAddressSpace, UserLayout, UserPageAccess, UserVma, UserVmaKind,
};

const MAX_LOAD_SEGMENTS: usize = 8;
pub const USER_IMAGE_START: usize = mmu::INIT_USER_WINDOW_START;
#[cfg(not(feature = "el0-fault-containment-self-test"))]
pub const USER_IMAGE_END: usize = USER_IMAGE_START + mmu::MAX_USER_LOAD_PAGES * PAGE_SIZE;
#[cfg(feature = "el0-fault-containment-self-test")]
pub const USER_CODE_START: usize = USER_IMAGE_START;
#[cfg(feature = "storage-server-runtime")]
pub const USER_STACK_PAGES: usize = 64;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    feature = "androidbox-interactive0"
))]
pub const USER_STACK_PAGES: usize = 20;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    not(feature = "androidbox-interactive0"),
    feature = "androidbox-apk-install0"
))]
pub const USER_STACK_PAGES: usize = 8;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    not(feature = "androidbox-interactive0"),
    not(feature = "androidbox-apk-install0")
))]
pub const USER_STACK_PAGES: usize = 4;
const _: () = assert!(USER_STACK_PAGES <= mmu::USER_STACK_PAGE_LIMIT);
pub const USER_STACK_GUARD_LOW: usize =
    mmu::INIT_USER_WINDOW_END - (USER_STACK_PAGES + 2) * PAGE_SIZE;
pub const USER_STACK_START: usize = USER_STACK_GUARD_LOW + PAGE_SIZE;
pub const USER_STACK_END: usize = mmu::INIT_USER_WINDOW_END - PAGE_SIZE;
pub const USER_STACK_GUARD_HIGH: usize = USER_STACK_END;

#[cfg(feature = "el0-fault-containment-self-test")]
unsafe extern "C" {
    static __user_init_template_start: u8;
    static __user_init_template_end: u8;
    static __user_init_fault_template_entry: u8;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserBootError {
    #[cfg(not(feature = "el0-fault-containment-self-test"))]
    Elf(ElfError),
    InvalidImage,
    AddressInUse,
    Frame(AllocateError),
    AddressSpace(AddressSpaceError),
    Query(UnmapError),
    PermissionMismatch,
    Cleanup,
}

impl UserBootError {
    pub const fn as_str(self) -> &'static str {
        match self {
            #[cfg(not(feature = "el0-fault-containment-self-test"))]
            Self::Elf(_) => "embedded user ELF failed the strict AArch64 ET_EXEC profile",
            Self::InvalidImage => {
                "embedded EL0 image violates the bounded multi-segment load policy"
            }
            Self::AddressInUse => "an EL0 image, stack, or guard page exists in the boot root",
            Self::Frame(error) => error.as_str(),
            Self::AddressSpace(error) => error.as_str(),
            Self::Query(error) => error.as_str(),
            Self::PermissionMismatch => {
                "EL0 leaf descriptor permissions do not match the RX/R--/RW- policy"
            }
            Self::Cleanup => "failed to roll back an unpublished EL0 address space",
        }
    }
}

#[derive(Clone, Copy)]
pub struct UserBootInfo {
    pub image_id: UserImageId,
    pub image_digest: u64,
    pub is_elf: bool,
    pub elf_bytes: usize,
    pub load_segments: usize,
    pub load_pages: usize,
    pub rx_pages: usize,
    pub ro_pages: usize,
    pub rw_pages: usize,
    pub bss_bytes: usize,
    pub memory_bytes: usize,
    pub entry: usize,
    pub code_start: usize,
    pub code_end: usize,
    pub code_bytes: usize,
    pub file_bytes: usize,
    pub stack_start: usize,
    pub stack_end: usize,
    pub stack_pages: usize,
    pub mapped_pages: usize,
    pub verified_leaf_pages: usize,
    pub code_physical: usize,
    pub stack_physical: usize,
    pub kernel_root: usize,
    pub user_root: usize,
    pub asid: u8,
    pub private_table_frames: usize,
    pub address_space_tlb_invalidations: u64,
    pub boot_user_unmapped: bool,
    pub heap_shared: bool,
}

#[derive(Clone, Copy)]
struct PlannedSegment<'a> {
    virtual_start: usize,
    page_start: usize,
    page_end: usize,
    file_data: &'a [u8],
    access: UserPageAccess,
}

impl PlannedSegment<'_> {
    #[cfg(not(feature = "el0-fault-containment-self-test"))]
    fn page_count(self) -> usize {
        (self.page_end - self.page_start) / PAGE_SIZE
    }
}

struct EmbeddedImage<'a> {
    image_id: UserImageId,
    image_digest: u64,
    is_elf: bool,
    elf_bytes: usize,
    public_load_segments: usize,
    segment_count: usize,
    segments: [Option<PlannedSegment<'a>>; MAX_LOAD_SEGMENTS],
    entry: usize,
    code_start: usize,
    code_end: usize,
    load_pages: usize,
    rx_pages: usize,
    ro_pages: usize,
    rw_pages: usize,
    file_bytes: usize,
    memory_bytes: usize,
    bss_bytes: usize,
}

impl EmbeddedImage<'_> {
    fn segment(&self, index: usize) -> PlannedSegment<'_> {
        self.segments[index]
            .unwrap_or_else(|| panic!("validated embedded image lost a load segment"))
    }
}

#[must_use = "a prepared image owns an unpublished address space"]
pub(crate) struct PreparedUserProcess {
    pub info: UserBootInfo,
    pub address_space: UserAddressSpace,
}

/// Loads, publishes, and activates the embedded EL0 init process.
pub fn start() -> Result<UserBootInfo, UserBootError> {
    let prepared = prepare_image(UserImageId::Init)?;
    let info = prepared.info;
    crate::process::install_init(prepared);
    Ok(info)
}

/// Creates a fresh, unpublished image/stack address space. The process
/// manager owns the final scheduler publication transaction.
pub(crate) fn prepare_image(
    image_id: UserImageId,
) -> Result<Box<PreparedUserProcess>, UserBootError> {
    let image = embedded_image(image_id)?;
    if !boot_pages_are_unmapped(&image)? {
        return Err(UserBootError::AddressInUse);
    }

    let layout = build_layout(&image)?;
    let expected_pages = image
        .load_pages
        .checked_add(USER_STACK_PAGES)
        .ok_or(UserBootError::InvalidImage)?;
    let mut mappings =
        OwnedUserMappings::try_new(expected_pages, layout).map_err(UserBootError::AddressSpace)?;

    let populate_result = populate_image_pages(&image, &mut mappings)
        .and_then(|()| populate_stack_pages(&mut mappings));
    if let Err(error) = populate_result {
        mappings.release_unpublished();
        return Err(error);
    }

    let address_space = match UserAddressSpace::try_new(mappings) {
        Ok(address_space) => address_space,
        Err(failure) => {
            let error = failure.error;
            failure.mappings.release_unpublished();
            return Err(UserBootError::AddressSpace(error));
        }
    };
    let (verified_leaf_pages, boot_user_unmapped, heap_shared) =
        match verify_prepared_address_space(&image, &address_space) {
            Ok(evidence) => evidence,
            Err(error) => {
                release_unpublished_address_space(address_space)?;
                return Err(error);
            }
        };

    let code_physical = address_space.code_physical();
    let stack_physical = address_space.stack_physical();
    let kernel_root = mmu::kernel_translation_context().root_address();
    let user_root = address_space.root_address();
    let asid = address_space.asid();
    let private_table_frames = address_space.private_table_frames();
    let mapped_pages = address_space.mapped_page_count();
    let address_space_tlb_invalidations = mmu::address_space_tlb_invalidations();
    let info = UserBootInfo {
        image_id: image.image_id,
        image_digest: image.image_digest,
        is_elf: image.is_elf,
        elf_bytes: image.elf_bytes,
        load_segments: image.public_load_segments,
        load_pages: image.load_pages,
        rx_pages: image.rx_pages,
        ro_pages: image.ro_pages,
        rw_pages: image.rw_pages,
        bss_bytes: image.bss_bytes,
        memory_bytes: image.memory_bytes,
        entry: image.entry,
        code_start: image.code_start,
        code_end: image.code_end,
        code_bytes: image.code_end - image.code_start,
        file_bytes: image.file_bytes,
        stack_start: USER_STACK_START,
        stack_end: USER_STACK_END,
        stack_pages: USER_STACK_PAGES,
        mapped_pages,
        verified_leaf_pages,
        code_physical,
        stack_physical,
        kernel_root,
        user_root,
        asid,
        private_table_frames,
        address_space_tlb_invalidations,
        boot_user_unmapped,
        heap_shared,
    };
    let prepared = PreparedUserProcess {
        info,
        address_space,
    };
    match try_box_value(prepared) {
        Ok(prepared) => Ok(prepared),
        Err(prepared) => {
            release_unpublished_address_space(prepared.address_space)?;
            Err(UserBootError::AddressSpace(AddressSpaceError::OutOfMemory))
        }
    }
}

fn build_layout(image: &EmbeddedImage<'_>) -> Result<UserLayout, UserBootError> {
    let mut layout = UserLayout::new(image.entry, USER_STACK_START, USER_STACK_END);
    for index in 0..image.segment_count {
        let segment = image.segment(index);
        layout
            .try_add_vma(UserVma::new(
                segment.page_start,
                segment.page_end,
                segment.access,
                UserVmaKind::Load,
            ))
            .map_err(UserBootError::AddressSpace)?;
    }
    layout
        .try_add_vma(UserVma::new(
            USER_STACK_START,
            USER_STACK_END,
            UserPageAccess::ReadWrite,
            UserVmaKind::Stack,
        ))
        .and_then(|()| layout.try_add_guard(USER_STACK_GUARD_LOW))
        .and_then(|()| layout.try_add_guard(USER_STACK_GUARD_HIGH))
        .map_err(UserBootError::AddressSpace)?;
    Ok(layout)
}

fn populate_image_pages(
    image: &EmbeddedImage<'_>,
    mappings: &mut OwnedUserMappings,
) -> Result<(), UserBootError> {
    for index in 0..image.segment_count {
        let segment = image.segment(index);
        let file_end = segment
            .virtual_start
            .checked_add(segment.file_data.len())
            .ok_or(UserBootError::InvalidImage)?;
        let mut virtual_address = segment.page_start;
        while virtual_address < segment.page_end {
            let frame = memory::allocate_frame().map_err(UserBootError::Frame)?;
            let physical = frame.start_address();
            unsafe {
                ptr::write_bytes(physical as *mut u8, 0, PAGE_SIZE);
            }

            let copy_start = max(virtual_address, segment.virtual_start);
            let copy_end = min(virtual_address + PAGE_SIZE, file_end);
            if copy_start < copy_end {
                let source_offset = copy_start - segment.virtual_start;
                let destination_offset = copy_start - virtual_address;
                unsafe {
                    ptr::copy_nonoverlapping(
                        segment.file_data.as_ptr().add(source_offset),
                        (physical + destination_offset) as *mut u8,
                        copy_end - copy_start,
                    );
                }
            }
            if segment.access == UserPageAccess::ReadExecute {
                synchronize_instruction_cache(physical, PAGE_SIZE);
            }
            push_mapping(mappings, virtual_address, frame, segment.access)?;
            virtual_address += PAGE_SIZE;
        }
    }
    Ok(())
}

fn populate_stack_pages(mappings: &mut OwnedUserMappings) -> Result<(), UserBootError> {
    let mut virtual_address = USER_STACK_START;
    while virtual_address < USER_STACK_END {
        let frame = memory::allocate_frame().map_err(UserBootError::Frame)?;
        let physical = frame.start_address();
        unsafe {
            ptr::write_bytes(physical as *mut u8, 0, PAGE_SIZE);
        }
        push_mapping(mappings, virtual_address, frame, UserPageAccess::ReadWrite)?;
        virtual_address += PAGE_SIZE;
    }
    Ok(())
}

fn push_mapping(
    mappings: &mut OwnedUserMappings,
    virtual_address: usize,
    frame: memory::OwnedFrame,
    access: UserPageAccess,
) -> Result<(), UserBootError> {
    match mappings.try_push(virtual_address, frame, access) {
        Ok(()) => Ok(()),
        Err(failure) => {
            let error = failure.error;
            release_frame(failure.frame)?;
            Err(UserBootError::AddressSpace(error))
        }
    }
}

fn verify_prepared_address_space(
    image: &EmbeddedImage<'_>,
    address_space: &UserAddressSpace,
) -> Result<(usize, bool, bool), UserBootError> {
    let mut verified = 0;
    for index in 0..image.segment_count {
        let segment = image.segment(index);
        let mut virtual_address = segment.page_start;
        while virtual_address < segment.page_end {
            verify_leaf(address_space, virtual_address, segment.access)?;
            verified += 1;
            virtual_address += PAGE_SIZE;
        }
    }
    let mut virtual_address = USER_STACK_START;
    while virtual_address < USER_STACK_END {
        verify_leaf(address_space, virtual_address, UserPageAccess::ReadWrite)?;
        verified += 1;
        virtual_address += PAGE_SIZE;
    }

    if verified != image.load_pages + USER_STACK_PAGES
        || verified != address_space.mapped_page_count()
        || !address_space
            .guards_are_unmapped()
            .map_err(UserBootError::Query)?
    {
        return Err(UserBootError::PermissionMismatch);
    }
    let boot_user_unmapped = boot_pages_are_unmapped(image)?;
    let boot_heap = mmu::query_boot_page_descriptor(crate::kernel_heap::HEAP_START)
        .map_err(UserBootError::Query)?;
    let heap_shared = boot_heap.is_some()
        && address_space
            .query_page_descriptor(crate::kernel_heap::HEAP_START)
            .map_err(UserBootError::Query)?
            == boot_heap;
    if !boot_user_unmapped || !heap_shared {
        return Err(UserBootError::PermissionMismatch);
    }
    Ok((verified, boot_user_unmapped, heap_shared))
}

fn verify_leaf(
    address_space: &UserAddressSpace,
    virtual_address: usize,
    access: UserPageAccess,
) -> Result<(), UserBootError> {
    let physical = address_space
        .page_physical(virtual_address)
        .ok_or(UserBootError::PermissionMismatch)?;
    let expected = match access {
        UserPageAccess::ReadExecute => user_ro_x_page_descriptor(physical),
        UserPageAccess::ReadOnly => user_ro_nx_page_descriptor(physical),
        UserPageAccess::ReadWrite => user_rw_nx_page_descriptor(physical),
    }
    .map_err(|_| UserBootError::PermissionMismatch)?;
    if address_space
        .query_page_descriptor(virtual_address)
        .map_err(UserBootError::Query)?
        != Some(expected)
    {
        return Err(UserBootError::PermissionMismatch);
    }
    Ok(())
}

fn boot_pages_are_unmapped(image: &EmbeddedImage<'_>) -> Result<bool, UserBootError> {
    for index in 0..image.segment_count {
        let segment = image.segment(index);
        let mut virtual_address = segment.page_start;
        while virtual_address < segment.page_end {
            if mmu::query_boot_page_descriptor(virtual_address)
                .map_err(UserBootError::Query)?
                .is_some()
            {
                return Ok(false);
            }
            virtual_address += PAGE_SIZE;
        }
    }
    let mut virtual_address = USER_STACK_START;
    while virtual_address < USER_STACK_END {
        if mmu::query_boot_page_descriptor(virtual_address)
            .map_err(UserBootError::Query)?
            .is_some()
        {
            return Ok(false);
        }
        virtual_address += PAGE_SIZE;
    }
    for guard in [USER_STACK_GUARD_LOW, USER_STACK_GUARD_HIGH] {
        if mmu::query_boot_page_descriptor(guard)
            .map_err(UserBootError::Query)?
            .is_some()
        {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(not(feature = "el0-fault-containment-self-test"))]
fn embedded_image(image_id: UserImageId) -> Result<Box<EmbeddedImage<'static>>, UserBootError> {
    let catalog_image = crate::user_images::get(image_id);
    if catalog_image.id != image_id {
        return Err(UserBootError::InvalidImage);
    }
    let elf = ElfImage::parse(catalog_image.bytes).map_err(UserBootError::Elf)?;
    if elf.load_segment_count() != 3
        || elf.load_page_count() == 0
        || elf.load_page_count() > mmu::MAX_USER_LOAD_PAGES
    {
        return Err(UserBootError::InvalidImage);
    }

    let mut segments = [None; MAX_LOAD_SEGMENTS];
    let mut rx_segments = 0;
    let mut ro_segments = 0;
    let mut rw_segments = 0;
    let mut rx_pages = 0;
    let mut ro_pages = 0;
    let mut rw_pages = 0;
    let mut file_bytes = 0_usize;
    let mut memory_bytes = 0_usize;
    let mut bss_bytes = 0_usize;
    let mut rw_has_bss = false;
    let mut code_start = 0;
    let mut code_end = 0;

    for (index, segment) in elf.load_segments().enumerate() {
        let virtual_start =
            usize::try_from(segment.virtual_address()).map_err(|_| UserBootError::InvalidImage)?;
        let memory_size =
            usize::try_from(segment.memory_size()).map_err(|_| UserBootError::InvalidImage)?;
        let memory_end = virtual_start
            .checked_add(memory_size)
            .ok_or(UserBootError::InvalidImage)?;
        let page_range = segment.page_range();
        let page_start =
            usize::try_from(page_range.start).map_err(|_| UserBootError::InvalidImage)?;
        let page_end = usize::try_from(page_range.end).map_err(|_| UserBootError::InvalidImage)?;
        if index >= MAX_LOAD_SEGMENTS
            || virtual_start != page_start
            || page_start < USER_IMAGE_START
            || page_end > USER_IMAGE_END
            || page_start >= page_end
        {
            return Err(UserBootError::InvalidImage);
        }

        let access = match (segment.flags().writable(), segment.flags().executable()) {
            (false, true) => UserPageAccess::ReadExecute,
            (false, false) => UserPageAccess::ReadOnly,
            (true, false) => UserPageAccess::ReadWrite,
            (true, true) => return Err(UserBootError::InvalidImage),
        };
        let planned = PlannedSegment {
            virtual_start,
            page_start,
            page_end,
            file_data: segment.file_data(),
            access,
        };
        let pages = planned.page_count();
        match access {
            UserPageAccess::ReadExecute => {
                rx_segments += 1;
                rx_pages += pages;
                code_start = virtual_start;
                code_end = memory_end;
            }
            UserPageAccess::ReadOnly => {
                ro_segments += 1;
                ro_pages += pages;
            }
            UserPageAccess::ReadWrite => {
                rw_segments += 1;
                rw_pages += pages;
                rw_has_bss |= memory_size > segment.file_data().len();
            }
        }
        file_bytes = file_bytes
            .checked_add(segment.file_data().len())
            .ok_or(UserBootError::InvalidImage)?;
        memory_bytes = memory_bytes
            .checked_add(memory_size)
            .ok_or(UserBootError::InvalidImage)?;
        bss_bytes = bss_bytes
            .checked_add(memory_size - segment.file_data().len())
            .ok_or(UserBootError::InvalidImage)?;
        segments[index] = Some(planned);
    }

    let entry = usize::try_from(elf.entry()).map_err(|_| UserBootError::InvalidImage)?;
    if rx_segments != 1
        || ro_segments != 1
        || rw_segments != 1
        || segments[0].is_none_or(|segment| segment.access != UserPageAccess::ReadExecute)
        || segments[1].is_none_or(|segment| segment.access != UserPageAccess::ReadOnly)
        || segments[2].is_none_or(|segment| segment.access != UserPageAccess::ReadWrite)
        || !rw_has_bss
        || bss_bytes == 0
        || code_start > entry
        || entry >= code_end
        || !code_start.is_multiple_of(4)
        || !code_end.is_multiple_of(4)
        || rx_pages + ro_pages + rw_pages != elf.load_page_count()
    {
        return Err(UserBootError::InvalidImage);
    }

    try_box_value(EmbeddedImage {
        image_id,
        image_digest: catalog_image.digest,
        is_elf: true,
        elf_bytes: catalog_image.bytes.len(),
        public_load_segments: elf.load_segment_count(),
        segment_count: elf.load_segment_count(),
        segments,
        entry,
        code_start,
        code_end,
        load_pages: elf.load_page_count(),
        rx_pages,
        ro_pages,
        rw_pages,
        file_bytes,
        memory_bytes,
        bss_bytes,
    })
    .map_err(|_| UserBootError::AddressSpace(AddressSpaceError::OutOfMemory))
}

#[cfg(feature = "el0-fault-containment-self-test")]
fn embedded_image(image_id: UserImageId) -> Result<Box<EmbeddedImage<'static>>, UserBootError> {
    if image_id != UserImageId::Init {
        return Err(UserBootError::InvalidImage);
    }
    let template_start = core::ptr::addr_of!(__user_init_template_start) as usize;
    let template_end = core::ptr::addr_of!(__user_init_template_end) as usize;
    let code_bytes = template_end
        .checked_sub(template_start)
        .filter(|bytes| *bytes != 0 && *bytes <= PAGE_SIZE && bytes.is_multiple_of(4))
        .ok_or(UserBootError::InvalidImage)?;
    let fault_entry = core::ptr::addr_of!(__user_init_fault_template_entry) as usize;
    let entry_offset = fault_entry
        .checked_sub(template_start)
        .filter(|offset| *offset < code_bytes)
        .ok_or(UserBootError::InvalidImage)?;
    let file_data = unsafe { slice::from_raw_parts(template_start as *const u8, code_bytes) };
    let segment = PlannedSegment {
        virtual_start: USER_CODE_START,
        page_start: USER_CODE_START,
        page_end: USER_CODE_START + PAGE_SIZE,
        file_data,
        access: UserPageAccess::ReadExecute,
    };
    let mut segments = [None; MAX_LOAD_SEGMENTS];
    segments[0] = Some(segment);
    try_box_value(EmbeddedImage {
        image_id,
        image_digest: 0,
        is_elf: false,
        elf_bytes: 0,
        public_load_segments: 0,
        segment_count: 1,
        segments,
        entry: USER_CODE_START + entry_offset,
        code_start: USER_CODE_START,
        code_end: USER_CODE_START + code_bytes,
        load_pages: 1,
        rx_pages: 1,
        ro_pages: 0,
        rw_pages: 0,
        file_bytes: code_bytes,
        memory_bytes: code_bytes,
        bss_bytes: 0,
    })
    .map_err(|_| UserBootError::AddressSpace(AddressSpaceError::OutOfMemory))
}

fn try_box_value<T>(value: T) -> Result<Box<T>, T> {
    let layout = Layout::new::<T>();
    let Some(pointer) = NonNull::new(unsafe { alloc(layout) }.cast::<T>()) else {
        return Err(value);
    };
    unsafe {
        pointer.as_ptr().write(value);
        Ok(Box::from_raw(pointer.as_ptr()))
    }
}

fn release_frame(frame: memory::OwnedFrame) -> Result<(), UserBootError> {
    memory::deallocate_frame(frame).map_err(|_| UserBootError::Cleanup)
}

fn release_unpublished_address_space(address_space: UserAddressSpace) -> Result<(), UserBootError> {
    let translation = address_space.translation_context();
    let stopped = unsafe { StoppedAddressSpaceProof::new(translation) };
    address_space
        .destroy(stopped)
        .map_err(|_| UserBootError::Cleanup)
}

fn synchronize_instruction_cache(data_alias: usize, bytes: usize) {
    let ctr: u64;
    unsafe {
        asm!(
            "mrs {ctr}, ctr_el0",
            ctr = out(reg) ctr,
            options(nomem, nostack, preserves_flags)
        );
    }
    let data_line = 4_usize << ((ctr >> 16) & 0xf);
    let data_end = data_alias + bytes;
    let mut address = data_alias & !(data_line - 1);
    while address < data_end {
        unsafe {
            asm!(
                "dc cvau, {address}",
                address = in(reg) address,
                options(nostack, preserves_flags)
            );
        }
        address += data_line;
    }
    unsafe {
        asm!(
            "dsb ish",
            "ic iallu",
            "dsb ish",
            "isb",
            options(nostack, preserves_flags)
        );
    }
}
