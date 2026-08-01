use alloc::alloc::{Layout, alloc, dealloc};
use core::cell::UnsafeCell;
use core::mem::size_of;
use core::ptr::{self, NonNull, addr_of};
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, AtomicU64, AtomicUsize, Ordering};

use crate::arch::aarch64::exceptions::TrapFrame;
pub use bndr_abi::ProcessTerminationReason as UserTerminationReason;
use bndr_abi::{ObjectSignals, PROCESS_KILLED_EXIT_CODE, Status};
use bndroid_kernel::object_wait::{
    ObjectWaitCompletionKind, ObjectWaitItem, ObjectWaitSlot, ObjectWaitToken,
};
use bndroid_kernel::round_robin::next_runnable;
use bndroid_kernel::thread_state::{ThreadState, valid_transition};
use bndroid_kernel::time::deadline_reached;
use bndroid_kernel::timer_queue::{TimerWait, TimerWaitQueue};

const KERNEL_CONTEXT_COUNT: usize = 3;
// One manager, one provider, and two distinct client processes must be able
// to run concurrently for the M18 multi-client service proof.
const DYNAMIC_USER_CONTEXT_COUNT: usize = crate::limits::DYNAMIC_PROCESS_CAPACITY;
const USER_CONTEXT_COUNT: usize = 1 + DYNAMIC_USER_CONTEXT_COUNT;
const CONTEXT_COUNT: usize = KERNEL_CONTEXT_COUNT + USER_CONTEXT_COUNT;
const MONITOR_CONTEXT: usize = 0;
const WORKER_0_CONTEXT: usize = 1;
const WORKER_1_CONTEXT: usize = 2;
const USER_INIT_CONTEXT: usize = 3;
const DYNAMIC_USER_CONTEXT_START: usize = USER_INIT_CONTEXT + 1;
const DYNAMIC_USER_CONTEXT_END: usize = DYNAMIC_USER_CONTEXT_START + DYNAMIC_USER_CONTEXT_COUNT;
const WORKER_COUNT: usize = 2;
// The unified product keeps every UI process resident while StorageServer
// decodes a 4,160-byte block wire and moves its 4,096-byte payload through the
// broker. Unoptimized builds retain several bounded by-value temporaries at
// once, so the historical 32 KiB exception stack is not sufficient. Keep the
// larger bound local to M67; the canary remains an independent overflow gate.
#[cfg(feature = "unified-product-runtime")]
const WORKER_STACK_SIZE: usize = 64 * 1024;
#[cfg(all(
    feature = "storage-server-runtime",
    not(feature = "unified-product-runtime")
))]
const WORKER_STACK_SIZE: usize = 32 * 1024;
#[cfg(not(feature = "storage-server-runtime"))]
const WORKER_STACK_SIZE: usize = 16 * 1024;
const STACK_GUARD_BYTES: usize = 16;
const STACK_CANARY: u64 = 0xbad0_c0de_51ac_cafe;
const STACK_CANARY_INVERSE: u64 = !STACK_CANARY;

const STATE_INACTIVE: u8 = ThreadState::Inactive.raw();
const STATE_RUNNABLE: u8 = ThreadState::Runnable.raw();
const STATE_RUNNING: u8 = ThreadState::Running.raw();
const STATE_SLEEPING: u8 = ThreadState::Sleeping.raw();
const STATE_FAULTED: u8 = ThreadState::Faulted.raw();
const STATE_ZOMBIE: u8 = ThreadState::Zombie.raw();
const STATE_WAITING: u8 = ThreadState::Waiting.raw();
const MIN_SLEEP_TICKS: u64 = 2;
const MAX_SLEEP_TICKS: u64 = i64::MAX as u64;

unsafe extern "C" {
    fn __scheduler_worker_0() -> !;
    fn __scheduler_worker_1() -> !;
    static __stack_bottom: u8;
    static __stack_top: u8;
    static __text_start: u8;
    static __text_end: u8;
}

#[repr(C, align(4096))]
struct KernelStack(UnsafeCell<[u8; WORKER_STACK_SIZE]>);

// A stack is mutated only by the one context to which it is permanently
// assigned. Scheduler metadata is published before IRQs are unmasked.
unsafe impl Sync for KernelStack {}

impl KernelStack {
    const fn new() -> Self {
        Self(UnsafeCell::new([0; WORKER_STACK_SIZE]))
    }

    fn range(&'static self) -> (usize, usize) {
        let bottom = self.0.get().cast::<u8>() as usize;
        (bottom, bottom + WORKER_STACK_SIZE)
    }
}

impl OwnedKernelStack {
    fn try_new() -> Option<Self> {
        let layout = Layout::new::<KernelStack>();
        let pointer = NonNull::new(unsafe { alloc(layout) }.cast::<KernelStack>())?;
        // Initialize in place. Constructing a profile-sized
        // `KernelStack::new()` temporary here could itself exhaust the current
        // exception stack before it can be moved into the heap allocation.
        unsafe { ptr::write_bytes(pointer.as_ptr().cast::<u8>(), 0, layout.size()) };
        Some(Self { pointer })
    }

    fn range(&self) -> (usize, usize) {
        let bottom = unsafe { (*self.pointer.as_ptr()).0.get().cast::<u8>() as usize };
        (bottom, bottom + WORKER_STACK_SIZE)
    }
}

impl Drop for OwnedKernelStack {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.pointer.as_ptr());
            dealloc(
                self.pointer.as_ptr().cast::<u8>(),
                Layout::new::<KernelStack>(),
            );
        }
    }
}

impl Drop for StoppedUserThread {
    fn drop(&mut self) {
        // Keep the owned stack visibly part of the stopped-thread proof until
        // the reaper has finished using its metadata.
        let _ = self.stack.range();
    }
}

struct StaticContext {
    saved_frame: AtomicPtr<TrapFrame>,
    translation: AtomicU64,
    stack_bottom: AtomicUsize,
    stack_top: AtomicUsize,
    canary_address: AtomicUsize,
    state: AtomicU8,
}

struct OwnedKernelStack {
    pointer: NonNull<KernelStack>,
}

struct DynamicStackSlot(UnsafeCell<Option<OwnedKernelStack>>);

// The slot is installed, detached, and dropped only with local IRQ masked on
// the single boot CPU. It is not an SMP synchronization primitive.
unsafe impl Sync for DynamicStackSlot {}

pub struct PendingUserThread {
    dynamic_slot: usize,
    stack: OwnedKernelStack,
}

pub struct StoppedUserThread {
    pub process_id: u64,
    pub translation: crate::arch::aarch64::mmu::TranslationContext,
    pub selections: u64,
    stack: OwnedKernelStack,
}

struct IrqTimerWaitQueue(UnsafeCell<TimerWaitQueue<WORKER_COUNT>>);

// The queue is mutated only on this single core with PSTATE.I set: either by
// the timer IRQ or by the synchronous sleep dispatcher. Snapshot masks local
// IRQ before reading it. This is deliberately not an SMP synchronization
// primitive.
unsafe impl Sync for IrqTimerWaitQueue {}

struct UserObjectWaitSlots(UnsafeCell<[ObjectWaitSlot; USER_CONTEXT_COUNT]>);

// Object waits are published, consumed, and inspected only with local IRQ
// masked on the single boot CPU. PID/handle/epoch live in one logical slot.
unsafe impl Sync for UserObjectWaitSlots {}

impl IrqTimerWaitQueue {
    const fn new() -> Self {
        Self(UnsafeCell::new(TimerWaitQueue::new()))
    }
}

impl StaticContext {
    const fn new() -> Self {
        Self {
            saved_frame: AtomicPtr::new(ptr::null_mut()),
            translation: AtomicU64::new(0),
            stack_bottom: AtomicUsize::new(0),
            stack_top: AtomicUsize::new(0),
            canary_address: AtomicUsize::new(0),
            state: AtomicU8::new(STATE_INACTIVE),
        }
    }
}

static WORKER_0_STACK: KernelStack = KernelStack::new();
static WORKER_1_STACK: KernelStack = KernelStack::new();
static USER_INIT_KERNEL_STACK: KernelStack = KernelStack::new();
static USER_DYNAMIC_KERNEL_STACKS: [DynamicStackSlot; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { DynamicStackSlot(UnsafeCell::new(None)) }; DYNAMIC_USER_CONTEXT_COUNT];
static CONTEXTS: [StaticContext; CONTEXT_COUNT] = [const { StaticContext::new() }; CONTEXT_COUNT];
static TIMER_WAIT_QUEUE: IrqTimerWaitQueue = IrqTimerWaitQueue::new();
static USER_OBJECT_WAIT_SLOTS: UserObjectWaitSlots = UserObjectWaitSlots(UnsafeCell::new(
    [const { ObjectWaitSlot::new() }; USER_CONTEXT_COUNT],
));

