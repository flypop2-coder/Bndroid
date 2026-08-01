use alloc::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use core::cell::UnsafeCell;
use core::fmt;
use core::hint::spin_loop;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering, fence};

use bndr_abi::{ChannelMessageKind, ObjectSignals, Rights};

use crate::object::KernelObject;

pub const CHANNEL_CAPACITY: usize = 8;

static CHANNEL_IDS: ChannelIdAllocator = ChannelIdAllocator::new();

struct ChannelIdAllocator(AtomicU64);

impl ChannelIdAllocator {
    const fn new() -> Self {
        Self(AtomicU64::new(1))
    }

    #[cfg(test)]
    const fn starting_at(next: u64) -> Self {
        Self(AtomicU64::new(next))
    }

    fn allocate(&self) -> Result<u64, ChannelAllocError> {
        self.0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                (next != 0).then_some(next.wrapping_add(1))
            })
            .map_err(|_| ChannelAllocError)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferPayload {
    length: u8,
    data: [u8; bndr_abi::CHANNEL_MESSAGE_MAX_BYTES],
}

impl TransferPayload {
    pub fn new(data: [u8; bndr_abi::CHANNEL_MESSAGE_MAX_BYTES], length: usize) -> Option<Self> {
        let length = u8::try_from(length).ok()?;
        (usize::from(length) <= bndr_abi::CHANNEL_MESSAGE_MAX_BYTES)
            .then_some(Self { length, data })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.data[..usize::from(self.length)]
    }

    pub const fn length(&self) -> usize {
        self.length as usize
    }
}

#[derive(Debug, Default)]
pub enum Message {
    #[default]
    Empty,
    Scalar {
        sender_pid: u64,
        tag: u64,
        payload: u64,
    },
    Bytes {
        sender_pid: u64,
        length: u8,
        data: [u8; bndr_abi::CHANNEL_MESSAGE_MAX_BYTES],
    },
    Transfer {
        sender_pid: u64,
        length: u8,
        data: [u8; bndr_abi::CHANNEL_MESSAGE_MAX_BYTES],
        object: KernelObject,
        rights: Rights,
    },
}

impl Message {
    pub const fn scalar(tag: u64, payload: u64) -> Self {
        Self::Scalar {
            sender_pid: 0,
            tag,
            payload,
        }
    }

    pub fn bytes(data: [u8; bndr_abi::CHANNEL_MESSAGE_MAX_BYTES], length: usize) -> Option<Self> {
        let length = u8::try_from(length).ok()?;
        if usize::from(length) > bndr_abi::CHANNEL_MESSAGE_MAX_BYTES {
            return None;
        }
        Some(Self::Bytes {
            sender_pid: 0,
            length,
            data,
        })
    }

    pub fn transfer(
        payload: TransferPayload,
        object: impl Into<KernelObject>,
        rights: Rights,
    ) -> Self {
        Self::Transfer {
            sender_pid: 0,
            length: payload.length,
            data: payload.data,
            object: object.into(),
            rights,
        }
    }

    /// Attaches kernel-derived sender identity exactly once. Constructors use
    /// sender zero so isolated queue tests can build messages without a
    /// scheduler; zero, `Empty`, and attempts to restamp a message are
    /// rejected.
    pub fn with_sender(mut self, sender_pid: u64) -> Option<Self> {
        if sender_pid == 0 {
            return None;
        }
        let stored_sender = match &mut self {
            Self::Empty => return None,
            Self::Scalar { sender_pid, .. }
            | Self::Bytes { sender_pid, .. }
            | Self::Transfer { sender_pid, .. } => sender_pid,
        };
        if *stored_sender != 0 {
            return None;
        }
        *stored_sender = sender_pid;
        Some(self)
    }

    /// Returns the immutable sender identity attached by the syscall layer.
    /// Zero-valued test messages and `Empty` intentionally report `None`.
    pub const fn sender_pid(&self) -> Option<u64> {
        let sender_pid = match self {
            Self::Empty => return None,
            Self::Scalar { sender_pid, .. }
            | Self::Bytes { sender_pid, .. }
            | Self::Transfer { sender_pid, .. } => *sender_pid,
        };
        if sender_pid == 0 {
            None
        } else {
            Some(sender_pid)
        }
    }

