//! Allocation-free M53 post-recovery lifecycle-focus convergence evidence.
//!
//! The M49--M52 traces remain immutable owners of their existing recovery,
//! input, window, and output transcripts.  This independent witness fans out
//! from the same committed channel operations once the replacement Surface
//! and both stable UI clients have generation-qualified PIDs.  It authenticates
//! one fresh legacy-UI session, three contiguous lifecycle-focus generations,
//! both clients' private acknowledgements for every generation, and the old
//! M52 App completion ACK that closes the transcript.  Unrelated protocols are
//! deliberately ignored so the older leaf traces keep their exact counters.

use core::sync::atomic::{AtomicU64, Ordering};

use bndr_abi::UserImageId;
use bndr_ui::{ShellAppId, UiClientId, UiServerEvent, UiServerEventPayload};

use crate::service_dependency_trace::ChannelWriteKind;

pub const REBIND_FOCUS_ACK_MAGIC: u64 = 0x4d35_335f_5246_434b;
pub const LAUNCHER_FOCUS_ACK_MAGIC: u64 = 0x4d35_335f_4c46_434b;
pub const APP_FOCUS_ACK_MAGIC: u64 = 0x4d35_335f_4146_434b;
pub const M52_APP_COMPLETION_ACK_MAGIC: u64 = 0x4d35_325f_4150_434b;

const M53_MAGIC_NAMESPACE: u64 = 0x4d35_335f;

const REBIND_READY_LAUNCHER_WRITE: u64 = 1 << 0;
const REBIND_READY_LAUNCHER_READ: u64 = 1 << 1;
const REBIND_READY_APP_WRITE: u64 = 1 << 2;
const REBIND_READY_APP_READ: u64 = 1 << 3;
const REBIND_FOCUS_LAUNCHER_WRITE: u64 = 1 << 4;
const REBIND_FOCUS_LAUNCHER_READ: u64 = 1 << 5;
const REBIND_FOCUS_APP_WRITE: u64 = 1 << 6;
const REBIND_FOCUS_APP_READ: u64 = 1 << 7;
const REBIND_ACK_LAUNCHER_WRITE: u64 = 1 << 8;
const REBIND_ACK_LAUNCHER_READ: u64 = 1 << 9;
const REBIND_ACK_APP_WRITE: u64 = 1 << 10;
const REBIND_ACK_APP_READ: u64 = 1 << 11;

const LAUNCHER_FOCUS_LAUNCHER_WRITE: u64 = 1 << 12;
const LAUNCHER_FOCUS_LAUNCHER_READ: u64 = 1 << 13;
const LAUNCHER_FOCUS_APP_WRITE: u64 = 1 << 14;
const LAUNCHER_FOCUS_APP_READ: u64 = 1 << 15;
const LAUNCHER_ACK_LAUNCHER_WRITE: u64 = 1 << 16;
const LAUNCHER_ACK_LAUNCHER_READ: u64 = 1 << 17;
const LAUNCHER_ACK_APP_WRITE: u64 = 1 << 18;
const LAUNCHER_ACK_APP_READ: u64 = 1 << 19;

const APP_FOCUS_LAUNCHER_WRITE: u64 = 1 << 20;
const APP_FOCUS_LAUNCHER_READ: u64 = 1 << 21;
const APP_FOCUS_APP_WRITE: u64 = 1 << 22;
const APP_FOCUS_APP_READ: u64 = 1 << 23;
const APP_ACK_LAUNCHER_WRITE: u64 = 1 << 24;
const APP_ACK_LAUNCHER_READ: u64 = 1 << 25;
const APP_ACK_APP_WRITE: u64 = 1 << 26;
const APP_ACK_APP_READ: u64 = 1 << 27;

const M52_APP_COMPLETION_ACK_WRITE: u64 = 1 << 28;
const M52_APP_COMPLETION_ACK_READ: u64 = 1 << 29;

const REBIND_STAGE_BITS: u64 = (1 << 12) - 1;
const LAUNCHER_STAGE_BITS: u64 = (1 << 20) - 1;
const LIFECYCLE_BITS: u64 = (1 << 28) - 1;
const REQUIRED_BITS: u64 = (1 << 30) - 1;

