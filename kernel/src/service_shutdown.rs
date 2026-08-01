//! M66's authenticated resident-service shutdown ledger.
//!
//! Every non-storage resident process registers its exact ABI node, image,
//! dependency mask, and generation-qualified PID while the global shutdown
//! gate is open. After init closes admission with `SystemShutdown::Prepare`,
//! nodes may quiesce only in reverse dependency order. The StorageServer keeps
//! its separate M55--M65 volume-capability and durable-close proof.

use core::sync::atomic::{AtomicU64, Ordering};

use bndr_abi::{
    SHUTDOWN_SERVICE_ALL_MASK, SHUTDOWN_SERVICE_NODE_COUNT, ShutdownServiceNode, UserImageId,
};

use crate::shutdown::Phase;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegisterError {
    InvalidIdentity,
    InvalidDependency,
    InvalidPhase,
    Duplicate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuiesceError {
    InvalidIdentity,
    InvalidPhase,
    NotRegistered,
    AlreadyQuiesced,
    DependentsLive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub calls: u64,
    pub register_calls: u64,
    pub registrations: u64,
    pub quiesce_calls: u64,
    pub quiesces: u64,
    pub permission_denied: u64,
    pub invalid_arguments: u64,
    pub identity_rejections: u64,
    pub phase_rejections: u64,
    pub order_rejections: u64,
    pub replays: u64,
    pub invariant_errors: u64,
    pub registered_mask: u64,
    pub quiesced_mask: u64,
    pub pids: [u64; SHUTDOWN_SERVICE_NODE_COUNT],
}

impl Snapshot {
    pub const fn complete(self) -> bool {
        self.registered_mask == SHUTDOWN_SERVICE_ALL_MASK
            && self.quiesced_mask == SHUTDOWN_SERVICE_ALL_MASK
    }
}

static CALLS: AtomicU64 = AtomicU64::new(0);
static REGISTER_CALLS: AtomicU64 = AtomicU64::new(0);
static REGISTRATIONS: AtomicU64 = AtomicU64::new(0);
static QUIESCE_CALLS: AtomicU64 = AtomicU64::new(0);
static QUIESCES: AtomicU64 = AtomicU64::new(0);
static PERMISSION_DENIED: AtomicU64 = AtomicU64::new(0);
static INVALID_ARGUMENTS: AtomicU64 = AtomicU64::new(0);
static IDENTITY_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static PHASE_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static ORDER_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static REPLAYS: AtomicU64 = AtomicU64::new(0);
static INVARIANT_ERRORS: AtomicU64 = AtomicU64::new(0);
static REGISTERED_MASK: AtomicU64 = AtomicU64::new(0);
static QUIESCED_MASK: AtomicU64 = AtomicU64::new(0);
static PIDS: [AtomicU64; SHUTDOWN_SERVICE_NODE_COUNT] =
    [const { AtomicU64::new(0) }; SHUTDOWN_SERVICE_NODE_COUNT];

pub const fn reduce_register(
    registered_mask: u64,
    node: ShutdownServiceNode,
    image: UserImageId,
    dependency_mask: u64,
    phase: Phase,
) -> Result<u64, RegisterError> {
    if image as u64 != node.image_id() as u64 {
        return Err(RegisterError::InvalidIdentity);
    }
    if dependency_mask != node.dependency_mask() {
        return Err(RegisterError::InvalidDependency);
    }
    if !matches!(phase, Phase::Open) {
        return Err(RegisterError::InvalidPhase);
    }
    if registered_mask & node.bit() != 0 {
        return Err(RegisterError::Duplicate);
    }
    Ok(registered_mask | node.bit())
}

pub const fn reduce_quiesce(
    registered_mask: u64,
    quiesced_mask: u64,
    node: ShutdownServiceNode,
    phase: Phase,
) -> Result<u64, QuiesceError> {
    if !matches!(phase, Phase::Quiescing) {
        return Err(QuiesceError::InvalidPhase);
    }
    if registered_mask & node.bit() == 0 {
        return Err(QuiesceError::NotRegistered);
    }
    if quiesced_mask & node.bit() != 0 {
        return Err(QuiesceError::AlreadyQuiesced);
    }
    if node.dependent_mask() & !quiesced_mask != 0 {
        return Err(QuiesceError::DependentsLive);
    }
    Ok(quiesced_mask | node.bit())
}

pub fn record_call(register: bool) {
    CALLS.fetch_add(1, Ordering::Relaxed);
    if register {
        REGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
    } else {
        QUIESCE_CALLS.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn record_permission_denied() {
    PERMISSION_DENIED.fetch_add(1, Ordering::Relaxed);
}

pub fn record_invalid_argument() {
    INVALID_ARGUMENTS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_identity_rejection() {
    IDENTITY_REJECTIONS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_invariant_error() {
    INVARIANT_ERRORS.fetch_add(1, Ordering::Relaxed);
}

pub fn register(
    node: ShutdownServiceNode,
    pid: u64,
    image: UserImageId,
    dependency_mask: u64,
    phase: Phase,
) -> Result<u64, RegisterError> {
    if pid == 0 || image != node.image_id() {
        IDENTITY_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return Err(RegisterError::InvalidIdentity);
    }
    let current = REGISTERED_MASK.load(Ordering::Acquire);
    let next = match reduce_register(current, node, image, dependency_mask, phase) {
        Ok(next) => next,
        Err(error) => {
            match error {
                RegisterError::InvalidIdentity => {
                    IDENTITY_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                }
                RegisterError::InvalidDependency => {
                    INVALID_ARGUMENTS.fetch_add(1, Ordering::Relaxed);
                }
                RegisterError::InvalidPhase => {
                    PHASE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                }
                RegisterError::Duplicate => {
                    REPLAYS.fetch_add(1, Ordering::Relaxed);
                }
            }
            return Err(error);
        }
    };
    let index = node.raw() as usize;
    if PIDS[index]
        .compare_exchange(0, pid, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        REPLAYS.fetch_add(1, Ordering::Relaxed);
        return Err(RegisterError::Duplicate);
    }
    if REGISTERED_MASK
        .compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        PIDS[index].store(0, Ordering::Release);
        INVARIANT_ERRORS.fetch_add(1, Ordering::Relaxed);
        return Err(RegisterError::Duplicate);
    }
    REGISTRATIONS.fetch_add(1, Ordering::Relaxed);
    Ok(next)
}

pub fn quiesce(
    node: ShutdownServiceNode,
    pid: u64,
    image: UserImageId,
    phase: Phase,
) -> Result<u64, QuiesceError> {
    let index = node.raw() as usize;
    if pid == 0 || image != node.image_id() || PIDS[index].load(Ordering::Acquire) != pid {
        IDENTITY_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return Err(QuiesceError::InvalidIdentity);
    }
    let registered = REGISTERED_MASK.load(Ordering::Acquire);
    let current = QUIESCED_MASK.load(Ordering::Acquire);
    let next = match reduce_quiesce(registered, current, node, phase) {
        Ok(next) => next,
        Err(error) => {
            match error {
                QuiesceError::InvalidIdentity => {
                    IDENTITY_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                }
                QuiesceError::InvalidPhase | QuiesceError::NotRegistered => {
                    PHASE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                }
                QuiesceError::AlreadyQuiesced => {
                    REPLAYS.fetch_add(1, Ordering::Relaxed);
                }
                QuiesceError::DependentsLive => {
                    ORDER_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                }
            }
            return Err(error);
        }
    };
    if QUIESCED_MASK
        .compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        INVARIANT_ERRORS.fetch_add(1, Ordering::Relaxed);
        return Err(QuiesceError::AlreadyQuiesced);
    }
    QUIESCES.fetch_add(1, Ordering::Relaxed);
    Ok(next)
}

pub fn snapshot() -> Snapshot {
    Snapshot {
        calls: CALLS.load(Ordering::Acquire),
        register_calls: REGISTER_CALLS.load(Ordering::Acquire),
        registrations: REGISTRATIONS.load(Ordering::Acquire),
        quiesce_calls: QUIESCE_CALLS.load(Ordering::Acquire),
        quiesces: QUIESCES.load(Ordering::Acquire),
        permission_denied: PERMISSION_DENIED.load(Ordering::Acquire),
        invalid_arguments: INVALID_ARGUMENTS.load(Ordering::Acquire),
        identity_rejections: IDENTITY_REJECTIONS.load(Ordering::Acquire),
        phase_rejections: PHASE_REJECTIONS.load(Ordering::Acquire),
        order_rejections: ORDER_REJECTIONS.load(Ordering::Acquire),
        replays: REPLAYS.load(Ordering::Acquire),
        invariant_errors: INVARIANT_ERRORS.load(Ordering::Acquire),
        registered_mask: REGISTERED_MASK.load(Ordering::Acquire),
        quiesced_mask: QUIESCED_MASK.load(Ordering::Acquire),
        pids: core::array::from_fn(|index| PIDS[index].load(Ordering::Acquire)),
    }
}

#[cfg(test)]
mod tests {
    use bndr_abi::{SHUTDOWN_SERVICE_ALL_MASK, ShutdownServiceNode, UserImageId};

    use super::{QuiesceError, RegisterError, reduce_quiesce, reduce_register};
    use crate::shutdown::Phase;

    #[test]
    fn all_nodes_register_only_with_exact_identity_and_dependencies() {
        let nodes = [
            ShutdownServiceNode::ServiceManager,
            ShutdownServiceNode::Provider,
            ShutdownServiceNode::PrimaryClient,
            ShutdownServiceNode::SecondaryClient,
            ShutdownServiceNode::SurfaceServer,
            ShutdownServiceNode::InputServer,
            ShutdownServiceNode::Launcher,
            ShutdownServiceNode::App,
        ];
        let mut registered = 0;
        for node in nodes {
            registered = reduce_register(
                registered,
                node,
                node.image_id(),
                node.dependency_mask(),
                Phase::Open,
            )
            .unwrap();
        }
        assert_eq!(registered, SHUTDOWN_SERVICE_ALL_MASK);
        assert_eq!(
            reduce_register(
                0,
                ShutdownServiceNode::App,
                UserImageId::Launcher,
                ShutdownServiceNode::App.dependency_mask(),
                Phase::Open,
            ),
            Err(RegisterError::InvalidIdentity)
        );
        assert_eq!(
            reduce_register(
                0,
                ShutdownServiceNode::App,
                UserImageId::App,
                0,
                Phase::Open,
            ),
            Err(RegisterError::InvalidDependency)
        );
    }

    #[test]
    fn registration_replay_and_post_prepare_registration_fail_closed() {
        let node = ShutdownServiceNode::Provider;
        let registered = reduce_register(
            0,
            node,
            node.image_id(),
            node.dependency_mask(),
            Phase::Open,
        )
        .unwrap();
        assert_eq!(
            reduce_register(
                registered,
                node,
                node.image_id(),
                node.dependency_mask(),
                Phase::Open,
            ),
            Err(RegisterError::Duplicate)
        );
        assert_eq!(
            reduce_register(
                0,
                node,
                node.image_id(),
                node.dependency_mask(),
                Phase::Quiescing,
            ),
            Err(RegisterError::InvalidPhase)
        );
    }

    #[test]
    fn reverse_topological_quiesce_rejects_live_dependents() {
        let registered = SHUTDOWN_SERVICE_ALL_MASK;
        assert_eq!(
            reduce_quiesce(
                registered,
                0,
                ShutdownServiceNode::SurfaceServer,
                Phase::Quiescing,
            ),
            Err(QuiesceError::DependentsLive)
        );
        let leaves = ShutdownServiceNode::PrimaryClient.bit()
            | ShutdownServiceNode::SecondaryClient.bit()
            | ShutdownServiceNode::Launcher.bit()
            | ShutdownServiceNode::App.bit();
        let after_provider = reduce_quiesce(
            registered,
            leaves,
            ShutdownServiceNode::Provider,
            Phase::Quiescing,
        )
        .unwrap();
        let after_input = reduce_quiesce(
            registered,
            after_provider,
            ShutdownServiceNode::InputServer,
            Phase::Quiescing,
        )
        .unwrap();
        let after_manager = reduce_quiesce(
            registered,
            after_input,
            ShutdownServiceNode::ServiceManager,
            Phase::Quiescing,
        )
        .unwrap();
        assert_eq!(
            reduce_quiesce(
                registered,
                after_manager,
                ShutdownServiceNode::SurfaceServer,
                Phase::Quiescing,
            )
            .unwrap(),
            SHUTDOWN_SERVICE_ALL_MASK
        );
    }

    #[test]
    fn quiesce_requires_prepare_registration_and_single_use() {
        let node = ShutdownServiceNode::App;
        assert_eq!(
            reduce_quiesce(0, 0, node, Phase::Quiescing),
            Err(QuiesceError::NotRegistered)
        );
        assert_eq!(
            reduce_quiesce(node.bit(), 0, node, Phase::Open),
            Err(QuiesceError::InvalidPhase)
        );
        assert_eq!(
            reduce_quiesce(node.bit(), node.bit(), node, Phase::Quiescing),
            Err(QuiesceError::AlreadyQuiesced)
        );
    }
}
