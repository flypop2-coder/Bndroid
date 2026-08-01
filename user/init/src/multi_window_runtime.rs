//! M41 bounded userspace window-compositor runtime.
//!
//! This module is a child of `lifecycle_runtime`, so it can reuse the already
//! authenticated lifecycle/bootstrap helpers while keeping the M33--M40
//! profiles byte-for-byte isolated. Window policy stays in SurfaceServer;
//! the kernel continues to see only one Surface and its final output queue.

use super::*;

use core::cell::UnsafeCell;
#[cfg(feature = "app-data-async-recovery-runtime")]
use core::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "text-input-runtime")]
use bndr_abi::KeyInputSample;
#[cfg(feature = "app-data-runtime")]
use bndr_abi::{
    APP_DATA_DIRECTORY_ENTRY_WIRE_SIZE, APP_DATA_DIRECTORY_MAX_ENTRIES, AppDataDirectoryEntry,
    AppDataDirectoryEntryKind, AppDataPrincipal, FILE_REPLACE_REQUEST_WIRE_SIZE, FileReplaceCas,
    FileReplaceRequest,
};
use bndr_compositor::{Compositor, PixelLedger, PointerRoute};
#[cfg(all(
    feature = "soft-keyboard-runtime",
    not(feature = "input-server-runtime")
))]
use bndr_input::InputMethodEngine;
#[cfg(feature = "soft-keyboard-runtime")]
use bndr_input::{
    A_KEY_RECT, BACKSPACE_KEY_RECT, ENTER_KEY_RECT, InputAction, InputStatistics,
    TRUSTED_OVERLAY_RECT,
};
#[cfg(feature = "input-server-runtime")]
use bndr_input::{
    CommandStream as InputCommandStream, InputCommand, InputEvent as ServerInputEvent,
    InputEventPayload as ServerInputPayload, InputKey, InputRoute, InputWindowId, OwnerPid,
    RouteBounds, TextContext as ServerTextContext, TextPurpose, WindowRef,
};
#[cfg(any(
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
use bndr_input::{
    InputServiceFocus, InputServiceRecoveryMessage, InputServiceRecoveryPayload,
    InputServiceRecoveryPhase, InputServiceRecoverySequenceTracker,
};
#[cfg(feature = "service-supervisor-runtime")]
use bndr_sm::health::{FaultClass, ServiceIdentity};
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
use bndr_sm::health::{HealthFrame, HealthOpcode, ServiceKind};
#[cfg(any(
    feature = "service-dependency-runtime",
    all(
        feature = "input-server-surface-restart-runtime",
        not(feature = "input-server-restart-runtime")
    )
))]
use bndr_ui::{
    RecoveryCancelKind, RecoveryClientRole, SurfaceRecoveryCheckpoint, SurfaceRecoveryMessage,
    SurfaceRecoveryPayload, SurfaceRecoveryPhase,
};
use bndr_ui::{
    ShellRect, WINDOW_COMMAND_WIRE_SIZE, WINDOW_EVENT_WIRE_SIZE, WindowCommand,
    WindowCommandPayload, WindowCommandTracker, WindowEvent, WindowEventPayload,
    WindowEventTracker, WindowId,
};
#[cfg(feature = "text-input-runtime")]
use bndr_ui::{
    TEXT_INPUT_COMMAND_WIRE_SIZE, TEXT_INPUT_EVENT_WIRE_SIZE, TextEditor, TextInputClientTracker,
    TextInputCommand, TextInputCommandPayload, TextInputContext, TextInputEvent,
    TextInputEventPayload, TextInputServerTracker, TextInputSessionPhase, TextState,
};

pub(super) const READY_MAGIC: u64 = 0x4d34_315f_5749_4e44;
const PHASE_TWO_ROUTE_ACK_MAGIC: u64 = 0x4d34_315f_5231_4f4b;

const LAUNCHER_BOUNDS: ShellRect = ShellRect::new(0, 0, 208, 368);
const APP_BOUNDS: ShellRect = ShellRect::new(48, 80, 112, 160);
const LAUNCHER_HIDDEN_DAMAGE: ShellRect = ShellRect::new(64, 96, 32, 32);
const LAUNCHER_PARTIAL_DAMAGE: ShellRect = ShellRect::new(32, 64, 64, 64);
const APP_DAMAGE: ShellRect = ShellRect::new(16, 24, 40, 32);
#[cfg(feature = "persistent-window-runtime")]
const APP_PIXEL_COUNT: u32 = 112 * 160;
#[cfg(feature = "text-input-runtime")]
const TEXT_FIELD: ShellRect = ShellRect::new(8, 16, 96, 16);
#[cfg(feature = "text-input-runtime")]
const TEXT_FIELD_PIXEL_COUNT: usize = 96 * 16;
#[cfg(feature = "soft-keyboard-runtime")]
const SOFT_KEYBOARD_BOUNDS: ShellRect = ShellRect::new(0, 272, 208, 96);
#[cfg(feature = "soft-keyboard-runtime")]
const SOFT_KEYBOARD_PIXEL_COUNT: usize = 208 * 96;

const COLOR_LAUNCHER_BASE: u32 = 0x0016_2436;
const COLOR_APP_BASE: u32 = 0x0038_5a7c;
const COLOR_LAUNCHER_HIDDEN: u32 = 0x00d1_6b45;
const COLOR_LAUNCHER_PARTIAL: u32 = 0x0029_a36a;
const COLOR_APP_DAMAGE: u32 = 0x0084_4ec7;
#[cfg(feature = "post-recovery-interaction-runtime")]
const POST_RECOVERY_APP_DAMAGE: ShellRect = ShellRect::new(56, 72, 40, 32);
#[cfg(feature = "post-recovery-interaction-runtime")]
const COLOR_POST_RECOVERY_APP: u32 = 0x00fa_cc15;
#[cfg(feature = "post-recovery-focus-runtime")]
const POST_RECOVERY_LAUNCHER_DAMAGE: ShellRect = ShellRect::new(8, 16, 40, 32);
#[cfg(feature = "post-recovery-focus-runtime")]
const COLOR_POST_RECOVERY_LAUNCHER: u32 = 0x0034_c759;
#[cfg(feature = "post-recovery-focus-runtime")]
const POST_RECOVERY_LAUNCHER_ACK_MAGIC: u64 = 0x4d35_315f_4c41_434b;
#[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
const POST_RECOVERY_ROUNDTRIP_APP_ACK_MAGIC: u64 = 0x4d35_325f_4150_434b;
#[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
const POST_RECOVERY_READY_FOCUS_ACK_MAGIC: u64 = 0x4d35_335f_5246_434b;
#[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
const POST_RECOVERY_LAUNCHER_FOCUS_ACK_MAGIC: u64 = 0x4d35_335f_4c46_434b;
#[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
const POST_RECOVERY_APP_FOCUS_ACK_MAGIC: u64 = 0x4d35_335f_4146_434b;
#[cfg(feature = "app-data-runtime")]
const FAIL_APP_DATA_ROOT: u64 = 257;
#[cfg(feature = "app-data-runtime")]
const FAIL_APP_DATA_CAPABILITY: u64 = 258;
#[cfg(feature = "app-data-runtime")]
const FAIL_APP_DATA_LOCAL_REJECTION: u64 = 259;
#[cfg(feature = "app-data-runtime")]
const FAIL_APP_DATA_DIRECTORY: u64 = 260;
#[cfg(feature = "app-data-runtime")]
const FAIL_APP_DATA_REPLACE: u64 = 261;
#[cfg(feature = "app-data-runtime")]
const FAIL_APP_DATA_READ: u64 = 262;
#[cfg(feature = "app-data-runtime")]
const FAIL_APP_DATA_CAS: u64 = 263;
#[cfg(feature = "app-data-runtime")]
const FAIL_APP_DATA_CLEANUP: u64 = 264;
#[cfg(feature = "app-data-runtime")]
const APP_DATA_RETRY_LIMIT: usize = 4_096;
#[cfg(feature = "app-data-async-recovery-runtime")]
static APP_DATA_UNAVAILABLE_RETRIES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
const APP_DATA_STATE_DIRECTORY: &[u8] = b"state";
#[cfg(feature = "app-data-runtime")]
const APP_DATA_SESSION_PATH: &[u8] = b"state/session.bin";
#[cfg(feature = "app-data-runtime")]
const APP_DATA_MISSING_PATH: &[u8] = b"state/missing.bin";
#[cfg(feature = "app-data-runtime")]
const APP_DATA_V1_PAYLOAD: &[u8] = b"BNDR-APPDATA-SESSION-v1\n";
#[cfg(feature = "app-data-runtime")]
const APP_DATA_V2_PAYLOAD: &[u8] = b"BNDR-APPDATA-SESSION-v2\n";
#[cfg(any(
    feature = "service-dependency-runtime",
    all(
        feature = "input-server-surface-restart-runtime",
        not(feature = "input-server-restart-runtime")
    )
))]
const SURFACE_RECOVERY_DAMAGE: ShellRect = ShellRect::new(8, 8, 32, 24);
#[cfg(any(
    feature = "service-dependency-runtime",
    all(
        feature = "input-server-surface-restart-runtime",
        not(feature = "input-server-restart-runtime")
    )
))]
const COLOR_SURFACE_RECOVERY_CAPTURED: u32 = 0x00f9_7300;
#[cfg(any(
    feature = "service-dependency-runtime",
    all(
        feature = "input-server-surface-restart-runtime",
        not(feature = "input-server-restart-runtime")
    )
))]
const COLOR_SURFACE_RECOVERY_ACTIVE: u32 = 0x0014_b8a6;
#[cfg(any(
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
const INPUT_RESTART_DAMAGE: ShellRect = ShellRect::new(8, 8, 32, 24);
#[cfg(feature = "input-server-restart-runtime")]
const COLOR_INPUT_RESTART_GAP: u32 = 0x00f9_7300;
#[cfg(feature = "input-server-restart-runtime")]
const COLOR_INPUT_RESTART_ACTIVE: u32 = 0x0014_b8a6;
#[cfg(feature = "service-dependency-runtime")]
const COLOR_SERVICE_DEPENDENCY_ACTIVE: u32 = 0x0022_c55e;
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
const INPUT_DEGRADED_DAMAGE: ShellRect = ShellRect::new(8, 8, 32, 24);
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
const COLOR_INPUT_DEGRADED: u32 = 0x00dc_2626;
#[cfg(feature = "service-dependency-runtime")]
const SERVICE_HEALTH_TIMEOUT_NS: u64 = 100_000_000;
#[cfg(feature = "persistent-window-runtime")]
const COLOR_APP_RECREATED: u32 = 0x00b2_6b36;
#[cfg(feature = "text-input-runtime")]
const COLOR_TEXT_FIELD: u32 = 0x0011_1118;
#[cfg(feature = "text-input-runtime")]
const COLOR_TEXT_COMMITTED: u32 = 0x00f4_f4f5;
#[cfg(feature = "text-input-runtime")]
const COLOR_TEXT_PREEDIT: u32 = 0x00fa_cc15;
#[cfg(feature = "text-input-runtime")]
const TEXT_FIELD_PREEDIT_DIGEST: u64 = 0x90a6_0d86_afe0_de65;
#[cfg(feature = "text-input-runtime")]
const TEXT_FIELD_COMMITTED_DIGEST: u64 = 0x6e9b_51d9_b30b_78a5;
#[cfg(feature = "text-input-runtime")]
const TEXT_FIELD_EMPTY_DIGEST: u64 = 0x474f_aa23_8cb1_5725;
#[cfg(feature = "soft-keyboard-runtime")]
const COLOR_SOFT_KEYBOARD: u32 = 0x0018_1b24;
#[cfg(feature = "soft-keyboard-runtime")]
const COLOR_SOFT_KEY_A: u32 = 0x003b_82f6;
#[cfg(feature = "soft-keyboard-runtime")]
const COLOR_SOFT_KEY_BACKSPACE: u32 = 0x00ef_4444;
#[cfg(feature = "soft-keyboard-runtime")]
const COLOR_SOFT_KEY_ENTER: u32 = 0x0022_c55e;
#[cfg(feature = "text-input-runtime")]
const TEXT_KEY_BACKSPACE: u16 = 14;
#[cfg(feature = "text-input-runtime")]
const TEXT_KEY_ENTER: u16 = 28;
#[cfg(feature = "text-input-runtime")]
const TEXT_KEY_A: u16 = 30;

const SURFACE_PIXEL_COUNT: usize = GRAPHICS_BUFFER_PIXEL_COUNT;
const PHONE_OFFSET_X: u16 = 56;
const PHONE_OFFSET_Y: u16 = 64;

#[repr(align(4096))]
struct AlignedPixels(UnsafeCell<[u32; SURFACE_PIXEL_COUNT]>);

// Only the single authenticated SurfaceServer touches these stores. Keeping
// them out of its 16 KiB kernel stack and out of the 262 KiB kernel heap is a
// deliberate part of the bounded compositor design.
unsafe impl Sync for AlignedPixels {}

static LAUNCHER_LAYER: AlignedPixels = AlignedPixels(UnsafeCell::new([0; SURFACE_PIXEL_COUNT]));
static APP_LAYER: AlignedPixels = AlignedPixels(UnsafeCell::new([0; SURFACE_PIXEL_COUNT]));

#[cfg(feature = "text-input-runtime")]
#[repr(align(64))]
struct TextFieldPixels(UnsafeCell<[u32; TEXT_FIELD_PIXEL_COUNT]>);

// The authenticated SurfaceServer is the sole writer and consumes this
// tightly packed source synchronously in `Compositor::apply_pixels`.
#[cfg(feature = "text-input-runtime")]
unsafe impl Sync for TextFieldPixels {}

#[cfg(feature = "text-input-runtime")]
static TEXT_FIELD_PIXELS: TextFieldPixels =
    TextFieldPixels(UnsafeCell::new([COLOR_TEXT_FIELD; TEXT_FIELD_PIXEL_COUNT]));

#[cfg(feature = "soft-keyboard-runtime")]
#[repr(align(64))]
struct SoftKeyboardPixels(UnsafeCell<[u32; SOFT_KEYBOARD_PIXEL_COUNT]>);

// SurfaceServer owns this retained trusted overlay for the lifetime of the
// M44 leaf runtime. It is never shared with a client process.
#[cfg(feature = "soft-keyboard-runtime")]
unsafe impl Sync for SoftKeyboardPixels {}

#[cfg(feature = "soft-keyboard-runtime")]
static SOFT_KEYBOARD_PIXELS: SoftKeyboardPixels = SoftKeyboardPixels(UnsafeCell::new(
    [COLOR_SOFT_KEYBOARD; SOFT_KEYBOARD_PIXEL_COUNT],
));

#[cfg(feature = "soft-keyboard-runtime")]
fn create_soft_keyboard_layer() -> &'static mut [u32] {
    let pixels = unsafe { &mut *SOFT_KEYBOARD_PIXELS.0.get() };
    pixels.fill(COLOR_SOFT_KEYBOARD);
    paint_soft_key(pixels, A_KEY_RECT, COLOR_SOFT_KEY_A);
    paint_soft_key(pixels, BACKSPACE_KEY_RECT, COLOR_SOFT_KEY_BACKSPACE);
    paint_soft_key(pixels, ENTER_KEY_RECT, COLOR_SOFT_KEY_ENTER);
    pixels
}

#[cfg(feature = "soft-keyboard-runtime")]
fn paint_soft_key(pixels: &mut [u32; SOFT_KEYBOARD_PIXEL_COUNT], key: ShellRect, color: u32) {
    let local_x = usize::from(
        key.x
            .checked_sub(TRUSTED_OVERLAY_RECT.x)
            .unwrap_or_else(|| fail(FAIL_RUNTIME)),
    );
    let local_y = usize::from(
        key.y
            .checked_sub(TRUSTED_OVERLAY_RECT.y)
            .unwrap_or_else(|| fail(FAIL_RUNTIME)),
    );
    for y in local_y..local_y + usize::from(key.height) {
        let start = y * usize::from(SOFT_KEYBOARD_BOUNDS.width) + local_x;
        pixels[start..start + usize::from(key.width)].fill(color);
    }
}

#[cfg(feature = "text-input-runtime")]
fn render_text_field(committed: &[u8], preedit: Option<u32>) -> &'static [u32] {
    if committed.len() > 8 || committed.iter().any(|byte| *byte != b'a') {
        fail(FAIL_RUNTIME);
    }
    if preedit.is_some_and(|scalar| scalar != u32::from(b'a'))
        || committed.len() + usize::from(preedit.is_some()) > 8
    {
        fail(FAIL_RUNTIME);
    }

    let pixels = unsafe { &mut *TEXT_FIELD_PIXELS.0.get() };
    pixels.fill(COLOR_TEXT_FIELD);
    for index in 0..committed.len() {
        draw_text_a(pixels, index, COLOR_TEXT_COMMITTED);
    }
    if preedit.is_some() {
        draw_text_a(pixels, committed.len(), COLOR_TEXT_PREEDIT);
    }
    pixels
}

#[cfg(feature = "text-input-runtime")]
fn draw_text_a(pixels: &mut [u32; TEXT_FIELD_PIXEL_COUNT], character: usize, color: u32) {
    // Eight 5x8 bitmap cells fit exactly when rendered at 2x with a two-pixel
    // inter-character gap. The glyph has sixteen set cells (64 raster pixels).
    const GLYPH_A: [u8; 8] = [
        0b00000, 0b01110, 0b10001, 0b00001, 0b01111, 0b10001, 0b01111, 0,
    ];
    let origin_x = character * 12;
    for (row, bits) in GLYPH_A.into_iter().enumerate() {
        for column in 0..5 {
            if bits & (1 << (4 - column)) == 0 {
                continue;
            }
            let x = origin_x + column * 2;
            let y = row * 2;
            for dy in 0..2 {
                let start = (y + dy) * usize::from(TEXT_FIELD.width) + x;
                pixels[start] = color;
                pixels[start + 1] = color;
            }
        }
    }
}

#[cfg(feature = "text-input-runtime")]
fn text_field_digest(pixels: &[u32]) -> u64 {
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    for pixel in pixels {
        for byte in pixel.to_le_bytes() {
            digest ^= u64::from(byte);
            digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    digest
}

pub(super) struct OutputBuffer {
    producer: OwnedUserHandle,
    server: OwnedUserHandle,
    mapping: u64,
    write_generation: u64,
}

impl OutputBuffer {
    fn create() -> Self {
        let created = syscall(
            SyscallNumber::GraphicsBufferCreate,
            GRAPHICS_BUFFER_FORMAT_XRGB8888,
            pack_graphics_buffer_geometry(GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_HEIGHT),
            GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE,
        );
        if created.status != Status::Ok.raw()
            || created.out1 == 0
            || created.out2 != GRAPHICS_BUFFER_LOGICAL_BYTES as u64
        {
            fail(FAIL_RUNTIME);
        }
        let producer = OwnedUserHandle::new(created.out1).unwrap_or_else(|| fail(FAIL_RUNTIME));
        let duplicated = syscall(
            SyscallNumber::HandleDuplicate,
            producer.raw(),
            u64::from(Rights::GRAPHICS_BUFFER_MAPPED_SERVER.bits()),
            0,
        );
        if duplicated.status != Status::Ok.raw() || duplicated.out1 == 0 || duplicated.out2 != 0 {
            fail(FAIL_RUNTIME);
        }
        let server = OwnedUserHandle::new(duplicated.out1).unwrap_or_else(|| fail(FAIL_RUNTIME));
        let mapping = map_graphics_buffer(producer.raw(), GRAPHICS_BUFFER_MAP_PRODUCER_RW);
        Self {
            producer,
            server,
            mapping,
            write_generation: 0,
        }
    }

    fn pixels_mut(&mut self) -> &mut [u32; SURFACE_PIXEL_COUNT] {
        let address = usize::try_from(self.mapping).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        if !address.is_multiple_of(4096) {
            fail(FAIL_RUNTIME);
        }
        unsafe { &mut *(address as *mut [u32; SURFACE_PIXEL_COUNT]) }
    }

    fn queue_and_acquire(&mut self) -> u64 {
        let queued = queue_graphics_buffer(self.producer.raw(), self.write_generation);
        acquire_graphics_buffer(self.server.raw(), queued);
        self.write_generation = queued;
        queued
    }
}

fn create_output_swapchain() -> [OutputBuffer; 2] {
    let first = OutputBuffer::create();
    let second = OutputBuffer::create();
    if first.producer.raw() == second.producer.raw()
        || first.server.raw() == second.server.raw()
        || first.mapping == second.mapping
    {
        fail(FAIL_RUNTIME);
    }
    let exhausted = syscall(
        SyscallNumber::GraphicsBufferCreate,
        GRAPHICS_BUFFER_FORMAT_XRGB8888,
        pack_graphics_buffer_geometry(GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_HEIGHT),
        GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE,
    );
    if exhausted.status != Status::OutOfMemory.raw() || exhausted.out1 != 0 || exhausted.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    [first, second]
}

fn acquire_frame_grant(capability: u64, previous_epoch: &mut u64, previous_boundary: &mut u64) {
    let ready = object_wait(capability, ObjectSignals::FRAME_READY);
    if ready.status != Status::Ok.raw()
        || ready.out1 & u64::from(ObjectSignals::FRAME_READY.bits()) == 0
        || ready.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
    let acquired = syscall(SyscallNumber::SurfaceFrameAcquire, capability, 0, 0);
    let expected_epoch = previous_epoch
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if acquired.status != Status::Ok.raw()
        || acquired.out1 != expected_epoch
        || acquired.out2 == 0
        || acquired.out2 <= *previous_boundary
    {
        fail(FAIL_RUNTIME);
    }
    *previous_epoch = acquired.out1;
    *previous_boundary = acquired.out2;
}

fn present_output(
    capability: u64,
    output: &mut OutputBuffer,
    global_frame: u32,
    focus_generation: u64,
) {
    let generation = output.queue_and_acquire();
    let focus = u32::try_from(focus_generation).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let descriptor = BufferPresent::client(global_frame, focus, generation)
        .and_then(|frame| frame.with_frame_id(global_frame, global_frame))
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let wire = descriptor.encode();
    let presented = syscall(
        SyscallNumber::SurfacePresentBuffer,
        capability,
        output.server.raw(),
        wire.as_ptr() as u64,
    );
    if presented.status != Status::Ok.raw()
        || presented.out1 != u64::from(global_frame)
        || presented.out2 != u64::from(global_frame)
    {
        fail(FAIL_RUNTIME);
    }
}

fn layer_mut(slot: usize) -> &'static mut [u32; SURFACE_PIXEL_COUNT] {
    match slot {
        0 => unsafe { &mut *LAUNCHER_LAYER.0.get() },
        1 => unsafe { &mut *APP_LAYER.0.get() },
        _ => fail(FAIL_RUNTIME),
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum WindowClient {
    Launcher,
    App,
}

struct InstalledApp {
    endpoint: OwnedUserHandle,
    pid: u64,
    identity: AppInstanceIdentity,
}

struct MultiWindowSurface {
    launcher: OwnedUserHandle,
    control: OwnedUserHandle,
    capability: OwnedUserHandle,
    init_pid: u64,
    session_id: u64,
    supervisor: UiSupervisorTracker,
    app: Option<InstalledApp>,
    launcher_pid: u64,
    supervisor_command_sequence: u64,
    supervisor_response_sequence: u64,
    lifecycle_focus_generation: u64,
    #[cfg(feature = "input-server-runtime")]
    input_route: InputRouteClient,
}

#[cfg(feature = "input-server-runtime")]
const INPUT_OVERLAY_WINDOW_RAW: u64 = 3;
#[cfg(feature = "input-server-runtime")]
const INPUT_PENDING_EVENT_CAPACITY: usize = 16;
#[cfg(feature = "input-server-runtime")]
const FAIL_INPUT_SURFACE_SUPERVISOR_BOOTSTRAP: u64 = 246;
#[cfg(feature = "input-server-runtime")]
const FAIL_INPUT_ROUTE_BOOTSTRAP: u64 = 247;
#[cfg(feature = "input-server-runtime")]
const FAIL_INPUT_ROUTE_READY: u64 = 248;
#[cfg(feature = "input-server-runtime")]
const FAIL_INPUT_SURFACE_ACQUIRE: u64 = 249;

#[cfg(feature = "input-server-runtime")]
struct InputRouteClient {
    endpoint: OwnedUserHandle,
    server_pid: u64,
    last_event_sequence: u64,
    last_physical_sequence: u64,
    route_sequence: u64,
    focus_sequence: u64,
    key_sequence: u64,
    focus: Option<WindowRef>,
    context: Option<ServerTextContext>,
    overlay_visible: bool,
    semantic_revision: u32,
    scene_ready: bool,
    scene_update_active: bool,
    pending_events: [Option<ServerInputEvent>; INPUT_PENDING_EVENT_CAPACITY],
    pending_head: usize,
    pending_len: usize,
    #[cfg(any(
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    input_session_id: u64,
    #[cfg(any(
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    route_epoch: u64,
}

#[cfg(feature = "input-server-runtime")]
impl InputRouteClient {
    fn bootstrap(endpoint: OwnedUserHandle, init_pid: u64) -> Self {
        let envelope = read_channel_envelope(endpoint.raw());
        if envelope.kind() != ChannelMessageKind::Bytes
            || envelope.logical_length() != bndr_input::INPUT_EVENT_WIRE_SIZE
            || envelope.sender_pid() == 0
            || envelope.sender_pid() == init_pid
            || envelope.received_handle().is_valid()
        {
            fail(FAIL_INPUT_ROUTE_READY);
        }
        let event = ServerInputEvent::decode(&envelope.data()[..bndr_input::INPUT_EVENT_WIRE_SIZE])
            .unwrap_or_else(|_| fail(FAIL_INPUT_ROUTE_READY));
        if event.sequence() != 1 || event.payload() != ServerInputPayload::Ready {
            fail(FAIL_INPUT_ROUTE_READY);
        }
        Self {
            endpoint,
            server_pid: envelope.sender_pid(),
            last_event_sequence: 1,
            last_physical_sequence: 0,
            route_sequence: 0,
            focus_sequence: 0,
            key_sequence: 0,
            focus: None,
            context: None,
            overlay_visible: false,
            semantic_revision: 0,
            scene_ready: false,
            scene_update_active: false,
            pending_events: [None; INPUT_PENDING_EVENT_CAPACITY],
            pending_head: 0,
            pending_len: 0,
            #[cfg(any(
                feature = "input-server-restart-runtime",
                feature = "service-dependency-runtime"
            ))]
            input_session_id: 1,
            #[cfg(any(
                feature = "input-server-restart-runtime",
                feature = "service-dependency-runtime"
            ))]
            route_epoch: 1,
        }
    }

    #[cfg(any(
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    fn bootstrap_replacement(
        endpoint: OwnedUserHandle,
        init_pid: u64,
        expected_server_pid: u64,
        input_session_id: u64,
        route_epoch: u64,
        physical_floor: u64,
    ) -> Self {
        let mut replacement = Self::bootstrap(endpoint, init_pid);
        let valid_recovery_generation = (route_epoch == 2 && physical_floor == 1)
            || (cfg!(feature = "service-dependency-runtime")
                && route_epoch == 3
                && physical_floor == 3);
        if replacement.server_pid != expected_server_pid
            || input_session_id != 2
            || !valid_recovery_generation
        {
            fail(FAIL_RUNTIME);
        }
        replacement.last_physical_sequence = physical_floor;
        replacement.input_session_id = input_session_id;
        replacement.route_epoch = route_epoch;
        replacement
    }

    fn read_wire_event(&mut self) -> ServerInputEvent {
        let envelope = read_channel_envelope(self.endpoint.raw());
        if envelope.kind() != ChannelMessageKind::Bytes
            || envelope.logical_length() != bndr_input::INPUT_EVENT_WIRE_SIZE
            || envelope.sender_pid() != self.server_pid
            || envelope.received_handle().is_valid()
        {
            fail(FAIL_RUNTIME);
        }
        let event = ServerInputEvent::decode(&envelope.data()[..bndr_input::INPUT_EVENT_WIRE_SIZE])
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        let expected = self
            .last_event_sequence
            .checked_add(1)
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        if event.sequence() != expected {
            fail(FAIL_RUNTIME);
        }
        self.last_event_sequence = expected;
        event
    }

    fn push_pending(&mut self, event: ServerInputEvent) {
        if self.pending_len == INPUT_PENDING_EVENT_CAPACITY {
            fail(FAIL_RUNTIME);
        }
        let tail = (self.pending_head + self.pending_len) % INPUT_PENDING_EVENT_CAPACITY;
        self.pending_events[tail] = Some(event);
        self.pending_len += 1;
    }

    fn pop_pending(&mut self) -> Option<ServerInputEvent> {
        if self.pending_len == 0 {
            return None;
        }
        let event = self.pending_events[self.pending_head]
            .take()
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        self.pending_head = (self.pending_head + 1) % INPUT_PENDING_EVENT_CAPACITY;
        self.pending_len -= 1;
        Some(event)
    }

    const fn has_pending(&self) -> bool {
        self.pending_len != 0
    }

    fn read_event(&mut self) -> ServerInputEvent {
        loop {
            let event = self.pop_pending().unwrap_or_else(|| self.read_wire_event());
            match event.payload() {
                ServerInputPayload::OverlayHidden { context_id } => {
                    let context = self.context.unwrap_or_else(|| fail(FAIL_RUNTIME));
                    if !self.overlay_visible || context.context_id() != context_id {
                        fail(FAIL_RUNTIME);
                    }
                    // InputServer can clear context as a direct consequence of
                    // pointer-driven focus. Consume that ordered notification
                    // before exposing the causative routed pointer to the
                    // compositor, keeping both process mirrors transactional.
                    self.context = None;
                    self.overlay_visible = false;
                    self.semantic_revision = 0;
                }
                ServerInputPayload::OverlayShown { .. }
                | ServerInputPayload::Ack { .. }
                | ServerInputPayload::Ready => fail(FAIL_RUNTIME),
                _ => return event,
            }
        }
    }

    fn accept_physical_sequence(&mut self, sequence: u64) {
        if sequence == 0 || sequence <= self.last_physical_sequence {
            fail(FAIL_RUNTIME);
        }
        self.last_physical_sequence = sequence;
    }

    fn next_route_sequence(&mut self) -> u64 {
        self.route_sequence = self
            .route_sequence
            .checked_add(1)
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        self.route_sequence
    }

    fn next_focus_sequence(&mut self) -> u64 {
        self.focus_sequence = self
            .focus_sequence
            .checked_add(1)
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        self.focus_sequence
    }

    fn next_key_sequence(&mut self) -> u64 {
        self.key_sequence = self
            .key_sequence
            .checked_add(1)
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        self.key_sequence
    }

    fn write_command(&mut self, command: InputCommand, stream: InputCommandStream) {
        write_wire(self.endpoint.raw(), &command.encode());
        loop {
            let event = self.read_wire_event();
            match event.payload() {
                ServerInputPayload::Ack {
                    stream: actual_stream,
                    acknowledged_sequence,
                } if actual_stream == stream && acknowledged_sequence == command.sequence() => {
                    return;
                }
                ServerInputPayload::Ack { .. }
                | ServerInputPayload::Ready
                | ServerInputPayload::OverlayShown { .. }
                | ServerInputPayload::OverlayHidden { .. } => fail(FAIL_RUNTIME),
                _ => self.push_pending(event),
            }
        }
    }

    fn wait_overlay_visibility(&mut self, context_id: u64, visible: bool) {
        let pending_to_scan = self.pending_len;
        for _ in 0..pending_to_scan {
            let event = self.pop_pending().unwrap_or_else(|| fail(FAIL_RUNTIME));
            let matches = match event.payload() {
                ServerInputPayload::OverlayShown { context_id: actual } => {
                    visible && actual == context_id
                }
                ServerInputPayload::OverlayHidden { context_id: actual } => {
                    !visible && actual == context_id
                }
                ServerInputPayload::Ack { .. } | ServerInputPayload::Ready => fail(FAIL_RUNTIME),
                _ => false,
            };
            if matches {
                return;
            }
            self.push_pending(event);
        }
        loop {
            let event = self.read_wire_event();
            let matches = match event.payload() {
                ServerInputPayload::OverlayShown { context_id: actual } => {
                    visible && actual == context_id
                }
                ServerInputPayload::OverlayHidden { context_id: actual } => {
                    !visible && actual == context_id
                }
                ServerInputPayload::Ack { .. } | ServerInputPayload::Ready => fail(FAIL_RUNTIME),
                _ => false,
            };
            if matches {
                return;
            }
            self.push_pending(event);
        }
    }

    fn begin_scene_update(&mut self) {
        if !self.scene_ready || self.scene_update_active {
            fail(FAIL_RUNTIME);
        }
        let sequence = self.next_route_sequence();
        let command = InputCommand::scene_begin(sequence).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        self.write_command(command, InputCommandStream::Route);
        self.scene_update_active = true;
    }

    fn commit_scene_update(&mut self) {
        if !self.scene_update_active {
            fail(FAIL_RUNTIME);
        }
        let sequence = self.next_route_sequence();
        let command = InputCommand::scene_commit(sequence).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        self.write_command(command, InputCommandStream::Route);
        self.scene_update_active = false;
        self.scene_ready = true;
    }

    fn upsert(&mut self, route: InputRoute) {
        let sequence = self.next_route_sequence();
        let command =
            InputCommand::route_upsert(sequence, route).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        self.write_command(command, InputCommandStream::Route);
    }

    fn remove(&mut self, target: WindowRef) {
        let removed_context = self.context.filter(|context| context.target() == target);
        let sequence = self.next_route_sequence();
        let command =
            InputCommand::route_remove(sequence, target).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        self.write_command(command, InputCommandStream::Route);
        if self.focus == Some(target) {
            self.focus = None;
        }
        if let Some(context) = removed_context {
            self.wait_overlay_visibility(context.context_id(), false);
            self.context = None;
            self.overlay_visible = false;
            self.semantic_revision = 0;
        }
    }

    fn set_focus(&mut self, target: Option<WindowRef>) {
        let previous_context = self.context;
        let sequence = self.next_focus_sequence();
        let command =
            InputCommand::set_focus(sequence, target).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        self.write_command(command, InputCommandStream::Focus);
        self.focus = target;
        if !self.scene_update_active {
            self.scene_ready = true;
        }
        if previous_context.is_some_and(|context| Some(context.target()) != target) {
            let context_id = previous_context
                .map(ServerTextContext::context_id)
                .unwrap_or_else(|| fail(FAIL_RUNTIME));
            self.wait_overlay_visibility(context_id, false);
            self.context = None;
            self.overlay_visible = false;
            self.semantic_revision = 0;
        }
    }

    fn set_context(&mut self, context: Option<ServerTextContext>) {
        let previous = self.context;
        let sequence = self.next_focus_sequence();
        let command = InputCommand::set_text_context(sequence, context)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        self.write_command(command, InputCommandStream::Focus);
        if previous == context {
            return;
        }
        if let Some(previous) = previous {
            self.wait_overlay_visibility(previous.context_id(), false);
        }
        self.context = context;
        self.overlay_visible = false;
        self.semantic_revision = 0;
        if let Some(context) = context {
            self.wait_overlay_visibility(context.context_id(), true);
            self.overlay_visible = true;
        }
    }

    #[cfg(any(
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    fn resync_snapshot(
        &mut self,
        launcher_route: InputRoute,
        app_route: InputRoute,
        launcher_focus: WindowRef,
    ) {
        let valid_recovery_generation = (self.route_epoch == 2 && self.last_physical_sequence == 1)
            || (cfg!(feature = "service-dependency-runtime")
                && self.route_epoch == 3
                && self.last_physical_sequence == 3);
        if self.input_session_id != 2
            || !valid_recovery_generation
            || self.last_event_sequence != 1
            || self.route_sequence != 0
            || self.focus_sequence != 0
            || self.scene_ready
            || self.scene_update_active
            || self.focus.is_some()
            || self.context.is_some()
            || self.overlay_visible
            || self.has_pending()
        {
            fail(FAIL_RUNTIME);
        }
        let reset_sequence = self.next_route_sequence();
        if reset_sequence != 1 {
            fail(FAIL_RUNTIME);
        }
        let reset =
            InputCommand::snapshot_reset(reset_sequence).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        self.write_command(reset, InputCommandStream::Route);
        self.scene_update_active = true;
        self.upsert(launcher_route);
        self.upsert(app_route);
        self.set_focus(Some(launcher_focus));
        self.set_context(None);
        self.commit_scene_update();
        if self.last_event_sequence != 7
            || self.route_sequence != 4
            || self.focus_sequence != 2
            || !self.scene_ready
            || self.scene_update_active
            || self.focus != Some(launcher_focus)
            || self.context.is_some()
            || self.overlay_visible
            || self.has_pending()
        {
            fail(FAIL_RUNTIME);
        }
    }
}

#[cfg(feature = "input-server-runtime")]
fn read_input_route_bootstrap(startup: u64, init_pid: u64) -> OwnedUserHandle {
    let envelope = read_channel_envelope(startup);
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != core::mem::size_of::<u64>()
        || envelope.sender_pid() != init_pid
        || envelope.data()[..core::mem::size_of::<u64>()]
            != INPUT_ROUTE_BOOTSTRAP_MAGIC.to_le_bytes()
    {
        if let Some(endpoint) = OwnedUserHandle::new(u64::from(envelope.received_handle().raw())) {
            close_owned(endpoint);
        }
        fail(FAIL_INPUT_ROUTE_BOOTSTRAP);
    }
    OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_INPUT_ROUTE_BOOTSTRAP))
}

