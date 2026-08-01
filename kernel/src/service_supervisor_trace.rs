//! Allocation-free M48 ServiceSupervisor/watchdog evidence.
//!
//! The M47 restart trace remains the authority for the first InputServer
//! replacement. This extension accepts only the canonical BSH1 transcript
//! that follows that recovery: one externally reported process exit, one
//! healthy replacement probe, one timed-out probe, terminal quarantine, and
//! one visible degraded acknowledgement. Kernel-stamped sender identities are
//! checked before any state bit advances.

use core::sync::atomic::{AtomicU64, Ordering};

use bndr_abi::UserImageId;
use bndr_sm::health::{
    FaultClass, HEALTH_FRAME_SIZE, HealthFrame, HealthOpcode, ServiceIdentity, ServiceKind,
};

pub const SERVICE_WATCHDOG_TIMEOUT_NS: u64 = 200_000_000;

const OLD_PROCESS_EXIT: u64 = 1 << 0;
const FIRST_PROBE: u64 = 1 << 1;
const FIRST_HEALTHY: u64 = 1 << 2;
const SECOND_PROBE: u64 = 1 << 3;
const WATCHDOG_REQUESTED: u64 = 1 << 4;
const WATCHDOG_TIMED_OUT: u64 = 1 << 5;
const QUARANTINE: u64 = 1 << 6;
const DEGRADED_OUTPUT: u64 = 1 << 7;
const DEGRADED_ACK: u64 = 1 << 8;
const REQUIRED_BITS: u64 = (1 << 9) - 1;

const FIRST_PROBE_READ: u64 = 1 << 0;
const SECOND_PROBE_READ: u64 = 1 << 1;
const REQUIRED_PROBE_READ_BITS: u64 = FIRST_PROBE_READ | SECOND_PROBE_READ;

static BITS: AtomicU64 = AtomicU64::new(0);
static PROBE_READ_BITS: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);
static DECODE_ERRORS: AtomicU64 = AtomicU64::new(0);
static OWNER_ERRORS: AtomicU64 = AtomicU64::new(0);
static PHASE_ERRORS: AtomicU64 = AtomicU64::new(0);
static PAYLOAD_ERRORS: AtomicU64 = AtomicU64::new(0);

static MESSAGES: AtomicU64 = AtomicU64::new(0);
static FAULT_MESSAGES: AtomicU64 = AtomicU64::new(0);
static PROBE_MESSAGES: AtomicU64 = AtomicU64::new(0);
static PROBE_READS: AtomicU64 = AtomicU64::new(0);
static HEALTHY_MESSAGES: AtomicU64 = AtomicU64::new(0);
static QUARANTINE_MESSAGES: AtomicU64 = AtomicU64::new(0);
static DEGRADED_ACK_MESSAGES: AtomicU64 = AtomicU64::new(0);

static OLD_INPUT_PID: AtomicU64 = AtomicU64::new(0);
static NEW_INPUT_PID: AtomicU64 = AtomicU64::new(0);
static INPUT_GENERATION: AtomicU64 = AtomicU64::new(0);
static SURFACE_TRANSPORT_OWNER_PID: AtomicU64 = AtomicU64::new(0);
static SURFACE_TRANSPORT_HANDLE: AtomicU64 = AtomicU64::new(0);
static INPUT_TRANSPORT_OWNER_PID: AtomicU64 = AtomicU64::new(0);
static INPUT_TRANSPORT_HANDLE: AtomicU64 = AtomicU64::new(0);

static WATCHDOG_REQUESTS: AtomicU64 = AtomicU64::new(0);
static WATCHDOG_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static WATCHDOG_PENDING: AtomicU64 = AtomicU64::new(0);
static WATCHDOG_REQUESTED_NS: AtomicU64 = AtomicU64::new(0);
static WATCHDOG_TIMEOUT_BASELINE: AtomicU64 = AtomicU64::new(0);
static WATCHDOG_TIMEOUT_OBSERVED: AtomicU64 = AtomicU64::new(0);