static ACTIVE: AtomicBool = AtomicBool::new(false);
static CURRENT_CONTEXT: AtomicUsize = AtomicUsize::new(MONITOR_CONTEXT);
static PREFERRED_CONTEXT: AtomicUsize = AtomicUsize::new(CONTEXT_COUNT);
static PREFERRED_SCANS: AtomicU64 = AtomicU64::new(0);
static PREFERRED_CANDIDATES: AtomicU64 = AtomicU64::new(0);
static PREFERRED_NO_CANDIDATE: AtomicU64 = AtomicU64::new(0);
static PREFERRED_REQUESTS: AtomicU64 = AtomicU64::new(0);
static PREFERRED_COALESCED: AtomicU64 = AtomicU64::new(0);
static PREFERRED_DISPATCHES: AtomicU64 = AtomicU64::new(0);
static PREFERRED_STALE: AtomicU64 = AtomicU64::new(0);
static TIMER_DISPATCHES: AtomicU64 = AtomicU64::new(0);
static CONTEXT_SWITCHES: AtomicU64 = AtomicU64::new(0);
static BLOCK_DISPATCHES: AtomicU64 = AtomicU64::new(0);
static MONITOR_SELECTED: AtomicU64 = AtomicU64::new(0);
static USER_INIT_SELECTED: AtomicU64 = AtomicU64::new(0);
static USER_DYNAMIC_SELECTED: [AtomicU64; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicU64::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static USER_INIT_LOWER_IRQS: AtomicU64 = AtomicU64::new(0);
static USER_DYNAMIC_LOWER_IRQS: [AtomicU64; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicU64::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static USER_DYNAMIC_EXIT_CODES: [AtomicU64; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicU64::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static USER_DYNAMIC_TERMINATION_REASONS: [AtomicU64; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicU64::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static USER_DYNAMIC_FAULT_REASONS: [AtomicU64; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicU64::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static USER_DYNAMIC_FAULT_ESRS: [AtomicU64; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicU64::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static USER_DYNAMIC_FAULT_FARS: [AtomicU64; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicU64::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static USER_DYNAMIC_FAULT_ELRS: [AtomicU64; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicU64::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static USER_DYNAMIC_FAULT_SP_EL0S: [AtomicU64; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicU64::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static USER_INIT_WAIT_PROCESS: AtomicU64 = AtomicU64::new(0);
static PROCESS_WAIT_BLOCKS: AtomicU64 = AtomicU64::new(0);
static PROCESS_WAIT_WAKES: AtomicU64 = AtomicU64::new(0);
static OBJECT_WAIT_BLOCKS: AtomicU64 = AtomicU64::new(0);
static OBJECT_WAIT_WAKES: AtomicU64 = AtomicU64::new(0);
static OBJECT_WAIT_CANCELS: AtomicU64 = AtomicU64::new(0);
static OBJECT_WAIT_ABANDONED_BY_TERMINATION: AtomicU64 = AtomicU64::new(0);
static WAIT_MANY_BLOCKS: AtomicU64 = AtomicU64::new(0);
static WAIT_MANY_WAKES: AtomicU64 = AtomicU64::new(0);
static WAIT_MANY_SIGNAL_WAKES: AtomicU64 = AtomicU64::new(0);
static WAIT_MANY_FINITE_SIGNAL_WAKES: AtomicU64 = AtomicU64::new(0);
static WAIT_MANY_TIMEOUT_WAKES: AtomicU64 = AtomicU64::new(0);
static WAIT_MANY_CANCELS: AtomicU64 = AtomicU64::new(0);
static WAIT_MANY_EARLY_FAILURES: AtomicU64 = AtomicU64::new(0);
static WAIT_MANY_ABANDONED_BY_TERMINATION: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_BLOCKS: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_WAKES: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_SIGNAL_WAKES: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_TIMEOUT_WAKES: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_CANCELS: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_EARLY_FAILURES: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_ABANDONED_BY_TERMINATION: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_MAX_ITEMS: AtomicUsize = AtomicUsize::new(0);
static WAIT_ARRAY_LATEST_EPOCH: AtomicU64 = AtomicU64::new(0);
static USER_INIT_FAULTS: AtomicU64 = AtomicU64::new(0);
static USER_INIT_FAULT_REASON: AtomicU64 = AtomicU64::new(0);
static USER_INIT_FAULT_ESR: AtomicU64 = AtomicU64::new(0);
static USER_INIT_FAULT_FAR: AtomicU64 = AtomicU64::new(0);
static USER_INIT_FAULT_ELR: AtomicU64 = AtomicU64::new(0);
static USER_INIT_FAULT_SP_EL0: AtomicU64 = AtomicU64::new(0);
static USER_SYNC_FAULT_DISPATCHES: AtomicU64 = AtomicU64::new(0);
static USER_CODE_START: AtomicUsize = AtomicUsize::new(0);
static USER_CODE_END: AtomicUsize = AtomicUsize::new(0);
static USER_STACK_START: AtomicUsize = AtomicUsize::new(0);
static USER_STACK_END: AtomicUsize = AtomicUsize::new(0);
static USER_DYNAMIC_CODE_STARTS: [AtomicUsize; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicUsize::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static USER_DYNAMIC_CODE_ENDS: [AtomicUsize; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicUsize::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static USER_DYNAMIC_STACK_STARTS: [AtomicUsize; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicUsize::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static USER_DYNAMIC_STACK_ENDS: [AtomicUsize; DYNAMIC_USER_CONTEXT_COUNT] =
    [const { AtomicUsize::new(0) }; DYNAMIC_USER_CONTEXT_COUNT];
static CONTEXT_PROCESS_IDS: [AtomicU64; CONTEXT_COUNT] =
    [const { AtomicU64::new(0) }; CONTEXT_COUNT];
static SLEEP_ENQUEUED: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static TIMER_WAKEUPS: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_BLOCK_TICK: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_READY_TICK: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_REQUEST_TICK: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_DEADLINE: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_REQUEST_COUNTER: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_COUNTER_DEADLINE: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_READY_COUNTER: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_SELECTED_BASELINE: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_OBSERVED_BASELINE: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_WORK_BASELINE: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_SELECTED_READY: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_OBSERVED_READY: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_WORK_READY: [AtomicU64; WORKER_COUNT] = [AtomicU64::new(0), AtomicU64::new(0)];
static SLEEP_FRAME_BASELINE: [AtomicUsize; WORKER_COUNT] =
    [AtomicUsize::new(0), AtomicUsize::new(0)];
static QUEUE_PEAK: AtomicUsize = AtomicUsize::new(0);
static OVERLAP_TICKS: AtomicU64 = AtomicU64::new(0);
static EARLY_WAKE_FAILURES: AtomicU64 = AtomicU64::new(0);
static BLOCKED_STATE_FAILURES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "scheduler-no-switch-self-test")]
static NO_SWITCH_ARMED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "scheduler-no-switch-self-test")]
static NO_SWITCH_INJECTED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "scheduler-blocked-run-self-test")]
static BLOCKED_RUN_INJECTED: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
pub static SCHED_WORKER_0_SELECTED: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_WORKER_1_SELECTED: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_WORKER_0_OBSERVED: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_WORKER_1_OBSERVED: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_WORKER_0_WORK: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_WORKER_1_WORK: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_REGISTER_FAILURES: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_EPOCH_FAILURES: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_SLEEP_STATUS_FAILURES: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_VALIDATION_COMPLETE: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_CURRENT_TICK: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_WORKER_0_SLEEP_RESUME_TICK: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_WORKER_1_SLEEP_RESUME_TICK: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_WORKER_0_SLEEP_RETURNS: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_WORKER_1_SLEEP_RETURNS: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_WORKER_0_SLEEP_EPOCH_BASELINE: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub static SCHED_WORKER_1_SLEEP_EPOCH_BASELINE: AtomicU64 = AtomicU64::new(0);

pub struct SchedulerInfo {
    pub context_count: usize,
    pub worker_stack_bytes: usize,
    pub monitor_stack_bytes: usize,
}

#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserFaultReason {
    InvalidReturnState = 1,
    SynchronousException = 2,
    InvalidSyscallContext = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserFaultSnapshot {
    pub count: u64,
    pub reason: u64,
    pub esr: u64,
    pub far: u64,
    pub elr: u64,
    pub sp_el0: u64,
    pub sync_dispatches: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalUserSnapshot {
    pub process_id: u64,
    pub translation: crate::arch::aarch64::mmu::TranslationContext,
    pub reason: u64,
    pub exit_code: u64,
    pub fault_reason: u64,
    pub fault_esr: u64,
    pub fault_far: u64,
    pub fault_elr: u64,
    pub fault_sp_el0: u64,
    pub code_start: usize,
    pub code_end: usize,
    pub stack_start: usize,
    pub stack_end: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminateUserError {
    NotFound,
    AlreadyTerminal,
    InvalidState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessWaitSchedulerSnapshot {
    pub blocks: u64,
    pub wakes: u64,
    pub pending_target: u64,
    pub waiting: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectWaitSchedulerSnapshot {
    pub blocks: u64,
    pub wakes: u64,
    pub cancels: u64,
    pub abandoned_by_termination: u64,
    pub pending: usize,
    pub latest_epoch: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaitManySchedulerSnapshot {
    pub blocks: u64,
    pub wakes: u64,
    pub signal_wakes: u64,
    pub finite_signal_wakes: u64,
    pub timeout_wakes: u64,
    pub cancels: u64,
    pub early_failures: u64,
    pub abandoned_by_termination: u64,
    pub pending: usize,
    pub latest_epoch: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaitArraySchedulerSnapshot {
    pub blocks: u64,
    pub wakes: u64,
    pub signal_wakes: u64,
    pub timeout_wakes: u64,
    pub cancels: u64,
    pub early_failures: u64,
    pub abandoned_by_termination: u64,
    pub pending: usize,
    pub max_items: usize,
    pub latest_epoch: u64,
}

#[cfg(feature = "storage-server-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoragePreferenceSnapshot {
    pub scans: u64,
    pub candidates: u64,
    pub no_candidate: u64,
    pub requests: u64,
    pub coalesced: u64,
    pub dispatches: u64,
    pub stale: u64,
    pub pending: bool,
}

const SLEEP_STATUS_OK: u64 = 0;
const SLEEP_STATUS_INVALID_DURATION: u64 = 1;
const SLEEP_STATUS_MONITOR: u64 = 2;
const SLEEP_STATUS_IRQ_MASKED: u64 = 3;
const SLEEP_STATUS_ALREADY_PENDING: u64 = 4;

#[derive(Clone, Copy)]
pub struct SchedulerSnapshot {
    pub timer_ticks: u64,
    pub timer_dispatches: u64,
    pub context_switches: u64,
    pub block_dispatches: u64,
    pub monitor_selected: u64,
    pub worker_selected: [u64; 2],
    pub worker_observed: [u64; 2],
    pub worker_work: [u64; 2],
    pub register_failures: u64,
    pub epoch_failures: u64,
    pub sleep_status_failures: u64,
    pub stack_failures: u64,
    pub sleep_enqueued: [u64; WORKER_COUNT],
    pub timer_wakeups: [u64; WORKER_COUNT],
    pub sleep_request_tick: [u64; WORKER_COUNT],
    pub sleep_block_tick: [u64; WORKER_COUNT],
    pub sleep_deadline: [u64; WORKER_COUNT],
    pub sleep_request_counter: [u64; WORKER_COUNT],
    pub sleep_counter_deadline: [u64; WORKER_COUNT],
    pub sleep_ready_counter: [u64; WORKER_COUNT],
    pub timer_period: u64,
    pub sleep_ready_tick: [u64; WORKER_COUNT],
    pub sleep_resume_tick: [u64; WORKER_COUNT],
    pub sleep_returns: [u64; WORKER_COUNT],
    pub sleep_epoch_baseline: [u64; WORKER_COUNT],
    pub selected_at_block: [u64; WORKER_COUNT],
    pub selected_at_ready: [u64; WORKER_COUNT],
    pub observed_at_block: [u64; WORKER_COUNT],
    pub observed_at_ready: [u64; WORKER_COUNT],
    pub work_at_block: [u64; WORKER_COUNT],
    pub work_at_ready: [u64; WORKER_COUNT],
    pub queue_len: usize,
    pub queue_peak: usize,
    pub overlap_ticks: u64,
    pub early_wake_failures: u64,
    pub blocked_state_failures: u64,
}

pub fn init() -> SchedulerInfo {
    if ACTIVE.load(Ordering::Acquire) {
        panic!("scheduler initialized twice");
    }

    reset_validation_counters();

    let kernel_translation = crate::arch::aarch64::mmu::kernel_translation_context();
    if crate::arch::aarch64::mmu::current_translation_context() != kernel_translation {
        panic!("scheduler initialized under the wrong kernel translation context");
    }
    for context in &CONTEXTS[..KERNEL_CONTEXT_COUNT] {
        context
            .translation
            .store(kernel_translation.raw(), Ordering::Relaxed);
    }
    for context in &CONTEXTS[KERNEL_CONTEXT_COUNT..] {
        context.translation.store(0, Ordering::Relaxed);
    }
    for process_id in &CONTEXT_PROCESS_IDS {
        process_id.store(0, Ordering::Relaxed);
    }
    if USER_DYNAMIC_KERNEL_STACKS
        .iter()
        .any(|slot| unsafe { (&*slot.0.get()).is_some() })
    {
        panic!("a dynamic user stack existed before scheduler initialization");
    }

    let monitor_range = (
        addr_of!(__stack_bottom) as usize,
        addr_of!(__stack_top) as usize,
    );
    let worker_0_range = WORKER_0_STACK.range();
    let worker_1_range = WORKER_1_STACK.range();
    let user_init_kernel_range = USER_INIT_KERNEL_STACK.range();
    assert_disjoint(monitor_range, worker_0_range);
    assert_disjoint(monitor_range, worker_1_range);
    assert_disjoint(worker_0_range, worker_1_range);
    assert_disjoint(monitor_range, user_init_kernel_range);
    assert_disjoint(worker_0_range, user_init_kernel_range);
    assert_disjoint(worker_1_range, user_init_kernel_range);

    configure_stack(MONITOR_CONTEXT, monitor_range);
    configure_stack(WORKER_0_CONTEXT, worker_0_range);
    configure_stack(WORKER_1_CONTEXT, worker_1_range);
    configure_stack(USER_INIT_CONTEXT, user_init_kernel_range);

    let worker_0_frame = build_initial_frame(
        worker_0_range.1,
        __scheduler_worker_0 as *const () as u64,
        0,
    );
    let worker_1_frame = build_initial_frame(
        worker_1_range.1,
        __scheduler_worker_1 as *const () as u64,
        1,
    );
    if worker_0_frame == worker_1_frame {
        panic!("scheduler workers share an initial frame");
    }

    CONTEXTS[MONITOR_CONTEXT]
        .saved_frame
        .store(ptr::null_mut(), Ordering::Relaxed);
    CONTEXTS[WORKER_0_CONTEXT]
        .saved_frame
        .store(worker_0_frame, Ordering::Relaxed);
    CONTEXTS[WORKER_1_CONTEXT]
        .saved_frame
        .store(worker_1_frame, Ordering::Relaxed);
    for context in &CONTEXTS[KERNEL_CONTEXT_COUNT..] {
        context
            .saved_frame
            .store(ptr::null_mut(), Ordering::Relaxed);
    }
    transition_state(MONITOR_CONTEXT, ThreadState::Inactive, ThreadState::Running);
    transition_state(
        WORKER_0_CONTEXT,
        ThreadState::Inactive,
        ThreadState::Runnable,
    );
    transition_state(
        WORKER_1_CONTEXT,
        ThreadState::Inactive,
        ThreadState::Runnable,
    );
    CURRENT_CONTEXT.store(MONITOR_CONTEXT, Ordering::Relaxed);

    validate_frame_or_panic(WORKER_0_CONTEXT, worker_0_frame);
    validate_frame_or_panic(WORKER_1_CONTEXT, worker_1_frame);
    ACTIVE.store(true, Ordering::Release);

    SchedulerInfo {
        context_count: CONTEXT_COUNT,
        worker_stack_bytes: WORKER_STACK_SIZE,
        monitor_stack_bytes: monitor_range.1 - monitor_range.0,
    }
}

/// Installs the first EL0 task after the fixed three-context scheduler proof.
///
/// The user task gets an independent kernel exception stack. It remains
/// inactive until this function publishes all code/stack bounds and its
/// initial EL0t TrapFrame with IRQ masked.
pub struct UserInitActivation {
    pub process_id: u64,
    pub argument0: u64,
    pub entry: usize,
    pub code_start: usize,
    pub code_end: usize,
    pub user_stack_start: usize,
    pub user_stack_end: usize,
    pub translation: crate::arch::aarch64::mmu::TranslationContext,
}

pub fn activate_user_init(activation: UserInitActivation) {
    let UserInitActivation {
        process_id,
        argument0,
        entry,
        code_start,
        code_end,
        user_stack_start,
        user_stack_end,
        translation,
    } = activation;
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let current = current_running_context();
    if !ACTIVE.load(Ordering::Acquire)
        || CONTEXTS[USER_INIT_CONTEXT].state.load(Ordering::Relaxed) != STATE_INACTIVE
        || process_id == 0
        || code_start > entry
        || entry >= code_end
        || !entry.is_multiple_of(4)
        || !code_start.is_multiple_of(4)
        || !code_end.is_multiple_of(4)
        || user_stack_start >= user_stack_end
        || !user_stack_start.is_multiple_of(16)
        || !user_stack_end.is_multiple_of(16)
        || translation.asid() != crate::arch::aarch64::mmu::INIT_USER_ASID
        || translation.root_address()
            == crate::arch::aarch64::mmu::kernel_translation_context().root_address()
        || object_wait_slots_ref()[0].is_pending()
    {
        panic!("scheduler received invalid EL0 init bounds");
    }
    assert_context_translation(current);

    USER_CODE_START.store(code_start, Ordering::Relaxed);
    USER_CODE_END.store(code_end, Ordering::Relaxed);
    USER_STACK_START.store(user_stack_start, Ordering::Relaxed);
    USER_STACK_END.store(user_stack_end, Ordering::Relaxed);
    CONTEXT_PROCESS_IDS[USER_INIT_CONTEXT].store(process_id, Ordering::Relaxed);
    let frame = build_user_frame(
        USER_INIT_KERNEL_STACK.range().1,
        entry,
        user_stack_end,
        argument0,
        bndr_abi::UserImageId::Init.raw(),
    );
    CONTEXTS[USER_INIT_CONTEXT]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    CONTEXTS[USER_INIT_CONTEXT]
        .translation
        .store(translation.raw(), Ordering::Relaxed);
    validate_frame_or_panic(USER_INIT_CONTEXT, frame);
    transition_state(
        USER_INIT_CONTEXT,
        ThreadState::Inactive,
        ThreadState::Runnable,
    );
    crate::arch::aarch64::restore_daif(saved_daif);
}

/// Allocates a private profile-bounded kernel/exception stack without
/// publishing a runnable context. The returned transaction is consumed by
/// `activate_user_process` only after the matching Process owns its address
/// space and handle table.
pub fn prepare_user_thread(dynamic_slot: usize) -> Result<PendingUserThread, ()> {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let result = ACTIVE
        .load(Ordering::Acquire)
        .then(|| dynamic_user_context_is_empty(dynamic_slot))
        .filter(|empty| *empty)
        .and_then(|_| {
            OwnedKernelStack::try_new().map(|stack| PendingUserThread {
                dynamic_slot,
                stack,
            })
        })
        .ok_or(());
    crate::arch::aarch64::restore_daif(saved_daif);
    result
}

#[allow(clippy::too_many_arguments)]
pub fn activate_user_process(
    pending: PendingUserThread,
    dynamic_slot: usize,
    process_id: u64,
    entry: usize,
    code_start: usize,
    code_end: usize,
    user_stack_start: usize,
    user_stack_end: usize,
    translation: crate::arch::aarch64::mmu::TranslationContext,
    argument0: u64,
    argument1: u64,
) {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let current = current_running_context();
    let context = dynamic_user_context(dynamic_slot)
        .unwrap_or_else(|| panic!("scheduler received an invalid dynamic EL0 slot"));
    if pending.dynamic_slot != dynamic_slot {
        panic!("prepared dynamic EL0 stack was committed to a different slot");
    }
    let wait_slot = user_wait_slot_for_context(context);
    let stack_range = pending.stack.range();
    if !ACTIVE.load(Ordering::Acquire)
        || !dynamic_user_context_is_empty(dynamic_slot)
        || process_id == 0
        || code_start > entry
        || entry >= code_end
        || !entry.is_multiple_of(4)
        || !code_start.is_multiple_of(4)
        || !code_end.is_multiple_of(4)
        || user_stack_start >= user_stack_end
        || !user_stack_start.is_multiple_of(16)
        || !user_stack_end.is_multiple_of(16)
        || translation.asid() == crate::arch::aarch64::mmu::KERNEL_ASID
        || translation.root_address()
            == crate::arch::aarch64::mmu::kernel_translation_context().root_address()
        || CONTEXT_PROCESS_IDS
            .iter()
            .any(|candidate| candidate.load(Ordering::Relaxed) == process_id)
        || object_wait_slots_ref()[wait_slot].is_pending()
    {
        panic!("scheduler received invalid dynamic EL0 process bounds");
    }
    let stack_slot = dynamic_stack_slot_mut(dynamic_slot);
    assert_context_translation(current);
    for context in &CONTEXTS {
        let bottom = context.canary_address.load(Ordering::Relaxed);
        let top = context.stack_top.load(Ordering::Relaxed);
        if bottom != 0 {
            assert_disjoint((bottom, top), stack_range);
        }
    }

    configure_stack(context, stack_range);
    USER_DYNAMIC_CODE_STARTS[dynamic_slot].store(code_start, Ordering::Relaxed);
    USER_DYNAMIC_CODE_ENDS[dynamic_slot].store(code_end, Ordering::Relaxed);
    USER_DYNAMIC_STACK_STARTS[dynamic_slot].store(user_stack_start, Ordering::Relaxed);
    USER_DYNAMIC_STACK_ENDS[dynamic_slot].store(user_stack_end, Ordering::Relaxed);
    USER_DYNAMIC_SELECTED[dynamic_slot].store(0, Ordering::Relaxed);
    USER_DYNAMIC_LOWER_IRQS[dynamic_slot].store(0, Ordering::Relaxed);
    USER_DYNAMIC_EXIT_CODES[dynamic_slot].store(0, Ordering::Relaxed);
    USER_DYNAMIC_TERMINATION_REASONS[dynamic_slot].store(0, Ordering::Relaxed);
    clear_dynamic_user_fault_witness(dynamic_slot);
    CONTEXT_PROCESS_IDS[context].store(process_id, Ordering::Relaxed);
    let frame = build_user_frame(stack_range.1, entry, user_stack_end, argument0, argument1);
    CONTEXTS[context]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    CONTEXTS[context]
        .translation
        .store(translation.raw(), Ordering::Relaxed);
    *stack_slot = Some(pending.stack);
    validate_frame_or_panic(context, frame);
    transition_state(context, ThreadState::Inactive, ThreadState::Runnable);
    crate::arch::aarch64::restore_daif(saved_daif);
}

pub fn validate_current_user_frame(frame: *mut TrapFrame) -> bool {
    if !ACTIVE.load(Ordering::Acquire) {
        return false;
    }
    let current = current_running_context();
    if !is_user_context(current) || CONTEXT_PROCESS_IDS[current].load(Ordering::Relaxed) == 0 {
        return false;
    }
    validate_frame_storage_or_panic(current, frame);
    validate_user_kernel_state_or_panic(current, frame);
    user_return_state_is_valid(current, frame)
}

pub fn current_process_id() -> Option<u64> {
    if !ACTIVE.load(Ordering::Acquire) {
        return None;
    }
    let current = current_running_context();
    is_user_context(current).then(|| CONTEXT_PROCESS_IDS[current].load(Ordering::Relaxed))
}

/// Authenticates the single-core monitor context used by M61's kernel-owned
/// StorageServer retirement ticket. This never grants an EL0 caller authority.
#[cfg(feature = "storage-server-owner-liveness-runtime")]
pub fn storage_watchdog_monitor_active() -> bool {
    crate::arch::aarch64::irq_is_masked()
        && current_running_context() == MONITOR_CONTEXT
        && CONTEXTS[MONITOR_CONTEXT].state.load(Ordering::Relaxed) == STATE_RUNNING
        && crate::arch::aarch64::mmu::current_translation_context()
            == crate::arch::aarch64::mmu::kernel_translation_context()
}

/// Reports whether one generation-qualified user process can still return to
/// EL0 and issue the exact retry that owns an asynchronous kernel completion.
/// Terminal and detached contexts are deliberately excluded.
#[cfg(any(feature = "app-data-runtime", feature = "androidbox-apk-install0"))]
pub fn user_process_can_retry_irq_masked(process_id: u64) -> bool {
    if !crate::arch::aarch64::irq_is_masked() || process_id == 0 {
        panic!("user completion liveness queried outside the IRQ-masked domain");
    }
    (USER_INIT_CONTEXT..CONTEXT_COUNT).any(|context| {
        if CONTEXT_PROCESS_IDS[context].load(Ordering::Relaxed) != process_id {
            return false;
        }
        matches!(
            CONTEXTS[context].state.load(Ordering::Relaxed),
            STATE_RUNNABLE | STATE_RUNNING | STATE_SLEEPING | STATE_WAITING
        )
    })
}

pub fn current_user_translation() -> Option<crate::arch::aarch64::mmu::TranslationContext> {
    if !ACTIVE.load(Ordering::Acquire) {
        return None;
    }
    let current = current_running_context();
    is_user_context(current).then(|| context_translation(current))
}

/// Blocks init on one generation-qualified dynamic process. The caller has
/// already validated that the target is live while local IRQ is masked, so a
/// target cannot exit between validation and publication of this wait token.
pub fn wait_current_user_process(frame: *mut TrapFrame, target: u64) -> *mut TrapFrame {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("scheduler process-wait dispatch entered with IRQ enabled");
    }
    let current = current_running_context();
    if current != USER_INIT_CONTEXT
        || target == 0
        || USER_INIT_WAIT_PROCESS.load(Ordering::Relaxed) != 0
        || object_wait_slots_ref()[0].is_pending()
        || CONTEXTS[USER_INIT_CONTEXT].state.load(Ordering::Relaxed) != STATE_RUNNING
    {
        panic!("scheduler received an invalid process wait");
    }
    validate_frame_or_panic(current, frame);
    CONTEXTS[current]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    USER_INIT_WAIT_PROCESS.store(target, Ordering::Relaxed);
    PROCESS_WAIT_BLOCKS.fetch_add(1, Ordering::Relaxed);
    transition_state(current, ThreadState::Running, ThreadState::Waiting);
    select_next_context(current)
}

/// Completes a previously published process wait after the monitor has
/// detached the terminal thread and destroyed its address space. Returning
/// `false` means no waiter exists for this exact generation-qualified PID, so
/// the process manager must retain a completion tombstone instead.
pub fn wake_user_process_wait(target: u64, exit_code: u64, termination_reason: u64) -> bool {
    if !crate::arch::aarch64::irq_is_masked()
        || UserTerminationReason::from_raw(termination_reason).is_none()
    {
        panic!("scheduler process-wait wake invariants failed");
    }
    let current = current_running_context();
    if current != MONITOR_CONTEXT {
        panic!("scheduler process waiter may only be woken by the monitor");
    }
    let state = CONTEXTS[USER_INIT_CONTEXT].state.load(Ordering::Relaxed);
    let pending = USER_INIT_WAIT_PROCESS.load(Ordering::Relaxed);
    if pending == 0 {
        return false;
    }
    if pending != target {
        return false;
    }
    if state != STATE_WAITING {
        panic!("scheduler process-wait token existed without a waiting thread");
    }
    let frame = CONTEXTS[USER_INIT_CONTEXT]
        .saved_frame
        .load(Ordering::Relaxed);
    validate_frame_or_panic(USER_INIT_CONTEXT, frame);
    crate::syscall::complete_process_wait(frame, exit_code, termination_reason);
    USER_INIT_WAIT_PROCESS.store(0, Ordering::Relaxed);
    transition_state(
        USER_INIT_CONTEXT,
        ThreadState::Waiting,
        ThreadState::Runnable,
    );
    PROCESS_WAIT_WAKES.fetch_add(1, Ordering::Release);
    true
}

pub fn process_wait_snapshot() -> ProcessWaitSchedulerSnapshot {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let pending_target = USER_INIT_WAIT_PROCESS.load(Ordering::Acquire);
    let object_wait_pending = object_wait_slots_ref()[0].is_pending();
    let thread_waiting = CONTEXTS[USER_INIT_CONTEXT].state.load(Ordering::Relaxed) == STATE_WAITING;
    if pending_target != 0 && object_wait_pending {
        panic!("init published process-wait and object-wait tokens together");
    }
    if thread_waiting != (pending_target != 0 || object_wait_pending) {
        panic!("init waiting state disagrees with its exact wait token");
    }
    let snapshot = ProcessWaitSchedulerSnapshot {
        blocks: PROCESS_WAIT_BLOCKS.load(Ordering::Acquire),
        wakes: PROCESS_WAIT_WAKES.load(Ordering::Acquire),
        pending_target,
        waiting: pending_target != 0,
    };
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

/// Blocks the currently running user context on one exact generation-qualified
/// handle. The syscall has already polled the object while local IRQ is
/// masked, so no channel mutation can occur before this token is published.
pub fn wait_current_user_object(
    frame: *mut TrapFrame,
    raw_handle: u64,
    requested: ObjectSignals,
) -> *mut TrapFrame {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("scheduler object-wait dispatch entered with IRQ enabled");
    }
    let current = current_running_context();
    let Some(slot) = user_wait_slot(current) else {
        panic!("kernel context attempted an object wait");
    };
    let process_id = CONTEXT_PROCESS_IDS[current].load(Ordering::Relaxed);
    if raw_handle == 0
        || requested.bits() == 0
        || process_id == 0
        || object_wait_slots_ref()[slot].is_pending()
        || CONTEXTS[current].state.load(Ordering::Relaxed) != STATE_RUNNING
        || (current == USER_INIT_CONTEXT && USER_INIT_WAIT_PROCESS.load(Ordering::Relaxed) != 0)
    {
        panic!("scheduler received an invalid object wait");
    }
    validate_frame_or_panic(current, frame);
    CONTEXTS[current]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    object_wait_slots_mut()[slot]
        .publish(process_id, raw_handle, requested)
        .unwrap_or_else(|_| panic!("scheduler could not publish a validated object wait"));
    OBJECT_WAIT_BLOCKS.fetch_add(1, Ordering::Relaxed);
    transition_state(current, ThreadState::Running, ThreadState::Waiting);
    select_next_context(current)
}

/// Blocks the current user context on a fixed two-item wait-any token. The
/// syscall has already validated both handles, signal masks, and the optional
/// serial-number-safe absolute deadline while local IRQ remained masked.
pub fn wait_current_user_objects(
    frame: *mut TrapFrame,
    items: &[ObjectWaitItem; bndr_abi::OBJECT_WAIT_MANY_ITEM_COUNT],
    deadline: Option<u64>,
) -> *mut TrapFrame {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("scheduler wait-many dispatch entered with IRQ enabled");
    }
    let current = current_running_context();
    let Some(slot) = user_wait_slot(current) else {
        panic!("kernel context attempted a wait-many operation");
    };
    let process_id = CONTEXT_PROCESS_IDS[current].load(Ordering::Relaxed);
    if process_id == 0
        || items
            .iter()
            .any(|item| item.raw_handle() == 0 || item.requested().bits() == 0)
        || object_wait_slots_ref()[slot].is_pending()
        || CONTEXTS[current].state.load(Ordering::Relaxed) != STATE_RUNNING
        || (current == USER_INIT_CONTEXT && USER_INIT_WAIT_PROCESS.load(Ordering::Relaxed) != 0)
    {
        panic!("scheduler received an invalid wait-many request");
    }
    validate_frame_or_panic(current, frame);
    CONTEXTS[current]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    object_wait_slots_mut()[slot]
        .publish_many(process_id, items, deadline)
        .unwrap_or_else(|_| panic!("scheduler could not publish a validated wait-many token"));
    WAIT_MANY_BLOCKS.fetch_add(1, Ordering::Relaxed);
    transition_state(current, ThreadState::Running, ThreadState::Waiting);
    select_next_context(current)
}

/// Blocks the current user context on the variable-length wait-array ABI.
///
/// The syscall decoder has already copied and validated the bounded user
/// array while local IRQ remained masked. The explicit array completion kind
/// is retained even for one or two items, so wakeup cannot accidentally use a
/// legacy syscall result layout.
pub fn wait_current_user_object_array(
    frame: *mut TrapFrame,
    items: &[ObjectWaitItem],
    deadline: Option<u64>,
) -> *mut TrapFrame {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("scheduler wait-array dispatch entered with IRQ enabled");
    }
    let current = current_running_context();
    let Some(slot) = user_wait_slot(current) else {
        panic!("kernel context attempted a wait-array operation");
    };
    let process_id = CONTEXT_PROCESS_IDS[current].load(Ordering::Relaxed);
    if process_id == 0
        || items.is_empty()
        || items.len() > bndr_abi::OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS
        || items
            .iter()
            .any(|item| item.raw_handle() == 0 || item.requested().bits() == 0)
        || object_wait_slots_ref()[slot].is_pending()
        || CONTEXTS[current].state.load(Ordering::Relaxed) != STATE_RUNNING
        || (current == USER_INIT_CONTEXT && USER_INIT_WAIT_PROCESS.load(Ordering::Relaxed) != 0)
    {
        panic!("scheduler received an invalid wait-array request");
    }
    validate_frame_or_panic(current, frame);
    CONTEXTS[current]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    let token = object_wait_slots_mut()[slot]
        .publish_array(process_id, items, deadline)
        .unwrap_or_else(|_| panic!("scheduler could not publish a validated wait-array token"));
    WAIT_ARRAY_BLOCKS.fetch_add(1, Ordering::Relaxed);
    WAIT_ARRAY_MAX_ITEMS.fetch_max(items.len(), Ordering::Relaxed);
    WAIT_ARRAY_LATEST_EPOCH.fetch_max(token.epoch(), Ordering::Relaxed);
    transition_state(current, ThreadState::Running, ThreadState::Waiting);
    select_next_context(current)
}

/// Re-polls every bounded user wait token after a committed Channel mutation.
/// Exact PID and raw-handle generations are resolved through the owning
/// process table, preventing an old token from waking on slot reuse.
pub fn wake_object_waiters() -> usize {
    wake_object_waiters_inner(false)
}

/// Re-polls object waits and gives the first context made runnable a one-shot
/// preference at the next normal exception-return scheduling boundary.
///
/// M55 uses this only after completing a monitor-side block request. It does
/// not switch stacks here and therefore cannot bypass TrapFrame, translation,
/// or return-state validation.
#[cfg(feature = "storage-server-runtime")]
pub fn wake_object_waiters_for_storage() -> usize {
    wake_object_waiters_inner(true)
}

fn wake_object_waiters_inner(prefer_first_woken: bool) -> usize {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("scheduler object-wait wake entered with IRQ enabled");
    }
    let mut woke = 0;
    let mut first_woken = None;
    for slot in 0..USER_CONTEXT_COUNT {
        let Some(token) = object_wait_slots_ref()[slot].token() else {
            continue;
        };
        let context = USER_INIT_CONTEXT + slot;
        if CONTEXT_PROCESS_IDS[context].load(Ordering::Relaxed) != token.process_id()
            || CONTEXTS[context].state.load(Ordering::Relaxed) != STATE_WAITING
        {
            panic!("object wait token lost its context identity");
        }
        // This one counter sample is the linearization point for a finite
        // wait. A reached hard deadline wins over a readiness or invalidation
        // observed by a later signal/close scan, even if its timer IRQ was
        // delayed while another syscall held the single-core IRQ-masked domain.
        let arbitrated_at = crate::arch::aarch64::timer::counter_value();
        if token
            .deadline()
            .is_some_and(|deadline| bndroid_kernel::time::deadline_reached(arbitrated_at, deadline))
        {
            complete_object_wait_slot(
                slot,
                context,
                token,
                Status::Timeout,
                u64::MAX,
                ObjectSignals::NONE,
                arbitrated_at,
            );
            if prefer_first_woken && first_woken.is_none() {
                first_woken = Some(context);
            }
            woke += 1;
            continue;
        }
        let mut ready = None;
        let mut invalid = None;
        for (index, item) in token.items().iter().copied().enumerate() {
            match crate::process::poll_object_signals(token.process_id(), item.raw_handle()) {
                Ok(observed) if observed.intersects(item.requested()) => {
                    if ready.is_none() {
                        ready = Some((index, observed));
                    }
                }
                Ok(_) => {}
                Err(crate::process::ObjectPollError::NotFound) => {
                    if invalid.is_none() {
                        invalid = Some(index);
                    }
                }
                Err(crate::process::ObjectPollError::PermissionDenied) => {
                    panic!("published object wait lost WAIT rights")
                }
            }
        }

        let (status, index, observed) = if let Some((index, observed)) = ready {
            (Status::Ok, index as u64, observed)
        } else if let Some(index) = invalid {
            (Status::NotFound, index as u64, ObjectSignals::NONE)
        } else {
            continue;
        };
        complete_object_wait_slot(slot, context, token, status, index, observed, arbitrated_at);
        if prefer_first_woken && first_woken.is_none() {
            first_woken = Some(context);
        }
        woke += 1;
    }
    if prefer_first_woken {
        PREFERRED_SCANS.fetch_add(1, Ordering::Relaxed);
        if let Some(context) = first_woken {
            PREFERRED_CANDIDATES.fetch_add(1, Ordering::Relaxed);
            if PREFERRED_CONTEXT
                .compare_exchange(CONTEXT_COUNT, context, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                PREFERRED_REQUESTS.fetch_add(1, Ordering::Relaxed);
            } else {
                // A previous one-shot preference is still pending. The newly
                // runnable context remains on the ordinary runnable ring, so
                // this is intentional coalescing rather than a lost wakeup.
                PREFERRED_COALESCED.fetch_add(1, Ordering::Relaxed);
            }
        } else {
            // A block completion may race ahead of StorageTake's wait syscall.
            // In that valid fast path the capability is already readable and
            // no blocked context exists to prefer.
            PREFERRED_NO_CANDIDATE.fetch_add(1, Ordering::Relaxed);
        }
    }
    woke
}

pub fn object_wait_snapshot() -> ObjectWaitSchedulerSnapshot {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let mut pending = 0;
    let mut latest_epoch = 0;
    for slot in object_wait_slots_ref() {
        // This legacy aggregate intentionally covers every object-token kind;
        // resident-topology validation uses it as the total blocked-token
        // count. Kind-specific ledgers live in wait_many_snapshot and
        // wait_array_snapshot.
        pending += usize::from(slot.is_pending());
        latest_epoch = latest_epoch.max(slot.latest_epoch());
    }
    let snapshot = ObjectWaitSchedulerSnapshot {
        blocks: OBJECT_WAIT_BLOCKS.load(Ordering::Acquire),
        wakes: OBJECT_WAIT_WAKES.load(Ordering::Acquire),
        cancels: OBJECT_WAIT_CANCELS.load(Ordering::Acquire),
        abandoned_by_termination: OBJECT_WAIT_ABANDONED_BY_TERMINATION.load(Ordering::Acquire),
        pending,
        latest_epoch,
    };
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

/// Returns the one pending object-wait token owned by `process_id`.
///
/// The caller must keep local IRQ masked while it resolves the token's
/// generation-qualified handles through the process table. Returning a copy
/// rather than a slot or TrapFrame reference prevents scheduler internals from
/// escaping that single-core serialization domain.
pub fn pending_object_wait_token_irq_masked(process_id: u64) -> Option<ObjectWaitToken> {
    if !crate::arch::aarch64::irq_is_masked() || process_id == 0 {
        panic!("pending object-wait token queried outside its IRQ-masked domain");
    }
    let mut found = None;
    for token in object_wait_slots_ref()
        .iter()
        .filter_map(ObjectWaitSlot::token)
    {
        if token.process_id() != process_id {
            continue;
        }
        if found.replace(token).is_some() {
            panic!("one process owned multiple pending object-wait tokens");
        }
    }
    found
}

pub fn wait_many_snapshot() -> WaitManySchedulerSnapshot {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let mut pending = 0;
    let mut latest_epoch = 0;
    for slot in object_wait_slots_ref() {
        if slot
            .token()
            .is_some_and(|token| token.completion_kind() == ObjectWaitCompletionKind::LegacyMany)
        {
            pending += 1;
        }
        latest_epoch = latest_epoch.max(slot.latest_epoch());
    }
    let snapshot = WaitManySchedulerSnapshot {
        blocks: WAIT_MANY_BLOCKS.load(Ordering::Acquire),
        wakes: WAIT_MANY_WAKES.load(Ordering::Acquire),
        signal_wakes: WAIT_MANY_SIGNAL_WAKES.load(Ordering::Acquire),
        finite_signal_wakes: WAIT_MANY_FINITE_SIGNAL_WAKES.load(Ordering::Acquire),
        timeout_wakes: WAIT_MANY_TIMEOUT_WAKES.load(Ordering::Acquire),
        cancels: WAIT_MANY_CANCELS.load(Ordering::Acquire),
        early_failures: WAIT_MANY_EARLY_FAILURES.load(Ordering::Acquire),
        abandoned_by_termination: WAIT_MANY_ABANDONED_BY_TERMINATION.load(Ordering::Acquire),
        pending,
        latest_epoch,
    };
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

pub fn wait_array_snapshot() -> WaitArraySchedulerSnapshot {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let pending = object_wait_slots_ref()
        .iter()
        .filter(|slot| {
            slot.token()
                .is_some_and(|token| token.completion_kind() == ObjectWaitCompletionKind::Array)
        })
        .count();
    let snapshot = WaitArraySchedulerSnapshot {
        blocks: WAIT_ARRAY_BLOCKS.load(Ordering::Acquire),
        wakes: WAIT_ARRAY_WAKES.load(Ordering::Acquire),
        signal_wakes: WAIT_ARRAY_SIGNAL_WAKES.load(Ordering::Acquire),
        timeout_wakes: WAIT_ARRAY_TIMEOUT_WAKES.load(Ordering::Acquire),
        cancels: WAIT_ARRAY_CANCELS.load(Ordering::Acquire),
        early_failures: WAIT_ARRAY_EARLY_FAILURES.load(Ordering::Acquire),
        abandoned_by_termination: WAIT_ARRAY_ABANDONED_BY_TERMINATION.load(Ordering::Acquire),
        pending,
        max_items: WAIT_ARRAY_MAX_ITEMS.load(Ordering::Acquire),
        latest_epoch: WAIT_ARRAY_LATEST_EPOCH.load(Ordering::Acquire),
    };
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

fn complete_object_wait_slot(
    slot: usize,
    context: usize,
    token: bndroid_kernel::object_wait::ObjectWaitToken,
    status: Status,
    index: u64,
    observed: ObjectSignals,
    arbitrated_at: u64,
) {
    let consumed = object_wait_slots_mut()[slot].take_exact_token(&token);
    if consumed != Some(token) {
        panic!("object wait completion lost its exact token");
    }
    let frame = CONTEXTS[context].saved_frame.load(Ordering::Relaxed);
    validate_frame_or_panic(context, frame);
    match token.completion_kind() {
        ObjectWaitCompletionKind::Single => {
            if token.count() != 1 || token.deadline().is_some() {
                panic!("single object wait completion used an impossible token shape");
            }
            if status == Status::NotFound {
                OBJECT_WAIT_CANCELS.fetch_add(1, Ordering::Relaxed);
            }
            crate::syscall::complete_object_wait(frame, token.process_id(), status, observed);
            OBJECT_WAIT_WAKES.fetch_add(1, Ordering::Release);
        }
        ObjectWaitCompletionKind::LegacyMany => {
            if token.count() == 0 || token.count() > bndr_abi::OBJECT_WAIT_MANY_ITEM_COUNT {
                panic!("legacy wait-many completion used an impossible token shape");
            }
            match status {
                Status::Ok => {
                    WAIT_MANY_SIGNAL_WAKES.fetch_add(1, Ordering::Relaxed);
                    if let Some(deadline) = token.deadline() {
                        if bndroid_kernel::time::deadline_reached(arbitrated_at, deadline) {
                            panic!("finite wait-many signal completed after its hard deadline");
                        }
                        WAIT_MANY_FINITE_SIGNAL_WAKES.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Status::Timeout => {
                    let deadline = token
                        .deadline()
                        .unwrap_or_else(|| panic!("unbounded wait-many completed as timeout"));
                    if !bndroid_kernel::time::deadline_reached(arbitrated_at, deadline) {
                        WAIT_MANY_EARLY_FAILURES.fetch_add(1, Ordering::Relaxed);
                        panic!("finite wait-many completed before its hard deadline");
                    }
                    WAIT_MANY_TIMEOUT_WAKES.fetch_add(1, Ordering::Relaxed);
                }
                Status::NotFound => {
                    WAIT_MANY_CANCELS.fetch_add(1, Ordering::Relaxed);
                }
                _ => panic!("wait-many completion used an impossible status"),
            }
            crate::syscall::complete_object_wait_many(
                frame,
                token.process_id(),
                status,
                index,
                observed,
            );
            WAIT_MANY_WAKES.fetch_add(1, Ordering::Release);
        }
        ObjectWaitCompletionKind::Array => {
            if token.count() == 0 || token.count() > bndr_abi::OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS {
                panic!("wait-array completion used an impossible token shape");
            }
            match status {
                Status::Ok => {
                    WAIT_ARRAY_SIGNAL_WAKES.fetch_add(1, Ordering::Relaxed);
                    if token.deadline().is_some_and(|deadline| {
                        bndroid_kernel::time::deadline_reached(arbitrated_at, deadline)
                    }) {
                        panic!("finite wait-array signal completed after its hard deadline");
                    }
                }
                Status::Timeout => {
                    let deadline = token
                        .deadline()
                        .unwrap_or_else(|| panic!("unbounded wait-array completed as timeout"));
                    if !bndroid_kernel::time::deadline_reached(arbitrated_at, deadline) {
                        WAIT_ARRAY_EARLY_FAILURES.fetch_add(1, Ordering::Relaxed);
                        panic!("finite wait-array completed before its hard deadline");
                    }
                    WAIT_ARRAY_TIMEOUT_WAKES.fetch_add(1, Ordering::Relaxed);
                }
                Status::NotFound => {
                    WAIT_ARRAY_CANCELS.fetch_add(1, Ordering::Relaxed);
                }
                _ => panic!("wait-array completion used an impossible status"),
            }
            crate::syscall::complete_object_wait_many_array(
                frame,
                token.process_id(),
                status,
                index,
                observed,
            );
            WAIT_ARRAY_WAKES.fetch_add(1, Ordering::Release);
        }
    }
    transition_state(context, ThreadState::Waiting, ThreadState::Runnable);
}

const fn user_wait_slot(context: usize) -> Option<usize> {
    if context >= USER_INIT_CONTEXT && context < CONTEXT_COUNT {
        Some(context - USER_INIT_CONTEXT)
    } else {
        None
    }
}

fn user_wait_slot_for_context(context: usize) -> usize {
    user_wait_slot(context).unwrap_or_else(|| panic!("user context lacked a wait-token slot"))
}

const fn dynamic_user_context(dynamic_slot: usize) -> Option<usize> {
    if dynamic_slot < DYNAMIC_USER_CONTEXT_COUNT {
        Some(DYNAMIC_USER_CONTEXT_START + dynamic_slot)
    } else {
        None
    }
}

const fn dynamic_user_index(context: usize) -> Option<usize> {
    if context >= DYNAMIC_USER_CONTEXT_START && context < DYNAMIC_USER_CONTEXT_END {
        Some(context - DYNAMIC_USER_CONTEXT_START)
    } else {
        None
    }
}

/// Reports whether one explicit dynamic scheduler slot owns no context state,
/// wait token, or kernel stack. The read masks local IRQ itself; process-slot
/// selection already holds that mask so the result stays stable through stack
/// preparation.
pub fn dynamic_user_context_is_empty(dynamic_slot: usize) -> bool {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let empty = dynamic_user_context_is_empty_irq_masked(dynamic_slot);
    crate::arch::aarch64::restore_daif(saved_daif);
    empty
}

#[cfg(any(
    not(feature = "el0-fault-containment-self-test"),
    feature = "storage-server-shutdown-orchestration-runtime"
))]
pub fn live_dynamic_user_context_count() -> usize {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let count = (0..DYNAMIC_USER_CONTEXT_COUNT)
        .filter(|slot| !dynamic_user_context_is_empty_irq_masked(*slot))
        .count();
    crate::arch::aarch64::restore_daif(saved_daif);
    count
}

#[cfg(any(
    not(feature = "el0-fault-containment-self-test"),
    feature = "storage-server-shutdown-orchestration-runtime"
))]
pub fn live_dynamic_kernel_stack_count() -> usize {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let count = (0..DYNAMIC_USER_CONTEXT_COUNT)
        .filter(|slot| dynamic_stack_slot_ref(*slot).is_some())
        .count();
    crate::arch::aarch64::restore_daif(saved_daif);
    count
}

fn dynamic_user_context_is_empty_irq_masked(dynamic_slot: usize) -> bool {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("dynamic user context availability checked with IRQ enabled");
    }
    if !ACTIVE.load(Ordering::Acquire) {
        return false;
    }
    let Some(context) = dynamic_user_context(dynamic_slot) else {
        return false;
    };
    let scheduler_context = &CONTEXTS[context];
    CONTEXT_PROCESS_IDS[context].load(Ordering::Relaxed) == 0
        && scheduler_context.state.load(Ordering::Relaxed) == STATE_INACTIVE
        && scheduler_context
            .saved_frame
            .load(Ordering::Relaxed)
            .is_null()
        && scheduler_context.translation.load(Ordering::Relaxed) == 0
        && scheduler_context.stack_bottom.load(Ordering::Relaxed) == 0
        && scheduler_context.stack_top.load(Ordering::Relaxed) == 0
        && scheduler_context.canary_address.load(Ordering::Relaxed) == 0
        && USER_DYNAMIC_CODE_STARTS[dynamic_slot].load(Ordering::Relaxed) == 0
        && USER_DYNAMIC_CODE_ENDS[dynamic_slot].load(Ordering::Relaxed) == 0
        && USER_DYNAMIC_STACK_STARTS[dynamic_slot].load(Ordering::Relaxed) == 0
        && USER_DYNAMIC_STACK_ENDS[dynamic_slot].load(Ordering::Relaxed) == 0
        && dynamic_stack_slot_ref(dynamic_slot).is_none()
        && !object_wait_slots_ref()[user_wait_slot_for_context(context)].is_pending()
}

fn dynamic_stack_slot_mut(dynamic_slot: usize) -> &'static mut Option<OwnedKernelStack> {
    let slot = USER_DYNAMIC_KERNEL_STACKS
        .get(dynamic_slot)
        .unwrap_or_else(|| panic!("invalid dynamic user stack slot"));
    unsafe { &mut *slot.0.get() }
}

fn dynamic_stack_slot_ref(dynamic_slot: usize) -> &'static Option<OwnedKernelStack> {
    let slot = USER_DYNAMIC_KERNEL_STACKS
        .get(dynamic_slot)
        .unwrap_or_else(|| panic!("invalid dynamic user stack slot"));
    unsafe { &*slot.0.get() }
}

/// Proves that a nested synchronous exception can be saved on the current
/// context's kernel stack without reaching its guard/canary region.
pub fn assert_current_kernel_stack_headroom(stack_pointer: usize, required: usize) {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("kernel-stack headroom checked with IRQ enabled");
    }
    let current = current_running_context();
    let context = &CONTEXTS[current];
    let bottom = context.stack_bottom.load(Ordering::Relaxed);
    let top = context.stack_top.load(Ordering::Relaxed);
    let minimum = bottom
        .checked_add(required)
        .unwrap_or_else(|| panic!("kernel-stack headroom overflow"));
    if required < size_of::<TrapFrame>()
        || !stack_pointer.is_multiple_of(16)
        || stack_pointer < minimum
        || stack_pointer > top
        || !stack_canary_intact(context)
    {
        panic!("kernel stack lacks nested-exception headroom");
    }
}

pub fn user_init_selection_count() -> u64 {
    USER_INIT_SELECTED.load(Ordering::Acquire)
}

pub fn user_init_lower_irq_count() -> u64 {
    USER_INIT_LOWER_IRQS.load(Ordering::Acquire)
}

pub fn user_init_fault_snapshot() -> UserFaultSnapshot {
    let count = USER_INIT_FAULTS.load(Ordering::Acquire);
    UserFaultSnapshot {
        count,
        reason: USER_INIT_FAULT_REASON.load(Ordering::Relaxed),
        esr: USER_INIT_FAULT_ESR.load(Ordering::Relaxed),
        far: USER_INIT_FAULT_FAR.load(Ordering::Relaxed),
        elr: USER_INIT_FAULT_ELR.load(Ordering::Relaxed),
        sp_el0: USER_INIT_FAULT_SP_EL0.load(Ordering::Relaxed),
        sync_dispatches: USER_SYNC_FAULT_DISPATCHES.load(Ordering::Relaxed),
    }
}

fn clear_dynamic_user_fault_witness(dynamic_slot: usize) {
    USER_DYNAMIC_FAULT_REASONS[dynamic_slot].store(0, Ordering::Relaxed);
    USER_DYNAMIC_FAULT_ESRS[dynamic_slot].store(0, Ordering::Relaxed);
    USER_DYNAMIC_FAULT_FARS[dynamic_slot].store(0, Ordering::Relaxed);
    USER_DYNAMIC_FAULT_ELRS[dynamic_slot].store(0, Ordering::Relaxed);
    USER_DYNAMIC_FAULT_SP_EL0S[dynamic_slot].store(0, Ordering::Relaxed);
}

/// Removes a faulting EL0 init task and returns a runnable kernel frame.
///
/// User-controlled ELR_EL1 and SP_EL0 values are evidence about the task, not
/// kernel invariants. The exception-frame pointer and kernel stack canary are
/// still privileged state and remain fail-stop checked.
pub fn fault_current_user(
    frame: *mut TrapFrame,
    reason: UserFaultReason,
    esr: u64,
    far: u64,
) -> *mut TrapFrame {
    fault_current_user_inner(frame, reason, esr, far, true)
}

fn fault_current_user_after_timer(
    frame: *mut TrapFrame,
    reason: UserFaultReason,
) -> *mut TrapFrame {
    fault_current_user_inner(frame, reason, 0, 0, false)
}

fn fault_current_user_inner(
    frame: *mut TrapFrame,
    reason: UserFaultReason,
    esr: u64,
    far: u64,
    synchronous_dispatch: bool,
) -> *mut TrapFrame {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("scheduler attempted to contain an EL0 fault with IRQ enabled");
    }
    let current = current_running_context();
    if !is_user_context(current) {
        panic!("scheduler received an EL0 fault outside the user context");
    }
    let wait_slot =
        user_wait_slot(current).unwrap_or_else(|| panic!("user fault lacked a wait-token slot"));
    if object_wait_slots_ref()[wait_slot].is_pending() {
        panic!("running faulted user retained an object wait token");
    }
    validate_frame_storage_or_panic(current, frame);
    validate_user_kernel_state_or_panic(current, frame);
    let (elr, sp_el0) = unsafe { ((*frame).elr_el1, (*frame).sp_el0) };
    #[cfg(feature = "mobile-ui-runtime")]
    if current != USER_INIT_CONTEXT {
        crate::kprintln!(
            "MOBILE_UI_USER_FAULT pid={} reason={} esr={:#018x} far={:#018x} elr={:#018x} sp={:#018x}",
            crate::process::current_live_user_process_id().unwrap_or(0),
            reason as u64,
            esr,
            far,
            elr,
            sp_el0,
        );
    }
    CONTEXTS[current]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    if current == USER_INIT_CONTEXT {
        USER_INIT_FAULT_REASON.store(reason as u64, Ordering::Relaxed);
        USER_INIT_FAULT_ESR.store(esr, Ordering::Relaxed);
        USER_INIT_FAULT_FAR.store(far, Ordering::Relaxed);
        USER_INIT_FAULT_ELR.store(elr, Ordering::Relaxed);
        USER_INIT_FAULT_SP_EL0.store(sp_el0, Ordering::Relaxed);
    } else {
        let dynamic_slot = dynamic_user_index(current)
            .unwrap_or_else(|| panic!("dynamic user fault lacked a context slot"));
        USER_DYNAMIC_EXIT_CODES[dynamic_slot].store(0, Ordering::Relaxed);
        USER_DYNAMIC_TERMINATION_REASONS[dynamic_slot]
            .store(UserTerminationReason::Faulted as u64, Ordering::Relaxed);
        USER_DYNAMIC_FAULT_REASONS[dynamic_slot].store(reason as u64, Ordering::Relaxed);
        USER_DYNAMIC_FAULT_ESRS[dynamic_slot].store(esr, Ordering::Relaxed);
        USER_DYNAMIC_FAULT_FARS[dynamic_slot].store(far, Ordering::Relaxed);
        USER_DYNAMIC_FAULT_ELRS[dynamic_slot].store(elr, Ordering::Relaxed);
        USER_DYNAMIC_FAULT_SP_EL0S[dynamic_slot].store(sp_el0, Ordering::Relaxed);
    }
    transition_state(current, ThreadState::Running, ThreadState::Faulted);
    if current == USER_INIT_CONTEXT {
        if synchronous_dispatch {
            USER_SYNC_FAULT_DISPATCHES.fetch_add(1, Ordering::Relaxed);
        }
        USER_INIT_FAULTS.fetch_add(1, Ordering::Release);
    }
    select_next_context(current)
}

/// Terminates a dynamic EL0 thread without returning to its TrapFrame. The
/// stack and address space remain owned until a later monitor-context reaper
/// observes that the hardware stack/TTBR switch has completed.
pub fn exit_current_user(frame: *mut TrapFrame, exit_code: u64) -> *mut TrapFrame {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("scheduler attempted EL0 exit with IRQ enabled");
    }
    let current = current_running_context();
    let Some(dynamic_slot) = dynamic_user_index(current) else {
        panic!("only a dynamic user process may use ThreadExit");
    };
    let wait_slot = user_wait_slot_for_context(current);
    if object_wait_slots_ref()[wait_slot].is_pending() {
        panic!("exiting dynamic process retained an object wait token");
    }
    validate_frame_storage_or_panic(current, frame);
    validate_user_kernel_state_or_panic(current, frame);
    CONTEXTS[current]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    USER_DYNAMIC_EXIT_CODES[dynamic_slot].store(exit_code, Ordering::Relaxed);
    USER_DYNAMIC_TERMINATION_REASONS[dynamic_slot]
        .store(UserTerminationReason::Exited as u64, Ordering::Relaxed);
    clear_dynamic_user_fault_witness(dynamic_slot);
    transition_state(current, ThreadState::Running, ThreadState::Zombie);
    select_next_context(current)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminationAuthority {
    Init,
    #[cfg(feature = "storage-server-owner-liveness-runtime")]
    MonitorStorageWatchdog,
}

/// Forces one non-running dynamic EL0 context into the terminal queue for an
/// authenticated init `ProcessTerminate` syscall.
pub fn terminate_user_process(process_id: u64) -> Result<(), TerminateUserError> {
    terminate_user_process_inner(process_id, TerminationAuthority::Init)
}

/// Monitor-only half of M61's StorageServer retirement path.
///
/// This is deliberately separate from the init syscall authority. The caller
/// has already matched a kernel-owned `(pid, broker_epoch, lease_generation)`
/// ticket. The ordinary terminal queue and process reaper retain exclusive
/// authority to close handles, release the broker, and destroy the address
/// space.
#[cfg(feature = "storage-server-owner-liveness-runtime")]
pub fn terminate_user_process_from_storage_watchdog(
    process_id: u64,
) -> Result<(), TerminateUserError> {
    terminate_user_process_inner(process_id, TerminationAuthority::MonitorStorageWatchdog)
}

fn terminate_user_process_inner(
    process_id: u64,
    authority: TerminationAuthority,
) -> Result<(), TerminateUserError> {
    if !crate::arch::aarch64::irq_is_masked() || process_id == 0 {
        return Err(TerminateUserError::InvalidState);
    }
    let current = current_running_context();
    let authority_valid = match authority {
        TerminationAuthority::Init => {
            current == USER_INIT_CONTEXT
                && CONTEXTS[USER_INIT_CONTEXT].state.load(Ordering::Relaxed) == STATE_RUNNING
        }
        #[cfg(feature = "storage-server-owner-liveness-runtime")]
        TerminationAuthority::MonitorStorageWatchdog => {
            current == MONITOR_CONTEXT
                && CONTEXTS[MONITOR_CONTEXT].state.load(Ordering::Relaxed) == STATE_RUNNING
                && crate::arch::aarch64::mmu::current_translation_context()
                    == crate::arch::aarch64::mmu::kernel_translation_context()
        }
    };
    if !authority_valid {
        return Err(TerminateUserError::InvalidState);
    }
    let Some(context) = (DYNAMIC_USER_CONTEXT_START..DYNAMIC_USER_CONTEXT_END)
        .find(|&candidate| CONTEXT_PROCESS_IDS[candidate].load(Ordering::Relaxed) == process_id)
    else {
        return Err(TerminateUserError::NotFound);
    };
    let dynamic_slot = dynamic_user_index(context)
        .unwrap_or_else(|| panic!("forced termination target lacked a dynamic slot"));
    let wait_slot = user_wait_slot_for_context(context);
    let state = CONTEXTS[context].state.load(Ordering::Relaxed);
    let source = match state {
        STATE_RUNNABLE => {
            if object_wait_slots_ref()[wait_slot].is_pending() {
                panic!("runnable forced-termination target retained an object wait token");
            }
            ThreadState::Runnable
        }
        STATE_WAITING => {
            let token = object_wait_slots_ref()[wait_slot]
                .token()
                .unwrap_or_else(|| panic!("waiting forced-termination target lost its token"));
            if token.process_id() != process_id
                || object_wait_slots_mut()[wait_slot].take_exact_token(&token) != Some(token)
            {
                panic!("forced termination could not consume the target wait token");
            }
            match token.completion_kind() {
                ObjectWaitCompletionKind::Single => {
                    OBJECT_WAIT_ABANDONED_BY_TERMINATION.fetch_add(1, Ordering::Relaxed);
                }
                ObjectWaitCompletionKind::LegacyMany => {
                    WAIT_MANY_ABANDONED_BY_TERMINATION.fetch_add(1, Ordering::Relaxed);
                }
                ObjectWaitCompletionKind::Array => {
                    WAIT_ARRAY_ABANDONED_BY_TERMINATION.fetch_add(1, Ordering::Relaxed);
                }
            }
            ThreadState::Waiting
        }
        STATE_ZOMBIE | STATE_FAULTED => return Err(TerminateUserError::AlreadyTerminal),
        STATE_INACTIVE => return Err(TerminateUserError::NotFound),
        STATE_RUNNING | STATE_SLEEPING => return Err(TerminateUserError::InvalidState),
        _ => panic!("forced termination observed an unknown thread state"),
    };
    let frame = CONTEXTS[context].saved_frame.load(Ordering::Relaxed);
    validate_frame_or_panic(context, frame);
    USER_DYNAMIC_EXIT_CODES[dynamic_slot].store(PROCESS_KILLED_EXIT_CODE, Ordering::Relaxed);
    USER_DYNAMIC_TERMINATION_REASONS[dynamic_slot]
        .store(UserTerminationReason::Killed as u64, Ordering::Relaxed);
    clear_dynamic_user_fault_witness(dynamic_slot);
    transition_state(context, source, ThreadState::Zombie);
    Ok(())
}

pub fn terminal_user_snapshot() -> Option<TerminalUserSnapshot> {
    for context in DYNAMIC_USER_CONTEXT_START..DYNAMIC_USER_CONTEXT_END {
        let state = CONTEXTS[context].state.load(Ordering::Acquire);
        if state != STATE_ZOMBIE && state != STATE_FAULTED {
            continue;
        }
        let process_id = CONTEXT_PROCESS_IDS[context].load(Ordering::Relaxed);
        if process_id == 0 {
            panic!("terminal dynamic user context lost its process identity");
        }
        let dynamic_slot = dynamic_user_index(context)
            .unwrap_or_else(|| panic!("terminal context lacked a dynamic slot"));
        return Some(TerminalUserSnapshot {
            process_id,
            translation: context_translation(context),
            reason: USER_DYNAMIC_TERMINATION_REASONS[dynamic_slot].load(Ordering::Relaxed),
            exit_code: USER_DYNAMIC_EXIT_CODES[dynamic_slot].load(Ordering::Relaxed),
            fault_reason: USER_DYNAMIC_FAULT_REASONS[dynamic_slot].load(Ordering::Relaxed),
            fault_esr: USER_DYNAMIC_FAULT_ESRS[dynamic_slot].load(Ordering::Relaxed),
            fault_far: USER_DYNAMIC_FAULT_FARS[dynamic_slot].load(Ordering::Relaxed),
            fault_elr: USER_DYNAMIC_FAULT_ELRS[dynamic_slot].load(Ordering::Relaxed),
            fault_sp_el0: USER_DYNAMIC_FAULT_SP_EL0S[dynamic_slot].load(Ordering::Relaxed),
            code_start: USER_DYNAMIC_CODE_STARTS[dynamic_slot].load(Ordering::Relaxed),
            code_end: USER_DYNAMIC_CODE_ENDS[dynamic_slot].load(Ordering::Relaxed),
            stack_start: USER_DYNAMIC_STACK_STARTS[dynamic_slot].load(Ordering::Relaxed),
            stack_end: USER_DYNAMIC_STACK_ENDS[dynamic_slot].load(Ordering::Relaxed),
        });
    }
    None
}

/// Detaches a terminal dynamic user thread after execution has resumed on a
/// kernel stack under a different translation context. Returning ownership of
/// the stack is the scheduler's stopped-thread proof for the process reaper.
pub fn detach_terminal_user(process_id: u64) -> Option<StoppedUserThread> {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let current = current_running_context();
    let matching = (DYNAMIC_USER_CONTEXT_START..DYNAMIC_USER_CONTEXT_END)
        .find(|&context| CONTEXT_PROCESS_IDS[context].load(Ordering::Relaxed) == process_id);
    let Some(context) = matching else {
        crate::arch::aarch64::restore_daif(saved_daif);
        return None;
    };
    let state = CONTEXTS[context].state.load(Ordering::Relaxed);
    if state != STATE_ZOMBIE && state != STATE_FAULTED {
        crate::arch::aarch64::restore_daif(saved_daif);
        return None;
    }
    let dynamic_slot = dynamic_user_index(context)
        .unwrap_or_else(|| panic!("terminal context lacked a dynamic slot"));
    let wait_slot = user_wait_slot_for_context(context);
    if process_id == 0
        || is_user_context(current)
        || object_wait_slots_ref()[wait_slot].is_pending()
    {
        panic!("terminal user thread detach violated process/current invariants");
    }
    let translation = context_translation(context);
    if crate::arch::aarch64::mmu::current_translation_context() == translation {
        panic!("terminal address space remained active during detach");
    }
    let stack_bottom = CONTEXTS[context].canary_address.load(Ordering::Relaxed);
    let stack_top = CONTEXTS[context].stack_top.load(Ordering::Relaxed);
    let live_sp: usize;
    unsafe {
        core::arch::asm!(
            "mov {live_sp}, sp",
            live_sp = out(reg) live_sp,
            options(nomem, nostack, preserves_flags)
        );
    }
    if (stack_bottom..stack_top).contains(&live_sp) || !stack_canary_intact(&CONTEXTS[context]) {
        panic!("terminal kernel stack was still live or corrupt during detach");
    }
    let stack = dynamic_stack_slot_mut(dynamic_slot)
        .take()
        .unwrap_or_else(|| panic!("terminal dynamic thread lost its kernel stack owner"));
    let expected = if state == STATE_ZOMBIE {
        ThreadState::Zombie
    } else {
        ThreadState::Faulted
    };
    transition_state(context, expected, ThreadState::Inactive);
    CONTEXTS[context]
        .saved_frame
        .store(ptr::null_mut(), Ordering::Relaxed);
    CONTEXTS[context].translation.store(0, Ordering::Relaxed);
    CONTEXTS[context].stack_bottom.store(0, Ordering::Relaxed);
    CONTEXTS[context].stack_top.store(0, Ordering::Relaxed);
    CONTEXTS[context].canary_address.store(0, Ordering::Relaxed);
    CONTEXT_PROCESS_IDS[context].store(0, Ordering::Relaxed);
    USER_DYNAMIC_CODE_STARTS[dynamic_slot].store(0, Ordering::Relaxed);
    USER_DYNAMIC_CODE_ENDS[dynamic_slot].store(0, Ordering::Relaxed);
    USER_DYNAMIC_STACK_STARTS[dynamic_slot].store(0, Ordering::Relaxed);
    USER_DYNAMIC_STACK_ENDS[dynamic_slot].store(0, Ordering::Relaxed);
    clear_dynamic_user_fault_witness(dynamic_slot);
    let selections = USER_DYNAMIC_SELECTED[dynamic_slot].load(Ordering::Relaxed);
    crate::arch::aarch64::restore_daif(saved_daif);
    Some(StoppedUserThread {
        process_id,
        translation,
        selections,
        stack,
    })
}

/// Saves the interrupted context and selects the next fixed runnable context.
///
/// This function is called only after the timer has been rearmed and its GIC
/// acknowledgement has been ended. PSTATE.I remains set throughout this Rust
/// call, so the single-core scheduler state cannot be re-entered.
pub fn on_timer_tick(frame: *mut TrapFrame) -> *mut TrapFrame {
    if !ACTIVE.load(Ordering::Acquire) {
        return frame;
    }
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("scheduler timer dispatch entered with IRQ enabled");
    }

    let current = current_running_context();

    let now_tick = crate::arch::aarch64::timer::tick_count();
    let now_counter = crate::arch::aarch64::timer::counter_value();
    SCHED_CURRENT_TICK.store(now_tick, Ordering::Release);
    wake_expired_sleepers(now_counter, now_tick);
    // User wait deadlines share the exact generation-qualified object token;
    // the periodic IRQ only supplies a sampling opportunity. This guarantees
    // no early expiry without reusing the context-only kernel sleep queue.
    wake_object_waiters();
    validate_sleeping_invariants();
    if sleeping_context_count() == WORKER_COUNT {
        OVERLAP_TICKS.fetch_add(1, Ordering::Relaxed);
    }

    if is_user_context(current) {
        if current == USER_INIT_CONTEXT {
            USER_INIT_LOWER_IRQS.fetch_add(1, Ordering::Relaxed);
        } else {
            let dynamic_slot = dynamic_user_index(current)
                .unwrap_or_else(|| panic!("dynamic user IRQ lacked a context slot"));
            USER_DYNAMIC_LOWER_IRQS[dynamic_slot].fetch_add(1, Ordering::Relaxed);
        }
        validate_frame_storage_or_panic(current, frame);
        validate_user_kernel_state_or_panic(current, frame);
        if !user_return_state_is_valid(current, frame) {
            TIMER_DISPATCHES.fetch_add(1, Ordering::Relaxed);
            return fault_current_user_after_timer(frame, UserFaultReason::InvalidReturnState);
        }
    } else {
        validate_frame_or_panic(current, frame);
    }
    CONTEXTS[current]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    transition_state(current, ThreadState::Running, ThreadState::Runnable);
    TIMER_DISPATCHES.fetch_add(1, Ordering::Relaxed);

    let _next_frame = select_next_context(current);

    // This fault injection proves that the QEMU checker rejects a scheduler
    // which updates bookkeeping but never returns the selected stack/frame.
    #[cfg(feature = "scheduler-no-switch-self-test")]
    {
        if NO_SWITCH_ARMED.load(Ordering::Acquire) {
            if !NO_SWITCH_INJECTED.swap(true, Ordering::AcqRel) {
                crate::kprintln!("SCHED_NO_SWITCH_INJECTED: returning the interrupted frame");
            }
            frame
        } else {
            _next_frame
        }
    }

    #[cfg(not(feature = "scheduler-no-switch-self-test"))]
    _next_frame
}

