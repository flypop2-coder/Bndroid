//! Boot-time QEMU ramfb discovery, splash rendering, and publication.

use core::{
    cell::UnsafeCell,
    mem::size_of,
    sync::atomic::{AtomicBool, Ordering},
};

use bndr_abi::{KeyInputSample, ObjectSignals};
use bndr_ui::{BufferPresent, InputSample, PresentFrame};

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
use bndroid_kernel::frame_clock::{
    FrameClock, FrameClockAcquireError, FrameClockPhase, FrameClockPresentError,
    FrameClockSnapshot, FrameGrant,
};
use bndroid_kernel::surface::SurfaceKeySnapshot;
use bndroid_kernel::{
    compositor::{
        self, CompositionEvidence, CursorState, Rect, ScenePublishEvidence, SceneUpdateEvidence,
        compose_full, publish_scene_damage, update_cursor as compose_cursor,
        update_scene as compose_scene, validate_scanout_composition, validate_scene_damage,
    },
    fdt::FwCfgMmioRegion,
    framebuffer::{
        self, BYTE_COUNT, HEIGHT, PIXEL_COUNT, STRIDE, WIDTH, boot_splash_pixel, pixel_digest,
        render_boot_splash,
    },
    fw_cfg::{
        DirectoryError, FILE_DIRECTORY_HEADER_BYTES, FILE_DIRECTORY_SELECTOR, FILE_ENTRY_BYTES,
        RAMFB_CONFIG_BYTES, RamFbConfig, directory_count, find_ramfb,
    },
    graphics_buffer::{GraphicsBuffer, GraphicsBufferError},
    surface::{
        SURFACE_ORIGIN_X, SURFACE_ORIGIN_Y, SurfaceCapability, SurfaceCommitEvidence, SurfaceError,
        SurfaceInputError, SurfaceInputQueue, SurfaceInputSnapshot, SurfaceKeyError,
        SurfaceKeyQueue, SurfaceOwner, SurfaceSession,
    },
    ui::{UiCommit, UiController, UiView, scene_pixel},
};

use crate::arch::aarch64::{self, dma};
use crate::driver::fw_cfg::{FwCfgError, FwCfgMmio};

const MAX_FW_CFG_FILES: usize = 64;
const DIRECTORY_BYTES: usize = FILE_DIRECTORY_HEADER_BYTES + MAX_FW_CFG_FILES * FILE_ENTRY_BYTES;

#[repr(C, align(4096))]
struct FramebufferStorage(UnsafeCell<[u32; PIXEL_COUNT]>);

// The boot CPU initializes both surfaces before publication. Every post-init
// guest access is serialized by the IRQ-save display critical section below;
// QEMU concurrently reads scanout as a DMA-like external observer.
unsafe impl Sync for FramebufferStorage {}

#[repr(C, align(4096))]
struct SceneStorage(UnsafeCell<[u32; PIXEL_COUNT]>);

// SCENE is guest-only. The display critical section enforces its one active
// writer and the SurfaceOwner state enforces its one logical owner.
unsafe impl Sync for SceneStorage {}

struct SurfaceBinding {
    capability: SurfaceCapability,
    process_id: u64,
    /// Set only by the trusted process reaper.  Explicit handle close freezes
    /// a session but cannot authorize a live process (or a concurrent peer)
    /// to take over the display namespace.
    rebindable: bool,
    /// A recovered binding owns the reservation immediately, but its reset
    /// `SurfaceSession` still reports `KernelFallback` until the first valid
    /// FULL frame.  Keep the kernel UI from becoming a second scene writer in
    /// that interval; invalid presents leave this bit set.
    awaiting_recovery_frame: bool,
    #[cfg(all(
        feature = "graphics-frame-clock-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    frame_clock: FrameClock,
}

struct CompositorState {
    ready: bool,
    cursor: CursorState,
    updates: u64,
    scene_commits: u64,
    surface: SurfaceSession,
    binding: Option<SurfaceBinding>,
    input: SurfaceInputQueue,
    key_input: SurfaceKeyQueue,
    #[cfg(all(
        feature = "graphics-frame-clock-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    frame_ungated_present_rejections: u64,
    #[cfg(all(
        feature = "graphics-frame-clock-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    frame_failed_present_preservations: u64,
}

struct CompositorStateStorage(UnsafeCell<CompositorState>);

// UnsafeCell access is permitted only while local IRQ is masked and the
// non-reentrant display borrow guard is held. This prevents a timer switch to
// an EL0 SurfacePresent syscall while foreground composition has live borrows.
unsafe impl Sync for CompositorStateStorage {}

#[repr(C, align(8))]
struct DirectoryStorage(UnsafeCell<[u8; DIRECTORY_BYTES]>);

// The scratch directory is used synchronously on the boot CPU before IRQs.
unsafe impl Sync for DirectoryStorage {}

static FRAMEBUFFER: FramebufferStorage = FramebufferStorage(UnsafeCell::new([0; PIXEL_COUNT]));
static SCENE: SceneStorage = SceneStorage(UnsafeCell::new([0; PIXEL_COUNT]));
static DIRECTORY: DirectoryStorage = DirectoryStorage(UnsafeCell::new([0; DIRECTORY_BYTES]));
static COMPOSITOR_STATE: CompositorStateStorage =
    CompositorStateStorage(UnsafeCell::new(CompositorState {
        ready: false,
        cursor: CursorState::hidden(),
        updates: 0,
        scene_commits: 0,
        surface: SurfaceSession::new(),
        binding: None,
        input: SurfaceInputQueue::new(),
        key_input: SurfaceKeyQueue::new(),
        #[cfg(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        ))]
        frame_ungated_present_rejections: 0,
        #[cfg(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        ))]
        frame_failed_present_preservations: 0,
    }));
static DISPLAY_BORROWED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayError {
    FwCfg(FwCfgError),
    Directory(DirectoryError),
    TooManyFiles,
    InvalidFramebufferAddress,
    Render(framebuffer::RenderError),
    Compose(compositor::CompositorError),
    SceneMismatch,
    AlreadyInitialized,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    NotReady,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    InvalidCursorCoordinate,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    InvalidPointerSample,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    CounterExhausted,
    Surface(SurfaceError),
    GraphicsBuffer(GraphicsBufferError),
    SurfaceInput(SurfaceInputError),
    SurfaceKey(SurfaceKeyError),
    SurfaceAlreadyAcquired,
    SurfaceNotAcquired,
    SurfaceCapabilityMismatch,
    SurfaceProcessMismatch,
    #[cfg_attr(
        not(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        )),
        allow(dead_code)
    )]
    FrameOpportunityUnavailable,
    #[cfg_attr(
        not(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        )),
        allow(dead_code)
    )]
    FrameGrantAlreadyOutstanding,
    #[cfg_attr(
        not(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        )),
        allow(dead_code)
    )]
    FrameGrantRequired,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    SceneWriterOwnedByUserspace,
}

