//! Shared-address-space Linux threads. All scheduler mutations occur on the
//! boot CPU with IRQ masked; each context still owns a separate kernel stack.
use super::*;
use crate::linux_runtime as runtime;
use bndroid_kernel::linux_abi::errno;
use bndroid_kernel::linux_sync::{self, FutexQueue, Wait};

#[cfg(feature = "art-runtime")]
pub(crate) mod binder;
#[cfg(feature = "art-runtime")]
mod io;
#[cfg(feature = "art-runtime")]
pub mod signals;

#[derive(Clone, Copy)]
struct Local {
    tid: u32,
    clear_tid: u64,
    signal_mask: u64,
    leader_exited: bool,
    timeout_result: u64,
    name: [u8; 16],
    #[cfg(feature = "art-runtime")]
    user_counter: u64,
    #[cfg(feature = "art-runtime")]
    kernel_counter: u64,
    #[cfg(feature = "art-runtime")]
    retired_user_counter: u64,
    #[cfg(feature = "art-runtime")]
    retired_kernel_counter: u64,
    #[cfg(feature = "art-runtime")]
    resumed_counter: u64,
}
impl Local {
    const EMPTY: Self = Self {
        tid: 0,
        clear_tid: 0,
        signal_mask: 0,
        leader_exited: false,
        timeout_result: 0,
        name: [0; 16],
        #[cfg(feature = "art-runtime")]
        user_counter: 0,
        #[cfg(feature = "art-runtime")]
        kernel_counter: 0,
        #[cfg(feature = "art-runtime")]
        retired_user_counter: 0,
        #[cfg(feature = "art-runtime")]
        retired_kernel_counter: 0,
        #[cfg(feature = "art-runtime")]
        resumed_counter: 0,
    };
}
struct Locals(UnsafeCell<[Local; CONTEXT_COUNT]>);
unsafe impl Sync for Locals {}
static LOCALS: Locals = Locals(UnsafeCell::new([Local::EMPTY; CONTEXT_COUNT]));
struct Waiters(UnsafeCell<FutexQueue<CONTEXT_COUNT>>);
unsafe impl Sync for Waiters {}
static WAITERS: Waiters = Waiters(UnsafeCell::new(FutexQueue::new()));
static STACKS: [DynamicStackSlot; LINUX_THREAD_CONTEXT_COUNT] =
    [const { DynamicStackSlot(UnsafeCell::new(None)) }; LINUX_THREAD_CONTEXT_COUNT];
static SELECTED: [AtomicU64; LINUX_THREAD_CONTEXT_COUNT] =
    [const { AtomicU64::new(0) }; LINUX_THREAD_CONTEXT_COUNT];
static NEXT_TID: AtomicU64 = AtomicU64::new(1024);
static CREATED: AtomicU64 = AtomicU64::new(0);
static REAPED: AtomicU64 = AtomicU64::new(0);
static WAITS: AtomicU64 = AtomicU64::new(0);
static WAKES: AtomicU64 = AtomicU64::new(0);
static TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static SLEEP_COMPLETIONS: AtomicU64 = AtomicU64::new(0);

fn local(context: usize) -> &'static mut Local {
    assert!(crate::arch::aarch64::irq_is_masked());
    unsafe { &mut (*LOCALS.0.get())[context] }
}

