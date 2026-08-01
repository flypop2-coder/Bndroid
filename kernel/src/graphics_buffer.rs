//! Fixed-capacity, generation-qualified userspace graphics buffers.
//!
//! The pool is intentionally static: the first buffer-backed UI path must not
//! make a 300 KiB allocation depend on the small kernel heap.  Every access is
//! serialized by one IRQ-masking, non-reentrant critical section so a
//! generation check and the corresponding pixel access are one atomic kernel
//! operation.

use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};

use bndr_abi::ObjectSignals;

/// Width of the userspace surface in XRGB8888 pixels.
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const GRAPHICS_BUFFER_WIDTH: usize = 208;
#[cfg(feature = "mobile-ui-runtime")]
pub const GRAPHICS_BUFFER_WIDTH: usize = 720;
/// Height of the userspace surface in XRGB8888 pixels.
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const GRAPHICS_BUFFER_HEIGHT: usize = 368;
#[cfg(feature = "mobile-ui-runtime")]
pub const GRAPHICS_BUFFER_HEIGHT: usize = 1_600;
/// Number of logical XRGB8888 pixels in one buffer.
pub const GRAPHICS_BUFFER_PIXEL_COUNT: usize = GRAPHICS_BUFFER_WIDTH * GRAPHICS_BUFFER_HEIGHT;
/// Bytes exposed through the graphics-buffer API.
pub const GRAPHICS_BUFFER_LOGICAL_BYTES: usize = GRAPHICS_BUFFER_PIXEL_COUNT * size_of::<u32>();
/// Page-rounded bytes reserved for each static backing slot.
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const GRAPHICS_BUFFER_BACKING_BYTES: usize = 75 * 4096;
#[cfg(feature = "mobile-ui-runtime")]
pub const GRAPHICS_BUFFER_BACKING_BYTES: usize = 1_125 * 4096;
/// Number of simultaneously live graphics-buffer identities.
///
/// Interactive-0 adds one SurfaceServer-owned system-chrome backing while the
/// two predecessor slots remain the Launcher/App content buffers.
#[cfg(feature = "androidbox-interactive0")]
pub const GRAPHICS_BUFFER_SLOT_COUNT: usize = 3;
#[cfg(not(feature = "androidbox-interactive0"))]
pub const GRAPHICS_BUFFER_SLOT_COUNT: usize = 2;

const BACKING_ALIGNMENT: usize = 4096;

#[cfg(not(feature = "mobile-ui-runtime"))]
const _: () = assert!(GRAPHICS_BUFFER_LOGICAL_BYTES == 306_176);
#[cfg(not(feature = "mobile-ui-runtime"))]
const _: () = assert!(GRAPHICS_BUFFER_BACKING_BYTES == 307_200);
#[cfg(feature = "mobile-ui-runtime")]
const _: () = assert!(GRAPHICS_BUFFER_LOGICAL_BYTES == 4_608_000);
#[cfg(feature = "mobile-ui-runtime")]
const _: () = assert!(GRAPHICS_BUFFER_BACKING_BYTES == 4_608_000);
const _: () = assert!(GRAPHICS_BUFFER_BACKING_BYTES.is_multiple_of(BACKING_ALIGNMENT));
const _: () = assert!(GRAPHICS_BUFFER_WIDTH == bndr_abi::GRAPHICS_BUFFER_WIDTH as usize);
const _: () = assert!(GRAPHICS_BUFFER_HEIGHT == bndr_abi::GRAPHICS_BUFFER_HEIGHT as usize);
const _: () = assert!(GRAPHICS_BUFFER_PIXEL_COUNT == bndr_abi::GRAPHICS_BUFFER_PIXEL_COUNT);
const _: () = assert!(GRAPHICS_BUFFER_LOGICAL_BYTES == bndr_abi::GRAPHICS_BUFFER_LOGICAL_BYTES);
const _: () = assert!(GRAPHICS_BUFFER_BACKING_BYTES == bndr_abi::GRAPHICS_BUFFER_BACKING_BYTES);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphicsBufferError {
    InvalidProducer,
    Exhausted,
    EmptyWrite,
    Unaligned,
    OutOfRange,
    NotMappable,
    MappableWriteDenied,
    InvalidQueueState,
    InvalidConsumer,
    NonCanonicalPixel,
    NonZeroPadding,
    WriteGenerationExhausted,
    StaleGeneration,
}

impl GraphicsBufferError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidProducer => "graphics buffer producer PID is invalid",
            Self::Exhausted => "graphics buffer pool is exhausted",
            Self::EmptyWrite => "graphics buffer write is empty",
            Self::Unaligned => "graphics buffer write is not pixel aligned",
            Self::OutOfRange => "graphics buffer write is outside the logical pixels",
            Self::NotMappable => "graphics buffer does not support shared mapping",
            Self::MappableWriteDenied => {
                "mapped graphics buffer cannot use the copy-write operation"
            }
            Self::InvalidQueueState => "graphics buffer queue state is invalid",
            Self::InvalidConsumer => "graphics buffer consumer PID is invalid or mismatched",
            Self::NonCanonicalPixel => "mapped graphics buffer contains non-canonical XRGB8888",
            Self::NonZeroPadding => "mapped graphics buffer rounded padding is not zero",
            Self::WriteGenerationExhausted => "graphics buffer write generation is exhausted",
            Self::StaleGeneration => "graphics buffer generation is stale",
        }
    }
}

/// Stable identity of one live allocation of a backing slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphicsBufferIdentity {
    slot: u8,
    generation: u64,
}

impl GraphicsBufferIdentity {
    /// Zero-based backing-slot index.
    pub const fn slot(self) -> u8 {
        self.slot
    }

    /// Non-zero allocation generation for the slot.
    pub const fn generation(self) -> u64 {
        self.generation
    }
}

/// One frame that must be abandoned when the sole mapped consumer dies.
///
/// The value is generation-qualified and records whether SurfaceServer had
/// already acquired the frame or died while it was still queued.  Process
/// teardown uses this as a preflight token: it restores the producer's page
/// permissions before committing the state transition back to `Writable`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsumerOwnerDeathFrame {
    generation: u64,
    acquired: bool,
}

impl ConsumerOwnerDeathFrame {
    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn was_acquired(self) -> bool {
        self.acquired
    }
}

/// Read-only queue/access state exported by [`pool_snapshot`].
///
/// `Vacant` is deliberately distinct from the internal legacy reset state so
/// callers cannot mistake an unallocated, scrubbed slot for a live copy-backed
/// buffer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphicsBufferAccessSnapshot {
    Vacant,
    Legacy,
    Writable,
    Queued { generation: u64 },
    Acquired { generation: u64, consumer_pid: u64 },
}

/// Immutable telemetry for one fixed graphics-buffer backing slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphicsBufferSlotSnapshot {
    /// Zero-based position in the static pool.
    pub slot: u8,
    /// Current live allocation generation, or the generation reserved for the
    /// next allocation when the slot is vacant.
    pub generation: u64,
    pub occupied: bool,
    /// Number of owning [`GraphicsBuffer`] values pinning this allocation.
    pub strong_references: usize,
    /// Generation-qualified producer process, or zero for a vacant slot.
    pub producer_pid: u64,
    /// Last published copy/queue generation, or zero for a vacant/fresh slot.
    pub write_generation: u64,
    pub access: GraphicsBufferAccessSnapshot,
    /// Whether every byte of the complete page-rounded backing is zero.
    pub backing_all_zero: bool,
}

/// Fixed-size, read-only view of both static graphics-buffer slots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphicsBufferPoolSnapshot {
    pub slots: [GraphicsBufferSlotSnapshot; GRAPHICS_BUFFER_SLOT_COUNT],
}

/// Transactional queue/acquire/release telemetry for one allocation epoch.
///
/// An epoch begins when a successful allocation claims the first slot of a
/// completely vacant pool. Retiring the last slot deliberately preserves the
/// completed epoch for inspection; the next first allocation resets every
/// transition counter while advancing [`Self::epoch`]. Acquisition order uses
/// one bit per success (`0` for slot zero, `1` for slot one), least-significant
/// bit first. The exact first 64 acquisitions remain available for strict
/// deterministic runtime validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphicsBufferPoolTelemetrySnapshot {
    pub epoch: u64,
    pub writable: usize,
    pub queued: usize,
    pub acquired: usize,
    pub peak_queued: usize,
    pub peak_acquired: usize,
    pub peak_in_flight: usize,
    pub dual_in_flight_publications: u64,
    pub selective_releases: u64,
    pub queue_successes: [u64; GRAPHICS_BUFFER_SLOT_COUNT],
    pub acquire_successes: [u64; GRAPHICS_BUFFER_SLOT_COUNT],
    pub release_successes: [u64; GRAPHICS_BUFFER_SLOT_COUNT],
    pub acquisition_order_bits: u64,
    pub acquisition_order_count: u64,
    pub acquisition_slot_switches: u64,
    pub first_acquisition_slot: Option<u8>,
    pub last_acquisition_slot: Option<u8>,
    pub acquisition_order_overflowed: bool,
    pub counter_overflowed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BufferAccess {
    Legacy,
    Writable,
    Queued { generation: u64 },
    Acquired { generation: u64, consumer_pid: u64 },
}

