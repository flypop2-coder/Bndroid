use alloc::alloc::{Layout, alloc, dealloc};
use alloc::boxed::Box;
use core::cell::UnsafeCell;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use bndr_abi::{HandleValue, ObjectSignals, ProcessTerminationReason, Rights, UserImageId};
#[cfg(feature = "resident-platform-shutdown-runtime")]
use bndr_abi::{
    SHUTDOWN_SERVICE_ALL_MASK, SHUTDOWN_SERVICE_EDGE_COUNT, SHUTDOWN_SERVICE_NODE_COUNT,
    ShutdownServiceNode,
};
use bndroid_kernel::channel::ChannelEndpoint;
use bndroid_kernel::graphics_buffer::{
    GraphicsBuffer, GraphicsBufferAccessSnapshot, GraphicsBufferIdentity, pool_snapshot,
};
use bndroid_kernel::handles::{HandleError, HandleTable};
#[cfg(feature = "input-server-runtime")]
use bndroid_kernel::input_broker::InputCapability;
use bndroid_kernel::memory;
use bndroid_kernel::object::KernelObject;
use bndroid_kernel::object_wait::{ObjectWaitCompletionKind, ObjectWaitToken};
use bndroid_kernel::process_wait::{CompletionSlot, decode_process_id, encode_process_id};
#[cfg(feature = "storage-server-runtime")]
use bndroid_kernel::storage_broker::StorageVolumeCapability;
use bndroid_kernel::surface::SurfaceCapability;

use crate::arch::aarch64::mmu::{
    self, AddressSpaceDestroyFailure, StoppedAddressSpaceProof, UserAddressSpace,
};
pub use crate::arch::aarch64::mmu::{
    GraphicsMappingAccess, GraphicsMappingError, GraphicsMappingRole, GraphicsMappingSnapshot,
};
use crate::scheduler;
use crate::syscall::HANDLE_CAPACITY;
use crate::userboot::{PreparedUserProcess, UserBootError};

// The final resident slice keeps the manager, provider, two independently
// identified clients, SurfaceServer, Launcher, and one generation-qualified
// App. M33 replaces the App once; M34 replaces it a second time after an
// unexpected death. Init occupies slot zero.
const PROCESS_CAPACITY: usize = crate::limits::PROCESS_CAPACITY;
const INIT_PROCESS_SLOT: usize = 0;
const FIRST_DYNAMIC_PROCESS_SLOT: usize = INIT_PROCESS_SLOT + 1;
const DYNAMIC_PROCESS_CAPACITY: usize = crate::limits::DYNAMIC_PROCESS_CAPACITY;
const _: () = assert!(PROCESS_CAPACITY == FIRST_DYNAMIC_PROCESS_SLOT + DYNAMIC_PROCESS_CAPACITY);
const CHILD_SPAWN_EVIDENCE_CAPACITY: usize = 10;
const USER_IMAGE_COUNT: usize = 7;
const BASELINE_RESIDENT_ENDPOINT_COUNT: usize = 12;
#[cfg(not(feature = "androidbox-restart0"))]
const M32_FINAL_RESIDENT_ENDPOINT_COUNT: usize = 20;
#[cfg(feature = "androidbox-restart0")]
const M32_FINAL_RESIDENT_ENDPOINT_COUNT: usize = 22;
const APP_LIFECYCLE_FINAL_RESIDENT_ENDPOINT_COUNT: usize = 26;
#[cfg(feature = "input-server-runtime")]
const INPUT_SERVER_FINAL_RESIDENT_ENDPOINT_COUNT: usize = 30;
#[cfg(feature = "input-server-runtime")]
const INPUT_SERVER_SURFACE_RESTART_RUNTIME_ACTIVE: bool = cfg!(all(
    feature = "input-server-surface-restart-runtime",
    not(feature = "input-server-restart-runtime"),
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime"),
    not(feature = "el0-fault-containment-self-test"),
    not(feature = "process-terminate-self-test")
));
#[cfg(feature = "input-server-runtime")]
const INPUT_SERVER_RESTART_RUNTIME_ACTIVE: bool = cfg!(feature = "input-server-restart-runtime");
#[cfg(feature = "input-server-runtime")]
const SERVICE_DEPENDENCY_RUNTIME_ACTIVE: bool = cfg!(feature = "service-dependency-runtime");
pub const RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY: usize = 2;
const MULTI_WINDOW_RUNTIME_ACTIVE: bool = cfg!(any(
    feature = "input-server-restart-runtime",
    all(
        feature = "multi-window-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime"),
        not(feature = "el0-fault-containment-self-test"),
        not(feature = "process-terminate-self-test")
    )
));
const GRAPHICS_SWAPCHAIN_RUNTIME_ACTIVE: bool = cfg!(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "multi-window-runtime"),
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
));
#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "input-server-runtime"),
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const GRAPHICS_PRODUCER_ORPHAN_FINAL_RESIDENT_ENDPOINT_COUNT: usize = 18;
#[cfg(not(feature = "input-server-runtime"))]
const RESIDENT_ENDPOINT_CAPACITY: usize = APP_LIFECYCLE_FINAL_RESIDENT_ENDPOINT_COUNT;
#[cfg(feature = "input-server-runtime")]
const RESIDENT_ENDPOINT_CAPACITY: usize = INPUT_SERVER_FINAL_RESIDENT_ENDPOINT_COUNT;
const BASELINE_RESIDENT_WAIT_TOKEN_COUNT: usize = 3;
#[cfg(not(feature = "androidbox-process0"))]
const M32_FINAL_RESIDENT_WAIT_TOKEN_COUNT: usize = 7;
#[cfg(feature = "androidbox-process0")]
const M32_FINAL_RESIDENT_WAIT_TOKEN_COUNT: usize = 8;
#[cfg(not(feature = "input-server-runtime"))]
pub(crate) const RESIDENT_WAIT_TOKEN_COUNT: usize = 8;
#[cfg(feature = "input-server-runtime")]
pub(crate) const RESIDENT_WAIT_TOKEN_COUNT: usize = 9;
const BASELINE_MANAGER_CHANNEL_WAIT_ITEM_COUNT: usize = 3;
const FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT: usize = 4;
const SERVICE_CHANNEL_WAIT_ITEM_COUNT: usize = 2;
const SURFACE_SERVER_WAIT_ITEM_COUNT: usize = 3;
const LAUNCHER_WAIT_ITEM_COUNT: usize = 1;
const APP_WAIT_ITEM_COUNT: usize = 1;
const PROCESS_TERMINATE_SELF_TEST_INIT_ARGUMENT: u64 = 0x5054_5354_494e_4954;

#[cfg(all(
    feature = "app-lifecycle-runtime",
    not(feature = "input-server-runtime"),
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime"),
    not(all(
        feature = "graphics-frame-clock-runtime",
        not(feature = "graphics-owner-death-runtime")
    ))
))]
const FINAL_APP_SPAWN_INDEX: usize = 8;
#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "input-server-runtime"),
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const FINAL_APP_SPAWN_INDEX: usize = 7;
#[cfg(all(
    feature = "input-server-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const FINAL_APP_SPAWN_INDEX: usize = 8;
#[cfg(all(
    feature = "graphics-surface-restart-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
const FINAL_APP_SPAWN_INDEX: usize = 7;
#[cfg(feature = "app-crash-recovery-runtime")]
const FINAL_APP_SPAWN_INDEX: usize = 9;
#[cfg(all(
    feature = "app-lifecycle-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime"),
    not(all(
        feature = "graphics-frame-clock-runtime",
        not(feature = "graphics-owner-death-runtime")
    ))
))]
const FINAL_APP_GRAPHICS_BUFFER_GENERATION: u64 = 2;
#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const FINAL_APP_GRAPHICS_BUFFER_GENERATION: u64 = 1;
#[cfg(all(
    feature = "graphics-surface-restart-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
const FINAL_APP_GRAPHICS_BUFFER_GENERATION: u64 = 1;
#[cfg(feature = "app-crash-recovery-runtime")]
const FINAL_APP_GRAPHICS_BUFFER_GENERATION: u64 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessId(u64);

impl ProcessId {
    const fn new(slot: usize, generation: u32) -> Self {
        match encode_process_id(slot, generation, PROCESS_CAPACITY) {
            Some(raw) => Self(raw),
            None => panic!("invalid bounded process ID"),
        }
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    const fn slot(self) -> Option<usize> {
        match decode_process_id(self.0, PROCESS_CAPACITY) {
            Some((slot, _)) => Some(slot),
            None => None,
        }
    }

    const fn generation(self) -> u32 {
        (self.0 >> 32) as u32
    }
}

pub enum SpawnError {
    ShouldWait,
    OutOfMemory,
    NotFound,
    PermissionDenied,
    InvalidState,
    Image(UserBootError),
}

pub enum ChildWait {
    Completed {
        exit_code: u64,
        reason: ProcessTerminationReason,
    },
    Live,
}

pub enum ChildWaitError {
    NotFound,
    InvalidState,
}

pub enum TerminateChildError {
    NotFound,
    InvalidState,
}

#[cfg(feature = "storage-server-owner-liveness-runtime")]
pub enum StorageWatchdogTerminateError {
    NotFound,
    AlreadyTerminal,
    InvalidState,
}

pub enum ObjectPollError {
    NotFound,
    PermissionDenied,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessGraphicsMappingError {
    InvalidState,
    NotFound,
    IdentityMismatch,
    AddressSpace(GraphicsMappingError),
}

impl ProcessGraphicsMappingError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidState => "graphics mapping operation is invalid for the current process",
            Self::NotFound => "generation-qualified graphics mapping process was not found",
            Self::IdentityMismatch => {
                "graphics mapping producer identity does not match the process"
            }
            Self::AddressSpace(error) => error.as_str(),
        }
    }
}

impl From<GraphicsMappingError> for ProcessGraphicsMappingError {
    fn from(error: GraphicsMappingError) -> Self {
        Self::AddressSpace(error)
    }
}

struct Process {
    id: ProcessId,
    image_id: UserImageId,
    address_space: Option<UserAddressSpace>,
    handles: Box<HandleTable<KernelObject, HANDLE_CAPACITY>>,
    startup_handle: Option<HandleValue>,
    is_init: bool,
}

#[derive(Clone, Copy)]
struct ProcessCompletion {
    id: ProcessId,
    exit_code: u64,
    reason: u64,
}

struct ProcessSlot {
    generation: u32,
    process: Option<Box<Process>>,
    completion: CompletionSlot<ProcessCompletion>,
}

impl ProcessSlot {
    const fn new() -> Self {
        Self {
            generation: 1,
            process: None,
            completion: CompletionSlot::new(),
        }
    }
}

struct ProcessTable(UnsafeCell<[ProcessSlot; PROCESS_CAPACITY]>);

// All access is serialized on the boot CPU with local IRQ masked. This is a
// deliberately single-core lifecycle domain, not an SMP lock.
unsafe impl Sync for ProcessTable {}

static TABLE: ProcessTable = ProcessTable(UnsafeCell::new(
    [const { ProcessSlot::new() }; PROCESS_CAPACITY],
));
static INITIALIZED: AtomicBool = AtomicBool::new(false);
static CREATED: AtomicU64 = AtomicU64::new(0);
static EXITED: AtomicU64 = AtomicU64::new(0);
static REAPED: AtomicU64 = AtomicU64::new(0);
static TERMINATED_EXITED: AtomicU64 = AtomicU64::new(0);
static TERMINATED_FAULTED: AtomicU64 = AtomicU64::new(0);
static TERMINATED_KILLED: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK_LIVE: AtomicU64 = AtomicU64::new(0);
static SPAWN_WAITS: AtomicU64 = AtomicU64::new(0);
static CAPACITY_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static WAIT_CALLS: AtomicU64 = AtomicU64::new(0);
static WAIT_COMPLETED: AtomicU64 = AtomicU64::new(0);
static WAIT_IMMEDIATE: AtomicU64 = AtomicU64::new(0);
static WAIT_STALE: AtomicU64 = AtomicU64::new(0);
static CHILD_SYSCALLS: AtomicU64 = AtomicU64::new(0);
static CHILD_SELECTIONS: AtomicU64 = AtomicU64::new(0);
static CHILD_HANDLE_ISOLATED: AtomicBool = AtomicBool::new(false);
static STARTUP_MOVES: AtomicU64 = AtomicU64::new(0);
static CHILD_FRAMES_ISOLATED: AtomicBool = AtomicBool::new(false);
static FIRST_CHILD_PID: AtomicU64 = AtomicU64::new(0);
static SECOND_CHILD_PID: AtomicU64 = AtomicU64::new(0);
static CHILD_SPAWN_PIDS: [AtomicU64; CHILD_SPAWN_EVIDENCE_CAPACITY] =
    [const { AtomicU64::new(0) }; CHILD_SPAWN_EVIDENCE_CAPACITY];
static CHILD_SPAWN_IMAGES: [AtomicU64; CHILD_SPAWN_EVIDENCE_CAPACITY] =
    [const { AtomicU64::new(0) }; CHILD_SPAWN_EVIDENCE_CAPACITY];
// M68 also preserves the sealed ten-entry historical ledger. Its eleventh
// child is the same-slot next-generation StorageServer recorded separately.
#[cfg(feature = "unified-product-liveness-runtime")]
static UNIFIED_PRODUCT_STORAGE_REPLACEMENT_PID: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-liveness-runtime")]
static UNIFIED_PRODUCT_STORAGE_REPLACEMENT_IMAGE: AtomicU64 = AtomicU64::new(0);
// M73 preserves M68--M72's first replacement ledger and records the second
// same-slot generation separately for its two independent rotation windows.
#[cfg(feature = "unified-product-event-supervision-runtime")]
static UNIFIED_PRODUCT_STORAGE_SECOND_REPLACEMENT_PID: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-event-supervision-runtime")]
static UNIFIED_PRODUCT_STORAGE_SECOND_REPLACEMENT_IMAGE: AtomicU64 = AtomicU64::new(0);
// M49 deliberately keeps the historical ten-entry spawn ledger sealed. Its
// eleventh child (the second InputServer generation) is recorded separately,
// so older profile arrays and their exact evidence remain byte-for-byte stable.
#[cfg(feature = "service-dependency-runtime")]
static SERVICE_DEPENDENCY_INPUT_REPLACEMENT_PID: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "service-dependency-runtime")]
static SERVICE_DEPENDENCY_INPUT_REPLACEMENT_IMAGE: AtomicU64 = AtomicU64::new(0);
static FIRST_CHILD_ASID: AtomicU64 = AtomicU64::new(0);
static LAST_CHILD_ASID: AtomicU64 = AtomicU64::new(0);
static FIRST_CHILD_ROOT: AtomicUsize = AtomicUsize::new(0);
static FIRST_CHILD_CODE_PHYSICAL: AtomicUsize = AtomicUsize::new(0);
static FIRST_CHILD_STACK_PHYSICAL: AtomicUsize = AtomicUsize::new(0);
static INIT_ROOT: AtomicUsize = AtomicUsize::new(0);
static INIT_CODE_PHYSICAL: AtomicUsize = AtomicUsize::new(0);
static INIT_STACK_PHYSICAL: AtomicUsize = AtomicUsize::new(0);
static SLOT_REUSED: AtomicBool = AtomicBool::new(false);
static ASID_REUSED: AtomicBool = AtomicBool::new(false);
static STALE_REJECTED: AtomicBool = AtomicBool::new(false);
static ROOT_ISOLATED: AtomicBool = AtomicBool::new(false);
static FRAME_RESTORED: AtomicBool = AtomicBool::new(true);
static HEAP_RESTORED: AtomicBool = AtomicBool::new(true);
static EXIT_VALIDATION_FAILURES: AtomicU64 = AtomicU64::new(0);
static NONSTANDARD_EXIT_CODES: AtomicU64 = AtomicU64::new(0);
static NON_TARGET_REAPS: AtomicU64 = AtomicU64::new(0);
static FRAME_DESTROY_DELTA_VALID: AtomicBool = AtomicBool::new(true);
static DISTINCT_LIVE_SLOTS: AtomicBool = AtomicBool::new(false);
static DISTINCT_LIVE_ASIDS: AtomicBool = AtomicBool::new(false);
static DISTINCT_LIVE_ROOTS: AtomicBool = AtomicBool::new(false);
static DISTINCT_LIVE_FRAMES: AtomicBool = AtomicBool::new(false);
static GRAPHICS_CONSUMER_DEATHS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_CONSUMER_DEATH_MAPPINGS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_CONSUMER_DEATH_QUEUED: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_CONSUMER_DEATH_ACQUIRED: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_CONSUMER_DEATH_PRODUCER_RESTORES: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_CONSUMER_DEATH_PRODUCER_ORPHANS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_CONSUMER_DEATH_RELEASES: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_CONSUMER_DEATH_WAKE_COMPLETIONS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "graphics-owner-death-runtime")]
pub struct GraphicsConsumerOwnerDeathSnapshot {
    pub deaths: u64,
    pub mappings: u64,
    pub queued: u64,
    pub acquired: u64,
    pub producer_restores: u64,
    pub producer_orphans: u64,
    pub releases: u64,
    pub wake_completions: u64,
}

/// ABI 46's exact three-buffer capability graph.
///
/// The historical resident snapshot intentionally retains its two-identity
/// bound for ABI 45 and predecessor profiles. The interactive profile adds a
/// third, independently produced system-chrome buffer, so it needs a separate
/// bounded proof instead of weakening those older exact contracts.
#[cfg(feature = "androidbox-interactive0")]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidboxInteractiveGraphicsEvidence {
    pub handle_count: usize,
    pub identity_count: usize,
    pub identity_handle_counts: [usize; 3],
    pub identity_producer_handle_counts: [usize; 3],
    pub identity_server_handle_counts: [usize; 3],
    pub producer_pids: [u64; 3],
    /// SurfaceServer, Launcher, and App producer-handle counts.
    pub producer_role_counts: [usize; 3],
    pub server_handle_count: usize,
    pub surface_server_pid: u64,
    pub valid: bool,
}

/// ABI 47's private App ↔ AndroidApp channel is deliberately outside the
/// historical resident service graph. This evidence proves the isolated
/// pair without weakening any predecessor endpoint, rights, or queue count.
#[cfg(feature = "androidbox-process0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidAppAuthorityEvidence {
    pub app_pid: u64,
    pub worker_pid: u64,
    pub app_asid: u64,
    pub worker_asid: u64,
    pub app_root: u64,
    pub worker_root: u64,
    pub app_image_digest: u64,
    pub worker_image_digest: u64,
    pub worker_live: u64,
    pub worker_handles: usize,
    pub worker_channel_count: usize,
    pub worker_read_only_vmo_count: usize,
    pub worker_unexpected_handle_count: usize,
    pub private_endpoint_count: usize,
    pub private_pair_count: usize,
    pub app_endpoint_count: usize,
    pub unexpected_private_owner_count: usize,
    pub endpoints_unique: bool,
    pub worker_rights_valid: bool,
    pub app_rights_valid: bool,
    pub queues_empty: bool,
    pub isolation_valid: bool,
    pub objects_valid: bool,
    pub runtime_objects_valid: bool,
    pub valid: bool,
}

#[cfg(feature = "androidbox-process0")]
impl AndroidAppAuthorityEvidence {
    const EMPTY: Self = Self {
        app_pid: 0,
        worker_pid: 0,
        app_asid: 0,
        worker_asid: 0,
        app_root: 0,
        worker_root: 0,
        app_image_digest: 0,
        worker_image_digest: 0,
        worker_live: 0,
        worker_handles: 0,
        worker_channel_count: 0,
        worker_read_only_vmo_count: 0,
        worker_unexpected_handle_count: 0,
        private_endpoint_count: 0,
        private_pair_count: 0,
        app_endpoint_count: 0,
        unexpected_private_owner_count: 0,
        endpoints_unique: true,
        worker_rights_valid: true,
        app_rights_valid: true,
        queues_empty: true,
        isolation_valid: false,
        objects_valid: false,
        runtime_objects_valid: false,
        valid: false,
    };
}

// M14 consumers in main.rs are wired separately from this atomic process/
// scheduler change; keep the complete evidence surface available meanwhile.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct ProcessSnapshot {
    pub process_capacity: usize,
    pub dynamic_capacity: usize,
    pub created: u64,
    pub exited: u64,
    pub reaped: u64,
    pub terminated_exited: u64,
    pub terminated_faulted: u64,
    pub terminated_killed: u64,
    pub live: u64,
    pub peak_live: u64,
    pub spawn_waits: u64,
    pub capacity_rejections: u64,
    pub wait_calls: u64,
    pub wait_completed: u64,
    pub wait_blocks: u64,
    pub wait_wakes: u64,
    pub wait_immediate: u64,
    pub wait_stale: u64,
    pub wait_pending: bool,
    pub supervisor_wait_pending: bool,
    pub supervisor_wait_target: u64,
    pub completion_pending: bool,
    pub child_syscalls: u64,
    pub child_selections: u64,
    pub first_child_pid: u64,
    pub second_child_pid: u64,
    pub child_spawn_pids: [u64; CHILD_SPAWN_EVIDENCE_CAPACITY],
    pub child_spawn_images: [u64; CHILD_SPAWN_EVIDENCE_CAPACITY],
    #[cfg(feature = "unified-product-liveness-runtime")]
    pub unified_product_storage_replacement_pid: u64,
    #[cfg(feature = "unified-product-liveness-runtime")]
    pub unified_product_storage_replacement_image: u64,
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    pub unified_product_storage_second_replacement_pid: u64,
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    pub unified_product_storage_second_replacement_image: u64,
    #[cfg(feature = "service-dependency-runtime")]
    pub service_dependency_input_replacement_pid: u64,
    #[cfg(feature = "service-dependency-runtime")]
    pub service_dependency_input_replacement_image: u64,
    pub live_image_counts: [u64; USER_IMAGE_COUNT],
    pub handle_counts_by_image: [usize; USER_IMAGE_COUNT],
    #[cfg(feature = "input-server-runtime")]
    pub input_server_live: u64,
    #[cfg(feature = "input-server-runtime")]
    pub input_server_handles: usize,
    #[cfg(feature = "storage-server-runtime")]
    pub storage_server_live: u64,
    #[cfg(feature = "storage-server-runtime")]
    pub storage_server_handles: usize,
    #[cfg(feature = "androidbox-process0")]
    pub android_app_live: u64,
    #[cfg(feature = "androidbox-process0")]
    pub android_app_handles: usize,
    #[cfg(feature = "androidbox-process0")]
    pub android_app_authority: AndroidAppAuthorityEvidence,
    pub total_handles: usize,
    pub resident_control_links_valid: bool,
    pub resident_control_queues_empty: bool,
    pub resident_wait_topology_valid: bool,
    pub resident_endpoint_count: usize,
    pub resident_channel_pair_count: usize,
    /// init-manager, init-provider, init-client, manager-provider,
    /// manager-client, provider-client peer-link counts, in that order.
    pub resident_cross_image_links: [usize; 6],
    /// Exact SurfaceServer--Launcher/App UI peer-pair count, kept separate
    /// from the six historical M20 core-service cross-image classes.
    pub resident_ui_channel_pair_count: usize,
    /// Init--SurfaceServer, Init--Launcher, Init--App,
    /// SurfaceServer--Launcher, and SurfaceServer--App peer-pair counts.
    /// This distinguishes M33/M34 lifecycle/control links from the two M32
    /// UI links without weakening any profile's exact graph proof.
    pub resident_window_channel_pairs: [usize; 5],
    pub resident_endpoints_unique: bool,
    pub resident_channel_rights_valid: bool,
    /// Number of live `SurfaceCapability` handles outside the channel graph.
    pub resident_surface_capability_count: usize,
    /// Session identity carried by the one resident surface capability.
    pub resident_surface_capability_session_id: u64,
    /// Generation-qualified PID owning the one resident surface capability.
    pub resident_surface_capability_owner_pid: u64,
    /// True only when the capability is uniquely owned by the selected live
    /// `SurfaceServer` generation.
    pub resident_surface_capability_owner_valid: bool,
    /// True only when the unique capability has exactly `SURFACE_DEFAULT`.
    pub resident_surface_capability_rights_valid: bool,
    #[cfg(feature = "input-server-runtime")]
    pub resident_input_capability_count: usize,
    #[cfg(feature = "input-server-runtime")]
    pub resident_input_capability_session_id: u64,
    #[cfg(feature = "input-server-runtime")]
    pub resident_input_capability_owner_pid: u64,
    #[cfg(feature = "input-server-runtime")]
    pub resident_input_capability_owner_valid: bool,
    #[cfg(feature = "input-server-runtime")]
    pub resident_input_capability_rights_valid: bool,
    #[cfg(feature = "input-server-runtime")]
    pub resident_input_channel_pair_count: usize,
    /// Compatibility view of the first resident generation-qualified
    /// graphics-buffer allocation. Single-buffer profiles still require
    /// exactly two handles naming this identity.
    pub resident_graphics_buffer_handle_count: usize,
    pub resident_graphics_buffer_producer_pid: u64,
    pub resident_graphics_buffer_identity: Option<GraphicsBufferIdentity>,
    /// Bounded identity-by-identity ownership evidence. M40 and M41 admit
    /// exactly two identities; historical profiles retain one at index zero.
    pub resident_graphics_buffer_identity_count: usize,
    pub resident_graphics_buffer_identities:
        [Option<GraphicsBufferIdentity>; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY],
    pub resident_graphics_buffer_identity_handle_counts:
        [usize; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY],
    pub resident_graphics_buffer_identity_owner_bitmaps:
        [u8; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY],
    pub resident_graphics_buffer_identity_producer_pids:
        [u64; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY],
    pub resident_graphics_buffer_identities_share_producer: bool,
    pub resident_graphics_buffer_owners_valid: bool,
    pub resident_graphics_buffer_rights_valid: bool,
    /// MMU-wide graphics mapping transactions. Unmaps include explicit ABI
    /// calls and stopped-process teardown; M41 mappings need not be shared.
    pub graphics_mapping_maps: u64,
    pub graphics_mapping_unmaps: u64,
    pub graphics_mapping_protects: u64,
    pub graphics_mapping_live_mappings: u64,
    pub graphics_mapping_live_pages: u64,
    /// Exact live process-address-space alias evidence for M35.
    pub resident_graphics_mapping_count: usize,
    pub resident_graphics_mapping_mapped_pages: usize,
    pub resident_graphics_mapping_producer_count: usize,
    pub resident_graphics_mapping_consumer_count: usize,
    pub resident_graphics_mapping_shared_pairs: usize,
    pub resident_graphics_mapping_roles_access_valid: bool,
    /// Exact profile-specific mapping topology. Historical mapped profiles
    /// require producer/consumer alias pairs; M41 instead requires two
    /// SurfaceServer-owned producer mappings and no consumer mappings.
    pub resident_graphics_mapping_topology_valid: bool,
    pub resident_graphics_mapping_shared_alias_valid: bool,
    pub resident_graphics_mapping_contexts_distinct: bool,
    pub resident_service_topology_valid: bool,
    pub resident_service_queues_empty: bool,
    pub resident_service_wait_topology_valid: bool,
    /// Exact current manager/provider/primary-client/secondary-client tokens,
    /// including PID, epoch, generation handles, masks, item count, and
    /// deadline. Array item order may rotate between otherwise stable samples.
    pub resident_wait_tokens: [Option<ObjectWaitToken>; RESIDENT_WAIT_TOKEN_COUNT],
    pub child_asid: u8,
    pub asid_reused: bool,
    pub slot_reused: bool,
    pub stale_rejected: bool,
    pub root_isolated: bool,
    pub child_frames_isolated: bool,
    pub frame_restored: bool,
    pub heap_restored: bool,
    pub child_handle_isolated: bool,
    pub startup_moves: u64,
    pub exit_validation_failures: u64,
    pub nonstandard_exit_codes: u64,
    pub non_target_reaps: u64,
    pub frame_destroy_delta_valid: bool,
    pub distinct_live_slots: bool,
    pub distinct_live_asids: bool,
    pub distinct_live_roots: bool,
    pub distinct_live_frames: bool,
}

#[cfg(feature = "resident-platform-shutdown-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentShutdownTopologySnapshot {
    pub valid: bool,
    pub processes: usize,
    pub service_nodes: usize,
    pub control_pairs: usize,
    pub dependency_pairs: usize,
    pub channel_pairs: usize,
    pub endpoints: usize,
    pub total_handles: usize,
    pub queues_empty: bool,
    pub rights_valid: bool,
    pub endpoints_unique: bool,
    pub startup_peers_valid: bool,
    pub storage_volume_valid: bool,
    pub ledger_pids_valid: bool,
}

#[derive(Clone, Copy)]
struct ResidentEndpoint<'a> {
    image_id: UserImageId,
    channel: &'a ChannelEndpoint,
}

#[cfg(feature = "resident-platform-shutdown-runtime")]
#[derive(Clone, Copy)]
struct ResidentShutdownEndpoint<'a> {
    owner: usize,
    channel: &'a ChannelEndpoint,
}

#[derive(Clone, Copy)]
struct ResidentTopologyEvidence {
    endpoint_count: usize,
    channel_pair_count: usize,
    cross_image_links: [usize; 6],
    ui_channel_pair_count: usize,
    window_channel_pairs: [usize; 5],
    within_image_links: usize,
    endpoints_unique: bool,
    channel_rights_valid: bool,
    surface_capability_count: usize,
    surface_capability_session_id: u64,
    surface_capability_owner_pid: u64,
    surface_capability_owner_valid: bool,
    surface_capability_rights_valid: bool,
    #[cfg(feature = "input-server-runtime")]
    input_capability_count: usize,
    #[cfg(feature = "input-server-runtime")]
    input_capability_session_id: u64,
    #[cfg(feature = "input-server-runtime")]
    input_capability_owner_pid: u64,
    #[cfg(feature = "input-server-runtime")]
    input_capability_owner_valid: bool,
    #[cfg(feature = "input-server-runtime")]
    input_capability_rights_valid: bool,
    #[cfg(feature = "input-server-runtime")]
    input_channel_pair_count: usize,
    #[cfg(feature = "androidbox-process0")]
    android_app_authority: AndroidAppAuthorityEvidence,
    graphics_buffer_handle_count: usize,
    graphics_buffer_producer_pid: u64,
    graphics_buffer_identity: Option<GraphicsBufferIdentity>,
    graphics_buffer_identity_count: usize,
    graphics_buffer_identities:
        [Option<GraphicsBufferIdentity>; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY],
    graphics_buffer_identity_handle_counts: [usize; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY],
    graphics_buffer_identity_owner_bitmaps: [u8; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY],
    graphics_buffer_identity_producer_pids: [u64; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY],
    graphics_buffer_identities_share_producer: bool,
    graphics_buffer_owner_bitmap: u8,
    graphics_buffer_rights_valid: bool,
    unexpected_handle_count: usize,
    service_topology_valid: bool,
    service_queues_empty: bool,
    service_wait_topology_valid: bool,
    wait_tokens: [Option<ObjectWaitToken>; RESIDENT_WAIT_TOKEN_COUNT],
}

#[derive(Clone, Copy)]
struct ResidentGraphicsMapping<'a> {
    process: &'a Process,
    mapping: GraphicsMappingSnapshot,
}

#[derive(Clone, Copy)]
struct ResidentGraphicsMappingEvidence {
    count: usize,
    mapped_pages: usize,
    producer_count: usize,
    consumer_count: usize,
    shared_pairs: usize,
    roles_access_valid: bool,
    topology_valid: bool,
    shared_alias_valid: bool,
    contexts_distinct: bool,
}

impl ResidentTopologyEvidence {
    const EMPTY: Self = Self {
        endpoint_count: 0,
        channel_pair_count: 0,
        cross_image_links: [0; 6],
        ui_channel_pair_count: 0,
        window_channel_pairs: [0; 5],
        within_image_links: 0,
        endpoints_unique: false,
        channel_rights_valid: false,
        surface_capability_count: 0,
        surface_capability_session_id: 0,
        surface_capability_owner_pid: 0,
        surface_capability_owner_valid: false,
        surface_capability_rights_valid: false,
        #[cfg(feature = "input-server-runtime")]
        input_capability_count: 0,
        #[cfg(feature = "input-server-runtime")]
        input_capability_session_id: 0,
        #[cfg(feature = "input-server-runtime")]
        input_capability_owner_pid: 0,
        #[cfg(feature = "input-server-runtime")]
        input_capability_owner_valid: false,
        #[cfg(feature = "input-server-runtime")]
        input_capability_rights_valid: false,
        #[cfg(feature = "input-server-runtime")]
        input_channel_pair_count: 0,
        #[cfg(feature = "androidbox-process0")]
        android_app_authority: AndroidAppAuthorityEvidence::EMPTY,
        graphics_buffer_handle_count: 0,
        graphics_buffer_producer_pid: 0,
        graphics_buffer_identity: None,
        graphics_buffer_identity_count: 0,
        graphics_buffer_identities: [None; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY],
        graphics_buffer_identity_handle_counts: [0; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY],
        graphics_buffer_identity_owner_bitmaps: [0; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY],
        graphics_buffer_identity_producer_pids: [0; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY],
        graphics_buffer_identities_share_producer: false,
        graphics_buffer_owner_bitmap: 0,
        graphics_buffer_rights_valid: false,
        unexpected_handle_count: 0,
        service_topology_valid: false,
        service_queues_empty: false,
        service_wait_topology_valid: false,
        wait_tokens: [None; RESIDENT_WAIT_TOKEN_COUNT],
    };
}

pub fn init() {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    if INITIALIZED.swap(true, Ordering::AcqRel) {
        panic!("process manager initialized twice");
    }
    let slots = slots_mut();
    if slots
        .iter()
        .any(|slot| slot.process.is_some() || slot.completion.is_pending())
    {
        panic!("process table was not empty during initialization");
    }
    reset_counters();
    crate::arch::aarch64::restore_daif(saved_daif);
}

pub fn install_init(prepared: Box<PreparedUserProcess>) {
    assert_initialized();
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let PreparedUserProcess {
        info,
        address_space,
    } = take_box(prepared);
    let slot = &mut slots_mut()[INIT_PROCESS_SLOT];
    if slot.process.is_some()
        || slot.completion.is_pending()
        || info.image_id != UserImageId::Init
        || info.asid != mmu::INIT_USER_ASID
    {
        panic!("init process publication invariants failed");
    }
    let id = ProcessId::new(INIT_PROCESS_SLOT, slot.generation);
    let translation = address_space.translation_context();
    let handles = HandleTable::try_new_boxed()
        .unwrap_or_else(|| panic!("kernel heap could not allocate init handle table"));
    #[cfg(all(
        not(feature = "el0-fault-containment-self-test"),
        not(bndroid_storage_irq_timeout_profile),
        not(feature = "process-terminate-self-test")
    ))]
    let (handles, system_root) = {
        let mut handles = handles;
        let catalog = bndroid_kernel::system_files::snapshot();
        if !catalog.ready || catalog.files != 2 || catalog.bytes != 68 {
            panic!("init system-directory capability preceded catalog publication");
        }
        let system_root = handles
            .insert(KernelObject::SystemDirectory, Rights::DIRECTORY_DEFAULT)
            .unwrap_or_else(|_| panic!("init system-directory capability installation failed"));
        (handles, system_root)
    };
    #[cfg(any(
        feature = "el0-fault-containment-self-test",
        bndroid_storage_irq_timeout_profile,
        feature = "process-terminate-self-test"
    ))]
    let system_root = HandleValue::INVALID;
    let process = Process {
        id,
        image_id: UserImageId::Init,
        address_space: Some(address_space),
        handles,
        startup_handle: None,
        is_init: true,
    };
    slot.process = Some(
        try_box(process).unwrap_or_else(|_| panic!("kernel heap could not allocate init Process")),
    );
    INIT_ROOT.store(info.user_root, Ordering::Relaxed);
    INIT_CODE_PHYSICAL.store(info.code_physical, Ordering::Relaxed);
    INIT_STACK_PHYSICAL.store(info.stack_physical, Ordering::Relaxed);
    CREATED.store(1, Ordering::Release);
    LIVE.store(1, Ordering::Release);
    PEAK_LIVE.store(1, Ordering::Release);
    // The permanent init Process must be part of the heap baseline, while
    // syscall state must exist before the runnable context is published.
    crate::syscall::init();
    scheduler::activate_user_init(scheduler::UserInitActivation {
        process_id: id.raw(),
        argument0: if cfg!(feature = "process-terminate-self-test") {
            PROCESS_TERMINATE_SELF_TEST_INIT_ARGUMENT
        } else {
            u64::from(system_root.raw())
        },
        entry: info.entry,
        code_start: info.code_start,
        code_end: info.code_end,
        user_stack_start: info.stack_start,
        user_stack_end: info.stack_end,
        translation,
    });
    crate::arch::aarch64::restore_daif(saved_daif);
}