impl DisplayError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FwCfg(error) => error.as_str(),
            Self::Directory(error) => error.as_str(),
            Self::TooManyFiles => "fw_cfg file directory exceeds the fixed M26 bound",
            Self::InvalidFramebufferAddress => {
                "ramfb storage is not page-aligned identity-mapped RAM"
            }
            Self::Render(error) => error.as_str(),
            Self::Compose(error) => error.as_str(),
            Self::SceneMismatch => "hidden cursor composition did not preserve the scene",
            Self::AlreadyInitialized => "ramfb compositor was initialized more than once",
            Self::NotReady => "ramfb compositor is not ready",
            Self::InvalidCursorCoordinate => "cursor coordinate is outside the scanout",
            Self::InvalidPointerSample => "pointer sample does not identify a visible coordinate",
            Self::CounterExhausted => "display commit counter is exhausted",
            Self::Surface(error) => match error {
                SurfaceError::WrongScenePixelCount => "surface scene has the wrong pixel count",
                SurfaceError::WrongBufferPixelCount => {
                    "graphics buffer has the wrong surface pixel count"
                }
                SurfaceError::NonCanonicalBufferPixel => {
                    "graphics buffer contains a noncanonical XRGB pixel"
                }
                SurfaceError::Protocol(_) => "surface presentation violated the protocol",
                SurfaceError::CommitCounterExhausted => "surface commit counter is exhausted",
                SurfaceError::Degraded => "surface session is degraded",
            },
            Self::GraphicsBuffer(error) => error.as_str(),
            Self::SurfaceInput(error) => match error {
                SurfaceInputError::InvalidSample(_) => "surface input sample is invalid",
                SurfaceInputError::SequenceExhausted => "surface input sequence is exhausted",
                SurfaceInputError::CounterExhausted => "surface input counter is exhausted",
            },
            Self::SurfaceKey(error) => match error {
                SurfaceKeyError::InvalidSample(_) => "surface key sample is invalid",
                SurfaceKeyError::Full => "surface key queue is full",
                SurfaceKeyError::SequenceExhausted => "surface key sequence is exhausted",
                SurfaceKeyError::CounterExhausted => "surface key counter is exhausted",
            },
            Self::SurfaceAlreadyAcquired => "userspace surface was already acquired",
            Self::SurfaceNotAcquired => "userspace surface has not been acquired",
            Self::SurfaceCapabilityMismatch => "userspace surface capability does not match",
            Self::SurfaceProcessMismatch => "userspace surface process identity does not match",
            Self::FrameOpportunityUnavailable => {
                "the software frame clock has no pending opportunity"
            }
            Self::FrameGrantAlreadyOutstanding => {
                "the software frame clock already has an outstanding grant"
            }
            Self::FrameGrantRequired => "surface presentation requires a software frame grant",
            Self::SceneWriterOwnedByUserspace => {
                "kernel scene commits are disabled after userspace handoff"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayEvidence {
    pub fw_cfg_base: usize,
    pub fw_cfg_size: usize,
    pub fw_cfg_features: u32,
    pub directory_files: u32,
    pub ramfb_selector: u16,
    pub dma_operations: u32,
    pub framebuffer_address: usize,
    pub framebuffer_digest: u64,
    pub scene_address: usize,
    pub scene_digest: u64,
    pub sample_status: u32,
    pub sample_cyan: u32,
    pub sample_purple: u32,
    pub sample_green: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BackingEvidence {
    framebuffer_address: usize,
    framebuffer_digest: u64,
    scene_address: usize,
    scene_digest: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
pub struct CursorUpdateEvidence {
    pub previous: CursorState,
    pub current: CursorState,
    pub composition: CompositionEvidence,
    pub updates: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
pub struct SceneCommitEvidence {
    pub composition: SceneUpdateEvidence,
    pub commits: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceAcquireEvidence {
    pub owner: SurfaceOwner,
    pub session_id: u64,
    pub process_id: u64,
    pub input: SurfaceInputSnapshot,
    pub key_input: SurfaceKeySnapshot,
    /// True when this acquisition atomically replaced a degraded session.
    pub recovered: bool,
    pub previous_session_id: u64,
    pub previous_process_id: u64,
    pub discarded_input: usize,
    pub discarded_keys: usize,
    pub frozen_scene_digest: u64,
    pub frozen_scanout_digest: u64,
    pub frozen_cursor: CursorState,
    pub frozen_cursor_pixels: usize,
    pub frozen_scanout_valid: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfacePresentEvidence {
    pub session_id: u64,
    pub process_id: u64,
    pub surface: SurfaceCommitEvidence,
    pub composition: ScenePublishEvidence,
    pub input: SurfaceInputSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceInputReadEvidence {
    pub owner: SurfaceOwner,
    pub sample: Option<InputSample>,
    pub input: SurfaceInputSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceKeyReadEvidence {
    pub owner: SurfaceOwner,
    pub sample: Option<KeyInputSample>,
    pub key_input: SurfaceKeySnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceDegradeEvidence {
    pub previous_owner: SurfaceOwner,
    pub current_owner: SurfaceOwner,
    pub session_id: u64,
    pub process_id: u64,
    pub last_frame_id: Option<u32>,
    pub commits: u64,
    pub scene_digest: u64,
    pub scanout_digest: u64,
    pub input: SurfaceInputSnapshot,
    pub key_input: SurfaceKeySnapshot,
    pub peer_closed_edge: bool,
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayFrameClockSnapshot {
    pub session_id: u64,
    pub process_id: u64,
    pub clock: FrameClockSnapshot,
    pub ungated_present_rejections: u64,
    pub failed_present_preservations: u64,
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceFrameAcquireEvidence {
    pub session_id: u64,
    pub process_id: u64,
    pub grant: FrameGrant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
pub struct FallbackUiEvidence {
    pub commit: UiCommit,
    pub scene: SceneCommitEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
pub enum PointerRouteEvidence {
    KernelFallback {
        ui: Option<FallbackUiEvidence>,
    },
    UserspaceBound {
        sample: InputSample,
        input: SurfaceInputSnapshot,
        readable_edge: bool,
        coalesced: bool,
    },
    Degraded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyRouteEvidence {
    KernelFallback,
    UserspaceBound {
        sample: KeyInputSample,
        key_input: SurfaceKeySnapshot,
        key_ready_edge: bool,
    },
    UserspaceRejected {
        error: SurfaceKeyError,
        key_input: SurfaceKeySnapshot,
    },
    Degraded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
pub struct PointerProcessEvidence {
    pub cursor: CursorUpdateEvidence,
    pub route: PointerRouteEvidence,
}

/// Configures a standalone QEMU ramfb when `etc/ramfb` is present.
///
/// A valid fw_cfg transport without that optional file is not an error; this
/// keeps hardware/self-test boots headless while the normal emulator profile
/// opts into an actual graphical console with `-device ramfb`.
pub fn init(region: Option<FwCfgMmioRegion>) -> Result<Option<DisplayEvidence>, DisplayError> {
    if unsafe { (*COMPOSITOR_STATE.0.get()).ready } {
        return Err(DisplayError::AlreadyInitialized);
    }
    let Some(region) = region else {
        let _backing = initialize_backing()?;
        dma::publish_to_device();
        finish_initialization();
        return Ok(None);
    };
    // SAFETY: FDT discovery accepted one enabled direct-root fw_cfg window;
    // the boot CPU creates its only owner before interrupts or other drivers.
    let mut fw_cfg = unsafe { FwCfgMmio::probe(region) }.map_err(DisplayError::FwCfg)?;

    let directory = unsafe { &mut *DIRECTORY.0.get() };
    directory.fill(0);
    fw_cfg
        .dma_read(
            FILE_DIRECTORY_SELECTOR,
            directory.as_mut_ptr() as usize,
            FILE_DIRECTORY_HEADER_BYTES,
        )
        .map_err(DisplayError::FwCfg)?;
    let file_count = directory_count(&directory[..FILE_DIRECTORY_HEADER_BYTES])
        .map_err(DisplayError::Directory)?;
    let file_count_usize = usize::try_from(file_count).map_err(|_| DisplayError::TooManyFiles)?;
    if file_count_usize > MAX_FW_CFG_FILES {
        return Err(DisplayError::TooManyFiles);
    }
    let directory_bytes = FILE_DIRECTORY_HEADER_BYTES
        .checked_add(
            file_count_usize
                .checked_mul(FILE_ENTRY_BYTES)
                .ok_or(DisplayError::TooManyFiles)?,
        )
        .ok_or(DisplayError::TooManyFiles)?;
    fw_cfg
        .dma_read(
            FILE_DIRECTORY_SELECTOR,
            directory.as_mut_ptr() as usize,
            directory_bytes,
        )
        .map_err(DisplayError::FwCfg)?;
    let Some(ramfb) = find_ramfb(&directory[..directory_bytes]).map_err(DisplayError::Directory)?
    else {
        let _backing = initialize_backing()?;
        dma::publish_to_device();
        finish_initialization();
        return Ok(None);
    };

    let backing = initialize_backing()?;
    let config = RamFbConfig::xrgb8888(
        backing.framebuffer_address as u64,
        WIDTH as u32,
        HEIGHT as u32,
        STRIDE as u32,
    );
    fw_cfg
        .dma_write(
            ramfb.selector,
            config.as_bytes().as_ptr() as usize,
            RAMFB_CONFIG_BYTES as usize,
        )
        .map_err(DisplayError::FwCfg)?;
    dma::publish_to_device();
    finish_initialization();

    Ok(Some(DisplayEvidence {
        fw_cfg_base: fw_cfg.base(),
        fw_cfg_size: fw_cfg.size(),
        fw_cfg_features: fw_cfg.features(),
        directory_files: file_count,
        ramfb_selector: ramfb.selector,
        dma_operations: fw_cfg.dma_operations(),
        framebuffer_address: backing.framebuffer_address,
        framebuffer_digest: backing.framebuffer_digest,
        scene_address: backing.scene_address,
        scene_digest: backing.scene_digest,
        sample_status: boot_splash_pixel(0, 0),
        sample_cyan: boot_splash_pixel(100, 160),
        sample_purple: boot_splash_pixel(100, 230),
        sample_green: boot_splash_pixel(100, 320),
    }))
}

/// Initializes the guest-owned scene and virtual scanout identically for
/// graphical and headless boots. The first userspace FULL frame covers only
/// the phone surface, so the surrounding boot scene must already be valid.
fn initialize_backing() -> Result<BackingEvidence, DisplayError> {
    let pixels = unsafe { &mut *FRAMEBUFFER.0.get() };
    let scene = unsafe { &mut *SCENE.0.get() };
    let framebuffer_address = pixels.as_mut_ptr() as usize;
    let scene_address = scene.as_mut_ptr() as usize;
    if !framebuffer_address.is_multiple_of(4096)
        || !scene_address.is_multiple_of(4096)
        || framebuffer_address.checked_add(BYTE_COUNT).is_none()
        || scene_address.checked_add(BYTE_COUNT).is_none()
        || size_of::<u32>() != framebuffer::BYTES_PER_PIXEL
    {
        return Err(DisplayError::InvalidFramebufferAddress);
    }
    let scene_digest = render_boot_splash(scene).map_err(DisplayError::Render)?;
    let framebuffer_digest = compose_full(scene, pixels, CursorState::hidden())
        .map_err(DisplayError::Compose)?
        .scanout_digest;
    if framebuffer_digest != scene_digest {
        return Err(DisplayError::SceneMismatch);
    }
    Ok(BackingEvidence {
        framebuffer_address,
        framebuffer_digest,
        scene_address,
        scene_digest,
    })
}

/// Publishes readiness only after every backing byte and optional external
/// ramfb configuration has completed successfully.
fn finish_initialization() {
    let state = unsafe { &mut *COMPOSITOR_STATE.0.get() };
    if state.ready {
        panic!("display initialization published readiness twice");
    }
    state.cursor = CursorState::hidden();
    state.updates = 0;
    state.scene_commits = 0;
    state.surface = SurfaceSession::new();
    state.binding = None;
    state.input = SurfaceInputQueue::new();
    state.key_input = SurfaceKeyQueue::new();
    #[cfg(all(
        feature = "graphics-frame-clock-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    {
        state.frame_ungated_present_rejections = 0;
        state.frame_failed_present_preservations = 0;
    }
    state.ready = true;
}

#[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
pub fn is_ready() -> bool {
    with_display(|state, _, _| state.ready)
}

/// Reserves the only userspace surface session without changing scene owner.
/// An initial binding keeps kernel fallback rendering until its first FULL
/// frame. A recovered binding suppresses fallback scene commits immediately,
/// preserving the frozen scene until its replacement frame is accepted.
pub fn acquire_surface(
    capability: &SurfaceCapability,
    process_id: u64,
) -> Result<SurfaceAcquireEvidence, DisplayError> {
    if process_id == 0 {
        return Err(DisplayError::SurfaceProcessMismatch);
    }
    with_display(|state, scene, pixels| {
        if !state.ready {
            return Err(DisplayError::NotReady);
        }
        let mut recovered = false;
        let mut previous_session_id = 0;
        let mut previous_process_id = 0;
        let mut discarded_input = 0;
        let mut discarded_keys = 0;
        let mut frozen_scene_digest = 0;
        let mut frozen_scanout_digest = 0;
        let mut frozen_cursor = CursorState::hidden();
        let mut frozen_cursor_pixels = 0;
        let mut frozen_scanout_valid = false;
        match (state.binding.as_ref(), state.surface.owner()) {
            (None, SurfaceOwner::KernelFallback) => {}
            (Some(previous), SurfaceOwner::Degraded) => {
                // A replacement SurfaceServer is the only actor allowed to
                // leave Degraded.  Preserve the frozen pixels, retire the old
                // capability and input namespace, then install a fresh frame
                // sequence and binding as one IRQ-masked display transaction.
                // There is never an interval in which the kernel becomes a
                // second scene writer.
                if !previous.rebindable
                    || previous.process_id == process_id
                    || !previous
                        .capability
                        .signals()
                        .contains(ObjectSignals::PEER_CLOSED)
                {
                    return Err(DisplayError::SurfaceAlreadyAcquired);
                }
                recovered = true;
                previous_session_id = previous.capability.session_id();
                previous_process_id = previous.process_id;
                discarded_input = state.input.snapshot().pending;
                discarded_keys = state.key_input.snapshot().pending;
                let frozen = validate_scanout_composition(scene, pixels, state.cursor)
                    .map_err(DisplayError::Compose)?;
                if !frozen.exact {
                    return Err(DisplayError::SceneMismatch);
                }
                frozen_scene_digest = frozen.scene_digest;
                frozen_scanout_digest = frozen.scanout_digest;
                frozen_cursor = state.cursor;
                frozen_cursor_pixels = frozen.cursor_pixels;
                frozen_scanout_valid = true;
                state.surface = SurfaceSession::new();
                state.input = SurfaceInputQueue::new();
                state.key_input = SurfaceKeyQueue::new();
            }
            _ => return Err(DisplayError::SurfaceAlreadyAcquired),
        }
        #[cfg(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        ))]
        let frame_clock = {
            let mut clock = FrameClock::new();
            clock
                .arm(crate::arch::aarch64::timer::logical_tick())
                .map_err(|_| DisplayError::CounterExhausted)?;
            clock
        };
        state.binding = Some(SurfaceBinding {
            capability: capability.clone(),
            process_id,
            rebindable: false,
            awaiting_recovery_frame: recovered,
            #[cfg(all(
                feature = "graphics-frame-clock-runtime",
                not(feature = "graphics-owner-death-runtime"),
                not(feature = "app-crash-recovery-runtime")
            ))]
            frame_clock,
        });
        Ok(SurfaceAcquireEvidence {
            owner: state.surface.owner(),
            session_id: capability.session_id(),
            process_id,
            input: state.input.snapshot(),
            key_input: state.key_input.snapshot(),
            recovered,
            previous_session_id,
            previous_process_id,
            discarded_input,
            discarded_keys,
            frozen_scene_digest,
            frozen_scanout_digest,
            frozen_cursor,
            frozen_cursor_pixels,
            frozen_scanout_valid,
        })
    })
}

/// Advances the active software frame clock from the architectural timer IRQ.
/// This path only updates bounded state and a level signal; rasterization and
/// framebuffer publication remain in the presenting EL0 syscall.
#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
pub fn on_timer_tick(now: u64) {
    with_display(|state, scene, pixels| {
        if !state.ready || state.surface.owner() == SurfaceOwner::Degraded {
            return;
        }
        let Some(binding) = state.binding.as_mut() else {
            return;
        };
        let capability = binding.capability.clone();
        let update = binding.frame_clock.on_tick(now);
        match update {
            Ok(false) => {}
            Ok(true) => {
                if !capability.signal_frame_ready() {
                    // A Waiting->Ready transition must be the only producer
                    // of this level edge. Fail closed if the Event and clock
                    // ever disagree instead of risking a lost wakeup.
                    let _ = degrade_surface_locked(state, scene, pixels, false);
                }
            }
            Err(_) => {
                let _ = degrade_surface_locked(state, scene, pixels, false);
            }
        }
    });
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
pub fn frame_clock_snapshot() -> Option<DisplayFrameClockSnapshot> {
    with_display(|state, _, _| {
        let binding = state.binding.as_ref()?;
        Some(DisplayFrameClockSnapshot {
            session_id: binding.capability.session_id(),
            process_id: binding.process_id,
            clock: binding.frame_clock.snapshot(),
            ungated_present_rejections: state.frame_ungated_present_rejections,
            failed_present_preservations: state.frame_failed_present_preservations,
        })
    })
}

/// Read-only accounting for the currently bound Surface input queue.
///
/// This is acceptance-test evidence only; queue mutation remains exclusively
/// in the IRQ enqueue and authenticated SurfaceReadInput paths.
#[cfg(feature = "persistent-window-runtime")]
pub fn surface_input_snapshot() -> SurfaceInputSnapshot {
    with_display(|state, _, _| state.input.snapshot())
}

/// Read-only accounting for the hardware-key FIFO of the active Surface.
#[cfg(feature = "text-input-runtime")]
pub fn surface_key_snapshot() -> SurfaceKeySnapshot {
    with_display(|state, _, _| state.key_input.snapshot())
}

/// Atomically clears the level FRAME_READY Event and installs its exact grant.
#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
pub fn acquire_surface_frame(
    capability: &SurfaceCapability,
    process_id: u64,
) -> Result<SurfaceFrameAcquireEvidence, DisplayError> {
    with_display(|state, _, _| {
        if !state.ready {
            return Err(DisplayError::NotReady);
        }
        validate_surface_binding(state, capability, process_id)?;
        if state.surface.owner() == SurfaceOwner::Degraded {
            return Err(DisplayError::Surface(SurfaceError::Degraded));
        }
        let binding = state
            .binding
            .as_mut()
            .unwrap_or_else(|| panic!("validated surface frame binding disappeared"));
        let grant = match binding.frame_clock.acquire() {
            Ok(grant) => grant,
            Err(FrameClockAcquireError::ShouldWait) => {
                return Err(DisplayError::FrameOpportunityUnavailable);
            }
            Err(FrameClockAcquireError::AlreadyOutstanding) => {
                return Err(DisplayError::FrameGrantAlreadyOutstanding);
            }
            Err(FrameClockAcquireError::Disarmed) => {
                return Err(DisplayError::SurfaceNotAcquired);
            }
            Err(FrameClockAcquireError::CounterExhausted) => {
                return Err(DisplayError::CounterExhausted);
            }
        };
        if !binding.capability.clear_frame_ready() {
            panic!("acquired frame grant lacked its level FRAME_READY signal");
        }
        Ok(SurfaceFrameAcquireEvidence {
            session_id: capability.session_id(),
            process_id,
            grant,
        })
    })
}

/// Commits one authenticated userspace frame and publishes its already-rastered
/// scene damage while preserving the cursor layer.
///
/// Strict damage validation happens before `SurfaceSession::present` performs
/// its first pixel write. The subsequent fixed-array publish operation cannot
/// fail, so the first FULL frame is one serialized logical handoff.
pub fn present_surface(
    capability: &SurfaceCapability,
    process_id: u64,
    frame: &PresentFrame,
) -> Result<SurfacePresentEvidence, DisplayError> {
    let global_damage = surface_global_damage(frame);
    let damage = validate_scene_damage(global_damage).map_err(DisplayError::Compose)?;
    with_display(|state, scene, pixels| {
        if !state.ready {
            return Err(DisplayError::NotReady);
        }
        validate_surface_binding(state, capability, process_id)?;
        if state.surface.owner() == SurfaceOwner::Degraded {
            return Err(DisplayError::Surface(SurfaceError::Degraded));
        }
        #[cfg(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        ))]
        require_frame_grant(state, capability, process_id)?;

        let surface = match state.surface.present(&mut scene[..], frame) {
            Ok(surface) => surface,
            Err(error) => {
                #[cfg(all(
                    feature = "graphics-frame-clock-runtime",
                    not(feature = "graphics-owner-death-runtime"),
                    not(feature = "app-crash-recovery-runtime")
                ))]
                record_preserved_frame_failure(state, capability, process_id);
                return Err(DisplayError::Surface(error));
            }
        };
        #[cfg(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        ))]
        commit_frame_grant(state);
        state
            .binding
            .as_mut()
            .unwrap_or_else(|| panic!("validated surface binding disappeared after present"))
            .awaiting_recovery_frame = false;
        if surface.global_damage != damage.rect() {
            panic!("validated surface damage changed before publication");
        }
        let composition = publish_scene_damage(scene, pixels, state.cursor, damage);
        dma::publish_to_device();
        Ok(SurfacePresentEvidence {
            session_id: capability.session_id(),
            process_id,
            surface,
            composition,
            input: state.input.snapshot(),
        })
    })
}

/// Commits one authenticated, generation-pinned XRGB8888 graphics buffer.
///
/// The buffer borrow encloses the display transaction: a producer cannot
/// advance its write generation between validation and the scene copy. The
/// display transaction itself validates every fallible condition before the
/// first scene byte changes, then reuses the existing cursor-preserving
/// compositor publication and DMA barrier.
pub fn present_surface_buffer(
    capability: &SurfaceCapability,
    process_id: u64,
    buffer: &GraphicsBuffer,
    frame: &BufferPresent,
) -> Result<SurfacePresentEvidence, DisplayError> {
    let global_damage = Rect::new(
        SURFACE_ORIGIN_X,
        SURFACE_ORIGIN_Y,
        usize::from(bndr_ui::SURFACE_WIDTH),
        usize::from(bndr_ui::SURFACE_HEIGHT),
    );
    let damage = validate_scene_damage(global_damage).map_err(DisplayError::Compose)?;
    let present = |source: &[u32]| {
        with_display(|state, scene, pixels| {
            if !state.ready {
                return Err(DisplayError::NotReady);
            }
            validate_surface_binding(state, capability, process_id)?;
            if state.surface.owner() == SurfaceOwner::Degraded {
                return Err(DisplayError::Surface(SurfaceError::Degraded));
            }
            #[cfg(all(
                feature = "graphics-frame-clock-runtime",
                not(feature = "graphics-owner-death-runtime"),
                not(feature = "app-crash-recovery-runtime")
            ))]
            require_frame_grant(state, capability, process_id)?;

            let surface = match state.surface.present_buffer(&mut scene[..], frame, source) {
                Ok(surface) => surface,
                Err(error) => {
                    #[cfg(all(
                        feature = "graphics-frame-clock-runtime",
                        not(feature = "graphics-owner-death-runtime"),
                        not(feature = "app-crash-recovery-runtime")
                    ))]
                    record_preserved_frame_failure(state, capability, process_id);
                    return Err(DisplayError::Surface(error));
                }
            };
            #[cfg(all(
                feature = "graphics-frame-clock-runtime",
                not(feature = "graphics-owner-death-runtime"),
                not(feature = "app-crash-recovery-runtime")
            ))]
            commit_frame_grant(state);
            state
                .binding
                .as_mut()
                .unwrap_or_else(|| {
                    panic!("validated surface binding disappeared after buffer present")
                })
                .awaiting_recovery_frame = false;
            if surface.global_damage != damage.rect() {
                panic!("validated graphics-buffer damage changed before publication");
            }
            let composition = publish_scene_damage(scene, pixels, state.cursor, damage);
            dma::publish_to_device();
            Ok(SurfacePresentEvidence {
                session_id: capability.session_id(),
                process_id,
                surface,
                composition,
                input: state.input.snapshot(),
            })
        })
    };
    if buffer.is_mappable() {
        buffer
            .with_acquired_pixels(frame.buffer_generation(), process_id, present)
            .map_err(DisplayError::GraphicsBuffer)?
    } else {
        buffer
            .with_pixels(frame.buffer_generation(), present)
            .map_err(DisplayError::GraphicsBuffer)?
    }
}

