//! Capability-scoped, AppData-bounded block and session broker for M55.
//!
//! This module deliberately knows nothing about paths, files, snapshots, or
//! checkpoint policy.  It authenticates the one StorageServer generation,
//! bounds block requests to the AppData partition view, and creates
//! principal-bound client Channels whose identity cannot be selected on the
//! wire.  The kernel binary's monitor takes one pending sector operation,
//! executes it with IRQs enabled, and publishes the terminal completion.

use core::cell::UnsafeCell;
use core::fmt;
use core::hint::spin_loop;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

use bndr_abi::{
    AppDataPrincipal, ObjectSignals, Rights, STORAGE_BLOCK_DATA_MAX_BYTES,
    STORAGE_BLOCK_MAX_SECTORS, StorageBlockOperation, StorageBlockRequest, StorageSessionBinding,
    UserImageId,
};

use crate::channel::{ChannelAllocError, ChannelEndpoint};

pub const SESSION_QUEUE_CAPACITY: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcquireError {
    InvalidOwner,
    DeviceOffline,
    AlreadyBound,
    EpochExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestError {
    CapabilityMismatch,
    RecoveryRequired,
    ServiceAbandoned,
    Busy,
    TokenExhausted,
    NoPendingRequest,
    TokenMismatch,
    InvalidTransition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionError {
    Unavailable,
    RecoveryRequired,
    PermissionDenied,
    InvalidRights,
    QueueFull,
    OutOfMemory,
    CapabilityMismatch,
    SessionIdExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionCode {
    Ok,
    Unavailable,
    OutcomeUnknown,
    RequiresReset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompletionInfo {
    operation: StorageBlockOperation,
    code: CompletionCode,
    sector_count: usize,
    data_bytes: usize,
}

impl CompletionInfo {
    pub const fn operation(self) -> StorageBlockOperation {
        self.operation
    }

    pub const fn code(self) -> CompletionCode {
        self.code
    }

    pub const fn sector_count(self) -> usize {
        self.sector_count
    }

    /// Bytes available through [`StorageBroker::copy_completion_data`].
    ///
    /// Only a successful read carries output bytes. Writes, flushes, and all
    /// failed completions expose zero so callers cannot copy stale or partial
    /// block data across the syscall boundary.
    pub const fn data_bytes(self) -> usize {
        self.data_bytes
    }
}

#[derive(Debug, Eq, PartialEq)]
struct StoredCompletion {
    token: u64,
    info: CompletionInfo,
    read_data: [u8; STORAGE_BLOCK_DATA_MAX_BYTES],
}

/// Unique, non-duplicable, non-transferable ownership of the AppData view.
///
/// Cloning is an internal kernel operation used by handle-table inspection;
/// userspace receives rights without `DUPLICATE` or `TRANSFER`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageVolumeCapability {
    owner_pid: u64,
    epoch: u64,
}

impl StorageVolumeCapability {
    const fn new(owner_pid: u64, epoch: u64) -> Self {
        Self { owner_pid, epoch }
    }

    pub const fn owner_pid(self) -> u64 {
        self.owner_pid
    }

    pub const fn epoch(self) -> u64 {
        self.epoch
    }

    pub fn signal_mask(self) -> ObjectSignals {
        ObjectSignals::from_bits(
            ObjectSignals::READABLE.bits()
                | ObjectSignals::WRITABLE.bits()
                | ObjectSignals::PEER_CLOSED.bits(),
        )
        .unwrap_or_else(|| panic!("storage capability mask escaped the ABI"))
    }

    pub fn signals(self) -> ObjectSignals {
        global().signals(self)
    }

    pub const fn same_volume(self, other: Self) -> bool {
        self.owner_pid == other.owner_pid && self.epoch == other.epoch
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Binding {
    capability: StorageVolumeCapability,
}

#[derive(Debug, Eq, PartialEq)]
enum BlockState {
    Idle,
    Pending(StorageBlockRequest),
    Running(StorageBlockRequest),
    Complete(StoredCompletion),
    RecoveryRequired,
}

struct PendingSession {
    endpoint: ChannelEndpoint,
    binding: StorageSessionBinding,
}

impl fmt::Debug for PendingSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PendingSession")
            .field("endpoint", &self.endpoint)
            .field("binding", &self.binding)
            .finish()
    }
}

#[derive(Debug)]
struct SessionQueue {
    slots: [Option<PendingSession>; SESSION_QUEUE_CAPACITY],
    head: usize,
    len: usize,
}

impl SessionQueue {
    const fn new() -> Self {
        Self {
            slots: [const { None }; SESSION_QUEUE_CAPACITY],
            head: 0,
            len: 0,
        }
    }

    const fn is_full(&self) -> bool {
        self.len == SESSION_QUEUE_CAPACITY
    }

    fn push(&mut self, session: PendingSession) {
        assert!(
            !self.is_full(),
            "preflighted storage session queue was full"
        );
        let tail = (self.head + self.len) % SESSION_QUEUE_CAPACITY;
        assert!(self.slots[tail].is_none());
        self.slots[tail] = Some(session);
        self.len += 1;
    }

    fn pop(&mut self) -> Option<PendingSession> {
        if self.len == 0 {
            return None;
        }
        let session = self.slots[self.head]
            .take()
            .unwrap_or_else(|| panic!("storage session queue lost its head"));
        self.head = (self.head + 1) % SESSION_QUEUE_CAPACITY;
        self.len -= 1;
        Some(session)
    }

    fn clear(&mut self) -> usize {
        let previous = self.len;
        while self.pop().is_some() {}
        previous
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrokerSnapshot {
    pub bound: bool,
    pub owner_pid: u64,
    pub epoch: u64,
    pub next_epoch: u64,
    pub next_token: u64,
    pub next_session_id: u64,
    pub pending_sessions: usize,
    pub state: u8,
    pub submissions: u64,
    pub completions: u64,
    pub retrievals: u64,
    pub submitted_sectors: u64,
    pub completed_sectors: u64,
    pub read_sectors: u64,
    pub write_sectors: u64,
    pub releases: u64,
    pub abandoned: u64,
    pub abandoned_running: bool,
    pub bounds_rejections: u64,
    pub device_offline: bool,
    pub offline_transitions: u64,
    pub offline_acquire_denials: u64,
}

/// Allocation-free policy state.  The surrounding spin lock is a portability
/// guard for host tests; the kernel calls it only from IRQ-serialized paths.
#[derive(Debug)]
pub struct StorageBroker {
    binding: Option<Binding>,
    state: BlockState,
    next_epoch: u64,
    next_token: u64,
    next_session_id: u64,
    sessions: SessionQueue,
    submissions: u64,
    completions: u64,
    retrievals: u64,
    submitted_sectors: u64,
    completed_sectors: u64,
    read_sectors: u64,
    write_sectors: u64,
    releases: u64,
    abandoned: u64,
    abandoned_running_token: u64,
    bounds_rejections: u64,
    device_offline: bool,
    offline_transitions: u64,
    offline_acquire_denials: u64,
}

impl Default for StorageBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBroker {
    pub const fn new() -> Self {
        Self {
            binding: None,
            state: BlockState::Idle,
            next_epoch: 1,
            next_token: 1,
            next_session_id: 1,
            sessions: SessionQueue::new(),
            submissions: 0,
            completions: 0,
            retrievals: 0,
            submitted_sectors: 0,
            completed_sectors: 0,
            read_sectors: 0,
            write_sectors: 0,
            releases: 0,
            abandoned: 0,
            abandoned_running_token: 0,
            bounds_rejections: 0,
            device_offline: false,
            offline_transitions: 0,
            offline_acquire_denials: 0,
        }
    }

    pub fn acquire(
        &mut self,
        owner_pid: u64,
        image_id: UserImageId,
    ) -> Result<StorageVolumeCapability, AcquireError> {
        if owner_pid == 0 || image_id != UserImageId::StorageServer {
            return Err(AcquireError::InvalidOwner);
        }
        // Authenticate the caller before exposing the boot-local device
        // health state. Only a legitimate StorageServer generation may
        // distinguish an offline device from an ordinary authority denial.
        if self.device_offline {
            self.offline_acquire_denials = self.offline_acquire_denials.saturating_add(1);
            return Err(AcquireError::DeviceOffline);
        }
        if self.binding.is_some() || !matches!(&self.state, BlockState::Idle) {
            return Err(AcquireError::AlreadyBound);
        }
        let epoch = self.next_epoch;
        self.next_epoch = epoch.checked_add(1).ok_or(AcquireError::EpochExhausted)?;
        let capability = StorageVolumeCapability::new(owner_pid, epoch);
        self.binding = Some(Binding { capability });
        Ok(capability)
    }

    fn validates(&self, capability: StorageVolumeCapability, owner_pid: u64) -> bool {
        owner_pid != 0
            && capability.owner_pid == owner_pid
            && self.binding.is_some_and(|binding| {
                binding.capability.same_volume(capability)
                    && binding.capability.owner_pid == owner_pid
            })
    }

    pub fn submit(
        &mut self,
        capability: StorageVolumeCapability,
        owner_pid: u64,
        request: StorageBlockRequest,
    ) -> Result<u64, RequestError> {
        if !self.validates(capability, owner_pid) {
            return Err(RequestError::CapabilityMismatch);
        }
        if matches!(&self.state, BlockState::RecoveryRequired) {
            return Err(RequestError::RecoveryRequired);
        }
        if !matches!(&self.state, BlockState::Idle) {
            return Err(RequestError::Busy);
        }
        // ABI decoding already enforces this range. Keep an independent
        // overflow-safe gate at the authority boundary so future direct kernel
        // callers cannot bypass the full `lba + count <= 1920` invariant.
        let sector_count = request.sector_count();
        if !matches!(request.operation(), StorageBlockOperation::Flush)
            && !storage_batch_in_bounds(request.relative_lba(), sector_count)
        {
            self.bounds_rejections = self.bounds_rejections.saturating_add(1);
            return Err(RequestError::InvalidTransition);
        }
        let accounted_sectors = if matches!(request.operation(), StorageBlockOperation::Flush) {
            0
        } else {
            sector_count as u64
        };
        let token = self.next_token;
        let next = token.checked_add(1).ok_or(RequestError::TokenExhausted)?;
        let submissions = self
            .submissions
            .checked_add(1)
            .ok_or(RequestError::TokenExhausted)?;
        let submitted_sectors = self
            .submitted_sectors
            .checked_add(accounted_sectors)
            .ok_or(RequestError::TokenExhausted)?;
        let request = request
            .with_kernel_token(token)
            .map_err(|_| RequestError::InvalidTransition)?;
        self.next_token = next;
        self.state = BlockState::Pending(request);
        self.submissions = submissions;
        self.submitted_sectors = submitted_sectors;
        Ok(token)
    }

    pub fn begin_service(&mut self) -> Option<StorageBlockRequest> {
        let BlockState::Pending(request) = &self.state else {
            return None;
        };
        let request = *request;
        self.state = BlockState::Running(request);
        Some(request)
    }

    pub fn finish_service(
        &mut self,
        request: StorageBlockRequest,
        code: CompletionCode,
        read_data: [u8; STORAGE_BLOCK_DATA_MAX_BYTES],
    ) -> Result<(), RequestError> {
        if matches!(&self.state, BlockState::RecoveryRequired)
            && self.binding.is_none()
            && self.abandoned_running_token != 0
        {
            if request.token() != self.abandoned_running_token {
                return Err(RequestError::TokenMismatch);
            }
            // The owner disappeared after the monitor accepted this request.
            // The physical result belongs to no live capability and must not
            // be published, but consuming the unique token lets recovery
            // proceed without turning a hostile close into a kernel panic.
            self.abandoned_running_token = 0;
            return Err(RequestError::ServiceAbandoned);
        }
        let BlockState::Running(running) = &self.state else {
            return Err(RequestError::InvalidTransition);
        };
        if self.abandoned_running_token != 0 {
            return Err(RequestError::InvalidTransition);
        }
        if running != &request {
            return Err(RequestError::TokenMismatch);
        }
        if code == CompletionCode::OutcomeUnknown
            && request.operation() == StorageBlockOperation::Read
        {
            return Err(RequestError::InvalidTransition);
        }
        let successful_read =
            request.operation() == StorageBlockOperation::Read && code == CompletionCode::Ok;
        let data_bytes = if successful_read {
            request.byte_len()
        } else {
            0
        };
        if read_data[data_bytes..].iter().any(|byte| *byte != 0) {
            return Err(RequestError::InvalidTransition);
        }
        let accounted_sectors = if matches!(request.operation(), StorageBlockOperation::Flush) {
            0
        } else {
            request.sector_count() as u64
        };
        let completions = self
            .completions
            .checked_add(1)
            .ok_or(RequestError::TokenExhausted)?;
        let completed_sectors = self
            .completed_sectors
            .checked_add(accounted_sectors)
            .ok_or(RequestError::TokenExhausted)?;
        let read_sectors = self
            .read_sectors
            .checked_add(if request.operation() == StorageBlockOperation::Read {
                accounted_sectors
            } else {
                0
            })
            .ok_or(RequestError::TokenExhausted)?;
        let write_sectors = self
            .write_sectors
            .checked_add(if request.operation() == StorageBlockOperation::Write {
                accounted_sectors
            } else {
                0
            })
            .ok_or(RequestError::TokenExhausted)?;
        self.state = BlockState::Complete(StoredCompletion {
            token: request.token(),
            info: CompletionInfo {
                operation: request.operation(),
                code,
                sector_count: request.sector_count(),
                data_bytes,
            },
            read_data,
        });
        self.completions = completions;
        self.completed_sectors = completed_sectors;
        self.read_sectors = read_sectors;
        self.write_sectors = write_sectors;
        Ok(())
    }

    pub fn completion_info(
        &self,
        capability: StorageVolumeCapability,
        owner_pid: u64,
        token: u64,
    ) -> Result<CompletionInfo, RequestError> {
        Ok(self.completion(capability, owner_pid, token)?.info)
    }

    pub fn copy_completion_data(
        &self,
        capability: StorageVolumeCapability,
        owner_pid: u64,
        token: u64,
        output: &mut [u8; STORAGE_BLOCK_DATA_MAX_BYTES],
    ) -> Result<usize, RequestError> {
        let completion = self.completion(capability, owner_pid, token)?;
        let bytes = completion.info.data_bytes();
        output[..bytes].copy_from_slice(&completion.read_data[..bytes]);
        output[bytes..].fill(0);
        Ok(bytes)
    }

    pub fn acknowledge_completion(
        &mut self,
        capability: StorageVolumeCapability,
        owner_pid: u64,
        token: u64,
    ) -> Result<(), RequestError> {
        let code = self.completion(capability, owner_pid, token)?.info.code();
        let retrievals = self
            .retrievals
            .checked_add(1)
            .ok_or(RequestError::TokenExhausted)?;
        self.state = if completion_requires_recovery(code) {
            BlockState::RecoveryRequired
        } else {
            BlockState::Idle
        };
        self.retrievals = retrievals;
        Ok(())
    }

    fn completion(
        &self,
        capability: StorageVolumeCapability,
        owner_pid: u64,
        token: u64,
    ) -> Result<&StoredCompletion, RequestError> {
        if !self.validates(capability, owner_pid) {
            return Err(RequestError::CapabilityMismatch);
        }
        match &self.state {
            BlockState::Pending(_) | BlockState::Running(_) => Err(RequestError::Busy),
            BlockState::Complete(completion) if completion.token == token => Ok(completion),
            BlockState::Complete(_) => Err(RequestError::TokenMismatch),
            BlockState::Idle | BlockState::RecoveryRequired => Err(RequestError::NoPendingRequest),
        }
    }

    pub fn complete_recovery(&mut self) -> Result<(), RequestError> {
        // Recovery is a fail-stop generation boundary.  The old owner must
        // first release its binding (which also closes queued sessions) so a
        // reset can never make an old capability writable again.
        if self.device_offline
            || self.binding.is_some()
            || self.abandoned_running_token != 0
            || !matches!(&self.state, BlockState::RecoveryRequired)
        {
            return Err(RequestError::InvalidTransition);
        }
        self.state = BlockState::Idle;
        Ok(())
    }

    /// Permanently fail-stops this boot's physical storage authority.
    ///
    /// The transition is legal only after the former StorageServer generation
    /// and every queued session have gone away, no accepted request remains to
    /// be reconciled, and the broker is already closed for recovery. Repeated
    /// calls are idempotent and do not inflate the transition counter.
    pub fn enter_device_offline(&mut self) -> Result<(), RequestError> {
        if self.device_offline {
            return Ok(());
        }
        if self.binding.is_some()
            || self.sessions.len != 0
            || self.abandoned_running_token != 0
            || !matches!(&self.state, BlockState::RecoveryRequired)
        {
            return Err(RequestError::InvalidTransition);
        }
        self.device_offline = true;
        self.offline_transitions = self.offline_transitions.saturating_add(1);
        Ok(())
    }

    /// Reconciles a physical recovery latch that becomes visible while the
    /// broker is ownerless and logically idle. This closes the narrow
    /// IRQ-rearm/broker-reopen race without granting any user process reset
    /// authority or consuming a new server epoch.
    pub fn require_ownerless_recovery(&mut self) -> Result<(), RequestError> {
        if self.binding.is_some() || self.sessions.len != 0 {
            return Err(RequestError::InvalidTransition);
        }
        match &self.state {
            BlockState::Idle | BlockState::RecoveryRequired => {
                self.state = BlockState::RecoveryRequired;
                Ok(())
            }
            BlockState::Pending(_) | BlockState::Running(_) | BlockState::Complete(_) => {
                Err(RequestError::InvalidTransition)
            }
        }
    }

    pub fn connect(
        &mut self,
        client_pid: u64,
        image_id: UserImageId,
        granted_rights: Rights,
    ) -> Result<ChannelEndpoint, SessionError> {
        // Authenticate every caller-controlled authority input before
        // exposing whether the StorageServer is bound, recovering, or
        // permanently offline.  Otherwise an unrelated image could use
        // `connect` as a boot-local storage-health oracle.
        let principal = principal_for_image(image_id).ok_or(SessionError::PermissionDenied)?;
        if client_pid == 0 {
            return Err(SessionError::PermissionDenied);
        }
        let bits = granted_rights.bits();
        if bits == 0 || bits & !bndr_abi::STORAGE_CONNECT_RIGHTS_MASK != 0 {
            return Err(SessionError::InvalidRights);
        }
        if matches!(&self.state, BlockState::RecoveryRequired) {
            return Err(SessionError::RecoveryRequired);
        }
        let Some(binding) = self.binding else {
            return Err(SessionError::Unavailable);
        };
        if self.sessions.is_full() {
            return Err(SessionError::QueueFull);
        }
        let session_id = self.next_session_id;
        let next_session_id = session_id
            .checked_add(1)
            .ok_or(SessionError::SessionIdExhausted)?;
        let session_binding = StorageSessionBinding::new(
            session_id,
            client_pid,
            principal,
            granted_rights,
            binding.capability.epoch,
        )
        .map_err(|_| SessionError::InvalidRights)?;
        let (client, server) = ChannelEndpoint::try_pair().map_err(map_channel_alloc)?;
        self.sessions.push(PendingSession {
            endpoint: server,
            binding: session_binding,
        });
        self.next_session_id = next_session_id;
        Ok(client)
    }

    pub fn accept(
        &mut self,
        capability: StorageVolumeCapability,
        owner_pid: u64,
    ) -> Result<(ChannelEndpoint, StorageSessionBinding), SessionError> {
        if !self.validates(capability, owner_pid) {
            return Err(SessionError::CapabilityMismatch);
        }
        if matches!(&self.state, BlockState::RecoveryRequired) {
            return Err(SessionError::RecoveryRequired);
        }
        let session = self.sessions.pop().ok_or(SessionError::Unavailable)?;
        Ok((session.endpoint, session.binding))
    }

    pub fn peek_session_binding(
        &self,
        capability: StorageVolumeCapability,
        owner_pid: u64,
    ) -> Result<StorageSessionBinding, SessionError> {
        if !self.validates(capability, owner_pid) {
            return Err(SessionError::CapabilityMismatch);
        }
        if matches!(&self.state, BlockState::RecoveryRequired) {
            return Err(SessionError::RecoveryRequired);
        }
        self.sessions.slots[self.sessions.head]
            .as_ref()
            .map(|session| session.binding)
            .ok_or(SessionError::Unavailable)
    }

    pub fn release_process(&mut self, process_id: u64) -> bool {
        let Some(binding) = self.binding else {
            return false;
        };
        if binding.capability.owner_pid != process_id {
            return false;
        }
        self.abandoned = self.abandoned.saturating_add(match &self.state {
            BlockState::Idle => 0,
            BlockState::Pending(_) | BlockState::Complete(_) => 1,
            BlockState::Running(_) | BlockState::RecoveryRequired => 1,
        });
        self.abandoned_running_token = match &self.state {
            BlockState::Running(request) => request.token(),
            BlockState::Idle
            | BlockState::Pending(_)
            | BlockState::Complete(_)
            | BlockState::RecoveryRequired => 0,
        };
        let recovery_required = match &self.state {
            // A running request may already own a published descriptor. Even
            // an abandoned read must be reset so its slot and token cannot
            // leak into a replacement StorageServer generation.
            BlockState::Running(_) => true,
            BlockState::Complete(completion)
                if completion_requires_recovery(completion.info.code()) =>
            {
                true
            }
            BlockState::RecoveryRequired => true,
            _ => false,
        };
        self.state = if recovery_required {
            BlockState::RecoveryRequired
        } else {
            BlockState::Idle
        };
        self.sessions.clear();
        self.binding = None;
        self.releases = self.releases.saturating_add(1);
        true
    }

    pub fn signals(&self, capability: StorageVolumeCapability) -> ObjectSignals {
        let Some(binding) = self.binding else {
            return ObjectSignals::PEER_CLOSED;
        };
        if !binding.capability.same_volume(capability) {
            return ObjectSignals::PEER_CLOSED;
        }
        let block = match &self.state {
            BlockState::Idle => ObjectSignals::WRITABLE,
            BlockState::Complete(_) => ObjectSignals::READABLE,
            BlockState::RecoveryRequired => ObjectSignals::PEER_CLOSED,
            BlockState::Pending(_) | BlockState::Running(_) => ObjectSignals::NONE,
        };
        if !matches!(&self.state, BlockState::RecoveryRequired) && self.sessions.len != 0 {
            ObjectSignals::from_bits(block.bits() | ObjectSignals::READABLE.bits())
                .unwrap_or_else(|| panic!("storage capability signals escaped the ABI"))
        } else {
            block
        }
    }

    pub fn snapshot(&self) -> BrokerSnapshot {
        let (owner_pid, epoch) = self
            .binding
            .map(|binding| (binding.capability.owner_pid, binding.capability.epoch))
            .unwrap_or((0, 0));
        BrokerSnapshot {
            bound: self.binding.is_some(),
            owner_pid,
            epoch,
            next_epoch: self.next_epoch,
            next_token: self.next_token,
            next_session_id: self.next_session_id,
            pending_sessions: self.sessions.len,
            state: match &self.state {
                BlockState::Idle => 0,
                BlockState::Pending(_) => 1,
                BlockState::Running(_) => 2,
                BlockState::Complete(_) => 3,
                BlockState::RecoveryRequired => 4,
            },
            submissions: self.submissions,
            completions: self.completions,
            retrievals: self.retrievals,
            submitted_sectors: self.submitted_sectors,
            completed_sectors: self.completed_sectors,
            read_sectors: self.read_sectors,
            write_sectors: self.write_sectors,
            releases: self.releases,
            abandoned: self.abandoned,
            abandoned_running: self.abandoned_running_token != 0,
            bounds_rejections: self.bounds_rejections,
            device_offline: self.device_offline,
            offline_transitions: self.offline_transitions,
            offline_acquire_denials: self.offline_acquire_denials,
        }
    }
}

fn storage_batch_in_bounds(relative_lba: u64, sector_count: usize) -> bool {
    sector_count != 0
        && sector_count <= STORAGE_BLOCK_MAX_SECTORS
        && relative_lba
            .checked_add(sector_count as u64)
            .is_some_and(|end| end <= bndr_abi::APP_DATA_VOLUME_SECTORS)
}

const fn completion_requires_recovery(code: CompletionCode) -> bool {
    matches!(
        code,
        CompletionCode::OutcomeUnknown | CompletionCode::RequiresReset
    )
}

fn map_channel_alloc(_: ChannelAllocError) -> SessionError {
    SessionError::OutOfMemory
}

pub const fn principal_for_image(image_id: UserImageId) -> Option<AppDataPrincipal> {
    match image_id {
        UserImageId::App => Some(AppDataPrincipal::PRIMARY_APP),
        UserImageId::Launcher => Some(AppDataPrincipal::LAUNCHER),
        _ => None,
    }
}

struct SpinMutex<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for SpinMutex<T> {}
unsafe impl<T: Send> Sync for SpinMutex<T> {}

impl<T> SpinMutex<T> {
    const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    fn lock(&self) -> SpinMutexGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_loop();
        }
        SpinMutexGuard { mutex: self }
    }
}

struct SpinMutexGuard<'a, T> {
    mutex: &'a SpinMutex<T>,
}

impl<T> Deref for SpinMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T> DerefMut for SpinMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Drop for SpinMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}

static GLOBAL: SpinMutex<StorageBroker> = SpinMutex::new(StorageBroker::new());

fn global() -> SpinMutexGuard<'static, StorageBroker> {
    GLOBAL.lock()
}