impl BufferAccess {
    const fn is_mappable(self) -> bool {
        !matches!(self, Self::Legacy)
    }
}

#[derive(Clone, Copy)]
struct SlotState {
    generation: u64,
    producer_pid: u64,
    write_generation: u64,
    references: usize,
    occupied: bool,
    access: BufferAccess,
}

impl SlotState {
    const fn vacant() -> Self {
        Self {
            generation: 1,
            producer_pid: 0,
            write_generation: 0,
            references: 0,
            occupied: false,
            access: BufferAccess::Legacy,
        }
    }
}

#[derive(Clone, Copy)]
struct PoolTelemetry {
    epoch: u64,
    peak_queued: usize,
    peak_acquired: usize,
    peak_in_flight: usize,
    dual_in_flight_publications: u64,
    selective_releases: u64,
    queue_successes: [u64; GRAPHICS_BUFFER_SLOT_COUNT],
    acquire_successes: [u64; GRAPHICS_BUFFER_SLOT_COUNT],
    release_successes: [u64; GRAPHICS_BUFFER_SLOT_COUNT],
    acquisition_order_bits: u64,
    acquisition_order_count: u64,
    acquisition_slot_switches: u64,
    first_acquisition_slot: Option<u8>,
    last_acquisition_slot: Option<u8>,
    acquisition_order_overflowed: bool,
    counter_overflowed: bool,
}

impl PoolTelemetry {
    const fn new() -> Self {
        Self {
            epoch: 0,
            peak_queued: 0,
            peak_acquired: 0,
            peak_in_flight: 0,
            dual_in_flight_publications: 0,
            selective_releases: 0,
            queue_successes: [0; GRAPHICS_BUFFER_SLOT_COUNT],
            acquire_successes: [0; GRAPHICS_BUFFER_SLOT_COUNT],
            release_successes: [0; GRAPHICS_BUFFER_SLOT_COUNT],
            acquisition_order_bits: 0,
            acquisition_order_count: 0,
            acquisition_slot_switches: 0,
            first_acquisition_slot: None,
            last_acquisition_slot: None,
            acquisition_order_overflowed: false,
            counter_overflowed: false,
        }
    }

    fn begin_epoch(&mut self) {
        let epoch = self
            .epoch
            .checked_add(1)
            .filter(|epoch| *epoch != 0)
            .unwrap_or_else(|| panic!("graphics-buffer telemetry epoch exhausted"));
        *self = Self {
            epoch,
            ..Self::new()
        };
    }

    fn record_queue(&mut self, slot: usize, slots: &[SlotState; GRAPHICS_BUFFER_SLOT_COUNT]) {
        increment_counter(
            &mut self.queue_successes[slot],
            &mut self.counter_overflowed,
        );
        let queued = count_access(slots, |access| {
            matches!(access, BufferAccess::Queued { .. })
        })
        .checked_add(1)
        .unwrap_or_else(|| panic!("graphics-buffer queued count overflowed"));
        let acquired = count_access(slots, |access| {
            matches!(access, BufferAccess::Acquired { .. })
        });
        let in_flight = queued
            .checked_add(acquired)
            .unwrap_or_else(|| panic!("graphics-buffer in-flight count overflowed"));
        self.peak_queued = self.peak_queued.max(queued);
        self.peak_acquired = self.peak_acquired.max(acquired);
        self.peak_in_flight = self.peak_in_flight.max(in_flight);
        if slots.iter().enumerate().any(|(other, state)| {
            other != slot && state.occupied && access_is_in_flight(state.access)
        }) {
            increment_counter(
                &mut self.dual_in_flight_publications,
                &mut self.counter_overflowed,
            );
        }
    }

    fn record_acquire(&mut self, slot: usize, slots: &[SlotState; GRAPHICS_BUFFER_SLOT_COUNT]) {
        increment_counter(
            &mut self.acquire_successes[slot],
            &mut self.counter_overflowed,
        );
        let acquired = count_access(slots, |access| {
            matches!(access, BufferAccess::Acquired { .. })
        })
        .checked_add(1)
        .unwrap_or_else(|| panic!("graphics-buffer acquired count overflowed"));
        let queued = count_access(slots, |access| {
            matches!(access, BufferAccess::Queued { .. })
        })
        .checked_sub(1)
        .unwrap_or_else(|| panic!("graphics-buffer acquire lacked queued state"));
        let in_flight = queued
            .checked_add(acquired)
            .unwrap_or_else(|| panic!("graphics-buffer in-flight count overflowed"));
        self.peak_queued = self.peak_queued.max(queued);
        self.peak_acquired = self.peak_acquired.max(acquired);
        self.peak_in_flight = self.peak_in_flight.max(in_flight);

        let slot = u8::try_from(slot)
            .unwrap_or_else(|_| panic!("graphics-buffer acquisition slot exceeded u8"));
        if self.first_acquisition_slot.is_none() {
            self.first_acquisition_slot = Some(slot);
        }
        if self
            .last_acquisition_slot
            .is_some_and(|previous| previous != slot)
        {
            increment_counter(
                &mut self.acquisition_slot_switches,
                &mut self.counter_overflowed,
            );
        }
        self.last_acquisition_slot = Some(slot);
        if self.acquisition_order_count < u64::BITS.into() {
            if slot == 1 {
                self.acquisition_order_bits |= 1_u64 << self.acquisition_order_count;
            }
        } else {
            self.acquisition_order_overflowed = true;
        }
        increment_counter(
            &mut self.acquisition_order_count,
            &mut self.counter_overflowed,
        );
    }

    fn record_release(&mut self, slot: usize, slots: &[SlotState; GRAPHICS_BUFFER_SLOT_COUNT]) {
        increment_counter(
            &mut self.release_successes[slot],
            &mut self.counter_overflowed,
        );
        if slots.iter().enumerate().any(|(other, state)| {
            other != slot && state.occupied && access_is_in_flight(state.access)
        }) {
            increment_counter(&mut self.selective_releases, &mut self.counter_overflowed);
        }
    }
}

struct PoolState {
    slots: [SlotState; GRAPHICS_BUFFER_SLOT_COUNT],
    telemetry: PoolTelemetry,
}

struct PoolStateStorage(UnsafeCell<PoolState>);

// All access is held behind `PoolBorrowGuard`.
unsafe impl Sync for PoolStateStorage {}

#[repr(align(4096))]
struct AlignedBacking(UnsafeCell<[u8; GRAPHICS_BUFFER_BACKING_BYTES]>);

const _: () = assert!(size_of::<AlignedBacking>() == GRAPHICS_BUFFER_BACKING_BYTES);
const _: () = assert!(align_of::<AlignedBacking>() == BACKING_ALIGNMENT);

impl AlignedBacking {
    const fn zeroed() -> Self {
        Self(UnsafeCell::new([0; GRAPHICS_BUFFER_BACKING_BYTES]))
    }
}

// All reads and writes are held behind `PoolBorrowGuard`; alignment makes the
// logical prefix valid for an XRGB8888 `u32` view.
unsafe impl Sync for AlignedBacking {}

static POOL_STATE: PoolStateStorage = PoolStateStorage(UnsafeCell::new(PoolState {
    slots: [const { SlotState::vacant() }; GRAPHICS_BUFFER_SLOT_COUNT],
    telemetry: PoolTelemetry::new(),
}));
static BACKINGS: [AlignedBacking; GRAPHICS_BUFFER_SLOT_COUNT] =
    [const { AlignedBacking::zeroed() }; GRAPHICS_BUFFER_SLOT_COUNT];
static POOL_BORROWED: AtomicBool = AtomicBool::new(false);

/// Captures allocation, queue, reference, and full-backing scrub state for
/// both static slots under one pool critical section.
pub fn pool_snapshot() -> GraphicsBufferPoolSnapshot {
    with_pool(|slots, _| GraphicsBufferPoolSnapshot {
        slots: core::array::from_fn(|slot| {
            let state = slots[slot];
            let access = if !state.occupied {
                GraphicsBufferAccessSnapshot::Vacant
            } else {
                match state.access {
                    BufferAccess::Legacy => GraphicsBufferAccessSnapshot::Legacy,
                    BufferAccess::Writable => GraphicsBufferAccessSnapshot::Writable,
                    BufferAccess::Queued { generation } => {
                        GraphicsBufferAccessSnapshot::Queued { generation }
                    }
                    BufferAccess::Acquired {
                        generation,
                        consumer_pid,
                    } => GraphicsBufferAccessSnapshot::Acquired {
                        generation,
                        consumer_pid,
                    },
                }
            };
            GraphicsBufferSlotSnapshot {
                slot: u8::try_from(slot)
                    .unwrap_or_else(|_| panic!("graphics-buffer slot index exceeded u8")),
                generation: state.generation,
                occupied: state.occupied,
                strong_references: state.references,
                producer_pid: state.producer_pid,
                write_generation: state.write_generation,
                access,
                backing_all_zero: backing(slot).iter().all(|byte| *byte == 0),
            }
        }),
    })
}

