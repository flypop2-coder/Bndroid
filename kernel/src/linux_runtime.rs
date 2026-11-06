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
#[cfg(feature = "linux-syscall-trace")]
static ABORT_MAPPING: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "linux-syscall-trace")]
static ABORT_OWNER: AtomicU64 = AtomicU64::new(0);

#[cfg_attr(feature = "art-runtime", inline(never))]
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
    #[cfg(feature = "linux-syscall-trace")]
    trace_abort(&registers);
    #[cfg(feature = "bionic-dynamic-runtime")]
    if let Some(next) = crate::scheduler::linux_threads::dispatch(frame, &registers) {
        return next;
    }
    #[cfg(feature = "bionic-dynamic-runtime")]
    if let Some(result) = crate::linux_files::dispatch(&registers) {
        return complete(frame, result);
    }
    let result = match registers[8] {
        25 => fcntl(registers[0], registers[1], registers[2]),
        57 => {
            if crate::process::linux_update_stream(registers[0], None) {
                0
            } else {
                errno(EBADF)
            }
        }
        // The supervised launch supplies /dev/null semantics on stdin.
        63 => {
            if registers[0] == 0 && crate::process::linux_stream_flags(0).is_some() {
                0
            } else {
                errno(EBADF)
            }
        }
        64 => write(registers[0], registers[1], registers[2]),
        66 => writev(registers[0], registers[1], registers[2]),
        96 => crate::process::linux_set_tid_address(user_address(registers[0])),
        278 => getrandom(registers[0], registers[1], registers[2]),
        #[cfg(feature = "art-runtime")]
        216 => {
            crate::process::with_current_linux_address_space(|space| space.linux_mremap(&registers))
                .unwrap_or_else(errno)
        }
        #[cfg(feature = "art-runtime")]
        227 => crate::process::with_current_linux_address_space(|space| {
            space.linux_msync(registers[0], registers[1], registers[2])
        })
        .unwrap_or_else(errno),
        215 | 222 | 226 => {
            let result =
                crate::process::with_current_linux_address_space(|space| match registers[8] {
                    215 => space.linux_munmap(registers[0], registers[1]),
                    222 => space.linux_mmap(registers[..6].try_into().unwrap()),
                    _ => space.linux_mprotect(registers[0], registers[1], registers[2]),
                });
            result.unwrap_or_else(errno)
        }
        93 | 94 => {
            let code = registers[0] & 255;
            crate::process::linux_clear_tid_on_exit();
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
        120 | 121 => {
            let pid = registers[0] as u32;
            let own = crate::process::current_live_user_process_id().unwrap() as u32;
            if pid != 0 && pid != own {
                errno(3)
            } else if registers[8] == 120 {
                0
            }
            // SCHED_OTHER, normal time sharing.
            else {
                copy_out(registers[1], &[0; 4])
            } // sched_priority = 0.
        }
        160 => uname(registers[0]),
        #[cfg(feature = "art-runtime")]
        169 => gettimeofday(registers[0], registers[1]),
        163 => getrlimit(registers[0], registers[1]),
        167 => prctl(&registers),
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

#[cfg(feature = "linux-syscall-trace")]
fn trace_abort(registers: &[u64; 31]) {
    // Bionic publishes its fatal message in a named private mapping. This
    // opt-in debugger observes it without implementing PR_SET_VMA or changing
    // guest behavior. Bound every read and retain process-generation ownership.
    let owner = crate::process::current_live_user_process_id().unwrap();
    if registers[8] == 167 && registers[0] == 0x53564d41 && registers[1] == 0 {
        let name = user_address(registers[4]);
        let mut bytes = [0; 14];
        if crate::process::current_user_range_is_readable(name, bytes.len())
            && usercopy::copy_from_user(&mut bytes, name).is_ok()
            && &bytes == b"abort message\0"
        {
            ABORT_MAPPING.store(user_address(registers[2]), Ordering::Relaxed);
            ABORT_OWNER.store(owner, Ordering::Relaxed);
        }
    }
    if matches!(registers[8], 93 | 94 | 131 | 240) && ABORT_OWNER.load(Ordering::Relaxed) == owner {
        let address = ABORT_MAPPING.swap(0, Ordering::Relaxed);
        if address == 0 {
            return;
        }
        let mut bytes = [0; 256];
        if crate::process::current_user_range_is_readable(address, bytes.len())
            && usercopy::copy_from_user(&mut bytes, address).is_ok()
            && u64::from_le_bytes(bytes[..8].try_into().unwrap()) == 0xb18e40886ac388f0
            && u64::from_le_bytes(bytes[8..16].try_into().unwrap()) == 0xc6dfba755a1de0b5
            && u64::from_le_bytes(bytes[16..24].try_into().unwrap()) >= 25
        {
            // Skip the magic and size fields. The message itself is an ASCII
            // diagnostic in current Bionic, truncated safely at this boundary.
            let end = bytes[24..]
                .iter()
                .position(|b| *b == 0)
                .map_or(bytes.len(), |n| n + 24);
            if let Ok(message) = core::str::from_utf8(&bytes[24..end]) {
                crate::kprintln!("LINUX_ABORT_MESSAGE pid={} message={}", owner, message);
            }
        }
    }
}

pub fn reject_svc(frame: *mut TrapFrame) -> *mut TrapFrame {
    PRIVATE_SVC.fetch_add(1, Ordering::Relaxed);
    complete(frame, errno(ENOSYS))
}

pub(crate) fn complete(frame: *mut TrapFrame, value: u64) -> *mut TrapFrame {
    #[cfg(feature = "linux-syscall-trace")]
    unsafe {
        if value == errno(12) && matches!((*frame).x[8], 215 | 216 | 222 | 226) {
            let (pages, tables, regions) =
                crate::process::with_current_linux_address_space(|space| {
                    space.linux_mapping_usage()
                });
            crate::kprintln!(
                "LINUX_MEMORY_LIMIT pages={} tables={} regions={} heap_free={}",
                pages,
                tables,
                regions,
                crate::kernel_heap::stats().unwrap().free_bytes
            );
        }
        crate::kprintln!(
            "LINUX_SYSCALL_TRACE nr={} args={:#x},{:#x},{:#x},{:#x},{:#x},{:#x} result={:#x}",
            (*frame).x[8],
            (*frame).x[0],
            (*frame).x[1],
            (*frame).x[2],
            (*frame).x[3],
            (*frame).x[4],
            (*frame).x[5],
            value
        );
    }
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
    let address = user_address(address);
    if !matches!(descriptor, 1 | 2) || crate::process::linux_stream_flags(descriptor).is_none() {
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

pub(crate) fn copy_out(address: u64, bytes: &[u8]) -> u64 {
    let address = user_address(address);
    if !crate::process::current_user_range_is_writable(address, bytes.len()) {
        return bad_address();
    }
    // Linux regular-file reads can exceed the native IPC copy bound. Keep the
    // low-level fault-recoverable copy contract and validate the whole target
    // before copying it in bounded chunks under this IRQ-masked syscall.
    for (index, chunk) in bytes.chunks(usercopy::MAX_USER_COPY_BYTES).enumerate() {
        if usercopy::copy_to_user(
            address + (index * usercopy::MAX_USER_COPY_BYTES) as u64,
            chunk,
        )
        .is_err()
        {
            return bad_address();
        }
    }
    0
}

fn writev(descriptor: u64, address: u64, count: u64) -> u64 {
    let address = user_address(address);
    if !matches!(descriptor, 1 | 2) || crate::process::linux_stream_flags(descriptor).is_none() {
        return errno(EBADF);
    }
    if count > 64 {
        return errno(EINVAL);
    }
    if !crate::process::current_user_range_is_readable(address, count as usize * 16) {
        return bad_address();
    }
    let mut total = 0;
    for index in 0..count {
        let mut iovec = [0; 16];
        if usercopy::copy_from_user(&mut iovec, address + index * 16).is_err() {
            return if total == 0 { bad_address() } else { total };
        }
        let base = u64::from_le_bytes(iovec[..8].try_into().unwrap());
        let requested = u64::from_le_bytes(iovec[8..].try_into().unwrap());
        let written = write(descriptor, base, requested.min(256 - total));
        if (written as i64) < 0 {
            return if total == 0 { written } else { total };
        }
        total += written;
        if written < requested || total == 256 {
            break;
        }
    }
    total
}

fn fcntl(descriptor: u64, command: u64, argument: u64) -> u64 {
    let Some(flags) = crate::process::linux_stream_flags(descriptor) else {
        return errno(EBADF);
    };
    match command {
        1 => flags, // F_GETFD
        2 if argument & !1 == 0 => {
            assert!(crate::process::linux_update_stream(
                descriptor,
                Some(argument)
            ));
            0
        }
        3 => u64::from(descriptor != 0), // F_GETFL: stdin RO, console outputs WO.
        _ => errno(EINVAL),
    }
}

pub(crate) fn getrandom(address: u64, requested: u64, flags: u64) -> u64 {
    let address = user_address(address);
    if flags & !3 != 0 {
        return errno(EINVAL);
    }
    let count = requested.min(256) as usize;
    if count == 0 {
        return 0;
    }
    if !crate::process::current_user_range_is_writable(address, count) {
        return bad_address();
    }
    let mut bytes = [0; 256];
    if crate::linux_entropy::fill(&mut bytes[..count]).is_err() {
        return errno(11);
    }
    let status = copy_out(address, &bytes[..count]);
    if status != 0 {
        return status;
    }
    count as u64
}

pub(crate) fn user_address(address: u64) -> u64 {
    if crate::process::linux_tagged_addresses(None) {
        address & 0x00ff_ffff_ffff_ffff
    } else {
        address
    }
}

fn prctl(registers: &[u64; 31]) -> u64 {
    match registers[0] {
        55 if registers[1] & !1 == 0 && registers[2..5] == [0; 3] => {
            crate::process::linux_tagged_addresses(Some(registers[1] != 0));
            0
        }
        56 if registers[1..5] == [0; 4] => u64::from(crate::process::linux_tagged_addresses(None)),
        _ => errno(EINVAL),
    }
}

#[cfg(feature = "art-runtime")]
pub fn realtime_anchor() -> bndroid_kernel::linux_sync::RealtimeAnchor {
    use core::sync::atomic::AtomicBool;
    static READY: AtomicBool = AtomicBool::new(false);
    static SECONDS: AtomicU64 = AtomicU64::new(0);
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    assert!(aarch64::irq_is_masked());
    // This profile has one CPU and no RTC setters. Sample once, then advance
    // from the same counter used for absolute wait deadlines.
    if !READY.load(Ordering::Relaxed) {
        let seconds = crate::platform::qemu_virt::read_pl031_unix_seconds();
        SECONDS.store(seconds, Ordering::Relaxed);
        COUNTER.store(aarch64::timer::counter_value(), Ordering::Relaxed);
        READY.store(true, Ordering::Relaxed);
        crate::kprintln!(
            "LINUX_REALTIME_SOURCE source=qemu-pl031 seconds={} initial_resolution_seconds=1",
            seconds
        );
    }
    bndroid_kernel::linux_sync::RealtimeAnchor {
        seconds: SECONDS.load(Ordering::Relaxed),
        counter: COUNTER.load(Ordering::Relaxed),
    }
}

fn clock_gettime(clock: u64, address: u64) -> u64 {
    #[cfg(feature = "art-runtime")]
    if matches!(clock, 2 | 3) {
        let counter = crate::scheduler::linux_threads::cpu_counter(clock == 3);
        return monotonic_timespec(counter, aarch64::timer::frequency_hz())
            .map_or_else(|| errno(EINVAL), |bytes| copy_out(address, &bytes));
    }
    #[cfg(feature = "art-runtime")]
    if matches!(clock, 0 | 5) {
        let anchor = realtime_anchor();
        let frequency = aarch64::timer::frequency_hz();
        let mut elapsed = aarch64::timer::counter_value().wrapping_sub(anchor.counter);
        if clock == 5 {
            let quantum = (frequency / 100).max(1);
            elapsed -= elapsed % quantum;
        }
        return anchor
            .timespec(anchor.counter.wrapping_add(elapsed), frequency)
            .map_or_else(|| errno(EINVAL), |bytes| copy_out(address, &bytes));
    }
    if !matches!(clock, 1 | 6) {
        return errno(EINVAL);
    } // CLOCK_MONOTONIC and CLOCK_MONOTONIC_COARSE.
    let frequency = aarch64::timer::frequency_hz();
    let mut counter = aarch64::timer::counter_value();
    if clock == 6 {
        let quantum = (frequency / 100).max(1);
        counter -= counter % quantum;
    }
    let Some(bytes) = monotonic_timespec(counter, frequency) else {
        return errno(EINVAL);
    };
    copy_out(address, &bytes)
}

#[cfg(feature = "art-runtime")]
fn gettimeofday(address: u64, zone: u64) -> u64 {
    for (pointer, size) in [(address, 16), (zone, 8)] {
        if pointer != 0
            && !crate::process::current_user_range_is_writable(user_address(pointer), size)
        {
            bad_address();
            return errno(EFAULT);
        }
    }
    if address != 0 {
        let anchor = realtime_anchor();
        let Some(mut bytes) = anchor.timespec(
            aarch64::timer::counter_value(),
            aarch64::timer::frequency_hz(),
        ) else {
            return errno(EINVAL);
        };
        let micros = u64::from_le_bytes(bytes[8..].try_into().unwrap()) / 1000;
        bytes[8..].copy_from_slice(&micros.to_le_bytes());
        let result = copy_out(address, &bytes);
        if result != 0 {
            return result;
        }
    }
    // The profile's kernel timezone is UTC, with no settimeofday interface.
    // This obsolete timezone argument is separate from userspace zoneinfo.
    if zone != 0 {
        copy_out(zone, &[0; 8])
    } else {
        0
    }
}

fn getrlimit(resource: u64, address: u64) -> u64 {
    let limit = match resource {
        3 => (crate::userboot::USER_STACK_END - crate::userboot::USER_STACK_START) as u64,
        #[cfg(feature = "bionic-dynamic-runtime")]
        7 => bndroid_kernel::linux_fd::CAPACITY as u64,
        #[cfg(not(feature = "bionic-dynamic-runtime"))]
        7 => 3,
        _ => return errno(EINVAL),
    };
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(&limit.to_le_bytes());
    bytes[8..].copy_from_slice(&limit.to_le_bytes());
    copy_out(address, &bytes)
}

fn uname(address: u64) -> u64 {
    let mut bytes = [0; 65 * 6];
    for (index, value) in [
        "Linux",
        "bndroid",
        "0.1.0-bndroid",
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
            let memory = aarch64::mmu::linux_memory_snapshot();
            #[cfg(feature = "art-runtime")]
            crate::kprintln!(
                "LINUX_PAGING_OK discarded={} demand_zero={}",
                memory.discarded_pages,
                memory.demand_zero_pages
            );
            #[cfg(feature = "bionic-dynamic-runtime")]
            crate::scheduler::linux_threads::evidence("LINUX_THREADS_FAILURE_CLEANUP");
            crate::kprintln!(
                "LINUX_RUNTIME_FAILURE_CLEANUP live_processes={} faulted={} reaped={} live_pages={} live_tables={} live_files={}",
                processes.live,
                processes.terminated_faulted,
                processes.reaped,
                memory.pages,
                memory.tables,
                bndroid_kernel::linux_fd::live_descriptions()
            );
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
            let memory = aarch64::mmu::linux_memory_snapshot();
            #[cfg(feature = "art-runtime")]
            crate::kprintln!(
                "LINUX_PAGING_OK discarded={} demand_zero={}",
                memory.discarded_pages,
                memory.demand_zero_pages
            );
            assert_eq!(memory.pages, 0);
            assert_eq!(memory.tables, 0);
            #[cfg(feature = "bionic-dynamic-runtime")]
            crate::scheduler::linux_threads::evidence("LINUX_THREADS_OK");
            if cfg!(feature = "bionic-runtime") {
                assert!(memory.peak_pages > 0 && memory.reclaimed_pages >= memory.peak_pages);
                assert!(crate::linux_entropy::consumed() > 32);
                #[cfg(feature = "bionic-dynamic-runtime")]
                assert_eq!(bndroid_kernel::linux_fd::live_descriptions(), 0);
                crate::kprintln!(
                    "{} abi=70 native_processes=2 native_exits=2 native_reaped=2 remaining_processes=1 native_handles=0 linux_calls={} unsupported={} bad_address={} live_anonymous_pages={} live_anonymous_tables={} peak_anonymous_pages={} reclaimed_anonymous_pages={} entropy_bytes={} loader={} live_file_descriptions={} bionic_startup=1 art=0",
                    if cfg!(feature = "bionic-dynamic-runtime") {
                        "BIONIC_DYNAMIC_RUNTIME_OK"
                    } else {
                        "BIONIC_RUNTIME_OK"
                    },
                    CALLS.load(Ordering::Relaxed),
                    UNKNOWN.load(Ordering::Relaxed),
                    BAD_ADDRESS.load(Ordering::Relaxed),
                    memory.pages,
                    memory.tables,
                    memory.peak_pages,
                    memory.reclaimed_pages,
                    crate::linux_entropy::consumed(),
                    if cfg!(feature = "bionic-dynamic-runtime") {
                        "upstream-android-linker"
                    } else {
                        "ndk-static-bionic"
                    },
                    bndroid_kernel::linux_fd::live_descriptions()
                );
                loop {
                    aarch64::wait_for_interrupt();
                }
            }
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
        // Two cold interpreter launches of the upstream ART/core jars exceed
        // the native smoke test's 30 seconds. Keep a finite ART-specific bound.
        let timeout_ticks = if cfg!(feature = "framework-runtime") {
            30000
        } else if cfg!(feature = "art-runtime") {
            9000
        } else {
            3000
        };
        if aarch64::timer::tick_count().wrapping_sub(start) > timeout_ticks {
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

#[cfg(feature = "bionic-dynamic-runtime")]
pub(crate) fn copy_in(address: u64, bytes: &mut [u8]) -> Result<(), i64> {
    let address = user_address(address);
    if !crate::process::current_user_range_is_readable(address, bytes.len()) {
        bad_address();
        return Err(EFAULT);
    }
    // AArch64 signal frames exceed the native IPC copy window. Validate all
    // pages first, then retain the bounded fault-recoverable copy primitive.
    for (index, chunk) in bytes.chunks_mut(usercopy::MAX_USER_COPY_BYTES).enumerate() {
        if usercopy::copy_from_user(
            chunk,
            address + (index * usercopy::MAX_USER_COPY_BYTES) as u64,
        )
        .is_err()
        {
            bad_address();
            return Err(EFAULT);
        }
    }
    Ok(())
}

#[cfg(feature = "bionic-dynamic-runtime")]
pub(crate) fn record_group_exit(process: u64, code: u64) {
    EXITS.fetch_add(1, Ordering::Release);
    crate::kprintln!(
        "LINUX_PROCESS_EXIT pid={} code={} native_handles=0",
        process,
        code
    );
}