/// Runs the no-switch fault injection after boot-time IRQ users have proved
/// their independent success contracts. Timer dispatches before this point
/// remain real context switches, so a longer storage probe cannot consume the
/// one-shot scheduler fault before its test baseline is visible. Once armed,
/// the boot context stays here so no success path can race the injected tick.
#[cfg(feature = "scheduler-no-switch-self-test")]
pub fn run_no_switch_self_test() {
    if crate::arch::aarch64::irq_is_masked() {
        panic!("scheduler no-switch self-test entered with IRQ masked");
    }
    if NO_SWITCH_INJECTED.load(Ordering::Acquire) || NO_SWITCH_ARMED.swap(true, Ordering::AcqRel) {
        panic!("scheduler no-switch self-test was armed more than once");
    }
    loop {
        crate::arch::aarch64::wait_for_interrupt();
    }
}

/// Handles the kernel-thread sleep SVC with IRQ already masked by the vector.
pub fn sleep_current(frame: *mut TrapFrame) -> *mut TrapFrame {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("scheduler sleep dispatch entered with IRQ enabled");
    }
    if !ACTIVE.load(Ordering::Acquire) {
        return sleep_error(frame, SLEEP_STATUS_INVALID_DURATION);
    }
    let current = current_running_context();
    validate_frame_or_panic(current, frame);
    let ticks = unsafe { (*frame).x[0] };

    if !(MIN_SLEEP_TICKS..=MAX_SLEEP_TICKS).contains(&ticks) {
        return sleep_error(frame, SLEEP_STATUS_INVALID_DURATION);
    }
    let Some(worker) = worker_index(current) else {
        return sleep_error(frame, SLEEP_STATUS_MONITOR);
    };
    if unsafe { (*frame).spsr_el1 } & (1 << 7) != 0 {
        return sleep_error(frame, SLEEP_STATUS_IRQ_MASKED);
    }
    if SLEEP_ENQUEUED[worker].load(Ordering::Relaxed)
        != TIMER_WAKEUPS[worker].load(Ordering::Relaxed)
    {
        return sleep_error(frame, SLEEP_STATUS_ALREADY_PENDING);
    }

    let Some(timer_deadline) = crate::arch::aarch64::timer::sleep_deadline_after_ticks(ticks)
    else {
        return sleep_error(frame, SLEEP_STATUS_INVALID_DURATION);
    };
    let now_tick = timer_deadline.logical_tick;
    let logical_deadline = now_tick.wrapping_add(ticks);

    let wait = TimerWait {
        context: current,
        deadline: timer_deadline.deadline_counter,
    };
    timer_wait_queue_mut()
        .insert(timer_deadline.request_counter, wait)
        .unwrap_or_else(|_| panic!("scheduler timer wait queue insertion failed"));
    update_queue_peak();

    let selected = worker_selected_counter(worker).load(Ordering::Acquire);
    let observed = worker_observed_counter(worker).load(Ordering::Acquire);
    let work = worker_work_counter(worker).load(Ordering::Acquire);
    SLEEP_REQUEST_TICK[worker].store(now_tick, Ordering::Relaxed);
    SLEEP_BLOCK_TICK[worker].store(now_tick, Ordering::Relaxed);
    SLEEP_DEADLINE[worker].store(logical_deadline, Ordering::Relaxed);
    SLEEP_REQUEST_COUNTER[worker].store(timer_deadline.request_counter, Ordering::Relaxed);
    SLEEP_COUNTER_DEADLINE[worker].store(timer_deadline.deadline_counter, Ordering::Relaxed);
    SLEEP_SELECTED_BASELINE[worker].store(selected, Ordering::Relaxed);
    SLEEP_OBSERVED_BASELINE[worker].store(observed, Ordering::Relaxed);
    SLEEP_WORK_BASELINE[worker].store(work, Ordering::Relaxed);
    SLEEP_FRAME_BASELINE[worker].store(frame as usize, Ordering::Relaxed);
    SLEEP_ENQUEUED[worker].fetch_add(1, Ordering::Relaxed);

    unsafe { (*frame).x[0] = SLEEP_STATUS_OK };
    CONTEXTS[current]
        .saved_frame
        .store(frame, Ordering::Relaxed);
    transition_state(current, ThreadState::Running, ThreadState::Sleeping);
    BLOCK_DISPATCHES.fetch_add(1, Ordering::Relaxed);
    select_next_context(current)
}

