//! M67 unified interactive product runtime plus the opt-in M68--M74 liveness
//! layers.
//!
//! The M45 UI/input graph remains live through its complete host-driven
//! interaction transcript.  Only an authenticated power-key request then
//! moves the already-running services onto this control plane: the real
//! Launcher and App exercise AppData through one StorageServer, every live
//! service registers its kernel-authenticated shutdown identity, and init
//! performs bounded reverse-topological quiesce before the durable platform
//! boundary. M68 additionally supervises that StorageServer with real BSH1
//! probes, a finite timeout, bounded backoff, and one same-slot next-generation
//! replacement before any AppData client work begins.
//! M69 retains that exact recovery root, adds the already-running App as a
//! second BSH1 service, and enforces one hard `StorageServer -> App` edge:
//! the App acknowledges a blocked state before replacement and cannot perform
//! AppData work until the replacement is healthy and the edge is resumed.
//! M71 keeps M70's FDT-validated PSCI backend and moves the health path onto a
//! transactional five-service/four-edge catalog. It runs sixteen additional
//! healthy batches, then proves two simultaneous tolerated misses and a full
//! next-round recovery without weakening the ordinary restart budget.
//! M72 obtains that catalog from one kernel-owned immutable BMF1 VMO, validates
//! it completely, binds four already-running residents by declared node, and
//! launches StorageServer from the manifest before constructing the BSH1
//! supervisor. Its production order intentionally differs from M71's literals.
//! M73 keeps that kernel-owned service graph active before the power request,
//! drives catalog-wide health batches until an authenticated external event,
//! and demonstrates two clean StorageServer rotations with dependency
//! cancellation, restart-budget rearming, and in-flight health draining.
//! M74 preserves that runtime while requiring the external BMS1 envelope to
//! pass kernel-side RSA-2048 PKCS#1 v1.5 SHA-256 verification and the static
//! rollback floor before the immutable BMF1 VMO can be published. Signature
//! and valid-old-generation fixtures both fail closed before init-ready. M77
//! additionally requires init to open a boot-local maintenance session backed
//! by a separately signed, persistently audited BMA1 authorization before any
//! mutating supervisor report is accepted.

use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
use bndr_abi::MAINTENANCE_SESSION_OPEN_FLAGS_NONE;
use bndr_abi::{
    AppDataPrincipal, ChannelMessageKind, ChannelReadEnvelope, OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS,
    OBJECT_WAIT_TIMEOUT_INFINITE, ObjectSignals, PROCESS_SPAWN_FLAGS_NONE,
    ProcessTerminationReason, SERVICE_SHUTDOWN_FLAGS_NONE, SERVICE_SHUTDOWN_QUIESCE,
    SERVICE_SHUTDOWN_REGISTER, SHUTDOWN_SERVICE_ALL_MASK, SYSTEM_SHUTDOWN_COMMIT,
    SYSTEM_SHUTDOWN_FLAGS_NONE, SYSTEM_SHUTDOWN_PREPARE, ShutdownServiceNode, Status,
    SyscallNumber, UserImageId,
};
#[cfg(feature = "unified-product-liveness-runtime")]
use bndr_abi::{PROCESS_KILLED_EXIT_CODE, PROCESS_TERMINATE_FLAGS_NONE};
#[cfg(feature = "unified-product-manifest-supervision-runtime")]
use bndr_abi::{SERVICE_MANIFEST_OPEN_FLAGS_NONE, VMO_READ_MAX_BYTES, pack_vmo_read};
#[cfg(feature = "unified-product-event-supervision-runtime")]
use bndr_abi::{
    SERVICE_SUPERVISOR_REPORT_ACTIVE, SERVICE_SUPERVISOR_REPORT_CANCEL_WINDOW,
    SERVICE_SUPERVISOR_REPORT_ROTATION_BEGIN, SERVICE_SUPERVISOR_REPORT_ROTATION_COMPLETE,
    SERVICE_SUPERVISOR_REPORT_STOP_DRAINED, SERVICE_SUPERVISOR_REPORT_STOP_REQUESTED,
    SERVICE_SUPERVISOR_REPORT_UI_CONVERGENCE_QUERY,
};
#[cfg(feature = "unified-product-continuous-supervision-runtime")]
use bndr_sm::health::{DependencyDefinition, HealthDeadlineOutcome, ServiceDefinition};
#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
use bndr_sm::health::{DependencyImpact, DependencyKind, DependencyTransitions};
#[cfg(feature = "unified-product-liveness-runtime")]
use bndr_sm::health::{
    FaultClass, FaultDisposition, HealthFrame, HealthOpcode, ServiceIdentity, ServiceKind,
    ServicePhase, ServicePolicy, ServiceSupervisor,
};
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
use bndr_sm::maintenance_authorization::MAINTENANCE_OPERATION_STORAGE_ROTATION;
#[cfg(all(
    feature = "unified-product-persistent-rollback-runtime",
    not(feature = "unified-product-key-rotation-runtime")
))]
use bndr_sm::manifest::PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION;
#[cfg(all(
    feature = "unified-product-manifest-supervision-runtime",
    not(feature = "unified-product-verified-manifest-runtime")
))]
use bndr_sm::manifest::SERVICE_MANIFEST_GENERATION;
#[cfg(feature = "unified-product-continuous-supervision-runtime")]
use bndr_sm::manifest::SERVICE_MANIFEST_MAX_SERVICES;
#[cfg(all(
    feature = "unified-product-verified-manifest-runtime",
    not(feature = "unified-product-persistent-rollback-runtime")
))]
use bndr_sm::manifest::VERIFIED_SERVICE_MANIFEST_GENERATION;
#[cfg(feature = "unified-product-key-rotation-runtime")]
use bndr_sm::manifest::{
    KEY_ROTATION_RETIRED_KEY_FIXTURE_GENERATION, KEY_ROTATION_SERVICE_MANIFEST_GENERATION,
    KEY_ROTATION_TRANSITION_SERVICE_MANIFEST_GENERATION,
};
#[cfg(feature = "unified-product-manifest-supervision-runtime")]
use bndr_sm::manifest::{
    ManifestService, PRODUCT_SERVICE_MANIFEST_DEPENDENCY_COUNT,
    PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT, PRODUCT_SERVICE_MANIFEST_SIZE,
    SERVICE_MANIFEST_MAX_DEPENDENCIES, ServiceLaunchMode, ServiceManifest,
};

#[cfg(feature = "unified-product-liveness-runtime")]
use super::read_channel_envelope_now;
use super::storage_server_runtime::{
    SERVER_IDLE_TAG, SERVER_READY_TAG, SERVER_SHUTDOWN_ACK_TAG, SERVER_SHUTDOWN_COMMAND_TAG,
    SERVER_SHUTDOWN_EXIT_CODE, resident_client_work, resident_storage_admission_probe,
};
use super::{
    INIT_READY_MAGIC, OwnedUserHandle, ResidentChildren, assert_stale_handle, close_owned,
    exit_child, fail, object_wait_many_array, pack_user_wait_item, read_scalar_envelope,
    signal_union, syscall, write_scalar,
};

pub(super) const M67_STORAGE_READY_PREFIX: u64 = 0x4d36_3747_0000_0000;
pub(super) const M67_STORAGE_PROOF: u64 = 0x4d36_3750_080f_0301;
#[cfg(feature = "unified-product-liveness-runtime")]
pub(super) const M68_STORAGE_READY_PREFIX: u64 = 0x4d36_3847_0000_0000;
#[cfg(feature = "unified-product-liveness-runtime")]
pub(super) const M68_STORAGE_PROOF: u64 = 0x4d36_3850_0302_0101;
#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
pub(super) const M69_STORAGE_READY_PREFIX: u64 = 0x4d36_3947_0000_0000;
#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
pub(super) const M69_STORAGE_PROOF: u64 = 0x4d36_3950_0807_0201;
#[cfg(feature = "unified-product-psci-shutdown-runtime")]
pub(super) const M70_STORAGE_READY_PREFIX: u64 = 0x4d37_3047_0000_0000;
#[cfg(feature = "unified-product-psci-shutdown-runtime")]
pub(super) const M70_STORAGE_PROOF: u64 = 0x4d37_3050_0807_0201;
#[cfg(feature = "unified-product-continuous-supervision-runtime")]
pub(super) const M71_STORAGE_READY_PREFIX: u64 = 0x4d37_3147_0000_0000;
#[cfg(feature = "unified-product-continuous-supervision-runtime")]
pub(super) const M71_STORAGE_PROOF: u64 = 0x4d37_3150_1505_0401;
#[cfg(feature = "unified-product-manifest-supervision-runtime")]
pub(super) const M72_STORAGE_READY_PREFIX: u64 = 0x4d37_3247_0000_0000;
#[cfg(feature = "unified-product-manifest-supervision-runtime")]
pub(super) const M72_STORAGE_PROOF: u64 = 0x4d37_3250_0105_0401;
#[cfg(feature = "unified-product-event-supervision-runtime")]
pub(super) const M73_STORAGE_READY_PREFIX: u64 = 0x4d37_3347_0000_0000;
#[cfg(feature = "unified-product-event-supervision-runtime")]
pub(super) const M73_STORAGE_PROOF: u64 = 0x4d37_3350_0205_0401;

#[cfg(feature = "unified-product-liveness-runtime")]
const STORAGE_HEALTH_TIMEOUT_NS: u64 = 100_000_000;
#[cfg(feature = "unified-product-liveness-runtime")]
const STORAGE_RESTART_BACKOFF_NS: u64 = 30_000_000;
#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
const PRODUCT_HEALTH_CADENCE_NS: u64 = 40_000_000;
#[cfg(feature = "unified-product-continuous-supervision-runtime")]
const M71_HEALTHY_SOAK_ROUNDS: u64 = 16;
#[cfg(feature = "unified-product-continuous-supervision-runtime")]
const M71_RESIDENT_MISS_SEQUENCE: u64 = 20;

const PRODUCT_COMMAND_TAG: u64 = 0x4d36_3750_434d_4421;
const PRODUCT_ACK_TAG: u64 = 0x4d36_3750_4143_4b21;
const PRODUCT_POWER_TAG: u64 = 0x4d36_3750_5057_5221;
#[cfg(feature = "unified-product-event-supervision-runtime")]
const PRODUCT_ROTATION_TAG: u64 = 0x4d37_3352_4f54_4154;
const PRODUCT_COMMAND_REGISTER: u64 = 1;
const PRODUCT_COMMAND_STORAGE_WORK: u64 = 2;
const PRODUCT_COMMAND_PROBE_QUIESCE: u64 = 3;
const PRODUCT_COMMAND_QUIESCE: u64 = 4;
const PRODUCT_COMMAND_EXIT: u64 = 5;
#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
const PRODUCT_COMMAND_DEPENDENCY_BLOCK: u64 = 6;
#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
const PRODUCT_COMMAND_DEPENDENCY_RESUME: u64 = 7;
const PRODUCT_NODE_EXIT_PREFIX: u64 = 0x4d36_3753_0000_0000;

const FAIL_ABI: u64 = 0x6701;
const FAIL_CONTROL: u64 = 0x6702;
const FAIL_POWER: u64 = 0x6703;
const FAIL_STORAGE: u64 = 0x6704;
const FAIL_REGISTER: u64 = 0x6705;
const FAIL_READY: u64 = 0x6706;
const FAIL_PREPARE: u64 = 0x6707;
const FAIL_QUIESCE: u64 = 0x6708;
const FAIL_WAIT: u64 = 0x6709;
const FAIL_EXIT: u64 = 0x670a;
#[cfg(feature = "unified-product-liveness-runtime")]
const FAIL_LIVENESS: u64 = 0x6801;
#[cfg(feature = "unified-product-manifest-supervision-runtime")]
const FAIL_MANIFEST: u64 = 0x7201;
#[cfg(feature = "unified-product-event-supervision-runtime")]
const FAIL_EVENT_SUPERVISION: u64 = 0x7301;

// Every executable image receives an isolated BSS, so this is process-local
// state even though all user images link the same runtime library.
static CONTROL_HANDLE: AtomicU64 = AtomicU64::new(0);
static INIT_PID: AtomicU64 = AtomicU64::new(0);
static NODE_PLUS_ONE: AtomicU64 = AtomicU64::new(0);
static POWER_REQUESTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-event-supervision-runtime")]
static ROTATION_REQUESTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
static APP_DEPENDENCY_STATE: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
static APP_HEALTH_PID: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
static APP_HEALTH_GENERATION: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
static APP_HEALTH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Child {
    control: u64,
    pid: u64,
    node: Option<ShutdownServiceNode>,
}

#[cfg(feature = "unified-product-manifest-supervision-runtime")]
type ProductSupervisor =
    ServiceSupervisor<SERVICE_MANIFEST_MAX_SERVICES, SERVICE_MANIFEST_MAX_DEPENDENCIES>;