/// Commits one client content buffer and one independently owned system-chrome
/// buffer as a single display transaction.
///
/// The graphics pool pins both exact generations until the scene copy and DMA
/// publication complete. `SurfaceSession::present_buffer_layers` validates
/// both complete rasters before changing the scene, so a stale generation,
/// malformed pixel, or sequence failure preserves the previous frame.
#[cfg(feature = "androidbox-interactive0")]
pub fn present_surface_buffer_layers(
    capability: &SurfaceCapability,
    process_id: u64,
    content_buffer: &GraphicsBuffer,
    chrome_buffer: &GraphicsBuffer,
    frame: &BufferPresent,
) -> Result<SurfacePresentEvidence, DisplayError> {
    let global_damage = Rect::new(
        SURFACE_ORIGIN_X,
        SURFACE_ORIGIN_Y,
        usize::from(bndr_ui::SURFACE_WIDTH),
        usize::from(bndr_ui::SURFACE_HEIGHT),
    );
    let damage = validate_scene_damage(global_damage).map_err(DisplayError::Compose)?;
    let present = |content: &[u32], chrome: &[u32]| {
        with_display(|state, scene, pixels| {
            if !state.ready {
                return Err(DisplayError::NotReady);
            }
            validate_surface_binding(state, capability, process_id)?;
            if state.surface.owner() == SurfaceOwner::Degraded {
                return Err(DisplayError::Surface(SurfaceError::Degraded));
            }
            #[cfg(all(
                feature = "graphics-frame-clock-runtime",
                not(feature = "graphics-owner-death-runtime"),
                not(feature = "app-crash-recovery-runtime")
            ))]
            require_frame_grant(state, capability, process_id)?;

            let surface =
                match state
                    .surface
                    .present_buffer_layers(&mut scene[..], frame, content, chrome)
                {
                    Ok(surface) => surface,
                    Err(error) => {
                        #[cfg(all(
                            feature = "graphics-frame-clock-runtime",
                            not(feature = "graphics-owner-death-runtime"),
                            not(feature = "app-crash-recovery-runtime")
                        ))]
                        record_preserved_frame_failure(state, capability, process_id);
                        return Err(DisplayError::Surface(error));
                    }
                };
            #[cfg(all(
                feature = "graphics-frame-clock-runtime",
                not(feature = "graphics-owner-death-runtime"),
                not(feature = "app-crash-recovery-runtime")
            ))]
            commit_frame_grant(state);
            state
                .binding
                .as_mut()
                .unwrap_or_else(|| {
                    panic!("validated surface binding disappeared after layered buffer present")
                })
                .awaiting_recovery_frame = false;
            if surface.global_damage != damage.rect() {
                panic!("validated layered-buffer damage changed before publication");
            }
            let composition = publish_scene_damage(scene, pixels, state.cursor, damage);
            dma::publish_to_device();
            Ok(SurfacePresentEvidence {
                session_id: capability.session_id(),
                process_id,
                surface,
                composition,
                input: state.input.snapshot(),
            })
        })
    };
    content_buffer
        .with_pixels_pair(
            frame.buffer_generation(),
            chrome_buffer,
            frame.system_chrome_generation(),
            present,
        )
        .map_err(DisplayError::GraphicsBuffer)?
}

