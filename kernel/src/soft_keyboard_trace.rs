//! Immutable evidence for the M44 software-keyboard text-session extension.
//!
//! The authenticated ABI-21 BTI1/BTE1 decoder supplies [`TextChannelTrace`]
//! values after the syscall layer has resolved both process identities.  This
//! state machine is deliberately narrower than the protocol: after the exact
//! M43 transcript is complete, it accepts only the two focus-scoped sessions,
//! eight paced output commits, and five editor revisions used by the M44 boot
//! proof.  All runtime state is fixed-size atomic storage; recording performs
//! no allocation.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bndr_ui::{TextInputCommandPayload, TextInputEventPayload, TextState};

use crate::ui_trace::{TextChannelTrace, UiTraceRole};

const APP_WINDOW_TOKEN: u64 = (2_u64 << 32) | 1;
const SESSION_TWO: u64 = 2;
const SESSION_TWO_FOCUS: u64 = 7;
const SESSION_THREE: u64 = 3;
const SESSION_THREE_FOCUS: u64 = 9;

const OUTPUT_COUNT: usize = 8;
const OUTPUT_PHASES: [u64; OUTPUT_COUNT] = [1, 5, 9, 13, 17, 21, 24, 27];
const OUTPUT_SLOTS: [usize; OUTPUT_COUNT] = [1, 0, 1, 0, 1, 0, 1, 0];
const OUTPUT_WRITE_GENERATIONS: [u64; OUTPUT_COUNT] = [7, 8, 8, 9, 9, 10, 10, 11];

const PHASE_SESSION_TWO_ACTIVATE: u64 = 0;
const PHASE_OVERLAY_SHOW_ONE: u64 = 1;
const PHASE_SESSION_TWO_ACTIVATED: u64 = 2;
const PHASE_PREEDIT_ONE: u64 = 3;
const PHASE_ACK_ONE: u64 = 4;
const PHASE_EDITOR_OUTPUT_ONE: u64 = 5;
const PHASE_RENDERED_ONE: u64 = 6;
const PHASE_COMMIT_ONE: u64 = 7;
const PHASE_ACK_TWO: u64 = 8;
const PHASE_EDITOR_OUTPUT_TWO: u64 = 9;
const PHASE_RENDERED_TWO: u64 = 10;
const PHASE_DELETE: u64 = 11;
const PHASE_ACK_THREE: u64 = 12;
const PHASE_EDITOR_OUTPUT_THREE: u64 = 13;
const PHASE_RENDERED_THREE: u64 = 14;
const PHASE_PREEDIT_TWO: u64 = 15;
const PHASE_ACK_FOUR: u64 = 16;
const PHASE_EDITOR_OUTPUT_FOUR: u64 = 17;
const PHASE_RENDERED_FOUR: u64 = 18;
const PHASE_COMMIT_TWO: u64 = 19;
const PHASE_ACK_FIVE: u64 = 20;
const PHASE_EDITOR_OUTPUT_FIVE: u64 = 21;
const PHASE_RENDERED_FIVE: u64 = 22;
const PHASE_SESSION_TWO_DEACTIVATE: u64 = 23;
const PHASE_OVERLAY_HIDE: u64 = 24;
const PHASE_SESSION_TWO_DEACTIVATED: u64 = 25;
const PHASE_SESSION_THREE_ACTIVATE: u64 = 26;
const PHASE_OVERLAY_SHOW_TWO: u64 = 27;
const PHASE_SESSION_THREE_ACTIVATED: u64 = 28;
const PHASE_COMPLETE: u64 = 29;