#[cfg(all(
    feature = "unified-product-continuous-supervision-runtime",
    not(feature = "unified-product-manifest-supervision-runtime")
))]
type ProductSupervisor = ServiceSupervisor<5, 4>;

#[cfg(feature = "unified-product-continuous-supervision-runtime")]
#[derive(Clone, Copy)]
struct SupervisedService {
    identity: ServiceIdentity,
    child: Child,
}

#[cfg(feature = "unified-product-continuous-supervision-runtime")]
struct SupervisedServices {
    entries: [Option<SupervisedService>; SERVICE_MANIFEST_MAX_SERVICES],
    length: usize,
}

#[cfg(feature = "unified-product-continuous-supervision-runtime")]
impl SupervisedServices {
    const fn new() -> Self {
        Self {
            entries: [None; SERVICE_MANIFEST_MAX_SERVICES],
            length: 0,
        }
    }

    fn push(&mut self, service: SupervisedService) {
        if self.length >= self.entries.len()
            || self
                .entries
                .iter()
                .flatten()
                .any(|entry| entry.identity.kind() == service.identity.kind())
        {
            fail(FAIL_LIVENESS);
        }
        self.entries[self.length] = Some(service);
        self.length += 1;
    }

    fn replace(&mut self, kind: ServiceKind, replacement: SupervisedService) {
        let entry = self
            .entries
            .iter_mut()
            .flatten()
            .find(|entry| entry.identity.kind() == kind)
            .unwrap_or_else(|| fail(FAIL_LIVENESS));
        if replacement.identity.kind() != kind {
            fail(FAIL_LIVENESS);
        }
        *entry = replacement;
    }

    fn require(&self, kind: ServiceKind) -> SupervisedService {
        self.iter()
            .find(|entry| entry.identity.kind() == kind)
            .unwrap_or_else(|| fail(FAIL_LIVENESS))
    }

    const fn len(&self) -> usize {
        self.length
    }

    fn get(&self, index: usize) -> SupervisedService {
        self.entries
            .get(index)
            .copied()
            .flatten()
            .unwrap_or_else(|| fail(FAIL_LIVENESS))
    }

    fn iter(&self) -> impl Iterator<Item = SupervisedService> + '_ {
        self.entries[..self.length].iter().copied().flatten()
    }
}

pub(super) fn init_runtime(system_root: u64) -> ! {
    expect_abi();
    super::lifecycle_runtime::init_runtime(system_root)
}

/// Arms one already-real service loop without changing its normal protocol.
///
/// Product commands are not sent until the kernel has sealed the complete
/// interactive transcript.  Consequently, once the first command is
/// intercepted, the service may remain on this terminal control path without
/// starving ordinary UI work.
pub(super) fn arm(control: u64, node: ShutdownServiceNode, init_pid: u64) {
    if control == 0
        || init_pid == 0
        || CONTROL_HANDLE
            .compare_exchange(0, control, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        || INIT_PID
            .compare_exchange(0, init_pid, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        || NODE_PLUS_ONE
            .compare_exchange(
                0,
                node.raw().saturating_add(1),
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
    {
        fail(FAIL_CONTROL);
    }
}

/// Consumes one terminal product command if the head envelope belongs to the
/// armed control channel. Non-product envelopes are returned to the existing
/// service protocol unchanged.
pub(super) fn dispatch_control(transport: u64, envelope: &ChannelReadEnvelope) -> bool {
    if transport != CONTROL_HANDLE.load(Ordering::Acquire) {
        return false;
    }
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    if envelope.kind() == ChannelMessageKind::Bytes
        && envelope.logical_length() == bndr_sm::health::HEALTH_FRAME_SIZE
        && envelope.data()[..bndr_sm::health::HEALTH_PROTOCOL_MAGIC.len()]
            == bndr_sm::health::HEALTH_PROTOCOL_MAGIC
    {
        dispatch_resident_health(transport, envelope);
        return true;
    }
    #[cfg(all(
        feature = "unified-product-multiservice-liveness-runtime",
        not(feature = "unified-product-continuous-supervision-runtime")
    ))]
    if envelope.kind() == ChannelMessageKind::Bytes
        && envelope.logical_length() == bndr_sm::health::HEALTH_FRAME_SIZE
        && envelope.data()[..bndr_sm::health::HEALTH_PROTOCOL_MAGIC.len()]
            == bndr_sm::health::HEALTH_PROTOCOL_MAGIC
    {
        dispatch_app_health(transport, envelope);
        return true;
    }
    let Some((tag, command)) = envelope.scalar_values() else {
        return false;
    };
    if tag != PRODUCT_COMMAND_TAG {
        return false;
    }
    let init_pid = INIT_PID.load(Ordering::Acquire);
    let node = armed_node();
    if envelope.kind() != ChannelMessageKind::Scalar
        || envelope.sender_pid() != init_pid
        || init_pid == 0
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_CONTROL);
    }

    match command {
        PRODUCT_COMMAND_REGISTER => {
            let registered = syscall(
                SyscallNumber::ServiceShutdown,
                SERVICE_SHUTDOWN_REGISTER,
                node.raw(),
                node.dependency_mask(),
            );
            if registered.status != Status::Ok.raw()
                || registered.out1 & node.bit() == 0
                || registered.out1 & !SHUTDOWN_SERVICE_ALL_MASK != 0
                || registered.out2 != node.dependency_mask()
            {
                fail(FAIL_REGISTER);
            }
            acknowledge(node, registered.out1);
        }
        PRODUCT_COMMAND_STORAGE_WORK => {
            let principal = match node {
                ShutdownServiceNode::Launcher => AppDataPrincipal::LAUNCHER,
                ShutdownServiceNode::App => {
                    #[cfg(feature = "unified-product-event-supervision-runtime")]
                    if APP_DEPENDENCY_STATE.load(Ordering::Acquire) == 1 {
                        fail(FAIL_LIVENESS);
                    }
                    #[cfg(all(
                        feature = "unified-product-multiservice-liveness-runtime",
                        not(feature = "unified-product-event-supervision-runtime")
                    ))]
                    if APP_DEPENDENCY_STATE.load(Ordering::Acquire) != 2 {
                        fail(FAIL_LIVENESS);
                    }
                    AppDataPrincipal::PRIMARY_APP
                }
                _ => fail(FAIL_STORAGE),
            };
            let stage = resident_client_work(transport, principal);
            acknowledge(node, stage);
        }
        PRODUCT_COMMAND_PROBE_QUIESCE => {
            if node != ShutdownServiceNode::SurfaceServer {
                fail(FAIL_QUIESCE);
            }
            let rejected = syscall(
                SyscallNumber::ServiceShutdown,
                SERVICE_SHUTDOWN_QUIESCE,
                node.raw(),
                SERVICE_SHUTDOWN_FLAGS_NONE,
            );
            if rejected.status != Status::InvalidState.raw()
                || rejected.out1 != 0
                || rejected.out2 != 0
            {
                fail(FAIL_QUIESCE);
            }
            acknowledge(node, 0);
        }
        PRODUCT_COMMAND_QUIESCE => {
            if matches!(
                node,
                ShutdownServiceNode::Launcher | ShutdownServiceNode::App
            ) {
                resident_storage_admission_probe();
            }
            let quiesced = syscall(
                SyscallNumber::ServiceShutdown,
                SERVICE_SHUTDOWN_QUIESCE,
                node.raw(),
                SERVICE_SHUTDOWN_FLAGS_NONE,
            );
            if quiesced.status != Status::Ok.raw()
                || quiesced.out1 & node.bit() == 0
                || quiesced.out1 & !SHUTDOWN_SERVICE_ALL_MASK != 0
                || quiesced.out2 != node.shutdown_wave()
            {
                fail(FAIL_QUIESCE);
            }
            acknowledge(node, quiesced.out1);
        }
        PRODUCT_COMMAND_EXIT => {
            #[cfg(feature = "unified-product-event-supervision-runtime")]
            if node == ShutdownServiceNode::App && APP_DEPENDENCY_STATE.load(Ordering::Acquire) == 1
            {
                fail(FAIL_LIVENESS);
            }
            #[cfg(all(
                feature = "unified-product-multiservice-liveness-runtime",
                not(feature = "unified-product-event-supervision-runtime")
            ))]
            if node == ShutdownServiceNode::App && APP_DEPENDENCY_STATE.load(Ordering::Acquire) != 2
            {
                fail(FAIL_LIVENESS);
            }
            exit_child(PRODUCT_NODE_EXIT_PREFIX | node.raw().saturating_add(1));
        }
        #[cfg(feature = "unified-product-multiservice-liveness-runtime")]
        PRODUCT_COMMAND_DEPENDENCY_BLOCK => {
            #[cfg(feature = "unified-product-event-supervision-runtime")]
            let expected = match APP_DEPENDENCY_STATE.load(Ordering::Acquire) {
                state @ (0 | 2) => state,
                _ => fail(FAIL_LIVENESS),
            };
            #[cfg(not(feature = "unified-product-event-supervision-runtime"))]
            let expected = 0;
            if node != ShutdownServiceNode::App
                || APP_DEPENDENCY_STATE
                    .compare_exchange(expected, 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
            {
                fail(FAIL_LIVENESS);
            }
            acknowledge(node, 1);
        }
        #[cfg(feature = "unified-product-multiservice-liveness-runtime")]
        PRODUCT_COMMAND_DEPENDENCY_RESUME => {
            if node != ShutdownServiceNode::App
                || APP_DEPENDENCY_STATE
                    .compare_exchange(1, 2, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
            {
                fail(FAIL_LIVENESS);
            }
            acknowledge(node, 2);
        }
        _ => fail(FAIL_CONTROL),
    }
    true
}

#[cfg(all(
    feature = "unified-product-multiservice-liveness-runtime",
    not(feature = "unified-product-continuous-supervision-runtime")
))]
fn dispatch_app_health(transport: u64, envelope: &ChannelReadEnvelope) {
    if armed_node() != ShutdownServiceNode::App
        || envelope.sender_pid() != INIT_PID.load(Ordering::Acquire)
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_LIVENESS);
    }
    let probe = HealthFrame::decode(envelope.data()).unwrap_or_else(|_| fail(FAIL_LIVENESS));
    let identity = probe.identity();
    let expected_sequence = APP_HEALTH_SEQUENCE
        .load(Ordering::Acquire)
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    let dependency_state = APP_DEPENDENCY_STATE.load(Ordering::Acquire);
    if probe.opcode() != HealthOpcode::Probe
        || probe.sequence() != expected_sequence
        || probe.interval_ns() != STORAGE_HEALTH_TIMEOUT_NS
        || !probe.flags().ack_required()
        || identity.kind() != ServiceKind::App
        || identity.generation() != process_generation(identity.pid())
        || process_slot(identity.pid()) == 0
        || !matches!((expected_sequence, dependency_state), (1 | 2, 0) | (3, 2))
    {
        fail(FAIL_LIVENESS);
    }
    if expected_sequence == 1 {
        if APP_HEALTH_PID
            .compare_exchange(0, identity.pid(), Ordering::AcqRel, Ordering::Acquire)
            .is_err()
            || APP_HEALTH_GENERATION
                .compare_exchange(
                    0,
                    u64::from(identity.generation()),
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_err()
        {
            fail(FAIL_LIVENESS);
        }
    } else if APP_HEALTH_PID.load(Ordering::Acquire) != identity.pid()
        || APP_HEALTH_GENERATION.load(Ordering::Acquire) != u64::from(identity.generation())
    {
        fail(FAIL_LIVENESS);
    }
    APP_HEALTH_SEQUENCE.store(expected_sequence, Ordering::Release);
    let healthy =
        HealthFrame::healthy(expected_sequence, identity).unwrap_or_else(|_| fail(FAIL_LIVENESS));
    write_health(transport, healthy);
}

