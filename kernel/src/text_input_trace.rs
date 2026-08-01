//! Immutable ABI-21 evidence for one focus-scoped text-input session.
//!
//! The complete M41/M42 window transcript remains owned by the earlier trace
//! modules. This extension accepts only the App slot-1 generation-2 context,
//! one BTI1/BTE1 session at compositor focus generation 5, five transactional
//! editor revisions, five post-ack output commits, and an explicit focus-loss
//! deactivation. Canonical traffic is decoded and process-authenticated by the
//! syscall layer before it reaches this state machine.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bndr_ui::{TextInputCommandPayload, TextInputEventPayload, TextState};

use crate::ui_trace::{TextChannelTrace, UiTraceRole};

const APP_WINDOW_TOKEN: u64 = (2_u64 << 32) | 1;
const TEXT_SESSION_ID: u64 = 1;
const FOCUS_GENERATION: u64 = 5;
const OUTPUT_FRAMES: usize = 5;
const OUTPUT_SLOTS: [usize; OUTPUT_FRAMES] = [0, 1, 0, 1, 0];
const OUTPUT_WRITE_GENERATIONS: [u64; OUTPUT_FRAMES] = [5, 5, 6, 6, 7];

const PHASE_ACTIVATE: u64 = 0;
const PHASE_ACTIVATED: u64 = 1;
const PHASE_PREEDIT_ONE: u64 = 2;
const PHASE_ACK_ONE: u64 = 3;
const PHASE_RENDERED_ONE: u64 = 4;
const PHASE_COMMIT_ONE: u64 = 5;
const PHASE_ACK_TWO: u64 = 6;
const PHASE_RENDERED_TWO: u64 = 7;
const PHASE_DELETE: u64 = 8;
const PHASE_ACK_THREE: u64 = 9;
const PHASE_RENDERED_THREE: u64 = 10;
const PHASE_PREEDIT_TWO: u64 = 11;
const PHASE_ACK_FOUR: u64 = 12;
const PHASE_RENDERED_FOUR: u64 = 13;
const PHASE_COMMIT_TWO: u64 = 14;
const PHASE_ACK_FIVE: u64 = 15;
const PHASE_RENDERED_FIVE: u64 = 16;
const PHASE_DEACTIVATE: u64 = 17;
const PHASE_DEACTIVATED: u64 = 18;
const PHASE_COMPLETE: u64 = 19;

