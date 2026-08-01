//! Allocation-free evidence for the M46 InputServer/SurfaceServer restart leaf.
//!
//! M46 deliberately stops at the immutable M41 window checkpoint.  Recovery
//! control therefore has its own bounded transcript: BIR1 proves that the
//! physical-input owner survives one exact route epoch gap, BSR1 proves that
//! the replacement SurfaceServer and both stable clients rebind, and two
//! recovery-only output commits prove that the frozen scene advances once on
//! each Surface session without being mistaken for M42/M43/M44 traffic.

use core::sync::atomic::{AtomicU64, Ordering};

use bndr_abi::UserImageId;
use bndr_ui::{
    RecoveryCancelKind, RecoveryClientRole, SurfaceRecoveryCheckpoint, SurfaceRecoveryMessage,
    SurfaceRecoveryPayload, SurfaceRecoveryPhase,
};

const BIR_MAGIC: &[u8; 4] = b"BIR1";
const BIR_VERSION: u16 = 1;
const BIR_WIRE_SIZE: usize = 64;
const GAP_CAPACITY: u64 = 16;
const GAP_PHYSICAL_BUDGET: u64 = 64;

const BIR_ACQUIRED: u64 = 1 << 0;
const BIR_BOUND_ONE: u64 = 1 << 1;
const BIR_READY_ONE: u64 = 1 << 2;
const BSR_ARMED: u64 = 1 << 3;
const OUTPUT_OLD_CAPTURED: u64 = 1 << 4;
const BSR_RESTART_REQUESTED: u64 = 1 << 5;
const BIR_ROUTE_LOST: u64 = 1 << 6;
const BIR_GAP_QUEUED: u64 = 1 << 7;
const BIR_BOUND_TWO: u64 = 1 << 8;
const BIR_READY_TWO: u64 = 1 << 9;
const BSR_BOOTSTRAP: u64 = 1 << 10;
const BSR_LAUNCHER_OFFER: u64 = 1 << 11;
const BSR_APP_OFFER: u64 = 1 << 12;
const BSR_GRAPH_PREPARED: u64 = 1 << 13;
const BSR_LAUNCHER_ACK: u64 = 1 << 14;
const BSR_APP_ACK: u64 = 1 << 15;
const OUTPUT_NEW_ACTIVE: u64 = 1 << 16;
const BIR_GAP_DRAINED: u64 = 1 << 17;
const BSR_ACTIVE: u64 = 1 << 18;
const BSR_CLIENT_CONTACT: u64 = 1 << 19;
const BSR_CLIENT_CONTACT_ACK: u64 = 1 << 20;
const BSR_CLIENT_CONTACT_CLEARED: u64 = 1 << 21;
const COMPLETE_BITS: u64 = (1 << 22) - 1;

static BITS: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);
static OWNER_ERRORS: AtomicU64 = AtomicU64::new(0);
static PAYLOAD_ERRORS: AtomicU64 = AtomicU64::new(0);
static PHASE_ERRORS: AtomicU64 = AtomicU64::new(0);
static BIR_MESSAGES: AtomicU64 = AtomicU64::new(0);
static BSR_MESSAGES: AtomicU64 = AtomicU64::new(0);
static OUTPUT_COMMITS: AtomicU64 = AtomicU64::new(0);