#[cfg(feature = "input-server-runtime")]
fn input_window_ref(window_id: WindowId) -> WindowRef {
    let raw_id = u64::try_from(window_id.slot())
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    WindowRef::try_new(
        InputWindowId::try_new(raw_id).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        u64::from(window_id.generation()),
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME))
}

#[cfg(feature = "input-server-runtime")]
fn input_overlay_ref(generation: u64) -> WindowRef {
    WindowRef::try_new(
        InputWindowId::try_new(INPUT_OVERLAY_WINDOW_RAW).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        generation,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME))
}

#[cfg(feature = "input-server-runtime")]
fn input_route_for_window(
    compositor: &Compositor<'_>,
    owner: u64,
    window_id: WindowId,
) -> InputRoute {
    let info = compositor
        .window_info(owner, window_id)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let x = PHONE_OFFSET_X
        .checked_add(info.bounds.x)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let y = PHONE_OFFSET_Y
        .checked_add(info.bounds.y)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    InputRoute::try_new(
        input_window_ref(window_id),
        OwnerPid::try_new(owner).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        RouteBounds::try_new(x, y, info.bounds.width, info.bounds.height)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        i32::try_from(info.z_index).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        true,
        true,
        false,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME))
}

#[cfg(any(
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
fn input_restart_route_for_window(
    compositor: &Compositor<'_>,
    owner: u64,
    window_id: WindowId,
) -> InputRoute {
    let info = compositor
        .window_info(owner, window_id)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    #[cfg(feature = "post-recovery-interaction-runtime")]
    let x = PHONE_OFFSET_X
        .checked_add(info.bounds.x)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    #[cfg(not(feature = "post-recovery-interaction-runtime"))]
    let x = info.bounds.x;
    #[cfg(feature = "post-recovery-interaction-runtime")]
    let y = PHONE_OFFSET_Y
        .checked_add(info.bounds.y)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    #[cfg(not(feature = "post-recovery-interaction-runtime"))]
    let y = info.bounds.y;
    InputRoute::try_new(
        input_window_ref(window_id),
        OwnerPid::try_new(owner).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        RouteBounds::try_new(x, y, info.bounds.width, info.bounds.height)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        i32::try_from(info.z_index).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        true,
        true,
        false,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME))
}

#[cfg(feature = "input-server-runtime")]
fn publish_input_routes_incremental(
    surface: &mut MultiWindowSurface,
    compositor: &Compositor<'_>,
    launcher: Option<(WindowId, u64)>,
    app: Option<(WindowId, u64)>,
) {
    for window_id in compositor.z_order().into_iter().flatten() {
        let owner = if launcher.is_some_and(|(candidate, _)| candidate == window_id) {
            launcher.map(|(_, owner)| owner)
        } else if app.is_some_and(|(candidate, _)| candidate == window_id) {
            app.map(|(_, owner)| owner)
        } else {
            None
        }
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
        let route = input_route_for_window(compositor, owner, window_id);
        surface.input_route.upsert(route);
    }
    sync_input_focus(surface, compositor);
}

#[cfg(feature = "input-server-runtime")]
fn prepare_input_window_command(surface: &mut MultiWindowSurface, command: WindowCommand) {
    if let WindowCommandPayload::Destroy { window_id } = command.payload() {
        surface.input_route.remove(input_window_ref(window_id));
    }
}

#[cfg(feature = "input-server-runtime")]
fn finish_input_window_command(
    surface: &mut MultiWindowSurface,
    compositor: &Compositor<'_>,
    command: WindowCommand,
    launcher: Option<(WindowId, u64)>,
    app: Option<(WindowId, u64)>,
) {
    match command.payload() {
        WindowCommandPayload::Create { .. }
        | WindowCommandPayload::Raise { .. }
        | WindowCommandPayload::Destroy { .. } => {
            publish_input_routes_incremental(surface, compositor, launcher, app);
        }
        WindowCommandPayload::Present { .. } => {}
    }
}

#[cfg(feature = "input-server-runtime")]
fn sync_input_focus(surface: &mut MultiWindowSurface, compositor: &Compositor<'_>) {
    let target = compositor.focus_state().window_id.map(input_window_ref);
    if !surface.input_route.scene_ready || surface.input_route.focus != target {
        surface.input_route.set_focus(target);
    }
}

#[cfg(feature = "input-server-runtime")]
fn install_input_overlay_route(surface: &mut MultiWindowSurface, generation: u64) {
    let route = InputRoute::try_new(
        input_overlay_ref(generation),
        OwnerPid::try_new(surface.init_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        RouteBounds::try_new(
            TRUSTED_OVERLAY_RECT.x,
            TRUSTED_OVERLAY_RECT.y,
            TRUSTED_OVERLAY_RECT.width,
            TRUSTED_OVERLAY_RECT.height,
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        i32::MAX,
        true,
        false,
        true,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    surface.input_route.upsert(route);
}

#[cfg(feature = "input-server-runtime")]
fn remove_input_overlay_route(surface: &mut MultiWindowSurface, generation: u64) {
    surface.input_route.remove(input_overlay_ref(generation));
}

#[cfg(feature = "input-server-runtime")]
fn activate_input_context(surface: &mut MultiWindowSurface, window_id: WindowId, context_id: u64) {
    let target = input_window_ref(window_id);
    if surface.input_route.focus != Some(target) {
        fail(FAIL_RUNTIME);
    }
    let context = ServerTextContext::try_new(target, context_id, TextPurpose::Normal)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    surface.input_route.set_context(Some(context));
}

#[cfg(feature = "input-server-runtime")]
fn deactivate_input_context(surface: &mut MultiWindowSurface) {
    if surface.input_route.context.is_some() {
        surface.input_route.set_context(None);
    }
}

fn send_window_event(transport: u64, event: WindowEvent) {
    write_wire(transport, &event.encode());
}

fn process_identity(identity: AppInstanceIdentity) -> u64 {
    (u64::from(identity.process().generation()) << 32) | u64::from(identity.process().pid())
}

fn publish_legacy_focus(surface: &MultiWindowSurface, client: UiClientId) {
    let app = (client == UiClientId::App).then_some(APP);
    let event = UiServerEvent::focus_changed(
        surface.session_id,
        client,
        app,
        surface.lifecycle_focus_generation,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    send_ui_event(surface.launcher.raw(), event);
    if let Some(installed) = surface.app.as_ref() {
        send_ui_event(installed.endpoint.raw(), event);
    }
}

#[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
fn read_post_recovery_lifecycle_focus_acks(surface: &MultiWindowSurface, expected_magic: u64) {
    read_authenticated_magic(surface.launcher.raw(), surface.launcher_pid, expected_magic);
    let installed = surface.app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
    read_authenticated_magic(installed.endpoint.raw(), installed.pid, expected_magic);
}

#[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
fn publish_post_recovery_ready_focus(surface: &MultiWindowSurface) {
    if surface.session_id != 2 || surface.lifecycle_focus_generation != 1 {
        fail(FAIL_RUNTIME);
    }
    let installed = surface.app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
    let ready = UiServerEvent::ready(surface.session_id).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    send_ui_event(surface.launcher.raw(), ready);
    send_ui_event(installed.endpoint.raw(), ready);
    publish_legacy_focus(surface, UiClientId::App);
    read_post_recovery_lifecycle_focus_acks(surface, POST_RECOVERY_READY_FOCUS_ACK_MAGIC);
}

#[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
fn advance_post_recovery_lifecycle_focus(
    surface: &mut MultiWindowSurface,
    expected_generation: u64,
    client: UiClientId,
    expected_magic: u64,
) {
    if surface.session_id != 2 || surface.lifecycle_focus_generation != expected_generation {
        fail(FAIL_RUNTIME);
    }
    surface.lifecycle_focus_generation = surface
        .lifecycle_focus_generation
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    publish_legacy_focus(surface, client);
    read_post_recovery_lifecycle_focus_acks(surface, expected_magic);
}

fn dispatch_multi_window_supervisor(surface: &mut MultiWindowSurface) {
    let (command, transferred) = read_supervisor_message(surface.control.raw(), surface.init_pid);
    let UiSupervisorControlPayload::Command {
        operation,
        app,
        identity,
    } = command.payload()
    else {
        if let Some(endpoint) = transferred {
            close_owned(endpoint);
        }
        fail(FAIL_RUNTIME);
    };
    let expected_command = next_sequence(&mut surface.supervisor_command_sequence);
    if command.sender_sequence() != expected_command
        || command.transaction_id() != expected_command
        || command.transaction_id() > TRANSACTION_COUNT
        || surface.supervisor.accept(command).is_err()
    {
        if let Some(endpoint) = transferred {
            close_owned(endpoint);
        }
        fail(FAIL_RUNTIME);
    }

    match (command.transaction_id(), operation) {
        (1, UiSupervisorOperation::InstallAppEndpoint) => {
            let endpoint = transferred.unwrap_or_else(|| fail(FAIL_RUNTIME));
            let identity = identity.unwrap_or_else(|| fail(FAIL_RUNTIME));
            let pid = process_identity(identity);
            if app != Some(APP) || pid == 0 || pid == surface.launcher_pid || surface.app.is_some()
            {
                close_owned(endpoint);
                fail(FAIL_RUNTIME);
            }
            send_ui_event(
                endpoint.raw(),
                UiServerEvent::ready(surface.session_id).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
            );
            send_ui_event(
                endpoint.raw(),
                UiServerEvent::focus_changed(
                    surface.session_id,
                    UiClientId::Launcher,
                    None,
                    surface.lifecycle_focus_generation,
                )
                .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
            );
            surface.app = Some(InstalledApp {
                endpoint,
                pid,
                identity,
            });
        }
        (2, UiSupervisorOperation::ActivateApp) => {
            let installed = surface.app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
            if transferred.is_some()
                || app != Some(APP)
                || identity != Some(installed.identity)
                || surface.lifecycle_focus_generation != 1
            {
                fail(FAIL_RUNTIME);
            }
            surface.lifecycle_focus_generation = 2;
            publish_legacy_focus(surface, UiClientId::App);
        }
        _ => {
            if let Some(endpoint) = transferred {
                close_owned(endpoint);
            }
            fail(FAIL_RUNTIME);
        }
    }

    let response_sequence = next_sequence(&mut surface.supervisor_response_sequence);
    let acknowledgement = UiSupervisorControlMessage::ack(
        response_sequence,
        command.transaction_id(),
        operation,
        UiSupervisorStatus::Applied,
        app,
        identity,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if surface.supervisor.accept(acknowledgement).is_err() {
        fail(FAIL_RUNTIME);
    }
    write_wire(surface.control.raw(), &acknowledgement.encode());
}

fn receive_expected_window_command(
    surface: &mut MultiWindowSurface,
    client: WindowClient,
) -> WindowCommand {
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    loop {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(surface.control.raw(), requested);
        let endpoint = match client {
            WindowClient::Launcher => Some(surface.launcher.raw()),
            WindowClient::App => surface.app.as_ref().map(|app| app.endpoint.raw()),
        };
        let count = if let Some(endpoint) = endpoint {
            items[1] = pack_user_wait_item(endpoint, requested);
            2
        } else {
            1
        };
        let ready = object_wait_many_array(&items, count, OBJECT_WAIT_TIMEOUT_INFINITE);
        let index = usize::try_from(ready.out1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        if ready.status != Status::Ok.raw()
            || index >= count
            || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
            || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
        {
            fail(FAIL_RUNTIME);
        }
        if index == 0 {
            dispatch_multi_window_supervisor(surface);
            continue;
        }

        let expected_sender = match client {
            WindowClient::Launcher => surface.launcher_pid,
            WindowClient::App => surface
                .app
                .as_ref()
                .map(|app| app.pid)
                .unwrap_or_else(|| fail(FAIL_RUNTIME)),
        };
        let (command, sender) = decode_window_command(endpoint.unwrap(), expected_sender);
        if client == WindowClient::Launcher && surface.launcher_pid == 0 {
            surface.launcher_pid = sender;
            if surface
                .app
                .as_ref()
                .is_some_and(|installed| installed.pid == sender)
            {
                fail(FAIL_RUNTIME);
            }
        }
        return command;
    }
}

fn next_scene_frame(scene_frame: &mut u32) -> u32 {
    *scene_frame = scene_frame
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    *scene_frame
}

fn validate_present_ledger(
    ledger: PixelLedger,
    content: u32,
    visible: u32,
    occluded: u32,
    recomposed: u32,
    scene_changed: bool,
) {
    if ledger.content_pixels != content
        || ledger.visible_pixels != visible
        || ledger.occluded_pixels != occluded
        || ledger.recomposed_pixels != recomposed
        || ledger.scene_changed != scene_changed
    {
        fail(FAIL_RUNTIME);
    }
}

fn output_digest(pixels: &[u32]) -> u64 {
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    for pixel in pixels {
        for byte in pixel.to_le_bytes() {
            digest ^= u64::from(byte);
            digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    digest
}

fn sync_output(compositor: &Compositor<'_>, output: &mut OutputBuffer) {
    let ledger = compositor
        .compose_full(output.pixels_mut())
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if ledger.recomposed_pixels != GRAPHICS_BUFFER_PIXEL_COUNT as u32 {
        fail(FAIL_RUNTIME);
    }
}

fn prepare_visible_output(
    surface: &MultiWindowSurface,
    output: &mut OutputBuffer,
    compositor: &Compositor<'_>,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) {
    acquire_frame_grant(surface.capability.raw(), last_epoch, last_boundary);
    sync_output(compositor, output);
}

fn publish_visible_output(
    surface: &MultiWindowSurface,
    output: &mut OutputBuffer,
    global_frame: &mut u32,
) {
    *global_frame = global_frame
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    present_output(
        surface.capability.raw(),
        output,
        *global_frame,
        surface.lifecycle_focus_generation,
    );
}

fn send_created_event(
    surface: &MultiWindowSurface,
    client: WindowClient,
    command: WindowCommand,
    scene_frame: u32,
    window_id: WindowId,
    bounds: ShellRect,
) {
    let event = WindowEvent::created(command.sequence(), scene_frame, window_id, bounds)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let endpoint = match client {
        WindowClient::Launcher => surface.launcher.raw(),
        WindowClient::App => surface
            .app
            .as_ref()
            .map(|app| app.endpoint.raw())
            .unwrap_or_else(|| fail(FAIL_RUNTIME)),
    };
    send_window_event(endpoint, event);
}

fn send_presented_event(
    surface: &MultiWindowSurface,
    client: WindowClient,
    command: WindowCommand,
    scene_frame: u32,
    window_id: WindowId,
    frame_id: u32,
) {
    let event = WindowEvent::presented(command.sequence(), scene_frame, window_id, frame_id)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let endpoint = match client {
        WindowClient::Launcher => surface.launcher.raw(),
        WindowClient::App => surface
            .app
            .as_ref()
            .map(|app| app.endpoint.raw())
            .unwrap_or_else(|| fail(FAIL_RUNTIME)),
    };
    send_window_event(endpoint, event);
}

fn send_raised_event(
    surface: &MultiWindowSurface,
    client: WindowClient,
    command: WindowCommand,
    scene_frame: u32,
    window_id: WindowId,
) {
    let event = WindowEvent::raised(command.sequence(), scene_frame, window_id)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let endpoint = match client {
        WindowClient::Launcher => surface.launcher.raw(),
        WindowClient::App => surface
            .app
            .as_ref()
            .map(|app| app.endpoint.raw())
            .unwrap_or_else(|| fail(FAIL_RUNTIME)),
    };
    send_window_event(endpoint, event);
}

#[cfg(not(feature = "input-server-runtime"))]
fn route_pointer_event(
    surface: &mut MultiWindowSurface,
    compositor: &mut Compositor<'_>,
    launcher_window: WindowId,
    app_window: WindowId,
    last_launcher_command: u64,
    last_app_command: u64,
    scene_frame: u32,
) -> bool {
    let input = syscall(
        SyscallNumber::SurfaceReadInput,
        surface.capability.raw(),
        0,
        0,
    );
    if input.status == Status::ShouldWait.raw() {
        return false;
    }
    if input.status != Status::Ok.raw() || input.out2 == 0 {
        fail(FAIL_RUNTIME);
    }
    let sample = InputSample::decode_registers(input.out1, input.out2)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let x = sample.x().saturating_sub(PHONE_OFFSET_X).min(207);
    let y = sample.y().saturating_sub(PHONE_OFFSET_Y).min(367);
    let route = if sample.pressed() {
        if compositor.captured_window().is_some() {
            compositor.pointer_move(x, y)
        } else {
            compositor.pointer_down(x, y)
        }
    } else if compositor.captured_window().is_some() {
        compositor.pointer_up(x, y)
    } else {
        return false;
    }
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let Some(route) = route else {
        return false;
    };
    let (endpoint, command_sequence) = if route.window_id == launcher_window {
        (surface.launcher.raw(), last_launcher_command)
    } else if route.window_id == app_window {
        (
            surface
                .app
                .as_ref()
                .map(|app| app.endpoint.raw())
                .unwrap_or_else(|| fail(FAIL_RUNTIME)),
            last_app_command,
        )
    } else {
        fail(FAIL_RUNTIME)
    };
    send_pointer_route(endpoint, command_sequence, scene_frame, route);
    true
}

#[cfg(any(
    feature = "service-dependency-runtime",
    all(
        feature = "input-server-surface-restart-runtime",
        not(feature = "input-server-restart-runtime")
    )
))]
fn wait_for_surface_restart_trigger(
    surface: &mut MultiWindowSurface,
    outputs: &mut [OutputBuffer; 2],
    compositor: &mut Compositor<'_>,
    app_window: WindowId,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) -> ! {
    let checkpoint = SurfaceRecoveryCheckpoint::MultiWindowInteractive;
    let armed = SurfaceRecoveryMessage::status(
        1,
        surface.session_id,
        1,
        checkpoint,
        0,
        SurfaceRecoveryPhase::Armed,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(surface.control.raw(), &armed.encode());

    loop {
        let event = surface.input_route.read_event();
        let ServerInputPayload::RoutedPointer(routed) = event.payload() else {
            fail(FAIL_RUNTIME);
        };
        surface
            .input_route
            .accept_physical_sequence(routed.input_sequence);
        if !routed.pressed {
            if routed.input_sequence != 1
                || routed.target.is_some()
                || routed.captured
                || routed.trusted_overlay
                || (routed.x, routed.y) != (136, 184)
            {
                fail(FAIL_RUNTIME);
            }
            continue;
        }
        if routed.input_sequence != 2
            || routed.target != Some(input_window_ref(app_window))
            || !routed.captured
            || routed.trusted_overlay
            || (routed.x, routed.y) != (136, 184)
            || compositor.captured_window().is_some()
        {
            fail(FAIL_RUNTIME);
        }
        let route = compositor
            .pointer_down(80, 120)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME))
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        if route.window_id != app_window
            || !route.pressed
            || !route.captured
            || route.focus_generation != 3
            || compositor.captured_window() != Some(app_window)
        {
            fail(FAIL_RUNTIME);
        }
        // This is a recovery-only contact. It establishes capture in both
        // authorities but deliberately does not enter BWE1, so the sealed
        // M41/M42 transcript cannot consume or duplicate the fault trigger.
        sync_input_focus(surface, compositor);
        let (app_pid, app_endpoint) = surface
            .app
            .as_ref()
            .map(|app| (app.pid, app.endpoint.raw()))
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        let contact = SurfaceRecoveryMessage::client_contact(
            1,
            surface.session_id,
            1,
            RecoveryClientRole::App,
            RecoveryCancelKind::ClientPointer,
            checkpoint,
            app_window,
            routed.input_sequence,
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        write_wire(app_endpoint, &contact.encode());
        let contact_ack = read_surface_recovery_message(app_endpoint, app_pid);
        let SurfaceRecoveryPayload::ClientContactAck {
            old_window,
            acknowledged_contact_sequence,
            physical_sequence,
            ..
        } = contact_ack.payload()
        else {
            fail(FAIL_RUNTIME);
        };
        if contact_ack.sender_sequence() != 1
            || !contact_ack.acknowledges_contact(contact)
            || old_window != app_window
            || acknowledged_contact_sequence != 1
            || physical_sequence != routed.input_sequence
        {
            fail(FAIL_RUNTIME);
        }
        let recovery_pixels =
            u32::from(SURFACE_RECOVERY_DAMAGE.width) * u32::from(SURFACE_RECOVERY_DAMAGE.height);
        if *global_frame != 6
            || *last_epoch != 6
            || outputs[0].write_generation != 3
            || recovery_pixels > 8_192
        {
            fail(FAIL_RUNTIME);
        }
        let prior_boundary = *last_boundary;
        prepare_visible_output(
            surface,
            &mut outputs[0],
            compositor,
            last_epoch,
            last_boundary,
        );
        let canonical_digest = output_digest(outputs[0].pixels_mut());
        let report = compositor
            .present(
                surface
                    .app
                    .as_ref()
                    .map(|app| app.pid)
                    .unwrap_or_else(|| fail(FAIL_RUNTIME)),
                app_window,
                SURFACE_RECOVERY_DAMAGE,
                COLOR_SURFACE_RECOVERY_CAPTURED,
                outputs[0].pixels_mut(),
            )
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        if report.content_generation != 4
            || report.global_damage
                != ShellRect::new(
                    APP_BOUNDS.x + SURFACE_RECOVERY_DAMAGE.x,
                    APP_BOUNDS.y + SURFACE_RECOVERY_DAMAGE.y,
                    SURFACE_RECOVERY_DAMAGE.width,
                    SURFACE_RECOVERY_DAMAGE.height,
                )
            || output_digest(outputs[0].pixels_mut()) == canonical_digest
        {
            fail(FAIL_RUNTIME);
        }
        validate_present_ledger(
            report.pixels,
            recovery_pixels,
            recovery_pixels,
            0,
            recovery_pixels,
            true,
        );
        publish_visible_output(surface, &mut outputs[0], global_frame);
        if *global_frame != 7
            || *last_epoch != 7
            || *last_boundary <= prior_boundary
            || outputs[0].write_generation != 4
        {
            fail(FAIL_RUNTIME);
        }
        let requested = SurfaceRecoveryMessage::status(
            2,
            surface.session_id,
            1,
            checkpoint,
            routed.input_sequence,
            SurfaceRecoveryPhase::RestartRequested,
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        write_wire(surface.control.raw(), &requested.encode());
        loop {
            core::hint::spin_loop();
        }
    }
}

#[cfg(feature = "input-server-restart-runtime")]
#[allow(clippy::too_many_arguments)]
fn run_input_server_restart(
    surface: &mut MultiWindowSurface,
    outputs: &mut [OutputBuffer; 2],
    compositor: &mut Compositor<'_>,
    launcher_window: WindowId,
    app_window: WindowId,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) -> ! {
    let launcher_ref = input_window_ref(launcher_window);
    let app_ref = input_window_ref(app_window);
    if surface.session_id != 1
        || surface.input_route.input_session_id != 1
        || surface.input_route.route_epoch != 1
        || surface.input_route.last_physical_sequence != 0
        || surface.input_route.focus != Some(launcher_ref)
        || surface.input_route.context.is_some()
        || surface.input_route.overlay_visible
        || !surface.input_route.scene_ready
        || surface.input_route.scene_update_active
        || surface.input_route.has_pending()
        || compositor.focus_state().window_id != Some(launcher_window)
        || compositor.focus_state().generation != 2
        || compositor.captured_window().is_some()
        || *global_frame != 6
        || *last_epoch != 6
        || *last_boundary == 0
    {
        fail(FAIL_RUNTIME);
    }

    // SurfaceServer is never a physical-input authority. This denied probe is
    // deliberately before Armed and must not mint a handle.
    let denied = syscall(SyscallNumber::InputAcquire, 0, 0, 0);
    if denied.status != Status::PermissionDenied.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_RUNTIME);
    }

    let mut recovery_sequences = InputServiceRecoverySequenceTracker::new();
    send_input_restart_status(
        surface,
        &mut recovery_sequences,
        1,
        0,
        InputServiceRecoveryPhase::Armed,
        InputServiceFocus::Launcher,
    );

    let trigger = surface.input_route.read_event();
    let ServerInputPayload::RoutedPointer(trigger) = trigger.payload() else {
        fail(FAIL_RUNTIME);
    };
    surface
        .input_route
        .accept_physical_sequence(trigger.input_sequence);
    if trigger.input_sequence != 1
        || trigger.target.is_some()
        || trigger.pressed
        || trigger.captured
        || trigger.trusted_overlay
        || (trigger.x, trigger.y) != (136, 184)
        || compositor.focus_state().window_id != Some(launcher_window)
        || compositor.captured_window().is_some()
    {
        fail(FAIL_RUNTIME);
    }
    send_input_restart_status(
        surface,
        &mut recovery_sequences,
        2,
        1,
        InputServiceRecoveryPhase::RestartRequested,
        InputServiceFocus::Launcher,
    );

    let old_input_pid = surface.input_route.server_pid;
    #[cfg(feature = "service-supervisor-runtime")]
    let old_service_identity = ServiceIdentity::new(
        ServiceKind::InputServer,
        process_generation(old_input_pid),
        old_input_pid,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let route_closed = object_wait(
        surface.input_route.endpoint.raw(),
        ObjectSignals::PEER_CLOSED,
    );
    if route_closed.status != Status::Ok.raw()
        || route_closed.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
        || route_closed.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
    publish_input_restart_damage(
        surface,
        outputs,
        compositor,
        app_window,
        COLOR_INPUT_RESTART_GAP,
        4,
        global_frame,
        last_epoch,
        last_boundary,
    );
    if *global_frame != 7
        || *last_epoch != 7
        || outputs[0].write_generation != 4
        || *last_boundary == 0
    {
        fail(FAIL_RUNTIME);
    }
    send_input_restart_status(
        surface,
        &mut recovery_sequences,
        3,
        1,
        InputServiceRecoveryPhase::RouteLost,
        InputServiceFocus::Launcher,
    );
    #[cfg(feature = "service-supervisor-runtime")]
    {
        let fault = HealthFrame::fault(1, old_service_identity, FaultClass::ProcessExit)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        write_wire(surface.control.raw(), &fault.encode());
    }

    let (offer, replacement_endpoint) = read_input_restart_offer(
        surface.control.raw(),
        surface.init_pid,
        &mut recovery_sequences,
    );
    let InputServiceRecoveryPayload::RebindOffer {
        old_input_pid: offered_old,
        new_input_pid,
        new_input_session_id,
        physical_sequence_floor,
    } = offer.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if offer.sequence() != 1
        || offer.surface_session() != 1
        || offer.route_epoch() != 2
        || offered_old.get() != old_input_pid
        || new_input_pid.get() == old_input_pid
        || new_input_session_id != 2
        || physical_sequence_floor != 1
    {
        fail(FAIL_RUNTIME);
    }
    let replacement = InputRouteClient::bootstrap_replacement(
        replacement_endpoint,
        surface.init_pid,
        new_input_pid.get(),
        new_input_session_id,
        offer.route_epoch(),
        physical_sequence_floor,
    );
    let retired = core::mem::replace(&mut surface.input_route, replacement);
    close_owned(retired.endpoint);
    let app_pid = surface
        .app
        .as_ref()
        .map(|app| app.pid)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let launcher_route =
        input_restart_route_for_window(compositor, surface.launcher_pid, launcher_window);
    let app_route = input_restart_route_for_window(compositor, app_pid, app_window);
    surface
        .input_route
        .resync_snapshot(launcher_route, app_route, launcher_ref);
    send_input_restart_status(
        surface,
        &mut recovery_sequences,
        4,
        1,
        InputServiceRecoveryPhase::ResyncPrepared,
        InputServiceFocus::Launcher,
    );

    let down = surface.input_route.read_event();
    let ServerInputPayload::RoutedPointer(down) = down.payload() else {
        fail(FAIL_RUNTIME);
    };
    surface
        .input_route
        .accept_physical_sequence(down.input_sequence);
    if down.input_sequence != 2
        || down.target != Some(app_ref)
        || !down.pressed
        || !down.captured
        || down.trusted_overlay
        || (down.x, down.y) != (136, 184)
    {
        fail(FAIL_RUNTIME);
    }
    let mirrored_down = compositor
        .pointer_down(80, 120)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if mirrored_down.window_id != app_window
        || !mirrored_down.pressed
        || !mirrored_down.captured
        || mirrored_down.focus_generation != 3
        || compositor.focus_state().window_id != Some(app_window)
        || compositor.captured_window() != Some(app_window)
    {
        fail(FAIL_RUNTIME);
    }
    // Pointer-down already moved focus atomically inside InputRouter. Mirror
    // that fact locally without emitting a seventh resync command.
    surface.input_route.focus = Some(app_ref);

    let up = surface.input_route.read_event();
    let ServerInputPayload::RoutedPointer(up) = up.payload() else {
        fail(FAIL_RUNTIME);
    };
    surface
        .input_route
        .accept_physical_sequence(up.input_sequence);
    if up.input_sequence != 3
        || up.target != Some(app_ref)
        || up.pressed
        || !up.captured
        || up.trusted_overlay
        || (up.x, up.y) != (136, 184)
    {
        fail(FAIL_RUNTIME);
    }
    let mirrored_up = compositor
        .pointer_up(80, 120)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if mirrored_up.window_id != app_window
        || mirrored_up.pressed
        || !mirrored_up.captured
        || mirrored_up.focus_generation != 3
        || compositor.focus_state().window_id != Some(app_window)
        || compositor.captured_window().is_some()
        || surface.input_route.last_event_sequence != 9
        || surface.input_route.last_physical_sequence != 3
        || surface.input_route.focus != Some(app_ref)
        || surface.input_route.context.is_some()
        || surface.input_route.has_pending()
    {
        fail(FAIL_RUNTIME);
    }

    publish_input_restart_damage(
        surface,
        outputs,
        compositor,
        app_window,
        COLOR_INPUT_RESTART_ACTIVE,
        5,
        global_frame,
        last_epoch,
        last_boundary,
    );
    if *global_frame != 8
        || *last_epoch != 8
        || outputs[0].write_generation != 5
        || *last_boundary == 0
    {
        fail(FAIL_RUNTIME);
    }
    send_input_restart_status(
        surface,
        &mut recovery_sequences,
        5,
        3,
        InputServiceRecoveryPhase::Active,
        InputServiceFocus::App,
    );

    #[cfg(feature = "service-supervisor-runtime")]
    finish_service_supervisor_surface(
        surface,
        outputs,
        compositor,
        app_window,
        global_frame,
        last_epoch,
        last_boundary,
    );

    #[cfg(not(feature = "service-supervisor-runtime"))]
    {
        let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
        let app = surface.app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(surface.input_route.endpoint.raw(), requested);
        items[1] = pack_user_wait_item(surface.control.raw(), requested);
        items[2] = pack_user_wait_item(surface.launcher.raw(), requested);
        items[3] = pack_user_wait_item(app.endpoint.raw(), requested);
        let ready = object_wait_many_array(&items, 4, OBJECT_WAIT_TIMEOUT_INFINITE);
        let _ = (ready, outputs, global_frame, last_epoch, last_boundary);
        fail(FAIL_RUNTIME)
    }
}

#[cfg(feature = "service-supervisor-runtime")]
#[allow(clippy::too_many_arguments)]
fn finish_service_supervisor_surface(
    surface: &mut MultiWindowSurface,
    outputs: &mut [OutputBuffer; 2],
    compositor: &mut Compositor<'_>,
    app_window: WindowId,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) -> ! {
    let replacement_identity = ServiceIdentity::new(
        ServiceKind::InputServer,
        process_generation(surface.input_route.server_pid),
        surface.input_route.server_pid,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut route_peer_closed = false;
    let mut quarantine_received = false;
    while !route_peer_closed || !quarantine_received {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        let mut count = 0_usize;
        let route_index = (!route_peer_closed).then_some(count);
        if route_index.is_some() {
            items[count] = pack_user_wait_item(surface.input_route.endpoint.raw(), requested);
            count += 1;
        }
        let control_index = (!quarantine_received).then_some(count);
        if control_index.is_some() {
            items[count] = pack_user_wait_item(surface.control.raw(), requested);
            count += 1;
        }
        let ready = object_wait_many_array(&items, count, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw()
            || usize::try_from(ready.out1).unwrap_or_else(|_| fail(FAIL_RUNTIME)) >= count
        {
            fail(FAIL_RUNTIME);
        }
        let ready_index = usize::try_from(ready.out1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        if route_index == Some(ready_index) {
            if ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
                || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) != 0
            {
                fail(FAIL_RUNTIME);
            }
            route_peer_closed = true;
        } else if control_index == Some(ready_index) {
            if ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
                || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
                || quarantine_received
            {
                fail(FAIL_RUNTIME);
            }
            let quarantine = read_surface_service_health(surface.control.raw(), surface.init_pid);
            if quarantine.opcode() != HealthOpcode::Quarantine
                || quarantine.sequence() < 2
                || quarantine.sequence() > 3
                || quarantine.identity() != replacement_identity
                || quarantine.fault_class() != Some(FaultClass::RestartBudgetExhausted)
                || quarantine.restart_attempt() != 2
                || quarantine.restart_budget() != 1
            {
                fail(FAIL_RUNTIME);
            }
            quarantine_received = true;
        } else {
            fail(FAIL_RUNTIME);
        }
    }
    let retired_route = surface.input_route.endpoint.retire();
    close_owned(retired_route);

    publish_input_degraded_damage(
        surface,
        outputs,
        compositor,
        app_window,
        6,
        global_frame,
        last_epoch,
        last_boundary,
    );
    if *global_frame != 9
        || *last_epoch != 9
        || outputs[0].write_generation != 6
        || *last_boundary == 0
    {
        fail(FAIL_RUNTIME);
    }

    let acknowledged =
        HealthFrame::degraded_ack(2, replacement_identity).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(surface.control.raw(), &acknowledged.encode());

    let app = surface.app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    items[0] = pack_user_wait_item(surface.control.raw(), requested);
    items[1] = pack_user_wait_item(surface.launcher.raw(), requested);
    items[2] = pack_user_wait_item(app.endpoint.raw(), requested);
    let ready = object_wait_many_array(&items, 3, OBJECT_WAIT_TIMEOUT_INFINITE);
    let _ = ready;
    fail(FAIL_RUNTIME)
}

#[cfg(feature = "service-supervisor-runtime")]
fn read_surface_service_health(transport: u64, expected_sender: u64) -> HealthFrame {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != bndr_sm::health::HEALTH_FRAME_SIZE
        || envelope.sender_pid() != expected_sender
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_RUNTIME);
    }
    HealthFrame::decode(envelope.data()).unwrap_or_else(|_| fail(FAIL_RUNTIME))
}

#[cfg(any(
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
fn send_input_restart_status(
    surface: &MultiWindowSurface,
    sequences: &mut InputServiceRecoverySequenceTracker,
    sequence: u64,
    physical_sequence_floor: u64,
    phase: InputServiceRecoveryPhase,
    focus: InputServiceFocus,
) {
    let message = InputServiceRecoveryMessage::surface_status(
        sequence,
        surface.session_id,
        surface.input_route.route_epoch,
        OwnerPid::try_new(surface.input_route.server_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        surface.input_route.input_session_id,
        physical_sequence_floor,
        phase,
        2,
        focus,
        false,
        false,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    sequences
        .accept(message)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(surface.control.raw(), &message.encode());
}

#[cfg(any(
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
fn read_input_restart_offer(
    transport: u64,
    expected_sender: u64,
    sequences: &mut InputServiceRecoverySequenceTracker,
) -> (InputServiceRecoveryMessage, OwnedUserHandle) {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != bndr_input::INPUT_SERVICE_RECOVERY_WIRE_SIZE
        || envelope.sender_pid() != expected_sender
        || !envelope.received_handle().is_valid()
    {
        if let Some(endpoint) = OwnedUserHandle::new(u64::from(envelope.received_handle().raw())) {
            close_owned(endpoint);
        }
        fail(FAIL_RUNTIME);
    }
    let endpoint = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let message = InputServiceRecoveryMessage::decode(
        &envelope.data()[..bndr_input::INPUT_SERVICE_RECOVERY_WIRE_SIZE],
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    sequences
        .accept(message)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    (message, endpoint)
}

#[cfg(any(
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
#[allow(clippy::too_many_arguments)]
fn publish_input_restart_damage(
    surface: &MultiWindowSurface,
    outputs: &mut [OutputBuffer; 2],
    compositor: &mut Compositor<'_>,
    app_window: WindowId,
    color: u32,
    expected_content_generation: u64,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) {
    let app_pid = surface
        .app
        .as_ref()
        .map(|app| app.pid)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    prepare_visible_output(
        surface,
        &mut outputs[0],
        compositor,
        last_epoch,
        last_boundary,
    );
    let prior_digest = output_digest(outputs[0].pixels_mut());
    let report = compositor
        .present(
            app_pid,
            app_window,
            INPUT_RESTART_DAMAGE,
            color,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let changed_pixels =
        u32::from(INPUT_RESTART_DAMAGE.width) * u32::from(INPUT_RESTART_DAMAGE.height);
    if report.content_generation != expected_content_generation
        || report.global_damage
            != ShellRect::new(
                APP_BOUNDS.x + INPUT_RESTART_DAMAGE.x,
                APP_BOUNDS.y + INPUT_RESTART_DAMAGE.y,
                INPUT_RESTART_DAMAGE.width,
                INPUT_RESTART_DAMAGE.height,
            )
        || changed_pixels > 8_192
        || output_digest(outputs[0].pixels_mut()) == prior_digest
    {
        fail(FAIL_RUNTIME);
    }
    validate_present_ledger(
        report.pixels,
        changed_pixels,
        changed_pixels,
        0,
        changed_pixels,
        true,
    );
    publish_visible_output(surface, &mut outputs[0], global_frame);
}

#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
#[allow(clippy::too_many_arguments)]
fn publish_input_degraded_damage(
    surface: &MultiWindowSurface,
    outputs: &mut [OutputBuffer; 2],
    compositor: &mut Compositor<'_>,
    app_window: WindowId,
    expected_content_generation: u64,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) {
    let app_pid = surface
        .app
        .as_ref()
        .map(|app| app.pid)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    prepare_visible_output(
        surface,
        &mut outputs[0],
        compositor,
        last_epoch,
        last_boundary,
    );
    let prior_digest = output_digest(outputs[0].pixels_mut());
    let report = compositor
        .present(
            app_pid,
            app_window,
            INPUT_DEGRADED_DAMAGE,
            COLOR_INPUT_DEGRADED,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let changed_pixels =
        u32::from(INPUT_DEGRADED_DAMAGE.width) * u32::from(INPUT_DEGRADED_DAMAGE.height);
    if report.content_generation != expected_content_generation
        || report.global_damage
            != ShellRect::new(
                APP_BOUNDS.x + INPUT_DEGRADED_DAMAGE.x,
                APP_BOUNDS.y + INPUT_DEGRADED_DAMAGE.y,
                INPUT_DEGRADED_DAMAGE.width,
                INPUT_DEGRADED_DAMAGE.height,
            )
        || changed_pixels > 8_192
        || output_digest(outputs[0].pixels_mut()) == prior_digest
    {
        fail(FAIL_RUNTIME);
    }
    validate_present_ledger(
        report.pixels,
        changed_pixels,
        changed_pixels,
        0,
        changed_pixels,
        true,
    );
    publish_visible_output(surface, &mut outputs[0], global_frame);
}

#[cfg(all(
    feature = "input-server-runtime",
    not(feature = "input-server-surface-restart-runtime")
))]
fn route_pointer_event(
    surface: &mut MultiWindowSurface,
    compositor: &mut Compositor<'_>,
    launcher_window: WindowId,
    app_window: WindowId,
    last_launcher_command: u64,
    last_app_command: u64,
    scene_frame: u32,
) -> bool {
    let event = surface.input_route.read_event();
    let ServerInputPayload::RoutedPointer(routed) = event.payload() else {
        fail(FAIL_RUNTIME);
    };
    surface
        .input_route
        .accept_physical_sequence(routed.input_sequence);
    if routed.trusted_overlay {
        fail(FAIL_RUNTIME);
    }
    let sample = InputSample::try_new(routed.input_sequence, routed.x, routed.y, routed.pressed)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let x = sample.x().saturating_sub(PHONE_OFFSET_X).min(207);
    let y = sample.y().saturating_sub(PHONE_OFFSET_Y).min(367);
    let route = if sample.pressed() {
        if compositor.captured_window().is_some() {
            compositor.pointer_move(x, y)
        } else {
            compositor.pointer_down(x, y)
        }
    } else if compositor.captured_window().is_some() {
        compositor.pointer_up(x, y)
    } else {
        if routed.target.is_some() || routed.captured {
            fail(FAIL_RUNTIME);
        }
        return false;
    }
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let Some(route) = route else {
        if routed.target.is_some() || routed.captured {
            fail(FAIL_RUNTIME);
        }
        return false;
    };
    if routed.target != Some(input_window_ref(route.window_id))
        || routed.pressed != route.pressed
        || routed.captured != route.captured
    {
        fail(FAIL_RUNTIME);
    }
    let (endpoint, command_sequence) = if route.window_id == launcher_window {
        (surface.launcher.raw(), last_launcher_command)
    } else if route.window_id == app_window {
        (
            surface
                .app
                .as_ref()
                .map(|app| app.endpoint.raw())
                .unwrap_or_else(|| fail(FAIL_RUNTIME)),
            last_app_command,
        )
    } else {
        fail(FAIL_RUNTIME)
    };
    send_pointer_route(endpoint, command_sequence, scene_frame, route);
    sync_input_focus(surface, compositor);
    true
}

#[cfg(feature = "persistent-window-runtime")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct PersistentPointerContact {
    contact_active: bool,
    rejected_until_release: bool,
}

#[cfg(feature = "persistent-window-runtime")]
impl PersistentPointerContact {
    const fn new() -> Self {
        Self {
            contact_active: false,
            rejected_until_release: false,
        }
    }

    fn begin(&mut self) {
        self.contact_active = true;
        self.rejected_until_release = false;
    }

    fn reject_until_release(&mut self) {
        if self.contact_active {
            self.rejected_until_release = true;
        }
    }

    fn finish_release(&mut self) {
        self.contact_active = false;
        self.rejected_until_release = false;
    }

    fn reconcile_capture(&mut self, capture_active: bool) {
        if self.contact_active && !self.rejected_until_release && !capture_active {
            self.rejected_until_release = true;
        }
    }

    const fn is_quiescent(self) -> bool {
        !self.contact_active && !self.rejected_until_release
    }
}

#[cfg(feature = "persistent-window-runtime")]
#[cfg(not(feature = "input-server-runtime"))]
#[allow(clippy::too_many_arguments)]
fn route_persistent_pointer_event(
    surface: &mut MultiWindowSurface,
    compositor: &mut Compositor<'_>,
    launcher_window: Option<WindowId>,
    app_window: Option<WindowId>,
    last_launcher_command: u64,
    last_app_command: u64,
    scene_frame: u32,
    contact: &mut PersistentPointerContact,
) -> bool {
    let Some(sample) = read_persistent_pointer_sample(surface) else {
        return false;
    };
    route_persistent_pointer_sample(
        surface,
        compositor,
        launcher_window,
        app_window,
        last_launcher_command,
        last_app_command,
        scene_frame,
        contact,
        sample,
    )
}

#[cfg(feature = "persistent-window-runtime")]
#[cfg(not(feature = "input-server-runtime"))]
fn read_persistent_pointer_sample(surface: &MultiWindowSurface) -> Option<InputSample> {
    let input = syscall(
        SyscallNumber::SurfaceReadInput,
        surface.capability.raw(),
        0,
        0,
    );
    if input.status == Status::ShouldWait.raw() {
        return None;
    }
    if input.status != Status::Ok.raw() || input.out2 == 0 {
        fail(FAIL_RUNTIME);
    }
    Some(
        InputSample::decode_registers(input.out1, input.out2)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
    )
}

#[cfg(feature = "persistent-window-runtime")]
#[allow(clippy::too_many_arguments)]
fn route_persistent_pointer_sample(
    surface: &mut MultiWindowSurface,
    compositor: &mut Compositor<'_>,
    launcher_window: Option<WindowId>,
    app_window: Option<WindowId>,
    last_launcher_command: u64,
    last_app_command: u64,
    scene_frame: u32,
    contact: &mut PersistentPointerContact,
    sample: InputSample,
    #[cfg(feature = "input-server-runtime")] expected: bndr_input::RoutedPointer,
) -> bool {
    contact.reconcile_capture(compositor.captured_window().is_some());
    if contact.rejected_until_release {
        #[cfg(feature = "input-server-runtime")]
        if expected.target.is_some() {
            fail(FAIL_RUNTIME);
        }
        if !sample.pressed() {
            contact.finish_release();
        }
        return false;
    }

    if !contact.contact_active && compositor.captured_window().is_some() {
        fail(FAIL_RUNTIME);
    }

    let surface_x = i32::from(sample.x()) - i32::from(PHONE_OFFSET_X);
    let surface_y = i32::from(sample.y()) - i32::from(PHONE_OFFSET_Y);
    let inside_phone = surface_x >= 0
        && surface_y >= 0
        && surface_x < i32::try_from(GRAPHICS_BUFFER_WIDTH).unwrap_or_else(|_| fail(FAIL_RUNTIME))
        && surface_y < i32::try_from(GRAPHICS_BUFFER_HEIGHT).unwrap_or_else(|_| fail(FAIL_RUNTIME));

    let route = if contact.contact_active {
        if sample.pressed() {
            compositor.pointer_move_unbounded(surface_x, surface_y)
        } else {
            let route = compositor.pointer_up_unbounded(surface_x, surface_y);
            contact.finish_release();
            route
        }
    } else if !sample.pressed() {
        #[cfg(feature = "input-server-runtime")]
        if expected.target.is_some() || expected.captured {
            fail(FAIL_RUNTIME);
        }
        return false;
    } else {
        contact.begin();
        if !inside_phone {
            #[cfg(feature = "input-server-runtime")]
            if expected.target.is_some() || expected.captured {
                fail(FAIL_RUNTIME);
            }
            contact.reject_until_release();
            return false;
        }
        compositor.pointer_down(surface_x as u16, surface_y as u16)
    }
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));

    let Some(route) = route else {
        #[cfg(feature = "input-server-runtime")]
        if expected.target.is_some() {
            fail(FAIL_RUNTIME);
        }
        contact.reject_until_release();
        return false;
    };
    #[cfg(feature = "input-server-runtime")]
    if expected.target != Some(input_window_ref(route.window_id))
        || expected.pressed != route.pressed
        || expected.captured != route.captured
        || expected.trusted_overlay
    {
        fail(FAIL_RUNTIME);
    }
    let (endpoint, command_sequence) = if Some(route.window_id) == launcher_window {
        (surface.launcher.raw(), last_launcher_command)
    } else if Some(route.window_id) == app_window {
        (
            surface
                .app
                .as_ref()
                .map(|app| app.endpoint.raw())
                .unwrap_or_else(|| fail(FAIL_RUNTIME)),
            last_app_command,
        )
    } else {
        fail(FAIL_RUNTIME)
    };
    send_pointer_route(endpoint, command_sequence, scene_frame, route);
    #[cfg(feature = "input-server-runtime")]
    sync_input_focus(surface, compositor);
    true
}

#[cfg(all(
    feature = "persistent-window-runtime",
    feature = "input-server-runtime"
))]
#[allow(clippy::too_many_arguments)]
fn route_server_persistent_pointer(
    surface: &mut MultiWindowSurface,
    compositor: &mut Compositor<'_>,
    launcher_window: Option<WindowId>,
    app_window: Option<WindowId>,
    last_launcher_command: u64,
    last_app_command: u64,
    scene_frame: u32,
    contact: &mut PersistentPointerContact,
    soft: &mut SoftKeyboardRuntime,
    routed: bndr_input::RoutedPointer,
) -> bool {
    surface
        .input_route
        .accept_physical_sequence(routed.input_sequence);
    let sample = InputSample::try_new(routed.input_sequence, routed.x, routed.y, routed.pressed)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    soft.reconcile_server_visibility(surface.input_route.overlay_visible);
    if routed.trusted_overlay {
        let expected_target = input_overlay_ref(u64::from(soft.overlay_shows));
        if routed.target != Some(expected_target) || !routed.captured {
            fail(FAIL_RUNTIME);
        }
        if !soft.is_visible() {
            soft.input_statistics.hidden_drops =
                soft.input_statistics.hidden_drops.saturating_add(1);
            return false;
        }
        if routed.pressed {
            if soft.input_overlay_contact.is_none()
                && let Some(key) = bndr_input::key_at(routed.x, routed.y)
            {
                soft.input_overlay_contact = Some(key);
                soft.input_statistics.contacts = soft.input_statistics.contacts.saturating_add(1);
            }
        } else if let Some(captured) = soft.input_overlay_contact.take() {
            if bndr_input::key_at(routed.x, routed.y) == Some(captured) {
                soft.input_statistics.activations =
                    soft.input_statistics.activations.saturating_add(1);
            } else {
                soft.input_statistics.cancels = soft.input_statistics.cancels.saturating_add(1);
            }
        }
        return false;
    }
    if soft.input_started && !soft.is_visible() && compositor.system_overlay_info().is_none() {
        soft.input_statistics.hidden_drops = soft.input_statistics.hidden_drops.saturating_add(1);
    }
    let routed_to_window = route_persistent_pointer_sample(
        surface,
        compositor,
        launcher_window,
        app_window,
        last_launcher_command,
        last_app_command,
        scene_frame,
        contact,
        sample,
        routed,
    );
    soft.reconcile_server_visibility(surface.input_route.overlay_visible);
    routed_to_window
}

fn send_pointer_route(endpoint: u64, command_sequence: u64, scene_frame: u32, route: PointerRoute) {
    let event = WindowEvent::input_route(
        command_sequence,
        scene_frame,
        route.window_id,
        route.global_x,
        route.global_y,
        route.local_x,
        route.local_y,
        route.pressed,
        route.captured,
        route.focus_generation,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    send_window_event(endpoint, event);
}

#[cfg(any(
    feature = "service-dependency-runtime",
    feature = "input-server-restart-runtime",
    all(
        feature = "input-server-surface-restart-runtime",
        not(feature = "input-server-restart-runtime")
    )
))]
#[allow(clippy::too_many_arguments)]
fn replay_m41_pointer_routes(
    surface: &mut MultiWindowSurface,
    compositor: &mut Compositor<'_>,
    launcher_window: WindowId,
    app_window: WindowId,
    last_launcher_command: u64,
    last_app_command: u64,
    scene_frame: u32,
    expected_routes: u8,
) {
    let (
        expected_window,
        endpoint,
        command_sequence,
        previous_focus,
        previous_focus_generation,
        expected_focus_generation,
        steps,
        step_count,
    ) = match (
        expected_routes,
        last_launcher_command,
        last_app_command,
        scene_frame,
    ) {
        (3, 3, 2, 5) => (
            app_window,
            surface
                .app
                .as_ref()
                .map(|app| app.endpoint.raw())
                .unwrap_or_else(|| fail(FAIL_RUNTIME)),
            2,
            None,
            0,
            1,
            [
                (80_u16, 120_u16, 32_i32, 40_i32, true),
                (16, 32, -32, -48, true),
                (16, 32, -32, -48, false),
            ],
            3_usize,
        ),
        (2, 5, 3, 8) => (
            launcher_window,
            surface.launcher.raw(),
            5,
            Some(app_window),
            1,
            2,
            [
                (80_u16, 120_u16, 80_i32, 120_i32, true),
                (80, 120, 80, 120, false),
                (0, 0, 0, 0, false),
            ],
            2_usize,
        ),
        _ => fail(FAIL_RUNTIME),
    };
    let focus = compositor.focus_state();
    if focus.window_id != previous_focus
        || focus.generation != previous_focus_generation
        || compositor.captured_window().is_some()
        || compositor.topmost_hit_test(steps[0].0, steps[0].1) != Some(expected_window)
        || !surface.input_route.scene_ready
        || surface.input_route.scene_update_active
        || surface.input_route.focus != previous_focus.map(input_window_ref)
        || surface.input_route.last_physical_sequence != 0
        || surface.input_route.has_pending()
    {
        fail(FAIL_RUNTIME);
    }

    for (index, &(global_x, global_y, local_x, local_y, pressed)) in
        steps[..step_count].iter().enumerate()
    {
        let route = if index == 0 {
            compositor.pointer_down(global_x, global_y)
        } else if index + 1 == step_count {
            compositor.pointer_up(global_x, global_y)
        } else {
            compositor.pointer_move(global_x, global_y)
        }
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
        if route.window_id != expected_window
            || route.global_x != global_x
            || route.global_y != global_y
            || route.local_x != local_x
            || route.local_y != local_y
            || route.pressed != pressed
            || !route.captured
            || route.focus_generation != expected_focus_generation
        {
            fail(FAIL_RUNTIME);
        }
        send_pointer_route(endpoint, command_sequence, scene_frame, route);
        sync_input_focus(surface, compositor);

        let expected_capture = pressed.then_some(expected_window);
        let focus = compositor.focus_state();
        if compositor.captured_window() != expected_capture
            || focus.window_id != Some(expected_window)
            || focus.generation != expected_focus_generation
            || surface.input_route.focus != Some(input_window_ref(expected_window))
            || surface.input_route.last_physical_sequence != 0
            || surface.input_route.has_pending()
        {
            fail(FAIL_RUNTIME);
        }
    }
}