#[cfg(feature = "art-runtime")]
static ACCOUNT_DEPTH: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "art-runtime")]
static ACCOUNT_CONTEXT: AtomicU64 = AtomicU64::new(CONTEXT_COUNT as u64);
#[cfg(feature = "art-runtime")]
static ACCOUNT_START: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "art-runtime")]
#[unsafe(no_mangle)]
extern "C" fn linux_account_enter(frame: *mut TrapFrame) -> *mut TrapFrame {
    // Nested EL1 usercopy fixups must not finish the outer syscall's charge.
    if ACCOUNT_DEPTH.fetch_add(1, Ordering::Relaxed) != 0 {
        return frame;
    }
    ACCOUNT_CONTEXT.store(CONTEXT_COUNT as u64, Ordering::Relaxed);
    if unsafe { (*frame).spsr_el1 & 0x1f } == 0
        && current_process_id().is_some_and(crate::process::is_linux_process)
    {
        let now = crate::arch::aarch64::timer::counter_value();
        let context = current_running_context();
        let entry = local(context);
        entry.user_counter = entry
            .user_counter
            .saturating_add(now.wrapping_sub(entry.resumed_counter));
        ACCOUNT_CONTEXT.store(context as u64, Ordering::Relaxed);
        ACCOUNT_START.store(now, Ordering::Relaxed);
    }
    frame
}
#[cfg(feature = "art-runtime")]
#[unsafe(no_mangle)]
extern "C" fn linux_account_leave(frame: *mut TrapFrame) -> *mut TrapFrame {
    if ACCOUNT_DEPTH.fetch_sub(1, Ordering::Relaxed) != 1 {
        return frame;
    }
    let now = crate::arch::aarch64::timer::counter_value();
    let context = ACCOUNT_CONTEXT.swap(CONTEXT_COUNT as u64, Ordering::Relaxed) as usize;
    if context < CONTEXT_COUNT {
        let delta = now.wrapping_sub(ACCOUNT_START.load(Ordering::Relaxed));
        local(context).kernel_counter = local(context).kernel_counter.saturating_add(delta);
    }
    if unsafe { (*frame).spsr_el1 & 0x1f } == 0
        && current_process_id().is_some_and(crate::process::is_linux_process)
    {
        local(current_running_context()).resumed_counter = now;
    }
    frame
}

#[cfg(feature = "art-runtime")]
pub fn cpu_counter(per_thread: bool) -> u64 {
    let current = current_running_context();
    let process = owner(current);
    let mut total = 0u64;
    for context in DYNAMIC_USER_CONTEXT_START..CONTEXT_COUNT {
        if owner(context) == process && (!per_thread || context == current) {
            total = total
                .saturating_add(local(context).user_counter)
                .saturating_add(local(context).kernel_counter);
        }
    }
    if !per_thread {
        let main = local(leader(process));
        total = total
            .saturating_add(main.retired_user_counter)
            .saturating_add(main.retired_kernel_counter);
    }
    if ACCOUNT_DEPTH.load(Ordering::Relaxed) != 0
        && ACCOUNT_CONTEXT.load(Ordering::Relaxed) == current as u64
    {
        total = total.saturating_add(
            crate::arch::aarch64::timer::counter_value()
                .wrapping_sub(ACCOUNT_START.load(Ordering::Relaxed)),
        );
    }
    total
}

#[cfg(feature = "art-runtime")]
pub fn proc_stat(process: u64, parent: u32, started: u64) -> bndroid_kernel::linux_proc::Stat {
    let main = leader(process);
    let mut name = local(main).name;
    if name[0] == 0 {
        name[..8].copy_from_slice(b"linker64");
    }
    let mut user = local(main).retired_user_counter;
    let mut kernel = local(main).retired_kernel_counter;
    let mut threads = 0;
    for (context, state) in CONTEXTS.iter().enumerate().skip(DYNAMIC_USER_CONTEXT_START) {
        if owner(context) != process {
            continue;
        }
        user = user.saturating_add(local(context).user_counter);
        kernel = kernel.saturating_add(local(context).kernel_counter);
        if matches!(
            state.state.load(Ordering::Relaxed),
            STATE_RUNNING | STATE_RUNNABLE | STATE_WAITING
        ) && !local(context).leader_exited
        {
            threads += 1;
        }
    }
    let active = ACCOUNT_CONTEXT.load(Ordering::Relaxed) as usize;
    if active < CONTEXT_COUNT && owner(active) == process {
        kernel = kernel.saturating_add(
            crate::arch::aarch64::timer::counter_value()
                .wrapping_sub(ACCOUNT_START.load(Ordering::Relaxed)),
        );
    }
    let ticks = |counter: u64| {
        (u128::from(counter) * 100 / u128::from(crate::arch::aarch64::timer::frequency_hz())) as u64
    };
    bndroid_kernel::linux_proc::Stat {
        pid: process as u32,
        parent,
        name,
        state: if CONTEXTS[main].state.load(Ordering::Relaxed) == STATE_WAITING {
            'S'
        } else {
            'R'
        },
        user_ticks: ticks(user),
        kernel_ticks: ticks(kernel),
        threads,
        start_ticks: ticks(started),
        virtual_bytes: 0,
        resident_pages: 0,
        resident_limit: 0,
        code_start: 0,
        code_end: 0,
        stack_start: (crate::userboot::USER_STACK_END - 4096) as u64,
        blocked_signals: local(main).signal_mask,
    }
}
fn queue() -> &'static mut FutexQueue<CONTEXT_COUNT> {
    assert!(crate::arch::aarch64::irq_is_masked());
    unsafe { &mut *WAITERS.0.get() }
}
fn stack(context: usize) -> &'static mut Option<OwnedKernelStack> {
    assert!(crate::arch::aarch64::irq_is_masked());
    unsafe { &mut *STACKS[context - DYNAMIC_USER_CONTEXT_END].0.get() }
}
pub(super) fn selection_counter(context: usize) -> &'static AtomicU64 {
    &SELECTED[context - DYNAMIC_USER_CONTEXT_END]
}
fn owner(context: usize) -> u64 {
    CONTEXT_PROCESS_IDS[context].load(Ordering::Relaxed)
}
fn leader(process: u64) -> usize {
    (DYNAMIC_USER_CONTEXT_START..DYNAMIC_USER_CONTEXT_END)
        .find(|context| owner(*context) == process)
        .expect("Linux thread group retains its leader")
}
fn current_tid() -> u64 {
    let context = current_running_context();
    let tid = local(context).tid;
    if tid == 0 {
        owner(context) & 0xffff_ffff
    } else {
        tid as u64
    }
}