/// Captures current two-slot ownership plus the complete allocation-epoch
/// transition ledger under the same pool critical section.
pub fn pool_telemetry_snapshot() -> GraphicsBufferPoolTelemetrySnapshot {
    with_pool(|slots, telemetry| GraphicsBufferPoolTelemetrySnapshot {
        epoch: telemetry.epoch,
        writable: count_access(slots, |access| access == BufferAccess::Writable),
        queued: count_access(slots, |access| {
            matches!(access, BufferAccess::Queued { .. })
        }),
        acquired: count_access(slots, |access| {
            matches!(access, BufferAccess::Acquired { .. })
        }),
        peak_queued: telemetry.peak_queued,
        peak_acquired: telemetry.peak_acquired,
        peak_in_flight: telemetry.peak_in_flight,
        dual_in_flight_publications: telemetry.dual_in_flight_publications,
        selective_releases: telemetry.selective_releases,
        queue_successes: telemetry.queue_successes,
        acquire_successes: telemetry.acquire_successes,
        release_successes: telemetry.release_successes,
        acquisition_order_bits: telemetry.acquisition_order_bits,
        acquisition_order_count: telemetry.acquisition_order_count,
        acquisition_slot_switches: telemetry.acquisition_slot_switches,
        first_acquisition_slot: telemetry.first_acquisition_slot,
        last_acquisition_slot: telemetry.last_acquisition_slot,
        acquisition_order_overflowed: telemetry.acquisition_order_overflowed,
        counter_overflowed: telemetry.counter_overflowed,
    })
}

/// An owning reference to a static XRGB8888 graphics buffer.
///
/// Clones retain the exact `(slot, generation)` allocation.  A slot cannot be
/// recycled until the last clone is dropped, at which point its complete
/// page-rounded backing is scrubbed before a new generation is published.
pub struct GraphicsBuffer {
    identity: GraphicsBufferIdentity,
    producer_pid: u64,
}

// The state is globally synchronized and an owning reference pins its slot.
unsafe impl Send for GraphicsBuffer {}
unsafe impl Sync for GraphicsBuffer {}

impl GraphicsBuffer {
    pub const LOGICAL_BYTES: usize = GRAPHICS_BUFFER_LOGICAL_BYTES;
    pub const BACKING_BYTES: usize = GRAPHICS_BUFFER_BACKING_BYTES;
    pub const PIXEL_COUNT: usize = GRAPHICS_BUFFER_PIXEL_COUNT;

    /// Claims one of the fixed static slots for a non-zero producer PID.
    ///
    /// Legacy buffers remain syscall-copy-backed and do not expose queue
    /// signals. ABI-19 shared mappings use [`Self::try_new_mappable`] instead.
    pub fn try_new(producer_pid: u64) -> Result<Self, GraphicsBufferError> {
        Self::try_new_with_access(producer_pid, BufferAccess::Legacy)
    }

    /// Claims a page-aligned shared-mapping buffer in its initial writable
    /// state.
    ///
    /// The complete page-rounded backing is stable for the lifetime of every
    /// clone. Queue publication is the only operation that advances its
    /// generation; the legacy copy-write entry point is deliberately denied.
    pub fn try_new_mappable(producer_pid: u64) -> Result<Self, GraphicsBufferError> {
        Self::try_new_with_access(producer_pid, BufferAccess::Writable)
    }

    fn try_new_with_access(
        producer_pid: u64,
        access: BufferAccess,
    ) -> Result<Self, GraphicsBufferError> {
        if producer_pid == 0 {
            return Err(GraphicsBufferError::InvalidProducer);
        }
        with_pool(|slots, telemetry| {
            let pool_was_vacant = slots.iter().all(|state| !state.occupied);
            let (slot_index, state) = slots
                .iter_mut()
                .enumerate()
                .find(|(_, state)| !state.occupied)
                .ok_or(GraphicsBufferError::Exhausted)?;
            if state.generation == 0
                || state.producer_pid != 0
                || state.write_generation != 0
                || state.references != 0
                || state.access != BufferAccess::Legacy
            {
                panic!("vacant graphics-buffer slot retained live state");
            }
            if pool_was_vacant {
                telemetry.begin_epoch();
            }
            state.producer_pid = producer_pid;
            state.references = 1;
            state.occupied = true;
            state.access = access;
            let slot = u8::try_from(slot_index)
                .unwrap_or_else(|_| panic!("graphics-buffer slot index exceeded u8"));
            Ok(Self {
                identity: GraphicsBufferIdentity {
                    slot,
                    generation: state.generation,
                },
                producer_pid,
            })
        })
    }

    pub const fn identity(&self) -> GraphicsBufferIdentity {
        self.identity
    }

    pub const fn slot(&self) -> u8 {
        self.identity.slot
    }

    pub const fn slot_generation(&self) -> u64 {
        self.identity.generation
    }

    pub const fn producer_pid(&self) -> u64 {
        self.producer_pid
    }

    /// Whether this allocation exposes its complete page-rounded backing to
    /// the shared-mapping ABI.
    pub fn is_mappable(&self) -> bool {
        with_pool(|slots, _| self.live_state(slots).access.is_mappable())
    }

    /// Stable page-aligned address of all [`GRAPHICS_BUFFER_BACKING_BYTES`].
    ///
    /// Calling this on a legacy copy-backed allocation is a kernel logic
    /// error. An owning `GraphicsBuffer` reference pins both the slot identity
    /// and this backing address until the mapping owner releases it.
    pub fn backing_address(&self) -> usize {
        with_pool(|slots, _| {
            let state = self.live_state(slots);
            if !state.access.is_mappable() {
                panic!("legacy graphics buffer does not expose a mapping address");
            }
            backing(usize::from(self.identity.slot)).as_ptr() as usize
        })
    }

    /// Signal classes accepted by object waits on this buffer.
    ///
    /// Shared mappings use `WRITABLE` as the producer release fence and
    /// `READABLE` as the queued consumer acquire fence. Legacy copy-backed
    /// buffers intentionally remain non-waitable.
    pub fn signal_mask(&self) -> ObjectSignals {
        with_pool(|slots, _| {
            if self.live_state(slots).access.is_mappable() {
                mappable_signal_mask()
            } else {
                ObjectSignals::NONE
            }
        })
    }

    /// Current level-triggered queue/fence signals.
    pub fn signals(&self) -> ObjectSignals {
        with_pool(|slots, _| match self.live_state(slots).access {
            BufferAccess::Writable => ObjectSignals::WRITABLE,
            BufferAccess::Queued { .. } => ObjectSignals::READABLE,
            BufferAccess::Legacy | BufferAccess::Acquired { .. } => ObjectSignals::NONE,
        })
    }

    /// Returns the generation produced by the most recent successful write.
    pub fn write_generation(&self) -> u64 {
        with_pool(|slots, _| self.live_state(slots).write_generation)
    }

    /// Copies one aligned byte range and publishes exactly one new generation.
    ///
    /// Every input pixel is canonicalized to XRGB8888 by clearing its high
    /// byte.  Validation and generation exhaustion are handled before the
    /// first backing byte changes, so every error is transactionally inert.
    pub fn write(&self, offset: usize, bytes: &[u8]) -> Result<u64, GraphicsBufferError> {
        if bytes.is_empty() {
            return Err(GraphicsBufferError::EmptyWrite);
        }
        if !offset.is_multiple_of(size_of::<u32>()) || !bytes.len().is_multiple_of(size_of::<u32>())
        {
            return Err(GraphicsBufferError::Unaligned);
        }
        let end = offset
            .checked_add(bytes.len())
            .filter(|end| *end <= GRAPHICS_BUFFER_LOGICAL_BYTES)
            .ok_or(GraphicsBufferError::OutOfRange)?;

        with_pool(|slots, _| {
            let state = self.live_state_mut(slots);
            if state.access.is_mappable() {
                return Err(GraphicsBufferError::MappableWriteDenied);
            }
            let next_generation = state
                .write_generation
                .checked_add(1)
                .ok_or(GraphicsBufferError::WriteGenerationExhausted)?;
            let backing = backing_mut(usize::from(self.identity.slot));
            for (source, destination) in bytes
                .chunks_exact(size_of::<u32>())
                .zip(backing[offset..end].chunks_exact_mut(size_of::<u32>()))
            {
                let pixel = u32::from_le_bytes(
                    source
                        .try_into()
                        .unwrap_or_else(|_| panic!("four-byte pixel chunk changed length")),
                ) & 0x00ff_ffff;
                destination.copy_from_slice(&pixel.to_le_bytes());
            }
            state.write_generation = next_generation;
            Ok(next_generation)
        })
    }

    /// Publishes the producer's current shared backing as one queued frame.
    ///
    /// `expected_generation` must name the last successfully released frame
    /// (zero for a fresh allocation). The complete logical XRGB8888 prefix
    /// and the page-rounded padding are validated before the generation or
    /// signal state changes. A failed queue is therefore transactionally
    /// inert and remains writable for correction and retry.
    pub fn queue(&self, expected_generation: u64) -> Result<u64, GraphicsBufferError> {
        with_pool(|slots, telemetry| {
            let slot = usize::from(self.identity.slot);
            let state = self.live_state_mut(slots);
            match state.access {
                BufferAccess::Legacy => return Err(GraphicsBufferError::NotMappable),
                BufferAccess::Writable => {}
                BufferAccess::Queued { .. } | BufferAccess::Acquired { .. } => {
                    return Err(GraphicsBufferError::InvalidQueueState);
                }
            }
            if state.write_generation != expected_generation {
                return Err(GraphicsBufferError::StaleGeneration);
            }
            let next_generation = state
                .write_generation
                .checked_add(1)
                .filter(|generation| *generation != 0)
                .ok_or(GraphicsBufferError::WriteGenerationExhausted)?;
            validate_mappable_backing(usize::from(self.identity.slot))?;
            telemetry.record_queue(slot, slots);
            let state = self.live_state_mut(slots);
            state.write_generation = next_generation;
            state.access = BufferAccess::Queued {
                generation: next_generation,
            };
            Ok(next_generation)
        })
    }