static PHASE: AtomicU64 = AtomicU64::new(PHASE_SESSION_TWO_ACTIVATE);
static MESSAGES: AtomicU64 = AtomicU64::new(0);
static COMMANDS: AtomicU64 = AtomicU64::new(0);
static EVENTS: AtomicU64 = AtomicU64::new(0);
static ACTIVATE_COMMANDS: AtomicU64 = AtomicU64::new(0);
static ACK_COMMANDS: AtomicU64 = AtomicU64::new(0);
static DEACTIVATE_COMMANDS: AtomicU64 = AtomicU64::new(0);
static ACTIVATED_EVENTS: AtomicU64 = AtomicU64::new(0);
static PREEDIT_EVENTS: AtomicU64 = AtomicU64::new(0);
static COMMIT_EVENTS: AtomicU64 = AtomicU64::new(0);
static DELETE_EVENTS: AtomicU64 = AtomicU64::new(0);
static RENDERED_EVENTS: AtomicU64 = AtomicU64::new(0);
static DEACTIVATED_EVENTS: AtomicU64 = AtomicU64::new(0);
static OUTPUT_COMMITS: AtomicU64 = AtomicU64::new(0);
static EDITOR_OUTPUTS: AtomicU64 = AtomicU64::new(0);
static OVERLAY_SHOWS: AtomicU64 = AtomicU64::new(0);
static OVERLAY_HIDES: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);
static OWNER_ERRORS: AtomicU64 = AtomicU64::new(0);
static PAYLOAD_ERRORS: AtomicU64 = AtomicU64::new(0);
static PHASE_ERRORS: AtomicU64 = AtomicU64::new(0);
static PREFIX_ERRORS: AtomicU64 = AtomicU64::new(0);
static FINAL_COMPLETE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoftKeyboardTracePhase {
    SessionTwoActivate,
    OverlayShowOne,
    SessionTwoActivated,
    PreeditOne,
    AckOne,
    EditorOutputOne,
    RenderedOne,
    CommitOne,
    AckTwo,
    EditorOutputTwo,
    RenderedTwo,
    Delete,
    AckThree,
    EditorOutputThree,
    RenderedThree,
    PreeditTwo,
    AckFour,
    EditorOutputFour,
    RenderedFour,
    CommitTwo,
    AckFive,
    EditorOutputFive,
    RenderedFive,
    SessionTwoDeactivate,
    OverlayHide,
    SessionTwoDeactivated,
    SessionThreeActivate,
    OverlayShowTwo,
    SessionThreeActivated,
    Complete,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SoftKeyboardTraceSnapshot {
    pub phase: SoftKeyboardTracePhase,
    pub messages: u64,
    pub commands: u64,
    pub events: u64,
    pub activate_commands: u64,
    pub ack_commands: u64,
    pub deactivate_commands: u64,
    pub activated_events: u64,
    pub preedit_events: u64,
    pub commit_events: u64,
    pub delete_events: u64,
    pub rendered_events: u64,
    pub deactivated_events: u64,
    pub output_commits: u64,
    pub editor_outputs: u64,
    pub overlay_shows: u64,
    pub overlay_hides: u64,
    pub errors: u64,
    pub owner_errors: u64,
    pub payload_errors: u64,
    pub phase_errors: u64,
    pub prefix_errors: u64,
    pub final_complete: bool,
}

/// Records one canonical, process-authenticated BTI1/BTE1 delivery.
pub fn record(trace: TextChannelTrace) -> bool {
    if !prefix_complete() {
        return reject_prefix();
    }

    let phase = PHASE.load(Ordering::Acquire);
    if phase == PHASE_COMPLETE {
        return reject_phase();
    }
    if !exact_roles(trace) {
        return reject_owner();
    }

    let (valid, command, counter): (bool, bool, &AtomicU64) = match phase {
        PHASE_SESSION_TWO_ACTIVATE => (
            matches!(
                trace,
                TextChannelTrace::Command { command, .. }
                    if exact_context(command.context(), SESSION_TWO, SESSION_TWO_FOCUS)
                        && command.sequence() == 8
                        && matches!(command.payload(), TextInputCommandPayload::Activate)
            ),
            true,
            &ACTIVATE_COMMANDS,
        ),
        PHASE_SESSION_TWO_ACTIVATED => (
            matches!(
                trace,
                TextChannelTrace::Event { event, .. }
                    if exact_context(event.context(), SESSION_TWO, SESSION_TWO_FOCUS)
                        && event.sequence() == 13
                        && matches!(
                            event.payload(),
                            TextInputEventPayload::Activated {
                                acknowledged_command_sequence: 8,
                                revision: 0,
                            }
                        )
            ),
            false,
            &ACTIVATED_EVENTS,
        ),
        PHASE_PREEDIT_ONE => (
            expected_preedit(trace, SESSION_TWO, SESSION_TWO_FOCUS, 14, 1),
            false,
            &PREEDIT_EVENTS,
        ),
        PHASE_ACK_ONE => (expected_ack(trace, 9, 14, 1, "a", 1), true, &ACK_COMMANDS),
        PHASE_RENDERED_ONE => (
            expected_rendered(trace, 15, 9, 1, "a", 1),
            false,
            &RENDERED_EVENTS,
        ),
        PHASE_COMMIT_ONE => (expected_commit(trace, 16, 2), false, &COMMIT_EVENTS),
        PHASE_ACK_TWO => (expected_ack(trace, 10, 16, 2, "aa", 2), true, &ACK_COMMANDS),
        PHASE_RENDERED_TWO => (
            expected_rendered(trace, 17, 10, 2, "aa", 2),
            false,
            &RENDERED_EVENTS,
        ),
        PHASE_DELETE => (
            matches!(
                trace,
                TextChannelTrace::Event { event, .. }
                    if exact_context(event.context(), SESSION_TWO, SESSION_TWO_FOCUS)
                        && event.sequence() == 18
                        && matches!(
                            event.payload(),
                            TextInputEventPayload::DeleteSurrounding {
                                revision: 3,
                                before_scalars: 1,
                                after_scalars: 0,
                            }
                        )
            ),
            false,
            &DELETE_EVENTS,
        ),
        PHASE_ACK_THREE => (expected_ack(trace, 11, 18, 3, "a", 1), true, &ACK_COMMANDS),
        PHASE_RENDERED_THREE => (
            expected_rendered(trace, 19, 11, 3, "a", 1),
            false,
            &RENDERED_EVENTS,
        ),
        PHASE_PREEDIT_TWO => (
            expected_preedit(trace, SESSION_TWO, SESSION_TWO_FOCUS, 20, 4),
            false,
            &PREEDIT_EVENTS,
        ),
        PHASE_ACK_FOUR => (expected_ack(trace, 12, 20, 4, "a", 1), true, &ACK_COMMANDS),
        PHASE_RENDERED_FOUR => (
            expected_rendered(trace, 21, 12, 4, "a", 1),
            false,
            &RENDERED_EVENTS,
        ),
        PHASE_COMMIT_TWO => (expected_commit(trace, 22, 5), false, &COMMIT_EVENTS),
        PHASE_ACK_FIVE => (expected_ack(trace, 13, 22, 5, "aa", 2), true, &ACK_COMMANDS),
        PHASE_RENDERED_FIVE => (
            expected_rendered(trace, 23, 13, 5, "aa", 2),
            false,
            &RENDERED_EVENTS,
        ),
        PHASE_SESSION_TWO_DEACTIVATE => (
            matches!(
                trace,
                TextChannelTrace::Command { command, .. }
                    if exact_context(command.context(), SESSION_TWO, SESSION_TWO_FOCUS)
                        && command.sequence() == 14
                        && matches!(command.payload(), TextInputCommandPayload::Deactivate)
            ),
            true,
            &DEACTIVATE_COMMANDS,
        ),
        PHASE_SESSION_TWO_DEACTIVATED => (
            matches!(
                trace,
                TextChannelTrace::Event { event, .. }
                    if exact_context(event.context(), SESSION_TWO, SESSION_TWO_FOCUS)
                        && event.sequence() == 24
                        && matches!(
                            event.payload(),
                            TextInputEventPayload::Deactivated {
                                acknowledged_command_sequence: 14,
                                revision: 5,
                            }
                        )
            ),
            false,
            &DEACTIVATED_EVENTS,
        ),
        PHASE_SESSION_THREE_ACTIVATE => (
            matches!(
                trace,
                TextChannelTrace::Command { command, .. }
                    if exact_context(command.context(), SESSION_THREE, SESSION_THREE_FOCUS)
                        && command.sequence() == 15
                        && matches!(command.payload(), TextInputCommandPayload::Activate)
            ),
            true,
            &ACTIVATE_COMMANDS,
        ),
        PHASE_SESSION_THREE_ACTIVATED => (
            matches!(
                trace,
                TextChannelTrace::Event { event, .. }
                    if exact_context(event.context(), SESSION_THREE, SESSION_THREE_FOCUS)
                        && event.sequence() == 25
                        && matches!(
                            event.payload(),
                            TextInputEventPayload::Activated {
                                acknowledged_command_sequence: 15,
                                revision: 0,
                            }
                        )
            ),
            false,
            &ACTIVATED_EVENTS,
        ),
        PHASE_OVERLAY_SHOW_ONE
        | PHASE_EDITOR_OUTPUT_ONE
        | PHASE_EDITOR_OUTPUT_TWO
        | PHASE_EDITOR_OUTPUT_THREE
        | PHASE_EDITOR_OUTPUT_FOUR
        | PHASE_EDITOR_OUTPUT_FIVE
        | PHASE_OVERLAY_HIDE
        | PHASE_OVERLAY_SHOW_TWO => return reject_phase(),
        _ => return reject_phase(),
    };

    if !valid {
        return reject_payload();
    }
    if PHASE
        .compare_exchange(phase, phase + 1, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return reject_phase();
    }

    counter.fetch_add(1, Ordering::Relaxed);
    if command {
        COMMANDS.fetch_add(1, Ordering::Relaxed);
    } else {
        EVENTS.fetch_add(1, Ordering::Relaxed);
    }
    MESSAGES.fetch_add(1, Ordering::Relaxed);
    if phase + 1 == PHASE_COMPLETE {
        FINAL_COMPLETE.store(true, Ordering::Release);
    }
    true
}

/// Records one exact graphics commit in the M44 overlay/editor schedule.
pub fn record_output_commit(
    slot: usize,
    allocation_generation: u32,
    write_generation: u64,
    frame_id: u32,
) -> bool {
    if !prefix_complete() {
        return reject_prefix();
    }

    let phase = PHASE.load(Ordering::Acquire);
    let Some(index) = OUTPUT_PHASES.iter().position(|expected| *expected == phase) else {
        return reject_phase();
    };
    if slot != OUTPUT_SLOTS[index]
        || allocation_generation != 1
        || write_generation != OUTPUT_WRITE_GENERATIONS[index]
        || frame_id != 14 + index as u32
    {
        return reject_payload();
    }
    if PHASE
        .compare_exchange(phase, phase + 1, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return reject_phase();
    }

    OUTPUT_COMMITS.fetch_add(1, Ordering::Relaxed);
    match index {
        0 | 7 => {
            OVERLAY_SHOWS.fetch_add(1, Ordering::Relaxed);
        }
        1..=5 => {
            EDITOR_OUTPUTS.fetch_add(1, Ordering::Relaxed);
        }
        6 => {
            OVERLAY_HIDES.fetch_add(1, Ordering::Relaxed);
        }
        _ => unreachable!("fixed M44 output schedule escaped its domain"),
    }
    true
}

/// Accounts for a wire rejected before a [`TextChannelTrace`] could be built.
pub fn record_invalid_wire(owner_error: bool) -> bool {
    ERRORS.fetch_add(1, Ordering::Relaxed);
    if owner_error {
        OWNER_ERRORS.fetch_add(1, Ordering::Relaxed);
    } else {
        PAYLOAD_ERRORS.fetch_add(1, Ordering::Relaxed);
    }
    false
}

pub fn snapshot() -> SoftKeyboardTraceSnapshot {
    SoftKeyboardTraceSnapshot {
        phase: decode_phase(PHASE.load(Ordering::Acquire)),
        messages: MESSAGES.load(Ordering::Acquire),
        commands: COMMANDS.load(Ordering::Acquire),
        events: EVENTS.load(Ordering::Acquire),
        activate_commands: ACTIVATE_COMMANDS.load(Ordering::Acquire),
        ack_commands: ACK_COMMANDS.load(Ordering::Acquire),
        deactivate_commands: DEACTIVATE_COMMANDS.load(Ordering::Acquire),
        activated_events: ACTIVATED_EVENTS.load(Ordering::Acquire),
        preedit_events: PREEDIT_EVENTS.load(Ordering::Acquire),
        commit_events: COMMIT_EVENTS.load(Ordering::Acquire),
        delete_events: DELETE_EVENTS.load(Ordering::Acquire),
        rendered_events: RENDERED_EVENTS.load(Ordering::Acquire),
        deactivated_events: DEACTIVATED_EVENTS.load(Ordering::Acquire),
        output_commits: OUTPUT_COMMITS.load(Ordering::Acquire),
        editor_outputs: EDITOR_OUTPUTS.load(Ordering::Acquire),
        overlay_shows: OVERLAY_SHOWS.load(Ordering::Acquire),
        overlay_hides: OVERLAY_HIDES.load(Ordering::Acquire),
        errors: ERRORS.load(Ordering::Acquire),
        owner_errors: OWNER_ERRORS.load(Ordering::Acquire),
        payload_errors: PAYLOAD_ERRORS.load(Ordering::Acquire),
        phase_errors: PHASE_ERRORS.load(Ordering::Acquire),
        prefix_errors: PREFIX_ERRORS.load(Ordering::Acquire),
        final_complete: FINAL_COMPLETE.load(Ordering::Acquire),
    }
}

fn exact_roles(trace: TextChannelTrace) -> bool {
    matches!(
        trace,
        TextChannelTrace::Command {
            sender_role: UiTraceRole::App,
            receiver_role: UiTraceRole::SurfaceServer,
            ..
        } | TextChannelTrace::Event {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            ..
        }
    )
}

fn exact_context(context: bndr_ui::TextInputContext, session: u64, focus: u64) -> bool {
    context.window_id().token() == APP_WINDOW_TOKEN
        && context.session_id() == session
        && context.focus_generation() == focus
}

fn expected_preedit(
    trace: TextChannelTrace,
    session: u64,
    focus: u64,
    sequence: u64,
    revision: u32,
) -> bool {
    matches!(
        trace,
        TextChannelTrace::Event { event, .. }
            if exact_context(event.context(), session, focus)
                && event.sequence() == sequence
                && matches!(
                    event.payload(),
                    TextInputEventPayload::Preedit { revision: seen, scalar: 'a' }
                        if seen == revision
                )
    )
}

fn expected_commit(trace: TextChannelTrace, sequence: u64, revision: u32) -> bool {
    matches!(
        trace,
        TextChannelTrace::Event { event, .. }
            if exact_context(event.context(), SESSION_TWO, SESSION_TWO_FOCUS)
                && event.sequence() == sequence
                && matches!(
                    event.payload(),
                    TextInputEventPayload::Commit { revision: seen, text }
                        if seen == revision && text.as_str() == "a"
                )
    )
}

fn expected_ack(
    trace: TextChannelTrace,
    sequence: u64,
    acknowledged_event: u64,
    revision: u32,
    committed: &str,
    selection: u8,
) -> bool {
    matches!(
        trace,
        TextChannelTrace::Command { command, .. }
            if exact_context(command.context(), SESSION_TWO, SESSION_TWO_FOCUS)
                && command.sequence() == sequence
                && matches!(
                    command.payload(),
                    TextInputCommandPayload::StateAck {
                        acknowledged_event_sequence,
                        state,
                    } if acknowledged_event_sequence == acknowledged_event
                        && exact_state(state, revision, committed, selection)
                )
    )
}

fn expected_rendered(
    trace: TextChannelTrace,
    sequence: u64,
    acknowledged_command: u64,
    revision: u32,
    committed: &str,
    selection: u8,
) -> bool {
    matches!(
        trace,
        TextChannelTrace::Event { event, .. }
            if exact_context(event.context(), SESSION_TWO, SESSION_TWO_FOCUS)
                && event.sequence() == sequence
                && matches!(
                    event.payload(),
                    TextInputEventPayload::Rendered {
                        acknowledged_command_sequence,
                        state,
                    } if acknowledged_command_sequence == acknowledged_command
                        && exact_state(state, revision, committed, selection)
                )
    )
}

fn exact_state(state: TextState, revision: u32, committed: &str, selection: u8) -> bool {
    state.revision() == revision
        && state.committed().as_str() == committed
        && state.selection_start() == selection
        && state.selection_end() == selection
}

#[cfg(not(test))]
fn prefix_complete() -> bool {
    crate::text_input_trace::snapshot().final_complete
}

#[cfg(test)]
static TEST_PREFIX_COMPLETE: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
fn prefix_complete() -> bool {
    TEST_PREFIX_COMPLETE.load(Ordering::Acquire)
}

fn reject_owner() -> bool {
    ERRORS.fetch_add(1, Ordering::Relaxed);
    OWNER_ERRORS.fetch_add(1, Ordering::Relaxed);
    false
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

fn reject_prefix() -> bool {
    ERRORS.fetch_add(1, Ordering::Relaxed);
    PREFIX_ERRORS.fetch_add(1, Ordering::Relaxed);
    false
}

const fn decode_phase(phase: u64) -> SoftKeyboardTracePhase {
    match phase {
        PHASE_SESSION_TWO_ACTIVATE => SoftKeyboardTracePhase::SessionTwoActivate,
        PHASE_OVERLAY_SHOW_ONE => SoftKeyboardTracePhase::OverlayShowOne,
        PHASE_SESSION_TWO_ACTIVATED => SoftKeyboardTracePhase::SessionTwoActivated,
        PHASE_PREEDIT_ONE => SoftKeyboardTracePhase::PreeditOne,
        PHASE_ACK_ONE => SoftKeyboardTracePhase::AckOne,
        PHASE_EDITOR_OUTPUT_ONE => SoftKeyboardTracePhase::EditorOutputOne,
        PHASE_RENDERED_ONE => SoftKeyboardTracePhase::RenderedOne,
        PHASE_COMMIT_ONE => SoftKeyboardTracePhase::CommitOne,
        PHASE_ACK_TWO => SoftKeyboardTracePhase::AckTwo,
        PHASE_EDITOR_OUTPUT_TWO => SoftKeyboardTracePhase::EditorOutputTwo,
        PHASE_RENDERED_TWO => SoftKeyboardTracePhase::RenderedTwo,
        PHASE_DELETE => SoftKeyboardTracePhase::Delete,
        PHASE_ACK_THREE => SoftKeyboardTracePhase::AckThree,
        PHASE_EDITOR_OUTPUT_THREE => SoftKeyboardTracePhase::EditorOutputThree,
        PHASE_RENDERED_THREE => SoftKeyboardTracePhase::RenderedThree,
        PHASE_PREEDIT_TWO => SoftKeyboardTracePhase::PreeditTwo,
        PHASE_ACK_FOUR => SoftKeyboardTracePhase::AckFour,
        PHASE_EDITOR_OUTPUT_FOUR => SoftKeyboardTracePhase::EditorOutputFour,
        PHASE_RENDERED_FOUR => SoftKeyboardTracePhase::RenderedFour,
        PHASE_COMMIT_TWO => SoftKeyboardTracePhase::CommitTwo,
        PHASE_ACK_FIVE => SoftKeyboardTracePhase::AckFive,
        PHASE_EDITOR_OUTPUT_FIVE => SoftKeyboardTracePhase::EditorOutputFive,
        PHASE_RENDERED_FIVE => SoftKeyboardTracePhase::RenderedFive,
        PHASE_SESSION_TWO_DEACTIVATE => SoftKeyboardTracePhase::SessionTwoDeactivate,
        PHASE_OVERLAY_HIDE => SoftKeyboardTracePhase::OverlayHide,
        PHASE_SESSION_TWO_DEACTIVATED => SoftKeyboardTracePhase::SessionTwoDeactivated,
        PHASE_SESSION_THREE_ACTIVATE => SoftKeyboardTracePhase::SessionThreeActivate,
        PHASE_OVERLAY_SHOW_TWO => SoftKeyboardTracePhase::OverlayShowTwo,
        PHASE_SESSION_THREE_ACTIVATED => SoftKeyboardTracePhase::SessionThreeActivated,
        PHASE_COMPLETE => SoftKeyboardTracePhase::Complete,
        _ => SoftKeyboardTracePhase::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use bndr_ui::{TextInputCommand, TextInputContext, TextInputEvent, WindowId};

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn context(session: u64, focus: u64) -> TextInputContext {
        TextInputContext::try_new(WindowId::try_new(1, 2).unwrap(), session, focus).unwrap()
    }

    fn state(revision: u32, committed: &str, selection: u8) -> TextState {
        TextState::try_new(revision, committed, selection, selection).unwrap()
    }

    fn command(command: TextInputCommand) -> TextChannelTrace {
        TextChannelTrace::Command {
            sender_role: UiTraceRole::App,
            receiver_role: UiTraceRole::SurfaceServer,
            command,
        }
    }

    fn event(event: TextInputEvent) -> TextChannelTrace {
        TextChannelTrace::Event {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            event,
        }
    }

    fn output(index: usize) {
        assert!(record_output_commit(
            OUTPUT_SLOTS[index],
            1,
            OUTPUT_WRITE_GENERATIONS[index],
            14 + index as u32,
        ));
    }

    fn reset(prefix_complete: bool) {
        for counter in [
            &MESSAGES,
            &COMMANDS,
            &EVENTS,
            &ACTIVATE_COMMANDS,
            &ACK_COMMANDS,
            &DEACTIVATE_COMMANDS,
            &ACTIVATED_EVENTS,
            &PREEDIT_EVENTS,
            &COMMIT_EVENTS,
            &DELETE_EVENTS,
            &RENDERED_EVENTS,
            &DEACTIVATED_EVENTS,
            &OUTPUT_COMMITS,
            &EDITOR_OUTPUTS,
            &OVERLAY_SHOWS,
            &OVERLAY_HIDES,
            &ERRORS,
            &OWNER_ERRORS,
            &PAYLOAD_ERRORS,
            &PHASE_ERRORS,
            &PREFIX_ERRORS,
        ] {
            counter.store(0, Ordering::Release);
        }
        PHASE.store(PHASE_SESSION_TWO_ACTIVATE, Ordering::Release);
        FINAL_COMPLETE.store(false, Ordering::Release);
        TEST_PREFIX_COMPLETE.store(prefix_complete, Ordering::Release);
    }

    fn run_exact_extension() {
        let session_two = context(SESSION_TWO, SESSION_TWO_FOCUS);
        assert!(record(command(
            TextInputCommand::activate(8, session_two).unwrap()
        )));
        output(0);
        assert!(record(event(
            TextInputEvent::activated(13, session_two, 8, 0).unwrap()
        )));

        let edits = [
            (
                TextInputEvent::preedit(14, session_two, 1, 'a').unwrap(),
                TextInputCommand::state_ack(9, session_two, 14, state(1, "a", 1)).unwrap(),
                TextInputEvent::rendered(15, session_two, 9, state(1, "a", 1)).unwrap(),
            ),
            (
                TextInputEvent::commit(16, session_two, 2, "a").unwrap(),
                TextInputCommand::state_ack(10, session_two, 16, state(2, "aa", 2)).unwrap(),
                TextInputEvent::rendered(17, session_two, 10, state(2, "aa", 2)).unwrap(),
            ),
            (
                TextInputEvent::delete_surrounding(18, session_two, 3, 1, 0).unwrap(),
                TextInputCommand::state_ack(11, session_two, 18, state(3, "a", 1)).unwrap(),
                TextInputEvent::rendered(19, session_two, 11, state(3, "a", 1)).unwrap(),
            ),
            (
                TextInputEvent::preedit(20, session_two, 4, 'a').unwrap(),
                TextInputCommand::state_ack(12, session_two, 20, state(4, "a", 1)).unwrap(),
                TextInputEvent::rendered(21, session_two, 12, state(4, "a", 1)).unwrap(),
            ),
            (
                TextInputEvent::commit(22, session_two, 5, "a").unwrap(),
                TextInputCommand::state_ack(13, session_two, 22, state(5, "aa", 2)).unwrap(),
                TextInputEvent::rendered(23, session_two, 13, state(5, "aa", 2)).unwrap(),
            ),
        ];
        for (index, (edit, ack, rendered)) in edits.into_iter().enumerate() {
            assert!(record(event(edit)));
            assert!(record(command(ack)));
            output(index + 1);
            assert!(record(event(rendered)));
        }

        assert!(record(command(
            TextInputCommand::deactivate(14, session_two).unwrap()
        )));
        output(6);
        assert!(record(event(
            TextInputEvent::deactivated(24, session_two, 14, 5).unwrap()
        )));

        let session_three = context(SESSION_THREE, SESSION_THREE_FOCUS);
        assert!(record(command(
            TextInputCommand::activate(15, session_three).unwrap()
        )));
        output(7);
        assert!(record(event(
            TextInputEvent::activated(25, session_three, 15, 0).unwrap()
        )));
    }

    #[test]
    fn requires_the_completed_m43_prefix_and_accounts_for_invalid_wires() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        reset(false);
        let session = context(SESSION_TWO, SESSION_TWO_FOCUS);

        assert!(!record(command(
            TextInputCommand::activate(8, session).unwrap()
        )));
        assert!(!record_output_commit(1, 1, 7, 14));
        assert!(!record_invalid_wire(true));
        assert!(!record_invalid_wire(false));

        let seen = snapshot();
        assert_eq!(seen.phase, SoftKeyboardTracePhase::SessionTwoActivate);
        assert_eq!((seen.messages, seen.output_commits), (0, 0));
        assert_eq!((seen.errors, seen.prefix_errors), (4, 2));
        assert_eq!((seen.owner_errors, seen.payload_errors), (1, 1));
        assert!(!seen.final_complete);
    }

    #[test]
    fn exact_extension_reaches_complete_and_rejects_all_post_complete_traffic() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        reset(true);
        run_exact_extension();

        let complete = snapshot();
        assert_eq!(complete.phase, SoftKeyboardTracePhase::Complete);
        assert_eq!(
            (complete.messages, complete.commands, complete.events),
            (21, 8, 13)
        );
        assert_eq!(
            (
                complete.activate_commands,
                complete.ack_commands,
                complete.deactivate_commands,
            ),
            (2, 5, 1)
        );
        assert_eq!(
            (
                complete.activated_events,
                complete.preedit_events,
                complete.commit_events,
                complete.delete_events,
                complete.rendered_events,
                complete.deactivated_events,
            ),
            (2, 2, 2, 1, 5, 1)
        );
        assert_eq!(
            (
                complete.output_commits,
                complete.editor_outputs,
                complete.overlay_shows,
                complete.overlay_hides,
            ),
            (8, 5, 2, 1)
        );
        assert_eq!(complete.errors, 0);
        assert!(complete.final_complete);

        let session_three = context(SESSION_THREE, SESSION_THREE_FOCUS);
        assert!(!record(event(
            TextInputEvent::activated(25, session_three, 15, 0).unwrap()
        )));
        assert!(!record_output_commit(1, 1, 12, 22));
        let rejected = snapshot();
        assert_eq!(rejected.phase, SoftKeyboardTracePhase::Complete);
        assert_eq!(
            (rejected.messages, rejected.output_commits),
            (complete.messages, complete.output_commits)
        );
        assert_eq!((rejected.errors, rejected.phase_errors), (2, 2));
        assert!(rejected.final_complete);
    }

    #[test]
    fn rejects_order_owner_context_state_and_output_drift_transactionally() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        reset(true);
        let session = context(SESSION_TWO, SESSION_TWO_FOCUS);
        let wrong_context = context(SESSION_TWO, SESSION_TWO_FOCUS + 1);

        assert!(!record(command(
            TextInputCommand::activate(8, wrong_context).unwrap()
        )));
        let wrong_owner = TextChannelTrace::Command {
            sender_role: UiTraceRole::Launcher,
            receiver_role: UiTraceRole::SurfaceServer,
            command: TextInputCommand::activate(8, session).unwrap(),
        };
        assert!(!record(wrong_owner));
        assert_eq!(snapshot().phase, SoftKeyboardTracePhase::SessionTwoActivate);
        assert_eq!(snapshot().messages, 0);

        assert!(record(command(
            TextInputCommand::activate(8, session).unwrap()
        )));
        assert!(!record(event(
            TextInputEvent::activated(13, session, 8, 0).unwrap()
        )));
        assert_eq!(snapshot().phase, SoftKeyboardTracePhase::OverlayShowOne);
        assert_eq!(snapshot().output_commits, 0);

        for (slot, allocation, write, frame) in
            [(0, 1, 7, 14), (1, 2, 7, 14), (1, 1, 8, 14), (1, 1, 7, 15)]
        {
            assert!(!record_output_commit(slot, allocation, write, frame));
            assert_eq!(snapshot().phase, SoftKeyboardTracePhase::OverlayShowOne);
            assert_eq!(snapshot().output_commits, 0);
        }
        output(0);
        assert!(record(event(
            TextInputEvent::activated(13, session, 8, 0).unwrap()
        )));
        assert!(record(event(
            TextInputEvent::preedit(14, session, 1, 'a').unwrap()
        )));

        assert!(!record(command(
            TextInputCommand::state_ack(9, session, 14, state(1, "", 0)).unwrap()
        )));
        assert_eq!(snapshot().phase, SoftKeyboardTracePhase::AckOne);
        assert_eq!(snapshot().ack_commands, 0);
        assert!(record(command(
            TextInputCommand::state_ack(9, session, 14, state(1, "a", 1)).unwrap()
        )));

        let seen = snapshot();
        assert_eq!(seen.phase, SoftKeyboardTracePhase::EditorOutputOne);
        assert_eq!((seen.messages, seen.commands, seen.events), (4, 2, 2));
        assert_eq!((seen.owner_errors, seen.payload_errors), (1, 6));
        assert_eq!(seen.phase_errors, 1);
        assert_eq!(seen.errors, 8);
    }
}
