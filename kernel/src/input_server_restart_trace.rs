//! Allocation-free M47 InputServer restart evidence.
//!
//! The trace is deliberately independent from the M41 window transcript. It
//! accepts only canonical BIR1/BIP1/BIC1/BIE1 candidates, authenticates their
//! kernel-stamped sender image/PID, and advances one closed restart state
//! machine. Older unrelated wires are ignored. A malformed candidate, wrong
//! owner, replay, gap, or phase violation records an error without advancing
//! any success state.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bndr_abi::UserImageId;
use bndr_input::{
    CommandStream, InputCommand, InputCommandPayload, InputEvent, InputEventPayload, InputRoute,
    InputRouteControl, InputRouteControlPayload, InputServiceFocus, InputServiceRecoveryMessage,
    InputServiceRecoveryPayload, InputServiceRecoveryPhase, WindowRef,
};

pub const INPUT_RESTART_BACKOFF_NS: u64 = 30_000_000;

const BIR_OLD_ACQUIRED: u64 = 1 << 0;
const BIR_OLD_BOUND: u64 = 1 << 1;
const BIR_OLD_READY: u64 = 1 << 2;
const PERMISSION_DENIED: u64 = 1 << 3;
const BIP_ARMED: u64 = 1 << 4;
const OUTPUT_GAP: u64 = 1 << 5;
const BIP_RESTART_REQUESTED: u64 = 1 << 6;
const BIP_ROUTE_LOST: u64 = 1 << 7;
const BACKOFF_REQUESTED: u64 = 1 << 8;
const BACKOFF_TIMED_OUT: u64 = 1 << 9;
const BIR_NEW_ACQUIRED: u64 = 1 << 10;
const BIR_NEW_BOUND: u64 = 1 << 11;
const BIR_NEW_READY: u64 = 1 << 12;
const BIE_NEW_READY_PENDING: u64 = 1 << 13;
const BIP_REBIND_OFFER: u64 = 1 << 14;
const BIP_RESYNC_PREPARED: u64 = 1 << 15;
const BIC_RESET: u64 = 1 << 16;
const BIC_LAUNCHER_ROUTE: u64 = 1 << 17;
const BIC_APP_ROUTE: u64 = 1 << 18;
const BIC_LAUNCHER_FOCUS: u64 = 1 << 19;
const BIC_TEXT_NONE: u64 = 1 << 20;
const BIC_SCENE_COMMIT: u64 = 1 << 21;
const BIE_RESYNC_ACKS: u64 = 1 << 22;
const BIE_APP_DOWN: u64 = 1 << 23;
const BIE_APP_UP: u64 = 1 << 24;
const OUTPUT_POST: u64 = 1 << 25;
const BIP_ACTIVE: u64 = 1 << 26;

const RESYNC_COMMAND_BITS: u64 = BIC_RESET
    | BIC_LAUNCHER_ROUTE
    | BIC_APP_ROUTE
    | BIC_LAUNCHER_FOCUS
    | BIC_TEXT_NONE
    | BIC_SCENE_COMMIT;
const REQUIRED_BITS: u64 = (1 << 27) - 1;

static BITS: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);
static DECODE_ERRORS: AtomicU64 = AtomicU64::new(0);
static OWNER_ERRORS: AtomicU64 = AtomicU64::new(0);
static PHASE_ERRORS: AtomicU64 = AtomicU64::new(0);
static PAYLOAD_ERRORS: AtomicU64 = AtomicU64::new(0);

static BIR_MESSAGES: AtomicU64 = AtomicU64::new(0);
static BIP_MESSAGES: AtomicU64 = AtomicU64::new(0);
static BIC_RESYNC_MESSAGES: AtomicU64 = AtomicU64::new(0);
static BIE_RESYNC_MESSAGES: AtomicU64 = AtomicU64::new(0);
static BIE_ROUTED_MESSAGES: AtomicU64 = AtomicU64::new(0);
static OUTPUT_COMMITS: AtomicU64 = AtomicU64::new(0);

static INIT_PID: AtomicU64 = AtomicU64::new(0);
static SURFACE_PID: AtomicU64 = AtomicU64::new(0);
static OLD_INPUT_PID: AtomicU64 = AtomicU64::new(0);
static NEW_INPUT_PID: AtomicU64 = AtomicU64::new(0);
static LAUNCHER_PID: AtomicU64 = AtomicU64::new(0);
static APP_PID: AtomicU64 = AtomicU64::new(0);
static OLD_INPUT_SESSION: AtomicU64 = AtomicU64::new(0);
static NEW_INPUT_SESSION: AtomicU64 = AtomicU64::new(0);
static OLD_ROUTE_EPOCH: AtomicU64 = AtomicU64::new(0);
static NEW_ROUTE_EPOCH: AtomicU64 = AtomicU64::new(0);
static OLD_FLOOR: AtomicU64 = AtomicU64::new(0);
static NEW_FLOOR: AtomicU64 = AtomicU64::new(0);

static OUTSTANDING_STREAM: AtomicU64 = AtomicU64::new(0);
static OUTSTANDING_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static LAST_BIE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static RESYNC_ACKS: AtomicU64 = AtomicU64::new(0);

