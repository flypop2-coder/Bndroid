//! Linux arm64 syscall personality for a separately loaded native process.
//! Unsupported calls return -ENOSYS. They never fall through to Bndroid IPC.

use crate::arch::aarch64::{self, exceptions::TrapFrame, usercopy};
use bndr_abi::UserImageId;
use bndroid_kernel::linux_abi::{EBADF, EFAULT, EINVAL, ENOSYS, errno, monotonic_timespec};
use core::sync::atomic::{AtomicU64, Ordering};

static CALLS: AtomicU64 = AtomicU64::new(0);
static UNKNOWN: AtomicU64 = AtomicU64::new(0);
static BAD_ADDRESS: AtomicU64 = AtomicU64::new(0);
static PRIVATE_SVC: AtomicU64 = AtomicU64::new(0);
static WRITTEN: AtomicU64 = AtomicU64::new(0);
static EXITS: AtomicU64 = AtomicU64::new(0);

pub fn dispatch(frame: *mut TrapFrame) -> *mut TrapFrame {
    assert_eq!(
        crate::process::current_live_user_image_id(),
        Some(UserImageId::LinuxApp)
    );
    assert!(aarch64::irq_is_masked());
    assert!(crate::process::with_current_handles(
        |table| table.is_empty()
    ));
    CALLS.fetch_add(1, Ordering::Relaxed);
    // SAFETY: the common SVC path validated this process's saved TrapFrame.
    let registers = unsafe { (*frame).x };
    let result = match registers[8] {
        64 => write(registers[0], registers[1], registers[2]),
        93 | 94 => {
            let code = registers[0] & 255;
            // Exercise accounting while the native image is still alive,
            // independently of when the monitor next gets a time slice.
            let snapshot = crate::process::snapshot();
            assert_eq!(snapshot.linux_app_live, 1);
            assert_eq!(snapshot.linux_app_handles, 0);
            EXITS.fetch_add(1, Ordering::Release);
            crate::kprintln!(
                "LINUX_PROCESS_EXIT pid={} code={} native_handles=0",
                crate::process::current_live_user_process_id().unwrap(),
                code
            );
            return crate::scheduler::exit_current_user(frame, code);
        }
        113 => clock_gettime(registers[0], registers[1]),
        160 => uname(registers[0]),
        172 | 178 => crate::process::current_live_user_process_id().unwrap() & 0xffff_ffff,
        173 => 1,           // This entry profile is spawned only by Init.
        174..=177 => 10000, // One application principal in this first profile.
        number => {
            UNKNOWN.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "LINUX_SYSCALL_UNSUPPORTED pid={} number={}",
                crate::process::current_live_user_process_id().unwrap(),
                number
            );
            errno(ENOSYS)
        }
    };
    complete(frame, result)
}

pub fn reject_svc(frame: *mut TrapFrame) -> *mut TrapFrame {
    PRIVATE_SVC.fetch_add(1, Ordering::Relaxed);
    complete(frame, errno(ENOSYS))
}

fn complete(frame: *mut TrapFrame, value: u64) -> *mut TrapFrame {
    // Linux returns just x0. In particular do not clear x1/x2 as native IPC does.
    unsafe {
        (*frame).x[0] = value;
    }
    frame
}

fn bad_address() -> u64 {
    BAD_ADDRESS.fetch_add(1, Ordering::Relaxed);
    errno(EFAULT)
}

fn write(descriptor: u64, address: u64, requested: u64) -> u64 {
    if descriptor != 1 && descriptor != 2 {
        return errno(EBADF);
    }
    // A positive short write is legal and bounds IRQ-masked console work.
    let count = requested.min(256) as usize;
    if count == 0 {
        return 0;
    }
    if !crate::process::current_user_range_is_readable(address, count) {
        return bad_address();
    }
    let mut bytes = [0; 256];
    if usercopy::copy_from_user(&mut bytes[..count], address).is_err() {
        return bad_address();
    }
    crate::uart::Uart.write_bytes(&bytes[..count]);
    WRITTEN.fetch_add(count as u64, Ordering::Relaxed);
    count as u64
}

fn copy_out(address: u64, bytes: &[u8]) -> u64 {
    if !crate::process::current_user_range_is_writable(address, bytes.len())
        || usercopy::copy_to_user(address, bytes).is_err()
    {
        return bad_address();
    }
    0
}

fn clock_gettime(clock: u64, address: u64) -> u64 {
    if clock != 1 {
        return errno(EINVAL);
    } // CLOCK_MONOTONIC
    let Some(bytes) = monotonic_timespec(
        aarch64::timer::counter_value(),
        aarch64::timer::frequency_hz(),
    ) else {
        return errno(EINVAL);
    };
    copy_out(address, &bytes)
}

fn uname(address: u64) -> u64 {
    let mut bytes = [0; 65 * 6];
    for (index, value) in [
        "Bndroid",
        "bndroid",
        "0.1.0",
        "Linux ABI compatibility",
        "aarch64",
        "",
    ]
    .iter()
    .enumerate()
    {
        bytes[index * 65..index * 65 + value.len()].copy_from_slice(value.as_bytes());
    }
    copy_out(address, &bytes)
}

pub fn monitor() -> ! {
    let start = aarch64::timer::tick_count();
    loop {
        crate::process::reap_terminal_children();
        let syscalls = crate::syscall::snapshot();
        let processes = crate::process::snapshot();
        if syscalls.failure != 0 || crate::scheduler::user_init_fault_snapshot().count != 0 {
            panic!("LINUX_RUNTIME_FAIL init_failure={}", syscalls.failure);
        }
        if syscalls.ready {
            assert_eq!(processes.created, 3);
            assert_eq!(processes.reaped, 2);
            assert_eq!(processes.terminated_exited, 2);
            assert_eq!(processes.terminated_faulted, 0);
            assert_eq!(processes.live, 1);
            assert_eq!(processes.linux_app_live, 0);
            assert_eq!(processes.linux_app_handles, 0);
            assert_eq!(processes.total_handles, 0);
            assert_eq!(syscalls.channels_created, 2);
            assert_eq!(EXITS.load(Ordering::Acquire), 2);
            assert_eq!(UNKNOWN.load(Ordering::Relaxed), 8);
            assert_eq!(PRIVATE_SVC.load(Ordering::Relaxed), 2);
            assert_eq!(BAD_ADDRESS.load(Ordering::Relaxed), 10);
            crate::kprintln!(
                "LINUX_ABI_RUNTIME_OK abi=70 native_processes=2 native_exits=2 native_reaped=2 remaining_processes=1 native_handles=0 supervisor_channels=2 linux_calls={} unsupported=8 private_svc_rejected=2 bad_address=10 console_bytes={} stack=argc-argv-envp-auxv loader=ndk-static-elf linux_kernel=0 bionic_startup=0 art=0",
                CALLS.load(Ordering::Relaxed),
                WRITTEN.load(Ordering::Relaxed)
            );
            loop {
                aarch64::wait_for_interrupt();
            }
        }
        if aarch64::timer::tick_count().wrapping_sub(start) > 3000 {
            panic!(
                "LINUX_RUNTIME_TIMEOUT calls={} created={} live={} reaped={}",
                CALLS.load(Ordering::Relaxed),
                processes.created,
                processes.live,
                processes.reaped
            );
        }
        aarch64::wait_for_interrupt();
    }
}