    pub const fn scalar_payload(&self) -> Option<(u64, u64)> {
        match self {
            Self::Scalar { tag, payload, .. } => Some((*tag, *payload)),
            Self::Empty | Self::Bytes { .. } | Self::Transfer { .. } => None,
        }
    }

    /// Returns the queue-visible wire kind and logical payload length without
    /// exposing or moving an owned kernel object. Scalar messages contain one
    /// 64-bit tag and one 64-bit payload; byte and transfer messages report
    /// their actual bounded byte length.
    pub const fn metadata(&self) -> Option<(ChannelMessageKind, usize)> {
        match self {
            Self::Empty => None,
            Self::Scalar { .. } => Some((ChannelMessageKind::Scalar, 16)),
            Self::Bytes { length, .. } => Some((ChannelMessageKind::Bytes, *length as usize)),
            Self::Transfer { length, .. } => Some((ChannelMessageKind::Transfer, *length as usize)),
        }
    }

    pub fn bytes_payload(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes { length, data, .. } => Some(&data[..usize::from(*length)]),
            Self::Empty | Self::Scalar { .. } | Self::Transfer { .. } => None,
        }
    }

    pub fn transfer_payload(&self) -> Option<(&[u8], &KernelObject, Rights)> {
        match self {
            Self::Transfer {
                length,
                data,
                object,
                rights,
                ..
            } => Some((&data[..usize::from(*length)], object, *rights)),
            Self::Empty | Self::Scalar { .. } | Self::Bytes { .. } => None,
        }
    }

    pub fn into_transfer(
        self,
    ) -> Result<
        (
            [u8; bndr_abi::CHANNEL_MESSAGE_MAX_BYTES],
            usize,
            KernelObject,
            Rights,
        ),
        Self,
    > {
        match self {
            Self::Transfer {
                length,
                data,
                object,
                rights,
                ..
            } => Ok((data, usize::from(length), object, rights)),
            message => Err(message),
        }
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (
                Self::Scalar {
                    sender_pid: left_sender,
                    tag: left_tag,
                    payload: left_payload,
                },
                Self::Scalar {
                    sender_pid: right_sender,
                    tag: right_tag,
                    payload: right_payload,
                },
            ) => {
                left_sender == right_sender
                    && left_tag == right_tag
                    && left_payload == right_payload
            }
            (
                Self::Bytes {
                    sender_pid: left_sender,
                    length: left_length,
                    data: left_data,
                },
                Self::Bytes {
                    sender_pid: right_sender,
                    length: right_length,
                    data: right_data,
                },
            ) => {
                left_sender == right_sender
                    && left_length == right_length
                    && left_data == right_data
            }
            (
                Self::Transfer {
                    sender_pid: left_sender,
                    length: left_length,
                    data: left_data,
                    object: left_object,
                    rights: left_rights,
                },
                Self::Transfer {
                    sender_pid: right_sender,
                    length: right_length,
                    data: right_data,
                    object: right_object,
                    rights: right_rights,
                },
            ) => {
                left_sender == right_sender
                    && left_length == right_length
                    && left_data == right_data
                    && left_object.same_object(right_object)
                    && left_rights == right_rights
            }
            _ => false,
        }
    }
}

