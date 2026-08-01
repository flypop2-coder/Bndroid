//! Allocation-free M49 two-service dependency recovery evidence.
//!
//! M46's [`crate::input_surface_recovery_trace`] remains authoritative for the
//! complete SurfaceServer restart. This trace is dormant until that snapshot
//! is complete, then authenticates the independent BSH1, BIR1, BIP1, BIC1,
//! and BIE1 transcript for the InputServer watchdog and replacement. Unknown
//! protocol magics are ignored; a candidate family that is malformed or out
//! of phase is counted as an error and never advances state.

use core::sync::atomic::{AtomicU64, Ordering};

use bndr_abi::UserImageId;
use bndr_input::{
    CommandStream, InputCommand, InputCommandPayload, InputEvent, InputEventPayload, InputRoute,
    InputRouteControl, InputRouteControlPayload, InputServiceFocus, InputServiceRecoveryMessage,
    InputServiceRecoveryPayload, InputServiceRecoveryPhase, WindowRef,
};
use bndr_sm::health::{HEALTH_FRAME_SIZE, HealthFrame, HealthOpcode, ServiceIdentity, ServiceKind};

pub const SERVICE_DEPENDENCY_TIMEOUT_NS: u64 = 100_000_000;
pub const SERVICE_DEPENDENCY_BACKOFF_NS: u64 = 30_000_000;
pub const SERVICE_DEPENDENCY_HOLD_NS: u64 = 2_000_000_000;

const SURFACE_PROBE_WRITE: u64 = 1 << 0;
const SURFACE_PROBE_READ: u64 = 1 << 1;
const SURFACE_HEALTHY_WRITE: u64 = 1 << 2;
const SURFACE_HEALTHY_READ: u64 = 1 << 3;
const INPUT_ONE_PROBE_WRITE: u64 = 1 << 4;
const INPUT_ONE_PROBE_READ: u64 = 1 << 5;
const WATCHDOG_REQUEST: u64 = 1 << 6;
const WATCHDOG_TIMEOUT: u64 = 1 << 7;
const DEGRADED_OUTPUT: u64 = 1 << 8;
const BIP_ROUTE_LOST_WRITE: u64 = 1 << 9;
const BIP_ROUTE_LOST_READ: u64 = 1 << 10;
const BACKOFF_REQUEST: u64 = 1 << 11;
const BACKOFF_TIMEOUT: u64 = 1 << 12;
const BIR_ACQUIRED_WRITE: u64 = 1 << 13;
const BIR_ACQUIRED_READ: u64 = 1 << 14;
const BIR_BIND_WRITE: u64 = 1 << 15;
const BIR_BIND_READ: u64 = 1 << 16;
const BIE_READY_WRITE: u64 = 1 << 17;
const BIR_READY_WRITE: u64 = 1 << 18;
const BIR_READY_READ: u64 = 1 << 19;
const BIP_OFFER_WRITE: u64 = 1 << 20;
const BIP_OFFER_READ: u64 = 1 << 21;
const BIE_READY_READ: u64 = 1 << 22;
const BIP_PREPARED_WRITE: u64 = 1 << 23;
const BIP_PREPARED_READ: u64 = 1 << 24;
const RECOVERED_OUTPUT: u64 = 1 << 25;
const BIP_ACTIVE_WRITE: u64 = 1 << 26;
const BIP_ACTIVE_READ: u64 = 1 << 27;
const INPUT_TWO_PROBE_WRITE: u64 = 1 << 28;
const INPUT_TWO_PROBE_READ: u64 = 1 << 29;
const INPUT_TWO_HEALTHY_WRITE: u64 = 1 << 30;
const INPUT_TWO_HEALTHY_READ: u64 = 1 << 31;
const REQUIRED_BITS: u64 = (1_u64 << 32) - 1;

const SIX_STEPS: u64 = (1 << 6) - 1;
const WAIT_NONE: u64 = 0;
const WAIT_WATCHDOG: u64 = 1;
const WAIT_BACKOFF: u64 = 2;

static BITS: AtomicU64 = AtomicU64::new(0);
static COMMAND_WRITE_MASK: AtomicU64 = AtomicU64::new(0);
static COMMAND_READ_MASK: AtomicU64 = AtomicU64::new(0);
static ACK_WRITE_MASK: AtomicU64 = AtomicU64::new(0);
static ACK_READ_MASK: AtomicU64 = AtomicU64::new(0);

static ERRORS: AtomicU64 = AtomicU64::new(0);
static DECODE_ERRORS: AtomicU64 = AtomicU64::new(0);
static OWNER_ERRORS: AtomicU64 = AtomicU64::new(0);
static PHASE_ERRORS: AtomicU64 = AtomicU64::new(0);
static PAYLOAD_ERRORS: AtomicU64 = AtomicU64::new(0);

static CHANNEL_WRITES: AtomicU64 = AtomicU64::new(0);
static CHANNEL_READS: AtomicU64 = AtomicU64::new(0);
static BSH_WRITES: AtomicU64 = AtomicU64::new(0);
static BSH_READS: AtomicU64 = AtomicU64::new(0);
static BIR_WRITES: AtomicU64 = AtomicU64::new(0);
static BIR_READS: AtomicU64 = AtomicU64::new(0);
static BIP_WRITES: AtomicU64 = AtomicU64::new(0);
static BIP_READS: AtomicU64 = AtomicU64::new(0);
static BIC_WRITES: AtomicU64 = AtomicU64::new(0);
static BIC_READS: AtomicU64 = AtomicU64::new(0);
static BIE_WRITES: AtomicU64 = AtomicU64::new(0);
static BIE_READS: AtomicU64 = AtomicU64::new(0);

static INIT_PID: AtomicU64 = AtomicU64::new(0);
static SURFACE_PID: AtomicU64 = AtomicU64::new(0);
static SURFACE_GENERATION: AtomicU64 = AtomicU64::new(0);
static OLD_INPUT_PID: AtomicU64 = AtomicU64::new(0);
static OLD_INPUT_GENERATION: AtomicU64 = AtomicU64::new(0);
static NEW_INPUT_PID: AtomicU64 = AtomicU64::new(0);
static NEW_INPUT_GENERATION: AtomicU64 = AtomicU64::new(0);

static WAIT_PENDING: AtomicU64 = AtomicU64::new(WAIT_NONE);
static WATCHDOG_REQUESTS: AtomicU64 = AtomicU64::new(0);
static WATCHDOG_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static BACKOFF_REQUESTS: AtomicU64 = AtomicU64::new(0);
static BACKOFF_TIMEOUTS: AtomicU64 = AtomicU64::new(0);

