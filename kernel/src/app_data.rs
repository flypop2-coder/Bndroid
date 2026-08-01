//! M54 kernel-owned AppData service boundary.
//!
//! EL0 never drives the block device from an exception frame. A syscall copies
//! and validates one bounded request, publishes it here, and returns
//! `ShouldWait`. The boot-monitor kernel thread later executes the durable
//! operation with IRQs enabled, after which an exact retry consumes the result.
//! This avoids enabling the scheduler timer while a lower-EL syscall frame is
//! live and gives the single-core implementation an explicit transaction gate.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

use bndr_abi::Rights;
use bndr_appdata::{
    AppDataVolume, DirEntry, DurableVolumeIo, EntryKind, Error, IoError, MAX_ENTRIES,
    MAX_FILE_BYTES, MAX_PATH_BYTES, Mutation, ReadResult, RecoveryInfo, ReplaceCondition, Sector,
    VOLUME_SECTORS,
};

use crate::arch::aarch64;
use crate::driver::virtio::block::{
    BlockError, CooperativeRecoveryProgress, PhysicalCounter, deadline_after,
};
use crate::storage::{self, StorageError};
#[cfg(feature = "app-data-async-recovery-runtime")]
use bndroid_kernel::virtio::BlockRequestKind;

const EMPTY: u8 = 0;
const PENDING: u8 = 1;
const RUNNING: u8 = 2;
const COMPLETE: u8 = 3;
const RECOVERING: u8 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RequestKind {
    CreateDirectory,
    Replace,
    Unlink,
    Read,
    ReadDirectory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Request {
    kind: RequestKind,
    principal: u64,
    path: [u8; MAX_PATH_BYTES],
    path_len: u8,
    condition: ReplaceCondition,
    data_len: u16,
    cursor: u32,
}

impl Request {
    const EMPTY: Self = Self {
        kind: RequestKind::Read,
        principal: 0,
        path: [0; MAX_PATH_BYTES],
        path_len: 0,
        condition: ReplaceCondition::Any,
        data_len: 0,
        cursor: 0,
    };

    fn new_path(kind: RequestKind, principal: u64, path: &str) -> Self {
        let mut request = Self {
            kind,
            principal,
            ..Self::EMPTY
        };
        request.path_len = u8::try_from(path.len()).unwrap_or(0);
        request.path[..path.len()].copy_from_slice(path.as_bytes());
        request
    }

    fn path(&self) -> &str {
        core::str::from_utf8(&self.path[..usize::from(self.path_len)])
            .unwrap_or_else(|_| panic!("validated AppData request lost UTF-8 canonicality"))
    }

    const fn required_rights(&self) -> Rights {
        match self.kind {
            RequestKind::CreateDirectory | RequestKind::Replace | RequestKind::Unlink => {
                Rights::WRITE
            }
            RequestKind::Read | RequestKind::ReadDirectory => Rights::READ,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectoryCompletion {
    pub entry: DirEntry,
    pub next_cursor: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Outcome {
    None,
    Mutation(Result<Mutation, Error>),
    Read(Result<ReadResult, Error>),
    Directory(Result<DirectoryCompletion, Error>),
}

struct Workspace {
    request: Request,
    data: [u8; MAX_FILE_BYTES],
    directory: [DirEntry; MAX_ENTRIES],
    outcome: Outcome,
}

impl Workspace {
    const fn new() -> Self {
        Self {
            request: Request::EMPTY,
            data: [0; MAX_FILE_BYTES],
            directory: [DirEntry::EMPTY; MAX_ENTRIES],
            outcome: Outcome::None,
        }
    }
}

struct ServiceSlot {
    state: AtomicU8,
    owner: AtomicU64,
    key_a: AtomicU64,
    key_b: AtomicU64,
    value: UnsafeCell<Workspace>,
}

impl ServiceSlot {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(EMPTY),
            owner: AtomicU64::new(0),
            key_a: AtomicU64::new(0),
            key_b: AtomicU64::new(0),
            value: UnsafeCell::new(Workspace::new()),
        }
    }
}

// SAFETY: the state machine publishes one mutable owner at a time. Syscall
// paths may dereference `value` only while IRQ-masked and state is EMPTY or
// COMPLETE. The monitor dereferences it only after PENDING -> RUNNING or while
// finalizing RECOVERING, and no syscall dereferences RUNNING/RECOVERING
// storage.
unsafe impl Sync for ServiceSlot {}

static SERVICE: ServiceSlot = ServiceSlot::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);
static RUNTIME_READY: AtomicBool = AtomicBool::new(false);
static FIRST_LBA: AtomicU64 = AtomicU64::new(0);
static DEVICE_SECTORS: AtomicU64 = AtomicU64::new(0);
static DEADLINE_BUDGET: AtomicU64 = AtomicU64::new(0);

static BOOT_FORMATTED: AtomicBool = AtomicBool::new(false);
static BOOT_GENERATION: AtomicU64 = AtomicU64::new(0);
static BOOT_VALID_SNAPSHOTS: AtomicU64 = AtomicU64::new(0);
static BOOT_REJECTED_SNAPSHOTS: AtomicU64 = AtomicU64::new(0);
static BOOT_ENTRY_COUNT: AtomicU64 = AtomicU64::new(0);
static BOOT_LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static BOOT_READS: AtomicU64 = AtomicU64::new(0);
static BOOT_WRITES: AtomicU64 = AtomicU64::new(0);
static BOOT_FLUSHES: AtomicU64 = AtomicU64::new(0);
static RUNTIME_READS: AtomicU64 = AtomicU64::new(0);
static RUNTIME_WRITES: AtomicU64 = AtomicU64::new(0);
static RUNTIME_FLUSHES: AtomicU64 = AtomicU64::new(0);
static SUBMISSIONS: AtomicU64 = AtomicU64::new(0);
static SERVICE_COMPLETIONS: AtomicU64 = AtomicU64::new(0);
static RETRIEVALS: AtomicU64 = AtomicU64::new(0);
static MUTATION_COMMITS: AtomicU64 = AtomicU64::new(0);
static READ_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static DIRECTORY_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static CONFLICTS: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);
static OUTCOME_UNKNOWN: AtomicU64 = AtomicU64::new(0);
static ABANDONED: AtomicU64 = AtomicU64::new(0);
static LAST_GENERATION: AtomicU64 = AtomicU64::new(0);

// The AppData coordinator owns no reference across monitor turns. The durable
// operation's terminal Outcome stays in SERVICE while these atomics describe
// only the outer physical/IRQ/admission transaction.
static ASYNC_COORDINATOR_ACTIVE: AtomicBool = AtomicBool::new(false);
static ASYNC_PHYSICAL_COMPLETE: AtomicBool = AtomicBool::new(false);
static ASYNC_ACTIVE_IRQ_ID: AtomicU64 = AtomicU64::new(0);
static ASYNC_RECOVERY_STARTS: AtomicU64 = AtomicU64::new(0);
static ASYNC_PHYSICAL_COMPLETIONS: AtomicU64 = AtomicU64::new(0);
static ASYNC_RECOVERY_COMMITS: AtomicU64 = AtomicU64::new(0);
static ASYNC_RECOVERY_FAILURES: AtomicU64 = AtomicU64::new(0);
static ASYNC_RECOVERY_YIELDS: AtomicU64 = AtomicU64::new(0);
static ASYNC_TIMER_PROGRESS_WINDOWS: AtomicU64 = AtomicU64::new(0);
static ASYNC_WORKER_PROGRESS_WINDOWS: AtomicU64 = AtomicU64::new(0);
static ASYNC_EL0_PROGRESS_WINDOWS: AtomicU64 = AtomicU64::new(0);
static ASYNC_AUTHENTICATED_WAITS: AtomicU64 = AtomicU64::new(0);
static ASYNC_AUTHENTICATED_WAIT_DISPATCH_CHANGES: AtomicU64 = AtomicU64::new(0);
static ASYNC_LAST_AUTHENTICATED_WAIT_DISPATCH: AtomicU64 = AtomicU64::new(u64::MAX);
static ASYNC_BASE_TIMER_DISPATCHES: AtomicU64 = AtomicU64::new(0);
static ASYNC_BASE_WORKER_0: AtomicU64 = AtomicU64::new(0);
static ASYNC_BASE_WORKER_1: AtomicU64 = AtomicU64::new(0);
static ASYNC_BASE_AUTHENTICATED_WAITS: AtomicU64 = AtomicU64::new(0);
static ASYNC_BASE_AUTHENTICATED_WAIT_DISPATCH_CHANGES: AtomicU64 = AtomicU64::new(0);
static ASYNC_MAX_CONTROL_MASKED_TICKS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-async-recovery-runtime")]
static INJECTED_READ_NOTIFICATION_LOSSES: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub initialized: bool,
    pub formatted: bool,
    pub boot_generation: u64,
    pub boot_valid_snapshots: u64,
    pub boot_rejected_snapshots: u64,
    pub boot_entry_count: u64,
    pub boot_live_bytes: u64,
    pub boot_reads: u64,
    pub boot_writes: u64,
    pub boot_flushes: u64,
    pub runtime_reads: u64,
    pub runtime_writes: u64,
    pub runtime_flushes: u64,
    pub submissions: u64,
    pub service_completions: u64,
    pub retrievals: u64,
    pub mutation_commits: u64,
    pub read_successes: u64,
    pub directory_successes: u64,
    pub conflicts: u64,
    pub errors: u64,
    pub outcome_unknown: u64,
    pub abandoned: u64,
    pub last_generation: u64,
    pub queue_state: u8,
    pub injected_read_notification_losses: u64,
    pub async_recovery_starts: u64,
    pub async_physical_completions: u64,
    pub async_recovery_commits: u64,
    pub async_recovery_failures: u64,
    pub async_recovery_yields: u64,
    pub async_timer_progress_windows: u64,
    pub async_worker_progress_windows: u64,
    pub async_el0_progress_windows: u64,
    pub async_authenticated_waits: u64,
    pub async_authenticated_wait_dispatch_changes: u64,
    pub async_max_control_masked_ticks: u64,
    pub async_active: bool,
    pub async_physical_complete: bool,
}