static PHASE: AtomicU64 = AtomicU64::new(PHASE_ACTIVATE);
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
static ERRORS: AtomicU64 = AtomicU64::new(0);
static OWNER_ERRORS: AtomicU64 = AtomicU64::new(0);
static PAYLOAD_ERRORS: AtomicU64 = AtomicU64::new(0);
static PHASE_ERRORS: AtomicU64 = AtomicU64::new(0);
static FINAL_COMPLETE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextInputTracePhase {
    Activate,
    Activated,
    PreeditOne,
    AckOne,
    RenderedOne,
    CommitOne,
    AckTwo,
    RenderedTwo,
    Delete,
    AckThree,
    RenderedThree,
    PreeditTwo,
    AckFour,
    RenderedFour,
    CommitTwo,
    AckFive,
    RenderedFive,
    Deactivate,
    Deactivated,
    Complete,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextInputTraceSnapshot {
    pub phase: TextInputTracePhase,
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
    pub errors: u64,
    pub owner_errors: u64,
    pub payload_errors: u64,
    pub phase_errors: u64,
    pub final_complete: bool,
}

/// Records one canonical and authenticated BTI1/BTE1 delivery.
pub fn record(trace: TextChannelTrace) -> bool {
    let phase = PHASE.load(Ordering::Acquire);
    if phase == PHASE_COMPLETE {
        let _ = trace;
        return true;
    }

    let (valid, command, counter): (bool, bool, &AtomicU64) = match phase {
        PHASE_ACTIVATE => (
            matches!(
                trace,
                TextChannelTrace::Command {
                    sender_role: UiTraceRole::App,
                    receiver_role: UiTraceRole::SurfaceServer,
                    command,
                } if exact_context(command.context())
                    && command.sequence() == 1
                    && matches!(command.payload(), TextInputCommandPayload::Activate)
            ),
            true,
            &ACTIVATE_COMMANDS,
        ),
        PHASE_ACTIVATED => (
            matches!(
                trace,
                TextChannelTrace::Event {
                    sender_role: UiTraceRole::SurfaceServer,
                    receiver_role: UiTraceRole::App,
                    event,
                } if exact_context(event.context())
                    && event.sequence() == 1
                    && matches!(
                        event.payload(),
                        TextInputEventPayload::Activated {
                            acknowledged_command_sequence: 1,
                            revision: 0,
                        }
                    )
            ),
            false,
            &ACTIVATED_EVENTS,
        ),
        PHASE_PREEDIT_ONE => (expected_preedit(trace, 2, 1), false, &PREEDIT_EVENTS),
        PHASE_ACK_ONE => (
            expected_ack(trace, 2, 2, empty_state(1)),
            true,
            &ACK_COMMANDS,
        ),
        PHASE_RENDERED_ONE => (
            expected_rendered(trace, 3, 2, empty_state(1), 1),
            false,
            &RENDERED_EVENTS,
        ),
        PHASE_COMMIT_ONE => (expected_commit(trace, 4, 2), false, &COMMIT_EVENTS),
        PHASE_ACK_TWO => (expected_ack(trace, 3, 4, a_state(2)), true, &ACK_COMMANDS),
        PHASE_RENDERED_TWO => (
            expected_rendered(trace, 5, 3, a_state(2), 2),
            false,
            &RENDERED_EVENTS,
        ),
        PHASE_DELETE => (expected_delete(trace), false, &DELETE_EVENTS),
        PHASE_ACK_THREE => (
            expected_ack(trace, 4, 6, empty_state(3)),
            true,
            &ACK_COMMANDS,
        ),
        PHASE_RENDERED_THREE => (
            expected_rendered(trace, 7, 4, empty_state(3), 3),
            false,
            &RENDERED_EVENTS,
        ),
        PHASE_PREEDIT_TWO => (expected_preedit(trace, 8, 4), false, &PREEDIT_EVENTS),
        PHASE_ACK_FOUR => (
            expected_ack(trace, 5, 8, empty_state(4)),
            true,
            &ACK_COMMANDS,
        ),
        PHASE_RENDERED_FOUR => (
            expected_rendered(trace, 9, 5, empty_state(4), 4),
            false,
            &RENDERED_EVENTS,
        ),
        PHASE_COMMIT_TWO => (expected_commit(trace, 10, 5), false, &COMMIT_EVENTS),
        PHASE_ACK_FIVE => (expected_ack(trace, 6, 10, a_state(5)), true, &ACK_COMMANDS),
        PHASE_RENDERED_FIVE => (
            expected_rendered(trace, 11, 6, a_state(5), 5),
            false,
            &RENDERED_EVENTS,
        ),
        PHASE_DEACTIVATE => (
            matches!(
                trace,
                TextChannelTrace::Command {
                    sender_role: UiTraceRole::App,
                    receiver_role: UiTraceRole::SurfaceServer,
                    command,
                } if exact_context(command.context())
                    && command.sequence() == 7
                    && matches!(command.payload(), TextInputCommandPayload::Deactivate)
            ),
            true,
            &DEACTIVATE_COMMANDS,
        ),
        PHASE_DEACTIVATED => (
            matches!(
                trace,
                TextChannelTrace::Event {
                    sender_role: UiTraceRole::SurfaceServer,
                    receiver_role: UiTraceRole::App,
                    event,
                } if exact_context(event.context())
                    && event.sequence() == 12
                    && matches!(
                        event.payload(),
                        TextInputEventPayload::Deactivated {
                            acknowledged_command_sequence: 7,
                            revision: 5,
                        }
                    )
            ),
            false,
            &DEACTIVATED_EVENTS,
        ),
        _ => return reject_phase(),
    };

    if !valid {
        return reject_payload();
    }
    counter.fetch_add(1, Ordering::Relaxed);
    if command {
        COMMANDS.fetch_add(1, Ordering::Relaxed);
    } else {
        EVENTS.fetch_add(1, Ordering::Relaxed);
    }
    MESSAGES.fetch_add(1, Ordering::Relaxed);
    let next = phase + 1;
    PHASE.store(next, Ordering::Release);
    if next == PHASE_COMPLETE {
        FINAL_COMPLETE.store(true, Ordering::Release);
    }
    true
}

/// Links each rendered state acknowledgment to the exact paced output frame.
/// Frames 1..=8 are the immutable M41/M42 prefix and remain silent here.
pub fn record_output_commit(
    slot: usize,
    allocation_generation: u32,
    write_generation: u64,
    frame_id: u32,
) -> bool {
    if frame_id <= 8 {
        return true;
    }
    let index = OUTPUT_COMMITS.load(Ordering::Acquire) as usize;
    if index >= OUTPUT_FRAMES {
        return reject_phase();
    }
    let expected_phase = [
        PHASE_RENDERED_ONE,
        PHASE_RENDERED_TWO,
        PHASE_RENDERED_THREE,
        PHASE_RENDERED_FOUR,
        PHASE_RENDERED_FIVE,
    ][index];
    let valid = PHASE.load(Ordering::Acquire) == expected_phase
        && slot == OUTPUT_SLOTS[index]
        && allocation_generation == 1
        && write_generation == OUTPUT_WRITE_GENERATIONS[index]
        && frame_id == 9 + index as u32;
    if !valid {
        return reject_payload();
    }
    OUTPUT_COMMITS.fetch_add(1, Ordering::Release);
    true
}

pub fn record_invalid_wire(owner_error: bool) -> bool {
    ERRORS.fetch_add(1, Ordering::Relaxed);
    if owner_error {
        OWNER_ERRORS.fetch_add(1, Ordering::Relaxed);
    } else {
        PAYLOAD_ERRORS.fetch_add(1, Ordering::Relaxed);
    }
    false
}

pub fn snapshot() -> TextInputTraceSnapshot {
    TextInputTraceSnapshot {
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
        errors: ERRORS.load(Ordering::Acquire),
        owner_errors: OWNER_ERRORS.load(Ordering::Acquire),
        payload_errors: PAYLOAD_ERRORS.load(Ordering::Acquire),
        phase_errors: PHASE_ERRORS.load(Ordering::Acquire),
        final_complete: FINAL_COMPLETE.load(Ordering::Acquire),
    }
}

fn exact_context(context: bndr_ui::TextInputContext) -> bool {
    context.window_id().token() == APP_WINDOW_TOKEN
        && context.session_id() == TEXT_SESSION_ID
        && context.focus_generation() == FOCUS_GENERATION
}

fn expected_preedit(trace: TextChannelTrace, sequence: u64, revision: u32) -> bool {
    matches!(
        trace,
        TextChannelTrace::Event {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            event,
        } if exact_context(event.context())
            && event.sequence() == sequence
            && matches!(
                event.payload(),
                TextInputEventPayload::Preedit { revision: seen, scalar: 'a' } if seen == revision
            )
    )
}

fn expected_commit(trace: TextChannelTrace, sequence: u64, revision: u32) -> bool {
    matches!(
        trace,
        TextChannelTrace::Event {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            event,
        } if exact_context(event.context())
            && event.sequence() == sequence
            && matches!(
                event.payload(),
                TextInputEventPayload::Commit { revision: seen, text }
                    if seen == revision && text.as_str() == "a"
            )
    )
}

fn expected_delete(trace: TextChannelTrace) -> bool {
    matches!(
        trace,
        TextChannelTrace::Event {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            event,
        } if exact_context(event.context())
            && event.sequence() == 6
            && matches!(
                event.payload(),
                TextInputEventPayload::DeleteSurrounding {
                    revision: 3,
                    before_scalars: 1,
                    after_scalars: 0,
                }
            )
    )
}

fn expected_ack(
    trace: TextChannelTrace,
    sequence: u64,
    acknowledged_event: u64,
    expected_state: TextState,
) -> bool {
    matches!(
        trace,
        TextChannelTrace::Command {
            sender_role: UiTraceRole::App,
            receiver_role: UiTraceRole::SurfaceServer,
            command,
        } if exact_context(command.context())
            && command.sequence() == sequence
            && matches!(
                command.payload(),
                TextInputCommandPayload::StateAck {
                    acknowledged_event_sequence,
                    state,
                } if acknowledged_event_sequence == acknowledged_event && state == expected_state
            )
    )
}

fn expected_rendered(
    trace: TextChannelTrace,
    sequence: u64,
    acknowledged_command: u64,
    expected_state: TextState,
    required_outputs: u64,
) -> bool {
    OUTPUT_COMMITS.load(Ordering::Acquire) == required_outputs
        && matches!(
            trace,
            TextChannelTrace::Event {
                sender_role: UiTraceRole::SurfaceServer,
                receiver_role: UiTraceRole::App,
                event,
            } if exact_context(event.context())
                && event.sequence() == sequence
                && matches!(
                    event.payload(),
                    TextInputEventPayload::Rendered {
                        acknowledged_command_sequence,
                        state,
                    } if acknowledged_command_sequence == acknowledged_command
                        && state == expected_state
                )
        )
}

fn empty_state(revision: u32) -> TextState {
    TextState::try_new(revision, "", 0, 0).expect("static empty text state")
}

fn a_state(revision: u32) -> TextState {
    TextState::try_new(revision, "a", 1, 1).expect("static Latin text state")
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

const fn decode_phase(phase: u64) -> TextInputTracePhase {
    match phase {
        PHASE_ACTIVATE => TextInputTracePhase::Activate,
        PHASE_ACTIVATED => TextInputTracePhase::Activated,
        PHASE_PREEDIT_ONE => TextInputTracePhase::PreeditOne,
        PHASE_ACK_ONE => TextInputTracePhase::AckOne,
        PHASE_RENDERED_ONE => TextInputTracePhase::RenderedOne,
        PHASE_COMMIT_ONE => TextInputTracePhase::CommitOne,
        PHASE_ACK_TWO => TextInputTracePhase::AckTwo,
        PHASE_RENDERED_TWO => TextInputTracePhase::RenderedTwo,
        PHASE_DELETE => TextInputTracePhase::Delete,
        PHASE_ACK_THREE => TextInputTracePhase::AckThree,
        PHASE_RENDERED_THREE => TextInputTracePhase::RenderedThree,
        PHASE_PREEDIT_TWO => TextInputTracePhase::PreeditTwo,
        PHASE_ACK_FOUR => TextInputTracePhase::AckFour,
        PHASE_RENDERED_FOUR => TextInputTracePhase::RenderedFour,
        PHASE_COMMIT_TWO => TextInputTracePhase::CommitTwo,
        PHASE_ACK_FIVE => TextInputTracePhase::AckFive,
        PHASE_RENDERED_FIVE => TextInputTracePhase::RenderedFive,
        PHASE_DEACTIVATE => TextInputTracePhase::Deactivate,
        PHASE_DEACTIVATED => TextInputTracePhase::Deactivated,
        PHASE_COMPLETE => TextInputTracePhase::Complete,
        _ => TextInputTracePhase::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bndr_ui::{TextInputCommand, TextInputContext, TextInputEvent, WindowId};

    fn context() -> TextInputContext {
        TextInputContext::try_new(WindowId::try_new(1, 2).unwrap(), 1, 5).unwrap()
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

    fn output(step: usize) {
        assert!(record_output_commit(
            OUTPUT_SLOTS[step],
            1,
            OUTPUT_WRITE_GENERATIONS[step],
            9 + step as u32,
        ));
    }

    #[test]
    fn exact_text_session_reaches_immutable_complete_state() {
        let context = context();
        assert!(record(command(
            TextInputCommand::activate(1, context).unwrap()
        )));
        assert!(record(event(
            TextInputEvent::activated(1, context, 1, 0).unwrap()
        )));
        let transcript = [
            (
                TextInputEvent::preedit(2, context, 1, 'a').unwrap(),
                TextInputCommand::state_ack(2, context, 2, empty_state(1)).unwrap(),
                TextInputEvent::rendered(3, context, 2, empty_state(1)).unwrap(),
            ),
            (
                TextInputEvent::commit(4, context, 2, "a").unwrap(),
                TextInputCommand::state_ack(3, context, 4, a_state(2)).unwrap(),
                TextInputEvent::rendered(5, context, 3, a_state(2)).unwrap(),
            ),
            (
                TextInputEvent::delete_surrounding(6, context, 3, 1, 0).unwrap(),
                TextInputCommand::state_ack(4, context, 6, empty_state(3)).unwrap(),
                TextInputEvent::rendered(7, context, 4, empty_state(3)).unwrap(),
            ),
            (
                TextInputEvent::preedit(8, context, 4, 'a').unwrap(),
                TextInputCommand::state_ack(5, context, 8, empty_state(4)).unwrap(),
                TextInputEvent::rendered(9, context, 5, empty_state(4)).unwrap(),
            ),
            (
                TextInputEvent::commit(10, context, 5, "a").unwrap(),
                TextInputCommand::state_ack(6, context, 10, a_state(5)).unwrap(),
                TextInputEvent::rendered(11, context, 6, a_state(5)).unwrap(),
            ),
        ];
        for (step, (edit, ack, rendered)) in transcript.into_iter().enumerate() {
            assert!(record(event(edit)));
            assert!(record(command(ack)));
            output(step);
            assert!(record(event(rendered)));
        }
        assert!(record(command(
            TextInputCommand::deactivate(7, context).unwrap()
        )));
        assert!(record(event(
            TextInputEvent::deactivated(12, context, 7, 5).unwrap()
        )));

        let state = snapshot();
        assert_eq!(state.phase, TextInputTracePhase::Complete);
        assert_eq!((state.messages, state.commands, state.events), (19, 7, 12));
        assert_eq!(state.output_commits, 5);
        assert_eq!(state.errors, 0);
        assert!(state.final_complete);
        assert!(record(event(
            TextInputEvent::deactivated(12, context, 7, 5).unwrap()
        )));
        assert_eq!(snapshot(), state);
    }
}
