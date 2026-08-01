use core::{ops::Range, ptr};

const FDT_MAGIC: u32 = 0xd00d_feed;
const FDT_BEGIN_NODE: u32 = 1;
const FDT_END_NODE: u32 = 2;
const FDT_PROP: u32 = 3;
const FDT_NOP: u32 = 4;
const FDT_END: u32 = 9;
const HEADER_SIZE: usize = 40;
const MAX_FDT_SIZE: usize = 16 * 1024 * 1024;
pub const MAX_RESERVED_REGIONS: usize = 64;
pub const MAX_VIRTIO_MMIO_REGIONS: usize = 32;
const MAX_GICV2_CONTROLLERS: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryRegion {
    pub start: usize,
    pub size: usize,
}

impl MemoryRegion {
    pub fn end(self) -> usize {
        self.start.saturating_add(self.size)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReservedRegions {
    regions: [MemoryRegion; MAX_RESERVED_REGIONS],
    len: usize,
}

impl ReservedRegions {
    pub const fn new() -> Self {
        Self {
            regions: [MemoryRegion { start: 0, size: 0 }; MAX_RESERVED_REGIONS],
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[MemoryRegion] {
        &self.regions[..self.len]
    }

    pub fn iter(&self) -> core::slice::Iter<'_, MemoryRegion> {
        self.as_slice().iter()
    }

    pub fn push(&mut self, region: MemoryRegion) -> Result<(), FdtError> {
        if region.size == 0 {
            return Ok(());
        }
        region
            .start
            .checked_add(region.size)
            .ok_or(FdtError::AddressTooLarge)?;
        if self.len == MAX_RESERVED_REGIONS {
            return Err(FdtError::TooManyReservedRegions);
        }
        self.regions[self.len] = region;
        self.len += 1;
        Ok(())
    }

    pub fn push_range(&mut self, range: Range<usize>) -> Result<(), FdtError> {
        let size = range
            .end
            .checked_sub(range.start)
            .ok_or(FdtError::InvalidHeader)?;
        self.push(MemoryRegion {
            start: range.start,
            size,
        })
    }
}

impl Default for ReservedRegions {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> IntoIterator for &'a ReservedRegions {
    type Item = &'a MemoryRegion;
    type IntoIter = core::slice::Iter<'a, MemoryRegion>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GicInterruptTrigger {
    EdgeRising,
    LevelHigh,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GicInterrupt {
    /// Architectural GIC interrupt ID (SPI number plus the 32-SPI base).
    pub id: u16,
    pub trigger: GicInterruptTrigger,
    /// Resolved `interrupt-parent` phandle.
    pub parent_phandle: u32,
    /// Original GIC three-cell interrupt specifier retained as boot evidence.
    pub raw: [u32; 3],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VirtioMmioRegion {
    pub start: usize,
    pub size: usize,
    pub dma_coherent: bool,
    pub interrupts: Option<GicInterrupt>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FwCfgMmioRegion {
    pub start: usize,
    pub size: usize,
    pub dma_coherent: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PsciMethod {
    Hvc = 1,
    Smc = 2,
}

impl PsciMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hvc => "hvc",
            Self::Smc => "smc",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PsciCompatibleVersion {
    V0_2 = 1,
    V1_0 = 2,
}

impl PsciCompatibleVersion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V0_2 => "arm,psci-0.2",
            Self::V1_0 => "arm,psci-1.0",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PsciInfo {
    pub method: PsciMethod,
    pub compatible: PsciCompatibleVersion,
}

impl FwCfgMmioRegion {
    pub fn end(self) -> usize {
        self.start.saturating_add(self.size)
    }
}

impl VirtioMmioRegion {
    pub fn end(self) -> usize {
        self.start.saturating_add(self.size)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VirtioMmioRegions {
    regions: [VirtioMmioRegion; MAX_VIRTIO_MMIO_REGIONS],
    len: usize,
}

impl VirtioMmioRegions {
    const EMPTY_REGION: VirtioMmioRegion = VirtioMmioRegion {
        start: 0,
        size: 0,
        dma_coherent: false,
        interrupts: None,
    };

    pub const fn new() -> Self {
        Self {
            regions: [Self::EMPTY_REGION; MAX_VIRTIO_MMIO_REGIONS],
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[VirtioMmioRegion] {
        &self.regions[..self.len]
    }

    pub fn iter(&self) -> core::slice::Iter<'_, VirtioMmioRegion> {
        self.as_slice().iter()
    }

    pub fn push(&mut self, region: VirtioMmioRegion) -> Result<(), FdtError> {
        region
            .start
            .checked_add(region.size)
            .ok_or(FdtError::AddressTooLarge)?;
        if !region.start.is_multiple_of(4) || !region.size.is_multiple_of(4) || region.size < 0x200
        {
            return Err(FdtError::InvalidVirtioMmioReg);
        }
        if self.len == MAX_VIRTIO_MMIO_REGIONS {
            return Err(FdtError::TooManyVirtioMmioRegions);
        }
        self.regions[self.len] = region;
        self.len += 1;
        Ok(())
    }
}

impl Default for VirtioMmioRegions {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> IntoIterator for &'a VirtioMmioRegions {
    type Item = &'a VirtioMmioRegion;
    type IntoIter = core::slice::Iter<'a, VirtioMmioRegion>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FdtError {
    NullPointer,
    BadMagic,
    UnsupportedVersion,
    InvalidHeader,
    Truncated,
    InvalidToken,
    InvalidString,
    UnsupportedCells,
    MissingMemory,
    AddressTooLarge,
    TooManyReservedRegions,
    TooManyVirtioMmioRegions,
    MissingVirtioMmioReg,
    InvalidVirtioMmioReg,
    MultipleFwCfgMmioRegions,
    MissingFwCfgMmioReg,
    InvalidFwCfgMmioReg,
    InvalidVirtioMmioInterrupts,
    InvalidInterruptParent,
    InvalidGicV2InterruptController,
    TooManyGicV2InterruptControllers,
    DynamicReservedMemoryUnsupported,
    MissingPsciNode,
    MultiplePsciNodes,
    InvalidPsciCompatible,
    InvalidPsciStatus,
    MissingPsciMethod,
    InvalidPsciMethod,
    DuplicatePsciProperty,
}

impl FdtError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NullPointer => "null pointer",
            Self::BadMagic => "bad magic",
            Self::UnsupportedVersion => "unsupported version",
            Self::InvalidHeader => "invalid header",
            Self::Truncated => "truncated structure",
            Self::InvalidToken => "invalid structure token",
            Self::InvalidString => "invalid string table entry",
            Self::UnsupportedCells => "unsupported address/size cell count",
            Self::MissingMemory => "no memory node",
            Self::AddressTooLarge => "address does not fit usize",
            Self::TooManyReservedRegions => "too many reserved memory regions",
            Self::TooManyVirtioMmioRegions => "too many virtio-mmio regions",
            Self::MissingVirtioMmioReg => "virtio-mmio node has no reg property",
            Self::InvalidVirtioMmioReg => "invalid virtio-mmio reg property",
            Self::MultipleFwCfgMmioRegions => "multiple enabled fw_cfg MMIO regions",
            Self::MissingFwCfgMmioReg => "fw_cfg MMIO node has no reg property",
            Self::InvalidFwCfgMmioReg => "invalid fw_cfg MMIO reg property",
            Self::InvalidVirtioMmioInterrupts => "invalid virtio-mmio interrupts property",
            Self::InvalidInterruptParent => "invalid interrupt-parent reference",
            Self::InvalidGicV2InterruptController => "invalid GICv2 interrupt controller",
            Self::TooManyGicV2InterruptControllers => "too many GICv2 interrupt controllers",
            Self::DynamicReservedMemoryUnsupported => {
                "dynamic /reserved-memory allocation is not implemented"
            }
            Self::MissingPsciNode => "missing direct-root /psci node",
            Self::MultiplePsciNodes => "multiple direct-root /psci nodes",
            Self::InvalidPsciCompatible => "invalid /psci compatible property",
            Self::InvalidPsciStatus => "disabled or invalid /psci status property",
            Self::MissingPsciMethod => "missing /psci method property",
            Self::InvalidPsciMethod => "invalid /psci method property",
            Self::DuplicatePsciProperty => "duplicate security-relevant /psci property",
        }
    }
}

pub struct Fdt {
    base: *const u8,
    total_size: usize,
    reservation_offset: usize,
    structure_offset: usize,
    structure_size: usize,
    strings_offset: usize,
    strings_size: usize,
    version: u32,
}

#[derive(Clone, Copy)]
struct PendingVirtioMmioNode {
    compatible: bool,
    disabled: bool,
    dma_coherent: bool,
    reg: Option<(usize, usize)>,
    duplicate_reg: bool,
    interrupts: Option<[u32; 3]>,
    invalid_interrupts: bool,
    interrupt_parent: Option<u32>,
    invalid_interrupt_parent: bool,
}

#[derive(Clone, Copy)]
struct PendingFwCfgMmioNode {
    compatible: bool,
    disabled: bool,
    dma_coherent: bool,
    reg: Option<(usize, usize)>,
    duplicate_reg: bool,
}

#[derive(Clone, Copy)]
struct PendingPsciNode {
    compatible_seen: bool,
    compatible_v0_2: bool,
    compatible_v1_0: bool,
    method_seen: bool,
    method: Option<PsciMethod>,
    status_seen: bool,
    status_enabled: bool,
    invalid: bool,
    duplicate_property: bool,
}

impl PendingPsciNode {
    const EMPTY: Self = Self {
        compatible_seen: false,
        compatible_v0_2: false,
        compatible_v1_0: false,
        method_seen: false,
        method: None,
        status_seen: false,
        status_enabled: true,
        invalid: false,
        duplicate_property: false,
    };
}

impl PendingFwCfgMmioNode {
    const EMPTY: Self = Self {
        compatible: false,
        disabled: false,
        dma_coherent: false,
        reg: None,
        duplicate_reg: false,
    };
}

impl PendingVirtioMmioNode {
    const EMPTY: Self = Self {
        compatible: false,
        disabled: false,
        dma_coherent: false,
        reg: None,
        duplicate_reg: false,
        interrupts: None,
        invalid_interrupts: false,
        interrupt_parent: None,
        invalid_interrupt_parent: false,
    };
}

#[derive(Clone, Copy)]
struct PendingGicV2Node {
    compatible: bool,
    disabled: bool,
    interrupt_controller: bool,
    interrupt_cells: Option<u32>,
    phandle: Option<u32>,
    linux_phandle: Option<u32>,
    invalid: bool,
}

impl PendingGicV2Node {
    const EMPTY: Self = Self {
        compatible: false,
        disabled: false,
        interrupt_controller: false,
        interrupt_cells: None,
        phandle: None,
        linux_phandle: None,
        invalid: false,
    };
}

#[derive(Clone, Copy)]
struct InterruptTopology {
    root_parent: Option<u32>,
    controllers: [u32; MAX_GICV2_CONTROLLERS],
    controller_count: usize,
}

impl InterruptTopology {
    const fn new() -> Self {
        Self {
            root_parent: None,
            controllers: [0; MAX_GICV2_CONTROLLERS],
            controller_count: 0,
        }
    }

    fn push_controller(&mut self, phandle: u32) -> Result<(), FdtError> {
        if self.controllers[..self.controller_count].contains(&phandle) {
            return Err(FdtError::InvalidGicV2InterruptController);
        }
        if self.controller_count == MAX_GICV2_CONTROLLERS {
            return Err(FdtError::TooManyGicV2InterruptControllers);
        }
        self.controllers[self.controller_count] = phandle;
        self.controller_count += 1;
        Ok(())
    }

    fn contains_controller(self, phandle: u32) -> bool {
        self.controllers[..self.controller_count].contains(&phandle)
    }
}

impl Fdt {
    /// Validates an FDT supplied by the boot loader.
    ///
    /// # Safety
    ///
    /// `address` must point to a readable FDT blob whose complete `total_size`
    /// bytes remain mapped for the lifetime of this value.
    pub unsafe fn from_ptr(address: usize) -> Result<Self, FdtError> {
        if address == 0 {
            return Err(FdtError::NullPointer);
        }

        let base = address as *const u8;
        let magic = unsafe { read_be_u32(base, 0) };
        if magic != FDT_MAGIC {
            return Err(FdtError::BadMagic);
        }

        let total_size = unsafe { read_be_u32(base, 4) } as usize;
        if !(HEADER_SIZE..=MAX_FDT_SIZE).contains(&total_size) {
            return Err(FdtError::InvalidHeader);
        }
        address
            .checked_add(total_size)
            .ok_or(FdtError::AddressTooLarge)?;

        let structure_offset = unsafe { read_be_u32(base, 8) } as usize;
        let strings_offset = unsafe { read_be_u32(base, 12) } as usize;
        let reservation_offset = unsafe { read_be_u32(base, 16) } as usize;
        let version = unsafe { read_be_u32(base, 20) };
        let last_compatible_version = unsafe { read_be_u32(base, 24) };
        let strings_size = unsafe { read_be_u32(base, 32) } as usize;
        let structure_size = unsafe { read_be_u32(base, 36) } as usize;

        if version < 17 || last_compatible_version > 17 {
            return Err(FdtError::UnsupportedVersion);
        }
        if !range_within(structure_offset, structure_size, total_size)
            || !range_within(strings_offset, strings_size, total_size)
            || reservation_offset < HEADER_SIZE
            || !reservation_offset.is_multiple_of(8)
            || !range_within(reservation_offset, 16, total_size)
        {
            return Err(FdtError::InvalidHeader);
        }

        Ok(Self {
            base,
            total_size,
            reservation_offset,
            structure_offset,
            structure_size,
            strings_offset,
            strings_size,
            version,
        })
    }

    pub const fn version(&self) -> u32 {
        self.version
    }

    pub fn range(&self) -> core::ops::Range<usize> {
        let start = self.base as usize;
        start..start + self.total_size
    }

    pub fn memory_region(&self) -> Result<MemoryRegion, FdtError> {
        let structure = unsafe { self.base.add(self.structure_offset) };
        let mut cursor = 0_usize;
        let mut depth = -1_i32;
        let mut root_address_cells = 2_u32;
        let mut root_size_cells = 1_u32;
        let mut memory_depth = None;
        let mut memory_region = None;
        let mut saw_end = false;

        while cursor < self.structure_size {
            let token = self.structure_u32(&mut cursor)?;
            match token {
                FDT_BEGIN_NODE => {
                    depth += 1;
                    let name = self.structure_cstr(&mut cursor)?;
                    if depth == 1 && is_memory_node_name(name) {
                        memory_depth = Some(depth);
                    }
                }
                FDT_END_NODE => {
                    if depth < 0 {
                        return Err(FdtError::InvalidToken);
                    }
                    if memory_depth == Some(depth) {
                        memory_depth = None;
                    }
                    depth -= 1;
                }
                FDT_PROP => {
                    let length = self.structure_u32(&mut cursor)? as usize;
                    let name_offset = self.structure_u32(&mut cursor)? as usize;
                    let value_start = cursor;
                    let value_end = value_start.checked_add(length).ok_or(FdtError::Truncated)?;
                    if value_end > self.structure_size {
                        return Err(FdtError::Truncated);
                    }
                    let name = self.string_at(name_offset)?;

                    if depth == 0 && bytes_equal(name, b"#address-cells") && length == 4 {
                        root_address_cells = unsafe { read_be_u32(structure, value_start) };
                    } else if depth == 0 && bytes_equal(name, b"#size-cells") && length == 4 {
                        root_size_cells = unsafe { read_be_u32(structure, value_start) };
                    } else if memory_depth == Some(depth) && bytes_equal(name, b"reg") {
                        let cell_count = root_address_cells
                            .checked_add(root_size_cells)
                            .ok_or(FdtError::UnsupportedCells)?
                            as usize;
                        if !(1..=2).contains(&root_address_cells)
                            || !(1..=2).contains(&root_size_cells)
                            || length < cell_count * 4
                        {
                            return Err(FdtError::UnsupportedCells);
                        }

                        let address = read_cells(
                            unsafe { structure.add(value_start) },
                            root_address_cells as usize,
                        );
                        let size = read_cells(
                            unsafe { structure.add(value_start + root_address_cells as usize * 4) },
                            root_size_cells as usize,
                        );
                        let start =
                            usize::try_from(address).map_err(|_| FdtError::AddressTooLarge)?;
                        let size = usize::try_from(size).map_err(|_| FdtError::AddressTooLarge)?;
                        if size == 0 || start.checked_add(size).is_none() {
                            return Err(FdtError::InvalidHeader);
                        }
                        if memory_region.is_none() {
                            memory_region = Some(MemoryRegion { start, size });
                        }
                    }

                    cursor = align_up_4(value_end).ok_or(FdtError::Truncated)?;
                }
                FDT_NOP => {}
                FDT_END => {
                    if depth != -1 {
                        return Err(FdtError::InvalidToken);
                    }
                    saw_end = true;
                    break;
                }
                _ => return Err(FdtError::InvalidToken),
            }
        }

        if !saw_end {
            return Err(FdtError::Truncated);
        }
        memory_region.ok_or(FdtError::MissingMemory)
    }

    /// Collects the root interrupt parent and enabled, direct-root GICv2
    /// controllers. This separate pass makes controller and property order
    /// irrelevant to virtio device discovery.
    fn interrupt_topology(&self) -> Result<InterruptTopology, FdtError> {
        let structure = unsafe { self.base.add(self.structure_offset) };
        let mut cursor = 0_usize;
        let mut depth = -1_i32;
        let mut pending = None;
        let mut topology = InterruptTopology::new();
        let mut saw_root_parent = false;
        let mut saw_end = false;

        while cursor < self.structure_size {
            let token = self.structure_u32(&mut cursor)?;
            match token {
                FDT_BEGIN_NODE => {
                    depth += 1;
                    let _name = self.structure_cstr(&mut cursor)?;
                    if depth == 1 {
                        pending = Some(PendingGicV2Node::EMPTY);
                    }
                }
                FDT_END_NODE => {
                    if depth < 0 {
                        return Err(FdtError::InvalidToken);
                    }
                    if depth == 1 {
                        let node = pending.take().ok_or(FdtError::InvalidToken)?;
                        if node.compatible && !node.disabled {
                            if node.invalid
                                || !node.interrupt_controller
                                || node.interrupt_cells != Some(3)
                            {
                                return Err(FdtError::InvalidGicV2InterruptController);
                            }
                            let phandle = match (node.phandle, node.linux_phandle) {
                                (Some(phandle), None) | (None, Some(phandle)) => phandle,
                                (Some(phandle), Some(linux_phandle))
                                    if phandle == linux_phandle =>
                                {
                                    phandle
                                }
                                _ => return Err(FdtError::InvalidGicV2InterruptController),
                            };
                            if phandle == 0 || phandle == u32::MAX {
                                return Err(FdtError::InvalidGicV2InterruptController);
                            }
                            topology.push_controller(phandle)?;
                        }
                    }
                    depth -= 1;
                }
                FDT_PROP => {
                    let length = self.structure_u32(&mut cursor)? as usize;
                    let name_offset = self.structure_u32(&mut cursor)? as usize;
                    let value_start = cursor;
                    let value_end = value_start.checked_add(length).ok_or(FdtError::Truncated)?;
                    if value_end > self.structure_size {
                        return Err(FdtError::Truncated);
                    }
                    let name = self.string_at(name_offset)?;

                    if depth == 0 && bytes_equal(name, b"interrupt-parent") {
                        if saw_root_parent || length != 4 {
                            return Err(FdtError::InvalidInterruptParent);
                        }
                        saw_root_parent = true;
                        let phandle = unsafe { read_be_u32(structure, value_start) };
                        if phandle == 0 || phandle == u32::MAX {
                            return Err(FdtError::InvalidInterruptParent);
                        }
                        topology.root_parent = Some(phandle);
                    } else if depth == 1 {
                        let node = pending.as_mut().ok_or(FdtError::InvalidToken)?;
                        if bytes_equal(name, b"compatible") {
                            let value = unsafe {
                                core::slice::from_raw_parts(structure.add(value_start), length)
                            };
                            node.compatible |= string_list_contains(value, b"arm,cortex-a15-gic")
                                || string_list_contains(value, b"arm,gic-400");
                        } else if bytes_equal(name, b"status") {
                            let value = unsafe {
                                core::slice::from_raw_parts(structure.add(value_start), length)
                            };
                            node.disabled = dt_string_equals(value, b"disabled");
                        } else if bytes_equal(name, b"interrupt-controller") {
                            if node.interrupt_controller || length != 0 {
                                node.invalid = true;
                            } else {
                                node.interrupt_controller = true;
                            }
                        } else if bytes_equal(name, b"#interrupt-cells") {
                            if node.interrupt_cells.is_some() || length != 4 {
                                node.invalid = true;
                            } else {
                                node.interrupt_cells =
                                    Some(unsafe { read_be_u32(structure, value_start) });
                            }
                        } else if bytes_equal(name, b"phandle") {
                            if node.phandle.is_some() || length != 4 {
                                node.invalid = true;
                            } else {
                                node.phandle = Some(unsafe { read_be_u32(structure, value_start) });
                            }
                        } else if bytes_equal(name, b"linux,phandle") {
                            if node.linux_phandle.is_some() || length != 4 {
                                node.invalid = true;
                            } else {
                                node.linux_phandle =
                                    Some(unsafe { read_be_u32(structure, value_start) });
                            }
                        }
                    }

                    cursor = align_up_4(value_end).ok_or(FdtError::Truncated)?;
                }
                FDT_NOP => {}
                FDT_END => {
                    if depth != -1 || pending.is_some() {
                        return Err(FdtError::InvalidToken);
                    }
                    saw_end = true;
                    break;
                }
                _ => return Err(FdtError::InvalidToken),
            }
        }

        if !saw_end {
            return Err(FdtError::Truncated);
        }
        Ok(topology)
    }

    /// Discovers enabled virtio-mmio transports that are direct children of
    /// the device-tree root. The result is fixed-capacity and allocation-free.
    pub fn virtio_mmio_regions(&self) -> Result<VirtioMmioRegions, FdtError> {
        let interrupt_topology = self.interrupt_topology()?;
        let structure = unsafe { self.base.add(self.structure_offset) };
        let mut cursor = 0_usize;
        let mut depth = -1_i32;
        let mut root_address_cells = Some(2_u32);
        let mut root_size_cells = Some(1_u32);
        let mut pending = None;
        let mut regions = VirtioMmioRegions::new();
        let mut saw_end = false;

        while cursor < self.structure_size {
            let token = self.structure_u32(&mut cursor)?;
            match token {
                FDT_BEGIN_NODE => {
                    depth += 1;
                    let _name = self.structure_cstr(&mut cursor)?;
                    if depth == 1 {
                        pending = Some(PendingVirtioMmioNode::EMPTY);
                    }
                }
                FDT_END_NODE => {
                    if depth < 0 {
                        return Err(FdtError::InvalidToken);
                    }
                    if depth == 1 {
                        let node = pending.take().ok_or(FdtError::InvalidToken)?;
                        if node.compatible && !node.disabled {
                            let address_cells =
                                root_address_cells.ok_or(FdtError::UnsupportedCells)?;
                            let size_cells = root_size_cells.ok_or(FdtError::UnsupportedCells)?;
                            let region = virtio_mmio_region_from_node(
                                structure,
                                node,
                                address_cells,
                                size_cells,
                                interrupt_topology,
                            )?;
                            regions.push(region)?;
                        }
                    }
                    depth -= 1;
                }
                FDT_PROP => {
                    let length = self.structure_u32(&mut cursor)? as usize;
                    let name_offset = self.structure_u32(&mut cursor)? as usize;
                    let value_start = cursor;
                    let value_end = value_start.checked_add(length).ok_or(FdtError::Truncated)?;
                    if value_end > self.structure_size {
                        return Err(FdtError::Truncated);
                    }
                    let name = self.string_at(name_offset)?;

                    if depth == 0 && bytes_equal(name, b"#address-cells") {
                        root_address_cells =
                            (length == 4).then(|| unsafe { read_be_u32(structure, value_start) });
                    } else if depth == 0 && bytes_equal(name, b"#size-cells") {
                        root_size_cells =
                            (length == 4).then(|| unsafe { read_be_u32(structure, value_start) });
                    } else if depth == 1 {
                        let node = pending.as_mut().ok_or(FdtError::InvalidToken)?;
                        if bytes_equal(name, b"compatible") {
                            let value = unsafe {
                                core::slice::from_raw_parts(structure.add(value_start), length)
                            };
                            node.compatible |= string_list_contains(value, b"virtio,mmio");
                        } else if bytes_equal(name, b"reg") {
                            if node.reg.replace((value_start, length)).is_some() {
                                node.duplicate_reg = true;
                            }
                        } else if bytes_equal(name, b"status") {
                            let value = unsafe {
                                core::slice::from_raw_parts(structure.add(value_start), length)
                            };
                            node.disabled = dt_string_equals(value, b"disabled");
                        } else if bytes_equal(name, b"dma-coherent") {
                            node.dma_coherent = true;
                        } else if bytes_equal(name, b"interrupts") {
                            if node.interrupts.is_some() || node.invalid_interrupts || length != 12
                            {
                                node.invalid_interrupts = true;
                            } else {
                                node.interrupts = Some([
                                    unsafe { read_be_u32(structure, value_start) },
                                    unsafe { read_be_u32(structure, value_start + 4) },
                                    unsafe { read_be_u32(structure, value_start + 8) },
                                ]);
                            }
                        } else if bytes_equal(name, b"interrupt-parent") {
                            if node.interrupt_parent.is_some()
                                || node.invalid_interrupt_parent
                                || length != 4
                            {
                                node.invalid_interrupt_parent = true;
                            } else {
                                let phandle = unsafe { read_be_u32(structure, value_start) };
                                if phandle == 0 || phandle == u32::MAX {
                                    node.invalid_interrupt_parent = true;
                                } else {
                                    node.interrupt_parent = Some(phandle);
                                }
                            }
                        }
                    }

                    cursor = align_up_4(value_end).ok_or(FdtError::Truncated)?;
                }
                FDT_NOP => {}
                FDT_END => {
                    if depth != -1 || pending.is_some() {
                        return Err(FdtError::InvalidToken);
                    }
                    saw_end = true;
                    break;
                }
                _ => return Err(FdtError::InvalidToken),
            }
        }

        if !saw_end {
            return Err(FdtError::Truncated);
        }
        Ok(regions)
    }

    /// Discovers QEMU's optional direct-root `qemu,fw-cfg-mmio` transport.
    ///
    /// QEMU `virt` exposes at most one such device.  Treating duplicates and
    /// malformed matching nodes as fatal avoids silently programming an
    /// attacker-chosen MMIO range from an ambiguous device tree.
    pub fn fw_cfg_mmio_region(&self) -> Result<Option<FwCfgMmioRegion>, FdtError> {
        let structure = unsafe { self.base.add(self.structure_offset) };
        let mut cursor = 0_usize;
        let mut depth = -1_i32;
        let mut root_address_cells = Some(2_u32);
        let mut root_size_cells = Some(1_u32);
        let mut pending = None;
        let mut found = None;
        let mut saw_end = false;

        while cursor < self.structure_size {
            let token = self.structure_u32(&mut cursor)?;
            match token {
                FDT_BEGIN_NODE => {
                    depth += 1;
                    let _name = self.structure_cstr(&mut cursor)?;
                    if depth == 1 {
                        pending = Some(PendingFwCfgMmioNode::EMPTY);
                    }
                }
                FDT_END_NODE => {
                    if depth < 0 {
                        return Err(FdtError::InvalidToken);
                    }
                    if depth == 1 {
                        let node = pending.take().ok_or(FdtError::InvalidToken)?;
                        if node.compatible && !node.disabled {
                            if found.is_some() {
                                return Err(FdtError::MultipleFwCfgMmioRegions);
                            }
                            found = Some(fw_cfg_mmio_region_from_node(
                                structure,
                                node,
                                root_address_cells.ok_or(FdtError::UnsupportedCells)?,
                                root_size_cells.ok_or(FdtError::UnsupportedCells)?,
                            )?);
                        }
                    }
                    depth -= 1;
                }
                FDT_PROP => {
                    let length = self.structure_u32(&mut cursor)? as usize;
                    let name_offset = self.structure_u32(&mut cursor)? as usize;
                    let value_start = cursor;
                    let value_end = value_start.checked_add(length).ok_or(FdtError::Truncated)?;
                    if value_end > self.structure_size {
                        return Err(FdtError::Truncated);
                    }
                    let name = self.string_at(name_offset)?;

                    if depth == 0 && bytes_equal(name, b"#address-cells") {
                        root_address_cells =
                            (length == 4).then(|| unsafe { read_be_u32(structure, value_start) });
                    } else if depth == 0 && bytes_equal(name, b"#size-cells") {
                        root_size_cells =
                            (length == 4).then(|| unsafe { read_be_u32(structure, value_start) });
                    } else if depth == 1 {
                        let node = pending.as_mut().ok_or(FdtError::InvalidToken)?;
                        if bytes_equal(name, b"compatible") {
                            let value = unsafe {
                                core::slice::from_raw_parts(structure.add(value_start), length)
                            };
                            node.compatible |= string_list_contains(value, b"qemu,fw-cfg-mmio");
                        } else if bytes_equal(name, b"reg") {
                            if node.reg.replace((value_start, length)).is_some() {
                                node.duplicate_reg = true;
                            }
                        } else if bytes_equal(name, b"status") {
                            let value = unsafe {
                                core::slice::from_raw_parts(structure.add(value_start), length)
                            };
                            node.disabled = dt_string_equals(value, b"disabled");
                        } else if bytes_equal(name, b"dma-coherent") {
                            node.dma_coherent = true;
                        }
                    }

                    cursor = align_up_4(value_end).ok_or(FdtError::Truncated)?;
                }
                FDT_NOP => {}
                FDT_END => {
                    if depth != -1 || pending.is_some() {
                        return Err(FdtError::InvalidToken);
                    }
                    saw_end = true;
                    break;
                }
                _ => return Err(FdtError::InvalidToken),
            }
        }

        if !saw_end {
            return Err(FdtError::Truncated);
        }
        Ok(found)
    }

    /// Discovers the one direct-root `/psci` firmware conduit used by the
    /// current QEMU `virt` profile.
    ///
    /// This is intentionally stricter than generic device-tree matching:
    /// legacy `arm,psci` nodes, nested nodes, duplicate security-relevant
    /// properties, disabled nodes, and any conduit other than exact `hvc` or
    /// `smc` are rejected. Runtime code must still issue `PSCI_VERSION` before
    /// trusting the returned transport.
    pub fn psci(&self) -> Result<PsciInfo, FdtError> {
        let structure = unsafe { self.base.add(self.structure_offset) };
        let mut cursor = 0_usize;
        let mut depth = -1_i32;
        let mut pending = None;
        let mut found = None;
        let mut saw_psci_node = false;
        let mut saw_end = false;

        while cursor < self.structure_size {
            let token = self.structure_u32(&mut cursor)?;
            match token {
                FDT_BEGIN_NODE => {
                    depth += 1;
                    let name = self.structure_cstr(&mut cursor)?;
                    if depth == 1 && bytes_equal(name, b"psci") {
                        if saw_psci_node {
                            return Err(FdtError::MultiplePsciNodes);
                        }
                        saw_psci_node = true;
                        pending = Some(PendingPsciNode::EMPTY);
                    }
                }
                FDT_END_NODE => {
                    if depth < 0 {
                        return Err(FdtError::InvalidToken);
                    }
                    if depth == 1
                        && let Some(node) = pending.take()
                    {
                        if node.duplicate_property {
                            return Err(FdtError::DuplicatePsciProperty);
                        }
                        if !node.compatible_seen
                            || node.invalid
                            || (!node.compatible_v1_0 && !node.compatible_v0_2)
                        {
                            return Err(FdtError::InvalidPsciCompatible);
                        }
                        if node.status_seen && !node.status_enabled {
                            return Err(FdtError::InvalidPsciStatus);
                        }
                        if !node.method_seen {
                            return Err(FdtError::MissingPsciMethod);
                        }
                        let method = node.method.ok_or(FdtError::InvalidPsciMethod)?;
                        found = Some(PsciInfo {
                            method,
                            compatible: if node.compatible_v1_0 {
                                PsciCompatibleVersion::V1_0
                            } else {
                                PsciCompatibleVersion::V0_2
                            },
                        });
                    }
                    depth -= 1;
                }
                FDT_PROP => {
                    let length = self.structure_u32(&mut cursor)? as usize;
                    let name_offset = self.structure_u32(&mut cursor)? as usize;
                    let value_start = cursor;
                    let value_end = value_start.checked_add(length).ok_or(FdtError::Truncated)?;
                    if value_end > self.structure_size {
                        return Err(FdtError::Truncated);
                    }
                    let name = self.string_at(name_offset)?;

                    if depth == 1
                        && let Some(node) = pending.as_mut()
                    {
                        let value = unsafe {
                            core::slice::from_raw_parts(structure.add(value_start), length)
                        };
                        if bytes_equal(name, b"compatible") {
                            if node.compatible_seen {
                                node.duplicate_property = true;
                            }
                            node.compatible_seen = true;
                            node.compatible_v1_0 = string_list_contains(value, b"arm,psci-1.0");
                            node.compatible_v0_2 = string_list_contains(value, b"arm,psci-0.2");
                        } else if bytes_equal(name, b"method") {
                            if node.method_seen {
                                node.duplicate_property = true;
                            }
                            node.method_seen = true;
                            node.method = if dt_string_equals(value, b"hvc") {
                                Some(PsciMethod::Hvc)
                            } else if dt_string_equals(value, b"smc") {
                                Some(PsciMethod::Smc)
                            } else {
                                None
                            };
                        } else if bytes_equal(name, b"status") {
                            if node.status_seen {
                                node.duplicate_property = true;
                            }
                            node.status_seen = true;
                            node.status_enabled =
                                dt_string_equals(value, b"okay") || dt_string_equals(value, b"ok");
                        }
                    }

                    cursor = align_up_4(value_end).ok_or(FdtError::Truncated)?;
                }
                FDT_NOP => {}
                FDT_END => {
                    if depth != -1 || pending.is_some() {
                        return Err(FdtError::InvalidToken);
                    }
                    saw_end = true;
                    break;
                }
                _ => return Err(FdtError::InvalidToken),
            }
        }

        if !saw_end {
            return Err(FdtError::Truncated);
        }
        if !saw_psci_node {
            return Err(FdtError::MissingPsciNode);
        }
        found.ok_or(FdtError::InvalidPsciCompatible)
    }

    /// Collects reservations declared by both the header reservation map and
    /// static child nodes below `/reserved-memory`.
    pub fn reserved_regions(&self) -> Result<ReservedRegions, FdtError> {
        let mut regions = ReservedRegions::new();
        self.header_reservations(&mut regions)?;
        self.reserved_memory_nodes(&mut regions)?;
        Ok(regions)
    }

    fn header_reservations(&self, regions: &mut ReservedRegions) -> Result<(), FdtError> {
        let mut cursor = self.reservation_offset;
        let reservation_limit = [self.structure_offset, self.strings_offset, self.total_size]
            .into_iter()
            .filter(|offset| *offset > self.reservation_offset)
            .min()
            .unwrap_or(self.total_size);

        loop {
            let end = cursor.checked_add(16).ok_or(FdtError::Truncated)?;
            if end > reservation_limit {
                return Err(FdtError::Truncated);
            }

            let address = unsafe { read_be_u64(self.base, cursor) };
            let size = unsafe { read_be_u64(self.base, cursor + 8) };
            cursor = end;
            if address == 0 && size == 0 {
                return Ok(());
            }

            regions.push(memory_region_from_u64(address, size)?)?;
        }
    }

    fn reserved_memory_nodes(&self, regions: &mut ReservedRegions) -> Result<(), FdtError> {
        let structure = unsafe { self.base.add(self.structure_offset) };
        let mut cursor = 0_usize;
        let mut depth = -1_i32;
        let mut root_address_cells = 2_u32;
        let mut root_size_cells = 1_u32;
        let mut reserved_memory_depth = None;
        let mut reserved_address_cells = root_address_cells;
        let mut reserved_size_cells = root_size_cells;
        let mut reserved_child_depth = None;
        let mut reserved_child_has_reg = false;
        let mut reserved_child_has_size = false;
        let mut saw_end = false;

        while cursor < self.structure_size {
            let token = self.structure_u32(&mut cursor)?;
            match token {
                FDT_BEGIN_NODE => {
                    depth += 1;
                    let name = self.structure_cstr(&mut cursor)?;
                    if depth == 1 && bytes_equal(name, b"reserved-memory") {
                        reserved_memory_depth = Some(depth);
                        reserved_address_cells = root_address_cells;
                        reserved_size_cells = root_size_cells;
                    } else if reserved_memory_depth.is_some_and(|parent| depth == parent + 1) {
                        reserved_child_depth = Some(depth);
                        reserved_child_has_reg = false;
                        reserved_child_has_size = false;
                    }
                }
                FDT_END_NODE => {
                    if depth < 0 {
                        return Err(FdtError::InvalidToken);
                    }
                    if reserved_child_depth == Some(depth) {
                        if reserved_child_has_size && !reserved_child_has_reg {
                            return Err(FdtError::DynamicReservedMemoryUnsupported);
                        }
                        reserved_child_depth = None;
                    }
                    if reserved_memory_depth == Some(depth) {
                        reserved_memory_depth = None;
                    }
                    depth -= 1;
                }
                FDT_PROP => {
                    let length = self.structure_u32(&mut cursor)? as usize;
                    let name_offset = self.structure_u32(&mut cursor)? as usize;
                    let value_start = cursor;
                    let value_end = value_start.checked_add(length).ok_or(FdtError::Truncated)?;
                    if value_end > self.structure_size {
                        return Err(FdtError::Truncated);
                    }
                    let name = self.string_at(name_offset)?;

                    if depth == 0 && bytes_equal(name, b"#address-cells") && length == 4 {
                        root_address_cells = unsafe { read_be_u32(structure, value_start) };
                    } else if depth == 0 && bytes_equal(name, b"#size-cells") && length == 4 {
                        root_size_cells = unsafe { read_be_u32(structure, value_start) };
                    } else if reserved_memory_depth == Some(depth)
                        && bytes_equal(name, b"#address-cells")
                        && length == 4
                    {
                        reserved_address_cells = unsafe { read_be_u32(structure, value_start) };
                    } else if reserved_memory_depth == Some(depth)
                        && bytes_equal(name, b"#size-cells")
                        && length == 4
                    {
                        reserved_size_cells = unsafe { read_be_u32(structure, value_start) };
                    } else if reserved_memory_depth.is_some_and(|parent| depth == parent + 1)
                        && bytes_equal(name, b"reg")
                    {
                        collect_reg_regions(
                            structure,
                            value_start,
                            length,
                            reserved_address_cells,
                            reserved_size_cells,
                            regions,
                        )?;
                        reserved_child_has_reg = true;
                    } else if reserved_memory_depth.is_some_and(|parent| depth == parent + 1)
                        && bytes_equal(name, b"size")
                    {
                        // Per DTSpec, `reg` takes precedence when both exist.
                        // A child with only `size` is rejected at END_NODE:
                        // dynamic address selection is not implemented yet.
                        reserved_child_has_size = true;
                    }

                    cursor = align_up_4(value_end).ok_or(FdtError::Truncated)?;
                }
                FDT_NOP => {}
                FDT_END => {
                    if depth != -1 {
                        return Err(FdtError::InvalidToken);
                    }
                    saw_end = true;
                    break;
                }
                _ => return Err(FdtError::InvalidToken),
            }
        }

        if !saw_end {
            return Err(FdtError::Truncated);
        }
        Ok(())
    }

    fn structure_u32(&self, cursor: &mut usize) -> Result<u32, FdtError> {
        let end = cursor.checked_add(4).ok_or(FdtError::Truncated)?;
        if end > self.structure_size {
            return Err(FdtError::Truncated);
        }
        let value = unsafe { read_be_u32(self.base.add(self.structure_offset), *cursor) };
        *cursor = end;
        Ok(value)
    }

    fn structure_cstr<'a>(&'a self, cursor: &mut usize) -> Result<&'a [u8], FdtError> {
        let start = *cursor;
        let structure = unsafe { self.base.add(self.structure_offset) };
        while *cursor < self.structure_size {
            let byte = unsafe { ptr::read(structure.add(*cursor)) };
            if byte == 0 {
                let value =
                    unsafe { core::slice::from_raw_parts(structure.add(start), *cursor - start) };
                *cursor = align_up_4(*cursor + 1).ok_or(FdtError::Truncated)?;
                if *cursor > self.structure_size {
                    return Err(FdtError::Truncated);
                }
                return Ok(value);
            }
            *cursor += 1;
        }
        Err(FdtError::Truncated)
    }

    fn string_at(&self, offset: usize) -> Result<&[u8], FdtError> {
        if offset >= self.strings_size {
            return Err(FdtError::InvalidString);
        }
        let strings = unsafe { self.base.add(self.strings_offset) };
        let mut length = 0;
        while offset + length < self.strings_size {
            if unsafe { ptr::read(strings.add(offset + length)) } == 0 {
                return Ok(unsafe { core::slice::from_raw_parts(strings.add(offset), length) });
            }
            length += 1;
        }
        Err(FdtError::InvalidString)
    }
}

fn range_within(offset: usize, size: usize, total: usize) -> bool {
    offset.checked_add(size).is_some_and(|end| end <= total)
}

fn align_up_4(value: usize) -> Option<usize> {
    value.checked_add(3).map(|value| value & !3)
}

fn starts_with(value: &[u8], prefix: &[u8]) -> bool {
    value.len() >= prefix.len() && bytes_equal(&value[..prefix.len()], prefix)
}

fn is_memory_node_name(value: &[u8]) -> bool {
    bytes_equal(value, b"memory") || starts_with(value, b"memory@")
}

fn bytes_equal(left: &[u8], right: &[u8]) -> bool {
    left == right
}

fn dt_string_equals(value: &[u8], expected: &[u8]) -> bool {
    value.len() == expected.len() + 1
        && bytes_equal(&value[..expected.len()], expected)
        && value[expected.len()] == 0
}

fn string_list_contains(value: &[u8], expected: &[u8]) -> bool {
    let mut offset = 0;
    let mut found = false;
    while offset < value.len() {
        let Some(relative_end) = value[offset..].iter().position(|byte| *byte == 0) else {
            return false;
        };
        let end = offset + relative_end;
        found |= bytes_equal(&value[offset..end], expected);
        offset = end + 1;
    }
    found
}

fn fw_cfg_mmio_region_from_node(
    structure: *const u8,
    node: PendingFwCfgMmioNode,
    address_cells: u32,
    size_cells: u32,
) -> Result<FwCfgMmioRegion, FdtError> {
    let (value_start, length) = node.reg.ok_or(FdtError::MissingFwCfgMmioReg)?;
    if node.duplicate_reg || !(1..=2).contains(&address_cells) || !(1..=2).contains(&size_cells) {
        return Err(FdtError::InvalidFwCfgMmioReg);
    }
    let tuple_cells = address_cells
        .checked_add(size_cells)
        .ok_or(FdtError::InvalidFwCfgMmioReg)? as usize;
    let tuple_size = tuple_cells
        .checked_mul(4)
        .ok_or(FdtError::InvalidFwCfgMmioReg)?;
    if length != tuple_size {
        return Err(FdtError::InvalidFwCfgMmioReg);
    }

    let address = read_cells(
        unsafe { structure.add(value_start) },
        address_cells as usize,
    );
    let size = read_cells(
        unsafe { structure.add(value_start + address_cells as usize * 4) },
        size_cells as usize,
    );
    let memory = memory_region_from_u64(address, size)?;
    if !memory.start.is_multiple_of(8) || memory.size < 24 {
        return Err(FdtError::InvalidFwCfgMmioReg);
    }
    Ok(FwCfgMmioRegion {
        start: memory.start,
        size: memory.size,
        dma_coherent: node.dma_coherent,
    })
}

fn virtio_mmio_region_from_node(
    structure: *const u8,
    node: PendingVirtioMmioNode,
    address_cells: u32,
    size_cells: u32,
    interrupt_topology: InterruptTopology,
) -> Result<VirtioMmioRegion, FdtError> {
    let (value_start, length) = node.reg.ok_or(FdtError::MissingVirtioMmioReg)?;
    if node.duplicate_reg {
        return Err(FdtError::InvalidVirtioMmioReg);
    }
    if !(1..=2).contains(&address_cells) || !(1..=2).contains(&size_cells) {
        return Err(FdtError::UnsupportedCells);
    }

    let tuple_cells = address_cells
        .checked_add(size_cells)
        .ok_or(FdtError::UnsupportedCells)? as usize;
    let tuple_size = tuple_cells
        .checked_mul(4)
        .ok_or(FdtError::UnsupportedCells)?;
    if length != tuple_size {
        return Err(FdtError::InvalidVirtioMmioReg);
    }

    let address = read_cells(
        unsafe { structure.add(value_start) },
        address_cells as usize,
    );
    let size = read_cells(
        unsafe { structure.add(value_start + address_cells as usize * 4) },
        size_cells as usize,
    );
    let memory = memory_region_from_u64(address, size)?;
    if !memory.start.is_multiple_of(4) || !memory.size.is_multiple_of(4) || memory.size < 0x200 {
        return Err(FdtError::InvalidVirtioMmioReg);
    }
    if node.invalid_interrupts {
        return Err(FdtError::InvalidVirtioMmioInterrupts);
    }
    if node.invalid_interrupt_parent {
        return Err(FdtError::InvalidInterruptParent);
    }

    let interrupts = match node.interrupts {
        Some(raw) => {
            let parent_phandle = node
                .interrupt_parent
                .or(interrupt_topology.root_parent)
                .ok_or(FdtError::InvalidInterruptParent)?;
            if !interrupt_topology.contains_controller(parent_phandle) {
                return Err(FdtError::InvalidInterruptParent);
            }
            Some(parse_gic_interrupt(raw, parent_phandle)?)
        }
        None => None,
    };

    Ok(VirtioMmioRegion {
        start: memory.start,
        size: memory.size,
        dma_coherent: node.dma_coherent,
        interrupts,
    })
}

fn parse_gic_interrupt(raw: [u32; 3], parent_phandle: u32) -> Result<GicInterrupt, FdtError> {
    let [interrupt_type, number, flags] = raw;
    if interrupt_type != 0 || number > 987 {
        return Err(FdtError::InvalidVirtioMmioInterrupts);
    }
    let trigger = match flags {
        1 => GicInterruptTrigger::EdgeRising,
        4 => GicInterruptTrigger::LevelHigh,
        _ => return Err(FdtError::InvalidVirtioMmioInterrupts),
    };
    let id = u16::try_from(number + 32).map_err(|_| FdtError::InvalidVirtioMmioInterrupts)?;
    Ok(GicInterrupt {
        id,
        trigger,
        parent_phandle,
        raw,
    })
}

fn collect_reg_regions(
    structure: *const u8,
    value_start: usize,
    length: usize,
    address_cells: u32,
    size_cells: u32,
    regions: &mut ReservedRegions,
) -> Result<(), FdtError> {
    if !(1..=2).contains(&address_cells) || !(1..=2).contains(&size_cells) {
        return Err(FdtError::UnsupportedCells);
    }

    let tuple_cells = address_cells
        .checked_add(size_cells)
        .ok_or(FdtError::UnsupportedCells)? as usize;
    let tuple_size = tuple_cells
        .checked_mul(4)
        .ok_or(FdtError::UnsupportedCells)?;
    if length == 0 {
        return Ok(());
    }
    if !length.is_multiple_of(tuple_size) {
        return Err(FdtError::UnsupportedCells);
    }

    let mut offset = 0;
    while offset < length {
        let address = read_cells(
            unsafe { structure.add(value_start + offset) },
            address_cells as usize,
        );
        let size = read_cells(
            unsafe { structure.add(value_start + offset + address_cells as usize * 4) },
            size_cells as usize,
        );
        regions.push(memory_region_from_u64(address, size)?)?;
        offset += tuple_size;
    }
    Ok(())
}

fn memory_region_from_u64(address: u64, size: u64) -> Result<MemoryRegion, FdtError> {
    address.checked_add(size).ok_or(FdtError::AddressTooLarge)?;
    let start = usize::try_from(address).map_err(|_| FdtError::AddressTooLarge)?;
    let size = usize::try_from(size).map_err(|_| FdtError::AddressTooLarge)?;
    start.checked_add(size).ok_or(FdtError::AddressTooLarge)?;
    Ok(MemoryRegion { start, size })
}

fn read_cells(address: *const u8, count: usize) -> u64 {
    let mut value = 0_u64;
    for index in 0..count {
        value = (value << 32) | unsafe { read_be_u32(address, index * 4) } as u64;
    }
    value
}

unsafe fn read_be_u32(base: *const u8, offset: usize) -> u32 {
    let bytes = unsafe { ptr::read_unaligned(base.add(offset).cast::<[u8; 4]>()) };
    u32::from_be_bytes(bytes)
}

unsafe fn read_be_u64(base: *const u8, offset: usize) -> u64 {
    let bytes = unsafe { ptr::read_unaligned(base.add(offset).cast::<[u8; 8]>()) };
    u64::from_be_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::{
        FDT_MAGIC, Fdt, FdtError, FwCfgMmioRegion, GicInterrupt, GicInterruptTrigger,
        MAX_RESERVED_REGIONS, MAX_VIRTIO_MMIO_REGIONS, MemoryRegion, PsciCompatibleVersion,
        PsciInfo, PsciMethod, VirtioMmioRegion,
    };
    use std::{format, vec, vec::Vec};

    #[test]
    fn parses_qemu_style_memory_node() {
        let blob = build_fdt(b"memory@40000000");
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(fdt.version(), 17);
        assert_eq!(
            fdt.memory_region(),
            Ok(MemoryRegion {
                start: 0x4000_0000,
                size: 0x0800_0000,
            })
        );
    }

    #[test]
    fn accepts_memory_node_without_unit_address() {
        let blob = build_fdt(b"memory");
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(fdt.memory_region().expect("memory node").size, 128 << 20);
    }

    #[test]
    fn discovers_strict_qemu_psci_hvc_profile() {
        let blob = build_psci_fdt(&[PsciNodeSpec {
            compatible: Some(b"arm,psci-1.0\0arm,psci-0.2\0arm,psci\0"),
            method: Some(b"hvc\0"),
            status: None,
            duplicate_compatible: false,
            duplicate_method: false,
            duplicate_status: false,
            nested: false,
        }]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.psci(),
            Ok(PsciInfo {
                method: PsciMethod::Hvc,
                compatible: PsciCompatibleVersion::V1_0,
            })
        );
    }

    #[test]
    fn accepts_enabled_psci_v0_2_smc_profile() {
        let blob = build_psci_fdt(&[PsciNodeSpec {
            compatible: Some(b"vendor,firmware\0arm,psci-0.2\0"),
            method: Some(b"smc\0"),
            status: Some(b"okay\0"),
            duplicate_compatible: false,
            duplicate_method: false,
            duplicate_status: false,
            nested: false,
        }]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.psci(),
            Ok(PsciInfo {
                method: PsciMethod::Smc,
                compatible: PsciCompatibleVersion::V0_2,
            })
        );
    }

    #[test]
    fn psci_discovery_rejects_missing_nested_and_duplicate_nodes() {
        let blob = build_psci_fdt(&[]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(fdt.psci(), Err(FdtError::MissingPsciNode));

        let mut nested = psci_node();
        nested.nested = true;
        let blob = build_psci_fdt(&[nested]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(fdt.psci(), Err(FdtError::MissingPsciNode));

        let blob = build_psci_fdt(&[psci_node(), psci_node()]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(fdt.psci(), Err(FdtError::MultiplePsciNodes));
    }

    #[test]
    fn psci_discovery_rejects_legacy_or_malformed_compatible_lists() {
        for compatible in [
            None,
            Some(b"arm,psci\0".as_slice()),
            Some(b"arm,psci-1.0".as_slice()),
        ] {
            let mut node = psci_node();
            node.compatible = compatible;
            let blob = build_psci_fdt(&[node]);
            let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
            assert_eq!(fdt.psci(), Err(FdtError::InvalidPsciCompatible));
        }
    }

    #[test]
    fn psci_discovery_rejects_disabled_malformed_or_duplicate_status() {
        for status in [
            Some(b"disabled\0".as_slice()),
            Some(b"enabled\0".as_slice()),
        ] {
            let mut node = psci_node();
            node.status = status;
            let blob = build_psci_fdt(&[node]);
            let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
            assert_eq!(fdt.psci(), Err(FdtError::InvalidPsciStatus));
        }

        let mut duplicate = psci_node();
        duplicate.status = Some(b"okay\0");
        duplicate.duplicate_status = true;
        let blob = build_psci_fdt(&[duplicate]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(fdt.psci(), Err(FdtError::DuplicatePsciProperty));
    }

    #[test]
    fn psci_discovery_rejects_missing_invalid_or_duplicate_method() {
        let mut missing = psci_node();
        missing.method = None;
        let blob = build_psci_fdt(&[missing]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(fdt.psci(), Err(FdtError::MissingPsciMethod));

        let mut invalid = psci_node();
        invalid.method = Some(b"hypercall\0");
        let blob = build_psci_fdt(&[invalid]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(fdt.psci(), Err(FdtError::InvalidPsciMethod));

        let mut duplicate = psci_node();
        duplicate.duplicate_method = true;
        let blob = build_psci_fdt(&[duplicate]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(fdt.psci(), Err(FdtError::DuplicatePsciProperty));
    }

    #[test]
    fn psci_discovery_rejects_duplicate_compatible_property() {
        let mut node = psci_node();
        node.duplicate_compatible = true;
        let blob = build_psci_fdt(&[node]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(fdt.psci(), Err(FdtError::DuplicatePsciProperty));
    }

    #[test]
    fn discovers_one_coherent_qemu_fw_cfg_mmio_region() {
        let node = VirtioNodeSpec {
            name: b"fw-cfg@9020000",
            compatible: Some(b"vendor,other\0qemu,fw-cfg-mmio\0"),
            reg: Some(vec![0, 0x0902_0000, 0, 0x18]),
            status: None,
            dma_coherent: true,
            interrupts: None,
            interrupt_parent: None,
            nested: false,
        };
        let blob = build_virtio_fdt(2, 2, &[node]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.fw_cfg_mmio_region(),
            Ok(Some(FwCfgMmioRegion {
                start: 0x0902_0000,
                size: 0x18,
                dma_coherent: true,
            }))
        );
    }

    #[test]
    fn fw_cfg_discovery_distinguishes_absence_disabled_and_duplicates() {
        let blob = build_virtio_fdt(2, 2, &[virtio_node(0x0a00_0000)]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(fdt.fw_cfg_mmio_region(), Ok(None));

        let disabled = VirtioNodeSpec {
            name: b"fw-cfg@9020000",
            compatible: Some(b"qemu,fw-cfg-mmio\0"),
            reg: None,
            status: Some(b"disabled\0"),
            dma_coherent: true,
            interrupts: None,
            interrupt_parent: None,
            nested: false,
        };
        let blob = build_virtio_fdt(2, 2, &[disabled]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(fdt.fw_cfg_mmio_region(), Ok(None));

        let first = VirtioNodeSpec {
            name: b"fw-cfg@9020000",
            compatible: Some(b"qemu,fw-cfg-mmio\0"),
            reg: Some(vec![0, 0x0902_0000, 0, 0x18]),
            status: None,
            dma_coherent: true,
            interrupts: None,
            interrupt_parent: None,
            nested: false,
        };
        let mut second = first.clone();
        second.name = b"fw-cfg@9030000";
        second.reg = Some(vec![0, 0x0903_0000, 0, 0x18]);
        let blob = build_virtio_fdt(2, 2, &[first, second]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(
            fdt.fw_cfg_mmio_region(),
            Err(FdtError::MultipleFwCfgMmioRegions)
        );
    }

    #[test]
    fn rejects_malformed_enabled_fw_cfg_reg() {
        let missing = VirtioNodeSpec {
            name: b"fw-cfg@9020000",
            compatible: Some(b"qemu,fw-cfg-mmio\0"),
            reg: None,
            status: None,
            dma_coherent: true,
            interrupts: None,
            interrupt_parent: None,
            nested: false,
        };
        let blob = build_virtio_fdt(2, 2, &[missing]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(fdt.fw_cfg_mmio_region(), Err(FdtError::MissingFwCfgMmioReg));

        let too_small = VirtioNodeSpec {
            name: b"fw-cfg@9020000",
            compatible: Some(b"qemu,fw-cfg-mmio\0"),
            reg: Some(vec![0, 0x0902_0000, 0, 0x10]),
            status: None,
            dma_coherent: true,
            interrupts: None,
            interrupt_parent: None,
            nested: false,
        };
        let blob = build_virtio_fdt(2, 2, &[too_small]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(fdt.fw_cfg_mmio_region(), Err(FdtError::InvalidFwCfgMmioReg));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut blob = build_fdt(b"memory@40000000");
        blob[0] = 0;

        assert!(matches!(
            unsafe { Fdt::from_ptr(blob.as_ptr() as usize) },
            Err(FdtError::BadMagic)
        ));
    }

    #[test]
    fn rejects_structure_outside_blob() {
        let mut blob = build_fdt(b"memory@40000000");
        let invalid_size = (blob.len() as u32).saturating_add(4).to_be_bytes();
        blob[36..40].copy_from_slice(&invalid_size);

        assert!(matches!(
            unsafe { Fdt::from_ptr(blob.as_ptr() as usize) },
            Err(FdtError::InvalidHeader)
        ));
    }

    #[test]
    fn discovers_direct_root_virtio_mmio_with_string_list_and_unordered_properties() {
        let mut matching = virtio_node(0x0a00_0000);
        matching.compatible = Some(b"vendor,transport\0virtio,mmio\0");
        matching.dma_coherent = true;
        matching.interrupts = Some(vec![0, 47, 1]);

        let mut prefix_only = virtio_node(0x0a00_0200);
        prefix_only.compatible = Some(b"virtio,mmio-v2\0");
        let blob = build_virtio_fdt(2, 2, &[matching, prefix_only]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.virtio_mmio_regions()
                .expect("virtio-mmio regions")
                .as_slice(),
            &[VirtioMmioRegion {
                start: 0x0a00_0000,
                size: 0x200,
                dma_coherent: true,
                interrupts: Some(GicInterrupt {
                    id: 79,
                    trigger: GicInterruptTrigger::EdgeRising,
                    parent_phandle: 0x8001,
                    raw: [0, 47, 1],
                }),
            }]
        );
    }

    #[test]
    fn skips_disabled_virtio_mmio_even_when_reg_is_missing() {
        let mut disabled = virtio_node(0x0a00_0000);
        disabled.reg = None;
        disabled.status = Some(b"disabled\0");
        let blob = build_virtio_fdt(2, 2, &[disabled]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert!(
            fdt.virtio_mmio_regions()
                .expect("disabled skipped")
                .is_empty()
        );
    }

    #[test]
    fn rejects_enabled_compatible_node_without_reg() {
        let mut missing = virtio_node(0x0a00_0000);
        missing.reg = None;
        let blob = build_virtio_fdt(2, 2, &[missing]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.virtio_mmio_regions(),
            Err(FdtError::MissingVirtioMmioReg)
        );
    }

    #[test]
    fn rejects_unsupported_root_cells_for_compatible_node() {
        let mut node = virtio_node(0x0a00_0000);
        node.reg = Some(vec![0, 0, 0x0a00_0000, 0, 0x200]);
        let blob = build_virtio_fdt(3, 2, &[node]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(fdt.virtio_mmio_regions(), Err(FdtError::UnsupportedCells));
    }

    #[test]
    fn rejects_reg_with_more_than_one_tuple() {
        let mut node = virtio_node(0x0a00_0000);
        node.reg = Some(vec![0, 0x0a00_0000, 0, 0x200, 0, 0x0a00_0200, 0, 0x200]);
        let blob = build_virtio_fdt(2, 2, &[node]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.virtio_mmio_regions(),
            Err(FdtError::InvalidVirtioMmioReg)
        );
    }

    #[test]
    fn reports_fixed_virtio_mmio_capacity_overflow() {
        let nodes: Vec<_> = (0..=MAX_VIRTIO_MMIO_REGIONS)
            .map(|index| virtio_node(0x0a00_0000 + index as u32 * 0x200))
            .collect();
        let blob = build_virtio_fdt(2, 2, &nodes);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.virtio_mmio_regions(),
            Err(FdtError::TooManyVirtioMmioRegions)
        );
    }

    #[test]
    fn accepts_noncoherent_polling_transport_without_interrupts() {
        let blob = build_virtio_fdt(2, 2, &[virtio_node(0x0a00_0000)]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.virtio_mmio_regions()
                .expect("polling transport")
                .as_slice(),
            &[VirtioMmioRegion {
                start: 0x0a00_0000,
                size: 0x200,
                dma_coherent: false,
                interrupts: None,
            }]
        );
    }

    #[test]
    fn rejects_virtio_mmio_address_range_overflow() {
        let mut node = virtio_node(0x0a00_0000);
        node.reg = Some(vec![u32::MAX, 0xffff_f000, 0, 0x2000]);
        let blob = build_virtio_fdt(2, 2, &[node]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(fdt.virtio_mmio_regions(), Err(FdtError::AddressTooLarge));
    }

    #[test]
    fn ignores_nested_compatible_nodes() {
        let mut nested = virtio_node(0x0a00_0000);
        nested.nested = true;
        let blob = build_virtio_fdt(2, 2, &[nested]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert!(
            fdt.virtio_mmio_regions()
                .expect("nested ignored")
                .is_empty()
        );
    }

    #[test]
    fn rejects_misaligned_small_reg_and_malformed_interrupts() {
        let mut misaligned = virtio_node(0x0a00_0000);
        misaligned.reg = Some(vec![0, 0x0a00_0002, 0, 0x100]);
        let blob = build_virtio_fdt(2, 2, &[misaligned]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(
            fdt.virtio_mmio_regions(),
            Err(FdtError::InvalidVirtioMmioReg)
        );

        let mut malformed_interrupts = virtio_node(0x0a00_0000);
        malformed_interrupts.interrupts = Some(vec![0, 48]);
        let blob = build_virtio_fdt(2, 2, &[malformed_interrupts]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(
            fdt.virtio_mmio_regions(),
            Err(FdtError::InvalidVirtioMmioInterrupts)
        );
    }

    #[test]
    fn resolves_controller_after_virtio_linux_phandle_and_local_parent_override() {
        let mut node = virtio_node(0x0a00_0000);
        node.interrupts = Some(vec![0, 32, 4]);
        node.interrupt_parent = Some(0x9001);
        let mut gic = gic_node(0x9001);
        gic.compatible = b"vendor,gic\0arm,gic-400\0";
        gic.phandle = None;
        gic.linux_phandle = Some(0x9001);
        let blob = build_virtio_fdt_with_topology(2, 2, &[node], Some(0xdead), Some(&gic), true);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.virtio_mmio_regions()
                .expect("locally routed interrupt")
                .as_slice()[0]
                .interrupts,
            Some(GicInterrupt {
                id: 64,
                trigger: GicInterruptTrigger::LevelHigh,
                parent_phandle: 0x9001,
                raw: [0, 32, 4],
            })
        );
    }

    #[test]
    fn accepts_maximum_gic_spi_number_and_rejects_the_next_one() {
        let mut maximum = virtio_node(0x0a00_0000);
        maximum.interrupts = Some(vec![0, 987, 1]);
        let blob = build_virtio_fdt(2, 2, &[maximum]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(
            fdt.virtio_mmio_regions().expect("maximum SPI").as_slice()[0]
                .interrupts
                .expect("interrupt")
                .id,
            1019
        );

        let mut out_of_range = virtio_node(0x0a00_0000);
        out_of_range.interrupts = Some(vec![0, 988, 1]);
        let blob = build_virtio_fdt(2, 2, &[out_of_range]);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(
            fdt.virtio_mmio_regions(),
            Err(FdtError::InvalidVirtioMmioInterrupts)
        );
    }

    #[test]
    fn rejects_missing_and_unknown_interrupt_parents() {
        let mut node = virtio_node(0x0a00_0000);
        node.interrupts = Some(vec![0, 47, 1]);
        let gic = gic_node(0x8001);

        let blob = build_virtio_fdt_with_topology(2, 2, &[node.clone()], None, Some(&gic), false);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(
            fdt.virtio_mmio_regions(),
            Err(FdtError::InvalidInterruptParent)
        );

        let blob = build_virtio_fdt_with_topology(2, 2, &[node], Some(0x9001), Some(&gic), false);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(
            fdt.virtio_mmio_regions(),
            Err(FdtError::InvalidInterruptParent)
        );
    }

    #[test]
    fn rejects_missing_wrong_and_malformed_gic_interrupt_cells() {
        let mut node = virtio_node(0x0a00_0000);
        node.interrupts = Some(vec![0, 47, 1]);
        for cells in [None, Some(vec![2]), Some(vec![3, 4])] {
            let mut gic = gic_node(0x8001);
            gic.interrupt_cells = cells;
            let blob = build_virtio_fdt_with_topology(
                2,
                2,
                &[node.clone()],
                Some(0x8001),
                Some(&gic),
                false,
            );
            let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
            assert_eq!(
                fdt.virtio_mmio_regions(),
                Err(FdtError::InvalidGicV2InterruptController)
            );
        }
    }

    #[test]
    fn rejects_non_spi_and_noncanonical_gic_flags() {
        for raw in [[1, 47, 1], [0, 47, 5]] {
            let mut node = virtio_node(0x0a00_0000);
            node.interrupts = Some(raw.to_vec());
            let blob = build_virtio_fdt(2, 2, &[node]);
            let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
            assert_eq!(
                fdt.virtio_mmio_regions(),
                Err(FdtError::InvalidVirtioMmioInterrupts)
            );
        }
    }

    #[test]
    fn rejects_gic_without_controller_marker_or_consistent_phandle_aliases() {
        let mut node = virtio_node(0x0a00_0000);
        node.interrupts = Some(vec![0, 47, 1]);

        let mut missing_marker = gic_node(0x8001);
        missing_marker.interrupt_controller = false;
        let blob = build_virtio_fdt_with_topology(
            2,
            2,
            &[node.clone()],
            Some(0x8001),
            Some(&missing_marker),
            false,
        );
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(
            fdt.virtio_mmio_regions(),
            Err(FdtError::InvalidGicV2InterruptController)
        );

        let mut mismatched_aliases = gic_node(0x8001);
        mismatched_aliases.linux_phandle = Some(0x8002);
        let blob = build_virtio_fdt_with_topology(
            2,
            2,
            &[node],
            Some(0x8001),
            Some(&mismatched_aliases),
            false,
        );
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");
        assert_eq!(
            fdt.virtio_mmio_regions(),
            Err(FdtError::InvalidGicV2InterruptController)
        );
    }

    #[test]
    fn parses_header_memory_reservation_map() {
        let blob = build_fdt_with_reserved(
            b"memory@40000000",
            &[(0x4100_0000, 0x2000), (0x4200_1000, 0x3000)],
            &[],
            None,
        );
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.reserved_regions().expect("reservations").as_slice(),
            &[
                MemoryRegion {
                    start: 0x4100_0000,
                    size: 0x2000,
                },
                MemoryRegion {
                    start: 0x4200_1000,
                    size: 0x3000,
                },
            ]
        );
    }

    #[test]
    fn parses_static_reserved_memory_children() {
        let blob = build_fdt_with_reserved(
            b"memory@40000000",
            &[],
            &[(0x4300_0000, 0x18_000), (0x4400_1000, 0x3000)],
            None,
        );
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.reserved_regions().expect("reservations").as_slice(),
            &[
                MemoryRegion {
                    start: 0x4300_0000,
                    size: 0x18_000,
                },
                MemoryRegion {
                    start: 0x4400_1000,
                    size: 0x3000,
                },
            ]
        );
    }

    #[test]
    fn reports_fixed_reservation_capacity_overflow() {
        let reservations: Vec<_> = (0..=MAX_RESERVED_REGIONS)
            .map(|index| (0x4000_0000 + index as u64 * 0x1000, 0x1000))
            .collect();
        let blob = build_fdt_with_reserved(b"memory@40000000", &reservations, &[], None);
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.reserved_regions(),
            Err(FdtError::TooManyReservedRegions)
        );
    }

    #[test]
    fn rejects_unsupported_dynamic_reserved_memory() {
        let blob = build_fdt_with_reserved(b"memory@40000000", &[], &[], Some(0x20_000));
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.reserved_regions(),
            Err(FdtError::DynamicReservedMemoryUnsupported)
        );
    }

    #[test]
    fn reg_takes_precedence_over_size_in_reserved_child() {
        let blob = build_fdt_with_reserved(
            b"memory@40000000",
            &[],
            &[(0x4300_0000, 0x4000)],
            Some(0x20_000),
        );
        let fdt = unsafe { Fdt::from_ptr(blob.as_ptr() as usize) }.expect("valid FDT");

        assert_eq!(
            fdt.reserved_regions()
                .expect("static reservation")
                .as_slice(),
            &[MemoryRegion {
                start: 0x4300_0000,
                size: 0x4000,
            }]
        );
    }

    #[derive(Clone, Copy)]
    struct PsciNodeSpec {
        compatible: Option<&'static [u8]>,
        method: Option<&'static [u8]>,
        status: Option<&'static [u8]>,
        duplicate_compatible: bool,
        duplicate_method: bool,
        duplicate_status: bool,
        nested: bool,
    }

    fn psci_node() -> PsciNodeSpec {
        PsciNodeSpec {
            compatible: Some(b"arm,psci-1.0\0arm,psci-0.2\0"),
            method: Some(b"hvc\0"),
            status: None,
            duplicate_compatible: false,
            duplicate_method: false,
            duplicate_status: false,
            nested: false,
        }
    }

    fn build_psci_fdt(nodes: &[PsciNodeSpec]) -> Vec<u8> {
        const HEADER_SIZE: usize = 40;
        let strings = b"compatible\0method\0status\0";
        let mut structure = Vec::new();
        push_u32(&mut structure, 1); // FDT_BEGIN_NODE
        push_aligned_bytes(&mut structure, b"");

        for node in nodes {
            if node.nested {
                push_u32(&mut structure, 1); // FDT_BEGIN_NODE
                push_aligned_bytes(&mut structure, b"soc");
            }
            push_u32(&mut structure, 1); // FDT_BEGIN_NODE
            push_aligned_bytes(&mut structure, b"psci");

            if let Some(method) = node.method {
                push_property(
                    &mut structure,
                    property_name_offset(strings, b"method"),
                    method,
                );
                if node.duplicate_method {
                    push_property(
                        &mut structure,
                        property_name_offset(strings, b"method"),
                        method,
                    );
                }
            }
            if let Some(status) = node.status {
                push_property(
                    &mut structure,
                    property_name_offset(strings, b"status"),
                    status,
                );
                if node.duplicate_status {
                    push_property(
                        &mut structure,
                        property_name_offset(strings, b"status"),
                        status,
                    );
                }
            }
            if let Some(compatible) = node.compatible {
                push_property(
                    &mut structure,
                    property_name_offset(strings, b"compatible"),
                    compatible,
                );
                if node.duplicate_compatible {
                    push_property(
                        &mut structure,
                        property_name_offset(strings, b"compatible"),
                        compatible,
                    );
                }
            }

            push_u32(&mut structure, 2); // FDT_END_NODE
            if node.nested {
                push_u32(&mut structure, 2); // FDT_END_NODE
            }
        }

        push_u32(&mut structure, 2); // FDT_END_NODE
        push_u32(&mut structure, 9); // FDT_END

        let structure_offset = HEADER_SIZE + 16;
        let strings_offset = structure_offset + structure.len();
        let total_size = strings_offset + strings.len();
        let mut blob = Vec::with_capacity(total_size);
        for value in [
            FDT_MAGIC,
            total_size as u32,
            structure_offset as u32,
            strings_offset as u32,
            HEADER_SIZE as u32,
            17,
            16,
            0,
            strings.len() as u32,
            structure.len() as u32,
        ] {
            push_u32(&mut blob, value);
        }
        push_u64(&mut blob, 0);
        push_u64(&mut blob, 0);
        assert_eq!(blob.len(), structure_offset);
        blob.extend_from_slice(&structure);
        blob.extend_from_slice(strings);
        blob
    }

    #[derive(Clone)]
    struct VirtioNodeSpec {
        name: &'static [u8],
        compatible: Option<&'static [u8]>,
        reg: Option<Vec<u32>>,
        status: Option<&'static [u8]>,
        dma_coherent: bool,
        interrupts: Option<Vec<u32>>,
        interrupt_parent: Option<u32>,
        nested: bool,
    }

    fn virtio_node(address: u32) -> VirtioNodeSpec {
        VirtioNodeSpec {
            name: b"virtio_mmio@a000000",
            compatible: Some(b"virtio,mmio\0"),
            reg: Some(vec![0, address, 0, 0x200]),
            status: None,
            dma_coherent: false,
            interrupts: None,
            interrupt_parent: None,
            nested: false,
        }
    }

    #[derive(Clone)]
    struct GicNodeSpec {
        compatible: &'static [u8],
        phandle: Option<u32>,
        linux_phandle: Option<u32>,
        interrupt_cells: Option<Vec<u32>>,
        interrupt_controller: bool,
    }

    fn gic_node(phandle: u32) -> GicNodeSpec {
        GicNodeSpec {
            compatible: b"arm,cortex-a15-gic\0",
            phandle: Some(phandle),
            linux_phandle: None,
            interrupt_cells: Some(vec![3]),
            interrupt_controller: true,
        }
    }

    fn build_virtio_fdt(address_cells: u32, size_cells: u32, nodes: &[VirtioNodeSpec]) -> Vec<u8> {
        let needs_interrupt_topology = nodes.iter().any(|node| node.interrupts.is_some());
        let default_gic = gic_node(0x8001);
        build_virtio_fdt_with_topology(
            address_cells,
            size_cells,
            nodes,
            needs_interrupt_topology.then_some(0x8001),
            needs_interrupt_topology.then_some(&default_gic),
            false,
        )
    }

    fn build_virtio_fdt_with_topology(
        address_cells: u32,
        size_cells: u32,
        nodes: &[VirtioNodeSpec],
        root_parent: Option<u32>,
        gic: Option<&GicNodeSpec>,
        gic_after_virtio: bool,
    ) -> Vec<u8> {
        const HEADER_SIZE: usize = 40;
        let strings = b"#address-cells\0#size-cells\0compatible\0reg\0status\0dma-coherent\0interrupts\0interrupt-parent\0interrupt-controller\0#interrupt-cells\0phandle\0linux,phandle\0";

        let mut structure = Vec::new();
        push_u32(&mut structure, 1); // FDT_BEGIN_NODE
        push_aligned_bytes(&mut structure, b"");
        push_property(
            &mut structure,
            property_name_offset(strings, b"#address-cells"),
            &address_cells.to_be_bytes(),
        );
        push_property(
            &mut structure,
            property_name_offset(strings, b"#size-cells"),
            &size_cells.to_be_bytes(),
        );
        if let Some(root_parent) = root_parent {
            push_property(
                &mut structure,
                property_name_offset(strings, b"interrupt-parent"),
                &root_parent.to_be_bytes(),
            );
        }

        if !gic_after_virtio && let Some(gic) = gic {
            push_gic_node(&mut structure, strings, gic);
        }

        for node in nodes {
            if node.nested {
                push_u32(&mut structure, 1); // FDT_BEGIN_NODE
                push_aligned_bytes(&mut structure, b"soc");
            }
            push_u32(&mut structure, 1); // FDT_BEGIN_NODE
            push_aligned_bytes(&mut structure, node.name);

            // Deliberately emit `compatible` last: discovery must defer the
            // decision until END_NODE rather than depend on property order.
            if let Some(reg) = &node.reg {
                let mut bytes = Vec::with_capacity(reg.len() * 4);
                for cell in reg {
                    push_u32(&mut bytes, *cell);
                }
                push_property(
                    &mut structure,
                    property_name_offset(strings, b"reg"),
                    &bytes,
                );
            }
            if let Some(status) = node.status {
                push_property(
                    &mut structure,
                    property_name_offset(strings, b"status"),
                    status,
                );
            }
            if node.dma_coherent {
                push_property(
                    &mut structure,
                    property_name_offset(strings, b"dma-coherent"),
                    &[],
                );
            }
            if let Some(interrupts) = &node.interrupts {
                let mut bytes = Vec::with_capacity(interrupts.len() * 4);
                for cell in interrupts {
                    push_u32(&mut bytes, *cell);
                }
                push_property(
                    &mut structure,
                    property_name_offset(strings, b"interrupts"),
                    &bytes,
                );
            }
            if let Some(interrupt_parent) = node.interrupt_parent {
                push_property(
                    &mut structure,
                    property_name_offset(strings, b"interrupt-parent"),
                    &interrupt_parent.to_be_bytes(),
                );
            }
            if let Some(compatible) = node.compatible {
                push_property(
                    &mut structure,
                    property_name_offset(strings, b"compatible"),
                    compatible,
                );
            }

            push_u32(&mut structure, 2); // FDT_END_NODE
            if node.nested {
                push_u32(&mut structure, 2); // FDT_END_NODE
            }
        }

        if gic_after_virtio && let Some(gic) = gic {
            push_gic_node(&mut structure, strings, gic);
        }

        push_u32(&mut structure, 2); // FDT_END_NODE
        push_u32(&mut structure, 9); // FDT_END

        let structure_offset = HEADER_SIZE + 16;
        let strings_offset = structure_offset + structure.len();
        let total_size = strings_offset + strings.len();

        let mut blob = Vec::with_capacity(total_size);
        for value in [
            FDT_MAGIC,
            total_size as u32,
            structure_offset as u32,
            strings_offset as u32,
            HEADER_SIZE as u32,
            17,
            16,
            0,
            strings.len() as u32,
            structure.len() as u32,
        ] {
            push_u32(&mut blob, value);
        }
        push_u64(&mut blob, 0);
        push_u64(&mut blob, 0);
        assert_eq!(blob.len(), structure_offset);
        blob.extend_from_slice(&structure);
        blob.extend_from_slice(strings);
        blob
    }

    fn push_gic_node(structure: &mut Vec<u8>, strings: &[u8], gic: &GicNodeSpec) {
        push_u32(structure, 1); // FDT_BEGIN_NODE
        push_aligned_bytes(structure, b"intc@8000000");

        // Deliberately mix aliases and emit `compatible` last so controller
        // recognition cannot depend on property order.
        if let Some(phandle) = gic.phandle {
            push_property(
                structure,
                property_name_offset(strings, b"phandle"),
                &phandle.to_be_bytes(),
            );
        }
        if gic.interrupt_controller {
            push_property(
                structure,
                property_name_offset(strings, b"interrupt-controller"),
                &[],
            );
        }
        if let Some(interrupt_cells) = &gic.interrupt_cells {
            let mut bytes = Vec::with_capacity(interrupt_cells.len() * 4);
            for cell in interrupt_cells {
                push_u32(&mut bytes, *cell);
            }
            push_property(
                structure,
                property_name_offset(strings, b"#interrupt-cells"),
                &bytes,
            );
        }
        if let Some(linux_phandle) = gic.linux_phandle {
            push_property(
                structure,
                property_name_offset(strings, b"linux,phandle"),
                &linux_phandle.to_be_bytes(),
            );
        }
        push_property(
            structure,
            property_name_offset(strings, b"compatible"),
            gic.compatible,
        );
        push_u32(structure, 2); // FDT_END_NODE
    }

    fn build_fdt(memory_node_name: &[u8]) -> Vec<u8> {
        build_fdt_with_reserved(memory_node_name, &[], &[], None)
    }

    fn build_fdt_with_reserved(
        memory_node_name: &[u8],
        header_reserved: &[(u64, u64)],
        reserved_memory: &[(u64, u64)],
        dynamic_reserved_size: Option<u64>,
    ) -> Vec<u8> {
        const HEADER_SIZE: usize = 40;
        const ADDRESS_CELLS_OFFSET: u32 = 0;
        const SIZE_CELLS_OFFSET: u32 = 15;
        const REG_OFFSET: u32 = 27;
        const RANGES_OFFSET: u32 = 31;
        const DYNAMIC_SIZE_OFFSET: u32 = 38;

        let mut structure = Vec::new();
        push_u32(&mut structure, 1); // FDT_BEGIN_NODE
        push_aligned_bytes(&mut structure, b"");
        push_property(&mut structure, ADDRESS_CELLS_OFFSET, &2_u32.to_be_bytes());
        push_property(&mut structure, SIZE_CELLS_OFFSET, &2_u32.to_be_bytes());
        push_u32(&mut structure, 1); // FDT_BEGIN_NODE
        push_aligned_bytes(&mut structure, memory_node_name);

        let mut reg = Vec::new();
        for cell in [0_u32, 0x4000_0000, 0, 0x0800_0000] {
            push_u32(&mut reg, cell);
        }
        push_property(&mut structure, REG_OFFSET, &reg);
        push_u32(&mut structure, 2); // FDT_END_NODE

        if !reserved_memory.is_empty() || dynamic_reserved_size.is_some() {
            push_u32(&mut structure, 1); // FDT_BEGIN_NODE
            push_aligned_bytes(&mut structure, b"reserved-memory");
            push_property(&mut structure, ADDRESS_CELLS_OFFSET, &2_u32.to_be_bytes());
            push_property(&mut structure, SIZE_CELLS_OFFSET, &2_u32.to_be_bytes());
            push_property(&mut structure, RANGES_OFFSET, &[]);

            for (index, &(address, size)) in reserved_memory.iter().enumerate() {
                push_u32(&mut structure, 1); // FDT_BEGIN_NODE
                let node_name = format!("region@{address:x}");
                push_aligned_bytes(&mut structure, node_name.as_bytes());
                let mut reserved_reg = Vec::new();
                push_u64_cells(&mut reserved_reg, address);
                push_u64_cells(&mut reserved_reg, size);
                push_property(&mut structure, REG_OFFSET, &reserved_reg);
                if index == 0
                    && let Some(dynamic_size) = dynamic_reserved_size
                {
                    let mut value = Vec::new();
                    push_u64_cells(&mut value, dynamic_size);
                    push_property(&mut structure, DYNAMIC_SIZE_OFFSET, &value);
                }
                push_u32(&mut structure, 2); // FDT_END_NODE
            }

            if reserved_memory.is_empty()
                && let Some(size) = dynamic_reserved_size
            {
                push_u32(&mut structure, 1); // FDT_BEGIN_NODE
                push_aligned_bytes(&mut structure, b"dynamic");
                let mut value = Vec::new();
                push_u64_cells(&mut value, size);
                push_property(&mut structure, DYNAMIC_SIZE_OFFSET, &value);
                push_u32(&mut structure, 2); // FDT_END_NODE
            }

            push_u32(&mut structure, 2); // FDT_END_NODE
        }

        push_u32(&mut structure, 2); // FDT_END_NODE
        push_u32(&mut structure, 9); // FDT_END

        let strings = b"#address-cells\0#size-cells\0reg\0ranges\0size\0";
        let reservation_size = (header_reserved.len() + 1) * 16;
        let structure_offset = HEADER_SIZE + reservation_size;
        let strings_offset = structure_offset + structure.len();
        let total_size = strings_offset + strings.len();

        let mut blob = Vec::with_capacity(total_size);
        for value in [
            FDT_MAGIC,
            total_size as u32,
            structure_offset as u32,
            strings_offset as u32,
            HEADER_SIZE as u32,
            17,
            16,
            0,
            strings.len() as u32,
            structure.len() as u32,
        ] {
            push_u32(&mut blob, value);
        }
        for &(address, size) in header_reserved {
            push_u64(&mut blob, address);
            push_u64(&mut blob, size);
        }
        push_u64(&mut blob, 0);
        push_u64(&mut blob, 0);
        assert_eq!(blob.len(), structure_offset);
        blob.extend_from_slice(&structure);
        blob.extend_from_slice(strings);
        blob
    }

    fn property_name_offset(strings: &[u8], expected: &[u8]) -> u32 {
        let mut offset = 0;
        while offset < strings.len() {
            let end = offset
                + strings[offset..]
                    .iter()
                    .position(|byte| *byte == 0)
                    .expect("terminated property name");
            if &strings[offset..end] == expected {
                return offset as u32;
            }
            offset = end + 1;
        }
        panic!("missing property name")
    }

    fn push_property(structure: &mut Vec<u8>, name_offset: u32, value: &[u8]) {
        push_u32(structure, 3); // FDT_PROP
        push_u32(structure, value.len() as u32);
        push_u32(structure, name_offset);
        structure.extend_from_slice(value);
        while !structure.len().is_multiple_of(4) {
            structure.push(0);
        }
    }

    fn push_aligned_bytes(structure: &mut Vec<u8>, value: &[u8]) {
        structure.extend_from_slice(value);
        structure.push(0);
        while !structure.len().is_multiple_of(4) {
            structure.push(0);
        }
    }

    fn push_u32(output: &mut Vec<u8>, value: u32) {
        output.extend_from_slice(&value.to_be_bytes());
    }

    fn push_u64(output: &mut Vec<u8>, value: u64) {
        output.extend_from_slice(&value.to_be_bytes());
    }

    fn push_u64_cells(output: &mut Vec<u8>, value: u64) {
        push_u32(output, (value >> 32) as u32);
        push_u32(output, value as u32);
    }
}
