//! Signal ownership and delivery for the single-CPU Linux thread personality.
use super::*;
use bndroid_kernel::linux_signal::{self as wire, Action, AltStack, Context, Info, Pending};

#[derive(Clone, Copy)]
pub(super) struct RestartCall {
    args: [u64; 6],
    pc: u64,
    deadline: Option<u64>,
}
#[derive(Clone, Copy)]
enum Waiting {
    None,
    Restartable(RestartCall),
    Sleep { remaining: u64, deadline: u64 },
    Signal { set: u64, info: u64 },
}
#[derive(Clone, Copy)]
struct Thread {
    stack: AltStack,
    pending: Pending,
    wait: Waiting,
    interrupted: Option<RestartCall>,
    restart: Option<RestartCall>,
    restart_frames: [Option<(u64, RestartCall)>; 16],
    restore_mask: Option<u64>,
}
impl Thread {
    const EMPTY: Self = Self {
        stack: AltStack::EMPTY,
        pending: Pending::EMPTY,
        wait: Waiting::None,
        interrupted: None,
        restart: None,
        restart_frames: [None; 16],
        restore_mask: None,
    };
}
struct Group {
    actions: [Action; 64],
    pending: Pending,
    trampoline: u64,
}
impl Group {
    const EMPTY: Self = Self {
        actions: [Action::EMPTY; 64],
        pending: Pending::EMPTY,
        trampoline: 0,
    };
}
struct Threads(UnsafeCell<[Thread; CONTEXT_COUNT]>);
struct Groups(UnsafeCell<[Group; DYNAMIC_USER_CONTEXT_COUNT]>);
unsafe impl Sync for Threads {}
unsafe impl Sync for Groups {}
static THREADS: Threads = Threads(UnsafeCell::new([Thread::EMPTY; CONTEXT_COUNT]));
static GROUPS: Groups = Groups(UnsafeCell::new(
    [const { Group::EMPTY }; DYNAMIC_USER_CONTEXT_COUNT],
));
static DELIVERED: AtomicU64 = AtomicU64::new(0);
static RETURNED: AtomicU64 = AtomicU64::new(0);
static INTERRUPTED: AtomicU64 = AtomicU64::new(0);
fn thread(context: usize) -> &'static mut Thread {
    assert!(crate::arch::aarch64::irq_is_masked());
    unsafe { &mut (*THREADS.0.get())[context] }
}
fn group(context: usize) -> &'static mut Group {
    let index = leader(owner(context)) - DYNAMIC_USER_CONTEXT_START;
    unsafe { &mut (*GROUPS.0.get())[index] }
}
pub(super) fn reset(context: usize) {
    *thread(context) = Thread::EMPTY;
    if (DYNAMIC_USER_CONTEXT_START..DYNAMIC_USER_CONTEXT_END).contains(&context) {
        unsafe {
            (*GROUPS.0.get())[context - DYNAMIC_USER_CONTEXT_START] = Group::EMPTY;
        }
    }
}
fn live(context: usize, process: u64) -> bool {
    owner(context) == process
        && !local(context).leader_exited
        && matches!(
            CONTEXTS[context].state.load(Ordering::Relaxed),
            STATE_RUNNING | STATE_RUNNABLE | STATE_WAITING
        )
}
fn tid(context: usize) -> u32 {
    if local(context).tid == 0 {
        owner(context) as u32
    } else {
        local(context).tid
    }
}
fn return_valid(pc: u64, sp: u64) -> bool {
    crate::process::linux_return_state_is_valid(
        owner(current_running_context()),
        pc as usize,
        sp as usize,
    ) == Some(true)
}
#[inline(never)]
pub(super) fn dispatch(frame: *mut TrapFrame, r: &[u64; 31]) -> Option<*mut TrapFrame> {
    let result = match r[8] {
        129 | 130 | 131 | 138 | 240 => send(r),
        132 => altstack(frame, r),
        133 => return Some(suspend(frame, r)),
        134 => action(r),
        136 => pending(r),
        137 => return Some(wait_signal(frame, r)),
        139 => return Some(sigreturn(frame)),
        _ => return None,
    };
    Some(runtime::complete(frame, result.unwrap_or_else(errno)))
}
fn action(r: &[u64; 31]) -> Result<u64, i64> {
    let signal = wire::number(r[0])?;
    if r[3] != 8 || r[1] != 0 && matches!(signal, wire::SIGKILL | wire::SIGSTOP) {
        return Err(22);
    }
    let context = current_running_context();
    let previous = group(context).actions[signal as usize - 1];
    if r[2] != 0 && !writable(runtime::user_address(r[2]), 32) {
        return Err(14);
    }
    let next = if r[1] != 0 {
        let mut bytes = [0; 32];
        runtime::copy_in(r[1], &mut bytes)?;
        Some(Action::decode(&bytes))
    } else {
        None
    };
    if let Some(next) = next
        && next.handler > 1
        && next.flags & wire::RESTORER == 0
        && group(context).trampoline == 0
    {
        let address =
            crate::process::with_current_linux_address_space(|s| s.linux_signal_trampoline())?;
        group(context).trampoline = address;
    }
    if r[2] != 0 {
        output(r[2], &previous.encode())?;
    }
    if let Some(next) = next {
        group(context).actions[signal as usize - 1] = next;
        if next.handler == 1 || next.handler == 0 && wire::ignored_by_default(signal) {
            group(context).pending.discard(signal);
            for c in DYNAMIC_USER_CONTEXT_START..CONTEXT_COUNT {
                if owner(c) == owner(context) {
                    thread(c).pending.discard(signal);
                }
            }
        }
    }
    Ok(0)
}
fn altstack(frame: *mut TrapFrame, r: &[u64; 31]) -> Result<u64, i64> {
    let context = current_running_context();
    let previous = thread(context).stack;
    let sp = unsafe { (*frame).sp_el0 };
    let next = if r[0] != 0 {
        if previous.contains(sp) {
            return Err(1);
        }
        let mut bytes = [0; 24];
        runtime::copy_in(r[0], &mut bytes)?;
        Some(AltStack::decode(
            &bytes,
            bndroid_kernel::linux_vm::ARENA_END,
            false,
        )?)
    } else {
        None
    };
    if r[1] != 0 {
        output(r[1], &previous.encode(sp))?;
    }
    if let Some(next) = next {
        thread(context).stack = next;
    }
    Ok(0)
}
fn pending(r: &[u64; 31]) -> Result<u64, i64> {
    if r[1] != 8 {
        return Err(22);
    }
    let c = current_running_context();
    let mask = (thread(c).pending.mask() | group(c).pending.mask()) & local(c).signal_mask;
    output(r[0], &mask.to_le_bytes())?;
    Ok(0)
}
fn ignored(context: usize, signal: u8) -> bool {
    let a = group(context).actions[signal as usize - 1];
    a.handler == 1 || a.handler == 0 && (wire::ignored_by_default(signal) || signal == 18)
}
fn pop(context: usize, mask: u64) -> Option<Info> {
    thread(context)
        .pending
        .pop(mask)
        .or_else(|| group(context).pending.pop(mask))
}
pub(super) fn has_pending(context: usize) -> bool {
    (thread(context).pending.mask() | group(context).pending.mask()) & !local(context).signal_mask
        != 0
}
fn send(r: &[u64; 31]) -> Result<u64, i64> {
    let current = current_running_context();
    let process = owner(current);
    let (wanted, raw, directed) = match r[8] {
        129 | 138 => {
            let pid = r[0] as u32 as i32;
            if r[8] == 138 && pid <= 0 {
                return Err(22);
            }
            if pid != 0 && pid != process as u32 as i32 && pid != -(process as u32 as i32) {
                return Err(3);
            }
            (0, r[1], false)
        }
        130 => {
            if r[0] as i32 <= 0 {
                return Err(22);
            }
            (r[0] as u32, r[1], true)
        }
        _ => {
            if r[0] as i32 <= 0 || r[1] as i32 <= 0 {
                return Err(22);
            }
            if r[0] as u32 != process as u32 {
                return Err(3);
            }
            (r[1] as u32, r[2], true)
        }
    };
    let target = if directed {
        (DYNAMIC_USER_CONTEXT_START..CONTEXT_COUNT)
            .find(|c| live(*c, process) && tid(*c) == wanted)
            .ok_or(3)?
    } else {
        current
    };
    if raw == 0 {
        return Ok(0);
    }
    let signal = wire::number(raw)?;
    // Job-control stop/continue requires an external process/session model.
    // Do not report a stop that leaves the group running.
    if matches!(signal, 19..=22) && group(current).actions[signal as usize - 1].handler == 0 {
        return Err(38);
    }
    let mut info = Info {
        signal,
        code: if directed { -6 } else { 0 },
        pid: process as u32,
        uid: 10000,
        address: 0,
        value: 0,
    };
    if matches!(r[8], 138 | 240) {
        let mut bytes = [0; 128];
        runtime::copy_in(r[if r[8] == 138 { 2 } else { 3 }], &mut bytes)?;
        let code = wire::u32_at(&bytes, 8) as i32;
        if !matches!(code, -1 | -6) || code == -6 && target != current {
            return Err(1);
        }
        info.code = code;
        info.value = wire::u64_at(&bytes, 24);
    }
    // sigwait consumes matching blocked signals without invoking handlers.
    for c in DYNAMIC_USER_CONTEXT_START..CONTEXT_COUNT {
        if (!directed || c == target)
            && live(c, process)
            && let Waiting::Signal {
                set,
                info: destination,
            } = thread(c).wait
            && set & wire::bit(signal) != 0
        {
            let result = if destination == 0 {
                Ok(())
            } else {
                output(destination, &info.encode(false))
            };
            queue().cancel(c);
            wake(1 << c, result.map(|_| signal as u64).unwrap_or_else(errno));
            return Ok(0);
        }
    }
    if ignored(current, signal) {
        return Ok(0);
    }
    if directed {
        thread(target).pending.push(info)?;
    } else {
        group(current).pending.push(info)?;
    }
    for c in DYNAMIC_USER_CONTEXT_START..CONTEXT_COUNT {
        if (!directed || c == target)
            && live(c, process)
            && local(c).signal_mask & wire::bit(signal) == 0
        {
            interrupt(c);
            if !directed {
                break;
            }
        }
    }
    Ok(0)
}
pub(super) fn clear_wait(context: usize) {
    thread(context).wait = Waiting::None;
}
pub(super) fn note_futex(frame: *mut TrapFrame, r: &[u64; 31], deadline: Option<u64>) {
    note_restartable(frame, r[..6].try_into().unwrap(), deadline);
}
pub(super) fn note_restartable(frame: *mut TrapFrame, args: [u64; 6], deadline: Option<u64>) {
    thread(current_running_context()).wait = Waiting::Restartable(RestartCall {
        args,
        pc: unsafe { (*frame).elr_el1 },
        deadline,
    });
}
pub(super) fn restart_deadline(frame: *mut TrapFrame, r: &[u64; 31]) -> Option<Option<u64>> {
    thread(current_running_context())
        .restart
        .take()
        .filter(|saved| saved.pc == unsafe { (*frame).elr_el1 } && saved.args == r[..6])
        .map(|saved| saved.deadline)
}
pub(super) fn note_sleep(remaining: u64, deadline: u64) {
    thread(current_running_context()).wait = Waiting::Sleep {
        remaining,
        deadline,
    };
}
pub(super) fn interrupt(context: usize) {
    if CONTEXTS[context].state.load(Ordering::Relaxed) != STATE_WAITING || !queue().cancel(context)
    {
        return;
    }
    let wait = thread(context).wait;
    super::binder::cancel(context);
    super::io::cancel(context);
    let mut result = errno(4);
    if let Waiting::Restartable(call) = wait {
        thread(context).interrupted = Some(call);
    }
    if let Waiting::Sleep {
        remaining,
        deadline,
    } = wait
        && remaining != 0
    {
        let now = crate::arch::aarch64::timer::counter_value();
        let left = if deadline_reached(now, deadline) {
            0
        } else {
            deadline.wrapping_sub(now)
        };
        let ns = (u128::from(left) * 1_000_000_000
            / u128::from(crate::arch::aarch64::timer::frequency_hz())) as u64;
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&(ns / 1_000_000_000).to_le_bytes());
        bytes[8..].copy_from_slice(&(ns % 1_000_000_000).to_le_bytes());
        if let Err(error) = output(remaining, &bytes) {
            result = errno(error);
        }
    }
    wake(1 << context, result);
    INTERRUPTED.fetch_add(1, Ordering::Relaxed);
}
fn wait_signal(frame: *mut TrapFrame, r: &[u64; 31]) -> *mut TrapFrame {
    let result = (|| {
        if r[3] != 8 {
            return Err(22);
        }
        let mut bytes = [0; 8];
        runtime::copy_in(r[0], &mut bytes)?;
        let set = u64::from_le_bytes(bytes) & !wire::UNBLOCKABLE;
        let c = current_running_context();
        let end = if r[2] == 0 {
            None
        } else {
            Some(deadline(r[2], false)?)
        };
        if r[1] != 0 && !writable(runtime::user_address(r[1]), 128) {
            return Err(14);
        }
        if let Some(info) = pop(c, set) {
            if r[1] != 0 {
                output(r[1], &info.encode(false))?;
            }
            return Ok(Some(info.signal as u64));
        }
        if end
            .is_some_and(|end| deadline_reached(crate::arch::aarch64::timer::counter_value(), end))
        {
            return Err(11);
        }
        queue().insert(
            c,
            Wait {
                process: owner(c),
                address: 0,
                bitset: !0,
                deadline: end,
            },
        )?;
        thread(c).wait = Waiting::Signal { set, info: r[1] };
        local(c).timeout_result = errno(11);
        Ok(None)
    })();
    match result {
        Ok(None) => {
            runtime::complete(frame, 0);
            block(frame, ThreadState::Waiting)
        }
        Ok(Some(value)) => runtime::complete(frame, value),
        Err(error) => runtime::complete(frame, errno(error)),
    }
}
fn suspend(frame: *mut TrapFrame, r: &[u64; 31]) -> *mut TrapFrame {
    let result = (|| {
        if r[1] != 8 {
            return Err(22);
        }
        let mut bytes = [0; 8];
        runtime::copy_in(r[0], &mut bytes)?;
        let c = current_running_context();
        queue().insert(
            c,
            Wait {
                process: owner(c),
                address: 0,
                bitset: !0,
                deadline: None,
            },
        )?;
        thread(c).restore_mask = Some(local(c).signal_mask);
        local(c).signal_mask = u64::from_le_bytes(bytes) & !wire::UNBLOCKABLE;
        Ok(())
    })();
    match result {
        Ok(()) => {
            runtime::complete(frame, errno(4));
            block(frame, ThreadState::Waiting)
        }
        Err(error) => runtime::complete(frame, errno(error)),
    }
}
pub(super) fn begin_wait_mask(mask: Option<u64>) {
    if let Some(mask) = mask {
        let c = current_running_context();
        assert!(thread(c).restore_mask.is_none());
        thread(c).restore_mask = Some(local(c).signal_mask);
        local(c).signal_mask = mask & !wire::UNBLOCKABLE;
    }
}
pub(super) fn restore_wait_mask() {
    let c = current_running_context();
    if let Some(mask) = thread(c).restore_mask.take() {
        local(c).signal_mask = mask;
    }
}
fn restart_interrupted(frame: *mut TrapFrame, restart: bool) {
    let c = current_running_context();
    if let Some(call) = thread(c).interrupted.take()
        && restart
    {
        unsafe {
            (&mut (*frame).x)[..6].copy_from_slice(&call.args);
            (*frame).elr_el1 = call.pc - 4;
        }
        thread(c).restart = Some(call);
    }
}
fn deliver(frame: *mut TrapFrame, info: Info, esr: u64) -> Result<(), i64> {
    let c = current_running_context();
    let action = group(c).actions[info.signal as usize - 1];
    let restorer = if action.flags & wire::RESTORER != 0 {
        action.restorer
    } else {
        group(c).trampoline
    };
    let before = unsafe { &mut *frame };
    let stack = thread(c).stack;
    let top = if action.flags & wire::ONSTACK != 0
        && stack.flags & wire::DISABLE == 0
        && !stack.contains(before.sp_el0)
    {
        stack.address.checked_add(stack.size).ok_or(14)?
    } else {
        before.sp_el0
    };
    let sp = (top.checked_sub(wire::RECORD_BYTES as u64).ok_or(14)?) & !15;
    if stack.contains(top.saturating_sub(1)) && sp < stack.address {
        return Err(14);
    }
    if !return_valid(action.handler, sp) || !return_valid(restorer, sp) {
        return Err(14);
    }
    restart_interrupted(frame, action.flags & wire::RESTART != 0);
    let before = unsafe { &mut *frame };
    let saved = Context {
        registers: before.x,
        sp: before.sp_el0,
        pc: before.elr_el1,
        pstate: before.spsr_el1,
        vectors: before.q,
        fpsr: before.fpsr as u32,
        fpcr: before.fpcr as u32,
        mask: thread(c).restore_mask.unwrap_or(local(c).signal_mask),
        stack,
    };
    let bytes = saved.encode(info, esr, action.flags & wire::EXPOSE_TAGBITS != 0)?;
    // A handler may itself wait or receive nested signals. Arm the interrupted
    // futex only when its own sigreturn restores the original syscall site.
    let restart = thread(c)
        .restart
        .take()
        .filter(|call| saved.pc == call.pc - 4 && saved.registers[..6] == call.args);
    let slot = if restart.is_some() {
        Some(
            thread(c)
                .restart_frames
                .iter()
                .position(|entry| {
                    entry.is_none() || entry.is_some_and(|(address, _)| address == sp)
                })
                .ok_or(12)?,
        )
    } else {
        None
    };
    output(sp, &bytes)?;
    if let (Some(slot), Some(call)) = (slot, restart) {
        thread(c).restart_frames[slot] = Some((sp, call));
    }
    before.x[0] = info.signal as u64;
    before.x[1] = sp;
    before.x[2] = sp + wire::UC_OFFSET as u64;
    before.x[29] = sp + wire::FRAME_BYTES as u64;
    before.x[30] = restorer;
    before.sp_el0 = sp;
    before.elr_el1 = action.handler;
    local(c).signal_mask |= action.mask;
    if action.flags & wire::NODEFER == 0 {
        local(c).signal_mask |= wire::bit(info.signal);
    }
    local(c).signal_mask &= !wire::UNBLOCKABLE;
    if stack.flags & wire::AUTODISARM != 0 {
        thread(c).stack = AltStack::EMPTY;
    }
    if action.flags & wire::RESETHAND != 0 {
        group(c).actions[info.signal as usize - 1] = Action::EMPTY;
    }
    thread(c).restore_mask = None;
    DELIVERED.fetch_add(1, Ordering::Relaxed);
    Ok(())
}
fn fatal(frame: *mut TrapFrame, signal: u8) -> *mut TrapFrame {
    crate::kprintln!(
        "LINUX_SIGNAL_FATAL pid={} tid={} signal={}",
        owner(current_running_context()),
        current_tid(),
        signal
    );
    crate::scheduler::fault_current_user(frame, UserFaultReason::LinuxSignal, 0, 0)
}
#[unsafe(no_mangle)]
extern "C" fn linux_signal_return(frame: *mut TrapFrame) -> *mut TrapFrame {
    // Assembly has switched to this frame's kernel stack after the scheduler's
    // TTBR/current-context commit. A fatal signal returns the next frame for
    // another stack switch before any further user copies are attempted.
    if unsafe { (*frame).spsr_el1 & 0x1f } != 0
        || !current_process_id().is_some_and(crate::process::is_linux_process)
    {
        return frame;
    }
    // Binder completions can copy user buffers only here, after both TTBR and
    // SP belong to the resumed process. Signal delivery then sees the final
    // syscall result, or EINTR if the queued read was cancelled.
    super::binder::complete_ready(frame);
    if let Some(next) = super::io::complete_ready(frame) {
        // run() may reblock after another reader consumed the readiness. In
        // that case assembly must switch stacks before any further user copy.
        if next != frame {
            return next;
        }
    }
    let c = current_running_context();
    let info = loop {
        match pop(c, !local(c).signal_mask) {
            Some(info) if ignored(c, info.signal) => continue,
            result => break result,
        }
    };
    let Some(info) = info else {
        restore_wait_mask();
        restart_interrupted(frame, true);
        return frame;
    };
    let action = group(c).actions[info.signal as usize - 1];
    if action.handler == 0 || info.signal == wire::SIGKILL {
        return fatal(frame, info.signal);
    }
    if deliver(frame, info, 0).is_err() {
        return fatal(frame, wire::SIGSEGV);
    }
    frame
}
pub fn synchronous(frame: *mut TrapFrame, esr: u64, far: u64) -> Option<*mut TrapFrame> {
    if !current_process_id().is_some_and(crate::process::is_linux_process) {
        return None;
    }
    let class = (esr >> 26) & 63;
    let (signal, code) = match class {
        0x20 | 0x24 => {
            let address = runtime::user_address(far);
            let (eof, mapped) = crate::process::with_current_linux_address_space(|s| {
                (s.linux_file_eof(address), s.linux_user_mapped(address))
            });
            if eof {
                (7, 2)
            } else if esr & 0x3f == 0x21 {
                (7, 1)
            } else {
                // Privileged identity mappings are not user VMAs. A hardware
                // permission fault there is still SEGV_MAPERR to this process.
                (11, if mapped { 2 } else { 1 })
            }
        }
        0x22 | 0x26 => (7, 1),
        0x00 => (4, 1),
        0x3c => (5, 1),
        _ => return None,
    };
    let c = current_running_context();
    let action = group(c).actions[signal as usize - 1];
    if action.handler <= 1 || local(c).signal_mask & wire::bit(signal) != 0 {
        return None;
    }
    let info = Info {
        signal,
        code,
        pid: 0,
        uid: 0,
        address: match class {
            0x00 | 0x3c | 0x22 => unsafe { (*frame).elr_el1 },
            0x26 => unsafe { (*frame).sp_el0 },
            _ => far,
        },
        value: 0,
    };
    Some(if deliver(frame, info, esr).is_ok() {
        frame
    } else {
        fatal(frame, wire::SIGSEGV)
    })
}
#[inline(never)]
fn sigreturn(frame: *mut TrapFrame) -> *mut TrapFrame {
    let result: Result<(), i64> = (|| {
        let sp = unsafe { (*frame).sp_el0 };
        if !sp.is_multiple_of(16) {
            return Err(14);
        }
        let mut bytes = alloc::vec::Vec::new();
        bytes.try_reserve_exact(wire::FRAME_BYTES).map_err(|_| 12)?;
        bytes.resize(wire::FRAME_BYTES, 0);
        runtime::copy_in(sp, &mut bytes)?;
        let restored =
            Context::decode(&bytes, bndroid_kernel::linux_vm::ARENA_END).inspect_err(|_error| {
                #[cfg(feature = "linux-syscall-trace")]
                crate::kprintln!(
                    "LINUX_SIGRETURN_DECODE error={} sp={:#x} pstate={:#x} magic={:#x} size={}",
                    _error,
                    sp,
                    wire::u64_at(&bytes, 576),
                    wire::u32_at(&bytes, 592),
                    wire::u32_at(&bytes, 596)
                );
            })?;
        if !return_valid(restored.pc, restored.sp) {
            #[cfg(feature = "linux-syscall-trace")]
            crate::kprintln!(
                "LINUX_SIGRETURN_ADDRESS pc={:#x} sp={:#x}",
                restored.pc,
                restored.sp
            );
            return Err(14);
        }
        let c = current_running_context();
        let restart = thread(c)
            .restart_frames
            .iter_mut()
            .find(|slot| slot.is_some_and(|(address, _)| address == sp))
            .and_then(Option::take)
            .map(|(_, call)| call)
            .filter(|call| restored.pc == call.pc - 4 && restored.registers[..6] == call.args);
        thread(c).restart = restart;
        let target = unsafe { &mut *frame };
        target.x = restored.registers;
        target.sp_el0 = restored.sp;
        target.elr_el1 = restored.pc;
        target.spsr_el1 = restored.pstate;
        target.q = restored.vectors;
        target.fpcr = restored.fpcr as u64;
        target.fpsr = restored.fpsr as u64;
        local(c).signal_mask = restored.mask;
        thread(c).stack = restored.stack;
        RETURNED.fetch_add(1, Ordering::Relaxed);
        Ok(())
    })();
    if result.is_err() {
        fatal(frame, wire::SIGSEGV)
    } else {
        frame
    }
}
pub(super) fn evidence() {
    let pending = (DYNAMIC_USER_CONTEXT_START..CONTEXT_COUNT)
        .filter(|c| thread(*c).pending.mask() != 0)
        .count();
    let group_pending = unsafe { &*GROUPS.0.get() }
        .iter()
        .filter(|g| g.pending.mask() != 0)
        .count();
    crate::kprintln!(
        "LINUX_SIGNALS_OK delivered={} returned={} interrupted={} pending={}",
        DELIVERED.load(Ordering::Relaxed),
        RETURNED.load(Ordering::Relaxed),
        INTERRUPTED.load(Ordering::Relaxed),
        pending + group_pending
    );
    assert_eq!(pending + group_pending, 0);
}