pub fn spawn_image(
    startup_handle: HandleValue,
    image_id: UserImageId,
) -> Result<ProcessId, SpawnError> {
    assert_initialized();
    if !crate::arch::aarch64::irq_is_masked() || !current_is_init() || !image_id.is_spawnable() {
        return Err(SpawnError::InvalidState);
    }
    #[cfg(feature = "input-server-runtime")]
    if image_id == UserImageId::InputServer
        && slots_ref().iter().any(|slot| {
            slot.process
                .as_ref()
                .is_some_and(|process| process.image_id == UserImageId::InputServer)
        })
    {
        // Input delivery has exactly one authenticated consumer generation.
        // A replacement becomes spawnable only after the previous generation
        // has completed teardown and vacated its process slot.
        return Err(SpawnError::InvalidState);
    }
    #[cfg(feature = "storage-server-runtime")]
    if image_id == UserImageId::StorageServer
        && slots_ref().iter().any(|slot| {
            slot.process
                .as_ref()
                .is_some_and(|process| process.image_id == UserImageId::StorageServer)
        })
    {
        return Err(SpawnError::InvalidState);
    }
    #[cfg(feature = "androidbox-process0")]
    if image_id == UserImageId::AndroidApp
        && slots_ref().iter().any(|slot| {
            slot.process
                .as_ref()
                .is_some_and(|process| process.image_id == UserImageId::AndroidApp)
        })
    {
        return Err(SpawnError::InvalidState);
    }
    let Some(process_slot) = first_available_dynamic_slot(slots_ref()) else {
        SPAWN_WAITS.fetch_add(1, Ordering::Relaxed);
        CAPACITY_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return Err(SpawnError::ShouldWait);
    };
    let dynamic_slot = process_slot - FIRST_DYNAMIC_PROCESS_SLOT;
    match with_current_handles(|table| {
        table
            .get(startup_handle, Rights::TRANSFER)
            .map(|object| matches!(object, KernelObject::Channel(_)))
    }) {
        Ok(true) => {}
        Ok(false) => return Err(SpawnError::InvalidState),
        Err(HandleError::InvalidHandle) => return Err(SpawnError::NotFound),
        Err(HandleError::AccessDenied | HandleError::RightsEscalation) => {
            return Err(SpawnError::PermissionDenied);
        }
        Err(HandleError::TableFull) => return Err(SpawnError::InvalidState),
    }

    let prepared = crate::userboot::prepare_image(image_id).map_err(SpawnError::Image)?;
    commit_prepared_spawn(
        startup_handle,
        image_id,
        process_slot,
        dynamic_slot,
        prepared,
    )
}

// Keep ELF parsing/address-space construction and process publication in
// separate kernel-stack frames. In an unoptimized kernel the two bounded
// transactions each carry substantial rollback state; nesting both beneath
// the EL0 exception/syscall frames can otherwise exhaust a 16 KiB per-process
// kernel stack even though neither transaction is individually unbounded.
#[inline(never)]
fn commit_prepared_spawn(
    startup_handle: HandleValue,
    image_id: UserImageId,
    process_slot: usize,
    dynamic_slot: usize,
    prepared: Box<PreparedUserProcess>,
) -> Result<ProcessId, SpawnError> {
    let pending = match scheduler::prepare_user_thread(dynamic_slot) {
        Ok(pending) => pending,
        Err(()) => {
            let PreparedUserProcess { address_space, .. } = take_box(prepared);
            destroy_unpublished(address_space);
            return Err(SpawnError::OutOfMemory);
        }
    };

    let handles = match HandleTable::try_new_boxed() {
        Some(handles) => handles,
        None => {
            drop(pending);
            let PreparedUserProcess { address_space, .. } = take_box(prepared);
            destroy_unpublished(address_space);
            return Err(SpawnError::OutOfMemory);
        }
    };
    let PreparedUserProcess {
        info,
        address_space,
    } = take_box(prepared);
    if info.image_id != image_id {
        panic!("prepared child image ID changed before publication");
    }

    let generation = slots_ref()[process_slot].generation;
    if first_available_dynamic_slot(slots_ref()) != Some(process_slot)
        || slots_ref()[process_slot].process.is_some()
        || slots_ref()[process_slot].completion.is_pending()
    {
        panic!("child process slot changed during IRQ-masked construction");
    }
    let id = ProcessId::new(process_slot, generation);
    let translation = address_space.translation_context();
    if !address_space_is_isolated_from_live(&address_space, slots_ref()) {
        panic!("dynamic child shared an ASID, root, or user frame with a live process");
    }
    let process = Process {
        id,
        image_id,
        address_space: Some(address_space),
        handles,
        startup_handle: None,
        is_init: false,
    };
    let mut process = match try_box(process) {
        Ok(process) => process,
        Err(mut process) => {
            drop(pending);
            destroy_unpublished(
                process
                    .address_space
                    .take()
                    .unwrap_or_else(|| panic!("failed Process lost its address space")),
            );
            return Err(SpawnError::OutOfMemory);
        }
    };

    let child_asid = info.asid;
    let child_root = info.user_root;
    let child_code_physical = info.code_physical;
    let child_stack_physical = info.stack_physical;
    if child_asid == mmu::KERNEL_ASID
        || child_asid == mmu::INIT_USER_ASID
        || child_root == INIT_ROOT.load(Ordering::Relaxed)
        || child_code_physical == INIT_CODE_PHYSICAL.load(Ordering::Relaxed)
        || child_stack_physical == INIT_STACK_PHYSICAL.load(Ordering::Relaxed)
    {
        panic!("dynamic child did not receive isolated address-space resources");
    }

    // All recoverable allocations and image validation precede this commit.
    // Reserving the empty child table first means a successful source take is
    // followed only by an infallible destination insertion and publication.
    let child_startup_handle = {
        let reservation = process
            .handles
            .reserve_slot()
            .unwrap_or_else(|_| panic!("new child handle table had no startup slot"));
        let owned = match with_current_handles(|table| {
            table.take_owned(startup_handle, Rights::TRANSFER)
        }) {
            Ok(owned) => owned,
            Err(error) => {
                drop(reservation);
                drop(pending);
                destroy_unpublished_process(process);
                return Err(match error {
                    HandleError::InvalidHandle => SpawnError::NotFound,
                    HandleError::AccessDenied | HandleError::RightsEscalation => {
                        SpawnError::PermissionDenied
                    }
                    HandleError::TableFull => SpawnError::InvalidState,
                });
            }
        };
        #[cfg(feature = "androidbox-process0")]
        if image_id == UserImageId::AndroidApp {
            let (object, rights) = owned.into_parts();
            if rights != Rights::CHANNEL_DEFAULT {
                drop(reservation);
                drop(pending);
                destroy_unpublished_process(process);
                return Err(SpawnError::PermissionDenied);
            }
            reservation.insert_parts(object, Rights::ANDROID_APP_CHANNEL)
        } else {
            reservation.insert(owned)
        }
        #[cfg(not(feature = "androidbox-process0"))]
        reservation.insert(owned)
    };
    if process.handles.len() != 1 {
        panic!("child inherited handles outside the explicit startup move");
    }
    process.startup_handle = Some(child_startup_handle);

    slots_mut()[process_slot].process = Some(process);
    STARTUP_MOVES.fetch_add(1, Ordering::Relaxed);
    CHILD_HANDLE_ISOLATED.store(true, Ordering::Relaxed);
    CHILD_FRAMES_ISOLATED.store(true, Ordering::Relaxed);
    ROOT_ISOLATED.store(true, Ordering::Relaxed);
    let previous_created = CREATED.fetch_add(1, Ordering::Relaxed);
    let child_spawn_index = previous_created
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
        .unwrap_or_else(|| panic!("dynamic child spawn accounting underflowed"));
    if let Some(evidence_slot) = CHILD_SPAWN_PIDS.get(child_spawn_index) {
        evidence_slot.store(id.raw(), Ordering::Relaxed);
    }
    if let Some(evidence_slot) = CHILD_SPAWN_IMAGES.get(child_spawn_index) {
        evidence_slot.store(image_id.raw(), Ordering::Relaxed);
    }
    #[cfg(feature = "unified-product-liveness-runtime")]
    if child_spawn_index == CHILD_SPAWN_EVIDENCE_CAPACITY {
        if image_id != UserImageId::StorageServer {
            panic!("M68 eleventh child was not the StorageServer replacement");
        }
        UNIFIED_PRODUCT_STORAGE_REPLACEMENT_PID.store(id.raw(), Ordering::Relaxed);
        UNIFIED_PRODUCT_STORAGE_REPLACEMENT_IMAGE.store(image_id.raw(), Ordering::Relaxed);
    }
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    if child_spawn_index == CHILD_SPAWN_EVIDENCE_CAPACITY + 1 {
        if image_id != UserImageId::StorageServer {
            panic!("M73 twelfth child was not the second StorageServer replacement");
        }
        UNIFIED_PRODUCT_STORAGE_SECOND_REPLACEMENT_PID.store(id.raw(), Ordering::Relaxed);
        UNIFIED_PRODUCT_STORAGE_SECOND_REPLACEMENT_IMAGE.store(image_id.raw(), Ordering::Relaxed);
    }
    #[cfg(all(
        feature = "service-dependency-runtime",
        not(feature = "unified-product-liveness-runtime")
    ))]
    if child_spawn_index == CHILD_SPAWN_EVIDENCE_CAPACITY {
        if image_id != UserImageId::InputServer {
            panic!("M49 eleventh child was not the InputServer replacement");
        }
        SERVICE_DEPENDENCY_INPUT_REPLACEMENT_PID.store(id.raw(), Ordering::Relaxed);
        SERVICE_DEPENDENCY_INPUT_REPLACEMENT_IMAGE.store(image_id.raw(), Ordering::Relaxed);
    }
    let live = LIVE.fetch_add(1, Ordering::Relaxed) + 1;
    PEAK_LIVE.fetch_max(live, Ordering::Relaxed);
    if previous_created == 1 {
        FIRST_CHILD_PID.store(id.raw(), Ordering::Relaxed);
        FIRST_CHILD_ASID.store(u64::from(child_asid), Ordering::Relaxed);
        FIRST_CHILD_ROOT.store(child_root, Ordering::Relaxed);
        FIRST_CHILD_CODE_PHYSICAL.store(child_code_physical, Ordering::Relaxed);
        FIRST_CHILD_STACK_PHYSICAL.store(child_stack_physical, Ordering::Relaxed);
    } else if previous_created == 2 {
        SECOND_CHILD_PID.store(id.raw(), Ordering::Relaxed);
        SLOT_REUSED.store(
            id.slot() == ProcessId(FIRST_CHILD_PID.load(Ordering::Relaxed)).slot(),
            Ordering::Relaxed,
        );
        ASID_REUSED.store(
            u64::from(child_asid) == FIRST_CHILD_ASID.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        STALE_REJECTED.store(
            lookup(ProcessId(FIRST_CHILD_PID.load(Ordering::Relaxed))).is_none(),
            Ordering::Relaxed,
        );
        // Reusing physical frame addresses after completed teardown is legal.
        let _ = (
            child_root == FIRST_CHILD_ROOT.load(Ordering::Relaxed),
            child_code_physical == FIRST_CHILD_CODE_PHYSICAL.load(Ordering::Relaxed),
            child_stack_physical == FIRST_CHILD_STACK_PHYSICAL.load(Ordering::Relaxed),
        );
    }
    LAST_CHILD_ASID.store(u64::from(child_asid), Ordering::Relaxed);
    let live_isolation = live_dynamic_isolation(slots_ref());
    if live_isolation.live == DYNAMIC_PROCESS_CAPACITY {
        DISTINCT_LIVE_SLOTS.store(true, Ordering::Relaxed);
        DISTINCT_LIVE_ASIDS.store(live_isolation.asids, Ordering::Relaxed);
        DISTINCT_LIVE_ROOTS.store(live_isolation.roots, Ordering::Relaxed);
        DISTINCT_LIVE_FRAMES.store(live_isolation.frames, Ordering::Relaxed);
        if !live_isolation.asids || !live_isolation.roots || !live_isolation.frames {
            panic!("concurrent children did not own pairwise-isolated address spaces");
        }
    }

    scheduler::activate_user_process(
        pending,
        dynamic_slot,
        id.raw(),
        info.entry,
        info.code_start,
        info.code_end,
        info.stack_start,
        info.stack_end,
        translation,
        u64::from(child_startup_handle.raw()),
        image_id.raw(),
    );
    Ok(id)
}

pub fn reap_terminal_children() -> usize {
    if !INITIALIZED.load(Ordering::Acquire) {
        return 0;
    }
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    if mmu::current_translation_context() != mmu::kernel_translation_context() {
        crate::arch::aarch64::restore_daif(saved_daif);
        return 0;
    }
    let mut reaped = 0;
    while reaped < DYNAMIC_PROCESS_CAPACITY {
        let Some(terminal) = scheduler::terminal_user_snapshot() else {
            break;
        };
        reap_terminal_child(terminal);
        reaped += 1;
    }
    crate::arch::aarch64::restore_daif(saved_daif);
    reaped
}

fn reap_terminal_child(terminal: scheduler::TerminalUserSnapshot) {
    let id = ProcessId(terminal.process_id);
    let termination_reason = ProcessTerminationReason::from_raw(terminal.reason)
        .unwrap_or_else(|| panic!("terminal scheduler state carried an invalid reason"));
    let process_slot = id
        .slot()
        .filter(|slot| *slot >= FIRST_DYNAMIC_PROCESS_SLOT)
        .unwrap_or_else(|| panic!("terminal scheduler process ID did not identify a child slot"));
    let stopped_thread = scheduler::detach_terminal_user(id.raw())
        .unwrap_or_else(|| panic!("terminal child could not be detached from scheduler"));
    if stopped_thread.translation != terminal.translation
        || stopped_thread.process_id != id.raw()
        || (stopped_thread.selections == 0
            && termination_reason != ProcessTerminationReason::Killed)
        || (termination_reason != ProcessTerminationReason::Faulted
            && (terminal.fault_reason != 0
                || terminal.fault_esr != 0
                || terminal.fault_far != 0
                || terminal.fault_elr != 0
                || terminal.fault_sp_el0 != 0))
    {
        panic!("stopped scheduler thread evidence was inconsistent");
    }

    let (mut process, generation) = {
        let slot = &mut slots_mut()[process_slot];
        let process = slot
            .process
            .take()
            .unwrap_or_else(|| panic!("terminal child had no Process owner"));
        (process, slot.generation)
    };
    if process.id != id
        || process.image_id == UserImageId::Init
        || process.is_init
        || id.generation() != generation
    {
        panic!("terminal Process generation or role was inconsistent");
    }
    let image_id = process.image_id;
    #[cfg(feature = "androidbox-restart0")]
    let android_app_restart_fault_reaped = image_id == UserImageId::AndroidApp
        && crate::syscall::record_android_app_restart_fault_reap(
            id.raw(),
            terminal.exit_code,
            termination_reason,
            terminal.fault_reason,
            terminal.fault_esr,
            terminal.fault_far,
            terminal.fault_elr,
            terminal.fault_sp_el0,
            terminal.code_start,
            terminal.code_end,
            terminal.stack_start,
            terminal.stack_end,
        );
    #[cfg(feature = "androidbox-process0")]
    if image_id == UserImageId::AndroidApp
        && crate::package_manager::revoke_image_grant_for_execution_owner(id.raw())
    {
        crate::kprintln!(
            "ANDROID_PACKAGE_IMAGE_GRANT_REVOKE_OK owner={} image=android-app reason=process-exit",
            id.raw(),
        );
    }
    #[cfg(feature = "androidbox-restart0")]
    if image_id == UserImageId::AndroidApp
        && !android_app_restart_fault_reaped
        && crate::package_manager::revoke_restart_image_grant_for_execution_owner(id.raw())
    {
        crate::kprintln!(
            "ANDROID_PACKAGE_RESTART_IMAGE_GRANT_REVOKE_OK owner={} image=android-app reason=unauthorized-process-exit",
            id.raw(),
        );
    }
    let mut surface_capability = None;
    #[cfg(feature = "input-server-runtime")]
    let mut input_capability = None;
    #[cfg(feature = "storage-server-runtime")]
    let mut storage_capability: Option<StorageVolumeCapability> = None;
    for (object, rights) in process.handles.live_entries() {
        if let Some(capability) = object.as_surface()
            && (surface_capability.replace(capability.clone()).is_some()
                || rights != Rights::SURFACE_DEFAULT)
        {
            panic!("terminal process owned an invalid surface capability set");
        }
        #[cfg(feature = "input-server-runtime")]
        if let Some(capability) = object.as_input()
            && (input_capability.replace(capability.clone()).is_some()
                || rights != Rights::INPUT_DEFAULT)
        {
            panic!("terminal process owned an invalid input capability set");
        }
        #[cfg(feature = "storage-server-runtime")]
        if let Some(capability) = object.as_storage_volume()
            && (storage_capability.replace(*capability).is_some()
                || rights != Rights::STORAGE_VOLUME_DEFAULT)
        {
            panic!("terminal process owned an invalid storage capability set");
        }
    }
    if surface_capability.is_some() && image_id != UserImageId::SurfaceServer {
        panic!("non-SurfaceServer process owned the surface capability");
    }
    if image_id == UserImageId::SurfaceServer {
        let evidence =
            crate::display::degrade_surface_for_process(id.raw()).unwrap_or_else(|error| {
                panic!(
                    "surface-server termination could not freeze its scene: {}",
                    error.as_str()
                )
            });
        if let Some(evidence) = evidence {
            crate::kprintln!(
                "SURFACE_DEGRADED owner=userspace reason=process-exit pid={} session={} last_frame={} commits={} pending={} peer_closed_edge={} scene_digest={:#018x} scanout_digest={:#018x}",
                evidence.process_id,
                evidence.session_id,
                evidence.last_frame_id.unwrap_or(0),
                evidence.commits,
                evidence.input.pending,
                u8::from(evidence.peer_closed_edge),
                evidence.scene_digest,
                evidence.scanout_digest,
            );
        }
    }
    #[cfg(feature = "input-server-runtime")]
    {
        if input_capability.is_some() && image_id != UserImageId::InputServer {
            panic!("non-InputServer process owned the input capability");
        }
        if image_id == UserImageId::InputServer {
            let evidence = crate::input_stream::release_process(id.raw());
            if evidence.is_some() != input_capability.is_some()
                || evidence.as_ref().is_some_and(|evidence| {
                    input_capability.as_ref().is_none_or(|capability| {
                        evidence.session_id != capability.session_id()
                            || evidence.process_id != capability.process_id()
                    })
                })
            {
                panic!("input-server termination and capability ownership diverged");
            }
            if let Some(evidence) = evidence {
                crate::kprintln!(
                    "INPUT_RELEASE_OK reason=process-exit pid={} session={} acquisition_floor={} release_floor={} discarded={} failed={} peer_closed_edge={}",
                    evidence.process_id,
                    evidence.session_id,
                    evidence.acquisition_floor,
                    evidence.release_floor,
                    evidence.discarded,
                    u8::from(evidence.failed),
                    u8::from(evidence.peer_closed_edge),
                );
            }
        }
    }
    #[cfg(feature = "storage-server-runtime")]
    {
        if storage_capability.is_some() && image_id != UserImageId::StorageServer {
            panic!("non-StorageServer process owned the storage volume capability");
        }
        if image_id == UserImageId::StorageServer {
            let released = bndroid_kernel::storage_broker::release_process(id.raw());
            if released != storage_capability.is_some() {
                panic!("StorageServer termination and volume capability ownership diverged");
            }
        }
    }
    let address_space = process
        .address_space
        .take()
        .unwrap_or_else(|| panic!("terminal Process lost its address space"));
    if address_space.translation_context() != terminal.translation {
        panic!("terminal Process and scheduler translation contexts disagreed");
    }
    // Mapping metadata, not the handle table, is authoritative: userspace may
    // have closed the handle while the address space still pins its alias.
    // Retain fixed-size clones across address-space destruction so all dead
    // consumer leaves and stale ASID translations disappear before producer
    // permissions or BufferQueue state are changed.
    let graphics_consumer_pins = (image_id == UserImageId::SurfaceServer)
        .then(|| {
            let mut pins = address_space.pinned_graphics_mappings();
            for pin in &mut pins {
                if pin
                    .as_ref()
                    .is_some_and(|(_, mapping)| mapping.role != GraphicsMappingRole::Consumer)
                {
                    *pin = None;
                }
            }
            pins
        })
        .filter(|pins| pins.iter().any(Option::is_some));
    let graphics_consumer_died = graphics_consumer_pins.is_some();
    let expected_destroyed_frames = address_space
        .mapped_page_count()
        .checked_add(address_space.private_table_frames())
        .unwrap_or_else(|| panic!("terminal address-space frame count overflowed"));
    let heap_before = crate::kernel_heap::stats()
        .unwrap_or_else(|| panic!("kernel heap missing during process reap"))
        .free_bytes;
    // Dropping Process closes every still-owned handle. User code is not
    // required to close handles before exit, and a forgotten handle must not
    // turn process termination into a kernel panic.
    drop(process);
    CHILD_SELECTIONS.fetch_add(stopped_thread.selections, Ordering::Relaxed);
    drop(stopped_thread);

    let frames_before_destroy = memory::frame_allocator_stats().allocated_frames;
    let stopped = unsafe { StoppedAddressSpaceProof::new(terminal.translation) };
    address_space
        .destroy(stopped)
        .unwrap_or_else(|failure| destroy_failure_is_fatal(failure));
    if let Some(pins) = graphics_consumer_pins {
        recover_graphics_consumer_owner_death(id.raw(), pins);
    }
    let wake_completions = scheduler::wake_object_waiters();
    if graphics_consumer_died {
        GRAPHICS_CONSUMER_DEATH_WAKE_COMPLETIONS
            .fetch_add(wake_completions as u64, Ordering::Relaxed);
    }
    let frames_after_destroy = memory::frame_allocator_stats().allocated_frames;
    if frames_before_destroy.checked_sub(frames_after_destroy) != Some(expected_destroyed_frames) {
        FRAME_RESTORED.store(false, Ordering::Relaxed);
        FRAME_DESTROY_DELTA_VALID.store(false, Ordering::Relaxed);
    }
    let heap_after = crate::kernel_heap::stats()
        .unwrap_or_else(|| panic!("kernel heap missing after process reap"))
        .free_bytes;
    if heap_after < heap_before {
        HEAP_RESTORED.store(false, Ordering::Relaxed);
    }

    EXITED.fetch_add(1, Ordering::Relaxed);
    REAPED.fetch_add(1, Ordering::Relaxed);
    match termination_reason {
        ProcessTerminationReason::Exited => {
            TERMINATED_EXITED.fetch_add(1, Ordering::Relaxed);
        }
        ProcessTerminationReason::Faulted => {
            TERMINATED_FAULTED.fetch_add(1, Ordering::Relaxed);
        }
        ProcessTerminationReason::Killed => {
            TERMINATED_KILLED.fetch_add(1, Ordering::Relaxed);
        }
    }
    let old_live = LIVE.fetch_sub(1, Ordering::AcqRel);
    if !(2..=PROCESS_CAPACITY as u64).contains(&old_live) {
        panic!("process live count did not include init plus live children");
    }
    if termination_reason == ProcessTerminationReason::Exited
        && terminal.exit_code != bndr_abi::CHILD_EXIT_MAGIC
    {
        NONSTANDARD_EXIT_CODES.fetch_add(1, Ordering::Relaxed);
    }

    let completion = ProcessCompletion {
        id,
        exit_code: terminal.exit_code,
        reason: terminal.reason,
    };
    let pending_target = scheduler::process_wait_snapshot().pending_target;
    let woke_waiter =
        scheduler::wake_user_process_wait(id.raw(), terminal.exit_code, terminal.reason);
    if pending_target != 0 && pending_target != id.raw() {
        NON_TARGET_REAPS.fetch_add(1, Ordering::Relaxed);
    }
    let slot = &mut slots_mut()[process_slot];
    if slot.process.is_some() || slot.completion.is_pending() || slot.generation != generation {
        panic!("child process slot changed during IRQ-masked teardown");
    }
    if woke_waiter {
        retire_dynamic_generation(slot, generation);
        WAIT_COMPLETED.fetch_add(1, Ordering::Relaxed);
    } else if slot.completion.publish(id.raw(), completion).is_err() {
        panic!("child completion tombstone publication failed");
    }
}

fn recover_graphics_consumer_owner_death(
    consumer_pid: u64,
    pins: [Option<(GraphicsBuffer, GraphicsMappingSnapshot)>; mmu::USER_GRAPHICS_MAPPING_CAPACITY],
) {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("graphics consumer owner-death cleanup ran with IRQ enabled");
    }
    GRAPHICS_CONSUMER_DEATHS.fetch_add(1, Ordering::Relaxed);
    for (buffer, consumer_mapping) in pins.into_iter().flatten() {
        if consumer_mapping.role != GraphicsMappingRole::Consumer
            || consumer_mapping.access != GraphicsMappingAccess::ReadOnly
            || consumer_mapping.producer_pid != buffer.producer_pid()
            || consumer_mapping.identity != buffer.identity()
            || consumer_mapping.backing_address != buffer.backing_address()
            || consumer_mapping.producer_pid == consumer_pid
        {
            panic!("SurfaceServer teardown found an invalid consumer graphics mapping");
        }
        GRAPHICS_CONSUMER_DEATH_MAPPINGS.fetch_add(1, Ordering::Relaxed);
        let pending = buffer
            .consumer_owner_death_frame(consumer_pid)
            .unwrap_or_else(|error| {
                panic!(
                    "SurfaceServer teardown could not preflight its graphics frame: {}",
                    error.as_str()
                )
            });
        let Some(pending) = pending else {
            continue;
        };

        let producer_status =
            match restore_graphics_buffer_producer_mapping(buffer.producer_pid(), &buffer) {
                Ok(mapping) => {
                    if mapping.role != GraphicsMappingRole::Producer
                        || mapping.access != GraphicsMappingAccess::ReadWrite
                        || mapping.identity != buffer.identity()
                        || mapping.backing_address != consumer_mapping.backing_address
                    {
                        panic!(
                            "owner-death producer restore returned inconsistent mapping evidence"
                        );
                    }
                    GRAPHICS_CONSUMER_DEATH_PRODUCER_RESTORES.fetch_add(1, Ordering::Relaxed);
                    "restored"
                }
                Err(ProcessGraphicsMappingError::NotFound) => {
                    GRAPHICS_CONSUMER_DEATH_PRODUCER_ORPHANS.fetch_add(1, Ordering::Relaxed);
                    "orphan"
                }
                Err(error) => {
                    panic!(
                        "SurfaceServer teardown could not restore the graphics producer: {}",
                        error.as_str()
                    )
                }
            };
        buffer
            .abandon_consumer_owner_death(pending, consumer_pid)
            .unwrap_or_else(|error| {
                panic!(
                    "preflighted SurfaceServer graphics abandonment failed: {}",
                    error.as_str()
                )
            });
        if pending.was_acquired() {
            GRAPHICS_CONSUMER_DEATH_ACQUIRED.fetch_add(1, Ordering::Relaxed);
        } else {
            GRAPHICS_CONSUMER_DEATH_QUEUED.fetch_add(1, Ordering::Relaxed);
        }
        GRAPHICS_CONSUMER_DEATH_RELEASES.fetch_add(1, Ordering::Relaxed);
        crate::syscall::record_graphics_buffer_owner_death_release();
        crate::kprintln!(
            "GRAPHICS_CONSUMER_ABANDON_OK consumer_pid={} producer_pid={} generation={} state={} producer={} consumer_unmapped=1 writable=1",
            consumer_pid,
            buffer.producer_pid(),
            pending.generation(),
            if pending.was_acquired() {
                "acquired"
            } else {
                "queued"
            },
            producer_status,
        );
    }
}

/// Marks one live generation-qualified dynamic child for monitor-side reap.
pub fn terminate_child(raw: u64) -> Result<(), TerminateChildError> {
    assert_initialized();
    if !crate::arch::aarch64::irq_is_masked() || !current_is_init() {
        return Err(TerminateChildError::InvalidState);
    }
    let id = ProcessId(raw);
    let Some(process_slot) = id.slot().filter(|slot| *slot >= FIRST_DYNAMIC_PROCESS_SLOT) else {
        return Err(TerminateChildError::NotFound);
    };
    if id.generation() == 0 {
        return Err(TerminateChildError::NotFound);
    }
    let slot = &slots_ref()[process_slot];
    if id.generation() != slot.generation {
        return Err(TerminateChildError::NotFound);
    }
    let Some(process) = slot.process.as_ref() else {
        return Err(TerminateChildError::NotFound);
    };
    if process.id != id || process.is_init || process.image_id == UserImageId::Init {
        panic!("forced termination target disagreed with its process-table identity");
    }
    match scheduler::terminate_user_process(id.raw()) {
        Ok(()) => Ok(()),
        Err(scheduler::TerminateUserError::AlreadyTerminal) => {
            Err(TerminateChildError::InvalidState)
        }
        Err(scheduler::TerminateUserError::InvalidState) => Err(TerminateChildError::InvalidState),
        Err(scheduler::TerminateUserError::NotFound) => {
            panic!("live process-table entry had no scheduler context during termination")
        }
    }
}

/// Reports whether the exact generation-qualified M61 retirement target still
/// owns a process-table entry. A missing result is publishable only after the
/// ordinary monitor reaper has closed its handles and destroyed its address
/// space; broker unbinding alone is intentionally insufficient.
#[cfg(feature = "storage-server-owner-liveness-runtime")]
pub fn storage_watchdog_target_present(
    raw: u64,
    broker_epoch: u64,
) -> Result<bool, TerminateChildError> {
    if !scheduler::storage_watchdog_monitor_active() || broker_epoch == 0 {
        return Err(TerminateChildError::InvalidState);
    }
    let id = ProcessId(raw);
    let Some(process_slot) = id.slot().filter(|slot| *slot >= FIRST_DYNAMIC_PROCESS_SLOT) else {
        return Err(TerminateChildError::NotFound);
    };
    if id.generation() == 0 {
        return Err(TerminateChildError::NotFound);
    }
    let slot = &slots_ref()[process_slot];
    if id.generation() != slot.generation {
        return Ok(false);
    }
    let Some(process) = slot.process.as_ref() else {
        return Ok(false);
    };
    let holds_volume = validate_storage_watchdog_target(process, id, broker_epoch)?;
    if !holds_volume {
        return Err(TerminateChildError::InvalidState);
    }
    Ok(true)
}

/// Marks the exact M61 StorageServer ticket owner terminal from monitor
/// context. Broker release, accepted-session close, and address-space teardown
/// remain exclusively owned by `reap_terminal_children`.
#[cfg(feature = "storage-server-owner-liveness-runtime")]
pub fn terminate_storage_owner_from_watchdog(
    raw: u64,
    broker_epoch: u64,
) -> Result<(), StorageWatchdogTerminateError> {
    if !scheduler::storage_watchdog_monitor_active() || broker_epoch == 0 {
        return Err(StorageWatchdogTerminateError::InvalidState);
    }
    let id = ProcessId(raw);
    let Some(process_slot) = id.slot().filter(|slot| *slot >= FIRST_DYNAMIC_PROCESS_SLOT) else {
        return Err(StorageWatchdogTerminateError::NotFound);
    };
    if id.generation() == 0 {
        return Err(StorageWatchdogTerminateError::NotFound);
    }
    let slot = &slots_ref()[process_slot];
    if id.generation() != slot.generation {
        return Err(StorageWatchdogTerminateError::NotFound);
    }
    let Some(process) = slot.process.as_ref() else {
        return Err(StorageWatchdogTerminateError::NotFound);
    };
    let holds_volume = validate_storage_watchdog_target(process, id, broker_epoch)
        .map_err(|_| StorageWatchdogTerminateError::InvalidState)?;
    if !holds_volume {
        return Err(StorageWatchdogTerminateError::InvalidState);
    }
    match scheduler::terminate_user_process_from_storage_watchdog(id.raw()) {
        Ok(()) => Ok(()),
        Err(scheduler::TerminateUserError::AlreadyTerminal) => {
            Err(StorageWatchdogTerminateError::AlreadyTerminal)
        }
        Err(scheduler::TerminateUserError::InvalidState) => {
            Err(StorageWatchdogTerminateError::InvalidState)
        }
        Err(scheduler::TerminateUserError::NotFound) => {
            panic!("M61 live process-table ticket had no scheduler context")
        }
    }
}

#[cfg(feature = "storage-server-owner-liveness-runtime")]
fn validate_storage_watchdog_target(
    process: &Process,
    id: ProcessId,
    broker_epoch: u64,
) -> Result<bool, TerminateChildError> {
    if process.id != id
        || process.is_init
        || process.image_id != UserImageId::StorageServer
        || broker_epoch == 0
    {
        return Err(TerminateChildError::InvalidState);
    }
    let mut storage_capability = None;
    for (object, rights) in process.handles.live_entries() {
        if let Some(capability) = object.as_storage_volume()
            && (storage_capability.replace(*capability).is_some()
                || rights != Rights::STORAGE_VOLUME_DEFAULT
                || capability.owner_pid() != id.raw()
                || capability.epoch() != broker_epoch)
        {
            return Err(TerminateChildError::InvalidState);
        }
    }
    // HandleClose rejects a live volume capability. Its continued presence is
    // therefore part of the exact process-lifetime retirement proof.
    Ok(storage_capability.is_some())
}

/// Classifies a generation-qualified child wait while local IRQ is masked.
/// A retained completion closes the race where the monitor reaps a child
/// before init reaches `ProcessWait`; a live result is immediately followed by
/// scheduler wait-token publication in the same IRQ-masked syscall.
pub fn wait_child(raw: u64) -> Result<ChildWait, ChildWaitError> {
    assert_initialized();
    if !crate::arch::aarch64::irq_is_masked() || !current_is_init() {
        return Err(ChildWaitError::InvalidState);
    }
    WAIT_CALLS.fetch_add(1, Ordering::Relaxed);
    let id = ProcessId(raw);
    let Some(process_slot) = id.slot().filter(|slot| *slot >= FIRST_DYNAMIC_PROCESS_SLOT) else {
        WAIT_STALE.fetch_add(1, Ordering::Relaxed);
        return Err(ChildWaitError::NotFound);
    };
    if id.generation() == 0 {
        WAIT_STALE.fetch_add(1, Ordering::Relaxed);
        return Err(ChildWaitError::NotFound);
    }
    let slot = &mut slots_mut()[process_slot];
    if id.generation() != slot.generation {
        WAIT_STALE.fetch_add(1, Ordering::Relaxed);
        return Err(ChildWaitError::NotFound);
    }
    if let Some(completion) = slot.completion.take(id.raw()) {
        let reason = ProcessTerminationReason::from_raw(completion.reason)
            .unwrap_or_else(|| panic!("process completion carried an invalid termination reason"));
        if completion.id != id {
            panic!("process completion tombstone was inconsistent");
        }
        retire_dynamic_generation(slot, id.generation());
        WAIT_COMPLETED.fetch_add(1, Ordering::Relaxed);
        WAIT_IMMEDIATE.fetch_add(1, Ordering::Relaxed);
        return Ok(ChildWait::Completed {
            exit_code: completion.exit_code,
            reason,
        });
    }
    if slot
        .process
        .as_ref()
        .is_some_and(|process| process.id == id)
    {
        return Ok(ChildWait::Live);
    }
    WAIT_STALE.fetch_add(1, Ordering::Relaxed);
    Err(ChildWaitError::NotFound)
}