#[cfg(feature = "unified-product-continuous-supervision-runtime")]
fn dispatch_resident_health(transport: u64, envelope: &ChannelReadEnvelope) {
    let node = armed_node();
    let expected_kind = match node {
        ShutdownServiceNode::ServiceManager => ServiceKind::ServiceManager,
        ShutdownServiceNode::SurfaceServer => ServiceKind::SurfaceServer,
        ShutdownServiceNode::InputServer => ServiceKind::InputServer,
        ShutdownServiceNode::App => ServiceKind::App,
        _ => fail(FAIL_LIVENESS),
    };
    if envelope.sender_pid() != INIT_PID.load(Ordering::Acquire)
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_LIVENESS);
    }
    let probe = HealthFrame::decode(envelope.data()).unwrap_or_else(|_| fail(FAIL_LIVENESS));
    let identity = probe.identity();
    let expected_sequence = APP_HEALTH_SEQUENCE
        .load(Ordering::Acquire)
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    let sequence_in_profile = true;
    #[cfg(not(feature = "unified-product-event-supervision-runtime"))]
    let sequence_in_profile = expected_sequence <= 21;
    if probe.opcode() != HealthOpcode::Probe
        || probe.sequence() != expected_sequence
        || probe.interval_ns() != STORAGE_HEALTH_TIMEOUT_NS
        || !probe.flags().ack_required()
        || identity.kind() != expected_kind
        || identity.generation() != process_generation(identity.pid())
        || process_slot(identity.pid()) == 0
        || !sequence_in_profile
    {
        fail(FAIL_LIVENESS);
    }
    if expected_kind == ServiceKind::App {
        let dependency_state = APP_DEPENDENCY_STATE.load(Ordering::Acquire);
        #[cfg(feature = "unified-product-event-supervision-runtime")]
        let dependency_state_valid = matches!(dependency_state, 0 | 2);
        #[cfg(not(feature = "unified-product-event-supervision-runtime"))]
        let dependency_state_valid = matches!(
            (expected_sequence, dependency_state),
            (1 | 2, 0) | (3..=21, 2)
        );
        if !dependency_state_valid {
            fail(FAIL_LIVENESS);
        }
    }
    if expected_sequence == 1 {
        if APP_HEALTH_PID
            .compare_exchange(0, identity.pid(), Ordering::AcqRel, Ordering::Acquire)
            .is_err()
            || APP_HEALTH_GENERATION
                .compare_exchange(
                    0,
                    u64::from(identity.generation()),
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_err()
        {
            fail(FAIL_LIVENESS);
        }
    } else if APP_HEALTH_PID.load(Ordering::Acquire) != identity.pid()
        || APP_HEALTH_GENERATION.load(Ordering::Acquire) != u64::from(identity.generation())
    {
        fail(FAIL_LIVENESS);
    }
    APP_HEALTH_SEQUENCE.store(expected_sequence, Ordering::Release);
    #[cfg(not(feature = "unified-product-event-supervision-runtime"))]
    if expected_sequence == M71_RESIDENT_MISS_SEQUENCE
        && matches!(
            expected_kind,
            ServiceKind::SurfaceServer | ServiceKind::InputServer
        )
    {
        // Both live services consume and authenticate the same batched probe
        // but deliberately omit one Healthy reply. Init must observe one
        // shared deadline, tolerate exactly one miss per policy, and then
        // recover both on sequence 21.
        return;
    }
    let healthy =
        HealthFrame::healthy(expected_sequence, identity).unwrap_or_else(|_| fail(FAIL_LIVENESS));
    write_health(transport, healthy);
}

pub(super) fn request_power_key(sequence: u64, value: u8) {
    if armed_node() != ShutdownServiceNode::InputServer || sequence == 0 {
        fail(FAIL_POWER);
    }
    if value != bndr_abi::InputEvent::KEY_VALUE_PRESS {
        return;
    }
    if POWER_REQUESTS
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        fail(FAIL_POWER);
    }
    write_scalar(
        CONTROL_HANDLE.load(Ordering::Acquire),
        PRODUCT_POWER_TAG,
        sequence,
    );
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
pub(super) fn request_storage_rotation(sequence: u64, value: u8) {
    if armed_node() != ShutdownServiceNode::InputServer || sequence == 0 {
        fail(FAIL_EVENT_SUPERVISION);
    }
    if value != bndr_abi::InputEvent::KEY_VALUE_PRESS {
        return;
    }
    let previous = ROTATION_REQUESTS.load(Ordering::Acquire);
    let ordinal = previous
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
    if ordinal > 2
        || sequence & 0xff00_0000_0000_0000 != 0
        || ROTATION_REQUESTS
            .compare_exchange(previous, ordinal, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
    {
        fail(FAIL_EVENT_SUPERVISION);
    }
    write_scalar(
        CONTROL_HANDLE.load(Ordering::Acquire),
        PRODUCT_ROTATION_TAG,
        (ordinal << 56) | sequence,
    );
}

pub(super) fn complete_runtime(
    children: &ResidentChildren,
    surface: OwnedUserHandle,
    launcher: OwnedUserHandle,
    app: OwnedUserHandle,
) -> ! {
    let input = children
        .input_server_control
        .as_ref()
        .unwrap_or_else(|| fail(FAIL_CONTROL));
    let nodes = [
        Child {
            control: children.manager_control.raw(),
            pid: children.manager_pid,
            node: Some(ShutdownServiceNode::ServiceManager),
        },
        Child {
            control: children.provider_control.raw(),
            pid: children.provider_pid,
            node: Some(ShutdownServiceNode::Provider),
        },
        Child {
            control: children.client_control.raw(),
            pid: children.client_pid,
            node: Some(ShutdownServiceNode::PrimaryClient),
        },
        Child {
            control: children
                .secondary_control
                .as_ref()
                .unwrap_or_else(|| fail(FAIL_CONTROL))
                .raw(),
            pid: children.secondary_pid,
            node: Some(ShutdownServiceNode::SecondaryClient),
        },
        Child {
            control: surface.raw(),
            pid: children.surface_server_pid,
            node: Some(ShutdownServiceNode::SurfaceServer),
        },
        Child {
            control: input.raw(),
            pid: children.input_server_pid,
            node: Some(ShutdownServiceNode::InputServer),
        },
        Child {
            control: launcher.raw(),
            pid: children.launcher_pid,
            node: Some(ShutdownServiceNode::Launcher),
        },
        Child {
            control: app.raw(),
            pid: children.app_pid,
            node: Some(ShutdownServiceNode::App),
        },
    ];
    validate_node_table(&nodes);
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    wait_for_kernel_ui_convergence(&nodes);
    #[cfg(not(feature = "unified-product-event-supervision-runtime"))]
    wait_for_power_request(&nodes);

    #[cfg(feature = "unified-product-manifest-supervision-runtime")]
    let manifest = open_product_service_manifest();

    // The storage owner is deliberately the final spawn. This preserves the
    // exact M45 interactive identity transcript and makes slot 9 an explicit
    // product-only extension rather than a replacement UI process.
    #[cfg(feature = "unified-product-manifest-supervision-runtime")]
    let server = {
        let service = manifest
            .service(ServiceKind::StorageServer)
            .filter(|service| {
                service.launch_mode() == ServiceLaunchMode::Spawn
                    && service.shutdown_node().is_none()
            })
            .unwrap_or_else(|| fail(FAIL_MANIFEST));
        spawn(service.image(), service.shutdown_node())
    };
    #[cfg(not(feature = "unified-product-manifest-supervision-runtime"))]
    let server = spawn(UserImageId::StorageServer, None);
    let boot_generation = expect_server_generation(server, SERVER_READY_TAG);
    if boot_generation == 0 || boot_generation & !u64::from(u32::MAX) != 0 {
        fail(FAIL_STORAGE);
    }
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    let (server, boot_generation) =
        run_event_supervision(server, boot_generation, &nodes, &manifest);
    #[cfg(all(
        feature = "unified-product-multiservice-liveness-runtime",
        not(feature = "unified-product-event-supervision-runtime")
    ))]
    let (server, boot_generation) = recover_product_services(
        server,
        boot_generation,
        nodes[ShutdownServiceNode::App.raw() as usize],
        &nodes,
        #[cfg(feature = "unified-product-manifest-supervision-runtime")]
        &manifest,
    );
    #[cfg(all(
        feature = "unified-product-liveness-runtime",
        not(feature = "unified-product-multiservice-liveness-runtime")
    ))]
    let (server, boot_generation) = recover_storage_server(server, boot_generation);

    expect_status(
        syscall(
            SyscallNumber::SystemShutdown,
            SYSTEM_SHUTDOWN_PREPARE,
            boot_generation,
            SYSTEM_SHUTDOWN_FLAGS_NONE,
        ),
        Status::InvalidState,
        FAIL_PREPARE,
    );
    expect_status(
        syscall(
            SyscallNumber::ServiceShutdown,
            SERVICE_SHUTDOWN_REGISTER,
            ShutdownServiceNode::App.raw(),
            ShutdownServiceNode::App.dependency_mask(),
        ),
        Status::PermissionDenied,
        FAIL_REGISTER,
    );

    let mut expected_registered = 0_u64;
    for child in nodes {
        let node = child.node.unwrap_or_else(|| fail(FAIL_CONTROL));
        expected_registered |= node.bit();
        command_and_expect(child, PRODUCT_COMMAND_REGISTER, expected_registered);
    }
    if expected_registered != SHUTDOWN_SERVICE_ALL_MASK {
        fail(FAIL_REGISTER);
    }

    let launcher_stage = command_and_expect(
        nodes[ShutdownServiceNode::Launcher.raw() as usize],
        PRODUCT_COMMAND_STORAGE_WORK,
        u64::MAX,
    );
    let app_stage = command_and_expect(
        nodes[ShutdownServiceNode::App.raw() as usize],
        PRODUCT_COMMAND_STORAGE_WORK,
        u64::MAX,
    );
    let final_generation = expect_server_generation(server, SERVER_IDLE_TAG);
    validate_storage_proof(boot_generation, final_generation, launcher_stage, app_stage);

    #[cfg(feature = "unified-product-event-supervision-runtime")]
    let (ready_prefix, product_proof) = (M73_STORAGE_READY_PREFIX, M73_STORAGE_PROOF);
    #[cfg(all(
        feature = "unified-product-manifest-supervision-runtime",
        not(feature = "unified-product-event-supervision-runtime")
    ))]
    let (ready_prefix, product_proof) = (M72_STORAGE_READY_PREFIX, M72_STORAGE_PROOF);
    #[cfg(all(
        feature = "unified-product-continuous-supervision-runtime",
        not(feature = "unified-product-manifest-supervision-runtime")
    ))]
    let (ready_prefix, product_proof) = (M71_STORAGE_READY_PREFIX, M71_STORAGE_PROOF);
    #[cfg(all(
        feature = "unified-product-psci-shutdown-runtime",
        not(feature = "unified-product-continuous-supervision-runtime")
    ))]
    let (ready_prefix, product_proof) = (M70_STORAGE_READY_PREFIX, M70_STORAGE_PROOF);
    #[cfg(all(
        feature = "unified-product-multiservice-liveness-runtime",
        not(feature = "unified-product-psci-shutdown-runtime")
    ))]
    let (ready_prefix, product_proof) = (M69_STORAGE_READY_PREFIX, M69_STORAGE_PROOF);
    #[cfg(all(
        feature = "unified-product-liveness-runtime",
        not(feature = "unified-product-multiservice-liveness-runtime")
    ))]
    let (ready_prefix, product_proof) = (M68_STORAGE_READY_PREFIX, M68_STORAGE_PROOF);
    #[cfg(not(feature = "unified-product-liveness-runtime"))]
    let (ready_prefix, product_proof) = (M67_STORAGE_READY_PREFIX, M67_STORAGE_PROOF);
    let ready = syscall(
        SyscallNumber::InitReady,
        INIT_READY_MAGIC,
        ready_prefix | final_generation,
        product_proof,
    );
    if ready.status != Status::Ok.raw() || ready.out1 != 0 || ready.out2 != 0 {
        fail(FAIL_READY);
    }

    let prepared = syscall(
        SyscallNumber::SystemShutdown,
        SYSTEM_SHUTDOWN_PREPARE,
        final_generation,
        SYSTEM_SHUTDOWN_FLAGS_NONE,
    );
    if prepared.status != Status::Ok.raw()
        || prepared.out1 != final_generation
        || prepared.out2 != 0
    {
        fail(FAIL_PREPARE);
    }
    exercise_spawn_barrier();

    let surface_node = nodes[ShutdownServiceNode::SurfaceServer.raw() as usize];
    command_and_expect(surface_node, PRODUCT_COMMAND_PROBE_QUIESCE, 0);

    let order = [
        ShutdownServiceNode::PrimaryClient,
        ShutdownServiceNode::SecondaryClient,
        ShutdownServiceNode::Launcher,
        ShutdownServiceNode::App,
        ShutdownServiceNode::Provider,
        ShutdownServiceNode::InputServer,
        ShutdownServiceNode::ServiceManager,
        ShutdownServiceNode::SurfaceServer,
    ];
    let mut quiesced_mask = 0_u64;
    for node in order {
        quiesced_mask |= node.bit();
        command_and_expect(
            nodes[node.raw() as usize],
            PRODUCT_COMMAND_QUIESCE,
            quiesced_mask,
        );
    }
    if quiesced_mask != SHUTDOWN_SERVICE_ALL_MASK {
        fail(FAIL_QUIESCE);
    }

    // Keep all quiesced processes alive until every logical edge is sealed.
    // They are now blocked exclusively on their init control endpoints, so
    // subsequent peer closure cannot re-enter an ordinary service protocol.
    for child in nodes {
        write_scalar(child.control, PRODUCT_COMMAND_TAG, PRODUCT_COMMAND_EXIT);
    }
    for child in nodes {
        wait_for_exit(
            child,
            PRODUCT_NODE_EXIT_PREFIX
                | child
                    .node
                    .unwrap_or_else(|| fail(FAIL_EXIT))
                    .raw()
                    .saturating_add(1),
        );
    }

    write_scalar(
        server.control,
        SERVER_SHUTDOWN_COMMAND_TAG,
        final_generation,
    );
    let (sender, (tag, acknowledged_generation)) = read_scalar_envelope(server.control);
    if sender != server.pid
        || tag != SERVER_SHUTDOWN_ACK_TAG
        || acknowledged_generation != final_generation
    {
        fail(FAIL_STORAGE);
    }
    wait_for_exit(server, SERVER_SHUTDOWN_EXIT_CODE);

    let committed = syscall(
        SyscallNumber::SystemShutdown,
        SYSTEM_SHUTDOWN_COMMIT,
        final_generation,
        SYSTEM_SHUTDOWN_FLAGS_NONE,
    );
    if committed.status != Status::Ok.raw()
        || committed.out1 != final_generation
        || committed.out2 != 0
    {
        fail(FAIL_PREPARE);
    }
    loop {
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

fn wait_for_power_request(nodes: &[Child; 8]) {
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    for (item, child) in items.iter_mut().zip(nodes) {
        *item = pack_user_wait_item(child.control, requested);
    }
    let ready = object_wait_many_array(&items, nodes.len(), OBJECT_WAIT_TIMEOUT_INFINITE);
    let expected_index = ShutdownServiceNode::InputServer.raw();
    if ready.status != Status::Ok.raw()
        || ready.out1 != expected_index
        || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
        || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
    {
        fail(FAIL_POWER);
    }
    let input = nodes[expected_index as usize];
    let (sender, (tag, sequence)) = read_scalar_envelope(input.control);
    if sender != input.pid || tag != PRODUCT_POWER_TAG || sequence == 0 {
        fail(FAIL_POWER);
    }
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn wait_for_kernel_ui_convergence(nodes: &[Child; 8]) {
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    for (item, child) in items.iter_mut().zip(nodes) {
        *item = pack_user_wait_item(child.control, requested);
    }
    loop {
        let queried = syscall(
            SyscallNumber::ServiceSupervisorReport,
            SERVICE_SUPERVISOR_REPORT_UI_CONVERGENCE_QUERY,
            0,
            0,
        );
        if queried.status == Status::Ok.raw() && queried.out1 == 0 && queried.out2 == 0 {
            return;
        }
        if queried.status != Status::ShouldWait.raw() || queried.out1 != 0 || queried.out2 != 0 {
            fail(FAIL_EVENT_SUPERVISION);
        }
        let waited = object_wait_many_array(&items, nodes.len(), PRODUCT_HEALTH_CADENCE_NS);
        if waited.status != Status::Timeout.raw() || waited.out1 != u64::MAX || waited.out2 != 0 {
            fail(FAIL_EVENT_SUPERVISION);
        }
    }
}

fn command_and_expect(child: Child, command: u64, expected: u64) -> u64 {
    write_scalar(child.control, PRODUCT_COMMAND_TAG, command);
    let (sender, (tag, payload)) = read_scalar_envelope(child.control);
    let raw_node = payload >> 56;
    let value = payload & 0x00ff_ffff_ffff_ffff;
    let node = child.node.unwrap_or_else(|| fail(FAIL_CONTROL));
    if sender != child.pid
        || tag != PRODUCT_ACK_TAG
        || raw_node != node.raw()
        || (expected != u64::MAX && value != expected)
    {
        fail(FAIL_CONTROL);
    }
    value
}

fn acknowledge(node: ShutdownServiceNode, value: u64) {
    if value & 0xff00_0000_0000_0000 != 0 {
        fail(FAIL_CONTROL);
    }
    write_scalar(
        CONTROL_HANDLE.load(Ordering::Acquire),
        PRODUCT_ACK_TAG,
        (node.raw() << 56) | value,
    );
}

fn armed_node() -> ShutdownServiceNode {
    let stored = NODE_PLUS_ONE.load(Ordering::Acquire);
    ShutdownServiceNode::from_raw(stored.wrapping_sub(1)).unwrap_or_else(|| fail(FAIL_CONTROL))
}

fn validate_node_table(nodes: &[Child; 8]) {
    for (index, child) in nodes.iter().enumerate() {
        let node = child.node.unwrap_or_else(|| fail(FAIL_CONTROL));
        if node.raw() as usize != index
            || child.control == 0
            || child.pid == 0
            || nodes[index + 1..]
                .iter()
                .any(|other| other.control == child.control || other.pid == child.pid)
        {
            fail(FAIL_CONTROL);
        }
    }
}

#[cfg(feature = "unified-product-manifest-supervision-runtime")]
fn open_product_service_manifest() -> ServiceManifest {
    if PRODUCT_SERVICE_MANIFEST_SIZE > VMO_READ_MAX_BYTES {
        fail(FAIL_MANIFEST);
    }
    expect_status(
        syscall(
            SyscallNumber::ServiceManifestOpen,
            SERVICE_MANIFEST_OPEN_FLAGS_NONE + 1,
            0,
            0,
        ),
        Status::InvalidArgument,
        FAIL_MANIFEST,
    );
    let opened = syscall(
        SyscallNumber::ServiceManifestOpen,
        SERVICE_MANIFEST_OPEN_FLAGS_NONE,
        0,
        0,
    );
    if opened.status != Status::Ok.raw()
        || opened.out1 == 0
        || opened.out2 != PRODUCT_SERVICE_MANIFEST_SIZE as u64
    {
        fail(FAIL_MANIFEST);
    }
    let mut wire = [0_u8; PRODUCT_SERVICE_MANIFEST_SIZE];
    let read = syscall(
        SyscallNumber::VmoRead,
        opened.out1,
        wire.as_mut_ptr() as u64,
        pack_vmo_read(0, PRODUCT_SERVICE_MANIFEST_SIZE as u32),
    );
    if read.status != Status::Ok.raw()
        || read.out1 != PRODUCT_SERVICE_MANIFEST_SIZE as u64
        || read.out2 != PRODUCT_SERVICE_MANIFEST_SIZE as u64
    {
        fail(FAIL_MANIFEST);
    }
    close_handle(opened.out1);
    let manifest = ServiceManifest::decode(&wire).unwrap_or_else(|_| fail(FAIL_MANIFEST));
    validate_product_manifest_profile(&manifest);
    manifest
}

#[cfg(feature = "unified-product-manifest-supervision-runtime")]
fn validate_product_manifest_profile(manifest: &ServiceManifest) {
    #[cfg(feature = "unified-product-key-rotation-runtime")]
    let generation_matches = matches!(
        manifest.generation(),
        KEY_ROTATION_TRANSITION_SERVICE_MANIFEST_GENERATION
            | KEY_ROTATION_SERVICE_MANIFEST_GENERATION
            | KEY_ROTATION_RETIRED_KEY_FIXTURE_GENERATION
    );
    #[cfg(all(
        feature = "unified-product-persistent-rollback-runtime",
        not(feature = "unified-product-key-rotation-runtime")
    ))]
    let expected_generation = PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION;
    #[cfg(all(
        feature = "unified-product-verified-manifest-runtime",
        not(feature = "unified-product-persistent-rollback-runtime")
    ))]
    let expected_generation = VERIFIED_SERVICE_MANIFEST_GENERATION;
    #[cfg(not(feature = "unified-product-verified-manifest-runtime"))]
    let expected_generation = SERVICE_MANIFEST_GENERATION;
    #[cfg(not(feature = "unified-product-key-rotation-runtime"))]
    let generation_matches = manifest.generation() == expected_generation;
    if !generation_matches
        || manifest.wire_size() != PRODUCT_SERVICE_MANIFEST_SIZE
        || manifest.fingerprint() == 0
        || manifest.service_count() != PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT
        || manifest.dependency_count() != PRODUCT_SERVICE_MANIFEST_DEPENDENCY_COUNT
    {
        fail(FAIL_MANIFEST);
    }
    for kind in [
        ServiceKind::ServiceManager,
        ServiceKind::SurfaceServer,
        ServiceKind::InputServer,
        ServiceKind::StorageServer,
        ServiceKind::App,
    ] {
        let service = manifest
            .service(kind)
            .unwrap_or_else(|| fail(FAIL_MANIFEST));
        let expected_tolerance = u32::from(kind != ServiceKind::StorageServer);
        if service.policy().restart_budget() != 1
            || service.policy().restart_backoff_ns() != STORAGE_RESTART_BACKOFF_NS
            || service.policy().health_timeout_ns() != STORAGE_HEALTH_TIMEOUT_NS
            || service.policy().missed_probe_tolerance() != expected_tolerance
        {
            fail(FAIL_MANIFEST);
        }
    }
    let mut resident = 0;
    let mut spawned = 0;
    for service in manifest.services() {
        match service.launch_mode() {
            ServiceLaunchMode::BindResident if service.shutdown_node().is_some() => resident += 1,
            ServiceLaunchMode::Spawn if service.shutdown_node().is_none() => spawned += 1,
            _ => fail(FAIL_MANIFEST),
        }
    }
    if resident != 4 || spawned != 1 {
        fail(FAIL_MANIFEST);
    }
    for (prerequisite, dependent, kind) in [
        (
            ServiceKind::ServiceManager,
            ServiceKind::SurfaceServer,
            DependencyKind::Hard,
        ),
        (
            ServiceKind::SurfaceServer,
            ServiceKind::InputServer,
            DependencyKind::Hard,
        ),
        (
            ServiceKind::InputServer,
            ServiceKind::App,
            DependencyKind::Soft,
        ),
        (
            ServiceKind::StorageServer,
            ServiceKind::App,
            DependencyKind::Hard,
        ),
    ] {
        if !manifest.dependencies().any(|dependency| {
            dependency.prerequisite() == prerequisite
                && dependency.dependent() == dependent
                && dependency.kind() == kind
        }) {
            fail(FAIL_MANIFEST);
        }
    }
}