fn current_running_context() -> usize {
    let current = CURRENT_CONTEXT.load(Ordering::Relaxed);
    let running_count = CONTEXTS
        .iter()
        .filter(|context| context.state.load(Ordering::Relaxed) == STATE_RUNNING)
        .count();
    if current >= CONTEXT_COUNT
        || CONTEXTS[current].state.load(Ordering::Relaxed) != STATE_RUNNING
        || running_count != 1
    {
        panic!("scheduler current-context invariant failed");
    }
    assert_context_translation(current);
    current
}

fn context_translation(context: usize) -> crate::arch::aarch64::mmu::TranslationContext {
    if context >= CONTEXT_COUNT {
        panic!("scheduler translation lookup used an invalid context");
    }
    let raw = CONTEXTS[context].translation.load(Ordering::Relaxed);
    crate::arch::aarch64::mmu::TranslationContext::from_raw(raw)
        .unwrap_or_else(|| panic!("scheduler context has no valid translation root"))
}

fn assert_context_translation(context: usize) {
    if crate::arch::aarch64::mmu::current_translation_context() != context_translation(context) {
        panic!("scheduler current context and TTBR0 disagree");
    }
}

fn transition_state(context: usize, from: ThreadState, to: ThreadState) {
    if context >= CONTEXT_COUNT || !valid_transition(from, to) {
        panic!("scheduler requested an invalid thread-state transition");
    }
    let observed = CONTEXTS[context].state.load(Ordering::Relaxed);
    if observed != from.raw() {
        panic!("scheduler thread-state source invariant failed");
    }
    CONTEXTS[context].state.store(to.raw(), Ordering::Relaxed);
}

