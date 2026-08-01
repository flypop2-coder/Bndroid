//! Fixed-capacity, generation-qualified physical-input broker.
//!
//! The boot kernel owns one [`InputBroker`]. Its callers serialize access with
//! local IRQ masking: keyboard reports enter from hard IRQ context, while
//! pointer reports and syscalls mask IRQs around the same state. Keeping the
//! queue itself allocation-free makes overload behavior deterministic.

use alloc::alloc::{Layout, alloc, dealloc};
use core::{
    fmt,
    ptr::NonNull,
    sync::atomic::{AtomicU32, AtomicUsize, Ordering, fence},
};

use bndr_abi::{InputEvent, InputEventError, InputEventPayload, ObjectSignals};

use crate::event::EventAllocError;

pub const INPUT_EVENT_QUEUE_CAPACITY: usize = 64;

/// The unique, non-duplicable, non-transferable reader capability for one
/// broker session. Kernel ownership keeps its signal state alive until the
/// session is explicitly released, even if the userspace handle is closing.
#[derive(Clone, Debug)]
pub struct InputCapability {
    session_id: u64,
    process_id: u64,
    acquisition_floor: u64,
    signals: InputSignals,
}

impl InputCapability {
    fn try_new(
        session_id: u64,
        process_id: u64,
        acquisition_floor: u64,
    ) -> Result<Self, EventAllocError> {
        if session_id == 0 || process_id == 0 {
            return Err(EventAllocError);
        }
        Ok(Self {
            session_id,
            process_id,
            acquisition_floor,
            signals: InputSignals::try_new()?,
        })
    }

    pub const fn session_id(&self) -> u64 {
        self.session_id
    }

    pub const fn process_id(&self) -> u64 {
        self.process_id
    }

    pub const fn acquisition_floor(&self) -> u64 {
        self.acquisition_floor
    }

    pub fn same_input(&self, other: &Self) -> bool {
        self.session_id == other.session_id
            && self.process_id == other.process_id
            && self.acquisition_floor == other.acquisition_floor
            && self.signals.same_state(&other.signals)
    }

    pub fn signal_mask(&self) -> ObjectSignals {
        ObjectSignals::from_bits(ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits())
            .unwrap_or_else(|| panic!("input capability signal mask is not in the ABI"))
    }

    pub fn signals(&self) -> ObjectSignals {
        ObjectSignals::from_bits(self.signals.bits())
            .unwrap_or_else(|| panic!("input capability produced invalid signal bits"))
    }

    fn signal_readable(&self) -> bool {
        self.signals.signal(ObjectSignals::READABLE)
    }

    fn clear_readable(&self) -> bool {
        self.signals.clear(ObjectSignals::READABLE)
    }

    fn mark_peer_closed(&self) -> bool {
        self.signals.signal(ObjectSignals::PEER_CLOSED)
    }
}

struct InputSignals {
    inner: NonNull<InputSignalsInner>,
}

struct InputSignalsInner {
    strong: AtomicUsize,
    bits: AtomicU32,
}

unsafe impl Send for InputSignals {}
unsafe impl Sync for InputSignals {}

impl InputSignals {
    fn try_new() -> Result<Self, EventAllocError> {
        let inner = NonNull::new(
            unsafe { alloc(Layout::new::<InputSignalsInner>()) }.cast::<InputSignalsInner>(),
        )
        .ok_or(EventAllocError)?;
        unsafe {
            inner.as_ptr().write(InputSignalsInner {
                strong: AtomicUsize::new(1),
                bits: AtomicU32::new(0),
            });
        }
        Ok(Self { inner })
    }

    fn signal(&self, signal: ObjectSignals) -> bool {
        debug_assert!(signal == ObjectSignals::READABLE || signal == ObjectSignals::PEER_CLOSED);
        self.inner().bits.fetch_or(signal.bits(), Ordering::AcqRel) & signal.bits() == 0
    }

    fn clear(&self, signal: ObjectSignals) -> bool {
        debug_assert!(signal == ObjectSignals::READABLE);
        self.inner()
            .bits
            .fetch_and(!signal.bits(), Ordering::AcqRel)
            & signal.bits()
            != 0
    }

    fn bits(&self) -> u32 {
        self.inner().bits.load(Ordering::Acquire)
    }

