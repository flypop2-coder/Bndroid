//! Allocation-free M50 post-recovery interaction evidence.
//!
//! M49 remains an immutable prefix. This trace starts only after the complete
//! two-service dependency snapshot is visible, then authenticates one rejected
//! outside-phone contact, one App contact, the resulting App presentation, and
//! a second health round for both replacement services.

use core::sync::atomic::{AtomicU64, Ordering};

use bndr_abi::UserImageId;
use bndr_input::{InputEvent, InputEventPayload, RoutedPointer};
use bndr_sm::health::{HEALTH_FRAME_SIZE, HealthFrame, HealthOpcode, ServiceIdentity, ServiceKind};
use bndr_ui::{ShellRect, WindowCommand, WindowCommandPayload, WindowEvent, WindowEventPayload};

use crate::service_dependency_trace::ChannelWriteKind;

const INVALID_DOWN_WRITE: u64 = 1 << 0;
const INVALID_DOWN_READ: u64 = 1 << 1;
const INVALID_UP_WRITE: u64 = 1 << 2;
const INVALID_UP_READ: u64 = 1 << 3;
const APP_DOWN_BIE_WRITE: u64 = 1 << 4;
const APP_DOWN_BIE_READ: u64 = 1 << 5;
const APP_DOWN_BWE_WRITE: u64 = 1 << 6;
const APP_DOWN_BWE_READ: u64 = 1 << 7;
const APP_UP_BIE_WRITE: u64 = 1 << 8;
const APP_UP_BIE_READ: u64 = 1 << 9;
const APP_UP_BWE_WRITE: u64 = 1 << 10;
const APP_UP_BWE_READ: u64 = 1 << 11;
const APP_PRESENT_WRITE: u64 = 1 << 12;
const APP_PRESENT_READ: u64 = 1 << 13;
const OUTPUT_COMMIT: u64 = 1 << 14;
const PRESENTED_WRITE: u64 = 1 << 15;
const PRESENTED_READ: u64 = 1 << 16;
const SURFACE_PROBE_WRITE: u64 = 1 << 17;
const SURFACE_PROBE_READ: u64 = 1 << 18;
const SURFACE_HEALTHY_WRITE: u64 = 1 << 19;
const SURFACE_HEALTHY_READ: u64 = 1 << 20;
const INPUT_PROBE_WRITE: u64 = 1 << 21;
const INPUT_PROBE_READ: u64 = 1 << 22;
const INPUT_HEALTHY_WRITE: u64 = 1 << 23;
const INPUT_HEALTHY_READ: u64 = 1 << 24;
const REQUIRED_BITS: u64 = (1 << 25) - 1;

pub const INVALID_X: u16 = 16;
pub const INVALID_Y: u16 = 32;
pub const APP_X: u16 = 136;
pub const APP_Y: u16 = 184;
pub const APP_LOCAL_X: i32 = 32;
pub const APP_LOCAL_Y: i32 = 40;
pub const APP_DAMAGE: ShellRect = ShellRect::new(56, 72, 40, 32);
pub const APP_COLOR: u32 = 0x00fa_cc15;

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
static BWE_WRITES: AtomicU64 = AtomicU64::new(0);
static BWE_READS: AtomicU64 = AtomicU64::new(0);
static BWC_WRITES: AtomicU64 = AtomicU64::new(0);
static BWC_READS: AtomicU64 = AtomicU64::new(0);
static BSH_WRITES: AtomicU64 = AtomicU64::new(0);
static BSH_READS: AtomicU64 = AtomicU64::new(0);
static OUTPUT_COMMITS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PostRecoveryInteractionTraceSnapshot {
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
    pub bwe_writes: u64,
    pub bwe_reads: u64,
    pub bwc_writes: u64,
    pub bwc_reads: u64,
    pub bsh_writes: u64,
    pub bsh_reads: u64,
    pub output_commits: u64,
    pub rejected: bool,
    pub routed: bool,
    pub presented: bool,
    pub post_healthy: bool,
    pub complete: bool,
}

