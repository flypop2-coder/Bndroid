use alloc::alloc::alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::arch::asm;
use core::cell::UnsafeCell;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bndroid_kernel::aarch64_paging::{
    DESCRIPTOR_BLOCK, DESCRIPTOR_TABLE_OR_PAGE, descriptor_address, descriptor_type,
    kernel_rw_nx_page_descriptor, user_ro_nx_page_descriptor, user_ro_x_page_descriptor,
    user_rw_nx_page_descriptor,
};
#[cfg(not(feature = "mobile-ui-runtime"))]
use bndroid_kernel::graphics_buffer::GRAPHICS_BUFFER_SLOT_COUNT;
use bndroid_kernel::graphics_buffer::{GraphicsBuffer, GraphicsBufferIdentity};
use bndroid_kernel::memory::{self, OwnedFrame, PAGE_SIZE, PhysFrame};

const ENTRY_COUNT: usize = 512;
const LEVEL_1_BLOCK_SIZE: usize = 1024 * 1024 * 1024;
const LEVEL_2_BLOCK_SIZE: usize = 2 * 1024 * 1024;
pub const EARLY_RAM_START: usize = 0x4000_0000;
pub const EARLY_RAM_END: usize = 0x8000_0000;
pub const DYNAMIC_MAP_START: usize = 0x0000_0001_0000_0000;
pub const DYNAMIC_MAP_END: usize = 0x0000_0002_0000_0000;
pub const INIT_USER_WINDOW_START: usize = DYNAMIC_MAP_END;
pub const INIT_USER_WINDOW_END: usize = INIT_USER_WINDOW_START + LEVEL_2_BLOCK_SIZE;
pub const MAX_USER_LOAD_PAGES: usize = 256;
#[cfg(feature = "storage-server-runtime")]
pub const USER_STACK_PAGE_LIMIT: usize = 64;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    feature = "androidbox-interactive0"
))]
pub const USER_STACK_PAGE_LIMIT: usize = 20;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    not(feature = "androidbox-interactive0"),
    feature = "androidbox-apk-install0"
))]
pub const USER_STACK_PAGE_LIMIT: usize = 8;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    not(feature = "androidbox-interactive0"),
    not(feature = "androidbox-apk-install0")
))]
pub const USER_STACK_PAGE_LIMIT: usize = 4;
pub const MAX_USER_MAPPED_PAGES: usize = MAX_USER_LOAD_PAGES + USER_STACK_PAGE_LIMIT;
pub const MAX_USER_VMAS: usize = 9;
pub const MAX_USER_GUARDS: usize = 2;
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const USER_GRAPHICS_MAPPING_CAPACITY: usize = 2;
// A 720x1600 backing spans more than the single 2 MiB EL0 page-table window.
// The mobile preview therefore keeps the bounded copy-write ABI and exposes no
// shared graphics mapping slots.
#[cfg(feature = "mobile-ui-runtime")]
pub const USER_GRAPHICS_MAPPING_CAPACITY: usize = 0;
pub const USER_GRAPHICS_MAPPING_PAGES: usize = bndr_abi::GRAPHICS_BUFFER_BACKING_BYTES / PAGE_SIZE;
pub const USER_GRAPHICS_MAP_ADDRESS: usize = bndr_abi::GRAPHICS_BUFFER_MAP_ADDRESS as usize;
pub const USER_GRAPHICS_MAP_STRIDE: usize = bndr_abi::GRAPHICS_BUFFER_MAP_STRIDE as usize;
pub const KERNEL_ASID: u8 = 0;
/// Compatibility value for the first user address space allocated at boot.
pub const INIT_USER_ASID: u8 = 1;
const DYNAMIC_LEVEL_1_FIRST: usize = (DYNAMIC_MAP_START >> 30) & (ENTRY_COUNT - 1);
const DYNAMIC_LEVEL_1_COUNT: usize = (DYNAMIC_MAP_END - DYNAMIC_MAP_START) / LEVEL_1_BLOCK_SIZE;
const INIT_USER_LEVEL_1_INDEX: usize = (INIT_USER_WINDOW_START >> 30) & (ENTRY_COUNT - 1);
const TTBR_ASID_SHIFT: u32 = 48;
const TTBR_ASID_MASK: u64 = 0xff << TTBR_ASID_SHIFT;
const TTBR_BASE_MASK: u64 = 0x0000_ffff_ffff_f000;
const ACCESS_FLAG: u64 = 1 << 10;
const INNER_SHAREABLE: u64 = 0b11 << 8;
const READ_ONLY_EL1: u64 = 0b10 << 6;
const ATTR_DEVICE: u64 = 0 << 2;
const ATTR_NORMAL: u64 = 1 << 2;
const PRIVILEGED_EXECUTE_NEVER: u64 = 1 << 53;
const UNPRIVILEGED_EXECUTE_NEVER: u64 = 1 << 54;

const _: () = assert!(usize::BITS >= 64);
#[cfg(not(feature = "mobile-ui-runtime"))]
const _: () = assert!(USER_GRAPHICS_MAPPING_CAPACITY == GRAPHICS_BUFFER_SLOT_COUNT);
#[cfg(not(feature = "mobile-ui-runtime"))]
const _: () = assert!(USER_GRAPHICS_MAPPING_PAGES == 75);
#[cfg(feature = "mobile-ui-runtime")]
const _: () = assert!(USER_GRAPHICS_MAPPING_CAPACITY == 0);
#[cfg(feature = "mobile-ui-runtime")]
const _: () = assert!(USER_GRAPHICS_MAPPING_PAGES == 1_125);
const _: () = assert!(bndr_abi::GRAPHICS_BUFFER_BACKING_BYTES.is_multiple_of(PAGE_SIZE));
const _: () = assert!(USER_GRAPHICS_MAP_STRIDE >= bndr_abi::GRAPHICS_BUFFER_BACKING_BYTES);
const _: () =
    assert!(USER_GRAPHICS_MAP_ADDRESS >= INIT_USER_WINDOW_START + MAX_USER_LOAD_PAGES * PAGE_SIZE);
const _: () = assert!(
    USER_GRAPHICS_MAP_ADDRESS + USER_GRAPHICS_MAPPING_CAPACITY * USER_GRAPHICS_MAP_STRIDE
        <= INIT_USER_WINDOW_END - (USER_STACK_PAGE_LIMIT + 2) * PAGE_SIZE
);

const SCTLR_M: u64 = 1 << 0;
const SCTLR_C: u64 = 1 << 2;
const SCTLR_SA: u64 = 1 << 3;
const SCTLR_SA0: u64 = 1 << 4;
const SCTLR_I: u64 = 1 << 12;
const SCTLR_NTWI: u64 = 1 << 16;
const SCTLR_WXN: u64 = 1 << 19;
const SCTLR_EL1_RES1: u64 = (1 << 29) | (1 << 28) | (1 << 23) | (1 << 22) | (1 << 20) | (1 << 11);

#[repr(C, align(4096))]
struct PageTable([u64; ENTRY_COUNT]);

struct BootPageTable(UnsafeCell<PageTable>);

// These tables are only mutated once on the boot CPU before interrupts and SMP.
unsafe impl Sync for BootPageTable {}

struct SharedPageTable(UnsafeCell<PageTable>);

impl SharedPageTable {
    const fn new() -> Self {
        Self(UnsafeCell::new(PageTable([0; ENTRY_COUNT])))
    }
}

// These L2 tables are initialized on the boot CPU, then all descriptor
// mutations are serialized by PAGE_TABLE_LOCK with the local IRQ masked.
unsafe impl Sync for SharedPageTable {}

static LEVEL_0: BootPageTable = BootPageTable(UnsafeCell::new(PageTable([0; ENTRY_COUNT])));
static LEVEL_1: BootPageTable = BootPageTable(UnsafeCell::new(PageTable([0; ENTRY_COUNT])));
static LEVEL_2_RAM: BootPageTable = BootPageTable(UnsafeCell::new(PageTable([0; ENTRY_COUNT])));
static LEVEL_3_KERNEL_0: BootPageTable =
    BootPageTable(UnsafeCell::new(PageTable([0; ENTRY_COUNT])));
static LEVEL_3_KERNEL_1: BootPageTable =
    BootPageTable(UnsafeCell::new(PageTable([0; ENTRY_COUNT])));
static LEVEL_3_KERNEL_2: BootPageTable =
    BootPageTable(UnsafeCell::new(PageTable([0; ENTRY_COUNT])));
static DYNAMIC_LEVEL_2: [SharedPageTable; DYNAMIC_LEVEL_1_COUNT] = [
    SharedPageTable::new(),
    SharedPageTable::new(),
    SharedPageTable::new(),
    SharedPageTable::new(),
];
static MMU_ENABLED: AtomicBool = AtomicBool::new(false);
static PAN_SUPPORTED: AtomicBool = AtomicBool::new(false);
static PAGE_TABLE_LOCK: AtomicBool = AtomicBool::new(false);
static MAP_COUNT: AtomicU64 = AtomicU64::new(0);
static UNMAP_COUNT: AtomicU64 = AtomicU64::new(0);
static TLB_INVALIDATION_COUNT: AtomicU64 = AtomicU64::new(0);
static TABLE_ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static TABLE_FREE_COUNT: AtomicU64 = AtomicU64::new(0);
static ADDRESS_SPACE_TLB_INVALIDATION_COUNT: AtomicU64 = AtomicU64::new(0);
// One ownership bit for every 8-bit ASID. Bit zero is permanently reserved
// for the boot/kernel translation context; user allocation is deterministic
// first-fit, so an empty pool yields ASID 1, then ASID 2.
static USER_ASID_POOL: [AtomicU64; 4] = [
    AtomicU64::new(1),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];
static ADDRESS_SPACE_CREATION_COUNT: AtomicU64 = AtomicU64::new(0);
static ADDRESS_SPACE_RECYCLE_COUNT: AtomicU64 = AtomicU64::new(0);
static LIVE_ADDRESS_SPACE_COUNT: AtomicU64 = AtomicU64::new(0);
static USER_GRAPHICS_MAP_COUNT: AtomicU64 = AtomicU64::new(0);
static USER_GRAPHICS_UNMAP_COUNT: AtomicU64 = AtomicU64::new(0);
static USER_GRAPHICS_PROTECT_COUNT: AtomicU64 = AtomicU64::new(0);
static LIVE_USER_GRAPHICS_MAPPING_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vm-bookkeeping-only-unmap-self-test")]
static BOOKKEEPING_ONLY_UNMAP_INJECTED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" {
    static __text_start: u8;
    static __text_end: u8;
    static __rodata_start: u8;
    static __rodata_end: u8;
}

#[cfg(feature = "mmu-protection-self-test")]
pub unsafe fn trigger_text_write_fault() {
    let text = core::ptr::addr_of!(__text_start).cast_mut().cast::<u32>();
    unsafe {
        core::ptr::write_volatile(text, 0);
    }
}