/// Reads at most one normalized sample from the exact bound surface session.
/// Empty and degraded queues are represented by `sample=None`; the capability
/// signal bits let the syscall layer distinguish SHOULD_WAIT from PEER_CLOSED.
pub fn read_surface_input(
    capability: &SurfaceCapability,
    process_id: u64,
) -> Result<SurfaceInputReadEvidence, DisplayError> {
    with_display(|state, _, _| {
        if !state.ready {
            return Err(DisplayError::NotReady);
        }
        validate_surface_binding(state, capability, process_id)?;
        if state.surface.owner() == SurfaceOwner::Degraded {
            return Ok(SurfaceInputReadEvidence {
                owner: SurfaceOwner::Degraded,
                sample: None,
                input: state.input.snapshot(),
            });
        }
        let sample = state.input.pop().map_err(DisplayError::SurfaceInput)?;
        let input = state.input.snapshot();
        if input.pending == 0 {
            state
                .binding
                .as_ref()
                .unwrap_or_else(|| panic!("validated surface binding disappeared"))
                .capability
                .clear_readable();
        }
        Ok(SurfaceInputReadEvidence {
            owner: state.surface.owner(),
            sample,
            input,
        })
    })
}

/// Reads at most one hardware-key transition from the exact bound Surface.
/// KEY_READY is cleared only after the key FIFO becomes empty; pointer
/// READABLE is an independent level and is never touched by this operation.
pub fn read_surface_key(
    capability: &SurfaceCapability,
    process_id: u64,
) -> Result<SurfaceKeyReadEvidence, DisplayError> {
    with_display(|state, _, _| {
        if !state.ready {
            return Err(DisplayError::NotReady);
        }
        validate_surface_binding(state, capability, process_id)?;
        if state.surface.owner() == SurfaceOwner::Degraded {
            return Ok(SurfaceKeyReadEvidence {
                owner: SurfaceOwner::Degraded,
                sample: None,
                key_input: state.key_input.snapshot(),
            });
        }
        let sample = state.key_input.pop().map_err(DisplayError::SurfaceKey)?;
        let key_input = state.key_input.snapshot();
        if key_input.pending == 0 {
            state
                .binding
                .as_ref()
                .unwrap_or_else(|| panic!("validated surface binding disappeared"))
                .capability
                .clear_key_ready();
        }
        Ok(SurfaceKeyReadEvidence {
            owner: state.surface.owner(),
            sample,
            key_input,
        })
    })
}