static BITS: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);
static DECODE_ERRORS: AtomicU64 = AtomicU64::new(0);
static OWNER_ERRORS: AtomicU64 = AtomicU64::new(0);
static PHASE_ERRORS: AtomicU64 = AtomicU64::new(0);
static PAYLOAD_ERRORS: AtomicU64 = AtomicU64::new(0);
static CHANNEL_WRITES: AtomicU64 = AtomicU64::new(0);
static CHANNEL_READS: AtomicU64 = AtomicU64::new(0);
static BUE_WRITES: AtomicU64 = AtomicU64::new(0);
static BUE_READS: AtomicU64 = AtomicU64::new(0);
static ACK_WRITES: AtomicU64 = AtomicU64::new(0);
static ACK_READS: AtomicU64 = AtomicU64::new(0);
static BOUNDARY_WRITES: AtomicU64 = AtomicU64::new(0);
static BOUNDARY_READS: AtomicU64 = AtomicU64::new(0);
static OUTPUT_COMMITS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PostRecoveryLifecycleFocusTraceSnapshot {
    pub bits: u64,
    pub errors: u64,
    pub decode_errors: u64,
    pub owner_errors: u64,
    pub phase_errors: u64,
    pub payload_errors: u64,
    pub channel_writes: u64,
    pub channel_reads: u64,
    pub bue_writes: u64,
    pub bue_reads: u64,
    pub ack_writes: u64,
    pub ack_reads: u64,
    pub boundary_writes: u64,
    pub boundary_reads: u64,
    pub output_commits: u64,
    pub rebound_complete: bool,
    pub launcher_complete: bool,
    pub app_complete: bool,
    pub boundary_complete: bool,
    pub complete: bool,
}

pub fn snapshot() -> PostRecoveryLifecycleFocusTraceSnapshot {
    let bits = BITS.load(Ordering::Acquire);
    let errors = ERRORS.load(Ordering::Acquire);
    let channel_writes = CHANNEL_WRITES.load(Ordering::Acquire);
    let channel_reads = CHANNEL_READS.load(Ordering::Acquire);
    let bue_writes = BUE_WRITES.load(Ordering::Acquire);
    let bue_reads = BUE_READS.load(Ordering::Acquire);
    let ack_writes = ACK_WRITES.load(Ordering::Acquire);
    let ack_reads = ACK_READS.load(Ordering::Acquire);
    let boundary_writes = BOUNDARY_WRITES.load(Ordering::Acquire);
    let boundary_reads = BOUNDARY_READS.load(Ordering::Acquire);
    let output_commits = OUTPUT_COMMITS.load(Ordering::Acquire);
    PostRecoveryLifecycleFocusTraceSnapshot {
        bits,
        errors,
        decode_errors: DECODE_ERRORS.load(Ordering::Acquire),
        owner_errors: OWNER_ERRORS.load(Ordering::Acquire),
        phase_errors: PHASE_ERRORS.load(Ordering::Acquire),
        payload_errors: PAYLOAD_ERRORS.load(Ordering::Acquire),
        channel_writes,
        channel_reads,
        bue_writes,
        bue_reads,
        ack_writes,
        ack_reads,
        boundary_writes,
        boundary_reads,
        output_commits,
        rebound_complete: has_all(bits, REBIND_STAGE_BITS),
        launcher_complete: has_all(bits, LAUNCHER_STAGE_BITS),
        app_complete: has_all(bits, LIFECYCLE_BITS),
        boundary_complete: has_all(
            bits,
            M52_APP_COMPLETION_ACK_WRITE | M52_APP_COMPLETION_ACK_READ,
        ),
        complete: bits == REQUIRED_BITS
            && errors == 0
            && channel_writes == 14
            && channel_reads == 14
            && bue_writes == 8
            && bue_reads == 8
            && ack_writes == 6
            && ack_reads == 6
            && boundary_writes == 1
            && boundary_reads == 1
            && output_commits == 0,
    }
}

#[derive(Clone, Copy)]
struct TraceOwners {
    surface_pid: u64,
    launcher_pid: u64,
    app_pid: u64,
}