pub enum Poll<T> {
    Pending,
    Ready(Result<T, Error>),
}

pub enum ReadDisposition<T> {
    Consume(T),
    Preserve(T),
}

#[derive(Clone, Copy)]
enum IoPhase {
    Boot,
    Runtime,
}

struct StorageAdapter {
    clock: PhysicalCounter,
    deadline_budget: u64,
    device_sectors: u64,
    phase: IoPhase,
}

impl StorageAdapter {
    fn new(deadline_budget: u64, device_sectors: u64, phase: IoPhase) -> Self {
        Self {
            clock: PhysicalCounter,
            deadline_budget,
            device_sectors,
            phase,
        }
    }

    fn deadline(&mut self) -> Result<u64, IoError> {
        deadline_after(&mut self.clock, self.deadline_budget).ok_or(IoError::Device)
    }

    fn record_read(&self) {
        match self.phase {
            IoPhase::Boot => BOOT_READS.fetch_add(1, Ordering::Relaxed),
            IoPhase::Runtime => RUNTIME_READS.fetch_add(1, Ordering::Relaxed),
        };
    }

    fn record_write(&self) {
        match self.phase {
            IoPhase::Boot => BOOT_WRITES.fetch_add(1, Ordering::Relaxed),
            IoPhase::Runtime => RUNTIME_WRITES.fetch_add(1, Ordering::Relaxed),
        };
    }

    fn record_flush(&self) {
        match self.phase {
            IoPhase::Boot => BOOT_FLUSHES.fetch_add(1, Ordering::Relaxed),
            IoPhase::Runtime => RUNTIME_FLUSHES.fetch_add(1, Ordering::Relaxed),
        };
    }

    fn in_appdata(&self, lba: u64) -> bool {
        let first = FIRST_LBA.load(Ordering::Acquire);
        (first..first.saturating_add(VOLUME_SECTORS)).contains(&lba)
    }
}

impl DurableVolumeIo for StorageAdapter {
    fn sector_count(&self) -> u64 {
        self.device_sectors
    }

    fn read_sector(&mut self, lba: u64, output: &mut Sector) -> Result<(), IoError> {
        if !self.in_appdata(lba) {
            return Err(IoError::OutOfBounds);
        }
        let deadline = self.deadline()?;
        storage::read_sector_irq(lba, output, &mut self.clock, deadline).map_err(map_read_error)?;
        self.record_read();
        Ok(())
    }