#[derive(Clone, Copy)]
pub enum MmuError {
    AlreadyEnabled,
    Unsupported4KiBGranule,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapError {
    MmuNotEnabled,
    InvalidVirtualAddress,
    InvalidPhysicalFrame,
    OutOfMemory,
    AlreadyMapped,
    BlockConflict,
    Busy,
}

#[derive(Debug)]
pub struct MapFailure {
    pub error: MapError,
    pub frame: OwnedFrame,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranslationContext(u64);

impl TranslationContext {
    pub const fn from_raw(raw: u64) -> Option<Self> {
        if raw & !(TTBR_BASE_MASK | TTBR_ASID_MASK) != 0
            || raw & TTBR_BASE_MASK == 0
            || raw & (PAGE_SIZE as u64 - 1) != 0
        {
            return None;
        }
        Some(Self(raw))
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn asid(self) -> u8 {
        ((self.0 & TTBR_ASID_MASK) >> TTBR_ASID_SHIFT) as u8
    }

    pub const fn root_address(self) -> usize {
        (self.0 & TTBR_BASE_MASK) as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressSpaceError {
    MmuNotEnabled,
    InvalidVirtualAddress,
    InvalidPhysicalFrame,
    InvalidLayout,
    TooManyMappings,
    AsidInUse,
    OutOfMemory,
    Busy,
    KernelHierarchyCorrupt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum UserPageAccess {
    ReadExecute,
    #[allow(dead_code)]
    ReadOnly,
    ReadWrite,
}

/// The authority under which one fixed graphics-buffer alias is installed.
///
/// A producer starts read-write and may transition between read-write and
/// read-only as buffers move through queue/release. A consumer is permanently
/// read-only for the complete lifetime of its mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphicsMappingRole {
    Producer,
    Consumer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphicsMappingAccess {
    ReadOnly,
    ReadWrite,
}

impl GraphicsMappingAccess {
    const fn user_page_access(self) -> UserPageAccess {
        match self {
            Self::ReadOnly => UserPageAccess::ReadOnly,
            Self::ReadWrite => UserPageAccess::ReadWrite,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphicsMappingSnapshot {
    pub address: usize,
    pub backing_address: usize,
    pub pages: usize,
    pub identity: GraphicsBufferIdentity,
    pub producer_pid: u64,
    pub role: GraphicsMappingRole,
    pub access: GraphicsMappingAccess,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphicsMappingError {
    MmuNotEnabled,
    OutOfMemory,
    InvalidBuffer,
    InvalidAddress,
    AlreadyMapped,
    NotMapped,
    IdentityMismatch,
    WrongRole,
    WrongAccess,
    Busy,
    HierarchyCorrupt,
}

impl GraphicsMappingError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MmuNotEnabled => "MMU is not enabled",
            Self::OutOfMemory => "out of memory for graphics mapping metadata",
            Self::InvalidBuffer => "graphics buffer cannot be mapped",
            Self::InvalidAddress => "graphics mapping address is invalid",
            Self::AlreadyMapped => "graphics mapping slot is already occupied",
            Self::NotMapped => "graphics mapping does not exist",
            Self::IdentityMismatch => "graphics mapping names a different buffer generation",
            Self::WrongRole => "graphics mapping role forbids this operation",
            Self::WrongAccess => "graphics mapping has the wrong access state",
            Self::Busy => "page-table mutation is already in progress",
            Self::HierarchyCorrupt => "graphics mapping leaves disagree with their metadata",
        }
    }
}

#[derive(Debug)]
struct UserGraphicsBufferMapping {
    address: usize,
    backing_address: usize,
    role: GraphicsMappingRole,
    access: GraphicsMappingAccess,
    buffer: GraphicsBuffer,
}

impl UserGraphicsBufferMapping {
    fn snapshot(&self) -> GraphicsMappingSnapshot {
        GraphicsMappingSnapshot {
            address: self.address,
            backing_address: self.backing_address,
            pages: USER_GRAPHICS_MAPPING_PAGES,
            identity: self.buffer.identity(),
            producer_pid: self.buffer.producer_pid(),
            role: self.role,
            access: self.access,
        }
    }

    fn contains_page(&self, address: usize) -> bool {
        address >= self.address
            && address
                < self
                    .address
                    .checked_add(bndr_abi::GRAPHICS_BUFFER_BACKING_BYTES)
                    .unwrap_or_else(|| panic!("validated graphics mapping range overflowed"))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserVmaKind {
    Load,
    Stack,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserVma {
    start: usize,
    end: usize,
    access: UserPageAccess,
    kind: UserVmaKind,
}

impl UserVma {
    pub const fn new(start: usize, end: usize, access: UserPageAccess, kind: UserVmaKind) -> Self {
        Self {
            start,
            end,
            access,
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserLayout {
    entry: usize,
    stack_start: usize,
    stack_end: usize,
    vmas: [Option<UserVma>; MAX_USER_VMAS],
    vma_count: usize,
    guards: [Option<usize>; MAX_USER_GUARDS],
    guard_count: usize,
}

impl UserLayout {
    pub const fn new(entry: usize, stack_start: usize, stack_end: usize) -> Self {
        Self {
            entry,
            stack_start,
            stack_end,
            vmas: [None; MAX_USER_VMAS],
            vma_count: 0,
            guards: [None; MAX_USER_GUARDS],
            guard_count: 0,
        }
    }

    pub fn try_add_vma(&mut self, vma: UserVma) -> Result<(), AddressSpaceError> {
        let Some(slot) = self.vmas.get_mut(self.vma_count) else {
            return Err(AddressSpaceError::TooManyMappings);
        };
        *slot = Some(vma);
        self.vma_count += 1;
        Ok(())
    }

    pub fn try_add_guard(&mut self, address: usize) -> Result<(), AddressSpaceError> {
        let Some(slot) = self.guards.get_mut(self.guard_count) else {
            return Err(AddressSpaceError::TooManyMappings);
        };
        *slot = Some(address);
        self.guard_count += 1;
        Ok(())
    }
}

#[derive(Debug)]
struct OwnedUserPage {
    virtual_address: usize,
    frame: OwnedFrame,
    access: UserPageAccess,
}

#[derive(Debug)]
pub struct UserPageInsertFailure {
    pub error: AddressSpaceError,
    pub frame: OwnedFrame,
}

#[derive(Debug)]
struct UserAddressSpaceMetadata {
    layout: UserLayout,
    graphics_mappings: [Option<Box<UserGraphicsBufferMapping>>; USER_GRAPHICS_MAPPING_CAPACITY],
}

/// A bounded, fallibly allocated set of unpublished user-page ownership
/// tokens. Capacity is reserved before any frame is inserted, so `try_push`
/// never needs to allocate and every rejection returns the submitted frame.
#[must_use = "release the frames or transfer them to a user address space"]
#[derive(Debug)]
pub struct OwnedUserMappings {
    pages: Vec<OwnedUserPage>,
    expected_pages: usize,
    metadata: Box<UserAddressSpaceMetadata>,
}

impl OwnedUserMappings {
    pub fn try_new(expected_pages: usize, layout: UserLayout) -> Result<Self, AddressSpaceError> {
        if expected_pages == 0 || expected_pages > MAX_USER_MAPPED_PAGES {
            return Err(AddressSpaceError::TooManyMappings);
        }
        let mut pages = Vec::new();
        pages
            .try_reserve_exact(expected_pages)
            .map_err(|_| AddressSpaceError::OutOfMemory)?;
        let metadata =
            try_box_address_space_metadata(layout).ok_or(AddressSpaceError::OutOfMemory)?;
        Ok(Self {
            pages,
            expected_pages,
            metadata,
        })
    }

    pub fn try_push(
        &mut self,
        virtual_address: usize,
        frame: OwnedFrame,
        access: UserPageAccess,
    ) -> Result<(), UserPageInsertFailure> {
        let fail = |error, frame| UserPageInsertFailure { error, frame };
        if self.pages.len() >= self.expected_pages {
            return Err(fail(AddressSpaceError::TooManyMappings, frame));
        }
        if !valid_init_user_page(virtual_address)
            || self
                .pages
                .last()
                .is_some_and(|previous| previous.virtual_address >= virtual_address)
        {
            return Err(fail(AddressSpaceError::InvalidVirtualAddress, frame));
        }
        if !valid_owned_normal_frame(&frame)
            || self
                .pages
                .iter()
                .any(|page| page.frame.start_address() == frame.start_address())
        {
            return Err(fail(AddressSpaceError::InvalidPhysicalFrame, frame));
        }
        self.pages.push(OwnedUserPage {
            virtual_address,
            frame,
            access,
        });
        Ok(())
    }

    /// Releases a mapping set that was never installed in a translation
    /// hierarchy. Calling this for pages owned by `UserAddressSpace` is not
    /// possible through the public ownership API.
    pub fn release_unpublished(self) {
        for page in self.pages {
            release_destroyed_address_space_frame(page.frame, "unpublished user");
        }
    }

    fn page_physical(&self, virtual_address: usize) -> Option<usize> {
        self.pages
            .iter()
            .find(|page| page.virtual_address == virtual_address)
            .map(|page| page.frame.start_address())
    }
}

/// A non-destructive generic construction failure. The caller regains every
/// user frame while all private table frames and the provisional ASID have
/// already been rolled back internally.
#[must_use = "a failed construction still owns every submitted user frame"]
pub struct UserAddressSpaceFailure {
    pub error: AddressSpaceError,
    pub mappings: OwnedUserMappings,
}

/// Owns the complete private init translation hierarchy and its user leaves.
///
/// Kernel/MMIO subtrees are permanent boot objects referenced by this private
/// L1 and are therefore not owned here. Dropping this value intentionally does
/// not reclaim anything: a process reaper must call [`Self::destroy`] with a
/// matching [`StoppedAddressSpaceProof`] so hardware can no longer walk these
/// frames before their ownership is returned to the allocator.
#[must_use = "the address space owns page tables and user frames"]
pub struct UserAddressSpace {
    translation: TranslationContext,
    root: OwnedFrame,
    level_1: OwnedFrame,
    user_level_2: OwnedFrame,
    user_level_3: OwnedFrame,
    mappings: OwnedUserMappings,
    asid_lease: AsidLease,
}

struct UnpublishedUserTables {
    root: Option<OwnedFrame>,
    level_1: Option<OwnedFrame>,
    user_level_2: Option<OwnedFrame>,
    user_level_3: Option<OwnedFrame>,
}

struct AsidClaim {
    asid: u8,
    committed: bool,
}

struct AsidLease {
    asid: u8,
}

/// Linear evidence that a translation context has been removed from every
/// scheduler run queue and is no longer executing on any CPU.
///
/// The proof is deliberately neither `Copy` nor `Clone`. Its constructor is
/// crate-private and unsafe because the scheduler/reaper, not the MMU module,
/// must establish the global stopped-state invariant. The safe destroy path
/// additionally checks that the proof names the exact root and ASID it owns.
#[must_use = "a stopped proof must be consumed by address-space destruction"]
pub struct StoppedAddressSpaceProof {
    translation: TranslationContext,
}

impl StoppedAddressSpaceProof {
    /// Creates stopped-state evidence for one translation context.
    ///
    /// # Safety
    ///
    /// The caller must have removed `translation` from all scheduler queues,
    /// stopped every thread that can activate it, synchronized with every CPU
    /// that could still be executing it, and prevented future activation. No
    /// pointer, executable context, or in-flight page-table walker may depend
    /// on any private frame owned by that address space.
    pub(crate) const unsafe fn new(translation: TranslationContext) -> Self {
        Self { translation }
    }

    pub const fn translation_context(&self) -> TranslationContext {
        self.translation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressSpaceDestroyError {
    TranslationMismatch,
    StillActive,
    Busy,
    HierarchyCorrupt,
}

/// A non-destructive destroy failure. Both linear values are returned so the
/// reaper can repair its state or retry without leaking ownership.
#[must_use = "a failed destruction still owns its address space and stopped proof"]
pub struct AddressSpaceDestroyFailure {
    pub error: AddressSpaceDestroyError,
    pub address_space: UserAddressSpace,
    pub stopped: StoppedAddressSpaceProof,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AddressSpaceStats {
    /// Fully constructed address spaces returned to their caller.
    pub creations: u64,
    /// Address spaces completely destroyed through a stopped proof.
    pub recycles: u64,
    /// Successful creations minus successful explicit destruction.
    pub live: u64,
    /// ASID-targeted invalidations performed for construction and teardown.
    pub tlb_invalidations: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphicsMappingStats {
    pub maps: u64,
    /// Explicit unmaps plus stopped-address-space cleanup.
    pub unmaps: u64,
    /// Successful break-before-make permission transitions.
    pub protects: u64,
    pub live_mappings: u64,
    pub live_pages: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnmapError {
    MmuNotEnabled,
    InvalidVirtualAddress,
    NotMapped,
    BlockConflict,
    Busy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DynamicMappingStats {
    pub maps: u64,
    pub unmaps: u64,
    pub tlb_invalidations: u64,
    /// Allocator-backed L3 tables published below the permanent shared L2s.
    pub table_allocations: u64,
    /// Allocator-backed empty L3 tables reclaimed after invalidation.
    pub table_frees: u64,
}

impl MmuError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyEnabled => "MMU was already enabled at kernel entry",
            Self::Unsupported4KiBGranule => "CPU does not support a 4 KiB translation granule",
        }
    }
}

impl MapError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MmuNotEnabled => "MMU is not enabled",
            Self::InvalidVirtualAddress => "virtual page is outside the dynamic arena",
            Self::InvalidPhysicalFrame => "physical frame is invalid for normal RAM mapping",
            Self::OutOfMemory => "out of frames for an intermediate page table",
            Self::AlreadyMapped => "virtual page is already mapped",
            Self::BlockConflict => "page-table walk encountered a block mapping",
            Self::Busy => "page-table mutation is already in progress",
        }
    }
}

impl AddressSpaceError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MmuNotEnabled => "MMU is not enabled",
            Self::InvalidVirtualAddress => "user page is outside the private init window",
            Self::InvalidPhysicalFrame => "user or page-table physical frame is invalid",
            Self::InvalidLayout => "user VMAs, stack, entry, or guard layout is invalid",
            Self::TooManyMappings => "user mapping exceeds the bounded page or VMA limit",
            Self::AsidInUse => "all user ASIDs are already owned by live address spaces",
            Self::OutOfMemory => "out of memory for user ownership or translation metadata",
            Self::Busy => "page-table mutation is already in progress",
            Self::KernelHierarchyCorrupt => "shared kernel translation hierarchy is corrupt",
        }
    }
}

impl UnmapError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MmuNotEnabled => "MMU is not enabled",
            Self::InvalidVirtualAddress => "virtual page is outside the dynamic arena",
            Self::NotMapped => "virtual page is not mapped",
            Self::BlockConflict => "page-table walk encountered a block mapping",
            Self::Busy => "page-table mutation is already in progress",
        }
    }
}

/// Creates the early EL1 translation regime and enables the MMU.
///
/// The low gigabyte is Device-nGnRE for QEMU MMIO. RAM from 0x4000_0000 to
/// 0x8000_0000 is normal memory. Kernel text is read-only/executable, rodata is
/// read-only/non-executable, and every writable mapping is non-executable. The
/// 4--8 GiB dynamic arena is rooted in four permanent shared L2 tables so a
/// future address-space root can reuse the exact kernel mapping hierarchy.
///
/// # Safety
///
/// Must run exactly once on the boot CPU at EL1 with the MMU and data cache
/// disabled, before interrupts or SMP are enabled.
pub unsafe fn init_identity_map() -> Result<(), MmuError> {
    let sctlr_before: u64;
    let memory_features: u64;
    let memory_features_1: u64;
    unsafe {
        asm!(
            "mrs {sctlr}, SCTLR_EL1",
            "mrs {features}, ID_AA64MMFR0_EL1",
            "mrs {features_1}, ID_AA64MMFR1_EL1",
            sctlr = out(reg) sctlr_before,
            features = out(reg) memory_features,
            features_1 = out(reg) memory_features_1,
            options(nomem, nostack, preserves_flags)
        );
    }
    if sctlr_before & SCTLR_M != 0 {
        return Err(MmuError::AlreadyEnabled);
    }
    if (memory_features >> 28) & 0xf == 0xf {
        return Err(MmuError::Unsupported4KiBGranule);
    }

    let level_0 = unsafe { &mut *LEVEL_0.0.get() };
    let level_1 = unsafe { &mut *LEVEL_1.0.get() };
    let level_2 = unsafe { &mut *LEVEL_2_RAM.0.get() };
    let level_3_0 = unsafe { &mut *LEVEL_3_KERNEL_0.0.get() };
    let level_3_1 = unsafe { &mut *LEVEL_3_KERNEL_1.0.get() };
    let level_3_2 = unsafe { &mut *LEVEL_3_KERNEL_2.0.get() };
    level_0.0.fill(0);
    level_1.0.fill(0);
    level_2.0.fill(0);
    level_3_0.0.fill(0);
    level_3_1.0.fill(0);
    level_3_2.0.fill(0);

    level_0.0[0] = table_descriptor(level_1);

    // 0x0000_0000..0x3fff_ffff: QEMU virt MMIO, Device-nGnRE, RW, XN.
    level_1.0[0] = DESCRIPTOR_BLOCK
        | ACCESS_FLAG
        | ATTR_DEVICE
        | PRIVILEGED_EXECUTE_NEVER
        | UNPRIVILEGED_EXECUTE_NEVER;
    level_1.0[1] = table_descriptor(level_2);

    // Keep one permanent L2 root for every 1 GiB slice of the 4--8 GiB
    // dynamic arena. Runtime mapping only allocates L3 tables beneath these
    // roots; empty L2 roots remain published for the lifetime of the kernel.
    for (offset, storage) in DYNAMIC_LEVEL_2.iter().enumerate() {
        let table = unsafe { &mut *storage.0.get() };
        table.0.fill(0);
        level_1.0[DYNAMIC_LEVEL_1_FIRST + offset] = table_descriptor(table);
    }

    // Split the first 6 MiB of RAM into pages so a growing debug kernel can
    // retain exact text/rodata/data W^X permissions. A feature-specific BSS
    // tail (including M54's larger monitor stack) may occupy the following
    // already-normal, writable, non-executable 2 MiB block; remaining early
    // RAM uses the same non-executable block policy.
    level_2.0[0] = table_descriptor(level_3_0);
    level_2.0[1] = table_descriptor(level_3_1);
    level_2.0[2] = table_descriptor(level_3_2);
    for (index, entry) in level_2.0.iter_mut().enumerate().skip(3) {
        let address = EARLY_RAM_START + index * LEVEL_2_BLOCK_SIZE;
        *entry = address as u64
            | DESCRIPTOR_BLOCK
            | ACCESS_FLAG
            | INNER_SHAREABLE
            | ATTR_NORMAL
            | PRIVILEGED_EXECUTE_NEVER
            | UNPRIVILEGED_EXECUTE_NEVER;
    }

    let text = symbol_range(
        core::ptr::addr_of!(__text_start),
        core::ptr::addr_of!(__text_end),
    );
    let rodata = symbol_range(
        core::ptr::addr_of!(__rodata_start),
        core::ptr::addr_of!(__rodata_end),
    );
    for (table_index, level_3) in [level_3_0, level_3_1, level_3_2].into_iter().enumerate() {
        for (index, entry) in level_3.0.iter_mut().enumerate() {
            let address = EARLY_RAM_START + table_index * LEVEL_2_BLOCK_SIZE + index * PAGE_SIZE;
            let mut attributes = ACCESS_FLAG | INNER_SHAREABLE | ATTR_NORMAL;
            if text.contains(&address) {
                attributes |= READ_ONLY_EL1 | UNPRIVILEGED_EXECUTE_NEVER;
            } else if rodata.contains(&address) {
                attributes |= READ_ONLY_EL1 | PRIVILEGED_EXECUTE_NEVER | UNPRIVILEGED_EXECUTE_NEVER;
            } else {
                attributes |= PRIVILEGED_EXECUTE_NEVER | UNPRIVILEGED_EXECUTE_NEVER;
            }
            *entry = address as u64 | DESCRIPTOR_TABLE_OR_PAGE | attributes;
        }
    }

    let mair = 0x04_u64 | (0xff_u64 << 8); // Device-nGnRE, Normal WB/WA.
    let physical_address_range = (memory_features & 0xf).min(0b101);
    let tcr = 16_u64 // T0SZ: 48-bit lower virtual addresses.
        | (0b01 << 8) // IRGN0: inner write-back, write-allocate.
        | (0b01 << 10) // ORGN0: outer write-back, write-allocate.
        | (0b11 << 12) // SH0: inner shareable.
        | (16 << 16) // T1SZ, defined even while TTBR1 walks are disabled.
        | (1 << 23) // EPD1: disable TTBR1 walks during early boot.
        | (0b01 << 24) // IRGN1: inner write-back, write-allocate.
        | (0b01 << 26) // ORGN1: outer write-back, write-allocate.
        | (0b11 << 28) // SH1: inner shareable.
        | (0b10 << 30) // TG1: 4 KiB (00 is reserved for TG1).
        | (physical_address_range << 32);
    let ttbr0 = level_0.0.as_ptr() as u64;
    let pan_supported = (memory_features_1 >> 20) & 0xf != 0;
    let mut sctlr = SCTLR_EL1_RES1
        | SCTLR_M
        | SCTLR_C
        | SCTLR_SA
        | SCTLR_SA0
        | SCTLR_I
        | SCTLR_NTWI // Permit EL0 WFI; timer IRQ still preempts and reschedules it.
        | SCTLR_WXN;
    if pan_supported {
        // Bit 23 is RES1 before FEAT_PAN, but becomes SPAN when PAN exists.
        // SPAN=0 makes exceptions from EL0 automatically enter with PAN set.
        sctlr &= !(1 << 23);
    }

    unsafe {
        asm!(
            "msr MAIR_EL1, {mair}",
            "msr TCR_EL1, {tcr}",
            "msr TTBR0_EL1, {ttbr0}",
            "msr CNTKCTL_EL1, xzr",
            "msr PMUSERENR_EL0, xzr",
            "ic iallu",
            "dsb sy",
            "isb",
            "tlbi vmalle1",
            "dsb sy",
            "isb",
            "msr SCTLR_EL1, {sctlr}",
            "isb",
            mair = in(reg) mair,
            tcr = in(reg) tcr,
            ttbr0 = in(reg) ttbr0,
            sctlr = in(reg) sctlr,
            options(nostack, preserves_flags)
        );
    }
    if pan_supported {
        unsafe {
            asm!(
                "msr S3_0_C4_C2_3, {pan}",
                "isb",
                pan = in(reg) 1_u64 << 22,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
    PAN_SUPPORTED.store(pan_supported, Ordering::Release);
    MMU_ENABLED.store(true, Ordering::Release);
    Ok(())
}

pub fn pan_supported() -> bool {
    PAN_SUPPORTED.load(Ordering::Acquire)
}

pub fn pan_is_enabled() -> bool {
    if !pan_supported() {
        return false;
    }
    let pan: u64;
    unsafe {
        asm!(
            "mrs {pan}, S3_0_C4_C2_3",
            pan = out(reg) pan,
            options(nomem, nostack, preserves_flags)
        );
    }
    pan & (1 << 22) != 0
}

pub fn kernel_translation_context() -> TranslationContext {
    if !MMU_ENABLED.load(Ordering::Acquire) {
        panic!("kernel translation context requested before MMU initialization");
    }
    translation_context(LEVEL_0.0.get() as usize, KERNEL_ASID)
        .unwrap_or_else(|| panic!("kernel translation root is invalid"))
}

pub fn current_translation_context() -> TranslationContext {
    let ttbr: u64;
    unsafe {
        asm!(
            "mrs {ttbr}, TTBR0_EL1",
            ttbr = out(reg) ttbr,
            options(nomem, nostack, preserves_flags)
        );
    }
    TranslationContext(ttbr & (TTBR_BASE_MASK | TTBR_ASID_MASK))
}

pub fn current_asid() -> u8 {
    current_translation_context().asid()
}

/// Switches TTBR0_EL1 to a previously sealed translation context.
///
/// The single-core scheduler must keep IRQ masked across this operation and
/// its matching CURRENT_CONTEXT/state commit. A user ASID is not returned to
/// the pool until stopped-context teardown has completed its ASID TLBI, so a
/// normal context switch needs no per-switch invalidation.
pub fn activate_translation_context(context: TranslationContext) {
    if !super::irq_is_masked() {
        panic!("TTBR0 context switch attempted with IRQ enabled");
    }
    unsafe {
        asm!(
            "dsb ish",
            "msr TTBR0_EL1, {ttbr}",
            "isb",
            ttbr = in(reg) context.raw(),
            options(nostack, preserves_flags)
        );
    }
}

impl UnpublishedUserTables {
    fn try_allocate() -> Result<Self, AddressSpaceError> {
        let mut tables = Self {
            root: None,
            level_1: None,
            user_level_2: None,
            user_level_3: None,
        };
        tables.root = Some(memory::allocate_frame().map_err(|_| AddressSpaceError::OutOfMemory)?);
        tables.level_1 =
            Some(memory::allocate_frame().map_err(|_| AddressSpaceError::OutOfMemory)?);
        tables.user_level_2 =
            Some(memory::allocate_frame().map_err(|_| AddressSpaceError::OutOfMemory)?);
        tables.user_level_3 =
            Some(memory::allocate_frame().map_err(|_| AddressSpaceError::OutOfMemory)?);
        Ok(tables)
    }

    fn root(&self) -> &OwnedFrame {
        self.root
            .as_ref()
            .unwrap_or_else(|| panic!("unpublished user root was missing"))
    }

    fn level_1(&self) -> &OwnedFrame {
        self.level_1
            .as_ref()
            .unwrap_or_else(|| panic!("unpublished user L1 was missing"))
    }

    fn user_level_2(&self) -> &OwnedFrame {
        self.user_level_2
            .as_ref()
            .unwrap_or_else(|| panic!("unpublished user L2 was missing"))
    }

    fn user_level_3(&self) -> &OwnedFrame {
        self.user_level_3
            .as_ref()
            .unwrap_or_else(|| panic!("unpublished user L3 was missing"))
    }

    fn into_frames(mut self) -> (OwnedFrame, OwnedFrame, OwnedFrame, OwnedFrame) {
        (
            self.root
                .take()
                .unwrap_or_else(|| panic!("sealed user root was missing")),
            self.level_1
                .take()
                .unwrap_or_else(|| panic!("sealed user L1 was missing")),
            self.user_level_2
                .take()
                .unwrap_or_else(|| panic!("sealed user L2 was missing")),
            self.user_level_3
                .take()
                .unwrap_or_else(|| panic!("sealed user L3 was missing")),
        )
    }
}

impl Drop for UnpublishedUserTables {
    fn drop(&mut self) {
        for frame in [
            self.user_level_3.take(),
            self.user_level_2.take(),
            self.level_1.take(),
            self.root.take(),
        ]
        .into_iter()
        .flatten()
        {
            release_unpublished_table(frame);
        }
    }
}

impl UserAddressSpace {
    /// Builds one bounded multi-VMA address space without publishing it to the
    /// scheduler. Every user frame remains represented by one ownership token
    /// in `mappings`; construction failures return that complete set.
    pub fn try_new(mappings: OwnedUserMappings) -> Result<Self, UserAddressSpaceFailure> {
        let fail = |error, mappings| UserAddressSpaceFailure { error, mappings };
        if !MMU_ENABLED.load(Ordering::Acquire) {
            return Err(fail(AddressSpaceError::MmuNotEnabled, mappings));
        }
        if let Err(error) = validate_user_mappings(&mappings) {
            return Err(fail(error, mappings));
        }

        let Some(asid_claim) = AsidClaim::try_acquire() else {
            return Err(fail(AddressSpaceError::AsidInUse, mappings));
        };
        let tables = match UnpublishedUserTables::try_allocate() {
            Ok(tables) => tables,
            Err(error) => return Err(fail(error, mappings)),
        };
        let guard = match PageTableGuard::acquire() {
            Ok(guard) => guard,
            Err(()) => return Err(fail(AddressSpaceError::Busy, mappings)),
        };
        let hierarchy_valid = unsafe {
            initialize_user_hierarchy(
                tables.root(),
                tables.level_1(),
                tables.user_level_2(),
                tables.user_level_3(),
                &mappings,
            ) && verify_user_leaves(tables.user_level_3(), &mappings, &[])
        };
        if !hierarchy_valid {
            drop(guard);
            return Err(fail(AddressSpaceError::KernelHierarchyCorrupt, mappings));
        }

        let translation = translation_context(tables.root().start_address(), asid_claim.asid())
            .unwrap_or_else(|| panic!("allocator returned an invalid private root frame"));
        // This also removes translations left by an earlier owner of a
        // recycled ASID before the new root can be published.
        unsafe { invalidate_asid(asid_claim.asid()) };
        drop(guard);

        let (root, level_1, user_level_2, user_level_3) = tables.into_frames();
        let address_space = Self {
            translation,
            root,
            level_1,
            user_level_2,
            user_level_3,
            mappings,
            asid_lease: asid_claim.commit(),
        };
        ADDRESS_SPACE_CREATION_COUNT.fetch_add(1, Ordering::Relaxed);
        LIVE_ADDRESS_SPACE_COUNT.fetch_add(1, Ordering::Relaxed);
        Ok(address_space)
    }

    pub const fn translation_context(&self) -> TranslationContext {
        self.translation
    }

    pub const fn root_address(&self) -> usize {
        self.translation.root_address()
    }

    pub const fn asid(&self) -> u8 {
        self.translation.asid()
    }

    pub fn code_physical(&self) -> usize {
        let entry_page = self.mappings.metadata.layout.entry & !(PAGE_SIZE - 1);
        self.mappings
            .page_physical(entry_page)
            .unwrap_or_else(|| panic!("address-space entry page lost its physical owner"))
    }

    pub fn stack_physical(&self) -> usize {
        self.mappings
            .page_physical(self.mappings.metadata.layout.stack_start)
            .unwrap_or_else(|| panic!("address-space stack lost its physical owner"))
    }

    pub fn mapped_page_count(&self) -> usize {
        self.mappings.pages.len()
    }

    pub fn page_physical(&self, virtual_address: usize) -> Option<usize> {
        self.mappings.page_physical(virtual_address)
    }

    /// Installs one fixed-address alias of a generation-pinned graphics
    /// buffer. The backing pages remain owned by the static graphics pool;
    /// this address space owns only a `GraphicsBuffer` clone which prevents
    /// scrub/reuse until every leaf has been invalidated.
    pub fn map_graphics_buffer(
        &mut self,
        buffer: GraphicsBuffer,
        role: GraphicsMappingRole,
    ) -> Result<GraphicsMappingSnapshot, GraphicsMappingError> {
        if !MMU_ENABLED.load(Ordering::Acquire) {
            return Err(GraphicsMappingError::MmuNotEnabled);
        }
        if !buffer.is_mappable() {
            return Err(GraphicsMappingError::InvalidBuffer);
        }
        let slot = usize::from(buffer.slot());
        let Some(address) = graphics_mapping_address(slot) else {
            return Err(GraphicsMappingError::InvalidBuffer);
        };
        if self.mappings.metadata.graphics_mappings[slot].is_some()
            || self
                .mappings
                .metadata
                .graphics_mappings
                .iter()
                .flatten()
                .any(|mapping| mapping.buffer.same_buffer(&buffer))
        {
            return Err(GraphicsMappingError::AlreadyMapped);
        }
        let backing_address = buffer.backing_address();
        if !valid_graphics_backing(backing_address) {
            return Err(GraphicsMappingError::InvalidBuffer);
        }
        let access = match role {
            GraphicsMappingRole::Producer => GraphicsMappingAccess::ReadWrite,
            GraphicsMappingRole::Consumer => GraphicsMappingAccess::ReadOnly,
        };
        // Allocate the small pin record before publishing any descriptor. The
        // address space itself retains only two optional pointers, keeping all
        // spawn/reap by-value paths below the fixed kernel-stack budget.
        let mapping = try_box_graphics_mapping(UserGraphicsBufferMapping {
            address,
            backing_address,
            role,
            access,
            buffer,
        })
        .map_err(|mapping| {
            drop(mapping);
            GraphicsMappingError::OutOfMemory
        })?;
        let snapshot = mapping.snapshot();

        let guard = PageTableGuard::acquire().map_err(|()| GraphicsMappingError::Busy)?;
        if !unsafe { graphics_mapping_range_is_unmapped(&self.user_level_3, address) } {
            drop(guard);
            return Err(GraphicsMappingError::AlreadyMapped);
        }
        // SAFETY: the fixed range is private to this address space, every
        // target leaf was just proven invalid, and the page-table guard holds
        // IRQ exclusion through descriptor publication and ASID invalidation.
        unsafe {
            write_graphics_mapping_leaves(&self.user_level_3, address, backing_address, access);
            invalidate_asid(self.translation.asid());
        }
        self.mappings.metadata.graphics_mappings[slot] = Some(mapping);
        USER_GRAPHICS_MAP_COUNT.fetch_add(1, Ordering::Relaxed);
        LIVE_USER_GRAPHICS_MAPPING_COUNT.fetch_add(1, Ordering::Relaxed);
        drop(guard);
        Ok(snapshot)
    }

    /// Changes the authenticated producer alias between writable/dequeued and
    /// read-only/queued states using Arm break-before-make ordering.
    pub fn protect_graphics_buffer_mapping(
        &mut self,
        buffer: &GraphicsBuffer,
        expected: GraphicsMappingAccess,
        replacement: GraphicsMappingAccess,
    ) -> Result<GraphicsMappingSnapshot, GraphicsMappingError> {
        if expected == replacement {
            return Err(GraphicsMappingError::WrongAccess);
        }
        let slot = usize::from(buffer.slot());
        let Some(mapping) = self
            .mappings
            .metadata
            .graphics_mappings
            .get(slot)
            .and_then(Option::as_ref)
        else {
            return Err(GraphicsMappingError::NotMapped);
        };
        if !mapping.buffer.same_buffer(buffer) {
            return Err(GraphicsMappingError::IdentityMismatch);
        }
        if mapping.role != GraphicsMappingRole::Producer {
            return Err(GraphicsMappingError::WrongRole);
        }
        if mapping.access != expected {
            return Err(GraphicsMappingError::WrongAccess);
        }
        let address = mapping.address;
        let backing_address = mapping.backing_address;

        let guard = PageTableGuard::acquire().map_err(|()| GraphicsMappingError::Busy)?;
        if !unsafe {
            graphics_mapping_leaves_match(&self.user_level_3, address, backing_address, expected)
        } {
            drop(guard);
            return Err(GraphicsMappingError::HierarchyCorrupt);
        }
        // SAFETY: all old leaves exactly match the pinned mapping. Clearing,
        // invalidating, publishing the replacement attributes, and performing
        // the second invalidation implement break-before-make for this ASID.
        unsafe {
            clear_graphics_mapping_leaves(&self.user_level_3, address);
            invalidate_asid(self.translation.asid());
            write_graphics_mapping_leaves(
                &self.user_level_3,
                address,
                backing_address,
                replacement,
            );
            invalidate_asid(self.translation.asid());
        }
        let mapping = self.mappings.metadata.graphics_mappings[slot]
            .as_mut()
            .unwrap_or_else(|| panic!("graphics mapping vanished under page-table guard"));
        mapping.access = replacement;
        let snapshot = mapping.snapshot();
        USER_GRAPHICS_PROTECT_COUNT.fetch_add(1, Ordering::Relaxed);
        drop(guard);
        Ok(snapshot)
    }

    /// Removes a normal mapping. A producer read-only mapping is queued and
    /// cannot be discarded before SurfaceServer releases it; stopped-process
    /// destruction has a separate unconditional cleanup path.
    pub fn unmap_graphics_buffer(
        &mut self,
        buffer: &GraphicsBuffer,
        address: usize,
    ) -> Result<GraphicsMappingSnapshot, GraphicsMappingError> {
        let Some(slot) = graphics_mapping_slot_for_address(address) else {
            return Err(GraphicsMappingError::InvalidAddress);
        };
        let Some(mapping) = self.mappings.metadata.graphics_mappings[slot].as_ref() else {
            return Err(GraphicsMappingError::NotMapped);
        };
        if mapping.address != address {
            return Err(GraphicsMappingError::InvalidAddress);
        }
        if !mapping.buffer.same_buffer(buffer) {
            return Err(GraphicsMappingError::IdentityMismatch);
        }
        if mapping.role == GraphicsMappingRole::Producer
            && mapping.access == GraphicsMappingAccess::ReadOnly
        {
            return Err(GraphicsMappingError::WrongAccess);
        }
        let backing_address = mapping.backing_address;
        let access = mapping.access;

        let guard = PageTableGuard::acquire().map_err(|()| GraphicsMappingError::Busy)?;
        if !unsafe {
            graphics_mapping_leaves_match(&self.user_level_3, address, backing_address, access)
        } {
            drop(guard);
            return Err(GraphicsMappingError::HierarchyCorrupt);
        }
        // SAFETY: the exact live leaves are cleared while the pin remains in
        // `graphics_mappings`; ASID invalidation completes before it is taken.
        unsafe {
            clear_graphics_mapping_leaves(&self.user_level_3, address);
            invalidate_asid(self.translation.asid());
        }
        let mapping = self.mappings.metadata.graphics_mappings[slot]
            .take()
            .unwrap_or_else(|| panic!("graphics mapping vanished under page-table guard"));
        let snapshot = mapping.snapshot();
        record_graphics_mapping_removals(1);
        drop(guard);
        drop(mapping);
        Ok(snapshot)
    }

    pub fn graphics_mapping_snapshot(
        &self,
        buffer: &GraphicsBuffer,
    ) -> Option<GraphicsMappingSnapshot> {
        self.mappings
            .metadata
            .graphics_mappings
            .get(usize::from(buffer.slot()))?
            .as_ref()
            .filter(|mapping| mapping.buffer.same_buffer(buffer))
            .map(|mapping| mapping.snapshot())
    }

    pub fn graphics_mapping_snapshots(
        &self,
    ) -> [Option<GraphicsMappingSnapshot>; USER_GRAPHICS_MAPPING_CAPACITY] {
        core::array::from_fn(|slot| {
            self.mappings.metadata.graphics_mappings[slot]
                .as_ref()
                .map(|mapping| mapping.snapshot())
        })
    }

    /// Clones the generation pins for lifecycle teardown without relying on
    /// userspace handles still being open.
    ///
    /// A process may close the handle used to create a mapping; the mapping's
    /// private heap record remains the authoritative pin until its leaves are
    /// invalidated. The fixed array performs no allocation and lets the
    /// reaper retain each buffer across [`Self::destroy`].
    pub fn pinned_graphics_mappings(
        &self,
    ) -> [Option<(GraphicsBuffer, GraphicsMappingSnapshot)>; USER_GRAPHICS_MAPPING_CAPACITY] {
        core::array::from_fn(|slot| {
            self.mappings.metadata.graphics_mappings[slot]
                .as_ref()
                .map(|mapping| (mapping.buffer.clone(), mapping.snapshot()))
        })
    }

    /// Proves that every page touched by one user destination range is a live
    /// read-write mapping owned or generation-pinned by this address space.
    /// Mapping mutations and the immediately following user copy share the
    /// same IRQ-masked process domain, so the checked access cannot change in
    /// between.
    pub fn user_range_is_writable(&self, address: u64, bytes: usize) -> bool {
        self.user_range_has_access(address, bytes, |access| access == UserPageAccess::ReadWrite)
    }

    /// Proves that every page touched by one user source range is mapped and
    /// readable from EL0. All three supported user leaf classes are readable;
    /// execute permission never substitutes for a missing user mapping.
    ///
    /// Mapping mutation is serialized in the same IRQ-masked process domain,
    /// so this validation can precede one all-or-nothing copy-from-user.
    pub fn user_range_is_readable(&self, address: u64, bytes: usize) -> bool {
        self.user_range_has_access(address, bytes, |_| true)
    }

    fn user_range_has_access(
        &self,
        address: u64,
        bytes: usize,
        accepts: impl Fn(UserPageAccess) -> bool,
    ) -> bool {
        let Ok(start) = usize::try_from(address) else {
            return false;
        };
        if bytes == 0 {
            return true;
        }
        let Some(last) = start.checked_add(bytes - 1) else {
            return false;
        };
        let first_page = start & !(PAGE_SIZE - 1);
        let last_page = last & !(PAGE_SIZE - 1);
        let mut page = first_page;
        loop {
            let owned_access = self
                .mappings
                .pages
                .iter()
                .find(|mapping| mapping.virtual_address == page)
                .map(|mapping| mapping.access);
            let borrowed_access = self
                .mappings
                .metadata
                .graphics_mappings
                .iter()
                .flatten()
                .find(|mapping| mapping.contains_page(page))
                .map(|mapping| mapping.access.user_page_access());
            if owned_access
                .or(borrowed_access)
                .is_none_or(|access| !accepts(access))
            {
                return false;
            }
            if page == last_page {
                return true;
            }
            let Some(next) = page.checked_add(PAGE_SIZE) else {
                return false;
            };
            page = next;
        }
    }

    /// Checks every image, data, and stack owner rather than comparing only
    /// representative code/stack pages or a lossy fingerprint.
    pub fn user_frames_are_disjoint(&self, other: &Self) -> bool {
        self.mappings.pages.iter().all(|page| {
            other
                .mappings
                .pages
                .iter()
                .all(|other_page| page.frame.start_address() != other_page.frame.start_address())
        })
    }

    pub const fn private_table_frames(&self) -> usize {
        let _ = (
            &self.root,
            &self.level_1,
            &self.user_level_2,
            &self.user_level_3,
        );
        4
    }

    pub fn query_page_descriptor(&self, virtual_address: usize) -> Result<Option<u64>, UnmapError> {
        let _guard = PageTableGuard::acquire().map_err(|()| UnmapError::Busy)?;
        unsafe { walk_page_descriptor(self.root.start_address(), virtual_address) }
    }

    pub fn guards_are_unmapped(&self) -> Result<bool, UnmapError> {
        for index in 0..self.mappings.metadata.layout.guard_count {
            let address = self.mappings.metadata.layout.guards[index]
                .unwrap_or_else(|| panic!("address-space layout lost a guard page"));
            if self.query_page_descriptor(address)?.is_some() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Reclaims this stopped address space in hardware-safe ownership order.
    ///
    /// The stopped proof prevents any future activation. After an ASID-wide
    /// invalidation completes, all user leaves are released first, then their
    /// private L3, L2, L1, and root tables. The ASID ownership bit is
    /// deliberately the final resource released, so it cannot be reused while
    /// any frame reachable through its former translation context is live.
    // The non-destructive error must return the complete address-space and
    // every graphics-buffer pin without attempting a fallible heap allocation.
    #[allow(clippy::result_large_err)]
    pub fn destroy(
        self,
        stopped: StoppedAddressSpaceProof,
    ) -> Result<(), AddressSpaceDestroyFailure> {
        if stopped.translation != self.translation {
            return Err(AddressSpaceDestroyFailure {
                error: AddressSpaceDestroyError::TranslationMismatch,
                address_space: self,
                stopped,
            });
        }
        if current_translation_context() == self.translation {
            return Err(AddressSpaceDestroyFailure {
                error: AddressSpaceDestroyError::StillActive,
                address_space: self,
                stopped,
            });
        }
        let guard = match PageTableGuard::acquire() {
            Ok(guard) => guard,
            Err(()) => {
                return Err(AddressSpaceDestroyFailure {
                    error: AddressSpaceDestroyError::Busy,
                    address_space: self,
                    stopped,
                });
            }
        };

        if !unsafe {
            verify_user_leaves(
                &self.user_level_3,
                &self.mappings,
                &self.mappings.metadata.graphics_mappings,
            )
        } {
            drop(guard);
            return Err(AddressSpaceDestroyFailure {
                error: AddressSpaceDestroyError::HierarchyCorrupt,
                address_space: self,
                stopped,
            });
        }
        // SAFETY: every page belongs to this private L3, the stopped proof
        // prevents all future walkers, and the page-table guard serializes the
        // descriptor writes through the completing ASID-wide invalidation.
        unsafe {
            clear_user_leaves(
                &self.user_level_3,
                &self.mappings,
                &self.mappings.metadata.graphics_mappings,
            );
            invalidate_asid(self.translation.asid());
        }
        let removed_graphics_mappings = self
            .mappings
            .metadata
            .graphics_mappings
            .iter()
            .flatten()
            .count() as u64;
        record_graphics_mapping_removals(removed_graphics_mappings);

        let Self {
            translation,
            root,
            level_1,
            user_level_2,
            user_level_3,
            mut mappings,
            asid_lease,
        } = self;
        if translation.asid() != asid_lease.asid() {
            panic!("address-space ASID lease does not match its translation context");
        }
        // Heap-backed page metadata must not be dropped while PAGE_TABLE_LOCK
        // is held because the global allocator has its own IRQ-masked lock.
        drop(guard);

        // These clones are the ownership pins for borrowed static backing.
        // Their final drop may scrub and recycle a pool slot, so it must occur
        // only after every leaf and stale ASID translation is gone.
        for mapping in &mut mappings.metadata.graphics_mappings {
            drop(mapping.take());
        }

        for page in mappings.pages {
            release_destroyed_address_space_frame(page.frame, "user leaf");
        }
        release_destroyed_address_space_frame(user_level_3, "L3 table");
        release_destroyed_address_space_frame(user_level_2, "L2 table");
        release_destroyed_address_space_frame(level_1, "L1 table");
        release_destroyed_address_space_frame(root, "root table");

        ADDRESS_SPACE_RECYCLE_COUNT.fetch_add(1, Ordering::Relaxed);
        let previous_live = LIVE_ADDRESS_SPACE_COUNT.fetch_sub(1, Ordering::AcqRel);
        if previous_live == 0 {
            panic!("address-space live count underflow during destruction");
        }
        asid_lease.release();
        Ok(())
    }
}

impl AsidClaim {
    fn try_acquire() -> Option<Self> {
        for asid in 1..=u8::MAX {
            let word = usize::from(asid) / u64::BITS as usize;
            let mask = 1_u64 << (usize::from(asid) % u64::BITS as usize);
            let previous = USER_ASID_POOL[word].fetch_or(mask, Ordering::AcqRel);
            if previous & mask == 0 {
                return Some(Self {
                    asid,
                    committed: false,
                });
            }
        }
        None
    }

    const fn asid(&self) -> u8 {
        self.asid
    }

    fn commit(mut self) -> AsidLease {
        self.committed = true;
        AsidLease { asid: self.asid }
    }
}

impl Drop for AsidClaim {
    fn drop(&mut self) {
        if !self.committed {
            release_user_asid(self.asid);
        }
    }
}

impl AsidLease {
    const fn asid(&self) -> u8 {
        self.asid
    }

    fn release(self) {
        release_user_asid(self.asid);
    }
}

pub fn query_boot_page_descriptor(virtual_address: usize) -> Result<Option<u64>, UnmapError> {
    if !MMU_ENABLED.load(Ordering::Acquire) {
        return Err(UnmapError::MmuNotEnabled);
    }
    if virtual_address >= (1_usize << 48) || !virtual_address.is_multiple_of(PAGE_SIZE) {
        return Err(UnmapError::InvalidVirtualAddress);
    }
    let _guard = PageTableGuard::acquire().map_err(|()| UnmapError::Busy)?;
    unsafe { walk_page_descriptor(LEVEL_0.0.get() as usize, virtual_address) }
}

pub fn address_space_tlb_invalidations() -> u64 {
    ADDRESS_SPACE_TLB_INVALIDATION_COUNT.load(Ordering::Acquire)
}

pub fn address_space_stats() -> AddressSpaceStats {
    AddressSpaceStats {
        creations: ADDRESS_SPACE_CREATION_COUNT.load(Ordering::Acquire),
        recycles: ADDRESS_SPACE_RECYCLE_COUNT.load(Ordering::Acquire),
        live: LIVE_ADDRESS_SPACE_COUNT.load(Ordering::Acquire),
        tlb_invalidations: address_space_tlb_invalidations(),
    }
}

pub fn graphics_mapping_stats() -> GraphicsMappingStats {
    let live_mappings = LIVE_USER_GRAPHICS_MAPPING_COUNT.load(Ordering::Acquire);
    GraphicsMappingStats {
        maps: USER_GRAPHICS_MAP_COUNT.load(Ordering::Acquire),
        unmaps: USER_GRAPHICS_UNMAP_COUNT.load(Ordering::Acquire),
        protects: USER_GRAPHICS_PROTECT_COUNT.load(Ordering::Acquire),
        live_mappings,
        live_pages: live_mappings
            .checked_mul(USER_GRAPHICS_MAPPING_PAGES as u64)
            .unwrap_or_else(|| panic!("live graphics mapping page count overflowed")),
    }
}

/// Maps one allocator-owned frame as kernel read/write, normal, and
/// non-executable in the dedicated 4--8 GiB dynamic arena.
///
/// The ownership token is consumed only on success. The arena's L2 roots are
/// permanent static tables; only an L3 table is allocated from the frame
/// allocator when a 2 MiB region receives its first leaf.
pub fn map_kernel_rw_page(virtual_address: usize, frame: OwnedFrame) -> Result<(), MapFailure> {
    let physical_address = frame.start_address();
    let leaf = match kernel_rw_nx_page_descriptor(physical_address) {
        Ok(leaf) => leaf,
        Err(_) => {
            return Err(MapFailure {
                error: MapError::InvalidPhysicalFrame,
                frame,
            });
        }
    };
    map_page(virtual_address, frame, leaf)
}

fn map_page(virtual_address: usize, frame: OwnedFrame, leaf: u64) -> Result<(), MapFailure> {
    if !MMU_ENABLED.load(Ordering::Acquire) {
        return Err(MapFailure {
            error: MapError::MmuNotEnabled,
            frame,
        });
    }
    if !valid_dynamic_page(virtual_address) {
        return Err(MapFailure {
            error: MapError::InvalidVirtualAddress,
            frame,
        });
    }
    let physical_address = frame.start_address();
    if !physical_address.is_multiple_of(PAGE_SIZE)
        || !(EARLY_RAM_START..EARLY_RAM_END).contains(&physical_address)
    {
        return Err(MapFailure {
            error: MapError::InvalidPhysicalFrame,
            frame,
        });
    }

    let _guard = match PageTableGuard::acquire() {
        Ok(guard) => guard,
        Err(()) => {
            return Err(MapFailure {
                error: MapError::Busy,
                frame,
            });
        }
    };
    unsafe { map_page_locked(virtual_address, frame, leaf) }
}

/// Removes one dynamic mapping and returns its frame ownership only after all
/// descriptor writes and TLB/walk-cache invalidation are complete.
///
/// # Safety
///
/// No reference, allocator metadata, executable context, or concurrent raw
/// access may still depend on this virtual page. The caller must also ensure
/// that no subsystem treats the mapping as permanent (in particular, live
/// kernel-heap backing pages must never be passed here).
pub unsafe fn unmap_kernel_page(virtual_address: usize) -> Result<OwnedFrame, UnmapError> {
    if !MMU_ENABLED.load(Ordering::Acquire) {
        return Err(UnmapError::MmuNotEnabled);
    }
    if !valid_dynamic_page(virtual_address) {
        return Err(UnmapError::InvalidVirtualAddress);
    }
    let _guard = PageTableGuard::acquire().map_err(|()| UnmapError::Busy)?;
    unsafe { unmap_kernel_page_locked(virtual_address) }
}

pub fn query_kernel_page(virtual_address: usize) -> Result<Option<PhysFrame>, UnmapError> {
    query_page_descriptor(virtual_address).map(|descriptor| {
        descriptor.map(|leaf| unsafe {
            PhysFrame::from_start_address_unchecked(descriptor_address(leaf))
        })
    })
}

/// Returns the complete leaf descriptor for one dynamic virtual page.
///
/// This is an internal verification surface used to prove that a published
/// user mapping has the requested AP/PXN/UXN/nG attributes, rather than only
/// checking that some physical frame is reachable.
pub fn query_page_descriptor(virtual_address: usize) -> Result<Option<u64>, UnmapError> {
    if !MMU_ENABLED.load(Ordering::Acquire) {
        return Err(UnmapError::MmuNotEnabled);
    }
    if !valid_dynamic_page(virtual_address) {
        return Err(UnmapError::InvalidVirtualAddress);
    }
    let _guard = PageTableGuard::acquire().map_err(|()| UnmapError::Busy)?;
    unsafe { query_page_descriptor_locked(virtual_address) }
}

pub fn dynamic_mapping_stats() -> DynamicMappingStats {
    DynamicMappingStats {
        maps: MAP_COUNT.load(Ordering::Acquire),
        unmaps: UNMAP_COUNT.load(Ordering::Acquire),
        tlb_invalidations: TLB_INVALIDATION_COUNT.load(Ordering::Acquire),
        table_allocations: TABLE_ALLOC_COUNT.load(Ordering::Acquire),
        table_frees: TABLE_FREE_COUNT.load(Ordering::Acquire),
    }
}

unsafe fn map_page_locked(
    virtual_address: usize,
    frame: OwnedFrame,
    leaf: u64,
) -> Result<(), MapFailure> {
    let l1_index = (virtual_address >> 30) & 0x1ff;
    let l2_index = (virtual_address >> 21) & 0x1ff;
    let l3_index = (virtual_address >> 12) & 0x1ff;
    let l1_entry = unsafe { (*LEVEL_1.0.get()).0.as_ptr().add(l1_index) };
    let l1_value = unsafe { ptr::read_volatile(l1_entry) };
    match descriptor_type(l1_value) {
        0 => panic!("dynamic arena lost its permanent L2 root"),
        DESCRIPTOR_BLOCK => {
            return Err(MapFailure {
                error: MapError::BlockConflict,
                frame,
            });
        }
        DESCRIPTOR_TABLE_OR_PAGE => {
            let l2 = table_from_descriptor(l1_value);
            let l2_entry = unsafe { (*l2).0.as_mut_ptr().add(l2_index) };
            let l2_value = unsafe { ptr::read_volatile(l2_entry) };
            match descriptor_type(l2_value) {
                0 => {
                    let Ok(l3_owner) = memory::allocate_frame() else {
                        return Err(MapFailure {
                            error: MapError::OutOfMemory,
                            frame,
                        });
                    };
                    let l3_address = l3_owner.start_address();
                    let l3 = unsafe { zero_table(&l3_owner) };
                    unsafe {
                        ptr::write_volatile((*l3).0.as_mut_ptr().add(l3_index), leaf);
                        publish_new_hierarchy(l2_entry, table_descriptor_address(l3_address));
                    }
                    TABLE_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
                }
                DESCRIPTOR_BLOCK => {
                    return Err(MapFailure {
                        error: MapError::BlockConflict,
                        frame,
                    });
                }
                DESCRIPTOR_TABLE_OR_PAGE => {
                    let l3 = table_from_descriptor(l2_value);
                    let l3_entry = unsafe { (*l3).0.as_mut_ptr().add(l3_index) };
                    if descriptor_type(unsafe { ptr::read_volatile(l3_entry) }) != 0 {
                        return Err(MapFailure {
                            error: MapError::AlreadyMapped,
                            frame,
                        });
                    }
                    unsafe {
                        ptr::write_volatile(l3_entry, leaf);
                        invalidate_all_translations();
                    }
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }

    MAP_COUNT.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

unsafe fn unmap_kernel_page_locked(virtual_address: usize) -> Result<OwnedFrame, UnmapError> {
    let l1_index = (virtual_address >> 30) & 0x1ff;
    let l2_index = (virtual_address >> 21) & 0x1ff;
    let l3_index = (virtual_address >> 12) & 0x1ff;
    let l1_entry = unsafe { (*LEVEL_1.0.get()).0.as_ptr().add(l1_index) };
    let l1_value = unsafe { ptr::read_volatile(l1_entry) };
    match descriptor_type(l1_value) {
        0 => return Err(UnmapError::NotMapped),
        DESCRIPTOR_BLOCK => return Err(UnmapError::BlockConflict),
        DESCRIPTOR_TABLE_OR_PAGE => {}
        _ => unreachable!(),
    }

    let l2 = table_from_descriptor(l1_value);
    let l2_entry = unsafe { (*l2).0.as_mut_ptr().add(l2_index) };
    let l2_value = unsafe { ptr::read_volatile(l2_entry) };
    match descriptor_type(l2_value) {
        0 => return Err(UnmapError::NotMapped),
        DESCRIPTOR_BLOCK => return Err(UnmapError::BlockConflict),
        DESCRIPTOR_TABLE_OR_PAGE => {}
        _ => unreachable!(),
    }

    let l3 = table_from_descriptor(l2_value);
    let l3_entry = unsafe { (*l3).0.as_mut_ptr().add(l3_index) };
    let leaf = unsafe { ptr::read_volatile(l3_entry) };
    if descriptor_type(leaf) == 0 {
        return Err(UnmapError::NotMapped);
    }
    if descriptor_type(leaf) != DESCRIPTOR_TABLE_OR_PAGE {
        return Err(UnmapError::BlockConflict);
    }

    #[cfg(feature = "vm-bookkeeping-only-unmap-self-test")]
    if !BOOKKEEPING_ONLY_UNMAP_INJECTED.swap(true, Ordering::AcqRel) {
        UNMAP_COUNT.fetch_add(1, Ordering::Relaxed);
        return Ok(unsafe {
            OwnedFrame::from_allocated_address_unchecked(descriptor_address(leaf))
        });
    }

    unsafe { ptr::write_volatile(l3_entry, 0) };
    let l3_empty = unsafe { table_is_empty(l3) };
    if l3_empty {
        unsafe { ptr::write_volatile(l2_entry, 0) };
    }

    // This completion point must precede reconstruction or release of every
    // ownership token whose physical page was reachable by the old walk.
    unsafe { invalidate_all_translations() };

    if l3_empty {
        reclaim_detached_l3(descriptor_address(l2_value));
    }

    UNMAP_COUNT.fetch_add(1, Ordering::Relaxed);
    Ok(unsafe { OwnedFrame::from_allocated_address_unchecked(descriptor_address(leaf)) })
}

unsafe fn query_page_descriptor_locked(virtual_address: usize) -> Result<Option<u64>, UnmapError> {
    let l1_value = unsafe {
        ptr::read_volatile(
            (*LEVEL_1.0.get())
                .0
                .as_ptr()
                .add((virtual_address >> 30) & 0x1ff),
        )
    };
    match descriptor_type(l1_value) {
        0 => return Ok(None),
        DESCRIPTOR_BLOCK => return Err(UnmapError::BlockConflict),
        DESCRIPTOR_TABLE_OR_PAGE => {}
        _ => unreachable!(),
    }
    let l2 = table_from_descriptor(l1_value);
    let l2_value =
        unsafe { ptr::read_volatile((*l2).0.as_ptr().add((virtual_address >> 21) & 0x1ff)) };
    match descriptor_type(l2_value) {
        0 => return Ok(None),
        DESCRIPTOR_BLOCK => return Err(UnmapError::BlockConflict),
        DESCRIPTOR_TABLE_OR_PAGE => {}
        _ => unreachable!(),
    }
    let l3 = table_from_descriptor(l2_value);
    let leaf = unsafe { ptr::read_volatile((*l3).0.as_ptr().add((virtual_address >> 12) & 0x1ff)) };
    match descriptor_type(leaf) {
        0 => Ok(None),
        DESCRIPTOR_TABLE_OR_PAGE => Ok(Some(leaf)),
        _ => Err(UnmapError::BlockConflict),
    }
}

struct PageTableGuard {
    saved_daif: u64,
}

impl PageTableGuard {
    fn acquire() -> Result<Self, ()> {
        let saved_daif = super::save_and_mask_irq();
        if PAGE_TABLE_LOCK
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            super::restore_daif(saved_daif);
            return Err(());
        }
        Ok(Self { saved_daif })
    }
}

impl Drop for PageTableGuard {
    fn drop(&mut self) {
        PAGE_TABLE_LOCK.store(false, Ordering::Release);
        super::restore_daif(self.saved_daif);
    }
}

unsafe fn zero_table(frame: &OwnedFrame) -> *mut PageTable {
    let table = frame.start_address() as *mut PageTable;
    unsafe { ptr::write_bytes(table.cast::<u8>(), 0, PAGE_SIZE) };
    table
}

unsafe fn publish_new_hierarchy(entry: *mut u64, descriptor: u64) {
    unsafe {
        asm!("dsb ishst", options(nostack, preserves_flags));
        ptr::write_volatile(entry, descriptor);
        invalidate_all_translations();
    }
}

unsafe fn invalidate_all_translations() {
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
            options(nostack, preserves_flags)
        );
    }
    TLB_INVALIDATION_COUNT.fetch_add(1, Ordering::Relaxed);
}

unsafe fn invalidate_asid(asid: u8) {
    let operand = u64::from(asid) << TTBR_ASID_SHIFT;
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi aside1is, {operand}",
            "dsb ish",
            "isb",
            operand = in(reg) operand,
            options(nostack, preserves_flags)
        );
    }
    ADDRESS_SPACE_TLB_INVALIDATION_COUNT.fetch_add(1, Ordering::Relaxed);
}

unsafe fn initialize_user_hierarchy(
    root_owner: &OwnedFrame,
    level_1_owner: &OwnedFrame,
    user_level_2_owner: &OwnedFrame,
    user_level_3_owner: &OwnedFrame,
    mappings: &OwnedUserMappings,
) -> bool {
    let boot_level_1 = unsafe { &*LEVEL_1.0.get() };
    let mmio = unsafe { ptr::read_volatile(boot_level_1.0.as_ptr()) };
    let ram = unsafe { ptr::read_volatile(boot_level_1.0.as_ptr().add(1)) };
    if descriptor_type(mmio) != DESCRIPTOR_BLOCK
        || descriptor_type(ram) != DESCRIPTOR_TABLE_OR_PAGE
        || descriptor_address(ram) != LEVEL_2_RAM.0.get() as usize
    {
        return false;
    }
    for (offset, shared) in DYNAMIC_LEVEL_2.iter().enumerate() {
        let descriptor = unsafe {
            ptr::read_volatile(boot_level_1.0.as_ptr().add(DYNAMIC_LEVEL_1_FIRST + offset))
        };
        if descriptor_type(descriptor) != DESCRIPTOR_TABLE_OR_PAGE
            || descriptor_address(descriptor) != shared.0.get() as usize
        {
            return false;
        }
    }
    if descriptor_type(unsafe {
        ptr::read_volatile(boot_level_1.0.as_ptr().add(INIT_USER_LEVEL_1_INDEX))
    }) != 0
    {
        return false;
    }

    let root = unsafe { zero_table(root_owner) };
    let level_1 = unsafe { zero_table(level_1_owner) };
    let user_level_2 = unsafe { zero_table(user_level_2_owner) };
    let user_level_3 = unsafe { zero_table(user_level_3_owner) };
    unsafe {
        ptr::write_volatile(
            (*root).0.as_mut_ptr(),
            table_descriptor_address(level_1_owner.start_address()),
        );
        ptr::write_volatile((*level_1).0.as_mut_ptr(), mmio);
        ptr::write_volatile((*level_1).0.as_mut_ptr().add(1), ram);
        for offset in 0..DYNAMIC_LEVEL_1_COUNT {
            let descriptor =
                ptr::read_volatile(boot_level_1.0.as_ptr().add(DYNAMIC_LEVEL_1_FIRST + offset));
            ptr::write_volatile(
                (*level_1)
                    .0
                    .as_mut_ptr()
                    .add(DYNAMIC_LEVEL_1_FIRST + offset),
                descriptor,
            );
        }
        ptr::write_volatile(
            (*level_1).0.as_mut_ptr().add(INIT_USER_LEVEL_1_INDEX),
            table_descriptor_address(user_level_2_owner.start_address()),
        );
        ptr::write_volatile(
            (*user_level_2).0.as_mut_ptr(),
            table_descriptor_address(user_level_3_owner.start_address()),
        );
        for page in &mappings.pages {
            let leaf = user_leaf_descriptor(page)
                .unwrap_or_else(|_| panic!("validated user page lost its descriptor"));
            ptr::write_volatile(
                (*user_level_3)
                    .0
                    .as_mut_ptr()
                    .add((page.virtual_address >> 12) & 0x1ff),
                leaf,
            );
        }
        asm!("dsb ishst", options(nostack, preserves_flags));
    }
    true
}

unsafe fn verify_user_leaves(
    user_level_3_owner: &OwnedFrame,
    mappings: &OwnedUserMappings,
    graphics_mappings: &[Option<Box<UserGraphicsBufferMapping>>],
) -> bool {
    let user_level_3 = user_level_3_owner.start_address() as *const PageTable;
    for index in 0..ENTRY_COUNT {
        let virtual_address = INIT_USER_WINDOW_START + index * PAGE_SIZE;
        let owned = mappings
            .pages
            .iter()
            .find(|page| page.virtual_address == virtual_address);
        let borrowed = graphics_mappings
            .iter()
            .flatten()
            .find(|mapping| mapping.contains_page(virtual_address));
        if owned.is_some() && borrowed.is_some() {
            return false;
        }
        let expected = match (owned, borrowed) {
            (Some(page), None) => user_leaf_descriptor(page)
                .unwrap_or_else(|_| panic!("validated user page lost its descriptor")),
            (None, Some(mapping)) => {
                let offset = virtual_address - mapping.address;
                graphics_mapping_leaf_descriptor(mapping.backing_address + offset, mapping.access)
                    .unwrap_or_else(|_| panic!("validated graphics mapping lost its descriptor"))
            }
            (None, None) => 0,
            (Some(_), Some(_)) => unreachable!(),
        };
        if unsafe { ptr::read_volatile((*user_level_3).0.as_ptr().add(index)) } != expected {
            return false;
        }
    }
    graphics_mappings.iter().enumerate().all(|(slot, mapping)| {
        mapping.as_ref().is_none_or(|mapping| {
            graphics_mapping_address(slot) == Some(mapping.address)
                && usize::from(mapping.buffer.slot()) == slot
                && valid_graphics_backing(mapping.backing_address)
                && match mapping.role {
                    GraphicsMappingRole::Producer => true,
                    GraphicsMappingRole::Consumer => {
                        mapping.access == GraphicsMappingAccess::ReadOnly
                    }
                }
        })
    })
}

unsafe fn clear_user_leaves(
    user_level_3_owner: &OwnedFrame,
    mappings: &OwnedUserMappings,
    graphics_mappings: &[Option<Box<UserGraphicsBufferMapping>>],
) {
    let user_level_3 = user_level_3_owner.start_address() as *mut PageTable;
    for page in &mappings.pages {
        unsafe {
            ptr::write_volatile(
                (*user_level_3)
                    .0
                    .as_mut_ptr()
                    .add((page.virtual_address >> 12) & 0x1ff),
                0,
            );
        }
    }
    for mapping in graphics_mappings.iter().flatten() {
        unsafe { clear_graphics_mapping_leaves(user_level_3_owner, mapping.address) };
    }
    unsafe { asm!("dsb ishst", options(nostack, preserves_flags)) };
}

unsafe fn walk_page_descriptor(
    root_address: usize,
    virtual_address: usize,
) -> Result<Option<u64>, UnmapError> {
    if root_address & (PAGE_SIZE - 1) != 0
        || root_address >= (1_usize << 48)
        || virtual_address >= (1_usize << 48)
        || !virtual_address.is_multiple_of(PAGE_SIZE)
    {
        return Err(UnmapError::InvalidVirtualAddress);
    }
    let indices = [
        (virtual_address >> 39) & 0x1ff,
        (virtual_address >> 30) & 0x1ff,
        (virtual_address >> 21) & 0x1ff,
        (virtual_address >> 12) & 0x1ff,
    ];
    let mut table = root_address as *const PageTable;
    for (level, index) in indices.into_iter().enumerate() {
        let descriptor = unsafe { ptr::read_volatile((*table).0.as_ptr().add(index)) };
        match descriptor_type(descriptor) {
            0 => return Ok(None),
            DESCRIPTOR_BLOCK => return Err(UnmapError::BlockConflict),
            DESCRIPTOR_TABLE_OR_PAGE if level == 3 => return Ok(Some(descriptor)),
            DESCRIPTOR_TABLE_OR_PAGE => {
                table = descriptor_address(descriptor) as *const PageTable;
            }
            _ => unreachable!(),
        }
    }
    unreachable!()
}

unsafe fn table_is_empty(table: *const PageTable) -> bool {
    (0..ENTRY_COUNT).all(|index| unsafe { ptr::read_volatile((*table).0.as_ptr().add(index)) == 0 })
}

fn reclaim_detached_l3(address: usize) {
    memory::deallocate_frame(unsafe { OwnedFrame::from_allocated_address_unchecked(address) })
        .unwrap_or_else(|_| panic!("failed to reclaim a detached L3 page-table frame"));
    TABLE_FREE_COUNT.fetch_add(1, Ordering::Relaxed);
}

fn release_unpublished_table(frame: OwnedFrame) {
    memory::deallocate_frame(frame)
        .unwrap_or_else(|_| panic!("failed to release an unpublished page-table frame"));
}

fn release_destroyed_address_space_frame(frame: OwnedFrame, role: &str) {
    memory::deallocate_frame(frame)
        .unwrap_or_else(|_| panic!("failed to release destroyed address-space {role} frame"));
}

fn release_user_asid(asid: u8) {
    if asid == KERNEL_ASID {
        panic!("attempted to release reserved kernel ASID 0");
    }
    let word = usize::from(asid) / u64::BITS as usize;
    let mask = 1_u64 << (usize::from(asid) % u64::BITS as usize);
    let previous = USER_ASID_POOL[word].fetch_and(!mask, Ordering::AcqRel);
    if previous & mask == 0 {
        panic!("attempted to release an unowned user ASID");
    }
}

fn record_graphics_mapping_removals(count: u64) {
    if count == 0 {
        return;
    }
    LIVE_USER_GRAPHICS_MAPPING_COUNT
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |live| {
            live.checked_sub(count)
        })
        .unwrap_or_else(|_| panic!("live graphics mapping count underflowed"));
    USER_GRAPHICS_UNMAP_COUNT.fetch_add(count, Ordering::Relaxed);
}

fn try_box_address_space_metadata(layout: UserLayout) -> Option<Box<UserAddressSpaceMetadata>> {
    let pointer = ptr::NonNull::new(
        unsafe { alloc(Layout::new::<UserAddressSpaceMetadata>()) }
            .cast::<UserAddressSpaceMetadata>(),
    )?;
    unsafe {
        let raw = pointer.as_ptr();
        ptr::addr_of_mut!((*raw).layout).write(layout);
        ptr::addr_of_mut!((*raw).graphics_mappings).write(core::array::from_fn(|_| None));
        Some(Box::from_raw(pointer.as_ptr()))
    }
}

fn try_box_graphics_mapping(
    mapping: UserGraphicsBufferMapping,
) -> Result<Box<UserGraphicsBufferMapping>, UserGraphicsBufferMapping> {
    let Some(pointer) = ptr::NonNull::new(
        unsafe { alloc(Layout::new::<UserGraphicsBufferMapping>()) }
            .cast::<UserGraphicsBufferMapping>(),
    ) else {
        return Err(mapping);
    };
    unsafe {
        pointer.as_ptr().write(mapping);
        Ok(Box::from_raw(pointer.as_ptr()))
    }
}

fn validate_user_mappings(mappings: &OwnedUserMappings) -> Result<(), AddressSpaceError> {
    if mappings.expected_pages == 0
        || mappings.expected_pages > MAX_USER_MAPPED_PAGES
        || mappings.pages.len() != mappings.expected_pages
    {
        return Err(AddressSpaceError::TooManyMappings);
    }

    let layout = &mappings.metadata.layout;
    if layout.vma_count == 0
        || layout.vma_count > MAX_USER_VMAS
        || layout.guard_count != MAX_USER_GUARDS
        || !layout.entry.is_multiple_of(4)
        || layout.stack_start >= layout.stack_end
        || !layout.stack_start.is_multiple_of(PAGE_SIZE)
        || !layout.stack_end.is_multiple_of(PAGE_SIZE)
    {
        return Err(AddressSpaceError::InvalidLayout);
    }
    if layout.vmas[layout.vma_count..].iter().any(Option::is_some)
        || layout.guards[layout.guard_count..]
            .iter()
            .any(Option::is_some)
    {
        return Err(AddressSpaceError::InvalidLayout);
    }
    let stack_bytes = layout
        .stack_end
        .checked_sub(layout.stack_start)
        .ok_or(AddressSpaceError::InvalidLayout)?;
    let stack_pages = stack_bytes / PAGE_SIZE;
    if stack_pages == 0 || stack_pages > USER_STACK_PAGE_LIMIT {
        return Err(AddressSpaceError::InvalidLayout);
    }
    let expected_guard_low = layout
        .stack_start
        .checked_sub(PAGE_SIZE)
        .ok_or(AddressSpaceError::InvalidLayout)?;
    let expected_guard_high = layout.stack_end;
    if !valid_init_user_page(expected_guard_low) || !valid_init_user_page(expected_guard_high) {
        return Err(AddressSpaceError::InvalidLayout);
    }

    let mut previous_end = None;
    let mut vma_page_total = 0_usize;
    let mut stack_vmas = 0_usize;
    let mut entry_is_executable = false;
    for index in 0..layout.vma_count {
        let vma = layout.vmas[index].ok_or(AddressSpaceError::InvalidLayout)?;
        if !valid_user_vma_range(vma.start, vma.end)
            || previous_end.is_some_and(|end| vma.start < end)
        {
            return Err(AddressSpaceError::InvalidLayout);
        }
        previous_end = Some(vma.end);
        if vma.kind == UserVmaKind::Stack {
            stack_vmas += 1;
            if vma.start != layout.stack_start
                || vma.end != layout.stack_end
                || vma.access != UserPageAccess::ReadWrite
            {
                return Err(AddressSpaceError::InvalidLayout);
            }
        }
        if vma.access == UserPageAccess::ReadExecute
            && layout.entry >= vma.start
            && layout.entry < vma.end
        {
            entry_is_executable = true;
        }
        let expected = (vma.end - vma.start) / PAGE_SIZE;
        let actual = mappings
            .pages
            .iter()
            .filter(|page| page.virtual_address >= vma.start && page.virtual_address < vma.end)
            .count();
        if actual != expected {
            return Err(AddressSpaceError::InvalidLayout);
        }
        vma_page_total = vma_page_total
            .checked_add(expected)
            .ok_or(AddressSpaceError::TooManyMappings)?;
    }
    if stack_vmas != 1 || !entry_is_executable || vma_page_total != mappings.pages.len() {
        return Err(AddressSpaceError::InvalidLayout);
    }

    for (index, page) in mappings.pages.iter().enumerate() {
        if !valid_init_user_page(page.virtual_address)
            || !same_user_l3_window(page.virtual_address)
            || !valid_owned_normal_frame(&page.frame)
            || mappings.pages[..index].iter().any(|previous| {
                previous.virtual_address >= page.virtual_address
                    || previous.frame.start_address() == page.frame.start_address()
            })
            || user_leaf_descriptor(page).is_err()
        {
            return Err(
                if !valid_owned_normal_frame(&page.frame)
                    || mappings.pages[..index].iter().any(|previous| {
                        previous.frame.start_address() == page.frame.start_address()
                    })
                {
                    AddressSpaceError::InvalidPhysicalFrame
                } else {
                    AddressSpaceError::InvalidVirtualAddress
                },
            );
        }
        let mut matching_vma = None;
        for vma in layout.vmas[..layout.vma_count].iter().flatten() {
            if page.virtual_address >= vma.start && page.virtual_address < vma.end {
                if matching_vma.is_some() {
                    return Err(AddressSpaceError::InvalidLayout);
                }
                matching_vma = Some(*vma);
            }
        }
        if matching_vma.is_none_or(|vma| vma.access != page.access) {
            return Err(AddressSpaceError::InvalidLayout);
        }
    }

    let mut saw_low_guard = false;
    let mut saw_high_guard = false;
    for index in 0..layout.guard_count {
        let guard = layout.guards[index].ok_or(AddressSpaceError::InvalidLayout)?;
        if !valid_init_user_page(guard)
            || !same_user_l3_window(guard)
            || layout.guards[..index].contains(&Some(guard))
            || mappings
                .pages
                .iter()
                .any(|page| page.virtual_address == guard)
            || layout.vmas[..layout.vma_count]
                .iter()
                .flatten()
                .any(|vma| guard >= vma.start && guard < vma.end)
        {
            return Err(AddressSpaceError::InvalidLayout);
        }
        saw_low_guard |= guard == expected_guard_low;
        saw_high_guard |= guard == expected_guard_high;
    }
    if !saw_low_guard || !saw_high_guard {
        return Err(AddressSpaceError::InvalidLayout);
    }
    Ok(())
}

fn user_leaf_descriptor(page: &OwnedUserPage) -> Result<u64, AddressSpaceError> {
    let descriptor = match page.access {
        UserPageAccess::ReadExecute => user_ro_x_page_descriptor(page.frame.start_address()),
        UserPageAccess::ReadOnly => user_ro_nx_page_descriptor(page.frame.start_address()),
        UserPageAccess::ReadWrite => user_rw_nx_page_descriptor(page.frame.start_address()),
    };
    descriptor.map_err(|_| AddressSpaceError::InvalidPhysicalFrame)
}

fn graphics_mapping_leaf_descriptor(
    physical_address: usize,
    access: GraphicsMappingAccess,
) -> Result<u64, AddressSpaceError> {
    let descriptor = match access {
        GraphicsMappingAccess::ReadOnly => user_ro_nx_page_descriptor(physical_address),
        GraphicsMappingAccess::ReadWrite => user_rw_nx_page_descriptor(physical_address),
    };
    descriptor.map_err(|_| AddressSpaceError::InvalidPhysicalFrame)
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn graphics_mapping_address(slot: usize) -> Option<usize> {
    if slot >= USER_GRAPHICS_MAPPING_CAPACITY {
        return None;
    }
    USER_GRAPHICS_MAP_ADDRESS.checked_add(slot.checked_mul(USER_GRAPHICS_MAP_STRIDE)?)
}

#[cfg(feature = "mobile-ui-runtime")]
fn graphics_mapping_address(_slot: usize) -> Option<usize> {
    None
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn graphics_mapping_slot_for_address(address: usize) -> Option<usize> {
    (0..USER_GRAPHICS_MAPPING_CAPACITY)
        .find(|slot| graphics_mapping_address(*slot) == Some(address))
}

#[cfg(feature = "mobile-ui-runtime")]
fn graphics_mapping_slot_for_address(_address: usize) -> Option<usize> {
    None
}

fn valid_graphics_backing(address: usize) -> bool {
    address.is_multiple_of(PAGE_SIZE)
        && address >= EARLY_RAM_START
        && address
            .checked_add(bndr_abi::GRAPHICS_BUFFER_BACKING_BYTES)
            .is_some_and(|end| end <= EARLY_RAM_END)
}

unsafe fn graphics_mapping_range_is_unmapped(
    user_level_3_owner: &OwnedFrame,
    address: usize,
) -> bool {
    let level_3 = user_level_3_owner.start_address() as *const PageTable;
    (0..USER_GRAPHICS_MAPPING_PAGES).all(|page| {
        let virtual_address = address + page * PAGE_SIZE;
        unsafe {
            ptr::read_volatile(
                (*level_3)
                    .0
                    .as_ptr()
                    .add((virtual_address >> 12) & (ENTRY_COUNT - 1)),
            ) == 0
        }
    })
}

unsafe fn graphics_mapping_leaves_match(
    user_level_3_owner: &OwnedFrame,
    address: usize,
    backing_address: usize,
    access: GraphicsMappingAccess,
) -> bool {
    let level_3 = user_level_3_owner.start_address() as *const PageTable;
    (0..USER_GRAPHICS_MAPPING_PAGES).all(|page| {
        let virtual_address = address + page * PAGE_SIZE;
        let physical_address = backing_address + page * PAGE_SIZE;
        let expected = graphics_mapping_leaf_descriptor(physical_address, access)
            .unwrap_or_else(|_| panic!("validated graphics backing lost its leaf descriptor"));
        unsafe {
            ptr::read_volatile(
                (*level_3)
                    .0
                    .as_ptr()
                    .add((virtual_address >> 12) & (ENTRY_COUNT - 1)),
            ) == expected
        }
    })
}

unsafe fn write_graphics_mapping_leaves(
    user_level_3_owner: &OwnedFrame,
    address: usize,
    backing_address: usize,
    access: GraphicsMappingAccess,
) {
    let level_3 = user_level_3_owner.start_address() as *mut PageTable;
    for page in 0..USER_GRAPHICS_MAPPING_PAGES {
        let virtual_address = address + page * PAGE_SIZE;
        let physical_address = backing_address + page * PAGE_SIZE;
        let descriptor = graphics_mapping_leaf_descriptor(physical_address, access)
            .unwrap_or_else(|_| panic!("validated graphics backing lost its leaf descriptor"));
        unsafe {
            ptr::write_volatile(
                (*level_3)
                    .0
                    .as_mut_ptr()
                    .add((virtual_address >> 12) & (ENTRY_COUNT - 1)),
                descriptor,
            );
        }
    }
    unsafe { asm!("dsb ishst", options(nostack, preserves_flags)) };
}

unsafe fn clear_graphics_mapping_leaves(user_level_3_owner: &OwnedFrame, address: usize) {
    let level_3 = user_level_3_owner.start_address() as *mut PageTable;
    for page in 0..USER_GRAPHICS_MAPPING_PAGES {
        let virtual_address = address + page * PAGE_SIZE;
        unsafe {
            ptr::write_volatile(
                (*level_3)
                    .0
                    .as_mut_ptr()
                    .add((virtual_address >> 12) & (ENTRY_COUNT - 1)),
                0,
            );
        }
    }
    unsafe { asm!("dsb ishst", options(nostack, preserves_flags)) };
}

const fn valid_user_vma_range(start: usize, end: usize) -> bool {
    start < end
        && start >= INIT_USER_WINDOW_START
        && end <= INIT_USER_WINDOW_END
        && start.is_multiple_of(PAGE_SIZE)
        && end.is_multiple_of(PAGE_SIZE)
        && same_user_l3_window(start)
        && same_user_l3_window(end - PAGE_SIZE)
}

const fn same_user_l3_window(address: usize) -> bool {
    (address >> 39) & 0x1ff == (INIT_USER_WINDOW_START >> 39) & 0x1ff
        && (address >> 30) & 0x1ff == (INIT_USER_WINDOW_START >> 30) & 0x1ff
        && (address >> 21) & 0x1ff == (INIT_USER_WINDOW_START >> 21) & 0x1ff
}

const fn valid_owned_normal_frame(frame: &OwnedFrame) -> bool {
    let address = frame.start_address();
    address.is_multiple_of(PAGE_SIZE) && address >= EARLY_RAM_START && address < EARLY_RAM_END
}

const fn valid_init_user_page(address: usize) -> bool {
    address >= INIT_USER_WINDOW_START
        && address < INIT_USER_WINDOW_END
        && address.is_multiple_of(PAGE_SIZE)
}

const fn translation_context(root_address: usize, asid: u8) -> Option<TranslationContext> {
    if root_address & (PAGE_SIZE - 1) != 0 || root_address >= (1_usize << 48) {
        return None;
    }
    Some(TranslationContext(
        root_address as u64 | (asid as u64) << TTBR_ASID_SHIFT,
    ))
}

const fn valid_dynamic_page(address: usize) -> bool {
    address >= DYNAMIC_MAP_START && address < DYNAMIC_MAP_END && address & (PAGE_SIZE - 1) == 0
}

fn table_from_descriptor(descriptor: u64) -> *mut PageTable {
    descriptor_address(descriptor) as *mut PageTable
}

const fn table_descriptor_address(address: usize) -> u64 {
    address as u64 | DESCRIPTOR_TABLE_OR_PAGE
}

fn table_descriptor(table: &PageTable) -> u64 {
    table.0.as_ptr() as u64 | DESCRIPTOR_TABLE_OR_PAGE
}

fn symbol_range(start: *const u8, end: *const u8) -> core::ops::Range<usize> {
    start as usize..end as usize
}