static PERMISSION_AUDITS: AtomicU64 = AtomicU64::new(0);
static PERMISSION_CALLER_PID: AtomicU64 = AtomicU64::new(0);
static PERMISSION_HANDLES_DELTA: AtomicU64 = AtomicU64::new(0);
static PERMISSION_SESSIONS_DELTA: AtomicU64 = AtomicU64::new(0);

static BACKOFF_REQUESTS: AtomicU64 = AtomicU64::new(0);
static BACKOFF_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static BACKOFF_REQUESTED_NS: AtomicU64 = AtomicU64::new(0);
static BACKOFF_TIMEOUT_BASELINE: AtomicU64 = AtomicU64::new(0);
static BACKOFF_TIMEOUT_OBSERVED: AtomicU64 = AtomicU64::new(0);
static BACKOFF_EARLY_SPAWN: AtomicBool = AtomicBool::new(false);

static GAP_OUTPUT_SLOT: AtomicU64 = AtomicU64::new(u64::MAX);
static GAP_OUTPUT_WRITE_GENERATION: AtomicU64 = AtomicU64::new(0);
static GAP_OUTPUT_FRAME: AtomicU64 = AtomicU64::new(0);
static POST_OUTPUT_SLOT: AtomicU64 = AtomicU64::new(u64::MAX);
static POST_OUTPUT_WRITE_GENERATION: AtomicU64 = AtomicU64::new(0);
static POST_OUTPUT_FRAME: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelWriteKind {
    Bytes,
    Transfer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputServerRestartTraceSnapshot {
    pub bits: u64,
    pub errors: u64,
    pub decode_errors: u64,
    pub owner_errors: u64,
    pub phase_errors: u64,
    pub payload_errors: u64,
    pub bir_messages: u64,
    pub bip_messages: u64,
    pub bic_resync_messages: u64,
    pub bie_resync_messages: u64,
    pub bie_routed_messages: u64,
    pub output_commits: u64,
    pub init_pid: u64,
    pub surface_pid: u64,
    pub old_input_pid: u64,
    pub new_input_pid: u64,
    pub launcher_pid: u64,
    pub app_pid: u64,
    pub old_input_session: u64,
    pub new_input_session: u64,
    pub old_route_epoch: u64,
    pub new_route_epoch: u64,
    pub old_floor: u64,
    pub new_floor: u64,
    pub resync_acks: u64,
    pub permission_audits: u64,
    pub permission_caller_pid: u64,
    pub permission_handles_delta: u64,
    pub permission_sessions_delta: u64,
    pub backoff_requests: u64,
    pub backoff_timeouts: u64,
    pub backoff_requested_ns: u64,
    pub backoff_timeout_baseline: u64,
    pub backoff_timeout_observed: u64,
    pub backoff_early_spawn: bool,
    pub gap_output_slot: u64,
    pub gap_output_write_generation: u64,
    pub gap_output_frame: u64,
    pub post_output_slot: u64,
    pub post_output_write_generation: u64,
    pub post_output_frame: u64,
    pub armed: bool,
    pub gap_ready: bool,
    pub backoff_complete: bool,
    pub reacquired: bool,
    pub resync_complete: bool,
    pub route_complete: bool,
    pub complete: bool,
}

pub fn snapshot() -> InputServerRestartTraceSnapshot {
    let bits = BITS.load(Ordering::Acquire);
    let errors = ERRORS.load(Ordering::Acquire);
    let bir_messages = BIR_MESSAGES.load(Ordering::Acquire);
    let bip_messages = BIP_MESSAGES.load(Ordering::Acquire);
    let bic_resync_messages = BIC_RESYNC_MESSAGES.load(Ordering::Acquire);
    let bie_resync_messages = BIE_RESYNC_MESSAGES.load(Ordering::Acquire);
    let bie_routed_messages = BIE_ROUTED_MESSAGES.load(Ordering::Acquire);
    let output_commits = OUTPUT_COMMITS.load(Ordering::Acquire);
    InputServerRestartTraceSnapshot {
        bits,
        errors,
        decode_errors: DECODE_ERRORS.load(Ordering::Acquire),
        owner_errors: OWNER_ERRORS.load(Ordering::Acquire),
        phase_errors: PHASE_ERRORS.load(Ordering::Acquire),
        payload_errors: PAYLOAD_ERRORS.load(Ordering::Acquire),
        bir_messages,
        bip_messages,
        bic_resync_messages,
        bie_resync_messages,
        bie_routed_messages,
        output_commits,
        init_pid: INIT_PID.load(Ordering::Acquire),
        surface_pid: SURFACE_PID.load(Ordering::Acquire),
        old_input_pid: OLD_INPUT_PID.load(Ordering::Acquire),
        new_input_pid: NEW_INPUT_PID.load(Ordering::Acquire),
        launcher_pid: LAUNCHER_PID.load(Ordering::Acquire),
        app_pid: APP_PID.load(Ordering::Acquire),
        old_input_session: OLD_INPUT_SESSION.load(Ordering::Acquire),
        new_input_session: NEW_INPUT_SESSION.load(Ordering::Acquire),
        old_route_epoch: OLD_ROUTE_EPOCH.load(Ordering::Acquire),
        new_route_epoch: NEW_ROUTE_EPOCH.load(Ordering::Acquire),
        old_floor: OLD_FLOOR.load(Ordering::Acquire),
        new_floor: NEW_FLOOR.load(Ordering::Acquire),
        resync_acks: RESYNC_ACKS.load(Ordering::Acquire),
        permission_audits: PERMISSION_AUDITS.load(Ordering::Acquire),
        permission_caller_pid: PERMISSION_CALLER_PID.load(Ordering::Acquire),
        permission_handles_delta: PERMISSION_HANDLES_DELTA.load(Ordering::Acquire),
        permission_sessions_delta: PERMISSION_SESSIONS_DELTA.load(Ordering::Acquire),
        backoff_requests: BACKOFF_REQUESTS.load(Ordering::Acquire),
        backoff_timeouts: BACKOFF_TIMEOUTS.load(Ordering::Acquire),
        backoff_requested_ns: BACKOFF_REQUESTED_NS.load(Ordering::Acquire),
        backoff_timeout_baseline: BACKOFF_TIMEOUT_BASELINE.load(Ordering::Acquire),
        backoff_timeout_observed: BACKOFF_TIMEOUT_OBSERVED.load(Ordering::Acquire),
        backoff_early_spawn: BACKOFF_EARLY_SPAWN.load(Ordering::Acquire),
        gap_output_slot: GAP_OUTPUT_SLOT.load(Ordering::Acquire),
        gap_output_write_generation: GAP_OUTPUT_WRITE_GENERATION.load(Ordering::Acquire),
        gap_output_frame: GAP_OUTPUT_FRAME.load(Ordering::Acquire),
        post_output_slot: POST_OUTPUT_SLOT.load(Ordering::Acquire),
        post_output_write_generation: POST_OUTPUT_WRITE_GENERATION.load(Ordering::Acquire),
        post_output_frame: POST_OUTPUT_FRAME.load(Ordering::Acquire),
        armed: bits & (BIR_OLD_READY | BIP_ARMED) == (BIR_OLD_READY | BIP_ARMED),
        gap_ready: bits & BIP_ROUTE_LOST != 0,
        backoff_complete: bits & (BACKOFF_REQUESTED | BACKOFF_TIMED_OUT)
            == (BACKOFF_REQUESTED | BACKOFF_TIMED_OUT),
        reacquired: bits & (BIR_NEW_ACQUIRED | BIR_NEW_BOUND | BIR_NEW_READY)
            == (BIR_NEW_ACQUIRED | BIR_NEW_BOUND | BIR_NEW_READY),
        resync_complete: bits & (BIP_RESYNC_PREPARED | RESYNC_COMMAND_BITS | BIE_RESYNC_ACKS)
            == (BIP_RESYNC_PREPARED | RESYNC_COMMAND_BITS | BIE_RESYNC_ACKS),
        route_complete: bits & (BIE_APP_DOWN | BIE_APP_UP) == (BIE_APP_DOWN | BIE_APP_UP),
        complete: bits == REQUIRED_BITS
            && errors == 0
            && bir_messages == 6
            && bip_messages == 6
            && bic_resync_messages == 6
            && bie_resync_messages == 7
            && bie_routed_messages == 2
            && output_commits == 2,
    }
}

/// Records one committed channel write. Non-M47 wire families are ignored.
pub fn record_channel_write(
    sender_pid: u64,
    sender_image: UserImageId,
    wire: &[u8],
    write_kind: ChannelWriteKind,
) -> bool {
    if wire.starts_with(b"BIR1") {
        let Ok(message) = InputRouteControl::decode(wire) else {
            return reject_decode();
        };
        return record_bir(sender_pid, sender_image, message, write_kind);
    }
    if wire.starts_with(b"BIP1") {
        let Ok(message) = InputServiceRecoveryMessage::decode(wire) else {
            return reject_decode();
        };
        return record_bip(sender_pid, sender_image, message, write_kind);
    }
    if wire.starts_with(b"BIC1") {
        let Ok(command) = InputCommand::decode(wire) else {
            return reject_decode();
        };
        return record_bic(sender_pid, sender_image, command, write_kind);
    }
    if wire.starts_with(b"BIE1") {
        let Ok(event) = InputEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bie(sender_pid, sender_image, event, write_kind);
    }
    true
}

#[allow(clippy::too_many_arguments)]
pub fn record_input_acquire_permission_denial(
    caller_pid: u64,
    caller_image: UserImageId,
    handles_before: usize,
    handles_after: usize,
    next_session_before: u64,
    next_session_after: u64,
) -> bool {
    let bits = BITS.load(Ordering::Acquire);
    let valid = caller_image == UserImageId::SurfaceServer
        && caller_pid == SURFACE_PID.load(Ordering::Acquire)
        && caller_pid != 0
        && bits & BIR_OLD_READY != 0
        && bits & PERMISSION_DENIED == 0
        && handles_before == handles_after
        && next_session_before == next_session_after
        && next_session_before == 2;
    if !valid {
        return reject_owner();
    }
    PERMISSION_CALLER_PID.store(caller_pid, Ordering::Release);
    PERMISSION_HANDLES_DELTA.store(
        u64::try_from(handles_after.saturating_sub(handles_before)).unwrap_or(u64::MAX),
        Ordering::Release,
    );
    PERMISSION_SESSIONS_DELTA.store(
        next_session_after.saturating_sub(next_session_before),
        Ordering::Release,
    );
    PERMISSION_AUDITS.fetch_add(1, Ordering::Relaxed);
    commit_bit(PERMISSION_DENIED)
}

#[allow(clippy::too_many_arguments)]
pub fn record_backoff_request(
    init_pid: u64,
    requested_ns: u64,
    stable_surface_peer: bool,
    created: u64,
    exited: u64,
    reaped: u64,
    live: u64,
    replacement_spawn_pid: u64,
    timeout_wakes: u64,
) -> bool {
    let bits = BITS.load(Ordering::Acquire);
    let valid = init_pid == INIT_PID.load(Ordering::Acquire)
        && init_pid != 0
        && requested_ns == INPUT_RESTART_BACKOFF_NS
        && stable_surface_peer
        && created == 10
        && exited == 2
        && reaped == 2
        && live == 8
        && replacement_spawn_pid == 0
        && bits & BIP_ROUTE_LOST != 0
        && bits & (BACKOFF_REQUESTED | BIR_NEW_ACQUIRED) == 0;
    if !valid {
        return reject_phase();
    }
    BACKOFF_REQUESTS.fetch_add(1, Ordering::Relaxed);
    BACKOFF_REQUESTED_NS.store(requested_ns, Ordering::Release);
    BACKOFF_TIMEOUT_BASELINE.store(timeout_wakes, Ordering::Release);
    commit_bit(BACKOFF_REQUESTED)
}

#[allow(clippy::too_many_arguments)]
pub fn record_backoff_timeout(
    init_pid: u64,
    created: u64,
    exited: u64,
    reaped: u64,
    live: u64,
    replacement_spawn_pid: u64,
    timeout_wakes: u64,
) -> bool {
    let bits = BITS.load(Ordering::Acquire);
    let baseline = BACKOFF_TIMEOUT_BASELINE.load(Ordering::Acquire);
    let early_spawn = created != 10 || live != 8 || replacement_spawn_pid != 0;
    BACKOFF_EARLY_SPAWN.store(early_spawn, Ordering::Release);
    let valid = init_pid == INIT_PID.load(Ordering::Acquire)
        && init_pid != 0
        && bits & BACKOFF_REQUESTED != 0
        && bits & BACKOFF_TIMED_OUT == 0
        && created == 10
        && exited == 2
        && reaped == 2
        && live == 8
        && replacement_spawn_pid == 0
        && timeout_wakes == baseline.saturating_add(1);
    if !valid {
        return reject_phase();
    }
    BACKOFF_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
    BACKOFF_TIMEOUT_OBSERVED.store(timeout_wakes, Ordering::Release);
    commit_bit(BACKOFF_TIMED_OUT)
}

/// Accounts the two M47-only output commits after the sealed M41 prefix.
#[allow(clippy::too_many_arguments)]
pub fn record_output_commit(
    process_id: u64,
    surface_session: u64,
    slot: usize,
    allocation_generation: u32,
    write_generation: u64,
    frame_id: u32,
) -> bool {
    let bits = BITS.load(Ordering::Acquire);
    if process_id != SURFACE_PID.load(Ordering::Acquire)
        || process_id == 0
        || surface_session != 1
        || allocation_generation != 1
    {
        return reject_owner();
    }
    let bit = if bits & OUTPUT_GAP == 0 {
        if bits & BIP_RESTART_REQUESTED == 0
            || bits & BIP_ROUTE_LOST != 0
            || slot != 0
            || write_generation != 4
            || frame_id != 7
        {
            return reject_payload();
        }
        GAP_OUTPUT_SLOT.store(slot as u64, Ordering::Release);
        GAP_OUTPUT_WRITE_GENERATION.store(write_generation, Ordering::Release);
        GAP_OUTPUT_FRAME.store(u64::from(frame_id), Ordering::Release);
        OUTPUT_GAP
    } else if bits & OUTPUT_POST == 0 {
        if bits & (BIP_RESYNC_PREPARED | RESYNC_COMMAND_BITS | BIE_RESYNC_ACKS)
            != (BIP_RESYNC_PREPARED | RESYNC_COMMAND_BITS | BIE_RESYNC_ACKS)
            || bits & (BIE_APP_DOWN | BIE_APP_UP) != (BIE_APP_DOWN | BIE_APP_UP)
            || slot != 0
            || write_generation != 5
            || frame_id != 8
        {
            return reject_payload();
        }
        POST_OUTPUT_SLOT.store(slot as u64, Ordering::Release);
        POST_OUTPUT_WRITE_GENERATION.store(write_generation, Ordering::Release);
        POST_OUTPUT_FRAME.store(u64::from(frame_id), Ordering::Release);
        OUTPUT_POST
    } else {
        return reject_phase();
    };
    OUTPUT_COMMITS.fetch_add(1, Ordering::Relaxed);
    commit_bit(bit)
}

fn record_bir(
    sender_pid: u64,
    sender_image: UserImageId,
    message: InputRouteControl,
    write_kind: ChannelWriteKind,
) -> bool {
    let bits = BITS.load(Ordering::Acquire);
    let bit = match message.payload() {
        InputRouteControlPayload::Acquired {
            physical_sequence_floor,
        } if bits & BIR_OLD_ACQUIRED == 0 => {
            if bits != 0
                || sender_image != UserImageId::InputServer
                || sender_pid == 0
                || write_kind != ChannelWriteKind::Bytes
                || message.sequence() != 1
                || message.route_epoch() != 0
                || message.input_session_id() != 1
                || physical_sequence_floor != 0
            {
                return reject_bir_owner_or_payload(sender_image, UserImageId::InputServer);
            }
            OLD_INPUT_PID.store(sender_pid, Ordering::Release);
            OLD_INPUT_SESSION.store(1, Ordering::Release);
            OLD_FLOOR.store(0, Ordering::Release);
            BIR_OLD_ACQUIRED
        }
        InputRouteControlPayload::Bind {
            expected_surface_pid,
            expected_physical_floor,
        } if bits & BIR_OLD_BOUND == 0 => {
            if bits & BIR_OLD_ACQUIRED == 0
                || sender_image != UserImageId::Init
                || sender_pid == 0
                || write_kind != ChannelWriteKind::Transfer
                || message.sequence() != 1
                || message.route_epoch() != 1
                || message.input_session_id() != 1
                || expected_physical_floor != 0
                || expected_surface_pid.get() == 0
            {
                return reject_bir_owner_or_payload(sender_image, UserImageId::Init);
            }
            INIT_PID.store(sender_pid, Ordering::Release);
            SURFACE_PID.store(expected_surface_pid.get(), Ordering::Release);
            OLD_ROUTE_EPOCH.store(1, Ordering::Release);
            BIR_OLD_BOUND
        }
        InputRouteControlPayload::Ready {
            bound_surface_pid,
            physical_sequence_floor,
        } if bits & BIR_OLD_READY == 0 => {
            if bits & BIR_OLD_BOUND == 0
                || sender_image != UserImageId::InputServer
                || sender_pid != OLD_INPUT_PID.load(Ordering::Acquire)
                || write_kind != ChannelWriteKind::Bytes
                || message.sequence() != 2
                || message.route_epoch() != 1
                || message.input_session_id() != 1
                || bound_surface_pid.get() != SURFACE_PID.load(Ordering::Acquire)
                || physical_sequence_floor != 0
            {
                return reject_bir_owner_or_payload(sender_image, UserImageId::InputServer);
            }
            BIR_OLD_READY
        }
        InputRouteControlPayload::Acquired {
            physical_sequence_floor,
        } => {
            let old_pid = OLD_INPUT_PID.load(Ordering::Acquire);
            if bits & BACKOFF_TIMED_OUT == 0
                || bits & BIR_NEW_ACQUIRED != 0
                || sender_image != UserImageId::InputServer
                || sender_pid == 0
                || sender_pid == old_pid
                || write_kind != ChannelWriteKind::Bytes
                || message.sequence() != 1
                || message.route_epoch() != 0
                || message.input_session_id() != 2
                || physical_sequence_floor != 1
                || !same_slot_next_generation(old_pid, sender_pid)
            {
                return reject_bir_owner_or_payload(sender_image, UserImageId::InputServer);
            }
            NEW_INPUT_PID.store(sender_pid, Ordering::Release);
            NEW_INPUT_SESSION.store(2, Ordering::Release);
            NEW_FLOOR.store(1, Ordering::Release);
            BIR_NEW_ACQUIRED
        }
        InputRouteControlPayload::Bind {
            expected_surface_pid,
            expected_physical_floor,
        } => {
            if bits & BIR_NEW_ACQUIRED == 0
                || bits & BIR_NEW_BOUND != 0
                || sender_image != UserImageId::Init
                || sender_pid != INIT_PID.load(Ordering::Acquire)
                || write_kind != ChannelWriteKind::Transfer
                || message.sequence() != 2
                || message.route_epoch() != 2
                || message.input_session_id() != 2
                || expected_surface_pid.get() != SURFACE_PID.load(Ordering::Acquire)
                || expected_physical_floor != 1
            {
                return reject_bir_owner_or_payload(sender_image, UserImageId::Init);
            }
            NEW_ROUTE_EPOCH.store(2, Ordering::Release);
            BIR_NEW_BOUND
        }
        InputRouteControlPayload::Ready {
            bound_surface_pid,
            physical_sequence_floor,
        } => {
            if bits & BIR_NEW_BOUND == 0
                || bits & BIR_NEW_READY != 0
                || sender_image != UserImageId::InputServer
                || sender_pid != NEW_INPUT_PID.load(Ordering::Acquire)
                || write_kind != ChannelWriteKind::Bytes
                || message.sequence() != 2
                || message.route_epoch() != 2
                || message.input_session_id() != 2
                || bound_surface_pid.get() != SURFACE_PID.load(Ordering::Acquire)
                || physical_sequence_floor != 1
            {
                return reject_bir_owner_or_payload(sender_image, UserImageId::InputServer);
            }
            BIR_NEW_READY
        }
        InputRouteControlPayload::RouteLost { .. } | InputRouteControlPayload::GapStatus { .. } => {
            return reject_phase();
        }
    };
    BIR_MESSAGES.fetch_add(1, Ordering::Relaxed);
    commit_bit(bit)
}

fn record_bip(
    sender_pid: u64,
    sender_image: UserImageId,
    message: InputServiceRecoveryMessage,
    write_kind: ChannelWriteKind,
) -> bool {
    let bits = BITS.load(Ordering::Acquire);
    let bit = match message.payload() {
        InputServiceRecoveryPayload::SurfaceStatus {
            input_pid,
            input_session_id,
            physical_sequence_floor,
            phase,
            route_count,
            focus,
            capture_active,
            text_active,
        } => {
            if write_kind != ChannelWriteKind::Bytes
                || sender_image != UserImageId::SurfaceServer
                || sender_pid != SURFACE_PID.load(Ordering::Acquire)
                || message.surface_session() != 1
                || route_count != 2
                || capture_active
                || text_active
            {
                return reject_owner();
            }
            match phase {
                InputServiceRecoveryPhase::Armed => {
                    if bits & (BIR_OLD_READY | PERMISSION_DENIED)
                        != (BIR_OLD_READY | PERMISSION_DENIED)
                        || bits & BIP_ARMED != 0
                        || message.sequence() != 1
                        || message.route_epoch() != 1
                        || input_pid.get() != OLD_INPUT_PID.load(Ordering::Acquire)
                        || input_session_id != 1
                        || physical_sequence_floor != 0
                        || focus != InputServiceFocus::Launcher
                    {
                        return reject_payload();
                    }
                    BIP_ARMED
                }
                InputServiceRecoveryPhase::RestartRequested => {
                    if bits & BIP_ARMED == 0
                        || bits & (BIP_RESTART_REQUESTED | OUTPUT_GAP | BIP_ROUTE_LOST) != 0
                        || message.sequence() != 2
                        || message.route_epoch() != 1
                        || input_pid.get() != OLD_INPUT_PID.load(Ordering::Acquire)
                        || input_session_id != 1
                        || physical_sequence_floor != 1
                        || focus != InputServiceFocus::Launcher
                    {
                        return reject_payload();
                    }
                    BIP_RESTART_REQUESTED
                }
                InputServiceRecoveryPhase::RouteLost => {
                    if bits & (BIP_RESTART_REQUESTED | OUTPUT_GAP)
                        != (BIP_RESTART_REQUESTED | OUTPUT_GAP)
                        || bits & BIP_ROUTE_LOST != 0
                        || message.sequence() != 3
                        || message.route_epoch() != 1
                        || input_pid.get() != OLD_INPUT_PID.load(Ordering::Acquire)
                        || input_session_id != 1
                        || physical_sequence_floor != 1
                        || focus != InputServiceFocus::Launcher
                    {
                        return reject_payload();
                    }
                    BIP_ROUTE_LOST
                }
                InputServiceRecoveryPhase::ResyncPrepared => {
                    if bits
                        & (BIR_NEW_READY | BIP_REBIND_OFFER | RESYNC_COMMAND_BITS | BIE_RESYNC_ACKS)
                        != (BIR_NEW_READY
                            | BIP_REBIND_OFFER
                            | RESYNC_COMMAND_BITS
                            | BIE_RESYNC_ACKS)
                        || bits & BIP_RESYNC_PREPARED != 0
                        || message.sequence() != 4
                        || message.route_epoch() != 2
                        || input_pid.get() != NEW_INPUT_PID.load(Ordering::Acquire)
                        || input_session_id != 2
                        || physical_sequence_floor != 1
                        || focus != InputServiceFocus::Launcher
                    {
                        return reject_payload();
                    }
                    BIP_RESYNC_PREPARED
                }
                InputServiceRecoveryPhase::Active => {
                    if bits
                        & (RESYNC_COMMAND_BITS
                            | BIE_RESYNC_ACKS
                            | BIE_APP_DOWN
                            | BIE_APP_UP
                            | OUTPUT_POST)
                        != (RESYNC_COMMAND_BITS
                            | BIE_RESYNC_ACKS
                            | BIE_APP_DOWN
                            | BIE_APP_UP
                            | OUTPUT_POST)
                        || bits & BIP_ACTIVE != 0
                        || message.sequence() != 5
                        || message.route_epoch() != 2
                        || input_pid.get() != NEW_INPUT_PID.load(Ordering::Acquire)
                        || input_session_id != 2
                        || physical_sequence_floor != 3
                        || focus != InputServiceFocus::App
                    {
                        return reject_payload();
                    }
                    BIP_ACTIVE
                }
            }
        }
        InputServiceRecoveryPayload::RebindOffer {
            old_input_pid,
            new_input_pid,
            new_input_session_id,
            physical_sequence_floor,
        } => {
            if write_kind != ChannelWriteKind::Transfer
                || sender_image != UserImageId::Init
                || sender_pid != INIT_PID.load(Ordering::Acquire)
                || bits & (BACKOFF_TIMED_OUT | BIR_NEW_READY | BIE_NEW_READY_PENDING)
                    != (BACKOFF_TIMED_OUT | BIR_NEW_READY | BIE_NEW_READY_PENDING)
                || bits & BIP_REBIND_OFFER != 0
                || message.sequence() != 1
                || message.surface_session() != 1
                || message.route_epoch() != 2
                || old_input_pid.get() != OLD_INPUT_PID.load(Ordering::Acquire)
                || new_input_pid.get() != NEW_INPUT_PID.load(Ordering::Acquire)
                || new_input_session_id != 2
                || physical_sequence_floor != 1
            {
                return reject_owner();
            }
            // The replacement Ready event was written before its route endpoint
            // was transferred. Publish it into the counted BIE1 resync ledger
            // only at this authenticated offer linearization point.
            BIE_RESYNC_MESSAGES.fetch_add(1, Ordering::Relaxed);
            BIP_REBIND_OFFER
        }
    };
    BIP_MESSAGES.fetch_add(1, Ordering::Relaxed);
    commit_bit(bit)
}

fn record_bic(
    sender_pid: u64,
    sender_image: UserImageId,
    command: InputCommand,
    write_kind: ChannelWriteKind,
) -> bool {
    let bits = BITS.load(Ordering::Acquire);
    if bits & BIP_REBIND_OFFER == 0 {
        return true;
    }
    if sender_image != UserImageId::SurfaceServer
        || sender_pid != SURFACE_PID.load(Ordering::Acquire)
        || write_kind != ChannelWriteKind::Bytes
        || OUTSTANDING_STREAM.load(Ordering::Acquire) != 0
    {
        return reject_owner();
    }

    let (bit, stream) = match command.payload() {
        InputCommandPayload::SnapshotReset => {
            if bits & BIP_REBIND_OFFER == 0
                || bits & (BIP_RESYNC_PREPARED | BIC_RESET) != 0
                || command.sequence() != 1
            {
                return reject_phase();
            }
            (BIC_RESET, CommandStream::Route)
        }
        InputCommandPayload::RouteUpsert(route) if command.sequence() == 2 => {
            if bits & BIC_RESET == 0 || bits & BIC_LAUNCHER_ROUTE != 0 || !valid_route(route, 1, 0)
            {
                return reject_payload();
            }
            LAUNCHER_PID.store(route.owner_pid().get(), Ordering::Release);
            (BIC_LAUNCHER_ROUTE, CommandStream::Route)
        }
        InputCommandPayload::RouteUpsert(route) if command.sequence() == 3 => {
            if bits & BIC_LAUNCHER_ROUTE == 0
                || bits & BIC_APP_ROUTE != 0
                || !valid_route(route, 2, 1)
                || route.owner_pid().get() == LAUNCHER_PID.load(Ordering::Acquire)
            {
                return reject_payload();
            }
            APP_PID.store(route.owner_pid().get(), Ordering::Release);
            (BIC_APP_ROUTE, CommandStream::Route)
        }
        InputCommandPayload::SetFocus(Some(target)) if command.sequence() == 1 => {
            if bits & (BIC_LAUNCHER_ROUTE | BIC_APP_ROUTE) != (BIC_LAUNCHER_ROUTE | BIC_APP_ROUTE)
                || bits & BIC_LAUNCHER_FOCUS != 0
                || !window_is(target, 1)
            {
                return reject_payload();
            }
            (BIC_LAUNCHER_FOCUS, CommandStream::Focus)
        }
        InputCommandPayload::SetTextContext(None) => {
            if bits & BIC_LAUNCHER_FOCUS == 0
                || bits & BIC_TEXT_NONE != 0
                || command.sequence() != 2
            {
                return reject_payload();
            }
            (BIC_TEXT_NONE, CommandStream::Focus)
        }
        InputCommandPayload::SceneCommit => {
            if bits
                & (BIC_RESET
                    | BIC_LAUNCHER_ROUTE
                    | BIC_APP_ROUTE
                    | BIC_LAUNCHER_FOCUS
                    | BIC_TEXT_NONE)
                != (BIC_RESET
                    | BIC_LAUNCHER_ROUTE
                    | BIC_APP_ROUTE
                    | BIC_LAUNCHER_FOCUS
                    | BIC_TEXT_NONE)
                || bits & BIC_SCENE_COMMIT != 0
                || command.sequence() != 4
            {
                return reject_phase();
            }
            (BIC_SCENE_COMMIT, CommandStream::Route)
        }
        InputCommandPayload::SceneBegin
        | InputCommandPayload::RouteRemove(_)
        | InputCommandPayload::SetFocus(Some(_))
        | InputCommandPayload::SetFocus(None)
        | InputCommandPayload::SetTextContext(Some(_))
        | InputCommandPayload::RouteUpsert(_) => return reject_phase(),
    };
    OUTSTANDING_STREAM.store(stream.raw().into(), Ordering::Release);
    OUTSTANDING_SEQUENCE.store(command.sequence(), Ordering::Release);
    BIC_RESYNC_MESSAGES.fetch_add(1, Ordering::Relaxed);
    commit_bit(bit)
}

fn record_bie(
    sender_pid: u64,
    sender_image: UserImageId,
    event: InputEvent,
    write_kind: ChannelWriteKind,
) -> bool {
    let bits = BITS.load(Ordering::Acquire);
    if sender_image != UserImageId::InputServer || write_kind != ChannelWriteKind::Bytes {
        return if bits & BIP_REBIND_OFFER == 0 {
            true
        } else {
            reject_owner()
        };
    }
    let new_pid = NEW_INPUT_PID.load(Ordering::Acquire);
    if bits & BIP_REBIND_OFFER == 0 {
        if bits & BIR_NEW_BOUND != 0
            && bits & BIR_NEW_READY == 0
            && sender_pid == new_pid
            && bits & BIE_NEW_READY_PENDING == 0
            && event.sequence() == 1
            && event.payload() == InputEventPayload::Ready
        {
            LAST_BIE_SEQUENCE.store(1, Ordering::Release);
            return commit_bit(BIE_NEW_READY_PENDING);
        }
        return true;
    }
    if sender_pid != new_pid || new_pid == 0 {
        return reject_owner();
    }
    let expected_event_sequence = LAST_BIE_SEQUENCE
        .load(Ordering::Acquire)
        .checked_add(1)
        .unwrap_or(0);
    if event.sequence() != expected_event_sequence {
        return reject_phase();
    }

    match event.payload() {
        InputEventPayload::Ack {
            stream,
            acknowledged_sequence,
        } => {
            if u64::from(stream.raw()) != OUTSTANDING_STREAM.load(Ordering::Acquire)
                || acknowledged_sequence != OUTSTANDING_SEQUENCE.load(Ordering::Acquire)
            {
                return reject_payload();
            }
            OUTSTANDING_STREAM.store(0, Ordering::Release);
            OUTSTANDING_SEQUENCE.store(0, Ordering::Release);
            LAST_BIE_SEQUENCE.store(event.sequence(), Ordering::Release);
            let next_acks = RESYNC_ACKS.load(Ordering::Acquire).saturating_add(1);
            if next_acks > 6 {
                return reject_phase();
            }
            RESYNC_ACKS.store(next_acks, Ordering::Release);
            BIE_RESYNC_MESSAGES.fetch_add(1, Ordering::Relaxed);
            if next_acks == 6 {
                commit_bit(BIE_RESYNC_ACKS)
            } else {
                true
            }
        }
        InputEventPayload::RoutedPointer(pointer) => {
            if bits & (BIP_RESYNC_PREPARED | RESYNC_COMMAND_BITS | BIE_RESYNC_ACKS)
                != (BIP_RESYNC_PREPARED | RESYNC_COMMAND_BITS | BIE_RESYNC_ACKS)
                || pointer.target.is_none_or(|target| !window_is(target, 2))
                || (pointer.x, pointer.y) != (136, 184)
                || !pointer.captured
                || pointer.trusted_overlay
            {
                return reject_payload();
            }
            let bit = if bits & BIE_APP_DOWN == 0 {
                if pointer.input_sequence != 2 || !pointer.pressed || event.sequence() != 8 {
                    return reject_payload();
                }
                BIE_APP_DOWN
            } else {
                if bits & BIE_APP_UP != 0
                    || pointer.input_sequence != 3
                    || pointer.pressed
                    || event.sequence() != 9
                {
                    return reject_payload();
                }
                BIE_APP_UP
            };
            LAST_BIE_SEQUENCE.store(event.sequence(), Ordering::Release);
            BIE_ROUTED_MESSAGES.fetch_add(1, Ordering::Relaxed);
            commit_bit(bit)
        }
        InputEventPayload::Ready
        | InputEventPayload::RoutedKey { .. }
        | InputEventPayload::OverlayShown { .. }
        | InputEventPayload::OverlayHidden { .. }
        | InputEventPayload::Preedit { .. }
        | InputEventPayload::Commit { .. }
        | InputEventPayload::DeleteSurrounding { .. } => reject_phase(),
    }
}

fn valid_route(route: InputRoute, window_id: u64, z_order: i32) -> bool {
    let bounds = route.bounds();
    let expected_bounds = match window_id {
        1 => (0, 0, 208, 368),
        2 => (48, 80, 112, 160),
        _ => return false,
    };
    window_is(route.target(), window_id)
        && route.owner_pid().get() != 0
        && (bounds.x(), bounds.y(), bounds.width(), bounds.height()) == expected_bounds
        && route.z_order() == z_order
        && route.is_visible()
        && route.is_focusable()
        && !route.is_trusted_overlay()
}

fn window_is(target: WindowRef, window_id: u64) -> bool {
    target.window_id().get() == window_id && target.generation() == 1
}

fn same_slot_next_generation(old_pid: u64, new_pid: u64) -> bool {
    old_pid != 0
        && new_pid != 0
        && (old_pid as u32) == (new_pid as u32)
        && (new_pid >> 32) == (old_pid >> 32).checked_add(1).unwrap_or(0)
}

fn commit_bit(bit: u64) -> bool {
    let previous = BITS.fetch_or(bit, Ordering::AcqRel);
    if previous & bit != 0 {
        return reject_phase();
    }
    true
}

fn reject_bir_owner_or_payload(actual: UserImageId, expected: UserImageId) -> bool {
    if actual != expected {
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