static DEGRADED_SLOT: AtomicU64 = AtomicU64::new(u64::MAX);
static DEGRADED_ALLOCATION: AtomicU64 = AtomicU64::new(0);
static DEGRADED_WRITE_GENERATION: AtomicU64 = AtomicU64::new(0);
static DEGRADED_FRAME: AtomicU64 = AtomicU64::new(0);
static RECOVERED_SLOT: AtomicU64 = AtomicU64::new(u64::MAX);
static RECOVERED_ALLOCATION: AtomicU64 = AtomicU64::new(0);
static RECOVERED_WRITE_GENERATION: AtomicU64 = AtomicU64::new(0);
static RECOVERED_FRAME: AtomicU64 = AtomicU64::new(0);
static OUTPUT_COMMITS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelWriteKind {
    Bytes,
    Transfer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceDependencyTraceSnapshot {
    pub bits: u64,
    pub command_write_mask: u64,
    pub command_read_mask: u64,
    pub ack_write_mask: u64,
    pub ack_read_mask: u64,
    pub errors: u64,
    pub decode_errors: u64,
    pub owner_errors: u64,
    pub phase_errors: u64,
    pub payload_errors: u64,
    pub channel_writes: u64,
    pub channel_reads: u64,
    pub bsh_writes: u64,
    pub bsh_reads: u64,
    pub bir_writes: u64,
    pub bir_reads: u64,
    pub bip_writes: u64,
    pub bip_reads: u64,
    pub bic_writes: u64,
    pub bic_reads: u64,
    pub bie_writes: u64,
    pub bie_reads: u64,
    pub init_pid: u64,
    pub surface_pid: u64,
    pub surface_generation: u64,
    pub old_input_pid: u64,
    pub old_input_generation: u64,
    pub new_input_pid: u64,
    pub new_input_generation: u64,
    pub watchdog_requests: u64,
    pub watchdog_timeouts: u64,
    pub backoff_requests: u64,
    pub backoff_timeouts: u64,
    pub wait_pending: u64,
    pub output_commits: u64,
    pub degraded_slot: u64,
    pub degraded_allocation_generation: u64,
    pub degraded_write_generation: u64,
    pub degraded_frame: u64,
    pub recovered_slot: u64,
    pub recovered_allocation_generation: u64,
    pub recovered_write_generation: u64,
    pub recovered_frame: u64,
    pub base_complete: bool,
    pub surface_healthy: bool,
    pub input_watchdog_complete: bool,
    pub degraded: bool,
    pub input_backoff_complete: bool,
    pub replacement_bound: bool,
    pub snapshot_resynced: bool,
    pub recovered: bool,
    pub input_healthy: bool,
    pub complete: bool,
}

pub fn snapshot() -> ServiceDependencyTraceSnapshot {
    let base = crate::input_surface_recovery_trace::snapshot();
    let bits = BITS.load(Ordering::Acquire);
    let command_write_mask = COMMAND_WRITE_MASK.load(Ordering::Acquire);
    let command_read_mask = COMMAND_READ_MASK.load(Ordering::Acquire);
    let ack_write_mask = ACK_WRITE_MASK.load(Ordering::Acquire);
    let ack_read_mask = ACK_READ_MASK.load(Ordering::Acquire);
    let errors = ERRORS.load(Ordering::Acquire);
    let channel_writes = CHANNEL_WRITES.load(Ordering::Acquire);
    let channel_reads = CHANNEL_READS.load(Ordering::Acquire);
    let bsh_writes = BSH_WRITES.load(Ordering::Acquire);
    let bsh_reads = BSH_READS.load(Ordering::Acquire);
    let bir_writes = BIR_WRITES.load(Ordering::Acquire);
    let bir_reads = BIR_READS.load(Ordering::Acquire);
    let bip_writes = BIP_WRITES.load(Ordering::Acquire);
    let bip_reads = BIP_READS.load(Ordering::Acquire);
    let bic_writes = BIC_WRITES.load(Ordering::Acquire);
    let bic_reads = BIC_READS.load(Ordering::Acquire);
    let bie_writes = BIE_WRITES.load(Ordering::Acquire);
    let bie_reads = BIE_READS.load(Ordering::Acquire);
    let init_pid = INIT_PID.load(Ordering::Acquire);
    let surface_pid = SURFACE_PID.load(Ordering::Acquire);
    let surface_generation = SURFACE_GENERATION.load(Ordering::Acquire);
    let old_input_pid = OLD_INPUT_PID.load(Ordering::Acquire);
    let old_input_generation = OLD_INPUT_GENERATION.load(Ordering::Acquire);
    let new_input_pid = NEW_INPUT_PID.load(Ordering::Acquire);
    let new_input_generation = NEW_INPUT_GENERATION.load(Ordering::Acquire);
    let watchdog_requests = WATCHDOG_REQUESTS.load(Ordering::Acquire);
    let watchdog_timeouts = WATCHDOG_TIMEOUTS.load(Ordering::Acquire);
    let backoff_requests = BACKOFF_REQUESTS.load(Ordering::Acquire);
    let backoff_timeouts = BACKOFF_TIMEOUTS.load(Ordering::Acquire);
    let wait_pending = WAIT_PENDING.load(Ordering::Acquire);
    let output_commits = OUTPUT_COMMITS.load(Ordering::Acquire);
    let degraded_slot = DEGRADED_SLOT.load(Ordering::Acquire);
    let degraded_allocation_generation = DEGRADED_ALLOCATION.load(Ordering::Acquire);
    let degraded_write_generation = DEGRADED_WRITE_GENERATION.load(Ordering::Acquire);
    let degraded_frame = DEGRADED_FRAME.load(Ordering::Acquire);
    let recovered_slot = RECOVERED_SLOT.load(Ordering::Acquire);
    let recovered_allocation_generation = RECOVERED_ALLOCATION.load(Ordering::Acquire);
    let recovered_write_generation = RECOVERED_WRITE_GENERATION.load(Ordering::Acquire);
    let recovered_frame = RECOVERED_FRAME.load(Ordering::Acquire);
    ServiceDependencyTraceSnapshot {
        bits,
        command_write_mask,
        command_read_mask,
        ack_write_mask,
        ack_read_mask,
        errors,
        decode_errors: DECODE_ERRORS.load(Ordering::Acquire),
        owner_errors: OWNER_ERRORS.load(Ordering::Acquire),
        phase_errors: PHASE_ERRORS.load(Ordering::Acquire),
        payload_errors: PAYLOAD_ERRORS.load(Ordering::Acquire),
        channel_writes,
        channel_reads,
        bsh_writes,
        bsh_reads,
        bir_writes,
        bir_reads,
        bip_writes,
        bip_reads,
        bic_writes,
        bic_reads,
        bie_writes,
        bie_reads,
        init_pid,
        surface_pid,
        surface_generation,
        old_input_pid,
        old_input_generation,
        new_input_pid,
        new_input_generation,
        watchdog_requests,
        watchdog_timeouts,
        backoff_requests,
        backoff_timeouts,
        wait_pending,
        output_commits,
        degraded_slot,
        degraded_allocation_generation,
        degraded_write_generation,
        degraded_frame,
        recovered_slot,
        recovered_allocation_generation,
        recovered_write_generation,
        recovered_frame,
        base_complete: base.complete,
        surface_healthy: has_all(
            bits,
            SURFACE_PROBE_WRITE | SURFACE_PROBE_READ | SURFACE_HEALTHY_WRITE | SURFACE_HEALTHY_READ,
        ),
        input_watchdog_complete: has_all(
            bits,
            INPUT_ONE_PROBE_WRITE | INPUT_ONE_PROBE_READ | WATCHDOG_REQUEST | WATCHDOG_TIMEOUT,
        ),
        degraded: has_all(
            bits,
            DEGRADED_OUTPUT | BIP_ROUTE_LOST_WRITE | BIP_ROUTE_LOST_READ,
        ),
        input_backoff_complete: has_all(bits, BACKOFF_REQUEST | BACKOFF_TIMEOUT),
        replacement_bound: has_all(
            bits,
            BIR_ACQUIRED_WRITE
                | BIR_ACQUIRED_READ
                | BIR_BIND_WRITE
                | BIR_BIND_READ
                | BIE_READY_WRITE
                | BIR_READY_WRITE
                | BIR_READY_READ
                | BIP_OFFER_WRITE
                | BIP_OFFER_READ
                | BIE_READY_READ,
        ),
        snapshot_resynced: command_write_mask == SIX_STEPS
            && command_read_mask == SIX_STEPS
            && ack_write_mask == SIX_STEPS
            && ack_read_mask == SIX_STEPS
            && has_all(bits, BIP_PREPARED_WRITE | BIP_PREPARED_READ),
        recovered: has_all(bits, RECOVERED_OUTPUT | BIP_ACTIVE_WRITE | BIP_ACTIVE_READ),
        input_healthy: has_all(
            bits,
            INPUT_TWO_PROBE_WRITE
                | INPUT_TWO_PROBE_READ
                | INPUT_TWO_HEALTHY_WRITE
                | INPUT_TWO_HEALTHY_READ,
        ),
        complete: base.complete
            && bits == REQUIRED_BITS
            && command_write_mask == SIX_STEPS
            && command_read_mask == SIX_STEPS
            && ack_write_mask == SIX_STEPS
            && ack_read_mask == SIX_STEPS
            && errors == 0
            && channel_writes == 25
            && channel_reads == 25
            && bsh_writes == 5
            && bsh_reads == 5
            && bir_writes == 3
            && bir_reads == 3
            && bip_writes == 4
            && bip_reads == 4
            && bic_writes == 6
            && bic_reads == 6
            && bie_writes == 7
            && bie_reads == 7
            && init_pid == base.init_pid
            && surface_pid == base.new_surface_pid
            && surface_generation == 2
            && old_input_pid == base.input_server_pid
            && old_input_generation == 1
            && same_slot_next_generation(old_input_pid, new_input_pid)
            && new_input_generation == 2
            && output_commits == 2
            && degraded_slot == 0
            && degraded_allocation_generation == 2
            && degraded_write_generation == 2
            && degraded_frame == 2
            && recovered_slot == 0
            && recovered_allocation_generation == 2
            && recovered_write_generation == 3
            && recovered_frame == 3
            && wait_pending == WAIT_NONE
            && watchdog_requests == 1
            && watchdog_timeouts == 1
            && backoff_requests == 1
            && backoff_timeouts == 1,
    }
}

/// Records one committed channel write. Unknown protocol families are not
/// candidates and therefore do not affect this trace.
pub fn record_channel_write(
    sender_pid: u64,
    sender_image: UserImageId,
    wire: &[u8],
    kind: ChannelWriteKind,
) -> bool {
    if !crate::input_surface_recovery_trace::snapshot().complete {
        return true;
    }
    if wire.starts_with(b"BSH1") {
        let Some(frame) = decode_health(wire) else {
            return false;
        };
        return record_health_write(sender_pid, sender_image, frame, kind);
    }
    if wire.starts_with(b"BIR1") {
        let Ok(message) = InputRouteControl::decode(wire) else {
            return reject_decode();
        };
        return record_bir_write(sender_pid, sender_image, message, kind);
    }
    if wire.starts_with(b"BIP1") {
        let Ok(message) = InputServiceRecoveryMessage::decode(wire) else {
            return reject_decode();
        };
        return record_bip_write(sender_pid, sender_image, message, kind);
    }
    if wire.starts_with(b"BIC1") {
        let Ok(command) = InputCommand::decode(wire) else {
            return reject_decode();
        };
        return record_bic_write(sender_pid, sender_image, command, kind);
    }
    if wire.starts_with(b"BIE1") {
        let Ok(event) = InputEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bie_write(sender_pid, sender_image, event, kind);
    }
    true
}

/// Records one successfully dequeued channel message. A read is accepted only
/// after the exact corresponding committed write has been observed.
pub fn record_channel_read(
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    wire: &[u8],
    kind: ChannelWriteKind,
) -> bool {
    if !crate::input_surface_recovery_trace::snapshot().complete {
        return true;
    }
    // The sealed M46 trace is write-authoritative, so its final queued BIR1
    // frame can be dequeued just after `base.complete` becomes visible. M49
    // always begins with Init's SurfaceServer BSH1 write; until that committed
    // write arms this trace, a read cannot belong to M49 and is ignored.
    if BITS.load(Ordering::Acquire) == 0 {
        return true;
    }
    if wire.starts_with(b"BSH1") {
        let Some(frame) = decode_health(wire) else {
            return false;
        };
        return record_health_read(reader_pid, reader_image, sender_pid, frame, kind);
    }
    if wire.starts_with(b"BIR1") {
        let Ok(message) = InputRouteControl::decode(wire) else {
            return reject_decode();
        };
        return record_bir_read(reader_pid, reader_image, sender_pid, message, kind);
    }
    if wire.starts_with(b"BIP1") {
        let Ok(message) = InputServiceRecoveryMessage::decode(wire) else {
            return reject_decode();
        };
        return record_bip_read(reader_pid, reader_image, sender_pid, message, kind);
    }
    if wire.starts_with(b"BIC1") {
        let Ok(command) = InputCommand::decode(wire) else {
            return reject_decode();
        };
        return record_bic_read(reader_pid, reader_image, sender_pid, command, kind);
    }
    if wire.starts_with(b"BIE1") {
        let Ok(event) = InputEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bie_read(reader_pid, reader_image, sender_pid, event, kind);
    }
    true
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

fn record_health_write(
    sender_pid: u64,
    sender_image: UserImageId,
    frame: HealthFrame,
    kind: ChannelWriteKind,
) -> bool {
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let base = crate::input_surface_recovery_trace::snapshot();
    seed_base_identities(base);
    let bits = BITS.load(Ordering::Acquire);
    let identity = frame.identity();
    let bit = match (frame.opcode(), identity.kind()) {
        (HealthOpcode::Probe, ServiceKind::SurfaceServer) => {
            if bits != 0 {
                return reject_phase();
            }
            if sender_image != UserImageId::Init || sender_pid != base.init_pid {
                return reject_owner();
            }
            if !valid_probe(frame, base.new_surface_pid, 2) {
                return reject_payload();
            }
            SURFACE_PROBE_WRITE
        }
        (HealthOpcode::Healthy, ServiceKind::SurfaceServer) => {
            if !has_all(bits, SURFACE_PROBE_WRITE | SURFACE_PROBE_READ)
                || bits & SURFACE_HEALTHY_WRITE != 0
            {
                return reject_phase();
            }
            if sender_image != UserImageId::SurfaceServer || sender_pid != base.new_surface_pid {
                return reject_owner();
            }
            if !valid_healthy(frame, base.new_surface_pid, 2) {
                return reject_payload();
            }
            SURFACE_HEALTHY_WRITE
        }
        (HealthOpcode::Probe, ServiceKind::InputServer)
            if identity.pid() == base.input_server_pid =>
        {
            if bits & SURFACE_HEALTHY_READ == 0 || bits & INPUT_ONE_PROBE_WRITE != 0 {
                return reject_phase();
            }
            if sender_image != UserImageId::Init || sender_pid != base.init_pid {
                return reject_owner();
            }
            if !valid_probe(frame, base.input_server_pid, 1) {
                return reject_payload();
            }
            INPUT_ONE_PROBE_WRITE
        }
        (HealthOpcode::Probe, ServiceKind::InputServer)
            if identity.pid() == NEW_INPUT_PID.load(Ordering::Acquire)
                && NEW_INPUT_PID.load(Ordering::Acquire) != 0 =>
        {
            if !has_all(bits, RECOVERED_OUTPUT | BIP_ACTIVE_WRITE | BIP_ACTIVE_READ)
                || bits & INPUT_TWO_PROBE_WRITE != 0
                || COMMAND_READ_MASK.load(Ordering::Acquire) != SIX_STEPS
                || ACK_READ_MASK.load(Ordering::Acquire) != SIX_STEPS
            {
                return reject_phase();
            }
            if sender_image != UserImageId::Init || sender_pid != base.init_pid {
                return reject_owner();
            }
            if !valid_probe(frame, identity.pid(), 2) {
                return reject_payload();
            }
            INPUT_TWO_PROBE_WRITE
        }
        (HealthOpcode::Healthy, ServiceKind::InputServer)
            if identity.pid() == NEW_INPUT_PID.load(Ordering::Acquire)
                && NEW_INPUT_PID.load(Ordering::Acquire) != 0 =>
        {
            if bits & INPUT_TWO_PROBE_READ == 0 || bits & INPUT_TWO_HEALTHY_WRITE != 0 {
                return reject_phase();
            }
            if sender_image != UserImageId::InputServer || sender_pid != identity.pid() {
                return reject_owner();
            }
            if !valid_healthy(frame, identity.pid(), 2) {
                return reject_payload();
            }
            INPUT_TWO_HEALTHY_WRITE
        }
        _ => return reject_payload(),
    };
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
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let base = crate::input_surface_recovery_trace::snapshot();
    seed_base_identities(base);
    let bits = BITS.load(Ordering::Acquire);
    let identity = frame.identity();
    let bit = match (frame.opcode(), identity.kind()) {
        (HealthOpcode::Probe, ServiceKind::SurfaceServer) => {
            if bits & SURFACE_PROBE_WRITE == 0 || bits & SURFACE_PROBE_READ != 0 {
                return reject_phase();
            }
            if reader_image != UserImageId::SurfaceServer
                || reader_pid != base.new_surface_pid
                || sender_pid != base.init_pid
            {
                return reject_owner();
            }
            if !valid_probe(frame, base.new_surface_pid, 2) {
                return reject_payload();
            }
            SURFACE_PROBE_READ
        }
        (HealthOpcode::Healthy, ServiceKind::SurfaceServer) => {
            if bits & SURFACE_HEALTHY_WRITE == 0 || bits & SURFACE_HEALTHY_READ != 0 {
                return reject_phase();
            }
            if reader_image != UserImageId::Init
                || reader_pid != base.init_pid
                || sender_pid != base.new_surface_pid
            {
                return reject_owner();
            }
            if !valid_healthy(frame, base.new_surface_pid, 2) {
                return reject_payload();
            }
            SURFACE_HEALTHY_READ
        }
        (HealthOpcode::Probe, ServiceKind::InputServer)
            if identity.pid() == base.input_server_pid =>
        {
            if bits & INPUT_ONE_PROBE_WRITE == 0 || bits & INPUT_ONE_PROBE_READ != 0 {
                return reject_phase();
            }
            if reader_image != UserImageId::InputServer
                || reader_pid != base.input_server_pid
                || sender_pid != base.init_pid
            {
                return reject_owner();
            }
            if !valid_probe(frame, base.input_server_pid, 1) {
                return reject_payload();
            }
            INPUT_ONE_PROBE_READ
        }
        (HealthOpcode::Probe, ServiceKind::InputServer)
            if identity.pid() == NEW_INPUT_PID.load(Ordering::Acquire)
                && NEW_INPUT_PID.load(Ordering::Acquire) != 0 =>
        {
            let new_input = NEW_INPUT_PID.load(Ordering::Acquire);
            if bits & INPUT_TWO_PROBE_WRITE == 0 || bits & INPUT_TWO_PROBE_READ != 0 {
                return reject_phase();
            }
            if reader_image != UserImageId::InputServer
                || reader_pid != new_input
                || sender_pid != base.init_pid
            {
                return reject_owner();
            }
            if !valid_probe(frame, new_input, 2) {
                return reject_payload();
            }
            INPUT_TWO_PROBE_READ
        }
        (HealthOpcode::Healthy, ServiceKind::InputServer)
            if identity.pid() == NEW_INPUT_PID.load(Ordering::Acquire)
                && NEW_INPUT_PID.load(Ordering::Acquire) != 0 =>
        {
            let new_input = NEW_INPUT_PID.load(Ordering::Acquire);
            if bits & INPUT_TWO_HEALTHY_WRITE == 0 || bits & INPUT_TWO_HEALTHY_READ != 0 {
                return reject_phase();
            }
            if reader_image != UserImageId::Init
                || reader_pid != base.init_pid
                || sender_pid != new_input
            {
                return reject_owner();
            }
            if !valid_healthy(frame, new_input, 2) {
                return reject_payload();
            }
            INPUT_TWO_HEALTHY_READ
        }
        _ => return reject_payload(),
    };
    if !commit_bit(bit) {
        return false;
    }
    BSH_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

/// Records one finite wait request. The three 2-second visual/cadence holds and the
/// bounded waits that receive Healthy are deliberately not watchdog/backoff
/// evidence.
pub fn record_wait_request(init_pid: u64, timeout_ns: u64, expected_peer: bool) -> bool {
    let base = crate::input_surface_recovery_trace::snapshot();
    if !base.complete {
        return true;
    }
    if timeout_ns != SERVICE_DEPENDENCY_TIMEOUT_NS
        && timeout_ns != SERVICE_DEPENDENCY_BACKOFF_NS
        && timeout_ns != SERVICE_DEPENDENCY_HOLD_NS
    {
        return true;
    }
    if init_pid != base.init_pid || init_pid == 0 {
        return reject_owner();
    }
    if !expected_peer {
        return reject_payload();
    }
    if timeout_ns == SERVICE_DEPENDENCY_HOLD_NS {
        return true;
    }

    let bits = BITS.load(Ordering::Acquire);
    let pending = if bits & INPUT_ONE_PROBE_WRITE != 0 && bits & WATCHDOG_REQUEST == 0 {
        WAIT_WATCHDOG
    } else if bits & BIP_ROUTE_LOST_READ != 0 && bits & BACKOFF_REQUEST == 0 {
        WAIT_BACKOFF
    } else if (bits & SURFACE_PROBE_WRITE != 0 && bits & SURFACE_HEALTHY_READ == 0)
        || (bits & INPUT_TWO_PROBE_WRITE != 0 && bits & INPUT_TWO_HEALTHY_READ == 0)
    {
        // These waits are response deadlines. Their successful reads carry the
        // evidence; they must not be counted as timeout paths.
        return true;
    } else {
        return reject_phase();
    };
    if WAIT_PENDING
        .compare_exchange(WAIT_NONE, pending, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return reject_phase();
    }
    let bit = if pending == WAIT_WATCHDOG {
        WATCHDOG_REQUEST
    } else {
        BACKOFF_REQUEST
    };
    if !commit_bit(bit) {
        let _ =
            WAIT_PENDING.compare_exchange(pending, WAIT_NONE, Ordering::AcqRel, Ordering::Acquire);
        return false;
    }
    if pending == WAIT_WATCHDOG {
        WATCHDOG_REQUESTS.fetch_add(1, Ordering::Relaxed);
    } else {
        BACKOFF_REQUESTS.fetch_add(1, Ordering::Relaxed);
    }
    true
}

/// Records a timeout wake corresponding to the last counted health or backoff request.
/// Timeout wakes from each 2-second visual/cadence hold are ignored by phase.
pub fn record_wait_timeout(init_pid: u64) -> bool {
    let base = crate::input_surface_recovery_trace::snapshot();
    if !base.complete {
        return true;
    }
    if init_pid != base.init_pid || init_pid == 0 {
        return reject_owner();
    }
    let bits = BITS.load(Ordering::Acquire);
    match WAIT_PENDING.load(Ordering::Acquire) {
        WAIT_WATCHDOG => {
            if bits & INPUT_ONE_PROBE_READ == 0
                || bits & WATCHDOG_REQUEST == 0
                || bits & WATCHDOG_TIMEOUT != 0
            {
                return reject_phase();
            }
            if !commit_bit(WATCHDOG_TIMEOUT) {
                return false;
            }
            WAIT_PENDING.store(WAIT_NONE, Ordering::Release);
            WATCHDOG_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
            true
        }
        WAIT_BACKOFF => {
            if bits & BIP_ROUTE_LOST_READ == 0
                || bits & BACKOFF_REQUEST == 0
                || bits & BACKOFF_TIMEOUT != 0
            {
                return reject_phase();
            }
            if !commit_bit(BACKOFF_TIMEOUT) {
                return false;
            }
            WAIT_PENDING.store(WAIT_NONE, Ordering::Release);
            BACKOFF_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
            true
        }
        WAIT_NONE
            if bits == 0
                || (bits & BIP_ROUTE_LOST_READ != 0 && bits & BACKOFF_REQUEST == 0)
                || (bits & SURFACE_HEALTHY_WRITE != 0 && bits & SURFACE_HEALTHY_READ == 0)
                || (bits & INPUT_TWO_HEALTHY_WRITE != 0 && bits & INPUT_TWO_HEALTHY_READ == 0)
                || has_all(bits, INPUT_TWO_HEALTHY_READ | BIP_ACTIVE_READ) =>
        {
            true
        }
        _ => reject_phase(),
    }
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
    let base = crate::input_surface_recovery_trace::snapshot();
    if !base.complete {
        return true;
    }
    seed_base_identities(base);
    if process_id != base.new_surface_pid || process_id == 0 || session_id != 2 {
        return true;
    }
    let bits = BITS.load(Ordering::Acquire);
    let (bit, expected_write, expected_frame) = if bits & DEGRADED_OUTPUT == 0 {
        if !has_all(bits, SURFACE_HEALTHY_READ | WATCHDOG_TIMEOUT)
            || bits & BIP_ROUTE_LOST_WRITE != 0
        {
            return reject_phase();
        }
        (DEGRADED_OUTPUT, 2, 2)
    } else if bits & RECOVERED_OUTPUT == 0 {
        if bits & BIP_PREPARED_READ == 0
            || COMMAND_READ_MASK.load(Ordering::Acquire) != SIX_STEPS
            || ACK_READ_MASK.load(Ordering::Acquire) != SIX_STEPS
            || bits & BIP_ACTIVE_WRITE != 0
        {
            return reject_phase();
        }
        (RECOVERED_OUTPUT, 3, 3)
    } else {
        return reject_phase();
    };
    if slot != 0
        || allocation_generation != 2
        || write_generation != expected_write
        || frame_id != expected_frame
    {
        return reject_payload();
    }
    let (slot_store, allocation_store, write_store, frame_store) = if bit == DEGRADED_OUTPUT {
        (
            &DEGRADED_SLOT,
            &DEGRADED_ALLOCATION,
            &DEGRADED_WRITE_GENERATION,
            &DEGRADED_FRAME,
        )
    } else {
        (
            &RECOVERED_SLOT,
            &RECOVERED_ALLOCATION,
            &RECOVERED_WRITE_GENERATION,
            &RECOVERED_FRAME,
        )
    };
    if !commit_bit(bit) {
        return false;
    }
    slot_store.store(slot as u64, Ordering::Release);
    allocation_store.store(u64::from(allocation_generation), Ordering::Release);
    write_store.store(write_generation, Ordering::Release);
    frame_store.store(u64::from(frame_id), Ordering::Release);
    OUTPUT_COMMITS.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bir_write(
    sender_pid: u64,
    sender_image: UserImageId,
    message: InputRouteControl,
    kind: ChannelWriteKind,
) -> bool {
    let base = crate::input_surface_recovery_trace::snapshot();
    seed_base_identities(base);
    let bits = BITS.load(Ordering::Acquire);
    let (bit, new_input) = match message.payload() {
        InputRouteControlPayload::Acquired {
            physical_sequence_floor,
        } => {
            if !has_all(bits, BIP_ROUTE_LOST_READ | BACKOFF_TIMEOUT)
                || bits & BIR_ACQUIRED_WRITE != 0
            {
                return reject_phase();
            }
            if sender_image != UserImageId::InputServer {
                return reject_owner();
            }
            if kind != ChannelWriteKind::Bytes {
                return reject_payload();
            }
            if !same_slot_next_generation(base.input_server_pid, sender_pid)
                || pid_generation(base.input_server_pid) != 1
                || pid_generation(sender_pid) != 2
            {
                return reject_owner();
            }
            if message.sequence() != 1
                || message.route_epoch() != 0
                || message.input_session_id() != 2
                || physical_sequence_floor != 3
            {
                return reject_payload();
            }
            (BIR_ACQUIRED_WRITE, Some(sender_pid))
        }
        InputRouteControlPayload::Bind {
            expected_surface_pid,
            expected_physical_floor,
        } => {
            if bits & BIR_ACQUIRED_READ == 0 || bits & BIR_BIND_WRITE != 0 {
                return reject_phase();
            }
            if sender_image != UserImageId::Init || sender_pid != base.init_pid {
                return reject_owner();
            }
            if kind != ChannelWriteKind::Transfer {
                return reject_payload();
            }
            if message.sequence() != 2
                || message.route_epoch() != 3
                || message.input_session_id() != 2
                || expected_surface_pid.get() != base.new_surface_pid
                || expected_physical_floor != 3
            {
                return reject_payload();
            }
            (BIR_BIND_WRITE, None)
        }
        InputRouteControlPayload::Ready {
            bound_surface_pid,
            physical_sequence_floor,
        } => {
            if !has_all(bits, BIR_BIND_READ | BIE_READY_WRITE) || bits & BIR_READY_WRITE != 0 {
                return reject_phase();
            }
            let expected_input = NEW_INPUT_PID.load(Ordering::Acquire);
            if sender_image != UserImageId::InputServer
                || sender_pid != expected_input
                || expected_input == 0
            {
                return reject_owner();
            }
            if kind != ChannelWriteKind::Bytes {
                return reject_payload();
            }
            if message.sequence() != 2
                || message.route_epoch() != 3
                || message.input_session_id() != 2
                || bound_surface_pid.get() != base.new_surface_pid
                || physical_sequence_floor != 3
            {
                return reject_payload();
            }
            (BIR_READY_WRITE, None)
        }
        InputRouteControlPayload::RouteLost { .. } | InputRouteControlPayload::GapStatus { .. } => {
            return reject_payload();
        }
    };
    if !commit_bit(bit) {
        return false;
    }
    if let Some(pid) = new_input {
        NEW_INPUT_PID.store(pid, Ordering::Release);
        NEW_INPUT_GENERATION.store(u64::from(pid_generation(pid)), Ordering::Release);
    }
    BIR_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bir_read(
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    message: InputRouteControl,
    kind: ChannelWriteKind,
) -> bool {
    let base = crate::input_surface_recovery_trace::snapshot();
    seed_base_identities(base);
    let bits = BITS.load(Ordering::Acquire);
    let new_input = NEW_INPUT_PID.load(Ordering::Acquire);
    let bit = match message.payload() {
        InputRouteControlPayload::Acquired {
            physical_sequence_floor,
        } => {
            if bits & BIR_ACQUIRED_WRITE == 0 || bits & BIR_ACQUIRED_READ != 0 {
                return reject_phase();
            }
            if reader_image != UserImageId::Init
                || reader_pid != base.init_pid
                || sender_pid != new_input
                || new_input == 0
            {
                return reject_owner();
            }
            if kind != ChannelWriteKind::Bytes {
                return reject_payload();
            }
            if message.sequence() != 1
                || message.route_epoch() != 0
                || message.input_session_id() != 2
                || physical_sequence_floor != 3
            {
                return reject_payload();
            }
            BIR_ACQUIRED_READ
        }
        InputRouteControlPayload::Bind {
            expected_surface_pid,
            expected_physical_floor,
        } => {
            if bits & BIR_BIND_WRITE == 0 || bits & BIR_BIND_READ != 0 {
                return reject_phase();
            }
            if reader_image != UserImageId::InputServer
                || reader_pid != new_input
                || sender_pid != base.init_pid
                || new_input == 0
            {
                return reject_owner();
            }
            if kind != ChannelWriteKind::Transfer {
                return reject_payload();
            }
            if message.sequence() != 2
                || message.route_epoch() != 3
                || message.input_session_id() != 2
                || expected_surface_pid.get() != base.new_surface_pid
                || expected_physical_floor != 3
            {
                return reject_payload();
            }
            BIR_BIND_READ
        }
        InputRouteControlPayload::Ready {
            bound_surface_pid,
            physical_sequence_floor,
        } => {
            if bits & BIR_READY_WRITE == 0 || bits & BIR_READY_READ != 0 {
                return reject_phase();
            }
            if reader_image != UserImageId::Init
                || reader_pid != base.init_pid
                || sender_pid != new_input
                || new_input == 0
            {
                return reject_owner();
            }
            if kind != ChannelWriteKind::Bytes {
                return reject_payload();
            }
            if message.sequence() != 2
                || message.route_epoch() != 3
                || message.input_session_id() != 2
                || bound_surface_pid.get() != base.new_surface_pid
                || physical_sequence_floor != 3
            {
                return reject_payload();
            }
            BIR_READY_READ
        }
        InputRouteControlPayload::RouteLost { .. } | InputRouteControlPayload::GapStatus { .. } => {
            return reject_payload();
        }
    };
    if !commit_bit(bit) {
        return false;
    }
    BIR_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bip_write(
    sender_pid: u64,
    sender_image: UserImageId,
    message: InputServiceRecoveryMessage,
    kind: ChannelWriteKind,
) -> bool {
    let base = crate::input_surface_recovery_trace::snapshot();
    seed_base_identities(base);
    let bits = BITS.load(Ordering::Acquire);
    let new_input = NEW_INPUT_PID.load(Ordering::Acquire);
    let bit = match message.payload() {
        InputServiceRecoveryPayload::SurfaceStatus { phase, .. } => {
            if sender_image != UserImageId::SurfaceServer || sender_pid != base.new_surface_pid {
                return reject_owner();
            }
            if kind != ChannelWriteKind::Bytes {
                return reject_payload();
            }
            match phase {
                InputServiceRecoveryPhase::RouteLost => {
                    if !has_all(bits, WATCHDOG_TIMEOUT | DEGRADED_OUTPUT)
                        || bits & BIP_ROUTE_LOST_WRITE != 0
                    {
                        return reject_phase();
                    }
                    if !valid_bip_status(
                        message,
                        1,
                        2,
                        base.input_server_pid,
                        1,
                        3,
                        InputServiceRecoveryPhase::RouteLost,
                    ) {
                        return reject_payload();
                    }
                    BIP_ROUTE_LOST_WRITE
                }
                InputServiceRecoveryPhase::ResyncPrepared => {
                    if !has_all(bits, BIP_OFFER_READ | BIE_READY_READ)
                        || bits & BIP_PREPARED_WRITE != 0
                        || COMMAND_READ_MASK.load(Ordering::Acquire) != SIX_STEPS
                        || ACK_READ_MASK.load(Ordering::Acquire) != SIX_STEPS
                    {
                        return reject_phase();
                    }
                    if !valid_bip_status(
                        message,
                        2,
                        3,
                        new_input,
                        2,
                        3,
                        InputServiceRecoveryPhase::ResyncPrepared,
                    ) {
                        return reject_payload();
                    }
                    BIP_PREPARED_WRITE
                }
                InputServiceRecoveryPhase::Active => {
                    if !has_all(bits, BIP_PREPARED_READ | RECOVERED_OUTPUT)
                        || bits & BIP_ACTIVE_WRITE != 0
                    {
                        return reject_phase();
                    }
                    if !valid_bip_status(
                        message,
                        3,
                        3,
                        new_input,
                        2,
                        3,
                        InputServiceRecoveryPhase::Active,
                    ) {
                        return reject_payload();
                    }
                    BIP_ACTIVE_WRITE
                }
                InputServiceRecoveryPhase::Armed | InputServiceRecoveryPhase::RestartRequested => {
                    return reject_phase();
                }
            }
        }
        InputServiceRecoveryPayload::RebindOffer {
            old_input_pid,
            new_input_pid,
            new_input_session_id,
            physical_sequence_floor,
        } => {
            if bits & BIR_READY_READ == 0 || bits & BIP_OFFER_WRITE != 0 {
                return reject_phase();
            }
            if sender_image != UserImageId::Init || sender_pid != base.init_pid {
                return reject_owner();
            }
            if kind != ChannelWriteKind::Transfer {
                return reject_payload();
            }
            if message.sequence() != 1
                || message.surface_session() != 2
                || message.route_epoch() != 3
                || old_input_pid.get() != base.input_server_pid
                || new_input_pid.get() != new_input
                || new_input == 0
                || new_input_session_id != 2
                || physical_sequence_floor != 3
            {
                return reject_payload();
            }
            BIP_OFFER_WRITE
        }
    };
    if !commit_bit(bit) {
        return false;
    }
    BIP_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bip_read(
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    message: InputServiceRecoveryMessage,
    kind: ChannelWriteKind,
) -> bool {
    let base = crate::input_surface_recovery_trace::snapshot();
    seed_base_identities(base);
    let bits = BITS.load(Ordering::Acquire);
    let new_input = NEW_INPUT_PID.load(Ordering::Acquire);
    let bit = match message.payload() {
        InputServiceRecoveryPayload::SurfaceStatus { phase, .. } => {
            if reader_image != UserImageId::Init
                || reader_pid != base.init_pid
                || sender_pid != base.new_surface_pid
            {
                return reject_owner();
            }
            if kind != ChannelWriteKind::Bytes {
                return reject_payload();
            }
            match phase {
                InputServiceRecoveryPhase::RouteLost => {
                    if bits & BIP_ROUTE_LOST_WRITE == 0 || bits & BIP_ROUTE_LOST_READ != 0 {
                        return reject_phase();
                    }
                    if !valid_bip_status(
                        message,
                        1,
                        2,
                        base.input_server_pid,
                        1,
                        3,
                        InputServiceRecoveryPhase::RouteLost,
                    ) {
                        return reject_payload();
                    }
                    BIP_ROUTE_LOST_READ
                }
                InputServiceRecoveryPhase::ResyncPrepared => {
                    if bits & BIP_PREPARED_WRITE == 0 || bits & BIP_PREPARED_READ != 0 {
                        return reject_phase();
                    }
                    if !valid_bip_status(
                        message,
                        2,
                        3,
                        new_input,
                        2,
                        3,
                        InputServiceRecoveryPhase::ResyncPrepared,
                    ) {
                        return reject_payload();
                    }
                    BIP_PREPARED_READ
                }
                InputServiceRecoveryPhase::Active => {
                    if bits & BIP_ACTIVE_WRITE == 0 || bits & BIP_ACTIVE_READ != 0 {
                        return reject_phase();
                    }
                    if !valid_bip_status(
                        message,
                        3,
                        3,
                        new_input,
                        2,
                        3,
                        InputServiceRecoveryPhase::Active,
                    ) {
                        return reject_payload();
                    }
                    BIP_ACTIVE_READ
                }
                InputServiceRecoveryPhase::Armed | InputServiceRecoveryPhase::RestartRequested => {
                    return reject_phase();
                }
            }
        }
        InputServiceRecoveryPayload::RebindOffer {
            old_input_pid,
            new_input_pid,
            new_input_session_id,
            physical_sequence_floor,
        } => {
            if bits & BIP_OFFER_WRITE == 0 || bits & BIP_OFFER_READ != 0 {
                return reject_phase();
            }
            if reader_image != UserImageId::SurfaceServer
                || reader_pid != base.new_surface_pid
                || sender_pid != base.init_pid
            {
                return reject_owner();
            }
            if kind != ChannelWriteKind::Transfer {
                return reject_payload();
            }
            if message.sequence() != 1
                || message.surface_session() != 2
                || message.route_epoch() != 3
                || old_input_pid.get() != base.input_server_pid
                || new_input_pid.get() != new_input
                || new_input == 0
                || new_input_session_id != 2
                || physical_sequence_floor != 3
            {
                return reject_payload();
            }
            BIP_OFFER_READ
        }
    };
    if !commit_bit(bit) {
        return false;
    }
    BIP_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bic_write(
    sender_pid: u64,
    sender_image: UserImageId,
    command: InputCommand,
    kind: ChannelWriteKind,
) -> bool {
    let base = crate::input_surface_recovery_trace::snapshot();
    seed_base_identities(base);
    let bits = BITS.load(Ordering::Acquire);
    if !has_all(bits, BIP_OFFER_READ | BIE_READY_READ) || bits & BIP_PREPARED_WRITE != 0 {
        return reject_phase();
    }
    if sender_image != UserImageId::SurfaceServer || sender_pid != base.new_surface_pid {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let write_mask = COMMAND_WRITE_MASK.load(Ordering::Acquire);
    let Some(index) = contiguous_len(write_mask) else {
        return reject_phase();
    };
    if index >= 6 {
        return reject_phase();
    }
    let prior = low_mask(index);
    if COMMAND_READ_MASK.load(Ordering::Acquire) != prior
        || ACK_WRITE_MASK.load(Ordering::Acquire) != prior
        || ACK_READ_MASK.load(Ordering::Acquire) != prior
    {
        return reject_phase();
    }
    if !valid_resync_command(command, index, base.launcher_pid, base.app_pid) {
        return reject_payload();
    }
    if !commit_mask(&COMMAND_WRITE_MASK, 1 << index) {
        return false;
    }
    BIC_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bic_read(
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    command: InputCommand,
    kind: ChannelWriteKind,
) -> bool {
    let base = crate::input_surface_recovery_trace::snapshot();
    seed_base_identities(base);
    let bits = BITS.load(Ordering::Acquire);
    if !has_all(bits, BIP_OFFER_READ | BIE_READY_READ) || bits & BIP_PREPARED_WRITE != 0 {
        return reject_phase();
    }
    let new_input = NEW_INPUT_PID.load(Ordering::Acquire);
    if reader_image != UserImageId::InputServer
        || reader_pid != new_input
        || sender_pid != base.new_surface_pid
        || new_input == 0
    {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let read_mask = COMMAND_READ_MASK.load(Ordering::Acquire);
    let Some(index) = contiguous_len(read_mask) else {
        return reject_phase();
    };
    if index >= 6 {
        return reject_phase();
    }
    let prior = low_mask(index);
    let current = 1 << index;
    if COMMAND_WRITE_MASK.load(Ordering::Acquire) != prior | current
        || ACK_WRITE_MASK.load(Ordering::Acquire) != prior
        || ACK_READ_MASK.load(Ordering::Acquire) != prior
    {
        return reject_phase();
    }
    if !valid_resync_command(command, index, base.launcher_pid, base.app_pid) {
        return reject_payload();
    }
    if !commit_mask(&COMMAND_READ_MASK, current) {
        return false;
    }
    BIC_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bie_write(
    sender_pid: u64,
    sender_image: UserImageId,
    event: InputEvent,
    kind: ChannelWriteKind,
) -> bool {
    let base = crate::input_surface_recovery_trace::snapshot();
    seed_base_identities(base);
    let bits = BITS.load(Ordering::Acquire);
    let new_input = NEW_INPUT_PID.load(Ordering::Acquire);
    if sender_image != UserImageId::InputServer || sender_pid != new_input || new_input == 0 {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    if event.payload() == InputEventPayload::Ready {
        if bits & BIR_BIND_READ == 0
            || bits & (BIE_READY_WRITE | BIR_READY_WRITE | BIP_OFFER_WRITE) != 0
        {
            return reject_phase();
        }
        if event.sequence() != 1 {
            return reject_payload();
        }
        if !commit_bit(BIE_READY_WRITE) {
            return false;
        }
    } else {
        if !has_all(bits, BIP_OFFER_READ | BIE_READY_READ) || bits & BIP_PREPARED_WRITE != 0 {
            return reject_phase();
        }
        let write_mask = ACK_WRITE_MASK.load(Ordering::Acquire);
        let Some(index) = contiguous_len(write_mask) else {
            return reject_phase();
        };
        if index >= 6 {
            return reject_phase();
        }
        let prior = low_mask(index);
        let current = 1 << index;
        if COMMAND_WRITE_MASK.load(Ordering::Acquire) != prior | current
            || COMMAND_READ_MASK.load(Ordering::Acquire) != prior | current
            || ACK_READ_MASK.load(Ordering::Acquire) != prior
        {
            return reject_phase();
        }
        if !valid_resync_ack(event, index) {
            return reject_payload();
        }
        if !commit_mask(&ACK_WRITE_MASK, current) {
            return false;
        }
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
    let base = crate::input_surface_recovery_trace::snapshot();
    seed_base_identities(base);
    let bits = BITS.load(Ordering::Acquire);
    let new_input = NEW_INPUT_PID.load(Ordering::Acquire);
    if reader_image != UserImageId::SurfaceServer
        || reader_pid != base.new_surface_pid
        || sender_pid != new_input
        || new_input == 0
    {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    if event.payload() == InputEventPayload::Ready {
        if !has_all(bits, BIP_OFFER_READ | BIE_READY_WRITE) || bits & BIE_READY_READ != 0 {
            return reject_phase();
        }
        if event.sequence() != 1 {
            return reject_payload();
        }
        if !commit_bit(BIE_READY_READ) {
            return false;
        }
    } else {
        if !has_all(bits, BIP_OFFER_READ | BIE_READY_READ) || bits & BIP_PREPARED_WRITE != 0 {
            return reject_phase();
        }
        let read_mask = ACK_READ_MASK.load(Ordering::Acquire);
        let Some(index) = contiguous_len(read_mask) else {
            return reject_phase();
        };
        if index >= 6 {
            return reject_phase();
        }
        let prior = low_mask(index);
        let current = 1 << index;
        if COMMAND_WRITE_MASK.load(Ordering::Acquire) != prior | current
            || COMMAND_READ_MASK.load(Ordering::Acquire) != prior | current
            || ACK_WRITE_MASK.load(Ordering::Acquire) != prior | current
        {
            return reject_phase();
        }
        if !valid_resync_ack(event, index) {
            return reject_payload();
        }
        if !commit_mask(&ACK_READ_MASK, current) {
            return false;
        }
    }
    BIE_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

#[allow(clippy::too_many_arguments)]
fn valid_bip_status(
    message: InputServiceRecoveryMessage,
    sequence: u64,
    route_epoch: u64,
    input_pid: u64,
    input_session_id: u64,
    physical_sequence_floor: u64,
    expected_phase: InputServiceRecoveryPhase,
) -> bool {
    if message.sequence() != sequence
        || message.surface_session() != 2
        || message.route_epoch() != route_epoch
        || input_pid == 0
    {
        return false;
    }
    matches!(
        message.payload(),
        InputServiceRecoveryPayload::SurfaceStatus {
            input_pid: actual_pid,
            input_session_id: actual_session,
            physical_sequence_floor: actual_floor,
            phase,
            route_count: 2,
            focus: InputServiceFocus::App,
            capture_active: false,
            text_active: false,
        } if actual_pid.get() == input_pid
            && actual_session == input_session_id
            && actual_floor == physical_sequence_floor
            && phase == expected_phase
    )
}

fn valid_resync_command(
    command: InputCommand,
    index: usize,
    launcher_pid: u64,
    app_pid: u64,
) -> bool {
    let launcher_bounds = if cfg!(feature = "post-recovery-interaction-runtime") {
        (56, 64, 208, 368)
    } else {
        (0, 0, 208, 368)
    };
    let app_bounds = if cfg!(feature = "post-recovery-interaction-runtime") {
        (104, 144, 112, 160)
    } else {
        (48, 80, 112, 160)
    };
    match (index, command.payload()) {
        (0, InputCommandPayload::SnapshotReset) => command.sequence() == 1,
        (1, InputCommandPayload::RouteUpsert(route)) => {
            command.sequence() == 2 && valid_route(route, 1, launcher_pid, launcher_bounds, 0)
        }
        (2, InputCommandPayload::RouteUpsert(route)) => {
            command.sequence() == 3 && valid_route(route, 2, app_pid, app_bounds, 1)
        }
        (3, InputCommandPayload::SetFocus(Some(target))) => {
            command.sequence() == 1 && window_is(target, 2)
        }
        (4, InputCommandPayload::SetTextContext(None)) => command.sequence() == 2,
        (5, InputCommandPayload::SceneCommit) => command.sequence() == 4,
        _ => false,
    }
}

fn valid_route(
    route: InputRoute,
    window_id: u64,
    owner_pid: u64,
    expected_bounds: (u16, u16, u16, u16),
    z_order: i32,
) -> bool {
    let bounds = route.bounds();
    window_is(route.target(), window_id)
        && owner_pid != 0
        && route.owner_pid().get() == owner_pid
        && (bounds.x(), bounds.y(), bounds.width(), bounds.height()) == expected_bounds
        && route.z_order() == z_order
        && route.is_visible()
        && route.is_focusable()
        && !route.is_trusted_overlay()
}

fn window_is(target: WindowRef, window_id: u64) -> bool {
    target.window_id().get() == window_id && target.generation() == 1
}

fn valid_resync_ack(event: InputEvent, index: usize) -> bool {
    let (stream, acknowledged_sequence) = match index {
        0 => (CommandStream::Route, 1),
        1 => (CommandStream::Route, 2),
        2 => (CommandStream::Route, 3),
        3 => (CommandStream::Focus, 1),
        4 => (CommandStream::Focus, 2),
        5 => (CommandStream::Route, 4),
        _ => return false,
    };
    event.sequence() == index as u64 + 2
        && event.payload()
            == (InputEventPayload::Ack {
                stream,
                acknowledged_sequence,
            })
}

fn seed_base_identities(
    base: crate::input_surface_recovery_trace::InputSurfaceRecoveryTraceSnapshot,
) {
    seed_value(&INIT_PID, base.init_pid);
    seed_value(&SURFACE_PID, base.new_surface_pid);
    seed_value(
        &SURFACE_GENERATION,
        u64::from(pid_generation(base.new_surface_pid)),
    );
    seed_value(&OLD_INPUT_PID, base.input_server_pid);
    seed_value(
        &OLD_INPUT_GENERATION,
        u64::from(pid_generation(base.input_server_pid)),
    );
}

fn seed_value(target: &AtomicU64, value: u64) {
    if value != 0 {
        let _ = target.compare_exchange(0, value, Ordering::AcqRel, Ordering::Acquire);
    }
}

fn valid_probe(frame: HealthFrame, pid: u64, generation: u32) -> bool {
    valid_health_identity(frame.identity(), pid, generation)
        && frame.sequence() == 1
        && frame.interval_ns() == SERVICE_DEPENDENCY_TIMEOUT_NS
        && frame.fault_class().is_none()
        && frame.restart_attempt() == 0
        && frame.restart_budget() == 0
}

fn valid_healthy(frame: HealthFrame, pid: u64, generation: u32) -> bool {
    valid_health_identity(frame.identity(), pid, generation)
        && frame.sequence() == 1
        && frame.interval_ns() == 0
        && frame.fault_class().is_none()
        && frame.restart_attempt() == 0
        && frame.restart_budget() == 0
}

fn valid_health_identity(identity: ServiceIdentity, pid: u64, generation: u32) -> bool {
    pid != 0
        && identity.pid() == pid
        && identity.generation() == generation
        && pid_generation(pid) == generation
}

const fn pid_generation(pid: u64) -> u32 {
    (pid >> 32) as u32
}

fn same_slot_next_generation(old_pid: u64, new_pid: u64) -> bool {
    old_pid != 0
        && new_pid != 0
        && old_pid as u32 == new_pid as u32
        && pid_generation(new_pid) == pid_generation(old_pid).checked_add(1).unwrap_or(0)
}

const fn has_all(bits: u64, expected: u64) -> bool {
    bits & expected == expected
}

fn contiguous_len(mask: u64) -> Option<usize> {
    let length = mask.count_ones() as usize;
    (mask == low_mask(length)).then_some(length)
}

const fn low_mask(length: usize) -> u64 {
    if length == 0 {
        0
    } else if length >= 64 {
        u64::MAX
    } else {
        (1_u64 << length) - 1
    }
}

fn commit_bit(bit: u64) -> bool {
    let previous = BITS.fetch_or(bit, Ordering::AcqRel);
    if previous & bit != 0 {
        return reject_phase();
    }
    true
}

fn commit_mask(mask: &AtomicU64, bit: u64) -> bool {
    let previous = mask.fetch_or(bit, Ordering::AcqRel);
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
