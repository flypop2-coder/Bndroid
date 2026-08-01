//! Extension evidence for the M42 persistent userspace window session.
//!
//! M42 deliberately keeps the complete M41 transcript as an immutable prefix.
//! Once that prefix has converged, this module proves that the same resident
//! SurfaceServer continues to route captured input, destroy the App window,
//! recreate its reserved slot with a fresh generation, present it, and accept
//! an idempotent raise.  The wrapper functions below route the prefix to
//! `window_trace` and only account the extension here.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[cfg(test)]
use bndr_ui::WindowId;
use bndr_ui::{ShellRect, WindowCommandOpcode};

use crate::ui_trace::{UiTraceRole, WindowChannelTrace};

const APP_GENERATION_ONE_TOKEN: u64 = (1_u64 << 32) | 1;
const APP_GENERATION_TWO_TOKEN: u64 = (2_u64 << 32) | 1;
const APP_BOUNDS: ShellRect = ShellRect::new(48, 80, 112, 160);
const APP_LOCAL_BOUNDS: ShellRect = ShellRect::new(0, 0, 112, 160);
pub const APP_RECREATED_COLOR: u32 = 0x00b2_6b36;

const PHASE_INPUT_DOWN: u64 = 0;
const PHASE_INPUT_MOVE: u64 = 1;
const PHASE_INPUT_UP: u64 = 2;
const PHASE_DESTROY_COMMAND: u64 = 3;
const PHASE_DESTROY_OUTPUT: u64 = 4;
const PHASE_DESTROYED_EVENT: u64 = 5;
const PHASE_CREATE_COMMAND: u64 = 6;
const PHASE_CREATED_EVENT: u64 = 7;
const PHASE_PRESENT_COMMAND: u64 = 8;
const PHASE_PRESENT_OUTPUT: u64 = 9;
const PHASE_PRESENTED_EVENT: u64 = 10;
const PHASE_RAISE_COMMAND: u64 = 11;
const PHASE_RAISED_EVENT: u64 = 12;
const PHASE_COMPLETE: u64 = 13;