fn select_next_context(previous: usize) -> *mut TrapFrame {
    if !crate::arch::aarch64::irq_is_masked() || CURRENT_CONTEXT.load(Ordering::Relaxed) != previous
    {
        panic!("scheduler context selection escaped its IRQ-masked commit boundary");
    }
    assert_context_translation(previous);

    #[cfg(feature = "scheduler-blocked-run-self-test")]
    if !BLOCKED_RUN_INJECTED.load(Ordering::Acquire) && timer_wait_queue_ref().len() == WORKER_COUNT
    {
        let injected = WORKER_0_CONTEXT;
        if CONTEXTS[injected].state.load(Ordering::Relaxed) == STATE_SLEEPING {
            BLOCKED_RUN_INJECTED.store(true, Ordering::Release);
            let injected_frame = CONTEXTS[injected].saved_frame.load(Ordering::Relaxed);
            validate_frame_or_panic(injected, injected_frame);
            return commit_selected_context(previous, injected, injected_frame, true);
        }
    }

    let preferred = PREFERRED_CONTEXT.swap(CONTEXT_COUNT, Ordering::AcqRel);
    let next = if preferred < CONTEXT_COUNT
        && CONTEXTS[preferred].state.load(Ordering::Relaxed) == STATE_RUNNABLE
    {
        PREFERRED_DISPATCHES.fetch_add(1, Ordering::Relaxed);
        preferred
    } else {
        if preferred < CONTEXT_COUNT {
            PREFERRED_STALE.fetch_add(1, Ordering::Relaxed);
        }
        next_runnable(previous, CONTEXT_COUNT, |candidate| {
            CONTEXTS[candidate].state.load(Ordering::Relaxed) == STATE_RUNNABLE
        })
        .unwrap_or_else(|| panic!("scheduler has no runnable context"))
    };
    let next_frame = CONTEXTS[next].saved_frame.load(Ordering::Relaxed);
    validate_frame_or_panic(next, next_frame);
    if worker_index(next).is_some() && context_is_queued(next) {
        BLOCKED_STATE_FAILURES.fetch_add(1, Ordering::Relaxed);
        panic!("scheduler selected a context that remains in the timer wait queue");
    }

    commit_selected_context(previous, next, next_frame, false)
}