// Keep thread/VM branch temporaries out of the common syscall frame under LTO.
#[cfg_attr(feature = "art-runtime", inline(never))]
pub fn dispatch(frame: *mut TrapFrame, r: &[u64; 31]) -> Option<*mut TrapFrame> {
    #[cfg(feature = "art-runtime")]
    if let Some(next) = io::dispatch(frame, r) {
        return Some(next);
    }
    #[cfg(feature = "art-runtime")]
    if let Some(next) = binder::dispatch(frame, r) {
        return Some(next);
    }
    #[cfg(feature = "art-runtime")]
    if let Some(next) = signals::dispatch(frame, r) {
        return Some(next);
    }
    let result = match r[8] {
        93 | 94 => return Some(exit(frame, r[0] & 255, r[8] == 94)),
        96 => {
            local(current_running_context()).clear_tid = runtime::user_address(r[0]);
            Ok(current_tid())
        }
        98 => return Some(futex(frame, r)),
        101 | 115 => return Some(sleep(frame, r)),
        124 => {
            runtime::complete(frame, 0);
            return Some(block(frame, ThreadState::Runnable));
        }
        135 => signal_mask(r),
        #[cfg(feature = "art-runtime")]
        141 => {
            if r[0] != 0 {
                Err(22)
            } else {
                let pid = owner(current_running_context());
                let wanted = r[1] as u32;
                let found = wanted == 0
                    || (DYNAMIC_USER_CONTEXT_START..CONTEXT_COUNT).any(|context| {
                        owner(context) == pid
                            && matches!(
                                CONTEXTS[context].state.load(Ordering::Relaxed),
                                STATE_RUNNING | STATE_RUNNABLE | STATE_WAITING
                            )
                            && if local(context).tid == 0 {
                                pid as u32 == wanted
                            } else {
                                local(context).tid == wanted
                            }
                    });
                // Linux getpriority returns 20 - nice. Every current Linux
                // thread uses SCHED_OTHER/normal priority; changes unsupported.
                if found { Ok(20) } else { Err(3) }
            }
        }
        167 if matches!(r[0], 15 | 16) => thread_name(r),
        178 => Ok(current_tid()),
        220 => clone_thread(frame, r),
        _ => return None,
    };
    Some(runtime::complete(frame, result.unwrap_or_else(errno)))
}