#[cfg(not(feature = "input-server-surface-restart-runtime"))]
#[allow(clippy::too_many_arguments)]
fn receive_pointer_routes(
    surface: &mut MultiWindowSurface,
    compositor: &mut Compositor<'_>,
    launcher_window: WindowId,
    app_window: WindowId,
    last_launcher_command: u64,
    last_app_command: u64,
    scene_frame: u32,
    expected_routes: u8,
) {
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut routed = 0_u8;
    while routed < expected_routes {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(surface.control.raw(), requested);
        #[cfg(not(feature = "input-server-runtime"))]
        {
            items[1] = pack_user_wait_item(surface.capability.raw(), requested);
        }
        #[cfg(feature = "input-server-runtime")]
        {
            items[1] = pack_user_wait_item(surface.input_route.endpoint.raw(), requested);
        }
        #[cfg(feature = "input-server-runtime")]
        let ready = if surface.input_route.has_pending() {
            None
        } else {
            Some(object_wait_many_array(
                &items,
                2,
                OBJECT_WAIT_TIMEOUT_INFINITE,
            ))
        };
        #[cfg(not(feature = "input-server-runtime"))]
        let ready = Some(object_wait_many_array(
            &items,
            2,
            OBJECT_WAIT_TIMEOUT_INFINITE,
        ));
        let ready_index = ready.as_ref().map_or(1, |result| result.out1);
        let ready_signals = ready
            .as_ref()
            .map_or(u64::from(ObjectSignals::READABLE.bits()), |result| {
                result.out2
            });
        if ready
            .as_ref()
            .is_some_and(|result| result.status != Status::Ok.raw())
            || ready_index > 1
            || ready_signals & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
            || ready_signals & u64::from(ObjectSignals::READABLE.bits()) == 0
        {
            fail(FAIL_RUNTIME);
        }
        if ready_index == 0 {
            dispatch_multi_window_supervisor(surface);
        } else if route_pointer_event(
            surface,
            compositor,
            launcher_window,
            app_window,
            last_launcher_command,
            last_app_command,
            scene_frame,
        ) {
            routed += 1;
        }
    }
}

#[cfg(feature = "persistent-window-runtime")]
struct PersistentClientWindow<'a> {
    client: WindowClient,
    owner: u64,
    slot: usize,
    bounds: ShellRect,
    window_id: Option<WindowId>,
    free_layer: Option<&'a mut [u32]>,
    commands: WindowCommandTracker,
    last_command: u64,
    last_frame: Option<u32>,
}

#[cfg(feature = "persistent-window-runtime")]
fn publish_persistent_output(
    surface: &MultiWindowSurface,
    output: &mut OutputBuffer,
    pixels: PixelLedger,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) {
    if !pixels.scene_changed {
        return;
    }
    acquire_frame_grant(surface.capability.raw(), last_epoch, last_boundary);
    publish_visible_output(surface, output, global_frame);
}

#[cfg(feature = "soft-keyboard-runtime")]
#[allow(clippy::too_many_arguments)]
fn show_soft_keyboard(
    surface: &mut MultiWindowSurface,
    compositor: &mut Compositor<'static>,
    outputs: &mut [OutputBuffer; 2],
    soft: &mut SoftKeyboardRuntime,
    window_id: WindowId,
    context_id: u64,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) {
    #[cfg(not(feature = "input-server-runtime"))]
    let _ = (window_id, context_id);
    if soft.is_visible() || compositor.system_overlay_info().is_some() {
        fail(FAIL_RUNTIME);
    }
    #[cfg(feature = "input-server-runtime")]
    {
        let generation = u64::from(soft.overlay_shows)
            .checked_add(1)
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        install_input_overlay_route(surface, generation);
    }
    let layer = soft.free_layer.take().unwrap_or_else(|| fail(FAIL_RUNTIME));
    let output_index = usize::try_from(*global_frame & 1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let output = &mut outputs[output_index];
    sync_output(compositor, output);
    let report = compositor
        .install_system_overlay(SOFT_KEYBOARD_BOUNDS, layer, output.pixels_mut())
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if report.content_generation != 1
        || report.global_damage != SOFT_KEYBOARD_BOUNDS
        || report.pixels.content_pixels != SOFT_KEYBOARD_PIXEL_COUNT as u32
        || report.pixels.visible_pixels != SOFT_KEYBOARD_PIXEL_COUNT as u32
        || report.pixels.occluded_pixels != 0
        || !report.pixels.scene_changed
    {
        fail(FAIL_RUNTIME);
    }
    publish_persistent_output(
        surface,
        output,
        report.pixels,
        global_frame,
        last_epoch,
        last_boundary,
    );
    #[cfg(feature = "input-server-runtime")]
    {
        activate_input_context(surface, window_id, context_id);
        if !surface.input_route.overlay_visible {
            fail(FAIL_RUNTIME);
        }
    }
    soft.show_input();
    soft.overlay_shows = soft
        .overlay_shows
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
}

#[cfg(feature = "soft-keyboard-runtime")]
#[allow(clippy::too_many_arguments)]
fn hide_soft_keyboard(
    surface: &mut MultiWindowSurface,
    compositor: &mut Compositor<'static>,
    outputs: &mut [OutputBuffer; 2],
    soft: &mut SoftKeyboardRuntime,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) {
    #[cfg(not(feature = "input-server-runtime"))]
    if !soft.is_visible() || soft.free_layer.is_some() {
        fail(FAIL_RUNTIME);
    }
    #[cfg(feature = "input-server-runtime")]
    if soft.free_layer.is_some() {
        fail(FAIL_RUNTIME);
    }
    #[cfg(feature = "input-server-runtime")]
    {
        deactivate_input_context(surface);
        soft.reconcile_server_visibility(surface.input_route.overlay_visible);
    }
    let output_index = usize::try_from(*global_frame & 1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let output = &mut outputs[output_index];
    sync_output(compositor, output);
    let removed = compositor
        .remove_system_overlay(output.pixels_mut())
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if removed.bounds() != SOFT_KEYBOARD_BOUNDS
        || removed.content_generation() != 1
        || !removed.pixels().scene_changed
    {
        fail(FAIL_RUNTIME);
    }
    let pixels = removed.pixels();
    soft.free_layer = Some(removed.into_layer());
    publish_persistent_output(
        surface,
        output,
        pixels,
        global_frame,
        last_epoch,
        last_boundary,
    );
    #[cfg(feature = "input-server-runtime")]
    remove_input_overlay_route(surface, u64::from(soft.overlay_shows));
    soft.hide_input();
    soft.overlay_hides = soft
        .overlay_hides
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
}

#[cfg(feature = "persistent-window-runtime")]
fn validate_app_witness_topology(
    compositor: &Compositor<'_>,
    app_window: Option<WindowId>,
    focused: Option<WindowId>,
    focus_generation: u64,
) {
    let z_order = compositor.z_order();
    let launcher_valid =
        z_order[0].is_some_and(|window_id| window_id.slot() == 0 && window_id.generation() == 1);
    let app_valid = match app_window {
        Some(window_id) => {
            compositor.window_count() == 2 && window_id.slot() == 1 && z_order[1] == Some(window_id)
        }
        None => compositor.window_count() == 1 && z_order[1].is_none(),
    };
    let focus = compositor.focus_state();
    if !launcher_valid
        || !app_valid
        || focus.window_id != focused
        || focus.generation != focus_generation
        || compositor.captured_window().is_some()
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(feature = "persistent-window-runtime")]
#[allow(clippy::too_many_arguments)]
fn dispatch_persistent_window_command(
    surface: &MultiWindowSurface,
    compositor: &mut Compositor<'static>,
    outputs: &mut [OutputBuffer; 2],
    state: &mut PersistentClientWindow<'static>,
    command: WindowCommand,
    endpoint: u64,
    scene_frame: &mut u32,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) -> bool {
    let mut candidate = state.commands;
    if candidate.accept(command).is_err() {
        fail(FAIL_RUNTIME);
    }
    let app_witness_sequence =
        state.client == WindowClient::App && matches!(command.sequence(), 5..=8);
    if app_witness_sequence {
        let valid = match (command.sequence(), command.payload()) {
            (5, WindowCommandPayload::Destroy { window_id }) => {
                *scene_frame == 9
                    && *global_frame == 6
                    && *last_epoch == 6
                    && window_id.slot() == 1
                    && window_id.generation() == 1
            }
            (6, WindowCommandPayload::Create { bounds }) => {
                *scene_frame == 10 && *global_frame == 7 && *last_epoch == 7 && bounds == APP_BOUNDS
            }
            (
                7,
                WindowCommandPayload::Present {
                    window_id,
                    frame_id,
                    damage,
                    color,
                },
            ) => {
                *scene_frame == 11
                    && *global_frame == 7
                    && *last_epoch == 7
                    && window_id.slot() == 1
                    && window_id.generation() == 2
                    && frame_id == 1
                    && damage == ShellRect::new(0, 0, APP_BOUNDS.width, APP_BOUNDS.height)
                    && color == COLOR_APP_RECREATED
            }
            (8, WindowCommandPayload::Raise { window_id }) => {
                *scene_frame == 12
                    && *global_frame == 8
                    && *last_epoch == 8
                    && window_id.slot() == 1
                    && window_id.generation() == 2
            }
            _ => false,
        };
        if !valid {
            fail(FAIL_RUNTIME);
        }
    }
    let output_index = usize::try_from(*global_frame & 1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let output = &mut outputs[output_index];

    match command.payload() {
        WindowCommandPayload::Create { bounds } => {
            if state.window_id.is_some() || bounds != state.bounds {
                fail(FAIL_RUNTIME);
            }
            let layer = state
                .free_layer
                .take()
                .unwrap_or_else(|| fail(FAIL_RUNTIME));
            sync_output(compositor, output);
            let report = match compositor.create_in_slot(
                state.slot,
                state.owner,
                bounds,
                bndr_compositor::BACKGROUND_COLOR,
                layer,
                output.pixels_mut(),
            ) {
                Ok(report) => report,
                Err(failure) => {
                    let (_, layer) = failure.into_parts();
                    state.free_layer = Some(layer);
                    fail(FAIL_RUNTIME)
                }
            };
            if report.window_id.slot() != state.slot
                || report.window_id.generation() <= 1
                || report.content_generation != 1
            {
                fail(FAIL_RUNTIME);
            }
            if state.client == WindowClient::App && command.sequence() == 6 {
                if report.window_id.generation() != 2 {
                    fail(FAIL_RUNTIME);
                }
                validate_present_ledger(
                    report.pixels,
                    APP_PIXEL_COUNT,
                    APP_PIXEL_COUNT,
                    0,
                    APP_PIXEL_COUNT,
                    true,
                );
                validate_app_witness_topology(compositor, Some(report.window_id), None, 4);
            }
            state.window_id = Some(report.window_id);
            state.last_frame = None;
            send_window_event(
                endpoint,
                WindowEvent::created(
                    command.sequence(),
                    next_scene_frame(scene_frame),
                    report.window_id,
                    bounds,
                )
                .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
            );
        }
        WindowCommandPayload::Present {
            window_id,
            frame_id,
            damage,
            color,
        } => {
            if state.window_id != Some(window_id)
                || state
                    .last_frame
                    .is_some_and(|previous| frame_id <= previous)
            {
                fail(FAIL_RUNTIME);
            }
            sync_output(compositor, output);
            let report = compositor
                .present(state.owner, window_id, damage, color, output.pixels_mut())
                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            if state.client == WindowClient::App && command.sequence() == 7 {
                if report.content_generation != 2 || report.global_damage != APP_BOUNDS {
                    fail(FAIL_RUNTIME);
                }
                validate_present_ledger(
                    report.pixels,
                    APP_PIXEL_COUNT,
                    APP_PIXEL_COUNT,
                    0,
                    APP_PIXEL_COUNT,
                    true,
                );
            }
            publish_persistent_output(
                surface,
                output,
                report.pixels,
                global_frame,
                last_epoch,
                last_boundary,
            );
            if state.client == WindowClient::App && command.sequence() == 7 {
                if *global_frame != 8 || outputs[1].write_generation != 4 {
                    fail(FAIL_RUNTIME);
                }
                validate_app_witness_topology(compositor, Some(window_id), None, 4);
            }
            state.last_frame = Some(frame_id);
            send_window_event(
                endpoint,
                WindowEvent::presented(
                    command.sequence(),
                    next_scene_frame(scene_frame),
                    window_id,
                    frame_id,
                )
                .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
            );
        }
        WindowCommandPayload::Raise { window_id } => {
            if state.window_id != Some(window_id) {
                fail(FAIL_RUNTIME);
            }
            sync_output(compositor, output);
            let pixels = compositor
                .raise(state.owner, window_id, output.pixels_mut())
                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            if state.client == WindowClient::App
                && command.sequence() == 8
                && pixels != PixelLedger::EMPTY
            {
                fail(FAIL_RUNTIME);
            }
            publish_persistent_output(
                surface,
                output,
                pixels,
                global_frame,
                last_epoch,
                last_boundary,
            );
            if state.client == WindowClient::App && command.sequence() == 8 {
                if *global_frame != 8
                    || outputs[0].write_generation != 4
                    || outputs[1].write_generation != 4
                {
                    fail(FAIL_RUNTIME);
                }
                validate_app_witness_topology(compositor, Some(window_id), None, 4);
            }
            send_window_event(
                endpoint,
                WindowEvent::raised(command.sequence(), next_scene_frame(scene_frame), window_id)
                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
            );
        }
        WindowCommandPayload::Destroy { window_id } => {
            if state.window_id != Some(window_id) {
                fail(FAIL_RUNTIME);
            }
            sync_output(compositor, output);
            let destroyed = compositor
                .destroy(state.owner, window_id, output.pixels_mut())
                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            if destroyed.window_id() != window_id
                || destroyed.owner() != state.owner
                || destroyed.bounds() != state.bounds
            {
                fail(FAIL_RUNTIME);
            }
            let pixels = destroyed.pixels();
            if state.client == WindowClient::App && command.sequence() == 5 {
                validate_present_ledger(pixels, 0, 0, 0, APP_PIXEL_COUNT, true);
            }
            let layer = destroyed.into_layer();
            publish_persistent_output(
                surface,
                output,
                pixels,
                global_frame,
                last_epoch,
                last_boundary,
            );
            if state.client == WindowClient::App && command.sequence() == 5 {
                if *global_frame != 7 || outputs[0].write_generation != 4 {
                    fail(FAIL_RUNTIME);
                }
                validate_app_witness_topology(compositor, None, None, 4);
            }
            state.window_id = None;
            state.free_layer = Some(layer);
            state.last_frame = None;
            send_window_event(
                endpoint,
                WindowEvent::destroyed(
                    command.sequence(),
                    next_scene_frame(scene_frame),
                    window_id,
                )
                .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
            );
        }
    }

    state.commands = candidate;
    state.last_command = command.sequence();
    let witness_complete = state.client == WindowClient::App
        && command.sequence() == 8
        && state
            .window_id
            .is_some_and(|window_id| window_id.slot() == 1 && window_id.generation() == 2);
    if witness_complete {
        if *scene_frame != 13 || state.last_frame != Some(1) {
            fail(FAIL_RUNTIME);
        }
        validate_app_witness_topology(compositor, state.window_id, None, 4);
    }
    witness_complete
}

#[cfg(feature = "persistent-window-runtime")]
#[allow(clippy::too_many_arguments)]
fn cleanup_persistent_client(
    surface: &mut MultiWindowSurface,
    compositor: &mut Compositor<'static>,
    outputs: &mut [OutputBuffer; 2],
    state: &mut PersistentClientWindow<'static>,
    launcher_route: Option<(WindowId, u64)>,
    app_route: Option<(WindowId, u64)>,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) {
    let Some(window_id) = state.window_id else {
        return;
    };
    #[cfg(feature = "input-server-runtime")]
    surface.input_route.remove(input_window_ref(window_id));
    #[cfg(not(feature = "input-server-runtime"))]
    let _ = (launcher_route, app_route);
    let output_index = usize::try_from(*global_frame & 1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let output = &mut outputs[output_index];
    sync_output(compositor, output);
    let destroyed = compositor
        .destroy(state.owner, window_id, output.pixels_mut())
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let pixels = destroyed.pixels();
    let layer = destroyed.into_layer();
    publish_persistent_output(
        surface,
        output,
        pixels,
        global_frame,
        last_epoch,
        last_boundary,
    );
    state.window_id = None;
    state.free_layer = Some(layer);
    state.last_frame = None;
    #[cfg(feature = "input-server-runtime")]
    publish_input_routes_incremental(surface, compositor, launcher_route, app_route);
}

#[cfg(feature = "text-input-runtime")]
struct TextServerRuntime {
    tracker: TextInputServerTracker,
    context: Option<TextInputContext>,
    state: TextState,
    preedit: Option<char>,
    event_sequence: u64,
    active_transitions: u8,
    active_releases: u8,
    unfocused_transitions: u8,
    renders: u8,
    app_focus_announced: bool,
    launcher_focus_announced: bool,
}

#[cfg(feature = "soft-keyboard-runtime")]
struct SoftKeyboardRuntime {
    #[cfg(not(feature = "input-server-runtime"))]
    ime: InputMethodEngine,
    #[cfg(feature = "input-server-runtime")]
    input_visible: bool,
    #[cfg(feature = "input-server-runtime")]
    input_statistics: InputStatistics,
    #[cfg(feature = "input-server-runtime")]
    input_overlay_contact: Option<bndr_input::SoftKey>,
    #[cfg(feature = "input-server-runtime")]
    input_started: bool,
    free_layer: Option<&'static mut [u32]>,
    focus_stage: u8,
    renders: u8,
    preedit_actions: u8,
    commit_actions: u8,
    delete_actions: u8,
    overlay_shows: u8,
    overlay_hides: u8,
}

#[cfg(feature = "soft-keyboard-runtime")]
impl SoftKeyboardRuntime {
    fn new() -> Self {
        Self {
            #[cfg(not(feature = "input-server-runtime"))]
            ime: InputMethodEngine::new(),
            #[cfg(feature = "input-server-runtime")]
            input_visible: false,
            #[cfg(feature = "input-server-runtime")]
            input_statistics: InputStatistics::default(),
            #[cfg(feature = "input-server-runtime")]
            input_overlay_contact: None,
            #[cfg(feature = "input-server-runtime")]
            input_started: false,
            free_layer: Some(create_soft_keyboard_layer()),
            focus_stage: 0,
            renders: 0,
            preedit_actions: 0,
            commit_actions: 0,
            delete_actions: 0,
            overlay_shows: 0,
            overlay_hides: 0,
        }
    }

    fn is_visible(&self) -> bool {
        #[cfg(not(feature = "input-server-runtime"))]
        {
            self.ime.is_visible()
        }
        #[cfg(feature = "input-server-runtime")]
        {
            self.input_visible
        }
    }

    fn show_input(&mut self) {
        #[cfg(not(feature = "input-server-runtime"))]
        self.ime.show();
        #[cfg(feature = "input-server-runtime")]
        {
            self.input_visible = true;
            self.input_started = true;
        }
    }

    fn hide_input(&mut self) {
        #[cfg(not(feature = "input-server-runtime"))]
        self.ime.hide();
        #[cfg(feature = "input-server-runtime")]
        {
            self.input_visible = false;
            if self.input_overlay_contact.take().is_some() {
                self.input_statistics.cancels = self.input_statistics.cancels.saturating_add(1);
            }
        }
    }

    fn reconcile_server_visibility(&mut self, visible: bool) {
        #[cfg(feature = "input-server-runtime")]
        {
            self.input_visible = visible;
            if !visible {
                self.input_overlay_contact = None;
            }
        }
        #[cfg(not(feature = "input-server-runtime"))]
        let _ = visible;
    }

    fn statistics(&self) -> InputStatistics {
        #[cfg(not(feature = "input-server-runtime"))]
        {
            self.ime.statistics()
        }
        #[cfg(feature = "input-server-runtime")]
        {
            self.input_statistics
        }
    }
}

#[cfg(feature = "text-input-runtime")]
impl TextServerRuntime {
    fn new() -> Self {
        Self {
            tracker: TextInputServerTracker::new(),
            context: None,
            state: TextState::default(),
            preedit: None,
            event_sequence: 0,
            active_transitions: 0,
            active_releases: 0,
            unfocused_transitions: 0,
            renders: 0,
            app_focus_announced: false,
            launcher_focus_announced: false,
        }
    }

    fn next_event_sequence(&mut self) -> u64 {
        self.event_sequence = self
            .event_sequence
            .checked_add(1)
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        self.event_sequence
    }

    #[cfg(not(feature = "input-server-runtime"))]
    fn key_wait_enabled(&self) -> bool {
        !self.tracker.outstanding_acknowledgment()
    }

    fn context(&self) -> TextInputContext {
        self.context.unwrap_or_else(|| fail(FAIL_RUNTIME))
    }

    fn send_event(&mut self, endpoint: u64, event: TextInputEvent) {
        if self.tracker.send(event).is_err() || event.sequence() != self.event_sequence {
            fail(FAIL_RUNTIME);
        }
        write_wire(endpoint, &event.encode());
    }
}

#[cfg(feature = "text-input-runtime")]
fn expected_text_state(step: u8) -> TextState {
    let (revision, committed, cursor) = match step {
        1 => (1, "", 0),
        2 => (2, "a", 1),
        3 => (3, "", 0),
        4 => (4, "", 0),
        5 => (5, "a", 1),
        _ => fail(FAIL_RUNTIME),
    };
    TextState::try_new(revision, committed, cursor, cursor).unwrap_or_else(|_| fail(FAIL_RUNTIME))
}

#[cfg(feature = "soft-keyboard-runtime")]
fn expected_soft_text_state(step: u8) -> TextState {
    let (revision, committed, cursor) = match step {
        1 => (1, "a", 1),
        2 => (2, "aa", 2),
        3 => (3, "a", 1),
        4 => (4, "a", 1),
        5 => (5, "aa", 2),
        _ => fail(FAIL_RUNTIME),
    };
    TextState::try_new(revision, committed, cursor, cursor).unwrap_or_else(|_| fail(FAIL_RUNTIME))
}

#[cfg(feature = "text-input-runtime")]
fn send_text_key_event(text: &mut TextServerRuntime, endpoint: u64, sample: KeyInputSample) {
    const EXPECTED: [(u16, u8); 10] = [
        (TEXT_KEY_A, KeyInputSample::VALUE_PRESS),
        (TEXT_KEY_A, KeyInputSample::VALUE_RELEASE),
        (TEXT_KEY_ENTER, KeyInputSample::VALUE_PRESS),
        (TEXT_KEY_ENTER, KeyInputSample::VALUE_RELEASE),
        (TEXT_KEY_BACKSPACE, KeyInputSample::VALUE_PRESS),
        (TEXT_KEY_BACKSPACE, KeyInputSample::VALUE_RELEASE),
        (TEXT_KEY_A, KeyInputSample::VALUE_PRESS),
        (TEXT_KEY_A, KeyInputSample::VALUE_RELEASE),
        (TEXT_KEY_ENTER, KeyInputSample::VALUE_PRESS),
        (TEXT_KEY_ENTER, KeyInputSample::VALUE_RELEASE),
    ];
    let index = usize::from(text.active_transitions);
    if index >= EXPECTED.len()
        || sample.sequence() != u64::from(text.active_transitions) + 1
        || (sample.code(), sample.value()) != EXPECTED[index]
    {
        fail(FAIL_RUNTIME);
    }
    text.active_transitions += 1;
    if sample.value() == KeyInputSample::VALUE_RELEASE {
        text.active_releases += 1;
        return;
    }
    if sample.value() != KeyInputSample::VALUE_PRESS || text.tracker.outstanding_acknowledgment() {
        fail(FAIL_RUNTIME);
    }

    let context = text.context();
    let revision = text
        .tracker
        .revision()
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let sequence = text.next_event_sequence();
    let event = match (sample.code(), revision) {
        (TEXT_KEY_A, 1 | 4) => {
            text.preedit = Some('a');
            TextInputEvent::preedit(sequence, context, revision, 'a')
        }
        (TEXT_KEY_ENTER, 2 | 5) if text.preedit == Some('a') => {
            text.preedit = None;
            TextInputEvent::commit(sequence, context, revision, "a")
        }
        (TEXT_KEY_BACKSPACE, 3)
            if text.preedit.is_none() && text.state.committed().as_str() == "a" =>
        {
            TextInputEvent::delete_surrounding(sequence, context, revision, 1, 0)
        }
        _ => fail(FAIL_RUNTIME),
    }
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    text.send_event(endpoint, event);
}

#[cfg(feature = "soft-keyboard-runtime")]
fn send_soft_input_action(
    text: &mut TextServerRuntime,
    soft: &mut SoftKeyboardRuntime,
    endpoint: u64,
    action: InputAction,
) {
    if text.tracker.phase() != TextInputSessionPhase::Active
        || text.tracker.outstanding_acknowledgment()
        || text.context().session_id() != 2
        || text.context().focus_generation() != 7
        || soft.focus_stage != 1
    {
        fail(FAIL_RUNTIME);
    }
    let revision = text
        .tracker
        .revision()
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let sequence = text.next_event_sequence();
    let context = text.context();
    let event = match action {
        InputAction::Preedit('a') => {
            soft.preedit_actions = soft
                .preedit_actions
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_RUNTIME));
            text.preedit = Some('a');
            TextInputEvent::preedit(sequence, context, revision, 'a')
        }
        InputAction::Commit("a") if text.preedit == Some('a') => {
            soft.commit_actions = soft
                .commit_actions
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_RUNTIME));
            text.preedit = None;
            TextInputEvent::commit(sequence, context, revision, "a")
        }
        InputAction::DeleteSurrounding {
            before_scalars: 1,
            after_scalars: 0,
        } if text.preedit.is_none() => {
            soft.delete_actions = soft
                .delete_actions
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_RUNTIME));
            TextInputEvent::delete_surrounding(sequence, context, revision, 1, 0)
        }
        _ => fail(FAIL_RUNTIME),
    }
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    text.send_event(endpoint, event);
}