    fn write_sector(&mut self, lba: u64, input: &Sector) -> Result<(), IoError> {
        if !self.in_appdata(lba) {
            return Err(IoError::OutOfBounds);
        }
        let deadline = self.deadline()?;
        storage::write_appdata_sector_irq(lba, input, &mut self.clock, deadline)
            .map_err(map_mutation_error)?;
        self.record_write();
        Ok(())
    }

    fn flush(&mut self) -> Result<(), IoError> {
        let deadline = self.deadline()?;
        storage::flush_irq(&mut self.clock, deadline).map_err(map_mutation_error)?;
        self.record_flush();
        Ok(())
    }
}

pub fn initialize(
    counter_frequency: u64,
    first_lba: u64,
    partition_sectors: u64,
) -> Result<RecoveryInfo, Error> {
    if INITIALIZED.load(Ordering::Acquire) {
        return Err(Error::AlreadyFormatted);
    }
    if counter_frequency == 0
        || first_lba != storage::APPDATA_PARTITION_FIRST_LBA
        || partition_sectors != VOLUME_SECTORS
        || partition_sectors != storage::APPDATA_PARTITION_SECTORS
        || aarch64::irq_is_masked()
    {
        return Err(Error::VolumeBounds);
    }
    let (device_sectors, read_only) =
        storage::device_contract().map_err(|error| Error::Io(map_read_error(error)))?;
    if read_only || first_lba.saturating_add(partition_sectors) > device_sectors {
        return Err(Error::VolumeBounds);
    }
    FIRST_LBA.store(first_lba, Ordering::Release);
    DEVICE_SECTORS.store(device_sectors, Ordering::Release);
    DEADLINE_BUDGET.store(counter_frequency, Ordering::Release);

    let volume = AppDataVolume::new(first_lba);
    let mut io = StorageAdapter::new(counter_frequency, device_sectors, IoPhase::Boot);
    let (formatted, recovery) = match volume.recover(&mut io) {
        Ok(recovery) => (false, recovery),
        Err(Error::Unformatted) => (true, volume.format_virgin(&mut io)?),
        Err(error) => return Err(error),
    };
    BOOT_FORMATTED.store(formatted, Ordering::Relaxed);
    publish_boot_recovery(recovery);
    LAST_GENERATION.store(recovery.generation, Ordering::Relaxed);
    INITIALIZED.store(true, Ordering::Release);
    Ok(recovery)
}

/// Opens the AppData authority gate only after the monitor has independently
/// verified the complete M52/M53 wire protocol. The monitor still withholds
/// the predecessor seals until the bounded AppData probe closes its temporary
/// capabilities and restores the exact resident object-wait topology.
pub fn publish_runtime_ready() {
    if !INITIALIZED.load(Ordering::Acquire) {
        panic!("AppData runtime gate opened before volume initialization");
    }
    RUNTIME_READY.store(true, Ordering::Release);
}

pub fn runtime_ready() -> bool {
    RUNTIME_READY.load(Ordering::Acquire)
}

pub fn poll_create_directory(owner: u64, principal: u64, path: &str) -> Poll<Mutation> {
    let request = Request::new_path(RequestKind::CreateDirectory, principal, path);
    poll_mutation(owner, request, &[])
}

pub fn poll_replace(
    owner: u64,
    principal: u64,
    path: &str,
    bytes: &[u8],
    condition: ReplaceCondition,
) -> Poll<Mutation> {
    let mut request = Request::new_path(RequestKind::Replace, principal, path);
    request.condition = condition;
    request.data_len = u16::try_from(bytes.len()).unwrap_or(u16::MAX);
    poll_mutation(owner, request, bytes)
}

pub fn poll_unlink(owner: u64, principal: u64, path: &str) -> Poll<Mutation> {
    let request = Request::new_path(RequestKind::Unlink, principal, path);
    poll_mutation(owner, request, &[])
}

pub fn poll_directory(owner: u64, principal: u64, cursor: u32) -> Poll<DirectoryCompletion> {
    let mut request = Request {
        kind: RequestKind::ReadDirectory,
        principal,
        ..Request::EMPTY
    };
    request.cursor = cursor;
    let (key_a, key_b) = request_keys(owner, &request, &[]);
    match poll_slot(owner, key_a, key_b, &request, &[]) {
        SlotPoll::Enqueue => {
            enqueue(owner, key_a, key_b, request, &[]);
            Poll::Pending
        }
        SlotPoll::Pending => Poll::Pending,
        SlotPoll::Complete => {
            let outcome = completed_workspace().outcome;
            let Outcome::Directory(result) = outcome else {
                panic!("AppData directory request completed with the wrong outcome kind");
            };
            consume_completed();
            RETRIEVALS.fetch_add(1, Ordering::Relaxed);
            Poll::Ready(result)
        }
    }
}

pub fn poll_read<T>(
    owner: u64,
    principal: u64,
    path: &str,
    consume: impl FnOnce(Result<(&[u8], ReadResult), Error>) -> ReadDisposition<T>,
) -> Result<Option<T>, Error> {
    let request = Request::new_path(RequestKind::Read, principal, path);
    let (key_a, key_b) = request_keys(owner, &request, &[]);
    match poll_slot(owner, key_a, key_b, &request, &[]) {
        SlotPoll::Enqueue => {
            enqueue(owner, key_a, key_b, request, &[]);
            Ok(None)
        }
        SlotPoll::Pending => Ok(None),
        SlotPoll::Complete => {
            let workspace = completed_workspace();
            let Outcome::Read(result) = workspace.outcome else {
                panic!("AppData read request completed with the wrong outcome kind");
            };
            let completion = result.map(|read| (&workspace.data[..read.bytes], read));
            match consume(completion) {
                ReadDisposition::Consume(value) => {
                    consume_completed();
                    RETRIEVALS.fetch_add(1, Ordering::Relaxed);
                    Ok(Some(value))
                }
                ReadDisposition::Preserve(value) => Ok(Some(value)),
            }
        }
    }
}