#[cfg(feature = "storage-server-runtime")]
pub fn storage_preference_counts() -> StoragePreferenceSnapshot {
    // Dispatch consumption happens in the timer exception path. Capture the
    // counter set and one-slot pending state under one IRQ-masked boundary so
    // the exact retirement equation cannot observe half of that transition.
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let snapshot = StoragePreferenceSnapshot {
        scans: PREFERRED_SCANS.load(Ordering::Acquire),
        candidates: PREFERRED_CANDIDATES.load(Ordering::Acquire),
        no_candidate: PREFERRED_NO_CANDIDATE.load(Ordering::Acquire),
        requests: PREFERRED_REQUESTS.load(Ordering::Acquire),
        coalesced: PREFERRED_COALESCED.load(Ordering::Acquire),
        dispatches: PREFERRED_DISPATCHES.load(Ordering::Acquire),
        stale: PREFERRED_STALE.load(Ordering::Acquire),
        pending: PREFERRED_CONTEXT.load(Ordering::Acquire) < CONTEXT_COUNT,
    };
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

fn commit_selected_context(
    previous: usize,
    next: usize,
    next_frame: *mut TrapFrame,
    injected_blocked_run: bool,
) -> *mut TrapFrame {
    if injected_blocked_run {
        CONTEXTS[next].state.store(STATE_RUNNING, Ordering::Relaxed);
    } else {
        transition_state(next, ThreadState::Runnable, ThreadState::Running);
    }
    let next_translation = context_translation(next);
    if crate::arch::aarch64::mmu::current_translation_context() != next_translation {
        crate::arch::aarch64::mmu::activate_translation_context(next_translation);
    }
    if crate::arch::aarch64::mmu::current_translation_context() != next_translation {
        panic!("scheduler TTBR0 commit did not take effect");
    }
    CURRENT_CONTEXT.store(next, Ordering::Release);
    selection_counter(next).fetch_add(1, Ordering::Release);
    if next != previous {
        CONTEXT_SWITCHES.fetch_add(1, Ordering::Relaxed);
    }
    next_frame
}

fn wake_expired_sleepers(now_counter: u64, now_tick: u64) {
    while let Some(wait) = timer_wait_queue_mut().pop_expired(now_counter) {
        let Some(worker) = worker_index(wait.context) else {
            BLOCKED_STATE_FAILURES.fetch_add(1, Ordering::Relaxed);
            panic!("timer wait queue contained the monitor context");
        };
        if !deadline_reached(now_counter, wait.deadline) {
            EARLY_WAKE_FAILURES.fetch_add(1, Ordering::Relaxed);
            panic!("scheduler attempted to wake a context before its deadline");
        }
        if CONTEXTS[wait.context].state.load(Ordering::Relaxed) != STATE_SLEEPING
            || SLEEP_COUNTER_DEADLINE[worker].load(Ordering::Relaxed) != wait.deadline
        {
            BLOCKED_STATE_FAILURES.fetch_add(1, Ordering::Relaxed);
            panic!("timer wait queue contained a stale context entry");
        }
        validate_sleeping_metrics(worker);

        SLEEP_READY_TICK[worker].store(now_tick, Ordering::Relaxed);
        SLEEP_READY_COUNTER[worker].store(now_counter, Ordering::Relaxed);
        SLEEP_SELECTED_READY[worker].store(
            worker_selected_counter(worker).load(Ordering::Acquire),
            Ordering::Relaxed,
        );
        SLEEP_OBSERVED_READY[worker].store(
            worker_observed_counter(worker).load(Ordering::Acquire),
            Ordering::Relaxed,
        );
        SLEEP_WORK_READY[worker].store(
            worker_work_counter(worker).load(Ordering::Acquire),
            Ordering::Relaxed,
        );
        transition_state(wait.context, ThreadState::Sleeping, ThreadState::Runnable);
        TIMER_WAKEUPS[worker].fetch_add(1, Ordering::Release);
    }
}

fn validate_sleeping_invariants() {
    let mut sleeping = 0;
    for (worker, deadline_slot) in SLEEP_COUNTER_DEADLINE.iter().enumerate() {
        let context = worker_context(worker);
        let is_sleeping = CONTEXTS[context].state.load(Ordering::Relaxed) == STATE_SLEEPING;
        let deadline = deadline_slot.load(Ordering::Relaxed);
        let queued = timer_wait_queue_ref().contains(context, deadline);
        if is_sleeping {
            sleeping += 1;
            if !queued {
                BLOCKED_STATE_FAILURES.fetch_add(1, Ordering::Relaxed);
                panic!("sleeping context is missing from the timer wait queue");
            }
            validate_sleeping_metrics(worker);
        } else if queued {
            BLOCKED_STATE_FAILURES.fetch_add(1, Ordering::Relaxed);
            if worker_work_counter(worker).load(Ordering::Acquire)
                != SLEEP_WORK_BASELINE[worker].load(Ordering::Relaxed)
            {
                #[cfg(feature = "scheduler-blocked-run-self-test")]
                crate::kprintln!(
                    "SLEEP_BLOCKED_RUN_INJECTED: queued context changed its work counter"
                );
                panic!("scheduler blocked context executed while asleep");
            }
            panic!("non-sleeping context remains in the timer wait queue");
        }
    }
    if sleeping != timer_wait_queue_ref().len() {
        BLOCKED_STATE_FAILURES.fetch_add(1, Ordering::Relaxed);
        panic!("timer wait queue length disagrees with sleeping states");
    }
}

fn validate_sleeping_metrics(worker: usize) {
    let context = worker_context(worker);
    let selected = worker_selected_counter(worker).load(Ordering::Acquire);
    let observed = worker_observed_counter(worker).load(Ordering::Acquire);
    let work = worker_work_counter(worker).load(Ordering::Acquire);
    let frame = CONTEXTS[context].saved_frame.load(Ordering::Relaxed) as usize;
    if selected != SLEEP_SELECTED_BASELINE[worker].load(Ordering::Relaxed)
        || observed != SLEEP_OBSERVED_BASELINE[worker].load(Ordering::Relaxed)
        || work != SLEEP_WORK_BASELINE[worker].load(Ordering::Relaxed)
        || frame != SLEEP_FRAME_BASELINE[worker].load(Ordering::Relaxed)
    {
        BLOCKED_STATE_FAILURES.fetch_add(1, Ordering::Relaxed);
        panic!("scheduler sleeping context executed or lost its saved frame");
    }
}

fn context_is_queued(context: usize) -> bool {
    let Some(worker) = worker_index(context) else {
        return false;
    };
    timer_wait_queue_ref().contains(
        context,
        SLEEP_COUNTER_DEADLINE[worker].load(Ordering::Relaxed),
    )
}

fn sleeping_context_count() -> usize {
    (0..WORKER_COUNT)
        .filter(|&worker| {
            CONTEXTS[worker_context(worker)]
                .state
                .load(Ordering::Relaxed)
                == STATE_SLEEPING
        })
        .count()
}

fn update_queue_peak() {
    let len = timer_wait_queue_ref().len();
    QUEUE_PEAK.fetch_max(len, Ordering::Relaxed);
}

fn sleep_error(frame: *mut TrapFrame, status: u64) -> *mut TrapFrame {
    unsafe { (*frame).x[0] = status };
    frame
}

fn timer_wait_queue_mut() -> &'static mut TimerWaitQueue<WORKER_COUNT> {
    unsafe { &mut *TIMER_WAIT_QUEUE.0.get() }
}