pub fn acquire(
    owner_pid: u64,
    image_id: UserImageId,
) -> Result<StorageVolumeCapability, AcquireError> {
    global().acquire(owner_pid, image_id)
}

pub fn submit(
    capability: StorageVolumeCapability,
    owner_pid: u64,
    request: StorageBlockRequest,
) -> Result<u64, RequestError> {
    global().submit(capability, owner_pid, request)
}

pub fn begin_service() -> Option<StorageBlockRequest> {
    global().begin_service()
}

pub fn finish_service(
    request: StorageBlockRequest,
    code: CompletionCode,
    read_data: [u8; STORAGE_BLOCK_DATA_MAX_BYTES],
) -> Result<(), RequestError> {
    global().finish_service(request, code, read_data)
}

pub fn completion_info(
    capability: StorageVolumeCapability,
    owner_pid: u64,
    token: u64,
) -> Result<CompletionInfo, RequestError> {
    global().completion_info(capability, owner_pid, token)
}

pub fn copy_completion_data(
    capability: StorageVolumeCapability,
    owner_pid: u64,
    token: u64,
    output: &mut [u8; STORAGE_BLOCK_DATA_MAX_BYTES],
) -> Result<usize, RequestError> {
    global().copy_completion_data(capability, owner_pid, token, output)
}