impl Eq for Message {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelError {
    ShouldWait,
    PeerClosed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelReadFailure<E> {
    Channel(ChannelError),
    Rejected(E),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelWriteFailure<E> {
    Channel(ChannelError),
    Rejected(E),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChannelAllocError;

#[derive(Debug)]
struct MessageQueue {
    entries: [Message; CHANNEL_CAPACITY],
    head: usize,
    len: usize,
}

impl MessageQueue {
    const fn new() -> Self {
        Self {
            entries: [const { Message::Empty }; CHANNEL_CAPACITY],
            head: 0,
            len: 0,
        }
    }

    fn push(&mut self, message: Message) -> Result<(), ChannelError> {
        if self.len == CHANNEL_CAPACITY {
            return Err(ChannelError::ShouldWait);
        }
        let tail = (self.head + self.len) % CHANNEL_CAPACITY;
        self.entries[tail] = message;
        self.len += 1;
        Ok(())
    }

    const fn is_full(&self) -> bool {
        self.len == CHANNEL_CAPACITY
    }

    fn pop(&mut self) -> Option<Message> {
        if self.len == 0 {
            return None;
        }
        let message = mem::replace(&mut self.entries[self.head], Message::Empty);
        self.head = (self.head + 1) % CHANNEL_CAPACITY;
        self.len -= 1;
        Some(message)
    }

    fn push_front(&mut self, message: Message) {
        assert!(
            self.len < CHANNEL_CAPACITY,
            "channel rollback queue was full"
        );
        self.head = (self.head + CHANNEL_CAPACITY - 1) % CHANNEL_CAPACITY;
        if !matches!(self.entries[self.head], Message::Empty) {
            panic!("channel rollback destination was not empty");
        }
        self.entries[self.head] = message;
        self.len += 1;
    }

    fn front(&self) -> Option<&Message> {
        (self.len != 0).then_some(&self.entries[self.head])
    }
}

#[derive(Debug)]
struct ChannelState {
    inbound: [MessageQueue; 2],
    endpoint_references: [usize; 2],
}

impl ChannelState {
    const fn new() -> Self {
        Self {
            inbound: [MessageQueue::new(), MessageQueue::new()],
            endpoint_references: [1, 1],
        }
    }
}

struct ChannelInner {
    id: u64,
    strong: AtomicUsize,
    state: SpinMutex<ChannelState>,
}

/// One reference to one side of a bidirectional, bounded channel.
pub struct ChannelEndpoint {
    inner: NonNull<ChannelInner>,
    side: usize,
}

// The pointee is immutable except through its atomic count and SpinMutex.
// Every endpoint owns one strong count, so the allocation outlives all access.
unsafe impl Send for ChannelEndpoint {}
unsafe impl Sync for ChannelEndpoint {}

impl ChannelEndpoint {
    pub fn pair() -> (Self, Self) {
        Self::try_pair().unwrap_or_else(|_| handle_alloc_error(Layout::new::<ChannelInner>()))
    }

    /// Allocates a channel pair without invoking the global OOM handler.
    pub fn try_pair() -> Result<(Self, Self), ChannelAllocError> {
        Self::try_pair_with(|layout| unsafe { alloc(layout) })
    }

    fn try_pair_with(
        allocate: impl FnOnce(Layout) -> *mut u8,
    ) -> Result<(Self, Self), ChannelAllocError> {
        // IDs may be skipped after allocation failure. They are never reused:
        // zero permanently records exhaustion after u64::MAX is handed out.
        let id = CHANNEL_IDS.allocate()?;
        let layout = Layout::new::<ChannelInner>();
        let inner =
            NonNull::new(allocate(layout).cast::<ChannelInner>()).ok_or(ChannelAllocError)?;
        unsafe {
            inner.as_ptr().write(ChannelInner {
                id,
                strong: AtomicUsize::new(2),
                state: SpinMutex::new(ChannelState::new()),
            });
        }
        Ok((Self { inner, side: 0 }, Self { inner, side: 1 }))
    }

    pub fn write(&self, message: Message) -> Result<(), ChannelError> {
        let peer = self.side ^ 1;
        let mut state = self.inner().state.lock();
        if state.endpoint_references[peer] == 0 {
            return Err(ChannelError::PeerClosed);
        }
        state.inbound[peer].push(message)
    }

    /// Checks peer/queue commitability before running `operation`, then
    /// publishes exactly one message only when that operation succeeds.
    pub fn write_with<E>(
        &self,
        operation: impl FnOnce() -> Result<Message, E>,
    ) -> Result<(), ChannelWriteFailure<E>> {
        let peer = self.side ^ 1;
        let mut state = self.inner().state.lock();
        if state.endpoint_references[peer] == 0 {
            return Err(ChannelWriteFailure::Channel(ChannelError::PeerClosed));
        }
        if state.inbound[peer].is_full() {
            return Err(ChannelWriteFailure::Channel(ChannelError::ShouldWait));
        }
        let message = operation().map_err(ChannelWriteFailure::Rejected)?;
        state.inbound[peer]
            .push(message)
            .unwrap_or_else(|_| panic!("channel queue became full during a locked transaction"));
        Ok(())
    }

    pub fn read(&self) -> Result<Message, ChannelError> {
        let peer = self.side ^ 1;
        let mut state = self.inner().state.lock();
        if let Some(message) = state.inbound[self.side].pop() {
            return Ok(message);
        }
        if state.endpoint_references[peer] == 0 {
            Err(ChannelError::PeerClosed)
        } else {
            Err(ChannelError::ShouldWait)
        }
    }

    /// Samples the queue head without consuming it. A queued message wins over
    /// peer closure, matching `read` and level-triggered READABLE semantics.
    pub fn peek(&self) -> Result<(ChannelMessageKind, usize), ChannelError> {
        let peer = self.side ^ 1;
        let state = self.inner().state.lock();
        if let Some(message) = state.inbound[self.side].front() {
            return Ok(message
                .metadata()
                .unwrap_or_else(|| panic!("channel queue contained an empty message")));
        }
        if state.endpoint_references[peer] == 0 {
            Err(ChannelError::PeerClosed)
        } else {
            Err(ChannelError::ShouldWait)
        }
    }

    /// Runs a non-blocking transaction against the queue head. The message is
    /// removed only when `operation` succeeds; rejection preserves FIFO state.
    pub fn read_with<R, E>(
        &self,
        operation: impl FnOnce(&Message) -> Result<R, E>,
    ) -> Result<R, ChannelReadFailure<E>> {
        let peer = self.side ^ 1;
        let mut state = self.inner().state.lock();
        let Some(message) = state.inbound[self.side].front() else {
            return if state.endpoint_references[peer] == 0 {
                Err(ChannelReadFailure::Channel(ChannelError::PeerClosed))
            } else {
                Err(ChannelReadFailure::Channel(ChannelError::ShouldWait))
            };
        };
        match operation(message) {
            Ok(value) => {
                drop(
                    state.inbound[self.side]
                        .pop()
                        .unwrap_or_else(|| panic!("channel head disappeared during transaction")),
                );
                Ok(value)
            }
            Err(error) => Err(ChannelReadFailure::Rejected(error)),
        }
    }

    /// Removes the queue head for an owning transaction. A rejected operation
    /// must return the same message, which is restored at the head before the
    /// channel lock is released. This lets handle-table insertion fail without
    /// consuming or dropping a transferred kernel object.
    pub fn read_owned_with<R, E>(
        &self,
        operation: impl FnOnce(Message) -> Result<R, (E, Message)>,
    ) -> Result<R, ChannelReadFailure<E>> {
        let peer = self.side ^ 1;
        let mut state = self.inner().state.lock();
        let Some(message) = state.inbound[self.side].pop() else {
            return if state.endpoint_references[peer] == 0 {
                Err(ChannelReadFailure::Channel(ChannelError::PeerClosed))
            } else {
                Err(ChannelReadFailure::Channel(ChannelError::ShouldWait))
            };
        };
        match operation(message) {
            Ok(value) => Ok(value),
            Err((error, message)) => {
                state.inbound[self.side].push_front(message);
                Err(ChannelReadFailure::Rejected(error))
            }
        }
    }

    pub fn queued_messages(&self) -> usize {
        self.inner().state.lock().inbound[self.side].len
    }

    /// Returns the complete level-triggered signal state for this endpoint.
    /// The queue lengths and peer lifetime are sampled under one state lock so
    /// callers never observe a combination assembled across Channel commits.
    pub fn signals(&self) -> ObjectSignals {
        let peer = self.side ^ 1;
        let state = self.inner().state.lock();
        let mut bits = 0;
        if state.inbound[self.side].len != 0 {
            bits |= ObjectSignals::READABLE.bits();
        }
        if state.endpoint_references[peer] == 0 {
            bits |= ObjectSignals::PEER_CLOSED.bits();
        } else if !state.inbound[peer].is_full() {
            bits |= ObjectSignals::WRITABLE.bits();
        }
        ObjectSignals::from_bits(bits)
            .unwrap_or_else(|| panic!("channel produced invalid object signals"))
    }

    pub fn same_channel(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    pub fn same_endpoint(&self, other: &Self) -> bool {
        self.same_channel(other) && self.side == other.side
    }

    pub fn is_peer_of(&self, other: &Self) -> bool {
        self.same_channel(other) && self.side != other.side
    }

    /// Returns whether this transport Channel may own a source Channel in one
    /// of its queued messages. Every permitted ownership edge goes from a
    /// smaller, older ID to a larger, newer ID, so a path can never return to
    /// its starting ID and form a reference cycle.
    pub fn can_own_channel(&self, source: &Self) -> bool {
        self.inner().id < source.inner().id
    }

    fn inner(&self) -> &ChannelInner {
        unsafe { self.inner.as_ref() }
    }
}

impl fmt::Debug for ChannelEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChannelEndpoint")
            .field("inner", &self.inner)
            .field("channel_id", &self.inner().id)
            .field("side", &self.side)
            .finish()
    }
}

impl Clone for ChannelEndpoint {
    fn clone(&self) -> Self {
        let inner = self.inner();
        inner
            .strong
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |strong| {
                strong
                    .checked_add(1)
                    .filter(|next| *next <= isize::MAX as usize)
            })
            .unwrap_or_else(|_| panic!("channel strong-reference overflow"));
        let references = &mut inner.state.lock().endpoint_references[self.side];
        *references = references
            .checked_add(1)
            .unwrap_or_else(|| panic!("channel endpoint reference overflow"));
        Self {
            inner: self.inner,
            side: self.side,
        }
    }
}

impl Drop for ChannelEndpoint {
    fn drop(&mut self) {
        let inner = self.inner();
        let abandoned = {
            let mut state = inner.state.lock();
            let references = &mut state.endpoint_references[self.side];
            assert!(*references != 0, "channel endpoint reference underflow");
            *references -= 1;
            (*references == 0)
                .then(|| mem::replace(&mut state.inbound[self.side], MessageQueue::new()))
        };
        // Dropping an owning transfer may close another Channel and acquire
        // its state lock. Never run that cascade while this Channel is locked.
        drop(abandoned);
        if inner.strong.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                core::ptr::drop_in_place(self.inner.as_ptr());
                dealloc(
                    self.inner.as_ptr().cast::<u8>(),
                    Layout::new::<ChannelInner>(),
                );
            }
        }
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

#[cfg(test)]
mod tests {
    use super::{
        CHANNEL_CAPACITY, ChannelAllocError, ChannelEndpoint, ChannelError, ChannelIdAllocator,
        ChannelReadFailure, ChannelWriteFailure, Message, TransferPayload,
    };
    use crate::event::Event;
    use crate::object::KernelObject;
    use bndr_abi::{ChannelMessageKind, ObjectSignals};