fn timer_wait_queue_ref() -> &'static TimerWaitQueue<WORKER_COUNT> {
    unsafe { &*TIMER_WAIT_QUEUE.0.get() }
}

fn object_wait_slots_mut() -> &'static mut [ObjectWaitSlot; USER_CONTEXT_COUNT] {
    unsafe { &mut *USER_OBJECT_WAIT_SLOTS.0.get() }
}

fn object_wait_slots_ref() -> &'static [ObjectWaitSlot; USER_CONTEXT_COUNT] {
    unsafe { &*USER_OBJECT_WAIT_SLOTS.0.get() }
}

pub fn snapshot() -> SchedulerSnapshot {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let snapshot = SchedulerSnapshot {
        timer_ticks: crate::arch::aarch64::timer::tick_count(),
        timer_dispatches: TIMER_DISPATCHES.load(Ordering::Acquire),
        context_switches: CONTEXT_SWITCHES.load(Ordering::Acquire),
        block_dispatches: BLOCK_DISPATCHES.load(Ordering::Acquire),
        monitor_selected: MONITOR_SELECTED.load(Ordering::Acquire),
        worker_selected: [
            SCHED_WORKER_0_SELECTED.load(Ordering::Acquire),
            SCHED_WORKER_1_SELECTED.load(Ordering::Acquire),
        ],
        worker_observed: [
            SCHED_WORKER_0_OBSERVED.load(Ordering::Acquire),
            SCHED_WORKER_1_OBSERVED.load(Ordering::Acquire),
        ],
        worker_work: [
            SCHED_WORKER_0_WORK.load(Ordering::Acquire),
            SCHED_WORKER_1_WORK.load(Ordering::Acquire),
        ],
        register_failures: SCHED_REGISTER_FAILURES.load(Ordering::Acquire),
        epoch_failures: SCHED_EPOCH_FAILURES.load(Ordering::Acquire),
        sleep_status_failures: SCHED_SLEEP_STATUS_FAILURES.load(Ordering::Acquire),
        stack_failures: u64::from(!all_stack_canaries_intact()),
        sleep_enqueued: [
            SLEEP_ENQUEUED[0].load(Ordering::Acquire),
            SLEEP_ENQUEUED[1].load(Ordering::Acquire),
        ],
        timer_wakeups: [
            TIMER_WAKEUPS[0].load(Ordering::Acquire),
            TIMER_WAKEUPS[1].load(Ordering::Acquire),
        ],
        sleep_request_tick: [
            SLEEP_REQUEST_TICK[0].load(Ordering::Acquire),
            SLEEP_REQUEST_TICK[1].load(Ordering::Acquire),
        ],
        sleep_block_tick: [
            SLEEP_BLOCK_TICK[0].load(Ordering::Acquire),
            SLEEP_BLOCK_TICK[1].load(Ordering::Acquire),
        ],
        sleep_deadline: [
            SLEEP_DEADLINE[0].load(Ordering::Acquire),
            SLEEP_DEADLINE[1].load(Ordering::Acquire),
        ],
        sleep_request_counter: [
            SLEEP_REQUEST_COUNTER[0].load(Ordering::Acquire),
            SLEEP_REQUEST_COUNTER[1].load(Ordering::Acquire),
        ],
        sleep_counter_deadline: [
            SLEEP_COUNTER_DEADLINE[0].load(Ordering::Acquire),
            SLEEP_COUNTER_DEADLINE[1].load(Ordering::Acquire),
        ],
        sleep_ready_counter: [
            SLEEP_READY_COUNTER[0].load(Ordering::Acquire),
            SLEEP_READY_COUNTER[1].load(Ordering::Acquire),
        ],
        timer_period: crate::arch::aarch64::timer::period_counter_ticks(),
        sleep_ready_tick: [
            SLEEP_READY_TICK[0].load(Ordering::Acquire),
            SLEEP_READY_TICK[1].load(Ordering::Acquire),
        ],
        sleep_resume_tick: [
            SCHED_WORKER_0_SLEEP_RESUME_TICK.load(Ordering::Acquire),
            SCHED_WORKER_1_SLEEP_RESUME_TICK.load(Ordering::Acquire),
        ],
        sleep_returns: [
            SCHED_WORKER_0_SLEEP_RETURNS.load(Ordering::Acquire),
            SCHED_WORKER_1_SLEEP_RETURNS.load(Ordering::Acquire),
        ],
        sleep_epoch_baseline: [
            SCHED_WORKER_0_SLEEP_EPOCH_BASELINE.load(Ordering::Acquire),
            SCHED_WORKER_1_SLEEP_EPOCH_BASELINE.load(Ordering::Acquire),
        ],
        selected_at_block: [
            SLEEP_SELECTED_BASELINE[0].load(Ordering::Acquire),
            SLEEP_SELECTED_BASELINE[1].load(Ordering::Acquire),
        ],
        selected_at_ready: [
            SLEEP_SELECTED_READY[0].load(Ordering::Acquire),
            SLEEP_SELECTED_READY[1].load(Ordering::Acquire),
        ],
        observed_at_block: [
            SLEEP_OBSERVED_BASELINE[0].load(Ordering::Acquire),
            SLEEP_OBSERVED_BASELINE[1].load(Ordering::Acquire),
        ],
        observed_at_ready: [
            SLEEP_OBSERVED_READY[0].load(Ordering::Acquire),
            SLEEP_OBSERVED_READY[1].load(Ordering::Acquire),
        ],
        work_at_block: [
            SLEEP_WORK_BASELINE[0].load(Ordering::Acquire),
            SLEEP_WORK_BASELINE[1].load(Ordering::Acquire),
        ],
        work_at_ready: [
            SLEEP_WORK_READY[0].load(Ordering::Acquire),
            SLEEP_WORK_READY[1].load(Ordering::Acquire),
        ],
        queue_len: timer_wait_queue_ref().len(),
        queue_peak: QUEUE_PEAK.load(Ordering::Acquire),
        overlap_ticks: OVERLAP_TICKS.load(Ordering::Acquire),
        early_wake_failures: EARLY_WAKE_FAILURES.load(Ordering::Acquire),
        blocked_state_failures: BLOCKED_STATE_FAILURES.load(Ordering::Acquire),
    };
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

pub fn finish_validation() {
    SCHED_VALIDATION_COMPLETE.store(1, Ordering::Release);
}

fn reset_validation_counters() {
    PREFERRED_CONTEXT.store(CONTEXT_COUNT, Ordering::Relaxed);
    PREFERRED_SCANS.store(0, Ordering::Relaxed);
    PREFERRED_CANDIDATES.store(0, Ordering::Relaxed);
    PREFERRED_NO_CANDIDATE.store(0, Ordering::Relaxed);
    PREFERRED_REQUESTS.store(0, Ordering::Relaxed);
    PREFERRED_COALESCED.store(0, Ordering::Relaxed);
    PREFERRED_DISPATCHES.store(0, Ordering::Relaxed);
    PREFERRED_STALE.store(0, Ordering::Relaxed);
    TIMER_DISPATCHES.store(0, Ordering::Relaxed);
    CONTEXT_SWITCHES.store(0, Ordering::Relaxed);
    BLOCK_DISPATCHES.store(0, Ordering::Relaxed);
    MONITOR_SELECTED.store(0, Ordering::Relaxed);
    USER_INIT_SELECTED.store(0, Ordering::Relaxed);
    USER_INIT_LOWER_IRQS.store(0, Ordering::Relaxed);
    for dynamic_slot in 0..DYNAMIC_USER_CONTEXT_COUNT {
        USER_DYNAMIC_SELECTED[dynamic_slot].store(0, Ordering::Relaxed);
        USER_DYNAMIC_LOWER_IRQS[dynamic_slot].store(0, Ordering::Relaxed);
        USER_DYNAMIC_EXIT_CODES[dynamic_slot].store(0, Ordering::Relaxed);
        USER_DYNAMIC_TERMINATION_REASONS[dynamic_slot].store(0, Ordering::Relaxed);
        clear_dynamic_user_fault_witness(dynamic_slot);
    }
    USER_INIT_WAIT_PROCESS.store(0, Ordering::Relaxed);
    PROCESS_WAIT_BLOCKS.store(0, Ordering::Relaxed);
    PROCESS_WAIT_WAKES.store(0, Ordering::Relaxed);
    *object_wait_slots_mut() = [const { ObjectWaitSlot::new() }; USER_CONTEXT_COUNT];
    OBJECT_WAIT_BLOCKS.store(0, Ordering::Relaxed);
    OBJECT_WAIT_WAKES.store(0, Ordering::Relaxed);
    OBJECT_WAIT_CANCELS.store(0, Ordering::Relaxed);
    OBJECT_WAIT_ABANDONED_BY_TERMINATION.store(0, Ordering::Relaxed);
    WAIT_MANY_BLOCKS.store(0, Ordering::Relaxed);
    WAIT_MANY_WAKES.store(0, Ordering::Relaxed);
    WAIT_MANY_SIGNAL_WAKES.store(0, Ordering::Relaxed);
    WAIT_MANY_FINITE_SIGNAL_WAKES.store(0, Ordering::Relaxed);
    WAIT_MANY_TIMEOUT_WAKES.store(0, Ordering::Relaxed);
    WAIT_MANY_CANCELS.store(0, Ordering::Relaxed);
    WAIT_MANY_EARLY_FAILURES.store(0, Ordering::Relaxed);
    WAIT_MANY_ABANDONED_BY_TERMINATION.store(0, Ordering::Relaxed);
    WAIT_ARRAY_BLOCKS.store(0, Ordering::Relaxed);
    WAIT_ARRAY_WAKES.store(0, Ordering::Relaxed);
    WAIT_ARRAY_SIGNAL_WAKES.store(0, Ordering::Relaxed);
    WAIT_ARRAY_TIMEOUT_WAKES.store(0, Ordering::Relaxed);
    WAIT_ARRAY_CANCELS.store(0, Ordering::Relaxed);
    WAIT_ARRAY_EARLY_FAILURES.store(0, Ordering::Relaxed);
    WAIT_ARRAY_ABANDONED_BY_TERMINATION.store(0, Ordering::Relaxed);
    WAIT_ARRAY_MAX_ITEMS.store(0, Ordering::Relaxed);
    WAIT_ARRAY_LATEST_EPOCH.store(0, Ordering::Relaxed);
    USER_INIT_FAULTS.store(0, Ordering::Relaxed);
    USER_INIT_FAULT_REASON.store(0, Ordering::Relaxed);
    USER_INIT_FAULT_ESR.store(0, Ordering::Relaxed);
    USER_INIT_FAULT_FAR.store(0, Ordering::Relaxed);
    USER_INIT_FAULT_ELR.store(0, Ordering::Relaxed);
    USER_INIT_FAULT_SP_EL0.store(0, Ordering::Relaxed);
    USER_SYNC_FAULT_DISPATCHES.store(0, Ordering::Relaxed);
    USER_CODE_START.store(0, Ordering::Relaxed);
    USER_CODE_END.store(0, Ordering::Relaxed);
    USER_STACK_START.store(0, Ordering::Relaxed);
    USER_STACK_END.store(0, Ordering::Relaxed);
    for dynamic_slot in 0..DYNAMIC_USER_CONTEXT_COUNT {
        USER_DYNAMIC_CODE_STARTS[dynamic_slot].store(0, Ordering::Relaxed);
        USER_DYNAMIC_CODE_ENDS[dynamic_slot].store(0, Ordering::Relaxed);
        USER_DYNAMIC_STACK_STARTS[dynamic_slot].store(0, Ordering::Relaxed);
        USER_DYNAMIC_STACK_ENDS[dynamic_slot].store(0, Ordering::Relaxed);
    }
    SCHED_WORKER_0_SELECTED.store(0, Ordering::Relaxed);
    SCHED_WORKER_1_SELECTED.store(0, Ordering::Relaxed);
    SCHED_WORKER_0_OBSERVED.store(0, Ordering::Relaxed);
    SCHED_WORKER_1_OBSERVED.store(0, Ordering::Relaxed);
    SCHED_WORKER_0_WORK.store(0, Ordering::Relaxed);
    SCHED_WORKER_1_WORK.store(0, Ordering::Relaxed);
    SCHED_REGISTER_FAILURES.store(0, Ordering::Relaxed);
    SCHED_EPOCH_FAILURES.store(0, Ordering::Relaxed);
    SCHED_SLEEP_STATUS_FAILURES.store(0, Ordering::Relaxed);
    SCHED_VALIDATION_COMPLETE.store(0, Ordering::Relaxed);
    SCHED_CURRENT_TICK.store(0, Ordering::Relaxed);
    SCHED_WORKER_0_SLEEP_RESUME_TICK.store(0, Ordering::Relaxed);
    SCHED_WORKER_1_SLEEP_RESUME_TICK.store(0, Ordering::Relaxed);
    SCHED_WORKER_0_SLEEP_RETURNS.store(0, Ordering::Relaxed);
    SCHED_WORKER_1_SLEEP_RETURNS.store(0, Ordering::Relaxed);
    SCHED_WORKER_0_SLEEP_EPOCH_BASELINE.store(0, Ordering::Relaxed);
    SCHED_WORKER_1_SLEEP_EPOCH_BASELINE.store(0, Ordering::Relaxed);
    QUEUE_PEAK.store(0, Ordering::Relaxed);
    OVERLAP_TICKS.store(0, Ordering::Relaxed);
    EARLY_WAKE_FAILURES.store(0, Ordering::Relaxed);
    BLOCKED_STATE_FAILURES.store(0, Ordering::Relaxed);
    reset_worker_sleep_metrics(0);
    reset_worker_sleep_metrics(1);
    *timer_wait_queue_mut() = TimerWaitQueue::new();
    #[cfg(feature = "scheduler-no-switch-self-test")]
    {
        NO_SWITCH_ARMED.store(false, Ordering::Relaxed);
        NO_SWITCH_INJECTED.store(false, Ordering::Relaxed);
    }
    #[cfg(feature = "scheduler-blocked-run-self-test")]
    BLOCKED_RUN_INJECTED.store(false, Ordering::Relaxed);
}

fn reset_worker_sleep_metrics(worker: usize) {
    SLEEP_ENQUEUED[worker].store(0, Ordering::Relaxed);
    TIMER_WAKEUPS[worker].store(0, Ordering::Relaxed);
    SLEEP_REQUEST_TICK[worker].store(0, Ordering::Relaxed);
    SLEEP_BLOCK_TICK[worker].store(0, Ordering::Relaxed);
    SLEEP_DEADLINE[worker].store(0, Ordering::Relaxed);
    SLEEP_REQUEST_COUNTER[worker].store(0, Ordering::Relaxed);
    SLEEP_COUNTER_DEADLINE[worker].store(0, Ordering::Relaxed);
    SLEEP_READY_COUNTER[worker].store(0, Ordering::Relaxed);
    SLEEP_READY_TICK[worker].store(0, Ordering::Relaxed);
    SLEEP_SELECTED_BASELINE[worker].store(0, Ordering::Relaxed);
    SLEEP_SELECTED_READY[worker].store(0, Ordering::Relaxed);
    SLEEP_OBSERVED_BASELINE[worker].store(0, Ordering::Relaxed);
    SLEEP_OBSERVED_READY[worker].store(0, Ordering::Relaxed);
    SLEEP_WORK_BASELINE[worker].store(0, Ordering::Relaxed);
    SLEEP_WORK_READY[worker].store(0, Ordering::Relaxed);
    SLEEP_FRAME_BASELINE[worker].store(0, Ordering::Relaxed);
}

fn configure_stack(index: usize, range: (usize, usize)) {
    let (canary_address, top) = range;
    if !canary_address.is_multiple_of(16)
        || !top.is_multiple_of(16)
        || top <= canary_address + STACK_GUARD_BYTES + size_of::<TrapFrame>()
    {
        panic!("scheduler received an invalid stack range");
    }
    unsafe {
        (canary_address as *mut u64).write_volatile(STACK_CANARY);
        ((canary_address + size_of::<u64>()) as *mut u64).write_volatile(STACK_CANARY_INVERSE);
    }
    CONTEXTS[index]
        .canary_address
        .store(canary_address, Ordering::Relaxed);
    CONTEXTS[index]
        .stack_bottom
        .store(canary_address + STACK_GUARD_BYTES, Ordering::Relaxed);
    CONTEXTS[index].stack_top.store(top, Ordering::Relaxed);
}

fn build_initial_frame(stack_top: usize, entry: u64, argument: u64) -> *mut TrapFrame {
    let frame_address = stack_top - size_of::<TrapFrame>();
    let frame = frame_address as *mut TrapFrame;
    let image = TrapFrame::for_kernel_thread(
        entry,
        argument,
        scheduler_thread_returned as *const () as u64,
        crate::arch::aarch64::mmu::pan_supported(),
    );
    unsafe { frame.write(image) };
    frame
}

fn build_user_frame(
    stack_top: usize,
    entry: usize,
    user_stack_top: usize,
    argument0: u64,
    argument1: u64,
) -> *mut TrapFrame {
    let frame_address = stack_top - size_of::<TrapFrame>();
    let frame = frame_address as *mut TrapFrame;
    let mut image = TrapFrame::for_user_thread(entry as u64, user_stack_top as u64, argument0);
    image.x[1] = argument1;
    unsafe { frame.write(image) };
    frame
}

fn validate_frame_or_panic(index: usize, frame: *mut TrapFrame) {
    validate_frame_storage_or_panic(index, frame);
    if is_user_context(index) {
        validate_user_kernel_state_or_panic(index, frame);
        if !user_return_state_is_valid(index, frame) {
            panic!("scheduler stored an invalid EL0 return state");
        }
    } else {
        validate_kernel_return_state_or_panic(frame);
    }
}

fn validate_frame_storage_or_panic(index: usize, frame: *mut TrapFrame) {
    if index >= CONTEXT_COUNT || frame.is_null() || !(frame as usize).is_multiple_of(16) {
        panic!("scheduler saved-frame identity check failed");
    }
    let context = &CONTEXTS[index];
    let frame_address = frame as usize;
    let bottom = context.stack_bottom.load(Ordering::Relaxed);
    let top = context.stack_top.load(Ordering::Relaxed);
    let frame_end = frame_address.checked_add(size_of::<TrapFrame>());
    if frame_address < bottom || frame_end.is_none_or(|end| end > top) {
        panic!("scheduler saved frame escaped its assigned stack");
    }
    if !stack_canary_intact(context) {
        let canary_address = context.canary_address.load(Ordering::Relaxed);
        let (actual, actual_inverse) = if canary_address == 0 {
            (0, 0)
        } else {
            unsafe {
                (
                    (canary_address as *const u64).read_volatile(),
                    ((canary_address + size_of::<u64>()) as *const u64).read_volatile(),
                )
            }
        };
        crate::kprintln!(
            "STACK_CANARY_DIAG context={} frame={:#018x} bottom={:#018x} top={:#018x} canary={:#018x}/{:#018x} expected={:#018x}/{:#018x}",
            index,
            frame_address,
            bottom,
            top,
            actual,
            actual_inverse,
            STACK_CANARY,
            STACK_CANARY_INVERSE,
        );
        panic!("scheduler stack canary was corrupted");
    }
}

fn validate_user_kernel_state_or_panic(index: usize, frame: *mut TrapFrame) {
    let spsr = unsafe { (*frame).spsr_el1 };
    if spsr & 0x3df != 0x340 || spsr & (1 << 23) != 0 {
        crate::kprintln!(
            "USER_RETURN_PRIVILEGE_DIAG context={} pid={} frame={:#018x} spsr={:#018x} elr={:#018x} sp_el0={:#018x}",
            index,
            CONTEXT_PROCESS_IDS[index].load(Ordering::Relaxed),
            frame as usize,
            spsr,
            unsafe { (*frame).elr_el1 },
            unsafe { (*frame).sp_el0 },
        );
        panic!("scheduler saved frame has an invalid privileged EL0 return state");
    }
}

fn user_return_state_is_valid(index: usize, frame: *mut TrapFrame) -> bool {
    let (elr, sp_el0) = unsafe { ((*frame).elr_el1 as usize, (*frame).sp_el0 as usize) };
    let (code_start, code_end, user_stack_start, user_stack_end) = user_bounds(index);
    elr.is_multiple_of(4)
        && elr >= code_start
        && elr < code_end
        && sp_el0.is_multiple_of(16)
        && sp_el0 >= user_stack_start
        && sp_el0 <= user_stack_end
}

fn user_bounds(index: usize) -> (usize, usize, usize, usize) {
    if index == USER_INIT_CONTEXT {
        (
            USER_CODE_START.load(Ordering::Relaxed),
            USER_CODE_END.load(Ordering::Relaxed),
            USER_STACK_START.load(Ordering::Relaxed),
            USER_STACK_END.load(Ordering::Relaxed),
        )
    } else if let Some(dynamic_slot) = dynamic_user_index(index) {
        (
            USER_DYNAMIC_CODE_STARTS[dynamic_slot].load(Ordering::Relaxed),
            USER_DYNAMIC_CODE_ENDS[dynamic_slot].load(Ordering::Relaxed),
            USER_DYNAMIC_STACK_STARTS[dynamic_slot].load(Ordering::Relaxed),
            USER_DYNAMIC_STACK_ENDS[dynamic_slot].load(Ordering::Relaxed),
        )
    } else {
        panic!("user bounds requested for a kernel context")
    }
}

const fn is_user_context(index: usize) -> bool {
    index >= KERNEL_CONTEXT_COUNT && index < CONTEXT_COUNT
}

fn validate_kernel_return_state_or_panic(frame: *mut TrapFrame) {
    let (elr, spsr) = unsafe { ((*frame).elr_el1 as usize, (*frame).spsr_el1) };
    let text_start = addr_of!(__text_start) as usize;
    let text_end = addr_of!(__text_end) as usize;
    let required_pan = u64::from(crate::arch::aarch64::mmu::pan_supported()) << 22;
    if !elr.is_multiple_of(4)
        || elr < text_start
        || elr >= text_end
        || spsr & 0x1f != 0x5
        || spsr & (1 << 22) != required_pan
        || spsr & (1 << 23) != 0
    {
        panic!("scheduler saved frame has an invalid EL1 return state");
    }
}

fn all_stack_canaries_intact() -> bool {
    CONTEXTS.iter().all(stack_canary_intact)
}

fn stack_canary_intact(context: &StaticContext) -> bool {
    let address = context.canary_address.load(Ordering::Relaxed);
    if address == 0 {
        return context.state.load(Ordering::Relaxed) == STATE_INACTIVE
            && context.saved_frame.load(Ordering::Relaxed).is_null()
            && context.translation.load(Ordering::Relaxed) == 0;
    }
    unsafe {
        (address as *const u64).read_volatile() == STACK_CANARY
            && ((address + size_of::<u64>()) as *const u64).read_volatile() == STACK_CANARY_INVERSE
    }
}

fn selection_counter(index: usize) -> &'static AtomicU64 {
    match index {
        MONITOR_CONTEXT => &MONITOR_SELECTED,
        WORKER_0_CONTEXT => &SCHED_WORKER_0_SELECTED,
        WORKER_1_CONTEXT => &SCHED_WORKER_1_SELECTED,
        USER_INIT_CONTEXT => &USER_INIT_SELECTED,
        _ => dynamic_user_index(index)
            .map(|dynamic_slot| &USER_DYNAMIC_SELECTED[dynamic_slot])
            .unwrap_or_else(|| panic!("invalid scheduler context index")),
    }
}