#[cfg(all(feature = "text-input-runtime", not(feature = "input-server-runtime")))]
fn read_text_key(capability: u64) -> Option<KeyInputSample> {
    let read = syscall(SyscallNumber::SurfaceReadKey, capability, 0, 0);
    if read.status == Status::ShouldWait.raw() {
        return None;
    }
    if read.status != Status::Ok.raw() || read.out2 == 0 {
        fail(FAIL_RUNTIME);
    }
    Some(
        KeyInputSample::decode_registers(read.out1, read.out2)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
    )
}

#[cfg(all(feature = "text-input-runtime", not(feature = "input-server-runtime")))]
#[allow(clippy::too_many_arguments)]
fn process_text_key(surface: &MultiWindowSurface, text: &mut TextServerRuntime) -> bool {
    let Some(sample) = read_text_key(surface.capability.raw()) else {
        return false;
    };
    let endpoint = surface
        .app
        .as_ref()
        .map(|app| app.endpoint.raw())
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    match text.tracker.phase() {
        TextInputSessionPhase::Active => send_text_key_event(text, endpoint, sample),
        TextInputSessionPhase::Deactivated => {
            let expected_value = if text.unfocused_transitions == 0 {
                KeyInputSample::VALUE_PRESS
            } else {
                KeyInputSample::VALUE_RELEASE
            };
            if text.unfocused_transitions >= 2
                || sample.sequence() != 11 + u64::from(text.unfocused_transitions)
                || sample.code() != TEXT_KEY_A
                || sample.value() != expected_value
            {
                fail(FAIL_RUNTIME);
            }
            text.unfocused_transitions += 1;
        }
        _ => fail(FAIL_RUNTIME),
    }
    true
}

#[cfg(feature = "input-server-runtime")]
fn process_server_text_key(
    surface: &mut MultiWindowSurface,
    text: &mut TextServerRuntime,
    target: Option<WindowRef>,
    key: InputKey,
    pressed: bool,
) {
    let sequence = surface.input_route.next_key_sequence();
    let code = match key {
        InputKey::A => TEXT_KEY_A,
        InputKey::Backspace => TEXT_KEY_BACKSPACE,
        InputKey::Enter => TEXT_KEY_ENTER,
    };
    let value = if pressed {
        KeyInputSample::VALUE_PRESS
    } else {
        KeyInputSample::VALUE_RELEASE
    };
    let sample =
        KeyInputSample::try_new(sequence, code, value).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let endpoint = surface
        .app
        .as_ref()
        .map(|app| app.endpoint.raw())
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    match text.tracker.phase() {
        TextInputSessionPhase::Active => {
            let expected = text.context().window_id();
            if target != Some(input_window_ref(expected)) {
                fail(FAIL_RUNTIME);
            }
            send_text_key_event(text, endpoint, sample);
        }
        TextInputSessionPhase::Deactivated => {
            let expected_value = if text.unfocused_transitions == 0 {
                KeyInputSample::VALUE_PRESS
            } else {
                KeyInputSample::VALUE_RELEASE
            };
            // InputServer routes every physical key to the one focused
            // generation even when no text context is active. Prove that the
            // unfocused text-editor traffic belongs to the launcher focus,
            // then account for it without forwarding it to the App editor.
            if surface.input_route.context.is_some()
                || target != surface.input_route.focus
                || target.is_none()
                || text.unfocused_transitions >= 2
                || sample.sequence() != 11 + u64::from(text.unfocused_transitions)
                || sample.code() != TEXT_KEY_A
                || sample.value() != expected_value
            {
                fail(FAIL_RUNTIME);
            }
            text.unfocused_transitions += 1;
        }
        _ => fail(FAIL_RUNTIME),
    }
}

#[cfg(feature = "input-server-runtime")]
fn process_server_semantic_event(
    surface: &mut MultiWindowSurface,
    text: &mut TextServerRuntime,
    soft: &mut SoftKeyboardRuntime,
    payload: ServerInputPayload,
) {
    let (context_id, target, revision, action) = match payload {
        ServerInputPayload::Preedit {
            context_id,
            target,
            revision,
            text,
        } if text.as_str() == "a" => (context_id, target, revision, InputAction::Preedit('a')),
        ServerInputPayload::Commit {
            context_id,
            target,
            revision,
            text,
        } if text.as_str() == "a" => (context_id, target, revision, InputAction::Commit("a")),
        ServerInputPayload::DeleteSurrounding {
            context_id,
            target,
            revision,
            before_scalars: 1,
            after_scalars: 0,
        } => (
            context_id,
            target,
            revision,
            InputAction::DeleteSurrounding {
                before_scalars: 1,
                after_scalars: 0,
            },
        ),
        _ => fail(FAIL_RUNTIME),
    };
    let context = surface
        .input_route
        .context
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let expected_revision = surface
        .input_route
        .semantic_revision
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if context.context_id() != context_id
        || context.target() != target
        || revision != expected_revision
        || !surface.input_route.overlay_visible
    {
        fail(FAIL_RUNTIME);
    }
    surface.input_route.semantic_revision = expected_revision;
    let endpoint = surface
        .app
        .as_ref()
        .map(|app| app.endpoint.raw())
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    send_soft_input_action(text, soft, endpoint, action);
}