    #[test]
    fn fallible_pair_reports_allocator_exhaustion() {
        assert!(matches!(
            ChannelEndpoint::try_pair_with(|_| core::ptr::null_mut()),
            Err(ChannelAllocError)
        ));
    }

    #[test]
    fn channel_id_exhaustion_is_fallible_and_never_returns_zero() {
        let ids = ChannelIdAllocator::starting_at(u64::MAX);
        assert_eq!(ids.allocate(), Ok(u64::MAX));
        assert_eq!(ids.allocate(), Err(ChannelAllocError));

        let exhausted = ChannelIdAllocator::starting_at(0);
        assert_eq!(exhausted.allocate(), Err(ChannelAllocError));
    }

    #[test]
    fn sends_in_both_directions_and_preserves_fifo() {
        let (left, right) = ChannelEndpoint::pair();
        left.write(Message::scalar(1, 11)).unwrap();
        left.write(Message::scalar(2, 22)).unwrap();
        right.write(Message::scalar(3, 33)).unwrap();
        assert_eq!(right.read().unwrap(), Message::scalar(1, 11));
        assert_eq!(right.read().unwrap(), Message::scalar(2, 22));
        assert_eq!(left.read().unwrap(), Message::scalar(3, 33));
    }

    #[test]
    fn sender_identity_is_nonzero_once_only_and_survives_typed_access() {
        assert_eq!(Message::Empty.sender_pid(), None);
        assert!(Message::Empty.with_sender(1).is_none());
        assert!(Message::scalar(1, 2).with_sender(0).is_none());

        let scalar = Message::scalar(1, 2).with_sender(41).unwrap();
        assert_eq!(scalar.sender_pid(), Some(41));
        assert_eq!(scalar.metadata(), Some((ChannelMessageKind::Scalar, 16)));
        assert_eq!(scalar.scalar_payload(), Some((1, 2)));
        assert_eq!(scalar, Message::scalar(1, 2).with_sender(41).unwrap());
        assert_ne!(scalar, Message::scalar(1, 2).with_sender(42).unwrap());
        assert!(scalar.with_sender(43).is_none());

        let mut data = [0_u8; bndr_abi::CHANNEL_MESSAGE_MAX_BYTES];
        data[..4].copy_from_slice(b"data");
        let bytes = Message::bytes(data, 4).unwrap().with_sender(44).unwrap();
        assert_eq!(bytes.sender_pid(), Some(44));
        assert_eq!(bytes.metadata(), Some((ChannelMessageKind::Bytes, 4)));
        assert_eq!(bytes.bytes_payload(), Some(&b"data"[..]));

        let (transferred, peer) = ChannelEndpoint::pair();
        let transfer = Message::transfer(
            TransferPayload::new(data, 4).unwrap(),
            KernelObject::from(transferred),
            bndr_abi::Rights::READ,
        )
        .with_sender(45)
        .unwrap();
        assert_eq!(transfer.sender_pid(), Some(45));
        assert_eq!(transfer.metadata(), Some((ChannelMessageKind::Transfer, 4)));
        assert_eq!(transfer.transfer_payload().unwrap().0, b"data");
        let original_sender = transfer.sender_pid().unwrap();
        let (data, length, object, rights) = transfer.into_transfer().unwrap();
        let rebuilt =
            Message::transfer(TransferPayload::new(data, length).unwrap(), object, rights)
                .with_sender(original_sender)
                .unwrap();
        assert_eq!(rebuilt.sender_pid(), Some(45));
        assert!(
            rebuilt
                .transfer_payload()
                .unwrap()
                .1
                .as_channel()
                .unwrap()
                .same_channel(&peer)
        );
    }