static DEGRADED_OUTPUT_SLOT: AtomicU64 = AtomicU64::new(u64::MAX);
static DEGRADED_OUTPUT_WRITE_GENERATION: AtomicU64 = AtomicU64::new(0);
static DEGRADED_OUTPUT_FRAME: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelWriteKind {
    Bytes,
    Transfer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChannelTransportEvidence {
    pub owner_pid: u64,
    pub peer_pid: Option<u64>,
    pub raw_handle: u64,
    pub surface_binding_match: bool,
    pub input_binding_match: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceSupervisorTraceSnapshot {
    pub bits: u64,
    pub probe_read_bits: u64,
    pub errors: u64,
    pub decode_errors: u64,
    pub owner_errors: u64,
    pub phase_errors: u64,
    pub payload_errors: u64,
    pub messages: u64,
    pub fault_messages: u64,
    pub probe_messages: u64,
    pub probe_reads: u64,
    pub healthy_messages: u64,
    pub quarantine_messages: u64,
    pub degraded_ack_messages: u64,
    pub old_input_pid: u64,
    pub new_input_pid: u64,
    pub input_generation: u64,
    pub surface_transport_owner_pid: u64,
    pub surface_transport_handle: u64,
    pub input_transport_owner_pid: u64,
    pub input_transport_handle: u64,
    pub watchdog_requests: u64,
    pub watchdog_timeouts: u64,
    pub watchdog_requested_ns: u64,
    pub watchdog_timeout_baseline: u64,
    pub watchdog_timeout_observed: u64,
    pub degraded_output_slot: u64,
    pub degraded_output_write_generation: u64,
    pub degraded_output_frame: u64,
    pub first_fault: bool,
    pub first_healthy: bool,
    pub watchdog_complete: bool,
    pub quarantined: bool,
    pub complete: bool,
}

pub fn snapshot() -> ServiceSupervisorTraceSnapshot {
    let bits = BITS.load(Ordering::Acquire);
    let probe_read_bits = PROBE_READ_BITS.load(Ordering::Acquire);
    let errors = ERRORS.load(Ordering::Acquire);
    let messages = MESSAGES.load(Ordering::Acquire);
    let fault_messages = FAULT_MESSAGES.load(Ordering::Acquire);
    let probe_messages = PROBE_MESSAGES.load(Ordering::Acquire);
    let healthy_messages = HEALTHY_MESSAGES.load(Ordering::Acquire);
    let quarantine_messages = QUARANTINE_MESSAGES.load(Ordering::Acquire);
    let degraded_ack_messages = DEGRADED_ACK_MESSAGES.load(Ordering::Acquire);
    let surface_transport_owner_pid = SURFACE_TRANSPORT_OWNER_PID.load(Ordering::Acquire);
    let surface_transport_handle = SURFACE_TRANSPORT_HANDLE.load(Ordering::Acquire);
    let input_transport_owner_pid = INPUT_TRANSPORT_OWNER_PID.load(Ordering::Acquire);
    let input_transport_handle = INPUT_TRANSPORT_HANDLE.load(Ordering::Acquire);
    ServiceSupervisorTraceSnapshot {
        bits,
        probe_read_bits,
        errors,
        decode_errors: DECODE_ERRORS.load(Ordering::Acquire),
        owner_errors: OWNER_ERRORS.load(Ordering::Acquire),
        phase_errors: PHASE_ERRORS.load(Ordering::Acquire),
        payload_errors: PAYLOAD_ERRORS.load(Ordering::Acquire),
        messages,
        fault_messages,
        probe_messages,
        probe_reads: PROBE_READS.load(Ordering::Acquire),
        healthy_messages,
        quarantine_messages,
        degraded_ack_messages,
        old_input_pid: OLD_INPUT_PID.load(Ordering::Acquire),
        new_input_pid: NEW_INPUT_PID.load(Ordering::Acquire),
        input_generation: INPUT_GENERATION.load(Ordering::Acquire),
        surface_transport_owner_pid,
        surface_transport_handle,
        input_transport_owner_pid,
        input_transport_handle,
        watchdog_requests: WATCHDOG_REQUESTS.load(Ordering::Acquire),
        watchdog_timeouts: WATCHDOG_TIMEOUTS.load(Ordering::Acquire),
        watchdog_requested_ns: WATCHDOG_REQUESTED_NS.load(Ordering::Acquire),
        watchdog_timeout_baseline: WATCHDOG_TIMEOUT_BASELINE.load(Ordering::Acquire),
        watchdog_timeout_observed: WATCHDOG_TIMEOUT_OBSERVED.load(Ordering::Acquire),
        degraded_output_slot: DEGRADED_OUTPUT_SLOT.load(Ordering::Acquire),
        degraded_output_write_generation: DEGRADED_OUTPUT_WRITE_GENERATION.load(Ordering::Acquire),
        degraded_output_frame: DEGRADED_OUTPUT_FRAME.load(Ordering::Acquire),
        first_fault: bits & OLD_PROCESS_EXIT != 0,
        first_healthy: bits & (FIRST_PROBE | FIRST_HEALTHY) == (FIRST_PROBE | FIRST_HEALTHY)
            && probe_read_bits & FIRST_PROBE_READ != 0,
        watchdog_complete: bits & (SECOND_PROBE | WATCHDOG_REQUESTED | WATCHDOG_TIMED_OUT)
            == (SECOND_PROBE | WATCHDOG_REQUESTED | WATCHDOG_TIMED_OUT)
            && probe_read_bits & SECOND_PROBE_READ != 0,
        quarantined: bits & (QUARANTINE | DEGRADED_OUTPUT | DEGRADED_ACK)
            == (QUARANTINE | DEGRADED_OUTPUT | DEGRADED_ACK),
        complete: bits == REQUIRED_BITS
            && probe_read_bits == REQUIRED_PROBE_READ_BITS
            && PROBE_READS.load(Ordering::Acquire) == 2
            && WATCHDOG_PENDING.load(Ordering::Acquire) == 0
            && surface_transport_owner_pid != 0
            && surface_transport_handle != 0
            && input_transport_owner_pid != 0
            && input_transport_handle != 0
            && errors == 0
            && messages == 6
            && fault_messages == 1
            && probe_messages == 2
            && healthy_messages == 1
            && quarantine_messages == 1
            && degraded_ack_messages == 1,
    }
}

/// Records one committed channel write. Non-BSH1 families are ignored.
pub fn record_channel_write(
    sender_pid: u64,
    sender_image: UserImageId,
    transport: ChannelTransportEvidence,
    wire: &[u8],
    write_kind: ChannelWriteKind,
) -> bool {
    let restart = crate::input_server_restart_trace::snapshot();
    let Some(transport_kind) = supervisor_transport_kind(sender_pid, transport.peer_pid, restart)
    else {
        return true;
    };
    if !transport_matches_registered_binding(transport_kind, transport) {
        return true;
    }
    if !wire.starts_with(b"BSH1") {
        return true;
    }
    let Ok(wire) = <&[u8; HEALTH_FRAME_SIZE]>::try_from(wire) else {
        return reject_decode();
    };
    let Ok(frame) = HealthFrame::decode(wire) else {
        return reject_decode();
    };
    if write_kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }

    let identity = frame.identity();
    if identity.kind() != ServiceKind::InputServer {
        return reject_payload();
    }
    let bits = BITS.load(Ordering::Acquire);
    let bit = match frame.opcode() {
        HealthOpcode::Fault => {
            if bits & OLD_PROCESS_EXIT != 0
                || !restart.gap_ready
                || transport_kind != SupervisorTransportKind::Surface
                || sender_image != UserImageId::SurfaceServer
                || sender_pid != restart.surface_pid
                || frame.sequence() != 1
                || frame.fault_class() != Some(FaultClass::ProcessExit)
                || identity.pid() != restart.old_input_pid
                || !identity_matches_pid(identity)
            {
                return reject_owner_or_payload(
                    sender_image,
                    UserImageId::SurfaceServer,
                    sender_pid,
                    restart.surface_pid,
                );
            }
            if !bind_transport(
                SupervisorTransportKind::Surface,
                transport,
                restart.surface_pid,
            ) {
                return false;
            }
            OLD_INPUT_PID.store(identity.pid(), Ordering::Release);
            FAULT_MESSAGES.fetch_add(1, Ordering::Relaxed);
            OLD_PROCESS_EXIT
        }
        HealthOpcode::Probe if bits & FIRST_PROBE == 0 => {
            if bits & OLD_PROCESS_EXIT == 0
                || !restart.complete
                || transport_kind != SupervisorTransportKind::Input
                || sender_image != UserImageId::Init
                || sender_pid != restart.init_pid
                || frame.sequence() != 1
                || frame.interval_ns() != SERVICE_WATCHDOG_TIMEOUT_NS
                || identity.pid() != restart.new_input_pid
                || !identity_matches_pid(identity)
            {
                return reject_owner_or_payload(
                    sender_image,
                    UserImageId::Init,
                    sender_pid,
                    restart.init_pid,
                );
            }
            if !bind_transport(SupervisorTransportKind::Input, transport, restart.init_pid) {
                return false;
            }
            NEW_INPUT_PID.store(identity.pid(), Ordering::Release);
            INPUT_GENERATION.store(u64::from(identity.generation()), Ordering::Release);
            PROBE_MESSAGES.fetch_add(1, Ordering::Relaxed);
            FIRST_PROBE
        }
        HealthOpcode::Healthy => {
            if bits & FIRST_PROBE == 0
                || PROBE_READ_BITS.load(Ordering::Acquire) & FIRST_PROBE_READ == 0
                || bits & FIRST_HEALTHY != 0
                || transport_kind != SupervisorTransportKind::Input
                || sender_image != UserImageId::InputServer
                || sender_pid != restart.new_input_pid
                || frame.sequence() != 1
                || identity.pid() != restart.new_input_pid
                || !identity_matches_pid(identity)
            {
                return reject_owner_or_payload(
                    sender_image,
                    UserImageId::InputServer,
                    sender_pid,
                    restart.new_input_pid,
                );
            }
            HEALTHY_MESSAGES.fetch_add(1, Ordering::Relaxed);
            FIRST_HEALTHY
        }
        HealthOpcode::Probe => {
            if bits & FIRST_HEALTHY == 0
                || bits & SECOND_PROBE != 0
                || transport_kind != SupervisorTransportKind::Input
                || sender_image != UserImageId::Init
                || sender_pid != restart.init_pid
                || frame.sequence() != 2
                || frame.interval_ns() != SERVICE_WATCHDOG_TIMEOUT_NS
                || identity.pid() != restart.new_input_pid
                || !identity_matches_pid(identity)
            {
                return reject_owner_or_payload(
                    sender_image,
                    UserImageId::Init,
                    sender_pid,
                    restart.init_pid,
                );
            }
            PROBE_MESSAGES.fetch_add(1, Ordering::Relaxed);
            SECOND_PROBE
        }
        HealthOpcode::Quarantine => {
            if bits & WATCHDOG_TIMED_OUT == 0
                || bits & QUARANTINE != 0
                || transport_kind != SupervisorTransportKind::Surface
                || sender_image != UserImageId::Init
                || sender_pid != restart.init_pid
                || frame.sequence() != 3
                || frame.fault_class() != Some(FaultClass::RestartBudgetExhausted)
                || frame.restart_attempt() != 2
                || frame.restart_budget() != 1
                || identity.pid() != restart.new_input_pid
                || !identity_matches_pid(identity)
            {
                return reject_owner_or_payload(
                    sender_image,
                    UserImageId::Init,
                    sender_pid,
                    restart.init_pid,
                );
            }
            QUARANTINE_MESSAGES.fetch_add(1, Ordering::Relaxed);
            QUARANTINE
        }
        HealthOpcode::DegradedAck => {
            if bits & (QUARANTINE | DEGRADED_OUTPUT) != (QUARANTINE | DEGRADED_OUTPUT)
                || bits & DEGRADED_ACK != 0
                || transport_kind != SupervisorTransportKind::Surface
                || sender_image != UserImageId::SurfaceServer
                || sender_pid != restart.surface_pid
                || frame.sequence() != 2
                || identity.pid() != restart.new_input_pid
                || !identity_matches_pid(identity)
            {
                return reject_owner_or_payload(
                    sender_image,
                    UserImageId::SurfaceServer,
                    sender_pid,
                    restart.surface_pid,
                );
            }
            DEGRADED_ACK_MESSAGES.fetch_add(1, Ordering::Relaxed);
            DEGRADED_ACK
        }
    };
    MESSAGES.fetch_add(1, Ordering::Relaxed);
    commit_bit(bit)
}

/// Records one successfully dequeued health envelope. Only reads performed by
/// the replacement InputServer on its live Init peer are candidates; BSH1 on
/// every other transport is deliberately invisible to this trace.
#[allow(clippy::too_many_arguments)]
pub fn record_channel_read(
    reader_pid: u64,
    reader_image: UserImageId,
    transport: ChannelTransportEvidence,
    sender_pid: u64,
    wire: &[u8],
    read_kind: ChannelWriteKind,
) -> bool {
    let restart = crate::input_server_restart_trace::snapshot();
    if reader_pid != restart.new_input_pid
        || restart.new_input_pid == 0
        || transport.peer_pid != Some(restart.init_pid)
    {
        return true;
    }
    if INPUT_TRANSPORT_OWNER_PID.load(Ordering::Acquire) != 0 && !transport.input_binding_match {
        return true;
    }
    if !wire.starts_with(b"BSH1") {
        return true;
    }
    let Ok(wire) = <&[u8; HEALTH_FRAME_SIZE]>::try_from(wire) else {
        return reject_decode();
    };
    let Ok(frame) = HealthFrame::decode(wire) else {
        return reject_decode();
    };
    if transport.owner_pid != reader_pid
        || reader_image != UserImageId::InputServer
        || sender_pid != restart.init_pid
        || restart.init_pid == 0
    {
        return reject_owner();
    }
    let identity = frame.identity();
    if INPUT_TRANSPORT_OWNER_PID.load(Ordering::Acquire) == 0 {
        return reject_phase();
    }
    if read_kind != ChannelWriteKind::Bytes
        || frame.opcode() != HealthOpcode::Probe
        || identity.kind() != ServiceKind::InputServer
        || identity.pid() != restart.new_input_pid
        || !identity_matches_pid(identity)
        || frame.interval_ns() != SERVICE_WATCHDOG_TIMEOUT_NS
    {
        return reject_payload();
    }

    let bits = BITS.load(Ordering::Acquire);
    let read_bits = PROBE_READ_BITS.load(Ordering::Acquire);
    let read_bit = match frame.sequence() {
        1 => {
            if bits & FIRST_PROBE == 0
                || bits & FIRST_HEALTHY != 0
                || read_bits & FIRST_PROBE_READ != 0
            {
                return reject_phase();
            }
            FIRST_PROBE_READ
        }
        2 => {
            if bits & (FIRST_HEALTHY | SECOND_PROBE) != (FIRST_HEALTHY | SECOND_PROBE)
                || bits & WATCHDOG_REQUESTED != 0
                || read_bits & FIRST_PROBE_READ == 0
                || read_bits & SECOND_PROBE_READ != 0
            {
                return reject_phase();
            }
            SECOND_PROBE_READ
        }
        _ => return reject_payload(),
    };
    let previous = PROBE_READ_BITS.fetch_or(read_bit, Ordering::AcqRel);
    if previous & read_bit != 0 {
        return reject_phase();
    }
    PROBE_READS.fetch_add(1, Ordering::Relaxed);
    if read_bit == SECOND_PROBE_READ {
        publish_pending_watchdog_request()
    } else {
        true
    }
}

#[allow(clippy::too_many_arguments)]
pub fn record_watchdog_request(
    init_pid: u64,
    requested_ns: u64,
    stable_input_peer: bool,
    created: u64,
    exited: u64,
    reaped: u64,
    live: u64,
    timeout_wakes: u64,
) -> bool {
    let restart = crate::input_server_restart_trace::snapshot();
    let bits = BITS.load(Ordering::Acquire);
    if init_pid != restart.init_pid
        || init_pid == 0
        || requested_ns != SERVICE_WATCHDOG_TIMEOUT_NS
        || !stable_input_peer
        || bits & SECOND_PROBE == 0
        || bits & (WATCHDOG_REQUESTED | WATCHDOG_TIMED_OUT) != 0
        || WATCHDOG_PENDING.load(Ordering::Acquire) != 0
        || created != 11
        || exited != 2
        || reaped != 2
        || live != 9
    {
        return reject_phase();
    }
    WATCHDOG_REQUESTED_NS.store(requested_ns, Ordering::Release);
    WATCHDOG_TIMEOUT_BASELINE.store(timeout_wakes, Ordering::Release);
    if PROBE_READ_BITS.load(Ordering::Acquire) & SECOND_PROBE_READ != 0 {
        publish_watchdog_request()
    } else {
        // Init may enter the finite wait before the single-core scheduler has
        // run InputServer. Preserve the authenticated wait request, but do
        // not publish watchdog evidence until Probe 2 is actually dequeued.
        WATCHDOG_PENDING.store(1, Ordering::Release);
        true
    }
}

pub fn record_watchdog_timeout(
    init_pid: u64,
    created: u64,
    exited: u64,
    reaped: u64,
    live: u64,
    timeout_wakes: u64,
) -> bool {
    let restart = crate::input_server_restart_trace::snapshot();
    let bits = BITS.load(Ordering::Acquire);
    let baseline = WATCHDOG_TIMEOUT_BASELINE.load(Ordering::Acquire);
    if init_pid != restart.init_pid
        || init_pid == 0
        || bits & WATCHDOG_REQUESTED == 0
        || PROBE_READ_BITS.load(Ordering::Acquire) & SECOND_PROBE_READ == 0
        || bits & WATCHDOG_TIMED_OUT != 0
        || created != 11
        || exited != 2
        || reaped != 2
        || live != 9
        || timeout_wakes != baseline.saturating_add(1)
    {
        return reject_phase();
    }
    WATCHDOG_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
    WATCHDOG_TIMEOUT_OBSERVED.store(timeout_wakes, Ordering::Release);
    commit_bit(WATCHDOG_TIMED_OUT)
}

#[allow(clippy::too_many_arguments)]
pub fn record_degraded_output(
    process_id: u64,
    surface_session: u64,
    slot: usize,
    allocation_generation: u32,
    write_generation: u64,
    frame_id: u32,
) -> bool {
    let bits = BITS.load(Ordering::Acquire);
    if bits & QUARANTINE == 0 || bits & DEGRADED_OUTPUT != 0 {
        return true;
    }
    let restart = crate::input_server_restart_trace::snapshot();
    if process_id != restart.surface_pid
        || process_id == 0
        || surface_session != 1
        || allocation_generation != 1
    {
        return reject_owner();
    }
    if slot != 0 || write_generation != 6 || frame_id != 9 {
        return reject_payload();
    }
    DEGRADED_OUTPUT_SLOT.store(slot as u64, Ordering::Release);
    DEGRADED_OUTPUT_WRITE_GENERATION.store(write_generation, Ordering::Release);
    DEGRADED_OUTPUT_FRAME.store(u64::from(frame_id), Ordering::Release);
    commit_bit(DEGRADED_OUTPUT)
}

fn identity_matches_pid(identity: ServiceIdentity) -> bool {
    identity.pid() != 0 && identity.generation() == (identity.pid() >> 32) as u32
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SupervisorTransportKind {
    Surface,
    Input,
}

fn supervisor_transport_kind(
    sender_pid: u64,
    transport_peer_pid: Option<u64>,
    restart: crate::input_server_restart_trace::InputServerRestartTraceSnapshot,
) -> Option<SupervisorTransportKind> {
    let peer_pid = transport_peer_pid?;
    if restart.init_pid == 0 {
        return None;
    }
    if restart.surface_pid != 0
        && ((sender_pid == restart.init_pid && peer_pid == restart.surface_pid)
            || (sender_pid == restart.surface_pid && peer_pid == restart.init_pid))
    {
        return Some(SupervisorTransportKind::Surface);
    }
    if restart.new_input_pid != 0
        && ((sender_pid == restart.init_pid && peer_pid == restart.new_input_pid)
            || (sender_pid == restart.new_input_pid && peer_pid == restart.init_pid))
    {
        return Some(SupervisorTransportKind::Input);
    }
    None
}

fn transport_matches_registered_binding(
    kind: SupervisorTransportKind,
    transport: ChannelTransportEvidence,
) -> bool {
    match kind {
        SupervisorTransportKind::Surface => {
            SURFACE_TRANSPORT_OWNER_PID.load(Ordering::Acquire) == 0
                || transport.surface_binding_match
        }
        SupervisorTransportKind::Input => {
            INPUT_TRANSPORT_OWNER_PID.load(Ordering::Acquire) == 0 || transport.input_binding_match
        }
    }
}

fn bind_transport(
    kind: SupervisorTransportKind,
    transport: ChannelTransportEvidence,
    expected_owner_pid: u64,
) -> bool {
    if transport.owner_pid != expected_owner_pid
        || expected_owner_pid == 0
        || u32::try_from(transport.raw_handle).map_or(true, |handle| handle == 0)
    {
        return reject_owner();
    }
    let (owner, handle) = match kind {
        SupervisorTransportKind::Surface => {
            (&SURFACE_TRANSPORT_OWNER_PID, &SURFACE_TRANSPORT_HANDLE)
        }
        SupervisorTransportKind::Input => (&INPUT_TRANSPORT_OWNER_PID, &INPUT_TRANSPORT_HANDLE),
    };
    if owner.load(Ordering::Acquire) != 0 {
        return reject_phase();
    }
    handle.store(transport.raw_handle, Ordering::Release);
    owner.store(expected_owner_pid, Ordering::Release);
    true
}

fn publish_pending_watchdog_request() -> bool {
    if WATCHDOG_PENDING.swap(0, Ordering::AcqRel) == 0 {
        return true;
    }
    publish_watchdog_request()
}

fn publish_watchdog_request() -> bool {
    if PROBE_READ_BITS.load(Ordering::Acquire) & SECOND_PROBE_READ == 0
        || BITS.load(Ordering::Acquire) & (WATCHDOG_REQUESTED | WATCHDOG_TIMED_OUT) != 0
        || WATCHDOG_REQUESTS.load(Ordering::Acquire) != 0
    {
        return reject_phase();
    }
    WATCHDOG_REQUESTS.fetch_add(1, Ordering::Relaxed);
    commit_bit(WATCHDOG_REQUESTED)
}

fn commit_bit(bit: u64) -> bool {
    let previous = BITS.fetch_or(bit, Ordering::AcqRel);
    if previous & bit != 0 {
        return reject_phase();
    }
    true
}

fn reject_owner_or_payload(
    actual_image: UserImageId,
    expected_image: UserImageId,
    actual_pid: u64,
    expected_pid: u64,
) -> bool {
    if actual_image != expected_image || actual_pid != expected_pid || expected_pid == 0 {
        reject_owner()
    } else {
        reject_payload()
    }
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