/// Freezes the last scene for an explicitly closed or failed capability.
pub fn degrade_surface(
    capability: &SurfaceCapability,
    process_id: u64,
) -> Result<SurfaceDegradeEvidence, DisplayError> {
    with_display(|state, scene, pixels| {
        if !state.ready {
            return Err(DisplayError::NotReady);
        }
        validate_surface_binding(state, capability, process_id)?;
        Ok(degrade_surface_locked(state, scene, pixels, false))
    })
}

/// Trusted process-reaper variant. An unrelated generation-qualified PID is a
/// no-op, while the exact bound process transitions the session to Degraded.
#[allow(dead_code)]
pub fn degrade_surface_for_process(
    process_id: u64,
) -> Result<Option<SurfaceDegradeEvidence>, DisplayError> {
    with_display(|state, scene, pixels| {
        if !state.ready {
            return Err(DisplayError::NotReady);
        }
        let Some(binding) = state.binding.as_ref() else {
            return Ok(None);
        };
        if binding.process_id != process_id {
            return Ok(None);
        }
        if state.surface.owner() == SurfaceOwner::Degraded {
            state
                .binding
                .as_mut()
                .unwrap_or_else(|| panic!("degraded surface binding disappeared"))
                .rebindable = true;
            return Ok(None);
        }
        Ok(Some(degrade_surface_locked(state, scene, pixels, true)))
    })
}