    #[test]
    fn peek_reports_kind_and_length_without_consuming_fifo() {
        let (sender, receiver) = ChannelEndpoint::pair();
        assert_eq!(receiver.peek(), Err(ChannelError::ShouldWait));

        sender.write(Message::scalar(1, 2)).unwrap();
        let mut data = [0_u8; bndr_abi::CHANNEL_MESSAGE_MAX_BYTES];
        data[..4].copy_from_slice(b"peek");
        sender.write(Message::bytes(data, 4).unwrap()).unwrap();
        let (transferred, peer) = ChannelEndpoint::pair();
        sender
            .write(Message::transfer(
                TransferPayload::new(data, 3).unwrap(),
                KernelObject::from(transferred),
                bndr_abi::Rights::CHANNEL_DEFAULT,
            ))
            .unwrap();

        assert_eq!(receiver.peek(), Ok((ChannelMessageKind::Scalar, 16)));
        assert_eq!(receiver.peek(), Ok((ChannelMessageKind::Scalar, 16)));
        assert_eq!(receiver.queued_messages(), 3);
        assert_eq!(receiver.read().unwrap(), Message::scalar(1, 2));
        assert_eq!(receiver.peek(), Ok((ChannelMessageKind::Bytes, 4)));
        assert_eq!(receiver.queued_messages(), 2);
        assert_eq!(receiver.read().unwrap().bytes_payload().unwrap(), b"peek");
        assert_eq!(receiver.peek(), Ok((ChannelMessageKind::Transfer, 3)));
        let message = receiver.read().unwrap();
        let (_, length, restored, _) = message.into_transfer().unwrap();
        assert_eq!(length, 3);
        assert!(restored.into_channel().unwrap().same_channel(&peer));
        assert_eq!(receiver.peek(), Err(ChannelError::ShouldWait));

        drop(sender);
        assert_eq!(receiver.peek(), Err(ChannelError::PeerClosed));
    }