pub fn acknowledge_completion(
    capability: StorageVolumeCapability,
    owner_pid: u64,
    token: u64,
) -> Result<(), RequestError> {
    global().acknowledge_completion(capability, owner_pid, token)
}

pub fn complete_recovery() -> Result<(), RequestError> {
    global().complete_recovery()
}

pub fn enter_device_offline() -> Result<(), RequestError> {
    global().enter_device_offline()
}

pub fn require_ownerless_recovery() -> Result<(), RequestError> {
    global().require_ownerless_recovery()
}

pub fn connect(
    client_pid: u64,
    image_id: UserImageId,
    granted_rights: Rights,
) -> Result<ChannelEndpoint, SessionError> {
    global().connect(client_pid, image_id, granted_rights)
}

pub fn accept(
    capability: StorageVolumeCapability,
    owner_pid: u64,
) -> Result<(ChannelEndpoint, StorageSessionBinding), SessionError> {
    global().accept(capability, owner_pid)
}

pub fn peek_session_binding(
    capability: StorageVolumeCapability,
    owner_pid: u64,
) -> Result<StorageSessionBinding, SessionError> {
    global().peek_session_binding(capability, owner_pid)
}

pub fn release_process(process_id: u64) -> bool {
    global().release_process(process_id)
}

pub fn snapshot() -> BrokerSnapshot {
    global().snapshot()
}