/// Executes at most one queued operation from the IRQ-enabled monitor thread.
pub fn service_pending() {
    if !INITIALIZED.load(Ordering::Acquire) {
        return;
    }
    if aarch64::irq_is_masked() {
        panic!("AppData service attempted durable I/O with IRQ masked");
    }
    let saved_daif = aarch64::save_and_mask_irq();
    match SERVICE.state.load(Ordering::Acquire) {
        PENDING if !published_owner_can_retry() => {
            discard_abandoned();
            SERVICE_COMPLETIONS.fetch_add(1, Ordering::Relaxed);
            aarch64::restore_daif(saved_daif);
            return;
        }
        COMPLETE if !published_owner_can_retry() => {
            discard_abandoned();
            aarch64::restore_daif(saved_daif);
            return;
        }
        PENDING => {}
        RECOVERING => {
            aarch64::restore_daif(saved_daif);
            service_recovering();
            return;
        }
        EMPTY | RUNNING | COMPLETE => {
            aarch64::restore_daif(saved_daif);
            return;
        }
        _ => panic!("AppData queue entered an invalid state"),
    }
    crate::scheduler::assert_current_kernel_stack_headroom(aarch64::stack_pointer(), 160 * 1024);
    if SERVICE
        .state
        .compare_exchange(PENDING, RUNNING, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        aarch64::restore_daif(saved_daif);
        return;
    }
    aarch64::restore_daif(saved_daif);

    execute_running_operation();
    if storage::recovery_required() {
        // Publish the frozen Outcome before exposing RECOVERING. The original
        // mutation is never replayed: recovery only makes a later explicit
        // request safe.
        storage::require_recovery();
        SERVICE.state.store(RECOVERING, Ordering::Release);
        start_async_recovery_attempt();
        return;
    }

    let saved_daif = aarch64::save_and_mask_irq();
    finalize_service_irq_masked();
    aarch64::restore_daif(saved_daif);
}

/// Executes one complete AppData operation. Every reference in this function
/// dies before a cooperative recovery attempt can span monitor turns.
fn execute_running_operation() {
    if SERVICE.state.load(Ordering::Acquire) != RUNNING {
        panic!("AppData operation executed outside the RUNNING state");
    }
    // SAFETY: RUNNING grants the monitor exclusive workspace access. Syscall
    // paths consult only atomics until COMPLETE is published.
    let workspace = unsafe { &mut *SERVICE.value.get() };
    let request = workspace.request;
    #[cfg(feature = "app-data-async-recovery-runtime")]
    arm_one_read_notification_loss(request.kind);

    let first_lba = FIRST_LBA.load(Ordering::Acquire);
    let device_sectors = DEVICE_SECTORS.load(Ordering::Acquire);
    let deadline_budget = DEADLINE_BUDGET.load(Ordering::Acquire);
    let volume = AppDataVolume::new(first_lba);
    let mut io = StorageAdapter::new(deadline_budget, device_sectors, IoPhase::Runtime);
    let path = request.path();
    let principal = request.principal;
    workspace.outcome = match request.kind {
        RequestKind::CreateDirectory => {
            Outcome::Mutation(volume.create_dir(&mut io, principal, path))
        }
        RequestKind::Replace => {
            let length = usize::from(request.data_len);
            Outcome::Mutation(volume.replace(
                &mut io,
                principal,
                path,
                &workspace.data[..length],
                request.condition,
            ))
        }
        RequestKind::Unlink => Outcome::Mutation(volume.unlink(&mut io, principal, path)),
        RequestKind::Read => {
            workspace.data.fill(0);
            Outcome::Read(volume.read(&mut io, principal, path, &mut workspace.data))
        }
        RequestKind::ReadDirectory => {
            workspace.directory.fill(DirEntry::EMPTY);
            let listed = volume.list_all(&mut io, principal, &mut workspace.directory);
            let result = listed.and_then(|list| {
                let cursor = usize::try_from(request.cursor).map_err(|_| Error::NotFound)?;
                if cursor >= list.total || cursor >= list.written {
                    return Err(Error::NotFound);
                }
                let next_cursor = request
                    .cursor
                    .checked_add(1)
                    .ok_or(Error::GenerationExhausted)?;
                Ok(DirectoryCompletion {
                    entry: workspace.directory[cursor],
                    next_cursor,
                })
            });
            Outcome::Directory(result)
        }
    };
}

#[cfg(feature = "app-data-async-recovery-runtime")]
fn arm_one_read_notification_loss(kind: RequestKind) {
    if kind != RequestKind::Read || INJECTED_READ_NOTIFICATION_LOSSES.load(Ordering::Acquire) != 0 {
        return;
    }

    storage::suppress_next_notification_for_recovery_test(BlockRequestKind::Read).unwrap_or_else(
        |error| {
            panic!(
                "AppData async recovery could not arm its read notification loss: {}",
                error.as_str()
            )
        },
    );
    if INJECTED_READ_NOTIFICATION_LOSSES
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        panic!("AppData read notification loss was armed more than once");
    }
}

/// Starts a fresh physical attempt for the frozen RECOVERING request. Failure
/// leaves both storage latches closed and the slot intact for a later retry.
fn start_async_recovery_attempt() {
    if aarch64::irq_is_masked()
        || SERVICE.state.load(Ordering::Acquire) != RECOVERING
        || ASYNC_PHYSICAL_COMPLETE.load(Ordering::Acquire)
        || storage::async_recovery_active()
    {
        panic!("AppData recovery attempt started from an invalid state");
    }
    if ASYNC_COORDINATOR_ACTIVE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        panic!("AppData recovery attempt overlapped its predecessor");
    }
    storage::require_recovery();

    let Some(block_irq) = crate::interrupt::registered_block_irq() else {
        fail_async_recovery_attempt();
        return;
    };
    if storage::irq_id() != Some(block_irq.id) {
        fail_async_recovery_attempt();
        return;
    }
    ASYNC_ACTIVE_IRQ_ID.store(u64::from(block_irq.id), Ordering::Release);

    let saved_daif = aarch64::save_and_mask_irq();
    let control_started = aarch64::timer::counter_value();
    let disabled = crate::interrupt::disable_block_irq_masked(block_irq);
    record_async_control_masked_since(control_started);
    aarch64::restore_daif(saved_daif);
    if disabled.is_err() {
        fail_async_recovery_attempt();
        return;
    }

    ASYNC_RECOVERY_STARTS.fetch_add(1, Ordering::Relaxed);
    let mut clock = PhysicalCounter;
    let budget = DEADLINE_BUDGET.load(Ordering::Acquire);
    let started = deadline_after(&mut clock, budget)
        .ok_or(StorageError::NotReady)
        .and_then(|deadline| storage::begin_async_recovery(&mut clock, deadline));
    match started {
        Ok(CooperativeRecoveryProgress::Pending(_)) => {
            ASYNC_RECOVERY_YIELDS.fetch_add(1, Ordering::Relaxed);
        }
        Ok(CooperativeRecoveryProgress::Complete) => {
            panic!("AppData cooperative recovery completed without yielding")
        }
        Err(_) => fail_async_recovery_attempt(),
    }
}