#[cfg(feature = "text-input-runtime")]
#[allow(clippy::too_many_arguments)]
fn render_acknowledged_text_state(
    surface: &MultiWindowSurface,
    compositor: &mut Compositor<'static>,
    outputs: &mut [OutputBuffer; 2],
    app_state: &PersistentClientWindow<'static>,
    text: &mut TextServerRuntime,
    state: TextState,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) {
    let step = text
        .renders
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if step > 5 || state != expected_text_state(step) {
        fail(FAIL_RUNTIME);
    }
    let committed = state.committed();
    let pixels = render_text_field(
        committed.as_bytes(),
        text.preedit.map(|scalar| scalar as u32),
    );
    let expected_digest = match step {
        1 | 4 => TEXT_FIELD_PREEDIT_DIGEST,
        2 | 5 => TEXT_FIELD_COMMITTED_DIGEST,
        3 => TEXT_FIELD_EMPTY_DIGEST,
        _ => fail(FAIL_RUNTIME),
    };
    if text_field_digest(pixels) != expected_digest {
        fail(FAIL_RUNTIME);
    }

    let window_id = app_state.window_id.unwrap_or_else(|| fail(FAIL_RUNTIME));
    let output_index = usize::try_from(*global_frame & 1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let output = &mut outputs[output_index];
    sync_output(compositor, output);
    let report = compositor
        .apply_pixels(
            app_state.owner,
            window_id,
            TEXT_FIELD,
            pixels,
            output.pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if report.window_id != window_id
        || report.content_generation != u64::from(step) + 2
        || report.global_damage != ShellRect::new(56, 96, 96, 16)
    {
        fail(FAIL_RUNTIME);
    }
    validate_present_ledger(
        report.pixels,
        TEXT_FIELD_PIXEL_COUNT as u32,
        TEXT_FIELD_PIXEL_COUNT as u32,
        0,
        TEXT_FIELD_PIXEL_COUNT as u32,
        true,
    );
    publish_persistent_output(
        surface,
        output,
        report.pixels,
        global_frame,
        last_epoch,
        last_boundary,
    );
    let expected_frame = 8 + u32::from(step);
    let expected_generation = 4 + u64::from(step).div_ceil(2);
    if *global_frame != expected_frame || output.write_generation != expected_generation {
        fail(FAIL_RUNTIME);
    }
    text.state = state;
    text.renders = step;
}

#[cfg(feature = "soft-keyboard-runtime")]
#[allow(clippy::too_many_arguments)]
fn render_soft_text_state(
    surface: &MultiWindowSurface,
    compositor: &mut Compositor<'static>,
    outputs: &mut [OutputBuffer; 2],
    app_state: &PersistentClientWindow<'static>,
    text: &mut TextServerRuntime,
    soft: &mut SoftKeyboardRuntime,
    state: TextState,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) {
    let step = soft
        .renders
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if step > 5 || state != expected_soft_text_state(step) || !soft.is_visible() {
        fail(FAIL_RUNTIME);
    }
    let pixels = render_text_field(
        state.committed().as_bytes(),
        text.preedit.map(|scalar| scalar as u32),
    );
    let window_id = app_state.window_id.unwrap_or_else(|| fail(FAIL_RUNTIME));
    let output_index = usize::try_from(*global_frame & 1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let output = &mut outputs[output_index];
    sync_output(compositor, output);
    let report = compositor
        .apply_pixels(
            app_state.owner,
            window_id,
            TEXT_FIELD,
            pixels,
            output.pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if report.window_id != window_id
        || report.content_generation != u64::from(step) + 7
        || report.global_damage != ShellRect::new(56, 96, 96, 16)
        || compositor.system_overlay_info().is_none()
    {
        fail(FAIL_RUNTIME);
    }
    validate_present_ledger(
        report.pixels,
        TEXT_FIELD_PIXEL_COUNT as u32,
        TEXT_FIELD_PIXEL_COUNT as u32,
        0,
        TEXT_FIELD_PIXEL_COUNT as u32,
        true,
    );
    publish_persistent_output(
        surface,
        output,
        report.pixels,
        global_frame,
        last_epoch,
        last_boundary,
    );
    let expected_frame = 14 + u32::from(step);
    let expected_generation = match step {
        1 => 8,
        2 => 8,
        3 => 9,
        4 => 9,
        5 => 10,
        _ => fail(FAIL_RUNTIME),
    };
    if *global_frame != expected_frame || output.write_generation != expected_generation {
        fail(FAIL_RUNTIME);
    }
    text.state = state;
    soft.renders = step;
}

#[cfg(feature = "soft-keyboard-runtime")]
#[allow(clippy::too_many_arguments)]
fn dispatch_soft_text_input_command(
    surface: &mut MultiWindowSurface,
    compositor: &mut Compositor<'static>,
    outputs: &mut [OutputBuffer; 2],
    app_state: &PersistentClientWindow<'static>,
    text: &mut TextServerRuntime,
    soft: &mut SoftKeyboardRuntime,
    command: TextInputCommand,
    endpoint: u64,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) {
    let context = command.context();
    let expected_focus = match context.session_id() {
        2 => 7,
        3 => 9,
        _ => fail(FAIL_RUNTIME),
    };
    let expected_context = TextInputContext::try_new(
        app_state.window_id.unwrap_or_else(|| fail(FAIL_RUNTIME)),
        context.session_id(),
        expected_focus,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if context != expected_context || text.tracker.receive(command).is_err() {
        fail(FAIL_RUNTIME);
    }

    match command.payload() {
        TextInputCommandPayload::Activate if context.session_id() == 2 => {
            if command.sequence() != 8
                || soft.focus_stage != 1
                || soft.overlay_shows != 0
                || compositor.focus_state().window_id != app_state.window_id
                || compositor.focus_state().generation != 7
                || text.state != expected_text_state(5)
            {
                fail(FAIL_RUNTIME);
            }
            text.context = Some(expected_context);
            text.state = TextState::try_new(0, "a", 1, 1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
            text.preedit = None;
            show_soft_keyboard(
                surface,
                compositor,
                outputs,
                soft,
                app_state.window_id.unwrap_or_else(|| fail(FAIL_RUNTIME)),
                context.session_id(),
                global_frame,
                last_epoch,
                last_boundary,
            );
            if *global_frame != 14 {
                fail(FAIL_RUNTIME);
            }
            let sequence = text.next_event_sequence();
            let event = TextInputEvent::activated(sequence, expected_context, 8, 0)
                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            if sequence != 13 {
                fail(FAIL_RUNTIME);
            }
            text.send_event(endpoint, event);
        }
        TextInputCommandPayload::StateAck {
            acknowledged_event_sequence,
            state,
        } if context.session_id() == 2 => {
            let step = u8::try_from(
                command
                    .sequence()
                    .checked_sub(8)
                    .unwrap_or_else(|| fail(FAIL_RUNTIME)),
            )
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            let expected_event = 12 + u64::from(step) * 2;
            if !(1..=5).contains(&step)
                || acknowledged_event_sequence != expected_event
                || state != expected_soft_text_state(step)
            {
                fail(FAIL_RUNTIME);
            }
            render_soft_text_state(
                surface,
                compositor,
                outputs,
                app_state,
                text,
                soft,
                state,
                global_frame,
                last_epoch,
                last_boundary,
            );
            let sequence = text.next_event_sequence();
            let event =
                TextInputEvent::rendered(sequence, expected_context, command.sequence(), state)
                    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            if sequence != expected_event + 1 {
                fail(FAIL_RUNTIME);
            }
            text.send_event(endpoint, event);
        }
        TextInputCommandPayload::Deactivate if context.session_id() == 2 => {
            if command.sequence() != 14
                || soft.focus_stage != 2
                || soft.renders != 5
                || text.state != expected_soft_text_state(5)
                || compositor.focus_state().window_id == app_state.window_id
                || compositor.focus_state().generation != 8
            {
                fail(FAIL_RUNTIME);
            }
            hide_soft_keyboard(
                surface,
                compositor,
                outputs,
                soft,
                global_frame,
                last_epoch,
                last_boundary,
            );
            if *global_frame != 20 {
                fail(FAIL_RUNTIME);
            }
            let sequence = text.next_event_sequence();
            let event = TextInputEvent::deactivated(
                sequence,
                expected_context,
                command.sequence(),
                text.state.revision(),
            )
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            if sequence != 24 {
                fail(FAIL_RUNTIME);
            }
            text.send_event(endpoint, event);
        }
        TextInputCommandPayload::Activate if context.session_id() == 3 => {
            let statistics = soft.statistics();
            if command.sequence() != 15
                || soft.focus_stage != 3
                || soft.renders != 5
                || soft.preedit_actions != 2
                || soft.commit_actions != 2
                || soft.delete_actions != 1
                || soft.overlay_shows != 1
                || soft.overlay_hides != 1
                || statistics.contacts != 5
                || statistics.activations != 5
                || statistics.cancels != 0
                || statistics.hidden_drops != 6
                || compositor.focus_state().window_id != app_state.window_id
                || compositor.focus_state().generation != 9
                || text.state != expected_soft_text_state(5)
            {
                fail(FAIL_RUNTIME);
            }
            text.context = Some(expected_context);
            text.state = TextState::try_new(0, "aa", 2, 2).unwrap_or_else(|_| fail(FAIL_RUNTIME));
            text.preedit = None;
            show_soft_keyboard(
                surface,
                compositor,
                outputs,
                soft,
                app_state.window_id.unwrap_or_else(|| fail(FAIL_RUNTIME)),
                context.session_id(),
                global_frame,
                last_epoch,
                last_boundary,
            );
            if *global_frame != 21 || soft.overlay_shows != 2 {
                fail(FAIL_RUNTIME);
            }
            let sequence = text.next_event_sequence();
            let event = TextInputEvent::activated(sequence, expected_context, 15, 0)
                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            if sequence != 25 {
                fail(FAIL_RUNTIME);
            }
            text.send_event(endpoint, event);
        }
        _ => fail(FAIL_RUNTIME),
    }
}

#[cfg(feature = "text-input-runtime")]
#[allow(clippy::too_many_arguments)]
fn dispatch_text_input_command(
    surface: &mut MultiWindowSurface,
    compositor: &mut Compositor<'static>,
    outputs: &mut [OutputBuffer; 2],
    app_state: &PersistentClientWindow<'static>,
    text: &mut TextServerRuntime,
    #[cfg(feature = "soft-keyboard-runtime")] soft: &mut SoftKeyboardRuntime,
    command: TextInputCommand,
    endpoint: u64,
    global_frame: &mut u32,
    last_epoch: &mut u64,
    last_boundary: &mut u64,
) {
    #[cfg(feature = "soft-keyboard-runtime")]
    if command.context().session_id() != 1 {
        dispatch_soft_text_input_command(
            surface,
            compositor,
            outputs,
            app_state,
            text,
            soft,
            command,
            endpoint,
            global_frame,
            last_epoch,
            last_boundary,
        );
        return;
    }
    let expected_context = TextInputContext::try_new(
        app_state.window_id.unwrap_or_else(|| fail(FAIL_RUNTIME)),
        1,
        5,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if command.context() != expected_context || text.tracker.receive(command).is_err() {
        fail(FAIL_RUNTIME);
    }

    match command.payload() {
        TextInputCommandPayload::Activate => {
            if command.sequence() != 1
                || text.context.is_some()
                || !text.app_focus_announced
                || compositor.focus_state().window_id != app_state.window_id
                || compositor.focus_state().generation != 5
            {
                fail(FAIL_RUNTIME);
            }
            text.context = Some(expected_context);
            text.state = TextState::default();
            let sequence = text.next_event_sequence();
            let event = TextInputEvent::activated(sequence, expected_context, 1, 0)
                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            text.send_event(endpoint, event);
        }
        TextInputCommandPayload::StateAck {
            acknowledged_event_sequence,
            state,
        } => {
            let step = command
                .sequence()
                .checked_sub(1)
                .unwrap_or_else(|| fail(FAIL_RUNTIME));
            let step = u8::try_from(step).unwrap_or_else(|_| fail(FAIL_RUNTIME));
            let expected_event_sequence = u64::from(step) * 2;
            if !(1..=5).contains(&step)
                || acknowledged_event_sequence != expected_event_sequence
                || state != expected_text_state(step)
            {
                fail(FAIL_RUNTIME);
            }
            render_acknowledged_text_state(
                surface,
                compositor,
                outputs,
                app_state,
                text,
                state,
                global_frame,
                last_epoch,
                last_boundary,
            );
            let sequence = text.next_event_sequence();
            let event =
                TextInputEvent::rendered(sequence, expected_context, command.sequence(), state)
                    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            text.send_event(endpoint, event);
        }
        TextInputCommandPayload::Deactivate => {
            if command.sequence() != 7
                || text.renders != 5
                || !text.launcher_focus_announced
                || compositor.focus_state().window_id == app_state.window_id
                || compositor.focus_state().generation != 6
            {
                fail(FAIL_RUNTIME);
            }
            let sequence = text.next_event_sequence();
            let event = TextInputEvent::deactivated(
                sequence,
                expected_context,
                command.sequence(),
                text.state.revision(),
            )
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            text.send_event(endpoint, event);
        }
    }
}

#[cfg(feature = "persistent-window-runtime")]
#[allow(clippy::too_many_arguments)]
fn persistent_window_session(
    surface: &mut MultiWindowSurface,
    outputs: &mut [OutputBuffer; 2],
    compositor: &mut Compositor<'static>,
    launcher_commands: WindowCommandTracker,
    app_commands: WindowCommandTracker,
    launcher_window: WindowId,
    app_window: WindowId,
    mut scene_frame: u32,
    mut global_frame: u32,
    mut last_epoch: u64,
    mut last_boundary: u64,
) -> ! {
    let app_owner = surface
        .app
        .as_ref()
        .map(|app| app.pid)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let mut launcher_state = PersistentClientWindow {
        client: WindowClient::Launcher,
        owner: surface.launcher_pid,
        slot: 0,
        bounds: LAUNCHER_BOUNDS,
        window_id: Some(launcher_window),
        free_layer: None,
        commands: launcher_commands,
        last_command: 5,
        last_frame: Some(3),
    };
    let mut app_state = PersistentClientWindow {
        client: WindowClient::App,
        owner: app_owner,
        slot: 1,
        bounds: APP_BOUNDS,
        window_id: Some(app_window),
        free_layer: None,
        commands: app_commands,
        last_command: 4,
        last_frame: Some(2),
    };
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut launcher_alive = true;
    let mut pointer_contact = PersistentPointerContact::new();
    let mut ready_sent = false;
    #[cfg(feature = "text-input-runtime")]
    let mut text = TextServerRuntime::new();
    #[cfg(feature = "soft-keyboard-runtime")]
    let mut soft = SoftKeyboardRuntime::new();

    loop {
        #[cfg(all(feature = "text-input-runtime", not(feature = "input-server-runtime")))]
        let surface_requested = if text.key_wait_enabled() {
            signal_union(requested, ObjectSignals::KEY_READY)
        } else {
            requested
        };
        #[cfg(any(not(feature = "text-input-runtime"), feature = "input-server-runtime"))]
        let surface_requested = requested;
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        let mut sources = [0_u8; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        let mut count = 0_usize;
        #[cfg(not(feature = "input-server-runtime"))]
        {
            items[count] = pack_user_wait_item(surface.capability.raw(), surface_requested);
        }
        #[cfg(feature = "input-server-runtime")]
        {
            items[count] =
                pack_user_wait_item(surface.input_route.endpoint.raw(), surface_requested);
        }
        sources[count] = 0;
        count += 1;
        items[count] = pack_user_wait_item(surface.control.raw(), requested);
        sources[count] = 1;
        count += 1;
        if launcher_alive {
            items[count] = pack_user_wait_item(surface.launcher.raw(), requested);
            sources[count] = 2;
            count += 1;
        }
        if let Some(app) = surface.app.as_ref() {
            items[count] = pack_user_wait_item(app.endpoint.raw(), requested);
            sources[count] = 3;
            count += 1;
        }

        #[cfg(feature = "input-server-runtime")]
        let ready = if surface.input_route.has_pending() {
            None
        } else {
            Some(object_wait_many_array(
                &items,
                count,
                OBJECT_WAIT_TIMEOUT_INFINITE,
            ))
        };
        #[cfg(not(feature = "input-server-runtime"))]
        let ready = Some(object_wait_many_array(
            &items,
            count,
            OBJECT_WAIT_TIMEOUT_INFINITE,
        ));
        let index = ready.as_ref().map_or(0, |result| {
            usize::try_from(result.out1).unwrap_or_else(|_| fail(FAIL_RUNTIME))
        });
        let observed = ready
            .as_ref()
            .map_or(u64::from(ObjectSignals::READABLE.bits()), |result| {
                result.out2
            });
        let observed_requested = if sources.get(index).copied() == Some(0) {
            surface_requested
        } else {
            requested
        };
        if ready
            .as_ref()
            .is_some_and(|result| result.status != Status::Ok.raw())
            || index >= count
            || observed & u64::from(observed_requested.bits()) == 0
        {
            fail(FAIL_RUNTIME);
        }
        let readable = observed & u64::from(ObjectSignals::READABLE.bits()) != 0;
        let peer_closed = observed & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0;

        match sources[index] {
            0 => {
                if peer_closed {
                    fail(FAIL_RUNTIME);
                }
                if readable {
                    #[cfg(feature = "input-server-runtime")]
                    let routed = {
                        let event = surface.input_route.read_event();
                        match event.payload() {
                            ServerInputPayload::RoutedPointer(pointer) => {
                                route_server_persistent_pointer(
                                    surface,
                                    compositor,
                                    launcher_state.window_id,
                                    app_state.window_id,
                                    launcher_state.last_command,
                                    app_state.last_command,
                                    scene_frame,
                                    &mut pointer_contact,
                                    &mut soft,
                                    pointer,
                                )
                            }
                            ServerInputPayload::RoutedKey {
                                input_sequence,
                                target,
                                key,
                                pressed,
                            } => {
                                surface.input_route.accept_physical_sequence(input_sequence);
                                process_server_text_key(surface, &mut text, target, key, pressed);
                                false
                            }
                            payload @ (ServerInputPayload::Preedit { .. }
                            | ServerInputPayload::Commit { .. }
                            | ServerInputPayload::DeleteSurrounding { .. }) => {
                                process_server_semantic_event(
                                    surface, &mut text, &mut soft, payload,
                                );
                                false
                            }
                            _ => fail(FAIL_RUNTIME),
                        }
                    };
                    #[cfg(all(
                        feature = "soft-keyboard-runtime",
                        not(feature = "input-server-runtime")
                    ))]
                    let routed = {
                        let sample = read_persistent_pointer_sample(surface)
                            .unwrap_or_else(|| fail(FAIL_RUNTIME));
                        // The input method does not exist in the routing graph
                        // until its trusted overlay has been installed once.
                        // This keeps the immutable M41--M43 pointer prefix out
                        // of M44's hidden-input ledger while still accounting
                        // for every report between hide and session-3 show.
                        let (consumed, action) = if soft.overlay_shows == 0 {
                            (false, None)
                        } else {
                            let outcome = soft.ime.handle(sample);
                            (outcome.consumed(), outcome.action())
                        };
                        if consumed {
                            if let Some(action) = action {
                                let endpoint = surface
                                    .app
                                    .as_ref()
                                    .map(|app| app.endpoint.raw())
                                    .unwrap_or_else(|| fail(FAIL_RUNTIME));
                                send_soft_input_action(&mut text, &mut soft, endpoint, action);
                            }
                            false
                        } else {
                            route_persistent_pointer_sample(
                                surface,
                                compositor,
                                launcher_state.window_id,
                                app_state.window_id,
                                launcher_state.last_command,
                                app_state.last_command,
                                scene_frame,
                                &mut pointer_contact,
                                sample,
                            )
                        }
                    };
                    #[cfg(all(
                        not(feature = "soft-keyboard-runtime"),
                        not(feature = "input-server-runtime")
                    ))]
                    let routed = route_persistent_pointer_event(
                        surface,
                        compositor,
                        launcher_state.window_id,
                        app_state.window_id,
                        launcher_state.last_command,
                        app_state.last_command,
                        scene_frame,
                        &mut pointer_contact,
                    );
                    #[cfg(not(feature = "text-input-runtime"))]
                    let _ = routed;
                    #[cfg(feature = "text-input-runtime")]
                    if routed {
                        let focus = compositor.focus_state();
                        if focus.window_id == app_state.window_id
                            && focus.generation == 5
                            && !text.app_focus_announced
                        {
                            if app_state.last_command != 8
                                || app_state.last_frame != Some(1)
                                || !pointer_contact.contact_active
                            {
                                fail(FAIL_RUNTIME);
                            }
                            // The compositor's focus generation is 5 after the immutable M42
                            // pointer transcript.  Legacy UI focus notifications have their own
                            // contiguous stream and therefore advance from 2 to 3 here.
                            surface.lifecycle_focus_generation = 3;
                            publish_legacy_focus(surface, UiClientId::App);
                            text.app_focus_announced = true;
                        } else if focus.window_id == launcher_state.window_id
                            && focus.generation == 6
                            && !text.launcher_focus_announced
                        {
                            if text.renders != 5
                                || text.tracker.phase() != TextInputSessionPhase::Active
                                || text.tracker.outstanding_acknowledgment()
                                || !pointer_contact.contact_active
                            {
                                fail(FAIL_RUNTIME);
                            }
                            // Keep the legacy UI stream contiguous while the compositor advances
                            // its independently authenticated focus generation from 5 to 6.
                            surface.lifecycle_focus_generation = 4;
                            publish_legacy_focus(surface, UiClientId::Launcher);
                            text.launcher_focus_announced = true;
                        }
                        #[cfg(feature = "soft-keyboard-runtime")]
                        if focus.window_id == app_state.window_id
                            && focus.generation == 7
                            && soft.focus_stage == 0
                        {
                            if text.unfocused_transitions != 2
                                || text.tracker.phase() != TextInputSessionPhase::Deactivated
                                || text.tracker.outstanding_acknowledgment()
                                || soft.is_visible()
                                || !pointer_contact.contact_active
                                || global_frame != 13
                            {
                                fail(FAIL_RUNTIME);
                            }
                            surface.lifecycle_focus_generation = 5;
                            publish_legacy_focus(surface, UiClientId::App);
                            soft.focus_stage = 1;
                        }
                        #[cfg(feature = "soft-keyboard-runtime")]
                        if focus.window_id == launcher_state.window_id
                            && focus.generation == 8
                            && soft.focus_stage == 1
                            && pointer_contact.is_quiescent()
                        {
                            #[cfg(not(feature = "input-server-runtime"))]
                            let input_visibility_valid = soft.is_visible();
                            #[cfg(feature = "input-server-runtime")]
                            let input_visibility_valid = !soft.is_visible();
                            if soft.renders != 5
                                || text.tracker.phase() != TextInputSessionPhase::Active
                                || text.tracker.outstanding_acknowledgment()
                                || !input_visibility_valid
                                || !pointer_contact.is_quiescent()
                                || global_frame != 19
                            {
                                fail(FAIL_RUNTIME);
                            }
                            surface.lifecycle_focus_generation = 6;
                            publish_legacy_focus(surface, UiClientId::Launcher);
                            soft.focus_stage = 2;
                        }
                        #[cfg(feature = "soft-keyboard-runtime")]
                        if focus.window_id == app_state.window_id
                            && focus.generation == 9
                            && soft.focus_stage == 2
                        {
                            if text.tracker.phase() != TextInputSessionPhase::Deactivated
                                || text.tracker.outstanding_acknowledgment()
                                || soft.is_visible()
                                || !pointer_contact.contact_active
                                || global_frame != 20
                            {
                                fail(FAIL_RUNTIME);
                            }
                            surface.lifecycle_focus_generation = 7;
                            publish_legacy_focus(surface, UiClientId::App);
                            soft.focus_stage = 3;
                        }
                    }
                }
                #[cfg(all(feature = "text-input-runtime", not(feature = "input-server-runtime")))]
                if observed & u64::from(ObjectSignals::KEY_READY.bits()) != 0
                    && !process_text_key(surface, &mut text)
                {
                    fail(FAIL_RUNTIME);
                }
            }
            1 => {
                if readable {
                    dispatch_multi_window_supervisor(surface);
                } else if peer_closed {
                    fail(FAIL_RUNTIME);
                }
            }
            2 => {
                #[cfg(feature = "input-server-runtime")]
                if !surface.input_route.scene_update_active {
                    surface.input_route.begin_scene_update();
                    if surface.input_route.has_pending() {
                        continue;
                    }
                }
                if readable {
                    let (command, _) =
                        decode_window_command(surface.launcher.raw(), launcher_state.owner);
                    #[cfg(feature = "input-server-runtime")]
                    prepare_input_window_command(surface, command);
                    let witness_complete = dispatch_persistent_window_command(
                        surface,
                        compositor,
                        outputs,
                        &mut launcher_state,
                        command,
                        surface.launcher.raw(),
                        &mut scene_frame,
                        &mut global_frame,
                        &mut last_epoch,
                        &mut last_boundary,
                    );
                    #[cfg(feature = "input-server-runtime")]
                    finish_input_window_command(
                        surface,
                        compositor,
                        command,
                        launcher_state
                            .window_id
                            .map(|window_id| (window_id, launcher_state.owner)),
                        app_state
                            .window_id
                            .map(|window_id| (window_id, app_state.owner)),
                    );
                    pointer_contact.reconcile_capture(compositor.captured_window().is_some());
                    if witness_complete && !ready_sent && !pointer_contact.is_quiescent() {
                        fail(FAIL_RUNTIME);
                    }
                    #[cfg(not(feature = "text-input-runtime"))]
                    if witness_complete && !ready_sent {
                        write_wire(surface.control.raw(), &READY_MAGIC.to_le_bytes());
                        ready_sent = true;
                    }
                } else if peer_closed {
                    cleanup_persistent_client(
                        surface,
                        compositor,
                        outputs,
                        &mut launcher_state,
                        None,
                        app_state
                            .window_id
                            .map(|window_id| (window_id, app_state.owner)),
                        &mut global_frame,
                        &mut last_epoch,
                        &mut last_boundary,
                    );
                    pointer_contact.reconcile_capture(compositor.captured_window().is_some());
                    let closed = syscall(SyscallNumber::HandleClose, surface.launcher.raw(), 0, 0);
                    if closed.status != Status::Ok.raw() || closed.out1 != 0 || closed.out2 != 0 {
                        fail(FAIL_RUNTIME);
                    }
                    launcher_alive = false;
                }
                #[cfg(feature = "input-server-runtime")]
                surface.input_route.commit_scene_update();
            }
            3 => {
                let endpoint = surface
                    .app
                    .as_ref()
                    .map(|app| app.endpoint.raw())
                    .unwrap_or_else(|| fail(FAIL_RUNTIME));
                #[cfg(feature = "input-server-runtime")]
                if !surface.input_route.scene_update_active {
                    surface.input_route.begin_scene_update();
                    if surface.input_route.has_pending() {
                        continue;
                    }
                }
                if readable {
                    #[cfg(feature = "text-input-runtime")]
                    match decode_app_runtime_command(endpoint, app_state.owner) {
                        AppRuntimeCommand::Window(command) => {
                            #[cfg(feature = "input-server-runtime")]
                            prepare_input_window_command(surface, command);
                            let witness_complete = dispatch_persistent_window_command(
                                surface,
                                compositor,
                                outputs,
                                &mut app_state,
                                command,
                                endpoint,
                                &mut scene_frame,
                                &mut global_frame,
                                &mut last_epoch,
                                &mut last_boundary,
                            );
                            #[cfg(feature = "input-server-runtime")]
                            finish_input_window_command(
                                surface,
                                compositor,
                                command,
                                launcher_state
                                    .window_id
                                    .map(|window_id| (window_id, launcher_state.owner)),
                                app_state
                                    .window_id
                                    .map(|window_id| (window_id, app_state.owner)),
                            );
                            pointer_contact
                                .reconcile_capture(compositor.captured_window().is_some());
                            if witness_complete && !pointer_contact.is_quiescent() {
                                fail(FAIL_RUNTIME);
                            }
                        }
                        AppRuntimeCommand::Text(command) => dispatch_text_input_command(
                            surface,
                            compositor,
                            outputs,
                            &app_state,
                            &mut text,
                            #[cfg(feature = "soft-keyboard-runtime")]
                            &mut soft,
                            command,
                            endpoint,
                            &mut global_frame,
                            &mut last_epoch,
                            &mut last_boundary,
                        ),
                    }
                    #[cfg(not(feature = "text-input-runtime"))]
                    {
                        let (command, _) = decode_window_command(endpoint, app_state.owner);
                        let witness_complete = dispatch_persistent_window_command(
                            surface,
                            compositor,
                            outputs,
                            &mut app_state,
                            command,
                            endpoint,
                            &mut scene_frame,
                            &mut global_frame,
                            &mut last_epoch,
                            &mut last_boundary,
                        );
                        pointer_contact.reconcile_capture(compositor.captured_window().is_some());
                        if witness_complete && !ready_sent {
                            if !pointer_contact.is_quiescent() {
                                fail(FAIL_RUNTIME);
                            }
                            write_wire(surface.control.raw(), &READY_MAGIC.to_le_bytes());
                            ready_sent = true;
                        }
                    }
                } else if peer_closed {
                    cleanup_persistent_client(
                        surface,
                        compositor,
                        outputs,
                        &mut app_state,
                        launcher_state
                            .window_id
                            .map(|window_id| (window_id, launcher_state.owner)),
                        None,
                        &mut global_frame,
                        &mut last_epoch,
                        &mut last_boundary,
                    );
                    pointer_contact.reconcile_capture(compositor.captured_window().is_some());
                    let app = surface.app.take().unwrap_or_else(|| fail(FAIL_RUNTIME));
                    close_owned(app.endpoint);
                }
                #[cfg(feature = "input-server-runtime")]
                surface.input_route.commit_scene_update();
            }
            _ => fail(FAIL_RUNTIME),
        }

        #[cfg(feature = "text-input-runtime")]
        if text.unfocused_transitions == 2 && !ready_sent {
            if text.tracker.phase() != TextInputSessionPhase::Deactivated
                || text.active_transitions != 10
                || text.active_releases != 5
                || text.renders != 5
                || !text.app_focus_announced
                || !text.launcher_focus_announced
                || !pointer_contact.is_quiescent()
                || global_frame != 13
                || scene_frame != 13
            {
                fail(FAIL_RUNTIME);
            }
            // This is the lifecycle supervisor's resident-topology handoff,
            // not the leaf runtime's final success signal.  M44 continues to
            // accept tablet input after init consumes the same M43-prefix
            // magic and returns to its permanent wait array.
            write_wire(surface.control.raw(), &READY_MAGIC.to_le_bytes());
            ready_sent = true;
        }

        #[cfg(feature = "soft-keyboard-runtime")]
        if soft.focus_stage == 3
            && text.context().session_id() == 3
            && text.tracker.phase() == TextInputSessionPhase::Active
            && !text.tracker.outstanding_acknowledgment()
            && ready_sent
        {
            let statistics = soft.statistics();
            if text.event_sequence != 25
                || text.state
                    != TextState::try_new(0, "aa", 2, 2).unwrap_or_else(|_| fail(FAIL_RUNTIME))
                || !soft.is_visible()
                || soft.free_layer.is_some()
                || compositor.system_overlay_info().is_none()
                || statistics.contacts != 5
                || statistics.activations != 5
                || statistics.cancels != 0
                || statistics.hidden_drops != 6
                || !pointer_contact.is_quiescent()
                || global_frame != 21
                || scene_frame != 13
            {
                fail(FAIL_RUNTIME);
            }
        }
    }
}

#[cfg(any(
    feature = "service-dependency-runtime",
    all(
        feature = "input-server-surface-restart-runtime",
        not(feature = "input-server-restart-runtime")
    )
))]
fn surface_recovery_runtime(
    startup: u64,
    init_pid: u64,
    control: OwnedUserHandle,
    capability: OwnedUserHandle,
    mut input_route: InputRouteClient,
    session_id: u64,
) -> ! {
    if session_id != 2 {
        fail(FAIL_RUNTIME);
    }
    let (app_init_pid, app_endpoint) =
        read_bootstrap_transfer(startup, UiBootstrapEndpointKind::AppUi);
    if app_init_pid != init_pid {
        close_owned(app_endpoint);
        fail(FAIL_RUNTIME);
    }
    let bootstrap = read_surface_recovery_message(startup, init_pid);
    let SurfaceRecoveryPayload::Bootstrap {
        input_server_pid,
        input_session_id,
        physical_floor,
        checkpoint,
        expected_gap_events,
        cancel_kind,
    } = bootstrap.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if bootstrap.sender_sequence() != 1
        || bootstrap.surface_session() != session_id
        || bootstrap.route_epoch() != 2
        || input_server_pid != input_route.server_pid
        || input_session_id == 0
        || (cfg!(feature = "service-dependency-runtime") && input_session_id != 1)
        || physical_floor != 2
        || checkpoint != SurfaceRecoveryCheckpoint::MultiWindowInteractive
        || expected_gap_events != 1
        || cancel_kind != RecoveryCancelKind::ClientPointer
        || input_route.last_physical_sequence != 0
    {
        fail(FAIL_RUNTIME);
    }
    input_route.last_physical_sequence = physical_floor;
    #[cfg(feature = "service-dependency-runtime")]
    {
        if input_route.input_session_id != 1 || input_route.route_epoch != 1 {
            fail(FAIL_RUNTIME);
        }
        input_route.input_session_id = input_session_id;
        input_route.route_epoch = bootstrap.route_epoch();
    }
    let launcher = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let launcher_window = WindowId::try_new(0, 1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let app_window = WindowId::try_new(1, 1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let launcher_offer = SurfaceRecoveryMessage::client_rebind_offer(
        1,
        session_id,
        2,
        RecoveryClientRole::Launcher,
        RecoveryCancelKind::None,
        checkpoint,
        launcher_window,
        physical_floor,
        0,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let app_offer = SurfaceRecoveryMessage::client_rebind_offer(
        1,
        session_id,
        2,
        RecoveryClientRole::App,
        RecoveryCancelKind::ClientPointer,
        checkpoint,
        app_window,
        physical_floor,
        physical_floor,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(launcher.raw(), &launcher_offer.encode());
    write_wire(app_endpoint.raw(), &app_offer.encode());
    let graph_prepared = SurfaceRecoveryMessage::status(
        1,
        session_id,
        2,
        checkpoint,
        physical_floor,
        SurfaceRecoveryPhase::GraphPrepared,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(control.raw(), &graph_prepared.encode());

    let launcher_pid = read_surface_rebind_ack(launcher.raw(), launcher_offer, launcher_window);
    let app_pid = read_surface_rebind_ack(app_endpoint.raw(), app_offer, app_window);
    if launcher_pid == 0
        || app_pid == 0
        || launcher_pid == app_pid
        || launcher_pid == init_pid
        || app_pid == init_pid
        || launcher_pid == input_server_pid
        || app_pid == input_server_pid
    {
        fail(FAIL_RUNTIME);
    }

    let outputs = &mut create_output_swapchain();
    let mut compositor: Compositor<'static> = Compositor::new();
    sync_output(&compositor, &mut outputs[0]);
    let launcher_layer = &mut layer_mut(0)[..SURFACE_PIXEL_COUNT];
    let created_launcher = compositor
        .create(
            launcher_pid,
            LAUNCHER_BOUNDS,
            bndr_compositor::BACKGROUND_COLOR,
            launcher_layer,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if created_launcher.window_id != launcher_window {
        fail(FAIL_RUNTIME);
    }
    compositor
        .present(
            launcher_pid,
            launcher_window,
            LAUNCHER_BOUNDS,
            COLOR_LAUNCHER_BASE,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));

    sync_output(&compositor, &mut outputs[1]);
    let app_layer_area = usize::from(APP_BOUNDS.width) * usize::from(APP_BOUNDS.height);
    let app_layer = &mut layer_mut(1)[..app_layer_area];
    let created_app = compositor
        .create(
            app_pid,
            APP_BOUNDS,
            bndr_compositor::BACKGROUND_COLOR,
            app_layer,
            outputs[1].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if created_app.window_id != app_window {
        fail(FAIL_RUNTIME);
    }
    compositor
        .present(
            app_pid,
            app_window,
            ShellRect::new(0, 0, APP_BOUNDS.width, APP_BOUNDS.height),
            COLOR_APP_BASE,
            outputs[1].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    sync_output(&compositor, &mut outputs[0]);
    compositor
        .present(
            launcher_pid,
            launcher_window,
            LAUNCHER_HIDDEN_DAMAGE,
            COLOR_LAUNCHER_HIDDEN,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if compositor
        .focus(app_pid, app_window)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
        .generation
        != 1
    {
        fail(FAIL_RUNTIME);
    }
    sync_output(&compositor, &mut outputs[0]);
    compositor
        .present(
            launcher_pid,
            launcher_window,
            LAUNCHER_PARTIAL_DAMAGE,
            COLOR_LAUNCHER_PARTIAL,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    sync_output(&compositor, &mut outputs[1]);
    compositor
        .present(
            app_pid,
            app_window,
            APP_DAMAGE,
            COLOR_APP_DAMAGE,
            outputs[1].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    sync_output(&compositor, &mut outputs[0]);
    compositor
        .raise(launcher_pid, launcher_window, outputs[0].pixels_mut())
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if compositor
        .focus(launcher_pid, launcher_window)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
        .generation
        != 2
    {
        fail(FAIL_RUNTIME);
    }
    sync_output(&compositor, &mut outputs[1]);
    compositor
        .raise(app_pid, app_window, outputs[1].pixels_mut())
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));

    // Reconstruct the server-side contact, then cancel it without emitting a
    // BWE1 release. The client applies the matching atomic cancel in BSR1.
    let trigger = compositor
        .pointer_down(80, 120)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if trigger.window_id != app_window
        || trigger.focus_generation != 3
        || compositor.captured_window() != Some(app_window)
    {
        fail(FAIL_RUNTIME);
    }
    let cancelled = compositor
        .pointer_up(80, 120)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if cancelled.window_id != app_window
        || cancelled.focus_generation != 3
        || compositor.captured_window().is_some()
    {
        fail(FAIL_RUNTIME);
    }

    let identity =
        AppInstanceIdentity::try_new(1, process_slot(app_pid), process_generation(app_pid))
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let mut surface = MultiWindowSurface {
        launcher,
        control,
        capability,
        init_pid,
        session_id,
        supervisor: UiSupervisorTracker::new(),
        app: Some(InstalledApp {
            endpoint: app_endpoint,
            pid: app_pid,
            identity,
        }),
        launcher_pid,
        supervisor_command_sequence: 2,
        supervisor_response_sequence: 2,
        #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
        lifecycle_focus_generation: 1,
        #[cfg(not(feature = "post-recovery-lifecycle-focus-runtime"))]
        lifecycle_focus_generation: 2,
        input_route,
    };

    // Session 2 owns a fresh frame ledger. Publish one recovery-only visual
    // transition so the replacement clears its initial frame obligation
    // without advancing the sealed BWC1/BWE1 scene transcript.
    let mut recovery_frame = 0_u32;
    let mut recovery_epoch = 0_u64;
    let mut recovery_boundary = 0_u64;
    if outputs[0].write_generation != 0 || SURFACE_RECOVERY_DAMAGE.width == 0 {
        fail(FAIL_RUNTIME);
    }
    prepare_visible_output(
        &surface,
        &mut outputs[0],
        &compositor,
        &mut recovery_epoch,
        &mut recovery_boundary,
    );
    let captured_digest = output_digest(outputs[0].pixels_mut());
    let report = compositor
        .present(
            app_pid,
            app_window,
            SURFACE_RECOVERY_DAMAGE,
            COLOR_SURFACE_RECOVERY_ACTIVE,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let recovery_pixels =
        u32::from(SURFACE_RECOVERY_DAMAGE.width) * u32::from(SURFACE_RECOVERY_DAMAGE.height);
    if report.content_generation != 4
        || report.global_damage
            != ShellRect::new(
                APP_BOUNDS.x + SURFACE_RECOVERY_DAMAGE.x,
                APP_BOUNDS.y + SURFACE_RECOVERY_DAMAGE.y,
                SURFACE_RECOVERY_DAMAGE.width,
                SURFACE_RECOVERY_DAMAGE.height,
            )
        || recovery_pixels > 8_192
        || output_digest(outputs[0].pixels_mut()) == captured_digest
    {
        fail(FAIL_RUNTIME);
    }
    validate_present_ledger(
        report.pixels,
        recovery_pixels,
        recovery_pixels,
        0,
        recovery_pixels,
        true,
    );
    publish_visible_output(&surface, &mut outputs[0], &mut recovery_frame);
    if recovery_frame != 1
        || recovery_epoch != 1
        || recovery_boundary == 0
        || outputs[0].write_generation != 1
    {
        fail(FAIL_RUNTIME);
    }

    publish_input_routes_incremental(
        &mut surface,
        &compositor,
        Some((launcher_window, launcher_pid)),
        Some((app_window, app_pid)),
    );
    surface.input_route.begin_scene_update();
    surface.input_route.commit_scene_update();
    let release = surface.input_route.read_event();
    let ServerInputPayload::RoutedPointer(release) = release.payload() else {
        fail(FAIL_RUNTIME);
    };
    surface
        .input_route
        .accept_physical_sequence(release.input_sequence);
    if release.input_sequence != 3
        || release.target.is_some()
        || release.pressed
        || release.captured
        || release.trusted_overlay
        || (release.x, release.y) != (136, 184)
        || compositor.focus_state().window_id != Some(app_window)
        || compositor.focus_state().generation != 3
        || compositor.captured_window().is_some()
    {
        fail(FAIL_RUNTIME);
    }

    #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
    publish_post_recovery_ready_focus(&surface);

    let active = SurfaceRecoveryMessage::status(
        2,
        session_id,
        2,
        checkpoint,
        release.input_sequence,
        SurfaceRecoveryPhase::Active,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(surface.control.raw(), &active.encode());
    write_wire(surface.control.raw(), &READY_MAGIC.to_le_bytes());

    #[cfg(feature = "service-dependency-runtime")]
    finish_service_dependency_surface(
        &mut surface,
        outputs,
        &mut compositor,
        launcher_window,
        app_window,
        &mut recovery_frame,
        &mut recovery_epoch,
        &mut recovery_boundary,
    );

    #[cfg(not(feature = "service-dependency-runtime"))]
    {
        let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
        let app = surface.app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(surface.input_route.endpoint.raw(), requested);
        items[1] = pack_user_wait_item(surface.control.raw(), requested);
        items[2] = pack_user_wait_item(surface.launcher.raw(), requested);
        items[3] = pack_user_wait_item(app.endpoint.raw(), requested);
        let ready = object_wait_many_array(&items, 4, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw() {
            fail(FAIL_RUNTIME);
        }
        fail(FAIL_RUNTIME)
    }
}

#[cfg(feature = "service-dependency-runtime")]
enum ServiceDependencyControl {
    Health(HealthFrame),
    SurfaceRecovery(SurfaceRecoveryMessage),
    InputRecovery(InputServiceRecoveryMessage, OwnedUserHandle),
}

#[cfg(feature = "service-dependency-runtime")]
fn read_service_dependency_control(
    transport: u64,
    expected_sender: u64,
) -> ServiceDependencyControl {
    let envelope = read_channel_envelope(transport);
    let received = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()));
    if envelope.sender_pid() != expected_sender
        || envelope.logical_length() != bndr_sm::health::HEALTH_FRAME_SIZE
    {
        if let Some(received) = received {
            close_owned(received);
        }
        fail(FAIL_RUNTIME);
    }
    let data = envelope.data();
    let magic = [data[0], data[1], data[2], data[3]];
    if magic == *b"BSH1" {
        if envelope.kind() != ChannelMessageKind::Bytes || received.is_some() {
            if let Some(received) = received {
                close_owned(received);
            }
            fail(FAIL_RUNTIME);
        }
        let frame = HealthFrame::decode(data).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        ServiceDependencyControl::Health(frame)
    } else if magic == *b"BSR1" {
        if envelope.kind() != ChannelMessageKind::Bytes || received.is_some() {
            if let Some(received) = received {
                close_owned(received);
            }
            fail(FAIL_RUNTIME);
        }
        let message = SurfaceRecoveryMessage::decode(&data[..bndr_ui::SURFACE_RECOVERY_WIRE_SIZE])
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        ServiceDependencyControl::SurfaceRecovery(message)
    } else if magic == *b"BIP1" {
        if envelope.kind() != ChannelMessageKind::Transfer || received.is_none() {
            if let Some(received) = received {
                close_owned(received);
            }
            fail(FAIL_RUNTIME);
        }
        let message = InputServiceRecoveryMessage::decode(
            &data[..bndr_input::INPUT_SERVICE_RECOVERY_WIRE_SIZE],
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        ServiceDependencyControl::InputRecovery(
            message,
            received.unwrap_or_else(|| fail(FAIL_RUNTIME)),
        )
    } else {
        if let Some(received) = received {
            close_owned(received);
        }
        fail(FAIL_RUNTIME)
    }
}

#[cfg(feature = "service-dependency-runtime")]
fn read_service_dependency_health(transport: u64, expected_sender: u64) -> HealthFrame {
    match read_service_dependency_control(transport, expected_sender) {
        ServiceDependencyControl::Health(frame) => frame,
        ServiceDependencyControl::SurfaceRecovery(message) => {
            let _ = message;
            fail(FAIL_RUNTIME)
        }
        ServiceDependencyControl::InputRecovery(_, endpoint) => {
            close_owned(endpoint);
            fail(FAIL_RUNTIME)
        }
    }
}

#[cfg(feature = "service-dependency-runtime")]
fn read_service_dependency_offer(
    transport: u64,
    expected_sender: u64,
    sequences: &mut InputServiceRecoverySequenceTracker,
) -> (InputServiceRecoveryMessage, OwnedUserHandle) {
    match read_service_dependency_control(transport, expected_sender) {
        ServiceDependencyControl::InputRecovery(message, endpoint) => {
            sequences
                .accept(message)
                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            (message, endpoint)
        }
        ServiceDependencyControl::Health(_) | ServiceDependencyControl::SurfaceRecovery(_) => {
            fail(FAIL_RUNTIME)
        }
    }
}

#[cfg(feature = "service-dependency-runtime")]
#[allow(clippy::too_many_arguments)]
fn finish_service_dependency_surface(
    surface: &mut MultiWindowSurface,
    outputs: &mut [OutputBuffer; 2],
    compositor: &mut Compositor<'_>,
    launcher_window: WindowId,
    app_window: WindowId,
    recovery_frame: &mut u32,
    recovery_epoch: &mut u64,
    recovery_boundary: &mut u64,
) -> ! {
    let launcher_ref = input_window_ref(launcher_window);
    let app_ref = input_window_ref(app_window);
    let app_pid = surface
        .app
        .as_ref()
        .map(|app| app.pid)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if surface.session_id != 2
        || surface.input_route.input_session_id != 1
        || surface.input_route.route_epoch != 2
        || surface.input_route.last_physical_sequence != 3
        || surface.input_route.last_event_sequence != 7
        || surface.input_route.route_sequence != 4
        || surface.input_route.focus_sequence != 1
        || surface.input_route.key_sequence != 0
        || surface.input_route.focus != Some(app_ref)
        || surface.input_route.context.is_some()
        || surface.input_route.overlay_visible
        || !surface.input_route.scene_ready
        || surface.input_route.scene_update_active
        || surface.input_route.has_pending()
        || compositor.focus_state().window_id != Some(app_window)
        || compositor.focus_state().generation != 3
        || compositor.captured_window().is_some()
        || *recovery_frame != 1
        || *recovery_epoch != 1
        || *recovery_boundary == 0
        || outputs[0].write_generation != 1
    {
        fail(FAIL_RUNTIME);
    }

    let probe = read_service_dependency_health(surface.control.raw(), surface.init_pid);
    let surface_identity = probe.identity();
    if probe.opcode() != HealthOpcode::Probe
        || probe.sequence() != 1
        || probe.interval_ns() != SERVICE_HEALTH_TIMEOUT_NS
        || surface_identity.kind() != ServiceKind::SurfaceServer
        || surface_identity.generation() != 2
        || surface_identity.generation() != process_generation(surface_identity.pid())
        || process_slot(surface_identity.pid()) == 0
        || surface_identity.pid() == surface.init_pid
        || surface_identity.pid() == surface.input_route.server_pid
        || surface_identity.pid() == surface.launcher_pid
        || surface_identity.pid() == app_pid
    {
        fail(FAIL_RUNTIME);
    }
    let healthy = HealthFrame::healthy(1, surface_identity).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(surface.control.raw(), &healthy.encode());

    let old_input_pid = surface.input_route.server_pid;
    let route_closed = object_wait(
        surface.input_route.endpoint.raw(),
        signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED),
    );
    if route_closed.status != Status::Ok.raw()
        || route_closed.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
        || route_closed.out1 & u64::from(ObjectSignals::READABLE.bits()) != 0
        || route_closed.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }

    let degraded_boundary = *recovery_boundary;
    publish_input_degraded_damage(
        surface,
        outputs,
        compositor,
        app_window,
        5,
        recovery_frame,
        recovery_epoch,
        recovery_boundary,
    );
    if *recovery_frame != 2
        || *recovery_epoch != 2
        || *recovery_boundary <= degraded_boundary
        || outputs[0].write_generation != 2
    {
        fail(FAIL_RUNTIME);
    }

    let mut recovery_sequences = InputServiceRecoverySequenceTracker::new();
    send_input_restart_status(
        surface,
        &mut recovery_sequences,
        1,
        3,
        InputServiceRecoveryPhase::RouteLost,
        InputServiceFocus::App,
    );

    let (offer, replacement_endpoint) = read_service_dependency_offer(
        surface.control.raw(),
        surface.init_pid,
        &mut recovery_sequences,
    );
    let InputServiceRecoveryPayload::RebindOffer {
        old_input_pid: offered_old,
        new_input_pid,
        new_input_session_id,
        physical_sequence_floor,
    } = offer.payload()
    else {
        close_owned(replacement_endpoint);
        fail(FAIL_RUNTIME);
    };
    if offer.sequence() != 1
        || offer.surface_session() != 2
        || offer.route_epoch() != 3
        || offered_old.get() != old_input_pid
        || new_input_pid.get() == old_input_pid
        || new_input_session_id != 2
        || physical_sequence_floor != 3
    {
        close_owned(replacement_endpoint);
        fail(FAIL_RUNTIME);
    }
    let replacement = InputRouteClient::bootstrap_replacement(
        replacement_endpoint,
        surface.init_pid,
        new_input_pid.get(),
        new_input_session_id,
        offer.route_epoch(),
        physical_sequence_floor,
    );
    let retired = core::mem::replace(&mut surface.input_route, replacement);
    close_owned(retired.endpoint);

    let launcher_route =
        input_restart_route_for_window(compositor, surface.launcher_pid, launcher_window);
    let app_route = input_restart_route_for_window(compositor, app_pid, app_window);
    surface
        .input_route
        .resync_snapshot(launcher_route, app_route, app_ref);
    if surface.input_route.last_event_sequence != 7
        || surface.input_route.last_physical_sequence != 3
        || surface.input_route.route_sequence != 4
        || surface.input_route.focus_sequence != 2
        || surface.input_route.key_sequence != 0
        || surface.input_route.focus != Some(app_ref)
        || surface.input_route.context.is_some()
        || surface.input_route.overlay_visible
        || !surface.input_route.scene_ready
        || surface.input_route.scene_update_active
        || surface.input_route.has_pending()
        || launcher_ref == app_ref
    {
        fail(FAIL_RUNTIME);
    }
    send_input_restart_status(
        surface,
        &mut recovery_sequences,
        2,
        3,
        InputServiceRecoveryPhase::ResyncPrepared,
        InputServiceFocus::App,
    );

    let recovered_boundary = *recovery_boundary;
    publish_input_restart_damage(
        surface,
        outputs,
        compositor,
        app_window,
        COLOR_SERVICE_DEPENDENCY_ACTIVE,
        6,
        recovery_frame,
        recovery_epoch,
        recovery_boundary,
    );
    if *recovery_frame != 3
        || *recovery_epoch != 3
        || *recovery_boundary <= recovered_boundary
        || outputs[0].write_generation != 3
    {
        fail(FAIL_RUNTIME);
    }
    send_input_restart_status(
        surface,
        &mut recovery_sequences,
        3,
        3,
        InputServiceRecoveryPhase::Active,
        InputServiceFocus::App,
    );

    #[cfg(feature = "post-recovery-interaction-runtime")]
    finish_post_recovery_interaction(
        surface,
        outputs,
        compositor,
        launcher_window,
        app_window,
        surface_identity,
        recovery_frame,
        recovery_epoch,
        recovery_boundary,
    );

    #[cfg(not(feature = "post-recovery-interaction-runtime"))]
    {
        let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
        let app = surface.app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(surface.input_route.endpoint.raw(), requested);
        items[1] = pack_user_wait_item(surface.control.raw(), requested);
        items[2] = pack_user_wait_item(surface.launcher.raw(), requested);
        items[3] = pack_user_wait_item(app.endpoint.raw(), requested);
        let ready = object_wait_many_array(&items, 4, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw() {
            fail(FAIL_RUNTIME);
        }
        fail(FAIL_RUNTIME)
    }
}

#[cfg(feature = "post-recovery-interaction-runtime")]
#[allow(clippy::too_many_arguments)]
fn finish_post_recovery_interaction(
    surface: &mut MultiWindowSurface,
    outputs: &mut [OutputBuffer; 2],
    compositor: &mut Compositor<'_>,
    launcher_window: WindowId,
    app_window: WindowId,
    surface_identity: bndr_sm::health::ServiceIdentity,
    recovery_frame: &mut u32,
    recovery_epoch: &mut u64,
    recovery_boundary: &mut u64,
) -> ! {
    let app_ref = input_window_ref(app_window);
    let (app_pid, app_endpoint) = surface
        .app
        .as_ref()
        .map(|app| (app.pid, app.endpoint.raw()))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if surface.input_route.input_session_id != 2
        || surface.input_route.route_epoch != 3
        || surface.input_route.last_event_sequence != 7
        || surface.input_route.last_physical_sequence != 3
        || surface.input_route.route_sequence != 4
        || surface.input_route.focus_sequence != 2
        || surface.input_route.focus != Some(app_ref)
        || surface.input_route.context.is_some()
        || surface.input_route.overlay_visible
        || surface.input_route.scene_update_active
        || !surface.input_route.scene_ready
        || surface.input_route.has_pending()
        || compositor.focus_state().window_id != Some(app_window)
        || compositor.focus_state().generation != 3
        || compositor.captured_window().is_some()
        || *recovery_frame != 3
        || *recovery_epoch != 3
        || outputs[0].write_generation != 3
    {
        fail(FAIL_RUNTIME);
    }

    for physical_sequence in 4_u64..=7 {
        let event = surface.input_route.read_event();
        let ServerInputPayload::RoutedPointer(routed) = event.payload() else {
            fail(FAIL_RUNTIME);
        };
        surface
            .input_route
            .accept_physical_sequence(routed.input_sequence);
        if event.sequence()
            != physical_sequence
                .checked_add(4)
                .unwrap_or_else(|| fail(FAIL_RUNTIME))
            || routed.input_sequence != physical_sequence
            || routed.trusted_overlay
        {
            fail(FAIL_RUNTIME);
        }

        if physical_sequence <= 5 {
            if routed.target.is_some()
                || routed.captured
                || (routed.x, routed.y) != (16, 32)
                || routed.pressed != (physical_sequence == 4)
                || compositor.captured_window().is_some()
            {
                fail(FAIL_RUNTIME);
            }
            continue;
        }

        if routed.target != Some(app_ref)
            || !routed.captured
            || (routed.x, routed.y) != (136, 184)
            || routed.pressed != (physical_sequence == 6)
        {
            fail(FAIL_RUNTIME);
        }
        let route = if routed.pressed {
            compositor.pointer_down(80, 120)
        } else {
            compositor.pointer_up(80, 120)
        }
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
        if route.window_id != app_window
            || (route.global_x, route.global_y) != (80, 120)
            || (route.local_x, route.local_y) != (32, 40)
            || route.pressed != routed.pressed
            || !route.captured
            || route.focus_generation != 3
            || (physical_sequence == 6 && compositor.captured_window() != Some(app_window))
            || (physical_sequence == 7 && compositor.captured_window().is_some())
        {
            fail(FAIL_RUNTIME);
        }
        send_pointer_route(
            app_endpoint,
            4,
            9,
            PointerRoute {
                global_x: routed.x,
                global_y: routed.y,
                ..route
            },
        );
    }

    if surface.input_route.last_event_sequence != 11
        || surface.input_route.last_physical_sequence != 7
        || compositor.captured_window().is_some()
        || *recovery_frame != 3
        || *recovery_epoch != 3
        || outputs[0].write_generation != 3
    {
        fail(FAIL_RUNTIME);
    }
    let command = receive_expected_window_command(surface, WindowClient::App);
    let WindowCommandPayload::Present {
        window_id,
        frame_id,
        damage,
        color,
    } = command.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if command.sequence() != 5
        || window_id != app_window
        || frame_id != 3
        || damage != POST_RECOVERY_APP_DAMAGE
        || color != COLOR_POST_RECOVERY_APP
    {
        fail(FAIL_RUNTIME);
    }

    let prior_boundary = *recovery_boundary;
    prepare_visible_output(
        surface,
        &mut outputs[0],
        compositor,
        recovery_epoch,
        recovery_boundary,
    );
    let prior_digest = output_digest(outputs[0].pixels_mut());
    let report = compositor
        .present(app_pid, app_window, damage, color, outputs[0].pixels_mut())
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let changed_pixels = u32::from(damage.width) * u32::from(damage.height);
    if report.content_generation != 7
        || report.global_damage
            != ShellRect::new(
                APP_BOUNDS.x + damage.x,
                APP_BOUNDS.y + damage.y,
                damage.width,
                damage.height,
            )
        || output_digest(outputs[0].pixels_mut()) == prior_digest
    {
        fail(FAIL_RUNTIME);
    }
    validate_present_ledger(
        report.pixels,
        changed_pixels,
        changed_pixels,
        0,
        changed_pixels,
        true,
    );
    publish_visible_output(surface, &mut outputs[0], recovery_frame);
    if *recovery_frame != 4
        || *recovery_epoch != 4
        || *recovery_boundary <= prior_boundary
        || outputs[0].write_generation != 4
    {
        fail(FAIL_RUNTIME);
    }
    send_presented_event(
        surface,
        WindowClient::App,
        command,
        10,
        app_window,
        frame_id,
    );
    read_authenticated_magic(app_endpoint, app_pid, POST_RECOVERY_APP_ACK_MAGIC);
    write_wire(
        surface.control.raw(),
        &POST_RECOVERY_INTERACTION_READY_MAGIC.to_le_bytes(),
    );

    let probe = read_service_dependency_health(surface.control.raw(), surface.init_pid);
    if probe.opcode() != HealthOpcode::Probe
        || probe.sequence() != 2
        || probe.interval_ns() != SERVICE_HEALTH_TIMEOUT_NS
        || probe.identity() != surface_identity
        || surface.input_route.last_event_sequence != 11
        || surface.input_route.last_physical_sequence != 7
        || surface.input_route.has_pending()
        || compositor.focus_state().window_id != Some(app_window)
        || compositor.focus_state().generation != 3
        || compositor.captured_window().is_some()
        || *recovery_frame != 4
        || *recovery_epoch != 4
        || outputs[0].write_generation != 4
    {
        fail(FAIL_RUNTIME);
    }
    let healthy = HealthFrame::healthy(2, surface_identity).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(surface.control.raw(), &healthy.encode());

    #[cfg(feature = "post-recovery-focus-runtime")]
    finish_post_recovery_focus(
        surface,
        outputs,
        compositor,
        launcher_window,
        app_window,
        recovery_frame,
        recovery_epoch,
        recovery_boundary,
    );

    #[cfg(not(feature = "post-recovery-focus-runtime"))]
    {
        let _ = launcher_window;
        wait_post_recovery_resident(surface, app_endpoint)
    }
}

#[cfg(feature = "post-recovery-focus-runtime")]
#[allow(clippy::too_many_arguments)]
fn finish_post_recovery_focus(
    surface: &mut MultiWindowSurface,
    outputs: &mut [OutputBuffer; 2],
    compositor: &mut Compositor<'_>,
    launcher_window: WindowId,
    app_window: WindowId,
    recovery_frame: &mut u32,
    recovery_epoch: &mut u64,
    recovery_boundary: &mut u64,
) -> ! {
    let launcher_ref = input_window_ref(launcher_window);
    let app_ref = input_window_ref(app_window);
    let app_endpoint = surface
        .app
        .as_ref()
        .map(|app| app.endpoint.raw())
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if surface.input_route.input_session_id != 2
        || surface.input_route.route_epoch != 3
        || surface.input_route.last_event_sequence != 11
        || surface.input_route.last_physical_sequence != 7
        || surface.input_route.route_sequence != 4
        || surface.input_route.focus_sequence != 2
        || surface.input_route.focus != Some(app_ref)
        || surface.input_route.context.is_some()
        || surface.input_route.overlay_visible
        || surface.input_route.scene_update_active
        || !surface.input_route.scene_ready
        || surface.input_route.has_pending()
        || compositor.focus_state().window_id != Some(app_window)
        || compositor.focus_state().generation != 3
        || compositor.captured_window().is_some()
        || *recovery_frame != 4
        || *recovery_epoch != 4
        || outputs[0].write_generation != 4
    {
        fail(FAIL_RUNTIME);
    }

    for physical_sequence in 8_u64..=9 {
        wait_post_recovery_focus_input(surface, app_endpoint);
        let event = surface.input_route.read_event();
        let ServerInputPayload::RoutedPointer(routed) = event.payload() else {
            fail(FAIL_RUNTIME);
        };
        surface
            .input_route
            .accept_physical_sequence(routed.input_sequence);
        let expected_event_sequence = if physical_sequence == 8 { 12 } else { 14 };
        if event.sequence() != expected_event_sequence
            || routed.input_sequence != physical_sequence
            || routed.target != Some(launcher_ref)
            || !routed.captured
            || routed.trusted_overlay
            || (routed.x, routed.y) != (80, 96)
            || routed.pressed != (physical_sequence == 8)
        {
            fail(FAIL_RUNTIME);
        }

        let logical_x = routed
            .x
            .checked_sub(PHONE_OFFSET_X)
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        let logical_y = routed
            .y
            .checked_sub(PHONE_OFFSET_Y)
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        let route = if routed.pressed {
            compositor.pointer_down(logical_x, logical_y)
        } else {
            compositor.pointer_up(logical_x, logical_y)
        }
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
        if route.window_id != launcher_window
            || (route.global_x, route.global_y) != (24, 32)
            || (route.local_x, route.local_y) != (24, 32)
            || route.pressed != routed.pressed
            || !route.captured
            || route.focus_generation != 4
            || (physical_sequence == 8 && compositor.captured_window() != Some(launcher_window))
            || (physical_sequence == 9 && compositor.captured_window().is_some())
        {
            fail(FAIL_RUNTIME);
        }
        if physical_sequence == 8 {
            sync_input_focus(surface, compositor);
            if surface.input_route.focus_sequence != 3
                || surface.input_route.focus != Some(launcher_ref)
                || surface.input_route.last_event_sequence != 13
                || surface.input_route.has_pending()
            {
                fail(FAIL_RUNTIME);
            }
            #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
            advance_post_recovery_lifecycle_focus(
                surface,
                1,
                UiClientId::Launcher,
                POST_RECOVERY_LAUNCHER_FOCUS_ACK_MAGIC,
            );
        } else if surface.input_route.focus_sequence != 3
            || surface.input_route.focus != Some(launcher_ref)
        {
            fail(FAIL_RUNTIME);
        }
        send_pointer_route(
            surface.launcher.raw(),
            5,
            10,
            PointerRoute {
                global_x: routed.x,
                global_y: routed.y,
                ..route
            },
        );
    }

    if surface.input_route.last_event_sequence != 14
        || surface.input_route.last_physical_sequence != 9
        || surface.input_route.focus_sequence != 3
        || surface.input_route.focus != Some(launcher_ref)
        || surface.input_route.has_pending()
        || compositor.focus_state().window_id != Some(launcher_window)
        || compositor.focus_state().generation != 4
        || compositor.captured_window().is_some()
        || *recovery_frame != 4
        || *recovery_epoch != 4
        || outputs[0].write_generation != 4
    {
        fail(FAIL_RUNTIME);
    }

    let command = receive_expected_window_command(surface, WindowClient::Launcher);
    let WindowCommandPayload::Present {
        window_id,
        frame_id,
        damage,
        color,
    } = command.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if command.sequence() != 6
        || window_id != launcher_window
        || frame_id != 4
        || damage != POST_RECOVERY_LAUNCHER_DAMAGE
        || color != COLOR_POST_RECOVERY_LAUNCHER
    {
        fail(FAIL_RUNTIME);
    }

    let prior_boundary = *recovery_boundary;
    prepare_visible_output(
        surface,
        &mut outputs[0],
        compositor,
        recovery_epoch,
        recovery_boundary,
    );
    let prior_digest = output_digest(outputs[0].pixels_mut());
    let report = compositor
        .present(
            surface.launcher_pid,
            launcher_window,
            damage,
            color,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let changed_pixels = u32::from(damage.width) * u32::from(damage.height);
    if report.content_generation != 5
        || report.global_damage != damage
        || output_digest(outputs[0].pixels_mut()) == prior_digest
    {
        fail(FAIL_RUNTIME);
    }
    validate_present_ledger(
        report.pixels,
        changed_pixels,
        changed_pixels,
        0,
        changed_pixels,
        true,
    );
    publish_visible_output(surface, &mut outputs[0], recovery_frame);
    if *recovery_frame != 5
        || *recovery_epoch != 5
        || *recovery_boundary <= prior_boundary
        || outputs[0].write_generation != 5
    {
        fail(FAIL_RUNTIME);
    }
    send_presented_event(
        surface,
        WindowClient::Launcher,
        command,
        11,
        launcher_window,
        frame_id,
    );
    read_authenticated_magic(
        surface.launcher.raw(),
        surface.launcher_pid,
        POST_RECOVERY_LAUNCHER_ACK_MAGIC,
    );

    #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
    finish_post_recovery_focus_roundtrip(
        surface,
        outputs,
        compositor,
        launcher_window,
        app_window,
        recovery_frame,
        recovery_epoch,
        recovery_boundary,
    );

    #[cfg(not(feature = "post-recovery-focus-roundtrip-runtime"))]
    wait_post_recovery_resident(surface, app_endpoint)
}

#[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
#[allow(clippy::too_many_arguments)]
fn finish_post_recovery_focus_roundtrip(
    surface: &mut MultiWindowSurface,
    outputs: &mut [OutputBuffer; 2],
    compositor: &mut Compositor<'_>,
    launcher_window: WindowId,
    app_window: WindowId,
    recovery_frame: &mut u32,
    recovery_epoch: &mut u64,
    recovery_boundary: &mut u64,
) -> ! {
    let launcher_ref = input_window_ref(launcher_window);
    let app_ref = input_window_ref(app_window);
    let (app_pid, app_endpoint) = surface
        .app
        .as_ref()
        .map(|app| (app.pid, app.endpoint.raw()))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if surface.session_id != 2
        || surface.lifecycle_focus_generation != 2
        || surface.input_route.input_session_id != 2
        || surface.input_route.route_epoch != 3
        || surface.input_route.last_event_sequence != 14
        || surface.input_route.last_physical_sequence != 9
        || surface.input_route.route_sequence != 4
        || surface.input_route.focus_sequence != 3
        || surface.input_route.key_sequence != 0
        || surface.input_route.focus != Some(launcher_ref)
        || surface.input_route.context.is_some()
        || surface.input_route.overlay_visible
        || surface.input_route.scene_update_active
        || !surface.input_route.scene_ready
        || surface.input_route.has_pending()
        || compositor.focus_state().window_id != Some(launcher_window)
        || compositor.focus_state().generation != 4
        || compositor.captured_window().is_some()
        || *recovery_frame != 5
        || *recovery_epoch != 5
        || *recovery_boundary == 0
        || outputs[0].write_generation != 5
        || outputs[1].write_generation != 0
    {
        fail(FAIL_RUNTIME);
    }

    for physical_sequence in 10_u64..=11 {
        wait_post_recovery_focus_input(surface, app_endpoint);
        let event = surface.input_route.read_event();
        let ServerInputPayload::RoutedPointer(routed) = event.payload() else {
            fail(FAIL_RUNTIME);
        };
        surface
            .input_route
            .accept_physical_sequence(routed.input_sequence);
        let expected_event_sequence = if physical_sequence == 10 { 15 } else { 17 };
        if event.sequence() != expected_event_sequence
            || routed.input_sequence != physical_sequence
            || routed.target != Some(app_ref)
            || !routed.captured
            || routed.trusted_overlay
            || (routed.x, routed.y) != (136, 184)
            || routed.pressed != (physical_sequence == 10)
        {
            fail(FAIL_RUNTIME);
        }

        let logical_x = routed
            .x
            .checked_sub(PHONE_OFFSET_X)
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        let logical_y = routed
            .y
            .checked_sub(PHONE_OFFSET_Y)
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        let route = if routed.pressed {
            compositor.pointer_down(logical_x, logical_y)
        } else {
            compositor.pointer_up(logical_x, logical_y)
        }
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
        if route.window_id != app_window
            || (route.global_x, route.global_y) != (80, 120)
            || (route.local_x, route.local_y) != (32, 40)
            || route.pressed != routed.pressed
            || !route.captured
            || route.focus_generation != 5
            || (physical_sequence == 10 && compositor.captured_window() != Some(app_window))
            || (physical_sequence == 11 && compositor.captured_window().is_some())
        {
            fail(FAIL_RUNTIME);
        }
        if physical_sequence == 10 {
            sync_input_focus(surface, compositor);
            if surface.input_route.focus_sequence != 4
                || surface.input_route.focus != Some(app_ref)
                || surface.input_route.last_event_sequence != 16
                || surface.input_route.has_pending()
            {
                fail(FAIL_RUNTIME);
            }
            #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
            advance_post_recovery_lifecycle_focus(
                surface,
                2,
                UiClientId::App,
                POST_RECOVERY_APP_FOCUS_ACK_MAGIC,
            );
        } else if surface.input_route.focus_sequence != 4
            || surface.input_route.focus != Some(app_ref)
        {
            fail(FAIL_RUNTIME);
        }
        send_pointer_route(
            app_endpoint,
            5,
            11,
            PointerRoute {
                global_x: routed.x,
                global_y: routed.y,
                ..route
            },
        );
    }

    if surface.input_route.last_event_sequence != 17
        || surface.input_route.last_physical_sequence != 11
        || surface.input_route.route_sequence != 4
        || surface.input_route.focus_sequence != 4
        || surface.input_route.key_sequence != 0
        || surface.input_route.focus != Some(app_ref)
        || surface.input_route.context.is_some()
        || surface.input_route.overlay_visible
        || surface.input_route.scene_update_active
        || !surface.input_route.scene_ready
        || surface.input_route.has_pending()
        || compositor.focus_state().window_id != Some(app_window)
        || compositor.focus_state().generation != 5
        || compositor.captured_window().is_some()
        || *recovery_frame != 5
        || *recovery_epoch != 5
        || outputs[0].write_generation != 5
        || outputs[1].write_generation != 0
    {
        fail(FAIL_RUNTIME);
    }
    #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
    if surface.lifecycle_focus_generation != 3 {
        fail(FAIL_RUNTIME);
    }

    let command = receive_expected_window_command(surface, WindowClient::App);
    let WindowCommandPayload::Present {
        window_id,
        frame_id,
        damage,
        color,
    } = command.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if command.sequence() != 6
        || window_id != app_window
        || frame_id != 4
        || damage != POST_RECOVERY_APP_DAMAGE
        || color != COLOR_APP_DAMAGE
    {
        fail(FAIL_RUNTIME);
    }

    let prior_boundary = *recovery_boundary;
    prepare_visible_output(
        surface,
        &mut outputs[0],
        compositor,
        recovery_epoch,
        recovery_boundary,
    );
    let prior_digest = output_digest(outputs[0].pixels_mut());
    let report = compositor
        .present(app_pid, app_window, damage, color, outputs[0].pixels_mut())
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let changed_pixels = u32::from(damage.width) * u32::from(damage.height);
    if changed_pixels != 1_280
        || report.content_generation != 8
        || report.global_damage
            != ShellRect::new(
                APP_BOUNDS.x + damage.x,
                APP_BOUNDS.y + damage.y,
                damage.width,
                damage.height,
            )
        || output_digest(outputs[0].pixels_mut()) == prior_digest
    {
        fail(FAIL_RUNTIME);
    }
    validate_present_ledger(
        report.pixels,
        changed_pixels,
        changed_pixels,
        0,
        changed_pixels,
        true,
    );
    publish_visible_output(surface, &mut outputs[0], recovery_frame);
    if *recovery_frame != 6
        || *recovery_epoch != 6
        || *recovery_boundary <= prior_boundary
        || outputs[0].write_generation != 6
        || outputs[1].write_generation != 0
    {
        fail(FAIL_RUNTIME);
    }
    send_presented_event(
        surface,
        WindowClient::App,
        command,
        12,
        app_window,
        frame_id,
    );
    read_authenticated_magic(app_endpoint, app_pid, POST_RECOVERY_ROUNDTRIP_APP_ACK_MAGIC);

    wait_post_recovery_resident(surface, app_endpoint)
}

#[cfg(feature = "post-recovery-focus-runtime")]
fn wait_post_recovery_focus_input(surface: &MultiWindowSurface, app_endpoint: u64) {
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    items[0] = pack_user_wait_item(surface.input_route.endpoint.raw(), requested);
    items[1] = pack_user_wait_item(surface.control.raw(), requested);
    items[2] = pack_user_wait_item(surface.launcher.raw(), requested);
    items[3] = pack_user_wait_item(app_endpoint, requested);
    let ready = object_wait_many_array(&items, 4, OBJECT_WAIT_TIMEOUT_INFINITE);
    if ready.status != Status::Ok.raw()
        || ready.out1 != 0
        || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
        || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(feature = "post-recovery-interaction-runtime")]
fn wait_post_recovery_resident(surface: &MultiWindowSurface, app_endpoint: u64) -> ! {
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    items[0] = pack_user_wait_item(surface.input_route.endpoint.raw(), requested);
    items[1] = pack_user_wait_item(surface.control.raw(), requested);
    items[2] = pack_user_wait_item(surface.launcher.raw(), requested);
    items[3] = pack_user_wait_item(app_endpoint, requested);
    let ready = object_wait_many_array(&items, 4, OBJECT_WAIT_TIMEOUT_INFINITE);
    let _ = ready;
    fail(FAIL_RUNTIME)
}

#[cfg(any(
    feature = "service-dependency-runtime",
    all(
        feature = "input-server-surface-restart-runtime",
        not(feature = "input-server-restart-runtime")
    )
))]
fn read_surface_recovery_message(transport: u64, expected_sender: u64) -> SurfaceRecoveryMessage {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != bndr_ui::SURFACE_RECOVERY_WIRE_SIZE
        || envelope.sender_pid() != expected_sender
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_RUNTIME);
    }
    SurfaceRecoveryMessage::decode(&envelope.data()[..bndr_ui::SURFACE_RECOVERY_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
}

#[cfg(any(
    feature = "service-dependency-runtime",
    all(
        feature = "input-server-surface-restart-runtime",
        not(feature = "input-server-restart-runtime")
    )
))]
fn read_surface_rebind_ack(
    transport: u64,
    offer: SurfaceRecoveryMessage,
    expected_window: WindowId,
) -> u64 {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != bndr_ui::SURFACE_RECOVERY_WIRE_SIZE
        || envelope.sender_pid() == 0
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_RUNTIME);
    }
    let ack =
        SurfaceRecoveryMessage::decode(&envelope.data()[..bndr_ui::SURFACE_RECOVERY_WIRE_SIZE])
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let SurfaceRecoveryPayload::ClientRebindAck {
        replacement_window, ..
    } = ack.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if ack.sender_sequence() != 1
        || !ack.acknowledges(offer)
        || replacement_window != expected_window
    {
        fail(FAIL_RUNTIME);
    }
    envelope.sender_pid()
}

#[cfg(any(
    feature = "service-dependency-runtime",
    all(
        feature = "input-server-surface-restart-runtime",
        not(feature = "input-server-restart-runtime")
    )
))]
#[allow(clippy::too_many_arguments)]
fn rebind_surface_client(
    lifecycle: u64,
    init_pid: u64,
    old_ui: OwnedUserHandle,
    _ui_tracker: &mut UiServerEventTracker,
    old_surface_pid: u64,
    endpoint_kind: UiBootstrapEndpointKind,
    expected_role: RecoveryClientRole,
    expected_cancel: RecoveryCancelKind,
    expected_window: WindowId,
    expected_cancel_after: u64,
    active_contact: Option<&mut Option<u64>>,
) -> (OwnedUserHandle, u64) {
    close_owned(old_ui);
    let (sender_pid, rebound) = read_bootstrap_transfer(lifecycle, endpoint_kind);
    if sender_pid != init_pid {
        close_owned(rebound);
        fail(FAIL_RUNTIME);
    }

    let envelope = read_channel_envelope(rebound.raw());
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != bndr_ui::SURFACE_RECOVERY_WIRE_SIZE
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() == 0
        || envelope.sender_pid() == init_pid
        || envelope.sender_pid() == old_surface_pid
    {
        fail(FAIL_RUNTIME);
    }
    let offer =
        SurfaceRecoveryMessage::decode(&envelope.data()[..bndr_ui::SURFACE_RECOVERY_WIRE_SIZE])
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let SurfaceRecoveryPayload::ClientRebindOffer {
        role,
        cancel_kind,
        checkpoint,
        old_window,
        physical_floor,
        cancel_after,
    } = offer.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if offer.sender_sequence() != 1
        || offer.surface_session() != 2
        || offer.route_epoch() != 2
        || role != expected_role
        || cancel_kind != expected_cancel
        || checkpoint != SurfaceRecoveryCheckpoint::MultiWindowInteractive
        || old_window != expected_window
        || physical_floor != 2
        || cancel_after != expected_cancel_after
    {
        fail(FAIL_RUNTIME);
    }

    match active_contact {
        None if expected_role == RecoveryClientRole::Launcher
            && expected_cancel == RecoveryCancelKind::None
            && expected_cancel_after == 0 => {}
        Some(active_contact)
            if expected_role == RecoveryClientRole::App
                && expected_cancel == RecoveryCancelKind::ClientPointer
                && *active_contact == Some(expected_cancel_after) =>
        {
            *active_contact = None;
        }
        _ => fail(FAIL_RUNTIME),
    }

    #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
    {
        if _ui_tracker.session_id() != Some(1)
            || _ui_tracker.last_input_sequence().is_some()
            || _ui_tracker.last_frame_id().is_some()
            || _ui_tracker.last_commit().is_some()
            || _ui_tracker.last_focus_generation() != Some(2)
            || _ui_tracker.active_client() != Some(UiClientId::App)
            || _ui_tracker.active_app() != Some(APP)
            || _ui_tracker.outstanding_frame_id().is_some()
            || _ui_tracker.degraded_reason().is_some()
        {
            fail(FAIL_RUNTIME);
        }
        *_ui_tracker = UiServerEventTracker::new();
    }

    let acknowledgment = SurfaceRecoveryMessage::client_rebind_ack(1, offer, expected_window)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(rebound.raw(), &acknowledgment.encode());
    (rebound, envelope.sender_pid())
}

