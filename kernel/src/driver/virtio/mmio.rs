use core::ptr::{read_volatile, write_volatile};

use crate::arch::aarch64::dma;

pub const MAGIC_VALUE: usize = 0x000;
pub const VERSION: usize = 0x004;
pub const DEVICE_ID: usize = 0x008;
const DEVICE_FEATURES: usize = 0x010;
const DEVICE_FEATURES_SELECT: usize = 0x014;
const DRIVER_FEATURES: usize = 0x020;
const DRIVER_FEATURES_SELECT: usize = 0x024;
const QUEUE_SELECT: usize = 0x030;
const QUEUE_SIZE_MAX: usize = 0x034;
const QUEUE_SIZE: usize = 0x038;
const QUEUE_READY: usize = 0x044;
const QUEUE_NOTIFY: usize = 0x050;
const INTERRUPT_STATUS: usize = 0x060;
const INTERRUPT_ACK: usize = 0x064;
const STATUS: usize = 0x070;
const QUEUE_DESCRIPTOR_LOW: usize = 0x080;
const QUEUE_DRIVER_LOW: usize = 0x090;
const QUEUE_DEVICE_LOW: usize = 0x0a0;
pub const CONFIG_GENERATION: usize = 0x0fc;
pub const CONFIG: usize = 0x100;
const BLOCK_CAPACITY_LOW: usize = CONFIG;
const BLOCK_CAPACITY_HIGH: usize = CONFIG + 4;

pub const MIN_REGION_SIZE: usize = BLOCK_CAPACITY_HIGH + 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MmioError {
    MisalignedBase,
    RegionTooSmall,
    AddressOverflow,
    ConfigOffsetOutOfBounds,
}

impl MmioError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MisalignedBase => "virtio MMIO base is not 32-bit aligned",
            Self::RegionTooSmall => "virtio MMIO region is too small",
            Self::AddressOverflow => "virtio MMIO region address overflow",
            Self::ConfigOffsetOutOfBounds => "virtio MMIO config offset is outside the region",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header {
    pub magic: u32,
    pub version: u32,
    pub device_id: u32,
}

/// A checked coordinate for the standardized virtio-mmio register window.
///
/// It is intentionally neither `Clone` nor `Copy`: one block driver uniquely
/// owns all status and queue programming for a transport.
pub struct MmioTransport {
    base: usize,
    size: usize,
}

impl MmioTransport {
    /// # Safety
    ///
    /// `base..base+size` must remain mapped as Device memory and identify one
    /// uniquely owned virtio-mmio register window for the lifetime of this
    /// value. No other code may mutate its status or queue registers.
    pub unsafe fn new(base: usize, size: usize) -> Result<Self, MmioError> {
        if !base.is_multiple_of(core::mem::align_of::<u32>()) {
            return Err(MmioError::MisalignedBase);
        }
        if size < MIN_REGION_SIZE {
            return Err(MmioError::RegionTooSmall);
        }
        base.checked_add(size).ok_or(MmioError::AddressOverflow)?;
        Ok(Self { base, size })
    }

    pub fn header(&self) -> Header {
        Header {
            magic: self.read32(MAGIC_VALUE),
            version: self.read32(VERSION),
            device_id: self.read32(DEVICE_ID),
        }
    }

    pub fn status(&self) -> u32 {
        self.read32(STATUS)
    }

    pub fn write_status(&mut self, status: u32) {
        self.write32(STATUS, status);
        dma::complete_mmio_write();
    }

    pub fn add_status(&mut self, status: u32) -> u32 {
        let next = self.status() | status;
        self.write_status(next);
        next
    }

    pub fn device_features(&mut self) -> u64 {
        self.write32(DEVICE_FEATURES_SELECT, 0);
        let low = self.read32(DEVICE_FEATURES);
        self.write32(DEVICE_FEATURES_SELECT, 1);
        let high = self.read32(DEVICE_FEATURES);
        u64::from(low) | (u64::from(high) << 32)
    }