static INIT_PID: AtomicU64 = AtomicU64::new(0);
static INPUT_SERVER_PID: AtomicU64 = AtomicU64::new(0);
static INPUT_SESSION: AtomicU64 = AtomicU64::new(0);
static OLD_SURFACE_PID: AtomicU64 = AtomicU64::new(0);
static NEW_SURFACE_PID: AtomicU64 = AtomicU64::new(0);
static LAUNCHER_PID: AtomicU64 = AtomicU64::new(0);
static APP_PID: AtomicU64 = AtomicU64::new(0);
static LOST_FLOOR: AtomicU64 = AtomicU64::new(0);
static LOST_WINDOW: AtomicU64 = AtomicU64::new(0);
static LOST_GENERATION: AtomicU64 = AtomicU64::new(0);
static QUEUED_ENQUEUED: AtomicU64 = AtomicU64::new(0);
static QUEUED_DEQUEUED: AtomicU64 = AtomicU64::new(0);
static DRAINED_FLOOR: AtomicU64 = AtomicU64::new(0);
static DRAINED_ENQUEUED: AtomicU64 = AtomicU64::new(0);
static DRAINED_DEQUEUED: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelWriteKind {
    Bytes,
    Transfer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputSurfaceRecoveryTraceSnapshot {
    pub bits: u64,
    pub errors: u64,
    pub owner_errors: u64,
    pub payload_errors: u64,
    pub phase_errors: u64,
    pub bir_messages: u64,
    pub bsr_messages: u64,
    pub output_commits: u64,
    pub init_pid: u64,
    pub input_server_pid: u64,
    pub input_session: u64,
    pub old_surface_pid: u64,
    pub new_surface_pid: u64,
    pub launcher_pid: u64,
    pub app_pid: u64,
    pub lost_floor: u64,
    pub lost_window: u64,
    pub lost_generation: u64,
    pub queued_enqueued: u64,
    pub queued_dequeued: u64,
    pub drained_floor: u64,
    pub drained_enqueued: u64,
    pub drained_dequeued: u64,
    pub client_contact_active: bool,
    pub client_contact_acknowledged: bool,
    pub client_contact_cleared: bool,
    pub bsr_surface_to_init: u64,
    pub bsr_init_to_surface: u64,
    pub bsr_surface_to_client: u64,
    pub bsr_client_to_surface: u64,
    pub armed: bool,
    pub gap_ready: bool,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BirMessage {
    kind: u8,
    sequence: u64,
    epoch: u64,
    session: u64,
    value_0: u64,
    value_1: u64,
    value_2: u64,
    value_3: u64,
    pending: u16,
    high_water: u16,
    coalesced: u16,
    capacity: u16,
}

/// Records a committed byte or transfer write when it belongs to M46.
///
/// Other channel protocols are ignored.  A BIR1/BSR1 candidate is accepted
/// only after its canonical decoder and the exact sender/phase contract agree.
pub fn record_channel_write(
    sender_pid: u64,
    sender_image: UserImageId,
    wire: &[u8],
    write_kind: ChannelWriteKind,
) -> bool {
    if wire.starts_with(BIR_MAGIC) {
        let Some(message) = decode_bir(wire) else {
            return reject_payload();
        };
        return record_bir(sender_pid, sender_image, message, write_kind);
    }
    if wire.starts_with(b"BSR1") {
        let Ok(message) = SurfaceRecoveryMessage::decode(wire) else {
            return reject_payload();
        };
        return record_bsr(sender_pid, sender_image, message, write_kind);
    }
    true
}

/// Accounts the two output commits intentionally outside the sealed M41
/// output ledger.
#[allow(clippy::too_many_arguments)]
pub fn record_output_commit(
    process_id: u64,
    session_id: u64,
    slot: usize,
    allocation_generation: u32,
    write_generation: u64,
    frame_id: u32,
) -> bool {
    let bits = BITS.load(Ordering::Acquire);
    let old = OLD_SURFACE_PID.load(Ordering::Acquire);
    let new = NEW_SURFACE_PID.load(Ordering::Acquire);
    let (bit, valid) = if bits & OUTPUT_OLD_CAPTURED == 0 {
        (
            OUTPUT_OLD_CAPTURED,
            bits & (BSR_ARMED | BSR_CLIENT_CONTACT | BSR_CLIENT_CONTACT_ACK)
                == (BSR_ARMED | BSR_CLIENT_CONTACT | BSR_CLIENT_CONTACT_ACK)
                && bits & BSR_RESTART_REQUESTED == 0
                && process_id == old
                && old != 0
                && session_id == 1
                && slot == 0
                && allocation_generation == 1
                && write_generation == 4
                && frame_id == 7,
        )
    } else if bits & OUTPUT_NEW_ACTIVE == 0 {
        (
            OUTPUT_NEW_ACTIVE,
            bits & (BSR_LAUNCHER_ACK | BSR_APP_ACK) == (BSR_LAUNCHER_ACK | BSR_APP_ACK)
                && bits & BIR_GAP_DRAINED == 0
                && process_id == new
                && new != 0
                && session_id == 2
                && slot == 0
                && allocation_generation == 2
                && write_generation == 1
                && frame_id == 1,
        )
    } else {
        return reject_phase();
    };
    if !valid {
        return reject_payload();
    }
    OUTPUT_COMMITS.fetch_add(1, Ordering::Relaxed);
    commit_bit(bit)
}

pub fn snapshot() -> InputSurfaceRecoveryTraceSnapshot {
    let bits = BITS.load(Ordering::Acquire);
    let errors = ERRORS.load(Ordering::Acquire);
    let bir_messages = BIR_MESSAGES.load(Ordering::Acquire);
    let bsr_messages = BSR_MESSAGES.load(Ordering::Acquire);
    let output_commits = OUTPUT_COMMITS.load(Ordering::Acquire);
    InputSurfaceRecoveryTraceSnapshot {
        bits,
        errors,
        owner_errors: OWNER_ERRORS.load(Ordering::Acquire),
        payload_errors: PAYLOAD_ERRORS.load(Ordering::Acquire),
        phase_errors: PHASE_ERRORS.load(Ordering::Acquire),
        bir_messages,
        bsr_messages,
        output_commits,
        init_pid: INIT_PID.load(Ordering::Acquire),
        input_server_pid: INPUT_SERVER_PID.load(Ordering::Acquire),
        input_session: INPUT_SESSION.load(Ordering::Acquire),
        old_surface_pid: OLD_SURFACE_PID.load(Ordering::Acquire),
        new_surface_pid: NEW_SURFACE_PID.load(Ordering::Acquire),
        launcher_pid: LAUNCHER_PID.load(Ordering::Acquire),
        app_pid: APP_PID.load(Ordering::Acquire),
        lost_floor: LOST_FLOOR.load(Ordering::Acquire),
        lost_window: LOST_WINDOW.load(Ordering::Acquire),
        lost_generation: LOST_GENERATION.load(Ordering::Acquire),
        queued_enqueued: QUEUED_ENQUEUED.load(Ordering::Acquire),
        queued_dequeued: QUEUED_DEQUEUED.load(Ordering::Acquire),
        drained_floor: DRAINED_FLOOR.load(Ordering::Acquire),
        drained_enqueued: DRAINED_ENQUEUED.load(Ordering::Acquire),
        drained_dequeued: DRAINED_DEQUEUED.load(Ordering::Acquire),
        client_contact_active: bits & BSR_CLIENT_CONTACT != 0,
        client_contact_acknowledged: bits & BSR_CLIENT_CONTACT_ACK != 0,
        client_contact_cleared: bits & BSR_CLIENT_CONTACT_CLEARED != 0,
        bsr_surface_to_init: u64::from(bits & BSR_ARMED != 0)
            + u64::from(bits & BSR_RESTART_REQUESTED != 0)
            + u64::from(bits & BSR_GRAPH_PREPARED != 0)
            + u64::from(bits & BSR_ACTIVE != 0),
        bsr_init_to_surface: u64::from(bits & BSR_BOOTSTRAP != 0),
        bsr_surface_to_client: u64::from(bits & BSR_CLIENT_CONTACT != 0)
            + u64::from(bits & BSR_LAUNCHER_OFFER != 0)
            + u64::from(bits & BSR_APP_OFFER != 0),
        bsr_client_to_surface: u64::from(bits & BSR_CLIENT_CONTACT_ACK != 0)
            + u64::from(bits & BSR_LAUNCHER_ACK != 0)
            + u64::from(bits & BSR_APP_ACK != 0),
        armed: bits & (BIR_READY_ONE | BSR_ARMED) == (BIR_READY_ONE | BSR_ARMED),
        gap_ready: bits & (OUTPUT_OLD_CAPTURED | BSR_RESTART_REQUESTED | BIR_ROUTE_LOST)
            == (OUTPUT_OLD_CAPTURED | BSR_RESTART_REQUESTED | BIR_ROUTE_LOST),
        complete: bits == COMPLETE_BITS
            && errors == 0
            && bir_messages == 8
            && bsr_messages == 11
            && output_commits == 2,
    }
}

fn record_bir(
    sender_pid: u64,
    sender_image: UserImageId,
    message: BirMessage,
    write_kind: ChannelWriteKind,
) -> bool {
    let bits = BITS.load(Ordering::Acquire);
    let input_pid = INPUT_SERVER_PID.load(Ordering::Acquire);
    let input_session = INPUT_SESSION.load(Ordering::Acquire);
    let init_pid = INIT_PID.load(Ordering::Acquire);
    let old_surface = OLD_SURFACE_PID.load(Ordering::Acquire);
    let new_surface = NEW_SURFACE_PID.load(Ordering::Acquire);
    let app_pid = APP_PID.load(Ordering::Acquire);
    let expected = match message.kind {
        1 => {
            if bits != 0
                || sender_image != UserImageId::InputServer
                || sender_pid == 0
                || write_kind != ChannelWriteKind::Bytes
                || message.sequence != 1
                || message.epoch != 0
                || message.value_0 != 0
            {
                return reject_bir_mismatch(sender_image, UserImageId::InputServer, bits != 0);
            }
            INPUT_SERVER_PID.store(sender_pid, Ordering::Release);
            INPUT_SESSION.store(message.session, Ordering::Release);
            BIR_ACQUIRED
        }
        2 if bits & BIR_BOUND_ONE == 0 => {
            if bits != BIR_ACQUIRED
                || sender_image != UserImageId::Init
                || sender_pid == 0
                || write_kind != ChannelWriteKind::Transfer
                || message.sequence != 1
                || message.epoch != 1
                || message.session != input_session
                || message.value_0 == 0
                || message.value_1 != 0
            {
                return reject_bir_mismatch(sender_image, UserImageId::Init, bits != BIR_ACQUIRED);
            }
            INIT_PID.store(sender_pid, Ordering::Release);
            OLD_SURFACE_PID.store(message.value_0, Ordering::Release);
            BIR_BOUND_ONE
        }
        3 if bits & BIR_READY_ONE == 0 => {
            if bits & (BIR_ACQUIRED | BIR_BOUND_ONE) != (BIR_ACQUIRED | BIR_BOUND_ONE)
                || sender_image != UserImageId::InputServer
                || sender_pid != input_pid
                || write_kind != ChannelWriteKind::Bytes
                || message.sequence != 2
                || message.epoch != 1
                || message.session != input_session
                || message.value_0 != old_surface
                || message.value_1 != 0
            {
                return reject_bir_mismatch(sender_image, UserImageId::InputServer, true);
            }
            BIR_READY_ONE
        }
        5 => {
            if bits & BSR_RESTART_REQUESTED == 0
                || bits & BIR_ROUTE_LOST != 0
                || bits & BSR_CLIENT_CONTACT_ACK == 0
                || sender_image != UserImageId::InputServer
                || sender_pid != input_pid
                || write_kind != ChannelWriteKind::Bytes
                || message.sequence != 3
                || message.epoch != 1
                || message.session != input_session
                || message.value_0 != 2
                || app_pid == 0
                || message.value_1 != app_pid
                || message.value_2 != 2
                || message.value_3 != 1
            {
                return reject_bir_mismatch(sender_image, UserImageId::InputServer, true);
            }
            LOST_FLOOR.store(message.value_0, Ordering::Release);
            LOST_WINDOW.store(message.value_2, Ordering::Release);
            LOST_GENERATION.store(message.value_3, Ordering::Release);
            BIR_ROUTE_LOST
        }
        7 if bits & BIR_GAP_QUEUED == 0 => {
            if bits & BIR_ROUTE_LOST == 0
                || sender_image != UserImageId::InputServer
                || sender_pid != input_pid
                || write_kind != ChannelWriteKind::Bytes
                || message.sequence != 4
                || message.epoch != 2
                || message.session != input_session
                || message.value_0 != 2
                || message.value_1 != 1
                || message.value_2 != 0
                || message.pending != 1
                || message.high_water != 1
                || message.coalesced != 0
            {
                return reject_bir_mismatch(sender_image, UserImageId::InputServer, true);
            }
            QUEUED_ENQUEUED.store(message.value_1, Ordering::Release);
            QUEUED_DEQUEUED.store(message.value_2, Ordering::Release);
            BIR_GAP_QUEUED
        }
        2 => {
            if bits & BIR_GAP_QUEUED == 0
                || bits & BIR_BOUND_TWO != 0
                || sender_image != UserImageId::Init
                || sender_pid != init_pid
                || write_kind != ChannelWriteKind::Transfer
                || message.sequence != 2
                || message.epoch != 2
                || message.session != input_session
                || message.value_0 == 0
                || message.value_0 == old_surface
                || message.value_1 != 2
            {
                return reject_bir_mismatch(sender_image, UserImageId::Init, true);
            }
            NEW_SURFACE_PID.store(message.value_0, Ordering::Release);
            BIR_BOUND_TWO
        }
        3 => {
            if bits & BIR_BOUND_TWO == 0
                || bits & BIR_READY_TWO != 0
                || sender_image != UserImageId::InputServer
                || sender_pid != input_pid
                || write_kind != ChannelWriteKind::Bytes
                || message.sequence != 5
                || message.epoch != 2
                || message.session != input_session
                || message.value_0 != new_surface
                || message.value_1 != 2
            {
                return reject_bir_mismatch(sender_image, UserImageId::InputServer, true);
            }
            BIR_READY_TWO
        }
        7 => {
            if bits & (OUTPUT_NEW_ACTIVE | BSR_LAUNCHER_ACK | BSR_APP_ACK)
                != (OUTPUT_NEW_ACTIVE | BSR_LAUNCHER_ACK | BSR_APP_ACK)
                || bits & BIR_GAP_DRAINED != 0
                || sender_image != UserImageId::InputServer
                || sender_pid != input_pid
                || write_kind != ChannelWriteKind::Bytes
                || message.sequence != 6
                || message.epoch != 2
                || message.session != input_session
                || message.value_0 != 3
                || message.value_1 != 1
                || message.value_2 != 1
                || message.pending != 0
                || message.high_water != 1
                || message.coalesced != 0
            {
                return reject_bir_mismatch(sender_image, UserImageId::InputServer, true);
            }
            DRAINED_FLOOR.store(message.value_0, Ordering::Release);
            DRAINED_ENQUEUED.store(message.value_1, Ordering::Release);
            DRAINED_DEQUEUED.store(message.value_2, Ordering::Release);
            BIR_GAP_DRAINED
        }
        _ => return reject_phase(),
    };
    BIR_MESSAGES.fetch_add(1, Ordering::Relaxed);
    commit_bit(expected)
}

fn record_bsr(
    sender_pid: u64,
    sender_image: UserImageId,
    message: SurfaceRecoveryMessage,
    write_kind: ChannelWriteKind,
) -> bool {
    if write_kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let bits = BITS.load(Ordering::Acquire);
    let old_surface = OLD_SURFACE_PID.load(Ordering::Acquire);
    let new_surface = NEW_SURFACE_PID.load(Ordering::Acquire);
    let init_pid = INIT_PID.load(Ordering::Acquire);
    let input_pid = INPUT_SERVER_PID.load(Ordering::Acquire);
    let input_session = INPUT_SESSION.load(Ordering::Acquire);
    let bit = match message.payload() {
        SurfaceRecoveryPayload::SurfaceStatus {
            checkpoint,
            related_physical_sequence,
            phase: SurfaceRecoveryPhase::Armed,
        } => {
            if bits & BIR_READY_ONE == 0
                || bits & BSR_ARMED != 0
                || sender_image != UserImageId::SurfaceServer
                || sender_pid != old_surface
                || message.sender_sequence() != 1
                || message.surface_session() != 1
                || message.route_epoch() != 1
                || checkpoint != SurfaceRecoveryCheckpoint::MultiWindowInteractive
                || related_physical_sequence != 0
            {
                return reject_bsr_mismatch(sender_image, UserImageId::SurfaceServer, true);
            }
            BSR_ARMED
        }
        SurfaceRecoveryPayload::ClientContact {
            role,
            cancel_kind,
            checkpoint,
            old_window,
            physical_sequence,
        } => {
            if bits & (BIR_READY_ONE | BSR_ARMED) != (BIR_READY_ONE | BSR_ARMED)
                || bits
                    & (BSR_CLIENT_CONTACT
                        | BSR_CLIENT_CONTACT_ACK
                        | OUTPUT_OLD_CAPTURED
                        | BSR_RESTART_REQUESTED)
                    != 0
                || sender_image != UserImageId::SurfaceServer
                || sender_pid != old_surface
                || message.sender_sequence() != 1
                || message.surface_session() != 1
                || message.route_epoch() != 1
                || role != RecoveryClientRole::App
                || cancel_kind != RecoveryCancelKind::ClientPointer
                || checkpoint != SurfaceRecoveryCheckpoint::MultiWindowInteractive
                || old_window.token() != (1_u64 << 32) | 1
                || physical_sequence != 2
            {
                return reject_bsr_mismatch(sender_image, UserImageId::SurfaceServer, true);
            }
            BSR_CLIENT_CONTACT
        }
        SurfaceRecoveryPayload::ClientContactAck {
            role,
            cancel_kind,
            checkpoint,
            old_window,
            acknowledged_contact_sequence,
            physical_sequence,
        } => {
            if bits & (BIR_READY_ONE | BSR_ARMED | BSR_CLIENT_CONTACT)
                != (BIR_READY_ONE | BSR_ARMED | BSR_CLIENT_CONTACT)
                || bits & (BSR_CLIENT_CONTACT_ACK | OUTPUT_OLD_CAPTURED | BSR_RESTART_REQUESTED)
                    != 0
                || sender_image != UserImageId::App
                || sender_pid == 0
                || APP_PID.load(Ordering::Acquire) != 0
                || message.sender_sequence() != 1
                || message.surface_session() != 1
                || message.route_epoch() != 1
                || role != RecoveryClientRole::App
                || cancel_kind != RecoveryCancelKind::ClientPointer
                || checkpoint != SurfaceRecoveryCheckpoint::MultiWindowInteractive
                || old_window.token() != (1_u64 << 32) | 1
                || acknowledged_contact_sequence != 1
                || physical_sequence != 2
            {
                return reject_bsr_mismatch(sender_image, UserImageId::App, true);
            }
            APP_PID.store(sender_pid, Ordering::Release);
            BSR_CLIENT_CONTACT_ACK
        }
        SurfaceRecoveryPayload::SurfaceStatus {
            checkpoint,
            related_physical_sequence,
            phase: SurfaceRecoveryPhase::RestartRequested,
        } => {
            if bits & (BSR_ARMED | OUTPUT_OLD_CAPTURED) != (BSR_ARMED | OUTPUT_OLD_CAPTURED)
                || bits & BSR_RESTART_REQUESTED != 0
                || sender_image != UserImageId::SurfaceServer
                || sender_pid != old_surface
                || message.sender_sequence() != 2
                || message.surface_session() != 1
                || message.route_epoch() != 1
                || checkpoint != SurfaceRecoveryCheckpoint::MultiWindowInteractive
                || related_physical_sequence != 2
            {
                return reject_bsr_mismatch(sender_image, UserImageId::SurfaceServer, true);
            }
            BSR_RESTART_REQUESTED
        }
        SurfaceRecoveryPayload::Bootstrap {
            input_server_pid,
            input_session_id,
            physical_floor,
            checkpoint,
            expected_gap_events,
            cancel_kind,
        } => {
            if bits & BIR_READY_TWO == 0
                || bits & BSR_BOOTSTRAP != 0
                || sender_image != UserImageId::Init
                || sender_pid != init_pid
                || message.sender_sequence() != 1
                || message.surface_session() != 2
                || message.route_epoch() != 2
                || input_server_pid != input_pid
                || input_session_id != input_session
                || physical_floor != 2
                || checkpoint != SurfaceRecoveryCheckpoint::MultiWindowInteractive
                || expected_gap_events != 1
                || cancel_kind != RecoveryCancelKind::ClientPointer
            {
                return reject_bsr_mismatch(sender_image, UserImageId::Init, true);
            }
            BSR_BOOTSTRAP
        }
        SurfaceRecoveryPayload::ClientRebindOffer {
            role,
            cancel_kind,
            checkpoint,
            old_window,
            physical_floor,
            cancel_after,
        } => {
            let (bit, expected_token, expected_cancel, expected_after) = match role {
                RecoveryClientRole::Launcher => {
                    (BSR_LAUNCHER_OFFER, 1_u64 << 32, RecoveryCancelKind::None, 0)
                }
                RecoveryClientRole::App => (
                    BSR_APP_OFFER,
                    (1_u64 << 32) | 1,
                    RecoveryCancelKind::ClientPointer,
                    2,
                ),
            };
            let role_phase_valid = match role {
                RecoveryClientRole::Launcher => {
                    bits & BSR_BOOTSTRAP != 0 && bits & (BSR_LAUNCHER_OFFER | BSR_APP_OFFER) == 0
                }
                RecoveryClientRole::App => {
                    bits & BSR_LAUNCHER_OFFER != 0 && bits & BSR_APP_OFFER == 0
                }
            };
            if !role_phase_valid
                || bits & bit != 0
                || sender_image != UserImageId::SurfaceServer
                || sender_pid != new_surface
                || message.sender_sequence() != 1
                || message.surface_session() != 2
                || message.route_epoch() != 2
                || checkpoint != SurfaceRecoveryCheckpoint::MultiWindowInteractive
                || old_window.token() != expected_token
                || physical_floor != 2
                || cancel_kind != expected_cancel
                || cancel_after != expected_after
            {
                return reject_bsr_mismatch(sender_image, UserImageId::SurfaceServer, true);
            }
            bit
        }
        SurfaceRecoveryPayload::SurfaceStatus {
            checkpoint,
            related_physical_sequence,
            phase: SurfaceRecoveryPhase::GraphPrepared,
        } => {
            if bits & (BSR_LAUNCHER_OFFER | BSR_APP_OFFER) != (BSR_LAUNCHER_OFFER | BSR_APP_OFFER)
                || bits & BSR_GRAPH_PREPARED != 0
                || sender_image != UserImageId::SurfaceServer
                || sender_pid != new_surface
                || message.sender_sequence() != 1
                || message.surface_session() != 2
                || message.route_epoch() != 2
                || checkpoint != SurfaceRecoveryCheckpoint::MultiWindowInteractive
                || related_physical_sequence != 2
            {
                return reject_bsr_mismatch(sender_image, UserImageId::SurfaceServer, true);
            }
            BSR_GRAPH_PREPARED
        }
        SurfaceRecoveryPayload::ClientRebindAck {
            role,
            cancel_kind,
            checkpoint,
            replacement_window,
            acknowledged_offer_sequence,
            cancel_after,
        } => {
            let (bit, expected_image, expected_token, expected_cancel, expected_after) = match role
            {
                RecoveryClientRole::Launcher => (
                    BSR_LAUNCHER_ACK,
                    UserImageId::Launcher,
                    1_u64 << 32,
                    RecoveryCancelKind::None,
                    0,
                ),
                RecoveryClientRole::App => (
                    BSR_APP_ACK,
                    UserImageId::App,
                    (1_u64 << 32) | 1,
                    RecoveryCancelKind::ClientPointer,
                    2,
                ),
            };
            let expected_sender_pid = match role {
                RecoveryClientRole::Launcher => 0,
                RecoveryClientRole::App => APP_PID.load(Ordering::Acquire),
            };
            let role_phase_valid = client_rebind_ack_phase_valid(bits, role);
            if !role_phase_valid
                || bits & bit != 0
                || sender_image != expected_image
                || sender_pid == 0
                || (expected_sender_pid != 0 && sender_pid != expected_sender_pid)
                || message.sender_sequence() != 1
                || message.surface_session() != 2
                || message.route_epoch() != 2
                || checkpoint != SurfaceRecoveryCheckpoint::MultiWindowInteractive
                || replacement_window.token() != expected_token
                || acknowledged_offer_sequence != 1
                || cancel_kind != expected_cancel
                || cancel_after != expected_after
            {
                return reject_bsr_mismatch(sender_image, expected_image, true);
            }
            match role {
                RecoveryClientRole::Launcher => LAUNCHER_PID.store(sender_pid, Ordering::Release),
                RecoveryClientRole::App => APP_PID.store(sender_pid, Ordering::Release),
            }
            if role == RecoveryClientRole::App {
                bit | BSR_CLIENT_CONTACT_CLEARED
            } else {
                bit
            }
        }
        SurfaceRecoveryPayload::SurfaceStatus {
            checkpoint,
            related_physical_sequence,
            phase: SurfaceRecoveryPhase::Active,
        } => {
            if bits & (BIR_GAP_DRAINED | OUTPUT_NEW_ACTIVE | BSR_CLIENT_CONTACT_CLEARED)
                != (BIR_GAP_DRAINED | OUTPUT_NEW_ACTIVE | BSR_CLIENT_CONTACT_CLEARED)
                || bits & BSR_ACTIVE != 0
                || sender_image != UserImageId::SurfaceServer
                || sender_pid != new_surface
                || message.sender_sequence() != 2
                || message.surface_session() != 2
                || message.route_epoch() != 2
                || checkpoint != SurfaceRecoveryCheckpoint::MultiWindowInteractive
                || related_physical_sequence != 3
            {
                return reject_bsr_mismatch(sender_image, UserImageId::SurfaceServer, true);
            }
            BSR_ACTIVE
        }
    };
    BSR_MESSAGES.fetch_add(1, Ordering::Relaxed);
    commit_bit(bit)
}

fn client_rebind_ack_phase_valid(bits: u64, role: RecoveryClientRole) -> bool {
    if bits & BSR_GRAPH_PREPARED == 0 {
        return false;
    }
    match role {
        RecoveryClientRole::Launcher => bits & BSR_LAUNCHER_ACK == 0,
        RecoveryClientRole::App => bits & (BSR_APP_ACK | BSR_CLIENT_CONTACT_CLEARED) == 0,
    }
}

fn decode_bir(wire: &[u8]) -> Option<BirMessage> {
    if wire.len() != BIR_WIRE_SIZE
        || wire.get(..4)? != BIR_MAGIC
        || read_u16(wire, 4)? != BIR_VERSION
        || wire[7] != 0
    {
        return None;
    }
    let kind = wire[6];
    let sequence = read_u64(wire, 8)?;
    let epoch = read_u64(wire, 16)?;
    let session = read_u64(wire, 24)?;
    let value_0 = read_u64(wire, 32)?;
    let value_1 = read_u64(wire, 40)?;
    let value_2 = read_u64(wire, 48)?;
    let value_3 = read_u64(wire, 56)?;
    if sequence == 0
        || session == 0
        || !(1..=7).contains(&kind)
        || (kind == 1 && epoch != 0)
        || (kind != 1 && epoch == 0)
    {
        return None;
    }
    let pending = read_u16(wire, 56)?;
    let high_water = read_u16(wire, 58)?;
    let coalesced = read_u16(wire, 60)?;
    let capacity = read_u16(wire, 62)?;
    let canonical = match kind {
        1 => value_1 == 0 && value_2 == 0 && value_3 == 0,
        2 | 3 => value_0 != 0 && value_2 == GAP_CAPACITY && value_3 == GAP_PHYSICAL_BUDGET,
        4 => value_1 == 0 && value_2 == 0 && value_3 == 0,
        5 | 6 => value_1 != 0 && value_2 != 0 && value_3 != 0,
        7 => {
            let stored = value_1.saturating_sub(u64::from(coalesced));
            u64::from(capacity) == GAP_CAPACITY
                && value_1 <= GAP_PHYSICAL_BUDGET
                && value_2 <= value_1
                && pending <= high_water
                && u64::from(high_water) <= GAP_CAPACITY
                && u64::from(coalesced) <= value_1
                && u64::from(high_water) <= stored
                && u64::from(pending) <= value_1 - value_2
                && ((value_1 == value_2 && pending == 0) || (value_1 > value_2 && pending != 0))
                && ((value_1 == 0 && high_water == 0 && coalesced == 0)
                    || (value_1 != 0 && high_water != 0))
        }
        _ => false,
    };
    canonical.then_some(BirMessage {
        kind,
        sequence,
        epoch,
        session,
        value_0,
        value_1,
        value_2,
        value_3,
        pending,
        high_water,
        coalesced,
        capacity,
    })
}

fn read_u16(wire: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        wire.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u64(wire: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        wire.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

fn commit_bit(bit: u64) -> bool {
    let previous = BITS.fetch_or(bit, Ordering::AcqRel);
    if previous & bit != 0 {
        return reject_phase();
    }
    true
}

fn reject_bir_mismatch(sender: UserImageId, expected: UserImageId, phase: bool) -> bool {
    if sender != expected {
        reject_owner()
    } else if phase {
        reject_phase()
    } else {
        reject_payload()
    }
}

fn reject_bsr_mismatch(sender: UserImageId, expected: UserImageId, phase: bool) -> bool {
    if sender != expected {
        reject_owner()
    } else if phase {
        reject_phase()
    } else {
        reject_payload()
    }
}

fn reject_owner() -> bool {
    OWNER_ERRORS.fetch_add(1, Ordering::Relaxed);
    ERRORS.fetch_add(1, Ordering::Release);
    false
}

fn reject_payload() -> bool {
    PAYLOAD_ERRORS.fetch_add(1, Ordering::Relaxed);
    ERRORS.fetch_add(1, Ordering::Release);
    false
}

fn reject_phase() -> bool {
    PHASE_ERRORS.fetch_add(1, Ordering::Relaxed);
    ERRORS.fetch_add(1, Ordering::Release);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_rebind_acks_accept_both_authenticated_serializations() {
        let prepared = BSR_GRAPH_PREPARED;
        assert!(client_rebind_ack_phase_valid(
            prepared,
            RecoveryClientRole::Launcher
        ));
        assert!(client_rebind_ack_phase_valid(
            prepared,
            RecoveryClientRole::App
        ));
        assert!(client_rebind_ack_phase_valid(
            prepared | BSR_LAUNCHER_ACK,
            RecoveryClientRole::App
        ));
        assert!(client_rebind_ack_phase_valid(
            prepared | BSR_APP_ACK | BSR_CLIENT_CONTACT_CLEARED,
            RecoveryClientRole::Launcher
        ));

        assert!(!client_rebind_ack_phase_valid(
            0,
            RecoveryClientRole::Launcher
        ));
        assert!(!client_rebind_ack_phase_valid(
            prepared | BSR_LAUNCHER_ACK,
            RecoveryClientRole::Launcher
        ));
        assert!(!client_rebind_ack_phase_valid(
            prepared | BSR_APP_ACK | BSR_CLIENT_CONTACT_CLEARED,
            RecoveryClientRole::App
        ));
    }
}