static PHASE: AtomicU64 = AtomicU64::new(PHASE_INPUT_DOWN);
static TRACES: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);
static OWNER_ERRORS: AtomicU64 = AtomicU64::new(0);
static PAYLOAD_ERRORS: AtomicU64 = AtomicU64::new(0);
static PHASE_ERRORS: AtomicU64 = AtomicU64::new(0);
static COMMANDS: AtomicU64 = AtomicU64::new(0);
static EVENTS: AtomicU64 = AtomicU64::new(0);
static INPUT_EVENTS: AtomicU64 = AtomicU64::new(0);
static OUTPUT_COMMITS: AtomicU64 = AtomicU64::new(0);
static CAPTURE_CLEARS: AtomicU64 = AtomicU64::new(0);
static SIGNED_OUTSIDE_ROUTES: AtomicU64 = AtomicU64::new(0);
static FINAL_COMPLETE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistentWindowTracePhase {
    InputDown,
    InputMove,
    InputUp,
    DestroyCommand,
    DestroyOutput,
    DestroyedEvent,
    CreateCommand,
    CreatedEvent,
    PresentCommand,
    PresentOutput,
    PresentedEvent,
    RaiseCommand,
    RaisedEvent,
    Complete,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PersistentWindowTraceSnapshot {
    pub phase: PersistentWindowTracePhase,
    pub traces: u64,
    pub errors: u64,
    pub owner_errors: u64,
    pub payload_errors: u64,
    pub phase_errors: u64,
    pub commands: u64,
    pub events: u64,
    pub input_events: u64,
    pub output_commits: u64,
    pub capture_clears: u64,
    pub signed_outside_routes: u64,
    pub input_ready: bool,
    pub capture_complete: bool,
    pub app_generation: u32,
    pub final_complete: bool,
}

/// Records a canonical BWC1/BWE1 read, preserving the complete M41 prefix.
pub fn record(trace: WindowChannelTrace) -> bool {
    if !crate::window_trace::snapshot().scenario_complete {
        return crate::window_trace::record(trace);
    }
    record_extension(trace)
}

/// Records one output commit. The first six commits remain owned by M41; the
/// destroy and recreated-present commits are extension evidence.
pub fn record_output_commit(
    slot: usize,
    allocation_generation: u32,
    write_generation: u64,
    frame_id: u32,
) -> bool {
    if crate::window_trace::snapshot().output_commits < 6 {
        return crate::window_trace::record_output_commit(
            slot,
            allocation_generation,
            write_generation,
            frame_id,
        );
    }

    record_output_extension(slot, allocation_generation, write_generation, frame_id)
}

fn record_output_extension(
    slot: usize,
    allocation_generation: u32,
    write_generation: u64,
    frame_id: u32,
) -> bool {
    let phase = PHASE.load(Ordering::Acquire);
    if phase == PHASE_COMPLETE {
        return true;
    }
    let valid = match phase {
        PHASE_DESTROY_OUTPUT => {
            slot == 0 && allocation_generation == 1 && write_generation == 4 && frame_id == 7
        }
        PHASE_PRESENT_OUTPUT => {
            slot == 1 && allocation_generation == 1 && write_generation == 4 && frame_id == 8
        }
        _ => return reject_phase(),
    };
    if !valid {
        return reject_payload();
    }
    OUTPUT_COMMITS.fetch_add(1, Ordering::Relaxed);
    PHASE.store(phase + 1, Ordering::Release);
    true
}

/// Makes malformed extension traffic fail closed without corrupting the M41
/// ledger that preceded it.
pub fn record_invalid_wire(owner_error: bool) -> bool {
    if !crate::window_trace::snapshot().scenario_complete {
        return crate::window_trace::record_invalid_wire(owner_error);
    }
    ERRORS.fetch_add(1, Ordering::Relaxed);
    if owner_error {
        OWNER_ERRORS.fetch_add(1, Ordering::Relaxed);
    } else {
        PAYLOAD_ERRORS.fetch_add(1, Ordering::Relaxed);
    }
    false
}

fn record_extension(trace: WindowChannelTrace) -> bool {
    let phase = PHASE.load(Ordering::Acquire);
    if phase == PHASE_COMPLETE {
        // The bounded M42 witness is already immutable. Canonical traffic is
        // allowed to continue through the resident session and is guarded by
        // the userspace command/event state machines rather than poisoning the
        // completed acceptance ledger.
        let _ = trace;
        return true;
    }
    let (valid, next_phase, is_command, is_event, is_input, clears, signed) = match phase {
        PHASE_INPUT_DOWN => (
            expected_input(trace, 80, 120, 32, 40, true),
            PHASE_INPUT_MOVE,
            false,
            true,
            true,
            false,
            false,
        ),
        PHASE_INPUT_MOVE => (
            expected_input(trace, 0, 0, -84, -124, true),
            PHASE_INPUT_UP,
            false,
            true,
            true,
            false,
            true,
        ),
        PHASE_INPUT_UP => (
            expected_input(trace, 0, 0, -84, -124, false),
            PHASE_DESTROY_COMMAND,
            false,
            true,
            true,
            true,
            true,
        ),
        PHASE_DESTROY_COMMAND => (
            expected_destroy_command(trace),
            PHASE_DESTROY_OUTPUT,
            true,
            false,
            false,
            false,
            false,
        ),
        PHASE_DESTROYED_EVENT => (
            expected_destroyed_event(trace),
            PHASE_CREATE_COMMAND,
            false,
            true,
            false,
            false,
            false,
        ),
        PHASE_CREATE_COMMAND => (
            expected_create_command(trace),
            PHASE_CREATED_EVENT,
            true,
            false,
            false,
            false,
            false,
        ),
        PHASE_CREATED_EVENT => (
            expected_created_event(trace),
            PHASE_PRESENT_COMMAND,
            false,
            true,
            false,
            false,
            false,
        ),
        PHASE_PRESENT_COMMAND => (
            expected_present_command(trace),
            PHASE_PRESENT_OUTPUT,
            true,
            false,
            false,
            false,
            false,
        ),
        PHASE_PRESENTED_EVENT => (
            expected_presented_event(trace),
            PHASE_RAISE_COMMAND,
            false,
            true,
            false,
            false,
            false,
        ),
        PHASE_RAISE_COMMAND => (
            expected_raise_command(trace),
            PHASE_RAISED_EVENT,
            true,
            false,
            false,
            false,
            false,
        ),
        PHASE_RAISED_EVENT => (
            expected_raised_event(trace),
            PHASE_COMPLETE,
            false,
            true,
            false,
            false,
            false,
        ),
        PHASE_DESTROY_OUTPUT | PHASE_PRESENT_OUTPUT | PHASE_COMPLETE => {
            return reject_phase();
        }
        _ => return reject_phase(),
    };
    if !valid {
        return reject_payload();
    }

    if is_command {
        COMMANDS.fetch_add(1, Ordering::Relaxed);
    }
    if is_event {
        EVENTS.fetch_add(1, Ordering::Relaxed);
    }
    if is_input {
        INPUT_EVENTS.fetch_add(1, Ordering::Relaxed);
    }
    if clears {
        CAPTURE_CLEARS.fetch_add(1, Ordering::Relaxed);
    }
    if signed {
        SIGNED_OUTSIDE_ROUTES.fetch_add(1, Ordering::Relaxed);
    }
    TRACES.fetch_add(1, Ordering::Relaxed);
    PHASE.store(next_phase, Ordering::Release);
    if next_phase == PHASE_COMPLETE {
        FINAL_COMPLETE.store(true, Ordering::Release);
    }
    true
}

fn expected_input(
    trace: WindowChannelTrace,
    global_x: u16,
    global_y: u16,
    local_x: i32,
    local_y: i32,
    pressed: bool,
) -> bool {
    matches!(
        trace,
        WindowChannelTrace::InputRoute {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 4,
            scene_frame: 9,
            window_id,
            global_x: actual_global_x,
            global_y: actual_global_y,
            local_x: actual_local_x,
            local_y: actual_local_y,
            pressed: actual_pressed,
            captured: true,
            focus_generation: 3,
        } if window_id.token() == APP_GENERATION_ONE_TOKEN
            && actual_global_x == global_x
            && actual_global_y == global_y
            && actual_local_x == local_x
            && actual_local_y == local_y
            && actual_pressed == pressed
    )
}

fn expected_destroy_command(trace: WindowChannelTrace) -> bool {
    matches!(
        trace,
        WindowChannelTrace::Command {
            sender_role: UiTraceRole::App,
            sequence: 5,
            opcode: WindowCommandOpcode::Destroy,
            window_id: Some(window_id),
            bounds: None,
            damage: None,
            frame_id: None,
            color: None,
        } if window_id.token() == APP_GENERATION_ONE_TOKEN
    )
}

fn expected_destroyed_event(trace: WindowChannelTrace) -> bool {
    matches!(
        trace,
        WindowChannelTrace::Destroyed {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 5,
            scene_frame: 10,
            window_id,
        } if window_id.token() == APP_GENERATION_ONE_TOKEN
    )
}

fn expected_create_command(trace: WindowChannelTrace) -> bool {
    matches!(
        trace,
        WindowChannelTrace::Command {
            sender_role: UiTraceRole::App,
            sequence: 6,
            opcode: WindowCommandOpcode::Create,
            window_id: None,
            bounds: Some(APP_BOUNDS),
            damage: None,
            frame_id: None,
            color: None,
        }
    )
}

fn expected_created_event(trace: WindowChannelTrace) -> bool {
    matches!(
        trace,
        WindowChannelTrace::Created {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 6,
            scene_frame: 11,
            window_id,
            bounds: APP_BOUNDS,
        } if window_id.token() == APP_GENERATION_TWO_TOKEN
    )
}

fn expected_present_command(trace: WindowChannelTrace) -> bool {
    matches!(
        trace,
        WindowChannelTrace::Command {
            sender_role: UiTraceRole::App,
            sequence: 7,
            opcode: WindowCommandOpcode::Present,
            window_id: Some(window_id),
            bounds: None,
            damage: Some(APP_LOCAL_BOUNDS),
            frame_id: Some(1),
            color: Some(APP_RECREATED_COLOR),
        } if window_id.token() == APP_GENERATION_TWO_TOKEN
    )
}

fn expected_presented_event(trace: WindowChannelTrace) -> bool {
    matches!(
        trace,
        WindowChannelTrace::Presented {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 7,
            scene_frame: 12,
            window_id,
            frame_id: 1,
        } if window_id.token() == APP_GENERATION_TWO_TOKEN
    )
}

fn expected_raise_command(trace: WindowChannelTrace) -> bool {
    matches!(
        trace,
        WindowChannelTrace::Command {
            sender_role: UiTraceRole::App,
            sequence: 8,
            opcode: WindowCommandOpcode::Raise,
            window_id: Some(window_id),
            bounds: None,
            damage: None,
            frame_id: None,
            color: None,
        } if window_id.token() == APP_GENERATION_TWO_TOKEN
    )
}

fn expected_raised_event(trace: WindowChannelTrace) -> bool {
    matches!(
        trace,
        WindowChannelTrace::Raised {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 8,
            scene_frame: 13,
            window_id,
        } if window_id.token() == APP_GENERATION_TWO_TOKEN
    )
}

fn reject_payload() -> bool {
    ERRORS.fetch_add(1, Ordering::Relaxed);
    PAYLOAD_ERRORS.fetch_add(1, Ordering::Relaxed);
    false
}

fn reject_phase() -> bool {
    ERRORS.fetch_add(1, Ordering::Relaxed);
    PHASE_ERRORS.fetch_add(1, Ordering::Relaxed);
    false
}

pub fn snapshot() -> PersistentWindowTraceSnapshot {
    let raw_phase = PHASE.load(Ordering::Acquire);
    PersistentWindowTraceSnapshot {
        phase: decode_phase(raw_phase),
        traces: TRACES.load(Ordering::Acquire),
        errors: ERRORS.load(Ordering::Acquire),
        owner_errors: OWNER_ERRORS.load(Ordering::Acquire),
        payload_errors: PAYLOAD_ERRORS.load(Ordering::Acquire),
        phase_errors: PHASE_ERRORS.load(Ordering::Acquire),
        commands: COMMANDS.load(Ordering::Acquire),
        events: EVENTS.load(Ordering::Acquire),
        input_events: INPUT_EVENTS.load(Ordering::Acquire),
        output_commits: OUTPUT_COMMITS.load(Ordering::Acquire),
        capture_clears: CAPTURE_CLEARS.load(Ordering::Acquire),
        signed_outside_routes: SIGNED_OUTSIDE_ROUTES.load(Ordering::Acquire),
        input_ready: raw_phase == PHASE_INPUT_DOWN,
        capture_complete: raw_phase >= PHASE_DESTROY_COMMAND,
        // Generation two is observable only after the Created event itself
        // has been accepted and the transcript has advanced beyond it.
        app_generation: if raw_phase >= PHASE_PRESENT_COMMAND {
            2
        } else {
            1
        },
        final_complete: FINAL_COMPLETE.load(Ordering::Acquire),
    }
}

const fn decode_phase(raw: u64) -> PersistentWindowTracePhase {
    match raw {
        PHASE_INPUT_DOWN => PersistentWindowTracePhase::InputDown,
        PHASE_INPUT_MOVE => PersistentWindowTracePhase::InputMove,
        PHASE_INPUT_UP => PersistentWindowTracePhase::InputUp,
        PHASE_DESTROY_COMMAND => PersistentWindowTracePhase::DestroyCommand,
        PHASE_DESTROY_OUTPUT => PersistentWindowTracePhase::DestroyOutput,
        PHASE_DESTROYED_EVENT => PersistentWindowTracePhase::DestroyedEvent,
        PHASE_CREATE_COMMAND => PersistentWindowTracePhase::CreateCommand,
        PHASE_CREATED_EVENT => PersistentWindowTracePhase::CreatedEvent,
        PHASE_PRESENT_COMMAND => PersistentWindowTracePhase::PresentCommand,
        PHASE_PRESENT_OUTPUT => PersistentWindowTracePhase::PresentOutput,
        PHASE_PRESENTED_EVENT => PersistentWindowTracePhase::PresentedEvent,
        PHASE_RAISE_COMMAND => PersistentWindowTracePhase::RaiseCommand,
        PHASE_RAISED_EVENT => PersistentWindowTracePhase::RaisedEvent,
        PHASE_COMPLETE => PersistentWindowTracePhase::Complete,
        _ => PersistentWindowTracePhase::Invalid,
    }
}

#[cfg(test)]
fn reset() {
    for atomic in [
        &TRACES,
        &ERRORS,
        &OWNER_ERRORS,
        &PAYLOAD_ERRORS,
        &PHASE_ERRORS,
        &COMMANDS,
        &EVENTS,
        &INPUT_EVENTS,
        &OUTPUT_COMMITS,
        &CAPTURE_CLEARS,
        &SIGNED_OUTSIDE_ROUTES,
    ] {
        atomic.store(0, Ordering::Relaxed);
    }
    PHASE.store(PHASE_INPUT_DOWN, Ordering::Relaxed);
    FINAL_COMPLETE.store(false, Ordering::Relaxed);
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

    fn app_id(generation: u32) -> WindowId {
        WindowId::try_new(1, generation).unwrap()
    }

    fn input(x: u16, y: u16, local_x: i32, local_y: i32, pressed: bool) -> WindowChannelTrace {
        WindowChannelTrace::InputRoute {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 4,
            scene_frame: 9,
            window_id: app_id(1),
            global_x: x,
            global_y: y,
            local_x,
            local_y,
            pressed,
            captured: true,
            focus_generation: 3,
        }
    }

    fn drive_capture() {
        assert!(record_extension(input(80, 120, 32, 40, true)));
        assert!(record_extension(input(0, 0, -84, -124, true)));
        assert!(record_extension(input(0, 0, -84, -124, false)));
    }

    #[test]
    fn capture_prefix_is_exact_and_transactional() {
        let _guard = TestGuard::acquire();
        let before = snapshot();
        assert!(!record_extension(input(81, 120, 32, 40, true)));
        let rejected = snapshot();
        assert_eq!(rejected.phase, before.phase);
        assert_eq!(rejected.traces, before.traces);
        assert_eq!(rejected.errors, 1);
        reset();
        drive_capture();
        let state = snapshot();
        assert_eq!(state.input_events, 3);
        assert_eq!(state.signed_outside_routes, 2);
        assert_eq!(state.capture_clears, 1);
        assert!(state.capture_complete);
    }

    #[test]
    fn full_extension_reuses_slot_with_fresh_generation() {
        let _guard = TestGuard::acquire();
        drive_capture();
        assert!(record_extension(WindowChannelTrace::Command {
            sender_role: UiTraceRole::App,
            sequence: 5,
            opcode: WindowCommandOpcode::Destroy,
            window_id: Some(app_id(1)),
            bounds: None,
            damage: None,
            frame_id: None,
            color: None,
        }));
        assert!(record_output_extension(0, 1, 4, 7));
        assert!(record_extension(WindowChannelTrace::Destroyed {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 5,
            scene_frame: 10,
            window_id: app_id(1),
        }));
        assert!(record_extension(WindowChannelTrace::Command {
            sender_role: UiTraceRole::App,
            sequence: 6,
            opcode: WindowCommandOpcode::Create,
            window_id: None,
            bounds: Some(APP_BOUNDS),
            damage: None,
            frame_id: None,
            color: None,
        }));
        assert_eq!(snapshot().app_generation, 1);
        assert!(record_extension(WindowChannelTrace::Created {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 6,
            scene_frame: 11,
            window_id: app_id(2),
            bounds: APP_BOUNDS,
        }));
        assert_eq!(snapshot().app_generation, 2);
        assert!(record_extension(WindowChannelTrace::Command {
            sender_role: UiTraceRole::App,
            sequence: 7,
            opcode: WindowCommandOpcode::Present,
            window_id: Some(app_id(2)),
            bounds: None,
            damage: Some(APP_LOCAL_BOUNDS),
            frame_id: Some(1),
            color: Some(APP_RECREATED_COLOR),
        }));
        assert!(record_output_extension(1, 1, 4, 8));
        assert!(record_extension(WindowChannelTrace::Presented {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 7,
            scene_frame: 12,
            window_id: app_id(2),
            frame_id: 1,
        }));
        assert!(record_extension(WindowChannelTrace::Command {
            sender_role: UiTraceRole::App,
            sequence: 8,
            opcode: WindowCommandOpcode::Raise,
            window_id: Some(app_id(2)),
            bounds: None,
            damage: None,
            frame_id: None,
            color: None,
        }));
        assert!(record_extension(WindowChannelTrace::Raised {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 8,
            scene_frame: 13,
            window_id: app_id(2),
        }));
        let state = snapshot();
        assert_eq!(state.phase, PersistentWindowTracePhase::Complete);
        assert_eq!((state.traces, state.commands, state.events), (11, 4, 7));
        assert_eq!((state.input_events, state.output_commits), (3, 2));
        assert_eq!(state.app_generation, 2);
        assert!(state.final_complete);
    }

    #[test]
    fn output_schedule_and_generation_fail_closed() {
        let _guard = TestGuard::acquire();
        drive_capture();
        assert!(record_extension(WindowChannelTrace::Command {
            sender_role: UiTraceRole::App,
            sequence: 5,
            opcode: WindowCommandOpcode::Destroy,
            window_id: Some(app_id(1)),
            bounds: None,
            damage: None,
            frame_id: None,
            color: None,
        }));
        let before = snapshot();
        assert!(!record_output_extension(1, 1, 4, 7));
        let after = snapshot();
        assert_eq!(after.phase, before.phase);
        assert_eq!(after.output_commits, before.output_commits);
        assert_eq!(after.errors, before.errors + 1);
    }
}