fn spawn(image: UserImageId, node: Option<ShutdownServiceNode>) -> Child {
    let channel = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    if channel.status != Status::Ok.raw()
        || channel.out1 == 0
        || channel.out2 == 0
        || channel.out1 == channel.out2
    {
        fail(FAIL_CONTROL);
    }
    let spawned = syscall(
        SyscallNumber::ProcessSpawn,
        channel.out2,
        image.raw(),
        PROCESS_SPAWN_FLAGS_NONE,
    );
    if spawned.status != Status::Ok.raw() || spawned.out1 == 0 || spawned.out2 != 0 {
        fail(FAIL_CONTROL);
    }
    assert_stale_handle(channel.out2);
    Child {
        control: channel.out1,
        pid: spawned.out1,
        node,
    }
}

fn expect_server_generation(server: Child, expected_tag: u64) -> u64 {
    let (sender, (tag, generation)) = read_scalar_envelope(server.control);
    if sender != server.pid || tag != expected_tag || generation == 0 {
        fail(FAIL_STORAGE);
    }
    generation
}

#[cfg(all(
    feature = "unified-product-continuous-supervision-runtime",
    not(feature = "unified-product-manifest-supervision-runtime")
))]
fn build_literal_product_supervisor(
    first: Child,
    app: Child,
    nodes: &[Child; 8],
    policy: ServicePolicy,
    resilient_policy: ServicePolicy,
) -> (ProductSupervisor, SupervisedServices) {
    let manager = nodes[ShutdownServiceNode::ServiceManager.raw() as usize];
    let surface = nodes[ShutdownServiceNode::SurfaceServer.raw() as usize];
    let input = nodes[ShutdownServiceNode::InputServer.raw() as usize];
    let manager_identity = product_identity(ServiceKind::ServiceManager, manager.pid);
    let surface_identity = product_identity(ServiceKind::SurfaceServer, surface.pid);
    let input_identity = product_identity(ServiceKind::InputServer, input.pid);
    let first_identity = storage_identity(first.pid);
    let app_identity = product_identity(ServiceKind::App, app.pid);
    let services = [
        ServiceDefinition::new(manager_identity, resilient_policy),
        ServiceDefinition::new(surface_identity, resilient_policy),
        ServiceDefinition::new(input_identity, resilient_policy),
        ServiceDefinition::new(first_identity, policy),
        ServiceDefinition::new(app_identity, resilient_policy),
    ];
    let dependencies = [
        DependencyDefinition::new(
            ServiceKind::ServiceManager,
            ServiceKind::SurfaceServer,
            DependencyKind::Hard,
        ),
        DependencyDefinition::new(
            ServiceKind::SurfaceServer,
            ServiceKind::InputServer,
            DependencyKind::Hard,
        ),
        DependencyDefinition::new(
            ServiceKind::InputServer,
            ServiceKind::App,
            DependencyKind::Soft,
        ),
        DependencyDefinition::new(
            ServiceKind::StorageServer,
            ServiceKind::App,
            DependencyKind::Hard,
        ),
    ];
    let supervisor = ServiceSupervisor::<5, 4>::from_catalog(&services, &dependencies)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    let mut runtime = SupervisedServices::new();
    for (identity, child) in [
        (manager_identity, manager),
        (surface_identity, surface),
        (input_identity, input),
        (first_identity, first),
        (app_identity, app),
    ] {
        runtime.push(SupervisedService { identity, child });
    }
    (supervisor, runtime)
}