/// Advances at most one driver phase per call. A completed physical rebuild is
/// deliberately followed by a fresh timer/worker/authenticated-EL0 window
/// before the short IRQ/admission commit is allowed to run.
fn service_recovering() {
    if aarch64::irq_is_masked() || SERVICE.state.load(Ordering::Acquire) != RECOVERING {
        panic!("AppData recovery monitor entered from an invalid state");
    }
    if !ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire) {
        if storage::async_recovery_active() {
            panic!("AppData physical recovery escaped its outer coordinator");
        }
        start_async_recovery_attempt();
        return;
    }

    let block_irq = active_block_irq();
    if ASYNC_PHYSICAL_COMPLETE.load(Ordering::Acquire) {
        if storage::async_recovery_active() {
            panic!("AppData physical completion retained an active driver phase");
        }
        if !async_progress_window_complete() {
            ASYNC_RECOVERY_YIELDS.fetch_add(1, Ordering::Relaxed);
            return;
        }
        ASYNC_TIMER_PROGRESS_WINDOWS.fetch_add(1, Ordering::Relaxed);
        ASYNC_WORKER_PROGRESS_WINDOWS.fetch_add(1, Ordering::Relaxed);
        ASYNC_EL0_PROGRESS_WINDOWS.fetch_add(1, Ordering::Relaxed);
        commit_async_recovery(block_irq);
        return;
    }

    if !storage::async_recovery_active() {
        panic!("AppData outer recovery coordinator lost its physical phase");
    }
    let mut clock = PhysicalCounter;
    match storage::poll_async_recovery(&mut clock) {
        Ok(CooperativeRecoveryProgress::Pending(_)) => {
            ASYNC_RECOVERY_YIELDS.fetch_add(1, Ordering::Relaxed);
        }
        Err(_) => fail_async_recovery_attempt(),
        Ok(CooperativeRecoveryProgress::Complete) => {
            ASYNC_PHYSICAL_COMPLETIONS.fetch_add(1, Ordering::Relaxed);
            publish_post_physical_progress_baseline();
            ASYNC_RECOVERY_YIELDS.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn active_block_irq() -> crate::interrupt::BlockIrqConfig {
    let expected = ASYNC_ACTIVE_IRQ_ID.load(Ordering::Acquire);
    let Some(block_irq) = crate::interrupt::registered_block_irq() else {
        storage::require_recovery();
        panic!("AppData block IRQ registration disappeared during recovery");
    };
    if expected == 0
        || expected != u64::from(block_irq.id)
        || storage::irq_id() != Some(block_irq.id)
    {
        storage::require_recovery();
        panic!("AppData block IRQ identity changed during recovery");
    }
    block_irq
}

fn publish_post_physical_progress_baseline() {
    let saved_daif = aarch64::save_and_mask_irq();
    let control_started = aarch64::timer::counter_value();
    let scheduler = crate::scheduler::snapshot();
    ASYNC_BASE_TIMER_DISPATCHES.store(scheduler.timer_dispatches, Ordering::Relaxed);
    ASYNC_BASE_WORKER_0.store(scheduler.worker_work[0], Ordering::Relaxed);
    ASYNC_BASE_WORKER_1.store(scheduler.worker_work[1], Ordering::Relaxed);
    ASYNC_BASE_AUTHENTICATED_WAITS.store(
        ASYNC_AUTHENTICATED_WAITS.load(Ordering::Relaxed),
        Ordering::Relaxed,
    );
    ASYNC_BASE_AUTHENTICATED_WAIT_DISPATCH_CHANGES.store(
        ASYNC_AUTHENTICATED_WAIT_DISPATCH_CHANGES.load(Ordering::Relaxed),
        Ordering::Relaxed,
    );
    ASYNC_LAST_AUTHENTICATED_WAIT_DISPATCH.store(u64::MAX, Ordering::Relaxed);
    ASYNC_PHYSICAL_COMPLETE.store(true, Ordering::Release);
    record_async_control_masked_since(control_started);
    aarch64::restore_daif(saved_daif);
}

fn async_progress_window_complete() -> bool {
    let scheduler = crate::scheduler::snapshot();
    scheduler
        .timer_dispatches
        .saturating_sub(ASYNC_BASE_TIMER_DISPATCHES.load(Ordering::Acquire))
        >= 2
        && scheduler.worker_work[0] > ASYNC_BASE_WORKER_0.load(Ordering::Acquire)
        && scheduler.worker_work[1] > ASYNC_BASE_WORKER_1.load(Ordering::Acquire)
        && ASYNC_AUTHENTICATED_WAITS
            .load(Ordering::Acquire)
            .saturating_sub(ASYNC_BASE_AUTHENTICATED_WAITS.load(Ordering::Acquire))
            >= 2
        && ASYNC_AUTHENTICATED_WAIT_DISPATCH_CHANGES
            .load(Ordering::Acquire)
            .saturating_sub(ASYNC_BASE_AUTHENTICATED_WAIT_DISPATCH_CHANGES.load(Ordering::Acquire))
            >= 2
}

fn commit_async_recovery(block_irq: crate::interrupt::BlockIrqConfig) {
    let saved_daif = aarch64::save_and_mask_irq();
    let control_started = aarch64::timer::counter_value();
    let rearmed = crate::interrupt::reenable_block_irq_masked(block_irq);
    let committed = if rearmed.is_ok() {
        match storage::open_recovery_admission(block_irq.id) {
            Ok(()) => true,
            Err(_) => {
                crate::interrupt::rollback_block_irq_rearm_masked(block_irq).unwrap_or_else(
                    |error| {
                        panic!(
                            "AppData admission failure could not roll back block IRQ: {}",
                            error.as_str()
                        )
                    },
                );
                false
            }
        }
    } else {
        false
    };

    if committed {
        ASYNC_RECOVERY_COMMITS.fetch_add(1, Ordering::Relaxed);
        ASYNC_PHYSICAL_COMPLETE.store(false, Ordering::Relaxed);
        ASYNC_ACTIVE_IRQ_ID.store(0, Ordering::Relaxed);
        ASYNC_COORDINATOR_ACTIVE.store(false, Ordering::Release);
        finalize_service_irq_masked();
    }
    record_async_control_masked_since(control_started);
    aarch64::restore_daif(saved_daif);

    if !committed {
        fail_async_recovery_attempt();
    }
}

fn fail_async_recovery_attempt() {
    storage::require_recovery();
    ASYNC_PHYSICAL_COMPLETE.store(false, Ordering::Release);
    ASYNC_ACTIVE_IRQ_ID.store(0, Ordering::Release);
    ASYNC_COORDINATOR_ACTIVE.store(false, Ordering::Release);
    ASYNC_RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
}

fn record_async_control_masked_since(started: u64) {
    let elapsed = aarch64::timer::counter_value().wrapping_sub(started);
    ASYNC_MAX_CONTROL_MASKED_TICKS.fetch_max(elapsed, Ordering::Relaxed);
}

fn finalize_service_irq_masked() {
    if !aarch64::irq_is_masked()
        || !matches!(SERVICE.state.load(Ordering::Acquire), RUNNING | RECOVERING)
    {
        panic!("AppData outcome finalized outside its exclusive state");
    }
    // SAFETY: RUNNING/RECOVERING grants the monitor the only workspace read;
    // Outcome is Copy and no reference escapes this statement.
    let outcome = unsafe { (*SERVICE.value.get()).outcome };
    record_outcome(outcome);
    SERVICE_COMPLETIONS.fetch_add(1, Ordering::Relaxed);
    if published_owner_can_retry() {
        SERVICE.state.store(COMPLETE, Ordering::Release);
    } else {
        discard_abandoned();
    }
}

pub fn snapshot() -> Snapshot {
    Snapshot {
        initialized: INITIALIZED.load(Ordering::Acquire),
        formatted: BOOT_FORMATTED.load(Ordering::Relaxed),
        boot_generation: BOOT_GENERATION.load(Ordering::Relaxed),
        boot_valid_snapshots: BOOT_VALID_SNAPSHOTS.load(Ordering::Relaxed),
        boot_rejected_snapshots: BOOT_REJECTED_SNAPSHOTS.load(Ordering::Relaxed),
        boot_entry_count: BOOT_ENTRY_COUNT.load(Ordering::Relaxed),
        boot_live_bytes: BOOT_LIVE_BYTES.load(Ordering::Relaxed),
        boot_reads: BOOT_READS.load(Ordering::Relaxed),
        boot_writes: BOOT_WRITES.load(Ordering::Relaxed),
        boot_flushes: BOOT_FLUSHES.load(Ordering::Relaxed),
        runtime_reads: RUNTIME_READS.load(Ordering::Relaxed),
        runtime_writes: RUNTIME_WRITES.load(Ordering::Relaxed),
        runtime_flushes: RUNTIME_FLUSHES.load(Ordering::Relaxed),
        submissions: SUBMISSIONS.load(Ordering::Relaxed),
        service_completions: SERVICE_COMPLETIONS.load(Ordering::Relaxed),
        retrievals: RETRIEVALS.load(Ordering::Relaxed),
        mutation_commits: MUTATION_COMMITS.load(Ordering::Relaxed),
        read_successes: READ_SUCCESSES.load(Ordering::Relaxed),
        directory_successes: DIRECTORY_SUCCESSES.load(Ordering::Relaxed),
        conflicts: CONFLICTS.load(Ordering::Relaxed),
        errors: ERRORS.load(Ordering::Relaxed),
        outcome_unknown: OUTCOME_UNKNOWN.load(Ordering::Relaxed),
        abandoned: ABANDONED.load(Ordering::Relaxed),
        last_generation: LAST_GENERATION.load(Ordering::Relaxed),
        queue_state: SERVICE.state.load(Ordering::Acquire),
        injected_read_notification_losses: injected_read_notification_losses(),
        async_recovery_starts: ASYNC_RECOVERY_STARTS.load(Ordering::Acquire),
        async_physical_completions: ASYNC_PHYSICAL_COMPLETIONS.load(Ordering::Acquire),
        async_recovery_commits: ASYNC_RECOVERY_COMMITS.load(Ordering::Acquire),
        async_recovery_failures: ASYNC_RECOVERY_FAILURES.load(Ordering::Acquire),
        async_recovery_yields: ASYNC_RECOVERY_YIELDS.load(Ordering::Acquire),
        async_timer_progress_windows: ASYNC_TIMER_PROGRESS_WINDOWS.load(Ordering::Acquire),
        async_worker_progress_windows: ASYNC_WORKER_PROGRESS_WINDOWS.load(Ordering::Acquire),
        async_el0_progress_windows: ASYNC_EL0_PROGRESS_WINDOWS.load(Ordering::Acquire),
        async_authenticated_waits: ASYNC_AUTHENTICATED_WAITS.load(Ordering::Acquire),
        async_authenticated_wait_dispatch_changes: ASYNC_AUTHENTICATED_WAIT_DISPATCH_CHANGES
            .load(Ordering::Acquire),
        async_max_control_masked_ticks: ASYNC_MAX_CONTROL_MASKED_TICKS.load(Ordering::Acquire),
        async_active: ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire),
        async_physical_complete: ASYNC_PHYSICAL_COMPLETE.load(Ordering::Acquire),
    }
}

#[cfg(feature = "app-data-async-recovery-runtime")]
fn injected_read_notification_losses() -> u64 {
    INJECTED_READ_NOTIFICATION_LOSSES.load(Ordering::Acquire)
}

#[cfg(not(feature = "app-data-async-recovery-runtime"))]
const fn injected_read_notification_losses() -> u64 {
    0
}

/// Records one exact-retry wait from the already authenticated syscall path.
/// Only the owner of the frozen request, after physical completion, can
/// contribute to the post-recovery EL0 progress window.
pub fn record_authenticated_recovery_wait(owner: u64) {
    if !aarch64::irq_is_masked() {
        panic!("AppData authenticated recovery wait recorded with IRQ enabled");
    }
    if owner == 0
        || SERVICE.state.load(Ordering::Acquire) != RECOVERING
        || !ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire)
        || !ASYNC_PHYSICAL_COMPLETE.load(Ordering::Acquire)
        || SERVICE.owner.load(Ordering::Relaxed) != owner
    {
        return;
    }

    ASYNC_AUTHENTICATED_WAITS.fetch_add(1, Ordering::Relaxed);
    let dispatches = crate::scheduler::snapshot().timer_dispatches;
    if ASYNC_LAST_AUTHENTICATED_WAIT_DISPATCH.swap(dispatches, Ordering::AcqRel) != dispatches {
        ASYNC_AUTHENTICATED_WAIT_DISPATCH_CHANGES.fetch_add(1, Ordering::Relaxed);
    }
}