    pub fn set_driver_features(&mut self, features: u64) {
        self.write32(DRIVER_FEATURES_SELECT, 0);
        self.write32(DRIVER_FEATURES, features as u32);
        self.write32(DRIVER_FEATURES_SELECT, 1);
        self.write32(DRIVER_FEATURES, (features >> 32) as u32);
        dma::complete_mmio_write();
    }

    pub fn select_queue(&mut self, index: u32) {
        self.write32(QUEUE_SELECT, index);
    }

    pub fn queue_size_max(&self) -> u32 {
        self.read32(QUEUE_SIZE_MAX)
    }

    pub fn queue_ready(&self) -> bool {
        self.read32(QUEUE_READY) != 0
    }

    pub fn configure_queue(
        &mut self,
        size: u16,
        descriptor_address: u64,
        available_address: u64,
        used_address: u64,
    ) {
        self.write32(QUEUE_SIZE, u32::from(size));
        self.write_address(QUEUE_DESCRIPTOR_LOW, descriptor_address);
        self.write_address(QUEUE_DRIVER_LOW, available_address);
        self.write_address(QUEUE_DEVICE_LOW, used_address);
        dma::publish_to_device();
        self.write32(QUEUE_READY, 1);
        dma::complete_mmio_write();
    }

    pub fn notify_queue(&mut self, index: u32) {
        self.write32(QUEUE_NOTIFY, index);
        dma::complete_mmio_write();
    }

    pub fn interrupt_status(&self) -> u32 {
        self.read32(INTERRUPT_STATUS)
    }

    pub fn acknowledge_interrupt(&mut self, bits: u32) {
        self.write32(INTERRUPT_ACK, bits);
        dma::complete_mmio_write();
    }

    pub fn config_generation(&self) -> u32 {
        self.read32(CONFIG_GENERATION)
    }

    pub fn block_capacity_once(&self) -> u64 {
        let low = self.read32(BLOCK_CAPACITY_LOW);
        let high = self.read32(BLOCK_CAPACITY_HIGH);
        u64::from(low) | (u64::from(high) << 32)
    }

    pub fn read_config_u8(&self, offset: usize) -> Result<u8, MmioError> {
        let address = self
            .base
            .checked_add(CONFIG)
            .and_then(|address| address.checked_add(offset))
            .ok_or(MmioError::AddressOverflow)?;
        let end = CONFIG
            .checked_add(offset)
            .and_then(|offset| offset.checked_add(1))
            .ok_or(MmioError::AddressOverflow)?;
        if end > self.size {
            return Err(MmioError::ConfigOffsetOutOfBounds);
        }
        Ok(unsafe { read_volatile(address as *const u8) })
    }

    pub fn write_config_u8(&mut self, offset: usize, value: u8) -> Result<(), MmioError> {
        let address = self
            .base
            .checked_add(CONFIG)
            .and_then(|address| address.checked_add(offset))
            .ok_or(MmioError::AddressOverflow)?;
        let end = CONFIG
            .checked_add(offset)
            .and_then(|offset| offset.checked_add(1))
            .ok_or(MmioError::AddressOverflow)?;
        if end > self.size {
            return Err(MmioError::ConfigOffsetOutOfBounds);
        }
        unsafe {
            write_volatile(address as *mut u8, value);
        }
        dma::complete_mmio_write();
        Ok(())
    }

    fn write_address(&mut self, low_offset: usize, address: u64) {
        self.write32(low_offset, address as u32);
        self.write32(low_offset + 4, (address >> 32) as u32);
    }

    fn read32(&self, offset: usize) -> u32 {
        debug_assert!(offset + size_of::<u32>() <= self.size);
        unsafe { read_volatile((self.base + offset) as *const u32) }
    }

    fn write32(&mut self, offset: usize, value: u32) {
        debug_assert!(offset + size_of::<u32>() <= self.size);
        unsafe {
            write_volatile((self.base + offset) as *mut u32, value);
        }
    }
}

const fn size_of<T>() -> usize {
    core::mem::size_of::<T>()
}
