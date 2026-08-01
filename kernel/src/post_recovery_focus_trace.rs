//! Allocation-free M51 post-recovery Launcher-focus evidence.
//!
//! M49 and M50 remain immutable prefixes. This trace starts only after the
//! complete M50 snapshot is visible, then authenticates one Launcher-only
//! contact, the resulting InputServer focus synchronization, one Launcher
//! presentation, and its exact output commit. Once the transcript is complete,
//! later resident traffic is outside this bounded witness and is ignored.

use core::sync::atomic::{AtomicU64, Ordering};

use bndr_abi::UserImageId;
use bndr_input::{
    CommandStream, InputCommand, InputCommandPayload, InputEvent, InputEventPayload, RoutedPointer,
    WindowRef,
};
use bndr_ui::{ShellRect, WindowCommand, WindowCommandPayload, WindowEvent, WindowEventPayload};

use crate::service_dependency_trace::ChannelWriteKind;

const DOWN_BIE_WRITE: u64 = 1 << 0;
const DOWN_BIE_READ: u64 = 1 << 1;
const FOCUS_BIC_WRITE: u64 = 1 << 2;
const FOCUS_BIC_READ: u64 = 1 << 3;
const FOCUS_ACK_WRITE: u64 = 1 << 4;
const FOCUS_ACK_READ: u64 = 1 << 5;
const DOWN_BWE_WRITE: u64 = 1 << 6;
const DOWN_BWE_READ: u64 = 1 << 7;
const UP_BIE_WRITE: u64 = 1 << 8;
const UP_BIE_READ: u64 = 1 << 9;
const UP_BWE_WRITE: u64 = 1 << 10;
const UP_BWE_READ: u64 = 1 << 11;
const PRESENT_WRITE: u64 = 1 << 12;
const PRESENT_READ: u64 = 1 << 13;
const OUTPUT_COMMIT: u64 = 1 << 14;
const PRESENTED_WRITE: u64 = 1 << 15;
const PRESENTED_READ: u64 = 1 << 16;
const REQUIRED_BITS: u64 = (1 << 17) - 1;

pub const LAUNCHER_X: u16 = 80;
pub const LAUNCHER_Y: u16 = 96;
pub const LAUNCHER_LOCAL_X: i32 = 24;
pub const LAUNCHER_LOCAL_Y: i32 = 32;
pub const LAUNCHER_DAMAGE: ShellRect = ShellRect::new(8, 16, 40, 32);
pub const LAUNCHER_COLOR: u32 = 0x0034_c759;

static BITS: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);
static DECODE_ERRORS: AtomicU64 = AtomicU64::new(0);
static OWNER_ERRORS: AtomicU64 = AtomicU64::new(0);
static PHASE_ERRORS: AtomicU64 = AtomicU64::new(0);
static PAYLOAD_ERRORS: AtomicU64 = AtomicU64::new(0);
static CHANNEL_WRITES: AtomicU64 = AtomicU64::new(0);
static CHANNEL_READS: AtomicU64 = AtomicU64::new(0);
static BIE_WRITES: AtomicU64 = AtomicU64::new(0);
static BIE_READS: AtomicU64 = AtomicU64::new(0);
static BIC_WRITES: AtomicU64 = AtomicU64::new(0);
static BIC_READS: AtomicU64 = AtomicU64::new(0);
static BWE_WRITES: AtomicU64 = AtomicU64::new(0);
static BWE_READS: AtomicU64 = AtomicU64::new(0);
static BWC_WRITES: AtomicU64 = AtomicU64::new(0);
static BWC_READS: AtomicU64 = AtomicU64::new(0);
static OUTPUT_COMMITS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PostRecoveryFocusTraceSnapshot {
    pub bits: u64,
    pub errors: u64,
    pub decode_errors: u64,
    pub owner_errors: u64,
    pub phase_errors: u64,
    pub payload_errors: u64,
    pub channel_writes: u64,
    pub channel_reads: u64,
    pub bie_writes: u64,
    pub bie_reads: u64,
    pub bic_writes: u64,
    pub bic_reads: u64,
    pub bwe_writes: u64,
    pub bwe_reads: u64,
    pub bwc_writes: u64,
    pub bwc_reads: u64,
    pub output_commits: u64,
    pub captured: bool,
    pub focus_synced: bool,
    pub presented: bool,
    pub complete: bool,
}