/// Routes one validated EV_KEY transition to the unique live Surface binding.
/// The driver calls this only after SYN_REPORT has committed the report. Raw
/// events remain in the virtio-input monitor queue, so this is a producer-side
/// tap rather than a second consumer.
pub fn process_key_transition(code: u16, value: u8) -> Result<KeyRouteEvidence, DisplayError> {
    with_display(|state, _, _| {
        if !state.ready {
            return Err(DisplayError::NotReady);
        }
        match state.surface.owner() {
            SurfaceOwner::KernelFallback
                if state
                    .binding
                    .as_ref()
                    .is_some_and(|binding| binding.awaiting_recovery_frame) =>
            {
                Ok(KeyRouteEvidence::Degraded)
            }
            SurfaceOwner::KernelFallback => Ok(KeyRouteEvidence::KernelFallback),
            SurfaceOwner::UserspaceBound => match state.key_input.push(code, value) {
                Ok(sample) => {
                    let key_ready_edge = state
                        .binding
                        .as_ref()
                        .unwrap_or_else(|| panic!("userspace-owned surface has no binding"))
                        .capability
                        .signal_key_ready();
                    Ok(KeyRouteEvidence::UserspaceBound {
                        sample,
                        key_input: state.key_input.snapshot(),
                        key_ready_edge,
                    })
                }
                Err(error @ SurfaceKeyError::Full) => Ok(KeyRouteEvidence::UserspaceRejected {
                    error,
                    key_input: state.key_input.snapshot(),
                }),
                Err(error) => Err(DisplayError::SurfaceKey(error)),
            },
            SurfaceOwner::Degraded => Ok(KeyRouteEvidence::Degraded),
        }
    })
}