pub fn with_current_handles<R>(
    operation: impl FnOnce(&mut HandleTable<KernelObject, HANDLE_CAPACITY>) -> R,
) -> R {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("process handle table accessed with IRQ enabled");
    }
    let id = ProcessId(
        scheduler::current_process_id()
            .unwrap_or_else(|| panic!("handle syscall executed outside a user process")),
    );
    let process = lookup_mut(id)
        .unwrap_or_else(|| panic!("scheduler current process ID was stale or missing"));
    operation(&mut process.handles)
}

pub fn poll_object_signals(
    raw_process_id: u64,
    raw_handle: u64,
) -> Result<ObjectSignals, ObjectPollError> {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("object signals polled with IRQ enabled");
    }
    let process = lookup(ProcessId(raw_process_id)).ok_or(ObjectPollError::NotFound)?;
    let raw_handle = u32::try_from(raw_handle).map_err(|_| ObjectPollError::NotFound)?;
    let handle = HandleValue::from_raw(raw_handle);
    if !handle.is_valid() {
        return Err(ObjectPollError::NotFound);
    }
    match process.handles.get(handle, Rights::WAIT) {
        Ok(object) => Ok(object.signals()),
        Err(HandleError::InvalidHandle) => Err(ObjectPollError::NotFound),
        Err(HandleError::AccessDenied | HandleError::RightsEscalation) => {
            Err(ObjectPollError::PermissionDenied)
        }
        Err(HandleError::TableFull) => panic!("read-only handle lookup reported table full"),
    }
}

pub fn is_init_process_id(raw_process_id: u64) -> bool {
    lookup(ProcessId(raw_process_id)).is_some_and(|process| process.is_init)
}

pub fn init_handle_count() -> usize {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let count = slots_ref()[INIT_PROCESS_SLOT]
        .process
        .as_ref()
        .map_or(0, |process| process.handles.len());
    crate::arch::aarch64::restore_daif(saved_daif);
    count
}

pub fn current_is_init() -> bool {
    let Some(raw) = scheduler::current_process_id() else {
        return false;
    };
    lookup(ProcessId(raw)).is_some_and(|process| process.is_init)
}

/// Authenticates the sole live userspace SurfaceServer image.
///
/// Both sides of the comparison are generation-qualified PIDs resolved inside
/// one IRQ-masked lifecycle domain. This does not depend on spawn order and
/// cannot accidentally grant Surface authority to the ServiceManager.
pub fn current_is_surface_server() -> bool {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("surface server identity checked with IRQ enabled");
    }
    let Some(current_pid) = scheduler::current_process_id() else {
        return false;
    };
    let Some(expected_pid) = unique_live_process_by_image(slots_ref(), UserImageId::SurfaceServer)
        .map(|process| process.id.raw())
    else {
        return false;
    };
    if current_pid != expected_pid {
        return false;
    }
    lookup(ProcessId(current_pid)).is_some_and(|process| {
        process.id.raw() == expected_pid
            && process.image_id == UserImageId::SurfaceServer
            && !process.is_init
    })
}

/// Authenticates the sole live generation of the mobile Launcher.
///
/// Android package relaunch is intentionally narrower than the read-only
/// package snapshot: only the current Launcher may request a fresh durable
/// verification and Activity execution. Both identities are compared as
/// generation-qualified PIDs inside the IRQ-masked syscall domain.
#[cfg(feature = "androidbox-apk-install0")]
pub fn current_is_android_package_launcher() -> bool {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("Android package launcher identity checked with IRQ enabled");
    }
    let Some(current_pid) = scheduler::current_process_id() else {
        return false;
    };
    let Some(expected_pid) = unique_live_process_by_image(slots_ref(), UserImageId::Launcher)
        .map(|process| process.id.raw())
    else {
        return false;
    };
    current_pid == expected_pid
        && lookup(ProcessId(current_pid)).is_some_and(|process| {
            process.id.raw() == expected_pid
                && process.image_id == UserImageId::Launcher
                && !process.is_init
        })
}

/// Authenticates the sole live generation of the built-in system App image.
///
/// ABI 52's Settings UI uses this only for one exact-identity package removal
/// syscall. It does not grant the process a package-store or block handle.
#[cfg(feature = "androidbox-runtime-uninstall1")]
pub fn current_is_android_package_settings() -> bool {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("Android package settings identity checked with IRQ enabled");
    }
    let Some(current_pid) = scheduler::current_process_id() else {
        return false;
    };
    let Some(expected_pid) =
        unique_live_process_by_image(slots_ref(), UserImageId::App).map(|process| process.id.raw())
    else {
        return false;
    };
    current_pid == expected_pid
        && lookup(ProcessId(current_pid)).is_some_and(|process| {
            process.id.raw() == expected_pid
                && process.image_id == UserImageId::App
                && !process.is_init
        })
}

#[cfg(feature = "androidbox-process0")]
const ANDROID_PACKAGE_EXECUTION_IMAGE: UserImageId = UserImageId::AndroidApp;
#[cfg(all(
    feature = "androidbox-el0-runtime0",
    not(feature = "androidbox-process0")
))]
const ANDROID_PACKAGE_EXECUTION_IMAGE: UserImageId = UserImageId::App;

/// Returns the generation-qualified PID of the sole live package-execution
/// process. ABI 45/46 use App; ABI 47 moves this authority to AndroidApp.
#[cfg(feature = "androidbox-el0-runtime0")]
pub fn android_package_execution_process_id_irq_masked() -> Option<u64> {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("Android package execution identity queried with IRQ enabled");
    }
    unique_live_process_by_image(slots_ref(), ANDROID_PACKAGE_EXECUTION_IMAGE)
        .filter(|process| !process.is_init)
        .map(|process| process.id.raw())
}

/// Authenticates the sole live generation allowed to consume an APK grant.
/// App remains the ABI 45/46 executor; only AndroidApp is accepted in ABI 47.
#[cfg(feature = "androidbox-el0-runtime0")]
pub fn current_is_android_package_execution_process() -> bool {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("Android package execution identity checked with IRQ enabled");
    }
    let Some(current_pid) = scheduler::current_process_id() else {
        return false;
    };
    android_package_execution_process_id_irq_masked() == Some(current_pid)
        && lookup(ProcessId(current_pid)).is_some_and(|process| {
            process.id.raw() == current_pid
                && process.image_id == ANDROID_PACKAGE_EXECUTION_IMAGE
                && !process.is_init
        })
}

/// Checks that an async package-relaunch completion still belongs to the
/// exact live Launcher generation that submitted it.
#[cfg(feature = "androidbox-apk-install0")]
pub fn android_package_relaunch_owner_can_retry_irq_masked(raw_process_id: u64) -> bool {
    if !crate::arch::aarch64::irq_is_masked() || raw_process_id == 0 {
        panic!("Android package relaunch owner queried outside its serialized domain");
    }
    crate::scheduler::user_process_can_retry_irq_masked(raw_process_id)
        && unique_live_process_by_image(slots_ref(), UserImageId::Launcher)
            .is_some_and(|process| process.id.raw() == raw_process_id && !process.is_init)
        && lookup(ProcessId(raw_process_id)).is_some_and(|process| {
            process.id.raw() == raw_process_id
                && process.image_id == UserImageId::Launcher
                && !process.is_init
        })
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
pub fn android_package_uninstall_owner_can_retry_irq_masked(raw_process_id: u64) -> bool {
    if !crate::arch::aarch64::irq_is_masked() || raw_process_id == 0 {
        panic!("Android package uninstall owner queried outside its serialized domain");
    }
    crate::scheduler::user_process_can_retry_irq_masked(raw_process_id)
        && unique_live_process_by_image(slots_ref(), UserImageId::App)
            .is_some_and(|process| process.id.raw() == raw_process_id && !process.is_init)
        && lookup(ProcessId(raw_process_id)).is_some_and(|process| {
            process.id.raw() == raw_process_id
                && process.image_id == UserImageId::App
                && !process.is_init
        })
}

/// Authenticates the sole live generation of the dedicated InputServer.
#[cfg(feature = "input-server-runtime")]
pub fn current_is_input_server() -> bool {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("input server identity checked with IRQ enabled");
    }
    let Some(current_pid) = scheduler::current_process_id() else {
        return false;
    };
    let Some(expected_pid) = unique_live_process_by_image(slots_ref(), UserImageId::InputServer)
        .map(|process| process.id.raw())
    else {
        return false;
    };
    current_pid == expected_pid
        && lookup(ProcessId(current_pid)).is_some_and(|process| {
            process.id.raw() == expected_pid
                && process.image_id == UserImageId::InputServer
                && !process.is_init
        })
}

/// Returns the generation-qualified identity of the current live user
/// process. A missing scheduler identity, a stale generation, or a scheduler
/// identity without a matching live `Process` is reported as `None`.
pub fn current_live_user_process_id() -> Option<u64> {
    let raw = scheduler::current_process_id()?;
    lookup(ProcessId(raw)).map(|process| process.id.raw())
}

/// Returns the authenticated image identity of the current generation-
/// qualified live process. This is used for narrow syscall policy decisions;
/// callers must still validate object rights and producer identity.
pub fn current_live_user_image_id() -> Option<UserImageId> {
    let raw = scheduler::current_process_id()?;
    lookup(ProcessId(raw)).map(|process| process.image_id)
}

/// Checks both generation-qualified thread liveness and retained AppData
/// authority before the monitor publishes or preserves an async completion.
#[cfg(feature = "app-data-runtime")]
pub fn app_data_owner_can_retry_irq_masked(
    raw_process_id: u64,
    principal: u64,
    required: Rights,
) -> bool {
    if !crate::arch::aarch64::irq_is_masked() || raw_process_id == 0 || principal == 0 {
        panic!("AppData completion owner queried outside its serialized domain");
    }
    if !scheduler::user_process_can_retry_irq_masked(raw_process_id) {
        return false;
    }
    lookup(ProcessId(raw_process_id)).is_some_and(|process| {
        process.image_id == UserImageId::App
            && process.handles.live_entries().any(|(object, rights)| {
                rights.contains(required)
                    && object
                        .as_app_data_directory()
                        .is_some_and(|root| root.principal().raw() == principal)
            })
    })
}

#[cfg(feature = "input-server-restart-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputServerRestartGapEvidence {
    pub created: u64,
    pub exited: u64,
    pub reaped: u64,
    pub live: u64,
    pub replacement_spawn_pid: u64,
}

/// Returns the lifecycle counters that must remain frozen throughout M47's
/// finite restart delay. The caller already owns the IRQ-masked lifecycle
/// domain, so the five observations form one scheduler-visible checkpoint.
#[cfg(feature = "input-server-restart-runtime")]
pub fn input_server_restart_gap_evidence() -> InputServerRestartGapEvidence {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("InputServer restart gap evidence sampled with IRQ enabled");
    }
    InputServerRestartGapEvidence {
        created: CREATED.load(Ordering::Acquire),
        exited: EXITED.load(Ordering::Acquire),
        reaped: REAPED.load(Ordering::Acquire),
        live: LIVE.load(Ordering::Acquire),
        replacement_spawn_pid: CHILD_SPAWN_PIDS[9].load(Ordering::Acquire),
    }
}

/// Authenticates that one current-process channel has exactly one live peer
/// endpoint and that peer belongs to the sole live process of `peer_image`.
#[cfg(any(
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
pub fn current_channel_peer_is_unique_live_image(raw_handle: u64, peer_image: UserImageId) -> bool {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("restart wait peer checked with IRQ enabled");
    }
    let Ok(raw_handle) = u32::try_from(raw_handle) else {
        return false;
    };
    let handle = HandleValue::from_raw(raw_handle);
    if !handle.is_valid() {
        return false;
    }
    let Some(current_pid) = scheduler::current_process_id() else {
        return false;
    };
    let Some(current) = lookup(ProcessId(current_pid)) else {
        return false;
    };
    let Ok(object) = current.handles.get(handle, Rights::WAIT) else {
        return false;
    };
    let Some(endpoint) = object.as_channel() else {
        return false;
    };
    let Some(expected_peer_pid) =
        unique_live_process_by_image(slots_ref(), peer_image).map(|process| process.id.raw())
    else {
        return false;
    };

    let mut peers = 0_usize;
    let mut peer_pid = 0_u64;
    for process in slots_ref()
        .iter()
        .filter_map(|slot| slot.process.as_deref())
    {
        for candidate in process
            .handles
            .live_entries()
            .filter_map(|(object, _)| object.as_channel())
        {
            if endpoint.is_peer_of(candidate) {
                peers = peers.saturating_add(1);
                peer_pid = process.id.raw();
            }
        }
    }
    peers == 1 && peer_pid == expected_peer_pid
}

/// Returns the generation-qualified PID that owns the sole live peer endpoint
/// of one current-process channel. The lookup is tied to the actual transport
/// handle supplied to the syscall; stale handles, non-channels, a missing
/// peer, or any duplicated peer endpoint produce no identity.
#[cfg(feature = "service-supervisor-runtime")]
pub fn current_channel_unique_live_peer_pid(raw_handle: u64) -> Option<u64> {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("service supervisor channel peer checked with IRQ enabled");
    }
    let raw_handle = u32::try_from(raw_handle).ok()?;
    let handle = HandleValue::from_raw(raw_handle);
    if !handle.is_valid() {
        return None;
    }
    let current_pid = scheduler::current_process_id()?;
    let current = lookup(ProcessId(current_pid))?;
    let endpoint = current
        .handles
        .get(handle, Rights::NONE)
        .ok()?
        .as_channel()?;

    let mut peer_count = 0_usize;
    let mut unique_peer_pid = None;
    for process in slots_ref()
        .iter()
        .filter_map(|slot| slot.process.as_deref())
    {
        for candidate in process
            .handles
            .live_entries()
            .filter_map(|(object, _)| object.as_channel())
        {
            if !endpoint.is_peer_of(candidate) {
                continue;
            }
            peer_count = peer_count.saturating_add(1);
            if peer_count != 1 {
                return None;
            }
            unique_peer_pid = Some(process.id.raw());
        }
    }
    if peer_count == 1 {
        unique_peer_pid
    } else {
        None
    }
}

/// Compares the actual current-process transport with a previously recorded
/// live handle without exposing Channel internals. `same_channel` binds both
/// endpoint sides to the channel's non-reused kernel identity; owner PID and
/// syscall direction separately authenticate which side is in use.
#[cfg(feature = "service-supervisor-runtime")]
pub fn current_channel_matches_live_binding(
    raw_handle: u64,
    bound_owner_pid: u64,
    bound_raw_handle: u64,
) -> bool {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("service supervisor channel binding checked with IRQ enabled");
    }
    if bound_owner_pid == 0 {
        return false;
    }
    let Ok(raw_handle) = u32::try_from(raw_handle) else {
        return false;
    };
    let Ok(bound_raw_handle) = u32::try_from(bound_raw_handle) else {
        return false;
    };
    let current_handle = HandleValue::from_raw(raw_handle);
    let bound_handle = HandleValue::from_raw(bound_raw_handle);
    if !current_handle.is_valid() || !bound_handle.is_valid() {
        return false;
    }
    let Some(current_pid) = scheduler::current_process_id() else {
        return false;
    };
    let Some(current) = lookup(ProcessId(current_pid)) else {
        return false;
    };
    let Some(bound_owner) = lookup(ProcessId(bound_owner_pid)) else {
        return false;
    };
    let Ok(current_object) = current.handles.get(current_handle, Rights::NONE) else {
        return false;
    };
    let Ok(bound_object) = bound_owner.handles.get(bound_handle, Rights::NONE) else {
        return false;
    };
    current_object
        .as_channel()
        .zip(bound_object.as_channel())
        .is_some_and(|(current, bound)| current.same_channel(bound))
}

/// Pins and maps one graphics buffer in the currently executing process.
/// Rights are still enforced by the syscall layer; this lifecycle boundary
/// independently authenticates the producer PID or unique SurfaceServer role.
pub fn map_current_graphics_buffer(
    buffer: GraphicsBuffer,
    role: GraphicsMappingRole,
) -> Result<GraphicsMappingSnapshot, ProcessGraphicsMappingError> {
    let raw_process_id = current_graphics_process_id()?;
    let role_valid = match role {
        GraphicsMappingRole::Producer => {
            raw_process_id == buffer.producer_pid()
                && lookup(ProcessId(raw_process_id)).is_some_and(|process| {
                    !process.is_init
                        && (matches!(process.image_id, UserImageId::Launcher | UserImageId::App)
                            || (MULTI_WINDOW_RUNTIME_ACTIVE
                                && process.image_id == UserImageId::SurfaceServer
                                && unique_live_process_by_image(
                                    slots_ref(),
                                    UserImageId::SurfaceServer,
                                )
                                .is_some_and(|surface| surface.id.raw() == raw_process_id)))
                })
        }
        GraphicsMappingRole::Consumer => {
            unique_live_process_by_image(slots_ref(), UserImageId::SurfaceServer)
                .is_some_and(|process| process.id.raw() == raw_process_id && !process.is_init)
                && raw_process_id != buffer.producer_pid()
        }
    };
    if !role_valid {
        return Err(ProcessGraphicsMappingError::IdentityMismatch);
    }
    let process =
        lookup_mut(ProcessId(raw_process_id)).ok_or(ProcessGraphicsMappingError::NotFound)?;
    let address_space = active_process_address_space_mut(process)?;
    address_space
        .map_graphics_buffer(buffer, role)
        .map_err(Into::into)
}

/// Removes the exact handle-authenticated mapping from the current process.
/// The address is checked independently, so a handle for one slot cannot
/// unmap a different generation or backing slot.
pub fn unmap_current_graphics_buffer(
    buffer: &GraphicsBuffer,
    address: usize,
) -> Result<GraphicsMappingSnapshot, ProcessGraphicsMappingError> {
    let raw_process_id = current_graphics_process_id()?;
    let process =
        lookup_mut(ProcessId(raw_process_id)).ok_or(ProcessGraphicsMappingError::NotFound)?;
    let address_space = active_process_address_space_mut(process)?;
    let mapping = address_space.graphics_mapping_snapshot(buffer).ok_or(
        ProcessGraphicsMappingError::AddressSpace(GraphicsMappingError::NotMapped),
    )?;
    if mapping.address != address {
        return Err(ProcessGraphicsMappingError::AddressSpace(
            GraphicsMappingError::InvalidAddress,
        ));
    }
    if mapping.role == GraphicsMappingRole::Consumer
        && buffer
            .consumer_owner_death_frame(raw_process_id)
            .map_err(|_| ProcessGraphicsMappingError::InvalidState)?
            .is_some()
    {
        // A normal consumer must release/discard a queued or acquired frame
        // before unmapping. Stopped-address-space destruction remains the
        // sole unconditional path and pairs its unmap with owner-death
        // producer restoration.
        return Err(ProcessGraphicsMappingError::InvalidState);
    }
    address_space
        .unmap_graphics_buffer(buffer, address)
        .map_err(Into::into)
}

/// Performs one exact producer access transition in the current address
/// space. Queue uses RW->RO; any replay or wrong-direction request fails
/// before the first page-table write.
pub fn protect_current_graphics_buffer_mapping(
    buffer: &GraphicsBuffer,
    expected: GraphicsMappingAccess,
    replacement: GraphicsMappingAccess,
) -> Result<GraphicsMappingSnapshot, ProcessGraphicsMappingError> {
    let raw_process_id = current_graphics_process_id()?;
    if raw_process_id != buffer.producer_pid() {
        return Err(ProcessGraphicsMappingError::IdentityMismatch);
    }
    let current_surface_output_producer =
        MULTI_WINDOW_RUNTIME_ACTIVE && current_is_surface_server();
    let process =
        lookup_mut(ProcessId(raw_process_id)).ok_or(ProcessGraphicsMappingError::NotFound)?;
    if process.is_init
        || !(matches!(process.image_id, UserImageId::Launcher | UserImageId::App)
            || (current_surface_output_producer && process.image_id == UserImageId::SurfaceServer))
    {
        return Err(ProcessGraphicsMappingError::InvalidState);
    }
    let address_space = active_process_address_space_mut(process)?;
    address_space
        .protect_graphics_buffer_mapping(buffer, expected, replacement)
        .map_err(Into::into)
}

/// Restores a successfully presented buffer to its exact live producer.
/// `producer_pid` is generation-qualified; stale PIDs cannot mutate a newly
/// reused process slot even when both generations map the same backing slot.
pub fn restore_graphics_buffer_producer_mapping(
    producer_pid: u64,
    buffer: &GraphicsBuffer,
) -> Result<GraphicsMappingSnapshot, ProcessGraphicsMappingError> {
    if !crate::arch::aarch64::irq_is_masked() {
        return Err(ProcessGraphicsMappingError::InvalidState);
    }
    if producer_pid == 0 || producer_pid != buffer.producer_pid() {
        return Err(ProcessGraphicsMappingError::IdentityMismatch);
    }
    let surface_output_producer = MULTI_WINDOW_RUNTIME_ACTIVE
        && unique_live_process_by_image(slots_ref(), UserImageId::SurfaceServer)
            .is_some_and(|surface| surface.id.raw() == producer_pid);
    let process =
        lookup_mut(ProcessId(producer_pid)).ok_or(ProcessGraphicsMappingError::NotFound)?;
    if process.id.raw() != producer_pid
        || process.is_init
        || !(matches!(process.image_id, UserImageId::Launcher | UserImageId::App)
            || (surface_output_producer && process.image_id == UserImageId::SurfaceServer))
    {
        return Err(ProcessGraphicsMappingError::IdentityMismatch);
    }
    let address_space = process
        .address_space
        .as_mut()
        .ok_or(ProcessGraphicsMappingError::InvalidState)?;
    address_space
        .protect_graphics_buffer_mapping(
            buffer,
            GraphicsMappingAccess::ReadOnly,
            GraphicsMappingAccess::ReadWrite,
        )
        .map_err(Into::into)
}

pub fn current_graphics_buffer_mapping_snapshot(
    buffer: &GraphicsBuffer,
) -> Result<GraphicsMappingSnapshot, ProcessGraphicsMappingError> {
    let raw_process_id = current_graphics_process_id()?;
    let process = lookup(ProcessId(raw_process_id)).ok_or(ProcessGraphicsMappingError::NotFound)?;
    process
        .address_space
        .as_ref()
        .and_then(|address_space| address_space.graphics_mapping_snapshot(buffer))
        .ok_or(ProcessGraphicsMappingError::AddressSpace(
            GraphicsMappingError::NotMapped,
        ))
}

/// Read-only convenience surface for syscall preflight. Detailed callers that
/// need to distinguish a stale process from a missing mapping use
/// [`current_graphics_buffer_mapping_snapshot`].
pub fn current_graphics_buffer_mapping(buffer: &GraphicsBuffer) -> Option<GraphicsMappingSnapshot> {
    current_graphics_buffer_mapping_snapshot(buffer).ok()
}

#[allow(dead_code)]
pub fn graphics_buffer_mapping_snapshot(
    raw_process_id: u64,
    buffer: &GraphicsBuffer,
) -> Result<GraphicsMappingSnapshot, ProcessGraphicsMappingError> {
    if !crate::arch::aarch64::irq_is_masked() {
        return Err(ProcessGraphicsMappingError::InvalidState);
    }
    let process = lookup(ProcessId(raw_process_id)).ok_or(ProcessGraphicsMappingError::NotFound)?;
    process
        .address_space
        .as_ref()
        .and_then(|address_space| address_space.graphics_mapping_snapshot(buffer))
        .ok_or(ProcessGraphicsMappingError::AddressSpace(
            GraphicsMappingError::NotMapped,
        ))
}

fn current_graphics_process_id() -> Result<u64, ProcessGraphicsMappingError> {
    if !crate::arch::aarch64::irq_is_masked() {
        return Err(ProcessGraphicsMappingError::InvalidState);
    }
    let raw_process_id =
        scheduler::current_process_id().ok_or(ProcessGraphicsMappingError::InvalidState)?;
    lookup(ProcessId(raw_process_id))
        .filter(|process| !process.is_init && process.image_id != UserImageId::Init)
        .map(|process| process.id.raw())
        .ok_or(ProcessGraphicsMappingError::NotFound)
}

fn active_process_address_space_mut(
    process: &mut Process,
) -> Result<&mut UserAddressSpace, ProcessGraphicsMappingError> {
    let address_space = process
        .address_space
        .as_mut()
        .ok_or(ProcessGraphicsMappingError::InvalidState)?;
    if mmu::current_translation_context() != address_space.translation_context() {
        return Err(ProcessGraphicsMappingError::InvalidState);
    }
    Ok(address_space)
}

/// Checks the immutable mapping metadata of the currently scheduled live
/// process before an all-or-nothing ABI copy. This is stronger than a lower-48
/// range check: every touched page must exist and have user read-write access.
pub fn current_user_range_is_writable(address: u64, bytes: usize) -> bool {
    let Some(raw) = scheduler::current_process_id() else {
        return false;
    };
    lookup(ProcessId(raw))
        .and_then(|process| process.address_space.as_ref())
        .is_some_and(|address_space| address_space.user_range_is_writable(address, bytes))
}

/// Checks the immutable mapping metadata of the currently scheduled live
/// process before an all-or-nothing ABI copy from user memory. RX, R--, and
/// RW- user leaves are readable; guard and unmapped pages are rejected.
pub fn current_user_range_is_readable(address: u64, bytes: usize) -> bool {
    let Some(raw) = scheduler::current_process_id() else {
        return false;
    };
    lookup(ProcessId(raw))
        .and_then(|process| process.address_space.as_ref())
        .is_some_and(|address_space| address_space.user_range_is_readable(address, bytes))
}

/// Returns the full generation-qualified PID only when exactly one live
/// process owns the requested image identity.
pub fn unique_live_process_id_for_image(image_id: UserImageId) -> Option<u64> {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("unique live image identity queried with IRQ enabled");
    }
    unique_live_process_by_image(slots_ref(), image_id).map(|process| process.id.raw())
}

pub fn record_child_syscall() {
    if current_is_init() || scheduler::current_process_id().is_none() {
        panic!("child syscall accounting used outside a dynamic process");
    }
    CHILD_SYSCALLS.fetch_add(1, Ordering::Relaxed);
}

fn resident_graphics_mapping_evidence(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
) -> ResidentGraphicsMappingEvidence {
    const CAPACITY: usize = PROCESS_CAPACITY * mmu::USER_GRAPHICS_MAPPING_CAPACITY;
    let mut collected: [Option<ResidentGraphicsMapping<'_>>; CAPACITY] = [None; CAPACITY];
    let mut count = 0_usize;
    let mut mapped_pages = 0_usize;
    let mut producer_count = 0_usize;
    let mut consumer_count = 0_usize;
    let mut producer_read_write_count = 0_usize;
    let mut producer_read_only_count = 0_usize;
    let mut roles_access_valid = true;

    for process in slots.iter().filter_map(|slot| slot.process.as_deref()) {
        let Some(address_space) = process.address_space.as_ref() else {
            roles_access_valid = false;
            continue;
        };
        for mapping in address_space
            .graphics_mapping_snapshots()
            .into_iter()
            .flatten()
        {
            let Some(destination) = collected.get_mut(count) else {
                panic!("resident graphics mapping evidence exceeded its fixed capacity");
            };
            *destination = Some(ResidentGraphicsMapping { process, mapping });
            count = count
                .checked_add(1)
                .unwrap_or_else(|| panic!("resident graphics mapping count overflowed"));
            mapped_pages = mapped_pages
                .checked_add(mapping.pages)
                .unwrap_or_else(|| panic!("resident graphics mapped-page count overflowed"));
            match mapping.role {
                GraphicsMappingRole::Producer => {
                    producer_count = producer_count.checked_add(1).unwrap_or_else(|| {
                        panic!("resident graphics producer mapping count overflowed")
                    });
                    match mapping.access {
                        GraphicsMappingAccess::ReadWrite => {
                            producer_read_write_count =
                                producer_read_write_count.checked_add(1).unwrap_or_else(|| {
                                    panic!("resident writable producer mapping count overflowed")
                                });
                        }
                        GraphicsMappingAccess::ReadOnly => {
                            producer_read_only_count =
                                producer_read_only_count.checked_add(1).unwrap_or_else(|| {
                                    panic!("resident read-only producer mapping count overflowed")
                                });
                        }
                    }
                    let producer_image_valid = process.image_id == UserImageId::App
                        || (MULTI_WINDOW_RUNTIME_ACTIVE
                            && process.image_id == UserImageId::SurfaceServer);
                    roles_access_valid &= producer_image_valid
                        && !process.is_init
                        && process.id.raw() == mapping.producer_pid
                        && (mapping.access == GraphicsMappingAccess::ReadWrite
                            || GRAPHICS_SWAPCHAIN_RUNTIME_ACTIVE
                            || MULTI_WINDOW_RUNTIME_ACTIVE);
                }
                GraphicsMappingRole::Consumer => {
                    consumer_count = consumer_count.checked_add(1).unwrap_or_else(|| {
                        panic!("resident graphics consumer mapping count overflowed")
                    });
                    roles_access_valid &= process.image_id == UserImageId::SurfaceServer
                        && !process.is_init
                        && process.id.raw() != mapping.producer_pid
                        && mapping.access == GraphicsMappingAccess::ReadOnly;
                }
            }
        }
    }

    if (GRAPHICS_SWAPCHAIN_RUNTIME_ACTIVE || MULTI_WINDOW_RUNTIME_ACTIVE) && count != 0 {
        let pool = pool_snapshot();
        let producer_access_matches_pool = collected
            .iter()
            .flatten()
            .filter(|entry| entry.mapping.role == GraphicsMappingRole::Producer)
            .all(|entry| {
                pool.slots
                    .get(usize::from(entry.mapping.identity.slot()))
                    .is_some_and(|slot| {
                        slot.occupied
                            && slot.generation == entry.mapping.identity.generation()
                            && slot.producer_pid == entry.mapping.producer_pid
                            && match slot.access {
                                GraphicsBufferAccessSnapshot::Writable => {
                                    entry.mapping.access == GraphicsMappingAccess::ReadWrite
                                }
                                GraphicsBufferAccessSnapshot::Queued { .. }
                                | GraphicsBufferAccessSnapshot::Acquired { .. } => {
                                    entry.mapping.access == GraphicsMappingAccess::ReadOnly
                                }
                                GraphicsBufferAccessSnapshot::Vacant
                                | GraphicsBufferAccessSnapshot::Legacy => false,
                            }
                    })
            });
        roles_access_valid &= producer_access_matches_pool;
        if GRAPHICS_SWAPCHAIN_RUNTIME_ACTIVE {
            roles_access_valid &= producer_read_write_count == 1 && producer_read_only_count == 1;
        } else {
            roles_access_valid &=
                producer_read_write_count.checked_add(producer_read_only_count) == Some(2);
        }
    }

    let producer = collected
        .iter()
        .flatten()
        .find(|entry| entry.mapping.role == GraphicsMappingRole::Producer);
    let consumer = collected
        .iter()
        .flatten()
        .find(|entry| entry.mapping.role == GraphicsMappingRole::Consumer);
    let mut shared_pairs = 0_usize;
    for left in collected
        .iter()
        .flatten()
        .filter(|entry| entry.mapping.role == GraphicsMappingRole::Producer)
    {
        for right in collected
            .iter()
            .flatten()
            .filter(|entry| entry.mapping.role == GraphicsMappingRole::Consumer)
        {
            if left.mapping.identity == right.mapping.identity
                && left.mapping.backing_address == right.mapping.backing_address
                && left.mapping.address == right.mapping.address
            {
                shared_pairs = shared_pairs
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("resident graphics shared-pair count overflowed"));
            }
        }
    }
    let expected_pair_count = if GRAPHICS_SWAPCHAIN_RUNTIME_ACTIVE {
        2
    } else {
        1
    };
    let expected_mapping_count = expected_pair_count * 2;
    let expected_mapped_pages = expected_mapping_count * mmu::USER_GRAPHICS_MAPPING_PAGES;
    let producer_identities_distinct = collected
        .iter()
        .flatten()
        .filter(|entry| entry.mapping.role == GraphicsMappingRole::Producer)
        .enumerate()
        .all(|(index, left)| {
            collected
                .iter()
                .flatten()
                .filter(|entry| entry.mapping.role == GraphicsMappingRole::Producer)
                .skip(index + 1)
                .all(|right| left.mapping.identity != right.mapping.identity)
        });
    let exact_alias_pairs = collected
        .iter()
        .flatten()
        .filter(|entry| entry.mapping.role == GraphicsMappingRole::Producer)
        .all(|producer| {
            collected
                .iter()
                .flatten()
                .filter(|entry| entry.mapping.role == GraphicsMappingRole::Consumer)
                .filter(|consumer| {
                    producer.mapping.identity == consumer.mapping.identity
                        && producer.mapping.backing_address == consumer.mapping.backing_address
                        && producer.mapping.address == consumer.mapping.address
                })
                .count()
                == 1
        })
        && collected
            .iter()
            .flatten()
            .filter(|entry| entry.mapping.role == GraphicsMappingRole::Consumer)
            .all(|consumer| {
                collected
                    .iter()
                    .flatten()
                    .filter(|entry| entry.mapping.role == GraphicsMappingRole::Producer)
                    .filter(|producer| {
                        producer.mapping.identity == consumer.mapping.identity
                            && producer.mapping.backing_address == consumer.mapping.backing_address
                            && producer.mapping.address == consumer.mapping.address
                    })
                    .count()
                    == 1
            });
    let historical_shared_alias_valid = count == expected_mapping_count
        && mapped_pages == expected_mapped_pages
        && collected
            .iter()
            .flatten()
            .all(|entry| entry.mapping.pages == mmu::USER_GRAPHICS_MAPPING_PAGES)
        && producer_count == expected_pair_count
        && consumer_count == expected_pair_count
        && shared_pairs == expected_pair_count
        && producer_identities_distinct
        && exact_alias_pairs;
    let multi_window_surface_pid = MULTI_WINDOW_RUNTIME_ACTIVE
        .then(|| unique_live_process_by_image(slots, UserImageId::SurfaceServer))
        .flatten()
        .map(|surface| surface.id.raw());
    let multi_window_topology_valid = count == 2
        && mapped_pages == 2 * mmu::USER_GRAPHICS_MAPPING_PAGES
        && producer_count == 2
        && consumer_count == 0
        && shared_pairs == 0
        && producer_identities_distinct
        && collected.iter().flatten().all(|entry| {
            entry.mapping.pages == mmu::USER_GRAPHICS_MAPPING_PAGES
                && entry.mapping.role == GraphicsMappingRole::Producer
                && entry.process.image_id == UserImageId::SurfaceServer
                && !entry.process.is_init
                && entry.process.id.raw() == entry.mapping.producer_pid
                && Some(entry.process.id.raw()) == multi_window_surface_pid
        });
    let topology_valid = if MULTI_WINDOW_RUNTIME_ACTIVE {
        multi_window_topology_valid
    } else {
        historical_shared_alias_valid
    };
    let shared_alias_valid = !MULTI_WINDOW_RUNTIME_ACTIVE && historical_shared_alias_valid;
    let contexts_distinct = shared_alias_valid
        && producer.zip(consumer).is_some_and(|(producer, consumer)| {
            collected
                .iter()
                .flatten()
                .all(|entry| match entry.mapping.role {
                    GraphicsMappingRole::Producer => entry.process.id == producer.process.id,
                    GraphicsMappingRole::Consumer => entry.process.id == consumer.process.id,
                })
                && producer.process.id != consumer.process.id
                && producer
                    .process
                    .address_space
                    .as_ref()
                    .is_some_and(|producer_space| {
                        consumer
                            .process
                            .address_space
                            .as_ref()
                            .is_some_and(|consumer_space| {
                                producer_space.asid() != consumer_space.asid()
                                    && producer_space.root_address()
                                        != consumer_space.root_address()
                            })
                    })
        });
    ResidentGraphicsMappingEvidence {
        count,
        mapped_pages,
        producer_count,
        consumer_count,
        shared_pairs,
        roles_access_valid,
        topology_valid,
        shared_alias_valid,
        contexts_distinct,
    }
}