pub fn snapshot() -> PostRecoveryFocusTraceSnapshot {
    let bits = BITS.load(Ordering::Acquire);
    let errors = ERRORS.load(Ordering::Acquire);
    let channel_writes = CHANNEL_WRITES.load(Ordering::Acquire);
    let channel_reads = CHANNEL_READS.load(Ordering::Acquire);
    let bie_writes = BIE_WRITES.load(Ordering::Acquire);
    let bie_reads = BIE_READS.load(Ordering::Acquire);
    let bic_writes = BIC_WRITES.load(Ordering::Acquire);
    let bic_reads = BIC_READS.load(Ordering::Acquire);
    let bwe_writes = BWE_WRITES.load(Ordering::Acquire);
    let bwe_reads = BWE_READS.load(Ordering::Acquire);
    let bwc_writes = BWC_WRITES.load(Ordering::Acquire);
    let bwc_reads = BWC_READS.load(Ordering::Acquire);
    let output_commits = OUTPUT_COMMITS.load(Ordering::Acquire);
    PostRecoveryFocusTraceSnapshot {
        bits,
        errors,
        decode_errors: DECODE_ERRORS.load(Ordering::Acquire),
        owner_errors: OWNER_ERRORS.load(Ordering::Acquire),
        phase_errors: PHASE_ERRORS.load(Ordering::Acquire),
        payload_errors: PAYLOAD_ERRORS.load(Ordering::Acquire),
        channel_writes,
        channel_reads,
        bie_writes,
        bie_reads,
        bic_writes,
        bic_reads,
        bwe_writes,
        bwe_reads,
        bwc_writes,
        bwc_reads,
        output_commits,
        captured: has_all(
            bits,
            DOWN_BIE_WRITE
                | DOWN_BIE_READ
                | FOCUS_BIC_WRITE
                | FOCUS_BIC_READ
                | FOCUS_ACK_WRITE
                | FOCUS_ACK_READ
                | DOWN_BWE_WRITE
                | DOWN_BWE_READ,
        ),
        focus_synced: has_all(
            bits,
            FOCUS_BIC_WRITE | FOCUS_BIC_READ | FOCUS_ACK_WRITE | FOCUS_ACK_READ,
        ),
        presented: has_all(
            bits,
            PRESENT_WRITE | PRESENT_READ | OUTPUT_COMMIT | PRESENTED_WRITE | PRESENTED_READ,
        ),
        complete: bits == REQUIRED_BITS
            && errors == 0
            && channel_writes == 8
            && channel_reads == 8
            && bie_writes == 3
            && bie_reads == 3
            && bic_writes == 1
            && bic_reads == 1
            && bwe_writes == 3
            && bwe_reads == 3
            && bwc_writes == 1
            && bwc_reads == 1
            && output_commits == 1,
    }
}

#[derive(Clone, Copy)]
struct TraceOwners {
    surface_pid: u64,
    input_pid: u64,
    launcher_pid: u64,
}

fn active_owners() -> Option<TraceOwners> {
    if !crate::post_recovery_interaction_trace::snapshot().complete {
        return None;
    }
    let dependency = crate::service_dependency_trace::snapshot();
    let recovery = crate::input_surface_recovery_trace::snapshot();
    Some(TraceOwners {
        surface_pid: dependency.surface_pid,
        input_pid: dependency.new_input_pid,
        launcher_pid: recovery.launcher_pid,
    })
}

pub fn record_channel_write(
    sender_pid: u64,
    sender_image: UserImageId,
    wire: &[u8],
    kind: ChannelWriteKind,
) -> bool {
    let Some(owners) = active_owners() else {
        return true;
    };
    record_channel_write_with(owners, sender_pid, sender_image, wire, kind)
}