#[cfg(test)]
mod tests {
    use super::{
        AcquireError, CompletionCode, CompletionInfo, RequestError, SessionError, StorageBroker,
        principal_for_image,
    };
    use bndr_abi::{
        APP_DATA_VOLUME_SECTORS, ObjectSignals, Rights, STORAGE_BLOCK_DATA_MAX_BYTES,
        STORAGE_BLOCK_MAX_SECTORS, STORAGE_SECTOR_SIZE, StorageBlockOperation, StorageBlockRequest,
        StorageBlockRequestError, UserImageId,
    };

    const SERVER_PID: u64 = (4_u64 << 32) | 1;
    const APP_PID: u64 = (5_u64 << 32) | 1;
    const LAUNCHER_PID: u64 = (6_u64 << 32) | 1;

    fn assert_connect_authority_oracle(broker: &mut StorageBroker, state: &str) {
        let before = broker.snapshot();
        for (client_pid, image_id, rights, expected) in [
            (
                APP_PID,
                UserImageId::Init,
                Rights::READ,
                SessionError::PermissionDenied,
            ),
            (
                0,
                UserImageId::App,
                Rights::READ,
                SessionError::PermissionDenied,
            ),
            (
                APP_PID,
                UserImageId::App,
                Rights::NONE,
                SessionError::InvalidRights,
            ),
            (
                APP_PID,
                UserImageId::App,
                Rights::TRANSFER,
                SessionError::InvalidRights,
            ),
            // Principal and process authentication deliberately precede the
            // rights check as well as every broker-health observation.
            (
                0,
                UserImageId::Init,
                Rights::TRANSFER,
                SessionError::PermissionDenied,
            ),
            (
                0,
                UserImageId::App,
                Rights::TRANSFER,
                SessionError::PermissionDenied,
            ),
        ] {
            assert_eq!(
                broker.connect(client_pid, image_id, rights).err(),
                Some(expected),
                "{state} changed the connect authority oracle"
            );
        }
        assert_eq!(
            broker.snapshot(),
            before,
            "{state} authority rejection mutated broker state"
        );
    }

    #[test]
    fn authority_and_principals_are_image_derived() {
        let mut broker = StorageBroker::new();
        assert!(broker.acquire(SERVER_PID, UserImageId::App).is_err());
        let cap = broker
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        assert_eq!(cap.owner_pid(), SERVER_PID);
        assert_eq!(cap.epoch(), 1);
        assert_eq!(broker.signals(cap), ObjectSignals::WRITABLE);
        assert_ne!(
            principal_for_image(UserImageId::App),
            principal_for_image(UserImageId::Launcher)
        );
        for image in [
            UserImageId::Init,
            UserImageId::ServiceManager,
            UserImageId::Provider,
            UserImageId::Client,
            UserImageId::SurfaceServer,
            UserImageId::InputServer,
            UserImageId::StorageServer,
        ] {
            assert_eq!(principal_for_image(image), None);
        }
    }

