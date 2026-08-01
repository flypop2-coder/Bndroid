//! Atomic, allocation-free evidence for the M41 userspace window compositor.
//!
//! The syscall layer authenticates both generation-qualified process identities
//! and decodes a canonical [`WindowChannelTrace`] before calling [`record`].
//! This module then proves the fixed two-client M41 transcript transactionally:
//! a rejected trace increments exactly one error category and otherwise leaves
//! every protocol counter, identity, damage ledger, z-order edge, and input
//! capture edge unchanged.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bndr_ui::{ShellRect, WindowCommandOpcode, WindowId};

use crate::ui_trace::{UiTraceRole, WindowChannelTrace};

const CLIENT_COUNT: usize = 2;
const INPUT_ROUTE_COUNT: usize = 5;

const LAUNCHER: usize = 0;
const APP: usize = 1;

const LAUNCHER_TOKEN: u64 = 1_u64 << 32;
const APP_TOKEN: u64 = (1_u64 << 32) | 1;

const LAUNCHER_BOUNDS: ShellRect = ShellRect::new(0, 0, 208, 368);
const APP_BOUNDS: ShellRect = ShellRect::new(48, 80, 112, 160);
const APP_LOCAL_BOUNDS: ShellRect = ShellRect::new(0, 0, 112, 160);
const LAUNCHER_HIDDEN_DAMAGE: ShellRect = ShellRect::new(64, 96, 32, 32);
const LAUNCHER_PARTIAL_DAMAGE: ShellRect = ShellRect::new(32, 64, 64, 64);
const APP_DAMAGE: ShellRect = ShellRect::new(16, 24, 40, 32);

const COLOR_LAUNCHER_BASE: u32 = 0x0016_2436;
const COLOR_APP_BASE: u32 = 0x0038_5a7c;
const COLOR_LAUNCHER_HIDDEN: u32 = 0x00d1_6b45;
const COLOR_LAUNCHER_PARTIAL: u32 = 0x0029_a36a;
const COLOR_APP_DAMAGE: u32 = 0x0084_4ec7;

const LAUNCHER_FULL_PIXELS: u64 = 208 * 368;
const APP_FULL_PIXELS: u64 = 112 * 160;
const LAUNCHER_HIDDEN_PIXELS: u64 = 32 * 32;
const LAUNCHER_PARTIAL_PIXELS: u64 = 64 * 64;
const LAUNCHER_PARTIAL_OCCLUDED_PIXELS: u64 = 48 * 48;
const APP_DAMAGE_PIXELS: u64 = 40 * 32;
const OPAQUE_OVERLAP_PIXELS: u64 = 112 * 160;

const EXPECTED_COMMANDS: u64 = 9;
const EXPECTED_COMMAND_EVENTS: u64 = 9;
const EXPECTED_INPUT_EVENTS: u64 = 5;
const OUTPUT_COMMIT_COUNT: usize = 6;
const EXPECTED_OUTPUT_SLOTS: [usize; OUTPUT_COMMIT_COUNT] = [0, 1, 0, 1, 0, 1];
const EXPECTED_OUTPUT_WRITE_GENERATIONS: [u64; OUTPUT_COMMIT_COUNT] = [1, 1, 2, 2, 3, 3];

const TOP_NONE: u64 = 0;
const TOP_LAUNCHER: u64 = 1;
const TOP_APP: u64 = 2;

const LIVE_LAUNCHER: u64 = 1 << LAUNCHER;
const LIVE_APP: u64 = 1 << APP;
const LIVE_BOTH: u64 = LIVE_LAUNCHER | LIVE_APP;

const fn sequence_bit(sequence: u64) -> u64 {
    1_u64 << sequence
}

const LAUNCHER_FINAL_ACK_MASK: u64 =
    sequence_bit(1) | sequence_bit(2) | sequence_bit(3) | sequence_bit(4) | sequence_bit(5);
const APP_FINAL_ACK_MASK: u64 =
    sequence_bit(1) | sequence_bit(2) | sequence_bit(3) | sequence_bit(4);

static TRACES: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);
static OWNER_ERRORS: AtomicU64 = AtomicU64::new(0);
static SEQUENCE_ERRORS: AtomicU64 = AtomicU64::new(0);
static PAYLOAD_ERRORS: AtomicU64 = AtomicU64::new(0);
static EVENT_ERRORS: AtomicU64 = AtomicU64::new(0);
static SCENE_ERRORS: AtomicU64 = AtomicU64::new(0);
static PHASE_ERRORS: AtomicU64 = AtomicU64::new(0);

static COMMANDS: AtomicU64 = AtomicU64::new(0);
static CREATE_COMMANDS: AtomicU64 = AtomicU64::new(0);
static PRESENT_COMMANDS: AtomicU64 = AtomicU64::new(0);
static RAISE_COMMANDS: AtomicU64 = AtomicU64::new(0);
static DESTROY_COMMANDS: AtomicU64 = AtomicU64::new(0);
static CLIENT_COMMANDS: [AtomicU64; CLIENT_COUNT] = [const { AtomicU64::new(0) }; CLIENT_COUNT];
static LAST_COMMAND_SEQUENCE: [AtomicU64; CLIENT_COUNT] =
    [const { AtomicU64::new(0) }; CLIENT_COUNT];
static COMMAND_MASK: [AtomicU64; CLIENT_COUNT] = [const { AtomicU64::new(0) }; CLIENT_COUNT];

static EVENTS: AtomicU64 = AtomicU64::new(0);
static CREATED_EVENTS: AtomicU64 = AtomicU64::new(0);
static PRESENTED_EVENTS: AtomicU64 = AtomicU64::new(0);
static RAISED_EVENTS: AtomicU64 = AtomicU64::new(0);
static INPUT_EVENTS: AtomicU64 = AtomicU64::new(0);
static DESTROYED_EVENTS: AtomicU64 = AtomicU64::new(0);
static CLIENT_EVENTS: [AtomicU64; CLIENT_COUNT] = [const { AtomicU64::new(0) }; CLIENT_COUNT];
static LAST_SCENE_FRAME: [AtomicU64; CLIENT_COUNT] = [const { AtomicU64::new(0) }; CLIENT_COUNT];
static ACK_MASK: [AtomicU64; CLIENT_COUNT] = [const { AtomicU64::new(0) }; CLIENT_COUNT];

static WINDOW_TOKENS: [AtomicU64; CLIENT_COUNT] = [const { AtomicU64::new(0) }; CLIENT_COUNT];
static LIVE_MASK: AtomicU64 = AtomicU64::new(0);
static TOP_SLOT: AtomicU64 = AtomicU64::new(TOP_NONE);
static Z_ORDER_CHANGES: AtomicU64 = AtomicU64::new(0);

static CONTENT_DAMAGE_PIXELS: [AtomicU64; CLIENT_COUNT] =
    [const { AtomicU64::new(0) }; CLIENT_COUNT];
static VISIBLE_DAMAGE_PIXELS: [AtomicU64; CLIENT_COUNT] =
    [const { AtomicU64::new(0) }; CLIENT_COUNT];
static OCCLUDED_DAMAGE_PIXELS: [AtomicU64; CLIENT_COUNT] =
    [const { AtomicU64::new(0) }; CLIENT_COUNT];
static CONTENT_PRESENTS: [AtomicU64; CLIENT_COUNT] = [const { AtomicU64::new(0) }; CLIENT_COUNT];
static LAST_CLIENT_FRAME: [AtomicU64; CLIENT_COUNT] = [const { AtomicU64::new(0) }; CLIENT_COUNT];
static LAST_COLOR: [AtomicU64; CLIENT_COUNT] = [const { AtomicU64::new(0) }; CLIENT_COUNT];
static RAISE_DAMAGE_PIXELS: AtomicU64 = AtomicU64::new(0);
static OUTPUT_REQUIRED_COMMITS: AtomicU64 = AtomicU64::new(0);
static HIDDEN_PRESENTS: AtomicU64 = AtomicU64::new(0);
static HIDDEN_CONTENT_RETAINED: AtomicBool = AtomicBool::new(false);
static PHASE_ONE_READY: AtomicBool = AtomicBool::new(false);
static PHASE_TWO_READY: AtomicBool = AtomicBool::new(false);
static SCENARIO_COMPLETE: AtomicBool = AtomicBool::new(false);

static OUTPUT_COMMITS: AtomicU64 = AtomicU64::new(0);
static OUTPUT_ERRORS: AtomicU64 = AtomicU64::new(0);
static OUTPUT_SLOTS: [AtomicU64; OUTPUT_COMMIT_COUNT] =
    [const { AtomicU64::new(0) }; OUTPUT_COMMIT_COUNT];
static OUTPUT_ALLOCATION_GENERATIONS: [AtomicU64; OUTPUT_COMMIT_COUNT] =
    [const { AtomicU64::new(0) }; OUTPUT_COMMIT_COUNT];
static OUTPUT_WRITE_GENERATIONS: [AtomicU64; OUTPUT_COMMIT_COUNT] =
    [const { AtomicU64::new(0) }; OUTPUT_COMMIT_COUNT];
static OUTPUT_FRAME_IDS: [AtomicU64; OUTPUT_COMMIT_COUNT] =
    [const { AtomicU64::new(0) }; OUTPUT_COMMIT_COUNT];

static INPUT_ROUTE_COMMITTED: [AtomicBool; INPUT_ROUTE_COUNT] =
    [const { AtomicBool::new(false) }; INPUT_ROUTE_COUNT];