fn active_owners() -> Option<TraceOwners> {
    let recovery = crate::input_surface_recovery_trace::snapshot();
    let owners = TraceOwners {
        surface_pid: recovery.new_surface_pid,
        launcher_pid: recovery.launcher_pid,
        app_pid: recovery.app_pid,
    };
    (owners.surface_pid != 0
        && owners.launcher_pid != 0
        && owners.app_pid != 0
        && owners.surface_pid != owners.launcher_pid
        && owners.surface_pid != owners.app_pid
        && owners.launcher_pid != owners.app_pid)
        .then_some(owners)
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
    let m52_complete = crate::post_recovery_focus_roundtrip_trace::snapshot().complete;
    record_channel_write_with(owners, sender_pid, sender_image, wire, kind, m52_complete)
}

fn record_channel_write_with(
    owners: TraceOwners,
    sender_pid: u64,
    sender_image: UserImageId,
    wire: &[u8],
    kind: ChannelWriteKind,
    m52_complete: bool,
) -> bool {
    if snapshot().complete {
        return true;
    }
    if wire.starts_with(b"BUE1") {
        let Ok(event) = UiServerEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bue_write(owners, sender_pid, sender_image, event, kind);
    }
    let Some(tag) = decode_tag(wire) else {
        return true;
    };
    if let Some(phase) = AckPhase::from_tag(tag) {
        return record_ack_write(owners, sender_pid, sender_image, phase, kind);
    }
    if tag == M52_APP_COMPLETION_ACK_MAGIC {
        return record_boundary_write(owners, sender_pid, sender_image, kind, m52_complete);
    }
    if tag >> 32 == M53_MAGIC_NAMESPACE {
        return reject_payload();
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
    if wire.starts_with(b"BUE1") {
        let Ok(event) = UiServerEvent::decode(wire) else {
            return reject_decode();
        };
        return record_bue_read(owners, reader_pid, reader_image, sender_pid, event, kind);
    }
    let Some(tag) = decode_tag(wire) else {
        return true;
    };
    if let Some(phase) = AckPhase::from_tag(tag) {
        return record_ack_read(owners, reader_pid, reader_image, sender_pid, phase, kind);
    }
    if tag == M52_APP_COMPLETION_ACK_MAGIC {
        return record_boundary_read(owners, reader_pid, reader_image, sender_pid, kind);
    }
    if tag >> 32 == M53_MAGIC_NAMESPACE {
        return reject_payload();
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
    if frame_id > 6 {
        OUTPUT_COMMITS.fetch_add(1, Ordering::Relaxed);
        return reject_payload();
    }
    if process_id != owners.surface_pid {
        return reject_owner();
    }
    if frame_id == 0
        || session_id != 2
        || slot != 0
        || allocation_generation != 2
        || write_generation != u64::from(frame_id)
    {
        return reject_payload();
    }
    true
}

fn record_bue_write(
    owners: TraceOwners,
    sender_pid: u64,
    sender_image: UserImageId,
    event: UiServerEvent,
    kind: ChannelWriteKind,
) -> bool {
    if sender_image != UserImageId::SurfaceServer || sender_pid != owners.surface_pid {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let index = BUE_WRITES.load(Ordering::Acquire) as usize;
    let Some(spec) = bue_write_spec(index) else {
        return reject_phase();
    };
    if !valid_bue(event, spec.payload) {
        return reject_payload();
    }
    if spec.prerequisite != 0 && !has_all(BITS.load(Ordering::Acquire), spec.prerequisite) {
        return reject_phase();
    }
    if !commit_bit(spec.bit) {
        return false;
    }
    BUE_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_bue_read(
    owners: TraceOwners,
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    event: UiServerEvent,
    kind: ChannelWriteKind,
) -> bool {
    if sender_pid != owners.surface_pid {
        return reject_owner();
    }
    let Some(client) = Client::from_owner(owners, reader_pid, reader_image) else {
        return reject_owner();
    };
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let Some(spec) = bue_read_spec(event, client) else {
        return reject_payload();
    };
    if !has_all(
        BITS.load(Ordering::Acquire),
        spec.prerequisite | spec.write_bit,
    ) {
        return reject_phase();
    }
    if !commit_bit(spec.read_bit) {
        return false;
    }
    BUE_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_ack_write(
    owners: TraceOwners,
    sender_pid: u64,
    sender_image: UserImageId,
    phase: AckPhase,
    kind: ChannelWriteKind,
) -> bool {
    let Some(client) = Client::from_owner(owners, sender_pid, sender_image) else {
        return reject_owner();
    };
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let spec = ack_spec(phase, client);
    if !has_all(BITS.load(Ordering::Acquire), spec.focus_read_bit) {
        return reject_phase();
    }
    if !commit_bit(spec.write_bit) {
        return false;
    }
    ACK_WRITES.fetch_add(1, Ordering::Relaxed);
    CHANNEL_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_ack_read(
    owners: TraceOwners,
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    phase: AckPhase,
    kind: ChannelWriteKind,
) -> bool {
    if reader_image != UserImageId::SurfaceServer || reader_pid != owners.surface_pid {
        return reject_owner();
    }
    let Some(client) = Client::from_sender(owners, sender_pid) else {
        return reject_owner();
    };
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    let spec = ack_spec(phase, client);
    if !has_all(BITS.load(Ordering::Acquire), spec.write_bit) {
        return reject_phase();
    }
    if !commit_bit(spec.read_bit) {
        return false;
    }
    ACK_READS.fetch_add(1, Ordering::Relaxed);
    CHANNEL_READS.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_boundary_write(
    owners: TraceOwners,
    sender_pid: u64,
    sender_image: UserImageId,
    kind: ChannelWriteKind,
    m52_complete: bool,
) -> bool {
    if sender_image != UserImageId::App || sender_pid != owners.app_pid {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    if !m52_complete || !has_all(BITS.load(Ordering::Acquire), LIFECYCLE_BITS) {
        return reject_phase();
    }
    if !commit_bit(M52_APP_COMPLETION_ACK_WRITE) {
        return false;
    }
    BOUNDARY_WRITES.fetch_add(1, Ordering::Relaxed);
    true
}

fn record_boundary_read(
    owners: TraceOwners,
    reader_pid: u64,
    reader_image: UserImageId,
    sender_pid: u64,
    kind: ChannelWriteKind,
) -> bool {
    if reader_image != UserImageId::SurfaceServer
        || reader_pid != owners.surface_pid
        || sender_pid != owners.app_pid
    {
        return reject_owner();
    }
    if kind != ChannelWriteKind::Bytes {
        return reject_payload();
    }
    if !has_all(BITS.load(Ordering::Acquire), M52_APP_COMPLETION_ACK_WRITE) {
        return reject_phase();
    }
    if !commit_bit(M52_APP_COMPLETION_ACK_READ) {
        return false;
    }
    BOUNDARY_READS.fetch_add(1, Ordering::Relaxed);
    true
}

#[derive(Clone, Copy)]
enum ExpectedBue {
    Ready,
    Focus {
        client: UiClientId,
        app: Option<ShellAppId>,
        generation: u64,
    },
}

#[derive(Clone, Copy)]
struct BueWriteSpec {
    bit: u64,
    prerequisite: u64,
    payload: ExpectedBue,
}

fn bue_write_spec(index: usize) -> Option<BueWriteSpec> {
    let ready = ExpectedBue::Ready;
    let rebound = ExpectedBue::Focus {
        client: UiClientId::App,
        app: Some(ShellAppId::Phone),
        generation: 1,
    };
    let launcher = ExpectedBue::Focus {
        client: UiClientId::Launcher,
        app: None,
        generation: 2,
    };
    let app = ExpectedBue::Focus {
        client: UiClientId::App,
        app: Some(ShellAppId::Phone),
        generation: 3,
    };
    match index {
        0 => Some(BueWriteSpec {
            bit: REBIND_READY_LAUNCHER_WRITE,
            prerequisite: 0,
            payload: ready,
        }),
        1 => Some(BueWriteSpec {
            bit: REBIND_READY_APP_WRITE,
            prerequisite: REBIND_READY_LAUNCHER_WRITE,
            payload: ready,
        }),
        2 => Some(BueWriteSpec {
            bit: REBIND_FOCUS_LAUNCHER_WRITE,
            prerequisite: REBIND_READY_APP_WRITE,
            payload: rebound,
        }),
        3 => Some(BueWriteSpec {
            bit: REBIND_FOCUS_APP_WRITE,
            prerequisite: REBIND_FOCUS_LAUNCHER_WRITE,
            payload: rebound,
        }),
        4 => Some(BueWriteSpec {
            bit: LAUNCHER_FOCUS_LAUNCHER_WRITE,
            prerequisite: REBIND_STAGE_BITS,
            payload: launcher,
        }),
        5 => Some(BueWriteSpec {
            bit: LAUNCHER_FOCUS_APP_WRITE,
            prerequisite: LAUNCHER_FOCUS_LAUNCHER_WRITE,
            payload: launcher,
        }),
        6 => Some(BueWriteSpec {
            bit: APP_FOCUS_LAUNCHER_WRITE,
            prerequisite: LAUNCHER_STAGE_BITS,
            payload: app,
        }),
        7 => Some(BueWriteSpec {
            bit: APP_FOCUS_APP_WRITE,
            prerequisite: APP_FOCUS_LAUNCHER_WRITE,
            payload: app,
        }),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct BueReadSpec {
    write_bit: u64,
    read_bit: u64,
    prerequisite: u64,
}

fn bue_read_spec(event: UiServerEvent, client: Client) -> Option<BueReadSpec> {
    if event.session_id() != 2 {
        return None;
    }
    match (event.payload(), client) {
        (UiServerEventPayload::Ready, Client::Launcher) => Some(BueReadSpec {
            write_bit: REBIND_READY_LAUNCHER_WRITE,
            read_bit: REBIND_READY_LAUNCHER_READ,
            prerequisite: 0,
        }),
        (UiServerEventPayload::Ready, Client::App) => Some(BueReadSpec {
            write_bit: REBIND_READY_APP_WRITE,
            read_bit: REBIND_READY_APP_READ,
            prerequisite: 0,
        }),
        (
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::App,
                app: Some(ShellAppId::Phone),
                focus_generation: 1,
            },
            Client::Launcher,
        ) => Some(BueReadSpec {
            write_bit: REBIND_FOCUS_LAUNCHER_WRITE,
            read_bit: REBIND_FOCUS_LAUNCHER_READ,
            prerequisite: REBIND_READY_LAUNCHER_READ,
        }),
        (
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::App,
                app: Some(ShellAppId::Phone),
                focus_generation: 1,
            },
            Client::App,
        ) => Some(BueReadSpec {
            write_bit: REBIND_FOCUS_APP_WRITE,
            read_bit: REBIND_FOCUS_APP_READ,
            prerequisite: REBIND_READY_APP_READ,
        }),
        (
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::Launcher,
                app: None,
                focus_generation: 2,
            },
            Client::Launcher,
        ) => Some(BueReadSpec {
            write_bit: LAUNCHER_FOCUS_LAUNCHER_WRITE,
            read_bit: LAUNCHER_FOCUS_LAUNCHER_READ,
            prerequisite: REBIND_STAGE_BITS,
        }),
        (
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::Launcher,
                app: None,
                focus_generation: 2,
            },
            Client::App,
        ) => Some(BueReadSpec {
            write_bit: LAUNCHER_FOCUS_APP_WRITE,
            read_bit: LAUNCHER_FOCUS_APP_READ,
            prerequisite: REBIND_STAGE_BITS,
        }),
        (
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::App,
                app: Some(ShellAppId::Phone),
                focus_generation: 3,
            },
            Client::Launcher,
        ) => Some(BueReadSpec {
            write_bit: APP_FOCUS_LAUNCHER_WRITE,
            read_bit: APP_FOCUS_LAUNCHER_READ,
            prerequisite: LAUNCHER_STAGE_BITS,
        }),
        (
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::App,
                app: Some(ShellAppId::Phone),
                focus_generation: 3,
            },
            Client::App,
        ) => Some(BueReadSpec {
            write_bit: APP_FOCUS_APP_WRITE,
            read_bit: APP_FOCUS_APP_READ,
            prerequisite: LAUNCHER_STAGE_BITS,
        }),
        _ => None,
    }
}

fn valid_bue(event: UiServerEvent, expected: ExpectedBue) -> bool {
    if event.session_id() != 2 {
        return false;
    }
    match (event.payload(), expected) {
        (UiServerEventPayload::Ready, ExpectedBue::Ready) => true,
        (
            UiServerEventPayload::FocusChanged {
                active_client,
                app,
                focus_generation,
            },
            ExpectedBue::Focus {
                client,
                app: expected_app,
                generation,
            },
        ) => active_client == client && app == expected_app && focus_generation == generation,
        _ => false,
    }
}

#[derive(Clone, Copy)]
enum Client {
    Launcher,
    App,
}

impl Client {
    fn from_owner(owners: TraceOwners, pid: u64, image: UserImageId) -> Option<Self> {
        match (image, pid) {
            (UserImageId::Launcher, pid) if pid == owners.launcher_pid => Some(Self::Launcher),
            (UserImageId::App, pid) if pid == owners.app_pid => Some(Self::App),
            _ => None,
        }
    }

    fn from_sender(owners: TraceOwners, pid: u64) -> Option<Self> {
        if pid == owners.launcher_pid {
            Some(Self::Launcher)
        } else if pid == owners.app_pid {
            Some(Self::App)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
enum AckPhase {
    Rebind,
    Launcher,
    App,
}

impl AckPhase {
    const fn from_tag(tag: u64) -> Option<Self> {
        match tag {
            REBIND_FOCUS_ACK_MAGIC => Some(Self::Rebind),
            LAUNCHER_FOCUS_ACK_MAGIC => Some(Self::Launcher),
            APP_FOCUS_ACK_MAGIC => Some(Self::App),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct AckSpec {
    focus_read_bit: u64,
    write_bit: u64,
    read_bit: u64,
}

const fn ack_spec(phase: AckPhase, client: Client) -> AckSpec {
    match (phase, client) {
        (AckPhase::Rebind, Client::Launcher) => AckSpec {
            focus_read_bit: REBIND_FOCUS_LAUNCHER_READ,
            write_bit: REBIND_ACK_LAUNCHER_WRITE,
            read_bit: REBIND_ACK_LAUNCHER_READ,
        },
        (AckPhase::Rebind, Client::App) => AckSpec {
            focus_read_bit: REBIND_FOCUS_APP_READ,
            write_bit: REBIND_ACK_APP_WRITE,
            read_bit: REBIND_ACK_APP_READ,
        },
        (AckPhase::Launcher, Client::Launcher) => AckSpec {
            focus_read_bit: LAUNCHER_FOCUS_LAUNCHER_READ,
            write_bit: LAUNCHER_ACK_LAUNCHER_WRITE,
            read_bit: LAUNCHER_ACK_LAUNCHER_READ,
        },
        (AckPhase::Launcher, Client::App) => AckSpec {
            focus_read_bit: LAUNCHER_FOCUS_APP_READ,
            write_bit: LAUNCHER_ACK_APP_WRITE,
            read_bit: LAUNCHER_ACK_APP_READ,
        },
        (AckPhase::App, Client::Launcher) => AckSpec {
            focus_read_bit: APP_FOCUS_LAUNCHER_READ,
            write_bit: APP_ACK_LAUNCHER_WRITE,
            read_bit: APP_ACK_LAUNCHER_READ,
        },
        (AckPhase::App, Client::App) => AckSpec {
            focus_read_bit: APP_FOCUS_APP_READ,
            write_bit: APP_ACK_APP_WRITE,
            read_bit: APP_ACK_APP_READ,
        },
    }
}

fn decode_tag(wire: &[u8]) -> Option<u64> {
    let bytes: [u8; 8] = wire.try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
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
    use std::sync::Mutex;

    const SURFACE_PID: u64 = (2_u64 << 32) | 5;
    const LAUNCHER_PID: u64 = (1_u64 << 32) | 6;
    const APP_PID: u64 = (1_u64 << 32) | 7;
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn owners() -> TraceOwners {
        TraceOwners {
            surface_pid: SURFACE_PID,
            launcher_pid: LAUNCHER_PID,
            app_pid: APP_PID,
        }
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
            &BUE_WRITES,
            &BUE_READS,
            &ACK_WRITES,
            &ACK_READS,
            &BOUNDARY_WRITES,
            &BOUNDARY_READS,
            &OUTPUT_COMMITS,
        ] {
            counter.store(0, Ordering::Relaxed);
        }
    }

    fn ready() -> [u8; bndr_ui::UI_SERVER_EVENT_WIRE_SIZE] {
        UiServerEvent::ready(2).unwrap().encode()
    }

    fn focus(client: UiClientId, app: Option<ShellAppId>, generation: u64) -> [u8; 64] {
        UiServerEvent::focus_changed(2, client, app, generation)
            .unwrap()
            .encode()
    }

    fn write_bue(wire: &[u8]) -> bool {
        record_channel_write_with(
            owners(),
            SURFACE_PID,
            UserImageId::SurfaceServer,
            wire,
            ChannelWriteKind::Bytes,
            false,
        )
    }

    fn read_bue(client: Client, wire: &[u8]) -> bool {
        let (pid, image) = match client {
            Client::Launcher => (LAUNCHER_PID, UserImageId::Launcher),
            Client::App => (APP_PID, UserImageId::App),
        };
        record_channel_read_with(
            owners(),
            pid,
            image,
            SURFACE_PID,
            wire,
            ChannelWriteKind::Bytes,
        )
    }

    fn write_ack(client: Client, magic: u64) -> bool {
        let (pid, image) = match client {
            Client::Launcher => (LAUNCHER_PID, UserImageId::Launcher),
            Client::App => (APP_PID, UserImageId::App),
        };
        record_channel_write_with(
            owners(),
            pid,
            image,
            &magic.to_le_bytes(),
            ChannelWriteKind::Bytes,
            false,
        )
    }

    fn read_ack(client: Client, magic: u64) -> bool {
        let sender = match client {
            Client::Launcher => LAUNCHER_PID,
            Client::App => APP_PID,
        };
        record_channel_read_with(
            owners(),
            SURFACE_PID,
            UserImageId::SurfaceServer,
            sender,
            &magic.to_le_bytes(),
            ChannelWriteKind::Bytes,
        )
    }

    fn finish_focus_phase(launcher_wire: &[u8], app_wire: &[u8], acknowledgement: u64) {
        assert!(write_bue(launcher_wire));
        assert!(write_bue(app_wire));
        assert!(read_bue(Client::App, app_wire));
        assert!(read_bue(Client::Launcher, launcher_wire));
        assert!(write_ack(Client::App, acknowledgement));
        assert!(write_ack(Client::Launcher, acknowledgement));
        assert!(read_ack(Client::App, acknowledgement));
        assert!(read_ack(Client::Launcher, acknowledgement));
    }

    #[test]
    fn exact_transcript_accepts_reversed_client_ack_order_and_closes_on_m52_ack() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();

        let ready = ready();
        let rebound = focus(UiClientId::App, Some(ShellAppId::Phone), 1);
        assert!(write_bue(&ready));
        assert!(write_bue(&ready));
        assert!(write_bue(&rebound));
        assert!(write_bue(&rebound));
        assert!(read_bue(Client::App, &ready));
        assert!(read_bue(Client::Launcher, &ready));
        assert!(read_bue(Client::App, &rebound));
        assert!(read_bue(Client::Launcher, &rebound));
        assert!(write_ack(Client::App, REBIND_FOCUS_ACK_MAGIC));
        assert!(write_ack(Client::Launcher, REBIND_FOCUS_ACK_MAGIC));
        assert!(read_ack(Client::App, REBIND_FOCUS_ACK_MAGIC));
        assert!(read_ack(Client::Launcher, REBIND_FOCUS_ACK_MAGIC));
        let rebound_snapshot = snapshot();
        assert!(rebound_snapshot.rebound_complete);
        assert_eq!(
            (
                rebound_snapshot.channel_writes,
                rebound_snapshot.channel_reads
            ),
            (6, 6)
        );

        let launcher = focus(UiClientId::Launcher, None, 2);
        finish_focus_phase(&launcher, &launcher, LAUNCHER_FOCUS_ACK_MAGIC);
        let launcher_snapshot = snapshot();
        assert!(launcher_snapshot.launcher_complete);
        assert_eq!(
            (
                launcher_snapshot.channel_writes,
                launcher_snapshot.channel_reads,
            ),
            (10, 10)
        );

        let app = focus(UiClientId::App, Some(ShellAppId::Phone), 3);
        finish_focus_phase(&app, &app, APP_FOCUS_ACK_MAGIC);
        let app_snapshot = snapshot();
        assert!(app_snapshot.app_complete);
        assert!(!app_snapshot.complete);
        assert_eq!(
            (app_snapshot.channel_writes, app_snapshot.channel_reads),
            (14, 14)
        );
        assert_eq!((app_snapshot.bue_writes, app_snapshot.bue_reads), (8, 8));
        assert_eq!((app_snapshot.ack_writes, app_snapshot.ack_reads), (6, 6));

        for frame in 1..=6 {
            assert!(record_output_commit_with(
                owners(),
                SURFACE_PID,
                2,
                0,
                2,
                u64::from(frame),
                frame,
            ));
        }
        assert_eq!(snapshot().output_commits, 0);

        let boundary = M52_APP_COMPLETION_ACK_MAGIC.to_le_bytes();
        assert!(record_channel_write_with(
            owners(),
            APP_PID,
            UserImageId::App,
            &boundary,
            ChannelWriteKind::Bytes,
            true,
        ));
        assert!(record_channel_read_with(
            owners(),
            SURFACE_PID,
            UserImageId::SurfaceServer,
            APP_PID,
            &boundary,
            ChannelWriteKind::Bytes,
        ));
        let complete = snapshot();
        assert!(complete.boundary_complete);
        assert!(complete.complete);
        assert_eq!((complete.boundary_writes, complete.boundary_reads), (1, 1));

        assert!(record_channel_write_with(
            owners(),
            APP_PID,
            UserImageId::App,
            b"unrelated",
            ChannelWriteKind::Transfer,
            true,
        ));
        assert_eq!(snapshot(), complete);
    }

    #[test]
    fn wrong_owner_order_and_payload_fail_closed() {
        let _guard = TEST_LOCK.lock().unwrap();
        let ready = ready();
        let rebound = focus(UiClientId::App, Some(ShellAppId::Phone), 1);

        reset();
        assert!(!record_channel_write_with(
            owners(),
            APP_PID,
            UserImageId::App,
            &ready,
            ChannelWriteKind::Bytes,
            false,
        ));
        assert_eq!(snapshot().owner_errors, 1);

        reset();
        assert!(!write_bue(&rebound));
        assert_eq!(snapshot().payload_errors, 1);

        reset();
        let wrong_session = UiServerEvent::ready(1).unwrap().encode();
        assert!(!write_bue(&wrong_session));
        assert_eq!(snapshot().payload_errors, 1);

        reset();
        assert!(write_bue(&ready));
        assert!(write_bue(&ready));
        assert!(write_bue(&rebound));
        assert!(write_bue(&rebound));
        assert!(!write_ack(Client::Launcher, REBIND_FOCUS_ACK_MAGIC));
        assert_eq!(snapshot().phase_errors, 1);
    }

    #[test]
    fn malformed_candidates_and_boundary_before_m52_are_rejected() {
        let _guard = TEST_LOCK.lock().unwrap();

        reset();
        assert!(!record_channel_write_with(
            owners(),
            SURFACE_PID,
            UserImageId::SurfaceServer,
            b"BUE1",
            ChannelWriteKind::Bytes,
            false,
        ));
        assert_eq!(snapshot().decode_errors, 1);

        reset();
        let corrupt_m53 = 0x4d35_335f_ffff_ffff_u64.to_le_bytes();
        assert!(!record_channel_write_with(
            owners(),
            APP_PID,
            UserImageId::App,
            &corrupt_m53,
            ChannelWriteKind::Bytes,
            false,
        ));
        assert_eq!(snapshot().payload_errors, 1);

        reset();
        let boundary = M52_APP_COMPLETION_ACK_MAGIC.to_le_bytes();
        assert!(!record_channel_write_with(
            owners(),
            APP_PID,
            UserImageId::App,
            &boundary,
            ChannelWriteKind::Bytes,
            false,
        ));
        assert_eq!(snapshot().phase_errors, 1);
    }

    #[test]
    fn output_six_is_ignored_but_output_seven_is_an_error() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        assert!(record_output_commit_with(
            owners(),
            SURFACE_PID,
            2,
            0,
            2,
            6,
            6,
        ));
        assert_eq!(snapshot().output_commits, 0);
        assert!(!record_output_commit_with(
            owners(),
            SURFACE_PID,
            2,
            0,
            2,
            7,
            7,
        ));
        let failed = snapshot();
        assert_eq!(failed.output_commits, 1);
        assert_eq!(failed.payload_errors, 1);
        assert_eq!(failed.errors, 1);
    }
}