#[cfg(feature = "graphics-owner-death-runtime")]
pub fn graphics_consumer_owner_death_snapshot() -> GraphicsConsumerOwnerDeathSnapshot {
    GraphicsConsumerOwnerDeathSnapshot {
        deaths: GRAPHICS_CONSUMER_DEATHS.load(Ordering::Acquire),
        mappings: GRAPHICS_CONSUMER_DEATH_MAPPINGS.load(Ordering::Acquire),
        queued: GRAPHICS_CONSUMER_DEATH_QUEUED.load(Ordering::Acquire),
        acquired: GRAPHICS_CONSUMER_DEATH_ACQUIRED.load(Ordering::Acquire),
        producer_restores: GRAPHICS_CONSUMER_DEATH_PRODUCER_RESTORES.load(Ordering::Acquire),
        producer_orphans: GRAPHICS_CONSUMER_DEATH_PRODUCER_ORPHANS.load(Ordering::Acquire),
        releases: GRAPHICS_CONSUMER_DEATH_RELEASES.load(Ordering::Acquire),
        wake_completions: GRAPHICS_CONSUMER_DEATH_WAKE_COMPLETIONS.load(Ordering::Acquire),
    }
}

#[cfg(feature = "androidbox-interactive0")]
fn collect_androidbox_interactive_graphics_evidence(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
) -> AndroidboxInteractiveGraphicsEvidence {
    const IDENTITY_CAPACITY: usize = 3;
    let surface_server_pid = unique_live_process_by_image(slots, UserImageId::SurfaceServer)
        .map_or(0, |process| process.id.raw());
    let launcher_pid = unique_live_process_by_image(slots, UserImageId::Launcher)
        .map_or(0, |process| process.id.raw());
    let app_pid =
        unique_live_process_by_image(slots, UserImageId::App).map_or(0, |process| process.id.raw());
    let role_pids_valid = surface_server_pid != 0
        && launcher_pid != 0
        && app_pid != 0
        && surface_server_pid != launcher_pid
        && surface_server_pid != app_pid
        && launcher_pid != app_pid;

    let mut identities = [None; IDENTITY_CAPACITY];
    let mut evidence = AndroidboxInteractiveGraphicsEvidence {
        handle_count: 0,
        identity_count: 0,
        identity_handle_counts: [0; IDENTITY_CAPACITY],
        identity_producer_handle_counts: [0; IDENTITY_CAPACITY],
        identity_server_handle_counts: [0; IDENTITY_CAPACITY],
        producer_pids: [0; IDENTITY_CAPACITY],
        producer_role_counts: [0; 3],
        server_handle_count: 0,
        surface_server_pid,
        valid: role_pids_valid,
    };

    for process in slots.iter().filter_map(|slot| slot.process.as_deref()) {
        for (object, rights) in process.handles.live_entries() {
            let Some(buffer) = object.as_graphics_buffer() else {
                continue;
            };
            evidence.handle_count = evidence
                .handle_count
                .checked_add(1)
                .unwrap_or_else(|| panic!("ABI 46 graphics handle count overflowed"));

            let identity = buffer.identity();
            let identity_index = identities[..evidence.identity_count]
                .iter()
                .position(|candidate| *candidate == Some(identity))
                .or_else(|| {
                    let index = evidence.identity_count;
                    let destination = identities.get_mut(index)?;
                    *destination = Some(identity);
                    evidence.identity_count = evidence
                        .identity_count
                        .checked_add(1)
                        .unwrap_or_else(|| panic!("ABI 46 graphics identity count overflowed"));
                    evidence.producer_pids[index] = buffer.producer_pid();
                    Some(index)
                });
            let Some(identity_index) = identity_index else {
                evidence.valid = false;
                continue;
            };
            evidence.identity_handle_counts[identity_index] = evidence.identity_handle_counts
                [identity_index]
                .checked_add(1)
                .unwrap_or_else(|| panic!("ABI 46 identity handle count overflowed"));
            evidence.valid &= evidence.producer_pids[identity_index] == buffer.producer_pid();

            if rights == Rights::GRAPHICS_BUFFER_DEFAULT {
                evidence.identity_producer_handle_counts[identity_index] = evidence
                    .identity_producer_handle_counts[identity_index]
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("ABI 46 producer-right count overflowed"));
                evidence.valid &= process.id.raw() == buffer.producer_pid();
                let role_index = match process.image_id {
                    UserImageId::SurfaceServer if process.id.raw() == surface_server_pid => Some(0),
                    UserImageId::Launcher if process.id.raw() == launcher_pid => Some(1),
                    UserImageId::App if process.id.raw() == app_pid => Some(2),
                    UserImageId::Init
                    | UserImageId::ServiceManager
                    | UserImageId::Provider
                    | UserImageId::Client
                    | UserImageId::SurfaceServer
                    | UserImageId::Launcher
                    | UserImageId::App
                    | UserImageId::InputServer
                    | UserImageId::StorageServer => None,
                    #[cfg(feature = "androidbox-process0")]
                    UserImageId::AndroidApp => None,
                };
                if let Some(role_index) = role_index {
                    evidence.producer_role_counts[role_index] = evidence.producer_role_counts
                        [role_index]
                        .checked_add(1)
                        .unwrap_or_else(|| panic!("ABI 46 producer-role count overflowed"));
                } else {
                    evidence.valid = false;
                }
            } else if rights == Rights::GRAPHICS_BUFFER_SERVER {
                evidence.identity_server_handle_counts[identity_index] = evidence
                    .identity_server_handle_counts[identity_index]
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("ABI 46 server-right count overflowed"));
                evidence.server_handle_count = evidence
                    .server_handle_count
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("ABI 46 server handle count overflowed"));
                evidence.valid &= process.image_id == UserImageId::SurfaceServer
                    && process.id.raw() == surface_server_pid;
            } else {
                evidence.valid = false;
            }
        }
    }

    evidence.valid &= evidence.handle_count == 6
        && evidence.identity_count == IDENTITY_CAPACITY
        && evidence.identity_handle_counts == [2, 2, 2]
        && evidence.identity_producer_handle_counts == [1, 1, 1]
        && evidence.identity_server_handle_counts == [1, 1, 1]
        && evidence.producer_role_counts == [1, 1, 1]
        && evidence.server_handle_count == 3
        && evidence
            .producer_pids
            .iter()
            .all(|producer_pid| *producer_pid != 0)
        && evidence.producer_pids[0] != evidence.producer_pids[1]
        && evidence.producer_pids[0] != evidence.producer_pids[2]
        && evidence.producer_pids[1] != evidence.producer_pids[2];
    evidence
}

/// Samples ABI 46's complete graphics-handle topology in the same serialized
/// process-table domain used by the historical resident snapshot.
#[cfg(feature = "androidbox-interactive0")]
pub fn androidbox_interactive_graphics_evidence() -> AndroidboxInteractiveGraphicsEvidence {
    assert_initialized();
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let evidence = collect_androidbox_interactive_graphics_evidence(slots_ref());
    crate::arch::aarch64::restore_daif(saved_daif);
    evidence
}

pub fn snapshot() -> ProcessSnapshot {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let wait = scheduler::process_wait_snapshot();
    let completion_pending = slots_ref()[FIRST_DYNAMIC_PROCESS_SLOT..]
        .iter()
        .any(|slot| slot.completion.is_pending());
    let supervisor_wait_pending = wait.waiting || wait.pending_target != 0;
    let mut live_image_counts = [0_u64; USER_IMAGE_COUNT];
    let mut handle_counts_by_image = [0_usize; USER_IMAGE_COUNT];
    #[cfg(feature = "input-server-runtime")]
    let mut input_server_live = 0_u64;
    #[cfg(feature = "input-server-runtime")]
    let mut input_server_handles = 0_usize;
    #[cfg(feature = "storage-server-runtime")]
    let mut storage_server_live = 0_u64;
    #[cfg(feature = "storage-server-runtime")]
    let mut storage_server_handles = 0_usize;
    #[cfg(feature = "androidbox-process0")]
    let mut android_app_live = 0_u64;
    #[cfg(feature = "androidbox-process0")]
    let mut android_app_handles = 0_usize;
    let mut total_handles = 0_usize;
    for process in slots_ref()
        .iter()
        .filter_map(|slot| slot.process.as_deref())
    {
        #[cfg(feature = "storage-server-runtime")]
        if process.image_id == UserImageId::StorageServer {
            storage_server_live = storage_server_live
                .checked_add(1)
                .unwrap_or_else(|| panic!("StorageServer live count overflowed"));
            storage_server_handles = storage_server_handles
                .checked_add(process.handles.len())
                .unwrap_or_else(|| panic!("StorageServer handle count overflowed"));
            total_handles = total_handles
                .checked_add(process.handles.len())
                .unwrap_or_else(|| panic!("live process handle count overflowed"));
            continue;
        }
        #[cfg(feature = "input-server-runtime")]
        if process.image_id == UserImageId::InputServer {
            input_server_live = input_server_live
                .checked_add(1)
                .unwrap_or_else(|| panic!("InputServer live count overflowed"));
            input_server_handles = input_server_handles
                .checked_add(process.handles.len())
                .unwrap_or_else(|| panic!("InputServer handle count overflowed"));
            total_handles = total_handles
                .checked_add(process.handles.len())
                .unwrap_or_else(|| panic!("live process handle count overflowed"));
            continue;
        }
        #[cfg(feature = "androidbox-process0")]
        if process.image_id == UserImageId::AndroidApp {
            android_app_live = android_app_live
                .checked_add(1)
                .unwrap_or_else(|| panic!("AndroidApp live count overflowed"));
            android_app_handles = android_app_handles
                .checked_add(process.handles.len())
                .unwrap_or_else(|| panic!("AndroidApp handle count overflowed"));
            total_handles = total_handles
                .checked_add(process.handles.len())
                .unwrap_or_else(|| panic!("live process handle count overflowed"));
            continue;
        }
        let image_index = usize::try_from(process.image_id.raw() - 1)
            .unwrap_or_else(|_| panic!("live process image ID did not fit an index"));
        *live_image_counts
            .get_mut(image_index)
            .unwrap_or_else(|| panic!("live process image ID escaped the catalog")) += 1;
        handle_counts_by_image[image_index] = handle_counts_by_image[image_index]
            .checked_add(process.handles.len())
            .unwrap_or_else(|| panic!("per-image handle count overflowed"));
        total_handles = total_handles
            .checked_add(process.handles.len())
            .unwrap_or_else(|| panic!("live process handle count overflowed"));
    }
    let child_spawn_pids = [
        CHILD_SPAWN_PIDS[0].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[1].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[2].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[3].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[4].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[5].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[6].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[7].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[8].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[9].load(Ordering::Acquire),
    ];
    let child_spawn_images = [
        CHILD_SPAWN_IMAGES[0].load(Ordering::Acquire),
        CHILD_SPAWN_IMAGES[1].load(Ordering::Acquire),
        CHILD_SPAWN_IMAGES[2].load(Ordering::Acquire),
        CHILD_SPAWN_IMAGES[3].load(Ordering::Acquire),
        CHILD_SPAWN_IMAGES[4].load(Ordering::Acquire),
        CHILD_SPAWN_IMAGES[5].load(Ordering::Acquire),
        CHILD_SPAWN_IMAGES[6].load(Ordering::Acquire),
        CHILD_SPAWN_IMAGES[7].load(Ordering::Acquire),
        CHILD_SPAWN_IMAGES[8].load(Ordering::Acquire),
        CHILD_SPAWN_IMAGES[9].load(Ordering::Acquire),
    ];
    let resident =
        resident_service_topology(slots_ref(), wait, child_spawn_pids, child_spawn_images);
    let graphics_mapping_stats = mmu::graphics_mapping_stats();
    let resident_graphics_mappings = resident_graphics_mapping_evidence(slots_ref());
    let snapshot = ProcessSnapshot {
        process_capacity: PROCESS_CAPACITY,
        dynamic_capacity: DYNAMIC_PROCESS_CAPACITY,
        created: CREATED.load(Ordering::Acquire),
        exited: EXITED.load(Ordering::Acquire),
        reaped: REAPED.load(Ordering::Acquire),
        terminated_exited: TERMINATED_EXITED.load(Ordering::Acquire),
        terminated_faulted: TERMINATED_FAULTED.load(Ordering::Acquire),
        terminated_killed: TERMINATED_KILLED.load(Ordering::Acquire),
        live: LIVE.load(Ordering::Acquire),
        peak_live: PEAK_LIVE.load(Ordering::Acquire),
        spawn_waits: SPAWN_WAITS.load(Ordering::Acquire),
        capacity_rejections: CAPACITY_REJECTIONS.load(Ordering::Acquire),
        wait_calls: WAIT_CALLS.load(Ordering::Acquire),
        wait_completed: WAIT_COMPLETED.load(Ordering::Acquire),
        wait_blocks: wait.blocks,
        wait_wakes: wait.wakes,
        wait_immediate: WAIT_IMMEDIATE.load(Ordering::Acquire),
        wait_stale: WAIT_STALE.load(Ordering::Acquire),
        wait_pending: supervisor_wait_pending || completion_pending,
        supervisor_wait_pending,
        supervisor_wait_target: wait.pending_target,
        completion_pending,
        child_syscalls: CHILD_SYSCALLS.load(Ordering::Acquire),
        child_selections: CHILD_SELECTIONS.load(Ordering::Acquire),
        first_child_pid: FIRST_CHILD_PID.load(Ordering::Acquire),
        second_child_pid: SECOND_CHILD_PID.load(Ordering::Acquire),
        child_spawn_pids,
        child_spawn_images,
        #[cfg(feature = "unified-product-liveness-runtime")]
        unified_product_storage_replacement_pid: UNIFIED_PRODUCT_STORAGE_REPLACEMENT_PID
            .load(Ordering::Acquire),
        #[cfg(feature = "unified-product-liveness-runtime")]
        unified_product_storage_replacement_image: UNIFIED_PRODUCT_STORAGE_REPLACEMENT_IMAGE
            .load(Ordering::Acquire),
        #[cfg(feature = "unified-product-event-supervision-runtime")]
        unified_product_storage_second_replacement_pid:
            UNIFIED_PRODUCT_STORAGE_SECOND_REPLACEMENT_PID.load(Ordering::Acquire),
        #[cfg(feature = "unified-product-event-supervision-runtime")]
        unified_product_storage_second_replacement_image:
            UNIFIED_PRODUCT_STORAGE_SECOND_REPLACEMENT_IMAGE.load(Ordering::Acquire),
        #[cfg(feature = "service-dependency-runtime")]
        service_dependency_input_replacement_pid: SERVICE_DEPENDENCY_INPUT_REPLACEMENT_PID
            .load(Ordering::Acquire),
        #[cfg(feature = "service-dependency-runtime")]
        service_dependency_input_replacement_image: SERVICE_DEPENDENCY_INPUT_REPLACEMENT_IMAGE
            .load(Ordering::Acquire),
        live_image_counts,
        handle_counts_by_image,
        #[cfg(feature = "input-server-runtime")]
        input_server_live,
        #[cfg(feature = "input-server-runtime")]
        input_server_handles,
        #[cfg(feature = "storage-server-runtime")]
        storage_server_live,
        #[cfg(feature = "storage-server-runtime")]
        storage_server_handles,
        #[cfg(feature = "androidbox-process0")]
        android_app_live,
        #[cfg(feature = "androidbox-process0")]
        android_app_handles,
        #[cfg(feature = "androidbox-process0")]
        android_app_authority: resident.android_app_authority,
        total_handles,
        // Compatibility aliases carry the selected baseline/final service
        // graph, all-queue, and exact-token semantics.
        resident_control_links_valid: resident.service_topology_valid,
        resident_control_queues_empty: resident.service_queues_empty,
        resident_wait_topology_valid: resident.service_wait_topology_valid,
        resident_endpoint_count: resident.endpoint_count,
        resident_channel_pair_count: resident.channel_pair_count,
        resident_cross_image_links: resident.cross_image_links,
        resident_ui_channel_pair_count: resident.ui_channel_pair_count,
        resident_window_channel_pairs: resident.window_channel_pairs,
        resident_endpoints_unique: resident.endpoints_unique,
        resident_channel_rights_valid: resident.channel_rights_valid,
        resident_surface_capability_count: resident.surface_capability_count,
        resident_surface_capability_session_id: resident.surface_capability_session_id,
        resident_surface_capability_owner_pid: resident.surface_capability_owner_pid,
        resident_surface_capability_owner_valid: resident.surface_capability_owner_valid,
        resident_surface_capability_rights_valid: resident.surface_capability_rights_valid,
        #[cfg(feature = "input-server-runtime")]
        resident_input_capability_count: resident.input_capability_count,
        #[cfg(feature = "input-server-runtime")]
        resident_input_capability_session_id: resident.input_capability_session_id,
        #[cfg(feature = "input-server-runtime")]
        resident_input_capability_owner_pid: resident.input_capability_owner_pid,
        #[cfg(feature = "input-server-runtime")]
        resident_input_capability_owner_valid: resident.input_capability_owner_valid,
        #[cfg(feature = "input-server-runtime")]
        resident_input_capability_rights_valid: resident.input_capability_rights_valid,
        #[cfg(feature = "input-server-runtime")]
        resident_input_channel_pair_count: resident.input_channel_pair_count,
        resident_graphics_buffer_handle_count: resident.graphics_buffer_handle_count,
        resident_graphics_buffer_producer_pid: resident.graphics_buffer_producer_pid,
        resident_graphics_buffer_identity: resident.graphics_buffer_identity,
        resident_graphics_buffer_identity_count: resident.graphics_buffer_identity_count,
        resident_graphics_buffer_identities: resident.graphics_buffer_identities,
        resident_graphics_buffer_identity_handle_counts: resident
            .graphics_buffer_identity_handle_counts,
        resident_graphics_buffer_identity_owner_bitmaps: resident
            .graphics_buffer_identity_owner_bitmaps,
        resident_graphics_buffer_identity_producer_pids: resident
            .graphics_buffer_identity_producer_pids,
        resident_graphics_buffer_identities_share_producer: resident
            .graphics_buffer_identities_share_producer,
        resident_graphics_buffer_owners_valid: if MULTI_WINDOW_RUNTIME_ACTIVE {
            resident.graphics_buffer_handle_count == 4
                && resident.graphics_buffer_identity_count == 2
                && resident
                    .graphics_buffer_identities
                    .iter()
                    .all(Option::is_some)
                && resident.graphics_buffer_identity_handle_counts == [2, 2]
                && resident.graphics_buffer_identity_owner_bitmaps == [0b01, 0b01]
                && resident.graphics_buffer_owner_bitmap == 0b01
                && resident.graphics_buffer_producer_pid != 0
                && resident.graphics_buffer_producer_pid == resident.surface_capability_owner_pid
                && resident.surface_capability_owner_valid
                && resident.graphics_buffer_identity_producer_pids
                    == [resident.graphics_buffer_producer_pid; 2]
                && resident.graphics_buffer_identities_share_producer
        } else if GRAPHICS_SWAPCHAIN_RUNTIME_ACTIVE {
            resident.graphics_buffer_handle_count == 4
                && resident.graphics_buffer_identity_count == 2
                && resident
                    .graphics_buffer_identities
                    .iter()
                    .all(Option::is_some)
                && resident.graphics_buffer_identity_handle_counts == [2, 2]
                && resident.graphics_buffer_identity_owner_bitmaps == [0b11, 0b11]
                && resident.graphics_buffer_producer_pid != 0
                && resident.graphics_buffer_identity_producer_pids
                    == [resident.graphics_buffer_producer_pid; 2]
                && resident.graphics_buffer_identities_share_producer
        } else {
            resident.graphics_buffer_identity_count == 1
                && resident.graphics_buffer_identity_handle_counts == [2, 0]
                && resident.graphics_buffer_identity_owner_bitmaps == [0b11, 0]
                && resident.graphics_buffer_owner_bitmap == 0b11
        },
        resident_graphics_buffer_rights_valid: resident.graphics_buffer_rights_valid,
        graphics_mapping_maps: graphics_mapping_stats.maps,
        graphics_mapping_unmaps: graphics_mapping_stats.unmaps,
        graphics_mapping_protects: graphics_mapping_stats.protects,
        graphics_mapping_live_mappings: graphics_mapping_stats.live_mappings,
        graphics_mapping_live_pages: graphics_mapping_stats.live_pages,
        resident_graphics_mapping_count: resident_graphics_mappings.count,
        resident_graphics_mapping_mapped_pages: resident_graphics_mappings.mapped_pages,
        resident_graphics_mapping_producer_count: resident_graphics_mappings.producer_count,
        resident_graphics_mapping_consumer_count: resident_graphics_mappings.consumer_count,
        resident_graphics_mapping_shared_pairs: resident_graphics_mappings.shared_pairs,
        resident_graphics_mapping_roles_access_valid: resident_graphics_mappings.roles_access_valid,
        resident_graphics_mapping_topology_valid: resident_graphics_mappings.topology_valid,
        resident_graphics_mapping_shared_alias_valid: resident_graphics_mappings.shared_alias_valid,
        resident_graphics_mapping_contexts_distinct: resident_graphics_mappings.contexts_distinct,
        resident_service_topology_valid: resident.service_topology_valid,
        resident_service_queues_empty: resident.service_queues_empty,
        resident_service_wait_topology_valid: resident.service_wait_topology_valid,
        resident_wait_tokens: resident.wait_tokens,
        child_asid: LAST_CHILD_ASID.load(Ordering::Acquire) as u8,
        asid_reused: ASID_REUSED.load(Ordering::Acquire),
        slot_reused: SLOT_REUSED.load(Ordering::Acquire),
        stale_rejected: STALE_REJECTED.load(Ordering::Acquire),
        root_isolated: ROOT_ISOLATED.load(Ordering::Acquire),
        child_frames_isolated: CHILD_FRAMES_ISOLATED.load(Ordering::Acquire),
        frame_restored: FRAME_RESTORED.load(Ordering::Acquire),
        heap_restored: HEAP_RESTORED.load(Ordering::Acquire),
        child_handle_isolated: CHILD_HANDLE_ISOLATED.load(Ordering::Acquire),
        startup_moves: STARTUP_MOVES.load(Ordering::Acquire),
        exit_validation_failures: EXIT_VALIDATION_FAILURES.load(Ordering::Acquire),
        nonstandard_exit_codes: NONSTANDARD_EXIT_CODES.load(Ordering::Acquire),
        non_target_reaps: NON_TARGET_REAPS.load(Ordering::Acquire),
        frame_destroy_delta_valid: FRAME_DESTROY_DELTA_VALID.load(Ordering::Acquire),
        distinct_live_slots: DISTINCT_LIVE_SLOTS.load(Ordering::Acquire),
        distinct_live_asids: DISTINCT_LIVE_ASIDS.load(Ordering::Acquire),
        distinct_live_roots: DISTINCT_LIVE_ROOTS.load(Ordering::Acquire),
        distinct_live_frames: DISTINCT_LIVE_FRAMES.load(Ordering::Acquire),
    };
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

/// Selects the resident proof that matches the exact live spawn generations.
///
/// InitReady is intentionally validated against the three-child baseline. A
/// Later publication in spawn slot seven switches the default profile to the
/// final M32 proof. The M33 profile requires replacement slot eight and the
/// M34 crash-recovery profile requires the second replacement in slot nine.
/// Intermediate secondary/UI/recovery construction cannot masquerade as any
/// final boundary. The caller owns one outer IRQ-masked domain, so token
/// publication, handle generation, and endpoint ownership cannot change.
fn resident_service_topology(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    child_spawn_pids: [u64; CHILD_SPAWN_EVIDENCE_CAPACITY],
    child_spawn_images: [u64; CHILD_SPAWN_EVIDENCE_CAPACITY],
) -> ResidentTopologyEvidence {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("resident service topology checked with IRQ enabled");
    }

    #[cfg(feature = "input-server-runtime")]
    {
        #[cfg(all(
            feature = "service-supervisor-runtime",
            not(feature = "service-dependency-runtime")
        ))]
        if service_supervisor_degraded_resident_ready(slots, child_spawn_pids[9]) {
            return resident_service_supervisor_degraded_topology(
                slots,
                process_wait,
                child_spawn_pids,
                child_spawn_images,
            );
        }
        resident_input_server_service_topology(
            slots,
            process_wait,
            child_spawn_pids,
            child_spawn_images,
        )
    }

    #[cfg(not(feature = "input-server-runtime"))]
    {
        let _ = child_spawn_images;
        // M38 deliberately retires both SurfaceServer and App after the producer
        // dies, then keeps only the stable Init--Launcher lifecycle link. Select
        // that smaller final graph before the generic app-lifecycle proof, which
        // necessarily requires both retired images to remain resident.
        #[cfg(all(
            feature = "graphics-producer-orphan-runtime",
            not(feature = "graphics-surface-restart-runtime"),
            not(feature = "app-crash-recovery-runtime")
        ))]
        if child_spawn_pids[7] != 0 {
            return resident_graphics_producer_orphan_service_topology(
                slots,
                process_wait,
                child_spawn_pids,
            );
        }

        if child_spawn_pids[7] != 0 {
            #[cfg(feature = "app-lifecycle-runtime")]
            {
                resident_app_lifecycle_service_topology(slots, process_wait, child_spawn_pids)
            }
            #[cfg(not(feature = "app-lifecycle-runtime"))]
            {
                resident_m32_service_topology(slots, process_wait, child_spawn_pids)
            }
        } else {
            resident_baseline_service_topology(slots, process_wait, child_spawn_pids)
        }
    }
}

#[cfg(feature = "service-supervisor-runtime")]
fn service_supervisor_degraded_resident_ready(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    replacement_raw_process_id: u64,
) -> bool {
    if replacement_raw_process_id == 0 {
        return false;
    }
    let replacement = ProcessId(replacement_raw_process_id);
    let Some(slot_index) = replacement.slot() else {
        return false;
    };
    let slot = &slots[slot_index];
    replacement.generation() != 0
        && replacement.generation().checked_add(1) == Some(slot.generation)
        && slot.process.is_none()
        && !slot.completion.is_pending()
}