#[cfg(any(
    feature = "service-dependency-runtime",
    all(
        feature = "input-server-surface-restart-runtime",
        not(feature = "input-server-restart-runtime")
    )
))]
#[allow(clippy::too_many_arguments)]
fn finish_launcher_lifecycle_before_rebind(
    lifecycle: u64,
    init_pid: u64,
    lifecycle_sequence: &mut bndr_ui::AppLifecycleSequenceTracker,
    request_sequence: u64,
    state_sequence: &mut u64,
    transaction_id: &mut u64,
    intermediate_seen: &mut bool,
    identity: Option<AppInstanceIdentity>,
) {
    let final_transaction = TRANSACTION_COUNT
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if request_sequence != TRANSACTION_COUNT
        || (*transaction_id != TRANSACTION_COUNT && *transaction_id != final_transaction)
        || identity.is_none()
    {
        fail(FAIL_RUNTIME);
    }

    while *transaction_id == TRANSACTION_COUNT {
        let (message, transfer) = read_lifecycle_message(lifecycle, init_pid, false);
        if transfer.is_some()
            || message.sender_sequence() != next_sequence(state_sequence)
            || message.transaction_id() != *transaction_id
            || lifecycle_sequence.accept(message).is_err()
        {
            fail(FAIL_RUNTIME);
        }
        let AppLifecyclePayload::StateChanged {
            app,
            state,
            reason,
            identity: state_identity,
        } = message.payload()
        else {
            fail(FAIL_RUNTIME);
        };
        let state_identity = state_identity.unwrap_or_else(|| fail(FAIL_RUNTIME));
        let action =
            lifecycle_action_for_transaction(*transaction_id).unwrap_or_else(|| fail(FAIL_RUNTIME));
        if app != APP || action != AppLifecycleAction::Activate {
            fail(FAIL_RUNTIME);
        }
        if !*intermediate_seen {
            if state != intermediate_state_for(action)
                || reason != AppLifecycleReason::Requested
                || identity != Some(state_identity)
            {
                fail(FAIL_RUNTIME);
            }
            *intermediate_seen = true;
        } else {
            if state != completed_state_for(action)
                || reason != AppLifecycleReason::Completed
                || identity != Some(state_identity)
            {
                fail(FAIL_RUNTIME);
            }
            *intermediate_seen = false;
            *transaction_id = transaction_id
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_RUNTIME));
        }
    }

    let expected_state_sequence = TRANSACTION_COUNT
        .checked_mul(2)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if *transaction_id != final_transaction
        || *intermediate_seen
        || *state_sequence != expected_state_sequence
    {
        fail(FAIL_RUNTIME);
    }
}

pub(super) fn surface_runtime(startup: u64) -> ! {
    let acquired = syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0);
    if acquired.status != Status::Ok.raw() || acquired.out1 == 0 || acquired.out2 == 0 {
        #[cfg(feature = "input-server-runtime")]
        fail(FAIL_INPUT_SURFACE_ACQUIRE);
        #[cfg(not(feature = "input-server-runtime"))]
        fail(FAIL_RUNTIME);
    }
    let capability = OwnedUserHandle::new(acquired.out1).unwrap_or_else(|| fail(FAIL_RUNTIME));
    #[cfg(feature = "input-server-runtime")]
    let (init_pid, control) = read_bootstrap_transfer_with_failure(
        startup,
        UiBootstrapEndpointKind::SurfaceSupervisor,
        FAIL_INPUT_SURFACE_SUPERVISOR_BOOTSTRAP,
    );
    #[cfg(feature = "unified-product-runtime")]
    super::super::product_runtime::arm(control.raw(), ShutdownServiceNode::SurfaceServer, init_pid);
    #[cfg(not(feature = "input-server-runtime"))]
    let (init_pid, control) =
        read_bootstrap_transfer(startup, UiBootstrapEndpointKind::SurfaceSupervisor);
    #[cfg(feature = "input-server-runtime")]
    let input_route = {
        let endpoint = read_input_route_bootstrap(startup, init_pid);
        InputRouteClient::bootstrap(endpoint, init_pid)
    };
    #[cfg(any(
        feature = "service-dependency-runtime",
        all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime")
        )
    ))]
    if acquired.out2 > 1 {
        surface_recovery_runtime(
            startup,
            init_pid,
            control,
            capability,
            input_route,
            acquired.out2,
        )
    }
    let launcher = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let outputs = &mut create_output_swapchain();
    let mut compositor: Compositor<'static> = Compositor::new();
    let mut surface = MultiWindowSurface {
        launcher,
        control,
        capability,
        init_pid,
        session_id: acquired.out2,
        supervisor: UiSupervisorTracker::new(),
        app: None,
        launcher_pid: 0,
        supervisor_command_sequence: 0,
        supervisor_response_sequence: 0,
        lifecycle_focus_generation: 1,
        #[cfg(feature = "input-server-runtime")]
        input_route,
    };
    let mut launcher_commands = WindowCommandTracker::new();
    let mut app_commands = WindowCommandTracker::new();
    let mut scene_frame = 0_u32;
    let mut global_frame = 0_u32;
    let mut last_epoch = 0_u64;
    let mut last_boundary = 0_u64;

    send_ui_event(
        surface.launcher.raw(),
        UiServerEvent::ready(surface.session_id).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
    );
    publish_legacy_focus(&surface, UiClientId::Launcher);

    let launcher_create = receive_expected_window_command(&mut surface, WindowClient::Launcher);
    let WindowCommandPayload::Create {
        bounds: launcher_bounds,
    } = launcher_create.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if launcher_create.sequence() != 1
        || launcher_bounds != LAUNCHER_BOUNDS
        || launcher_commands.accept(launcher_create).is_err()
        || surface.launcher_pid == 0
    {
        fail(FAIL_RUNTIME);
    }
    sync_output(&compositor, &mut outputs[0]);
    let launcher_layer = &mut layer_mut(0)[..SURFACE_PIXEL_COUNT];
    let launcher_report = compositor
        .create(
            surface.launcher_pid,
            launcher_bounds,
            bndr_compositor::BACKGROUND_COLOR,
            launcher_layer,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if launcher_report.window_id.slot() != 0
        || launcher_report.window_id.generation() != 1
        || launcher_report.content_generation != 1
        || launcher_report.pixels.scene_changed
    {
        fail(FAIL_RUNTIME);
    }
    let launcher_window = launcher_report.window_id;
    send_created_event(
        &surface,
        WindowClient::Launcher,
        launcher_create,
        next_scene_frame(&mut scene_frame),
        launcher_window,
        launcher_bounds,
    );

    let launcher_initial = receive_expected_window_command(&mut surface, WindowClient::Launcher);
    let WindowCommandPayload::Present {
        window_id,
        frame_id,
        damage,
        color,
    } = launcher_initial.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if launcher_initial.sequence() != 2
        || window_id != launcher_window
        || frame_id != 1
        || damage != LAUNCHER_BOUNDS
        || color != COLOR_LAUNCHER_BASE
        || launcher_commands.accept(launcher_initial).is_err()
    {
        fail(FAIL_RUNTIME);
    }
    prepare_visible_output(
        &surface,
        &mut outputs[0],
        &compositor,
        &mut last_epoch,
        &mut last_boundary,
    );
    let report = compositor
        .present(
            surface.launcher_pid,
            launcher_window,
            damage,
            color,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if report.content_generation != 2 || report.global_damage != LAUNCHER_BOUNDS {
        fail(FAIL_RUNTIME);
    }
    validate_present_ledger(report.pixels, 76_544, 76_544, 0, 76_544, true);
    publish_visible_output(&surface, &mut outputs[0], &mut global_frame);
    send_presented_event(
        &surface,
        WindowClient::Launcher,
        launcher_initial,
        next_scene_frame(&mut scene_frame),
        launcher_window,
        frame_id,
    );

    let app_create = receive_expected_window_command(&mut surface, WindowClient::App);
    let WindowCommandPayload::Create { bounds: app_bounds } = app_create.payload() else {
        fail(FAIL_RUNTIME);
    };
    let app_pid = surface
        .app
        .as_ref()
        .map(|app| app.pid)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if app_create.sequence() != 1
        || app_bounds != APP_BOUNDS
        || app_commands.accept(app_create).is_err()
    {
        fail(FAIL_RUNTIME);
    }
    sync_output(&compositor, &mut outputs[1]);
    let app_layer_area = usize::from(APP_BOUNDS.width) * usize::from(APP_BOUNDS.height);
    let app_layer = &mut layer_mut(1)[..app_layer_area];
    let app_report = compositor
        .create(
            app_pid,
            app_bounds,
            bndr_compositor::BACKGROUND_COLOR,
            app_layer,
            outputs[1].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if app_report.window_id.slot() != 1
        || app_report.window_id.generation() != 1
        || app_report.content_generation != 1
        || !app_report.pixels.scene_changed
    {
        fail(FAIL_RUNTIME);
    }
    let app_window = app_report.window_id;
    send_created_event(
        &surface,
        WindowClient::App,
        app_create,
        next_scene_frame(&mut scene_frame),
        app_window,
        app_bounds,
    );

    let app_initial = receive_expected_window_command(&mut surface, WindowClient::App);
    let WindowCommandPayload::Present {
        window_id,
        frame_id,
        damage,
        color,
    } = app_initial.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if app_initial.sequence() != 2
        || window_id != app_window
        || frame_id != 1
        || damage != ShellRect::new(0, 0, APP_BOUNDS.width, APP_BOUNDS.height)
        || color != COLOR_APP_BASE
        || app_commands.accept(app_initial).is_err()
    {
        fail(FAIL_RUNTIME);
    }
    prepare_visible_output(
        &surface,
        &mut outputs[1],
        &compositor,
        &mut last_epoch,
        &mut last_boundary,
    );
    let report = compositor
        .present(app_pid, app_window, damage, color, outputs[1].pixels_mut())
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if report.content_generation != 2 || report.global_damage != APP_BOUNDS {
        fail(FAIL_RUNTIME);
    }
    validate_present_ledger(report.pixels, 17_920, 17_920, 0, 17_920, true);
    publish_visible_output(&surface, &mut outputs[1], &mut global_frame);
    send_presented_event(
        &surface,
        WindowClient::App,
        app_initial,
        next_scene_frame(&mut scene_frame),
        app_window,
        frame_id,
    );

    #[cfg(feature = "input-server-runtime")]
    {
        let launcher_owner = surface.launcher_pid;
        publish_input_routes_incremental(
            &mut surface,
            &compositor,
            Some((launcher_window, launcher_owner)),
            Some((app_window, app_pid)),
        );
    }

    // App activation is acknowledged only after the App has consumed this
    // first visible presentation.  Complete that lifecycle handoff before
    // publishing the first interactive phase, so compositor focus and the
    // legacy lifecycle focus cannot diverge.
    while surface.supervisor_command_sequence < TRANSACTION_COUNT {
        let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
        let ready = object_wait(surface.control.raw(), requested);
        if ready.status != Status::Ok.raw()
            || ready.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
            || ready.out1 & u64::from(ObjectSignals::READABLE.bits()) == 0
            || ready.out2 != 0
        {
            fail(FAIL_RUNTIME);
        }
        dispatch_multi_window_supervisor(&mut surface);
    }

    let launcher_hidden = receive_expected_window_command(&mut surface, WindowClient::Launcher);
    let WindowCommandPayload::Present {
        window_id,
        frame_id,
        damage,
        color,
    } = launcher_hidden.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if launcher_hidden.sequence() != 3
        || window_id != launcher_window
        || frame_id != 2
        || damage != LAUNCHER_HIDDEN_DAMAGE
        || color != COLOR_LAUNCHER_HIDDEN
        || launcher_commands.accept(launcher_hidden).is_err()
    {
        fail(FAIL_RUNTIME);
    }
    sync_output(&compositor, &mut outputs[0]);
    let digest_before_hidden = output_digest(outputs[0].pixels_mut());
    let report = compositor
        .present(
            surface.launcher_pid,
            launcher_window,
            damage,
            color,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    validate_present_ledger(report.pixels, 1_024, 0, 1_024, 1_024, false);
    if report.content_generation != 3
        || report.global_damage != LAUNCHER_HIDDEN_DAMAGE
        || output_digest(outputs[0].pixels_mut()) != digest_before_hidden
        || global_frame != 2
    {
        fail(FAIL_RUNTIME);
    }
    send_presented_event(
        &surface,
        WindowClient::Launcher,
        launcher_hidden,
        next_scene_frame(&mut scene_frame),
        launcher_window,
        frame_id,
    );

    // Command four is the Launcher's causal acknowledgement that it consumed
    // the hidden scene-five Presented event.  Read and validate it before
    // publishing App input on a different channel; otherwise the App may run
    // first and make the read-observed kernel transcript depend on scheduling.
    // The actual partial present remains below, after the phase-one routes.
    let launcher_partial = receive_expected_window_command(&mut surface, WindowClient::Launcher);
    let WindowCommandPayload::Present {
        window_id,
        frame_id,
        damage,
        color,
    } = launcher_partial.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if launcher_partial.sequence() != 4
        || window_id != launcher_window
        || frame_id != 3
        || damage != LAUNCHER_PARTIAL_DAMAGE
        || color != COLOR_LAUNCHER_PARTIAL
        || launcher_commands.accept(launcher_partial).is_err()
    {
        fail(FAIL_RUNTIME);
    }

    #[cfg(any(
        feature = "service-dependency-runtime",
        feature = "input-server-restart-runtime",
        all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime")
        )
    ))]
    replay_m41_pointer_routes(
        &mut surface,
        &mut compositor,
        launcher_window,
        app_window,
        3,
        2,
        scene_frame,
        3,
    );
    #[cfg(not(any(
        feature = "service-dependency-runtime",
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime"
    )))]
    receive_pointer_routes(
        &mut surface,
        &mut compositor,
        launcher_window,
        app_window,
        3,
        2,
        scene_frame,
        3,
    );
    if compositor.focus_state().window_id != Some(app_window)
        || compositor.focus_state().generation != 1
        || compositor.captured_window().is_some()
    {
        fail(FAIL_RUNTIME);
    }

    prepare_visible_output(
        &surface,
        &mut outputs[0],
        &compositor,
        &mut last_epoch,
        &mut last_boundary,
    );
    let report = compositor
        .present(
            surface.launcher_pid,
            launcher_window,
            damage,
            color,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    validate_present_ledger(report.pixels, 4_096, 1_792, 2_304, 4_096, true);
    if report.content_generation != 4 {
        fail(FAIL_RUNTIME);
    }
    publish_visible_output(&surface, &mut outputs[0], &mut global_frame);
    send_presented_event(
        &surface,
        WindowClient::Launcher,
        launcher_partial,
        next_scene_frame(&mut scene_frame),
        launcher_window,
        frame_id,
    );

    let app_damage = receive_expected_window_command(&mut surface, WindowClient::App);
    let WindowCommandPayload::Present {
        window_id,
        frame_id,
        damage,
        color,
    } = app_damage.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if app_damage.sequence() != 3
        || window_id != app_window
        || frame_id != 2
        || damage != APP_DAMAGE
        || color != COLOR_APP_DAMAGE
        || app_commands.accept(app_damage).is_err()
    {
        fail(FAIL_RUNTIME);
    }
    prepare_visible_output(
        &surface,
        &mut outputs[1],
        &compositor,
        &mut last_epoch,
        &mut last_boundary,
    );
    let report = compositor
        .present(app_pid, app_window, damage, color, outputs[1].pixels_mut())
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    validate_present_ledger(report.pixels, 1_280, 1_280, 0, 1_280, true);
    if report.content_generation != 3 {
        fail(FAIL_RUNTIME);
    }
    publish_visible_output(&surface, &mut outputs[1], &mut global_frame);
    send_presented_event(
        &surface,
        WindowClient::App,
        app_damage,
        next_scene_frame(&mut scene_frame),
        app_window,
        frame_id,
    );

    let launcher_raise = receive_expected_window_command(&mut surface, WindowClient::Launcher);
    let WindowCommandPayload::Raise { window_id } = launcher_raise.payload() else {
        fail(FAIL_RUNTIME);
    };
    if launcher_raise.sequence() != 5
        || window_id != launcher_window
        || launcher_commands.accept(launcher_raise).is_err()
    {
        fail(FAIL_RUNTIME);
    }
    prepare_visible_output(
        &surface,
        &mut outputs[0],
        &compositor,
        &mut last_epoch,
        &mut last_boundary,
    );
    let ledger = compositor
        .raise(
            surface.launcher_pid,
            launcher_window,
            outputs[0].pixels_mut(),
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    validate_present_ledger(ledger, 0, 0, 0, 76_544, true);
    publish_visible_output(&surface, &mut outputs[0], &mut global_frame);
    send_raised_event(
        &surface,
        WindowClient::Launcher,
        launcher_raise,
        next_scene_frame(&mut scene_frame),
        launcher_window,
    );

    #[cfg(feature = "input-server-runtime")]
    {
        let launcher_owner = surface.launcher_pid;
        publish_input_routes_incremental(
            &mut surface,
            &compositor,
            Some((launcher_window, launcher_owner)),
            Some((app_window, app_pid)),
        );
    }

    #[cfg(any(
        feature = "service-dependency-runtime",
        feature = "input-server-restart-runtime",
        all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime")
        )
    ))]
    replay_m41_pointer_routes(
        &mut surface,
        &mut compositor,
        launcher_window,
        app_window,
        5,
        3,
        scene_frame,
        2,
    );
    #[cfg(not(any(
        feature = "service-dependency-runtime",
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime"
    )))]
    receive_pointer_routes(
        &mut surface,
        &mut compositor,
        launcher_window,
        app_window,
        5,
        3,
        scene_frame,
        2,
    );
    if compositor.focus_state().window_id != Some(launcher_window)
        || compositor.focus_state().generation != 2
        || compositor.captured_window().is_some()
    {
        fail(FAIL_RUNTIME);
    }

    // The first Launcher route is the cross-channel boundary for phase two.
    // Its authenticated acknowledgement prevents App command four from being
    // observed before Launcher has consumed the preceding Raised event and
    // established Launcher capture.  The second route may remain queued: the
    // kernel transcript deliberately permits that release to lag App raise.
    read_authenticated_magic(
        surface.launcher.raw(),
        surface.launcher_pid,
        PHASE_TWO_ROUTE_ACK_MAGIC,
    );

    let app_raise = receive_expected_window_command(&mut surface, WindowClient::App);
    let WindowCommandPayload::Raise { window_id } = app_raise.payload() else {
        fail(FAIL_RUNTIME);
    };
    if app_raise.sequence() != 4
        || window_id != app_window
        || app_commands.accept(app_raise).is_err()
    {
        fail(FAIL_RUNTIME);
    }
    prepare_visible_output(
        &surface,
        &mut outputs[1],
        &compositor,
        &mut last_epoch,
        &mut last_boundary,
    );
    let ledger = compositor
        .raise(app_pid, app_window, outputs[1].pixels_mut())
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    validate_present_ledger(ledger, 0, 0, 0, 17_920, true);
    publish_visible_output(&surface, &mut outputs[1], &mut global_frame);
    send_raised_event(
        &surface,
        WindowClient::App,
        app_raise,
        next_scene_frame(&mut scene_frame),
        app_window,
    );

    #[cfg(feature = "input-server-runtime")]
    {
        let launcher_owner = surface.launcher_pid;
        publish_input_routes_incremental(
            &mut surface,
            &compositor,
            Some((launcher_window, launcher_owner)),
            Some((app_window, app_pid)),
        );
    }

    while surface.supervisor_command_sequence < TRANSACTION_COUNT {
        let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
        let ready = object_wait(surface.control.raw(), requested);
        if ready.status != Status::Ok.raw()
            || ready.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
            || ready.out1 & u64::from(ObjectSignals::READABLE.bits()) == 0
            || ready.out2 != 0
        {
            fail(FAIL_RUNTIME);
        }
        dispatch_multi_window_supervisor(&mut surface);
    }
    if global_frame != 6
        || scene_frame != 9
        || compositor.window_count() != 2
        || compositor.z_order() != [Some(launcher_window), Some(app_window)]
        || outputs[0].write_generation != 3
        || outputs[1].write_generation != 3
        || last_epoch != 6
        || last_boundary == 0
    {
        fail(FAIL_RUNTIME);
    }
    #[cfg(any(
        feature = "service-dependency-runtime",
        all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime")
        )
    ))]
    wait_for_surface_restart_trigger(
        &mut surface,
        outputs,
        &mut compositor,
        app_window,
        &mut global_frame,
        &mut last_epoch,
        &mut last_boundary,
    );
    #[cfg(all(
        feature = "input-server-restart-runtime",
        not(feature = "service-dependency-runtime")
    ))]
    run_input_server_restart(
        &mut surface,
        outputs,
        &mut compositor,
        launcher_window,
        app_window,
        &mut global_frame,
        &mut last_epoch,
        &mut last_boundary,
    );
    #[cfg(feature = "persistent-window-runtime")]
    #[cfg(not(any(
        feature = "service-dependency-runtime",
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime"
    )))]
    persistent_window_session(
        &mut surface,
        outputs,
        &mut compositor,
        launcher_commands,
        app_commands,
        launcher_window,
        app_window,
        scene_frame,
        global_frame,
        last_epoch,
        last_boundary,
    );

    #[cfg(not(feature = "persistent-window-runtime"))]
    {
        write_wire(surface.control.raw(), &READY_MAGIC.to_le_bytes());
        // The completed compositor remains a blocked resident service.  This
        // preserves the same four-source wait topology as the lifecycle runtime
        // while allowing harmless trailing pointer motion to drain without a
        // userspace busy loop.
        let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
        loop {
            let app_endpoint = surface
                .app
                .as_ref()
                .map(|app| app.endpoint.raw())
                .unwrap_or_else(|| fail(FAIL_RUNTIME));
            let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
            items[0] = pack_user_wait_item(surface.capability.raw(), requested);
            items[1] = pack_user_wait_item(surface.control.raw(), requested);
            items[2] = pack_user_wait_item(surface.launcher.raw(), requested);
            items[3] = pack_user_wait_item(app_endpoint, requested);
            let ready = object_wait_many_array(&items, 4, OBJECT_WAIT_TIMEOUT_INFINITE);
            if ready.status != Status::Ok.raw()
                || ready.out1 > 3
                || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
                || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
            {
                fail(FAIL_RUNTIME);
            }
            if ready.out1 != 0
                || route_pointer_event(
                    &mut surface,
                    &mut compositor,
                    launcher_window,
                    app_window,
                    5,
                    4,
                    scene_frame,
                )
            {
                fail(FAIL_RUNTIME);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum ClientWireEvent {
    Legacy(UiServerEvent),
    Window(WindowEvent),
    #[cfg(feature = "text-input-runtime")]
    Text(TextInputEvent),
}

fn send_window_command(transport: u64, command: WindowCommand) {
    write_wire(transport, &command.encode());
}

fn read_client_wire_event(transport: u64, expected_sender: &mut Option<u64>) -> ClientWireEvent {
    let envelope = read_channel_envelope(transport);
    decode_client_wire_event(envelope, expected_sender)
}

fn decode_client_wire_event(
    envelope: ChannelReadEnvelope,
    expected_sender: &mut Option<u64>,
) -> ClientWireEvent {
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != WINDOW_EVENT_WIRE_SIZE
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() == 0
        || expected_sender.is_some_and(|pid| pid != envelope.sender_pid())
    {
        fail(FAIL_RUNTIME);
    }
    *expected_sender = Some(envelope.sender_pid());
    let wire = &envelope.data()[..WINDOW_EVENT_WIRE_SIZE];
    if let Ok(event) = UiServerEvent::decode(wire) {
        ClientWireEvent::Legacy(event)
    } else if let Ok(event) = WindowEvent::decode(wire) {
        ClientWireEvent::Window(event)
    } else {
        #[cfg(feature = "text-input-runtime")]
        {
            ClientWireEvent::Text(
                TextInputEvent::decode(&wire[..TEXT_INPUT_EVENT_WIRE_SIZE])
                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
            )
        }
        #[cfg(not(feature = "text-input-runtime"))]
        fail(FAIL_RUNTIME)
    }
}

#[cfg(any(
    feature = "service-dependency-runtime",
    all(
        feature = "input-server-surface-restart-runtime",
        not(feature = "input-server-restart-runtime")
    )
))]
fn accept_app_recovery_contact(
    transport: u64,
    envelope: ChannelReadEnvelope,
    expected_surface_pid: u64,
    expected_window: WindowId,
    active_contact: &mut Option<u64>,
) {
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != bndr_ui::SURFACE_RECOVERY_WIRE_SIZE
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() != expected_surface_pid
        || expected_surface_pid == 0
        || active_contact.is_some()
    {
        fail(FAIL_RUNTIME);
    }
    let contact =
        SurfaceRecoveryMessage::decode(&envelope.data()[..bndr_ui::SURFACE_RECOVERY_WIRE_SIZE])
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let SurfaceRecoveryPayload::ClientContact {
        role,
        cancel_kind,
        checkpoint,
        old_window,
        physical_sequence,
    } = contact.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if contact.sender_sequence() != 1
        || contact.surface_session() != 1
        || contact.route_epoch() != 1
        || role != RecoveryClientRole::App
        || cancel_kind != RecoveryCancelKind::ClientPointer
        || checkpoint != SurfaceRecoveryCheckpoint::MultiWindowInteractive
        || old_window != expected_window
        || physical_sequence != 2
    {
        fail(FAIL_RUNTIME);
    }

    *active_contact = Some(physical_sequence);
    let acknowledgment = SurfaceRecoveryMessage::client_contact_ack(1, contact)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(transport, &acknowledgment.encode());
}