#[cfg(feature = "unified-product-manifest-supervision-runtime")]
fn build_manifest_product_supervisor(
    manifest: &ServiceManifest,
    storage: Child,
    nodes: &[Child; 8],
) -> (ProductSupervisor, SupervisedServices) {
    let mut definitions = [None; SERVICE_MANIFEST_MAX_SERVICES];
    let mut runtime = SupervisedServices::new();
    for (index, service) in manifest.services().enumerate() {
        let child = resolve_manifest_service(service, storage, nodes);
        let identity = product_identity(service.kind(), child.pid);
        definitions[index] = Some(ServiceDefinition::new(identity, service.policy()));
        runtime.push(SupervisedService { identity, child });
    }
    let mut dependencies = [None; SERVICE_MANIFEST_MAX_DEPENDENCIES];
    for (index, dependency) in manifest.dependencies().enumerate() {
        dependencies[index] = Some(DependencyDefinition::new(
            dependency.prerequisite(),
            dependency.dependent(),
            dependency.kind(),
        ));
    }
    let supervisor = ProductSupervisor::from_catalog_iter(
        definitions.into_iter().flatten(),
        dependencies.into_iter().flatten(),
    )
    .unwrap_or_else(|_| fail(FAIL_MANIFEST));
    if supervisor.len() != manifest.service_count()
        || supervisor.dependency_count() != manifest.dependency_count()
        || runtime.len() != manifest.service_count()
    {
        fail(FAIL_MANIFEST);
    }
    (supervisor, runtime)
}