    #[test]
    fn reports_empty_and_full_without_losing_messages() {
        let (left, right) = ChannelEndpoint::pair();
        assert_eq!(right.read(), Err(ChannelError::ShouldWait));
        for index in 0..CHANNEL_CAPACITY as u64 {
            left.write(Message::scalar(index, index * 10)).unwrap();
        }
        assert_eq!(
            left.write(Message::scalar(99, 99)),
            Err(ChannelError::ShouldWait)
        );
        for index in 0..CHANNEL_CAPACITY as u64 {
            assert_eq!(right.read().unwrap(), Message::scalar(index, index * 10));
        }
    }

    #[test]
    fn new_pair_is_writable_but_not_readable_or_peer_closed() {
        let (left, right) = ChannelEndpoint::pair();
        assert_eq!(left.signals(), ObjectSignals::WRITABLE);
        assert_eq!(right.signals(), ObjectSignals::WRITABLE);
    }

    #[test]
    fn enqueue_makes_the_peer_readable() {
        let (sender, receiver) = ChannelEndpoint::pair();
        sender.write(Message::scalar(1, 2)).unwrap();
        let observed = receiver.signals();
        assert!(observed.contains(ObjectSignals::READABLE));
        assert!(observed.contains(ObjectSignals::WRITABLE));
        assert!(!observed.intersects(ObjectSignals::PEER_CLOSED));
    }

