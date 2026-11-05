//! Finite, host-supplied entropy for the local QEMU runtime. Bytes are consumed
//! once and scrubbed; exhaustion fails instead of recycling an initialization seed.
use crate::driver::fw_cfg::FwCfgMmio;
use bndroid_kernel::{
    fdt::FwCfgMmioRegion,
    fw_cfg::{
        FILE_DIRECTORY_HEADER_BYTES, FILE_DIRECTORY_SELECTOR, FILE_ENTRY_BYTES, directory_count,
        find_unique_file,
    },
};
use core::{
    cell::UnsafeCell,
    ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

pub const POOL_BYTES: usize = 65536;
const DIRECTORY_BYTES: usize = FILE_DIRECTORY_HEADER_BYTES + 64 * FILE_ENTRY_BYTES;
#[repr(align(4096))]
struct Storage(UnsafeCell<[u8; POOL_BYTES]>);
// Before READY, only boot initialization writes. Afterwards atomic reservation
// grants each consumer a disjoint slice, which is never made available again.
unsafe impl Sync for Storage {}
static POOL: Storage = Storage(UnsafeCell::new([0; POOL_BYTES]));
static READY: AtomicBool = AtomicBool::new(false);
static USED: AtomicUsize = AtomicUsize::new(0);

pub fn init(region: Option<FwCfgMmioRegion>) -> Result<(), &'static str> {
    if READY.load(Ordering::Acquire) {
        return Err("entropy already initialized");
    }
    let region = region.ok_or("entropy requires fw_cfg")?;
    // Boot initialization owns the transport after display setup, before any
    // userspace is runnable. This is not a userspace-accessible firmware file.
    let mut transport =
        unsafe { FwCfgMmio::probe(region) }.map_err(|_| "entropy transport failed")?;
    let mut directory = [0_u8; DIRECTORY_BYTES];
    transport
        .dma_read(
            FILE_DIRECTORY_SELECTOR,
            directory.as_mut_ptr() as usize,
            FILE_DIRECTORY_HEADER_BYTES,
        )
        .map_err(|_| "entropy directory read failed")?;
    let count = directory_count(&directory[..FILE_DIRECTORY_HEADER_BYTES])
        .map_err(|_| "entropy directory invalid")? as usize;
    if count > 64 {
        return Err("entropy directory too large");
    }
    let bytes = FILE_DIRECTORY_HEADER_BYTES + count * FILE_ENTRY_BYTES;
    transport
        .dma_read(
            FILE_DIRECTORY_SELECTOR,
            directory.as_mut_ptr() as usize,
            bytes,
        )
        .map_err(|_| "entropy directory read failed")?;
    let file = find_unique_file(&directory[..bytes], b"opt/bndroid/entropy")
        .map_err(|_| "entropy directory ambiguous")?
        .ok_or("missing host entropy input")?;
    if file.size as usize != POOL_BYTES {
        return Err("entropy input must contain 65536 bytes");
    }
    transport
        .dma_read(
            file.selector,
            POOL.0.get().cast::<u8>() as usize,
            POOL_BYTES,
        )
        .map_err(|_| "entropy input read failed")?;
    READY.store(true, Ordering::Release);
    crate::kprintln!(
        "LINUX_ENTROPY_OK source=host-fw-cfg bytes={} reuse=0",
        POOL_BYTES
    );
    Ok(())
}

pub fn fill(output: &mut [u8]) -> Result<(), ()> {
    if !READY.load(Ordering::Acquire) {
        return Err(());
    }
    let start = USED
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
            used.checked_add(output.len())
                .filter(|end| *end <= POOL_BYTES)
        })
        .map_err(|_| ())?;
    for (index, byte) in output.iter_mut().enumerate() {
        let source = unsafe { POOL.0.get().cast::<u8>().add(start + index) };
        *byte = unsafe { ptr::read_volatile(source) };
        unsafe {
            ptr::write_volatile(source, 0);
        }
    }
    Ok(())
}

pub fn consumed() -> usize {
    USED.load(Ordering::Acquire)
}