static INPUT_ROUTES: AtomicU64 = AtomicU64::new(0);
static CAPTURED_ROUTES: AtomicU64 = AtomicU64::new(0);
static SIGNED_OUTSIDE_ROUTES: AtomicU64 = AtomicU64::new(0);
static CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);
static CAPTURE_SLOT: AtomicU64 = AtomicU64::new(TOP_NONE);
static CAPTURE_CLEARS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowCommandCounts {
    pub create: u64,
    pub present: u64,
    pub raise: u64,
    pub destroy: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowEventCounts {
    pub created: u64,
    pub presented: u64,
    pub raised: u64,
    pub input: u64,
    pub destroyed: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowInputRouteSnapshot {
    pub receiver_role: UiTraceRole,
    pub command_sequence: u64,
    pub scene_frame: u32,
    pub window_id: WindowId,
    pub global_x: u16,
    pub global_y: u16,
    pub local_x: i32,
    pub local_y: i32,
    pub pressed: bool,
    pub captured: bool,
    pub focus_generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowTracePhase {
    Empty,
    LauncherCreated,
    LauncherPresented,
    AppCreated,
    AppPresented,
    HiddenDamageRetained,
    AppCaptureComplete,
    LauncherDamagePresented,
    AppDamagePresented,
    LauncherRaised,
    LauncherCaptureComplete,
    Complete,
    Teardown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowTraceSnapshot {
    pub traces: u64,
    pub errors: u64,
    pub owner_errors: u64,
    pub sequence_errors: u64,
    pub payload_errors: u64,
    pub event_errors: u64,
    pub scene_errors: u64,
    pub phase_errors: u64,
    pub commands: u64,
    pub command_counts: WindowCommandCounts,
    pub commands_by_client: [u64; CLIENT_COUNT],
    pub last_command_sequences: [u64; CLIENT_COUNT],
    pub command_masks: [u64; CLIENT_COUNT],
    pub events: u64,
    pub event_counts: WindowEventCounts,
    pub events_by_client: [u64; CLIENT_COUNT],
    pub last_scene_frames: [u32; CLIENT_COUNT],
    pub acknowledged_command_masks: [u64; CLIENT_COUNT],
    pub window_ids: [Option<WindowId>; CLIENT_COUNT],
    pub live_windows: u8,
    /// Bottom-to-top order; absent entries are `None` after teardown.
    pub z_order: [Option<u8>; CLIENT_COUNT],
    pub z_order_changes: u64,
    pub content_presents: [u64; CLIENT_COUNT],
    pub last_client_frames: [u32; CLIENT_COUNT],
    pub last_colors: [u32; CLIENT_COUNT],
    pub content_damage_pixels: [u64; CLIENT_COUNT],
    pub visible_damage_pixels: [u64; CLIENT_COUNT],
    pub occluded_damage_pixels: [u64; CLIENT_COUNT],
    pub raise_damage_pixels: u64,
    pub output_required_commits: u64,
    pub hidden_presents: u64,
    pub hidden_content_retained: bool,
    pub output_commits: u64,
    pub output_errors: u64,
    pub output_slots: [Option<u8>; OUTPUT_COMMIT_COUNT],
    pub output_allocation_generations: [u32; OUTPUT_COMMIT_COUNT],
    pub output_generations: [u64; OUTPUT_COMMIT_COUNT],
    pub output_frame_ids: [u32; OUTPUT_COMMIT_COUNT],
    pub input_route_count: u8,
    pub input_routes: [Option<WindowInputRouteSnapshot>; INPUT_ROUTE_COUNT],
    pub captured_routes: u64,
    pub signed_outside_routes: u64,
    pub capture_active: bool,
    pub capture_owner: Option<u8>,
    pub capture_clears: u64,
    pub phase: WindowTracePhase,
    pub phase_one_ready: bool,
    pub phase_one_complete: bool,
    pub phase_two_ready: bool,
    pub phase_two_complete: bool,
    pub scenario_complete: bool,
    pub final_complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RejectKind {
    Owner,
    Sequence,
    Payload,
    Event,
    Scene,
    Phase,
}

/// Records one authenticated, successfully delivered window-protocol read.
///
/// Returns `true` only when the trace advanced the strict M41 transcript.
/// Rejection is deliberately transactional apart from the error ledgers.
pub fn record(trace: WindowChannelTrace) -> bool {
    match trace {
        WindowChannelTrace::Command {
            sender_role,
            sequence,
            opcode,
            window_id,
            bounds,
            damage,
            frame_id,
            color,
        } => record_command(
            sender_role,
            sequence,
            opcode,
            window_id,
            bounds,
            damage,
            frame_id,
            color,
        ),
        WindowChannelTrace::Created {
            sender_role,
            receiver_role,
            command_sequence,
            scene_frame,
            window_id,
            bounds,
        } => record_created(
            sender_role,
            receiver_role,
            command_sequence,
            scene_frame,
            window_id,
            bounds,
        ),
        WindowChannelTrace::Presented {
            sender_role,
            receiver_role,
            command_sequence,
            scene_frame,
            window_id,
            frame_id,
        } => record_presented(
            sender_role,
            receiver_role,
            command_sequence,
            scene_frame,
            window_id,
            frame_id,
        ),
        WindowChannelTrace::Raised {
            sender_role,
            receiver_role,
            command_sequence,
            scene_frame,
            window_id,
        } => record_raised(
            sender_role,
            receiver_role,
            command_sequence,
            scene_frame,
            window_id,
        ),
        WindowChannelTrace::InputRoute {
            sender_role,
            receiver_role,
            command_sequence,
            scene_frame,
            window_id,
            global_x,
            global_y,
            local_x,
            local_y,
            pressed,
            captured,
            focus_generation,
        } => record_input(
            sender_role,
            receiver_role,
            command_sequence,
            scene_frame,
            window_id,
            global_x,
            global_y,
            local_x,
            local_y,
            pressed,
            captured,
            focus_generation,
        ),
        WindowChannelTrace::Destroyed {
            sender_role,
            receiver_role,
            command_sequence,
            scene_frame,
            window_id,
        } => record_destroyed(
            sender_role,
            receiver_role,
            command_sequence,
            scene_frame,
            window_id,
        ),
    }
}

/// Records one successful kernel scanout commit made on behalf of the M41
/// userspace compositor. The fixed ABABAB schedule is checked before any
/// output evidence advances.
pub fn record_output_commit(
    slot: usize,
    allocation_generation: u32,
    write_generation: u64,
    frame_id: u32,
) -> bool {
    let ordinal =
        usize::try_from(OUTPUT_COMMITS.load(Ordering::Acquire)).unwrap_or(OUTPUT_COMMIT_COUNT);
    let phase_ready = match ordinal {
        0 => command_pending(LAUNCHER, 2),
        1 => command_pending(APP, 2) && command_seen(LAUNCHER, 2) && live(LAUNCHER),
        2 => command_pending(LAUNCHER, 4) && PHASE_ONE_READY.load(Ordering::Acquire),
        3 => command_pending(APP, 3) && command_seen(LAUNCHER, 4),
        4 => command_pending(LAUNCHER, 5) && command_seen(APP, 3),
        5 => command_pending(APP, 4) && PHASE_TWO_READY.load(Ordering::Acquire),
        _ => false,
    };
    if ordinal >= OUTPUT_COMMIT_COUNT
        || slot != EXPECTED_OUTPUT_SLOTS[ordinal]
        || allocation_generation != 1
        || write_generation != EXPECTED_OUTPUT_WRITE_GENERATIONS[ordinal]
        || frame_id != u32::try_from(ordinal + 1).unwrap_or(u32::MAX)
        || !phase_ready
    {
        OUTPUT_ERRORS.fetch_add(1, Ordering::Relaxed);
        ERRORS.fetch_add(1, Ordering::Relaxed);
        return false;
    }

    OUTPUT_SLOTS[ordinal].store(
        u64::try_from(slot + 1).unwrap_or(u64::MAX),
        Ordering::Release,
    );
    OUTPUT_ALLOCATION_GENERATIONS[ordinal]
        .store(u64::from(allocation_generation), Ordering::Release);
    OUTPUT_WRITE_GENERATIONS[ordinal].store(write_generation, Ordering::Release);
    OUTPUT_FRAME_IDS[ordinal].store(u64::from(frame_id), Ordering::Release);
    OUTPUT_COMMITS.store(
        u64::try_from(ordinal + 1).unwrap_or(u64::MAX),
        Ordering::Release,
    );
    true
}

/// Records a BWC1/BWE1-shaped payload that failed authenticated decoding.
/// Direction/ownership failures are kept distinct from malformed canonical
/// payloads, while both fail the M41 transcript closed.
pub fn record_invalid_wire(owner_error: bool) -> bool {
    reject(if owner_error {
        RejectKind::Owner
    } else {
        RejectKind::Payload
    })
}

#[allow(clippy::too_many_arguments)]
fn record_command(
    sender_role: UiTraceRole,
    sequence: u64,
    opcode: WindowCommandOpcode,
    window_id: Option<WindowId>,
    bounds: Option<ShellRect>,
    damage: Option<ShellRect>,
    frame_id: Option<u32>,
    color: Option<u32>,
) -> bool {
    let Some(client) = client_index(sender_role) else {
        return reject(RejectKind::Owner);
    };
    let expected_sequence = LAST_COMMAND_SEQUENCE[client]
        .load(Ordering::Acquire)
        .checked_add(1);
    if expected_sequence != Some(sequence) || sequence == 0 || sequence >= u64::BITS.into() {
        return reject(RejectKind::Sequence);
    }

    let valid = match (client, sequence, opcode) {
        (LAUNCHER, 1, WindowCommandOpcode::Create) => {
            command_is_create(window_id, bounds, damage, frame_id, color, LAUNCHER_BOUNDS)
        }
        (LAUNCHER, 2, WindowCommandOpcode::Present) => {
            acked(LAUNCHER, 1)
                && command_is_present(
                    window_id,
                    damage,
                    frame_id,
                    color,
                    LAUNCHER_TOKEN,
                    LAUNCHER_BOUNDS,
                    1,
                    COLOR_LAUNCHER_BASE,
                )
        }
        (APP, 1, WindowCommandOpcode::Create) => {
            output_commits_at_least(1)
                && command_seen(LAUNCHER, 2)
                && live(LAUNCHER)
                && command_is_create(window_id, bounds, damage, frame_id, color, APP_BOUNDS)
        }
        (APP, 2, WindowCommandOpcode::Present) => {
            acked(APP, 1)
                && command_is_present(
                    window_id,
                    damage,
                    frame_id,
                    color,
                    APP_TOKEN,
                    APP_LOCAL_BOUNDS,
                    1,
                    COLOR_APP_BASE,
                )
        }
        (LAUNCHER, 3, WindowCommandOpcode::Present) => {
            acked(LAUNCHER, 2)
                && output_commits_at_least(2)
                && command_seen(APP, 2)
                && command_is_present(
                    window_id,
                    damage,
                    frame_id,
                    color,
                    LAUNCHER_TOKEN,
                    LAUNCHER_HIDDEN_DAMAGE,
                    2,
                    COLOR_LAUNCHER_HIDDEN,
                )
        }
        (LAUNCHER, 4, WindowCommandOpcode::Present) => {
            PHASE_ONE_READY.load(Ordering::Acquire)
                && output_commits_at_least(2)
                && command_is_present(
                    window_id,
                    damage,
                    frame_id,
                    color,
                    LAUNCHER_TOKEN,
                    LAUNCHER_PARTIAL_DAMAGE,
                    3,
                    COLOR_LAUNCHER_PARTIAL,
                )
        }
        (APP, 3, WindowCommandOpcode::Present) => {
            acked(APP, 2)
                && output_commits_at_least(3)
                && command_seen(LAUNCHER, 4)
                && INPUT_ROUTE_COMMITTED[2].load(Ordering::Acquire)
                && command_is_present(
                    window_id,
                    damage,
                    frame_id,
                    color,
                    APP_TOKEN,
                    APP_DAMAGE,
                    2,
                    COLOR_APP_DAMAGE,
                )
        }
        (LAUNCHER, 5, WindowCommandOpcode::Raise) => {
            acked(LAUNCHER, 2)
                && output_commits_at_least(4)
                && command_seen(APP, 3)
                && command_is_window_only(
                    window_id,
                    bounds,
                    damage,
                    frame_id,
                    color,
                    LAUNCHER_TOKEN,
                )
        }
        (APP, 4, WindowCommandOpcode::Raise) => {
            acked(APP, 2)
                && PHASE_TWO_READY.load(Ordering::Acquire)
                && output_commits_at_least(5)
                && INPUT_ROUTE_COMMITTED[3].load(Ordering::Acquire)
                && command_is_window_only(window_id, bounds, damage, frame_id, color, APP_TOKEN)
        }
        (LAUNCHER, 6, WindowCommandOpcode::Destroy) | (APP, 5, WindowCommandOpcode::Destroy) => {
            scenario_complete()
                && live(client)
                && command_is_window_only(
                    window_id,
                    bounds,
                    damage,
                    frame_id,
                    color,
                    expected_token(client),
                )
        }
        _ => return reject(RejectKind::Phase),
    };
    if !valid {
        return reject(RejectKind::Payload);
    }

    LAST_COMMAND_SEQUENCE[client].store(sequence, Ordering::Release);
    COMMAND_MASK[client].fetch_or(sequence_bit(sequence), Ordering::AcqRel);
    CLIENT_COMMANDS[client].fetch_add(1, Ordering::Relaxed);
    COMMANDS.fetch_add(1, Ordering::Relaxed);
    match opcode {
        WindowCommandOpcode::Create => CREATE_COMMANDS.fetch_add(1, Ordering::Relaxed),
        WindowCommandOpcode::Present => PRESENT_COMMANDS.fetch_add(1, Ordering::Relaxed),
        WindowCommandOpcode::Raise => RAISE_COMMANDS.fetch_add(1, Ordering::Relaxed),
        WindowCommandOpcode::Destroy => DESTROY_COMMANDS.fetch_add(1, Ordering::Relaxed),
    };
    accept()
}

fn record_created(
    sender_role: UiTraceRole,
    receiver_role: UiTraceRole,
    command_sequence: u64,
    scene_frame: u32,
    window_id: WindowId,
    bounds: ShellRect,
) -> bool {
    let Some(client) = authenticated_event_owner(sender_role, receiver_role, window_id) else {
        return reject(RejectKind::Owner);
    };
    let expected_scene = if client == LAUNCHER { 1 } else { 3 };
    if scene_frame != expected_scene {
        return reject(RejectKind::Scene);
    }
    if command_sequence != 1
        || !command_pending(client, command_sequence)
        || WINDOW_TOKENS[client].load(Ordering::Acquire) != 0
        || bounds != expected_bounds(client)
    {
        return reject(RejectKind::Event);
    }
    if (client == LAUNCHER && LIVE_MASK.load(Ordering::Acquire) != 0)
        || (client == APP
            && (!output_commits_at_least(1)
                || !command_seen(LAUNCHER, 2)
                || LIVE_MASK.load(Ordering::Acquire) != LIVE_LAUNCHER))
    {
        return reject(RejectKind::Phase);
    }

    commit_command_event(client, command_sequence, scene_frame);
    WINDOW_TOKENS[client].store(window_id.token(), Ordering::Release);
    LIVE_MASK.fetch_or(1_u64 << client, Ordering::AcqRel);
    set_top(client);
    CREATED_EVENTS.fetch_add(1, Ordering::Relaxed);
    accept_event(client)
}

fn record_presented(
    sender_role: UiTraceRole,
    receiver_role: UiTraceRole,
    command_sequence: u64,
    scene_frame: u32,
    window_id: WindowId,
    frame_id: u32,
) -> bool {
    let Some(client) = authenticated_event_owner(sender_role, receiver_role, window_id) else {
        return reject(RejectKind::Owner);
    };
    let Some((expected_scene, expected_frame, content, visible, occluded, color, hidden)) =
        expected_present_event(client, command_sequence)
    else {
        return reject(RejectKind::Event);
    };
    if scene_frame != expected_scene {
        return reject(RejectKind::Scene);
    }
    if frame_id != expected_frame
        || !command_pending(client, command_sequence)
        || !present_phase_ready(client, command_sequence)
    {
        return reject(RejectKind::Event);
    }

    commit_command_event(client, command_sequence, scene_frame);
    CONTENT_PRESENTS[client].fetch_add(1, Ordering::Relaxed);
    LAST_CLIENT_FRAME[client].store(u64::from(frame_id), Ordering::Release);
    LAST_COLOR[client].store(u64::from(color), Ordering::Release);
    CONTENT_DAMAGE_PIXELS[client].fetch_add(content, Ordering::Relaxed);
    VISIBLE_DAMAGE_PIXELS[client].fetch_add(visible, Ordering::Relaxed);
    OCCLUDED_DAMAGE_PIXELS[client].fetch_add(occluded, Ordering::Relaxed);
    if hidden {
        HIDDEN_PRESENTS.fetch_add(1, Ordering::Relaxed);
        HIDDEN_CONTENT_RETAINED.store(true, Ordering::Release);
        if OUTPUT_COMMITS.load(Ordering::Acquire) == 2 {
            PHASE_ONE_READY.store(true, Ordering::Release);
        }
    } else {
        OUTPUT_REQUIRED_COMMITS.fetch_add(1, Ordering::Relaxed);
    }
    PRESENTED_EVENTS.fetch_add(1, Ordering::Relaxed);
    accept_event(client)
}

fn record_raised(
    sender_role: UiTraceRole,
    receiver_role: UiTraceRole,
    command_sequence: u64,
    scene_frame: u32,
    window_id: WindowId,
) -> bool {
    let Some(client) = authenticated_event_owner(sender_role, receiver_role, window_id) else {
        return reject(RejectKind::Owner);
    };
    let (expected_sequence, expected_scene, expected_previous_top) = if client == LAUNCHER {
        (5, 8, TOP_APP)
    } else {
        (4, 9, TOP_LAUNCHER)
    };
    if scene_frame != expected_scene {
        return reject(RejectKind::Scene);
    }
    if command_sequence != expected_sequence
        || !command_pending(client, command_sequence)
        || LIVE_MASK.load(Ordering::Acquire) != LIVE_BOTH
        || TOP_SLOT.load(Ordering::Acquire) != expected_previous_top
        || (client == LAUNCHER && (!output_commits_at_least(5) || !command_seen(APP, 3)))
        || (client == APP
            && (!PHASE_TWO_READY.load(Ordering::Acquire) || !output_commits_at_least(6)))
    {
        return reject(RejectKind::Event);
    }

    commit_command_event(client, command_sequence, scene_frame);
    set_top(client);
    RAISE_DAMAGE_PIXELS.fetch_add(OPAQUE_OVERLAP_PIXELS, Ordering::Relaxed);
    OUTPUT_REQUIRED_COMMITS.fetch_add(1, Ordering::Relaxed);
    RAISED_EVENTS.fetch_add(1, Ordering::Relaxed);
    if client == LAUNCHER && OUTPUT_COMMITS.load(Ordering::Acquire) == 5 {
        PHASE_TWO_READY.store(true, Ordering::Release);
    }
    let accepted = accept_event(client);
    latch_scenario_if_complete();
    accepted
}

#[allow(clippy::too_many_arguments)]
fn record_input(
    sender_role: UiTraceRole,
    receiver_role: UiTraceRole,
    command_sequence: u64,
    scene_frame: u32,
    window_id: WindowId,
    global_x: u16,
    global_y: u16,
    local_x: i32,
    local_y: i32,
    pressed: bool,
    captured: bool,
    focus_generation: u64,
) -> bool {
    let Some(client) = authenticated_event_owner(sender_role, receiver_role, window_id) else {
        return reject(RejectKind::Owner);
    };
    let route_index =
        usize::try_from(INPUT_ROUTES.load(Ordering::Acquire)).unwrap_or(INPUT_ROUTE_COUNT);
    let Some(expected) = expected_input_route(route_index) else {
        return reject(RejectKind::Phase);
    };
    if scene_frame != expected.scene_frame {
        return reject(RejectKind::Scene);
    }
    if client != expected.client
        || command_sequence != expected.command_sequence
        || window_id.token() != expected.window_token
        || global_x != expected.global_x
        || global_y != expected.global_y
        || local_x != expected.local_x
        || local_y != expected.local_y
        || pressed != expected.pressed
        || captured != expected.captured
        || focus_generation != expected.focus_generation
    {
        return reject(RejectKind::Event);
    }
    if !input_phase_ready(route_index) {
        return reject(RejectKind::Phase);
    }
    if !scene_is_monotonic(client, scene_frame) {
        return reject(RejectKind::Scene);
    }

    LAST_SCENE_FRAME[client].store(u64::from(scene_frame), Ordering::Release);
    INPUT_ROUTE_COMMITTED[route_index].store(true, Ordering::Release);
    INPUT_ROUTES.fetch_add(1, Ordering::Relaxed);
    if captured {
        CAPTURED_ROUTES.fetch_add(1, Ordering::Relaxed);
    }
    if local_x < 0 || local_y < 0 {
        SIGNED_OUTSIDE_ROUTES.fetch_add(1, Ordering::Relaxed);
    }
    if pressed {
        CAPTURE_ACTIVE.store(true, Ordering::Release);
        CAPTURE_SLOT.store(top_encoding(client), Ordering::Release);
    } else {
        CAPTURE_ACTIVE.store(false, Ordering::Release);
        CAPTURE_SLOT.store(TOP_NONE, Ordering::Release);
        CAPTURE_CLEARS.fetch_add(1, Ordering::Relaxed);
    }
    INPUT_EVENTS.fetch_add(1, Ordering::Relaxed);
    let accepted = accept_event(client);
    latch_scenario_if_complete();
    accepted
}

fn record_destroyed(
    sender_role: UiTraceRole,
    receiver_role: UiTraceRole,
    command_sequence: u64,
    scene_frame: u32,
    window_id: WindowId,
) -> bool {
    let Some(client) = authenticated_event_owner(sender_role, receiver_role, window_id) else {
        return reject(RejectKind::Owner);
    };
    let expected_sequence = if client == LAUNCHER { 6 } else { 5 };
    let expected_scene = 10_u32.saturating_add(
        u32::try_from(DESTROYED_EVENTS.load(Ordering::Acquire)).unwrap_or(u32::MAX),
    );
    if scene_frame != expected_scene {
        return reject(RejectKind::Scene);
    }
    if command_sequence != expected_sequence
        || !command_pending(client, command_sequence)
        || !scenario_complete()
        || !live(client)
    {
        return reject(RejectKind::Event);
    }

    commit_command_event(client, command_sequence, scene_frame);
    let remaining = LIVE_MASK.fetch_and(!(1_u64 << client), Ordering::AcqRel) & !(1_u64 << client);
    WINDOW_TOKENS[client].store(0, Ordering::Release);
    if TOP_SLOT.load(Ordering::Acquire) == top_encoding(client) {
        let next_top = if remaining & LIVE_APP != 0 {
            TOP_APP
        } else if remaining & LIVE_LAUNCHER != 0 {
            TOP_LAUNCHER
        } else {
            TOP_NONE
        };
        TOP_SLOT.store(next_top, Ordering::Release);
        Z_ORDER_CHANGES.fetch_add(1, Ordering::Relaxed);
    }
    DESTROYED_EVENTS.fetch_add(1, Ordering::Relaxed);
    accept_event(client)
}

fn command_is_create(
    window_id: Option<WindowId>,
    bounds: Option<ShellRect>,
    damage: Option<ShellRect>,
    frame_id: Option<u32>,
    color: Option<u32>,
    expected_bounds: ShellRect,
) -> bool {
    window_id.is_none()
        && bounds == Some(expected_bounds)
        && damage.is_none()
        && frame_id.is_none()
        && color.is_none()
}

#[allow(clippy::too_many_arguments)]
fn command_is_present(
    window_id: Option<WindowId>,
    damage: Option<ShellRect>,
    frame_id: Option<u32>,
    color: Option<u32>,
    expected_token: u64,
    expected_damage: ShellRect,
    expected_frame: u32,
    expected_color: u32,
) -> bool {
    window_id.is_some_and(|id| id.token() == expected_token)
        && damage == Some(expected_damage)
        && frame_id == Some(expected_frame)
        && color == Some(expected_color)
}

fn command_is_window_only(
    window_id: Option<WindowId>,
    bounds: Option<ShellRect>,
    damage: Option<ShellRect>,
    frame_id: Option<u32>,
    color: Option<u32>,
    expected_token: u64,
) -> bool {
    window_id.is_some_and(|id| id.token() == expected_token)
        && bounds.is_none()
        && damage.is_none()
        && frame_id.is_none()
        && color.is_none()
}

fn expected_present_event(
    client: usize,
    command_sequence: u64,
) -> Option<(u32, u32, u64, u64, u64, u32, bool)> {
    match (client, command_sequence) {
        (LAUNCHER, 2) => Some((
            2,
            1,
            LAUNCHER_FULL_PIXELS,
            LAUNCHER_FULL_PIXELS,
            0,
            COLOR_LAUNCHER_BASE,
            false,
        )),
        (APP, 2) => Some((
            4,
            1,
            APP_FULL_PIXELS,
            APP_FULL_PIXELS,
            0,
            COLOR_APP_BASE,
            false,
        )),
        (LAUNCHER, 3) => Some((
            5,
            2,
            LAUNCHER_HIDDEN_PIXELS,
            0,
            LAUNCHER_HIDDEN_PIXELS,
            COLOR_LAUNCHER_HIDDEN,
            true,
        )),
        (LAUNCHER, 4) => Some((
            6,
            3,
            LAUNCHER_PARTIAL_PIXELS,
            LAUNCHER_PARTIAL_PIXELS - LAUNCHER_PARTIAL_OCCLUDED_PIXELS,
            LAUNCHER_PARTIAL_OCCLUDED_PIXELS,
            COLOR_LAUNCHER_PARTIAL,
            false,
        )),
        (APP, 3) => Some((
            7,
            2,
            APP_DAMAGE_PIXELS,
            APP_DAMAGE_PIXELS,
            0,
            COLOR_APP_DAMAGE,
            false,
        )),
        _ => None,
    }
}

fn present_phase_ready(client: usize, sequence: u64) -> bool {
    match (client, sequence) {
        (LAUNCHER, 2) => acked(LAUNCHER, 1) && live(LAUNCHER) && output_commits_at_least(1),
        (APP, 2) => acked(APP, 1) && live(APP) && output_commits_at_least(2),
        (LAUNCHER, 3) => {
            acked(LAUNCHER, 2) && command_seen(APP, 2) && live(APP) && output_commits_at_least(2)
        }
        (LAUNCHER, 4) => {
            PHASE_ONE_READY.load(Ordering::Acquire) && live(APP) && output_commits_at_least(3)
        }
        (APP, 3) => {
            acked(APP, 2)
                && command_seen(LAUNCHER, 4)
                && live(LAUNCHER)
                && output_commits_at_least(4)
        }
        _ => false,
    }
}

#[derive(Clone, Copy)]
struct ExpectedInputRoute {
    client: usize,
    command_sequence: u64,
    scene_frame: u32,
    window_token: u64,
    global_x: u16,
    global_y: u16,
    local_x: i32,
    local_y: i32,
    pressed: bool,
    captured: bool,
    focus_generation: u64,
}

const EXPECTED_INPUT_ROUTES: [ExpectedInputRoute; INPUT_ROUTE_COUNT] = [
    ExpectedInputRoute {
        client: APP,
        command_sequence: 2,
        scene_frame: 5,
        window_token: APP_TOKEN,
        global_x: 80,
        global_y: 120,
        local_x: 32,
        local_y: 40,
        pressed: true,
        captured: true,
        focus_generation: 1,
    },
    ExpectedInputRoute {
        client: APP,
        command_sequence: 2,
        scene_frame: 5,
        window_token: APP_TOKEN,
        global_x: 16,
        global_y: 32,
        local_x: -32,
        local_y: -48,
        pressed: true,
        captured: true,
        focus_generation: 1,
    },
    ExpectedInputRoute {
        client: APP,
        command_sequence: 2,
        scene_frame: 5,
        window_token: APP_TOKEN,
        global_x: 16,
        global_y: 32,
        local_x: -32,
        local_y: -48,
        pressed: false,
        captured: true,
        focus_generation: 1,
    },
    ExpectedInputRoute {
        client: LAUNCHER,
        command_sequence: 5,
        scene_frame: 8,
        window_token: LAUNCHER_TOKEN,
        global_x: 80,
        global_y: 120,
        local_x: 80,
        local_y: 120,
        pressed: true,
        captured: true,
        focus_generation: 2,
    },
    ExpectedInputRoute {
        client: LAUNCHER,
        command_sequence: 5,
        scene_frame: 8,
        window_token: LAUNCHER_TOKEN,
        global_x: 80,
        global_y: 120,
        local_x: 80,
        local_y: 120,
        pressed: false,
        captured: true,
        focus_generation: 2,
    },
];

fn expected_input_route(index: usize) -> Option<ExpectedInputRoute> {
    EXPECTED_INPUT_ROUTES.get(index).copied()
}

fn input_phase_ready(index: usize) -> bool {
    match index {
        0 => acked(LAUNCHER, 3) && top_is(APP) && !CAPTURE_ACTIVE.load(Ordering::Acquire),
        1 => CAPTURE_ACTIVE.load(Ordering::Acquire) && capture_is(APP),
        2 => CAPTURE_ACTIVE.load(Ordering::Acquire) && capture_is(APP),
        3 => acked(LAUNCHER, 5) && top_is(LAUNCHER) && !CAPTURE_ACTIVE.load(Ordering::Acquire),
        4 => CAPTURE_ACTIVE.load(Ordering::Acquire) && capture_is(LAUNCHER),
        _ => false,
    }
}

fn authenticated_event_owner(
    sender_role: UiTraceRole,
    receiver_role: UiTraceRole,
    window_id: WindowId,
) -> Option<usize> {
    if sender_role != UiTraceRole::SurfaceServer {
        return None;
    }
    let client = client_index(receiver_role)?;
    (window_id.slot() == client
        && window_id.generation() == 1
        && window_id.token() == expected_token(client))
    .then_some(client)
}

fn client_index(role: UiTraceRole) -> Option<usize> {
    match role {
        UiTraceRole::Launcher => Some(LAUNCHER),
        UiTraceRole::App => Some(APP),
        UiTraceRole::SurfaceServer => None,
    }
}

const fn expected_token(client: usize) -> u64 {
    if client == LAUNCHER {
        LAUNCHER_TOKEN
    } else {
        APP_TOKEN
    }
}

const fn expected_bounds(client: usize) -> ShellRect {
    if client == LAUNCHER {
        LAUNCHER_BOUNDS
    } else {
        APP_BOUNDS
    }
}

fn command_pending(client: usize, sequence: u64) -> bool {
    let bit = sequence_bit(sequence);
    COMMAND_MASK[client].load(Ordering::Acquire) & bit != 0
        && ACK_MASK[client].load(Ordering::Acquire) & bit == 0
}

fn command_seen(client: usize, sequence: u64) -> bool {
    COMMAND_MASK[client].load(Ordering::Acquire) & sequence_bit(sequence) != 0
}

fn output_commits_at_least(count: u64) -> bool {
    OUTPUT_COMMITS.load(Ordering::Acquire) >= count
}

fn acked(client: usize, sequence: u64) -> bool {
    ACK_MASK[client].load(Ordering::Acquire) & sequence_bit(sequence) != 0
}

fn commit_command_event(client: usize, sequence: u64, scene_frame: u32) {
    debug_assert!(command_pending(client, sequence));
    debug_assert!(scene_is_monotonic(client, scene_frame));
    ACK_MASK[client].fetch_or(sequence_bit(sequence), Ordering::AcqRel);
    LAST_SCENE_FRAME[client].store(u64::from(scene_frame), Ordering::Release);
}

fn scene_is_monotonic(client: usize, scene_frame: u32) -> bool {
    u64::from(scene_frame) >= LAST_SCENE_FRAME[client].load(Ordering::Acquire)
}

fn live(client: usize) -> bool {
    LIVE_MASK.load(Ordering::Acquire) & (1_u64 << client) != 0
}

fn top_is(client: usize) -> bool {
    TOP_SLOT.load(Ordering::Acquire) == top_encoding(client)
}

fn capture_is(client: usize) -> bool {
    CAPTURE_SLOT.load(Ordering::Acquire) == top_encoding(client)
}

const fn top_encoding(client: usize) -> u64 {
    if client == LAUNCHER {
        TOP_LAUNCHER
    } else {
        TOP_APP
    }
}

fn set_top(client: usize) {
    let encoded = top_encoding(client);
    if TOP_SLOT.swap(encoded, Ordering::AcqRel) != encoded {
        Z_ORDER_CHANGES.fetch_add(1, Ordering::Relaxed);
    }
}

fn accept_event(client: usize) -> bool {
    CLIENT_EVENTS[client].fetch_add(1, Ordering::Relaxed);
    EVENTS.fetch_add(1, Ordering::Relaxed);
    accept()
}

fn accept() -> bool {
    TRACES.fetch_add(1, Ordering::Relaxed);
    true
}

fn reject(kind: RejectKind) -> bool {
    ERRORS.fetch_add(1, Ordering::Relaxed);
    match kind {
        RejectKind::Owner => OWNER_ERRORS.fetch_add(1, Ordering::Relaxed),
        RejectKind::Sequence => SEQUENCE_ERRORS.fetch_add(1, Ordering::Relaxed),
        RejectKind::Payload => PAYLOAD_ERRORS.fetch_add(1, Ordering::Relaxed),
        RejectKind::Event => EVENT_ERRORS.fetch_add(1, Ordering::Relaxed),
        RejectKind::Scene => SCENE_ERRORS.fetch_add(1, Ordering::Relaxed),
        RejectKind::Phase => PHASE_ERRORS.fetch_add(1, Ordering::Relaxed),
    };
    false
}

fn scenario_complete() -> bool {
    SCENARIO_COMPLETE.load(Ordering::Acquire)
}

fn latch_scenario_if_complete() {
    if !SCENARIO_COMPLETE.load(Ordering::Acquire) && scenario_witness_complete() {
        SCENARIO_COMPLETE.store(true, Ordering::Release);
    }
}

fn scenario_witness_complete() -> bool {
    ERRORS.load(Ordering::Acquire) == 0
        && COMMANDS.load(Ordering::Acquire) >= EXPECTED_COMMANDS
        && CREATE_COMMANDS.load(Ordering::Acquire) == 2
        && PRESENT_COMMANDS.load(Ordering::Acquire) == 5
        && RAISE_COMMANDS.load(Ordering::Acquire) == 2
        && ACK_MASK[LAUNCHER].load(Ordering::Acquire) & LAUNCHER_FINAL_ACK_MASK
            == LAUNCHER_FINAL_ACK_MASK
        && ACK_MASK[APP].load(Ordering::Acquire) & APP_FINAL_ACK_MASK == APP_FINAL_ACK_MASK
        && CREATED_EVENTS.load(Ordering::Acquire) == 2
        && PRESENTED_EVENTS.load(Ordering::Acquire) == 5
        && RAISED_EVENTS.load(Ordering::Acquire) == 2
        && INPUT_EVENTS.load(Ordering::Acquire) == EXPECTED_INPUT_EVENTS
        && EVENTS.load(Ordering::Acquire) == EXPECTED_COMMAND_EVENTS + EXPECTED_INPUT_EVENTS
        && INPUT_ROUTES.load(Ordering::Acquire) == EXPECTED_INPUT_EVENTS
        && OUTPUT_COMMITS.load(Ordering::Acquire) == OUTPUT_COMMIT_COUNT as u64
        && OUTPUT_ERRORS.load(Ordering::Acquire) == 0
        && !CAPTURE_ACTIVE.load(Ordering::Acquire)
        && TOP_SLOT.load(Ordering::Acquire) == TOP_APP
}

pub fn snapshot() -> WindowTraceSnapshot {
    let live_mask = LIVE_MASK.load(Ordering::Acquire);
    let top = TOP_SLOT.load(Ordering::Acquire);
    let scenario_complete = scenario_complete();
    let destroyed = DESTROYED_EVENTS.load(Ordering::Acquire);
    let routes = INPUT_ROUTES.load(Ordering::Acquire);
    let z_order = z_order(live_mask, top);
    WindowTraceSnapshot {
        traces: TRACES.load(Ordering::Acquire),
        errors: ERRORS.load(Ordering::Acquire),
        owner_errors: OWNER_ERRORS.load(Ordering::Acquire),
        sequence_errors: SEQUENCE_ERRORS.load(Ordering::Acquire),
        payload_errors: PAYLOAD_ERRORS.load(Ordering::Acquire),
        event_errors: EVENT_ERRORS.load(Ordering::Acquire),
        scene_errors: SCENE_ERRORS.load(Ordering::Acquire),
        phase_errors: PHASE_ERRORS.load(Ordering::Acquire),
        commands: COMMANDS.load(Ordering::Acquire),
        command_counts: WindowCommandCounts {
            create: CREATE_COMMANDS.load(Ordering::Acquire),
            present: PRESENT_COMMANDS.load(Ordering::Acquire),
            raise: RAISE_COMMANDS.load(Ordering::Acquire),
            destroy: DESTROY_COMMANDS.load(Ordering::Acquire),
        },
        commands_by_client: core::array::from_fn(|client| {
            CLIENT_COMMANDS[client].load(Ordering::Acquire)
        }),
        last_command_sequences: core::array::from_fn(|client| {
            LAST_COMMAND_SEQUENCE[client].load(Ordering::Acquire)
        }),
        command_masks: core::array::from_fn(|client| COMMAND_MASK[client].load(Ordering::Acquire)),
        events: EVENTS.load(Ordering::Acquire),
        event_counts: WindowEventCounts {
            created: CREATED_EVENTS.load(Ordering::Acquire),
            presented: PRESENTED_EVENTS.load(Ordering::Acquire),
            raised: RAISED_EVENTS.load(Ordering::Acquire),
            input: INPUT_EVENTS.load(Ordering::Acquire),
            destroyed,
        },
        events_by_client: core::array::from_fn(|client| {
            CLIENT_EVENTS[client].load(Ordering::Acquire)
        }),
        last_scene_frames: core::array::from_fn(|client| {
            u32::try_from(LAST_SCENE_FRAME[client].load(Ordering::Acquire)).unwrap_or(u32::MAX)
        }),
        acknowledged_command_masks: core::array::from_fn(|client| {
            ACK_MASK[client].load(Ordering::Acquire)
        }),
        window_ids: core::array::from_fn(|client| {
            let token = WINDOW_TOKENS[client].load(Ordering::Acquire);
            (token != 0)
                .then(|| WindowId::from_token(token).ok())
                .flatten()
        }),
        live_windows: u8::try_from(live_mask.count_ones()).unwrap_or(u8::MAX),
        z_order,
        z_order_changes: Z_ORDER_CHANGES.load(Ordering::Acquire),
        content_presents: core::array::from_fn(|client| {
            CONTENT_PRESENTS[client].load(Ordering::Acquire)
        }),
        last_client_frames: core::array::from_fn(|client| {
            u32::try_from(LAST_CLIENT_FRAME[client].load(Ordering::Acquire)).unwrap_or(u32::MAX)
        }),
        last_colors: core::array::from_fn(|client| {
            u32::try_from(LAST_COLOR[client].load(Ordering::Acquire)).unwrap_or(u32::MAX)
        }),
        content_damage_pixels: core::array::from_fn(|client| {
            CONTENT_DAMAGE_PIXELS[client].load(Ordering::Acquire)
        }),
        visible_damage_pixels: core::array::from_fn(|client| {
            VISIBLE_DAMAGE_PIXELS[client].load(Ordering::Acquire)
        }),
        occluded_damage_pixels: core::array::from_fn(|client| {
            OCCLUDED_DAMAGE_PIXELS[client].load(Ordering::Acquire)
        }),
        raise_damage_pixels: RAISE_DAMAGE_PIXELS.load(Ordering::Acquire),
        output_required_commits: OUTPUT_REQUIRED_COMMITS.load(Ordering::Acquire),
        hidden_presents: HIDDEN_PRESENTS.load(Ordering::Acquire),
        hidden_content_retained: HIDDEN_CONTENT_RETAINED.load(Ordering::Acquire),
        output_commits: OUTPUT_COMMITS.load(Ordering::Acquire),
        output_errors: OUTPUT_ERRORS.load(Ordering::Acquire),
        output_slots: core::array::from_fn(|index| {
            let encoded = OUTPUT_SLOTS[index].load(Ordering::Acquire);
            (encoded != 0).then(|| u8::try_from(encoded - 1).unwrap_or(u8::MAX))
        }),
        output_allocation_generations: core::array::from_fn(|index| {
            u32::try_from(OUTPUT_ALLOCATION_GENERATIONS[index].load(Ordering::Acquire))
                .unwrap_or(u32::MAX)
        }),
        output_generations: core::array::from_fn(|index| {
            OUTPUT_WRITE_GENERATIONS[index].load(Ordering::Acquire)
        }),
        output_frame_ids: core::array::from_fn(|index| {
            u32::try_from(OUTPUT_FRAME_IDS[index].load(Ordering::Acquire)).unwrap_or(u32::MAX)
        }),
        input_route_count: u8::try_from(routes).unwrap_or(u8::MAX),
        input_routes: core::array::from_fn(|index| {
            INPUT_ROUTE_COMMITTED[index]
                .load(Ordering::Acquire)
                .then(|| input_route_snapshot(EXPECTED_INPUT_ROUTES[index]))
        }),
        captured_routes: CAPTURED_ROUTES.load(Ordering::Acquire),
        signed_outside_routes: SIGNED_OUTSIDE_ROUTES.load(Ordering::Acquire),
        capture_active: CAPTURE_ACTIVE.load(Ordering::Acquire),
        capture_owner: decode_top(CAPTURE_SLOT.load(Ordering::Acquire)),
        capture_clears: CAPTURE_CLEARS.load(Ordering::Acquire),
        phase: phase(destroyed),
        phase_one_ready: PHASE_ONE_READY.load(Ordering::Acquire),
        phase_one_complete: routes >= 3,
        phase_two_ready: PHASE_TWO_READY.load(Ordering::Acquire),
        phase_two_complete: routes == EXPECTED_INPUT_EVENTS,
        scenario_complete,
        final_complete: scenario_complete
            && destroyed == 0
            && live_mask == LIVE_BOTH
            && z_order == [Some(0), Some(1)],
    }
}

fn input_route_snapshot(expected: ExpectedInputRoute) -> WindowInputRouteSnapshot {
    WindowInputRouteSnapshot {
        receiver_role: if expected.client == LAUNCHER {
            UiTraceRole::Launcher
        } else {
            UiTraceRole::App
        },
        command_sequence: expected.command_sequence,
        scene_frame: expected.scene_frame,
        window_id: WindowId::from_token(expected.window_token)
            .unwrap_or_else(|_| unreachable!("constant M41 window token is canonical")),
        global_x: expected.global_x,
        global_y: expected.global_y,
        local_x: expected.local_x,
        local_y: expected.local_y,
        pressed: expected.pressed,
        captured: expected.captured,
        focus_generation: expected.focus_generation,
    }
}

fn z_order(live_mask: u64, top: u64) -> [Option<u8>; CLIENT_COUNT] {
    match (live_mask, top) {
        (LIVE_BOTH, TOP_LAUNCHER) => [Some(1), Some(0)],
        (LIVE_BOTH, TOP_APP) => [Some(0), Some(1)],
        (LIVE_LAUNCHER, TOP_LAUNCHER) => [Some(0), None],
        (LIVE_APP, TOP_APP) => [Some(1), None],
        _ => [None, None],
    }
}

fn decode_top(encoded: u64) -> Option<u8> {
    match encoded {
        TOP_LAUNCHER => Some(0),
        TOP_APP => Some(1),
        _ => None,
    }
}

fn phase(destroyed: u64) -> WindowTracePhase {
    if destroyed != 0 {
        return WindowTracePhase::Teardown;
    }
    let routes = INPUT_ROUTES.load(Ordering::Acquire);
    if scenario_complete() {
        WindowTracePhase::Complete
    } else if routes == EXPECTED_INPUT_EVENTS {
        WindowTracePhase::LauncherCaptureComplete
    } else if acked(LAUNCHER, 5) {
        WindowTracePhase::LauncherRaised
    } else if acked(APP, 3) {
        WindowTracePhase::AppDamagePresented
    } else if acked(LAUNCHER, 4) {
        WindowTracePhase::LauncherDamagePresented
    } else if routes >= 3 {
        WindowTracePhase::AppCaptureComplete
    } else if acked(LAUNCHER, 3) {
        WindowTracePhase::HiddenDamageRetained
    } else if acked(APP, 2) {
        WindowTracePhase::AppPresented
    } else if acked(APP, 1) {
        WindowTracePhase::AppCreated
    } else if acked(LAUNCHER, 2) {
        WindowTracePhase::LauncherPresented
    } else if acked(LAUNCHER, 1) {
        WindowTracePhase::LauncherCreated
    } else {
        WindowTracePhase::Empty
    }
}

#[cfg(test)]
fn reset() {
    for atomic in [
        &TRACES,
        &ERRORS,
        &OWNER_ERRORS,
        &SEQUENCE_ERRORS,
        &PAYLOAD_ERRORS,
        &EVENT_ERRORS,
        &SCENE_ERRORS,
        &PHASE_ERRORS,
        &COMMANDS,
        &CREATE_COMMANDS,
        &PRESENT_COMMANDS,
        &RAISE_COMMANDS,
        &DESTROY_COMMANDS,
        &EVENTS,
        &CREATED_EVENTS,
        &PRESENTED_EVENTS,
        &RAISED_EVENTS,
        &INPUT_EVENTS,
        &DESTROYED_EVENTS,
        &LIVE_MASK,
        &Z_ORDER_CHANGES,
        &RAISE_DAMAGE_PIXELS,
        &OUTPUT_REQUIRED_COMMITS,
        &HIDDEN_PRESENTS,
        &OUTPUT_COMMITS,
        &OUTPUT_ERRORS,
        &INPUT_ROUTES,
        &CAPTURED_ROUTES,
        &SIGNED_OUTSIDE_ROUTES,
        &CAPTURE_CLEARS,
    ] {
        atomic.store(0, Ordering::Relaxed);
    }
    TOP_SLOT.store(TOP_NONE, Ordering::Relaxed);
    CAPTURE_SLOT.store(TOP_NONE, Ordering::Relaxed);
    CAPTURE_ACTIVE.store(false, Ordering::Relaxed);
    HIDDEN_CONTENT_RETAINED.store(false, Ordering::Relaxed);
    PHASE_ONE_READY.store(false, Ordering::Relaxed);
    PHASE_TWO_READY.store(false, Ordering::Relaxed);
    SCENARIO_COMPLETE.store(false, Ordering::Relaxed);
    for client in 0..CLIENT_COUNT {
        for atomic in [
            &CLIENT_COMMANDS[client],
            &LAST_COMMAND_SEQUENCE[client],
            &COMMAND_MASK[client],
            &CLIENT_EVENTS[client],
            &LAST_SCENE_FRAME[client],
            &ACK_MASK[client],
            &WINDOW_TOKENS[client],
            &CONTENT_DAMAGE_PIXELS[client],
            &VISIBLE_DAMAGE_PIXELS[client],
            &OCCLUDED_DAMAGE_PIXELS[client],
            &CONTENT_PRESENTS[client],
            &LAST_CLIENT_FRAME[client],
            &LAST_COLOR[client],
        ] {
            atomic.store(0, Ordering::Relaxed);
        }
    }
    for committed in &INPUT_ROUTE_COMMITTED {
        committed.store(false, Ordering::Relaxed);
    }
    for index in 0..OUTPUT_COMMIT_COUNT {
        for atomic in [
            &OUTPUT_SLOTS[index],
            &OUTPUT_ALLOCATION_GENERATIONS[index],
            &OUTPUT_WRITE_GENERATIONS[index],
            &OUTPUT_FRAME_IDS[index],
        ] {
            atomic.store(0, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: AtomicBool = AtomicBool::new(false);

    struct TestGuard;

    impl TestGuard {
        fn acquire() -> Self {
            while TEST_LOCK
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                core::hint::spin_loop();
            }
            reset();
            Self
        }
    }

    impl Drop for TestGuard {
        fn drop(&mut self) {
            TEST_LOCK.store(false, Ordering::Release);
        }
    }

    fn id(client: usize) -> WindowId {
        WindowId::from_token(expected_token(client)).unwrap()
    }

    fn command(
        role: UiTraceRole,
        sequence: u64,
        opcode: WindowCommandOpcode,
        window_id: Option<WindowId>,
        bounds: Option<ShellRect>,
        damage: Option<ShellRect>,
        frame_id: Option<u32>,
        color: Option<u32>,
    ) -> WindowChannelTrace {
        WindowChannelTrace::Command {
            sender_role: role,
            sequence,
            opcode,
            window_id,
            bounds,
            damage,
            frame_id,
            color,
        }
    }

    fn create_command(client: usize) -> WindowChannelTrace {
        command(
            role(client),
            1,
            WindowCommandOpcode::Create,
            None,
            Some(expected_bounds(client)),
            None,
            None,
            None,
        )
    }

    fn created(client: usize, scene_frame: u32) -> WindowChannelTrace {
        WindowChannelTrace::Created {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: role(client),
            command_sequence: 1,
            scene_frame,
            window_id: id(client),
            bounds: expected_bounds(client),
        }
    }

    fn present_command(client: usize, sequence: u64) -> WindowChannelTrace {
        let (frame, damage, color) = match (client, sequence) {
            (LAUNCHER, 2) => (1, LAUNCHER_BOUNDS, COLOR_LAUNCHER_BASE),
            (LAUNCHER, 3) => (2, LAUNCHER_HIDDEN_DAMAGE, COLOR_LAUNCHER_HIDDEN),
            (LAUNCHER, 4) => (3, LAUNCHER_PARTIAL_DAMAGE, COLOR_LAUNCHER_PARTIAL),
            (APP, 2) => (1, APP_LOCAL_BOUNDS, COLOR_APP_BASE),
            (APP, 3) => (2, APP_DAMAGE, COLOR_APP_DAMAGE),
            _ => unreachable!(),
        };
        command(
            role(client),
            sequence,
            WindowCommandOpcode::Present,
            Some(id(client)),
            None,
            Some(damage),
            Some(frame),
            Some(color),
        )
    }

    fn presented(client: usize, sequence: u64, scene_frame: u32) -> WindowChannelTrace {
        let frame_id = match (client, sequence) {
            (LAUNCHER, 2) | (APP, 2) => 1,
            (LAUNCHER, 3) | (APP, 3) => 2,
            (LAUNCHER, 4) => 3,
            _ => unreachable!(),
        };
        WindowChannelTrace::Presented {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: role(client),
            command_sequence: sequence,
            scene_frame,
            window_id: id(client),
            frame_id,
        }
    }

    fn raise_command(client: usize, sequence: u64) -> WindowChannelTrace {
        command(
            role(client),
            sequence,
            WindowCommandOpcode::Raise,
            Some(id(client)),
            None,
            None,
            None,
            None,
        )
    }

    fn raised(client: usize, sequence: u64, scene_frame: u32) -> WindowChannelTrace {
        WindowChannelTrace::Raised {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: role(client),
            command_sequence: sequence,
            scene_frame,
            window_id: id(client),
        }
    }

    fn input(index: usize) -> WindowChannelTrace {
        let expected = EXPECTED_INPUT_ROUTES[index];
        WindowChannelTrace::InputRoute {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: role(expected.client),
            command_sequence: expected.command_sequence,
            scene_frame: expected.scene_frame,
            window_id: id(expected.client),
            global_x: expected.global_x,
            global_y: expected.global_y,
            local_x: expected.local_x,
            local_y: expected.local_y,
            pressed: expected.pressed,
            captured: expected.captured,
            focus_generation: expected.focus_generation,
        }
    }

    const fn role(client: usize) -> UiTraceRole {
        if client == LAUNCHER {
            UiTraceRole::Launcher
        } else {
            UiTraceRole::App
        }
    }

    fn through_hidden() {
        assert!(record(create_command(LAUNCHER)));
        assert!(record(created(LAUNCHER, 1)));
        assert!(record(present_command(LAUNCHER, 2)));
        assert!(record_output_commit(0, 1, 1, 1));
        assert!(record(presented(LAUNCHER, 2, 2)));
        assert!(record(create_command(APP)));
        assert!(record(created(APP, 3)));
        assert!(record(present_command(APP, 2)));
        assert!(record_output_commit(1, 1, 1, 2));
        assert!(record(presented(APP, 2, 4)));
        assert!(record(present_command(LAUNCHER, 3)));
        assert!(record(presented(LAUNCHER, 3, 5)));
    }

    fn happy_path() {
        through_hidden();
        for index in 0..3 {
            assert!(record(input(index)));
        }
        assert!(record(present_command(LAUNCHER, 4)));
        assert!(record_output_commit(0, 1, 2, 3));
        assert!(record(presented(LAUNCHER, 4, 6)));
        assert!(record(present_command(APP, 3)));
        assert!(record_output_commit(1, 1, 2, 4));
        assert!(record(presented(APP, 3, 7)));
        assert!(record(raise_command(LAUNCHER, 5)));
        assert!(record_output_commit(0, 1, 3, 5));
        assert!(record(raised(LAUNCHER, 5, 8)));
        for index in 3..5 {
            assert!(record(input(index)));
        }
        assert!(record(raise_command(APP, 4)));
        assert!(record_output_commit(1, 1, 3, 6));
        assert!(record(raised(APP, 4, 9)));
    }

    fn interleaved_happy_path() {
        assert!(record(create_command(LAUNCHER)));
        assert!(record(created(LAUNCHER, 1)));
        assert!(record(present_command(LAUNCHER, 2)));
        assert!(record_output_commit(0, 1, 1, 1));

        // The App channel may run through create and its first frame while
        // Launcher has not yet consumed the scene-2 Presented event.
        assert!(record(create_command(APP)));
        assert!(record(created(APP, 3)));
        assert!(record(present_command(APP, 2)));
        assert!(record_output_commit(1, 1, 1, 2));
        assert!(record(presented(APP, 2, 4)));
        assert!(record(presented(LAUNCHER, 2, 2)));

        assert!(record(present_command(LAUNCHER, 3)));
        assert!(record(presented(LAUNCHER, 3, 5)));
        assert!(record(input(0)));
        assert!(record(input(1)));

        // SurfaceServer may read command four before the final App route, but
        // command three is now an explicit continuation of that release.
        assert!(record(present_command(LAUNCHER, 4)));
        assert!(record_output_commit(0, 1, 2, 3));
        assert!(record(input(2)));
        assert!(record(present_command(APP, 3)));
        assert!(record_output_commit(1, 1, 2, 4));
        assert!(record(raise_command(LAUNCHER, 5)));
        assert!(record_output_commit(0, 1, 3, 5));
        assert!(record(presented(LAUNCHER, 4, 6)));
        assert!(record(raised(LAUNCHER, 5, 8)));
        let phase_two_ready = snapshot();
        assert_eq!(phase_two_ready.input_route_count, 3);
        assert!(!phase_two_ready.capture_active);
        assert_eq!(phase_two_ready.capture_owner, None);
        assert_eq!(phase_two_ready.output_commits, 5);
        assert!(phase_two_ready.phase_one_complete);
        assert!(phase_two_ready.phase_two_ready);
        assert!(record(input(3)));

        // The same partial order applies after SurfaceServer sends Launcher
        // release: App's queued raise and frame six need not wait for its read.
        assert!(record(raise_command(APP, 4)));
        assert!(record_output_commit(1, 1, 3, 6));
        assert!(record(presented(APP, 3, 7)));
        assert!(record(raised(APP, 4, 9)));
        let phase_two_release_lag = snapshot();
        assert_eq!(phase_two_release_lag.input_route_count, 4);
        assert!(phase_two_release_lag.capture_active);
        assert_eq!(phase_two_release_lag.capture_owner, Some(0));
        assert_eq!(phase_two_release_lag.output_commits, 6);
        assert!(!phase_two_release_lag.scenario_complete);
        assert!(!phase_two_release_lag.final_complete);
        assert!(record(input(4)));
    }

    #[test]
    fn exact_happy_path_proves_two_windows_damage_z_order_and_capture() {
        let _guard = TestGuard::acquire();
        happy_path();
        let state = snapshot();
        assert_eq!(state.traces, 23);
        assert_eq!(state.errors, 0);
        assert_eq!(state.commands, EXPECTED_COMMANDS);
        assert_eq!(
            state.command_counts,
            WindowCommandCounts {
                create: 2,
                present: 5,
                raise: 2,
                destroy: 0,
            }
        );
        assert_eq!(state.commands_by_client, [5, 4]);
        assert_eq!(state.last_command_sequences, [5, 4]);
        assert_eq!(
            state.events,
            EXPECTED_COMMAND_EVENTS + EXPECTED_INPUT_EVENTS
        );
        assert_eq!(
            state.event_counts,
            WindowEventCounts {
                created: 2,
                presented: 5,
                raised: 2,
                input: 5,
                destroyed: 0,
            }
        );
        assert_eq!(state.events_by_client, [7, 7]);
        assert_eq!(state.last_scene_frames, [8, 9]);
        assert_eq!(state.live_windows, 2);
        assert_eq!(state.z_order, [Some(0), Some(1)]);
        assert_eq!(state.z_order_changes, 4);
        assert_eq!(state.content_presents, [3, 2]);
        assert_eq!(state.last_client_frames, [3, 2]);
        assert_eq!(
            state.content_damage_pixels,
            [
                LAUNCHER_FULL_PIXELS + LAUNCHER_HIDDEN_PIXELS + LAUNCHER_PARTIAL_PIXELS,
                APP_FULL_PIXELS + APP_DAMAGE_PIXELS,
            ]
        );
        assert_eq!(
            state.visible_damage_pixels,
            [
                LAUNCHER_FULL_PIXELS + LAUNCHER_PARTIAL_PIXELS - LAUNCHER_PARTIAL_OCCLUDED_PIXELS,
                APP_FULL_PIXELS + APP_DAMAGE_PIXELS,
            ]
        );
        assert_eq!(
            state.occluded_damage_pixels,
            [LAUNCHER_HIDDEN_PIXELS + LAUNCHER_PARTIAL_OCCLUDED_PIXELS, 0,]
        );
        assert_eq!(state.raise_damage_pixels, OPAQUE_OVERLAP_PIXELS * 2);
        assert_eq!(state.output_required_commits, 6);
        assert_eq!(state.hidden_presents, 1);
        assert!(state.hidden_content_retained);
        assert_eq!(state.output_commits, 6);
        assert_eq!(state.output_errors, 0);
        assert_eq!(
            state.output_slots,
            [Some(0), Some(1), Some(0), Some(1), Some(0), Some(1)]
        );
        assert_eq!(state.output_allocation_generations, [1; 6]);
        assert_eq!(state.output_generations, [1, 1, 2, 2, 3, 3]);
        assert_eq!(state.output_frame_ids, [1, 2, 3, 4, 5, 6]);
        assert_eq!(state.input_route_count, 5);
        assert!(state.input_routes.iter().all(Option::is_some));
        assert_eq!(state.captured_routes, 5);
        assert_eq!(state.signed_outside_routes, 2);
        assert!(!state.capture_active);
        assert_eq!(state.capture_owner, None);
        assert_eq!(state.capture_clears, 2);
        assert_eq!(state.phase, WindowTracePhase::Complete);
        assert!(state.phase_one_ready);
        assert!(state.phase_one_complete);
        assert!(state.phase_two_ready);
        assert!(state.phase_two_complete);
        assert!(state.scenario_complete);
        assert!(state.final_complete);
    }

    #[test]
    fn cross_channel_reads_may_interleave_while_server_commit_order_stays_strict() {
        let _guard = TestGuard::acquire();
        interleaved_happy_path();
        let state = snapshot();
        assert_eq!(state.errors, 0);
        assert_eq!(state.traces, 23);
        assert_eq!(state.output_commits, 6);
        assert_eq!(state.last_scene_frames, [8, 9]);
        assert_eq!(state.z_order, [Some(0), Some(1)]);
        assert!(state.phase_one_ready);
        assert!(state.phase_two_ready);
        assert!(state.final_complete);
    }

    #[test]
    fn out_of_order_command_and_event_are_inert_except_for_errors() {
        let _guard = TestGuard::acquire();
        assert!(!record(present_command(LAUNCHER, 2)));
        let rejected_command = snapshot();
        assert_eq!(rejected_command.errors, 1);
        assert_eq!(rejected_command.sequence_errors, 1);
        assert_eq!(rejected_command.commands, 0);
        assert_eq!(rejected_command.last_command_sequences, [0, 0]);

        assert!(record(create_command(LAUNCHER)));
        assert!(!record(presented(LAUNCHER, 2, 2)));
        let rejected_event = snapshot();
        assert_eq!(rejected_event.errors, 2);
        assert_eq!(rejected_event.event_errors, 1);
        assert_eq!(rejected_event.events, 0);
        assert_eq!(rejected_event.acknowledged_command_masks, [0, 0]);
        assert!(record(created(LAUNCHER, 1)));
    }

    #[test]
    fn wrong_owner_cannot_claim_another_clients_window() {
        let _guard = TestGuard::acquire();
        assert!(record(create_command(LAUNCHER)));
        let forged = WindowChannelTrace::Created {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 1,
            scene_frame: 1,
            window_id: id(LAUNCHER),
            bounds: LAUNCHER_BOUNDS,
        };
        assert!(!record(forged));
        let state = snapshot();
        assert_eq!(state.errors, 1);
        assert_eq!(state.owner_errors, 1);
        assert_eq!(state.events, 0);
        assert_eq!(state.live_windows, 0);
        assert_eq!(state.window_ids, [None, None]);
        assert!(record(created(LAUNCHER, 1)));
    }

    #[test]
    fn signed_outside_coordinates_remain_captured_until_release() {
        let _guard = TestGuard::acquire();
        through_hidden();
        assert!(record(input(0)));
        let mut malformed = input(1);
        if let WindowChannelTrace::InputRoute { local_x, .. } = &mut malformed {
            *local_x = 32;
        }
        assert!(!record(malformed));
        let rejected = snapshot();
        assert_eq!(rejected.input_route_count, 1);
        assert!(rejected.capture_active);
        assert_eq!(rejected.capture_owner, Some(1));
        assert_eq!(rejected.signed_outside_routes, 0);

        assert!(record(input(1)));
        let moved = snapshot();
        assert_eq!(moved.input_route_count, 2);
        assert_eq!(moved.signed_outside_routes, 1);
        assert!(moved.capture_active);
        assert_eq!(moved.input_routes[1].unwrap().local_x, -32);
        assert_eq!(moved.input_routes[1].unwrap().local_y, -48);

        assert!(record(input(2)));
        let released = snapshot();
        assert!(!released.capture_active);
        assert_eq!(released.capture_owner, None);
        assert_eq!(released.capture_clears, 1);
        assert_eq!(released.signed_outside_routes, 2);
    }

    #[test]
    fn hidden_damage_updates_retained_content_without_requesting_output() {
        let _guard = TestGuard::acquire();
        through_hidden();
        let state = snapshot();
        assert_eq!(state.phase, WindowTracePhase::HiddenDamageRetained);
        assert_eq!(state.output_required_commits, 2);
        assert_eq!(state.hidden_presents, 1);
        assert!(state.hidden_content_retained);
        assert_eq!(state.output_commits, 2);
        assert_eq!(
            state.content_damage_pixels,
            [
                LAUNCHER_FULL_PIXELS + LAUNCHER_HIDDEN_PIXELS,
                APP_FULL_PIXELS,
            ]
        );
        assert_eq!(
            state.visible_damage_pixels,
            [LAUNCHER_FULL_PIXELS, APP_FULL_PIXELS]
        );
        assert_eq!(state.occluded_damage_pixels, [LAUNCHER_HIDDEN_PIXELS, 0]);
        assert_eq!(state.z_order, [Some(0), Some(1)]);
        assert!(state.phase_one_ready);
        assert!(!state.phase_one_complete);
    }

    #[test]
    fn incorrect_scene_frame_does_not_consume_pending_command() {
        let _guard = TestGuard::acquire();
        assert!(record(create_command(LAUNCHER)));
        assert!(!record(created(LAUNCHER, 2)));
        let rejected = snapshot();
        assert_eq!(rejected.scene_errors, 1);
        assert_eq!(rejected.events, 0);
        assert_eq!(rejected.acknowledged_command_masks, [0, 0]);
        assert!(record(created(LAUNCHER, 1)));
        assert_eq!(snapshot().last_scene_frames, [1, 0]);
    }

    #[test]
    fn rejected_output_commit_does_not_advance_the_ababab_schedule() {
        let _guard = TestGuard::acquire();
        assert!(record(create_command(LAUNCHER)));
        assert!(record(created(LAUNCHER, 1)));
        assert!(record(present_command(LAUNCHER, 2)));
        assert!(!record_output_commit(1, 1, 1, 1));
        let rejected = snapshot();
        assert_eq!(rejected.errors, 1);
        assert_eq!(rejected.output_errors, 1);
        assert_eq!(rejected.output_commits, 0);
        assert_eq!(rejected.output_slots, [None; 6]);
        assert_eq!(rejected.output_generations, [0; 6]);

        assert!(record_output_commit(0, 1, 1, 1));
        let accepted = snapshot();
        assert_eq!(accepted.output_commits, 1);
        assert_eq!(accepted.output_slots[0], Some(0));
        assert_eq!(accepted.output_generations[0], 1);
        assert_eq!(accepted.output_frame_ids[0], 1);
        assert!(!accepted.scenario_complete);
    }

    #[test]
    fn malformed_window_wire_fails_closed_without_advancing_transcript() {
        let _guard = TestGuard::acquire();
        assert!(!record_invalid_wire(false));
        let malformed = snapshot();
        assert_eq!(malformed.traces, 0);
        assert_eq!(malformed.errors, 1);
        assert_eq!(malformed.payload_errors, 1);
        assert_eq!(malformed.owner_errors, 0);

        reset();
        assert!(!record_invalid_wire(true));
        let wrong_owner = snapshot();
        assert_eq!(wrong_owner.traces, 0);
        assert_eq!(wrong_owner.errors, 1);
        assert_eq!(wrong_owner.payload_errors, 0);
        assert_eq!(wrong_owner.owner_errors, 1);
    }

    #[test]
    fn post_witness_destroy_is_tracked_and_invalidates_resident_completion() {
        let _guard = TestGuard::acquire();
        happy_path();
        assert!(record(command(
            UiTraceRole::App,
            5,
            WindowCommandOpcode::Destroy,
            Some(id(APP)),
            None,
            None,
            None,
            None,
        )));
        assert!(record(WindowChannelTrace::Destroyed {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 5,
            scene_frame: 10,
            window_id: id(APP),
        }));
        let state = snapshot();
        assert_eq!(state.command_counts.destroy, 1);
        assert_eq!(state.event_counts.destroyed, 1);
        assert_eq!(state.live_windows, 1);
        assert_eq!(state.z_order, [Some(0), None]);
        assert_eq!(state.phase, WindowTracePhase::Teardown);
        assert!(state.scenario_complete);
        assert!(!state.final_complete);
    }
}