/// Atomically updates the cursor and routes one normalized pointer report
/// according to the current scene owner.
///
/// Kernel fallback hit testing and its optional scene commit are inside the
/// same owner-check critical section. After handoff the kernel never advances
/// that controller and instead queues the report for the bound SurfaceServer.
#[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
pub fn process_pointer_sample(
    next: CursorState,
    fallback_ui: &mut UiController,
) -> Result<PointerProcessEvidence, DisplayError> {
    validate_cursor(next)?;
    if !next.visible {
        return Err(DisplayError::InvalidPointerSample);
    }
    with_display(|state, scene, pixels| {
        if !state.ready {
            return Err(DisplayError::NotReady);
        }
        let updates = next_counter(state.updates)?;
        let route = match state.surface.owner() {
            SurfaceOwner::KernelFallback
                if state
                    .binding
                    .as_ref()
                    .is_some_and(|binding| binding.awaiting_recovery_frame) =>
            {
                // A replacement has reserved the recovered namespace but has
                // not published its first valid frame. Preserve the scene and
                // use the same cursor-only/drop-input behavior as Degraded.
                PointerRouteEvidence::Degraded
            }
            SurfaceOwner::KernelFallback => {
                // Preflight the only additional display counter before the UI
                // controller can publish a transition of its own.
                let _ = next_counter(state.scene_commits)?;
                let cursor = update_cursor_locked(state, scene, pixels, next, updates);
                let ui = fallback_ui
                    .observe(next.x, next.y, next.pressed)
                    .map(|commit| {
                        let scene_evidence =
                            commit_ui_locked(state, scene, pixels, commit.to, commit.damage)
                                .unwrap_or_else(|_| {
                                    panic!(
                                        "validated fallback UI transition failed during composition"
                                    )
                                });
                        FallbackUiEvidence {
                            commit,
                            scene: scene_evidence,
                        }
                    });
                dma::publish_to_device();
                return Ok(PointerProcessEvidence {
                    cursor,
                    route: PointerRouteEvidence::KernelFallback { ui },
                });
            }
            SurfaceOwner::UserspaceBound => {
                let x = u16::try_from(next.x).map_err(|_| DisplayError::InvalidPointerSample)?;
                let y = u16::try_from(next.y).map_err(|_| DisplayError::InvalidPointerSample)?;
                let enqueued = state
                    .input
                    .push(x, y, next.pressed)
                    .map_err(DisplayError::SurfaceInput)?;
                let readable_edge = state
                    .binding
                    .as_ref()
                    .unwrap_or_else(|| panic!("userspace-owned surface has no binding"))
                    .capability
                    .signal_readable();
                PointerRouteEvidence::UserspaceBound {
                    sample: enqueued.sample,
                    input: state.input.snapshot(),
                    readable_edge,
                    coalesced: enqueued.coalesced,
                }
            }
            SurfaceOwner::Degraded => PointerRouteEvidence::Degraded,
        };
        let cursor = update_cursor_locked(state, scene, pixels, next, updates);
        dma::publish_to_device();
        Ok(PointerProcessEvidence { cursor, route })
    })
}

/// Publishes the visual cursor for a physical pointer sample without routing
/// that sample through the retired Surface input FIFO.
///
/// With a dedicated InputServer, the authenticated input broker is the sole
/// routing path.  Cursor composition remains a display concern: updating this
/// independent scanout layer here lets every later Surface present preserve
/// the latest physical pointer while leaving focus, capture, and delivery to
/// InputServer.
#[cfg(feature = "input-server-runtime")]
pub fn update_brokered_pointer_cursor(
    next: CursorState,
) -> Result<CursorUpdateEvidence, DisplayError> {
    validate_cursor(next)?;
    if !next.visible {
        return Err(DisplayError::InvalidPointerSample);
    }
    with_display(|state, scene, pixels| {
        if !state.ready {
            return Err(DisplayError::NotReady);
        }
        let updates = next_counter(state.updates)?;
        let cursor = update_cursor_locked(state, scene, pixels, next, updates);
        dma::publish_to_device();
        Ok(cursor)
    })
}

#[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
fn validate_cursor(next: CursorState) -> Result<(), DisplayError> {
    if next.visible && (next.x >= WIDTH || next.y >= HEIGHT) {
        Err(DisplayError::InvalidCursorCoordinate)
    } else {
        Ok(())
    }
}

#[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
fn next_counter(counter: u64) -> Result<u64, DisplayError> {
    counter.checked_add(1).ok_or(DisplayError::CounterExhausted)
}

#[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
fn update_cursor_locked(
    state: &mut CompositorState,
    scene: &[u32; PIXEL_COUNT],
    pixels: &mut [u32; PIXEL_COUNT],
    next: CursorState,
    updates: u64,
) -> CursorUpdateEvidence {
    let previous = state.cursor;
    let composition = compose_cursor(scene, pixels, previous, next)
        .unwrap_or_else(|_| panic!("fixed display arrays failed cursor composition"));
    state.cursor = next;
    state.updates = updates;
    CursorUpdateEvidence {
        previous,
        current: next,
        composition,
        updates,
    }
}

#[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
fn commit_ui_locked(
    state: &mut CompositorState,
    scene: &mut [u32; PIXEL_COUNT],
    pixels: &mut [u32; PIXEL_COUNT],
    view: UiView,
    damage: Rect,
) -> Result<SceneCommitEvidence, DisplayError> {
    if !state.ready {
        return Err(DisplayError::NotReady);
    }
    if state.surface.owner() != SurfaceOwner::KernelFallback
        || state
            .binding
            .as_ref()
            .is_some_and(|binding| binding.awaiting_recovery_frame)
    {
        return Err(DisplayError::SceneWriterOwnedByUserspace);
    }
    let commits = next_counter(state.scene_commits)?;
    let composition = compose_scene(scene, pixels, state.cursor, damage, |x, y| {
        scene_pixel(view, x, y)
    })
    .map_err(DisplayError::Compose)?;
    state.scene_commits = commits;
    Ok(SceneCommitEvidence {
        composition,
        commits,
    })
}