#[cfg(feature = "unified-product-manifest-supervision-runtime")]
fn resolve_manifest_service(service: ManifestService, storage: Child, nodes: &[Child; 8]) -> Child {
    let child = match service.launch_mode() {
        ServiceLaunchMode::BindResident => {
            let node = service
                .shutdown_node()
                .unwrap_or_else(|| fail(FAIL_MANIFEST));
            nodes[node.raw() as usize]
        }
        ServiceLaunchMode::Spawn => {
            if service.kind() != ServiceKind::StorageServer
                || service.shutdown_node().is_some()
                || service.image() != UserImageId::StorageServer
            {
                fail(FAIL_MANIFEST);
            }
            storage
        }
    };
    if child.control == 0
        || child.pid == 0
        || child.node != service.shutdown_node()
        || service
            .shutdown_node()
            .is_some_and(|node| node.image_id() != service.image())
    {
        fail(FAIL_MANIFEST);
    }
    child
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProductEventAction {
    Rotate { ordinal: u64, sequence: u64 },
    Power { sequence: u64 },
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn run_event_supervision(
    first: Child,
    mounted_generation: u64,
    nodes: &[Child; 8],
    manifest: &ServiceManifest,
) -> (Child, u64) {
    let (mut supervisor, mut services) = build_manifest_product_supervisor(manifest, first, nodes);
    if services.len() != PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT
        || services.require(ServiceKind::StorageServer).child != first
    {
        fail(FAIL_EVENT_SUPERVISION);
    }

    #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
    {
        expect_status(
            syscall(
                SyscallNumber::ServiceSupervisorReport,
                SERVICE_SUPERVISOR_REPORT_ACTIVE,
                1,
                services.len() as u64,
            ),
            Status::PermissionDenied,
            FAIL_EVENT_SUPERVISION,
        );
        expect_status(
            syscall(
                SyscallNumber::MaintenanceSessionOpen,
                MAINTENANCE_SESSION_OPEN_FLAGS_NONE + 1,
                0,
                0,
            ),
            Status::InvalidArgument,
            FAIL_EVENT_SUPERVISION,
        );
        let opened = syscall(
            SyscallNumber::MaintenanceSessionOpen,
            MAINTENANCE_SESSION_OPEN_FLAGS_NONE,
            0,
            0,
        );
        if opened.status != Status::Ok.raw()
            || opened.out1 == 0
            || opened.out2 != MAINTENANCE_OPERATION_STORAGE_ROTATION
        {
            fail(FAIL_EVENT_SUPERVISION);
        }
        expect_status(
            syscall(
                SyscallNumber::MaintenanceSessionOpen,
                MAINTENANCE_SESSION_OPEN_FLAGS_NONE,
                0,
                0,
            ),
            Status::InvalidState,
            FAIL_EVENT_SUPERVISION,
        );
    }

    expect_status(
        syscall(
            SyscallNumber::ServiceSupervisorReport,
            SERVICE_SUPERVISOR_REPORT_ACTIVE,
            0,
            0,
        ),
        Status::InvalidArgument,
        FAIL_EVENT_SUPERVISION,
    );

    let mut now_ns = 0_u64;
    let mut batch_count = 0_u64;
    for ordinal in 1..=3 {
        batch_count = batch_count
            .checked_add(1)
            .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
        if m73_run_healthy_batch(&mut supervisor, &services, now_ns).is_some() {
            fail(FAIL_EVENT_SUPERVISION);
        }
        if ordinal != 3 {
            if m73_wait_for_action(&services, PRODUCT_HEALTH_CADENCE_NS).is_some() {
                fail(FAIL_EVENT_SUPERVISION);
            }
            now_ns = now_ns
                .checked_add(PRODUCT_HEALTH_CADENCE_NS)
                .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
        }
    }
    m73_report(
        SERVICE_SUPERVISOR_REPORT_ACTIVE,
        batch_count,
        services.len() as u64,
    );

    let rotation_context = M73RotationContext {
        mounted_generation,
        app: nodes[ShutdownServiceNode::App.raw() as usize],
        manifest,
    };
    let mut storage = first;
    for expected_ordinal in 1..=2 {
        let action = loop {
            if let Some(action) = m73_wait_for_action(&services, PRODUCT_HEALTH_CADENCE_NS) {
                break action;
            }
            now_ns = now_ns
                .checked_add(PRODUCT_HEALTH_CADENCE_NS)
                .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
            batch_count = batch_count
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
            if let Some(action) = m73_run_healthy_batch(&mut supervisor, &services, now_ns) {
                break action;
            }
        };
        let ProductEventAction::Rotate { ordinal, sequence } = action else {
            fail(FAIL_EVENT_SUPERVISION);
        };
        if ordinal != expected_ordinal || sequence == 0 {
            fail(FAIL_EVENT_SUPERVISION);
        }
        storage = m73_rotate_storage(
            &mut supervisor,
            &mut services,
            storage,
            rotation_context,
            ordinal,
            &mut now_ns,
        );
    }

    // The evidence gate deliberately consumes InputServer's Healthy response
    // first. The remaining four already-issued probes stay in flight while
    // the authenticated power event cancels the open-ended supervision loop.
    now_ns = now_ns
        .checked_add(PRODUCT_HEALTH_CADENCE_NS)
        .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
    batch_count = batch_count
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
    let cancel_batch = m73_arm_and_send_batch(&mut supervisor, &services, now_ns);
    let input_index = services
        .iter()
        .position(|service| service.identity.kind() == ServiceKind::InputServer)
        .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
    let input = services.get(input_index);
    let input_probe = cancel_batch
        .get(input_index)
        .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
    record_product_healthy_response(&mut supervisor, input_probe, input.child);

    let pending = (services.len() - 1) as u64;
    m73_report(
        SERVICE_SUPERVISOR_REPORT_CANCEL_WINDOW,
        batch_count,
        pending,
    );
    let ProductEventAction::Power { sequence } =
        m73_wait_for_input_action(input.child, OBJECT_WAIT_TIMEOUT_INFINITE)
    else {
        fail(FAIL_EVENT_SUPERVISION);
    };
    if sequence == 0 {
        fail(FAIL_EVENT_SUPERVISION);
    }
    m73_report(
        SERVICE_SUPERVISOR_REPORT_STOP_REQUESTED,
        batch_count,
        pending,
    );

    for index in 0..services.len() {
        if index == input_index {
            continue;
        }
        let service = services.get(index);
        let probe = cancel_batch
            .get(index)
            .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
        record_product_healthy_response(&mut supervisor, probe, service.child);
    }
    m73_report(SERVICE_SUPERVISOR_REPORT_STOP_DRAINED, batch_count, 0);
    m73_validate_final(&supervisor, &services, storage);
    (storage, mounted_generation)
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
#[derive(Clone, Copy)]
struct M73RotationContext<'a> {
    mounted_generation: u64,
    app: Child,
    manifest: &'a ServiceManifest,
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn m73_rotate_storage(
    supervisor: &mut ProductSupervisor,
    services: &mut SupervisedServices,
    previous: Child,
    context: M73RotationContext<'_>,
    ordinal: u64,
    now_ns: &mut u64,
) -> Child {
    let previous_identity = services.require(ServiceKind::StorageServer).identity;
    if previous_identity != storage_identity(previous.pid) {
        fail(FAIL_EVENT_SUPERVISION);
    }
    command_and_expect(context.app, PRODUCT_COMMAND_DEPENDENCY_BLOCK, 1);
    m73_report(
        SERVICE_SUPERVISOR_REPORT_ROTATION_BEGIN,
        ordinal,
        previous.pid,
    );

    write_scalar(
        previous.control,
        SERVER_SHUTDOWN_COMMAND_TAG,
        context.mounted_generation,
    );
    let (sender, (tag, acknowledged_generation)) = read_scalar_envelope(previous.control);
    if sender != previous.pid
        || tag != SERVER_SHUTDOWN_ACK_TAG
        || acknowledged_generation != context.mounted_generation
    {
        fail(FAIL_EVENT_SUPERVISION);
    }
    wait_for_exit(previous, SERVER_SHUTDOWN_EXIT_CODE);

    let disposition = supervisor
        .classify_fault(previous_identity, FaultClass::ProcessExit)
        .unwrap_or_else(|_| fail(FAIL_EVENT_SUPERVISION));
    if disposition
        != (FaultDisposition::RestartAllowed {
            attempt: 1,
            budget: 1,
            class: FaultClass::ProcessExit,
        })
    {
        fail(FAIL_EVENT_SUPERVISION);
    }
    let blocked = supervisor
        .dependency_fault(previous_identity)
        .unwrap_or_else(|_| fail(FAIL_EVENT_SUPERVISION));
    require_dependency_transition(
        &blocked,
        0,
        previous_identity,
        DependencyImpact::Unaffected,
        DependencyImpact::HardBlocked,
    );
    require_dependency_transition(
        &blocked,
        1,
        product_identity(ServiceKind::App, context.app.pid),
        DependencyImpact::Unaffected,
        DependencyImpact::HardBlocked,
    );
    if blocked.len() != 2 {
        fail(FAIL_EVENT_SUPERVISION);
    }

    let backoff_deadline = supervisor
        .begin_backoff(previous_identity, *now_ns)
        .unwrap_or_else(|_| fail(FAIL_EVENT_SUPERVISION));
    if backoff_deadline
        != now_ns
            .checked_add(STORAGE_RESTART_BACKOFF_NS)
            .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION))
    {
        fail(FAIL_EVENT_SUPERVISION);
    }
    require_channel_timeout(context.app.control, STORAGE_RESTART_BACKOFF_NS);
    *now_ns = backoff_deadline;
    supervisor
        .begin_replacement(previous_identity, *now_ns)
        .unwrap_or_else(|_| fail(FAIL_EVENT_SUPERVISION));

    let storage_image = context
        .manifest
        .service(ServiceKind::StorageServer)
        .filter(|service| {
            service.launch_mode() == ServiceLaunchMode::Spawn && service.shutdown_node().is_none()
        })
        .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION))
        .image();
    let replacement = spawn(storage_image, None);
    let replacement_mounted_generation = expect_server_generation(replacement, SERVER_READY_TAG);
    require_next_process_generation(previous.pid, replacement.pid);
    if replacement_mounted_generation != context.mounted_generation {
        fail(FAIL_EVENT_SUPERVISION);
    }
    let replacement_identity = supervisor
        .install_replacement(
            previous_identity,
            process_generation(replacement.pid),
            replacement.pid,
        )
        .unwrap_or_else(|_| fail(FAIL_EVENT_SUPERVISION));
    services.replace(
        ServiceKind::StorageServer,
        SupervisedService {
            identity: replacement_identity,
            child: replacement,
        },
    );

    require_product_healthy(supervisor, replacement_identity, replacement, *now_ns, 1);
    let resumed = supervisor
        .dependency_recovered(replacement_identity)
        .unwrap_or_else(|_| fail(FAIL_EVENT_SUPERVISION));
    require_dependency_transition(
        &resumed,
        0,
        replacement_identity,
        DependencyImpact::HardBlocked,
        DependencyImpact::Unaffected,
    );
    require_dependency_transition(
        &resumed,
        1,
        product_identity(ServiceKind::App, context.app.pid),
        DependencyImpact::HardBlocked,
        DependencyImpact::Unaffected,
    );
    if resumed.len() != 2 {
        fail(FAIL_EVENT_SUPERVISION);
    }
    command_and_expect(context.app, PRODUCT_COMMAND_DEPENDENCY_RESUME, 2);

    require_channel_timeout(context.app.control, PRODUCT_HEALTH_CADENCE_NS);
    *now_ns = now_ns
        .checked_add(PRODUCT_HEALTH_CADENCE_NS)
        .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
    require_product_healthy(supervisor, replacement_identity, replacement, *now_ns, 2);
    supervisor
        .rearm_restart_budget(replacement_identity)
        .unwrap_or_else(|_| fail(FAIL_EVENT_SUPERVISION));
    let snapshot = supervisor
        .service(ServiceKind::StorageServer)
        .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
    if snapshot.identity != replacement_identity
        || snapshot.phase != ServicePhase::Healthy
        || snapshot.restarts_used != 0
        || snapshot.healthy_streak != 2
        || snapshot.restart_budget_rearms != ordinal as u32
        || snapshot.last_fault != Some(FaultClass::ProcessExit)
        || snapshot.total_missed_probes != 0
    {
        fail(FAIL_EVENT_SUPERVISION);
    }
    m73_report(
        SERVICE_SUPERVISOR_REPORT_ROTATION_COMPLETE,
        ordinal,
        replacement.pid,
    );
    replacement
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn m73_run_healthy_batch(
    supervisor: &mut ProductSupervisor,
    services: &SupervisedServices,
    now_ns: u64,
) -> Option<ProductEventAction> {
    let batch = m73_arm_and_send_batch(supervisor, services, now_ns);
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    for (index, service) in services.iter().enumerate() {
        items[index] = pack_user_wait_item(service.child.control, requested);
    }
    let mut received = [false; SERVICE_MANIFEST_MAX_SERVICES];
    let mut pending = services.len();
    let mut action = None;
    while pending != 0 {
        let ready = object_wait_many_array(&items, services.len(), OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw()
            || ready.out1 >= services.len() as u64
            || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
            || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
        {
            fail(FAIL_EVENT_SUPERVISION);
        }
        let index = usize::try_from(ready.out1).unwrap_or_else(|_| fail(FAIL_EVENT_SUPERVISION));
        let service = services.get(index);
        let envelope = read_channel_envelope_now(service.child.control);
        if envelope.kind() == ChannelMessageKind::Bytes {
            if received[index] {
                fail(FAIL_EVENT_SUPERVISION);
            }
            let probe = batch
                .get(index)
                .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
            record_product_healthy_envelope(supervisor, probe, service.child, &envelope);
            received[index] = true;
            pending -= 1;
        } else {
            let observed = m73_decode_action(service, &envelope);
            if action.replace(observed).is_some() {
                fail(FAIL_EVENT_SUPERVISION);
            }
        }
    }
    action
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn m73_arm_and_send_batch(
    supervisor: &mut ProductSupervisor,
    services: &SupervisedServices,
    now_ns: u64,
) -> bndr_sm::health::ProbeBatch<SERVICE_MANIFEST_MAX_SERVICES> {
    let batch = supervisor
        .arm_probe_batch(now_ns)
        .unwrap_or_else(|_| fail(FAIL_EVENT_SUPERVISION));
    if batch.len() != services.len() {
        fail(FAIL_EVENT_SUPERVISION);
    }
    for index in 0..services.len() {
        let service = services.get(index);
        let probe = batch
            .get(index)
            .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
        if probe.identity() != service.identity
            || probe.opcode() != HealthOpcode::Probe
            || probe.interval_ns() != STORAGE_HEALTH_TIMEOUT_NS
            || !probe.flags().ack_required()
        {
            fail(FAIL_EVENT_SUPERVISION);
        }
        write_health(service.child.control, probe);
    }
    batch
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn m73_wait_for_action(
    services: &SupervisedServices,
    timeout_ns: u64,
) -> Option<ProductEventAction> {
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    for (index, service) in services.iter().enumerate() {
        items[index] = pack_user_wait_item(service.child.control, requested);
    }
    let ready = object_wait_many_array(&items, services.len(), timeout_ns);
    if ready.status == Status::Timeout.raw() && ready.out1 == u64::MAX && ready.out2 == 0 {
        return None;
    }
    if ready.status != Status::Ok.raw()
        || ready.out1 >= services.len() as u64
        || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
        || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
    {
        fail(FAIL_EVENT_SUPERVISION);
    }
    let index = usize::try_from(ready.out1).unwrap_or_else(|_| fail(FAIL_EVENT_SUPERVISION));
    let service = services.get(index);
    let envelope = read_channel_envelope_now(service.child.control);
    Some(m73_decode_action(service, &envelope))
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn m73_wait_for_input_action(input: Child, timeout_ns: u64) -> ProductEventAction {
    require_channel_ready(input.control, timeout_ns);
    let envelope = read_channel_envelope_now(input.control);
    m73_decode_action(
        SupervisedService {
            identity: product_identity(ServiceKind::InputServer, input.pid),
            child: input,
        },
        &envelope,
    )
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn m73_decode_action(
    service: SupervisedService,
    envelope: &ChannelReadEnvelope,
) -> ProductEventAction {
    let Some((tag, payload)) = envelope.scalar_values() else {
        fail(FAIL_EVENT_SUPERVISION);
    };
    if service.identity.kind() != ServiceKind::InputServer
        || envelope.kind() != ChannelMessageKind::Scalar
        || envelope.sender_pid() != service.child.pid
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_EVENT_SUPERVISION);
    }
    match tag {
        PRODUCT_ROTATION_TAG => {
            let ordinal = payload >> 56;
            let sequence = payload & 0x00ff_ffff_ffff_ffff;
            if !(1..=2).contains(&ordinal) || sequence == 0 {
                fail(FAIL_EVENT_SUPERVISION);
            }
            ProductEventAction::Rotate { ordinal, sequence }
        }
        PRODUCT_POWER_TAG if payload != 0 => ProductEventAction::Power { sequence: payload },
        _ => fail(FAIL_EVENT_SUPERVISION),
    }
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn m73_report(operation: u64, argument1: u64, argument2: u64) {
    let reported = syscall(
        SyscallNumber::ServiceSupervisorReport,
        operation,
        argument1,
        argument2,
    );
    if reported.status != Status::Ok.raw() || reported.out1 != 0 || reported.out2 != 0 {
        fail(FAIL_EVENT_SUPERVISION);
    }
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn m73_validate_final(
    supervisor: &ProductSupervisor,
    services: &SupervisedServices,
    storage: Child,
) {
    if supervisor.len() != PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT
        || supervisor.dependency_count() != PRODUCT_SERVICE_MANIFEST_DEPENDENCY_COUNT
        || services.len() != PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT
    {
        fail(FAIL_EVENT_SUPERVISION);
    }
    for service in services.iter() {
        let snapshot = supervisor
            .service(service.identity.kind())
            .unwrap_or_else(|| fail(FAIL_EVENT_SUPERVISION));
        if snapshot.identity != service.identity
            || snapshot.phase != ServicePhase::Healthy
            || snapshot.restarts_used != 0
            || snapshot.probe_deadline_ns.is_some()
            || snapshot.backoff_deadline_ns.is_some()
            || snapshot.consecutive_missed_probes != 0
            || snapshot.total_missed_probes != 0
            || supervisor
                .dependency_impact(service.identity.kind())
                .unwrap_or_else(|_| fail(FAIL_EVENT_SUPERVISION))
                != DependencyImpact::Unaffected
        {
            fail(FAIL_EVENT_SUPERVISION);
        }
        if service.identity.kind() == ServiceKind::StorageServer {
            if service.child != storage
                || snapshot.restart_budget_rearms != 2
                || snapshot.healthy_streak < 2
                || snapshot.last_fault != Some(FaultClass::ProcessExit)
                || snapshot.last_outbound_sequence < 3
                || snapshot.last_inbound_sequence != snapshot.last_outbound_sequence
            {
                fail(FAIL_EVENT_SUPERVISION);
            }
        } else if snapshot.restart_budget_rearms != 0 || snapshot.last_fault.is_some() {
            fail(FAIL_EVENT_SUPERVISION);
        }
    }
}

#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
fn recover_product_services(
    first: Child,
    mounted_generation: u64,
    app: Child,
    nodes: &[Child; 8],
    #[cfg(feature = "unified-product-manifest-supervision-runtime")] manifest: &ServiceManifest,
) -> (Child, u64) {
    let first_identity = storage_identity(first.pid);
    let app_identity = product_identity(ServiceKind::App, app.pid);
    #[cfg(feature = "unified-product-manifest-supervision-runtime")]
    let policy = manifest
        .service(ServiceKind::StorageServer)
        .unwrap_or_else(|| fail(FAIL_MANIFEST))
        .policy();
    #[cfg(not(feature = "unified-product-manifest-supervision-runtime"))]
    let policy = ServicePolicy::new(1, STORAGE_RESTART_BACKOFF_NS, STORAGE_HEALTH_TIMEOUT_NS)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    #[cfg(feature = "unified-product-manifest-supervision-runtime")]
    let storage_image = manifest
        .service(ServiceKind::StorageServer)
        .unwrap_or_else(|| fail(FAIL_MANIFEST))
        .image();
    #[cfg(not(feature = "unified-product-manifest-supervision-runtime"))]
    let storage_image = UserImageId::StorageServer;
    #[cfg(feature = "unified-product-manifest-supervision-runtime")]
    let resilient_policy = manifest
        .service(ServiceKind::App)
        .unwrap_or_else(|| fail(FAIL_MANIFEST))
        .policy();
    #[cfg(all(
        feature = "unified-product-continuous-supervision-runtime",
        not(feature = "unified-product-manifest-supervision-runtime")
    ))]
    let resilient_policy = policy
        .with_missed_probe_tolerance(1)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    #[cfg(feature = "unified-product-manifest-supervision-runtime")]
    let (mut supervisor, mut supervised_services) =
        build_manifest_product_supervisor(manifest, first, nodes);
    #[cfg(all(
        feature = "unified-product-continuous-supervision-runtime",
        not(feature = "unified-product-manifest-supervision-runtime")
    ))]
    let (mut supervisor, mut supervised_services) =
        build_literal_product_supervisor(first, app, nodes, policy, resilient_policy);
    #[cfg(not(feature = "unified-product-continuous-supervision-runtime"))]
    let mut supervisor = ServiceSupervisor::<2, 1>::new();
    #[cfg(not(feature = "unified-product-continuous-supervision-runtime"))]
    supervisor
        .register(first_identity, policy)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    #[cfg(not(feature = "unified-product-continuous-supervision-runtime"))]
    supervisor
        .register(app_identity, policy)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    #[cfg(not(feature = "unified-product-continuous-supervision-runtime"))]
    supervisor
        .add_dependency(
            ServiceKind::StorageServer,
            ServiceKind::App,
            DependencyKind::Hard,
        )
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    #[cfg(not(feature = "unified-product-continuous-supervision-runtime"))]
    let _ = nodes;

    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    let manager_service = supervised_services.require(ServiceKind::ServiceManager);
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    let manager = manager_service.child;
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    let manager_identity = manager_service.identity;
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    let surface_service = supervised_services.require(ServiceKind::SurfaceServer);
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    let surface = surface_service.child;
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    let surface_identity = surface_service.identity;
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    let input_service = supervised_services.require(ServiceKind::InputServer);
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    let input = input_service.child;
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    let input_identity = input_service.identity;

    // Round 1 proves that every registered service speaks BSH1.
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    require_product_healthy(&mut supervisor, manager_identity, manager, 0, 1);
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    require_product_healthy(&mut supervisor, surface_identity, surface, 0, 1);
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    require_product_healthy(&mut supervisor, input_identity, input, 0, 1);
    require_product_healthy(&mut supervisor, first_identity, first, 0, 1);
    require_product_healthy(&mut supervisor, app_identity, app, 0, 1);
    require_channel_timeout(app.control, PRODUCT_HEALTH_CADENCE_NS);

    // Round 2 is separated by a real finite cadence wait and is entirely
    // healthy: it is not a fault-injection setup message.
    let mut now_ns = PRODUCT_HEALTH_CADENCE_NS;
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    require_product_healthy(&mut supervisor, manager_identity, manager, now_ns, 2);
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    require_product_healthy(&mut supervisor, surface_identity, surface, now_ns, 2);
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    require_product_healthy(&mut supervisor, input_identity, input, now_ns, 2);
    require_product_healthy(&mut supervisor, first_identity, first, now_ns, 2);
    require_product_healthy(&mut supervisor, app_identity, app, now_ns, 2);
    require_channel_timeout(app.control, PRODUCT_HEALTH_CADENCE_NS);
    now_ns = now_ns
        .checked_add(PRODUCT_HEALTH_CADENCE_NS)
        .unwrap_or_else(|| fail(FAIL_LIVENESS));

    let withheld = supervisor
        .arm_probe(first_identity, now_ns)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    validate_product_probe(withheld, first_identity, 3);
    write_health(first.control, withheld);
    require_channel_timeout(first.control, STORAGE_HEALTH_TIMEOUT_NS);
    now_ns = now_ns
        .checked_add(STORAGE_HEALTH_TIMEOUT_NS)
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    let disposition = supervisor
        .check_health_timeout(first_identity, now_ns)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS))
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    if disposition
        != (FaultDisposition::RestartAllowed {
            attempt: 1,
            budget: 1,
            class: FaultClass::HealthTimeout,
        })
    {
        fail(FAIL_LIVENESS);
    }

    let blocked = supervisor
        .dependency_fault(first_identity)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    require_dependency_transition(
        &blocked,
        0,
        first_identity,
        DependencyImpact::Unaffected,
        DependencyImpact::HardBlocked,
    );
    require_dependency_transition(
        &blocked,
        1,
        app_identity,
        DependencyImpact::Unaffected,
        DependencyImpact::HardBlocked,
    );
    if blocked.len() != 2 {
        fail(FAIL_LIVENESS);
    }
    command_and_expect(app, PRODUCT_COMMAND_DEPENDENCY_BLOCK, 1);

    let backoff_deadline = supervisor
        .begin_backoff(first_identity, now_ns)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    if backoff_deadline
        != now_ns
            .checked_add(STORAGE_RESTART_BACKOFF_NS)
            .unwrap_or_else(|| fail(FAIL_LIVENESS))
    {
        fail(FAIL_LIVENESS);
    }
    require_channel_timeout(first.control, STORAGE_RESTART_BACKOFF_NS);
    now_ns = backoff_deadline;
    supervisor
        .begin_replacement(first_identity, now_ns)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));

    terminate_and_reap_storage(first);
    let replacement = spawn(storage_image, None);
    let replacement_mounted_generation = expect_server_generation(replacement, SERVER_READY_TAG);
    require_next_process_generation(first.pid, replacement.pid);
    if replacement_mounted_generation != mounted_generation {
        fail(FAIL_LIVENESS);
    }
    let replacement_identity = supervisor
        .install_replacement(
            first_identity,
            process_generation(replacement.pid),
            replacement.pid,
        )
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    if replacement_identity != storage_identity(replacement.pid) {
        fail(FAIL_LIVENESS);
    }
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    supervised_services.replace(
        ServiceKind::StorageServer,
        SupervisedService {
            identity: replacement_identity,
            child: replacement,
        },
    );
    require_product_healthy(
        &mut supervisor,
        replacement_identity,
        replacement,
        now_ns,
        1,
    );

    let resumed = supervisor
        .dependency_recovered(replacement_identity)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    require_dependency_transition(
        &resumed,
        0,
        replacement_identity,
        DependencyImpact::HardBlocked,
        DependencyImpact::Unaffected,
    );
    require_dependency_transition(
        &resumed,
        1,
        app_identity,
        DependencyImpact::HardBlocked,
        DependencyImpact::Unaffected,
    );
    if resumed.len() != 2 {
        fail(FAIL_LIVENESS);
    }
    command_and_expect(app, PRODUCT_COMMAND_DEPENDENCY_RESUME, 2);

    // A third real cadence boundary proves both the resumed dependent and the
    // replacement remain healthy before any AppData command is admitted.
    require_channel_timeout(app.control, PRODUCT_HEALTH_CADENCE_NS);
    now_ns = now_ns
        .checked_add(PRODUCT_HEALTH_CADENCE_NS)
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    require_product_healthy(&mut supervisor, app_identity, app, now_ns, 3);
    require_product_healthy(
        &mut supervisor,
        replacement_identity,
        replacement,
        now_ns,
        2,
    );
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    require_product_healthy(&mut supervisor, manager_identity, manager, now_ns, 3);
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    require_product_healthy(&mut supervisor, surface_identity, surface, now_ns, 3);
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    require_product_healthy(&mut supervisor, input_identity, input, now_ns, 3);

    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    {
        now_ns = run_continuous_supervision(&mut supervisor, &supervised_services, now_ns);
    }

    let final_storage = supervisor
        .service(ServiceKind::StorageServer)
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    let final_app = supervisor
        .service(ServiceKind::App)
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    let (
        expected_services,
        expected_dependencies,
        expected_storage_sequence,
        expected_app_sequence,
        expected_now_ns,
        expected_app_policy,
    ) = (5, 4, 20, 21, 1_070_000_000, resilient_policy);
    #[cfg(not(feature = "unified-product-continuous-supervision-runtime"))]
    let (
        expected_services,
        expected_dependencies,
        expected_storage_sequence,
        expected_app_sequence,
        expected_now_ns,
        expected_app_policy,
    ) = (2, 1, 2, 3, 250_000_000, policy);
    if supervisor.len() != expected_services
        || supervisor.dependency_count() != expected_dependencies
        || supervisor.dependency_kind(ServiceKind::StorageServer, ServiceKind::App)
            != Some(DependencyKind::Hard)
        || supervisor
            .dependency_impact(ServiceKind::StorageServer)
            .unwrap_or_else(|_| fail(FAIL_LIVENESS))
            != DependencyImpact::Unaffected
        || supervisor
            .dependency_impact(ServiceKind::App)
            .unwrap_or_else(|_| fail(FAIL_LIVENESS))
            != DependencyImpact::Unaffected
        || final_storage.identity != replacement_identity
        || final_storage.policy != policy
        || final_storage.phase != ServicePhase::Healthy
        || final_storage.restarts_used != 1
        || final_storage.probe_deadline_ns.is_some()
        || final_storage.backoff_deadline_ns.is_some()
        || final_storage.last_fault != Some(FaultClass::HealthTimeout)
        || final_storage.last_outbound_sequence != expected_storage_sequence
        || final_storage.last_inbound_sequence != expected_storage_sequence
        || final_storage.consecutive_missed_probes != 0
        || final_storage.total_missed_probes != 1
        || final_app.identity != app_identity
        || final_app.policy != expected_app_policy
        || final_app.phase != ServicePhase::Healthy
        || final_app.restarts_used != 0
        || final_app.probe_deadline_ns.is_some()
        || final_app.backoff_deadline_ns.is_some()
        || final_app.last_fault.is_some()
        || final_app.last_outbound_sequence != expected_app_sequence
        || final_app.last_inbound_sequence != expected_app_sequence
        || final_app.consecutive_missed_probes != 0
        || final_app.total_missed_probes != 0
        || now_ns != expected_now_ns
    {
        fail(FAIL_LIVENESS);
    }
    #[cfg(feature = "unified-product-continuous-supervision-runtime")]
    validate_continuous_supervision_final(
        &supervisor,
        manager_identity,
        surface_identity,
        input_identity,
        resilient_policy,
    );
    (replacement, replacement_mounted_generation)
}