    #[test]
    fn full_queue_clears_sender_writable_and_pop_restores_it() {
        let (sender, receiver) = ChannelEndpoint::pair();
        for index in 0..CHANNEL_CAPACITY as u64 {
            sender.write(Message::scalar(index, index)).unwrap();
        }
        assert!(!sender.signals().intersects(ObjectSignals::WRITABLE));
        assert!(receiver.signals().contains(ObjectSignals::READABLE));

        assert_eq!(receiver.read().unwrap(), Message::scalar(0, 0));
        assert!(sender.signals().contains(ObjectSignals::WRITABLE));
    }

    #[test]
    fn duplicated_endpoints_keep_the_side_alive() {
        let (left, right) = ChannelEndpoint::pair();
        let left_duplicate = left.clone();
        drop(left);
        right.write(Message::scalar(7, 8)).unwrap();
        assert_eq!(left_duplicate.read().unwrap(), Message::scalar(7, 8));
        drop(left_duplicate);
        assert_eq!(
            right.write(Message::scalar(9, 9)),
            Err(ChannelError::PeerClosed)
        );
        assert_eq!(right.read(), Err(ChannelError::PeerClosed));
    }

    #[test]
    fn queued_messages_remain_readable_after_peer_closes() {
        let (left, right) = ChannelEndpoint::pair();
        left.write(Message::scalar(42, 84).with_sender(51).unwrap())
            .unwrap();
        drop(left);
        assert_eq!(right.queued_messages(), 1);
        let message = right.read().unwrap();
        assert_eq!(message.sender_pid(), Some(51));
        assert_eq!(message.scalar_payload(), Some((42, 84)));
        assert_eq!(right.read(), Err(ChannelError::PeerClosed));
    }

    #[test]
    fn queued_message_and_peer_close_are_observed_together() {
        let (sender, receiver) = ChannelEndpoint::pair();
        sender.write(Message::scalar(42, 84)).unwrap();
        drop(sender);

        let observed = receiver.signals();
        assert!(observed.contains(ObjectSignals::READABLE));
        assert!(observed.contains(ObjectSignals::PEER_CLOSED));
        assert!(!observed.intersects(ObjectSignals::WRITABLE));
    }

    #[test]
    fn peer_closed_rises_only_after_the_last_duplicate_closes() {
        let (left, right) = ChannelEndpoint::pair();
        let duplicate = left.clone();
        drop(left);
        assert_eq!(right.signals(), ObjectSignals::WRITABLE);

        drop(duplicate);
        assert_eq!(right.signals(), ObjectSignals::PEER_CLOSED);
    }

    #[test]
    fn rejected_typed_read_preserves_the_fifo_head() {
        let (left, right) = ChannelEndpoint::pair();
        assert_eq!(
            left.write_with(|| Err::<Message, _>(55_u8)),
            Err(ChannelWriteFailure::Rejected(55))
        );
        assert_eq!(right.queued_messages(), 0);
        let mut data = [0_u8; bndr_abi::CHANNEL_MESSAGE_MAX_BYTES];
        data[..4].copy_from_slice(b"test");
        let bytes = Message::bytes(data, 4).unwrap().with_sender(52).unwrap();
        left.write(bytes).unwrap();
        left.write(Message::scalar(9, 10)).unwrap();

        assert_eq!(
            right.read_with(|message| {
                assert_eq!(message.sender_pid(), Some(52));
                Err::<(), _>(77_u8)
            }),
            Err(ChannelReadFailure::Rejected(77))
        );
        assert_eq!(right.queued_messages(), 2);
        assert_eq!(
            right
                .read_with(|message| {
                    assert_eq!(message.sender_pid(), Some(52));
                    message.bytes_payload().ok_or(1_u8).map(<[u8]>::to_vec)
                })
                .unwrap(),
            b"test"
        );
        assert_eq!(right.read().unwrap(), Message::scalar(9, 10));
    }