    #[test]
    fn connect_authentication_precedes_healthy_recovery_and_offline_state() {
        // Healthy but not yet bound: authenticated clients observe ordinary
        // service unavailability, while invalid authority never reaches that
        // binding-state oracle.
        let mut unbound_healthy = StorageBroker::new();
        assert_connect_authority_oracle(&mut unbound_healthy, "unbound healthy");
        assert_eq!(
            unbound_healthy
                .connect(APP_PID, UserImageId::App, Rights::READ)
                .err(),
            Some(SessionError::Unavailable)
        );

        // The same authority oracle applies after a healthy StorageServer is
        // bound; a valid principal may then create a session normally.
        let mut bound_healthy = StorageBroker::new();
        bound_healthy
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        assert_connect_authority_oracle(&mut bound_healthy, "bound healthy");
        let healthy_client = bound_healthy
            .connect(APP_PID, UserImageId::App, Rights::READ)
            .unwrap();
        assert_eq!(bound_healthy.snapshot().pending_sessions, 1);
        drop(healthy_client);

        let mut recovering = StorageBroker::new();
        recovering.require_ownerless_recovery().unwrap();
        assert_connect_authority_oracle(&mut recovering, "recovery required");
        assert_eq!(
            recovering
                .connect(APP_PID, UserImageId::App, Rights::READ)
                .err(),
            Some(SessionError::RecoveryRequired)
        );

        let mut offline = StorageBroker::new();
        offline.require_ownerless_recovery().unwrap();
        offline.enter_device_offline().unwrap();
        assert_connect_authority_oracle(&mut offline, "device offline");
        assert_eq!(
            offline
                .connect(APP_PID, UserImageId::App, Rights::READ)
                .err(),
            Some(SessionError::RecoveryRequired)
        );
    }