pub fn snapshot() -> PostRecoveryInteractionTraceSnapshot {
    let bits = BITS.load(Ordering::Acquire);
    let errors = ERRORS.load(Ordering::Acquire);
    let channel_writes = CHANNEL_WRITES.load(Ordering::Acquire);
    let channel_reads = CHANNEL_READS.load(Ordering::Acquire);
    let bie_writes = BIE_WRITES.load(Ordering::Acquire);
    let bie_reads = BIE_READS.load(Ordering::Acquire);
    let bwe_writes = BWE_WRITES.load(Ordering::Acquire);
    let bwe_reads = BWE_READS.load(Ordering::Acquire);
    let bwc_writes = BWC_WRITES.load(Ordering::Acquire);
    let bwc_reads = BWC_READS.load(Ordering::Acquire);
    let bsh_writes = BSH_WRITES.load(Ordering::Acquire);
    let bsh_reads = BSH_READS.load(Ordering::Acquire);
    let output_commits = OUTPUT_COMMITS.load(Ordering::Acquire);
    PostRecoveryInteractionTraceSnapshot {
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
        bwe_writes,
        bwe_reads,
        bwc_writes,
        bwc_reads,
        bsh_writes,
        bsh_reads,
        output_commits,
        rejected: has_all(
            bits,
            INVALID_DOWN_WRITE | INVALID_DOWN_READ | INVALID_UP_WRITE | INVALID_UP_READ,
        ),
        routed: has_all(
            bits,
            APP_DOWN_BIE_WRITE
                | APP_DOWN_BIE_READ
                | APP_DOWN_BWE_WRITE
                | APP_DOWN_BWE_READ
                | APP_UP_BIE_WRITE
                | APP_UP_BIE_READ
                | APP_UP_BWE_WRITE
                | APP_UP_BWE_READ,
        ),
        presented: has_all(
            bits,
            APP_PRESENT_WRITE | APP_PRESENT_READ | OUTPUT_COMMIT | PRESENTED_WRITE | PRESENTED_READ,
        ),
        post_healthy: has_all(
            bits,
            SURFACE_PROBE_WRITE
                | SURFACE_PROBE_READ
                | SURFACE_HEALTHY_WRITE
                | SURFACE_HEALTHY_READ
                | INPUT_PROBE_WRITE
                | INPUT_PROBE_READ
                | INPUT_HEALTHY_WRITE
                | INPUT_HEALTHY_READ,
        ),
        complete: bits == REQUIRED_BITS
            && errors == 0
            && channel_writes == 12
            && channel_reads == 12
            && bie_writes == 4
            && bie_reads == 4
            && bwe_writes == 3
            && bwe_reads == 3
            && bwc_writes == 1
            && bwc_reads == 1
            && bsh_writes == 4
            && bsh_reads == 4
            && output_commits == 1,
    }
}

