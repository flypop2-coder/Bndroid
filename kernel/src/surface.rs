//! Transactional raster boundary for the fixed userspace UI surface.

use alloc::alloc::{Layout, alloc, dealloc};
use core::{
    fmt,
    ptr::NonNull,
    sync::atomic::{AtomicU32, AtomicUsize, Ordering, fence},
};

use bndr_ui::{
    BufferPresent, DamageRect, InputSample, InputSampleError, PresentFrame, PresentMode,
    ProtocolError, SURFACE_HEIGHT, SURFACE_WIDTH, validate_sequence,
};
#[cfg(feature = "androidbox-interactive0")]
use bndr_ui::{
    MOBILE_CONTENT_VIEWPORT_BOTTOM, MOBILE_CONTENT_VIEWPORT_HEIGHT, MOBILE_CONTENT_VIEWPORT_WIDTH,
    MOBILE_CONTENT_VIEWPORT_X, MOBILE_CONTENT_VIEWPORT_Y,
};

use crate::{
    compositor::Rect,
    event::EventAllocError,
    framebuffer::{PIXEL_COUNT, WIDTH},
};
#[cfg(feature = "androidbox-interactive0")]
use bndr_abi::Rights;
use bndr_abi::{KeyInputSample, KeyInputSampleError, ObjectSignals};

#[cfg(not(feature = "mobile-ui-runtime"))]
pub const SURFACE_ORIGIN_X: usize = 56;
#[cfg(feature = "mobile-ui-runtime")]
pub const SURFACE_ORIGIN_X: usize = 0;
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const SURFACE_ORIGIN_Y: usize = 64;
#[cfg(feature = "mobile-ui-runtime")]
pub const SURFACE_ORIGIN_Y: usize = 0;
pub const SURFACE_INPUT_QUEUE_CAPACITY: usize = 64;
pub const SURFACE_KEY_QUEUE_CAPACITY: usize = 32;

/// A unique, non-duplicable userspace producer capability. The handle-table
/// rights enforce move/duplicate denial; the cloned kernel copy exists only so
/// kernel producers can raise level-triggered READABLE, FRAME_READY, and
/// PEER_CLOSED signals.
#[derive(Clone, Debug)]
pub struct SurfaceCapability {
    session_id: u64,
    signals: SurfaceSignals,
}

impl SurfaceCapability {
    pub fn try_new(session_id: u64) -> Result<Self, EventAllocError> {
        if session_id == 0 {
            return Err(EventAllocError);
        }
        let signals = SurfaceSignals::try_new()?;
        Ok(Self {
            session_id,
            signals,
        })
    }

    pub const fn session_id(&self) -> u64 {
        self.session_id
    }

    pub fn same_surface(&self, other: &Self) -> bool {
        self.session_id == other.session_id && self.signals.same_state(&other.signals)
    }

    pub fn signals(&self) -> ObjectSignals {
        ObjectSignals::from_bits(self.signals.bits())
            .unwrap_or_else(|| panic!("surface capability produced invalid signal bits"))
    }

    pub fn signal_readable(&self) -> bool {
        self.signals.signal(ObjectSignals::READABLE)
    }

    pub fn clear_readable(&self) -> bool {
        self.signals.clear(ObjectSignals::READABLE)
    }

    pub fn signal_key_ready(&self) -> bool {
        self.signals.signal(ObjectSignals::KEY_READY)
    }

    pub fn clear_key_ready(&self) -> bool {
        self.signals.clear(ObjectSignals::KEY_READY)
    }

    pub fn mark_degraded(&self) -> bool {
        self.signals.signal(ObjectSignals::PEER_CLOSED)
    }

    pub fn signal_frame_ready(&self) -> bool {
        self.signals.signal(ObjectSignals::FRAME_READY)
    }

    pub fn clear_frame_ready(&self) -> bool {
        self.signals.clear(ObjectSignals::FRAME_READY)
    }
}

/// One allocation backs every independently level-triggered Surface signal.
/// Keeping the capability to `(session_id, pointer)` also prevents the
/// 32-entry HandleTable from growing when ABI v20 adds FRAME_READY.
struct SurfaceSignals {
    inner: NonNull<SurfaceSignalsInner>,
}

struct SurfaceSignalsInner {
    strong: AtomicUsize,
    bits: AtomicU32,
}

unsafe impl Send for SurfaceSignals {}
unsafe impl Sync for SurfaceSignals {}

impl SurfaceSignals {
    fn try_new() -> Result<Self, EventAllocError> {
        let inner = NonNull::new(
            unsafe { alloc(Layout::new::<SurfaceSignalsInner>()) }.cast::<SurfaceSignalsInner>(),
        )
        .ok_or(EventAllocError)?;
        unsafe {
            inner.as_ptr().write(SurfaceSignalsInner {
                strong: AtomicUsize::new(1),
                bits: AtomicU32::new(0),
            });
        }
        Ok(Self { inner })
    }

    fn signal(&self, signal: ObjectSignals) -> bool {
        debug_assert!(ObjectSignals::SURFACE_ALL.contains(signal));
        self.inner().bits.fetch_or(signal.bits(), Ordering::AcqRel) & signal.bits() == 0
    }

    fn clear(&self, signal: ObjectSignals) -> bool {
        debug_assert!(ObjectSignals::SURFACE_ALL.contains(signal));
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

    fn inner(&self) -> &SurfaceSignalsInner {
        unsafe { self.inner.as_ref() }
    }
}

impl Clone for SurfaceSignals {
    fn clone(&self) -> Self {
        self.inner()
            .strong
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |strong| {
                strong
                    .checked_add(1)
                    .filter(|next| *next <= isize::MAX as usize)
            })
            .unwrap_or_else(|_| panic!("surface signal strong-reference overflow"));
        Self { inner: self.inner }
    }
}

impl Drop for SurfaceSignals {
    fn drop(&mut self) {
        if self.inner().strong.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                core::ptr::drop_in_place(self.inner.as_ptr());
                dealloc(
                    self.inner.as_ptr().cast::<u8>(),
                    Layout::new::<SurfaceSignalsInner>(),
                );
            }
        }
    }
}