    #[test]
    fn uncommittable_write_never_runs_its_owning_operation() {
        let (sender, receiver) = ChannelEndpoint::pair();
        for index in 0..CHANNEL_CAPACITY as u64 {
            sender.write(Message::scalar(index, index)).unwrap();
        }
        let mut called = false;
        assert_eq!(
            sender.write_with(|| {
                called = true;
                Ok::<_, ()>(Message::scalar(99, 99))
            }),
            Err(ChannelWriteFailure::Channel(ChannelError::ShouldWait))
        );
        assert!(!called);

        drop(receiver);
        let (sender, receiver) = ChannelEndpoint::pair();
        drop(receiver);
        assert_eq!(
            sender.write_with(|| {
                called = true;
                Ok::<_, ()>(Message::scalar(100, 100))
            }),
            Err(ChannelWriteFailure::Channel(ChannelError::PeerClosed))
        );
        assert!(!called);
    }

    #[test]
    fn rejected_owning_read_restores_the_same_transferred_handle() {
        let (sender, receiver) = ChannelEndpoint::pair();
        let (transferred, peer) = ChannelEndpoint::pair();
        let mut data = [0_u8; bndr_abi::CHANNEL_MESSAGE_MAX_BYTES];
        data[..4].copy_from_slice(b"move");
        sender
            .write(
                Message::transfer(
                    TransferPayload::new(data, 4).unwrap(),
                    KernelObject::from(transferred),
                    bndr_abi::Rights::READ,
                )
                .with_sender(53)
                .unwrap(),
            )
            .unwrap();

        assert_eq!(
            receiver.read_owned_with(|message| {
                assert_eq!(message.sender_pid(), Some(53));
                Err::<(), _>((77_u8, message))
            }),
            Err(ChannelReadFailure::Rejected(77))
        );
        assert_eq!(receiver.queued_messages(), 1);
        let message = receiver.read().unwrap();
        assert_eq!(message.sender_pid(), Some(53));
        let (restored_data, length, restored, rights) = message.into_transfer().unwrap();
        let restored = restored.into_channel().unwrap();
        assert_eq!(&restored_data[..length], b"move");
        assert_eq!(rights, bndr_abi::Rights::READ);
        assert!(restored.same_channel(&peer));
    }

    #[test]
    fn unread_transferred_handle_is_closed_with_the_transport_queue() {
        let (sender, receiver) = ChannelEndpoint::pair();
        let (transferred, peer) = ChannelEndpoint::pair();
        sender
            .write(Message::transfer(
                TransferPayload::new([0; bndr_abi::CHANNEL_MESSAGE_MAX_BYTES], 0).unwrap(),
                KernelObject::from(transferred),
                bndr_abi::Rights::CHANNEL_DEFAULT,
            ))
            .unwrap();
        drop(sender);
        drop(receiver);

        assert_eq!(peer.read(), Err(ChannelError::PeerClosed));
        assert_eq!(
            peer.write(Message::scalar(1, 2)),
            Err(ChannelError::PeerClosed)
        );
    }

    #[test]
    fn unread_transferred_event_is_released_with_the_transport_queue() {
        let (sender, receiver) = ChannelEndpoint::pair();
        let event = Event::new();
        let observer = event.clone();
        assert_eq!(observer.reference_count(), 2);

        sender
            .write(Message::transfer(
                TransferPayload::new([0; bndr_abi::CHANNEL_MESSAGE_MAX_BYTES], 0).unwrap(),
                KernelObject::from(event),
                bndr_abi::Rights::WAIT,
            ))
            .unwrap();
        assert_eq!(observer.reference_count(), 2);

        drop(sender);
        drop(receiver);
        assert_eq!(observer.reference_count(), 1);
        assert!(observer.signal());
        assert_eq!(observer.signals(), ObjectSignals::SIGNALED);
    }

    #[test]
    fn channel_identity_covers_both_sides_but_not_other_channels() {
        let (left, right) = ChannelEndpoint::pair();
        let duplicate = left.clone();
        let (other, _other_peer) = ChannelEndpoint::pair();
        assert!(left.same_channel(&right));
        assert!(left.same_endpoint(&duplicate));
        assert!(!left.same_endpoint(&right));
        assert!(!left.same_channel(&other));
    }

    #[test]
    fn channel_ownership_edges_follow_strict_creation_order() {
        let (older_left, older_right) = ChannelEndpoint::pair();
        let (newer_left, newer_right) = ChannelEndpoint::pair();

        assert!(older_left.can_own_channel(&newer_left));
        assert!(older_right.can_own_channel(&newer_right));
        assert!(!newer_left.can_own_channel(&older_left));
        assert!(!newer_right.can_own_channel(&older_right));
        assert!(!older_left.can_own_channel(&older_right));
        assert!(!newer_left.can_own_channel(&newer_left));
    }
}