fn writable(address: u64, count: usize) -> bool {
    crate::process::current_user_range_is_writable(address, count)
}
fn output(address: u64, bytes: &[u8]) -> Result<(), i64> {
    let value = runtime::copy_out(address, bytes) as i64;
    if value < 0 { Err(-value) } else { Ok(()) }
}
fn clone_thread(frame: *mut TrapFrame, r: &[u64; 31]) -> Result<u64, i64> {
    linux_sync::validate_clone(r[0])?;
    let child_stack = runtime::user_address(r[1]);
    if !child_stack.is_multiple_of(16)
        || child_stack.checked_sub(16).is_none_or(|p| !writable(p, 16))
    {
        return Err(22);
    }
    let parent_tid = runtime::user_address(r[2]);
    let child_tid = runtime::user_address(r[4]);
    for (flag, address) in [
        (linux_sync::CLONE_PARENT_SETTID, parent_tid),
        (linux_sync::CLONE_CHILD_SETTID, child_tid),
    ] {
        if r[0] & flag != 0 && !writable(address, 4) {
            return Err(14);
        }
    }
    let context = (DYNAMIC_USER_CONTEXT_END..CONTEXT_COUNT)
        .find(|i| CONTEXTS[*i].state.load(Ordering::Relaxed) == STATE_INACTIVE)
        .ok_or(11)?;
    let allocated = OwnedKernelStack::try_new().ok_or(12)?;
    let tid = NEXT_TID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            (next < i32::MAX as u64).then_some(next + 1)
        })
        .map_err(|_| 11)? as u32;
    let current = current_running_context();
    let parent = *local(current);
    let process = owner(current);
    let range = allocated.range();
    let child_frame = (range.1 - size_of::<TrapFrame>()) as *mut TrapFrame;
    // Saved EL0 state was validated by the syscall entry. Copy into a private,
    // unpublished kernel stack before making any child context runnable.
    unsafe {
        ptr::copy_nonoverlapping(frame, child_frame, 1);
        (*child_frame).x[0] = 0;
        (*child_frame).sp_el0 = child_stack;
        if r[0] & linux_sync::CLONE_SETTLS != 0 {
            (*child_frame).tpidr_el0 = r[3];
        }
    }
    if r[0] & linux_sync::CLONE_PARENT_SETTID != 0 {
        output(parent_tid, &tid.to_le_bytes())?;
    }
    if r[0] & linux_sync::CLONE_CHILD_SETTID != 0 {
        output(child_tid, &tid.to_le_bytes())?;
    }
    configure_stack(context, range);
    *stack(context) = Some(allocated);
    *local(context) = Local {
        tid,
        clear_tid: if r[0] & linux_sync::CLONE_CHILD_CLEARTID != 0 {
            child_tid
        } else {
            0
        },
        signal_mask: parent.signal_mask,
        name: parent.name,
        ..Local::EMPTY
    };
    CONTEXT_PROCESS_IDS[context].store(process, Ordering::Relaxed);
    CONTEXTS[context]
        .saved_frame
        .store(child_frame, Ordering::Relaxed);
    CONTEXTS[context]
        .translation
        .store(context_translation(current).raw(), Ordering::Relaxed);
    selection_counter(context).store(0, Ordering::Relaxed);
    validate_frame_or_panic(context, child_frame);
    transition_state(context, ThreadState::Inactive, ThreadState::Runnable);
    CREATED.fetch_add(1, Ordering::Relaxed);
    Ok(tid as u64)
}

fn signal_mask(r: &[u64; 31]) -> Result<u64, i64> {
    if r[3] != 8 {
        return Err(22);
    }
    let current = current_running_context();
    let previous = local(current).signal_mask;
    let next = if r[1] != 0 {
        let mut bytes = [0; 8];
        runtime::copy_in(r[1], &mut bytes)?;
        let mask = u64::from_le_bytes(bytes) & !((1 << 8) | (1 << 18)); // KILL/STOP cannot be blocked.
        Some(match r[0] {
            0 => previous | mask,
            1 => previous & !mask,
            2 => mask,
            _ => return Err(22),
        })
    } else {
        None
    };
    if r[2] != 0 {
        output(r[2], &previous.to_le_bytes())?;
    }
    if let Some(mask) = next {
        local(current).signal_mask = mask;
    }
    Ok(0)
}
fn thread_name(r: &[u64; 31]) -> Result<u64, i64> {
    let context = current_running_context();
    if r[0] == 16 {
        let name = local(context).name;
        output(r[1], &name)?;
    } else {
        let mut name = [0; 16];
        for index in 0..15 {
            runtime::copy_in(
                r[1].checked_add(index as u64).ok_or(14)?,
                &mut name[index..index + 1],
            )?;
            if name[index] == 0 {
                break;
            }
        }
        local(context).name = name;
    }
    Ok(0)
}