impl fmt::Debug for SurfaceSignals {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SurfaceSignals")
            .field("inner", &self.inner)
            .field("bits", &self.bits())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceInputError {
    InvalidSample(InputSampleError),
    SequenceExhausted,
    CounterExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceInputEnqueueEvidence {
    pub sample: InputSample,
    /// True when overload replaced only the newest undelivered tail state.
    /// Its existing sequence is retained, so the delivered prefix remains
    /// gap-free while the most recent physical pressed/released state wins.
    pub coalesced: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SurfaceInputSnapshot {
    pub pending: usize,
    pub next_sequence: u64,
    pub enqueued: u64,
    pub dequeued: u64,
    pub high_watermark: usize,
    pub coalesced: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceInputQueue {
    slots: [Option<InputSample>; SURFACE_INPUT_QUEUE_CAPACITY],
    head: usize,
    len: usize,
    next_sequence: u64,
    enqueued: u64,
    dequeued: u64,
    high_watermark: usize,
    coalesced: u64,
}

impl Default for SurfaceInputQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceInputQueue {
    pub const fn new() -> Self {
        Self {
            slots: [None; SURFACE_INPUT_QUEUE_CAPACITY],
            head: 0,
            len: 0,
            next_sequence: 0,
            enqueued: 0,
            dequeued: 0,
            high_watermark: 0,
            coalesced: 0,
        }
    }

    pub fn push(
        &mut self,
        x: u16,
        y: u16,
        pressed: bool,
    ) -> Result<SurfaceInputEnqueueEvidence, SurfaceInputError> {
        if self.len == SURFACE_INPUT_QUEUE_CAPACITY {
            let tail = (self.head + self.len - 1) % SURFACE_INPUT_QUEUE_CAPACITY;
            let previous = self.slots[tail]
                .unwrap_or_else(|| panic!("full surface input queue tail was empty"));
            let coalesced = self
                .coalesced
                .checked_add(1)
                .ok_or(SurfaceInputError::CounterExhausted)?;
            let sample = InputSample::try_new(previous.sequence(), x, y, pressed)
                .map_err(SurfaceInputError::InvalidSample)?;
            self.slots[tail] = Some(sample);
            self.coalesced = coalesced;
            return Ok(SurfaceInputEnqueueEvidence {
                sample,
                coalesced: true,
            });
        }
        let sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(SurfaceInputError::SequenceExhausted)?;
        let sample = InputSample::try_new(sequence, x, y, pressed)
            .map_err(SurfaceInputError::InvalidSample)?;
        let enqueued = self
            .enqueued
            .checked_add(1)
            .ok_or(SurfaceInputError::CounterExhausted)?;
        let tail = (self.head + self.len) % SURFACE_INPUT_QUEUE_CAPACITY;
        if self.slots[tail].replace(sample).is_some() {
            panic!("surface input queue tail was occupied");
        }
        self.len += 1;
        self.next_sequence = sequence;
        self.enqueued = enqueued;
        self.high_watermark = self.high_watermark.max(self.len);
        Ok(SurfaceInputEnqueueEvidence {
            sample,
            coalesced: false,
        })
    }

    pub fn pop(&mut self) -> Result<Option<InputSample>, SurfaceInputError> {
        if self.len == 0 {
            return Ok(None);
        }
        let dequeued = self
            .dequeued
            .checked_add(1)
            .ok_or(SurfaceInputError::CounterExhausted)?;
        let sample = self.slots[self.head]
            .take()
            .unwrap_or_else(|| panic!("surface input queue head was empty"));
        self.head = (self.head + 1) % SURFACE_INPUT_QUEUE_CAPACITY;
        self.len -= 1;
        self.dequeued = dequeued;
        Ok(Some(sample))
    }

    pub const fn snapshot(&self) -> SurfaceInputSnapshot {
        SurfaceInputSnapshot {
            pending: self.len,
            next_sequence: self.next_sequence,
            enqueued: self.enqueued,
            dequeued: self.dequeued,
            high_watermark: self.high_watermark,
            coalesced: self.coalesced,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceKeyError {
    InvalidSample(KeyInputSampleError),
    Full,
    SequenceExhausted,
    CounterExhausted,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SurfaceKeySnapshot {
    pub pending: usize,
    pub next_sequence: u64,
    pub enqueued: u64,
    pub dequeued: u64,
    pub high_watermark: usize,
    pub overflow_rejections: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceKeyQueue {
    slots: [Option<KeyInputSample>; SURFACE_KEY_QUEUE_CAPACITY],
    head: usize,
    len: usize,
    next_sequence: u64,
    enqueued: u64,
    dequeued: u64,
    high_watermark: usize,
    overflow_rejections: u64,
}

impl Default for SurfaceKeyQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceKeyQueue {
    pub const fn new() -> Self {
        Self {
            slots: [None; SURFACE_KEY_QUEUE_CAPACITY],
            head: 0,
            len: 0,
            next_sequence: 0,
            enqueued: 0,
            dequeued: 0,
            high_watermark: 0,
            overflow_rejections: 0,
        }
    }

    /// Appends exactly one transition. Unlike pointer state, key transitions
    /// are never replaceable or coalesced; a full queue rejects the new sample
    /// while preserving every queued item and its sequence namespace.
    pub fn push(&mut self, code: u16, value: u8) -> Result<KeyInputSample, SurfaceKeyError> {
        let sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(SurfaceKeyError::SequenceExhausted)?;
        let sample = KeyInputSample::try_new(sequence, code, value)
            .map_err(SurfaceKeyError::InvalidSample)?;
        let enqueued = self
            .enqueued
            .checked_add(1)
            .ok_or(SurfaceKeyError::CounterExhausted)?;
        if self.len == SURFACE_KEY_QUEUE_CAPACITY {
            let overflow_rejections = self
                .overflow_rejections
                .checked_add(1)
                .ok_or(SurfaceKeyError::CounterExhausted)?;
            self.overflow_rejections = overflow_rejections;
            return Err(SurfaceKeyError::Full);
        }

        let tail = (self.head + self.len) % SURFACE_KEY_QUEUE_CAPACITY;
        if self.slots[tail].replace(sample).is_some() {
            panic!("surface key queue tail was occupied");
        }
        self.len += 1;
        self.next_sequence = sequence;
        self.enqueued = enqueued;
        self.high_watermark = self.high_watermark.max(self.len);
        Ok(sample)
    }

    pub fn pop(&mut self) -> Result<Option<KeyInputSample>, SurfaceKeyError> {
        if self.len == 0 {
            return Ok(None);
        }
        let dequeued = self
            .dequeued
            .checked_add(1)
            .ok_or(SurfaceKeyError::CounterExhausted)?;
        let sample = self.slots[self.head]
            .take()
            .unwrap_or_else(|| panic!("surface key queue head was empty"));
        self.head = (self.head + 1) % SURFACE_KEY_QUEUE_CAPACITY;
        self.len -= 1;
        self.dequeued = dequeued;
        Ok(Some(sample))
    }

    pub const fn snapshot(&self) -> SurfaceKeySnapshot {
        SurfaceKeySnapshot {
            pending: self.len,
            next_sequence: self.next_sequence,
            enqueued: self.enqueued,
            dequeued: self.dequeued,
            high_watermark: self.high_watermark,
            overflow_rejections: self.overflow_rejections,
        }
    }
}

/// One pure stage of syscall 62's layered-submit validation.
///
/// The stages mirror the order in which facts become available at the syscall
/// boundary: raw handle topology, handle-table rights, then pinned buffer and
/// wire metadata. Keeping the policy here makes every negative path testable
/// without a display, scheduler, handle table, or graphics allocation.
#[cfg(feature = "androidbox-interactive0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayeredPresentPreflight {
    HandleTopology {
        surface_content_and_chrome_are_distinct: bool,
    },
    BufferRights {
        content: Rights,
        chrome: Rights,
    },
    Submission(LayeredPresentSubmissionPreflight),
}

/// Immutable source and generation facts for the final syscall 62 preflight.
#[cfg(feature = "androidbox-interactive0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayeredPresentSubmissionPreflight {
    pub content_and_chrome_are_same_buffer: bool,
    pub content_is_mappable: bool,
    pub chrome_is_mappable: bool,
    pub content_producer_pid: u64,
    pub launcher_pid: Option<u64>,
    pub app_pid: Option<u64>,
    pub chrome_producer_pid: u64,
    pub current_surface_server_pid: u64,
    pub requested_content_generation: u64,
    pub current_content_generation: u64,
    pub requested_chrome_generation: u64,
    pub current_chrome_generation: u64,
}

/// Exact reason a layered submit was rejected before display publication.
#[cfg(feature = "androidbox-interactive0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayeredPresentPreflightError {
    AliasedHandles,
    InvalidContentRights,
    InvalidChromeRights,
    AliasedBuffers,
    MappableContent,
    MappableChrome,
    InvalidContentProducer,
    InvalidChromeProducer,
    ZeroChromeGeneration,
    StaleContentGeneration,
    StaleChromeGeneration,
}

/// Validates one syscall 62 preflight stage without touching kernel state.
///
/// A successful final `Submission` result does not replace the graphics
/// pool's atomic pair borrow. The display path repeats both generation checks
/// while pinning the two pixel slices through composition.
#[cfg(feature = "androidbox-interactive0")]
pub fn validate_layered_present_preflight(
    preflight: LayeredPresentPreflight,
) -> Result<(), LayeredPresentPreflightError> {
    match preflight {
        LayeredPresentPreflight::HandleTopology {
            surface_content_and_chrome_are_distinct,
        } => {
            if !surface_content_and_chrome_are_distinct {
                return Err(LayeredPresentPreflightError::AliasedHandles);
            }
        }
        LayeredPresentPreflight::BufferRights { content, chrome } => {
            if content.bits() != Rights::GRAPHICS_BUFFER_SERVER.bits() {
                return Err(LayeredPresentPreflightError::InvalidContentRights);
            }
            if chrome.bits() != Rights::GRAPHICS_BUFFER_SERVER.bits() {
                return Err(LayeredPresentPreflightError::InvalidChromeRights);
            }
        }
        LayeredPresentPreflight::Submission(submission) => {
            if submission.content_and_chrome_are_same_buffer {
                return Err(LayeredPresentPreflightError::AliasedBuffers);
            }
            if submission.content_is_mappable {
                return Err(LayeredPresentPreflightError::MappableContent);
            }
            if submission.chrome_is_mappable {
                return Err(LayeredPresentPreflightError::MappableChrome);
            }
            if submission.content_producer_pid == 0
                || (submission.launcher_pid != Some(submission.content_producer_pid)
                    && submission.app_pid != Some(submission.content_producer_pid))
            {
                return Err(LayeredPresentPreflightError::InvalidContentProducer);
            }
            if submission.current_surface_server_pid == 0
                || submission.chrome_producer_pid != submission.current_surface_server_pid
            {
                return Err(LayeredPresentPreflightError::InvalidChromeProducer);
            }
            if submission.requested_chrome_generation == 0 {
                return Err(LayeredPresentPreflightError::ZeroChromeGeneration);
            }
            if submission.requested_content_generation != submission.current_content_generation {
                return Err(LayeredPresentPreflightError::StaleContentGeneration);
            }
            if submission.requested_chrome_generation != submission.current_chrome_generation {
                return Err(LayeredPresentPreflightError::StaleChromeGeneration);
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceOwner {
    KernelFallback,
    UserspaceBound,
    Degraded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceError {
    WrongScenePixelCount,
    WrongBufferPixelCount,
    NonCanonicalBufferPixel,
    Protocol(ProtocolError),
    CommitCounterExhausted,
    Degraded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceCommitEvidence {
    pub previous_owner: SurfaceOwner,
    pub current_owner: SurfaceOwner,
    pub frame_id: u32,
    pub mode: PresentMode,
    pub local_damage: DamageRect,
    pub global_damage: Rect,
    pub raster_writes: usize,
    pub commits: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceSession {
    owner: SurfaceOwner,
    last_frame_id: Option<u32>,
    commits: u64,
}

impl Default for SurfaceSession {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceSession {
    pub const fn new() -> Self {
        Self {
            owner: SurfaceOwner::KernelFallback,
            last_frame_id: None,
            commits: 0,
        }
    }

    pub const fn owner(self) -> SurfaceOwner {
        self.owner
    }

    pub const fn last_frame_id(self) -> Option<u32> {
        self.last_frame_id
    }

    pub const fn commits(self) -> u64 {
        self.commits
    }

    /// Freezes the last committed scene after a userspace service failure.
    /// Ownership never falls back implicitly because that would create a
    /// second scene writer and a second frame-generation namespace.
    pub fn mark_degraded(&mut self) {
        self.owner = SurfaceOwner::Degraded;
    }

    /// Validates the complete frame and session sequence before the first
    /// pixel write, then runs an infallible bounded raster and publishes state.
    pub fn present(
        &mut self,
        scene: &mut [u32],
        frame: &PresentFrame,
    ) -> Result<SurfaceCommitEvidence, SurfaceError> {
        if self.owner == SurfaceOwner::Degraded {
            return Err(SurfaceError::Degraded);
        }
        if scene.len() != PIXEL_COUNT {
            return Err(SurfaceError::WrongScenePixelCount);
        }
        validate_sequence(self.last_frame_id, frame).map_err(SurfaceError::Protocol)?;
        let next_commits = self
            .commits
            .checked_add(1)
            .ok_or(SurfaceError::CommitCounterExhausted)?;
        let local_damage = frame.damage_rect();
        let global_damage = global_damage(local_damage);
        let mut raster_writes = 0_usize;

        if frame.mode() == PresentMode::Full {
            fill_rect(scene, DamageRect::FULL, frame.clear_color());
            raster_writes = usize::from(SURFACE_WIDTH) * usize::from(SURFACE_HEIGHT);
        }
        for rect in frame.rects() {
            fill_rect(scene, rect.damage(), rect.color());
            raster_writes = raster_writes
                .checked_add(usize::from(rect.width()) * usize::from(rect.height()))
                .expect("validated fixed surface raster count cannot overflow");
        }

        let previous_owner = self.owner;
        self.owner = SurfaceOwner::UserspaceBound;
        self.last_frame_id = Some(frame.frame_id());
        self.commits = next_commits;
        Ok(SurfaceCommitEvidence {
            previous_owner,
            current_owner: self.owner,
            frame_id: frame.frame_id(),
            mode: frame.mode(),
            local_damage,
            global_damage,
            raster_writes,
            commits: self.commits,
        })
    }

    /// Publishes a fully rastered client buffer into the fixed phone surface.
    ///
    /// M32a deliberately accepts only the canonical full-buffer command. The
    /// complete sequence, counter, source length, and XRGB high bytes are
    /// validated before the first scene pixel changes, preserving the same
    /// all-or-nothing boundary as the legacy solid-rectangle path.
    pub fn present_buffer(
        &mut self,
        scene: &mut [u32],
        frame: &BufferPresent,
        pixels: &[u32],
    ) -> Result<SurfaceCommitEvidence, SurfaceError> {
        if self.owner == SurfaceOwner::Degraded {
            return Err(SurfaceError::Degraded);
        }
        if scene.len() != PIXEL_COUNT {
            return Err(SurfaceError::WrongScenePixelCount);
        }
        let surface_pixels = usize::from(SURFACE_WIDTH) * usize::from(SURFACE_HEIGHT);
        if pixels.len() != surface_pixels {
            return Err(SurfaceError::WrongBufferPixelCount);
        }
        validate_buffer_frame_sequence(self.last_frame_id, frame.global_frame_id())?;
        if pixels.iter().any(|pixel| pixel & 0xff00_0000 != 0) {
            return Err(SurfaceError::NonCanonicalBufferPixel);
        }
        let next_commits = self
            .commits
            .checked_add(1)
            .ok_or(SurfaceError::CommitCounterExhausted)?;

        let surface_width = usize::from(SURFACE_WIDTH);
        for local_y in 0..usize::from(SURFACE_HEIGHT) {
            let source = local_y * surface_width;
            let destination = (SURFACE_ORIGIN_Y + local_y) * WIDTH + SURFACE_ORIGIN_X;
            scene[destination..destination + surface_width]
                .copy_from_slice(&pixels[source..source + surface_width]);
        }

        let previous_owner = self.owner;
        self.owner = SurfaceOwner::UserspaceBound;
        self.last_frame_id = Some(frame.global_frame_id());
        self.commits = next_commits;
        let local_damage = DamageRect::FULL;
        Ok(SurfaceCommitEvidence {
            previous_owner,
            current_owner: self.owner,
            frame_id: frame.global_frame_id(),
            mode: PresentMode::Full,
            local_damage,
            global_damage: global_damage(local_damage),
            raster_writes: surface_pixels,
            commits: self.commits,
        })
    }

    /// Publishes a client-owned content viewport beneath SurfaceServer-owned
    /// top and bottom system chrome.
    ///
    /// Both complete source buffers are validated before the first scene
    /// pixel changes. Only the canonical viewport is ever copied from
    /// `content_pixels`; every pixel outside it comes from `chrome_pixels`.
    #[cfg(feature = "androidbox-interactive0")]
    pub fn present_buffer_layers(
        &mut self,
        scene: &mut [u32],
        frame: &BufferPresent,
        content_pixels: &[u32],
        chrome_pixels: &[u32],
    ) -> Result<SurfaceCommitEvidence, SurfaceError> {
        if self.owner == SurfaceOwner::Degraded {
            return Err(SurfaceError::Degraded);
        }
        if scene.len() != PIXEL_COUNT {
            return Err(SurfaceError::WrongScenePixelCount);
        }
        let surface_width = usize::from(SURFACE_WIDTH);
        let surface_height = usize::from(SURFACE_HEIGHT);
        let surface_pixels = surface_width * surface_height;
        if content_pixels.len() != surface_pixels || chrome_pixels.len() != surface_pixels {
            return Err(SurfaceError::WrongBufferPixelCount);
        }
        if MOBILE_CONTENT_VIEWPORT_X != 0
            || usize::from(MOBILE_CONTENT_VIEWPORT_WIDTH) != surface_width
            || usize::from(MOBILE_CONTENT_VIEWPORT_Y)
                .checked_add(usize::from(MOBILE_CONTENT_VIEWPORT_HEIGHT))
                != Some(usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM))
            || usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM) > surface_height
        {
            return Err(SurfaceError::WrongBufferPixelCount);
        }
        validate_buffer_frame_sequence(self.last_frame_id, frame.global_frame_id())?;
        if content_pixels
            .iter()
            .chain(chrome_pixels.iter())
            .any(|pixel| pixel & 0xff00_0000 != 0)
        {
            return Err(SurfaceError::NonCanonicalBufferPixel);
        }
        let next_commits = self
            .commits
            .checked_add(1)
            .ok_or(SurfaceError::CommitCounterExhausted)?;

        let content_top = usize::from(MOBILE_CONTENT_VIEWPORT_Y);
        let content_bottom = usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM);
        for local_y in 0..surface_height {
            let source = local_y * surface_width;
            let destination = (SURFACE_ORIGIN_Y + local_y) * WIDTH + SURFACE_ORIGIN_X;
            let source_pixels = if (content_top..content_bottom).contains(&local_y) {
                content_pixels
            } else {
                chrome_pixels
            };
            scene[destination..destination + surface_width]
                .copy_from_slice(&source_pixels[source..source + surface_width]);
        }

        let previous_owner = self.owner;
        self.owner = SurfaceOwner::UserspaceBound;
        self.last_frame_id = Some(frame.global_frame_id());
        self.commits = next_commits;
        let local_damage = DamageRect::FULL;
        Ok(SurfaceCommitEvidence {
            previous_owner,
            current_owner: self.owner,
            frame_id: frame.global_frame_id(),
            mode: PresentMode::Full,
            local_damage,
            global_damage: global_damage(local_damage),
            raster_writes: surface_pixels,
            commits: self.commits,
        })
    }
}

fn validate_buffer_frame_sequence(
    previous_frame_id: Option<u32>,
    frame_id: u32,
) -> Result<(), SurfaceError> {
    let Some(previous) = previous_frame_id else {
        return if frame_id == 1 {
            Ok(())
        } else {
            Err(SurfaceError::Protocol(ProtocolError::FirstFrameIdMustBeOne))
        };
    };
    let expected = previous
        .checked_add(1)
        .ok_or(SurfaceError::Protocol(ProtocolError::FrameIdExhausted))?;
    if frame_id < expected {
        return Err(SurfaceError::Protocol(ProtocolError::FrameReplay));
    }
    if frame_id > expected {
        return Err(SurfaceError::Protocol(ProtocolError::FrameGap));
    }
    Ok(())
}

fn global_damage(local: DamageRect) -> Rect {
    Rect::new(
        SURFACE_ORIGIN_X + usize::from(local.x),
        SURFACE_ORIGIN_Y + usize::from(local.y),
        usize::from(local.width),
        usize::from(local.height),
    )
}

fn fill_rect(scene: &mut [u32], rect: DamageRect, color: u32) {
    let global = global_damage(rect);
    for y in global.y..global.y + global.height {
        let start = y * WIDTH + global.x;
        scene[start..start + global.width].fill(color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "mobile-ui-runtime"))]
    use crate::framebuffer::pixel_digest;
    use crate::framebuffer::{
        COLOR_CARD_CYAN, COLOR_CARD_GREEN, COLOR_CARD_PURPLE, COLOR_PHONE_HEADER,
        COLOR_PHONE_SCREEN, render_boot_splash,
    };
    use bndr_ui::SolidRect;
    use std::vec;

    fn rect(x: u8, y: u16, width: u8, height: u16, color: u32) -> SolidRect {
        SolidRect::try_new(x, y, width, height, color).unwrap()
    }

    fn home_frame() -> PresentFrame {
        PresentFrame::full(
            1,
            COLOR_PHONE_SCREEN,
            &[
                rect(0, 0, 208, 48, COLOR_PHONE_HEADER),
                rect(16, 68, 176, 72, COLOR_CARD_CYAN),
                rect(16, 156, 176, 72, COLOR_CARD_PURPLE),
                rect(16, 244, 176, 72, COLOR_CARD_GREEN),
            ],
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-interactive0")]
    fn valid_layered_submission_preflight() -> LayeredPresentSubmissionPreflight {
        LayeredPresentSubmissionPreflight {
            content_and_chrome_are_same_buffer: false,
            content_is_mappable: false,
            chrome_is_mappable: false,
            content_producer_pid: 41,
            launcher_pid: Some(41),
            app_pid: Some(42),
            chrome_producer_pid: 43,
            current_surface_server_pid: 43,
            requested_content_generation: 7,
            current_content_generation: 7,
            requested_chrome_generation: 9,
            current_chrome_generation: 9,
        }
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn layered_preflight_requires_distinct_handles_and_exact_server_rights() {
        assert_eq!(
            validate_layered_present_preflight(LayeredPresentPreflight::HandleTopology {
                surface_content_and_chrome_are_distinct: false,
            }),
            Err(LayeredPresentPreflightError::AliasedHandles)
        );
        assert_eq!(
            validate_layered_present_preflight(LayeredPresentPreflight::HandleTopology {
                surface_content_and_chrome_are_distinct: true,
            }),
            Ok(())
        );

        for (content, chrome, expected) in [
            (
                Rights::READ,
                Rights::GRAPHICS_BUFFER_SERVER,
                LayeredPresentPreflightError::InvalidContentRights,
            ),
            (
                Rights::GRAPHICS_BUFFER_DEFAULT,
                Rights::GRAPHICS_BUFFER_SERVER,
                LayeredPresentPreflightError::InvalidContentRights,
            ),
            (
                Rights::GRAPHICS_BUFFER_SERVER,
                Rights::READ,
                LayeredPresentPreflightError::InvalidChromeRights,
            ),
            (
                Rights::GRAPHICS_BUFFER_SERVER,
                Rights::GRAPHICS_BUFFER_MAPPED_SERVER,
                LayeredPresentPreflightError::InvalidChromeRights,
            ),
        ] {
            assert_eq!(
                validate_layered_present_preflight(LayeredPresentPreflight::BufferRights {
                    content,
                    chrome,
                }),
                Err(expected)
            );
        }
        assert_eq!(
            validate_layered_present_preflight(LayeredPresentPreflight::BufferRights {
                content: Rights::GRAPHICS_BUFFER_SERVER,
                chrome: Rights::GRAPHICS_BUFFER_SERVER,
            }),
            Ok(())
        );
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn layered_preflight_rejects_alias_mapping_and_wrong_producer_roles() {
        let valid = valid_layered_submission_preflight();
        for (submission, expected) in [
            (
                LayeredPresentSubmissionPreflight {
                    content_and_chrome_are_same_buffer: true,
                    ..valid
                },
                LayeredPresentPreflightError::AliasedBuffers,
            ),
            (
                LayeredPresentSubmissionPreflight {
                    content_is_mappable: true,
                    ..valid
                },
                LayeredPresentPreflightError::MappableContent,
            ),
            (
                LayeredPresentSubmissionPreflight {
                    chrome_is_mappable: true,
                    ..valid
                },
                LayeredPresentPreflightError::MappableChrome,
            ),
            (
                LayeredPresentSubmissionPreflight {
                    content_producer_pid: 44,
                    ..valid
                },
                LayeredPresentPreflightError::InvalidContentProducer,
            ),
            (
                LayeredPresentSubmissionPreflight {
                    content_producer_pid: 0,
                    launcher_pid: Some(0),
                    ..valid
                },
                LayeredPresentPreflightError::InvalidContentProducer,
            ),
            (
                LayeredPresentSubmissionPreflight {
                    chrome_producer_pid: 44,
                    ..valid
                },
                LayeredPresentPreflightError::InvalidChromeProducer,
            ),
            (
                LayeredPresentSubmissionPreflight {
                    current_surface_server_pid: 0,
                    chrome_producer_pid: 0,
                    ..valid
                },
                LayeredPresentPreflightError::InvalidChromeProducer,
            ),
        ] {
            assert_eq!(
                validate_layered_present_preflight(LayeredPresentPreflight::Submission(submission)),
                Err(expected)
            );
        }

        assert_eq!(
            validate_layered_present_preflight(LayeredPresentPreflight::Submission(valid)),
            Ok(())
        );
        assert_eq!(
            validate_layered_present_preflight(LayeredPresentPreflight::Submission(
                LayeredPresentSubmissionPreflight {
                    content_producer_pid: valid.app_pid.unwrap(),
                    ..valid
                }
            )),
            Ok(())
        );
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn layered_preflight_rejects_zero_and_both_stale_generations() {
        let valid = valid_layered_submission_preflight();
        for (submission, expected) in [
            (
                LayeredPresentSubmissionPreflight {
                    requested_chrome_generation: 0,
                    ..valid
                },
                LayeredPresentPreflightError::ZeroChromeGeneration,
            ),
            (
                LayeredPresentSubmissionPreflight {
                    requested_content_generation: valid.current_content_generation - 1,
                    ..valid
                },
                LayeredPresentPreflightError::StaleContentGeneration,
            ),
            (
                LayeredPresentSubmissionPreflight {
                    requested_chrome_generation: valid.current_chrome_generation - 1,
                    ..valid
                },
                LayeredPresentPreflightError::StaleChromeGeneration,
            ),
        ] {
            assert_eq!(
                validate_layered_present_preflight(LayeredPresentPreflight::Submission(submission)),
                Err(expected)
            );
        }
    }

    #[test]
    fn buffer_present_copies_every_real_pixel_into_the_fixed_surface() {
        let mut scene = vec![0x0001_0203; PIXEL_COUNT];
        let mut pixels = vec![0_u32; usize::from(SURFACE_WIDTH) * usize::from(SURFACE_HEIGHT)];
        for (index, pixel) in pixels.iter_mut().enumerate() {
            *pixel = (index as u32).wrapping_mul(0x0001_0101) & 0x00ff_ffff;
        }
        let frame = BufferPresent::client(1, 1, 7).unwrap();
        let mut session = SurfaceSession::new();
        let evidence = session.present_buffer(&mut scene, &frame, &pixels).unwrap();

        assert_eq!(evidence.previous_owner, SurfaceOwner::KernelFallback);
        assert_eq!(evidence.current_owner, SurfaceOwner::UserspaceBound);
        assert_eq!(evidence.frame_id, 1);
        assert_eq!(evidence.mode, PresentMode::Full);
        assert_eq!(evidence.local_damage, DamageRect::FULL);
        assert_eq!(evidence.raster_writes, pixels.len());
        assert_eq!(evidence.commits, 1);
        for y in 0..usize::from(SURFACE_HEIGHT) {
            let source = y * usize::from(SURFACE_WIDTH);
            let destination = (SURFACE_ORIGIN_Y + y) * WIDTH + SURFACE_ORIGIN_X;
            assert_eq!(
                &scene[destination..destination + usize::from(SURFACE_WIDTH)],
                &pixels[source..source + usize::from(SURFACE_WIDTH)]
            );
        }
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!(scene[0], 0x0001_0203);
        #[cfg(feature = "mobile-ui-runtime")]
        assert_eq!(scene, pixels);
    }

    #[test]
    fn buffer_present_rejects_bad_source_and_sequence_before_any_scene_write() {
        let original = vec![0x0004_0506; PIXEL_COUNT];
        let mut scene = original.clone();
        let canonical = vec![0x0012_3456; usize::from(SURFACE_WIDTH) * usize::from(SURFACE_HEIGHT)];
        let mut noncanonical = canonical.clone();
        noncanonical[canonical.len() / 2] = 0xff12_3456;
        let mut session = SurfaceSession::new();

        assert_eq!(
            session.present_buffer(
                &mut scene,
                &BufferPresent::client(2, 1, 1).unwrap(),
                &canonical,
            ),
            Err(SurfaceError::Protocol(ProtocolError::FirstFrameIdMustBeOne))
        );
        assert_eq!(
            session.present_buffer(
                &mut scene,
                &BufferPresent::client(1, 1, 1).unwrap(),
                &canonical[..canonical.len() - 1],
            ),
            Err(SurfaceError::WrongBufferPixelCount)
        );
        assert_eq!(
            session.present_buffer(
                &mut scene,
                &BufferPresent::client(1, 1, 1).unwrap(),
                &noncanonical,
            ),
            Err(SurfaceError::NonCanonicalBufferPixel)
        );
        assert_eq!(scene, original);
        assert_eq!(session.owner(), SurfaceOwner::KernelFallback);
        assert_eq!(session.last_frame_id(), None);
        assert_eq!(session.commits(), 0);
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn layered_present_copies_only_the_canonical_owner_for_each_row() {
        let surface_width = usize::from(SURFACE_WIDTH);
        let surface_height = usize::from(SURFACE_HEIGHT);
        let mut scene = vec![0x0001_0203; PIXEL_COUNT];
        let content = vec![0x0011_2233; surface_width * surface_height];
        let chrome = vec![0x0044_5566; surface_width * surface_height];
        let frame = BufferPresent::client(1, 1, 7)
            .unwrap()
            .with_system_chrome_generation(9)
            .unwrap();
        let mut session = SurfaceSession::new();
        let evidence = session
            .present_buffer_layers(&mut scene, &frame, &content, &chrome)
            .unwrap();

        assert_eq!(evidence.raster_writes, surface_width * surface_height);
        assert_eq!(evidence.frame_id, 1);
        for y in 0..surface_height {
            let expected = if (usize::from(MOBILE_CONTENT_VIEWPORT_Y)
                ..usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM))
                .contains(&y)
            {
                0x0011_2233
            } else {
                0x0044_5566
            };
            let row = &scene[y * WIDTH..y * WIDTH + surface_width];
            assert!(row.iter().all(|pixel| *pixel == expected));
        }
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn layered_present_validates_both_complete_sources_before_mutation() {
        let surface_pixels = usize::from(SURFACE_WIDTH) * usize::from(SURFACE_HEIGHT);
        let original = vec![0x000a_0b0c; PIXEL_COUNT];
        let canonical = vec![0x0011_2233; surface_pixels];
        let frame = BufferPresent::client(1, 1, 7)
            .unwrap()
            .with_system_chrome_generation(9)
            .unwrap();
        for corrupt_content in [true, false] {
            let mut content = canonical.clone();
            let mut chrome = canonical.clone();
            if corrupt_content {
                // Ignored out-of-viewport bytes are still part of the
                // validated client source transaction.
                content[0] = 0xff11_2233;
            } else {
                // Chrome cannot hide malformed bytes beneath content either.
                chrome[usize::from(MOBILE_CONTENT_VIEWPORT_Y) * usize::from(SURFACE_WIDTH)] =
                    0xff11_2233;
            }
            let mut scene = original.clone();
            let mut session = SurfaceSession::new();
            assert_eq!(
                session.present_buffer_layers(&mut scene, &frame, &content, &chrome),
                Err(SurfaceError::NonCanonicalBufferPixel)
            );
            assert_eq!(scene, original);
            assert_eq!(session, SurfaceSession::new());
        }
    }

    #[test]
    fn buffer_present_uses_the_global_not_client_frame_sequence() {
        let mut scene = vec![0; PIXEL_COUNT];
        let pixels = vec![0x000a_0b0c; usize::from(SURFACE_WIDTH) * usize::from(SURFACE_HEIGHT)];
        let mut session = SurfaceSession::new();
        session
            .present_buffer(
                &mut scene,
                &BufferPresent::try_new(1, 1, 1, 1).unwrap(),
                &pixels,
            )
            .unwrap();
        session
            .present_buffer(
                &mut scene,
                &BufferPresent::try_new(1, 2, 1, 2).unwrap(),
                &pixels,
            )
            .unwrap();
        let before = scene.clone();
        assert_eq!(
            session.present_buffer(
                &mut scene,
                &BufferPresent::try_new(2, 4, 1, 3).unwrap(),
                &pixels,
            ),
            Err(SurfaceError::Protocol(ProtocolError::FrameGap))
        );
        assert_eq!(scene, before);
        assert_eq!(session.last_frame_id(), Some(2));
        assert_eq!(session.commits(), 2);
    }

    #[cfg(not(feature = "mobile-ui-runtime"))]
    #[test]
    fn first_full_frame_hands_off_once_and_reproduces_the_m27_home_scene() {
        let mut expected = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut expected).unwrap();
        let mut scene = expected.clone();
        for y in SURFACE_ORIGIN_Y..SURFACE_ORIGIN_Y + usize::from(SURFACE_HEIGHT) {
            let start = y * WIDTH + SURFACE_ORIGIN_X;
            scene[start..start + usize::from(SURFACE_WIDTH)].fill(0x00ff_00ff);
        }
        let mut session = SurfaceSession::new();
        let evidence = session.present(&mut scene, &home_frame()).unwrap();
        assert_eq!(evidence.previous_owner, SurfaceOwner::KernelFallback);
        assert_eq!(evidence.current_owner, SurfaceOwner::UserspaceBound);
        assert_eq!(evidence.frame_id, 1);
        assert_eq!(evidence.mode, PresentMode::Full);
        assert_eq!(evidence.local_damage, DamageRect::FULL);
        assert_eq!(evidence.global_damage, Rect::new(56, 64, 208, 368));
        assert_eq!(evidence.raster_writes, 76_544 + 48 * 208 + 3 * 72 * 176);
        assert_eq!(session.last_frame_id(), Some(1));
        assert_eq!(session.commits(), 1);
        assert_eq!(scene, expected);
        assert_eq!(pixel_digest(&scene), 0x6ef9_c2b7_d15f_de25);
    }

    #[test]
    fn ordered_damage_rects_commit_after_exact_generation_validation() {
        let mut scene = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut scene).unwrap();
        let mut session = SurfaceSession::new();
        session.present(&mut scene, &home_frame()).unwrap();
        let frame = PresentFrame::damage(
            2,
            &[
                rect(10, 20, 30, 40, 0x0011_2233),
                rect(20, 30, 30, 40, 0x0044_5566),
            ],
        )
        .unwrap();
        let evidence = session.present(&mut scene, &frame).unwrap();
        assert_eq!(evidence.previous_owner, SurfaceOwner::UserspaceBound);
        assert_eq!(evidence.current_owner, SurfaceOwner::UserspaceBound);
        assert_eq!(
            evidence.local_damage,
            DamageRect {
                x: 10,
                y: 20,
                width: 40,
                height: 50
            }
        );
        assert_eq!(
            evidence.global_damage,
            Rect::new(SURFACE_ORIGIN_X + 10, SURFACE_ORIGIN_Y + 20, 40, 50)
        );
        assert_eq!(evidence.raster_writes, 2 * 30 * 40);
        assert_eq!(
            scene[(SURFACE_ORIGIN_Y + 35) * WIDTH + SURFACE_ORIGIN_X + 25],
            0x0044_5566
        );
        assert_eq!(
            scene[(SURFACE_ORIGIN_Y + 25) * WIDTH + SURFACE_ORIGIN_X + 15],
            0x0011_2233
        );
        assert_eq!(session.last_frame_id(), Some(2));
    }

    #[test]
    fn failed_first_frame_validation_is_all_or_none() {
        let mut scene = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut scene).unwrap();
        let before = scene.clone();
        let mut session = SurfaceSession::new();
        let damage = PresentFrame::damage(1, &[rect(0, 0, 1, 1, 0)]).unwrap();
        assert_eq!(
            session.present(&mut scene, &damage),
            Err(SurfaceError::Protocol(ProtocolError::FirstFrameMustBeFull))
        );
        assert_eq!(scene, before);
        assert_eq!(session, SurfaceSession::new());
    }

    #[test]
    fn replay_gap_and_wrong_scene_length_leave_state_and_pixels_unchanged() {
        let mut scene = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut scene).unwrap();
        let mut session = SurfaceSession::new();
        session.present(&mut scene, &home_frame()).unwrap();
        let committed_scene = scene.clone();
        let committed_session = session;
        for frame in [
            PresentFrame::damage(1, &[rect(0, 0, 1, 1, 0)]).unwrap(),
            PresentFrame::damage(3, &[rect(0, 0, 1, 1, 0)]).unwrap(),
        ] {
            assert!(matches!(
                session.present(&mut scene, &frame),
                Err(SurfaceError::Protocol(
                    ProtocolError::FrameReplay | ProtocolError::FrameGap
                ))
            ));
            assert_eq!(scene, committed_scene);
            assert_eq!(session, committed_session);
        }
        let mut short = vec![0_u32; PIXEL_COUNT - 1];
        assert_eq!(
            session.present(
                &mut short,
                &PresentFrame::damage(2, &[rect(0, 0, 1, 1, 0)]).unwrap()
            ),
            Err(SurfaceError::WrongScenePixelCount)
        );
        assert_eq!(session, committed_session);
    }

    #[test]
    fn exhausted_counters_and_degraded_owner_never_touch_the_scene() {
        let mut scene = vec![0x0012_3456_u32; PIXEL_COUNT];
        let before = scene.clone();
        let next = PresentFrame::damage(2, &[rect(0, 0, 1, 1, 0)]).unwrap();
        let mut exhausted_commits = SurfaceSession {
            owner: SurfaceOwner::UserspaceBound,
            last_frame_id: Some(1),
            commits: u64::MAX,
        };
        assert_eq!(
            exhausted_commits.present(&mut scene, &next),
            Err(SurfaceError::CommitCounterExhausted)
        );
        assert_eq!(scene, before);
        let buffer_pixels =
            vec![0x000a_0b0c; usize::from(SURFACE_WIDTH) * usize::from(SURFACE_HEIGHT)];
        let exhausted_buffer_state = exhausted_commits;
        assert_eq!(
            exhausted_commits.present_buffer(
                &mut scene,
                &BufferPresent::try_new(1, 2, 1, 1).unwrap(),
                &buffer_pixels,
            ),
            Err(SurfaceError::CommitCounterExhausted)
        );
        assert_eq!(scene, before);
        assert_eq!(exhausted_commits, exhausted_buffer_state);

        let mut exhausted_frames = SurfaceSession {
            owner: SurfaceOwner::UserspaceBound,
            last_frame_id: Some(u32::MAX),
            commits: 7,
        };
        assert_eq!(
            exhausted_frames.present(
                &mut scene,
                &PresentFrame::damage(u32::MAX, &[rect(0, 0, 1, 1, 0)]).unwrap()
            ),
            Err(SurfaceError::Protocol(ProtocolError::FrameIdExhausted))
        );
        assert_eq!(scene, before);
        let exhausted_buffer_state = exhausted_frames;
        assert_eq!(
            exhausted_frames.present_buffer(
                &mut scene,
                &BufferPresent::try_new(1, u32::MAX, 1, 1).unwrap(),
                &buffer_pixels,
            ),
            Err(SurfaceError::Protocol(ProtocolError::FrameIdExhausted))
        );
        assert_eq!(scene, before);
        assert_eq!(exhausted_frames, exhausted_buffer_state);

        let mut degraded = SurfaceSession::new();
        degraded.mark_degraded();
        assert_eq!(degraded.owner(), SurfaceOwner::Degraded);
        assert_eq!(
            degraded.present(&mut scene, &home_frame()),
            Err(SurfaceError::Degraded)
        );
        assert_eq!(scene, before);
        let degraded_state = degraded;
        assert_eq!(
            degraded.present_buffer(
                &mut scene,
                &BufferPresent::client(1, 1, 1).unwrap(),
                &buffer_pixels,
            ),
            Err(SurfaceError::Degraded)
        );
        assert_eq!(scene, before);
        assert_eq!(degraded, degraded_state);
    }

    #[test]
    fn surface_capability_clones_share_level_triggered_signals() {
        let capability = SurfaceCapability::try_new(1).unwrap();
        let clone = capability.clone();
        let distinct = SurfaceCapability::try_new(2).unwrap();
        assert!(capability.same_surface(&clone));
        assert!(!capability.same_surface(&distinct));
        assert_eq!(capability.signals(), ObjectSignals::NONE);
        assert!(clone.signal_frame_ready());
        assert_eq!(capability.signals(), ObjectSignals::FRAME_READY);
        assert!(clone.signal_readable());
        assert!(clone.signal_key_ready());
        assert_eq!(
            capability.signals(),
            ObjectSignals::from_bits(
                ObjectSignals::READABLE.bits()
                    | ObjectSignals::FRAME_READY.bits()
                    | ObjectSignals::KEY_READY.bits()
            )
            .unwrap()
        );
        assert!(capability.clear_readable());
        assert_eq!(
            capability.signals(),
            ObjectSignals::from_bits(
                ObjectSignals::FRAME_READY.bits() | ObjectSignals::KEY_READY.bits()
            )
            .unwrap()
        );
        assert!(clone.clear_key_ready());
        assert!(clone.mark_degraded());
        assert!(clone.clear_frame_ready());
        assert_eq!(capability.signals(), ObjectSignals::PEER_CLOSED);
        assert!(distinct.signal_frame_ready());
        assert_eq!(distinct.signals(), ObjectSignals::FRAME_READY);
        assert!(!capability.same_surface(&distinct));
        assert!(distinct.clear_frame_ready());
        assert_eq!(distinct.signals(), ObjectSignals::NONE);
    }

    #[test]
    fn surface_input_queue_is_fifo_and_sequences_reports_exactly_once() {
        let mut queue = SurfaceInputQueue::new();
        let first = queue.push(12, 342, false).unwrap();
        let second = queue.push(160, 342, true).unwrap();
        assert!(!first.coalesced);
        assert!(!second.coalesced);
        assert_eq!(first.sample.sequence(), 1);
        assert_eq!(second.sample.sequence(), 2);
        assert_eq!(queue.snapshot().pending, 2);
        assert_eq!(queue.snapshot().high_watermark, 2);
        assert_eq!(queue.pop(), Ok(Some(first.sample)));
        assert_eq!(queue.pop(), Ok(Some(second.sample)));
        assert_eq!(queue.pop(), Ok(None));
        assert_eq!(queue.snapshot().enqueued, 2);
        assert_eq!(queue.snapshot().dequeued, 2);
        assert_eq!(queue.snapshot().coalesced, 0);
    }

    #[test]
    fn full_input_queue_coalesces_only_the_tail_and_preserves_sequence_continuity() {
        let mut queue = SurfaceInputQueue::new();
        for index in 0..SURFACE_INPUT_QUEUE_CAPACITY {
            queue.push(index as u16, 0, false).unwrap();
        }
        for expected_sequence in 1..=5 {
            assert_eq!(queue.pop().unwrap().unwrap().sequence(), expected_sequence);
        }
        for index in 0..5 {
            queue.push((100 + index) as u16, 200, false).unwrap();
        }
        let pressed = queue.push(200, 300, true).unwrap();
        assert!(pressed.coalesced);
        assert_eq!(pressed.sample.sequence(), 69);
        assert!(pressed.sample.pressed());
        let released = queue.push(201, 301, false).unwrap();
        assert!(released.coalesced);
        assert_eq!(released.sample.sequence(), 69);
        assert!(!released.sample.pressed());

        let snapshot = queue.snapshot();
        assert_eq!(snapshot.pending, SURFACE_INPUT_QUEUE_CAPACITY);
        assert_eq!(snapshot.next_sequence, 69);
        assert_eq!(snapshot.enqueued, 69);
        assert_eq!(snapshot.dequeued, 5);
        assert_eq!(snapshot.high_watermark, SURFACE_INPUT_QUEUE_CAPACITY);
        assert_eq!(snapshot.coalesced, 2);
        for expected_sequence in 6..=69 {
            let sample = queue.pop().unwrap().unwrap();
            assert_eq!(sample.sequence(), expected_sequence);
            if expected_sequence == 69 {
                assert_eq!(
                    (sample.x(), sample.y(), sample.pressed()),
                    (201, 301, false)
                );
            }
        }
        assert_eq!(queue.pop(), Ok(None));
    }

    #[test]
    fn exhausted_input_counters_leave_fifo_state_unchanged() {
        let mut coalesce_exhausted = SurfaceInputQueue::new();
        for index in 0..SURFACE_INPUT_QUEUE_CAPACITY {
            coalesce_exhausted.push(index as u16, 0, false).unwrap();
        }
        coalesce_exhausted.coalesced = u64::MAX;
        let before = coalesce_exhausted;
        assert_eq!(
            coalesce_exhausted.push(0, 0, true),
            Err(SurfaceInputError::CounterExhausted)
        );
        assert_eq!(coalesce_exhausted, before);

        let mut exhausted = SurfaceInputQueue {
            next_sequence: u64::MAX,
            ..SurfaceInputQueue::new()
        };
        let before = exhausted;
        assert_eq!(
            exhausted.push(0, 0, false),
            Err(SurfaceInputError::SequenceExhausted)
        );
        assert_eq!(exhausted, before);
    }

    #[test]
    fn surface_key_queue_is_strict_fifo_without_coalescing() {
        let mut queue = SurfaceKeyQueue::new();
        let first = queue.push(30, KeyInputSample::VALUE_PRESS).unwrap();
        let second = queue.push(30, KeyInputSample::VALUE_REPEAT).unwrap();
        let third = queue.push(30, KeyInputSample::VALUE_RELEASE).unwrap();
        assert_eq!(
            (first.sequence(), second.sequence(), third.sequence()),
            (1, 2, 3)
        );
        assert_eq!(queue.snapshot().pending, 3);
        assert_eq!(queue.pop(), Ok(Some(first)));
        assert_eq!(queue.pop(), Ok(Some(second)));
        assert_eq!(queue.pop(), Ok(Some(third)));
        assert_eq!(queue.pop(), Ok(None));
        assert_eq!(
            queue.snapshot(),
            SurfaceKeySnapshot {
                pending: 0,
                next_sequence: 3,
                enqueued: 3,
                dequeued: 3,
                high_watermark: 3,
                overflow_rejections: 0,
            }
        );
    }

    #[test]
    fn full_key_queue_rejects_without_changing_fifo_or_sequence() {
        let mut queue = SurfaceKeyQueue::new();
        for index in 0..SURFACE_KEY_QUEUE_CAPACITY {
            queue
                .push(
                    30 + index as u16,
                    if index & 1 == 0 {
                        KeyInputSample::VALUE_PRESS
                    } else {
                        KeyInputSample::VALUE_RELEASE
                    },
                )
                .unwrap();
        }
        let before_slots = queue.slots;
        assert_eq!(
            queue.push(100, KeyInputSample::VALUE_PRESS),
            Err(SurfaceKeyError::Full)
        );
        let snapshot = queue.snapshot();
        assert_eq!(queue.slots, before_slots);
        assert_eq!(snapshot.pending, SURFACE_KEY_QUEUE_CAPACITY);
        assert_eq!(snapshot.next_sequence, SURFACE_KEY_QUEUE_CAPACITY as u64);
        assert_eq!(snapshot.enqueued, SURFACE_KEY_QUEUE_CAPACITY as u64);
        assert_eq!(snapshot.overflow_rejections, 1);
        for expected in 0..SURFACE_KEY_QUEUE_CAPACITY {
            let sample = queue.pop().unwrap().unwrap();
            assert_eq!(sample.code(), 30 + expected as u16);
            assert_eq!(sample.sequence(), expected as u64 + 1);
        }
    }

    #[test]
    fn invalid_and_exhausted_key_pushes_are_transactional() {
        let mut queue = SurfaceKeyQueue::new();
        for (code, value, error) in [
            (
                0,
                KeyInputSample::VALUE_PRESS,
                SurfaceKeyError::InvalidSample(KeyInputSampleError::InvalidCode),
            ),
            (
                30,
                3,
                SurfaceKeyError::InvalidSample(KeyInputSampleError::InvalidValue),
            ),
        ] {
            let before = queue;
            assert_eq!(queue.push(code, value), Err(error));
            assert_eq!(queue, before);
        }

        let mut sequence_exhausted = SurfaceKeyQueue {
            next_sequence: u64::MAX,
            ..SurfaceKeyQueue::new()
        };
        let before = sequence_exhausted;
        assert_eq!(
            sequence_exhausted.push(30, KeyInputSample::VALUE_PRESS),
            Err(SurfaceKeyError::SequenceExhausted)
        );
        assert_eq!(sequence_exhausted, before);

        let mut enqueue_exhausted = SurfaceKeyQueue {
            enqueued: u64::MAX,
            ..SurfaceKeyQueue::new()
        };
        let before = enqueue_exhausted;
        assert_eq!(
            enqueue_exhausted.push(30, KeyInputSample::VALUE_PRESS),
            Err(SurfaceKeyError::CounterExhausted)
        );
        assert_eq!(enqueue_exhausted, before);

        let mut dequeue_exhausted = SurfaceKeyQueue::new();
        dequeue_exhausted
            .push(30, KeyInputSample::VALUE_PRESS)
            .unwrap();
        dequeue_exhausted.dequeued = u64::MAX;
        let before = dequeue_exhausted;
        assert_eq!(
            dequeue_exhausted.pop(),
            Err(SurfaceKeyError::CounterExhausted)
        );
        assert_eq!(dequeue_exhausted, before);
    }
}