#[cfg(feature = "input-server-runtime")]
fn resident_input_server_service_topology(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    child_spawn_pids: [u64; CHILD_SPAWN_EVIDENCE_CAPACITY],
    child_spawn_images: [u64; CHILD_SPAWN_EVIDENCE_CAPACITY],
) -> ResidentTopologyEvidence {
    let mut evidence =
        collect_resident_topology(slots, INPUT_SERVER_FINAL_RESIDENT_ENDPOINT_COUNT, 1, 4, 1);

    let init = unique_live_process_by_image(slots, UserImageId::Init);
    let manager = live_spawned_process(slots, child_spawn_pids[3], UserImageId::ServiceManager);
    let provider = live_spawned_process(slots, child_spawn_pids[1], UserImageId::Provider);
    let primary = live_spawned_process(slots, child_spawn_pids[2], UserImageId::Client);
    let secondary = live_spawned_process(slots, child_spawn_pids[4], UserImageId::Client);
    #[cfg(feature = "service-dependency-runtime")]
    let dependency_input_pid = SERVICE_DEPENDENCY_INPUT_REPLACEMENT_PID.load(Ordering::Acquire);
    #[cfg(not(feature = "service-dependency-runtime"))]
    let dependency_input_pid = 0;
    let input_spawn_pid = if SERVICE_DEPENDENCY_RUNTIME_ACTIVE {
        dependency_input_pid
    } else if INPUT_SERVER_RESTART_RUNTIME_ACTIVE && child_spawn_pids[9] != 0 {
        child_spawn_pids[9]
    } else {
        child_spawn_pids[5]
    };
    let input_server = live_spawned_process(slots, input_spawn_pid, UserImageId::InputServer);
    let surface_spawn_index =
        if SERVICE_DEPENDENCY_RUNTIME_ACTIVE || INPUT_SERVER_SURFACE_RESTART_RUNTIME_ACTIVE {
            9
        } else {
            6
        };
    let surface_server = live_spawned_process(
        slots,
        child_spawn_pids[surface_spawn_index],
        UserImageId::SurfaceServer,
    );
    let launcher = live_spawned_process(slots, child_spawn_pids[7], UserImageId::Launcher);
    let app = live_spawned_process(slots, child_spawn_pids[8], UserImageId::App);
    let (
        Some(init),
        Some(manager),
        Some(provider),
        Some(primary),
        Some(secondary),
        Some(input_server),
        Some(surface_server),
        Some(launcher),
        Some(app),
    ) = (
        init,
        manager,
        provider,
        primary,
        secondary,
        input_server,
        surface_server,
        launcher,
        app,
    )
    else {
        return evidence;
    };

    evidence.wait_tokens = [
        scheduler::pending_object_wait_token_irq_masked(init.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(manager.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(provider.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(primary.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(secondary.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(input_server.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(surface_server.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(launcher.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(app.id.raw()),
    ];
    evidence.surface_capability_owner_valid = evidence.surface_capability_count == 1
        && evidence.surface_capability_owner_pid == surface_server.id.raw();

    let expected_live_pids = [
        manager.id.raw(),
        provider.id.raw(),
        primary.id.raw(),
        secondary.id.raw(),
        input_server.id.raw(),
        surface_server.id.raw(),
        launcher.id.raw(),
        app.id.raw(),
    ];
    let replacement_spawn_evidence_valid = if SERVICE_DEPENDENCY_RUNTIME_ACTIVE {
        #[cfg(feature = "service-dependency-runtime")]
        {
            child_spawn_pids.iter().all(|pid| *pid != 0)
                && child_spawn_images
                    == [
                        UserImageId::ServiceManager.raw(),
                        UserImageId::Provider.raw(),
                        UserImageId::Client.raw(),
                        UserImageId::ServiceManager.raw(),
                        UserImageId::Client.raw(),
                        UserImageId::InputServer.raw(),
                        UserImageId::SurfaceServer.raw(),
                        UserImageId::Launcher.raw(),
                        UserImageId::App.raw(),
                        UserImageId::SurfaceServer.raw(),
                    ]
                && dependency_input_pid != 0
                && SERVICE_DEPENDENCY_INPUT_REPLACEMENT_IMAGE.load(Ordering::Acquire)
                    == UserImageId::InputServer.raw()
                && exact_reaped_process_replacement(
                    slots,
                    child_spawn_pids[5],
                    input_server,
                    UserImageId::InputServer,
                )
                && exact_reaped_process_replacement(
                    slots,
                    child_spawn_pids[6],
                    surface_server,
                    UserImageId::SurfaceServer,
                )
        }
        #[cfg(not(feature = "service-dependency-runtime"))]
        {
            false
        }
    } else if INPUT_SERVER_RESTART_RUNTIME_ACTIVE {
        child_spawn_pids.iter().all(|pid| *pid != 0)
            && child_spawn_images
                == [
                    UserImageId::ServiceManager.raw(),
                    UserImageId::Provider.raw(),
                    UserImageId::Client.raw(),
                    UserImageId::ServiceManager.raw(),
                    UserImageId::Client.raw(),
                    UserImageId::InputServer.raw(),
                    UserImageId::SurfaceServer.raw(),
                    UserImageId::Launcher.raw(),
                    UserImageId::App.raw(),
                    UserImageId::InputServer.raw(),
                ]
            && exact_reaped_process_replacement(
                slots,
                child_spawn_pids[5],
                input_server,
                UserImageId::InputServer,
            )
    } else if INPUT_SERVER_SURFACE_RESTART_RUNTIME_ACTIVE {
        child_spawn_pids.iter().all(|pid| *pid != 0)
            && child_spawn_images[5] == UserImageId::InputServer.raw()
            && child_spawn_images[6] == UserImageId::SurfaceServer.raw()
            && child_spawn_images[7] == UserImageId::Launcher.raw()
            && child_spawn_images[8] == UserImageId::App.raw()
            && child_spawn_images[9] == UserImageId::SurfaceServer.raw()
            && exact_reaped_process_replacement(
                slots,
                child_spawn_pids[6],
                surface_server,
                UserImageId::SurfaceServer,
            )
    } else {
        child_spawn_pids[..9].iter().all(|pid| *pid != 0) && child_spawn_pids[9] == 0
    };
    let process_roles_valid = init.is_init
        && expected_live_pids.iter().all(|pid| *pid != 0)
        && expected_live_pids.iter().enumerate().all(|(index, pid)| {
            expected_live_pids[index + 1..]
                .iter()
                .all(|other| other != pid)
        })
        && replacement_spawn_evidence_valid
        && child_spawn_pids[0] != child_spawn_pids[3]
        && slots.iter().filter(|slot| slot.process.is_some()).count() == 9
        && init.handles.len() == 8
        && manager.handles.len() == 5
        && provider.handles.len() == 3
        && primary.handles.len().checked_add(secondary.handles.len()) == Some(4)
        && input_server.handles.len() == 3
        && surface_server.handles.len() == 9
        && launcher.handles.len() == 2
        && app.handles.len() == 2;
    let graphics_valid = evidence.graphics_buffer_handle_count == 4
        && evidence.graphics_buffer_producer_pid == surface_server.id.raw()
        && evidence.graphics_buffer_identity_count == 2
        && evidence
            .graphics_buffer_identities
            .iter()
            .all(Option::is_some)
        && evidence.graphics_buffer_identity == evidence.graphics_buffer_identities[0]
        && evidence.graphics_buffer_identity_handle_counts == [2, 2]
        && evidence.graphics_buffer_identity_owner_bitmaps == [0b01, 0b01]
        && evidence.graphics_buffer_identity_producer_pids == [surface_server.id.raw(); 2]
        && evidence.graphics_buffer_identities_share_producer
        && evidence.graphics_buffer_owner_bitmap == 0b01
        && evidence.graphics_buffer_rights_valid;

    evidence.service_topology_valid = process_roles_valid
        && evidence.endpoint_count == INPUT_SERVER_FINAL_RESIDENT_ENDPOINT_COUNT
        && evidence.channel_pair_count == 15
        && evidence.cross_image_links == [1, 1, 2, 2, 2, 0]
        && evidence.ui_channel_pair_count == 2
        && evidence.window_channel_pairs == [1, 1, 1, 1, 1]
        && evidence.input_channel_pair_count == 2
        && evidence.within_image_links == 0
        && evidence.endpoints_unique
        && evidence.channel_rights_valid
        && evidence.surface_capability_count == 1
        && evidence.surface_capability_session_id != 0
        && evidence.surface_capability_owner_valid
        && evidence.surface_capability_rights_valid
        && evidence.input_capability_count == 1
        && evidence.input_capability_session_id != 0
        && evidence.input_capability_owner_pid == input_server.id.raw()
        && evidence.input_capability_owner_valid
        && evidence.input_capability_rights_valid
        && graphics_valid
        && evidence.unexpected_handle_count == 0;

    let object_wait = scheduler::object_wait_snapshot();
    let wait_many = scheduler::wait_many_snapshot();
    let wait_array = scheduler::wait_array_snapshot();
    evidence.service_wait_topology_valid = evidence.service_topology_valid
        && evidence.service_queues_empty
        && if SERVICE_DEPENDENCY_RUNTIME_ACTIVE
            || INPUT_SERVER_SURFACE_RESTART_RUNTIME_ACTIVE
            || INPUT_SERVER_RESTART_RUNTIME_ACTIVE
        {
            exact_input_server_surface_restart_resident_service_waits(
                InputServerResidentProcesses {
                    init,
                    manager,
                    provider,
                    primary,
                    secondary,
                    input_server,
                    surface_server,
                    launcher,
                    app,
                },
                process_wait,
                evidence.wait_tokens,
            )
        } else {
            evidence.wait_tokens.iter().all(Option::is_some)
        }
        && !process_wait.waiting
        && process_wait.pending_target == 0
        && object_wait.pending == RESIDENT_WAIT_TOKEN_COUNT
        && wait_many.pending == 2
        && wait_array.pending == 7
        && wait_many.pending.checked_add(wait_array.pending) == Some(object_wait.pending);
    evidence
}

#[cfg(feature = "service-supervisor-runtime")]
fn resident_service_supervisor_degraded_topology(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    child_spawn_pids: [u64; CHILD_SPAWN_EVIDENCE_CAPACITY],
    child_spawn_images: [u64; CHILD_SPAWN_EVIDENCE_CAPACITY],
) -> ResidentTopologyEvidence {
    let mut evidence = collect_resident_topology(slots, 26, 1, 4, 0);

    let init = unique_live_process_by_image(slots, UserImageId::Init);
    let manager = live_spawned_process(slots, child_spawn_pids[3], UserImageId::ServiceManager);
    let provider = live_spawned_process(slots, child_spawn_pids[1], UserImageId::Provider);
    let primary = live_spawned_process(slots, child_spawn_pids[2], UserImageId::Client);
    let secondary = live_spawned_process(slots, child_spawn_pids[4], UserImageId::Client);
    let surface_server =
        live_spawned_process(slots, child_spawn_pids[6], UserImageId::SurfaceServer);
    let launcher = live_spawned_process(slots, child_spawn_pids[7], UserImageId::Launcher);
    let app = live_spawned_process(slots, child_spawn_pids[8], UserImageId::App);
    let (
        Some(init),
        Some(manager),
        Some(provider),
        Some(primary),
        Some(secondary),
        Some(surface_server),
        Some(launcher),
        Some(app),
    ) = (
        init,
        manager,
        provider,
        primary,
        secondary,
        surface_server,
        launcher,
        app,
    )
    else {
        return evidence;
    };

    evidence.wait_tokens = [
        scheduler::pending_object_wait_token_irq_masked(init.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(manager.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(provider.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(primary.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(secondary.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(surface_server.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(launcher.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(app.id.raw()),
        None,
    ];
    evidence.surface_capability_owner_valid = evidence.surface_capability_count == 1
        && evidence.surface_capability_owner_pid == surface_server.id.raw();

    let expected_live_pids = [
        manager.id.raw(),
        provider.id.raw(),
        primary.id.raw(),
        secondary.id.raw(),
        surface_server.id.raw(),
        launcher.id.raw(),
        app.id.raw(),
    ];
    let spawn_evidence_valid = child_spawn_pids.iter().all(|pid| *pid != 0)
        && child_spawn_images
            == [
                UserImageId::ServiceManager.raw(),
                UserImageId::Provider.raw(),
                UserImageId::Client.raw(),
                UserImageId::ServiceManager.raw(),
                UserImageId::Client.raw(),
                UserImageId::InputServer.raw(),
                UserImageId::SurfaceServer.raw(),
                UserImageId::Launcher.raw(),
                UserImageId::App.raw(),
                UserImageId::InputServer.raw(),
            ]
        && exact_reaped_input_server_generation_chain(
            slots,
            child_spawn_pids[5],
            child_spawn_pids[9],
        );
    let process_roles_valid = init.is_init
        && unique_live_process_by_image(slots, UserImageId::InputServer).is_none()
        && expected_live_pids.iter().all(|pid| *pid != 0)
        && expected_live_pids.iter().enumerate().all(|(index, pid)| {
            expected_live_pids[index + 1..]
                .iter()
                .all(|other| other != pid)
        })
        && spawn_evidence_valid
        && child_spawn_pids[0] != child_spawn_pids[3]
        && slots.iter().filter(|slot| slot.process.is_some()).count() == 8
        && init.handles.len() == 7
        && manager.handles.len() == 5
        && provider.handles.len() == 3
        && primary.handles.len().checked_add(secondary.handles.len()) == Some(4)
        && surface_server.handles.len() == 8
        && launcher.handles.len() == 2
        && app.handles.len() == 2;
    let graphics_valid = evidence.graphics_buffer_handle_count == 4
        && evidence.graphics_buffer_producer_pid == surface_server.id.raw()
        && evidence.graphics_buffer_identity_count == 2
        && evidence
            .graphics_buffer_identities
            .iter()
            .all(Option::is_some)
        && evidence.graphics_buffer_identity == evidence.graphics_buffer_identities[0]
        && evidence.graphics_buffer_identity_handle_counts == [2, 2]
        && evidence.graphics_buffer_identity_owner_bitmaps == [0b01, 0b01]
        && evidence.graphics_buffer_identity_producer_pids == [surface_server.id.raw(); 2]
        && evidence.graphics_buffer_identities_share_producer
        && evidence.graphics_buffer_owner_bitmap == 0b01
        && evidence.graphics_buffer_rights_valid;

    evidence.service_topology_valid = process_roles_valid
        && evidence.endpoint_count == 26
        && evidence.channel_pair_count == 13
        && evidence.cross_image_links == [1, 1, 2, 2, 2, 0]
        && evidence.ui_channel_pair_count == 2
        && evidence.window_channel_pairs == [1, 1, 1, 1, 1]
        && evidence.input_channel_pair_count == 0
        && evidence.within_image_links == 0
        && evidence.endpoints_unique
        && evidence.channel_rights_valid
        && evidence.surface_capability_count == 1
        && evidence.surface_capability_session_id != 0
        && evidence.surface_capability_owner_valid
        && evidence.surface_capability_rights_valid
        && evidence.input_capability_count == 0
        && evidence.input_capability_session_id == 0
        && evidence.input_capability_owner_pid == 0
        && !evidence.input_capability_owner_valid
        && evidence.input_capability_rights_valid
        && graphics_valid
        && evidence.unexpected_handle_count == 0;
    evidence.service_wait_topology_valid = evidence.service_topology_valid
        && evidence.service_queues_empty
        && exact_service_supervisor_degraded_resident_service_waits(
            ServiceSupervisorResidentProcesses {
                init,
                manager,
                provider,
                primary,
                secondary,
                surface_server,
                launcher,
                app,
            },
            process_wait,
            evidence.wait_tokens,
        );
    evidence
}

#[cfg(feature = "service-supervisor-runtime")]
fn exact_reaped_input_server_generation_chain(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    old_raw_process_id: u64,
    replacement_raw_process_id: u64,
) -> bool {
    if old_raw_process_id == 0 || replacement_raw_process_id == 0 {
        return false;
    }
    let old = ProcessId(old_raw_process_id);
    let replacement = ProcessId(replacement_raw_process_id);
    let (Some(old_slot), Some(replacement_slot)) = (old.slot(), replacement.slot()) else {
        return false;
    };
    if old_slot != replacement_slot {
        return false;
    }
    let slot = &slots[replacement_slot];
    old.generation() != 0
        && old.generation().checked_add(1) == Some(replacement.generation())
        && replacement.generation().checked_add(1) == Some(slot.generation)
        && slot.process.is_none()
        && !slot.completion.is_pending()
        && live_spawned_process(slots, old_raw_process_id, UserImageId::InputServer).is_none()
        && live_spawned_process(slots, replacement_raw_process_id, UserImageId::InputServer)
            .is_none()
}

/// Proves the M38 post-orphan resident graph after both graphics owners have
/// been reaped and both scrubbed pool slots have been reused by Launcher.
///
/// SurfaceServer and App are intentionally absent. The core registry graph is
/// unchanged, while Init retains exactly one lifecycle channel to Launcher.
#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "input-server-runtime"),
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn resident_graphics_producer_orphan_service_topology(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    child_spawn_pids: [u64; CHILD_SPAWN_EVIDENCE_CAPACITY],
) -> ResidentTopologyEvidence {
    let mut evidence = collect_resident_topology(
        slots,
        GRAPHICS_PRODUCER_ORPHAN_FINAL_RESIDENT_ENDPOINT_COUNT,
        0,
        0,
        0,
    );

    let init = unique_live_process_by_image(slots, UserImageId::Init);
    let manager = live_spawned_process(slots, child_spawn_pids[3], UserImageId::ServiceManager);
    let provider = live_spawned_process(slots, child_spawn_pids[1], UserImageId::Provider);
    let primary = live_spawned_process(slots, child_spawn_pids[2], UserImageId::Client);
    let secondary = live_spawned_process(slots, child_spawn_pids[4], UserImageId::Client);
    let launcher = live_spawned_process(slots, child_spawn_pids[6], UserImageId::Launcher);
    let (Some(init), Some(manager), Some(provider), Some(primary), Some(secondary), Some(launcher)) =
        (init, manager, provider, primary, secondary, launcher)
    else {
        return evidence;
    };

    evidence.wait_tokens = [
        scheduler::pending_object_wait_token_irq_masked(manager.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(provider.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(primary.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(secondary.id.raw()),
        None,
        None,
        None,
        None,
    ];
    evidence.surface_capability_owner_valid = false;

    let expected_live_pids = [
        manager.id.raw(),
        provider.id.raw(),
        primary.id.raw(),
        secondary.id.raw(),
        launcher.id.raw(),
    ];
    let retired_graphics_owners_absent =
        live_spawned_process(slots, child_spawn_pids[5], UserImageId::SurfaceServer).is_none()
            && live_spawned_process(slots, child_spawn_pids[7], UserImageId::App).is_none()
            && slots
                .iter()
                .filter_map(|slot| slot.process.as_deref())
                .all(|process| {
                    process.id.raw() != child_spawn_pids[5]
                        && process.id.raw() != child_spawn_pids[7]
                });
    let live_process_count = slots.iter().filter(|slot| slot.process.is_some()).count();
    let process_roles_valid = init.is_init
        && expected_live_pids.iter().all(|pid| *pid != 0)
        && expected_live_pids.iter().enumerate().all(|(index, pid)| {
            expected_live_pids[index + 1..]
                .iter()
                .all(|other| other != pid)
        })
        && child_spawn_pids[..8].iter().all(|pid| *pid != 0)
        && child_spawn_pids[8..].iter().all(|pid| *pid == 0)
        && child_spawn_pids[0] != child_spawn_pids[3]
        && retired_graphics_owners_absent
        && live_process_count == expected_live_pids.len() + 1
        && init.handles.len() == 5
        && manager.handles.len() == 5
        && provider.handles.len() == 3
        && primary.handles.len().checked_add(secondary.handles.len()) == Some(4)
        && launcher.handles.len() == 1;
    evidence.service_topology_valid = process_roles_valid
        && evidence.endpoint_count == GRAPHICS_PRODUCER_ORPHAN_FINAL_RESIDENT_ENDPOINT_COUNT
        && evidence.channel_pair_count == 9
        && evidence.cross_image_links == [1, 1, 2, 2, 2, 0]
        && evidence.ui_channel_pair_count == 0
        && evidence.window_channel_pairs == [0, 1, 0, 0, 0]
        && evidence.within_image_links == 0
        && evidence.endpoints_unique
        && evidence.channel_rights_valid
        && evidence.surface_capability_count == 0
        && evidence.surface_capability_session_id == 0
        && evidence.surface_capability_owner_pid == 0
        && !evidence.surface_capability_owner_valid
        && evidence.surface_capability_rights_valid
        && evidence.graphics_buffer_handle_count == 0
        && evidence.graphics_buffer_producer_pid == 0
        && evidence.graphics_buffer_identity.is_none()
        && evidence.graphics_buffer_owner_bitmap == 0
        && evidence.graphics_buffer_rights_valid
        && evidence.unexpected_handle_count == 0;

    evidence.service_wait_topology_valid = evidence.service_topology_valid
        && evidence.service_queues_empty
        && exact_graphics_producer_orphan_resident_service_waits(
            GraphicsProducerOrphanResidentProcesses {
                init,
                manager,
                provider,
                primary,
                secondary,
                launcher,
            },
            process_wait,
            evidence.wait_tokens,
        );
    evidence
}

/// Proves the 12-endpoint graph present when init publishes InitReady.
///
/// M20 changes the dispatch loops before adding the secondary Client, so the
/// strict baseline waits are manager Array(4), provider Array(2), and primary
/// Client LegacyMany(2). This validator remains separate from the final graph:
/// InitReady must not depend on a child that has not been spawned yet.
fn resident_baseline_service_topology(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    child_spawn_pids: [u64; CHILD_SPAWN_EVIDENCE_CAPACITY],
) -> ResidentTopologyEvidence {
    let mut evidence = collect_resident_topology(slots, BASELINE_RESIDENT_ENDPOINT_COUNT, 0, 0, 0);

    let init = unique_live_process_by_image(slots, UserImageId::Init);
    let manager = unique_live_process_by_image(slots, UserImageId::ServiceManager);
    let provider = unique_live_process_by_image(slots, UserImageId::Provider);
    let client = unique_live_process_by_image(slots, UserImageId::Client);
    let (Some(init), Some(manager), Some(provider), Some(client)) =
        (init, manager, provider, client)
    else {
        return evidence;
    };

    evidence.wait_tokens[0] = scheduler::pending_object_wait_token_irq_masked(manager.id.raw());
    evidence.wait_tokens[1] = scheduler::pending_object_wait_token_irq_masked(provider.id.raw());
    evidence.wait_tokens[2] = scheduler::pending_object_wait_token_irq_masked(client.id.raw());

    evidence.surface_capability_owner_valid = false;

    let process_roles_valid = init.is_init
        && !manager.is_init
        && !provider.is_init
        && !client.is_init
        && init.handles.len() == 3
        && manager.handles.len() == 4
        && provider.handles.len() == 3
        && client.handles.len() == 2
        && child_spawn_pids[1] == provider.id.raw()
        && child_spawn_pids[2] == client.id.raw()
        && child_spawn_pids[3] == manager.id.raw()
        && child_spawn_pids[..4].iter().all(|pid| *pid != 0)
        && child_spawn_pids[0] != child_spawn_pids[3]
        && child_spawn_pids[4..].iter().all(|pid| *pid == 0);
    evidence.service_topology_valid = process_roles_valid
        && evidence.endpoint_count == BASELINE_RESIDENT_ENDPOINT_COUNT
        && evidence.channel_pair_count == 6
        && evidence.cross_image_links == [1, 1, 1, 2, 1, 0]
        && evidence.ui_channel_pair_count == 0
        && evidence.window_channel_pairs == [0; 5]
        && evidence.within_image_links == 0
        && evidence.endpoints_unique
        && evidence.channel_rights_valid
        && evidence.surface_capability_count == 0
        && evidence.surface_capability_session_id == 0
        && evidence.surface_capability_owner_pid == 0
        && !evidence.surface_capability_owner_valid
        && evidence.surface_capability_rights_valid
        && evidence.graphics_buffer_handle_count == 0
        && evidence.graphics_buffer_producer_pid == 0
        && evidence.graphics_buffer_identity.is_none()
        && evidence.graphics_buffer_owner_bitmap == 0
        && evidence.graphics_buffer_rights_valid
        && evidence.unexpected_handle_count == 0;

    evidence.service_wait_topology_valid = evidence.service_topology_valid
        && evidence.service_queues_empty
        && exact_baseline_resident_service_waits(
            init,
            manager,
            provider,
            client,
            process_wait,
            evidence.wait_tokens,
        );
    evidence
}

/// Proves the default M32 core-service + two-client UI graph using exact spawn
/// identities.
///
/// Client is deliberately not looked up by unique image: two live instances
/// are required, and accepting either instance in the other's session would
/// erase the isolation property this proof is meant to establish.
#[cfg(not(feature = "app-lifecycle-runtime"))]
fn resident_m32_service_topology(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    child_spawn_pids: [u64; CHILD_SPAWN_EVIDENCE_CAPACITY],
) -> ResidentTopologyEvidence {
    #[cfg(feature = "androidbox-interactive0")]
    let expected_graphics_buffer_handle_count = 6;
    #[cfg(not(feature = "androidbox-interactive0"))]
    let expected_graphics_buffer_handle_count = 2;
    let mut evidence = collect_resident_topology(
        slots,
        M32_FINAL_RESIDENT_ENDPOINT_COUNT,
        1,
        expected_graphics_buffer_handle_count,
        0,
    );

    let init = unique_live_process_by_image(slots, UserImageId::Init);
    let manager = live_spawned_process(slots, child_spawn_pids[3], UserImageId::ServiceManager);
    let provider = live_spawned_process(slots, child_spawn_pids[1], UserImageId::Provider);
    let primary = live_spawned_process(slots, child_spawn_pids[2], UserImageId::Client);
    let secondary = live_spawned_process(slots, child_spawn_pids[4], UserImageId::Client);
    let surface_server =
        live_spawned_process(slots, child_spawn_pids[5], UserImageId::SurfaceServer);
    let launcher = live_spawned_process(slots, child_spawn_pids[6], UserImageId::Launcher);
    let app = live_spawned_process(slots, child_spawn_pids[7], UserImageId::App);
    #[cfg(all(feature = "androidbox-process0", not(feature = "androidbox-restart0")))]
    let android_app = live_spawned_process(slots, child_spawn_pids[8], UserImageId::AndroidApp);
    #[cfg(feature = "androidbox-restart0")]
    let android_app = unique_live_process_by_image(slots, UserImageId::AndroidApp);
    let (
        Some(init),
        Some(manager),
        Some(provider),
        Some(primary),
        Some(secondary),
        Some(surface_server),
        Some(launcher),
        Some(app),
    ) = (
        init,
        manager,
        provider,
        primary,
        secondary,
        surface_server,
        launcher,
        app,
    )
    else {
        return evidence;
    };

    #[cfg(feature = "androidbox-process0")]
    let android_app_token = android_app
        .and_then(|process| scheduler::pending_object_wait_token_irq_masked(process.id.raw()));
    #[cfg(not(feature = "androidbox-process0"))]
    let android_app_token = None;
    evidence.wait_tokens = [
        scheduler::pending_object_wait_token_irq_masked(manager.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(provider.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(primary.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(secondary.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(surface_server.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(launcher.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(app.id.raw()),
        android_app_token,
    ];

    evidence.surface_capability_owner_valid = evidence.surface_capability_count == 1
        && evidence.surface_capability_owner_pid == surface_server.id.raw();

    #[cfg(not(feature = "androidbox-process0"))]
    let expected_live_pids = [
        manager.id.raw(),
        provider.id.raw(),
        primary.id.raw(),
        secondary.id.raw(),
        surface_server.id.raw(),
        launcher.id.raw(),
        app.id.raw(),
    ];
    #[cfg(feature = "androidbox-process0")]
    let expected_live_pids = [
        manager.id.raw(),
        provider.id.raw(),
        primary.id.raw(),
        secondary.id.raw(),
        surface_server.id.raw(),
        launcher.id.raw(),
        app.id.raw(),
        android_app.map_or(0, |process| process.id.raw()),
    ];
    #[cfg(not(feature = "androidbox-process0"))]
    let spawn_evidence_valid = child_spawn_pids[..8].iter().all(|pid| *pid != 0)
        && child_spawn_pids[8..].iter().all(|pid| *pid == 0);
    #[cfg(all(feature = "androidbox-process0", not(feature = "androidbox-restart0")))]
    let spawn_evidence_valid = child_spawn_pids[..9].iter().all(|pid| *pid != 0)
        && child_spawn_pids[9] == 0
        && android_app.is_some_and(|process| child_spawn_pids[8] == process.id.raw());
    #[cfg(feature = "androidbox-restart0")]
    let spawn_evidence_valid = if child_spawn_pids[9] == 0 {
        child_spawn_pids[..9].iter().all(|pid| *pid != 0)
            && android_app.is_some_and(|process| child_spawn_pids[8] == process.id.raw())
    } else {
        let old = child_spawn_pids[8];
        let new = child_spawn_pids[9];
        child_spawn_pids.iter().all(|pid| *pid != 0)
            && android_app.is_some_and(|process| new == process.id.raw())
            && old != new
            && old as u32 == new as u32
            && ((old >> 32) as u32).checked_add(1) == Some((new >> 32) as u32)
    };
    #[cfg(not(feature = "androidbox-process0"))]
    let app_and_android_app_handles_valid = app.handles.len() == 2;
    #[cfg(all(feature = "androidbox-process0", not(feature = "androidbox-restart0")))]
    let app_and_android_app_handles_valid = app.handles.len() == 3
        && android_app.is_some_and(|process| process.handles.len() == 1)
        && evidence.android_app_authority.objects_valid;
    #[cfg(feature = "androidbox-restart0")]
    let app_and_android_app_handles_valid = app.handles.len() == 4
        && android_app.is_some_and(|process| process.handles.len() == 1)
        && evidence.android_app_authority.objects_valid;
    #[cfg(feature = "androidbox-interactive0")]
    let ui_process_handle_shape_valid = surface_server.handles.len() == 7
        && launcher.handles.len() == 2
        && app_and_android_app_handles_valid;
    #[cfg(not(feature = "androidbox-interactive0"))]
    let ui_process_handle_shape_valid = surface_server.handles.len() == 4
        && launcher.handles.len() == 1
        && app_and_android_app_handles_valid;
    #[cfg(feature = "androidbox-interactive0")]
    let graphics_topology_valid = collect_androidbox_interactive_graphics_evidence(slots).valid
        && evidence.graphics_buffer_handle_count == 6;
    #[cfg(not(feature = "androidbox-interactive0"))]
    let graphics_topology_valid = evidence.graphics_buffer_handle_count == 2
        && evidence.graphics_buffer_producer_pid == app.id.raw()
        && evidence.graphics_buffer_identity.is_some()
        && evidence.graphics_buffer_identity_count == 1
        && evidence.graphics_buffer_identities[0] == evidence.graphics_buffer_identity
        && evidence.graphics_buffer_identities[1].is_none()
        && evidence.graphics_buffer_identity_handle_counts == [2, 0]
        && evidence.graphics_buffer_identity_owner_bitmaps == [0b11, 0]
        && evidence.graphics_buffer_identity_producer_pids == [app.id.raw(), 0]
        && evidence.graphics_buffer_identities_share_producer
        && evidence.graphics_buffer_owner_bitmap == 0b11
        && evidence.graphics_buffer_rights_valid;
    let process_roles_valid = init.is_init
        && expected_live_pids.iter().all(|pid| *pid != 0)
        && expected_live_pids.iter().enumerate().all(|(index, pid)| {
            expected_live_pids[index + 1..]
                .iter()
                .all(|other| other != pid)
        })
        && spawn_evidence_valid
        && child_spawn_pids[0] != child_spawn_pids[3]
        && init.handles.len()
            == if cfg!(feature = "androidbox-restart0") {
                5
            } else {
                4
            }
        && manager.handles.len() == 5
        && provider.handles.len() == 3
        && primary.handles.len().checked_add(secondary.handles.len()) == Some(4)
        && ui_process_handle_shape_valid;
    evidence.service_topology_valid = process_roles_valid
        && evidence.endpoint_count == M32_FINAL_RESIDENT_ENDPOINT_COUNT
        && evidence.channel_pair_count
            == if cfg!(feature = "androidbox-restart0") {
                11
            } else {
                10
            }
        && evidence.cross_image_links == [1, 1, 2, 2, 2, 0]
        && evidence.ui_channel_pair_count == 2
        && evidence.window_channel_pairs
            == if cfg!(feature = "androidbox-restart0") {
                [0, 0, 1, 1, 1]
            } else {
                [0, 0, 0, 1, 1]
            }
        && evidence.within_image_links == 0
        && evidence.endpoints_unique
        && evidence.channel_rights_valid
        && evidence.surface_capability_count == 1
        && evidence.surface_capability_session_id != 0
        && evidence.surface_capability_owner_valid
        && evidence.surface_capability_rights_valid
        && graphics_topology_valid
        && evidence.unexpected_handle_count == 0;

    evidence.service_wait_topology_valid = evidence.service_topology_valid
        && evidence.service_queues_empty
        && exact_m32_resident_service_waits(
            FinalResidentProcesses {
                init,
                manager,
                provider,
                primary,
                secondary,
                surface_server,
                launcher,
                app,
            },
            process_wait,
            evidence.wait_tokens,
        );
    evidence
}

/// Proves the final M33/M34 core-service and generation-safe window graph.
///
/// Spawn slot seven is the first retired App. M33 ends with slot eight live;
/// M34 additionally requires slot eight retired and slot nine live. Every App
/// identity must reuse one process slot with an exactly incremented generation,
/// and the selected final identity must be the sole live App image.
#[cfg(all(
    feature = "app-lifecycle-runtime",
    not(feature = "input-server-runtime")
))]
fn resident_app_lifecycle_service_topology(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    child_spawn_pids: [u64; CHILD_SPAWN_EVIDENCE_CAPACITY],
) -> ResidentTopologyEvidence {
    let expected_graphics_buffer_handle_count =
        if MULTI_WINDOW_RUNTIME_ACTIVE || GRAPHICS_SWAPCHAIN_RUNTIME_ACTIVE {
            4
        } else {
            2
        };
    let mut evidence = collect_resident_topology(
        slots,
        APP_LIFECYCLE_FINAL_RESIDENT_ENDPOINT_COUNT,
        1,
        expected_graphics_buffer_handle_count,
        0,
    );

    let init = unique_live_process_by_image(slots, UserImageId::Init);
    let manager = live_spawned_process(slots, child_spawn_pids[3], UserImageId::ServiceManager);
    let provider = live_spawned_process(slots, child_spawn_pids[1], UserImageId::Provider);
    let primary = live_spawned_process(slots, child_spawn_pids[2], UserImageId::Client);
    let secondary = live_spawned_process(slots, child_spawn_pids[4], UserImageId::Client);
    #[cfg(any(
        not(feature = "graphics-surface-restart-runtime"),
        feature = "app-crash-recovery-runtime"
    ))]
    let surface_server =
        live_spawned_process(slots, child_spawn_pids[5], UserImageId::SurfaceServer);
    #[cfg(all(
        feature = "graphics-surface-restart-runtime",
        not(feature = "app-crash-recovery-runtime")
    ))]
    let surface_server =
        live_spawned_process(slots, child_spawn_pids[8], UserImageId::SurfaceServer);
    let launcher = live_spawned_process(slots, child_spawn_pids[6], UserImageId::Launcher);
    let app = unique_live_process_by_image(slots, UserImageId::App);
    let (
        Some(init),
        Some(manager),
        Some(provider),
        Some(primary),
        Some(secondary),
        Some(surface_server),
        Some(launcher),
        Some(app),
    ) = (
        init,
        manager,
        provider,
        primary,
        secondary,
        surface_server,
        launcher,
        app,
    )
    else {
        return evidence;
    };

    evidence.wait_tokens = [
        scheduler::pending_object_wait_token_irq_masked(init.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(manager.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(provider.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(primary.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(secondary.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(surface_server.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(launcher.id.raw()),
        scheduler::pending_object_wait_token_irq_masked(app.id.raw()),
    ];

    evidence.surface_capability_owner_valid = evidence.surface_capability_count == 1
        && evidence.surface_capability_owner_pid == surface_server.id.raw();

    let expected_live_pids = [
        manager.id.raw(),
        provider.id.raw(),
        primary.id.raw(),
        secondary.id.raw(),
        surface_server.id.raw(),
        launcher.id.raw(),
        app.id.raw(),
    ];
    #[cfg(all(
        any(
            not(feature = "graphics-surface-restart-runtime"),
            feature = "app-crash-recovery-runtime"
        ),
        not(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        ))
    ))]
    let app_generation_chain_valid =
        exact_app_generation_chain(slots, &child_spawn_pids[7..=FINAL_APP_SPAWN_INDEX], app);
    // M39--M41 deliberately keep one App alive while exercising frame pacing
    // and bounded buffer ownership. Its process identity must remain the first
    // generation; continuity is proved by the graphics ledgers instead of an
    // App replacement chain.
    #[cfg(all(
        feature = "graphics-frame-clock-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    let app_generation_chain_valid = child_spawn_pids[7] == app.id.raw()
        && app.id.generation() == 1
        && slots
            .get(app.id.slot().unwrap_or(PROCESS_CAPACITY))
            .is_some_and(|slot| {
                slot.process
                    .as_deref()
                    .is_some_and(|live| live.id == app.id)
            });
    // M37 deliberately keeps the original mapped App alive while replacing
    // only SurfaceServer.  Prove that this is still the first App generation
    // instead of weakening the multi-generation M33/M34 helper to accept a
    // one-element chain.
    #[cfg(all(
        feature = "graphics-surface-restart-runtime",
        not(feature = "app-crash-recovery-runtime")
    ))]
    let app_generation_chain_valid = child_spawn_pids[7] == app.id.raw()
        && app.id.generation() == 1
        && slots
            .get(app.id.slot().unwrap_or(PROCESS_CAPACITY))
            .is_some_and(|slot| {
                slot.process
                    .as_deref()
                    .is_some_and(|live| live.id == app.id)
            });
    #[cfg(any(
        not(feature = "graphics-surface-restart-runtime"),
        feature = "app-crash-recovery-runtime"
    ))]
    let spawn_evidence_valid = child_spawn_pids[..=FINAL_APP_SPAWN_INDEX]
        .iter()
        .all(|pid| *pid != 0)
        && child_spawn_pids[FINAL_APP_SPAWN_INDEX + 1..]
            .iter()
            .all(|pid| *pid == 0);
    #[cfg(all(
        feature = "graphics-surface-restart-runtime",
        not(feature = "app-crash-recovery-runtime")
    ))]
    let spawn_evidence_valid = child_spawn_pids[..=8].iter().all(|pid| *pid != 0)
        && child_spawn_pids[9] == 0
        && child_spawn_pids[8] == surface_server.id.raw();
    let graphics_process_handle_counts_valid = if MULTI_WINDOW_RUNTIME_ACTIVE {
        surface_server.handles.len() == 8 && app.handles.len() == 2
    } else if GRAPHICS_SWAPCHAIN_RUNTIME_ACTIVE {
        surface_server.handles.len() == 6 && app.handles.len() == 4
    } else {
        surface_server.handles.len() == 5 && app.handles.len() == 3
    };
    let resident_graphics_buffers_valid = if MULTI_WINDOW_RUNTIME_ACTIVE {
        evidence.graphics_buffer_handle_count == 4
            && evidence.graphics_buffer_producer_pid == surface_server.id.raw()
            && evidence.graphics_buffer_identity_count == 2
            && evidence
                .graphics_buffer_identities
                .iter()
                .all(Option::is_some)
            && evidence.graphics_buffer_identity == evidence.graphics_buffer_identities[0]
            && evidence.graphics_buffer_identity_handle_counts == [2, 2]
            && evidence.graphics_buffer_identity_owner_bitmaps == [0b01, 0b01]
            && evidence.graphics_buffer_identity_producer_pids == [surface_server.id.raw(); 2]
            && evidence.graphics_buffer_identities_share_producer
            && evidence.graphics_buffer_owner_bitmap == 0b01
            && evidence.graphics_buffer_rights_valid
    } else if GRAPHICS_SWAPCHAIN_RUNTIME_ACTIVE {
        evidence.graphics_buffer_handle_count == 4
            && evidence.graphics_buffer_producer_pid == app.id.raw()
            && evidence.graphics_buffer_identity_count == 2
            && evidence
                .graphics_buffer_identities
                .iter()
                .all(Option::is_some)
            && evidence.graphics_buffer_identity == evidence.graphics_buffer_identities[0]
            && evidence.graphics_buffer_identity_handle_counts == [2, 2]
            && evidence.graphics_buffer_identity_owner_bitmaps == [0b11, 0b11]
            && evidence.graphics_buffer_identity_producer_pids == [app.id.raw(); 2]
            && evidence.graphics_buffer_identities_share_producer
            && evidence.graphics_buffer_owner_bitmap == 0b11
            && evidence.graphics_buffer_rights_valid
    } else {
        evidence.graphics_buffer_handle_count == 2
            && evidence.graphics_buffer_producer_pid == app.id.raw()
            && evidence.graphics_buffer_identity.is_some_and(|identity| {
                identity.generation() == FINAL_APP_GRAPHICS_BUFFER_GENERATION
            })
            && evidence.graphics_buffer_identity_count == 1
            && evidence.graphics_buffer_identities[0] == evidence.graphics_buffer_identity
            && evidence.graphics_buffer_identities[1].is_none()
            && evidence.graphics_buffer_identity_handle_counts == [2, 0]
            && evidence.graphics_buffer_identity_owner_bitmaps == [0b11, 0]
            && evidence.graphics_buffer_identity_producer_pids == [app.id.raw(), 0]
            && evidence.graphics_buffer_identities_share_producer
            && evidence.graphics_buffer_owner_bitmap == 0b11
            && evidence.graphics_buffer_rights_valid
    };
    let process_roles_valid = init.is_init
        && expected_live_pids.iter().all(|pid| *pid != 0)
        && expected_live_pids.iter().enumerate().all(|(index, pid)| {
            expected_live_pids[index + 1..]
                .iter()
                .all(|other| other != pid)
        })
        && spawn_evidence_valid
        && child_spawn_pids[0] != child_spawn_pids[3]
        && child_spawn_pids[FINAL_APP_SPAWN_INDEX] == app.id.raw()
        && app_generation_chain_valid
        && init.handles.len() == 7
        && manager.handles.len() == 5
        && provider.handles.len() == 3
        && primary.handles.len().checked_add(secondary.handles.len()) == Some(4)
        && launcher.handles.len() == 2
        && graphics_process_handle_counts_valid;
    evidence.service_topology_valid = process_roles_valid
        && evidence.endpoint_count == APP_LIFECYCLE_FINAL_RESIDENT_ENDPOINT_COUNT
        && evidence.channel_pair_count == 13
        && evidence.cross_image_links == [1, 1, 2, 2, 2, 0]
        && evidence.ui_channel_pair_count == 2
        && evidence.window_channel_pairs == [1, 1, 1, 1, 1]
        && evidence.within_image_links == 0
        && evidence.endpoints_unique
        && evidence.channel_rights_valid
        && evidence.surface_capability_count == 1
        && evidence.surface_capability_session_id != 0
        && evidence.surface_capability_owner_valid
        && evidence.surface_capability_rights_valid
        && resident_graphics_buffers_valid
        && evidence.unexpected_handle_count == 0;

    evidence.service_wait_topology_valid = evidence.service_topology_valid
        && evidence.service_queues_empty
        && exact_app_lifecycle_resident_service_waits(
            FinalResidentProcesses {
                init,
                manager,
                provider,
                primary,
                secondary,
                surface_server,
                launcher,
                app,
            },
            process_wait,
            evidence.wait_tokens,
        );
    evidence
}

/// Requires a nonempty retired-generation prefix followed by the sole live
/// App. All identities must decode to one slot and strictly consecutive,
/// nonzero generations. A stale retired identity must not resolve as live.
#[cfg(all(
    feature = "app-lifecycle-runtime",
    not(feature = "input-server-runtime")
))]
fn exact_app_generation_chain(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    app_process_ids: &[u64],
    live_app: &Process,
) -> bool {
    if app_process_ids.len() < 2 || app_process_ids.last().copied() != Some(live_app.id.raw()) {
        return false;
    }
    let Some(expected_slot) = live_app.id.slot() else {
        return false;
    };
    let mut previous_generation: Option<u32> = None;
    for (index, raw_process_id) in app_process_ids.iter().copied().enumerate() {
        let id = ProcessId(raw_process_id);
        let generation = id.generation();
        if raw_process_id == 0
            || id.slot() != Some(expected_slot)
            || generation == 0
            || (index == 0 && generation != 1)
        {
            return false;
        }
        if previous_generation.is_some_and(|previous| previous.checked_add(1) != Some(generation)) {
            return false;
        }
        if index + 1 < app_process_ids.len()
            && live_spawned_process(slots, raw_process_id, UserImageId::App).is_some()
        {
            return false;
        }
        previous_generation = Some(generation);
    }
    true
}

/// Collects the capability graph without assigning service roles to nodes.
/// The role-specific baseline/final validators apply their own exact cardinality
/// and wait-token contracts to this common, non-vacuous evidence.
fn collect_resident_topology(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    expected_endpoint_count: usize,
    expected_surface_count: usize,
    expected_graphics_buffer_count: usize,
    expected_input_count: usize,
) -> ResidentTopologyEvidence {
    if expected_endpoint_count > RESIDENT_ENDPOINT_CAPACITY {
        panic!("resident topology endpoint expectation exceeded its bound");
    }

    let mut evidence = ResidentTopologyEvidence::EMPTY;
    let mut endpoints = [None; RESIDENT_ENDPOINT_CAPACITY];
    let mut stored_endpoints = 0_usize;
    #[cfg(feature = "androidbox-process0")]
    let mut android_app_authority = AndroidAppAuthorityEvidence::EMPTY;
    #[cfg(feature = "androidbox-process0")]
    let mut android_app_private_endpoints: [Option<&ChannelEndpoint>; 2] = [None; 2];
    #[cfg(feature = "androidbox-process0")]
    let mut stored_android_app_private_endpoints = 0_usize;
    #[cfg(feature = "androidbox-process0")]
    {
        for process in slots
            .iter()
            .filter_map(|slot| slot.process.as_deref())
            .filter(|process| process.image_id == UserImageId::AndroidApp)
        {
            android_app_authority.worker_live = android_app_authority
                .worker_live
                .checked_add(1)
                .unwrap_or_else(|| panic!("resident AndroidApp live count overflowed"));
            for (object, rights) in process.handles.live_entries() {
                android_app_authority.worker_handles = android_app_authority
                    .worker_handles
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("resident AndroidApp handle count overflowed"));
                let Some(channel) = object.as_channel() else {
                    if object.as_vmo().is_some() && rights == Rights::READ {
                        android_app_authority.worker_read_only_vmo_count = android_app_authority
                            .worker_read_only_vmo_count
                            .checked_add(1)
                            .unwrap_or_else(|| {
                                panic!("resident AndroidApp read-only VMO count overflowed")
                            });
                    } else {
                        android_app_authority.worker_unexpected_handle_count =
                            android_app_authority
                                .worker_unexpected_handle_count
                                .checked_add(1)
                                .unwrap_or_else(|| {
                                    panic!("resident AndroidApp unexpected-handle count overflowed")
                                });
                    }
                    continue;
                };
                android_app_authority.worker_channel_count = android_app_authority
                    .worker_channel_count
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("resident AndroidApp channel count overflowed"));
                android_app_authority.private_endpoint_count = android_app_authority
                    .private_endpoint_count
                    .checked_add(1)
                    .unwrap_or_else(|| {
                        panic!("resident AndroidApp private endpoint count overflowed")
                    });
                android_app_authority.worker_rights_valid &= rights == Rights::ANDROID_APP_CHANNEL;
                android_app_authority.queues_empty &= channel.queued_messages() == 0;
                if android_app_private_endpoints[..stored_android_app_private_endpoints]
                    .iter()
                    .flatten()
                    .any(|existing| existing.same_endpoint(channel))
                {
                    android_app_authority.endpoints_unique = false;
                }
                if let Some(destination) =
                    android_app_private_endpoints.get_mut(stored_android_app_private_endpoints)
                {
                    *destination = Some(channel);
                    stored_android_app_private_endpoints += 1;
                } else {
                    android_app_authority.endpoints_unique = false;
                }
            }
        }
    }
    #[cfg(feature = "androidbox-process0")]
    let stored_android_app_worker_endpoints = stored_android_app_private_endpoints;
    let mut channel_rights_valid = true;
    let mut surface_rights_valid = true;
    #[cfg(feature = "input-server-runtime")]
    let mut input_rights_valid = true;
    let mut graphics_buffer_rights_valid = true;
    let mapped_graphics_profile = cfg!(all(
        feature = "mapped-graphics-runtime",
        not(feature = "app-crash-recovery-runtime")
    ));
    let expected_graphics_server_rights = if mapped_graphics_profile {
        Rights::GRAPHICS_BUFFER_MAPPED_SERVER
    } else {
        Rights::GRAPHICS_BUFFER_SERVER
    };
    let expected_graphics_producer_rights = if mapped_graphics_profile {
        Rights::GRAPHICS_BUFFER_MAPPED_PRODUCER
    } else {
        Rights::GRAPHICS_BUFFER_DEFAULT
    };
    let mut surface_producer_rights_counts = [0_usize; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY];
    let mut surface_server_rights_counts = [0_usize; RESIDENT_GRAPHICS_BUFFER_IDENTITY_CAPACITY];
    let mut queues_empty = true;
    let mut live_handles = 0_usize;

    for process in slots.iter().filter_map(|slot| slot.process.as_deref()) {
        for (object, rights) in process.handles.live_entries() {
            #[cfg(feature = "androidbox-process0")]
            if process.image_id == UserImageId::AndroidApp {
                continue;
            }
            #[cfg(feature = "androidbox-process0")]
            if let Some(channel) = object.as_channel() {
                let mut shares_android_app_channel = false;
                let mut peer_matches = 0_usize;
                for worker in android_app_private_endpoints[..stored_android_app_worker_endpoints]
                    .iter()
                    .flatten()
                {
                    if worker.same_channel(channel) {
                        shares_android_app_channel = true;
                    }
                    if worker.is_peer_of(channel) {
                        peer_matches = peer_matches
                            .checked_add(1)
                            .unwrap_or_else(|| panic!("AndroidApp private pair count overflowed"));
                    }
                }
                if shares_android_app_channel {
                    android_app_authority.private_endpoint_count = android_app_authority
                        .private_endpoint_count
                        .checked_add(1)
                        .unwrap_or_else(|| {
                            panic!("resident AndroidApp private endpoint count overflowed")
                        });
                    android_app_authority.private_pair_count = android_app_authority
                        .private_pair_count
                        .checked_add(peer_matches)
                        .unwrap_or_else(|| {
                            panic!("resident AndroidApp private pair count overflowed")
                        });
                    if process.image_id == UserImageId::App {
                        android_app_authority.app_endpoint_count = android_app_authority
                            .app_endpoint_count
                            .checked_add(1)
                            .unwrap_or_else(|| {
                                panic!("resident AndroidApp App-endpoint count overflowed")
                            });
                        android_app_authority.app_rights_valid &= rights == Rights::CHANNEL_DEFAULT;
                    } else {
                        android_app_authority.unexpected_private_owner_count =
                            android_app_authority
                                .unexpected_private_owner_count
                                .checked_add(1)
                                .unwrap_or_else(|| {
                                    panic!("resident AndroidApp private owner count overflowed")
                                });
                        android_app_authority.app_rights_valid = false;
                    }
                    android_app_authority.queues_empty &= channel.queued_messages() == 0;
                    if android_app_private_endpoints[..stored_android_app_private_endpoints]
                        .iter()
                        .flatten()
                        .any(|existing| existing.same_endpoint(channel))
                    {
                        android_app_authority.endpoints_unique = false;
                    }
                    if let Some(destination) =
                        android_app_private_endpoints.get_mut(stored_android_app_private_endpoints)
                    {
                        *destination = Some(channel);
                        stored_android_app_private_endpoints += 1;
                    } else {
                        android_app_authority.endpoints_unique = false;
                    }
                    continue;
                }
            }
            live_handles = live_handles
                .checked_add(1)
                .unwrap_or_else(|| panic!("resident live-handle count overflowed"));

            if let Some(channel) = object.as_channel() {
                evidence.endpoint_count = evidence
                    .endpoint_count
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("resident endpoint count overflowed"));
                channel_rights_valid &= rights == Rights::CHANNEL_DEFAULT;
                queues_empty &= channel.queued_messages() == 0;
                if stored_endpoints < endpoints.len() {
                    endpoints[stored_endpoints] = Some(ResidentEndpoint {
                        image_id: process.image_id,
                        channel,
                    });
                    stored_endpoints += 1;
                }
                continue;
            }

            if let Some(surface) = object.as_surface() {
                evidence.surface_capability_count = evidence
                    .surface_capability_count
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("resident surface capability count overflowed"));
                surface_rights_valid &= rights == Rights::SURFACE_DEFAULT;
                if evidence.surface_capability_count == 1 {
                    evidence.surface_capability_session_id = surface.session_id();
                    evidence.surface_capability_owner_pid = process.id.raw();
                }
                continue;
            }

            #[cfg(feature = "input-server-runtime")]
            if let Some(input) = object.as_input() {
                evidence.input_capability_count = evidence
                    .input_capability_count
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("resident input capability count overflowed"));
                input_rights_valid &= rights == Rights::INPUT_DEFAULT;
                if evidence.input_capability_count == 1 {
                    evidence.input_capability_session_id = input.session_id();
                    evidence.input_capability_owner_pid = process.id.raw();
                }
                continue;
            }

            if let Some(buffer) = object.as_graphics_buffer() {
                evidence.graphics_buffer_handle_count = evidence
                    .graphics_buffer_handle_count
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("resident graphics-buffer handle count overflowed"));
                let identity = buffer.identity();
                let producer_pid = buffer.producer_pid();
                let identity_index = evidence.graphics_buffer_identities
                    [..evidence.graphics_buffer_identity_count]
                    .iter()
                    .position(|candidate| *candidate == Some(identity))
                    .or_else(|| {
                        let index = evidence.graphics_buffer_identity_count;
                        let destination = evidence.graphics_buffer_identities.get_mut(index);
                        match destination {
                            Some(destination) => {
                                *destination = Some(identity);
                                evidence.graphics_buffer_identity_count = evidence
                                    .graphics_buffer_identity_count
                                    .checked_add(1)
                                    .unwrap_or_else(|| {
                                        panic!("resident graphics-buffer identity count overflowed")
                                    });
                                evidence.graphics_buffer_identity_producer_pids[index] =
                                    producer_pid;
                                if index == 0 {
                                    evidence.graphics_buffer_identity = Some(identity);
                                    evidence.graphics_buffer_producer_pid = producer_pid;
                                    evidence.graphics_buffer_identities_share_producer = true;
                                } else {
                                    evidence.graphics_buffer_identities_share_producer &=
                                        evidence.graphics_buffer_producer_pid == producer_pid;
                                }
                                Some(index)
                            }
                            None => {
                                graphics_buffer_rights_valid = false;
                                None
                            }
                        }
                    });
                if let Some(identity_index) = identity_index {
                    evidence.graphics_buffer_identity_handle_counts[identity_index] = evidence
                        .graphics_buffer_identity_handle_counts[identity_index]
                        .checked_add(1)
                        .unwrap_or_else(|| {
                            panic!("resident graphics-buffer identity handle count overflowed")
                        });
                    graphics_buffer_rights_valid &= evidence.graphics_buffer_identity_producer_pids
                        [identity_index]
                        == producer_pid;
                    evidence.graphics_buffer_identities_share_producer &=
                        evidence.graphics_buffer_producer_pid == producer_pid;
                }
                match process.image_id {
                    UserImageId::SurfaceServer => {
                        evidence.graphics_buffer_owner_bitmap |= 0b01;
                        if let Some(identity_index) = identity_index {
                            evidence.graphics_buffer_identity_owner_bitmaps[identity_index] |= 0b01;
                        }
                        if MULTI_WINDOW_RUNTIME_ACTIVE {
                            graphics_buffer_rights_valid &= producer_pid == process.id.raw();
                            match (identity_index, rights) {
                                (Some(index), Rights::GRAPHICS_BUFFER_MAPPED_PRODUCER) => {
                                    surface_producer_rights_counts[index] =
                                        surface_producer_rights_counts[index]
                                            .checked_add(1)
                                            .unwrap_or_else(|| {
                                                panic!(
                                                    "resident SurfaceServer producer-right count overflowed"
                                                )
                                            });
                                }
                                (Some(index), Rights::GRAPHICS_BUFFER_MAPPED_SERVER) => {
                                    surface_server_rights_counts[index] =
                                        surface_server_rights_counts[index]
                                            .checked_add(1)
                                            .unwrap_or_else(|| {
                                                panic!(
                                                    "resident SurfaceServer server-right count overflowed"
                                                )
                                            });
                                }
                                _ => graphics_buffer_rights_valid = false,
                            }
                        } else {
                            graphics_buffer_rights_valid &=
                                rights == expected_graphics_server_rights;
                        }
                    }
                    UserImageId::App => {
                        evidence.graphics_buffer_owner_bitmap |= 0b10;
                        if let Some(identity_index) = identity_index {
                            evidence.graphics_buffer_identity_owner_bitmaps[identity_index] |= 0b10;
                        }
                        graphics_buffer_rights_valid &= !MULTI_WINDOW_RUNTIME_ACTIVE
                            && rights == expected_graphics_producer_rights
                            && producer_pid == process.id.raw();
                    }
                    UserImageId::Init
                    | UserImageId::ServiceManager
                    | UserImageId::Provider
                    | UserImageId::Client
                    | UserImageId::Launcher
                    | UserImageId::InputServer
                    | UserImageId::StorageServer => {
                        graphics_buffer_rights_valid = false;
                    }
                    #[cfg(feature = "androidbox-process0")]
                    UserImageId::AndroidApp => {
                        graphics_buffer_rights_valid = false;
                    }
                }
                continue;
            }

            evidence.unexpected_handle_count = evidence
                .unexpected_handle_count
                .checked_add(1)
                .unwrap_or_else(|| panic!("resident unexpected-handle count overflowed"));
        }
    }

    #[cfg(feature = "androidbox-process0")]
    {
        if let (Some(app), Some(worker)) = (
            unique_live_process_by_image(slots, UserImageId::App),
            unique_live_process_by_image(slots, UserImageId::AndroidApp),
        ) && let (Some(app_space), Some(worker_space)) =
            (app.address_space.as_ref(), worker.address_space.as_ref())
        {
            let app_image = crate::user_images::get(UserImageId::App);
            let worker_image = crate::user_images::get(UserImageId::AndroidApp);
            android_app_authority.app_pid = app.id.raw();
            android_app_authority.worker_pid = worker.id.raw();
            android_app_authority.app_asid = u64::from(app_space.asid());
            android_app_authority.worker_asid = u64::from(worker_space.asid());
            android_app_authority.app_root = app_space.root_address() as u64;
            android_app_authority.worker_root = worker_space.root_address() as u64;
            android_app_authority.app_image_digest = app_image.digest;
            android_app_authority.worker_image_digest = worker_image.digest;
            android_app_authority.isolation_valid = android_app_authority.app_pid != 0
                && android_app_authority.worker_pid != 0
                && android_app_authority.app_pid != android_app_authority.worker_pid
                && android_app_authority.app_asid != 0
                && android_app_authority.worker_asid != 0
                && android_app_authority.app_asid != android_app_authority.worker_asid
                && android_app_authority.app_root != 0
                && android_app_authority.worker_root != 0
                && android_app_authority.app_root != android_app_authority.worker_root
                && android_app_authority.app_image_digest != 0
                && android_app_authority.worker_image_digest != 0
                && android_app_authority.app_image_digest
                    != android_app_authority.worker_image_digest;
        }
        let channel_isolation_valid = android_app_authority.worker_live == 1
            && android_app_authority.worker_channel_count == 1
            && android_app_authority.worker_unexpected_handle_count == 0
            && android_app_authority.private_endpoint_count == 2
            && android_app_authority.private_pair_count == 1
            && android_app_authority.app_endpoint_count == 1
            && android_app_authority.unexpected_private_owner_count == 0
            && android_app_authority.endpoints_unique
            && android_app_authority.worker_rights_valid
            && android_app_authority.app_rights_valid
            && android_app_authority.isolation_valid;
        android_app_authority.objects_valid = channel_isolation_valid
            && android_app_authority.worker_handles == 1
            && android_app_authority.worker_read_only_vmo_count == 0;
        android_app_authority.runtime_objects_valid = channel_isolation_valid
            && android_app_authority.worker_read_only_vmo_count <= 1
            && android_app_authority.worker_handles
                == android_app_authority
                    .worker_channel_count
                    .checked_add(android_app_authority.worker_read_only_vmo_count)
                    .unwrap_or_else(|| panic!("resident AndroidApp handle shape overflowed"));
        android_app_authority.valid =
            android_app_authority.objects_valid && android_app_authority.queues_empty;
        evidence.android_app_authority = android_app_authority;
    }

    // Classify the unique surface separately from the fixed channel graph so
    // channel cardinality, pair, queue, and rights proofs retain their exact
    // pre-surface meaning.
    evidence.channel_rights_valid =
        evidence.endpoint_count == expected_endpoint_count && channel_rights_valid;
    evidence.surface_capability_rights_valid =
        evidence.surface_capability_count == expected_surface_count && surface_rights_valid;
    #[cfg(feature = "input-server-runtime")]
    {
        evidence.input_capability_rights_valid =
            evidence.input_capability_count == expected_input_count && input_rights_valid;
        evidence.input_capability_owner_valid = evidence.input_capability_count == 1
            && evidence.input_capability_session_id != 0
            && unique_live_process_by_image(slots, UserImageId::InputServer).is_some_and(
                |process| {
                    process.id.raw() == evidence.input_capability_owner_pid
                        && process.handles.live_entries().any(|(object, rights)| {
                            rights == Rights::INPUT_DEFAULT
                                && object.as_input().is_some_and(|capability| {
                                    capability.session_id() == evidence.input_capability_session_id
                                        && capability.process_id()
                                            == evidence.input_capability_owner_pid
                                })
                        })
                },
            );
    }
    let expected_graphics_buffer_identity_count = expected_graphics_buffer_count / 2;
    let multi_window_rights_valid = !MULTI_WINDOW_RUNTIME_ACTIVE
        || (expected_graphics_buffer_count == 0
            && surface_producer_rights_counts == [0, 0]
            && surface_server_rights_counts == [0, 0])
        || (expected_graphics_buffer_count == 4
            && surface_producer_rights_counts == [1, 1]
            && surface_server_rights_counts == [1, 1]);
    evidence.graphics_buffer_rights_valid = expected_graphics_buffer_count.is_multiple_of(2)
        && evidence.graphics_buffer_handle_count == expected_graphics_buffer_count
        && evidence.graphics_buffer_identity_count == expected_graphics_buffer_identity_count
        && graphics_buffer_rights_valid
        && multi_window_rights_valid
        && (evidence.graphics_buffer_identity_count == 0
            || evidence.graphics_buffer_identities_share_producer);
    // "All service queues are empty" is evidence only for the complete
    // expected resident graph. An empty or partial process table must not
    // satisfy that service-specific claim vacuously.
    let expected_handle_count = expected_endpoint_count
        .checked_add(expected_surface_count)
        .and_then(|count| count.checked_add(expected_graphics_buffer_count))
        .and_then(|count| count.checked_add(expected_input_count))
        .unwrap_or_else(|| panic!("resident expected-handle count overflowed"));
    evidence.service_queues_empty = queues_empty
        && evidence.endpoint_count == expected_endpoint_count
        && evidence.surface_capability_count == expected_surface_count
        && {
            #[cfg(feature = "input-server-runtime")]
            {
                evidence.input_capability_count == expected_input_count
            }
            #[cfg(not(feature = "input-server-runtime"))]
            {
                expected_input_count == 0
            }
        }
        && evidence.graphics_buffer_handle_count == expected_graphics_buffer_count
        && evidence.unexpected_handle_count == 0
        && live_handles == expected_handle_count;
    evidence.endpoints_unique = evidence.endpoint_count == expected_endpoint_count;

    for left_index in 0..stored_endpoints {
        let left = endpoints[left_index]
            .unwrap_or_else(|| panic!("resident endpoint prefix contained a hole"));
        for right in endpoints
            .iter()
            .take(stored_endpoints)
            .skip(left_index + 1)
            .map(|endpoint| {
                endpoint.unwrap_or_else(|| panic!("resident endpoint prefix contained a hole"))
            })
        {
            if left.channel.same_endpoint(right.channel) {
                evidence.endpoints_unique = false;
            }
            if !left.channel.is_peer_of(right.channel) {
                continue;
            }
            evidence.channel_pair_count = evidence
                .channel_pair_count
                .checked_add(1)
                .unwrap_or_else(|| panic!("resident channel-pair count overflowed"));
            if let Some(index) = cross_image_link_index(left.image_id, right.image_id) {
                evidence.cross_image_links[index] = evidence.cross_image_links[index]
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("resident cross-image link count overflowed"));
            } else if let Some(index) = window_channel_pair_index(left.image_id, right.image_id) {
                evidence.window_channel_pairs[index] = evidence.window_channel_pairs[index]
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("resident window channel-pair count overflowed"));
                if index >= 3 {
                    evidence.ui_channel_pair_count = evidence
                        .ui_channel_pair_count
                        .checked_add(1)
                        .unwrap_or_else(|| panic!("resident UI channel-pair count overflowed"));
                }
            } else if input_channel_pair(left.image_id, right.image_id) {
                #[cfg(feature = "input-server-runtime")]
                {
                    evidence.input_channel_pair_count = evidence
                        .input_channel_pair_count
                        .checked_add(1)
                        .unwrap_or_else(|| panic!("resident input channel-pair count overflowed"));
                }
            } else {
                evidence.within_image_links = evidence
                    .within_image_links
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("resident within-image link count overflowed"));
            }
        }
    }
    evidence
}