fn send_launcher_hidden_command(transport: u64, window_id: WindowId) {
    send_window_command(
        transport,
        WindowCommand::present(
            3,
            window_id,
            2,
            LAUNCHER_HIDDEN_DAMAGE,
            COLOR_LAUNCHER_HIDDEN,
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
    );
}

fn send_launcher_followup_batch(transport: u64, window_id: WindowId) {
    send_window_command(
        transport,
        WindowCommand::present(
            4,
            window_id,
            3,
            LAUNCHER_PARTIAL_DAMAGE,
            COLOR_LAUNCHER_PARTIAL,
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
    );
    send_window_command(
        transport,
        WindowCommand::raise(5, window_id).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
    );
}

fn send_app_window_batch(transport: u64, window_id: WindowId) {
    send_window_command(
        transport,
        WindowCommand::present(3, window_id, 2, APP_DAMAGE, COLOR_APP_DAMAGE)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
    );
    send_window_command(
        transport,
        WindowCommand::raise(4, window_id).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
    );
}

pub(super) fn launcher_runtime(startup: u64) -> ! {
    let denied = syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0);
    if denied.status != Status::PermissionDenied.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    let lifecycle = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let (init_pid, initial_ui) =
        read_bootstrap_transfer(lifecycle.raw(), UiBootstrapEndpointKind::LauncherUi);
    #[cfg(feature = "unified-product-runtime")]
    super::super::product_runtime::arm(lifecycle.raw(), ShutdownServiceNode::Launcher, init_pid);
    #[cfg(any(
        feature = "service-dependency-runtime",
        all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime")
        )
    ))]
    let mut ui = initial_ui;
    #[cfg(not(any(
        feature = "service-dependency-runtime",
        all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime")
        )
    )))]
    let ui = initial_ui;
    let mut lifecycle_sequence = bndr_ui::AppLifecycleSequenceTracker::new();
    let mut request_sequence = 0;
    let mut state_sequence = 0;
    let mut transaction_id = 1;
    let mut intermediate_seen = false;
    let mut identity = None;
    let mut surface_pid = None;
    let mut ui_tracker = UiServerEventTracker::new();
    let mut window_events = WindowEventTracker::new();
    let mut window_id = None;
    let mut initial_presented = false;
    let mut routed_inputs = 0_u8;
    #[cfg(feature = "post-recovery-focus-runtime")]
    let mut post_recovery_focus_routes = 0_u8;
    #[cfg(feature = "post-recovery-focus-runtime")]
    let mut post_recovery_focus_present_sent = false;
    #[cfg(feature = "post-recovery-focus-runtime")]
    let mut post_recovery_focus_presented = false;
    #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
    let mut post_recovery_lifecycle_ready = false;
    #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
    let mut post_recovery_lifecycle_stage = 0_u8;
    #[cfg(feature = "text-input-runtime")]
    let mut text_routed_inputs = 0_u8;
    #[cfg(feature = "soft-keyboard-runtime")]
    let mut soft_keyboard_routed_inputs = 0_u8;
    #[cfg(any(
        feature = "service-dependency-runtime",
        all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime")
        )
    ))]
    let mut recovery_rebound = false;

    send_launcher_request(
        lifecycle.raw(),
        &mut lifecycle_sequence,
        &mut request_sequence,
        transaction_id,
        AppLifecycleAction::Launch,
        None,
    );

    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    loop {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        let recovery_ui_first = cfg!(any(
            feature = "service-dependency-runtime",
            all(
                feature = "input-server-surface-restart-runtime",
                not(feature = "input-server-restart-runtime")
            )
        ));
        let (lifecycle_index, ui_index) = if recovery_ui_first { (1, 0) } else { (0, 1) };
        items[lifecycle_index] = pack_user_wait_item(lifecycle.raw(), requested);
        items[ui_index] = pack_user_wait_item(ui.raw(), requested);
        let ready = object_wait_many_array(&items, 2, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw()
            || ready.out1 > 1
            || ready.out2 & u64::from(requested.bits()) == 0
        {
            fail(FAIL_RUNTIME);
        }
        let ready_index = usize::try_from(ready.out1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        let peer_closed = ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0;
        let readable = ready.out2 & u64::from(ObjectSignals::READABLE.bits()) != 0;
        if peer_closed && !readable {
            #[cfg(any(
                feature = "service-dependency-runtime",
                all(
                    feature = "input-server-surface-restart-runtime",
                    not(feature = "input-server-restart-runtime")
                )
            ))]
            {
                if ready_index != ui_index
                    || recovery_rebound
                    || !initial_presented
                    || routed_inputs != 2
                {
                    fail(FAIL_RUNTIME);
                }
                finish_launcher_lifecycle_before_rebind(
                    lifecycle.raw(),
                    init_pid,
                    &mut lifecycle_sequence,
                    request_sequence,
                    &mut state_sequence,
                    &mut transaction_id,
                    &mut intermediate_seen,
                    identity,
                );
                let expected_window = window_id.unwrap_or_else(|| fail(FAIL_RUNTIME));
                let old_surface_pid = surface_pid.unwrap_or_else(|| fail(FAIL_RUNTIME));
                let (rebound, replacement_surface_pid) = rebind_surface_client(
                    lifecycle.raw(),
                    init_pid,
                    ui,
                    &mut ui_tracker,
                    old_surface_pid,
                    UiBootstrapEndpointKind::LauncherUi,
                    RecoveryClientRole::Launcher,
                    RecoveryCancelKind::None,
                    expected_window,
                    0,
                    None,
                );
                ui = rebound;
                surface_pid = Some(replacement_surface_pid);
                recovery_rebound = true;
                continue;
            }
            #[cfg(not(any(
                feature = "service-dependency-runtime",
                all(
                    feature = "input-server-surface-restart-runtime",
                    not(feature = "input-server-restart-runtime")
                )
            )))]
            fail(FAIL_RUNTIME);
        }
        #[cfg(not(any(
            feature = "service-dependency-runtime",
            all(
                feature = "input-server-surface-restart-runtime",
                not(feature = "input-server-restart-runtime")
            )
        )))]
        if peer_closed {
            fail(FAIL_RUNTIME);
        }
        #[cfg(any(
            feature = "service-dependency-runtime",
            all(
                feature = "input-server-surface-restart-runtime",
                not(feature = "input-server-restart-runtime")
            )
        ))]
        if peer_closed && ready_index != ui_index {
            fail(FAIL_RUNTIME);
        }
        if !readable {
            fail(FAIL_RUNTIME);
        }

        if ready_index == ui_index {
            match read_client_wire_event(ui.raw(), &mut surface_pid) {
                ClientWireEvent::Legacy(event) => {
                    if ui_tracker.accept(event).is_err() {
                        fail(FAIL_RUNTIME);
                    }
                    match event.payload() {
                        UiServerEventPayload::Ready => {
                            #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
                            if recovery_rebound {
                                let existing = window_id.unwrap_or_else(|| fail(FAIL_RUNTIME));
                                if event.session_id() != 2
                                    || post_recovery_lifecycle_ready
                                    || post_recovery_lifecycle_stage != 0
                                    || existing.slot() != 0
                                    || existing.generation() != 1
                                    || !initial_presented
                                    || routed_inputs != 2
                                    || post_recovery_focus_routes != 0
                                    || post_recovery_focus_present_sent
                                    || post_recovery_focus_presented
                                    || ui_tracker.session_id() != Some(2)
                                    || ui_tracker.last_input_sequence().is_some()
                                    || ui_tracker.last_frame_id().is_some()
                                    || ui_tracker.last_commit().is_some()
                                    || ui_tracker.last_focus_generation().is_some()
                                    || ui_tracker.active_client().is_some()
                                    || ui_tracker.active_app().is_some()
                                    || ui_tracker.outstanding_frame_id().is_some()
                                    || ui_tracker.degraded_reason().is_some()
                                {
                                    fail(FAIL_RUNTIME);
                                }
                                post_recovery_lifecycle_ready = true;
                                continue;
                            }
                            send_window_command(
                                ui.raw(),
                                WindowCommand::create(1, LAUNCHER_BOUNDS)
                                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                            );
                        }
                        UiServerEventPayload::FocusChanged {
                            active_client,
                            app,
                            focus_generation,
                        } => {
                            #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
                            if recovery_rebound {
                                let (expected_client, expected_app, expected_generation, ack_magic) =
                                    match post_recovery_lifecycle_stage {
                                        0 => (
                                            UiClientId::App,
                                            Some(APP),
                                            1,
                                            POST_RECOVERY_READY_FOCUS_ACK_MAGIC,
                                        ),
                                        1 => (
                                            UiClientId::Launcher,
                                            None,
                                            2,
                                            POST_RECOVERY_LAUNCHER_FOCUS_ACK_MAGIC,
                                        ),
                                        2 => (
                                            UiClientId::App,
                                            Some(APP),
                                            3,
                                            POST_RECOVERY_APP_FOCUS_ACK_MAGIC,
                                        ),
                                        _ => fail(FAIL_RUNTIME),
                                    };
                                let focus_stage_valid = match post_recovery_lifecycle_stage {
                                    0 | 1 => {
                                        post_recovery_focus_routes == 0
                                            && !post_recovery_focus_present_sent
                                            && !post_recovery_focus_presented
                                    }
                                    2 => {
                                        post_recovery_focus_routes == 2
                                            && post_recovery_focus_present_sent
                                            && post_recovery_focus_presented
                                    }
                                    _ => false,
                                };
                                if event.session_id() != 2
                                    || !post_recovery_lifecycle_ready
                                    || !focus_stage_valid
                                    || active_client != expected_client
                                    || app != expected_app
                                    || focus_generation != expected_generation
                                    || ui_tracker.session_id() != Some(2)
                                    || ui_tracker.last_focus_generation()
                                        != Some(expected_generation)
                                    || ui_tracker.active_client() != Some(expected_client)
                                    || ui_tracker.active_app() != expected_app
                                    || ui_tracker.outstanding_frame_id().is_some()
                                    || ui_tracker.degraded_reason().is_some()
                                {
                                    fail(FAIL_RUNTIME);
                                }
                                write_wire(ui.raw(), &ack_magic.to_le_bytes());
                                post_recovery_lifecycle_stage = post_recovery_lifecycle_stage
                                    .checked_add(1)
                                    .unwrap_or_else(|| fail(FAIL_RUNTIME));
                                continue;
                            }
                            let _ = (active_client, app, focus_generation);
                        }
                        _ => fail(FAIL_RUNTIME),
                    }
                }
                ClientWireEvent::Window(event) => {
                    if window_events.accept(event).is_err() {
                        fail(FAIL_RUNTIME);
                    }
                    match event.payload() {
                        WindowEventPayload::Created {
                            window_id: created,
                            bounds,
                        } if window_id.is_none()
                            && created.slot() == 0
                            && created.generation() == 1
                            && bounds == LAUNCHER_BOUNDS
                            && event.command_sequence() == 1
                            && event.scene_frame() == 1 =>
                        {
                            window_id = Some(created);
                            send_window_command(
                                ui.raw(),
                                WindowCommand::present(
                                    2,
                                    created,
                                    1,
                                    LAUNCHER_BOUNDS,
                                    COLOR_LAUNCHER_BASE,
                                )
                                .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                            );
                        }
                        WindowEventPayload::Presented {
                            window_id: presented,
                            frame_id: 1,
                        } if Some(presented) == window_id
                            && event.command_sequence() == 2
                            && event.scene_frame() == 2
                            && !initial_presented =>
                        {
                            initial_presented = true;
                            send_launcher_hidden_command(ui.raw(), presented);
                            if transaction_id == 2 && request_sequence == 1 {
                                send_launcher_request(
                                    lifecycle.raw(),
                                    &mut lifecycle_sequence,
                                    &mut request_sequence,
                                    transaction_id,
                                    AppLifecycleAction::Activate,
                                    identity,
                                );
                            }
                        }
                        WindowEventPayload::Presented {
                            window_id: presented,
                            frame_id: 2,
                        } if Some(presented) == window_id
                            && event.command_sequence() == 3
                            && event.scene_frame() == 5 =>
                        {
                            send_launcher_followup_batch(ui.raw(), presented);
                        }
                        WindowEventPayload::Presented {
                            window_id: presented,
                            frame_id: 3,
                        } if Some(presented) == window_id
                            && event.command_sequence() == 4
                            && event.scene_frame() == 6 => {}
                        WindowEventPayload::Raised { window_id: raised }
                            if Some(raised) == window_id
                                && event.command_sequence() == 5
                                && event.scene_frame() == 8 => {}
                        WindowEventPayload::InputRoute {
                            window_id: routed,
                            global_x: 80,
                            global_y: 120,
                            local_x: 80,
                            local_y: 120,
                            pressed,
                            captured: true,
                            focus_generation: 2,
                        } if Some(routed) == window_id
                            && event.command_sequence() == 5
                            && event.scene_frame() == 8
                            && routed_inputs < 2 =>
                        {
                            if pressed != (routed_inputs == 0) {
                                fail(FAIL_RUNTIME);
                            }
                            routed_inputs += 1;
                            if routed_inputs == 1 {
                                write_wire(ui.raw(), &PHASE_TWO_ROUTE_ACK_MAGIC.to_le_bytes());
                            }
                        }
                        #[cfg(feature = "post-recovery-focus-runtime")]
                        WindowEventPayload::InputRoute {
                            window_id: routed,
                            global_x: 80,
                            global_y: 96,
                            local_x: 24,
                            local_y: 32,
                            pressed,
                            captured: true,
                            focus_generation: 4,
                        } if recovery_rebound
                            && Some(routed) == window_id
                            && event.command_sequence() == 5
                            && event.scene_frame() == 10
                            && post_recovery_focus_routes < 2
                            && !post_recovery_focus_present_sent
                            && !post_recovery_focus_presented =>
                        {
                            if pressed != (post_recovery_focus_routes == 0) {
                                fail(FAIL_RUNTIME);
                            }
                            post_recovery_focus_routes += 1;
                            if post_recovery_focus_routes == 2 {
                                send_window_command(
                                    ui.raw(),
                                    WindowCommand::present(
                                        6,
                                        routed,
                                        4,
                                        POST_RECOVERY_LAUNCHER_DAMAGE,
                                        COLOR_POST_RECOVERY_LAUNCHER,
                                    )
                                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                                );
                                post_recovery_focus_present_sent = true;
                            }
                        }
                        #[cfg(feature = "post-recovery-focus-runtime")]
                        WindowEventPayload::Presented {
                            window_id: presented,
                            frame_id: 4,
                        } if recovery_rebound
                            && Some(presented) == window_id
                            && event.command_sequence() == 6
                            && event.scene_frame() == 11
                            && post_recovery_focus_routes == 2
                            && post_recovery_focus_present_sent
                            && !post_recovery_focus_presented =>
                        {
                            post_recovery_focus_presented = true;
                            write_wire(ui.raw(), &POST_RECOVERY_LAUNCHER_ACK_MAGIC.to_le_bytes());
                        }
                        #[cfg(feature = "text-input-runtime")]
                        WindowEventPayload::InputRoute {
                            window_id: routed,
                            global_x: 16,
                            global_y: 32,
                            local_x: 16,
                            local_y: 32,
                            pressed,
                            captured: true,
                            focus_generation: 6,
                        } if Some(routed) == window_id
                            && event.command_sequence() == 5
                            && event.scene_frame() == 13
                            && text_routed_inputs < 2 =>
                        {
                            if pressed != (text_routed_inputs == 0) {
                                fail(FAIL_RUNTIME);
                            }
                            text_routed_inputs += 1;
                        }
                        #[cfg(feature = "soft-keyboard-runtime")]
                        WindowEventPayload::InputRoute {
                            window_id: routed,
                            global_x,
                            global_y,
                            local_x,
                            local_y,
                            pressed,
                            captured: true,
                            focus_generation: 8,
                        } if Some(routed) == window_id
                            && event.command_sequence() == 5
                            && event.scene_frame() == 13
                            && soft_keyboard_routed_inputs < 4 =>
                        {
                            let expected = [
                                (16, 32, 16, 32, true),
                                (16, 32, 16, 32, false),
                                (32, 312, 32, 312, true),
                                (32, 312, 32, 312, false),
                            ][usize::from(soft_keyboard_routed_inputs)];
                            if (global_x, global_y, local_x, local_y, pressed) != expected {
                                fail(FAIL_RUNTIME);
                            }
                            soft_keyboard_routed_inputs += 1;
                        }
                        _ => fail(FAIL_RUNTIME),
                    }
                }
                #[cfg(feature = "text-input-runtime")]
                ClientWireEvent::Text(_) => fail(FAIL_RUNTIME),
            }
            continue;
        }

        if ready_index != lifecycle_index {
            fail(FAIL_RUNTIME);
        }
        let (message, transfer) = read_lifecycle_message(lifecycle.raw(), init_pid, false);
        if transfer.is_some()
            || message.sender_sequence() != next_sequence(&mut state_sequence)
            || message.transaction_id() != transaction_id
            || lifecycle_sequence.accept(message).is_err()
        {
            fail(FAIL_RUNTIME);
        }
        let AppLifecyclePayload::StateChanged {
            app,
            state,
            reason,
            identity: state_identity,
        } = message.payload()
        else {
            fail(FAIL_RUNTIME);
        };
        let state_identity = state_identity.unwrap_or_else(|| fail(FAIL_RUNTIME));
        let action =
            lifecycle_action_for_transaction(transaction_id).unwrap_or_else(|| fail(FAIL_RUNTIME));
        if app != APP {
            fail(FAIL_RUNTIME);
        }
        if !intermediate_seen {
            if state != intermediate_state_for(action)
                || reason != AppLifecycleReason::Requested
                || (action != AppLifecycleAction::Launch && identity != Some(state_identity))
            {
                fail(FAIL_RUNTIME);
            }
            if action == AppLifecycleAction::Launch {
                identity = Some(state_identity);
            }
            intermediate_seen = true;
        } else {
            if state != completed_state_for(action)
                || reason != AppLifecycleReason::Completed
                || identity != Some(state_identity)
            {
                fail(FAIL_RUNTIME);
            }
            intermediate_seen = false;
            transaction_id += 1;
            if transaction_id <= TRANSACTION_COUNT && (transaction_id != 2 || initial_presented) {
                let next = lifecycle_action_for_transaction(transaction_id)
                    .unwrap_or_else(|| fail(FAIL_RUNTIME));
                send_launcher_request(
                    lifecycle.raw(),
                    &mut lifecycle_sequence,
                    &mut request_sequence,
                    transaction_id,
                    next,
                    identity,
                );
            }
        }
    }
}

#[cfg(feature = "app-data-runtime")]
#[derive(Clone, Copy)]
struct AppDataListing {
    session_generation: Option<u64>,
    session_size: usize,
}

#[cfg(feature = "app-data-async-recovery-runtime")]
pub(super) fn app_data_unavailable_retries() -> u64 {
    APP_DATA_UNAVAILABLE_RETRIES.load(Ordering::Acquire)
}

#[cfg(feature = "app-data-runtime")]
fn retry_app_data_syscall(number: SyscallNumber, arg0: u64, arg1: u64, arg2: u64) -> SyscallResult {
    #[cfg(feature = "app-data-async-recovery-runtime")]
    let mut unavailable_retry_available = number == SyscallNumber::FileOpenAt;

    for _ in 0..APP_DATA_RETRY_LIMIT {
        let result = syscall(number, arg0, arg1, arg2);
        if result.status != Status::ShouldWait.raw() {
            #[cfg(feature = "app-data-async-recovery-runtime")]
            if result.status == Status::Unavailable.raw()
                && result.out1 == 0
                && result.out2 == 0
                && unavailable_retry_available
                && APP_DATA_UNAVAILABLE_RETRIES
                    .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            {
                unavailable_retry_available = false;
                continue;
            }
            return result;
        }
        if result.out1 != 0 || result.out2 != 0 {
            fail(FAIL_APP_DATA_DIRECTORY);
        }
        // The AppData ABI pins an in-flight operation to these exact register
        // arguments. An IRQ wake may be unrelated, so bounded exact retries
        // continue until the durable operation publishes a terminal result.
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
    fail(FAIL_APP_DATA_DIRECTORY)
}

#[cfg(feature = "app-data-runtime")]
fn expect_app_data_error(result: SyscallResult, expected: Status, reason: u64) {
    if result.status != expected.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(reason);
    }
}

#[cfg(feature = "app-data-runtime")]
fn app_data_path_syscall(number: SyscallNumber, root: u64, path: &[u8]) -> SyscallResult {
    retry_app_data_syscall(number, root, path.as_ptr() as u64, path.len() as u64)
}

#[cfg(feature = "app-data-runtime")]
fn app_data_replace(root: u64, path: &[u8], value: &[u8], cas: FileReplaceCas) -> SyscallResult {
    let request = FileReplaceRequest::new(path, value.as_ptr() as u64, value.len(), cas)
        .unwrap_or_else(|_| fail(FAIL_APP_DATA_REPLACE));
    let wire = request.encode();
    retry_app_data_syscall(
        SyscallNumber::FileReplaceAt,
        root,
        wire.as_ptr() as u64,
        FILE_REPLACE_REQUEST_WIRE_SIZE as u64,
    )
}

#[cfg(feature = "app-data-runtime")]
fn enumerate_app_data(root: u64) -> AppDataListing {
    let mut cursor = 0_u64;
    let mut count = 0_usize;
    let mut saw_state = false;
    let mut session_generation = None;
    let mut session_size = 0_usize;

    loop {
        let mut wire = [0xa5_u8; APP_DATA_DIRECTORY_ENTRY_WIRE_SIZE];
        let result = retry_app_data_syscall(
            SyscallNumber::DirectoryReadAt,
            root,
            cursor,
            wire.as_mut_ptr() as u64,
        );
        if result.status == Status::NotFound.raw() {
            if result.out1 != 0
                || result.out2 != 0
                || wire != [0xa5; APP_DATA_DIRECTORY_ENTRY_WIRE_SIZE]
            {
                fail(FAIL_APP_DATA_DIRECTORY);
            }
            break;
        }
        if result.status != Status::Ok.raw()
            || result.out1 <= cursor
            || result.out1 > APP_DATA_DIRECTORY_MAX_ENTRIES as u64
            || result.out2 != 0
            || count >= APP_DATA_DIRECTORY_MAX_ENTRIES
        {
            fail(FAIL_APP_DATA_DIRECTORY);
        }
        let entry =
            AppDataDirectoryEntry::decode(&wire).unwrap_or_else(|_| fail(FAIL_APP_DATA_DIRECTORY));
        match (entry.kind(), entry.path_bytes()) {
            (AppDataDirectoryEntryKind::Directory, APP_DATA_STATE_DIRECTORY) => {
                if saw_state {
                    fail(FAIL_APP_DATA_DIRECTORY);
                }
                saw_state = true;
            }
            (AppDataDirectoryEntryKind::File, APP_DATA_SESSION_PATH) => {
                if session_generation.replace(entry.generation()).is_some() {
                    fail(FAIL_APP_DATA_DIRECTORY);
                }
                session_size =
                    usize::try_from(entry.size()).unwrap_or_else(|_| fail(FAIL_APP_DATA_DIRECTORY));
            }
            _ => fail(FAIL_APP_DATA_DIRECTORY),
        }
        count += 1;
        cursor = result.out1;
    }

    if !saw_state || count != usize::from(session_generation.is_some()) + 1 {
        fail(FAIL_APP_DATA_DIRECTORY);
    }
    AppDataListing {
        session_generation,
        session_size,
    }
}

#[cfg(feature = "app-data-runtime")]
fn read_app_data_version(root: u64, expected_size: usize) -> u8 {
    let opened = retry_app_data_syscall(
        SyscallNumber::FileOpenAt,
        root,
        APP_DATA_SESSION_PATH.as_ptr() as u64,
        APP_DATA_SESSION_PATH.len() as u64,
    );
    if opened.status != Status::Ok.raw()
        || opened.out1 == 0
        || opened.out1 == root
        || opened.out2 != expected_size as u64
        || expected_size > APP_DATA_V1_PAYLOAD.len().max(APP_DATA_V2_PAYLOAD.len())
    {
        fail(FAIL_APP_DATA_READ);
    }
    let file = opened.out1;
    let mut bytes = [0xa5_u8; 64];
    let read = retry_app_data_syscall(
        SyscallNumber::VmoRead,
        file,
        bytes.as_mut_ptr() as u64,
        pack_vmo_read(0, expected_size as u32),
    );
    if read.status != Status::Ok.raw()
        || read.out1 != expected_size as u64
        || read.out2 != expected_size as u64
        || bytes[expected_size..].iter().any(|byte| *byte != 0xa5)
    {
        fail(FAIL_APP_DATA_READ);
    }
    close_storage_handle(file, FAIL_APP_DATA_CLEANUP);
    if expected_size == APP_DATA_V1_PAYLOAD.len() && &bytes[..expected_size] == APP_DATA_V1_PAYLOAD
    {
        1
    } else if expected_size == APP_DATA_V2_PAYLOAD.len()
        && &bytes[..expected_size] == APP_DATA_V2_PAYLOAD
    {
        2
    } else {
        fail(FAIL_APP_DATA_READ)
    }
}

#[cfg(feature = "app-data-runtime")]
fn alternate_generation(generation: u64) -> u64 {
    if generation == 1 { 2 } else { generation - 1 }
}

#[cfg(feature = "app-data-runtime")]
fn exercise_app_data_runtime() {
    expect_app_data_error(
        syscall(SyscallNumber::AppDataRootOpen, 1, 0, 0),
        Status::InvalidArgument,
        FAIL_APP_DATA_LOCAL_REJECTION,
    );
    let opened = retry_app_data_syscall(SyscallNumber::AppDataRootOpen, 0, 0, 0);
    if opened.status != Status::Ok.raw()
        || opened.out1 == 0
        || opened.out2 != AppDataPrincipal::PRIMARY_APP.raw()
    {
        fail(FAIL_APP_DATA_ROOT);
    }
    let root = opened.out1;

    expect_app_data_error(
        syscall(
            SyscallNumber::HandleDuplicate,
            root,
            u64::from(Rights::TRANSFER.bits()),
            0,
        ),
        Status::PermissionDenied,
        FAIL_APP_DATA_CAPABILITY,
    );
    let read_only = syscall(
        SyscallNumber::HandleDuplicate,
        root,
        u64::from(Rights::READ.bits()),
        0,
    );
    if read_only.status != Status::Ok.raw()
        || read_only.out1 == 0
        || read_only.out1 == root
        || read_only.out2 != 0
    {
        fail(FAIL_APP_DATA_CAPABILITY);
    }
    expect_app_data_error(
        syscall(
            SyscallNumber::DirectoryCreateAt,
            read_only.out1,
            APP_DATA_STATE_DIRECTORY.as_ptr() as u64,
            APP_DATA_STATE_DIRECTORY.len() as u64,
        ),
        Status::PermissionDenied,
        FAIL_APP_DATA_CAPABILITY,
    );
    close_storage_handle(read_only.out1, FAIL_APP_DATA_CLEANUP);

    let invalid_paths: [&[u8]; 7] = [
        b"/state",
        b".",
        b"..",
        b"state/./bad",
        b"state/../bad",
        b"a/b/c/d/e",
        b"state//bad",
    ];
    for path in invalid_paths {
        expect_app_data_error(
            syscall(
                SyscallNumber::DirectoryCreateAt,
                root,
                path.as_ptr() as u64,
                path.len() as u64,
            ),
            Status::InvalidArgument,
            FAIL_APP_DATA_LOCAL_REJECTION,
        );
    }
    expect_app_data_error(
        syscall(SyscallNumber::DirectoryCreateAt, root, 0, 0),
        Status::InvalidArgument,
        FAIL_APP_DATA_LOCAL_REJECTION,
    );
    expect_app_data_error(
        syscall(SyscallNumber::DirectoryCreateAt, root, USER_GUARD_LOW, 1),
        Status::BadAddress,
        FAIL_APP_DATA_LOCAL_REJECTION,
    );
    expect_app_data_error(
        syscall(
            SyscallNumber::FileReplaceAt,
            root,
            USER_GUARD_LOW,
            FILE_REPLACE_REQUEST_WIRE_SIZE as u64,
        ),
        Status::BadAddress,
        FAIL_APP_DATA_LOCAL_REJECTION,
    );
    expect_app_data_error(
        syscall(SyscallNumber::DirectoryReadAt, root, 0, USER_GUARD_LOW),
        Status::BadAddress,
        FAIL_APP_DATA_LOCAL_REJECTION,
    );

    let directory = app_data_path_syscall(
        SyscallNumber::DirectoryCreateAt,
        root,
        APP_DATA_STATE_DIRECTORY,
    );
    if !matches!(directory.status, status if status == Status::Ok.raw() || status == Status::Conflict.raw())
        || directory.out1 != 0
        || directory.out2 != 0
    {
        fail(FAIL_APP_DATA_DIRECTORY);
    }

    let initial = enumerate_app_data(root);
    let (current_generation, current_version, stale_generation) =
        if let Some(observed_generation) = initial.session_generation {
            let observed_version = read_app_data_version(root, initial.session_size);
            if observed_version == 1 {
                let replaced = app_data_replace(
                    root,
                    APP_DATA_SESSION_PATH,
                    APP_DATA_V2_PAYLOAD,
                    FileReplaceCas::Exact(observed_generation),
                );
                if replaced.status != Status::Ok.raw()
                    || replaced.out1 == 0
                    || replaced.out1 == observed_generation
                    || replaced.out2 != 0
                {
                    fail(FAIL_APP_DATA_REPLACE);
                }
                (replaced.out1, 2, observed_generation)
            } else if observed_version == 2 {
                (
                    observed_generation,
                    2,
                    alternate_generation(observed_generation),
                )
            } else {
                fail(FAIL_APP_DATA_READ)
            }
        } else {
            let created = app_data_replace(
                root,
                APP_DATA_SESSION_PATH,
                APP_DATA_V1_PAYLOAD,
                FileReplaceCas::CreateOnly,
            );
            if created.status != Status::Ok.raw() || created.out1 == 0 || created.out2 != 0 {
                fail(FAIL_APP_DATA_REPLACE);
            }
            (created.out1, 1, alternate_generation(created.out1))
        };

    let stale_payload = if current_version == 1 {
        APP_DATA_V2_PAYLOAD
    } else {
        APP_DATA_V1_PAYLOAD
    };
    expect_app_data_error(
        app_data_replace(
            root,
            APP_DATA_SESSION_PATH,
            stale_payload,
            FileReplaceCas::Exact(stale_generation),
        ),
        Status::Conflict,
        FAIL_APP_DATA_CAS,
    );
    expect_app_data_error(
        app_data_path_syscall(SyscallNumber::UnlinkAt, root, APP_DATA_MISSING_PATH),
        Status::NotFound,
        FAIL_APP_DATA_DIRECTORY,
    );

    let final_listing = enumerate_app_data(root);
    let expected_payload = if current_version == 1 {
        APP_DATA_V1_PAYLOAD
    } else {
        APP_DATA_V2_PAYLOAD
    };
    if final_listing.session_generation != Some(current_generation)
        || final_listing.session_size != expected_payload.len()
        || read_app_data_version(root, final_listing.session_size) != current_version
    {
        fail(FAIL_APP_DATA_READ);
    }
    #[cfg(feature = "app-data-async-recovery-runtime")]
    if app_data_unavailable_retries() != 1 {
        // M59 permits exactly one retry of the read-only FileOpenAt request
        // after the kernel publishes RequiresReset as Unavailable. Mutations,
        // OutcomeUnknown results, and any second Unavailable are never retried.
        fail(FAIL_APP_DATA_READ);
    }
    close_storage_handle(root, FAIL_APP_DATA_CLEANUP);
}