fn worker_index(context: usize) -> Option<usize> {
    match context {
        WORKER_0_CONTEXT => Some(0),
        WORKER_1_CONTEXT => Some(1),
        _ => None,
    }
}

fn worker_context(worker: usize) -> usize {
    match worker {
        0 => WORKER_0_CONTEXT,
        1 => WORKER_1_CONTEXT,
        _ => panic!("invalid scheduler worker index"),
    }
}

fn worker_selected_counter(worker: usize) -> &'static AtomicU64 {
    match worker {
        0 => &SCHED_WORKER_0_SELECTED,
        1 => &SCHED_WORKER_1_SELECTED,
        _ => panic!("invalid scheduler worker index"),
    }
}

fn worker_observed_counter(worker: usize) -> &'static AtomicU64 {
    match worker {
        0 => &SCHED_WORKER_0_OBSERVED,
        1 => &SCHED_WORKER_1_OBSERVED,
        _ => panic!("invalid scheduler worker index"),
    }
}

fn worker_work_counter(worker: usize) -> &'static AtomicU64 {
    match worker {
        0 => &SCHED_WORKER_0_WORK,
        1 => &SCHED_WORKER_1_WORK,
        _ => panic!("invalid scheduler worker index"),
    }
}

fn assert_disjoint(left: (usize, usize), right: (usize, usize)) {
    if left.0 < right.1 && right.0 < left.1 {
        panic!("scheduler stacks overlap");
    }
}

#[unsafe(no_mangle)]
extern "C" fn scheduler_thread_returned() -> ! {
    crate::kprintln!("THREAD_EXITED: static scheduler worker returned unexpectedly");
    crate::arch::aarch64::halt()
}