fn validate_surface_binding(
    state: &CompositorState,
    capability: &SurfaceCapability,
    process_id: u64,
) -> Result<(), DisplayError> {
    let Some(binding) = state.binding.as_ref() else {
        return Err(DisplayError::SurfaceNotAcquired);
    };
    if !binding.capability.same_surface(capability) {
        return Err(DisplayError::SurfaceCapabilityMismatch);
    }
    if process_id == 0 || binding.process_id != process_id {
        return Err(DisplayError::SurfaceProcessMismatch);
    }
    Ok(())
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn require_frame_grant(
    state: &mut CompositorState,
    capability: &SurfaceCapability,
    process_id: u64,
) -> Result<(), DisplayError> {
    let binding = state
        .binding
        .as_ref()
        .unwrap_or_else(|| panic!("validated paced surface binding disappeared"));
    let snapshot = binding.frame_clock.snapshot();
    if snapshot.phase == FrameClockPhase::Outstanding {
        // Preflight the only fallible clock mutation before SurfaceSession
        // writes its first scene pixel. The subsequent checked increment is
        // therefore guaranteed to succeed at the serialized commit point.
        if snapshot.presented == u64::MAX {
            return Err(DisplayError::CounterExhausted);
        }
        return Ok(());
    }
    state.frame_ungated_present_rejections = state
        .frame_ungated_present_rejections
        .checked_add(1)
        .ok_or(DisplayError::CounterExhausted)?;
    crate::kprintln!(
        "SURFACE_FRAME_PRESENT_REJECT_OK reason=no-grant atomic=1 pid={} session={} count={}",
        process_id,
        capability.session_id(),
        state.frame_ungated_present_rejections,
    );
    Err(DisplayError::FrameGrantRequired)
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn record_preserved_frame_failure(
    state: &mut CompositorState,
    capability: &SurfaceCapability,
    process_id: u64,
) {
    let binding = state
        .binding
        .as_ref()
        .unwrap_or_else(|| panic!("paced surface binding disappeared after failed validation"));
    let grant = binding
        .frame_clock
        .present_failed()
        .unwrap_or_else(|error| {
            panic!("failed paced present did not preserve its grant: {error:?}")
        });
    state.frame_failed_present_preservations = state
        .frame_failed_present_preservations
        .checked_add(1)
        .unwrap_or_else(|| panic!("paced present preservation counter exhausted"));
    crate::kprintln!(
        "SURFACE_FRAME_PRESENT_ABORT_OK grant_preserved=1 pid={} session={} epoch={} count={}",
        process_id,
        capability.session_id(),
        grant.epoch,
        state.frame_failed_present_preservations,
    );
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn commit_frame_grant(state: &mut CompositorState) {
    let binding = state
        .binding
        .as_mut()
        .unwrap_or_else(|| panic!("paced surface binding disappeared at commit"));
    binding
        .frame_clock
        .present_succeeded()
        .unwrap_or_else(|error| match error {
            FrameClockPresentError::Disarmed
            | FrameClockPresentError::NoOutstandingGrant
            | FrameClockPresentError::CounterExhausted => {
                panic!("prevalidated paced frame commit failed: {error:?}")
            }
        });
}

fn surface_global_damage(frame: &PresentFrame) -> Rect {
    let local = frame.damage_rect();
    Rect::new(
        SURFACE_ORIGIN_X + usize::from(local.x),
        SURFACE_ORIGIN_Y + usize::from(local.y),
        usize::from(local.width),
        usize::from(local.height),
    )
}

fn degrade_surface_locked(
    state: &mut CompositorState,
    scene: &[u32; PIXEL_COUNT],
    pixels: &[u32; PIXEL_COUNT],
    rebindable: bool,
) -> SurfaceDegradeEvidence {
    let binding = state
        .binding
        .as_ref()
        .unwrap_or_else(|| panic!("surface degradation lacked a binding"));
    let session_id = binding.capability.session_id();
    let process_id = binding.process_id;
    let previous_owner = state.surface.owner();
    #[cfg(all(
        feature = "graphics-frame-clock-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    {
        let binding = state
            .binding
            .as_mut()
            .unwrap_or_else(|| panic!("surface binding disappeared during clock degradation"));
        // Counter overflow already poisons the clock to Disarmed. Every other
        // path accounts a pending grant as discarded or an outstanding grant
        // as cancelled before PEER_CLOSED becomes observable.
        let _ = binding.frame_clock.degrade();
        binding.capability.clear_frame_ready();
    }
    state.surface.mark_degraded();
    state
        .binding
        .as_mut()
        .unwrap_or_else(|| panic!("surface binding disappeared during degradation"))
        .rebindable = rebindable;
    let peer_closed_edge = state
        .binding
        .as_ref()
        .unwrap_or_else(|| panic!("surface binding disappeared during degradation"))
        .capability
        .mark_degraded();
    SurfaceDegradeEvidence {
        previous_owner,
        current_owner: state.surface.owner(),
        session_id,
        process_id,
        last_frame_id: state.surface.last_frame_id(),
        commits: state.surface.commits(),
        scene_digest: pixel_digest(scene),
        scanout_digest: pixel_digest(pixels),
        input: state.input.snapshot(),
        key_input: state.key_input.snapshot(),
        peer_closed_edge,
    }
}

struct DisplayBorrowGuard {
    saved_daif: u64,
}

impl DisplayBorrowGuard {
    fn acquire() -> Self {
        let saved_daif = aarch64::save_and_mask_irq();
        if DISPLAY_BORROWED
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            aarch64::restore_daif(saved_daif);
            panic!("display critical section was re-entered");
        }
        Self { saved_daif }
    }
}

impl Drop for DisplayBorrowGuard {
    fn drop(&mut self) {
        if !DISPLAY_BORROWED.swap(false, Ordering::Release) {
            panic!("display critical-section guard lost its ownership");
        }
        aarch64::restore_daif(self.saved_daif);
    }
}

fn with_display<R>(
    operation: impl FnOnce(&mut CompositorState, &mut [u32; PIXEL_COUNT], &mut [u32; PIXEL_COUNT]) -> R,
) -> R {
    let _guard = DisplayBorrowGuard::acquire();
    let state = unsafe { &mut *COMPOSITOR_STATE.0.get() };
    let scene = unsafe { &mut *SCENE.0.get() };
    let pixels = unsafe { &mut *FRAMEBUFFER.0.get() };
    operation(state, scene, pixels)
}