fn poll_mutation(owner: u64, request: Request, bytes: &[u8]) -> Poll<Mutation> {
    let (key_a, key_b) = request_keys(owner, &request, bytes);
    match poll_slot(owner, key_a, key_b, &request, bytes) {
        SlotPoll::Enqueue => {
            enqueue(owner, key_a, key_b, request, bytes);
            Poll::Pending
        }
        SlotPoll::Pending => Poll::Pending,
        SlotPoll::Complete => {
            let outcome = completed_workspace().outcome;
            let Outcome::Mutation(result) = outcome else {
                panic!("AppData mutation completed with the wrong outcome kind");
            };
            consume_completed();
            RETRIEVALS.fetch_add(1, Ordering::Relaxed);
            Poll::Ready(result)
        }
    }
}

#[derive(Clone, Copy)]
enum SlotPoll {
    Enqueue,
    Pending,
    Complete,
}

fn poll_slot(owner: u64, key_a: u64, key_b: u64, request: &Request, bytes: &[u8]) -> SlotPoll {
    if !aarch64::irq_is_masked() {
        panic!("AppData queue polled outside the syscall IRQ domain");
    }
    match SERVICE.state.load(Ordering::Acquire) {
        EMPTY => SlotPoll::Enqueue,
        PENDING | RUNNING | RECOVERING => SlotPoll::Pending,
        COMPLETE => {
            if SERVICE.owner.load(Ordering::Relaxed) == owner
                && SERVICE.key_a.load(Ordering::Relaxed) == key_a
                && SERVICE.key_b.load(Ordering::Relaxed) == key_b
                && completed_request_matches(request, bytes)
            {
                SlotPoll::Complete
            } else if SERVICE.owner.load(Ordering::Relaxed) == owner {
                // A different operation from the same authenticated process is
                // an explicit abandonment of the prior terminal result. Its
                // durable effect remains discoverable through the new request.
                discard_abandoned();
                SlotPoll::Enqueue
            } else {
                SlotPoll::Pending
            }
        }
        _ => panic!("AppData queue entered an invalid state"),
    }
}

