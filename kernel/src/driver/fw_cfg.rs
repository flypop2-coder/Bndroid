//! QEMU `fw_cfg` MMIO transport with synchronous, bounded DMA operations.

use core::{hint::spin_loop, ptr};

use bndroid_kernel::{
    fdt::FwCfgMmioRegion,
    fw_cfg::{
        DMA_CONTROL_ERROR, DMA_CONTROL_READ, DMA_CONTROL_SELECT, DMA_CONTROL_WRITE, DMA_FEATURE,
        DmaAccess, FEATURES_SELECTOR, SIGNATURE_SELECTOR,
    },
};

use crate::arch::aarch64::dma;

const DATA_REGISTER: usize = 0;
const SELECTOR_REGISTER: usize = 8;
const DMA_ADDRESS_REGISTER: usize = 16;
const MIN_REGION_BYTES: usize = 24;
const DMA_SPIN_LIMIT: usize = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FwCfgError {
    MisalignedBase,
    RegionTooSmall,
    AddressOverflow,
    NonCoherent,
    InvalidSignature,
    MissingDma,
    EmptyTransfer,
    TransferTooLarge,
    DmaRejected,
    DmaTimedOut,
}

impl FwCfgError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MisalignedBase => "fw_cfg MMIO base is not 64-bit aligned",
            Self::RegionTooSmall => "fw_cfg MMIO window is smaller than 24 bytes",
            Self::AddressOverflow => "fw_cfg MMIO or DMA address overflows",
            Self::NonCoherent => "fw_cfg DMA transport is not coherent",
            Self::InvalidSignature => "fw_cfg signature is not QEMU",
            Self::MissingDma => "fw_cfg DMA feature is absent",
            Self::EmptyTransfer => "fw_cfg DMA transfer is empty",
            Self::TransferTooLarge => "fw_cfg DMA transfer exceeds u32 length",
            Self::DmaRejected => "fw_cfg DMA operation was rejected",
            Self::DmaTimedOut => "fw_cfg DMA operation did not complete",
        }
    }
}

pub struct FwCfgMmio {
    base: usize,
    size: usize,
    features: u32,
    dma_operations: u32,
}

impl FwCfgMmio {
    /// # Safety
    ///
    /// `region` must describe the uniquely owned QEMU fw_cfg register window,
    /// mapped as Device memory for the lifetime of this value.
    pub unsafe fn probe(region: FwCfgMmioRegion) -> Result<Self, FwCfgError> {
        if !region.start.is_multiple_of(core::mem::align_of::<u64>()) {
            return Err(FwCfgError::MisalignedBase);
        }
        if region.size < MIN_REGION_BYTES {
            return Err(FwCfgError::RegionTooSmall);
        }
        region
            .start
            .checked_add(region.size)
            .ok_or(FwCfgError::AddressOverflow)?;
        if !region.dma_coherent {
            return Err(FwCfgError::NonCoherent);
        }

        let mut transport = Self {
            base: region.start,
            size: region.size,
            features: 0,
            dma_operations: 0,
        };
        if transport.read_item::<4>(SIGNATURE_SELECTOR) != *b"QEMU" {
            return Err(FwCfgError::InvalidSignature);
        }
        transport.features = u32::from_le_bytes(transport.read_item(FEATURES_SELECTOR));
        if transport.features & DMA_FEATURE == 0 {
            return Err(FwCfgError::MissingDma);
        }
        Ok(transport)
    }

    pub const fn base(&self) -> usize {
        self.base
    }

    pub const fn size(&self) -> usize {
        self.size
    }

    pub const fn features(&self) -> u32 {
        self.features
    }

    pub const fn dma_operations(&self) -> u32 {
        self.dma_operations
    }

    /// Reads `length` bytes from one selector into identity-mapped coherent
    /// RAM at `physical_address`.
    pub fn dma_read(
        &mut self,
        selector: u16,
        physical_address: usize,
        length: usize,
    ) -> Result<(), FwCfgError> {
        self.dma_transfer(
            (u32::from(selector) << 16) | DMA_CONTROL_SELECT | DMA_CONTROL_READ,
            physical_address,
            length,
        )
    }

    /// Writes `length` bytes from identity-mapped coherent RAM at
    /// `physical_address` into one writable selector.
    pub fn dma_write(
        &mut self,
        selector: u16,
        physical_address: usize,
        length: usize,
    ) -> Result<(), FwCfgError> {
        self.dma_transfer(
            (u32::from(selector) << 16) | DMA_CONTROL_SELECT | DMA_CONTROL_WRITE,
            physical_address,
            length,
        )
    }

    fn read_item<const N: usize>(&mut self, selector: u16) -> [u8; N] {
        self.write_selector(selector);
        let mut bytes = [0_u8; N];
        for byte in &mut bytes {
            *byte = unsafe { ptr::read_volatile(self.base as *const u8) };
        }
        bytes
    }

    fn write_selector(&mut self, selector: u16) {
        debug_assert!(SELECTOR_REGISTER + core::mem::size_of::<u16>() <= self.size);
        unsafe {
            ptr::write_volatile(
                (self.base + SELECTOR_REGISTER) as *mut u16,
                selector.to_be(),
            );
        }
        dma::complete_mmio_write();
    }

    fn dma_transfer(
        &mut self,
        control: u32,
        physical_address: usize,
        length: usize,
    ) -> Result<(), FwCfgError> {
        if length == 0 {
            return Err(FwCfgError::EmptyTransfer);
        }
        let length = u32::try_from(length).map_err(|_| FwCfgError::TransferTooLarge)?;
        physical_address
            .checked_add(length as usize)
            .ok_or(FwCfgError::AddressOverflow)?;
        let physical_address =
            u64::try_from(physical_address).map_err(|_| FwCfgError::AddressOverflow)?;
        let mut access = DmaAccess::new(control, length, physical_address);
        let access_address = ptr::addr_of_mut!(access) as usize;
        access_address
            .checked_add(core::mem::size_of::<DmaAccess>())
            .ok_or(FwCfgError::AddressOverflow)?;

        dma::publish_to_device();
        debug_assert!(DMA_ADDRESS_REGISTER + core::mem::size_of::<u64>() <= self.size);
        unsafe {
            ptr::write_volatile(
                (self.base + DMA_ADDRESS_REGISTER) as *mut u64,
                (access_address as u64).to_be(),
            );
        }
        dma::complete_mmio_write();

        for _ in 0..DMA_SPIN_LIMIT {
            dma::acquire_from_device();
            let raw = unsafe { ptr::read_volatile(ptr::addr_of!(access.control_be)) };
            let result = u32::from_be(raw);
            if result == 0 {
                self.dma_operations = self.dma_operations.saturating_add(1);
                return Ok(());
            }
            if result & DMA_CONTROL_ERROR != 0 {
                return Err(FwCfgError::DmaRejected);
            }
            spin_loop();
        }
        Err(FwCfgError::DmaTimedOut)
    }
}

const _: () = assert!(DATA_REGISTER == 0);