#[cfg(feature = "unified-product-continuous-supervision-runtime")]
fn run_continuous_supervision(
    supervisor: &mut ProductSupervisor,
    services: &SupervisedServices,
    mut now_ns: u64,
) -> u64 {
    let cadence_transport = services.require(ServiceKind::App).child.control;
    for _ in 0..M71_HEALTHY_SOAK_ROUNDS {
        require_channel_timeout(cadence_transport, PRODUCT_HEALTH_CADENCE_NS);
        now_ns = now_ns
            .checked_add(PRODUCT_HEALTH_CADENCE_NS)
            .unwrap_or_else(|| fail(FAIL_LIVENESS));
        run_continuous_healthy_batch(supervisor, services, now_ns);
    }

    // One catalog-wide batch is consumed by every process at the same logical
    // instant. SurfaceServer and InputServer omit sequence 20; the other three
    // replies prove that the channels and processes remain independently live.
    require_channel_timeout(cadence_transport, PRODUCT_HEALTH_CADENCE_NS);
    now_ns = now_ns
        .checked_add(PRODUCT_HEALTH_CADENCE_NS)
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    let missed_batch = supervisor
        .arm_probe_batch(now_ns)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    if missed_batch.len() != services.len() {
        fail(FAIL_LIVENESS);
    }
    for (frame, service) in missed_batch.iter().zip(services.iter()) {
        if frame.identity() != service.identity {
            fail(FAIL_LIVENESS);
        }
        match service.identity.kind() {
            ServiceKind::SurfaceServer | ServiceKind::InputServer => {
                if frame.sequence() != M71_RESIDENT_MISS_SEQUENCE {
                    fail(FAIL_LIVENESS);
                }
            }
            ServiceKind::ServiceManager | ServiceKind::App => {
                if frame.sequence() != M71_RESIDENT_MISS_SEQUENCE {
                    fail(FAIL_LIVENESS);
                }
            }
            ServiceKind::StorageServer => {
                if frame.sequence() != M71_RESIDENT_MISS_SEQUENCE - 1 {
                    fail(FAIL_LIVENESS);
                }
            }
        }
        write_health(service.child.control, frame);
    }
    for (frame, service) in missed_batch.iter().zip(services.iter()) {
        if matches!(
            frame.identity().kind(),
            ServiceKind::SurfaceServer | ServiceKind::InputServer
        ) {
            continue;
        }
        record_product_healthy_response(supervisor, frame, service.child);
    }

    let surface = services.require(ServiceKind::SurfaceServer);
    let input = services.require(ServiceKind::InputServer);
    require_concurrent_health_timeout(
        surface.child.control,
        input.child.control,
        STORAGE_HEALTH_TIMEOUT_NS,
    );
    now_ns = now_ns
        .checked_add(STORAGE_HEALTH_TIMEOUT_NS)
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    for service in [surface, input] {
        if supervisor
            .check_health_deadline(service.identity, now_ns)
            .unwrap_or_else(|_| fail(FAIL_LIVENESS))
            != (HealthDeadlineOutcome::ToleratedMiss {
                consecutive: 1,
                tolerance: 1,
            })
        {
            fail(FAIL_LIVENESS);
        }
    }

    // The following real cadence round must restore both missed services and
    // reset their consecutive-miss counters while preserving total evidence.
    require_channel_timeout(cadence_transport, PRODUCT_HEALTH_CADENCE_NS);
    now_ns = now_ns
        .checked_add(PRODUCT_HEALTH_CADENCE_NS)
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    run_continuous_healthy_batch(supervisor, services, now_ns);
    now_ns
}

#[cfg(feature = "unified-product-continuous-supervision-runtime")]
fn run_continuous_healthy_batch(
    supervisor: &mut ProductSupervisor,
    services: &SupervisedServices,
    now_ns: u64,
) {
    let batch = supervisor
        .arm_probe_batch(now_ns)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    if batch.len() != services.len() {
        fail(FAIL_LIVENESS);
    }
    for (frame, service) in batch.iter().zip(services.iter()) {
        let snapshot = supervisor
            .service(service.identity.kind())
            .unwrap_or_else(|| fail(FAIL_LIVENESS));
        if frame.identity() != service.identity
            || frame.sequence() != snapshot.last_outbound_sequence
            || frame.opcode() != HealthOpcode::Probe
            || frame.interval_ns() != STORAGE_HEALTH_TIMEOUT_NS
        {
            fail(FAIL_LIVENESS);
        }
        write_health(service.child.control, frame);
    }
    for (frame, service) in batch.iter().zip(services.iter()) {
        record_product_healthy_response(supervisor, frame, service.child);
    }
}

#[cfg(feature = "unified-product-continuous-supervision-runtime")]
fn require_concurrent_health_timeout(first: u64, second: u64, timeout_ns: u64) {
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    items[0] = pack_user_wait_item(first, requested);
    items[1] = pack_user_wait_item(second, requested);
    let waited = object_wait_many_array(&items, 2, timeout_ns);
    if waited.status != Status::Timeout.raw() || waited.out1 != u64::MAX || waited.out2 != 0 {
        fail(FAIL_LIVENESS);
    }
}