fn record_channel_write_with(
    owners: TraceOwners,
    sender_pid: u64,
    sender_image: UserImageId,
    wire: &[u8],
    kind: ChannelWriteKind,
) -> bool {
    if snapshot().complete {
        return true;
    }
    if wire.starts_with(b"BIE1") {
        let Ok(event) = InputEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bie_write(owners, sender_pid, sender_image, event, kind);
    }
    if wire.starts_with(b"BIC1") {
        let Ok(command) = InputCommand::decode(wire) else {
            return reject_decode();
        };
        return record_bic_write(owners, sender_pid, sender_image, command, kind);
    }
    if wire.starts_with(b"BWE1") {
        let Ok(event) = WindowEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bwe_write(owners, sender_pid, sender_image, event, kind);
    }
    if wire.starts_with(b"BWC1") {
        let Ok(command) = WindowCommand::decode(wire) else {
            return reject_decode();
        };
        return record_bwc_write(owners, sender_pid, sender_image, command, kind);
    }
    true
}

pub fn record_channel_read(
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    wire: &[u8],
    kind: ChannelWriteKind,
) -> bool {
    let Some(owners) = active_owners() else {
        return true;
    };
    record_channel_read_with(owners, reader_pid, reader_image, sender_pid, wire, kind)
}

fn record_channel_read_with(
    owners: TraceOwners,
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    wire: &[u8],
    kind: ChannelWriteKind,
) -> bool {
    if snapshot().complete {
        return true;
    }
    if wire.starts_with(b"BIE1") {
        let Ok(event) = InputEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bie_read(owners, reader_pid, reader_image, sender_pid, event, kind);
    }
    if wire.starts_with(b"BIC1") {
        let Ok(command) = InputCommand::decode(wire) else {
            return reject_decode();
        };
        return record_bic_read(owners, reader_pid, reader_image, sender_pid, command, kind);
    }
    if wire.starts_with(b"BWE1") {
        let Ok(event) = WindowEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bwe_read(owners, reader_pid, reader_image, sender_pid, event, kind);
    }
    if wire.starts_with(b"BWC1") {
        let Ok(command) = WindowCommand::decode(wire) else {
            return reject_decode();
        };
        return record_bwc_read(owners, reader_pid, reader_image, sender_pid, command, kind);
    }
    true
}

#[allow(clippy::too_many_arguments)]
pub fn record_output_commit(
    process_id: u64,
    session_id: u64,
    slot: usize,
    allocation_generation: u32,
    write_generation: u64,
    frame_id: u32,
) -> bool {
    let Some(owners) = active_owners() else {
        return true;
    };
    record_output_commit_with(
        owners,
        process_id,
        session_id,
        slot,
        allocation_generation,
        write_generation,
        frame_id,
    )
}