fn completed_request_matches(request: &Request, bytes: &[u8]) -> bool {
    if SERVICE.state.load(Ordering::Acquire) != COMPLETE
        || bytes.len() != usize::from(request.data_len)
    {
        return false;
    }
    // SAFETY: COMPLETE makes the request and its submitted input immutable;
    // local syscall handling is serialized by the IRQ mask. Read operations
    // may replace the remainder of `data` with output, so only the submitted
    // request-length prefix participates in exact retry identity.
    let workspace = unsafe { &*SERVICE.value.get() };
    workspace.request == *request && workspace.data[..bytes.len()] == *bytes
}

fn enqueue(owner: u64, key_a: u64, key_b: u64, request: Request, bytes: &[u8]) {
    if owner == 0
        || !INITIALIZED.load(Ordering::Acquire)
        || SERVICE.state.load(Ordering::Acquire) != EMPTY
        || bytes.len() > MAX_FILE_BYTES
    {
        panic!("invalid AppData request publication");
    }
    // SAFETY: EMPTY plus the IRQ-masked syscall domain grants the sole write.
    let workspace = unsafe { &mut *SERVICE.value.get() };
    workspace.request = request;
    workspace.data.fill(0);
    workspace.data[..bytes.len()].copy_from_slice(bytes);
    workspace.outcome = Outcome::None;
    SERVICE.owner.store(owner, Ordering::Relaxed);
    SERVICE.key_a.store(key_a, Ordering::Relaxed);
    SERVICE.key_b.store(key_b, Ordering::Relaxed);
    SUBMISSIONS.fetch_add(1, Ordering::Relaxed);
    SERVICE.state.store(PENDING, Ordering::Release);
}

fn completed_workspace() -> &'static mut Workspace {
    if !aarch64::irq_is_masked() || SERVICE.state.load(Ordering::Acquire) != COMPLETE {
        panic!("AppData completion accessed outside its exclusive state");
    }
    // SAFETY: COMPLETE is immutable to the service and all syscall access is
    // serialized by the local IRQ mask.
    unsafe { &mut *SERVICE.value.get() }
}

fn consume_completed() {
    if !aarch64::irq_is_masked() || SERVICE.state.load(Ordering::Acquire) != COMPLETE {
        panic!("AppData completion consumed outside its exclusive state");
    }
    SERVICE.owner.store(0, Ordering::Relaxed);
    SERVICE.key_a.store(0, Ordering::Relaxed);
    SERVICE.key_b.store(0, Ordering::Relaxed);
    SERVICE.state.store(EMPTY, Ordering::Release);
}

fn published_owner_can_retry() -> bool {
    if !aarch64::irq_is_masked() {
        panic!("AppData completion liveness checked with IRQ enabled");
    }
    let state = SERVICE.state.load(Ordering::Acquire);
    if state != PENDING && state != RUNNING && state != RECOVERING && state != COMPLETE {
        return false;
    }
    // SAFETY: PENDING is immutable after release publication, RUNNING and
    // RECOVERING belong to the monitor, and COMPLETE is immutable until
    // consumed.
    let request = unsafe { &(*SERVICE.value.get()).request };
    crate::process::app_data_owner_can_retry_irq_masked(
        SERVICE.owner.load(Ordering::Relaxed),
        request.principal,
        request.required_rights(),
    )
}