pub(super) fn app_runtime(startup: u64) -> ! {
    let denied = syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0);
    if denied.status != Status::PermissionDenied.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    let lifecycle = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let envelope = read_channel_envelope(lifecycle.raw());
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != APP_LIFECYCLE_WIRE_SIZE
        || envelope.sender_pid() == 0
    {
        fail(FAIL_RUNTIME);
    }
    let init_pid = envelope.sender_pid();
    #[cfg(feature = "unified-product-runtime")]
    super::super::product_runtime::arm(lifecycle.raw(), ShutdownServiceNode::App, init_pid);
    let initial_ui = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    #[cfg(any(
        feature = "service-dependency-runtime",
        all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime")
        )
    ))]
    let mut ui = initial_ui;
    #[cfg(not(any(
        feature = "service-dependency-runtime",
        all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime")
        )
    )))]
    let ui = initial_ui;
    let launch = AppLifecycleMessage::decode(&envelope.data()[..APP_LIFECYCLE_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let AppLifecyclePayload::Command {
        app,
        action: AppLifecycleAction::Launch,
        identity,
    } = launch.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if app != APP || launch.transaction_id() != 1 || launch.sender_sequence() != 1 {
        fail(FAIL_RUNTIME);
    }
    send_app_ack(lifecycle.raw(), 1, AppLifecycleAction::Launch, identity);

    let mut expected_transaction = 2;
    let mut surface_pid = None;
    let mut ui_tracker = UiServerEventTracker::new();
    let mut window_events = WindowEventTracker::new();
    let mut window_id = None;
    let mut initial_presented = false;
    let mut pending_activation = None;
    let mut routed_inputs = 0_u8;
    #[cfg(all(
        feature = "persistent-window-runtime",
        not(feature = "post-recovery-interaction-runtime")
    ))]
    let mut persistent_routed_inputs = 0_u8;
    #[cfg(feature = "post-recovery-interaction-runtime")]
    let persistent_routed_inputs = 0_u8;
    #[cfg(feature = "post-recovery-interaction-runtime")]
    let mut post_recovery_routed_inputs = 0_u8;
    #[cfg(feature = "post-recovery-interaction-runtime")]
    let mut post_recovery_present_sent = false;
    #[cfg(feature = "post-recovery-interaction-runtime")]
    let mut post_recovery_presented = false;
    #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
    let mut post_recovery_roundtrip_routes = 0_u8;
    #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
    let mut post_recovery_roundtrip_present_sent = false;
    #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
    let mut post_recovery_roundtrip_presented = false;
    #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
    let mut post_recovery_lifecycle_ready = false;
    #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
    let mut post_recovery_lifecycle_stage = 0_u8;
    #[cfg(feature = "text-input-runtime")]
    let mut text_routed_inputs = 0_u8;
    #[cfg(feature = "text-input-runtime")]
    let mut text_tracker = TextInputClientTracker::new();
    #[cfg(feature = "text-input-runtime")]
    let mut text_editor = TextEditor::new();
    #[cfg(feature = "text-input-runtime")]
    let mut text_context: Option<TextInputContext> = None;
    #[cfg(feature = "text-input-runtime")]
    let mut text_command_sequence = 0_u64;
    #[cfg(feature = "text-input-runtime")]
    let mut rendered_states = 0_u8;
    #[cfg(feature = "text-input-runtime")]
    let mut deactivation_sent = false;
    #[cfg(feature = "soft-keyboard-runtime")]
    let mut soft_app_routes = 0_u8;
    #[cfg(feature = "soft-keyboard-runtime")]
    let mut soft_rendered_states = 0_u8;
    #[cfg(feature = "soft-keyboard-runtime")]
    let mut soft_deactivation_sent = false;
    #[cfg(any(
        feature = "service-dependency-runtime",
        all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime")
        )
    ))]
    let mut recovery_rebound = false;
    #[cfg(any(
        feature = "service-dependency-runtime",
        all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime")
        )
    ))]
    let mut recovery_active_contact = None;
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);

    loop {
        #[cfg(any(
            feature = "service-dependency-runtime",
            all(
                feature = "input-server-surface-restart-runtime",
                not(feature = "input-server-restart-runtime")
            )
        ))]
        if recovery_rebound && recovery_active_contact.is_some() {
            fail(FAIL_RUNTIME);
        }
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        let recovery_ui_first = cfg!(any(
            feature = "service-dependency-runtime",
            all(
                feature = "input-server-surface-restart-runtime",
                not(feature = "input-server-restart-runtime")
            )
        ));
        let (lifecycle_index, ui_index) = if recovery_ui_first { (1, 0) } else { (0, 1) };
        items[lifecycle_index] = pack_user_wait_item(lifecycle.raw(), requested);
        items[ui_index] = pack_user_wait_item(ui.raw(), requested);
        let ready = object_wait_many_array(&items, 2, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw()
            || ready.out1 > 1
            || ready.out2 & u64::from(requested.bits()) == 0
        {
            fail(FAIL_RUNTIME);
        }
        let ready_index = usize::try_from(ready.out1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        let peer_closed = ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0;
        let readable = ready.out2 & u64::from(ObjectSignals::READABLE.bits()) != 0;
        if peer_closed && !readable {
            #[cfg(any(
                feature = "service-dependency-runtime",
                all(
                    feature = "input-server-surface-restart-runtime",
                    not(feature = "input-server-restart-runtime")
                )
            ))]
            {
                if ready_index != ui_index
                    || recovery_rebound
                    || !initial_presented
                    || routed_inputs != 3
                    || persistent_routed_inputs != 0
                    || recovery_active_contact != Some(2)
                {
                    fail(FAIL_RUNTIME);
                }
                let expected_window = window_id.unwrap_or_else(|| fail(FAIL_RUNTIME));
                let old_surface_pid = surface_pid.unwrap_or_else(|| fail(FAIL_RUNTIME));
                let (rebound, replacement_surface_pid) = rebind_surface_client(
                    lifecycle.raw(),
                    init_pid,
                    ui,
                    &mut ui_tracker,
                    old_surface_pid,
                    UiBootstrapEndpointKind::AppUi,
                    RecoveryClientRole::App,
                    RecoveryCancelKind::ClientPointer,
                    expected_window,
                    2,
                    Some(&mut recovery_active_contact),
                );
                if recovery_active_contact.is_some() {
                    fail(FAIL_RUNTIME);
                }
                ui = rebound;
                surface_pid = Some(replacement_surface_pid);
                recovery_rebound = true;
                continue;
            }
            #[cfg(not(any(
                feature = "service-dependency-runtime",
                all(
                    feature = "input-server-surface-restart-runtime",
                    not(feature = "input-server-restart-runtime")
                )
            )))]
            fail(FAIL_RUNTIME);
        }
        #[cfg(not(any(
            feature = "service-dependency-runtime",
            all(
                feature = "input-server-surface-restart-runtime",
                not(feature = "input-server-restart-runtime")
            )
        )))]
        if peer_closed {
            fail(FAIL_RUNTIME);
        }
        #[cfg(any(
            feature = "service-dependency-runtime",
            all(
                feature = "input-server-surface-restart-runtime",
                not(feature = "input-server-restart-runtime")
            )
        ))]
        if peer_closed && ready_index != ui_index {
            fail(FAIL_RUNTIME);
        }
        if !readable {
            fail(FAIL_RUNTIME);
        }
        if ready_index == ui_index {
            let envelope = read_channel_envelope(ui.raw());
            #[cfg(any(
                feature = "service-dependency-runtime",
                all(
                    feature = "input-server-surface-restart-runtime",
                    not(feature = "input-server-restart-runtime")
                )
            ))]
            if envelope.data().starts_with(b"BSR1") {
                if recovery_rebound
                    || !initial_presented
                    || routed_inputs != 3
                    || persistent_routed_inputs != 0
                    || recovery_active_contact.is_some()
                {
                    fail(FAIL_RUNTIME);
                }
                accept_app_recovery_contact(
                    ui.raw(),
                    envelope,
                    surface_pid.unwrap_or_else(|| fail(FAIL_RUNTIME)),
                    window_id.unwrap_or_else(|| fail(FAIL_RUNTIME)),
                    &mut recovery_active_contact,
                );
                continue;
            }
            match decode_client_wire_event(envelope, &mut surface_pid) {
                ClientWireEvent::Legacy(event) => {
                    if ui_tracker.accept(event).is_err() {
                        fail(FAIL_RUNTIME);
                    }
                    match event.payload() {
                        UiServerEventPayload::Ready => {
                            #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
                            if recovery_rebound {
                                let existing = window_id.unwrap_or_else(|| fail(FAIL_RUNTIME));
                                if event.session_id() != 2
                                    || post_recovery_lifecycle_ready
                                    || post_recovery_lifecycle_stage != 0
                                    || existing.slot() != 1
                                    || existing.generation() != 1
                                    || !initial_presented
                                    || pending_activation.is_some()
                                    || routed_inputs != 3
                                    || persistent_routed_inputs != 0
                                    || recovery_active_contact.is_some()
                                    || post_recovery_routed_inputs != 0
                                    || post_recovery_present_sent
                                    || post_recovery_presented
                                    || post_recovery_roundtrip_routes != 0
                                    || post_recovery_roundtrip_present_sent
                                    || post_recovery_roundtrip_presented
                                    || ui_tracker.session_id() != Some(2)
                                    || ui_tracker.last_input_sequence().is_some()
                                    || ui_tracker.last_frame_id().is_some()
                                    || ui_tracker.last_commit().is_some()
                                    || ui_tracker.last_focus_generation().is_some()
                                    || ui_tracker.active_client().is_some()
                                    || ui_tracker.active_app().is_some()
                                    || ui_tracker.outstanding_frame_id().is_some()
                                    || ui_tracker.degraded_reason().is_some()
                                {
                                    fail(FAIL_RUNTIME);
                                }
                                post_recovery_lifecycle_ready = true;
                                continue;
                            }
                            send_window_command(
                                ui.raw(),
                                WindowCommand::create(1, APP_BOUNDS)
                                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                            );
                        }
                        UiServerEventPayload::FocusChanged {
                            active_client,
                            app,
                            focus_generation,
                        } => {
                            #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
                            if recovery_rebound {
                                let (expected_client, expected_app, expected_generation, ack_magic) =
                                    match post_recovery_lifecycle_stage {
                                        0 => (
                                            UiClientId::App,
                                            Some(APP),
                                            1,
                                            POST_RECOVERY_READY_FOCUS_ACK_MAGIC,
                                        ),
                                        1 => (
                                            UiClientId::Launcher,
                                            None,
                                            2,
                                            POST_RECOVERY_LAUNCHER_FOCUS_ACK_MAGIC,
                                        ),
                                        2 => (
                                            UiClientId::App,
                                            Some(APP),
                                            3,
                                            POST_RECOVERY_APP_FOCUS_ACK_MAGIC,
                                        ),
                                        _ => fail(FAIL_RUNTIME),
                                    };
                                let focus_stage_valid = match post_recovery_lifecycle_stage {
                                    0 => {
                                        post_recovery_routed_inputs == 0
                                            && !post_recovery_present_sent
                                            && !post_recovery_presented
                                            && post_recovery_roundtrip_routes == 0
                                            && !post_recovery_roundtrip_present_sent
                                            && !post_recovery_roundtrip_presented
                                    }
                                    1 | 2 => {
                                        post_recovery_routed_inputs == 2
                                            && post_recovery_present_sent
                                            && post_recovery_presented
                                            && post_recovery_roundtrip_routes == 0
                                            && !post_recovery_roundtrip_present_sent
                                            && !post_recovery_roundtrip_presented
                                    }
                                    _ => false,
                                };
                                if event.session_id() != 2
                                    || !post_recovery_lifecycle_ready
                                    || !focus_stage_valid
                                    || recovery_active_contact.is_some()
                                    || active_client != expected_client
                                    || app != expected_app
                                    || focus_generation != expected_generation
                                    || ui_tracker.session_id() != Some(2)
                                    || ui_tracker.last_focus_generation()
                                        != Some(expected_generation)
                                    || ui_tracker.active_client() != Some(expected_client)
                                    || ui_tracker.active_app() != expected_app
                                    || ui_tracker.outstanding_frame_id().is_some()
                                    || ui_tracker.degraded_reason().is_some()
                                {
                                    fail(FAIL_RUNTIME);
                                }
                                write_wire(ui.raw(), &ack_magic.to_le_bytes());
                                post_recovery_lifecycle_stage = post_recovery_lifecycle_stage
                                    .checked_add(1)
                                    .unwrap_or_else(|| fail(FAIL_RUNTIME));
                                continue;
                            }
                            #[cfg(feature = "text-input-runtime")]
                            if text_tracker.phase() == TextInputSessionPhase::Active
                                && active_client == UiClientId::Launcher
                            {
                                let context = text_context.unwrap_or_else(|| fail(FAIL_RUNTIME));
                                let m43 = context.session_id() == 1
                                    && focus_generation == 4
                                    && text_command_sequence == 6
                                    && rendered_states == 5
                                    && !deactivation_sent;
                                #[cfg(feature = "soft-keyboard-runtime")]
                                let m44 = context.session_id() == 2
                                    && focus_generation == 6
                                    && text_command_sequence == 13
                                    && soft_rendered_states == 5
                                    && !soft_deactivation_sent;
                                #[cfg(not(feature = "soft-keyboard-runtime"))]
                                let m44 = false;
                                if app.is_some()
                                    || (!m43 && !m44)
                                    || text_tracker.outstanding_acknowledgment()
                                {
                                    fail(FAIL_RUNTIME);
                                }
                                text_command_sequence = text_command_sequence
                                    .checked_add(1)
                                    .unwrap_or_else(|| fail(FAIL_RUNTIME));
                                let command =
                                    TextInputCommand::deactivate(text_command_sequence, context)
                                        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                                if text_tracker.send(command).is_err() {
                                    fail(FAIL_RUNTIME);
                                }
                                write_wire(ui.raw(), &command.encode());
                                if m43 {
                                    deactivation_sent = true;
                                } else {
                                    #[cfg(feature = "soft-keyboard-runtime")]
                                    {
                                        soft_deactivation_sent = true;
                                    }
                                }
                            }
                            #[cfg(not(feature = "text-input-runtime"))]
                            let _ = (active_client, app, focus_generation);
                        }
                        _ => fail(FAIL_RUNTIME),
                    }
                }
                ClientWireEvent::Window(event) => {
                    if window_events.accept(event).is_err() {
                        fail(FAIL_RUNTIME);
                    }
                    match event.payload() {
                        WindowEventPayload::Created {
                            window_id: created,
                            bounds,
                        } if window_id.is_none()
                            && created.slot() == 1
                            && created.generation() == 1
                            && bounds == APP_BOUNDS
                            && event.command_sequence() == 1
                            && event.scene_frame() == 3 =>
                        {
                            window_id = Some(created);
                            send_window_command(
                                ui.raw(),
                                WindowCommand::present(
                                    2,
                                    created,
                                    1,
                                    ShellRect::new(0, 0, APP_BOUNDS.width, APP_BOUNDS.height),
                                    COLOR_APP_BASE,
                                )
                                .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                            );
                        }
                        WindowEventPayload::Presented {
                            window_id: presented,
                            frame_id: 1,
                        } if Some(presented) == window_id
                            && event.command_sequence() == 2
                            && event.scene_frame() == 4
                            && !initial_presented =>
                        {
                            initial_presented = true;
                            if let Some(action) = pending_activation.take() {
                                send_app_ack(
                                    lifecycle.raw(),
                                    expected_transaction,
                                    action,
                                    identity,
                                );
                                expected_transaction += 1;
                            }
                        }
                        WindowEventPayload::Presented {
                            window_id: presented,
                            frame_id: 2,
                        } if Some(presented) == window_id
                            && event.command_sequence() == 3
                            && event.scene_frame() == 7 => {}
                        WindowEventPayload::Raised { window_id: raised }
                            if Some(raised) == window_id
                                && event.command_sequence() == 4
                                && event.scene_frame() == 9 => {}
                        WindowEventPayload::InputRoute {
                            window_id: routed,
                            global_x,
                            global_y,
                            local_x,
                            local_y,
                            pressed,
                            captured: true,
                            focus_generation: 1,
                        } if Some(routed) == window_id
                            && event.command_sequence() == 2
                            && event.scene_frame() == 5
                            && routed_inputs < 3 =>
                        {
                            let expected = [
                                (80, 120, 32, 40, true),
                                (16, 32, -32, -48, true),
                                (16, 32, -32, -48, false),
                            ][usize::from(routed_inputs)];
                            if (global_x, global_y, local_x, local_y, pressed) != expected {
                                fail(FAIL_RUNTIME);
                            }
                            routed_inputs += 1;
                            if routed_inputs == 3 {
                                // The existing damage command is also the
                                // App's causal acknowledgement that all three
                                // scene-five routes were consumed.  Surface
                                // blocks on this command before it can publish
                                // the later Launcher route group, preserving
                                // the global input transcript without adding a
                                // protocol message or ABI surface.
                                send_app_window_batch(ui.raw(), routed);
                            }
                        }
                        #[cfg(feature = "post-recovery-interaction-runtime")]
                        WindowEventPayload::InputRoute {
                            window_id: routed,
                            global_x: 136,
                            global_y: 184,
                            local_x: 32,
                            local_y: 40,
                            pressed,
                            captured: true,
                            focus_generation: 3,
                        } if recovery_rebound
                            && Some(routed) == window_id
                            && event.command_sequence() == 4
                            && event.scene_frame() == 9
                            && post_recovery_routed_inputs < 2
                            && !post_recovery_present_sent
                            && !post_recovery_presented =>
                        {
                            if pressed != (post_recovery_routed_inputs == 0) {
                                fail(FAIL_RUNTIME);
                            }
                            post_recovery_routed_inputs += 1;
                            if post_recovery_routed_inputs == 2 {
                                send_window_command(
                                    ui.raw(),
                                    WindowCommand::present(
                                        5,
                                        routed,
                                        3,
                                        POST_RECOVERY_APP_DAMAGE,
                                        COLOR_POST_RECOVERY_APP,
                                    )
                                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                                );
                                post_recovery_present_sent = true;
                            }
                        }
                        #[cfg(all(
                            feature = "persistent-window-runtime",
                            not(feature = "post-recovery-interaction-runtime")
                        ))]
                        WindowEventPayload::InputRoute {
                            window_id: routed,
                            global_x,
                            global_y,
                            local_x,
                            local_y,
                            pressed,
                            captured: true,
                            focus_generation: 3,
                        } if Some(routed) == window_id
                            && event.command_sequence() == 4
                            && event.scene_frame() == 9
                            && persistent_routed_inputs < 3 =>
                        {
                            let expected = [
                                (80, 120, 32, 40, true),
                                (0, 0, -84, -124, true),
                                (0, 0, -84, -124, false),
                            ][usize::from(persistent_routed_inputs)];
                            if (global_x, global_y, local_x, local_y, pressed) != expected {
                                fail(FAIL_RUNTIME);
                            }
                            persistent_routed_inputs += 1;
                            if persistent_routed_inputs == 3 {
                                send_window_command(
                                    ui.raw(),
                                    WindowCommand::destroy(5, routed)
                                        .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                                );
                            }
                        }
                        #[cfg(feature = "post-recovery-interaction-runtime")]
                        WindowEventPayload::Presented {
                            window_id: presented,
                            frame_id: 3,
                        } if recovery_rebound
                            && Some(presented) == window_id
                            && event.command_sequence() == 5
                            && event.scene_frame() == 10
                            && post_recovery_routed_inputs == 2
                            && post_recovery_present_sent
                            && !post_recovery_presented =>
                        {
                            post_recovery_presented = true;
                            write_wire(ui.raw(), &POST_RECOVERY_APP_ACK_MAGIC.to_le_bytes());
                        }
                        #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
                        WindowEventPayload::InputRoute {
                            window_id: routed,
                            global_x: 136,
                            global_y: 184,
                            local_x: 32,
                            local_y: 40,
                            pressed,
                            captured: true,
                            focus_generation: 5,
                        } if recovery_rebound
                            && Some(routed) == window_id
                            && event.command_sequence() == 5
                            && event.scene_frame() == 11
                            && post_recovery_routed_inputs == 2
                            && post_recovery_present_sent
                            && post_recovery_presented
                            && post_recovery_roundtrip_routes < 2
                            && !post_recovery_roundtrip_present_sent
                            && !post_recovery_roundtrip_presented =>
                        {
                            if pressed != (post_recovery_roundtrip_routes == 0) {
                                fail(FAIL_RUNTIME);
                            }
                            post_recovery_roundtrip_routes += 1;
                            if post_recovery_roundtrip_routes == 2 {
                                send_window_command(
                                    ui.raw(),
                                    WindowCommand::present(
                                        6,
                                        routed,
                                        4,
                                        POST_RECOVERY_APP_DAMAGE,
                                        COLOR_APP_DAMAGE,
                                    )
                                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                                );
                                post_recovery_roundtrip_present_sent = true;
                            }
                        }
                        #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
                        WindowEventPayload::Presented {
                            window_id: presented,
                            frame_id: 4,
                        } if recovery_rebound
                            && Some(presented) == window_id
                            && event.command_sequence() == 6
                            && event.scene_frame() == 12
                            && post_recovery_routed_inputs == 2
                            && post_recovery_present_sent
                            && post_recovery_presented
                            && post_recovery_roundtrip_routes == 2
                            && post_recovery_roundtrip_present_sent
                            && !post_recovery_roundtrip_presented =>
                        {
                            post_recovery_roundtrip_presented = true;
                            write_wire(
                                ui.raw(),
                                &POST_RECOVERY_ROUNDTRIP_APP_ACK_MAGIC.to_le_bytes(),
                            );
                            #[cfg(feature = "app-data-runtime")]
                            {
                                if post_recovery_lifecycle_stage != 3 {
                                    fail(FAIL_APP_DATA_AUTHORITY);
                                }
                                // The roundtrip App ACK is the final M52/M53
                                // protocol action. Starting durable I/O before
                                // this boundary would stall the last captured
                                // contact while the monitor services AppData.
                                exercise_app_data_runtime();
                            }
                        }
                        #[cfg(feature = "persistent-window-runtime")]
                        WindowEventPayload::Destroyed {
                            window_id: destroyed,
                        } if Some(destroyed) == window_id
                            && destroyed.slot() == 1
                            && destroyed.generation() == 1
                            && event.command_sequence() == 5
                            && event.scene_frame() == 10
                            && persistent_routed_inputs == 3 =>
                        {
                            if window_events.begin_next_lifecycle().is_err() {
                                fail(FAIL_RUNTIME);
                            }
                            window_id = None;
                            send_window_command(
                                ui.raw(),
                                WindowCommand::create(6, APP_BOUNDS)
                                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                            );
                        }
                        #[cfg(feature = "persistent-window-runtime")]
                        WindowEventPayload::Created {
                            window_id: created,
                            bounds,
                        } if window_id.is_none()
                            && created.slot() == 1
                            && created.generation() == 2
                            && bounds == APP_BOUNDS
                            && event.command_sequence() == 6
                            && event.scene_frame() == 11 =>
                        {
                            window_id = Some(created);
                            send_window_command(
                                ui.raw(),
                                WindowCommand::present(
                                    7,
                                    created,
                                    1,
                                    ShellRect::new(0, 0, APP_BOUNDS.width, APP_BOUNDS.height),
                                    COLOR_APP_RECREATED,
                                )
                                .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                            );
                        }
                        #[cfg(feature = "persistent-window-runtime")]
                        WindowEventPayload::Presented {
                            window_id: presented,
                            frame_id: 1,
                        } if Some(presented) == window_id
                            && presented.generation() == 2
                            && event.command_sequence() == 7
                            && event.scene_frame() == 12 =>
                        {
                            send_window_command(
                                ui.raw(),
                                WindowCommand::raise(8, presented)
                                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                            );
                        }
                        #[cfg(feature = "persistent-window-runtime")]
                        WindowEventPayload::Raised { window_id: raised }
                            if Some(raised) == window_id
                                && raised.generation() == 2
                                && event.command_sequence() == 8
                                && event.scene_frame() == 13 => {}
                        #[cfg(feature = "text-input-runtime")]
                        WindowEventPayload::InputRoute {
                            window_id: routed,
                            global_x: 80,
                            global_y: 120,
                            local_x: 32,
                            local_y: 40,
                            pressed,
                            captured: true,
                            focus_generation: 5,
                        } if Some(routed) == window_id
                            && routed.generation() == 2
                            && event.command_sequence() == 8
                            && event.scene_frame() == 13
                            && text_routed_inputs < 2 =>
                        {
                            if pressed != (text_routed_inputs == 0) {
                                fail(FAIL_RUNTIME);
                            }
                            text_routed_inputs += 1;
                            if pressed {
                                let context = TextInputContext::try_new(routed, 1, 5)
                                    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                                if text_context.replace(context).is_some()
                                    || text_command_sequence != 0
                                {
                                    fail(FAIL_RUNTIME);
                                }
                                text_command_sequence = 1;
                                let command = TextInputCommand::activate(1, context)
                                    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                                if text_tracker.send(command).is_err() {
                                    fail(FAIL_RUNTIME);
                                }
                                write_wire(ui.raw(), &command.encode());
                            }
                        }
                        #[cfg(feature = "soft-keyboard-runtime")]
                        WindowEventPayload::InputRoute {
                            window_id: routed,
                            global_x: 80,
                            global_y: 120,
                            local_x: 32,
                            local_y: 40,
                            pressed,
                            captured: true,
                            focus_generation,
                        } if Some(routed) == window_id
                            && routed.generation() == 2
                            && event.command_sequence() == 8
                            && event.scene_frame() == 13
                            && matches!(focus_generation, 7 | 9)
                            && soft_app_routes < 4 =>
                        {
                            let expected_focus = if soft_app_routes < 2 { 7 } else { 9 };
                            if focus_generation != expected_focus
                                || pressed != (soft_app_routes & 1 == 0)
                            {
                                fail(FAIL_RUNTIME);
                            }
                            soft_app_routes += 1;
                            if !pressed {
                                let (session, previous_session, expected_sequence, expected_state) =
                                    if focus_generation == 7 {
                                        (2, 1, 7, expected_text_state(5))
                                    } else {
                                        (3, 2, 14, expected_soft_text_state(5))
                                    };
                                let previous = text_context.unwrap_or_else(|| fail(FAIL_RUNTIME));
                                if previous.session_id() != previous_session
                                    || text_command_sequence != expected_sequence
                                    || text_tracker.phase() != TextInputSessionPhase::Deactivated
                                    || text_tracker.outstanding_acknowledgment()
                                    || text_editor.is_active()
                                    || text_editor.state() != expected_state
                                {
                                    fail(FAIL_RUNTIME);
                                }
                                let context =
                                    TextInputContext::try_new(routed, session, focus_generation)
                                        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                                text_context = Some(context);
                                text_command_sequence = text_command_sequence
                                    .checked_add(1)
                                    .unwrap_or_else(|| fail(FAIL_RUNTIME));
                                let command =
                                    TextInputCommand::activate(text_command_sequence, context)
                                        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                                if text_tracker.send(command).is_err() {
                                    fail(FAIL_RUNTIME);
                                }
                                write_wire(ui.raw(), &command.encode());
                            }
                        }
                        _ => fail(FAIL_RUNTIME),
                    }
                }
                #[cfg(feature = "text-input-runtime")]
                ClientWireEvent::Text(event) => {
                    let context = text_context.unwrap_or_else(|| fail(FAIL_RUNTIME));
                    if event.context() != context
                        || text_tracker.receive(event).is_err()
                        || text_editor.apply(event).is_err()
                    {
                        fail(FAIL_RUNTIME);
                    }
                    match event.payload() {
                        TextInputEventPayload::Activated {
                            acknowledged_command_sequence: 1,
                            revision: 0,
                        } if event.sequence() == 1
                            && text_editor.is_active()
                            && text_editor.state() == TextState::default()
                            && text_editor.preedit().is_none() => {}
                        TextInputEventPayload::Preedit {
                            revision,
                            scalar: 'a',
                        } if matches!((event.sequence(), revision), (2, 1) | (8, 4)) => {
                            let step = u8::try_from(event.sequence() / 2)
                                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                            if text_editor.state() != expected_text_state(step)
                                || text_editor.preedit() != Some('a')
                            {
                                fail(FAIL_RUNTIME);
                            }
                            text_command_sequence += 1;
                            let command = TextInputCommand::state_ack(
                                text_command_sequence,
                                context,
                                event.sequence(),
                                text_editor.state(),
                            )
                            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                            if text_command_sequence != u64::from(step) + 1
                                || text_tracker.send(command).is_err()
                            {
                                fail(FAIL_RUNTIME);
                            }
                            write_wire(ui.raw(), &command.encode());
                        }
                        TextInputEventPayload::Commit { revision, text }
                            if text.as_str() == "a"
                                && matches!((event.sequence(), revision), (4, 2) | (10, 5)) =>
                        {
                            let step = u8::try_from(event.sequence() / 2)
                                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                            if text_editor.state() != expected_text_state(step)
                                || text_editor.preedit().is_some()
                            {
                                fail(FAIL_RUNTIME);
                            }
                            text_command_sequence += 1;
                            let command = TextInputCommand::state_ack(
                                text_command_sequence,
                                context,
                                event.sequence(),
                                text_editor.state(),
                            )
                            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                            if text_command_sequence != u64::from(step) + 1
                                || text_tracker.send(command).is_err()
                            {
                                fail(FAIL_RUNTIME);
                            }
                            write_wire(ui.raw(), &command.encode());
                        }
                        TextInputEventPayload::DeleteSurrounding {
                            revision: 3,
                            before_scalars: 1,
                            after_scalars: 0,
                        } if event.sequence() == 6 => {
                            if text_editor.state() != expected_text_state(3)
                                || text_editor.preedit().is_some()
                            {
                                fail(FAIL_RUNTIME);
                            }
                            text_command_sequence += 1;
                            let command = TextInputCommand::state_ack(
                                text_command_sequence,
                                context,
                                event.sequence(),
                                text_editor.state(),
                            )
                            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                            if text_command_sequence != 4 || text_tracker.send(command).is_err() {
                                fail(FAIL_RUNTIME);
                            }
                            write_wire(ui.raw(), &command.encode());
                        }
                        TextInputEventPayload::Rendered {
                            acknowledged_command_sequence,
                            state,
                        } if context.session_id() == 1 => {
                            let step = u8::try_from((event.sequence() - 1) / 2)
                                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                            if !matches!(event.sequence(), 3 | 5 | 7 | 9 | 11)
                                || step != rendered_states + 1
                                || acknowledged_command_sequence != u64::from(step) + 1
                                || state != expected_text_state(step)
                                || text_editor.state() != state
                            {
                                fail(FAIL_RUNTIME);
                            }
                            rendered_states = step;
                        }
                        TextInputEventPayload::Deactivated {
                            acknowledged_command_sequence: 7,
                            revision: 5,
                        } if event.sequence() == 12
                            && deactivation_sent
                            && rendered_states == 5
                            && !text_editor.is_active()
                            && text_editor.state() == expected_text_state(5)
                            && text_editor.preedit().is_none() => {}
                        #[cfg(feature = "soft-keyboard-runtime")]
                        TextInputEventPayload::Activated {
                            acknowledged_command_sequence: 8,
                            revision: 0,
                        } if context.session_id() == 2
                            && event.sequence() == 13
                            && text_editor.is_active()
                            && text_editor.state()
                                == TextState::try_new(0, "a", 1, 1)
                                    .unwrap_or_else(|_| fail(FAIL_RUNTIME))
                            && text_editor.preedit().is_none() => {}
                        #[cfg(feature = "soft-keyboard-runtime")]
                        TextInputEventPayload::Preedit {
                            revision,
                            scalar: 'a',
                        } if context.session_id() == 2
                            && matches!((event.sequence(), revision), (14, 1) | (20, 4)) =>
                        {
                            let step = u8::try_from((event.sequence() - 12) / 2)
                                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                            if text_editor.state() != expected_soft_text_state(step)
                                || text_editor.preedit() != Some('a')
                            {
                                fail(FAIL_RUNTIME);
                            }
                            text_command_sequence = text_command_sequence
                                .checked_add(1)
                                .unwrap_or_else(|| fail(FAIL_RUNTIME));
                            let command = TextInputCommand::state_ack(
                                text_command_sequence,
                                context,
                                event.sequence(),
                                text_editor.state(),
                            )
                            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                            if text_command_sequence != u64::from(step) + 8
                                || text_tracker.send(command).is_err()
                            {
                                fail(FAIL_RUNTIME);
                            }
                            write_wire(ui.raw(), &command.encode());
                        }
                        #[cfg(feature = "soft-keyboard-runtime")]
                        TextInputEventPayload::Commit { revision, text }
                            if context.session_id() == 2
                                && text.as_str() == "a"
                                && matches!((event.sequence(), revision), (16, 2) | (22, 5)) =>
                        {
                            let step = u8::try_from((event.sequence() - 12) / 2)
                                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                            if text_editor.state() != expected_soft_text_state(step)
                                || text_editor.preedit().is_some()
                            {
                                fail(FAIL_RUNTIME);
                            }
                            text_command_sequence = text_command_sequence
                                .checked_add(1)
                                .unwrap_or_else(|| fail(FAIL_RUNTIME));
                            let command = TextInputCommand::state_ack(
                                text_command_sequence,
                                context,
                                event.sequence(),
                                text_editor.state(),
                            )
                            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                            if text_command_sequence != u64::from(step) + 8
                                || text_tracker.send(command).is_err()
                            {
                                fail(FAIL_RUNTIME);
                            }
                            write_wire(ui.raw(), &command.encode());
                        }
                        #[cfg(feature = "soft-keyboard-runtime")]
                        TextInputEventPayload::DeleteSurrounding {
                            revision: 3,
                            before_scalars: 1,
                            after_scalars: 0,
                        } if context.session_id() == 2 && event.sequence() == 18 => {
                            if text_editor.state() != expected_soft_text_state(3)
                                || text_editor.preedit().is_some()
                            {
                                fail(FAIL_RUNTIME);
                            }
                            text_command_sequence = text_command_sequence
                                .checked_add(1)
                                .unwrap_or_else(|| fail(FAIL_RUNTIME));
                            let command = TextInputCommand::state_ack(
                                text_command_sequence,
                                context,
                                event.sequence(),
                                text_editor.state(),
                            )
                            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                            if text_command_sequence != 11 || text_tracker.send(command).is_err() {
                                fail(FAIL_RUNTIME);
                            }
                            write_wire(ui.raw(), &command.encode());
                        }
                        #[cfg(feature = "soft-keyboard-runtime")]
                        TextInputEventPayload::Rendered {
                            acknowledged_command_sequence,
                            state,
                        } if context.session_id() == 2
                            && matches!(event.sequence(), 15 | 17 | 19 | 21 | 23) =>
                        {
                            let step = u8::try_from((event.sequence() - 13) / 2)
                                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                            if step != soft_rendered_states + 1
                                || acknowledged_command_sequence != u64::from(step) + 8
                                || state != expected_soft_text_state(step)
                                || text_editor.state() != state
                            {
                                fail(FAIL_RUNTIME);
                            }
                            soft_rendered_states = step;
                        }
                        #[cfg(feature = "soft-keyboard-runtime")]
                        TextInputEventPayload::Deactivated {
                            acknowledged_command_sequence: 14,
                            revision: 5,
                        } if context.session_id() == 2
                            && event.sequence() == 24
                            && soft_deactivation_sent
                            && soft_rendered_states == 5
                            && !text_editor.is_active()
                            && text_editor.state() == expected_soft_text_state(5)
                            && text_editor.preedit().is_none() => {}
                        #[cfg(feature = "soft-keyboard-runtime")]
                        TextInputEventPayload::Activated {
                            acknowledged_command_sequence: 15,
                            revision: 0,
                        } if context.session_id() == 3
                            && event.sequence() == 25
                            && soft_app_routes == 4
                            && text_editor.is_active()
                            && text_editor.state()
                                == TextState::try_new(0, "aa", 2, 2)
                                    .unwrap_or_else(|_| fail(FAIL_RUNTIME))
                            && text_editor.preedit().is_none() => {}
                        _ => fail(FAIL_RUNTIME),
                    }
                }
            }
            continue;
        }

        if ready_index != lifecycle_index {
            fail(FAIL_RUNTIME);
        }
        #[cfg(feature = "unified-product-runtime")]
        if expected_transaction > TRANSACTION_COUNT {
            // M45 treated any lifecycle traffic after its final activation as
            // a protocol violation. M67 deliberately begins its authenticated
            // product control plane at that exact steady-state boundary. The
            // shared envelope reader consumes every product command and never
            // returns before the terminal Exit command; any ordinary envelope
            // still falls through to the legacy failure below.
            let _unexpected = read_channel_envelope(lifecycle.raw());
            fail(FAIL_RUNTIME);
        }
        #[cfg(not(feature = "unified-product-runtime"))]
        if expected_transaction > TRANSACTION_COUNT {
            fail(FAIL_RUNTIME);
        }
        let (command, transfer) = read_lifecycle_message(lifecycle.raw(), init_pid, false);
        let AppLifecyclePayload::Command {
            app,
            action,
            identity: command_identity,
        } = command.payload()
        else {
            fail(FAIL_RUNTIME);
        };
        if transfer.is_some()
            || app != APP
            || command.sender_sequence() != app_endpoint_sequence(expected_transaction)
            || command.transaction_id() != expected_transaction
            || command_identity != identity
            || action != AppLifecycleAction::Activate
        {
            fail(FAIL_RUNTIME);
        }
        if initial_presented {
            send_app_ack(lifecycle.raw(), expected_transaction, action, identity);
            expected_transaction += 1;
        } else if pending_activation.replace(action).is_some() {
            fail(FAIL_RUNTIME);
        }
    }
}

pub(super) fn decode_window_command(transport: u64, expected_sender: u64) -> (WindowCommand, u64) {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != WINDOW_COMMAND_WIRE_SIZE
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() == 0
        || (expected_sender != 0 && envelope.sender_pid() != expected_sender)
    {
        fail(FAIL_RUNTIME);
    }
    let command = WindowCommand::decode(&envelope.data()[..WINDOW_COMMAND_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    (command, envelope.sender_pid())
}

#[cfg(feature = "text-input-runtime")]
enum AppRuntimeCommand {
    Window(WindowCommand),
    Text(TextInputCommand),
}

#[cfg(feature = "text-input-runtime")]
fn decode_app_runtime_command(transport: u64, expected_sender: u64) -> AppRuntimeCommand {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != WINDOW_COMMAND_WIRE_SIZE
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() != expected_sender
    {
        fail(FAIL_RUNTIME);
    }
    let wire = &envelope.data()[..WINDOW_COMMAND_WIRE_SIZE];
    if wire.starts_with(b"BWC1") {
        AppRuntimeCommand::Window(
            WindowCommand::decode(wire).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        )
    } else if wire.starts_with(b"BTI1") {
        AppRuntimeCommand::Text(
            TextInputCommand::decode(&wire[..TEXT_INPUT_COMMAND_WIRE_SIZE])
                .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        )
    } else {
        fail(FAIL_RUNTIME)
    }
}

pub(super) fn command_window_id(command: WindowCommand) -> Option<WindowId> {
    match command.payload() {
        WindowCommandPayload::Create { .. } => None,
        WindowCommandPayload::Present { window_id, .. }
        | WindowCommandPayload::Raise { window_id }
        | WindowCommandPayload::Destroy { window_id } => Some(window_id),
    }
}