    fn same_state(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn inner(&self) -> &InputSignalsInner {
        unsafe { self.inner.as_ref() }
    }
}

impl Clone for InputSignals {
    fn clone(&self) -> Self {
        self.inner()
            .strong
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |strong| {
                strong
                    .checked_add(1)
                    .filter(|next| *next <= isize::MAX as usize)
            })
            .unwrap_or_else(|_| panic!("input signal strong-reference overflow"));
        Self { inner: self.inner }
    }
}

impl Drop for InputSignals {
    fn drop(&mut self) {
        if self.inner().strong.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                core::ptr::drop_in_place(self.inner.as_ptr());
                dealloc(
                    self.inner.as_ptr().cast::<u8>(),
                    Layout::new::<InputSignalsInner>(),
                );
            }
        }
    }
}

impl fmt::Debug for InputSignals {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InputSignals")
            .field("inner", &self.inner)
            .field("bits", &self.bits())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputQueueError {
    Full,
    CounterExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEnqueueDisposition {
    Queued,
    Coalesced,
    DroppedUnbound,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputEnqueueEvidence {
    pub event: InputEvent,
    pub readable_edge: bool,
    pub coalesced: bool,
    pub pending: usize,
    pub disposition: InputEnqueueDisposition,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InputQueueSnapshot {
    pub pending: usize,
    pub enqueued: u64,
    pub dequeued: u64,
    pub coalesced: u64,
    pub high_watermark: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InputEventQueue {
    slots: [Option<InputEvent>; INPUT_EVENT_QUEUE_CAPACITY],
    head: usize,
    len: usize,
    enqueued: u64,
    dequeued: u64,
    coalesced: u64,
    high_watermark: usize,
}

impl InputEventQueue {
    const fn new() -> Self {
        Self {
            slots: [None; INPUT_EVENT_QUEUE_CAPACITY],
            head: 0,
            len: 0,
            enqueued: 0,
            dequeued: 0,
            coalesced: 0,
            high_watermark: 0,
        }
    }

    fn push(&mut self, event: InputEvent) -> Result<(InputEvent, bool), InputQueueError> {
        let next_enqueued = self
            .enqueued
            .checked_add(1)
            .ok_or(InputQueueError::CounterExhausted)?;
        if self.len == INPUT_EVENT_QUEUE_CAPACITY {
            let tail = (self.head + self.len - 1) % INPUT_EVENT_QUEUE_CAPACITY;
            let previous =
                self.slots[tail].unwrap_or_else(|| panic!("full input queue had an empty tail"));
            let before_tail = self.slots
                [(tail + INPUT_EVENT_QUEUE_CAPACITY - 1) % INPUT_EVENT_QUEUE_CAPACITY]
                .unwrap_or_else(|| panic!("full input queue had an empty penultimate slot"));
            let same_pointer_motion = matches!(
                (
                    before_tail.payload(),
                    previous.payload(),
                    event.payload(),
                ),
                (
                    InputEventPayload::Pointer {
                        pressed: before_pressed,
                        ..
                    },
                    InputEventPayload::Pointer {
                        pressed: previous_pressed,
                        ..
                    },
                    InputEventPayload::Pointer {
                        pressed: next_pressed,
                        ..
                    }
                ) if before_tail.device_id() == previous.device_id()
                    && previous.device_id() == event.device_id()
                    && before_pressed == previous_pressed
                    && previous_pressed == next_pressed
            );
            if !same_pointer_motion {
                return Err(InputQueueError::Full);
            }
            let next_coalesced = self
                .coalesced
                .checked_add(1)
                .ok_or(InputQueueError::CounterExhausted)?;
            let (x, y, pressed) = event
                .pointer()
                .unwrap_or_else(|| panic!("validated pointer coalescing lost its payload"));
            let replacement =
                InputEvent::try_pointer(previous.sequence(), event.device_id(), x, y, pressed)
                    .unwrap_or_else(|_| {
                        panic!("validated pointer motion could not retain its sequence")
                    });
            self.slots[tail] = Some(replacement);
            self.coalesced = next_coalesced;
            self.enqueued = next_enqueued;
            return Ok((replacement, true));
        }

        let tail = (self.head + self.len) % INPUT_EVENT_QUEUE_CAPACITY;
        if self.slots[tail].replace(event).is_some() {
            panic!("input queue overwrote a live slot");
        }
        self.len += 1;
        self.enqueued = next_enqueued;
        self.high_watermark = self.high_watermark.max(self.len);
        Ok((event, false))
    }

    fn pop(&mut self) -> Result<Option<InputEvent>, InputQueueError> {
        if self.len == 0 {
            return Ok(None);
        }
        let next_dequeued = self
            .dequeued
            .checked_add(1)
            .ok_or(InputQueueError::CounterExhausted)?;
        let event = self.slots[self.head]
            .take()
            .unwrap_or_else(|| panic!("non-empty input queue had an empty head"));
        self.head = (self.head + 1) % INPUT_EVENT_QUEUE_CAPACITY;
        self.len -= 1;
        self.dequeued = next_dequeued;
        Ok(Some(event))
    }

    fn clear(&mut self) -> usize {
        let discarded = self.len;
        for slot in &mut self.slots {
            *slot = None;
        }
        self.head = 0;
        self.len = 0;
        discarded
    }

    const fn snapshot(&self) -> InputQueueSnapshot {
        InputQueueSnapshot {
            pending: self.len,
            enqueued: self.enqueued,
            dequeued: self.dequeued,
            coalesced: self.coalesced,
            high_watermark: self.high_watermark,
        }
    }
}

#[derive(Clone, Debug)]
struct InputBinding {
    capability: InputCapability,
    failed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputAcquireError {
    InvalidProcess,
    AlreadyBound,
    SessionExhausted,
    OutOfMemory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputBrokerError {
    Unbound,
    PeerClosed,
    CapabilityMismatch,
    InvalidEvent(InputEventError),
    SequenceExhausted,
    Full,
    CounterExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputSessionInfo {
    pub session_id: u64,
    pub process_id: u64,
    pub acquisition_floor: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputReleaseEvidence {
    pub session_id: u64,
    pub process_id: u64,
    pub acquisition_floor: u64,
    pub release_floor: u64,
    pub discarded: usize,
    pub peer_closed_edge: bool,
    pub failed: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InputBrokerSnapshot {
    pub session_id: u64,
    pub process_id: u64,
    pub acquisition_floor: u64,
    pub next_session_id: u64,
    pub next_sequence: u64,
    pub failures: u64,
    pub releases: u64,
    pub release_discarded: u64,
    pub last_release_session_id: u64,
    pub last_release_process_id: u64,
    pub last_release_acquisition_floor: u64,
    pub last_release_floor: u64,
    pub last_release_discarded: usize,
    pub last_release_peer_closed_edge: bool,
    pub last_release_failed: bool,
    pub unbound_dropped: u64,
    pub unbound_pointer_dropped: u64,
    pub unbound_key_dropped: u64,
    pub queue: InputQueueSnapshot,
    pub bound: bool,
    pub failed: bool,
}

/// One serialized kernel broker. No event survives a capability generation,
/// while the global event sequence continues monotonically across restarts.
/// Valid reports observed without a bound InputServer consume that sequence
/// and are audited as safe drops; they are never replayed into a later
/// generation.
#[derive(Debug)]
pub struct InputBroker {
    binding: Option<InputBinding>,
    queue: InputEventQueue,
    next_session_id: u64,
    next_sequence: u64,
    failures: u64,
    releases: u64,
    release_discarded: u64,
    last_release: Option<InputReleaseEvidence>,
    unbound_dropped: u64,
    unbound_pointer_dropped: u64,
    unbound_key_dropped: u64,
}

impl Default for InputBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl InputBroker {
    pub const fn new() -> Self {
        Self {
            binding: None,
            queue: InputEventQueue::new(),
            next_session_id: 1,
            next_sequence: 1,
            failures: 0,
            releases: 0,
            release_discarded: 0,
            last_release: None,
            unbound_dropped: 0,
            unbound_pointer_dropped: 0,
            unbound_key_dropped: 0,
        }
    }

    pub fn acquire(&mut self, process_id: u64) -> Result<InputCapability, InputAcquireError> {
        if process_id == 0 {
            return Err(InputAcquireError::InvalidProcess);
        }
        if self.binding.is_some() {
            return Err(InputAcquireError::AlreadyBound);
        }
        let session_id = self.next_session_id;
        let next_session_id = session_id
            .checked_add(1)
            .ok_or(InputAcquireError::SessionExhausted)?;
        let acquisition_floor = self
            .next_sequence
            .checked_sub(1)
            .ok_or(InputAcquireError::SessionExhausted)?;
        let capability = InputCapability::try_new(session_id, process_id, acquisition_floor)
            .map_err(|_| InputAcquireError::OutOfMemory)?;
        if self.queue.clear() != 0 {
            panic!("unbound input broker retained stale events");
        }
        self.binding = Some(InputBinding {
            capability: capability.clone(),
            failed: false,
        });
        self.next_session_id = next_session_id;
        Ok(capability)
    }

    pub fn enqueue_pointer(
        &mut self,
        device_id: u8,
        x: u16,
        y: u16,
        pressed: bool,
    ) -> Result<InputEnqueueEvidence, InputBrokerError> {
        let sequence = self.preflight_enqueue()?;
        let event = InputEvent::try_pointer(sequence, device_id, x, y, pressed)
            .map_err(|error| self.poison(InputBrokerError::InvalidEvent(error)))?;
        self.commit_enqueue(event)
    }

    pub fn enqueue_key(
        &mut self,
        device_id: u8,
        code: u16,
        value: u8,
    ) -> Result<InputEnqueueEvidence, InputBrokerError> {
        let sequence = self.preflight_enqueue()?;
        let event = InputEvent::try_key(sequence, device_id, code, value)
            .map_err(|error| self.poison(InputBrokerError::InvalidEvent(error)))?;
        self.commit_enqueue(event)
    }

    fn preflight_enqueue(&mut self) -> Result<u64, InputBrokerError> {
        if self.binding.as_ref().is_some_and(|binding| binding.failed) {
            return Err(InputBrokerError::PeerClosed);
        }
        if self.next_sequence == 0 || self.next_sequence.checked_add(1).is_none() {
            return Err(self.poison(InputBrokerError::SequenceExhausted));
        }
        Ok(self.next_sequence)
    }

    fn commit_enqueue(
        &mut self,
        event: InputEvent,
    ) -> Result<InputEnqueueEvidence, InputBrokerError> {
        if self.binding.is_none() {
            return self.commit_unbound_drop(event);
        }
        let was_empty = self.queue.len == 0;
        let (event, coalesced) = match self.queue.push(event) {
            Ok(committed) => committed,
            Err(InputQueueError::Full) => return Err(self.poison(InputBrokerError::Full)),
            Err(InputQueueError::CounterExhausted) => {
                return Err(self.poison(InputBrokerError::CounterExhausted));
            }
        };
        if !coalesced {
            self.next_sequence = self
                .next_sequence
                .checked_add(1)
                .unwrap_or_else(|| panic!("preflighted input sequence overflowed"));
        }
        let readable_edge = was_empty
            && self
                .binding
                .as_ref()
                .unwrap_or_else(|| panic!("input binding disappeared during enqueue"))
                .capability
                .signal_readable();
        Ok(InputEnqueueEvidence {
            event,
            readable_edge,
            coalesced,
            pending: self.queue.len,
            disposition: if coalesced {
                InputEnqueueDisposition::Coalesced
            } else {
                InputEnqueueDisposition::Queued
            },
        })
    }

    fn commit_unbound_drop(
        &mut self,
        event: InputEvent,
    ) -> Result<InputEnqueueEvidence, InputBrokerError> {
        if self.queue.len != 0 {
            panic!("unbound input broker retained queued events");
        }
        let Some(next_sequence) = self.next_sequence.checked_add(1) else {
            return Err(self.poison(InputBrokerError::SequenceExhausted));
        };
        let Some(next_total) = self.unbound_dropped.checked_add(1) else {
            return Err(self.poison(InputBrokerError::CounterExhausted));
        };
        let (next_pointer, next_key) = match event.payload() {
            InputEventPayload::Pointer { .. } => {
                let Some(next_pointer) = self.unbound_pointer_dropped.checked_add(1) else {
                    return Err(self.poison(InputBrokerError::CounterExhausted));
                };
                (next_pointer, self.unbound_key_dropped)
            }
            InputEventPayload::Key { .. } => {
                let Some(next_key) = self.unbound_key_dropped.checked_add(1) else {
                    return Err(self.poison(InputBrokerError::CounterExhausted));
                };
                (self.unbound_pointer_dropped, next_key)
            }
        };
        self.next_sequence = next_sequence;
        self.unbound_dropped = next_total;
        self.unbound_pointer_dropped = next_pointer;
        self.unbound_key_dropped = next_key;
        Ok(InputEnqueueEvidence {
            event,
            readable_edge: false,
            coalesced: false,
            pending: 0,
            disposition: InputEnqueueDisposition::DroppedUnbound,
        })
    }

    fn poison(&mut self, error: InputBrokerError) -> InputBrokerError {
        self.failures = self
            .failures
            .checked_add(1)
            .unwrap_or_else(|| panic!("input failure counter overflowed"));
        self.queue.clear();
        if let Some(binding) = self.binding.as_mut() {
            binding.failed = true;
            binding.capability.clear_readable();
            binding.capability.mark_peer_closed();
        }
        error
    }

    pub fn session_info(
        &self,
        capability: &InputCapability,
        process_id: u64,
    ) -> Result<InputSessionInfo, InputBrokerError> {
        let Some(binding) = self.binding.as_ref() else {
            return Err(InputBrokerError::PeerClosed);
        };
        if process_id == 0
            || process_id != capability.process_id()
            || process_id != binding.capability.process_id()
            || !capability.same_input(&binding.capability)
        {
            return Err(InputBrokerError::CapabilityMismatch);
        }
        if binding.failed || capability.signals().intersects(ObjectSignals::PEER_CLOSED) {
            return Err(InputBrokerError::PeerClosed);
        }
        Ok(InputSessionInfo {
            session_id: capability.session_id(),
            process_id,
            acquisition_floor: capability.acquisition_floor(),
        })
    }

    pub fn read(
        &mut self,
        capability: &InputCapability,
        process_id: u64,
    ) -> Result<Option<InputEvent>, InputBrokerError> {
        let Some(binding) = self.binding.as_ref() else {
            return Err(InputBrokerError::PeerClosed);
        };
        if process_id == 0
            || process_id != capability.process_id()
            || process_id != binding.capability.process_id()
            || !capability.same_input(&binding.capability)
        {
            return Err(InputBrokerError::CapabilityMismatch);
        }
        if binding.failed || capability.signals().intersects(ObjectSignals::PEER_CLOSED) {
            return Err(InputBrokerError::PeerClosed);
        }
        let event = match self.queue.pop() {
            Ok(event) => event,
            Err(InputQueueError::CounterExhausted) => {
                return Err(self.poison(InputBrokerError::CounterExhausted));
            }
            Err(InputQueueError::Full) => unreachable!(),
        };
        if self.queue.len == 0 {
            binding.capability.clear_readable();
        }
        Ok(event)
    }

    pub fn release(
        &mut self,
        capability: &InputCapability,
        process_id: u64,
    ) -> Result<InputReleaseEvidence, InputBrokerError> {
        let Some(binding) = self.binding.as_ref() else {
            return Err(InputBrokerError::PeerClosed);
        };
        if process_id == 0
            || process_id != capability.process_id()
            || process_id != binding.capability.process_id()
            || !capability.same_input(&binding.capability)
        {
            return Err(InputBrokerError::CapabilityMismatch);
        }
        let discarded = self.queue.len;
        let next_releases = self
            .releases
            .checked_add(1)
            .ok_or(InputBrokerError::CounterExhausted)?;
        let next_release_discarded = self
            .release_discarded
            .checked_add(
                u64::try_from(discarded)
                    .unwrap_or_else(|_| panic!("input discard count did not fit u64")),
            )
            .ok_or(InputBrokerError::CounterExhausted)?;
        let release_floor = self
            .next_sequence
            .checked_sub(1)
            .unwrap_or_else(|| panic!("input sequence namespace lost its floor"));
        let binding = self
            .binding
            .take()
            .unwrap_or_else(|| panic!("validated input binding disappeared"));
        if self.queue.clear() != discarded {
            panic!("input release discard preflight changed");
        }
        binding.capability.clear_readable();
        let peer_closed_edge = binding.capability.mark_peer_closed();
        let evidence = InputReleaseEvidence {
            session_id: binding.capability.session_id(),
            process_id,
            acquisition_floor: binding.capability.acquisition_floor(),
            release_floor,
            discarded,
            peer_closed_edge,
            failed: binding.failed,
        };
        self.releases = next_releases;
        self.release_discarded = next_release_discarded;
        self.last_release = Some(evidence);
        Ok(evidence)
    }

    pub fn release_process(&mut self, process_id: u64) -> Option<InputReleaseEvidence> {
        let capability = self
            .binding
            .as_ref()
            .filter(|binding| binding.capability.process_id() == process_id)?
            .capability
            .clone();
        Some(
            self.release(&capability, process_id)
                .unwrap_or_else(|_| panic!("owned input capability could not be released")),
        )
    }

    pub fn snapshot(&self) -> InputBrokerSnapshot {
        let (session_id, process_id, acquisition_floor, bound, failed) = match &self.binding {
            Some(binding) => (
                binding.capability.session_id(),
                binding.capability.process_id(),
                binding.capability.acquisition_floor(),
                true,
                binding.failed,
            ),
            None => (0, 0, 0, false, false),
        };
        let last_release = self.last_release;
        InputBrokerSnapshot {
            session_id,
            process_id,
            acquisition_floor,
            next_session_id: self.next_session_id,
            next_sequence: self.next_sequence,
            failures: self.failures,
            releases: self.releases,
            release_discarded: self.release_discarded,
            last_release_session_id: last_release.map_or(0, |release| release.session_id),
            last_release_process_id: last_release.map_or(0, |release| release.process_id),
            last_release_acquisition_floor: last_release
                .map_or(0, |release| release.acquisition_floor),
            last_release_floor: last_release.map_or(0, |release| release.release_floor),
            last_release_discarded: last_release.map_or(0, |release| release.discarded),
            last_release_peer_closed_edge: last_release
                .is_some_and(|release| release.peer_closed_edge),
            last_release_failed: last_release.is_some_and(|release| release.failed),
            unbound_dropped: self.unbound_dropped,
            unbound_pointer_dropped: self.unbound_pointer_dropped,
            unbound_key_dropped: self.unbound_key_dropped,
            queue: self.queue.snapshot(),
            bound,
            failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pointer(sequence: u64, x: u16, pressed: bool) -> InputEvent {
        InputEvent::try_pointer(sequence, 2, x, 10, pressed).unwrap()
    }

    #[test]
    fn capability_uses_only_readable_and_peer_closed_signals() {
        let mut broker = InputBroker::new();
        let capability = broker.acquire(9).unwrap();
        assert_eq!(
            capability.signal_mask().bits(),
            ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits()
        );
        assert_eq!(capability.acquisition_floor(), 0);
        assert_eq!(capability.signals(), ObjectSignals::NONE);
    }

    #[test]
    fn unified_sequence_and_level_readable_cover_pointer_and_key() {
        let mut broker = InputBroker::new();
        let capability = broker.acquire(9).unwrap();
        let pointer = broker.enqueue_pointer(2, 10, 20, false).unwrap();
        let key = broker.enqueue_key(1, 30, 1).unwrap();
        assert_eq!(pointer.event.sequence(), 1);
        assert_eq!(key.event.sequence(), 2);
        assert_eq!(pointer.disposition, InputEnqueueDisposition::Queued);
        assert_eq!(key.disposition, InputEnqueueDisposition::Queued);
        assert!(pointer.readable_edge);
        assert!(!key.readable_edge);
        assert_eq!(capability.signals(), ObjectSignals::READABLE);
        assert_eq!(broker.read(&capability, 9), Ok(Some(pointer.event)));
        assert_eq!(capability.signals(), ObjectSignals::READABLE);
        assert_eq!(broker.read(&capability, 9), Ok(Some(key.event)));
        assert_eq!(capability.signals(), ObjectSignals::NONE);
        assert_eq!(broker.read(&capability, 9), Ok(None));
    }

    #[test]
    fn full_queue_coalesces_only_same_device_pointer_motion_state() {
        let mut queue = InputEventQueue::new();
        for sequence in 1..=INPUT_EVENT_QUEUE_CAPACITY as u64 {
            queue
                .push(pointer(sequence, (sequence % 300) as u16, true))
                .unwrap();
        }
        assert_eq!(queue.snapshot().pending, INPUT_EVENT_QUEUE_CAPACITY);
        let (replacement, coalesced) = queue.push(pointer(65, 200, true)).unwrap();
        assert!(coalesced);
        assert_eq!(replacement.sequence(), 64);
        assert_eq!(replacement.pointer(), Some((200, 10, true)));
        assert_eq!(queue.snapshot().coalesced, 1);
        assert_eq!(queue.snapshot().pending, INPUT_EVENT_QUEUE_CAPACITY);
        assert_eq!(
            queue.push(pointer(66, 201, false)),
            Err(InputQueueError::Full)
        );
        assert_eq!(
            queue.push(InputEvent::try_key(66, 1, 30, 1).unwrap()),
            Err(InputQueueError::Full)
        );
        assert_eq!(queue.pop().unwrap().unwrap().sequence(), 1);
        for expected in 2..64 {
            assert_eq!(queue.pop().unwrap().unwrap().sequence(), expected);
        }
        assert_eq!(queue.pop().unwrap().unwrap().sequence(), 64);
    }

    #[test]
    fn different_pointer_device_cannot_coalesce() {
        let mut queue = InputEventQueue::new();
        for sequence in 1..=INPUT_EVENT_QUEUE_CAPACITY as u64 {
            queue.push(pointer(sequence, 10, false)).unwrap();
        }
        let other_device = InputEvent::try_pointer(65, 3, 11, 10, false).unwrap();
        assert_eq!(queue.push(other_device), Err(InputQueueError::Full));
    }

    #[test]
    fn pointer_edge_at_full_tail_is_never_replaced() {
        let mut queue = InputEventQueue::new();
        for sequence in 1..INPUT_EVENT_QUEUE_CAPACITY as u64 {
            queue.push(pointer(sequence, 10, false)).unwrap();
        }
        queue.push(pointer(64, 11, true)).unwrap();
        assert_eq!(
            queue.push(pointer(65, 12, true)),
            Err(InputQueueError::Full),
            "the pressed edge at the tail is not motion and must remain lossless"
        );
    }

    #[test]
    fn noncoalescable_overload_fails_closed() {
        let mut broker = InputBroker::new();
        let capability = broker.acquire(9).unwrap();
        for index in 0..INPUT_EVENT_QUEUE_CAPACITY {
            broker
                .enqueue_key(1, 30 + index as u16, (index % 3) as u8)
                .unwrap();
        }
        assert_eq!(broker.enqueue_key(1, 100, 1), Err(InputBrokerError::Full));
        assert_eq!(capability.signals(), ObjectSignals::PEER_CLOSED);
        assert_eq!(
            broker.read(&capability, 9),
            Err(InputBrokerError::PeerClosed)
        );
        let snapshot = broker.snapshot();
        assert!(snapshot.failed);
        assert_eq!(snapshot.failures, 1);
        assert_eq!(snapshot.queue.pending, 0);
    }

    #[test]
    fn close_discards_queue_and_reacquire_is_generation_safe() {
        let mut broker = InputBroker::new();
        let first = broker.acquire(9).unwrap();
        broker.enqueue_pointer(2, 1, 2, false).unwrap();
        let release = broker.release(&first, 9).unwrap();
        assert_eq!(release.acquisition_floor, 0);
        assert_eq!(release.release_floor, 1);
        assert_eq!(release.discarded, 1);
        assert!(release.peer_closed_edge);
        assert_eq!(first.signals(), ObjectSignals::PEER_CLOSED);

        let second = broker.acquire(10).unwrap();
        assert_eq!(second.session_id(), first.session_id() + 1);
        assert_eq!(second.acquisition_floor(), 1);
        assert!(!second.same_input(&first));
        assert_eq!(
            broker.read(&first, 9),
            Err(InputBrokerError::CapabilityMismatch)
        );
        let event = broker.enqueue_key(1, 30, 1).unwrap().event;
        assert_eq!(event.sequence(), 2);
        assert_eq!(broker.read(&second, 10), Ok(Some(event)));
        let snapshot = broker.snapshot();
        assert_eq!(snapshot.release_discarded, 1);
        assert_eq!(snapshot.last_release_session_id, first.session_id());
        assert_eq!(snapshot.last_release_process_id, 9);
        assert_eq!(snapshot.last_release_acquisition_floor, 0);
        assert_eq!(snapshot.last_release_floor, 1);
        assert_eq!(snapshot.last_release_discarded, 1);
        assert!(snapshot.last_release_peer_closed_edge);
        assert!(!snapshot.last_release_failed);
    }

    #[test]
    fn unbound_reports_are_safe_drops_with_global_sequence_continuity() {
        let mut broker = InputBroker::new();
        let pointer = broker.enqueue_pointer(2, 10, 20, false).unwrap();
        let key = broker.enqueue_key(1, 30, 1).unwrap();
        assert_eq!(pointer.event.sequence(), 1);
        assert_eq!(key.event.sequence(), 2);
        assert_eq!(pointer.disposition, InputEnqueueDisposition::DroppedUnbound);
        assert_eq!(key.disposition, InputEnqueueDisposition::DroppedUnbound);
        assert!(!pointer.readable_edge);
        assert!(!key.readable_edge);
        assert_eq!(pointer.pending, 0);
        assert_eq!(key.pending, 0);

        let before_acquire = broker.snapshot();
        assert!(!before_acquire.bound);
        assert_eq!(before_acquire.next_sequence, 3);
        assert_eq!(before_acquire.unbound_dropped, 2);
        assert_eq!(before_acquire.unbound_pointer_dropped, 1);
        assert_eq!(before_acquire.unbound_key_dropped, 1);
        assert_eq!(before_acquire.queue, InputQueueSnapshot::default());

        let capability = broker.acquire(9).unwrap();
        assert_eq!(capability.acquisition_floor(), 2);
        assert_eq!(
            broker.session_info(&capability, 9),
            Ok(InputSessionInfo {
                session_id: 1,
                process_id: 9,
                acquisition_floor: 2,
            })
        );
        let delivered = broker.enqueue_pointer(2, 11, 21, false).unwrap();
        assert_eq!(delivered.event.sequence(), 3);
        assert_eq!(delivered.disposition, InputEnqueueDisposition::Queued);
        assert_eq!(broker.read(&capability, 9), Ok(Some(delivered.event)));
    }

    #[test]
    fn session_info_rejects_wrong_stale_and_released_capabilities() {
        let mut broker = InputBroker::new();
        let first = broker.acquire(9).unwrap();
        assert_eq!(
            broker.session_info(&first, 10),
            Err(InputBrokerError::CapabilityMismatch)
        );
        broker.release(&first, 9).unwrap();
        assert_eq!(first.signals(), ObjectSignals::PEER_CLOSED);
        assert_eq!(
            broker.session_info(&first, 9),
            Err(InputBrokerError::PeerClosed)
        );

        let second = broker.acquire(10).unwrap();
        assert_eq!(
            broker.session_info(&first, 9),
            Err(InputBrokerError::CapabilityMismatch)
        );
        assert_eq!(
            broker.session_info(&second, 10),
            Ok(InputSessionInfo {
                session_id: 2,
                process_id: 10,
                acquisition_floor: 0,
            })
        );
    }

    #[test]
    fn recovery_gap_discards_old_pending_and_captures_new_acquisition_floor() {
        let mut broker = InputBroker::new();
        let old = broker.acquire(9).unwrap();
        let pending = broker.enqueue_pointer(2, 10, 20, false).unwrap();
        assert_eq!(pending.event.sequence(), 1);
        let released = broker.release(&old, 9).unwrap();
        assert_eq!(released.discarded, 1);
        assert_eq!(released.release_floor, 1);
        assert_eq!(old.signals(), ObjectSignals::PEER_CLOSED);

        let dropped = broker.enqueue_key(1, 30, 1).unwrap();
        assert_eq!(dropped.event.sequence(), 2);
        assert_eq!(dropped.disposition, InputEnqueueDisposition::DroppedUnbound);
        let replacement = broker.acquire(10).unwrap();
        assert_eq!(replacement.session_id(), 2);
        assert_eq!(replacement.acquisition_floor(), 2);

        let delivered = broker.enqueue_pointer(2, 11, 21, false).unwrap();
        assert_eq!(delivered.event.sequence(), 3);
        assert_eq!(broker.read(&replacement, 10), Ok(Some(delivered.event)));
        let snapshot = broker.snapshot();
        assert_eq!(snapshot.queue.enqueued, 2);
        assert_eq!(snapshot.queue.dequeued, 1);
        assert_eq!(snapshot.queue.pending, 0);
        assert_eq!(snapshot.release_discarded, 1);
        assert_eq!(snapshot.unbound_dropped, 1);
        assert_eq!(snapshot.next_sequence, 4);
    }

    #[test]
    fn acquire_is_unique_and_authenticated() {
        let mut broker = InputBroker::new();
        let capability = broker.acquire(9).unwrap();
        assert!(matches!(
            broker.acquire(10),
            Err(InputAcquireError::AlreadyBound)
        ));
        assert_eq!(
            broker.read(&capability, 10),
            Err(InputBrokerError::CapabilityMismatch)
        );
    }

    #[test]
    fn invalid_hardware_event_poisoning_is_not_silent() {
        let mut broker = InputBroker::new();
        let capability = broker.acquire(9).unwrap();
        assert_eq!(
            broker.enqueue_pointer(0, 1, 2, false),
            Err(InputBrokerError::InvalidEvent(
                InputEventError::InvalidDeviceId
            ))
        );
        assert_eq!(capability.signals(), ObjectSignals::PEER_CLOSED);
        assert_eq!(broker.snapshot().failures, 1);
    }

    #[test]
    fn release_process_is_idempotently_absent_after_first_release() {
        let mut broker = InputBroker::new();
        broker.acquire(9).unwrap();
        assert_eq!(broker.release_process(10), None);
        assert!(broker.release_process(9).is_some());
        assert_eq!(broker.release_process(9), None);
        assert_eq!(broker.snapshot().releases, 1);
    }
}