fn input_channel_pair(left: UserImageId, right: UserImageId) -> bool {
    #[cfg(feature = "input-server-runtime")]
    {
        let pair = if left.raw() <= right.raw() {
            (left, right)
        } else {
            (right, left)
        };
        matches!(
            pair,
            (UserImageId::Init, UserImageId::InputServer)
                | (UserImageId::SurfaceServer, UserImageId::InputServer)
        )
    }
    #[cfg(not(feature = "input-server-runtime"))]
    {
        let _ = (left, right);
        false
    }
}

fn window_channel_pair_index(left: UserImageId, right: UserImageId) -> Option<usize> {
    let pair = if left.raw() <= right.raw() {
        (left, right)
    } else {
        (right, left)
    };
    match pair {
        (UserImageId::Init, UserImageId::SurfaceServer) => Some(0),
        (UserImageId::Init, UserImageId::Launcher) => Some(1),
        (UserImageId::Init, UserImageId::App) => Some(2),
        (UserImageId::SurfaceServer, UserImageId::Launcher) => Some(3),
        (UserImageId::SurfaceServer, UserImageId::App) => Some(4),
        _ => None,
    }
}

fn cross_image_link_index(left: UserImageId, right: UserImageId) -> Option<usize> {
    let pair = if left.raw() <= right.raw() {
        (left, right)
    } else {
        (right, left)
    };
    match pair {
        (UserImageId::Init, UserImageId::ServiceManager) => Some(0),
        (UserImageId::Init, UserImageId::Provider) => Some(1),
        (UserImageId::Init, UserImageId::Client) => Some(2),
        (UserImageId::ServiceManager, UserImageId::Provider) => Some(3),
        (UserImageId::ServiceManager, UserImageId::Client) => Some(4),
        (UserImageId::Provider, UserImageId::Client) => Some(5),
        _ => None,
    }
}

fn unique_live_process_by_image(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    image_id: UserImageId,
) -> Option<&Process> {
    let mut found = None;
    for process in slots
        .iter()
        .filter_map(|slot| slot.process.as_deref())
        .filter(|process| process.image_id == image_id)
    {
        if found.replace(process).is_some() {
            return None;
        }
    }
    found
}

fn live_spawned_process(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    raw_process_id: u64,
    image_id: UserImageId,
) -> Option<&Process> {
    if raw_process_id == 0 {
        return None;
    }
    let id = ProcessId(raw_process_id);
    let slot_index = id.slot()?;
    let slot = &slots[slot_index];
    let process = slot.process.as_deref()?;
    (slot.generation == id.generation()
        && process.id == id
        && process.image_id == image_id
        && !process.is_init)
        .then_some(process)
}

/// Proves that one retired child identity was fully reaped before a live
/// replacement reused the same bounded slot at exactly the next generation.
///
/// A live replacement in the slot is not enough by itself: the old
/// completion tombstone must also have been consumed, otherwise the slot was
/// not eligible for reuse through the normal spawn transaction.
#[cfg(feature = "input-server-runtime")]
fn exact_reaped_process_replacement(
    slots: &[ProcessSlot; PROCESS_CAPACITY],
    retired_raw_process_id: u64,
    replacement: &Process,
    image_id: UserImageId,
) -> bool {
    if retired_raw_process_id == 0 || replacement.image_id != image_id || replacement.is_init {
        return false;
    }
    let retired = ProcessId(retired_raw_process_id);
    let Some(slot_index) = retired.slot() else {
        return false;
    };
    let replacement_id = replacement.id;
    let slot = &slots[slot_index];
    retired.generation() != 0
        && replacement_id.slot() == Some(slot_index)
        && retired.generation().checked_add(1) == Some(replacement_id.generation())
        && slot.generation == replacement_id.generation()
        && !slot.completion.is_pending()
        && slot.process.as_deref().is_some_and(|process| {
            process.id == replacement_id && process.image_id == image_id && !process.is_init
        })
        && live_spawned_process(slots, retired_raw_process_id, image_id).is_none()
}

#[cfg(feature = "input-server-runtime")]
struct InputServerResidentProcesses<'a> {
    init: &'a Process,
    manager: &'a Process,
    provider: &'a Process,
    primary: &'a Process,
    secondary: &'a Process,
    input_server: &'a Process,
    surface_server: &'a Process,
    launcher: &'a Process,
    app: &'a Process,
}

#[cfg(feature = "service-supervisor-runtime")]
struct ServiceSupervisorResidentProcesses<'a> {
    init: &'a Process,
    manager: &'a Process,
    provider: &'a Process,
    primary: &'a Process,
    secondary: &'a Process,
    surface_server: &'a Process,
    launcher: &'a Process,
    app: &'a Process,
}