    /// Acquires one exact queued generation for a generation-qualified
    /// SurfaceServer process.
    ///
    /// The transition clears `READABLE`; no producer release fence becomes
    /// visible until the same consumer explicitly releases the frame after a
    /// successful present or intentional discard.
    pub fn acquire(
        &self,
        expected_generation: u64,
        consumer_pid: u64,
    ) -> Result<u64, GraphicsBufferError> {
        if consumer_pid == 0 {
            return Err(GraphicsBufferError::InvalidConsumer);
        }
        with_pool(|slots, telemetry| {
            let slot = usize::from(self.identity.slot);
            let state = self.live_state_mut(slots);
            let queued_generation = match state.access {
                BufferAccess::Legacy => return Err(GraphicsBufferError::NotMappable),
                BufferAccess::Queued { generation } => generation,
                BufferAccess::Writable | BufferAccess::Acquired { .. } => {
                    return Err(GraphicsBufferError::InvalidQueueState);
                }
            };
            if queued_generation != expected_generation
                || state.write_generation != expected_generation
            {
                return Err(GraphicsBufferError::StaleGeneration);
            }
            telemetry.record_acquire(slot, slots);
            let state = self.live_state_mut(slots);
            state.access = BufferAccess::Acquired {
                generation: queued_generation,
                consumer_pid,
            };
            Ok(queued_generation)
        })
    }

    /// Preflights the infallible post-present release boundary.
    ///
    /// Syscall code calls this before entering a display transaction. On the
    /// single-core IRQ-masked syscall path, a subsequent successful display
    /// commit can call [`Self::release`] without another process changing the
    /// acquired identity in between.
    pub fn can_release(
        &self,
        expected_generation: u64,
        consumer_pid: u64,
    ) -> Result<(), GraphicsBufferError> {
        if consumer_pid == 0 {
            return Err(GraphicsBufferError::InvalidConsumer);
        }
        with_pool(|slots, _| {
            validate_acquired(self.live_state(slots), expected_generation, consumer_pid)
        })
    }

    /// Releases one exact consumer acquisition and publishes `WRITABLE` as
    /// the producer's level-triggered release fence.
    pub fn release(
        &self,
        expected_generation: u64,
        consumer_pid: u64,
    ) -> Result<u64, GraphicsBufferError> {
        if consumer_pid == 0 {
            return Err(GraphicsBufferError::InvalidConsumer);
        }
        with_pool(|slots, telemetry| {
            let slot = usize::from(self.identity.slot);
            let state = self.live_state_mut(slots);
            validate_acquired(state, expected_generation, consumer_pid)?;
            telemetry.record_release(slot, slots);
            let state = self.live_state_mut(slots);
            state.access = BufferAccess::Writable;
            Ok(expected_generation)
        })
    }

    /// Preflights reaper-side cleanup for a dying mapped consumer.
    ///
    /// `Queued` is included because SurfaceServer can die after a producer
    /// publishes READABLE but before it executes Acquire. `Writable` needs no
    /// recovery. An acquisition owned by a different generation-qualified
    /// consumer is rejected rather than silently released.
    pub fn consumer_owner_death_frame(
        &self,
        consumer_pid: u64,
    ) -> Result<Option<ConsumerOwnerDeathFrame>, GraphicsBufferError> {
        if consumer_pid == 0 {
            return Err(GraphicsBufferError::InvalidConsumer);
        }
        with_pool(|slots, _| {
            let state = self.live_state(slots);
            match state.access {
                BufferAccess::Legacy => Err(GraphicsBufferError::NotMappable),
                BufferAccess::Writable => Ok(None),
                BufferAccess::Queued { generation } => {
                    if state.write_generation != generation {
                        return Err(GraphicsBufferError::StaleGeneration);
                    }
                    Ok(Some(ConsumerOwnerDeathFrame {
                        generation,
                        acquired: false,
                    }))
                }
                BufferAccess::Acquired {
                    generation,
                    consumer_pid: acquired_consumer,
                } => {
                    if state.write_generation != generation {
                        Err(GraphicsBufferError::StaleGeneration)
                    } else if acquired_consumer != consumer_pid {
                        Err(GraphicsBufferError::InvalidConsumer)
                    } else {
                        Ok(Some(ConsumerOwnerDeathFrame {
                            generation,
                            acquired: true,
                        }))
                    }
                }
            }
        })
    }

    /// Commits one preflighted consumer-owner-death abandonment.
    ///
    /// The process reaper calls this only after the dead consumer alias has
    /// been invalidated and the live producer alias has been restored to RW.
    /// Matching both generation and queued/acquired kind makes a stale token
    /// transactionally inert.
    pub fn abandon_consumer_owner_death(
        &self,
        expected: ConsumerOwnerDeathFrame,
        consumer_pid: u64,
    ) -> Result<u64, GraphicsBufferError> {
        if consumer_pid == 0 {
            return Err(GraphicsBufferError::InvalidConsumer);
        }
        with_pool(|slots, _| {
            let state = self.live_state_mut(slots);
            if state.write_generation != expected.generation {
                return Err(GraphicsBufferError::StaleGeneration);
            }
            match state.access {
                BufferAccess::Legacy => return Err(GraphicsBufferError::NotMappable),
                BufferAccess::Queued { generation }
                    if !expected.acquired && generation == expected.generation => {}
                BufferAccess::Acquired {
                    generation,
                    consumer_pid: acquired_consumer,
                } if expected.acquired
                    && generation == expected.generation
                    && acquired_consumer == consumer_pid => {}
                BufferAccess::Acquired {
                    consumer_pid: acquired_consumer,
                    ..
                } if acquired_consumer != consumer_pid => {
                    return Err(GraphicsBufferError::InvalidConsumer);
                }
                BufferAccess::Acquired { .. } if expected.acquired => {
                    return Err(GraphicsBufferError::InvalidConsumer);
                }
                BufferAccess::Writable
                | BufferAccess::Queued { .. }
                | BufferAccess::Acquired { .. } => {
                    return Err(GraphicsBufferError::InvalidQueueState);
                }
            }
            state.access = BufferAccess::Writable;
            Ok(expected.generation)
        })
    }

    /// Borrows the logical pixels of one exact consumer acquisition.
    ///
    /// This does not release the frame. Display failures therefore leave the
    /// buffer acquired and signal-free for retry or an explicit discard.
    pub fn with_acquired_pixels<R>(
        &self,
        expected_generation: u64,
        consumer_pid: u64,
        operation: impl for<'pixels> FnOnce(&'pixels [u32]) -> R,
    ) -> Result<R, GraphicsBufferError> {
        if consumer_pid == 0 {
            return Err(GraphicsBufferError::InvalidConsumer);
        }
        with_pool(|slots, _| {
            validate_acquired(self.live_state(slots), expected_generation, consumer_pid)?;
            Ok(operation(logical_pixels(usize::from(self.identity.slot))))
        })
    }

    /// Runs `operation` over an immutable logical pixel view only if the
    /// expected write generation is still current.
    ///
    /// Identity validation, generation validation, and the complete closure
    /// execute under the same critical section.  The slice therefore cannot
    /// be changed between validation and consumption.  The higher-ranked
    /// callback lifetime also prevents safe callers from retaining the view
    /// after the critical section ends.
    pub fn with_pixels<R>(
        &self,
        expected_generation: u64,
        operation: impl for<'pixels> FnOnce(&'pixels [u32]) -> R,
    ) -> Result<R, GraphicsBufferError> {
        with_pool(|slots, _| {
            let state = self.live_state(slots);
            if state.access.is_mappable() {
                return Err(GraphicsBufferError::InvalidQueueState);
            }
            if state.write_generation != expected_generation {
                return Err(GraphicsBufferError::StaleGeneration);
            }
            Ok(operation(logical_pixels(usize::from(self.identity.slot))))
        })
    }

    /// Atomically borrows two distinct legacy buffers at exact generations.
    ///
    /// Both identities and generations are validated under the same pool
    /// critical section before either pixel slice is exposed. This is the
    /// immutable source transaction used by Interactive-0 layered display
    /// composition.
    #[cfg(feature = "androidbox-interactive0")]
    pub fn with_pixels_pair<R>(
        &self,
        expected_generation: u64,
        other: &Self,
        other_expected_generation: u64,
        operation: impl for<'pixels> FnOnce(&'pixels [u32], &'pixels [u32]) -> R,
    ) -> Result<R, GraphicsBufferError> {
        if self.same_buffer(other) {
            return Err(GraphicsBufferError::InvalidQueueState);
        }
        with_pool(|slots, _| {
            let first = self.live_state(slots);
            let second = other.live_state(slots);
            if first.access.is_mappable() || second.access.is_mappable() {
                return Err(GraphicsBufferError::InvalidQueueState);
            }
            if first.write_generation != expected_generation
                || second.write_generation != other_expected_generation
            {
                return Err(GraphicsBufferError::StaleGeneration);
            }
            Ok(operation(
                logical_pixels(usize::from(self.identity.slot)),
                logical_pixels(usize::from(other.identity.slot)),
            ))
        })
    }

    pub const fn same_buffer(&self, other: &Self) -> bool {
        self.identity.slot == other.identity.slot
            && self.identity.generation == other.identity.generation
    }

    fn live_state<'a>(&self, slots: &'a [SlotState; GRAPHICS_BUFFER_SLOT_COUNT]) -> &'a SlotState {
        let state = slots
            .get(usize::from(self.identity.slot))
            .unwrap_or_else(|| panic!("graphics-buffer identity escaped the static pool"));
        if !state.occupied
            || state.generation != self.identity.generation
            || state.producer_pid != self.producer_pid
            || state.references == 0
        {
            panic!("graphics-buffer identity no longer names a live slot");
        }
        state
    }

    fn live_state_mut<'a>(
        &self,
        slots: &'a mut [SlotState; GRAPHICS_BUFFER_SLOT_COUNT],
    ) -> &'a mut SlotState {
        let state = slots
            .get_mut(usize::from(self.identity.slot))
            .unwrap_or_else(|| panic!("graphics-buffer identity escaped the static pool"));
        if !state.occupied
            || state.generation != self.identity.generation
            || state.producer_pid != self.producer_pid
            || state.references == 0
        {
            panic!("graphics-buffer identity no longer names a live slot");
        }
        state
    }

    #[cfg(test)]
    fn reference_count(&self) -> usize {
        with_pool(|slots, _| self.live_state(slots).references)
    }
}