    #[test]
    fn eight_sector_read_completion_is_copied_once_and_accounted_exactly() {
        assert!(core::mem::size_of::<CompletionInfo>() <= 32);
        let mut broker = StorageBroker::new();
        let cap = broker
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        let first_lba = APP_DATA_VOLUME_SECTORS - STORAGE_BLOCK_MAX_SECTORS as u64;
        let token = broker
            .submit(
                cap,
                SERVER_PID,
                StorageBlockRequest::read_batch(first_lba, STORAGE_BLOCK_MAX_SECTORS).unwrap(),
            )
            .unwrap();
        assert_eq!(token, 1);
        assert_eq!(broker.signals(cap), ObjectSignals::NONE);
        assert_eq!(
            broker.completion_info(cap, SERVER_PID, token),
            Err(RequestError::Busy)
        );
        let request = broker.begin_service().unwrap();
        assert_eq!(request.token(), token);
        assert_eq!(request.operation(), StorageBlockOperation::Read);
        assert_eq!(request.sector_count(), STORAGE_BLOCK_MAX_SECTORS);
        assert_eq!(request.byte_len(), STORAGE_BLOCK_DATA_MAX_BYTES);
        let mut data = [0_u8; STORAGE_BLOCK_DATA_MAX_BYTES];
        for (index, byte) in data.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(17).wrapping_add(3);
        }
        broker
            .finish_service(request, CompletionCode::Ok, data)
            .unwrap();
        assert_eq!(broker.signals(cap), ObjectSignals::READABLE);
        let info = broker.completion_info(cap, SERVER_PID, token).unwrap();
        assert_eq!(info.operation(), StorageBlockOperation::Read);
        assert_eq!(info.code(), CompletionCode::Ok);
        assert_eq!(info.sector_count(), STORAGE_BLOCK_MAX_SECTORS);
        assert_eq!(info.data_bytes(), STORAGE_BLOCK_DATA_MAX_BYTES);
        assert_eq!(
            broker.completion_info(cap, SERVER_PID, token + 1),
            Err(RequestError::TokenMismatch)
        );
        let mut copied = [0xcc; STORAGE_BLOCK_DATA_MAX_BYTES];
        assert_eq!(
            broker
                .copy_completion_data(cap, SERVER_PID, token, &mut copied)
                .unwrap(),
            STORAGE_BLOCK_DATA_MAX_BYTES
        );
        assert_eq!(copied, data);
        broker
            .acknowledge_completion(cap, SERVER_PID, token)
            .unwrap();
        assert_eq!(broker.signals(cap), ObjectSignals::WRITABLE);
        assert_eq!(
            broker.completion_info(cap, SERVER_PID, token),
            Err(RequestError::NoPendingRequest)
        );
        let snapshot = broker.snapshot();
        assert_eq!(snapshot.submissions, 1);
        assert_eq!(snapshot.completions, 1);
        assert_eq!(snapshot.retrievals, 1);
        assert_eq!(snapshot.submitted_sectors, 8);
        assert_eq!(snapshot.completed_sectors, 8);
        assert_eq!(snapshot.read_sectors, 8);
        assert_eq!(snapshot.write_sectors, 0);
    }

    #[test]
    fn batch_tail_boundary_and_sector_counts_are_canonical() {
        assert!(StorageBlockRequest::read(0).is_ok());
        assert!(StorageBlockRequest::read(APP_DATA_VOLUME_SECTORS - 1).is_ok());
        assert!(StorageBlockRequest::read(APP_DATA_VOLUME_SECTORS).is_err());
        assert!(StorageBlockRequest::read(u64::MAX).is_err());
        assert!(
            StorageBlockRequest::read_batch(
                APP_DATA_VOLUME_SECTORS - STORAGE_BLOCK_MAX_SECTORS as u64,
                STORAGE_BLOCK_MAX_SECTORS,
            )
            .is_ok()
        );
        assert_eq!(
            StorageBlockRequest::read_batch(
                APP_DATA_VOLUME_SECTORS - STORAGE_BLOCK_MAX_SECTORS as u64 + 1,
                STORAGE_BLOCK_MAX_SECTORS,
            ),
            Err(StorageBlockRequestError::RelativeLbaOutOfRange)
        );
        assert_eq!(
            StorageBlockRequest::read_batch(0, 0),
            Err(StorageBlockRequestError::InvalidSectorCount)
        );
        assert_eq!(
            StorageBlockRequest::read_batch(0, STORAGE_BLOCK_MAX_SECTORS + 1),
            Err(StorageBlockRequestError::InvalidSectorCount)
        );
        assert!(super::storage_batch_in_bounds(
            APP_DATA_VOLUME_SECTORS - STORAGE_BLOCK_MAX_SECTORS as u64,
            STORAGE_BLOCK_MAX_SECTORS,
        ));
        assert!(!super::storage_batch_in_bounds(
            APP_DATA_VOLUME_SECTORS - STORAGE_BLOCK_MAX_SECTORS as u64 + 1,
            STORAGE_BLOCK_MAX_SECTORS,
        ));
        assert!(!super::storage_batch_in_bounds(0, 0));
        assert!(!super::storage_batch_in_bounds(
            0,
            STORAGE_BLOCK_MAX_SECTORS + 1,
        ));
        assert!(!super::storage_batch_in_bounds(u64::MAX, 1));
    }

    #[test]
    fn write_batch_padding_and_completion_output_are_fail_closed() {
        let mut broker = StorageBroker::new();
        let cap = broker
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        let sectors = [[0x3c; STORAGE_SECTOR_SIZE], [0xa7; STORAGE_SECTOR_SIZE]];
        let submitted = StorageBlockRequest::write_batch(17, &sectors).unwrap();
        assert_eq!(submitted.sector_count(), 2);
        assert_eq!(submitted.byte_len(), 2 * STORAGE_SECTOR_SIZE);
        assert_eq!(&submitted.data()[..STORAGE_SECTOR_SIZE], &sectors[0]);
        assert_eq!(
            &submitted.data()[STORAGE_SECTOR_SIZE..2 * STORAGE_SECTOR_SIZE],
            &sectors[1]
        );
        assert!(
            submitted.data()[2 * STORAGE_SECTOR_SIZE..]
                .iter()
                .all(|byte| *byte == 0)
        );

        let token = broker.submit(cap, SERVER_PID, submitted).unwrap();
        let request = broker.begin_service().unwrap();
        let mut invalid_output = [0; STORAGE_BLOCK_DATA_MAX_BYTES];
        invalid_output[0] = 1;
        assert_eq!(
            broker.finish_service(request, CompletionCode::Ok, invalid_output),
            Err(RequestError::InvalidTransition)
        );
        broker
            .finish_service(
                request,
                CompletionCode::Ok,
                [0; STORAGE_BLOCK_DATA_MAX_BYTES],
            )
            .unwrap();
        let info = broker.completion_info(cap, SERVER_PID, token).unwrap();
        assert_eq!(info.operation(), StorageBlockOperation::Write);
        assert_eq!(info.sector_count(), 2);
        assert_eq!(info.data_bytes(), 0);
        let mut copied = [0xcc; STORAGE_BLOCK_DATA_MAX_BYTES];
        assert_eq!(
            broker
                .copy_completion_data(cap, SERVER_PID, token, &mut copied)
                .unwrap(),
            0
        );
        assert_eq!(copied, [0; STORAGE_BLOCK_DATA_MAX_BYTES]);
        broker
            .acknowledge_completion(cap, SERVER_PID, token)
            .unwrap();
        let snapshot = broker.snapshot();
        assert_eq!(snapshot.submitted_sectors, 2);
        assert_eq!(snapshot.completed_sectors, 2);
        assert_eq!(snapshot.read_sectors, 0);
        assert_eq!(snapshot.write_sectors, 2);
    }

    #[test]
    fn sessions_are_non_selectable_and_fifo_bound_to_epoch() {
        let mut broker = StorageBroker::new();
        let cap = broker
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        let app = broker
            .connect(APP_PID, UserImageId::App, Rights::READ)
            .unwrap();
        assert_eq!(
            broker.signals(cap),
            ObjectSignals::from_bits(
                ObjectSignals::READABLE.bits() | ObjectSignals::WRITABLE.bits()
            )
            .unwrap()
        );
        let launcher = broker
            .connect(
                LAUNCHER_PID,
                UserImageId::Launcher,
                Rights::from_bits(Rights::READ.bits() | Rights::WRITE.bits()).unwrap(),
            )
            .unwrap();
        assert!(matches!(
            broker.connect(APP_PID, UserImageId::Init, Rights::READ),
            Err(SessionError::PermissionDenied)
        ));
        assert!(matches!(
            broker.connect(APP_PID, UserImageId::App, Rights::TRANSFER),
            Err(SessionError::InvalidRights)
        ));
        let (app_server, app_binding) = broker.accept(cap, SERVER_PID).unwrap();
        let (launcher_server, launcher_binding) = broker.accept(cap, SERVER_PID).unwrap();
        assert_eq!(broker.signals(cap), ObjectSignals::WRITABLE);
        assert!(app.is_peer_of(&app_server));
        assert!(launcher.is_peer_of(&launcher_server));
        assert_eq!(app_binding.client_pid(), APP_PID);
        assert_eq!(app_binding.principal().raw(), 1);
        assert_eq!(launcher_binding.client_pid(), LAUNCHER_PID);
        assert_eq!(launcher_binding.principal().raw(), 2);
        assert_eq!(app_binding.server_epoch(), cap.epoch());
        assert_eq!(launcher_binding.server_epoch(), cap.epoch());
    }

    #[test]
    fn owner_release_closes_pending_sessions_and_stales_capability() {
        let mut broker = StorageBroker::new();
        let cap = broker
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        let client = broker
            .connect(APP_PID, UserImageId::App, Rights::READ)
            .unwrap();
        assert!(broker.release_process(SERVER_PID));
        assert_eq!(broker.signals(cap), ObjectSignals::PEER_CLOSED);
        assert!(client.signals().intersects(ObjectSignals::PEER_CLOSED));
        let next = broker
            .acquire(SERVER_PID + (1_u64 << 32), UserImageId::StorageServer)
            .unwrap();
        assert_eq!(next.epoch(), cap.epoch() + 1);
    }

    #[test]
    fn session_queue_is_bounded_without_consuming_an_id_on_rejection() {
        let mut broker = StorageBroker::new();
        let cap = broker
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        for index in 0..super::SESSION_QUEUE_CAPACITY {
            let _client = broker
                .connect(APP_PID + index as u64, UserImageId::App, Rights::READ)
                .unwrap();
        }
        let before = broker.snapshot();
        assert!(matches!(
            broker.connect(APP_PID + 99, UserImageId::App, Rights::READ),
            Err(SessionError::QueueFull)
        ));
        let rejected = broker.snapshot();
        assert_eq!(rejected.next_session_id, before.next_session_id);
        assert_eq!(rejected.pending_sessions, super::SESSION_QUEUE_CAPACITY);

        let (_, first) = broker.accept(cap, SERVER_PID).unwrap();
        assert_eq!(first.session_id(), 1);
        let replacement = broker
            .connect(APP_PID + 99, UserImageId::App, Rights::READ)
            .unwrap();
        let mut last = first;
        while broker.snapshot().pending_sessions != 0 {
            let (_, binding) = broker.accept(cap, SERVER_PID).unwrap();
            assert!(binding.session_id() > last.session_id());
            last = binding;
        }
        assert_eq!(
            last.session_id(),
            (super::SESSION_QUEUE_CAPACITY + 1) as u64
        );
        drop(replacement);
    }

    #[test]
    fn full_process_identity_and_epoch_prevent_stale_authority() {
        let mut broker = StorageBroker::new();
        let cap = broker
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        assert!(matches!(
            broker.submit(
                cap,
                SERVER_PID + (1_u64 << 32),
                StorageBlockRequest::flush(),
            ),
            Err(RequestError::CapabilityMismatch)
        ));
        assert!(broker.release_process(SERVER_PID));
        let replacement_pid = SERVER_PID + (1_u64 << 32);
        let replacement = broker
            .acquire(replacement_pid, UserImageId::StorageServer)
            .unwrap();
        assert_ne!(replacement.epoch(), cap.epoch());
        assert!(matches!(
            broker.submit(cap, replacement_pid, StorageBlockRequest::flush()),
            Err(RequestError::CapabilityMismatch)
        ));
        assert!(matches!(
            broker.accept(cap, replacement_pid),
            Err(SessionError::CapabilityMismatch)
        ));
    }

    #[test]
    fn reset_required_is_fail_stopped_at_a_new_owner_epoch() {
        let mut broker = StorageBroker::new();
        let cap = broker
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        let queued_client = broker
            .connect(APP_PID, UserImageId::App, Rights::READ)
            .unwrap();
        let token = broker
            .submit(cap, SERVER_PID, StorageBlockRequest::flush())
            .unwrap();
        let request = broker.begin_service().unwrap();
        broker
            .finish_service(
                request,
                CompletionCode::RequiresReset,
                [0; STORAGE_BLOCK_DATA_MAX_BYTES],
            )
            .unwrap();
        let info = broker.completion_info(cap, SERVER_PID, token).unwrap();
        assert_eq!(info.operation(), StorageBlockOperation::Flush);
        assert_eq!(info.code(), CompletionCode::RequiresReset);
        assert_eq!(info.sector_count(), 0);
        assert_eq!(info.data_bytes(), 0);
        broker
            .acknowledge_completion(cap, SERVER_PID, token)
            .unwrap();
        assert_eq!(broker.signals(cap), ObjectSignals::PEER_CLOSED);
        assert!(matches!(
            broker.submit(cap, SERVER_PID, StorageBlockRequest::read(0).unwrap()),
            Err(RequestError::RecoveryRequired)
        ));
        assert!(matches!(
            broker.connect(LAUNCHER_PID, UserImageId::Launcher, Rights::READ),
            Err(SessionError::RecoveryRequired)
        ));
        assert!(matches!(
            broker.peek_session_binding(cap, SERVER_PID),
            Err(SessionError::RecoveryRequired)
        ));
        assert!(matches!(
            broker.accept(cap, SERVER_PID),
            Err(SessionError::RecoveryRequired)
        ));
        let stale_pid = SERVER_PID + (1_u64 << 32);
        assert!(matches!(
            broker.submit(cap, stale_pid, StorageBlockRequest::read(0).unwrap()),
            Err(RequestError::CapabilityMismatch)
        ));
        assert!(matches!(
            broker.peek_session_binding(cap, stale_pid),
            Err(SessionError::CapabilityMismatch)
        ));
        assert!(matches!(
            broker.accept(cap, stale_pid),
            Err(SessionError::CapabilityMismatch)
        ));
        let snapshot = broker.snapshot();
        assert!(snapshot.bound);
        assert_eq!(snapshot.pending_sessions, 1);
        assert_eq!(snapshot.state, 4);
        assert_eq!(snapshot.submitted_sectors, 0);
        assert_eq!(snapshot.completed_sectors, 0);
        assert_eq!(snapshot.read_sectors, 0);
        assert_eq!(snapshot.write_sectors, 0);
        assert_eq!(
            broker.complete_recovery(),
            Err(RequestError::InvalidTransition)
        );
        assert_eq!(broker.signals(cap), ObjectSignals::PEER_CLOSED);

        assert!(broker.release_process(SERVER_PID));
        let released = broker.snapshot();
        assert!(!released.bound);
        assert_eq!(released.pending_sessions, 0);
        assert_eq!(released.state, 4);
        assert!(
            queued_client
                .signals()
                .intersects(ObjectSignals::PEER_CLOSED)
        );
        assert!(matches!(
            broker.connect(LAUNCHER_PID, UserImageId::Launcher, Rights::READ),
            Err(SessionError::RecoveryRequired)
        ));
        assert!(matches!(
            broker.submit(cap, SERVER_PID, StorageBlockRequest::read(0).unwrap()),
            Err(RequestError::CapabilityMismatch)
        ));
        assert!(matches!(
            broker.accept(cap, SERVER_PID),
            Err(SessionError::CapabilityMismatch)
        ));
        broker.complete_recovery().unwrap();
        assert_eq!(broker.signals(cap), ObjectSignals::PEER_CLOSED);

        let replacement_pid = SERVER_PID + (1_u64 << 32);
        let replacement = broker
            .acquire(replacement_pid, UserImageId::StorageServer)
            .unwrap();
        assert_eq!(replacement.epoch(), cap.epoch() + 1);
        assert_eq!(broker.signals(replacement), ObjectSignals::WRITABLE);
        assert!(matches!(
            broker.submit(cap, SERVER_PID, StorageBlockRequest::read(0).unwrap()),
            Err(RequestError::CapabilityMismatch)
        ));
        assert!(matches!(
            broker.accept(cap, SERVER_PID),
            Err(SessionError::CapabilityMismatch)
        ));
        assert!(matches!(
            broker.completion_info(cap, SERVER_PID, token),
            Err(RequestError::CapabilityMismatch)
        ));
        assert!(matches!(
            broker.completion_info(replacement, replacement_pid, token),
            Err(RequestError::NoPendingRequest)
        ));
        let replacement_token = broker
            .submit(
                replacement,
                replacement_pid,
                StorageBlockRequest::read(0).unwrap(),
            )
            .unwrap();
        assert_ne!(replacement_token, token);
        let replacement_request = broker.begin_service().unwrap();
        broker
            .finish_service(
                replacement_request,
                CompletionCode::Ok,
                [0; STORAGE_BLOCK_DATA_MAX_BYTES],
            )
            .unwrap();
        assert!(matches!(
            broker.completion_info(replacement, replacement_pid, token),
            Err(RequestError::TokenMismatch)
        ));
    }

    #[test]
    fn outcome_unknown_is_mutation_only_and_requires_recovery() {
        let mut broker = StorageBroker::new();
        let cap = broker
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        let token = broker
            .submit(
                cap,
                SERVER_PID,
                StorageBlockRequest::write(7, [0x5a; STORAGE_SECTOR_SIZE]).unwrap(),
            )
            .unwrap();
        let request = broker.begin_service().unwrap();
        broker
            .finish_service(
                request,
                CompletionCode::OutcomeUnknown,
                [0; STORAGE_BLOCK_DATA_MAX_BYTES],
            )
            .unwrap();
        assert_eq!(
            broker
                .completion_info(cap, SERVER_PID, token)
                .unwrap()
                .code(),
            CompletionCode::OutcomeUnknown
        );
        broker
            .acknowledge_completion(cap, SERVER_PID, token)
            .unwrap();
        assert_eq!(broker.signals(cap), ObjectSignals::PEER_CLOSED);
        assert!(matches!(
            broker.submit(cap, SERVER_PID, StorageBlockRequest::read(0).unwrap()),
            Err(RequestError::RecoveryRequired)
        ));
        assert_eq!(
            broker.complete_recovery(),
            Err(RequestError::InvalidTransition)
        );
        assert!(broker.release_process(SERVER_PID));
        assert_eq!(broker.snapshot().state, 4);
        broker.complete_recovery().unwrap();

        let replacement_pid = SERVER_PID + (1_u64 << 32);
        let replacement = broker
            .acquire(replacement_pid, UserImageId::StorageServer)
            .unwrap();
        let read_token = broker
            .submit(
                replacement,
                replacement_pid,
                StorageBlockRequest::read(0).unwrap(),
            )
            .unwrap();
        let read = broker.begin_service().unwrap();
        assert_eq!(
            broker.finish_service(
                read,
                CompletionCode::OutcomeUnknown,
                [0; STORAGE_BLOCK_DATA_MAX_BYTES],
            ),
            Err(RequestError::InvalidTransition)
        );
        broker
            .finish_service(
                read,
                CompletionCode::RequiresReset,
                [0; STORAGE_BLOCK_DATA_MAX_BYTES],
            )
            .unwrap();
        assert_eq!(
            broker
                .completion_info(replacement, replacement_pid, read_token)
                .unwrap()
                .code(),
            CompletionCode::RequiresReset
        );
    }

    #[test]
    fn owner_exit_preserves_recovery_for_running_or_poisoned_io() {
        let mut running = StorageBroker::new();
        let running_cap = running
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        running
            .submit(
                running_cap,
                SERVER_PID,
                StorageBlockRequest::read(0).unwrap(),
            )
            .unwrap();
        let running_request = running.begin_service().unwrap();
        assert!(running.release_process(SERVER_PID));
        let ownerless_running = running.snapshot();
        assert_eq!(ownerless_running.state, 4);
        assert!(ownerless_running.abandoned_running);
        assert!(
            running
                .acquire(SERVER_PID + (1_u64 << 32), UserImageId::StorageServer)
                .is_err()
        );
        assert_eq!(
            running.complete_recovery(),
            Err(RequestError::InvalidTransition)
        );
        let wrong_request = StorageBlockRequest::read(0)
            .unwrap()
            .with_kernel_token(running_request.token() + 1)
            .unwrap();
        assert_eq!(
            running.finish_service(
                wrong_request,
                CompletionCode::RequiresReset,
                [0; STORAGE_BLOCK_DATA_MAX_BYTES],
            ),
            Err(RequestError::TokenMismatch)
        );
        assert!(running.snapshot().abandoned_running);
        assert_eq!(
            running.finish_service(
                running_request,
                CompletionCode::RequiresReset,
                [0; STORAGE_BLOCK_DATA_MAX_BYTES],
            ),
            Err(RequestError::ServiceAbandoned)
        );
        let discarded = running.snapshot();
        assert!(!discarded.abandoned_running);
        assert_eq!(discarded.submissions, 1);
        assert_eq!(discarded.completions, 0);
        running.complete_recovery().unwrap();
        assert!(
            running
                .acquire(SERVER_PID + (1_u64 << 32), UserImageId::StorageServer)
                .is_ok()
        );

        let mut completed = StorageBroker::new();
        let completed_cap = completed
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        completed
            .submit(completed_cap, SERVER_PID, StorageBlockRequest::flush())
            .unwrap();
        let request = completed.begin_service().unwrap();
        completed
            .finish_service(
                request,
                CompletionCode::RequiresReset,
                [0; STORAGE_BLOCK_DATA_MAX_BYTES],
            )
            .unwrap();
        assert!(completed.release_process(SERVER_PID));
        assert_eq!(completed.snapshot().state, 4);
    }

    #[test]
    fn repeated_fail_stop_recovery_rotates_six_owners_without_reusing_authority() {
        let mut broker = StorageBroker::new();
        let mut owner_pid = SERVER_PID;
        let mut capability = broker
            .acquire(owner_pid, UserImageId::StorageServer)
            .unwrap();

        for (round, operation) in [
            StorageBlockOperation::Write,
            StorageBlockOperation::Read,
            StorageBlockOperation::Flush,
            StorageBlockOperation::Write,
            StorageBlockOperation::Read,
            StorageBlockOperation::Flush,
        ]
        .into_iter()
        .enumerate()
        {
            let request = match operation {
                StorageBlockOperation::Read => StorageBlockRequest::read(7).unwrap(),
                StorageBlockOperation::Write => {
                    StorageBlockRequest::write(7, [round as u8; STORAGE_SECTOR_SIZE]).unwrap()
                }
                StorageBlockOperation::Flush => StorageBlockRequest::flush(),
            };
            let token = broker.submit(capability, owner_pid, request).unwrap();
            let accepted = broker.begin_service().unwrap();
            let code = match operation {
                StorageBlockOperation::Read => CompletionCode::RequiresReset,
                StorageBlockOperation::Write | StorageBlockOperation::Flush => {
                    CompletionCode::OutcomeUnknown
                }
            };
            broker
                .finish_service(accepted, code, [0; STORAGE_BLOCK_DATA_MAX_BYTES])
                .unwrap();
            assert_eq!(
                broker
                    .completion_info(capability, owner_pid, token)
                    .unwrap()
                    .code(),
                code
            );
            broker
                .acknowledge_completion(capability, owner_pid, token)
                .unwrap();
            assert_eq!(broker.snapshot().state, 4);
            assert!(broker.release_process(owner_pid));
            let ownerless = broker.snapshot();
            assert!(!ownerless.bound);
            assert_eq!(ownerless.state, 4);
            assert_eq!(
                broker.complete_recovery(),
                Ok(()),
                "round {round} did not retain its ownerless recovery ticket"
            );

            let stale_capability = capability;
            let stale_owner = owner_pid;
            owner_pid = owner_pid.wrapping_add(1_u64 << 32);
            capability = broker
                .acquire(owner_pid, UserImageId::StorageServer)
                .unwrap();
            assert_eq!(capability.epoch(), round as u64 + 2);
            assert_eq!(
                broker.submit(
                    stale_capability,
                    stale_owner,
                    StorageBlockRequest::read(0).unwrap(),
                ),
                Err(RequestError::CapabilityMismatch)
            );
            assert_eq!(
                broker.completion_info(capability, owner_pid, token),
                Err(RequestError::NoPendingRequest)
            );
        }

        let final_snapshot = broker.snapshot();
        assert!(final_snapshot.bound);
        assert_eq!(final_snapshot.owner_pid, owner_pid);
        assert_eq!(final_snapshot.epoch, 7);
        assert_eq!(final_snapshot.next_epoch, 8);
        assert_eq!(final_snapshot.releases, 6);
        assert_eq!(final_snapshot.abandoned, 6);
        assert_eq!(final_snapshot.state, 0);
        assert_eq!(final_snapshot.pending_sessions, 0);
    }

    #[test]
    fn ownerless_physical_latch_can_reclose_an_idle_broker_without_advancing_epoch() {
        let mut broker = StorageBroker::new();
        broker.require_ownerless_recovery().unwrap();
        let latched = broker.snapshot();
        assert!(!latched.bound);
        assert_eq!(latched.state, 4);
        assert_eq!(latched.epoch, 0);
        assert_eq!(latched.next_epoch, 1);
        broker.require_ownerless_recovery().unwrap();
        broker.complete_recovery().unwrap();
        let capability = broker
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        assert_eq!(capability.epoch(), 1);
        assert_eq!(
            broker.require_ownerless_recovery(),
            Err(RequestError::InvalidTransition)
        );
    }

    #[test]
    fn device_offline_is_ownerless_recovery_only_and_rejects_abandoned_io() {
        let mut idle = StorageBroker::new();
        assert_eq!(
            idle.enter_device_offline(),
            Err(RequestError::InvalidTransition)
        );
        assert!(!idle.snapshot().device_offline);

        let capability = idle
            .acquire(SERVER_PID, UserImageId::StorageServer)
            .unwrap();
        assert_eq!(
            idle.enter_device_offline(),
            Err(RequestError::InvalidTransition)
        );
        idle.submit(
            capability,
            SERVER_PID,
            StorageBlockRequest::read(0).unwrap(),
        )
        .unwrap();
        let request = idle.begin_service().unwrap();
        assert!(idle.release_process(SERVER_PID));
        assert!(idle.snapshot().abandoned_running);
        assert_eq!(idle.snapshot().state, 4);
        assert_eq!(
            idle.enter_device_offline(),
            Err(RequestError::InvalidTransition)
        );
        assert_eq!(
            idle.finish_service(
                request,
                CompletionCode::RequiresReset,
                [0; STORAGE_BLOCK_DATA_MAX_BYTES],
            ),
            Err(RequestError::ServiceAbandoned)
        );
        assert!(!idle.snapshot().abandoned_running);
        idle.enter_device_offline().unwrap();
        assert!(idle.snapshot().device_offline);
    }

    #[test]
    fn device_offline_is_sticky_permission_first_and_counted_exactly() {
        let mut broker = StorageBroker::new();
        broker.require_ownerless_recovery().unwrap();

        // An ordinary recovery closure remains the historical AlreadyBound
        // result until the permanent-offline transition is committed.
        assert_eq!(
            broker.acquire(SERVER_PID, UserImageId::StorageServer),
            Err(AcquireError::AlreadyBound)
        );
        assert_eq!(
            broker.acquire(SERVER_PID, UserImageId::App),
            Err(AcquireError::InvalidOwner)
        );

        broker.enter_device_offline().unwrap();
        broker.enter_device_offline().unwrap();
        let transitioned = broker.snapshot();
        assert!(transitioned.device_offline);
        assert_eq!(transitioned.state, 4);
        assert_eq!(transitioned.offline_transitions, 1);
        assert_eq!(transitioned.offline_acquire_denials, 0);

        // Invalid callers cannot use StorageAcquire as a device-health oracle
        // and do not consume the legitimate-offline denial counter.
        assert_eq!(
            broker.acquire(0, UserImageId::StorageServer),
            Err(AcquireError::InvalidOwner)
        );
        assert_eq!(
            broker.acquire(SERVER_PID, UserImageId::App),
            Err(AcquireError::InvalidOwner)
        );
        assert_eq!(broker.snapshot().offline_acquire_denials, 0);

        assert_eq!(
            broker.acquire(SERVER_PID, UserImageId::StorageServer),
            Err(AcquireError::DeviceOffline)
        );
        assert_eq!(
            broker.acquire(
                SERVER_PID.wrapping_add(1_u64 << 32),
                UserImageId::StorageServer,
            ),
            Err(AcquireError::DeviceOffline)
        );
        let denied = broker.snapshot();
        assert_eq!(denied.offline_acquire_denials, 2);
        assert_eq!(denied.offline_transitions, 1);

        assert_eq!(
            broker.complete_recovery(),
            Err(RequestError::InvalidTransition)
        );
        broker.require_ownerless_recovery().unwrap();
        assert_eq!(broker.snapshot().state, 4);
        assert_eq!(
            broker.acquire(SERVER_PID, UserImageId::StorageServer),
            Err(AcquireError::DeviceOffline)
        );
        let sticky = broker.snapshot();
        assert!(sticky.device_offline);
        assert_eq!(sticky.offline_transitions, 1);
        assert_eq!(sticky.offline_acquire_denials, 3);
    }
}
