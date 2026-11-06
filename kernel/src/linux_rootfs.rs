//! Immutable, explicitly supplied runtime files. The host archive is read once
//! before userspace; lookup never reaches host paths or a writable guest disk.
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
    sync::atomic::{AtomicBool, Ordering},
};

use bndroid_kernel::linux_fs::{Archive, Kind, Node, Path};
const ROOTFS_BYTES: usize = if cfg!(feature = "framework-runtime") {
    320 * 1024 * 1024
} else if cfg!(feature = "art-runtime") {
    96 * 1024 * 1024
} else {
    8 * 1024 * 1024
};
const DIRECTORY_BYTES: usize = FILE_DIRECTORY_HEADER_BYTES + 64 * FILE_ENTRY_BYTES;
#[repr(align(4096))]
struct Storage(UnsafeCell<[u8; ROOTFS_BYTES]>);
unsafe impl Sync for Storage {}
static STORAGE: Storage = Storage(UnsafeCell::new([0; ROOTFS_BYTES]));
struct Catalog(UnsafeCell<Option<Archive<'static>>>);
unsafe impl Sync for Catalog {}
static CATALOG: Catalog = Catalog(UnsafeCell::new(None));
static READY: AtomicBool = AtomicBool::new(false);

pub fn init(region: Option<FwCfgMmioRegion>) -> Result<(), &'static str> {
    if READY.load(Ordering::Acquire) {
        return Err("rootfs already initialized");
    }
    let mut transport = unsafe { FwCfgMmio::probe(region.ok_or("rootfs requires fw_cfg")?) }
        .map_err(|_| "rootfs transport failed")?;
    let mut directory = [0; DIRECTORY_BYTES];
    transport
        .dma_read(
            FILE_DIRECTORY_SELECTOR,
            directory.as_mut_ptr() as usize,
            FILE_DIRECTORY_HEADER_BYTES,
        )
        .map_err(|_| "rootfs directory read failed")?;
    let count = directory_count(&directory[..FILE_DIRECTORY_HEADER_BYTES])
        .map_err(|_| "invalid directory")? as usize;
    if count > 64 {
        return Err("rootfs directory too large");
    }
    let length = FILE_DIRECTORY_HEADER_BYTES + count * FILE_ENTRY_BYTES;
    transport
        .dma_read(
            FILE_DIRECTORY_SELECTOR,
            directory.as_mut_ptr() as usize,
            length,
        )
        .map_err(|_| "rootfs directory read failed")?;
    let file = find_unique_file(&directory[..length], b"opt/bndroid/rootfs")
        .map_err(|_| "ambiguous rootfs input")?
        .ok_or("missing runtime rootfs input")?;
    if file.size as usize > ROOTFS_BYTES {
        return Err("runtime rootfs exceeds profile byte limit");
    }
    transport
        .dma_read(
            file.selector,
            STORAGE.0.get().cast::<u8>() as usize,
            file.size as usize,
        )
        .map_err(|_| "rootfs read failed")?;
    // Initialized exactly once before READY; the archive and file data are
    // immutable for all remaining kernel and guest lifetimes.
    let bytes =
        unsafe { core::slice::from_raw_parts(STORAGE.0.get().cast::<u8>(), file.size as usize) };
    let mut catalog = Archive::parse(bytes).map_err(|_| "invalid runtime USTAR archive")?;
    catalog
        .insert(Node {
            path: "apex/com.android.runtime/bin/linker64",
            data: crate::user_images::get(bndr_abi::UserImageId::LinuxApp).bytes,
            kind: Kind::File,
            mode: 0o555,
            inode: 2,
        })
        .map_err(|_| "duplicate interpreter")?;
    for (index, path) in ["dev/null", "dev/urandom"].into_iter().enumerate() {
        catalog
            .insert(Node {
                path,
                data: &[],
                kind: Kind::File,
                mode: 0o666,
                inode: 3 + index as u64,
            })
            .map_err(|_| "reserved device path")?;
    }
    #[cfg(feature = "art-runtime")]
    for (index, path) in ["proc", "proc/self", "proc/self/stat", "proc/self/maps"]
        .into_iter()
        .enumerate()
    {
        catalog
            .insert(Node {
                path,
                data: &[],
                kind: if index < 2 {
                    Kind::Directory
                } else {
                    Kind::File
                },
                mode: if index < 2 { 0o555 } else { 0o444 },
                inode: 0x10000 + index as u64,
            })
            .map_err(|_| "reserved proc path")?;
    }
    let files = catalog.len();
    unsafe {
        *CATALOG.0.get() = Some(catalog);
    }
    READY.store(true, Ordering::Release);
    crate::kprintln!(
        "LINUX_ROOTFS_OK files={} bytes={} writable=0",
        files,
        file.size
    );
    Ok(())
}

pub fn resolve(path: &Path, follow: bool) -> Result<Node<'static>, i64> {
    assert!(READY.load(Ordering::Acquire));
    unsafe { (*CATALOG.0.get()).as_ref().unwrap() }.resolve(path, follow)
}

pub fn usage() -> (usize, usize) {
    assert!(READY.load(Ordering::Acquire));
    let catalog = unsafe { (*CATALOG.0.get()).as_ref().unwrap() };
    (catalog.file_bytes(), catalog.len())
}