impl fmt::Debug for GraphicsBuffer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GraphicsBuffer")
            .field("identity", &self.identity)
            .field("producer_pid", &self.producer_pid)
            .finish()
    }
}

impl Clone for GraphicsBuffer {
    fn clone(&self) -> Self {
        with_pool(|slots, _| {
            let state = self.live_state_mut(slots);
            state.references = state
                .references
                .checked_add(1)
                .filter(|references| *references <= isize::MAX as usize)
                .unwrap_or_else(|| panic!("graphics-buffer reference count exhausted"));
        });
        Self {
            identity: self.identity,
            producer_pid: self.producer_pid,
        }
    }
}

impl Drop for GraphicsBuffer {
    fn drop(&mut self) {
        with_pool(|slots, _| {
            let state = self.live_state_mut(slots);
            state.references = state
                .references
                .checked_sub(1)
                .unwrap_or_else(|| panic!("graphics-buffer reference count underflowed"));
            if state.references != 0 {
                return;
            }

            // Scrub the complete page-rounded allocation, including the
            // currently unexposed tail, before publishing the slot as vacant.
            backing_mut(usize::from(self.identity.slot)).fill(0);
            state.generation = state
                .generation
                .checked_add(1)
                .filter(|generation| *generation != 0)
                .unwrap_or_else(|| panic!("graphics-buffer slot generation exhausted"));
            state.producer_pid = 0;
            state.write_generation = 0;
            state.occupied = false;
            state.access = BufferAccess::Legacy;
        });
    }
}

fn mappable_signal_mask() -> ObjectSignals {
    ObjectSignals::from_bits(ObjectSignals::READABLE.bits() | ObjectSignals::WRITABLE.bits())
        .unwrap_or_else(|| panic!("canonical graphics-buffer signal mask was rejected"))
}

fn validate_acquired(
    state: &SlotState,
    expected_generation: u64,
    consumer_pid: u64,
) -> Result<(), GraphicsBufferError> {
    match state.access {
        BufferAccess::Legacy => Err(GraphicsBufferError::NotMappable),
        BufferAccess::Acquired {
            generation,
            consumer_pid: acquired_consumer,
        } => {
            if generation != expected_generation || state.write_generation != expected_generation {
                Err(GraphicsBufferError::StaleGeneration)
            } else if acquired_consumer != consumer_pid {
                Err(GraphicsBufferError::InvalidConsumer)
            } else {
                Ok(())
            }
        }
        BufferAccess::Writable | BufferAccess::Queued { .. } => {
            Err(GraphicsBufferError::InvalidQueueState)
        }
    }
}

fn access_is_in_flight(access: BufferAccess) -> bool {
    matches!(
        access,
        BufferAccess::Queued { .. } | BufferAccess::Acquired { .. }
    )
}

fn count_access(
    slots: &[SlotState; GRAPHICS_BUFFER_SLOT_COUNT],
    predicate: impl Fn(BufferAccess) -> bool,
) -> usize {
    slots
        .iter()
        .filter(|state| state.occupied && predicate(state.access))
        .count()
}

fn increment_counter(counter: &mut u64, overflowed: &mut bool) {
    if let Some(next) = counter.checked_add(1) {
        *counter = next;
    } else {
        *overflowed = true;
    }
}

fn validate_mappable_backing(slot: usize) -> Result<(), GraphicsBufferError> {
    let bytes = backing(slot);
    if bytes[..GRAPHICS_BUFFER_LOGICAL_BYTES]
        .chunks_exact(size_of::<u32>())
        .any(|pixel| {
            u32::from_le_bytes(
                pixel
                    .try_into()
                    .unwrap_or_else(|_| panic!("four-byte pixel chunk changed length")),
            ) & 0xff00_0000
                != 0
        })
    {
        return Err(GraphicsBufferError::NonCanonicalPixel);
    }
    if bytes[GRAPHICS_BUFFER_LOGICAL_BYTES..]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(GraphicsBufferError::NonZeroPadding);
    }
    Ok(())
}

fn logical_pixels(slot: usize) -> &'static [u32] {
    let bytes = backing(slot);
    unsafe {
        core::slice::from_raw_parts(bytes.as_ptr().cast::<u32>(), GRAPHICS_BUFFER_PIXEL_COUNT)
    }
}

fn backing(slot: usize) -> &'static [u8; GRAPHICS_BUFFER_BACKING_BYTES] {
    unsafe { &*BACKINGS[slot].0.get() }
}

fn backing_mut(slot: usize) -> &'static mut [u8; GRAPHICS_BUFFER_BACKING_BYTES] {
    unsafe { &mut *BACKINGS[slot].0.get() }
}

fn with_pool<R>(
    operation: impl FnOnce(&mut [SlotState; GRAPHICS_BUFFER_SLOT_COUNT], &mut PoolTelemetry) -> R,
) -> R {
    let _guard = PoolBorrowGuard::acquire();
    let pool = unsafe { &mut *POOL_STATE.0.get() };
    operation(&mut pool.slots, &mut pool.telemetry)
}

struct PoolBorrowGuard {
    saved_daif: u64,
}

impl PoolBorrowGuard {
    fn acquire() -> Self {
        let saved_daif = save_and_mask_irq();
        if POOL_BORROWED
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            restore_daif(saved_daif);
            panic!("graphics-buffer pool critical section was re-entered");
        }
        Self { saved_daif }
    }
}

impl Drop for PoolBorrowGuard {
    fn drop(&mut self) {
        if !POOL_BORROWED.swap(false, Ordering::Release) {
            panic!("graphics-buffer pool guard lost its ownership");
        }
        restore_daif(self.saved_daif);
    }
}

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
fn save_and_mask_irq() -> u64 {
    let saved: u64;
    unsafe {
        core::arch::asm!(
            "mrs {saved}, daif",
            "msr daifset, #2",
            "isb",
            saved = out(reg) saved,
            options(nostack, preserves_flags)
        );
    }
    saved
}

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
fn restore_daif(saved: u64) {
    unsafe {
        core::arch::asm!(
            "msr daif, {saved}",
            "isb",
            saved = in(reg) saved,
            options(nostack, preserves_flags)
        );
    }
}

// Host tests run in userspace, where DAIF is privileged.  Their mutex below
// provides process-level serialization while this no-op models IRQ state.
#[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
fn save_and_mask_irq() -> u64 {
    0
}

#[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
fn restore_daif(_saved: u64) {}