#[cfg(feature = "art-runtime")]
fn deadline(address: u64, absolute: bool) -> Result<u64, i64> {
    clock_deadline(address, absolute, false)
}
fn clock_deadline(address: u64, absolute: bool, realtime: bool) -> Result<u64, i64> {
    let mut bytes = [0; 16];
    runtime::copy_in(address, &mut bytes)?;
    #[cfg(feature = "art-runtime")]
    if realtime && absolute {
        return runtime::realtime_anchor().deadline(
            i64::from_le_bytes(bytes[..8].try_into().unwrap()),
            i64::from_le_bytes(bytes[8..].try_into().unwrap()),
            crate::arch::aarch64::timer::frequency_hz(),
        );
    }
    #[cfg(not(feature = "art-runtime"))]
    if realtime {
        return Err(22);
    }
    linux_sync::deadline(
        i64::from_le_bytes(bytes[..8].try_into().unwrap()),
        i64::from_le_bytes(bytes[8..].try_into().unwrap()),
        crate::arch::aarch64::timer::frequency_hz(),
        crate::arch::aarch64::timer::counter_value(),
        absolute,
    )
}
fn futex(frame: *mut TrapFrame, r: &[u64; 31]) -> *mut TrapFrame {
    #[cfg(feature = "art-runtime")]
    let restarted_timeout = signals::restart_deadline(frame, r);
    #[cfg(not(feature = "art-runtime"))]
    let restarted_timeout: Option<Option<u64>> = None;
    let operation = r[1];
    let command = operation & !(128 | 256);
    let realtime = operation & 256 != 0;
    // Both forms key by the full address-space owner. Other processes cannot
    // alias these mappings. Priority inheritance remains unsupported.
    if !matches!(command, 0 | 1 | 3 | 4 | 9 | 10)
        || (realtime && (command != 9 || !cfg!(feature = "art-runtime")))
    {
        return runtime::complete(frame, errno(38));
    }
    let address = runtime::user_address(r[0]);
    if !address.is_multiple_of(4) {
        return runtime::complete(frame, errno(22));
    }
    let mut bytes = [0; 4];
    if let Err(error) = runtime::copy_in(address, &mut bytes) {
        return runtime::complete(frame, errno(error));
    }
    let bitset = if matches!(command, 9 | 10) {
        r[5] as u32
    } else {
        u32::MAX
    };
    if bitset == 0 {
        return runtime::complete(frame, errno(22));
    }
    let context = current_running_context();
    let process = owner(context);
    if matches!(command, 3 | 4) {
        let target = runtime::user_address(r[4]);
        if (r[2] as i32) < 0 || (r[3] as i32) < 0 || !target.is_multiple_of(4) {
            return runtime::complete(frame, errno(22));
        }
        let mut destination = [0; 4];
        if let Err(error) = runtime::copy_in(target, &mut destination) {
            return runtime::complete(frame, errno(error));
        }
        if command == 4 && u32::from_le_bytes(bytes) != r[5] as u32 {
            return runtime::complete(frame, errno(11));
        }
        // IRQs stay masked throughout comparison, migration and wakeup on
        // this single-CPU profile; no waiter can be lost between these steps.
        let (selected, moved) = queue().requeue(process, address, target, r[2] as u32, r[3] as u32);
        wake(selected, 0);
        WAKES.fetch_add(selected.count_ones() as u64, Ordering::Relaxed);
        return runtime::complete(frame, selected.count_ones() as u64 + u64::from(moved));
    }
    if matches!(command, 1 | 10) {
        if (r[2] as i32) < 0 {
            return runtime::complete(frame, errno(22));
        }
        let selected = queue().wake(process, address, r[2] as u32, bitset);
        wake(selected, 0);
        WAKES.fetch_add(selected.count_ones() as u64, Ordering::Relaxed);
        return runtime::complete(frame, selected.count_ones() as u64);
    }
    if u32::from_le_bytes(bytes) != r[2] as u32 {
        return runtime::complete(frame, errno(11));
    }
    let timeout = if let Some(saved) = restarted_timeout {
        saved
    } else if r[3] == 0 {
        None
    } else {
        match clock_deadline(r[3], command == 9, realtime) {
            Ok(end) => Some(end),
            Err(error) => return runtime::complete(frame, errno(error)),
        }
    };
    if timeout
        .is_some_and(|end| deadline_reached(crate::arch::aarch64::timer::counter_value(), end))
    {
        return runtime::complete(frame, errno(110));
    }
    let wait = Wait {
        process,
        address,
        bitset,
        deadline: timeout,
    };
    if let Err(error) = queue().insert(context, wait) {
        return runtime::complete(frame, errno(error));
    }
    local(context).timeout_result = errno(110);
    #[cfg(feature = "art-runtime")]
    signals::note_futex(frame, r, timeout);
    runtime::complete(frame, 0);
    WAITS.fetch_add(1, Ordering::Relaxed);
    block(frame, ThreadState::Waiting)
}
fn sleep(frame: *mut TrapFrame, r: &[u64; 31]) -> *mut TrapFrame {
    let (request, absolute, realtime) = if r[8] == 101 {
        (r[0], false, false)
    } else if (r[0] == 1 || (r[0] == 0 && cfg!(feature = "art-runtime"))) && r[1] <= 1 {
        (r[2], r[1] == 1, r[0] == 0)
    } else {
        return runtime::complete(frame, errno(22));
    };
    let end = match clock_deadline(request, absolute, realtime) {
        Ok(end) => end,
        Err(error) => return runtime::complete(frame, errno(error)),
    };
    if deadline_reached(crate::arch::aarch64::timer::counter_value(), end) {
        return runtime::complete(frame, 0);
    }
    let context = current_running_context();
    let wait = Wait {
        process: owner(context),
        address: 0,
        bitset: !0,
        deadline: Some(end),
    };
    if let Err(error) = queue().insert(context, wait) {
        return runtime::complete(frame, errno(error));
    }
    local(context).timeout_result = 0;
    #[cfg(feature = "art-runtime")]
    signals::note_sleep(
        if absolute {
            0
        } else if r[8] == 101 {
            r[1]
        } else {
            r[3]
        },
        end,
    );
    runtime::complete(frame, 0);
    block(frame, ThreadState::Waiting)
}
fn block(frame: *mut TrapFrame, state: ThreadState) -> *mut TrapFrame {
    let context = current_running_context();
    CONTEXTS[context]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    transition_state(context, ThreadState::Running, state);
    #[cfg(feature = "art-runtime")]
    if state == ThreadState::Waiting && signals::has_pending(context) {
        signals::interrupt(context);
    }
    BLOCK_DISPATCHES.fetch_add(1, Ordering::Relaxed);
    select_next_context(context)
}
fn wake(selected: u64, result: u64) {
    for (context, state) in CONTEXTS.iter().enumerate().skip(USER_INIT_CONTEXT) {
        if selected & (1 << context) == 0 {
            continue;
        }
        assert_eq!(state.state.load(Ordering::Relaxed), STATE_WAITING);
        let frame = state.saved_frame.load(Ordering::Relaxed);
        validate_frame_storage_or_panic(context, frame);
        unsafe {
            (*frame).x[0] = result;
        }
        #[cfg(feature = "art-runtime")]
        signals::clear_wait(context);
        transition_state(context, ThreadState::Waiting, ThreadState::Runnable);
    }
}
pub(super) fn wake_expired(now: u64) {
    #[cfg(feature = "art-runtime")]
    io::wake_ready(now);
    #[cfg(feature = "art-runtime")]
    binder::wake_ready();
    let selected = queue().expire(now);
    for context in USER_INIT_CONTEXT..CONTEXT_COUNT {
        if selected & (1 << context) != 0 {
            let result = local(context).timeout_result;
            wake(1 << context, result);
            if result == 0 {
                SLEEP_COMPLETIONS.fetch_add(1, Ordering::Relaxed);
            } else {
                TIMEOUTS.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}
fn clear_tid(context: usize) {
    let address = core::mem::take(&mut local(context).clear_tid);
    if address != 0 && output(address, &[0; 4]).is_ok() {
        let selected = queue().wake(owner(context), address, u32::MAX, !0);
        wake(selected, 0);
        WAKES.fetch_add(selected.count_ones() as u64, Ordering::Relaxed);
    }
}

fn living_siblings(process: u64) -> bool {
    (DYNAMIC_USER_CONTEXT_END..CONTEXT_COUNT).any(|context| {
        owner(context) == process
            && matches!(
                CONTEXTS[context].state.load(Ordering::Relaxed),
                STATE_RUNNING | STATE_RUNNABLE | STATE_WAITING
            )
    })
}
fn exit(frame: *mut TrapFrame, code: u64, group: bool) -> *mut TrapFrame {
    let context = current_running_context();
    let process = owner(context);
    let main = leader(process);
    #[cfg(feature = "art-runtime")]
    bndroid_kernel::linux_binder::with(|r| r.process_thread_exit(process, current_tid() as u32));
    CONTEXTS[context]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    clear_tid(context);
    if group {
        stop_group(process, code, UserTerminationReason::Exited, None);
        runtime::record_group_exit(process, code);
    } else if context == main && living_siblings(process) {
        local(main).leader_exited = true;
        transition_state(main, ThreadState::Running, ThreadState::Waiting);
    } else {
        transition_state(context, ThreadState::Running, ThreadState::Zombie);
        if context == main || (local(main).leader_exited && !living_siblings(process)) {
            stop_group(process, code, UserTerminationReason::Exited, None);
            runtime::record_group_exit(process, code);
        }
    }
    select_next_context(context)
}
fn stop_group(
    process: u64,
    code: u64,
    reason: UserTerminationReason,
    fault: Option<(UserFaultReason, u64, u64, u64, u64)>,
) {
    let main = leader(process);
    for (context, state) in CONTEXTS.iter().enumerate().skip(DYNAMIC_USER_CONTEXT_START) {
        if owner(context) != process {
            continue;
        }
        queue().cancel(context);
        #[cfg(feature = "art-runtime")]
        {
            binder::cancel(context);
            io::cancel(context);
            let tid = if local(context).tid == 0 {
                process as u32
            } else {
                local(context).tid
            };
            bndroid_kernel::linux_binder::with(|r| r.process_thread_exit(process, tid));
        }
        let source = match state.state.load(Ordering::Relaxed) {
            STATE_RUNNING => ThreadState::Running,
            STATE_RUNNABLE => ThreadState::Runnable,
            STATE_WAITING => ThreadState::Waiting,
            STATE_ZOMBIE | STATE_FAULTED => continue,
            _ => panic!("Linux group owned an invalid context state"),
        };
        transition_state(context, source, ThreadState::Zombie);
    }
    let slot = dynamic_user_index(main).unwrap();
    USER_DYNAMIC_EXIT_CODES[slot].store(code, Ordering::Relaxed);
    USER_DYNAMIC_TERMINATION_REASONS[slot].store(reason as u64, Ordering::Relaxed);
    clear_dynamic_user_fault_witness(slot);
    if let Some((why, esr, far, pc, sp)) = fault {
        USER_DYNAMIC_FAULT_REASONS[slot].store(why as u64, Ordering::Relaxed);
        USER_DYNAMIC_FAULT_ESRS[slot].store(esr, Ordering::Relaxed);
        USER_DYNAMIC_FAULT_FARS[slot].store(far, Ordering::Relaxed);
        USER_DYNAMIC_FAULT_ELRS[slot].store(pc, Ordering::Relaxed);
        USER_DYNAMIC_FAULT_SP_EL0S[slot].store(sp, Ordering::Relaxed);
    }
}
pub(super) fn fault_group(
    frame: *mut TrapFrame,
    reason: UserFaultReason,
    esr: u64,
    far: u64,
) -> *mut TrapFrame {
    let context = current_running_context();
    CONTEXTS[context]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    let fault = unsafe { (reason, esr, far, (*frame).elr_el1, (*frame).sp_el0) };
    stop_group(
        owner(context),
        0,
        UserTerminationReason::Faulted,
        Some(fault),
    );
    select_next_context(context)
}
pub(super) fn terminate_group(process: u64) -> Result<bool, TerminateUserError> {
    if !crate::process::is_linux_process(process) {
        return Ok(false);
    }
    let main = leader(process);
    if matches!(
        CONTEXTS[main].state.load(Ordering::Relaxed),
        STATE_ZOMBIE | STATE_FAULTED
    ) {
        return Err(TerminateUserError::AlreadyTerminal);
    }
    stop_group(
        process,
        PROCESS_KILLED_EXIT_CODE,
        UserTerminationReason::Killed,
        None,
    );
    Ok(true)
}

pub(super) fn reset_context(context: usize) {
    queue().cancel(context);
    #[cfg(feature = "art-runtime")]
    io::cancel(context);
    #[cfg(feature = "art-runtime")]
    binder::cancel(context);
    #[cfg(feature = "art-runtime")]
    signals::reset(context);
    *local(context) = Local::EMPTY;
}
pub fn reap() {
    assert!(crate::arch::aarch64::irq_is_masked());
    let current = current_running_context();
    for context in DYNAMIC_USER_CONTEXT_END..CONTEXT_COUNT {
        if CONTEXTS[context].state.load(Ordering::Relaxed) != STATE_ZOMBIE {
            continue;
        }
        assert_ne!(current, context);
        assert!(stack_canary_intact(&CONTEXTS[context]));
        let allocated = stack(context)
            .take()
            .expect("retired thread owns its kernel stack");
        #[cfg(feature = "art-runtime")]
        {
            let main = leader(owner(context));
            let retired = *local(context);
            local(main).retired_user_counter = local(main)
                .retired_user_counter
                .saturating_add(retired.user_counter);
            local(main).retired_kernel_counter = local(main)
                .retired_kernel_counter
                .saturating_add(retired.kernel_counter);
        }
        transition_state(context, ThreadState::Zombie, ThreadState::Inactive);
        CONTEXTS[context]
            .saved_frame
            .store(ptr::null_mut(), Ordering::Relaxed);
        CONTEXTS[context].translation.store(0, Ordering::Relaxed);
        CONTEXTS[context].stack_bottom.store(0, Ordering::Relaxed);
        CONTEXTS[context].stack_top.store(0, Ordering::Relaxed);
        CONTEXTS[context].canary_address.store(0, Ordering::Relaxed);
        CONTEXT_PROCESS_IDS[context].store(0, Ordering::Relaxed);
        reset_context(context);
        drop(allocated);
        REAPED.fetch_add(1, Ordering::Relaxed);
    }
}
pub fn evidence(marker: &str) {
    let saved = crate::arch::aarch64::save_and_mask_irq();
    let live = (DYNAMIC_USER_CONTEXT_END..CONTEXT_COUNT)
        .filter(|c| stack(*c).is_some())
        .count();
    let waits = queue().len();
    #[cfg(feature = "art-runtime")]
    {
        assert_eq!(binder::live(), 0);
        io::evidence();
        let stats = bndroid_kernel::linux_binder::with(|r| r.stats());
        crate::kprintln!(
            "LINUX_BINDER_OK opened={} closed={} submitted={} replied={} oneway={} failed={} deaths={} connections={} transactions={} buffers={} first_sender={} first_receiver={} flushes={} flush_returns={}",
            stats.opened,
            stats.closed,
            stats.submitted,
            stats.replied,
            stats.oneway,
            stats.failed,
            stats.deaths,
            stats.connections,
            stats.transactions,
            stats.buffers,
            stats.first_sender,
            stats.first_receiver,
            stats.flushes,
            stats.flush_returns
        );
        assert_eq!(
            (stats.connections, stats.transactions, stats.buffers),
            (0, 0, 0)
        );
    }
    crate::kprintln!(
        "{} created={} reaped={} live={} waiting={} waits={} wakes={} timeouts={} sleeps={}",
        marker,
        CREATED.load(Ordering::Relaxed),
        REAPED.load(Ordering::Relaxed),
        live,
        waits,
        WAITS.load(Ordering::Relaxed),
        WAKES.load(Ordering::Relaxed),
        TIMEOUTS.load(Ordering::Relaxed),
        SLEEP_COMPLETIONS.load(Ordering::Relaxed)
    );
    assert_eq!(live, 0);
    assert_eq!(waits, 0);
    assert_eq!(
        CREATED.load(Ordering::Relaxed),
        REAPED.load(Ordering::Relaxed)
    );
    #[cfg(feature = "art-runtime")]
    signals::evidence();
    crate::arch::aarch64::restore_daif(saved);
}