/// Proves the exact nine-token M46/M47 resident wait graph.
///
/// The InputServer token is deliberately heterogeneous: two channel items
/// plus its unique INPUT_DEFAULT capability. All other tokens contain only
/// generation-qualified channel handles. Endpoint peer counts close the
/// graph by role, so the right cardinalities cannot be satisfied by waiting
/// twice on one service or by parking on an unrelated live handle.
#[cfg(feature = "input-server-runtime")]
fn exact_input_server_surface_restart_resident_service_waits(
    processes: InputServerResidentProcesses<'_>,
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    tokens: [Option<ObjectWaitToken>; RESIDENT_WAIT_TOKEN_COUNT],
) -> bool {
    let InputServerResidentProcesses {
        init,
        manager,
        provider,
        primary,
        secondary,
        input_server,
        surface_server,
        launcher,
        app,
    } = processes;
    let [
        Some(init_token),
        Some(manager_token),
        Some(provider_token),
        Some(primary_token),
        Some(secondary_token),
        Some(input_server_token),
        Some(surface_server_token),
        Some(launcher_token),
        Some(app_token),
    ] = tokens
    else {
        return false;
    };
    let signal_bits = ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits();
    if !wait_token_metadata_valid(
        init,
        init_token,
        ObjectWaitCompletionKind::Array,
        8,
        signal_bits,
    ) || !wait_token_metadata_valid(
        manager,
        manager_token,
        ObjectWaitCompletionKind::Array,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        signal_bits,
    ) || !wait_token_metadata_valid(
        provider,
        provider_token,
        ObjectWaitCompletionKind::Array,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        signal_bits,
    ) || !wait_token_metadata_valid(
        primary,
        primary_token,
        ObjectWaitCompletionKind::LegacyMany,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        signal_bits,
    ) || !wait_token_metadata_valid(
        secondary,
        secondary_token,
        ObjectWaitCompletionKind::LegacyMany,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        signal_bits,
    ) || !wait_token_metadata_valid(
        input_server,
        input_server_token,
        ObjectWaitCompletionKind::Array,
        3,
        signal_bits,
    ) || !wait_token_metadata_valid(
        surface_server,
        surface_server_token,
        ObjectWaitCompletionKind::Array,
        4,
        signal_bits,
    ) || !wait_token_metadata_valid(
        launcher,
        launcher_token,
        ObjectWaitCompletionKind::Array,
        2,
        signal_bits,
    ) || !wait_token_metadata_valid(
        app,
        app_token,
        ObjectWaitCompletionKind::Array,
        2,
        signal_bits,
    ) {
        return false;
    }

    let Some(manager_startup) = startup_channel(manager) else {
        return false;
    };
    let Some(provider_startup) = startup_channel(provider) else {
        return false;
    };
    let Some(primary_startup) = startup_channel(primary) else {
        return false;
    };
    let Some(secondary_startup) = startup_channel(secondary) else {
        return false;
    };
    let Some(input_startup) = startup_channel(input_server) else {
        return false;
    };
    let Some(surface_startup) = startup_channel(surface_server) else {
        return false;
    };
    let Some(launcher_startup) = startup_channel(launcher) else {
        return false;
    };
    let Some(app_startup) = startup_channel(app) else {
        return false;
    };
    let Some(input_capability) = unique_input_capability(input_server) else {
        return false;
    };
    let Some(waited_input) = unique_token_input(input_server, input_server_token) else {
        return false;
    };
    if !waited_input.same_input(input_capability) {
        return false;
    }
    let Some(manager_provider_session) = unique_token_channel_peered_by(
        manager,
        manager_token,
        provider,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) else {
        return false;
    };
    let Some(provider_connector) = unique_token_channel_peered_by(
        provider,
        provider_token,
        manager,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
    ) else {
        return false;
    };

    let startup_peers_valid = peer_count_in_process(init, manager_startup) == 1
        && peer_count_in_process(init, provider_startup) == 1
        && peer_count_in_process(init, primary_startup) == 1
        && peer_count_in_process(init, secondary_startup) == 1
        && peer_count_in_process(init, input_startup) == 1
        && peer_count_in_process(init, launcher_startup) == 1
        && peer_count_in_process(init, app_startup) == 1
        && peer_count_in_process(launcher, surface_startup) == 1;
    let init_endpoints = token_channel_count(init, init_token) == 8
        && token_peer_match_count(init, init_token, manager, 8) == 1
        && token_peer_match_count(init, init_token, provider, 8) == 1
        && token_peer_match_count(init, init_token, primary, 8) == 1
        && token_peer_match_count(init, init_token, secondary, 8) == 1
        && token_peer_match_count(init, init_token, input_server, 8) == 1
        && token_peer_match_count(init, init_token, surface_server, 8) == 1
        && token_peer_match_count(init, init_token, launcher, 8) == 1
        && token_peer_match_count(init, init_token, app, 8) == 1;
    let core_endpoints = token_endpoint_match_count(
        manager,
        manager_token,
        manager_startup,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            provider,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            primary,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            secondary,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            provider,
            provider_token,
            provider_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            provider,
            provider_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            primary,
            primary_token,
            primary_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            primary,
            primary_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            secondary,
            secondary_token,
            secondary_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            secondary,
            secondary_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1;
    let input_endpoints = token_channel_count(input_server, input_server_token) == 2
        && token_input_count(input_server, input_server_token) == 1
        && token_endpoint_match_count(input_server, input_server_token, input_startup, 2) == 1
        && token_peer_match_count(input_server, input_server_token, init, 2) == 1
        && token_peer_match_count(input_server, input_server_token, surface_server, 2) == 1;
    let surface_endpoints = token_channel_count(surface_server, surface_server_token) == 4
        && token_endpoint_match_count(surface_server, surface_server_token, surface_startup, 4)
            == 1
        && token_peer_match_count(surface_server, surface_server_token, init, 4) == 1
        && token_peer_match_count(surface_server, surface_server_token, input_server, 4) == 1
        && token_peer_match_count(surface_server, surface_server_token, launcher, 4) == 1
        && token_peer_match_count(surface_server, surface_server_token, app, 4) == 1;
    let client_endpoints = token_channel_count(launcher, launcher_token) == 2
        && token_endpoint_match_count(launcher, launcher_token, launcher_startup, 2) == 1
        && token_peer_match_count(launcher, launcher_token, init, 2) == 1
        && token_peer_match_count(launcher, launcher_token, surface_server, 2) == 1
        && token_channel_count(app, app_token) == 2
        && token_endpoint_match_count(app, app_token, app_startup, 2) == 1
        && token_peer_match_count(app, app_token, init, 2) == 1
        && token_peer_match_count(app, app_token, surface_server, 2) == 1;
    let distinct = wait_token_channels_are_distinct(init, init_token, 8)
        && wait_token_channels_are_distinct(
            manager,
            manager_token,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            provider,
            provider_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            primary,
            primary_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            secondary,
            secondary_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(input_server, input_server_token, 2)
        && wait_token_channels_are_distinct(surface_server, surface_server_token, 4)
        && wait_token_channels_are_distinct(launcher, launcher_token, 2)
        && wait_token_channels_are_distinct(app, app_token, 2);
    let requested = ObjectSignals::from_bits(signal_bits)
        .unwrap_or_else(|| panic!("M46 resident signal mask was invalid"));
    let blocked = wait_token_channels_are_blocked(init, init_token, requested, 8)
        && wait_token_channels_are_blocked(
            manager,
            manager_token,
            requested,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_blocked(
            provider,
            provider_token,
            requested,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_blocked(
            primary,
            primary_token,
            requested,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_blocked(
            secondary,
            secondary_token,
            requested,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_blocked(input_server, input_server_token, requested, 2)
        && !input_capability.signals().intersects(requested)
        && wait_token_channels_are_blocked(surface_server, surface_server_token, requested, 4)
        && wait_token_channels_are_blocked(launcher, launcher_token, requested, 2)
        && wait_token_channels_are_blocked(app, app_token, requested, 2);
    let object_wait = scheduler::object_wait_snapshot();
    let wait_many = scheduler::wait_many_snapshot();
    let wait_array = scheduler::wait_array_snapshot();

    startup_peers_valid
        && init_endpoints
        && core_endpoints
        && input_endpoints
        && surface_endpoints
        && client_endpoints
        && !manager_provider_session.is_peer_of(provider_connector)
        && distinct
        && blocked
        && !process_wait.waiting
        && process_wait.pending_target == 0
        && object_wait.pending == RESIDENT_WAIT_TOKEN_COUNT
        && wait_many.pending == 2
        && wait_array.pending == 7
        && wait_many.pending.checked_add(wait_array.pending) == Some(object_wait.pending)
}

#[cfg(feature = "service-supervisor-runtime")]
fn exact_service_supervisor_degraded_resident_service_waits(
    processes: ServiceSupervisorResidentProcesses<'_>,
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    tokens: [Option<ObjectWaitToken>; RESIDENT_WAIT_TOKEN_COUNT],
) -> bool {
    let ServiceSupervisorResidentProcesses {
        init,
        manager,
        provider,
        primary,
        secondary,
        surface_server,
        launcher,
        app,
    } = processes;
    let [
        Some(init_token),
        Some(manager_token),
        Some(provider_token),
        Some(primary_token),
        Some(secondary_token),
        Some(surface_server_token),
        Some(launcher_token),
        Some(app_token),
        None,
    ] = tokens
    else {
        return false;
    };
    let signal_bits = ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits();
    if !wait_token_metadata_valid(
        init,
        init_token,
        ObjectWaitCompletionKind::Array,
        7,
        signal_bits,
    ) || !wait_token_metadata_valid(
        manager,
        manager_token,
        ObjectWaitCompletionKind::Array,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        signal_bits,
    ) || !wait_token_metadata_valid(
        provider,
        provider_token,
        ObjectWaitCompletionKind::Array,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        signal_bits,
    ) || !wait_token_metadata_valid(
        primary,
        primary_token,
        ObjectWaitCompletionKind::LegacyMany,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        signal_bits,
    ) || !wait_token_metadata_valid(
        secondary,
        secondary_token,
        ObjectWaitCompletionKind::LegacyMany,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        signal_bits,
    ) || !wait_token_metadata_valid(
        surface_server,
        surface_server_token,
        ObjectWaitCompletionKind::Array,
        3,
        signal_bits,
    ) || !wait_token_metadata_valid(
        launcher,
        launcher_token,
        ObjectWaitCompletionKind::Array,
        2,
        signal_bits,
    ) || !wait_token_metadata_valid(
        app,
        app_token,
        ObjectWaitCompletionKind::Array,
        2,
        signal_bits,
    ) {
        return false;
    }

    let Some(manager_startup) = startup_channel(manager) else {
        return false;
    };
    let Some(provider_startup) = startup_channel(provider) else {
        return false;
    };
    let Some(primary_startup) = startup_channel(primary) else {
        return false;
    };
    let Some(secondary_startup) = startup_channel(secondary) else {
        return false;
    };
    let Some(surface_startup) = startup_channel(surface_server) else {
        return false;
    };
    let Some(launcher_startup) = startup_channel(launcher) else {
        return false;
    };
    let Some(app_startup) = startup_channel(app) else {
        return false;
    };
    if unique_surface_capability(surface_server).is_none()
        || unique_token_surface(surface_server, surface_server_token).is_some()
    {
        return false;
    }
    let Some(manager_provider_session) = unique_token_channel_peered_by(
        manager,
        manager_token,
        provider,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) else {
        return false;
    };
    let Some(provider_connector) = unique_token_channel_peered_by(
        provider,
        provider_token,
        manager,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
    ) else {
        return false;
    };

    let startup_peers_valid = peer_count_in_process(init, manager_startup) == 1
        && peer_count_in_process(init, provider_startup) == 1
        && peer_count_in_process(init, primary_startup) == 1
        && peer_count_in_process(init, secondary_startup) == 1
        && peer_count_in_process(init, launcher_startup) == 1
        && peer_count_in_process(init, app_startup) == 1
        && peer_count_in_process(launcher, surface_startup) == 1;
    let init_endpoints = token_channel_count(init, init_token) == 7
        && token_peer_match_count(init, init_token, manager, 7) == 1
        && token_peer_match_count(init, init_token, provider, 7) == 1
        && token_peer_match_count(init, init_token, primary, 7) == 1
        && token_peer_match_count(init, init_token, secondary, 7) == 1
        && token_peer_match_count(init, init_token, surface_server, 7) == 1
        && token_peer_match_count(init, init_token, launcher, 7) == 1
        && token_peer_match_count(init, init_token, app, 7) == 1;
    let core_endpoints = token_endpoint_match_count(
        manager,
        manager_token,
        manager_startup,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            provider,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            primary,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            secondary,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            provider,
            provider_token,
            provider_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            provider,
            provider_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            primary,
            primary_token,
            primary_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            primary,
            primary_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            secondary,
            secondary_token,
            secondary_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            secondary,
            secondary_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1;
    let window_endpoints = token_channel_count(surface_server, surface_server_token) == 3
        && token_endpoint_match_count(surface_server, surface_server_token, surface_startup, 3)
            == 1
        && token_peer_match_count(surface_server, surface_server_token, init, 3) == 1
        && token_peer_match_count(surface_server, surface_server_token, launcher, 3) == 1
        && token_peer_match_count(surface_server, surface_server_token, app, 3) == 1
        && token_channel_count(launcher, launcher_token) == 2
        && token_endpoint_match_count(launcher, launcher_token, launcher_startup, 2) == 1
        && token_peer_match_count(launcher, launcher_token, init, 2) == 1
        && token_peer_match_count(launcher, launcher_token, surface_server, 2) == 1
        && token_channel_count(app, app_token) == 2
        && token_endpoint_match_count(app, app_token, app_startup, 2) == 1
        && token_peer_match_count(app, app_token, init, 2) == 1
        && token_peer_match_count(app, app_token, surface_server, 2) == 1;
    let distinct = wait_token_channels_are_distinct(init, init_token, 7)
        && wait_token_channels_are_distinct(
            manager,
            manager_token,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            provider,
            provider_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            primary,
            primary_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            secondary,
            secondary_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(surface_server, surface_server_token, 3)
        && wait_token_channels_are_distinct(launcher, launcher_token, 2)
        && wait_token_channels_are_distinct(app, app_token, 2);
    let requested = ObjectSignals::from_bits(signal_bits)
        .unwrap_or_else(|| panic!("M48 resident signal mask was invalid"));
    let blocked = wait_token_channels_are_blocked(init, init_token, requested, 7)
        && wait_token_channels_are_blocked(
            manager,
            manager_token,
            requested,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_blocked(
            provider,
            provider_token,
            requested,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_blocked(
            primary,
            primary_token,
            requested,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_blocked(
            secondary,
            secondary_token,
            requested,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_blocked(surface_server, surface_server_token, requested, 3)
        && wait_token_channels_are_blocked(launcher, launcher_token, requested, 2)
        && wait_token_channels_are_blocked(app, app_token, requested, 2);
    let object_wait = scheduler::object_wait_snapshot();
    let wait_many = scheduler::wait_many_snapshot();
    let wait_array = scheduler::wait_array_snapshot();

    startup_peers_valid
        && init_endpoints
        && core_endpoints
        && window_endpoints
        && !manager_provider_session.is_peer_of(provider_connector)
        && distinct
        && blocked
        && !process_wait.waiting
        && process_wait.pending_target == 0
        && object_wait.pending == 8
        && wait_many.pending == 2
        && wait_array.pending == 6
        && wait_many.pending.checked_add(wait_array.pending) == Some(object_wait.pending)
}

fn exact_baseline_resident_service_waits(
    init: &Process,
    manager: &Process,
    provider: &Process,
    client: &Process,
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    tokens: [Option<ObjectWaitToken>; RESIDENT_WAIT_TOKEN_COUNT],
) -> bool {
    let (Some(manager_token), Some(provider_token), Some(client_token)) =
        (tokens[0], tokens[1], tokens[2])
    else {
        return false;
    };
    if tokens[3..].iter().any(Option::is_some) {
        return false;
    }
    let expected_signal_bits = ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits();
    if !wait_token_metadata_valid(
        manager,
        manager_token,
        ObjectWaitCompletionKind::Array,
        BASELINE_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        provider,
        provider_token,
        ObjectWaitCompletionKind::Array,
        2,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        client,
        client_token,
        ObjectWaitCompletionKind::LegacyMany,
        2,
        expected_signal_bits,
    ) || scheduler::pending_object_wait_token_irq_masked(init.id.raw()).is_some()
    {
        return false;
    }

    let Some(manager_startup) = startup_channel(manager) else {
        return false;
    };
    let Some(provider_startup) = startup_channel(provider) else {
        return false;
    };
    let Some(client_startup) = startup_channel(client) else {
        return false;
    };
    let Some(manager_provider_session) = unique_token_channel_peered_by(
        manager,
        manager_token,
        provider,
        BASELINE_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) else {
        return false;
    };
    let Some(provider_connector) = unique_token_channel_peered_by(
        provider,
        provider_token,
        manager,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
    ) else {
        return false;
    };

    let startup_peers_owned_by_init = peer_count_in_process(init, manager_startup) == 1
        && peer_count_in_process(init, provider_startup) == 1
        && peer_count_in_process(init, client_startup) == 1;
    let endpoint_sets_valid = token_endpoint_match_count(
        manager,
        manager_token,
        manager_startup,
        BASELINE_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            provider,
            BASELINE_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            client,
            BASELINE_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            provider,
            provider_token,
            provider_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            provider,
            provider_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            client,
            client_token,
            client_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            client,
            client_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && !manager_provider_session.is_peer_of(provider_connector)
        && wait_token_channels_are_distinct(
            manager,
            manager_token,
            BASELINE_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            provider,
            provider_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(client, client_token, SERVICE_CHANNEL_WAIT_ITEM_COUNT);
    let requested = ObjectSignals::from_bits(expected_signal_bits)
        .unwrap_or_else(|| panic!("resident service signal mask was invalid"));
    let all_waits_blocked = wait_token_channels_are_blocked(
        manager,
        manager_token,
        requested,
        BASELINE_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) && wait_token_channels_are_blocked(
        provider,
        provider_token,
        requested,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
    ) && wait_token_channels_are_blocked(
        client,
        client_token,
        requested,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
    );
    let object_wait = scheduler::object_wait_snapshot();
    let wait_many = scheduler::wait_many_snapshot();
    let wait_array = scheduler::wait_array_snapshot();

    endpoint_sets_valid
        && startup_peers_owned_by_init
        && all_waits_blocked
        && process_wait.waiting
        && process_wait.pending_target == manager.id.raw()
        && object_wait.pending == BASELINE_RESIDENT_WAIT_TOKEN_COUNT
        && wait_many.pending == 1
        && wait_array.pending == 2
        && wait_many.pending.checked_add(wait_array.pending) == Some(object_wait.pending)
}

/// Requires the exact four stable service waits left by M38. Init and Launcher
/// are both spinning after the reuse handshake, and the reaped graphics
/// processes cannot retain scheduler tokens or a supervisor process wait.
#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "input-server-runtime"),
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
struct GraphicsProducerOrphanResidentProcesses<'a> {
    init: &'a Process,
    manager: &'a Process,
    provider: &'a Process,
    primary: &'a Process,
    secondary: &'a Process,
    launcher: &'a Process,
}

#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "input-server-runtime"),
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn exact_graphics_producer_orphan_resident_service_waits(
    processes: GraphicsProducerOrphanResidentProcesses<'_>,
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    tokens: [Option<ObjectWaitToken>; RESIDENT_WAIT_TOKEN_COUNT],
) -> bool {
    let GraphicsProducerOrphanResidentProcesses {
        init,
        manager,
        provider,
        primary,
        secondary,
        launcher,
    } = processes;
    let [
        Some(manager_token),
        Some(provider_token),
        Some(primary_token),
        Some(secondary_token),
        None,
        None,
        None,
        None,
    ] = tokens
    else {
        return false;
    };
    let expected_signal_bits = ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits();
    if !wait_token_metadata_valid(
        manager,
        manager_token,
        ObjectWaitCompletionKind::Array,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        provider,
        provider_token,
        ObjectWaitCompletionKind::Array,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        primary,
        primary_token,
        ObjectWaitCompletionKind::LegacyMany,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        secondary,
        secondary_token,
        ObjectWaitCompletionKind::LegacyMany,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        expected_signal_bits,
    ) || scheduler::pending_object_wait_token_irq_masked(init.id.raw()).is_some()
        || scheduler::pending_object_wait_token_irq_masked(launcher.id.raw()).is_some()
    {
        return false;
    }

    let Some(manager_startup) = startup_channel(manager) else {
        return false;
    };
    let Some(provider_startup) = startup_channel(provider) else {
        return false;
    };
    let Some(primary_startup) = startup_channel(primary) else {
        return false;
    };
    let Some(secondary_startup) = startup_channel(secondary) else {
        return false;
    };
    let Some(launcher_startup) = startup_channel(launcher) else {
        return false;
    };
    let Some(manager_provider_session) = unique_token_channel_peered_by(
        manager,
        manager_token,
        provider,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) else {
        return false;
    };
    let Some(provider_connector) = unique_token_channel_peered_by(
        provider,
        provider_token,
        manager,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
    ) else {
        return false;
    };

    let startup_peers_owned_by_init = peer_count_in_process(init, manager_startup) == 1
        && peer_count_in_process(init, provider_startup) == 1
        && peer_count_in_process(init, primary_startup) == 1
        && peer_count_in_process(init, secondary_startup) == 1
        && peer_count_in_process(init, launcher_startup) == 1;
    let endpoint_sets_valid = token_endpoint_match_count(
        manager,
        manager_token,
        manager_startup,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            provider,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            primary,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            secondary,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            provider,
            provider_token,
            provider_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            provider,
            provider_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            primary,
            primary_token,
            primary_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            primary,
            primary_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            secondary,
            secondary_token,
            secondary_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            secondary,
            secondary_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        // The two manager-provider pairs retain distinct session and registry
        // roles even though neither graphics owner remains resident.
        && !manager_provider_session.is_peer_of(provider_connector)
        && wait_token_channels_are_distinct(
            manager,
            manager_token,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            provider,
            provider_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            primary,
            primary_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            secondary,
            secondary_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        );
    let requested = ObjectSignals::from_bits(expected_signal_bits)
        .unwrap_or_else(|| panic!("resident service signal mask was invalid"));
    let all_waits_blocked = wait_token_channels_are_blocked(
        manager,
        manager_token,
        requested,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) && wait_token_channels_are_blocked(
        provider,
        provider_token,
        requested,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
    ) && wait_token_channels_are_blocked(
        primary,
        primary_token,
        requested,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
    ) && wait_token_channels_are_blocked(
        secondary,
        secondary_token,
        requested,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
    );
    let object_wait = scheduler::object_wait_snapshot();
    let wait_many = scheduler::wait_many_snapshot();
    let wait_array = scheduler::wait_array_snapshot();

    endpoint_sets_valid
        && startup_peers_owned_by_init
        && all_waits_blocked
        && !process_wait.waiting
        && process_wait.pending_target == 0
        && object_wait.pending == 4
        && wait_many.pending == 2
        && wait_array.pending == 2
        && wait_many.pending.checked_add(wait_array.pending) == Some(object_wait.pending)
}

struct FinalResidentProcesses<'a> {
    init: &'a Process,
    manager: &'a Process,
    provider: &'a Process,
    primary: &'a Process,
    secondary: &'a Process,
    surface_server: &'a Process,
    launcher: &'a Process,
    app: &'a Process,
}

#[cfg(not(feature = "app-lifecycle-runtime"))]
fn exact_m32_resident_service_waits(
    processes: FinalResidentProcesses<'_>,
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    tokens: [Option<ObjectWaitToken>; RESIDENT_WAIT_TOKEN_COUNT],
) -> bool {
    let FinalResidentProcesses {
        init,
        manager,
        provider,
        primary,
        secondary,
        surface_server,
        launcher,
        app,
    } = processes;
    #[cfg(not(feature = "androidbox-process0"))]
    let [
        Some(manager_token),
        Some(provider_token),
        Some(primary_token),
        Some(secondary_token),
        Some(surface_server_token),
        Some(launcher_token),
        Some(app_token),
        None,
    ] = tokens
    else {
        return false;
    };
    #[cfg(feature = "androidbox-process0")]
    let [
        Some(manager_token),
        Some(provider_token),
        Some(primary_token),
        Some(secondary_token),
        Some(surface_server_token),
        Some(launcher_token),
        Some(app_token),
        Some(android_app_token),
    ] = tokens
    else {
        return false;
    };
    #[cfg(feature = "androidbox-process0")]
    let Some(android_app) = unique_live_process_by_image(slots_ref(), UserImageId::AndroidApp)
    else {
        return false;
    };
    let expected_signal_bits = ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits();
    #[cfg(feature = "androidbox-process0")]
    let android_app_wait_metadata_valid = wait_token_metadata_valid(
        android_app,
        android_app_token,
        ObjectWaitCompletionKind::Single,
        1,
        expected_signal_bits,
    );
    #[cfg(not(feature = "androidbox-process0"))]
    let android_app_wait_metadata_valid = true;
    if !wait_token_metadata_valid(
        manager,
        manager_token,
        ObjectWaitCompletionKind::Array,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        provider,
        provider_token,
        ObjectWaitCompletionKind::Array,
        2,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        primary,
        primary_token,
        ObjectWaitCompletionKind::LegacyMany,
        2,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        secondary,
        secondary_token,
        ObjectWaitCompletionKind::LegacyMany,
        2,
        expected_signal_bits,
    ) || !surface_server_wait_metadata_valid(surface_server, surface_server_token)
        || !wait_token_metadata_valid(
            launcher,
            launcher_token,
            ObjectWaitCompletionKind::Single,
            LAUNCHER_WAIT_ITEM_COUNT,
            expected_signal_bits,
        )
        || !wait_token_metadata_valid(
            app,
            app_token,
            ObjectWaitCompletionKind::Single,
            APP_WAIT_ITEM_COUNT,
            expected_signal_bits,
        )
        || !android_app_wait_metadata_valid
        || scheduler::pending_object_wait_token_irq_masked(init.id.raw()).is_some()
    {
        return false;
    }

    let Some(manager_startup) = startup_channel(manager) else {
        return false;
    };
    let Some(provider_startup) = startup_channel(provider) else {
        return false;
    };
    let Some(primary_startup) = startup_channel(primary) else {
        return false;
    };
    let Some(secondary_startup) = startup_channel(secondary) else {
        return false;
    };
    let Some(surface_capability) = unique_surface_capability(surface_server) else {
        return false;
    };
    let Some(waited_surface) = unique_token_surface(surface_server, surface_server_token) else {
        return false;
    };
    if !waited_surface.same_surface(surface_capability) {
        return false;
    }
    let Some(surface_ui_channel) = startup_channel(surface_server) else {
        return false;
    };
    let Some(launcher_ui_channel) = startup_channel(launcher) else {
        return false;
    };
    if !surface_ui_channel.is_peer_of(launcher_ui_channel) {
        return false;
    }
    let Some(surface_app_channel) =
        unique_token_channel_peered_by(surface_server, surface_server_token, app, 2)
    else {
        return false;
    };
    #[cfg(not(feature = "androidbox-process0"))]
    let Some(app_ui_channel) = startup_channel(app) else {
        return false;
    };
    #[cfg(feature = "androidbox-process0")]
    let Some(app_ui_channel) = unique_channel_peer_in_process(app, surface_app_channel) else {
        return false;
    };
    if !surface_app_channel.is_peer_of(app_ui_channel)
        || surface_app_channel.same_endpoint(surface_ui_channel)
    {
        return false;
    }
    #[cfg(feature = "androidbox-process0")]
    let Some(android_app_channel) = android_app_startup_channel(android_app) else {
        return false;
    };
    #[cfg(feature = "androidbox-process0")]
    let Some(waited_android_app_channel) =
        android_app_token_channel(android_app, android_app_token, 0)
    else {
        return false;
    };
    #[cfg(feature = "androidbox-process0")]
    let Some(app_android_channel) = unique_channel_peer_in_process(app, android_app_channel) else {
        return false;
    };
    #[cfg(feature = "androidbox-process0")]
    if !waited_android_app_channel.same_endpoint(android_app_channel)
        || !android_app_channel.is_peer_of(app_android_channel)
        || app_android_channel.same_endpoint(app_ui_channel)
    {
        return false;
    }
    let Some(manager_provider_session) = unique_token_channel_peered_by(
        manager,
        manager_token,
        provider,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) else {
        return false;
    };
    let Some(provider_connector) = unique_token_channel_peered_by(
        provider,
        provider_token,
        manager,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
    ) else {
        return false;
    };

    let startup_peers_owned_by_init = peer_count_in_process(init, manager_startup) == 1
        && peer_count_in_process(init, provider_startup) == 1
        && peer_count_in_process(init, primary_startup) == 1
        && peer_count_in_process(init, secondary_startup) == 1;
    let endpoint_sets_valid = token_endpoint_match_count(
        manager,
        manager_token,
        manager_startup,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            provider,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            primary,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            secondary,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            provider,
            provider_token,
            provider_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            provider,
            provider_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            primary,
            primary_token,
            primary_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            primary,
            primary_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            secondary,
            secondary_token,
            secondary_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            secondary,
            secondary_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            surface_server,
            surface_server_token,
            surface_ui_channel,
            2,
        ) == 1
        && token_peer_match_count(surface_server, surface_server_token, launcher, 2) == 1
        && token_endpoint_match_count(
            surface_server,
            surface_server_token,
            surface_app_channel,
            2,
        ) == 1
        && token_peer_match_count(surface_server, surface_server_token, app, 2) == 1
        && token_endpoint_match_count(
            launcher,
            launcher_token,
            launcher_ui_channel,
            LAUNCHER_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            launcher,
            launcher_token,
            surface_server,
            LAUNCHER_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(app, app_token, app_ui_channel, APP_WAIT_ITEM_COUNT) == 1
        && token_peer_match_count(
            app,
            app_token,
            surface_server,
            APP_WAIT_ITEM_COUNT,
        ) == 1
        // The two manager-provider pairs have distinct roles: manager waits on
        // its provider session while provider waits on its registry connector.
        && !manager_provider_session.is_peer_of(provider_connector)
        && wait_token_channels_are_distinct(
            manager,
            manager_token,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            provider,
            provider_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            primary,
            primary_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            secondary,
            secondary_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(surface_server, surface_server_token, 2)
        && wait_token_channels_are_distinct(
            launcher,
            launcher_token,
            LAUNCHER_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(app, app_token, APP_WAIT_ITEM_COUNT);
    let requested = ObjectSignals::from_bits(expected_signal_bits)
        .unwrap_or_else(|| panic!("resident service signal mask was invalid"));
    #[cfg(feature = "androidbox-process0")]
    let android_app_wait_blocked = !android_app_channel.signals().intersects(requested);
    #[cfg(not(feature = "androidbox-process0"))]
    let android_app_wait_blocked = true;
    let all_waits_blocked =
        wait_token_channels_are_blocked(
            manager,
            manager_token,
            requested,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) && wait_token_channels_are_blocked(
            provider,
            provider_token,
            requested,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) && wait_token_channels_are_blocked(
            primary,
            primary_token,
            requested,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) && wait_token_channels_are_blocked(
            secondary,
            secondary_token,
            requested,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) && wait_token_channels_are_blocked(surface_server, surface_server_token, requested, 2)
            && !surface_capability
                .signals()
                .intersects(ObjectSignals::SURFACE_ALL)
            && wait_token_channels_are_blocked(
                launcher,
                launcher_token,
                requested,
                LAUNCHER_WAIT_ITEM_COUNT,
            )
            && wait_token_channels_are_blocked(app, app_token, requested, APP_WAIT_ITEM_COUNT)
            && android_app_wait_blocked;
    let object_wait = scheduler::object_wait_snapshot();
    let wait_many = scheduler::wait_many_snapshot();
    let wait_array = scheduler::wait_array_snapshot();

    endpoint_sets_valid
        && startup_peers_owned_by_init
        && all_waits_blocked
        && process_wait.waiting
        && {
            #[cfg(feature = "androidbox-restart0")]
            {
                process_wait.pending_target == manager.id.raw()
                    || process_wait.pending_target == android_app.id.raw()
            }
            #[cfg(not(feature = "androidbox-restart0"))]
            {
                process_wait.pending_target == manager.id.raw()
            }
        }
        && object_wait.pending == M32_FINAL_RESIDENT_WAIT_TOKEN_COUNT
        && wait_many.pending == 2
        && wait_array.pending == 3
        && wait_many
            .pending
            .checked_add(wait_array.pending)
            .and_then(|pending| {
                pending.checked_add(if cfg!(feature = "androidbox-process0") {
                    3
                } else {
                    2
                })
            })
            == Some(object_wait.pending)
}

#[cfg(all(
    feature = "app-lifecycle-runtime",
    not(feature = "input-server-runtime")
))]
fn exact_app_lifecycle_resident_service_waits(
    processes: FinalResidentProcesses<'_>,
    process_wait: scheduler::ProcessWaitSchedulerSnapshot,
    tokens: [Option<ObjectWaitToken>; RESIDENT_WAIT_TOKEN_COUNT],
) -> bool {
    let FinalResidentProcesses {
        init,
        manager,
        provider,
        primary,
        secondary,
        surface_server,
        launcher,
        app,
    } = processes;
    let [
        Some(init_token),
        Some(manager_token),
        Some(provider_token),
        Some(primary_token),
        Some(secondary_token),
        Some(surface_server_token),
        Some(launcher_token),
        Some(app_token),
    ] = tokens
    else {
        return false;
    };
    let expected_signal_bits = ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits();
    if !wait_token_metadata_valid(
        init,
        init_token,
        ObjectWaitCompletionKind::Array,
        7,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        manager,
        manager_token,
        ObjectWaitCompletionKind::Array,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        provider,
        provider_token,
        ObjectWaitCompletionKind::Array,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        primary,
        primary_token,
        ObjectWaitCompletionKind::LegacyMany,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        secondary,
        secondary_token,
        ObjectWaitCompletionKind::LegacyMany,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        expected_signal_bits,
    ) || !app_lifecycle_surface_wait_metadata_valid(
        surface_server,
        surface_server_token,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        launcher,
        launcher_token,
        ObjectWaitCompletionKind::Array,
        2,
        expected_signal_bits,
    ) || !wait_token_metadata_valid(
        app,
        app_token,
        ObjectWaitCompletionKind::Array,
        2,
        expected_signal_bits,
    ) {
        return false;
    }

    let Some(manager_startup) = startup_channel(manager) else {
        return false;
    };
    let Some(provider_startup) = startup_channel(provider) else {
        return false;
    };
    let Some(primary_startup) = startup_channel(primary) else {
        return false;
    };
    let Some(secondary_startup) = startup_channel(secondary) else {
        return false;
    };
    let Some(surface_startup) = startup_channel(surface_server) else {
        return false;
    };
    let Some(launcher_startup) = startup_channel(launcher) else {
        return false;
    };
    let Some(app_startup) = startup_channel(app) else {
        return false;
    };
    let Some(surface_capability) = unique_surface_capability(surface_server) else {
        return false;
    };
    let Some(waited_surface) = unique_token_surface(surface_server, surface_server_token) else {
        return false;
    };
    if !waited_surface.same_surface(surface_capability) {
        return false;
    }
    let Some(manager_provider_session) = unique_token_channel_peered_by(
        manager,
        manager_token,
        provider,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) else {
        return false;
    };
    let Some(provider_connector) = unique_token_channel_peered_by(
        provider,
        provider_token,
        manager,
        SERVICE_CHANNEL_WAIT_ITEM_COUNT,
    ) else {
        return false;
    };

    let startup_peers_owned_by_init = peer_count_in_process(init, manager_startup) == 1
        && peer_count_in_process(init, provider_startup) == 1
        && peer_count_in_process(init, primary_startup) == 1
        && peer_count_in_process(init, secondary_startup) == 1
        && peer_count_in_process(init, launcher_startup) == 1
        && peer_count_in_process(init, app_startup) == 1
        && peer_count_in_process(launcher, surface_startup) == 1;
    let init_endpoints_valid = token_channel_count(init, init_token) == 7
        && token_peer_match_count(init, init_token, manager, 7) == 1
        && token_peer_match_count(init, init_token, provider, 7) == 1
        && token_peer_match_count(init, init_token, primary, 7) == 1
        && token_peer_match_count(init, init_token, secondary, 7) == 1
        && token_peer_match_count(init, init_token, surface_server, 7) == 1
        && token_peer_match_count(init, init_token, launcher, 7) == 1
        && token_peer_match_count(init, init_token, app, 7) == 1;
    let core_endpoints_valid = token_endpoint_match_count(
        manager,
        manager_token,
        manager_startup,
        FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
    ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            provider,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            primary,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            manager,
            manager_token,
            secondary,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            provider,
            provider_token,
            provider_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            provider,
            provider_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            primary,
            primary_token,
            primary_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            primary,
            primary_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_endpoint_match_count(
            secondary,
            secondary_token,
            secondary_startup,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1
        && token_peer_match_count(
            secondary,
            secondary_token,
            manager,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        ) == 1;
    let window_endpoints_valid = token_channel_count(surface_server, surface_server_token) == 3
        && token_endpoint_match_count(surface_server, surface_server_token, surface_startup, 3)
            == 1
        && token_peer_match_count(surface_server, surface_server_token, init, 3) == 1
        && token_peer_match_count(surface_server, surface_server_token, launcher, 3) == 1
        && token_peer_match_count(surface_server, surface_server_token, app, 3) == 1
        && token_endpoint_match_count(launcher, launcher_token, launcher_startup, 2) == 1
        && token_peer_match_count(launcher, launcher_token, init, 2) == 1
        && token_peer_match_count(launcher, launcher_token, surface_server, 2) == 1
        && token_endpoint_match_count(app, app_token, app_startup, 2) == 1
        && token_peer_match_count(app, app_token, init, 2) == 1
        && token_peer_match_count(app, app_token, surface_server, 2) == 1;
    let distinct_endpoints = wait_token_channels_are_distinct(init, init_token, 7)
        && wait_token_channels_are_distinct(
            manager,
            manager_token,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            provider,
            provider_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            primary,
            primary_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(
            secondary,
            secondary_token,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_distinct(surface_server, surface_server_token, 3)
        && wait_token_channels_are_distinct(launcher, launcher_token, 2)
        && wait_token_channels_are_distinct(app, app_token, 2);
    let requested = ObjectSignals::from_bits(expected_signal_bits)
        .unwrap_or_else(|| panic!("resident service signal mask was invalid"));
    #[cfg(feature = "text-input-runtime")]
    let surface_requested =
        ObjectSignals::from_bits(expected_signal_bits | ObjectSignals::KEY_READY.bits())
            .unwrap_or_else(|| panic!("text-input SurfaceServer signal mask was invalid"));
    #[cfg(not(feature = "text-input-runtime"))]
    let surface_requested = requested;
    let all_waits_blocked = wait_token_channels_are_blocked(init, init_token, requested, 7)
        && wait_token_channels_are_blocked(
            manager,
            manager_token,
            requested,
            FINAL_MANAGER_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_blocked(
            provider,
            provider_token,
            requested,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_blocked(
            primary,
            primary_token,
            requested,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_blocked(
            secondary,
            secondary_token,
            requested,
            SERVICE_CHANNEL_WAIT_ITEM_COUNT,
        )
        && wait_token_channels_are_blocked(surface_server, surface_server_token, requested, 3)
        && !surface_capability.signals().intersects(surface_requested)
        && wait_token_channels_are_blocked(launcher, launcher_token, requested, 2)
        && wait_token_channels_are_blocked(app, app_token, requested, 2);
    let object_wait = scheduler::object_wait_snapshot();
    let wait_many = scheduler::wait_many_snapshot();
    let wait_array = scheduler::wait_array_snapshot();

    init_endpoints_valid
        && core_endpoints_valid
        && window_endpoints_valid
        && startup_peers_owned_by_init
        && !manager_provider_session.is_peer_of(provider_connector)
        && distinct_endpoints
        && all_waits_blocked
        && !process_wait.waiting
        && process_wait.pending_target == 0
        && object_wait.pending == RESIDENT_WAIT_TOKEN_COUNT
        && wait_many.pending == 2
        && wait_array.pending == 6
        && wait_many.pending.checked_add(wait_array.pending) == Some(object_wait.pending)
}

fn wait_token_metadata_valid(
    process: &Process,
    token: ObjectWaitToken,
    completion_kind: ObjectWaitCompletionKind,
    count: usize,
    expected_signal_bits: u32,
) -> bool {
    token.process_id() == process.id.raw()
        && token.completion_kind() == completion_kind
        && token.epoch() != 0
        && token.count() == count
        && token.deadline().is_none()
        && token
            .items()
            .iter()
            .all(|item| item.requested().bits() == expected_signal_bits)
}

#[cfg(feature = "app-lifecycle-runtime")]
fn app_lifecycle_surface_wait_metadata_valid(
    process: &Process,
    token: ObjectWaitToken,
    channel_signal_bits: u32,
) -> bool {
    if token.process_id() != process.id.raw()
        || token.completion_kind() != ObjectWaitCompletionKind::Array
        || token.epoch() == 0
        || token.count() != 4
        || token.deadline().is_some()
    {
        return false;
    }

    #[cfg(feature = "text-input-runtime")]
    let surface_signal_bits = channel_signal_bits | ObjectSignals::KEY_READY.bits();
    #[cfg(not(feature = "text-input-runtime"))]
    let surface_signal_bits = channel_signal_bits;

    token.items()[0].requested().bits() == surface_signal_bits
        && token.items()[1..]
            .iter()
            .all(|item| item.requested().bits() == channel_signal_bits)
}

fn surface_server_wait_metadata_valid(process: &Process, token: ObjectWaitToken) -> bool {
    // The resident M32 SurfaceServer waits only for input or degradation.
    // ABI v20 extends the capability mask with FRAME_READY, but the legacy
    // runtime must not silently begin requesting that independent signal.
    let requested = ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits();
    wait_token_metadata_valid(
        process,
        token,
        ObjectWaitCompletionKind::Array,
        SURFACE_SERVER_WAIT_ITEM_COUNT,
        requested,
    )
}

fn token_endpoint_match_count(
    process: &Process,
    token: ObjectWaitToken,
    expected: &ChannelEndpoint,
    channel_count: usize,
) -> usize {
    if token_channel_count(process, token) != channel_count {
        return 0;
    }
    (0..token.count())
        .filter_map(|index| token_channel(process, token, index))
        .filter(|channel| channel.same_endpoint(expected))
        .count()
}

fn token_peer_match_count(
    process: &Process,
    token: ObjectWaitToken,
    peer: &Process,
    channel_count: usize,
) -> usize {
    if token_channel_count(process, token) != channel_count {
        return 0;
    }
    (0..token.count())
        .filter_map(|index| token_channel(process, token, index))
        .filter(|channel| peer_count_in_process(peer, channel) == 1)
        .count()
}

fn unique_token_channel_peered_by<'a>(
    process: &'a Process,
    token: ObjectWaitToken,
    peer: &Process,
    channel_count: usize,
) -> Option<&'a ChannelEndpoint> {
    if token_channel_count(process, token) != channel_count {
        return None;
    }
    let mut found = None;
    for index in 0..token.count() {
        let Some(channel) = token_channel(process, token, index) else {
            continue;
        };
        if peer_count_in_process(peer, channel) != 1 {
            continue;
        }
        if found.replace(channel).is_some() {
            return None;
        }
    }
    found
}

fn wait_token_channels_are_distinct(
    process: &Process,
    token: ObjectWaitToken,
    channel_count: usize,
) -> bool {
    if token_channel_count(process, token) != channel_count {
        return false;
    }
    for left_index in 0..token.count() {
        let Some(left) = token_channel(process, token, left_index) else {
            continue;
        };
        for right_index in (left_index + 1)..token.count() {
            let Some(right) = token_channel(process, token, right_index) else {
                continue;
            };
            if left.same_endpoint(right) {
                return false;
            }
        }
    }
    true
}

fn wait_token_channels_are_blocked(
    process: &Process,
    token: ObjectWaitToken,
    requested: ObjectSignals,
    channel_count: usize,
) -> bool {
    token_channel_count(process, token) == channel_count
        && (0..token.count())
            .filter_map(|index| token_channel(process, token, index))
            .all(|channel| !channel.signals().intersects(requested))
}

fn token_channel_count(process: &Process, token: ObjectWaitToken) -> usize {
    (0..token.count())
        .filter(|index| token_channel(process, token, *index).is_some())
        .count()
}

fn token_channel(
    process: &Process,
    token: ObjectWaitToken,
    item_index: usize,
) -> Option<&ChannelEndpoint> {
    let item = token.items().get(item_index)?;
    let raw_handle = u32::try_from(item.raw_handle()).ok()?;
    let handle = HandleValue::from_raw(raw_handle);
    if !handle.is_valid() || process.handles.rights(handle).ok()? != Rights::CHANNEL_DEFAULT {
        return None;
    }
    process
        .handles
        .get(handle, Rights::CHANNEL_DEFAULT)
        .ok()?
        .as_channel()
}

fn token_surface(
    process: &Process,
    token: ObjectWaitToken,
    item_index: usize,
) -> Option<&SurfaceCapability> {
    let item = token.items().get(item_index)?;
    let raw_handle = u32::try_from(item.raw_handle()).ok()?;
    let handle = HandleValue::from_raw(raw_handle);
    if !handle.is_valid() || process.handles.rights(handle).ok()? != Rights::SURFACE_DEFAULT {
        return None;
    }
    process
        .handles
        .get(handle, Rights::SURFACE_DEFAULT)
        .ok()?
        .as_surface()
}

#[cfg(feature = "input-server-runtime")]
fn token_input(
    process: &Process,
    token: ObjectWaitToken,
    item_index: usize,
) -> Option<&InputCapability> {
    let item = token.items().get(item_index)?;
    let raw_handle = u32::try_from(item.raw_handle()).ok()?;
    let handle = HandleValue::from_raw(raw_handle);
    if !handle.is_valid() || process.handles.rights(handle).ok()? != Rights::INPUT_DEFAULT {
        return None;
    }
    process
        .handles
        .get(handle, Rights::INPUT_DEFAULT)
        .ok()?
        .as_input()
}

#[cfg(feature = "input-server-runtime")]
fn token_input_count(process: &Process, token: ObjectWaitToken) -> usize {
    (0..token.count())
        .filter(|index| token_input(process, token, *index).is_some())
        .count()
}

#[cfg(feature = "input-server-runtime")]
fn unique_token_input(process: &Process, token: ObjectWaitToken) -> Option<&InputCapability> {
    let mut found = None;
    for index in 0..token.count() {
        let Some(input) = token_input(process, token, index) else {
            continue;
        };
        if found.replace(input).is_some() {
            return None;
        }
    }
    found
}

#[cfg(feature = "input-server-runtime")]
fn unique_input_capability(process: &Process) -> Option<&InputCapability> {
    let mut found = None;
    for (object, rights) in process.handles.live_entries() {
        let Some(input) = object.as_input() else {
            continue;
        };
        if rights != Rights::INPUT_DEFAULT || found.replace(input).is_some() {
            return None;
        }
    }
    found
}

fn unique_token_surface(process: &Process, token: ObjectWaitToken) -> Option<&SurfaceCapability> {
    let mut found = None;
    for index in 0..token.count() {
        let Some(surface) = token_surface(process, token, index) else {
            continue;
        };
        if found.replace(surface).is_some() {
            return None;
        }
    }
    found
}

fn unique_surface_capability(process: &Process) -> Option<&SurfaceCapability> {
    let mut found = None;
    for (object, rights) in process.handles.live_entries() {
        let Some(surface) = object.as_surface() else {
            continue;
        };
        if rights != Rights::SURFACE_DEFAULT || found.replace(surface).is_some() {
            return None;
        }
    }
    found
}

fn startup_channel(process: &Process) -> Option<&ChannelEndpoint> {
    let handle = process.startup_handle?;
    if process.handles.rights(handle).ok()? != Rights::CHANNEL_DEFAULT {
        return None;
    }
    process
        .handles
        .get(handle, Rights::CHANNEL_DEFAULT)
        .ok()?
        .as_channel()
}

#[cfg(feature = "androidbox-process0")]
fn android_app_startup_channel(process: &Process) -> Option<&ChannelEndpoint> {
    if process.image_id != UserImageId::AndroidApp {
        return None;
    }
    let handle = process.startup_handle?;
    if process.handles.rights(handle).ok()? != Rights::ANDROID_APP_CHANNEL {
        return None;
    }
    process
        .handles
        .get(handle, Rights::ANDROID_APP_CHANNEL)
        .ok()?
        .as_channel()
}

#[cfg(feature = "androidbox-process0")]
fn android_app_token_channel(
    process: &Process,
    token: ObjectWaitToken,
    item_index: usize,
) -> Option<&ChannelEndpoint> {
    let item = token.items().get(item_index)?;
    let raw_handle = u32::try_from(item.raw_handle()).ok()?;
    let handle = HandleValue::from_raw(raw_handle);
    if !handle.is_valid() || process.handles.rights(handle).ok()? != Rights::ANDROID_APP_CHANNEL {
        return None;
    }
    process
        .handles
        .get(handle, Rights::ANDROID_APP_CHANNEL)
        .ok()?
        .as_channel()
}

#[cfg(feature = "androidbox-process0")]
fn unique_channel_peer_in_process<'a>(
    process: &'a Process,
    endpoint: &ChannelEndpoint,
) -> Option<&'a ChannelEndpoint> {
    let mut found = None;
    for candidate in process
        .handles
        .live_entries()
        .filter_map(|(object, _)| object.as_channel())
        .filter(|candidate| endpoint.is_peer_of(candidate))
    {
        if found.replace(candidate).is_some() {
            return None;
        }
    }
    found
}

fn peer_count_in_process(process: &Process, endpoint: &ChannelEndpoint) -> usize {
    process
        .handles
        .live_entries()
        .filter_map(|(object, _)| object.as_channel())
        .filter(|candidate| endpoint.is_peer_of(candidate))
        .count()
}

#[cfg(feature = "resident-platform-shutdown-runtime")]
const M66_RESIDENT_OWNER_COUNT: usize = 10;
#[cfg(feature = "resident-platform-shutdown-runtime")]
const M66_RESIDENT_ENDPOINT_COUNT: usize = 38;
#[cfg(feature = "resident-platform-shutdown-runtime")]
const M66_RESIDENT_CONTROL_PAIR_COUNT: usize = 9;
#[cfg(feature = "resident-platform-shutdown-runtime")]
const M66_RESIDENT_TOTAL_HANDLE_COUNT: usize = 39;

#[cfg(feature = "resident-platform-shutdown-runtime")]
const fn resident_shutdown_pair_kind(left: usize, right: usize) -> u8 {
    if left == 0 && right > 0 {
        return 1;
    }
    if left < 2 || right < 2 {
        return 0;
    }
    let Some(left_node) = ShutdownServiceNode::from_raw((left - 2) as u64) else {
        return 0;
    };
    let Some(right_node) = ShutdownServiceNode::from_raw((right - 2) as u64) else {
        return 0;
    };
    if left_node.dependency_mask() & right_node.bit() != 0
        || right_node.dependency_mask() & left_node.bit() != 0
    {
        2
    } else {
        0
    }
}

#[cfg(feature = "resident-platform-shutdown-runtime")]
const fn empty_resident_shutdown_topology() -> ResidentShutdownTopologySnapshot {
    ResidentShutdownTopologySnapshot {
        valid: false,
        processes: 0,
        service_nodes: 0,
        control_pairs: 0,
        dependency_pairs: 0,
        channel_pairs: 0,
        endpoints: 0,
        total_handles: 0,
        queues_empty: false,
        rights_valid: false,
        endpoints_unique: false,
        startup_peers_valid: false,
        storage_volume_valid: false,
        ledger_pids_valid: false,
    }
}

/// Authenticates one M66 service node against its exact spawn slot, PID
/// generation, and executable image. The two Client instances therefore
/// cannot exchange Primary/Secondary identities.
#[cfg(feature = "resident-platform-shutdown-runtime")]
pub fn resident_shutdown_node_identity_valid(
    raw_process_id: u64,
    node: ShutdownServiceNode,
) -> bool {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    #[cfg(feature = "unified-product-runtime")]
    let spawn_index = match node {
        ShutdownServiceNode::ServiceManager => 3,
        ShutdownServiceNode::Provider => 1,
        ShutdownServiceNode::PrimaryClient => 2,
        ShutdownServiceNode::SecondaryClient => 4,
        ShutdownServiceNode::SurfaceServer => 6,
        ShutdownServiceNode::InputServer => 5,
        ShutdownServiceNode::Launcher => 7,
        ShutdownServiceNode::App => 8,
    };
    #[cfg(not(feature = "unified-product-runtime"))]
    let spawn_index = node.raw() as usize + 1;
    let valid = CHILD_SPAWN_PIDS
        .get(spawn_index)
        .zip(CHILD_SPAWN_IMAGES.get(spawn_index))
        .is_some_and(|(pid, image)| {
            pid.load(Ordering::Acquire) == raw_process_id
                && image.load(Ordering::Acquire) == node.image_id().raw()
                && live_spawned_process(slots_ref(), raw_process_id, node.image_id()).is_some()
        });
    crate::arch::aarch64::restore_daif(saved_daif);
    valid
}

/// Proves M66's complete pre-Prepare resident graph from live kernel objects.
///
/// Owners are init, StorageServer, then the eight ABI service nodes. Exactly
/// nine init control pairs and ten dependency pairs must account for all 38
/// unique Channel endpoints; StorageServer alone additionally owns its volume
/// capability. No marker or userspace-declared count can satisfy this proof.
#[cfg(feature = "resident-platform-shutdown-runtime")]
pub fn resident_shutdown_topology_snapshot() -> ResidentShutdownTopologySnapshot {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let child_pids: [u64; 9] =
        core::array::from_fn(|index| CHILD_SPAWN_PIDS[index].load(Ordering::Acquire));
    let child_images: [u64; 9] =
        core::array::from_fn(|index| CHILD_SPAWN_IMAGES[index].load(Ordering::Acquire));
    let expected_images = [
        UserImageId::StorageServer.raw(),
        UserImageId::ServiceManager.raw(),
        UserImageId::Provider.raw(),
        UserImageId::Client.raw(),
        UserImageId::Client.raw(),
        UserImageId::SurfaceServer.raw(),
        UserImageId::InputServer.raw(),
        UserImageId::Launcher.raw(),
        UserImageId::App.raw(),
    ];
    if child_pids.contains(&0)
        || child_images != expected_images
        || CHILD_SPAWN_PIDS[9].load(Ordering::Acquire) != 0
        || CHILD_SPAWN_IMAGES[9].load(Ordering::Acquire) != 0
    {
        crate::arch::aarch64::restore_daif(saved_daif);
        return empty_resident_shutdown_topology();
    }
    for left in 0..child_pids.len() {
        if child_pids[(left + 1)..].contains(&child_pids[left]) {
            crate::arch::aarch64::restore_daif(saved_daif);
            return empty_resident_shutdown_topology();
        }
    }

    let slots = slots_ref();
    let Some(init) = unique_live_process_by_image(slots, UserImageId::Init) else {
        crate::arch::aarch64::restore_daif(saved_daif);
        return empty_resident_shutdown_topology();
    };
    let Some(storage) = live_spawned_process(slots, child_pids[0], UserImageId::StorageServer)
    else {
        crate::arch::aarch64::restore_daif(saved_daif);
        return empty_resident_shutdown_topology();
    };
    let Some(manager) = live_spawned_process(slots, child_pids[1], UserImageId::ServiceManager)
    else {
        crate::arch::aarch64::restore_daif(saved_daif);
        return empty_resident_shutdown_topology();
    };
    let Some(provider) = live_spawned_process(slots, child_pids[2], UserImageId::Provider) else {
        crate::arch::aarch64::restore_daif(saved_daif);
        return empty_resident_shutdown_topology();
    };
    let Some(primary) = live_spawned_process(slots, child_pids[3], UserImageId::Client) else {
        crate::arch::aarch64::restore_daif(saved_daif);
        return empty_resident_shutdown_topology();
    };
    let Some(secondary) = live_spawned_process(slots, child_pids[4], UserImageId::Client) else {
        crate::arch::aarch64::restore_daif(saved_daif);
        return empty_resident_shutdown_topology();
    };
    let Some(surface) = live_spawned_process(slots, child_pids[5], UserImageId::SurfaceServer)
    else {
        crate::arch::aarch64::restore_daif(saved_daif);
        return empty_resident_shutdown_topology();
    };
    let Some(input) = live_spawned_process(slots, child_pids[6], UserImageId::InputServer) else {
        crate::arch::aarch64::restore_daif(saved_daif);
        return empty_resident_shutdown_topology();
    };
    let Some(launcher) = live_spawned_process(slots, child_pids[7], UserImageId::Launcher) else {
        crate::arch::aarch64::restore_daif(saved_daif);
        return empty_resident_shutdown_topology();
    };
    let Some(app) = live_spawned_process(slots, child_pids[8], UserImageId::App) else {
        crate::arch::aarch64::restore_daif(saved_daif);
        return empty_resident_shutdown_topology();
    };
    let processes = [
        init, storage, manager, provider, primary, secondary, surface, input, launcher, app,
    ];
    let expected_handles = [9_usize, 2, 4, 4, 3, 3, 4, 4, 3, 3];
    let process_count = slots.iter().filter(|slot| slot.process.is_some()).count();
    let process_accounting_valid = process_count == M66_RESIDENT_OWNER_COUNT
        && CREATED.load(Ordering::Acquire) == 10
        && EXITED.load(Ordering::Acquire) == 0
        && REAPED.load(Ordering::Acquire) == 0
        && LIVE.load(Ordering::Acquire) == 10
        && PEAK_LIVE.load(Ordering::Acquire) == 10
        && slots[FIRST_DYNAMIC_PROCESS_SLOT..]
            .iter()
            .all(|slot| !slot.completion.is_pending());

    let mut endpoints: [Option<ResidentShutdownEndpoint<'_>>; M66_RESIDENT_ENDPOINT_COUNT] =
        [None; M66_RESIDENT_ENDPOINT_COUNT];
    let mut endpoint_count = 0_usize;
    let mut total_handles = 0_usize;
    let mut queues_empty = true;
    let mut rights_valid = true;
    let mut storage_volume_count = 0_usize;
    let mut storage_volume_valid = true;
    for (owner, process) in processes.iter().enumerate() {
        rights_valid &= process.handles.len() == expected_handles[owner];
        total_handles = total_handles
            .checked_add(process.handles.len())
            .unwrap_or_else(|| panic!("M66 resident handle count overflowed"));
        for (object, rights) in process.handles.live_entries() {
            if let Some(channel) = object.as_channel() {
                rights_valid &= rights == Rights::CHANNEL_DEFAULT;
                queues_empty &= channel.queued_messages() == 0;
                if let Some(slot) = endpoints.get_mut(endpoint_count) {
                    *slot = Some(ResidentShutdownEndpoint { owner, channel });
                } else {
                    rights_valid = false;
                }
                endpoint_count = endpoint_count
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("M66 resident endpoint count overflowed"));
                continue;
            }
            if owner == 1
                && let Some(volume) = object.as_storage_volume()
            {
                storage_volume_count += 1;
                storage_volume_valid &= rights == Rights::STORAGE_VOLUME_DEFAULT
                    && volume.owner_pid() == storage.id.raw()
                    && volume.epoch() == 1;
                continue;
            }
            rights_valid = false;
        }
    }

    let startup_peers_valid = init.is_init
        && init.startup_handle.is_none()
        && processes[1..].iter().all(|process| {
            !process.is_init
                && startup_channel(process)
                    .is_some_and(|startup| peer_count_in_process(init, startup) == 1)
        });

    let mut pair_counts = [0_u8; M66_RESIDENT_OWNER_COUNT * M66_RESIDENT_OWNER_COUNT];
    let mut endpoints_unique = endpoint_count == M66_RESIDENT_ENDPOINT_COUNT;
    let mut channel_pairs = 0_usize;
    if endpoint_count == M66_RESIDENT_ENDPOINT_COUNT {
        for left_index in 0..endpoint_count {
            let left = endpoints[left_index]
                .unwrap_or_else(|| panic!("M66 resident endpoint prefix contained a hole"));
            for right in endpoints
                .iter()
                .take(endpoint_count)
                .skip(left_index + 1)
                .map(|endpoint| {
                    endpoint
                        .unwrap_or_else(|| panic!("M66 resident endpoint prefix contained a hole"))
                })
            {
                endpoints_unique &= !left.channel.same_endpoint(right.channel);
                if !left.channel.is_peer_of(right.channel) {
                    continue;
                }
                channel_pairs += 1;
                let (owner_left, owner_right) = if left.owner < right.owner {
                    (left.owner, right.owner)
                } else {
                    (right.owner, left.owner)
                };
                let index = owner_left * M66_RESIDENT_OWNER_COUNT + owner_right;
                pair_counts[index] = pair_counts[index].saturating_add(1);
            }
        }
    }

    let mut control_pairs = 0_usize;
    let mut dependency_pairs = 0_usize;
    let mut pairs_valid = true;
    for left in 0..M66_RESIDENT_OWNER_COUNT {
        for right in (left + 1)..M66_RESIDENT_OWNER_COUNT {
            let kind = resident_shutdown_pair_kind(left, right);
            let observed = pair_counts[left * M66_RESIDENT_OWNER_COUNT + right];
            pairs_valid &= observed == u8::from(kind != 0);
            if observed == 1 {
                if kind == 1 {
                    control_pairs += 1;
                } else if kind == 2 {
                    dependency_pairs += 1;
                }
            }
        }
    }

    let service = bndroid_kernel::service_shutdown::snapshot();
    let ledger_pids_valid = service.registered_mask == SHUTDOWN_SERVICE_ALL_MASK
        && service.quiesced_mask == 0
        && service
            .pids
            .iter()
            .enumerate()
            .all(|(index, pid)| *pid == child_pids[index + 1]);
    let valid = process_accounting_valid
        && endpoint_count == M66_RESIDENT_ENDPOINT_COUNT
        && total_handles == M66_RESIDENT_TOTAL_HANDLE_COUNT
        && control_pairs == M66_RESIDENT_CONTROL_PAIR_COUNT
        && dependency_pairs == SHUTDOWN_SERVICE_EDGE_COUNT
        && channel_pairs
            == M66_RESIDENT_CONTROL_PAIR_COUNT
                .checked_add(SHUTDOWN_SERVICE_EDGE_COUNT)
                .unwrap_or(0)
        && queues_empty
        && rights_valid
        && endpoints_unique
        && pairs_valid
        && startup_peers_valid
        && storage_volume_count == 1
        && storage_volume_valid
        && ledger_pids_valid
        && scheduler::live_dynamic_user_context_count() == 9
        && scheduler::live_dynamic_kernel_stack_count() == 9;
    let snapshot = ResidentShutdownTopologySnapshot {
        valid,
        processes: process_count,
        service_nodes: SHUTDOWN_SERVICE_NODE_COUNT,
        control_pairs,
        dependency_pairs,
        channel_pairs,
        endpoints: endpoint_count,
        total_handles,
        queues_empty,
        rights_valid,
        endpoints_unique,
        startup_peers_valid,
        storage_volume_valid: storage_volume_count == 1 && storage_volume_valid,
        ledger_pids_valid,
    };
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

/// Validates only the pre-secondary InitReady capability baseline.
///
/// Post-M31 callers must use [`snapshot`] and require its final resident
/// topology fields; this boundary deliberately cannot validate any partial
/// secondary/UI construction after InitReady.
#[cfg(feature = "storage-server-runtime")]
pub fn storage_server_ready_topology_valid() -> bool {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("StorageServer ready topology checked with IRQ enabled");
    }

    let first_server_pid = CHILD_SPAWN_PIDS[0].load(Ordering::Acquire);
    let launcher_pid = CHILD_SPAWN_PIDS[1].load(Ordering::Acquire);
    let app_pid = CHILD_SPAWN_PIDS[2].load(Ordering::Acquire);
    let replacement_server_pid = CHILD_SPAWN_PIDS[3].load(Ordering::Acquire);
    let rebound_launcher_pid = CHILD_SPAWN_PIDS[4].load(Ordering::Acquire);
    let rebound_app_pid = CHILD_SPAWN_PIDS[5].load(Ordering::Acquire);
    let expected_images = [
        UserImageId::StorageServer.raw(),
        UserImageId::Launcher.raw(),
        UserImageId::App.raw(),
        UserImageId::StorageServer.raw(),
        UserImageId::Launcher.raw(),
        UserImageId::App.raw(),
    ];
    if [
        first_server_pid,
        launcher_pid,
        app_pid,
        replacement_server_pid,
        rebound_launcher_pid,
        rebound_app_pid,
    ]
    .contains(&0)
        || CHILD_SPAWN_IMAGES[..6]
            .iter()
            .zip(expected_images)
            .any(|(slot, expected)| slot.load(Ordering::Acquire) != expected)
        || CHILD_SPAWN_PIDS[6..]
            .iter()
            .any(|slot| slot.load(Ordering::Acquire) != 0)
        || CHILD_SPAWN_IMAGES[6..]
            .iter()
            .any(|slot| slot.load(Ordering::Acquire) != 0)
        || CREATED.load(Ordering::Acquire) != 7
        || LIVE.load(Ordering::Acquire) != 2
        || PEAK_LIVE.load(Ordering::Acquire) != 3
        || EXITED.load(Ordering::Acquire) != 5
        || REAPED.load(Ordering::Acquire) != 5
    {
        return false;
    }

    let first_server_id = ProcessId(first_server_pid);
    let replacement_server_id = ProcessId(replacement_server_pid);
    if first_server_id.slot().is_none()
        || first_server_id.slot() != replacement_server_id.slot()
        || replacement_server_id.generation()
            != first_server_id.generation().checked_add(1).unwrap_or(0)
    {
        return false;
    }

    let slots = slots_ref();
    if slots
        .iter()
        .filter_map(|slot| slot.process.as_deref())
        .count()
        != 2
        || slots[FIRST_DYNAMIC_PROCESS_SLOT..]
            .iter()
            .any(|slot| slot.completion.is_pending())
    {
        return false;
    }
    let Some(init) = unique_live_process_by_image(slots, UserImageId::Init) else {
        return false;
    };
    let Some(server) =
        live_spawned_process(slots, replacement_server_pid, UserImageId::StorageServer)
    else {
        return false;
    };
    if live_spawned_process(slots, first_server_pid, UserImageId::StorageServer).is_some()
        || live_spawned_process(slots, launcher_pid, UserImageId::Launcher).is_some()
        || live_spawned_process(slots, app_pid, UserImageId::App).is_some()
        || live_spawned_process(slots, rebound_launcher_pid, UserImageId::Launcher).is_some()
        || live_spawned_process(slots, rebound_app_pid, UserImageId::App).is_some()
        || unique_live_process_by_image(slots, UserImageId::Launcher).is_some()
        || unique_live_process_by_image(slots, UserImageId::App).is_some()
    {
        return false;
    }
    if !init.is_init
        || init.startup_handle.is_some()
        || server.is_init
        || init.handles.len() != 1
        || server.handles.len() != 2
    {
        return false;
    }

    let mut init_entries = init.handles.live_entries();
    let Some((init_object, init_rights)) = init_entries.next() else {
        return false;
    };
    if init_entries.next().is_some() || init_rights != Rights::CHANNEL_DEFAULT {
        return false;
    }
    let Some(init_startup) = init_object.as_channel() else {
        return false;
    };
    let Some(server_startup) = startup_channel(server) else {
        return false;
    };

    let mut server_channel_count = 0_usize;
    let mut volume = None;
    for (object, rights) in server.handles.live_entries() {
        if let Some(channel) = object.as_channel() {
            if rights != Rights::CHANNEL_DEFAULT || !channel.same_endpoint(server_startup) {
                return false;
            }
            server_channel_count += 1;
            continue;
        }
        if let Some(capability) = object.as_storage_volume() {
            if rights != Rights::STORAGE_VOLUME_DEFAULT || volume.replace(*capability).is_some() {
                return false;
            }
            continue;
        }
        return false;
    }
    let Some(volume) = volume else {
        return false;
    };

    server_channel_count == 1
        && volume.owner_pid() == server.id.raw()
        && volume.epoch() == 2
        && init_startup.is_peer_of(server_startup)
        && peer_count_in_process(init, server_startup) == 1
        && peer_count_in_process(server, init_startup) == 1
        && init_startup.queued_messages() == 0
        && server_startup.queued_messages() == 0
}

pub fn resident_control_topology_valid() -> bool {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("resident control topology checked with IRQ enabled");
    }
    let child_spawn_pids = [
        CHILD_SPAWN_PIDS[0].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[1].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[2].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[3].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[4].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[5].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[6].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[7].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[8].load(Ordering::Acquire),
        CHILD_SPAWN_PIDS[9].load(Ordering::Acquire),
    ];
    // InitReady is issued before spawn slots four through nine exist. Validate
    // the exact three-child graph directly so readiness never depends on the
    // later secondary-client or UI lifecycle.
    let topology = resident_baseline_service_topology(
        slots_ref(),
        scheduler::process_wait_snapshot(),
        child_spawn_pids,
    );
    topology.service_topology_valid && topology.service_queues_empty
}

/// Binds an init-owned control handle to one exact generation-qualified child
/// rather than requiring the child's executable image to be globally unique.
/// This is the identity primitive used by the M20 multi-session transcript:
/// two live Client images are legitimate, but neither may authenticate through
/// the other instance's startup channel.
pub fn init_handle_is_startup_peer_for_process(
    handle: HandleValue,
    raw_process_id: u64,
    image_id: UserImageId,
) -> bool {
    if !crate::arch::aarch64::irq_is_masked() || !image_id.is_spawnable() {
        panic!("resident process startup-peer identity checked outside its valid domain");
    }
    let Some(init) = slots_ref()[INIT_PROCESS_SLOT].process.as_deref() else {
        return false;
    };
    let Some(child) = lookup(ProcessId(raw_process_id)) else {
        return false;
    };
    if child.image_id != image_id || child.is_init {
        return false;
    }
    let Ok(object) = init.handles.get(handle, Rights::CHANNEL_DEFAULT) else {
        return false;
    };
    let Some(init_channel) = object.as_channel() else {
        return false;
    };
    startup_channel(child).is_some_and(|startup| init_channel.is_peer_of(startup))
}

fn lookup(id: ProcessId) -> Option<&'static Process> {
    let slot = id.slot()?;
    let process = slots_ref()[slot].process.as_deref()?;
    (process.id == id && id.generation() == slots_ref()[slot].generation).then_some(process)
}

fn lookup_mut(id: ProcessId) -> Option<&'static mut Process> {
    let slot = id.slot()?;
    let slots = slots_mut();
    if id.generation() != slots[slot].generation {
        return None;
    }
    let process = slots[slot].process.as_deref_mut()?;
    (process.id == id).then_some(process)
}

fn first_available_dynamic_slot(slots: &[ProcessSlot; PROCESS_CAPACITY]) -> Option<usize> {
    (FIRST_DYNAMIC_PROCESS_SLOT..PROCESS_CAPACITY)
        .find(|&slot| slots[slot].process.is_none() && !slots[slot].completion.is_pending())
}

fn address_space_is_isolated_from_live(
    candidate: &UserAddressSpace,
    slots: &[ProcessSlot; PROCESS_CAPACITY],
) -> bool {
    if candidate.asid() == mmu::KERNEL_ASID {
        return false;
    }
    slots
        .iter()
        .filter_map(|slot| slot.process.as_deref())
        .filter_map(|process| process.address_space.as_ref())
        .all(|live| {
            candidate.asid() != live.asid()
                && candidate.root_address() != live.root_address()
                && candidate.user_frames_are_disjoint(live)
        })
}

#[derive(Clone, Copy)]
struct LiveDynamicIsolation {
    live: usize,
    asids: bool,
    roots: bool,
    frames: bool,
}

fn live_dynamic_isolation(slots: &[ProcessSlot; PROCESS_CAPACITY]) -> LiveDynamicIsolation {
    let mut isolation = LiveDynamicIsolation {
        live: 0,
        asids: true,
        roots: true,
        frames: true,
    };
    for (left_offset, left_slot) in slots[FIRST_DYNAMIC_PROCESS_SLOT..].iter().enumerate() {
        let Some(left) = left_slot
            .process
            .as_deref()
            .and_then(|process| process.address_space.as_ref())
        else {
            continue;
        };
        isolation.live += 1;
        for right_slot in &slots[FIRST_DYNAMIC_PROCESS_SLOT + left_offset + 1..] {
            let Some(right) = right_slot
                .process
                .as_deref()
                .and_then(|process| process.address_space.as_ref())
            else {
                continue;
            };
            isolation.asids &= left.asid() != right.asid();
            isolation.roots &= left.root_address() != right.root_address();
            isolation.frames &= left.user_frames_are_disjoint(right);
        }
    }
    isolation
}

fn retire_dynamic_generation(slot: &mut ProcessSlot, generation: u32) {
    if slot.generation != generation || slot.process.is_some() || slot.completion.is_pending() {
        panic!("process generation retirement invariants failed");
    }
    slot.generation = generation
        .checked_add(1)
        .filter(|generation| *generation != 0)
        .unwrap_or_else(|| panic!("process slot generation exhausted"));
}

fn destroy_unpublished(address_space: UserAddressSpace) {
    let translation = address_space.translation_context();
    let proof = unsafe { StoppedAddressSpaceProof::new(translation) };
    address_space
        .destroy(proof)
        .unwrap_or_else(|failure| destroy_failure_is_fatal(failure));
}

fn destroy_unpublished_process(mut process: Box<Process>) {
    let address_space = process
        .address_space
        .take()
        .unwrap_or_else(|| panic!("unpublished Process lost its address space"));
    drop(process);
    destroy_unpublished(address_space);
}

fn destroy_failure_is_fatal(failure: AddressSpaceDestroyFailure) -> ! {
    let _ = (
        failure.error,
        failure.address_space.root_address(),
        failure.stopped.translation_context(),
    );
    panic!("stopped address-space destruction failed");
}

fn try_box<T>(value: T) -> Result<Box<T>, T> {
    let layout = Layout::new::<T>();
    let Some(pointer) = NonNull::new(unsafe { alloc(layout) }.cast::<T>()) else {
        return Err(value);
    };
    unsafe {
        pointer.as_ptr().write(value);
        Ok(Box::from_raw(pointer.as_ptr()))
    }
}

fn take_box<T>(value: Box<T>) -> T {
    let pointer = Box::into_raw(value);
    unsafe {
        let value = pointer.read();
        dealloc(pointer.cast::<u8>(), Layout::new::<T>());
        value
    }
}

fn slots_ref() -> &'static [ProcessSlot; PROCESS_CAPACITY] {
    unsafe { &*TABLE.0.get() }
}

fn slots_mut() -> &'static mut [ProcessSlot; PROCESS_CAPACITY] {
    unsafe { &mut *TABLE.0.get() }
}

fn assert_initialized() {
    if !INITIALIZED.load(Ordering::Acquire) {
        panic!("process manager used before initialization");
    }
}

fn reset_counters() {
    CREATED.store(0, Ordering::Relaxed);
    EXITED.store(0, Ordering::Relaxed);
    REAPED.store(0, Ordering::Relaxed);
    TERMINATED_EXITED.store(0, Ordering::Relaxed);
    TERMINATED_FAULTED.store(0, Ordering::Relaxed);
    TERMINATED_KILLED.store(0, Ordering::Relaxed);
    LIVE.store(0, Ordering::Relaxed);
    PEAK_LIVE.store(0, Ordering::Relaxed);
    SPAWN_WAITS.store(0, Ordering::Relaxed);
    CAPACITY_REJECTIONS.store(0, Ordering::Relaxed);
    WAIT_CALLS.store(0, Ordering::Relaxed);
    WAIT_COMPLETED.store(0, Ordering::Relaxed);
    WAIT_IMMEDIATE.store(0, Ordering::Relaxed);
    WAIT_STALE.store(0, Ordering::Relaxed);
    CHILD_SYSCALLS.store(0, Ordering::Relaxed);
    CHILD_SELECTIONS.store(0, Ordering::Relaxed);
    CHILD_HANDLE_ISOLATED.store(false, Ordering::Relaxed);
    STARTUP_MOVES.store(0, Ordering::Relaxed);
    CHILD_FRAMES_ISOLATED.store(false, Ordering::Relaxed);
    FIRST_CHILD_PID.store(0, Ordering::Relaxed);
    SECOND_CHILD_PID.store(0, Ordering::Relaxed);
    for pid in &CHILD_SPAWN_PIDS {
        pid.store(0, Ordering::Relaxed);
    }
    for image in &CHILD_SPAWN_IMAGES {
        image.store(0, Ordering::Relaxed);
    }
    #[cfg(feature = "unified-product-liveness-runtime")]
    {
        UNIFIED_PRODUCT_STORAGE_REPLACEMENT_PID.store(0, Ordering::Relaxed);
        UNIFIED_PRODUCT_STORAGE_REPLACEMENT_IMAGE.store(0, Ordering::Relaxed);
    }
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    {
        UNIFIED_PRODUCT_STORAGE_SECOND_REPLACEMENT_PID.store(0, Ordering::Relaxed);
        UNIFIED_PRODUCT_STORAGE_SECOND_REPLACEMENT_IMAGE.store(0, Ordering::Relaxed);
    }
    #[cfg(feature = "service-dependency-runtime")]
    {
        SERVICE_DEPENDENCY_INPUT_REPLACEMENT_PID.store(0, Ordering::Relaxed);
        SERVICE_DEPENDENCY_INPUT_REPLACEMENT_IMAGE.store(0, Ordering::Relaxed);
    }
    FIRST_CHILD_ASID.store(0, Ordering::Relaxed);
    LAST_CHILD_ASID.store(0, Ordering::Relaxed);
    FIRST_CHILD_ROOT.store(0, Ordering::Relaxed);
    FIRST_CHILD_CODE_PHYSICAL.store(0, Ordering::Relaxed);
    FIRST_CHILD_STACK_PHYSICAL.store(0, Ordering::Relaxed);
    INIT_ROOT.store(0, Ordering::Relaxed);
    INIT_CODE_PHYSICAL.store(0, Ordering::Relaxed);
    INIT_STACK_PHYSICAL.store(0, Ordering::Relaxed);
    SLOT_REUSED.store(false, Ordering::Relaxed);
    ASID_REUSED.store(false, Ordering::Relaxed);
    STALE_REJECTED.store(false, Ordering::Relaxed);
    ROOT_ISOLATED.store(false, Ordering::Relaxed);
    FRAME_RESTORED.store(true, Ordering::Relaxed);
    HEAP_RESTORED.store(true, Ordering::Relaxed);
    EXIT_VALIDATION_FAILURES.store(0, Ordering::Relaxed);
    NONSTANDARD_EXIT_CODES.store(0, Ordering::Relaxed);
    NON_TARGET_REAPS.store(0, Ordering::Relaxed);
    FRAME_DESTROY_DELTA_VALID.store(true, Ordering::Relaxed);
    DISTINCT_LIVE_SLOTS.store(false, Ordering::Relaxed);
    DISTINCT_LIVE_ASIDS.store(false, Ordering::Relaxed);
    DISTINCT_LIVE_ROOTS.store(false, Ordering::Relaxed);
    DISTINCT_LIVE_FRAMES.store(false, Ordering::Relaxed);
    GRAPHICS_CONSUMER_DEATHS.store(0, Ordering::Relaxed);
    GRAPHICS_CONSUMER_DEATH_MAPPINGS.store(0, Ordering::Relaxed);
    GRAPHICS_CONSUMER_DEATH_QUEUED.store(0, Ordering::Relaxed);
    GRAPHICS_CONSUMER_DEATH_ACQUIRED.store(0, Ordering::Relaxed);
    GRAPHICS_CONSUMER_DEATH_PRODUCER_RESTORES.store(0, Ordering::Relaxed);
    GRAPHICS_CONSUMER_DEATH_PRODUCER_ORPHANS.store(0, Ordering::Relaxed);
    GRAPHICS_CONSUMER_DEATH_RELEASES.store(0, Ordering::Relaxed);
    GRAPHICS_CONSUMER_DEATH_WAKE_COMPLETIONS.store(0, Ordering::Relaxed);
}