fn discard_abandoned() {
    if !aarch64::irq_is_masked() {
        panic!("AppData completion discarded with IRQ enabled");
    }
    let state = SERVICE.state.load(Ordering::Acquire);
    if state != PENDING && state != RUNNING && state != RECOVERING && state != COMPLETE {
        panic!("AppData abandonment observed no published request");
    }
    SERVICE.owner.store(0, Ordering::Relaxed);
    SERVICE.key_a.store(0, Ordering::Relaxed);
    SERVICE.key_b.store(0, Ordering::Relaxed);
    ABANDONED.fetch_add(1, Ordering::Relaxed);
    SERVICE.state.store(EMPTY, Ordering::Release);
}

fn request_keys(owner: u64, request: &Request, bytes: &[u8]) -> (u64, u64) {
    let mut a = 0xcbf2_9ce4_8422_2325_u64;
    let mut b = 0x9e37_79b9_7f4a_7c15_u64;
    for byte in owner
        .to_le_bytes()
        .into_iter()
        .chain([request.kind as u8])
        .chain(request.principal.to_le_bytes())
        .chain([request.path_len])
        .chain(request.path)
        .chain(request.data_len.to_le_bytes())
        .chain(request.cursor.to_le_bytes())
        .chain(condition_bytes(request.condition))
        .chain(bytes.iter().copied())
    {
        a = (a ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        b ^= u64::from(byte).wrapping_add(0x517c_c1b7_2722_0a95);
        b = b.rotate_left(13).wrapping_mul(0x9e37_79b1_85eb_ca87);
    }
    (a, b)
}

fn condition_bytes(condition: ReplaceCondition) -> [u8; 9] {
    let (tag, generation) = match condition {
        ReplaceCondition::Any => (0, 0),
        ReplaceCondition::Absent => (1, 0),
        ReplaceCondition::Generation(generation) => (2, generation),
    };
    let mut bytes = [0; 9];
    bytes[0] = tag;
    bytes[1..].copy_from_slice(&generation.to_le_bytes());
    bytes
}

fn record_outcome(outcome: Outcome) {
    match outcome {
        Outcome::Mutation(Ok(mutation)) => {
            MUTATION_COMMITS.fetch_add(1, Ordering::Relaxed);
            LAST_GENERATION.store(mutation.committed_generation, Ordering::Relaxed);
        }
        Outcome::Read(Ok(read)) => {
            READ_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            LAST_GENERATION.store(read.snapshot_generation, Ordering::Relaxed);
        }
        Outcome::Directory(Ok(_)) => {
            DIRECTORY_SUCCESSES.fetch_add(1, Ordering::Relaxed);
        }
        Outcome::Mutation(Err(Error::Conflict { .. })) => {
            CONFLICTS.fetch_add(1, Ordering::Relaxed);
        }
        Outcome::Mutation(Err(Error::OutcomeUnknown))
        | Outcome::Read(Err(Error::OutcomeUnknown))
        | Outcome::Directory(Err(Error::OutcomeUnknown)) => {
            OUTCOME_UNKNOWN.fetch_add(1, Ordering::Relaxed);
            ERRORS.fetch_add(1, Ordering::Relaxed);
        }
        Outcome::Mutation(Err(_)) | Outcome::Read(Err(_)) | Outcome::Directory(Err(_)) => {
            ERRORS.fetch_add(1, Ordering::Relaxed);
        }
        Outcome::None => panic!("AppData service published an empty completion"),
    }
}

fn publish_boot_recovery(recovery: RecoveryInfo) {
    BOOT_GENERATION.store(recovery.generation, Ordering::Relaxed);
    BOOT_VALID_SNAPSHOTS.store(u64::from(recovery.valid_snapshots), Ordering::Relaxed);
    BOOT_REJECTED_SNAPSHOTS.store(u64::from(recovery.rejected_snapshots), Ordering::Relaxed);
    BOOT_ENTRY_COUNT.store(u64::from(recovery.entry_count), Ordering::Relaxed);
    BOOT_LIVE_BYTES.store(u64::from(recovery.live_payload_bytes), Ordering::Relaxed);
}

const fn map_read_error(error: StorageError) -> IoError {
    match error {
        StorageError::Block(BlockError::SectorOutOfBounds)
        | StorageError::WriteOutsideAppDataPartition => IoError::OutOfBounds,
        StorageError::IrqNotArmed
        | StorageError::RecoveryRequired
        | StorageError::InterruptFailure
        | StorageError::Block(BlockError::DeviceNeedsReset)
        | StorageError::Block(BlockError::DeadlineExpired) => IoError::RequiresReset,
        _ => IoError::Device,
    }
}

const fn map_mutation_error(error: StorageError) -> IoError {
    match error {
        StorageError::SubmittedMutationOutcomeUnknown
        | StorageError::Block(BlockError::DeadlineExpired) => IoError::OutcomeUnknown,
        other => map_read_error(other),
    }
}

// Keep the submission boundary mechanically checked in every M54 build. Read
// failures require recovery but never claim a mutation, while only a storage
// error that proves descriptor publication maps to OutcomeUnknown.
const _: () = {
    assert!(matches!(
        map_mutation_error(StorageError::SubmittedMutationOutcomeUnknown),
        IoError::OutcomeUnknown
    ));
    assert!(matches!(
        map_mutation_error(StorageError::RecoveryRequired),
        IoError::RequiresReset
    ));
    assert!(matches!(
        map_mutation_error(StorageError::IrqNotArmed),
        IoError::RequiresReset
    ));
    assert!(matches!(
        map_read_error(StorageError::InterruptFailure),
        IoError::RequiresReset
    ));
    assert!(matches!(
        map_read_error(StorageError::Block(BlockError::DeadlineExpired)),
        IoError::RequiresReset
    ));
    assert!(matches!(
        map_read_error(StorageError::SubmittedMutationOutcomeUnknown),
        IoError::Device
    ));
};

pub const fn entry_kind_raw(kind: EntryKind) -> u16 {
    match kind {
        EntryKind::File => 1,
        EntryKind::Directory => 2,
    }
}