#[cfg(test)]
static TEST_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn test_serial_guard() -> std::sync::MutexGuard<'static, ()> {
    TEST_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::{
        GRAPHICS_BUFFER_BACKING_BYTES, GRAPHICS_BUFFER_LOGICAL_BYTES, GRAPHICS_BUFFER_PIXEL_COUNT,
        GraphicsBuffer, GraphicsBufferAccessSnapshot, GraphicsBufferError, backing, pool_snapshot,
        pool_telemetry_snapshot, test_serial_guard,
    };
    use bndr_abi::ObjectSignals;

    fn write_mapping(buffer: &GraphicsBuffer, offset: usize, bytes: &[u8]) {
        assert!(buffer.is_mappable());
        let end = offset.checked_add(bytes.len()).unwrap();
        assert!(end <= GRAPHICS_BUFFER_BACKING_BYTES);
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (buffer.backing_address() as *mut u8).add(offset),
                bytes.len(),
            );
        }
    }

    fn assert_two_slot_prefix(values: &[u64], expected: [u64; 2]) {
        assert_eq!(&values[..2], &expected);
        assert!(values[2..].iter().all(|value| *value == 0));
    }

    #[test]
    fn requires_a_producer_and_exhausts_the_fixed_pool() {
        let _serial = test_serial_guard();
        assert!(matches!(
            GraphicsBuffer::try_new(0),
            Err(GraphicsBufferError::InvalidProducer)
        ));
        let first = GraphicsBuffer::try_new(11).unwrap();
        let second = GraphicsBuffer::try_new(22).unwrap();
        assert_ne!(first.identity(), second.identity());
        #[cfg(feature = "androidbox-interactive0")]
        let third = GraphicsBuffer::try_new(33).unwrap();
        assert!(matches!(
            GraphicsBuffer::try_new(44),
            Err(GraphicsBufferError::Exhausted)
        ));
        assert_eq!(first.producer_pid(), 11);
        assert_eq!(second.producer_pid(), 22);
        #[cfg(feature = "androidbox-interactive0")]
        assert_eq!(third.producer_pid(), 33);
        assert_eq!(first.write_generation(), 0);
        assert_eq!(second.write_generation(), 0);
        #[cfg(feature = "androidbox-interactive0")]
        assert_eq!(third.write_generation(), 0);
    }

    #[test]
    fn pool_snapshot_proves_both_slot_lifecycles_and_complete_scrubbing() {
        let _serial = test_serial_guard();
        let initial = pool_snapshot();
        for (slot, state) in initial.slots.iter().enumerate() {
            assert_eq!(usize::from(state.slot), slot);
            assert_ne!(state.generation, 0);
            assert!(!state.occupied);
            assert_eq!(state.strong_references, 0);
            assert_eq!(state.producer_pid, 0);
            assert_eq!(state.write_generation, 0);
            assert_eq!(state.access, GraphicsBufferAccessSnapshot::Vacant);
            assert!(state.backing_all_zero);
        }

        let legacy = GraphicsBuffer::try_new(31).unwrap();
        let mapped = GraphicsBuffer::try_new_mappable(32).unwrap();
        let legacy_clone = legacy.clone();
        assert_eq!(legacy.write(0, &0x0012_3456_u32.to_le_bytes()), Ok(1));
        write_mapping(&mapped, 0, &0x0065_4321_u32.to_le_bytes());
        assert_eq!(mapped.queue(0), Ok(1));

        let queued = pool_snapshot();
        let legacy_live = queued.slots[usize::from(legacy.slot())];
        assert_eq!(legacy_live.generation, legacy.slot_generation());
        assert!(legacy_live.occupied);
        assert_eq!(legacy_live.strong_references, 2);
        assert_eq!(legacy_live.producer_pid, 31);
        assert_eq!(legacy_live.write_generation, 1);
        assert_eq!(legacy_live.access, GraphicsBufferAccessSnapshot::Legacy);
        assert!(!legacy_live.backing_all_zero);

        let mapped_slot = usize::from(mapped.slot());
        let mapped_generation = mapped.slot_generation();
        let mapped_live = queued.slots[mapped_slot];
        assert_eq!(mapped_live.generation, mapped_generation);
        assert!(mapped_live.occupied);
        assert_eq!(mapped_live.strong_references, 1);
        assert_eq!(mapped_live.producer_pid, 32);
        assert_eq!(mapped_live.write_generation, 1);
        assert_eq!(
            mapped_live.access,
            GraphicsBufferAccessSnapshot::Queued { generation: 1 }
        );
        assert!(!mapped_live.backing_all_zero);

        assert_eq!(mapped.acquire(1, 132), Ok(1));
        assert_eq!(
            pool_snapshot().slots[mapped_slot].access,
            GraphicsBufferAccessSnapshot::Acquired {
                generation: 1,
                consumer_pid: 132,
            }
        );

        let legacy_slot = usize::from(legacy.slot());
        let legacy_generation = legacy.slot_generation();
        drop(legacy_clone);
        drop(legacy);
        drop(mapped);

        let retired = pool_snapshot();
        for (slot, previous_generation) in [
            (legacy_slot, legacy_generation),
            (mapped_slot, mapped_generation),
        ] {
            let state = retired.slots[slot];
            assert_eq!(
                state.generation,
                previous_generation.checked_add(1).unwrap()
            );
            assert!(!state.occupied);
            assert_eq!(state.strong_references, 0);
            assert_eq!(state.producer_pid, 0);
            assert_eq!(state.write_generation, 0);
            assert_eq!(state.access, GraphicsBufferAccessSnapshot::Vacant);
            assert!(state.backing_all_zero);
        }
    }

    #[test]
    fn clones_pin_one_identity_until_the_last_drop_then_reuse_a_new_generation() {
        let _serial = test_serial_guard();
        let original = GraphicsBuffer::try_new(41).unwrap();
        let identity = original.identity();
        let clone = original.clone();
        assert!(original.same_buffer(&clone));
        assert_eq!(original.reference_count(), 2);
        assert_eq!(original.write(0, &0xff12_3456_u32.to_le_bytes()), Ok(1));
        drop(original);
        assert_eq!(clone.reference_count(), 1);
        assert_eq!(clone.with_pixels(1, |pixels| pixels[0]), Ok(0x0012_3456));

        let other = GraphicsBuffer::try_new(42).unwrap();
        assert_ne!(other.slot(), clone.slot());
        #[cfg(feature = "androidbox-interactive0")]
        let third = GraphicsBuffer::try_new(43).unwrap();
        assert!(matches!(
            GraphicsBuffer::try_new(45),
            Err(GraphicsBufferError::Exhausted)
        ));
        drop(clone);

        let reused = GraphicsBuffer::try_new(44).unwrap();
        assert_eq!(reused.slot(), identity.slot());
        assert!(reused.slot_generation() > identity.generation());
        assert_eq!(
            reused.with_pixels(0, |pixels| pixels[0]),
            Ok(0),
            "only the final clone may scrub and recycle the backing"
        );
        assert!(!reused.same_buffer(&other));
        #[cfg(feature = "androidbox-interactive0")]
        assert!(!reused.same_buffer(&third));
    }

    #[test]
    fn writes_canonical_xrgb_and_generation_qualified_reads_are_atomic() {
        let _serial = test_serial_guard();
        let buffer = GraphicsBuffer::try_new(51).unwrap();
        let source = [
            0xff, 0xff, 0xff, 0xff, // alpha/high byte is discarded
            0x78, 0x56, 0x34, 0x12,
        ];
        assert_eq!(buffer.write(4, &source), Ok(1));
        let pixels = buffer
            .with_pixels(1, |pixels| {
                assert_eq!(pixels.len(), GRAPHICS_BUFFER_PIXEL_COUNT);
                [pixels[0], pixels[1], pixels[2], pixels[pixels.len() - 1]]
            })
            .unwrap();
        assert_eq!(pixels, [0, 0x00ff_ffff, 0x0034_5678, 0]);
        assert!(matches!(
            buffer.with_pixels(0, |_| ()),
            Err(GraphicsBufferError::StaleGeneration)
        ));

        let tail = 0xabff_00cdu32.to_le_bytes();
        assert_eq!(
            buffer.write(GRAPHICS_BUFFER_LOGICAL_BYTES - 4, &tail),
            Ok(2)
        );
        assert_eq!(
            buffer.with_pixels(2, |pixels| pixels[pixels.len() - 1]),
            Ok(0x00ff_00cd)
        );
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn layered_pixel_borrow_validates_both_distinct_generations_atomically() {
        let _serial = test_serial_guard();
        let content = GraphicsBuffer::try_new(61).unwrap();
        let chrome = GraphicsBuffer::try_new(62).unwrap();
        assert_eq!(content.write(0, &0x0011_2233_u32.to_le_bytes()), Ok(1));
        assert_eq!(chrome.write(0, &0x0044_5566_u32.to_le_bytes()), Ok(1));
        assert_eq!(
            content.with_pixels_pair(1, &chrome, 1, |content_pixels, chrome_pixels| {
                (content_pixels[0], chrome_pixels[0])
            }),
            Ok((0x0011_2233, 0x0044_5566))
        );
        assert_eq!(
            content.with_pixels_pair(0, &chrome, 1, |_, _| ()),
            Err(GraphicsBufferError::StaleGeneration)
        );
        assert_eq!(
            content.with_pixels_pair(1, &content, 1, |_, _| ()),
            Err(GraphicsBufferError::InvalidQueueState)
        );
    }

    #[test]
    fn invalid_writes_leave_pixels_and_generation_unchanged() {
        let _serial = test_serial_guard();
        let buffer = GraphicsBuffer::try_new(61).unwrap();
        let initial = 0xff33_2211u32.to_le_bytes();
        assert_eq!(buffer.write(0, &initial), Ok(1));
        let before = buffer
            .with_pixels(1, |pixels| [pixels[0], pixels[1], pixels[pixels.len() - 1]])
            .unwrap();

        let failures = [
            buffer.write(0, &[]),
            buffer.write(1, &[0; 4]),
            buffer.write(0, &[0; 3]),
            buffer.write(GRAPHICS_BUFFER_LOGICAL_BYTES, &[0; 4]),
            buffer.write(usize::MAX - 3, &[0; 8]),
        ];
        assert_eq!(failures[0], Err(GraphicsBufferError::EmptyWrite));
        assert_eq!(failures[1], Err(GraphicsBufferError::Unaligned));
        assert_eq!(failures[2], Err(GraphicsBufferError::Unaligned));
        assert_eq!(failures[3], Err(GraphicsBufferError::OutOfRange));
        assert_eq!(failures[4], Err(GraphicsBufferError::OutOfRange));
        assert_eq!(buffer.write_generation(), 1);
        assert_eq!(
            buffer.with_pixels(1, |pixels| [pixels[0], pixels[1], pixels[pixels.len() - 1]]),
            Ok(before)
        );
    }

    #[test]
    fn last_drop_scrubs_the_logical_bytes_and_page_rounded_tail() {
        let _serial = test_serial_guard();
        let buffer = GraphicsBuffer::try_new(71).unwrap();
        let slot = usize::from(buffer.slot());
        assert_eq!(
            buffer.write(0, &[0xff; GRAPHICS_BUFFER_LOGICAL_BYTES]),
            Ok(1)
        );
        // Seed the unexposed tail to prove teardown scrubs all 307200 bytes.
        super::with_pool(|_, _| {
            super::backing_mut(slot)[GRAPHICS_BUFFER_LOGICAL_BYTES..].fill(0xa5);
        });
        drop(buffer);
        super::with_pool(|_, _| {
            assert_eq!(backing(slot).len(), GRAPHICS_BUFFER_BACKING_BYTES);
            assert!(backing(slot).iter().all(|byte| *byte == 0));
        });

        let reused = GraphicsBuffer::try_new(72).unwrap();
        assert_eq!(usize::from(reused.slot()), slot);
        assert_eq!(reused.write_generation(), 0);
        assert!(
            reused
                .with_pixels(0, |pixels| pixels.iter().all(|pixel| *pixel == 0))
                .unwrap()
        );
    }

    #[test]
    fn backing_slots_are_page_aligned_and_have_the_exact_static_size() {
        let _serial = test_serial_guard();
        let first = GraphicsBuffer::try_new(81).unwrap();
        let second = GraphicsBuffer::try_new(82).unwrap();
        super::with_pool(|_, _| {
            for buffer in [&first, &second] {
                let bytes = backing(usize::from(buffer.slot()));
                assert_eq!(bytes.len(), GRAPHICS_BUFFER_BACKING_BYTES);
                assert_eq!(bytes.as_ptr() as usize % 4096, 0);
            }
        });
    }

    #[test]
    fn mappable_buffer_exposes_all_rounded_pages_and_denies_copy_write() {
        let _serial = test_serial_guard();
        let buffer = GraphicsBuffer::try_new_mappable(91).unwrap();
        assert!(buffer.is_mappable());
        assert_eq!(buffer.backing_address() % 4096, 0);
        assert_eq!(buffer.write_generation(), 0);
        assert_eq!(
            buffer.signal_mask(),
            ObjectSignals::from_bits(
                ObjectSignals::READABLE.bits() | ObjectSignals::WRITABLE.bits()
            )
            .unwrap()
        );
        assert_eq!(buffer.signals(), ObjectSignals::WRITABLE);
        assert_eq!(
            buffer.write(0, &0x0012_3456_u32.to_le_bytes()),
            Err(GraphicsBufferError::MappableWriteDenied)
        );
        assert_eq!(buffer.write_generation(), 0);
        assert_eq!(buffer.signals(), ObjectSignals::WRITABLE);

        let legacy = GraphicsBuffer::try_new(92).unwrap();
        assert!(!legacy.is_mappable());
        assert_eq!(legacy.signal_mask(), ObjectSignals::NONE);
        assert_eq!(legacy.signals(), ObjectSignals::NONE);
        assert_eq!(legacy.queue(0), Err(GraphicsBufferError::NotMappable));
    }

    #[test]
    fn mapped_queue_acquire_and_release_are_generation_and_consumer_qualified() {
        let _serial = test_serial_guard();
        let buffer = GraphicsBuffer::try_new_mappable(101).unwrap();
        write_mapping(&buffer, 0, &0x0012_3456_u32.to_le_bytes());

        assert_eq!(buffer.queue(0), Ok(1));
        assert_eq!(buffer.write_generation(), 1);
        assert_eq!(buffer.signals(), ObjectSignals::READABLE);
        assert_eq!(buffer.queue(1), Err(GraphicsBufferError::InvalidQueueState));
        assert_eq!(
            buffer.acquire(0, 201),
            Err(GraphicsBufferError::StaleGeneration)
        );
        assert_eq!(
            buffer.acquire(1, 0),
            Err(GraphicsBufferError::InvalidConsumer)
        );
        assert_eq!(buffer.acquire(1, 201), Ok(1));
        assert_eq!(buffer.signals(), ObjectSignals::NONE);
        assert_eq!(
            buffer.can_release(1, 202),
            Err(GraphicsBufferError::InvalidConsumer)
        );
        assert_eq!(
            buffer.with_acquired_pixels(1, 202, |_| ()),
            Err(GraphicsBufferError::InvalidConsumer)
        );
        assert_eq!(
            buffer.with_acquired_pixels(0, 201, |_| ()),
            Err(GraphicsBufferError::StaleGeneration)
        );
        assert_eq!(
            buffer.with_acquired_pixels(1, 201, |pixels| pixels[0]),
            Ok(0x0012_3456)
        );
        assert_eq!(buffer.can_release(1, 201), Ok(()));
        assert_eq!(buffer.release(1, 201), Ok(1));
        assert_eq!(buffer.signals(), ObjectSignals::WRITABLE);
        assert_eq!(
            buffer.release(1, 201),
            Err(GraphicsBufferError::InvalidQueueState)
        );
        assert_eq!(buffer.queue(0), Err(GraphicsBufferError::StaleGeneration));
        assert_eq!(buffer.queue(1), Ok(2));
        assert_eq!(buffer.acquire(2, 201), Ok(2));
        assert_eq!(buffer.release(2, 201), Ok(2));
    }

    #[test]
    fn dual_slot_telemetry_proves_overlap_selective_release_and_alternating_acquisition() {
        let _serial = test_serial_guard();
        let first = GraphicsBuffer::try_new_mappable(102).unwrap();
        let second = GraphicsBuffer::try_new_mappable(102).unwrap();
        assert_eq!((first.slot(), second.slot()), (0, 1));

        assert_eq!(first.queue(0), Ok(1));
        assert_eq!(second.queue(0), Ok(1));
        assert_eq!(first.acquire(1, 202), Ok(1));
        assert_eq!(second.acquire(1, 202), Ok(1));

        assert_eq!(first.release(1, 202), Ok(1));
        assert_eq!(first.queue(1), Ok(2));
        assert_eq!(first.acquire(2, 202), Ok(2));
        assert_eq!(second.release(1, 202), Ok(1));
        assert_eq!(second.queue(1), Ok(2));
        assert_eq!(second.acquire(2, 202), Ok(2));

        assert_eq!(first.release(2, 202), Ok(2));
        assert_eq!(first.queue(2), Ok(3));
        assert_eq!(first.acquire(3, 202), Ok(3));
        assert_eq!(second.release(2, 202), Ok(2));
        assert_eq!(second.queue(2), Ok(3));
        assert_eq!(second.acquire(3, 202), Ok(3));
        assert_eq!(first.release(3, 202), Ok(3));
        assert_eq!(second.release(3, 202), Ok(3));

        let telemetry = pool_telemetry_snapshot();
        assert_ne!(telemetry.epoch, 0);
        assert_eq!(
            (telemetry.writable, telemetry.queued, telemetry.acquired),
            (2, 0, 0)
        );
        assert_eq!(
            (
                telemetry.peak_queued,
                telemetry.peak_acquired,
                telemetry.peak_in_flight,
            ),
            (2, 2, 2)
        );
        assert_eq!(telemetry.dual_in_flight_publications, 5);
        assert_eq!(telemetry.selective_releases, 5);
        assert_two_slot_prefix(&telemetry.queue_successes, [3, 3]);
        assert_two_slot_prefix(&telemetry.acquire_successes, [3, 3]);
        assert_two_slot_prefix(&telemetry.release_successes, [3, 3]);
        assert_eq!(telemetry.acquisition_order_bits, 0b10_1010);
        assert_eq!(telemetry.acquisition_order_count, 6);
        assert_eq!(telemetry.acquisition_slot_switches, 5);
        assert_eq!(telemetry.first_acquisition_slot, Some(0));
        assert_eq!(telemetry.last_acquisition_slot, Some(1));
        assert!(!telemetry.acquisition_order_overflowed);
        assert!(!telemetry.counter_overflowed);
    }

    #[test]
    fn failed_dual_slot_transitions_leave_all_telemetry_transactionally_unchanged() {
        let _serial = test_serial_guard();
        let first = GraphicsBuffer::try_new_mappable(103).unwrap();
        let second = GraphicsBuffer::try_new_mappable(103).unwrap();
        assert_eq!(first.queue(0), Ok(1));
        assert_eq!(second.queue(0), Ok(1));
        assert_eq!(first.acquire(1, 203), Ok(1));
        let before = pool_telemetry_snapshot();

        assert_eq!(first.queue(1), Err(GraphicsBufferError::InvalidQueueState));
        assert_eq!(
            first.acquire(1, 203),
            Err(GraphicsBufferError::InvalidQueueState)
        );
        assert_eq!(
            first.release(1, 204),
            Err(GraphicsBufferError::InvalidConsumer)
        );
        assert_eq!(
            second.acquire(2, 203),
            Err(GraphicsBufferError::StaleGeneration)
        );
        assert_eq!(pool_telemetry_snapshot(), before);

        assert_eq!(first.release(1, 203), Ok(1));
        assert_eq!(second.acquire(1, 203), Ok(1));
        assert_eq!(second.release(1, 203), Ok(1));
    }

    #[test]
    fn completed_epoch_remains_auditable_until_scrubbed_pool_is_reallocated() {
        let _serial = test_serial_guard();
        let first = GraphicsBuffer::try_new_mappable(104).unwrap();
        let second = GraphicsBuffer::try_new_mappable(104).unwrap();
        assert_eq!(first.queue(0), Ok(1));
        assert_eq!(second.queue(0), Ok(1));
        assert_eq!(first.acquire(1, 204), Ok(1));
        assert_eq!(second.acquire(1, 204), Ok(1));
        write_mapping(&first, 0, &[0x5a; GRAPHICS_BUFFER_BACKING_BYTES]);
        write_mapping(&second, 0, &[0xa5; GRAPHICS_BUFFER_BACKING_BYTES]);
        let live = pool_telemetry_snapshot();
        drop(first);
        drop(second);

        let scrubbed = pool_snapshot();
        assert!(scrubbed.slots.iter().all(|slot| {
            !slot.occupied
                && slot.strong_references == 0
                && slot.access == GraphicsBufferAccessSnapshot::Vacant
                && slot.backing_all_zero
        }));
        let retired = pool_telemetry_snapshot();
        assert_eq!(retired.epoch, live.epoch);
        assert_eq!(
            (retired.writable, retired.queued, retired.acquired),
            (0, 0, 0)
        );
        assert_eq!(retired.peak_queued, 2);
        assert_eq!(retired.peak_acquired, 2);
        assert_eq!(retired.peak_in_flight, 2);
        assert_eq!(retired.dual_in_flight_publications, 1);
        assert_two_slot_prefix(&retired.acquire_successes, [1, 1]);

        let next = GraphicsBuffer::try_new_mappable(105).unwrap();
        let reset = pool_telemetry_snapshot();
        assert_eq!(reset.epoch, retired.epoch.checked_add(1).unwrap());
        assert_eq!((reset.writable, reset.queued, reset.acquired), (1, 0, 0));
        assert_eq!(reset.peak_queued, 0);
        assert_eq!(reset.peak_acquired, 0);
        assert_eq!(reset.peak_in_flight, 0);
        assert_eq!(reset.dual_in_flight_publications, 0);
        assert_eq!(reset.selective_releases, 0);
        assert_two_slot_prefix(&reset.queue_successes, [0, 0]);
        assert_two_slot_prefix(&reset.acquire_successes, [0, 0]);
        assert_two_slot_prefix(&reset.release_successes, [0, 0]);
        assert_eq!(reset.acquisition_order_bits, 0);
        assert_eq!(reset.acquisition_order_count, 0);
        assert_eq!(reset.acquisition_slot_switches, 0);
        assert_eq!(reset.first_acquisition_slot, None);
        assert_eq!(reset.last_acquisition_slot, None);
        assert!(!reset.acquisition_order_overflowed);
        assert!(!reset.counter_overflowed);
        assert!(
            pool_snapshot()
                .slots
                .iter()
                .all(|slot| slot.backing_all_zero)
        );
        drop(next);
    }

    #[test]
    fn consumer_owner_death_abandons_queued_and_acquired_frames_without_stale_replay() {
        let _serial = test_serial_guard();
        let buffer = GraphicsBuffer::try_new_mappable(106).unwrap();

        assert_eq!(buffer.queue(0), Ok(1));
        let queued = buffer.consumer_owner_death_frame(206).unwrap().unwrap();
        assert_eq!(queued.generation(), 1);
        assert!(!queued.was_acquired());
        assert_eq!(buffer.abandon_consumer_owner_death(queued, 206), Ok(1));
        assert_eq!(buffer.signals(), ObjectSignals::WRITABLE);
        assert_eq!(
            buffer.abandon_consumer_owner_death(queued, 206),
            Err(GraphicsBufferError::InvalidQueueState)
        );

        assert_eq!(buffer.queue(1), Ok(2));
        assert_eq!(buffer.acquire(2, 206), Ok(2));
        assert_eq!(
            buffer.consumer_owner_death_frame(207),
            Err(GraphicsBufferError::InvalidConsumer)
        );
        let acquired = buffer.consumer_owner_death_frame(206).unwrap().unwrap();
        assert_eq!(acquired.generation(), 2);
        assert!(acquired.was_acquired());
        assert_eq!(
            buffer.abandon_consumer_owner_death(acquired, 207),
            Err(GraphicsBufferError::InvalidConsumer)
        );
        assert_eq!(buffer.abandon_consumer_owner_death(acquired, 206), Ok(2));
        assert_eq!(buffer.signals(), ObjectSignals::WRITABLE);
        assert_eq!(buffer.consumer_owner_death_frame(206), Ok(None));
    }

    #[cfg(not(feature = "mobile-ui-runtime"))]
    #[test]
    fn mapped_queue_rejects_noncanonical_pixels_and_nonzero_rounded_tail_atomically() {
        let _serial = test_serial_guard();
        let buffer = GraphicsBuffer::try_new_mappable(111).unwrap();
        write_mapping(&buffer, 0, &0xff12_3456_u32.to_le_bytes());
        assert_eq!(buffer.queue(0), Err(GraphicsBufferError::NonCanonicalPixel));
        assert_eq!(buffer.write_generation(), 0);
        assert_eq!(buffer.signals(), ObjectSignals::WRITABLE);

        write_mapping(&buffer, 0, &0x0012_3456_u32.to_le_bytes());
        write_mapping(&buffer, GRAPHICS_BUFFER_LOGICAL_BYTES, &[0xa5]);
        assert_eq!(buffer.queue(0), Err(GraphicsBufferError::NonZeroPadding));
        assert_eq!(buffer.write_generation(), 0);
        assert_eq!(buffer.signals(), ObjectSignals::WRITABLE);

        write_mapping(&buffer, GRAPHICS_BUFFER_LOGICAL_BYTES, &[0]);
        assert_eq!(buffer.queue(0), Ok(1));
        assert_eq!(buffer.signals(), ObjectSignals::READABLE);
    }

    #[test]
    fn acquired_pixel_operation_never_implicitly_releases_failed_or_successful_present() {
        let _serial = test_serial_guard();
        let buffer = GraphicsBuffer::try_new_mappable(121).unwrap();
        write_mapping(&buffer, 4, &0x0000_00a5_u32.to_le_bytes());
        assert_eq!(buffer.queue(0), Ok(1));
        assert_eq!(buffer.acquire(1, 221), Ok(1));

        let failed = buffer.with_acquired_pixels(1, 221, |pixels| {
            assert_eq!(pixels[1], 0x0000_00a5);
            Err::<(), u8>(7)
        });
        assert_eq!(failed, Ok(Err(7)));
        assert_eq!(buffer.signals(), ObjectSignals::NONE);
        assert_eq!(buffer.can_release(1, 221), Ok(()));

        assert_eq!(
            buffer.with_acquired_pixels(1, 221, |pixels| pixels[1]),
            Ok(0x0000_00a5)
        );
        assert_eq!(buffer.signals(), ObjectSignals::NONE);
        assert_eq!(buffer.release(1, 221), Ok(1));
        assert_eq!(buffer.signals(), ObjectSignals::WRITABLE);
    }

    #[test]
    fn last_mappable_drop_scrubs_mapping_and_resets_queue_state_before_reuse() {
        let _serial = test_serial_guard();
        let buffer = GraphicsBuffer::try_new_mappable(131).unwrap();
        let slot = usize::from(buffer.slot());
        let identity = buffer.identity();
        assert_eq!(buffer.queue(0), Ok(1));
        assert_eq!(buffer.acquire(1, 231), Ok(1));
        // Model a hostile mapping that dirties both logical pixels and the
        // rounded tail after ownership has moved to the consumer. Teardown
        // must scrub the entire allocation regardless of protocol state.
        write_mapping(&buffer, 0, &[0x5a; GRAPHICS_BUFFER_BACKING_BYTES]);
        drop(buffer);

        super::with_pool(|_, _| {
            assert!(backing(slot).iter().all(|byte| *byte == 0));
        });
        let reused = GraphicsBuffer::try_new_mappable(132).unwrap();
        assert_eq!(usize::from(reused.slot()), slot);
        assert!(reused.slot_generation() > identity.generation());
        assert_eq!(reused.write_generation(), 0);
        assert_eq!(reused.signals(), ObjectSignals::WRITABLE);
        assert_eq!(reused.queue(0), Ok(1));
    }
}
