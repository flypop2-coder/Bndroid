//! Native property endpoint. Only the current process's Init-issued key grants
//! authorize writes; neither a claimed UID nor a transferred socket does so.
use crate::{linux_runtime as runtime, process, userboot};
use alloc::vec::Vec;
use bndroid_kernel::{linux_fd::FileRef, linux_fs::Path, linux_properties as abi};
use core::{
    cell::UnsafeCell,
    sync::atomic::{Ordering, fence},
};
struct Area(UnsafeCell<Option<FileRef>>);
unsafe impl Sync for Area {}
static AREA: Area = Area(UnsafeCell::new(None));
fn slot() -> &'static mut Option<FileRef> {
    assert!(crate::arch::aarch64::irq_is_masked());
    unsafe { &mut *AREA.0.get() }
}
pub fn open() -> Result<FileRef, i64> {
    if slot().is_none() {
        let node =
            crate::linux_rootfs::resolve(&Path::normalize("", "/dev/__properties__")?, true)?;
        if node.data.len() != abi::AREA_BYTES {
            return Err(22);
        }
        let file = FileRef::try_properties()?;
        let memory = file.memory().unwrap();
        let prepared = memory.prepare_resize(abi::AREA_BYTES as u64)?;
        // New backing has no mappings or external references.
        unsafe {
            memory.commit_resize(prepared);
        }
        memory.write(0, node.data)?;
        *slot() = Some(file);
    }
    Ok(slot().as_ref().unwrap().clone())
}
pub fn reset() {
    *slot() = None;
}
fn set(name: &str, value: &str) -> Result<(), i64> {
    let profile = process::linux_security_profile(None)
        .and_then(userboot::linux_launch)
        .ok_or(13)?;
    if !bndr_sm::service_authority::may_set_property(profile.property_grants, name) {
        return Err(13);
    }
    let file = open()?;
    let memory = file.memory().unwrap();
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(abi::AREA_BYTES).map_err(|_| 12)?;
    bytes.resize(abi::AREA_BYTES, 0);
    memory.read(0, &mut bytes);
    let at = abi::update(&mut bytes, name, value)?;
    memory.write(148, &bytes[148..240])?;
    fence(Ordering::Release);
    let mut old = [0; 4];
    memory.read(at as u64, &mut old);
    memory.write(at as u64, &(u32::from_le_bytes(old) | 1).to_le_bytes())?;
    memory.write(at as u64 + 4, &bytes[at + 4..at + 96])?;
    fence(Ordering::Release);
    memory.write(at as u64, &bytes[at..at + 4])?;
    memory.write(4, &bytes[4..8])?;
    fence(Ordering::Release);
    crate::scheduler::linux_threads::wake_shared(memory.inode(), at as u64);
    crate::scheduler::linux_threads::wake_shared(memory.inode(), 4);
    crate::kprintln!(
        "LINUX_PROPERTY_SET name={} bytes={} profile={}",
        name,
        value.len(),
        process::linux_security_profile(None).unwrap()
    );
    Ok(())
}
pub fn write(file: &FileRef, bytes: &[u8]) -> Result<u64, i64> {
    file.control().with(|state| {
        if !state.connected {
            return Err(107);
        }
        if state.response.is_some() {
            return Err(32);
        }
        if state.request.len() + bytes.len() > 256 {
            return Err(90);
        }
        state.request.try_reserve(bytes.len()).map_err(|_| 12)?;
        state.request.extend_from_slice(bytes);
        let result = match abi::request(&state.request) {
            Ok(None) => return Ok(bytes.len() as u64),
            Ok(Some((name, value))) => set(name, value),
            Err(error) => Err(error),
        };
        let code: u32 = match result {
            Ok(()) => 0,
            Err(13) => 0x18,
            Err(_) => 0x1b,
        };
        state.response = Some(code.to_le_bytes().to_vec());
        Ok(bytes.len() as u64)
    })
}
pub fn recv(file: &FileRef, address: u64, count: u64, flags: u64) -> Result<u64, i64> {
    if flags & !(0x100 | 0x40 | 2) != 0 {
        return Err(95);
    }
    file.control().with(|state| {
        if !state.connected {
            return Err(107);
        }
        if count == 0 {
            return Ok(0);
        }
        let bytes = state.response.as_ref().ok_or(11)?;
        let n = (count as usize).min(bytes.len() - state.read);
        let result = runtime::copy_out(address, &bytes[state.read..state.read + n]) as i64;
        if result < 0 {
            return Err(-result);
        }
        if flags & 2 == 0 {
            state.read += n;
        }
        Ok(n as u64)
    })
}