#[cfg(feature = "unified-product-continuous-supervision-runtime")]
fn validate_continuous_supervision_final(
    supervisor: &ProductSupervisor,
    manager_identity: ServiceIdentity,
    surface_identity: ServiceIdentity,
    input_identity: ServiceIdentity,
    resilient_policy: ServicePolicy,
) {
    for (identity, missed) in [
        (manager_identity, 0),
        (surface_identity, 1),
        (input_identity, 1),
    ] {
        let snapshot = supervisor
            .service(identity.kind())
            .unwrap_or_else(|| fail(FAIL_LIVENESS));
        if snapshot.identity != identity
            || snapshot.policy != resilient_policy
            || snapshot.phase != ServicePhase::Healthy
            || snapshot.restarts_used != 0
            || snapshot.probe_deadline_ns.is_some()
            || snapshot.backoff_deadline_ns.is_some()
            || snapshot.last_fault.is_some()
            || snapshot.last_outbound_sequence != 21
            || snapshot.last_inbound_sequence != 21
            || snapshot.consecutive_missed_probes != 0
            || snapshot.total_missed_probes != missed
            || supervisor
                .dependency_impact(identity.kind())
                .unwrap_or_else(|_| fail(FAIL_LIVENESS))
                != DependencyImpact::Unaffected
        {
            fail(FAIL_LIVENESS);
        }
    }
    if supervisor.dependency_kind(ServiceKind::ServiceManager, ServiceKind::SurfaceServer)
        != Some(DependencyKind::Hard)
        || supervisor.dependency_kind(ServiceKind::SurfaceServer, ServiceKind::InputServer)
            != Some(DependencyKind::Hard)
        || supervisor.dependency_kind(ServiceKind::InputServer, ServiceKind::App)
            != Some(DependencyKind::Soft)
    {
        fail(FAIL_LIVENESS);
    }
}

#[cfg(all(
    feature = "unified-product-liveness-runtime",
    not(feature = "unified-product-multiservice-liveness-runtime")
))]
fn recover_storage_server(first: Child, mounted_generation: u64) -> (Child, u64) {
    let first_identity = storage_identity(first.pid);
    let policy = ServicePolicy::new(1, STORAGE_RESTART_BACKOFF_NS, STORAGE_HEALTH_TIMEOUT_NS)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    let mut supervisor = ServiceSupervisor::<1>::new();
    supervisor
        .register(first_identity, policy)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));

    require_product_healthy(&mut supervisor, first_identity, first, 0, 1);

    let withheld = supervisor
        .arm_probe(first_identity, 0)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    validate_product_probe(withheld, first_identity, 2);
    write_health(first.control, withheld);
    require_channel_timeout(first.control, STORAGE_HEALTH_TIMEOUT_NS);
    let disposition = supervisor
        .check_health_timeout(first_identity, STORAGE_HEALTH_TIMEOUT_NS)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS))
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    if disposition
        != (FaultDisposition::RestartAllowed {
            attempt: 1,
            budget: 1,
            class: FaultClass::HealthTimeout,
        })
    {
        fail(FAIL_LIVENESS);
    }

    let backoff_deadline = supervisor
        .begin_backoff(first_identity, STORAGE_HEALTH_TIMEOUT_NS)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    if backoff_deadline
        != STORAGE_HEALTH_TIMEOUT_NS
            .checked_add(STORAGE_RESTART_BACKOFF_NS)
            .unwrap_or_else(|| fail(FAIL_LIVENESS))
    {
        fail(FAIL_LIVENESS);
    }
    require_channel_timeout(first.control, STORAGE_RESTART_BACKOFF_NS);
    supervisor
        .begin_replacement(first_identity, backoff_deadline)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));

    terminate_and_reap_storage(first);
    let replacement = spawn(UserImageId::StorageServer, None);
    let replacement_mounted_generation = expect_server_generation(replacement, SERVER_READY_TAG);
    require_next_process_generation(first.pid, replacement.pid);
    if replacement_mounted_generation != mounted_generation {
        fail(FAIL_LIVENESS);
    }
    let replacement_identity = supervisor
        .install_replacement(
            first_identity,
            process_generation(replacement.pid),
            replacement.pid,
        )
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    if replacement_identity != storage_identity(replacement.pid) {
        fail(FAIL_LIVENESS);
    }
    require_product_healthy(&mut supervisor, replacement_identity, replacement, 0, 1);
    let final_service = supervisor
        .service(ServiceKind::StorageServer)
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    if final_service.identity != replacement_identity
        || final_service.policy != policy
        || final_service.phase != ServicePhase::Healthy
        || final_service.restarts_used != 1
        || final_service.probe_deadline_ns.is_some()
        || final_service.backoff_deadline_ns.is_some()
        || final_service.last_fault != Some(FaultClass::HealthTimeout)
        || final_service.last_outbound_sequence != 1
        || final_service.last_inbound_sequence != 1
    {
        fail(FAIL_LIVENESS);
    }
    (replacement, replacement_mounted_generation)
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn require_product_healthy<const CAPACITY: usize, const DEPENDENCY_CAPACITY: usize>(
    supervisor: &mut ServiceSupervisor<CAPACITY, DEPENDENCY_CAPACITY>,
    identity: ServiceIdentity,
    service: Child,
    now_ns: u64,
    expected_sequence: u64,
) {
    let probe = supervisor
        .arm_probe(identity, now_ns)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
    validate_product_probe(probe, identity, expected_sequence);
    write_health(service.control, probe);
    record_product_healthy_response(supervisor, probe, service);
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn record_product_healthy_response<const CAPACITY: usize, const DEPENDENCY_CAPACITY: usize>(
    supervisor: &mut ServiceSupervisor<CAPACITY, DEPENDENCY_CAPACITY>,
    probe: HealthFrame,
    service: Child,
) {
    require_channel_ready(service.control, STORAGE_HEALTH_TIMEOUT_NS);
    let envelope = read_channel_envelope_now(service.control);
    record_product_healthy_envelope(supervisor, probe, service, &envelope);
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn record_product_healthy_envelope<const CAPACITY: usize, const DEPENDENCY_CAPACITY: usize>(
    supervisor: &mut ServiceSupervisor<CAPACITY, DEPENDENCY_CAPACITY>,
    probe: HealthFrame,
    service: Child,
    envelope: &ChannelReadEnvelope,
) {
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != bndr_sm::health::HEALTH_FRAME_SIZE
        || envelope.sender_pid() != service.pid
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_LIVENESS);
    }
    let healthy = HealthFrame::decode(envelope.data()).unwrap_or_else(|_| fail(FAIL_LIVENESS));
    if healthy.opcode() != HealthOpcode::Healthy
        || healthy.sequence() != probe.sequence()
        || healthy.identity() != probe.identity()
        || healthy.fault_class().is_some()
    {
        fail(FAIL_LIVENESS);
    }
    supervisor
        .record_healthy(healthy)
        .unwrap_or_else(|_| fail(FAIL_LIVENESS));
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn validate_product_probe(probe: HealthFrame, identity: ServiceIdentity, sequence: u64) {
    if probe.opcode() != HealthOpcode::Probe
        || probe.sequence() != sequence
        || probe.identity() != identity
        || probe.interval_ns() != STORAGE_HEALTH_TIMEOUT_NS
        || !probe.flags().ack_required()
    {
        fail(FAIL_LIVENESS);
    }
}

#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
fn require_dependency_transition<const CAPACITY: usize>(
    transitions: &DependencyTransitions<CAPACITY>,
    index: usize,
    service: ServiceIdentity,
    previous: DependencyImpact,
    current: DependencyImpact,
) {
    let transition = transitions
        .get(index)
        .unwrap_or_else(|| fail(FAIL_LIVENESS));
    if transition.service != service
        || transition.previous != previous
        || transition.current != current
    {
        fail(FAIL_LIVENESS);
    }
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn write_health(transport: u64, frame: HealthFrame) {
    let wire = frame.encode();
    loop {
        let written = syscall(
            SyscallNumber::ChannelWriteBytes,
            transport,
            wire.as_ptr() as u64,
            wire.len() as u64,
        );
        if written.status == Status::Ok.raw() {
            if written.out1 != wire.len() as u64 || written.out2 != 0 {
                fail(FAIL_LIVENESS);
            }
            return;
        }
        if written.status != Status::ShouldWait.raw() || written.out1 != 0 || written.out2 != 0 {
            fail(FAIL_LIVENESS);
        }
        let requested = signal_union(ObjectSignals::WRITABLE, ObjectSignals::PEER_CLOSED);
        let waited = wait_one_channel(transport, requested, OBJECT_WAIT_TIMEOUT_INFINITE);
        if waited.status != Status::Ok.raw()
            || waited.out1 != 0
            || waited.out2 & u64::from(ObjectSignals::WRITABLE.bits()) == 0
            || waited.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
        {
            fail(FAIL_LIVENESS);
        }
    }
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn require_channel_ready(transport: u64, timeout_ns: u64) {
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut waited = wait_one_channel(transport, requested, timeout_ns);
    if waited.status == Status::Timeout.raw() && waited.out1 == u64::MAX && waited.out2 == 0 {
        waited = wait_one_channel(transport, requested, 0);
    }
    if waited.status != Status::Ok.raw()
        || waited.out1 != 0
        || waited.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
        || waited.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
    {
        fail(FAIL_LIVENESS);
    }
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn require_channel_timeout(transport: u64, timeout_ns: u64) {
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let waited = wait_one_channel(transport, requested, timeout_ns);
    if waited.status != Status::Timeout.raw() || waited.out1 != u64::MAX || waited.out2 != 0 {
        fail(FAIL_LIVENESS);
    }
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn wait_one_channel(
    transport: u64,
    requested: ObjectSignals,
    timeout_ns: u64,
) -> super::SyscallResult {
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    items[0] = pack_user_wait_item(transport, requested);
    object_wait_many_array(&items, 1, timeout_ns)
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn terminate_and_reap_storage(server: Child) {
    let terminated = syscall(
        SyscallNumber::ProcessTerminate,
        server.pid,
        PROCESS_TERMINATE_FLAGS_NONE,
        0,
    );
    if terminated.status != Status::Ok.raw() || terminated.out1 != 0 || terminated.out2 != 0 {
        fail(FAIL_LIVENESS);
    }
    let waited = syscall(SyscallNumber::ProcessWait, server.pid, 0, 0);
    if waited.status != Status::Ok.raw()
        || waited.out1 != PROCESS_KILLED_EXIT_CODE
        || waited.out2 != ProcessTerminationReason::Killed.raw()
    {
        fail(FAIL_LIVENESS);
    }
    close_handle(server.control);
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn storage_identity(pid: u64) -> ServiceIdentity {
    product_identity(ServiceKind::StorageServer, pid)
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn product_identity(kind: ServiceKind, pid: u64) -> ServiceIdentity {
    ServiceIdentity::new(kind, process_generation(pid), pid).unwrap_or_else(|_| fail(FAIL_LIVENESS))
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn require_next_process_generation(previous: u64, replacement: u64) {
    if process_slot(previous) == 0
        || process_slot(replacement) != process_slot(previous)
        || process_generation(replacement)
            != process_generation(previous)
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_LIVENESS))
    {
        fail(FAIL_LIVENESS);
    }
}

#[cfg(feature = "unified-product-liveness-runtime")]
const fn process_slot(pid: u64) -> u32 {
    pid as u32
}

#[cfg(feature = "unified-product-liveness-runtime")]
const fn process_generation(pid: u64) -> u32 {
    (pid >> 32) as u32
}

fn validate_storage_proof(before: u64, after: u64, launcher_stage: u64, app_stage: u64) {
    let Some(delta) = after.checked_sub(before) else {
        fail(FAIL_STORAGE);
    };
    let valid = launcher_stage == 2
        && match delta {
            4 => app_stage == 1,
            1 | 0 => app_stage == 2,
            _ => false,
        };
    if !valid {
        fail(FAIL_STORAGE);
    }
}

fn exercise_spawn_barrier() {
    let channel = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    if channel.status != Status::Ok.raw()
        || channel.out1 == 0
        || channel.out2 == 0
        || channel.out1 == channel.out2
    {
        fail(FAIL_CONTROL);
    }
    expect_status(
        syscall(
            SyscallNumber::ProcessSpawn,
            channel.out2,
            UserImageId::Launcher.raw(),
            PROCESS_SPAWN_FLAGS_NONE,
        ),
        Status::InvalidState,
        FAIL_PREPARE,
    );
    close_handle(channel.out1);
    close_handle(channel.out2);
}

fn wait_for_exit(child: Child, expected_exit_code: u64) {
    let waited = syscall(SyscallNumber::ProcessWait, child.pid, 0, 0);
    if waited.status != Status::Ok.raw()
        || waited.out1 != expected_exit_code
        || waited.out2 != ProcessTerminationReason::Exited.raw()
    {
        fail(FAIL_WAIT);
    }
    close_handle(child.control);
}

fn close_handle(raw: u64) {
    let handle = OwnedUserHandle::new(raw).unwrap_or_else(|| fail(FAIL_EXIT));
    close_owned(handle);
}

fn expect_status(result: super::SyscallResult, expected: Status, reason: u64) {
    if result.status != expected.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(reason);
    }
}

fn expect_abi() {
    let abi = syscall(SyscallNumber::AbiVersion, 0, 0, 0);
    if abi.status != Status::Ok.raw() || abi.out1 != bndr_abi::ABI_VERSION || abi.out2 != 0 {
        fail(FAIL_ABI);
    }
}