#[allow(clippy::too_many_arguments)]
fn record_output_commit_with(
    owners: TraceOwners,
    process_id: u64,
    session_id: u64,
    slot: usize,
    allocation_generation: u32,
    write_generation: u64,
    frame_id: u32,
) -> bool {
    if snapshot().complete {
        return true;
    }
    if process_id != owners.surface_pid {
        return reject_owner();
    }
    if session_id != 2
        || slot != 0
        || allocation_generation != 2
        || write_generation != 5
        || frame_id != 5
    {
        return reject_payload();
    }
    if !has_all(BITS.load(Ordering::Acquire), PRESENT_READ) {
        return reject_phase();
    }
    if !commit_bit(OUTPUT_COMMIT) {
        return false;
    }
    OUTPUT_COMMITS.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bie_write(
    owners: TraceOwners,
    sender_pid: u64,
    sender_image: UserImageId,
    event: InputEvent,
    kind: ChannelWriteKind,
) -> bool {
    if sender_image != UserImageId::InputServer || sender_pid != owners.input_pid {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let index = BIE_WRITES.load(Ordering::Acquire) as usize;
    let Some((write_bit, _, prerequisite)) = bie_bits(index) else {
        return reject_phase();
    };
    if !valid_bie(event, index) {
        return reject_payload();
    }
    if prerequisite != 0 && !has_all(BITS.load(Ordering::Acquire), prerequisite) {
        return reject_phase();
    }
    if !commit_bit(write_bit) {
        return false;
    }
    BIE_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bie_read(
    owners: TraceOwners,
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    event: InputEvent,
    kind: ChannelWriteKind,
) -> bool {
    if reader_image != UserImageId::SurfaceServer
        || reader_pid != owners.surface_pid
        || sender_pid != owners.input_pid
    {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let index = BIE_READS.load(Ordering::Acquire) as usize;
    let Some((write_bit, read_bit, _)) = bie_bits(index) else {
        return reject_phase();
    };
    if !valid_bie(event, index) {
        return reject_payload();
    }
    if !has_all(BITS.load(Ordering::Acquire), write_bit) {
        return reject_phase();
    }
    if !commit_bit(read_bit) {
        return false;
    }
    BIE_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

fn bie_bits(index: usize) -> Option<(u64, u64, u64)> {
    match index {
        0 => Some((DOWN_BIE_WRITE, DOWN_BIE_READ, 0)),
        1 => Some((FOCUS_ACK_WRITE, FOCUS_ACK_READ, FOCUS_BIC_READ)),
        2 => Some((UP_BIE_WRITE, UP_BIE_READ, DOWN_BWE_READ)),
        _ => None,
    }
}

fn valid_bie(event: InputEvent, index: usize) -> bool {
    match (index, event.payload()) {
        (0, InputEventPayload::RoutedPointer(pointer)) => {
            event.sequence() == 12 && valid_pointer(pointer, 8, true)
        }
        (
            1,
            InputEventPayload::Ack {
                stream: CommandStream::Focus,
                acknowledged_sequence: 3,
            },
        ) => event.sequence() == 13,
        (2, InputEventPayload::RoutedPointer(pointer)) => {
            event.sequence() == 14 && valid_pointer(pointer, 9, false)
        }
        _ => false,
    }
}

fn valid_pointer(pointer: RoutedPointer, physical_sequence: u64, pressed: bool) -> bool {
    pointer.input_sequence == physical_sequence
        && pointer.target.is_some_and(launcher_window_ref)
        && (pointer.x, pointer.y) == (LAUNCHER_X, LAUNCHER_Y)
        && pointer.pressed == pressed
        && pointer.captured
        && !pointer.trusted_overlay
}

fn record_bic_write(
    owners: TraceOwners,
    sender_pid: u64,
    sender_image: UserImageId,
    command: InputCommand,
    kind: ChannelWriteKind,
) -> bool {
    if sender_image != UserImageId::SurfaceServer || sender_pid != owners.surface_pid {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes || !valid_focus(command) {
        return reject_payload();
    }
    if !has_all(BITS.load(Ordering::Acquire), DOWN_BIE_READ) {
        return reject_phase();
    }
    if !commit_bit(FOCUS_BIC_WRITE) {
        return false;
    }
    BIC_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bic_read(
    owners: TraceOwners,
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    command: InputCommand,
    kind: ChannelWriteKind,
) -> bool {
    if reader_image != UserImageId::InputServer
        || reader_pid != owners.input_pid
        || sender_pid != owners.surface_pid
    {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes || !valid_focus(command) {
        return reject_payload();
    }
    if !has_all(BITS.load(Ordering::Acquire), FOCUS_BIC_WRITE) {
        return reject_phase();
    }
    if !commit_bit(FOCUS_BIC_READ) {
        return false;
    }
    BIC_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

fn valid_focus(command: InputCommand) -> bool {
    matches!(
        command.payload(),
        InputCommandPayload::SetFocus(Some(target))
            if command.sequence() == 3 && launcher_window_ref(target)
    )
}

fn record_bwe_write(
    owners: TraceOwners,
    sender_pid: u64,
    sender_image: UserImageId,
    event: WindowEvent,
    kind: ChannelWriteKind,
) -> bool {
    if sender_image != UserImageId::SurfaceServer || sender_pid != owners.surface_pid {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let index = BWE_WRITES.load(Ordering::Acquire) as usize;
    let Some((write_bit, _, prerequisite)) = bwe_bits(index) else {
        return reject_phase();
    };
    if !valid_bwe(event, index) {
        return reject_payload();
    }
    if !has_all(BITS.load(Ordering::Acquire), prerequisite) {
        return reject_phase();
    }
    if !commit_bit(write_bit) {
        return false;
    }
    BWE_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bwe_read(
    owners: TraceOwners,
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    event: WindowEvent,
    kind: ChannelWriteKind,
) -> bool {
    if reader_image != UserImageId::Launcher
        || reader_pid != owners.launcher_pid
        || sender_pid != owners.surface_pid
    {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let index = BWE_READS.load(Ordering::Acquire) as usize;
    let Some((write_bit, read_bit, _)) = bwe_bits(index) else {
        return reject_phase();
    };
    if !valid_bwe(event, index) {
        return reject_payload();
    }
    if !has_all(BITS.load(Ordering::Acquire), write_bit) {
        return reject_phase();
    }
    if !commit_bit(read_bit) {
        return false;
    }
    BWE_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

fn bwe_bits(index: usize) -> Option<(u64, u64, u64)> {
    match index {
        0 => Some((DOWN_BWE_WRITE, DOWN_BWE_READ, FOCUS_ACK_READ)),
        1 => Some((UP_BWE_WRITE, UP_BWE_READ, UP_BIE_READ)),
        2 => Some((PRESENTED_WRITE, PRESENTED_READ, OUTPUT_COMMIT)),
        _ => None,
    }
}

fn valid_bwe(event: WindowEvent, index: usize) -> bool {
    match (index, event.payload()) {
        (
            0 | 1,
            WindowEventPayload::InputRoute {
                window_id,
                global_x,
                global_y,
                local_x,
                local_y,
                pressed,
                captured,
                focus_generation,
            },
        ) => {
            event.command_sequence() == 5
                && event.scene_frame() == 10
                && launcher_window(window_id)
                && (global_x, global_y) == (LAUNCHER_X, LAUNCHER_Y)
                && (local_x, local_y) == (LAUNCHER_LOCAL_X, LAUNCHER_LOCAL_Y)
                && pressed == (index == 0)
                && captured
                && focus_generation == 4
        }
        (
            2,
            WindowEventPayload::Presented {
                window_id,
                frame_id: 4,
            },
        ) => {
            event.command_sequence() == 6 && event.scene_frame() == 11 && launcher_window(window_id)
        }
        _ => false,
    }
}

fn record_bwc_write(
    owners: TraceOwners,
    sender_pid: u64,
    sender_image: UserImageId,
    command: WindowCommand,
    kind: ChannelWriteKind,
) -> bool {
    if sender_image != UserImageId::Launcher || sender_pid != owners.launcher_pid {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes || !valid_present(command) {
        return reject_payload();
    }
    if !has_all(BITS.load(Ordering::Acquire), UP_BWE_READ) {
        return reject_phase();
    }
    if !commit_bit(PRESENT_WRITE) {
        return false;
    }
    BWC_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bwc_read(
    owners: TraceOwners,
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    command: WindowCommand,
    kind: ChannelWriteKind,
) -> bool {
    if reader_image != UserImageId::SurfaceServer
        || reader_pid != owners.surface_pid
        || sender_pid != owners.launcher_pid
    {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes || !valid_present(command) {
        return reject_payload();
    }
    if !has_all(BITS.load(Ordering::Acquire), PRESENT_WRITE) {
        return reject_phase();
    }
    if !commit_bit(PRESENT_READ) {
        return false;
    }
    BWC_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

fn valid_present(command: WindowCommand) -> bool {
    matches!(
        command.payload(),
        WindowCommandPayload::Present {
            window_id,
            frame_id: 4,
            damage: LAUNCHER_DAMAGE,
            color: LAUNCHER_COLOR,
        } if command.sequence() == 6 && launcher_window(window_id)
    )
}

fn launcher_window_ref(target: WindowRef) -> bool {
    target.window_id().get() == 1 && target.generation() == 1
}

fn launcher_window(window: bndr_ui::WindowId) -> bool {
    window.slot() == 0 && window.generation() == 1
}

const fn has_all(bits: u64, expected: u64) -> bool {
    bits & expected == expected
}

fn commit_bit(bit: u64) -> bool {
    let previous = BITS.fetch_or(bit, Ordering::AcqRel);
    if previous & bit != 0 {
        return reject_phase();
    }
    true
}

fn reject_decode() -> bool {
    DECODE_ERRORS.fetch_add(1, Ordering::Relaxed);
    ERRORS.fetch_add(1, Ordering::Relaxed);
    false
}

fn reject_owner() -> bool {
    OWNER_ERRORS.fetch_add(1, Ordering::Relaxed);
    ERRORS.fetch_add(1, Ordering::Relaxed);
    false
}

fn reject_phase() -> bool {
    PHASE_ERRORS.fetch_add(1, Ordering::Relaxed);
    ERRORS.fetch_add(1, Ordering::Relaxed);
    false
}

fn reject_payload() -> bool {
    PAYLOAD_ERRORS.fetch_add(1, Ordering::Relaxed);
    ERRORS.fetch_add(1, Ordering::Relaxed);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use bndr_input::{InputWindowId, WindowRef};
    use bndr_ui::WindowId;

    const SURFACE_PID: u64 = (2_u64 << 32) | 5;
    const INPUT_PID: u64 = (2_u64 << 32) | 8;
    const LAUNCHER_PID: u64 = (1_u64 << 32) | 6;

    fn owners() -> TraceOwners {
        TraceOwners {
            surface_pid: SURFACE_PID,
            input_pid: INPUT_PID,
            launcher_pid: LAUNCHER_PID,
        }
    }

    fn launcher_ref() -> WindowRef {
        WindowRef::try_new(InputWindowId::try_new(1).unwrap(), 1).unwrap()
    }

    fn launcher_window_id() -> WindowId {
        WindowId::try_new(0, 1).unwrap()
    }

    fn pointer_event(sequence: u64, physical: u64, pressed: bool) -> InputEvent {
        InputEvent::routed_pointer(
            sequence,
            RoutedPointer {
                input_sequence: physical,
                target: Some(launcher_ref()),
                x: LAUNCHER_X,
                y: LAUNCHER_Y,
                pressed,
                captured: true,
                trusted_overlay: false,
            },
        )
        .unwrap()
    }

    fn route_event(pressed: bool) -> WindowEvent {
        WindowEvent::input_route(
            5,
            10,
            launcher_window_id(),
            LAUNCHER_X,
            LAUNCHER_Y,
            LAUNCHER_LOCAL_X,
            LAUNCHER_LOCAL_Y,
            pressed,
            true,
            4,
        )
        .unwrap()
    }

    fn reset() {
        for counter in [
            &BITS,
            &ERRORS,
            &DECODE_ERRORS,
            &OWNER_ERRORS,
            &PHASE_ERRORS,
            &PAYLOAD_ERRORS,
            &CHANNEL_WRITES,
            &CHANNEL_READS,
            &BIE_WRITES,
            &BIE_READS,
            &BIC_WRITES,
            &BIC_READS,
            &BWE_WRITES,
            &BWE_READS,
            &BWC_WRITES,
            &BWC_READS,
            &OUTPUT_COMMITS,
        ] {
            counter.store(0, Ordering::Relaxed);
        }
    }

    #[test]
    fn exact_transcript_is_fail_closed_and_then_ignores_resident_tail() {
        let owners = owners();
        let down = pointer_event(12, 8, true).encode();
        let focus = InputCommand::set_focus(3, Some(launcher_ref()))
            .unwrap()
            .encode();

        reset();
        assert!(!record_channel_write_with(
            owners,
            LAUNCHER_PID,
            UserImageId::Launcher,
            &down,
            ChannelWriteKind::Bytes,
        ));
        assert_eq!(snapshot().owner_errors, 1);

        reset();
        assert!(!record_channel_write_with(
            owners,
            SURFACE_PID,
            UserImageId::SurfaceServer,
            &focus,
            ChannelWriteKind::Bytes,
        ));
        assert_eq!(snapshot().phase_errors, 1);

        reset();
        assert!(!record_channel_write_with(
            owners,
            INPUT_PID,
            UserImageId::InputServer,
            b"BIE1",
            ChannelWriteKind::Bytes,
        ));
        assert_eq!(snapshot().decode_errors, 1);

        reset();
        assert!(record_channel_write_with(
            owners,
            INPUT_PID,
            UserImageId::InputServer,
            &down,
            ChannelWriteKind::Bytes,
        ));
        assert!(record_channel_read_with(
            owners,
            SURFACE_PID,
            UserImageId::SurfaceServer,
            INPUT_PID,
            &down,
            ChannelWriteKind::Bytes,
        ));
        assert!(record_channel_write_with(
            owners,
            SURFACE_PID,
            UserImageId::SurfaceServer,
            &focus,
            ChannelWriteKind::Bytes,
        ));
        assert!(record_channel_read_with(
            owners,
            INPUT_PID,
            UserImageId::InputServer,
            SURFACE_PID,
            &focus,
            ChannelWriteKind::Bytes,
        ));

        let ack = InputEvent::ack(13, CommandStream::Focus, 3)
            .unwrap()
            .encode();
        assert!(record_channel_write_with(
            owners,
            INPUT_PID,
            UserImageId::InputServer,
            &ack,
            ChannelWriteKind::Bytes,
        ));
        assert!(record_channel_read_with(
            owners,
            SURFACE_PID,
            UserImageId::SurfaceServer,
            INPUT_PID,
            &ack,
            ChannelWriteKind::Bytes,
        ));

        let down_route = route_event(true).encode();
        assert!(record_channel_write_with(
            owners,
            SURFACE_PID,
            UserImageId::SurfaceServer,
            &down_route,
            ChannelWriteKind::Bytes,
        ));
        assert!(record_channel_read_with(
            owners,
            LAUNCHER_PID,
            UserImageId::Launcher,
            SURFACE_PID,
            &down_route,
            ChannelWriteKind::Bytes,
        ));
        assert!(snapshot().captured);
        assert!(snapshot().focus_synced);

        let up = pointer_event(14, 9, false).encode();
        assert!(record_channel_write_with(
            owners,
            INPUT_PID,
            UserImageId::InputServer,
            &up,
            ChannelWriteKind::Bytes,
        ));
        assert!(record_channel_read_with(
            owners,
            SURFACE_PID,
            UserImageId::SurfaceServer,
            INPUT_PID,
            &up,
            ChannelWriteKind::Bytes,
        ));

        let up_route = route_event(false).encode();
        assert!(record_channel_write_with(
            owners,
            SURFACE_PID,
            UserImageId::SurfaceServer,
            &up_route,
            ChannelWriteKind::Bytes,
        ));
        assert!(record_channel_read_with(
            owners,
            LAUNCHER_PID,
            UserImageId::Launcher,
            SURFACE_PID,
            &up_route,
            ChannelWriteKind::Bytes,
        ));

        let present =
            WindowCommand::present(6, launcher_window_id(), 4, LAUNCHER_DAMAGE, LAUNCHER_COLOR)
                .unwrap()
                .encode();
        assert!(record_channel_write_with(
            owners,
            LAUNCHER_PID,
            UserImageId::Launcher,
            &present,
            ChannelWriteKind::Bytes,
        ));
        assert!(record_channel_read_with(
            owners,
            SURFACE_PID,
            UserImageId::SurfaceServer,
            LAUNCHER_PID,
            &present,
            ChannelWriteKind::Bytes,
        ));
        assert!(record_output_commit_with(
            owners,
            SURFACE_PID,
            2,
            0,
            2,
            5,
            5,
        ));

        let presented = WindowEvent::presented(6, 11, launcher_window_id(), 4)
            .unwrap()
            .encode();
        assert!(record_channel_write_with(
            owners,
            SURFACE_PID,
            UserImageId::SurfaceServer,
            &presented,
            ChannelWriteKind::Bytes,
        ));
        assert!(record_channel_read_with(
            owners,
            LAUNCHER_PID,
            UserImageId::Launcher,
            SURFACE_PID,
            &presented,
            ChannelWriteKind::Bytes,
        ));

        let complete = snapshot();
        assert!(complete.presented);
        assert!(complete.complete);
        assert_eq!((complete.channel_writes, complete.channel_reads), (8, 8));
        assert_eq!((complete.bie_writes, complete.bie_reads), (3, 3));
        assert_eq!((complete.bic_writes, complete.bic_reads), (1, 1));
        assert_eq!((complete.bwe_writes, complete.bwe_reads), (3, 3));
        assert_eq!((complete.bwc_writes, complete.bwc_reads), (1, 1));
        assert_eq!(complete.output_commits, 1);

        assert!(record_channel_write_with(
            owners,
            LAUNCHER_PID,
            UserImageId::Launcher,
            &down,
            ChannelWriteKind::Transfer,
        ));
        assert_eq!(snapshot(), complete);
    }
}