pub fn record_channel_write(
    sender_pid: u64,
    sender_image: UserImageId,
    wire: &[u8],
    kind: ChannelWriteKind,
) -> bool {
    #[cfg(feature = "post-recovery-focus-runtime")]
    if snapshot().complete {
        return true;
    }
    if !crate::service_dependency_trace::snapshot().complete {
        return true;
    }
    if wire.starts_with(b"BIE1") {
        let Ok(event) = InputEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bie_write(sender_pid, sender_image, event, kind);
    }
    if wire.starts_with(b"BWE1") {
        let Ok(event) = WindowEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bwe_write(sender_pid, sender_image, event, kind);
    }
    if wire.starts_with(b"BWC1") {
        let Ok(command) = WindowCommand::decode(wire) else {
            return reject_decode();
        };
        return record_bwc_write(sender_pid, sender_image, command, kind);
    }
    if wire.starts_with(b"BSH1") {
        let Some(frame) = decode_health(wire) else {
            return false;
        };
        return record_health_write(sender_pid, sender_image, frame, kind);
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
    #[cfg(feature = "post-recovery-focus-runtime")]
    if snapshot().complete {
        return true;
    }
    if !crate::service_dependency_trace::snapshot().complete {
        return true;
    }
    if wire.starts_with(b"BIE1") {
        let Ok(event) = InputEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bie_read(reader_pid, reader_image, sender_pid, event, kind);
    }
    if wire.starts_with(b"BWE1") {
        let Ok(event) = WindowEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bwe_read(reader_pid, reader_image, sender_pid, event, kind);
    }
    if wire.starts_with(b"BWC1") {
        let Ok(command) = WindowCommand::decode(wire) else {
            return reject_decode();
        };
        return record_bwc_read(reader_pid, reader_image, sender_pid, command, kind);
    }
    if wire.starts_with(b"BSH1") {
        let Some(frame) = decode_health(wire) else {
            return false;
        };
        return record_health_read(reader_pid, reader_image, sender_pid, frame, kind);
    }
    true
}

pub fn record_output_commit(
    process_id: u64,
    session_id: u64,
    slot: usize,
    allocation_generation: u32,
    write_generation: u64,
    frame_id: u32,
) -> bool {
    #[cfg(feature = "post-recovery-focus-runtime")]
    if snapshot().complete {
        return true;
    }
    let dependency = crate::service_dependency_trace::snapshot();
    if !dependency.complete {
        return true;
    }
    if process_id != dependency.surface_pid
        || session_id != 2
        || slot != 0
        || allocation_generation != 2
        || write_generation != 4
        || frame_id != 4
        || !has_all(BITS.load(Ordering::Acquire), APP_PRESENT_READ)
    {
        return reject_payload();
    }
    if !commit_bit(OUTPUT_COMMIT) {
        return false;
    }
    OUTPUT_COMMITS.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bie_write(
    sender_pid: u64,
    sender_image: UserImageId,
    event: InputEvent,
    kind: ChannelWriteKind,
) -> bool {
    let dependency = crate::service_dependency_trace::snapshot();
    if sender_image != UserImageId::InputServer || sender_pid != dependency.new_input_pid {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let index = BIE_WRITES.load(Ordering::Acquire) as usize;
    let Some(bit) = bie_write_bit(index) else {
        return reject_phase();
    };
    if !valid_bie(event, index) {
        return reject_payload();
    }
    if index != 0 && BITS.load(Ordering::Acquire) & bie_write_bit(index - 1).unwrap_or(0) == 0 {
        return reject_phase();
    }
    if !commit_bit(bit) {
        return false;
    }
    BIE_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bie_read(
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    event: InputEvent,
    kind: ChannelWriteKind,
) -> bool {
    let dependency = crate::service_dependency_trace::snapshot();
    if reader_image != UserImageId::SurfaceServer
        || reader_pid != dependency.surface_pid
        || sender_pid != dependency.new_input_pid
    {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let index = BIE_READS.load(Ordering::Acquire) as usize;
    let Some((write_bit, read_bit)) = bie_bits(index) else {
        return reject_phase();
    };
    if !valid_bie(event, index) || BITS.load(Ordering::Acquire) & write_bit == 0 {
        return reject_phase();
    }
    if !commit_bit(read_bit) {
        return false;
    }
    BIE_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

fn valid_bie(event: InputEvent, index: usize) -> bool {
    let InputEventPayload::RoutedPointer(pointer) = event.payload() else {
        return false;
    };
    event.sequence() == 8 + index as u64 && valid_pointer(pointer, index)
}

fn valid_pointer(pointer: RoutedPointer, index: usize) -> bool {
    let pressed = index & 1 == 0;
    if pointer.input_sequence != 4 + index as u64 || pointer.pressed != pressed {
        return false;
    }
    if index < 2 {
        pointer.target.is_none()
            && !pointer.captured
            && !pointer.trusted_overlay
            && (pointer.x, pointer.y) == (INVALID_X, INVALID_Y)
    } else {
        pointer.target.is_some_and(app_window_ref)
            && pointer.captured
            && !pointer.trusted_overlay
            && (pointer.x, pointer.y) == (APP_X, APP_Y)
    }
}

fn record_bwe_write(
    sender_pid: u64,
    sender_image: UserImageId,
    event: WindowEvent,
    kind: ChannelWriteKind,
) -> bool {
    let dependency = crate::service_dependency_trace::snapshot();
    if sender_image != UserImageId::SurfaceServer || sender_pid != dependency.surface_pid {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let index = BWE_WRITES.load(Ordering::Acquire) as usize;
    let (bit, prerequisite) = match index {
        0 => (APP_DOWN_BWE_WRITE, APP_DOWN_BIE_READ),
        1 => (APP_UP_BWE_WRITE, APP_UP_BIE_READ),
        2 => (PRESENTED_WRITE, OUTPUT_COMMIT),
        _ => return reject_phase(),
    };
    if BITS.load(Ordering::Acquire) & prerequisite == 0 || !valid_bwe(event, index) {
        return reject_phase();
    }
    if !commit_bit(bit) {
        return false;
    }
    BWE_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bwe_read(
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    event: WindowEvent,
    kind: ChannelWriteKind,
) -> bool {
    let dependency = crate::service_dependency_trace::snapshot();
    let recovery = crate::input_surface_recovery_trace::snapshot();
    if reader_image != UserImageId::App
        || reader_pid != recovery.app_pid
        || sender_pid != dependency.surface_pid
    {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let index = BWE_READS.load(Ordering::Acquire) as usize;
    let (write_bit, read_bit) = match index {
        0 => (APP_DOWN_BWE_WRITE, APP_DOWN_BWE_READ),
        1 => (APP_UP_BWE_WRITE, APP_UP_BWE_READ),
        2 => (PRESENTED_WRITE, PRESENTED_READ),
        _ => return reject_phase(),
    };
    if BITS.load(Ordering::Acquire) & write_bit == 0 || !valid_bwe(event, index) {
        return reject_phase();
    }
    if !commit_bit(read_bit) {
        return false;
    }
    BWE_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
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
            event.command_sequence() == 4
                && event.scene_frame() == 9
                && app_window(window_id)
                && (global_x, global_y) == (APP_X, APP_Y)
                && (local_x, local_y) == (APP_LOCAL_X, APP_LOCAL_Y)
                && pressed == (index == 0)
                && captured
                && focus_generation == 3
        }
        (
            2,
            WindowEventPayload::Presented {
                window_id,
                frame_id,
            },
        ) => {
            event.command_sequence() == 5
                && event.scene_frame() == 10
                && app_window(window_id)
                && frame_id == 3
        }
        _ => false,
    }
}

fn record_bwc_write(
    sender_pid: u64,
    sender_image: UserImageId,
    command: WindowCommand,
    kind: ChannelWriteKind,
) -> bool {
    let recovery = crate::input_surface_recovery_trace::snapshot();
    if sender_image != UserImageId::App || sender_pid != recovery.app_pid {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes
        || BITS.load(Ordering::Acquire) & APP_UP_BWE_READ == 0
        || !valid_present(command)
    {
        return reject_payload();
    }
    if !commit_bit(APP_PRESENT_WRITE) {
        return false;
    }
    BWC_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bwc_read(
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    command: WindowCommand,
    kind: ChannelWriteKind,
) -> bool {
    let dependency = crate::service_dependency_trace::snapshot();
    let recovery = crate::input_surface_recovery_trace::snapshot();
    if reader_image != UserImageId::SurfaceServer
        || reader_pid != dependency.surface_pid
        || sender_pid != recovery.app_pid
    {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes
        || BITS.load(Ordering::Acquire) & APP_PRESENT_WRITE == 0
        || !valid_present(command)
    {
        return reject_payload();
    }
    if !commit_bit(APP_PRESENT_READ) {
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
            frame_id: 3,
            damage: APP_DAMAGE,
            color: APP_COLOR,
        } if command.sequence() == 5 && app_window(window_id)
    )
}

fn record_health_write(
    sender_pid: u64,
    sender_image: UserImageId,
    frame: HealthFrame,
    kind: ChannelWriteKind,
) -> bool {
    let dependency = crate::service_dependency_trace::snapshot();
    if kind != ChannelWriteKind::Bytes || !snapshot_presented() {
        return reject_phase();
    }
    let (bit, valid) = match (frame.opcode(), frame.identity().kind()) {
        (HealthOpcode::Probe, ServiceKind::SurfaceServer) => (
            SURFACE_PROBE_WRITE,
            sender_image == UserImageId::Init
                && sender_pid == dependency.init_pid
                && valid_probe(frame, dependency.surface_pid),
        ),
        (HealthOpcode::Healthy, ServiceKind::SurfaceServer) => (
            SURFACE_HEALTHY_WRITE,
            sender_image == UserImageId::SurfaceServer
                && sender_pid == dependency.surface_pid
                && BITS.load(Ordering::Acquire) & SURFACE_PROBE_READ != 0
                && valid_healthy(frame, dependency.surface_pid),
        ),
        (HealthOpcode::Probe, ServiceKind::InputServer) => (
            INPUT_PROBE_WRITE,
            sender_image == UserImageId::Init
                && sender_pid == dependency.init_pid
                && BITS.load(Ordering::Acquire) & SURFACE_HEALTHY_READ != 0
                && valid_probe(frame, dependency.new_input_pid),
        ),
        (HealthOpcode::Healthy, ServiceKind::InputServer) => (
            INPUT_HEALTHY_WRITE,
            sender_image == UserImageId::InputServer
                && sender_pid == dependency.new_input_pid
                && BITS.load(Ordering::Acquire) & INPUT_PROBE_READ != 0
                && valid_healthy(frame, dependency.new_input_pid),
        ),
        _ => return reject_payload(),
    };
    if !valid {
        return reject_owner();
    }
    if !commit_bit(bit) {
        return false;
    }
    BSH_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_health_read(
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    frame: HealthFrame,
    kind: ChannelWriteKind,
) -> bool {
    let dependency = crate::service_dependency_trace::snapshot();
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let (write_bit, read_bit, valid) = match (frame.opcode(), frame.identity().kind()) {
        (HealthOpcode::Probe, ServiceKind::SurfaceServer) => (
            SURFACE_PROBE_WRITE,
            SURFACE_PROBE_READ,
            reader_image == UserImageId::SurfaceServer
                && reader_pid == dependency.surface_pid
                && sender_pid == dependency.init_pid
                && valid_probe(frame, dependency.surface_pid),
        ),
        (HealthOpcode::Healthy, ServiceKind::SurfaceServer) => (
            SURFACE_HEALTHY_WRITE,
            SURFACE_HEALTHY_READ,
            reader_image == UserImageId::Init
                && reader_pid == dependency.init_pid
                && sender_pid == dependency.surface_pid
                && valid_healthy(frame, dependency.surface_pid),
        ),
        (HealthOpcode::Probe, ServiceKind::InputServer) => (
            INPUT_PROBE_WRITE,
            INPUT_PROBE_READ,
            reader_image == UserImageId::InputServer
                && reader_pid == dependency.new_input_pid
                && sender_pid == dependency.init_pid
                && valid_probe(frame, dependency.new_input_pid),
        ),
        (HealthOpcode::Healthy, ServiceKind::InputServer) => (
            INPUT_HEALTHY_WRITE,
            INPUT_HEALTHY_READ,
            reader_image == UserImageId::Init
                && reader_pid == dependency.init_pid
                && sender_pid == dependency.new_input_pid
                && valid_healthy(frame, dependency.new_input_pid),
        ),
        _ => return reject_payload(),
    };
    if !valid {
        return reject_owner();
    }
    if BITS.load(Ordering::Acquire) & write_bit == 0 || !commit_bit(read_bit) {
        return false;
    }
    BSH_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

fn valid_probe(frame: HealthFrame, pid: u64) -> bool {
    valid_identity(frame.identity(), pid)
        && frame.sequence() == 2
        && frame.interval_ns() == crate::service_dependency_trace::SERVICE_DEPENDENCY_TIMEOUT_NS
        && frame.fault_class().is_none()
        && frame.restart_attempt() == 0
        && frame.restart_budget() == 0
}

fn valid_healthy(frame: HealthFrame, pid: u64) -> bool {
    valid_identity(frame.identity(), pid)
        && frame.sequence() == 2
        && frame.interval_ns() == 0
        && frame.fault_class().is_none()
        && frame.restart_attempt() == 0
        && frame.restart_budget() == 0
}

fn valid_identity(identity: ServiceIdentity, pid: u64) -> bool {
    pid != 0 && identity.pid() == pid && identity.generation() == 2 && pid >> 32 == 2
}

fn decode_health(wire: &[u8]) -> Option<HealthFrame> {
    let Ok(wire) = <&[u8; HEALTH_FRAME_SIZE]>::try_from(wire) else {
        reject_decode();
        return None;
    };
    let Ok(frame) = HealthFrame::decode(wire) else {
        reject_decode();
        return None;
    };
    Some(frame)
}

fn bie_write_bit(index: usize) -> Option<u64> {
    bie_bits(index).map(|bits| bits.0)
}

fn bie_bits(index: usize) -> Option<(u64, u64)> {
    match index {
        0 => Some((INVALID_DOWN_WRITE, INVALID_DOWN_READ)),
        1 => Some((INVALID_UP_WRITE, INVALID_UP_READ)),
        2 => Some((APP_DOWN_BIE_WRITE, APP_DOWN_BIE_READ)),
        3 => Some((APP_UP_BIE_WRITE, APP_UP_BIE_READ)),
        _ => None,
    }
}

fn app_window_ref(target: bndr_input::WindowRef) -> bool {
    target.window_id().get() == 2 && target.generation() == 1
}

fn app_window(window: bndr_ui::WindowId) -> bool {
    window.slot() == 1 && window.generation() == 1
}

fn snapshot_presented() -> bool {
    has_all(
        BITS.load(Ordering::Acquire),
        APP_PRESENT_WRITE | APP_PRESENT_READ | OUTPUT_COMMIT | PRESENTED_WRITE | PRESENTED_READ,
    )
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
