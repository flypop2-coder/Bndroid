use core::cell::UnsafeCell;
use core::mem::size_of;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

#[cfg(feature = "unified-product-manifest-supervision-runtime")]
use bndr_abi::SERVICE_MANIFEST_OPEN_FLAGS_NONE;
#[cfg(feature = "androidbox-interactive0")]
use bndr_abi::unpack_surface_layer_handles;
use bndr_abi::{
    ABI_VERSION, CHANNEL_MESSAGE_MAX_BYTES, CHANNEL_READ_ENVELOPE_SIZE, ChannelMessageKind,
    ChannelReadEnvelope, EVENT_CREATE_SIGNALED, GRAPHICS_BUFFER_ACQUIRE_FLAGS_NONE,
    GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE, GRAPHICS_BUFFER_CREATE_FLAGS_NONE,
    GRAPHICS_BUFFER_FORMAT_XRGB8888, GRAPHICS_BUFFER_HEIGHT, GRAPHICS_BUFFER_LOGICAL_BYTES,
    GRAPHICS_BUFFER_MAP_CONSUMER_RO, GRAPHICS_BUFFER_MAP_PRODUCER_RW,
    GRAPHICS_BUFFER_QUEUE_FLAGS_NONE, GRAPHICS_BUFFER_RELEASE_FLAGS_NONE, GRAPHICS_BUFFER_WIDTH,
    GRAPHICS_BUFFER_WRITE_MAX_BYTES, HandleValue, OBJECT_WAIT_MANY_ARRAY_ITEM_WIRE_SIZE,
    OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS, OBJECT_WAIT_MANY_ARRAY_MAX_WIRE_SIZE,
    OBJECT_WAIT_MANY_ITEM_COUNT, OBJECT_WAIT_TIMEOUT_INFINITE, OBJECT_WAIT_TIMEOUT_POLL,
    ObjectSignals, PROCESS_SPAWN_FLAGS_NONE, PROCESS_TERMINATE_FLAGS_NONE, Rights,
    SYSTEM_FILE_PATH_MAX_BYTES, Status, SyscallNumber, UserImageId, VMO_READ_MAX_BYTES,
    unpack_graphics_buffer_geometry, unpack_graphics_buffer_write, unpack_transfer,
    unpack_vmo_read, unpack_wait_item,
};
#[cfg(feature = "androidbox-process0")]
use bndr_abi::{
    ANDROID_APP_MESSAGE_PAYLOAD_BYTES, ANDROID_APP_MESSAGE_WIRE_SIZE, AndroidAppMessage,
    AndroidAppMessageKind,
};
#[cfg(feature = "androidbox-scene-rpc2")]
use bndr_abi::{
    ANDROID_APP_SCENE_MAX_NODES, ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE,
    AndroidAppSceneNodeDescriptor, AndroidAppSceneNodeKind,
};
#[cfg(feature = "androidbox-restart0")]
use bndr_abi::{
    ANDROID_APP_SUPERVISOR_WIRE_SIZE, AndroidAppSupervisorMessage, AndroidAppSupervisorMessageKind,
};
#[cfg(feature = "app-data-runtime")]
use bndr_abi::{
    APP_DATA_DIRECTORY_ENTRY_WIRE_SIZE, APP_DATA_DIRECTORY_MAX_ENTRIES, APP_DATA_FILE_MAX_BYTES,
    APP_DATA_PATH_MAX_BYTES, AppDataDirectoryEntry, AppDataPrincipal,
    FILE_REPLACE_REQUEST_WIRE_SIZE, FileReplaceCas, FileReplaceRequest, validate_app_data_path,
};
#[cfg(feature = "androidbox-layout-row14")]
use bndr_abi::{AndroidAppSceneDimension, AndroidAppSceneOrientation};
#[cfg(feature = "storage-server-runtime")]
use bndr_abi::{
    IPC_BUFFER_CREATE_FLAGS_NONE, IPC_BUFFER_PAYLOAD_MAX_BYTES, STORAGE_BLOCK_DATA_MAX_BYTES,
    STORAGE_BLOCK_REQUEST_WIRE_SIZE, STORAGE_SECTOR_SIZE, STORAGE_SESSION_BINDING_WIRE_SIZE,
    StorageBlockOperation, StorageBlockRequest,
};
#[cfg(feature = "resident-platform-shutdown-runtime")]
use bndr_abi::{
    SERVICE_SHUTDOWN_FLAGS_NONE, SERVICE_SHUTDOWN_QUIESCE, SERVICE_SHUTDOWN_REGISTER,
    ShutdownServiceNode,
};
#[cfg(feature = "mobile-ui-runtime")]
use bndr_abi::{SYSTEM_CLOCK_READ_FLAGS_NONE, SYSTEM_CLOCK_SOURCE_QEMU_PL031};
#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
use bndr_abi::{SYSTEM_SHUTDOWN_COMMIT, SYSTEM_SHUTDOWN_FLAGS_NONE, SYSTEM_SHUTDOWN_PREPARE};
#[cfg(feature = "app-data-runtime")]
use bndr_appdata::{Error as AppDataError, IoError as AppDataIoError, ReplaceCondition};
use bndr_ui::{
    APP_LIFECYCLE_MAGIC, APP_LIFECYCLE_WIRE_SIZE, AppInstanceIdentity, AppLifecycleAction,
    AppLifecycleMessage, AppLifecyclePayload, AppLifecycleReason, AppLifecycleState,
    AppLifecycleTracker, BUFFER_PRESENT_WIRE_SIZE, BufferPresent, PRESENT_WIRE_SIZE, PresentFrame,
    PresentMode, ProtocolError, UI_SUPERVISOR_CONTROL_MAGIC, UI_SUPERVISOR_CONTROL_WIRE_SIZE,
    UiSupervisorControlMessage, UiSupervisorControlPayload, UiSupervisorOperation,
    UiSupervisorTracker,
};
#[cfg(feature = "surface-trace-evidence")]
use bndr_ui::{ShellAppId, UiClientId};
use bndroid_kernel::channel::{
    ChannelEndpoint, ChannelError, ChannelReadFailure, ChannelWriteFailure, Message,
    TransferPayload,
};
use bndroid_kernel::event::Event;
use bndroid_kernel::graphics_buffer::{GraphicsBuffer, GraphicsBufferError};
use bndroid_kernel::handles::{HandleError, HandleTable, OwnedHandle};
#[cfg(feature = "app-data-runtime")]
use bndroid_kernel::object::AppDataDirectoryCapability;
use bndroid_kernel::object::KernelObject;
use bndroid_kernel::object_wait::ObjectWaitItem;
#[cfg(feature = "androidbox-interactive0")]
use bndroid_kernel::surface::{
    LayeredPresentPreflight, LayeredPresentPreflightError, LayeredPresentSubmissionPreflight,
    validate_layered_present_preflight,
};
use bndroid_kernel::surface::{
    SURFACE_INPUT_QUEUE_CAPACITY, SurfaceCapability, SurfaceError, SurfaceInputError,
    SurfaceKeyError, SurfaceOwner,
};
#[cfg(feature = "surface-trace-evidence")]
use bndroid_kernel::ui_trace::{
    AuthenticatedUiChannelTrace, UiChannelTrace, UiTraceRole, decode_authenticated_ui_channel_read,
};

use crate::arch::aarch64::exceptions::TrapFrame;
use crate::arch::aarch64::mmu::{GraphicsMappingAccess, GraphicsMappingRole};

pub(crate) const HANDLE_CAPACITY: usize = 32;
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
pub const INIT_READY_MAGIC: u64 = 0xc0de_1a17_baad_f00d;
#[cfg(feature = "storage-server-runtime")]
pub const M55_STORAGE_READY_PREFIX: u64 = 0x4d35_3547_0000_0000;
#[cfg(feature = "storage-server-runtime")]
pub const M55_STORAGE_PROOF_PREFIX: u64 = 0x4d35_3550_0000_0000;
#[cfg(feature = "storage-server-runtime")]
const M55_STORAGE_PREFIX_MASK: u64 = 0xffff_ffff_0000_0000;
#[cfg(feature = "storage-server-runtime")]
const M55_STORAGE_PAYLOAD_MASK: u64 = 0x0000_0000_ffff_ffff;
#[cfg(all(
    feature = "storage-server-recovery-runtime",
    not(feature = "storage-server-repeated-recovery-runtime")
))]
const M56_STORAGE_READY_PREFIX: u64 = 0x4d35_3647_0000_0000;
#[cfg(all(
    feature = "storage-server-recovery-runtime",
    not(feature = "storage-server-repeated-recovery-runtime")
))]
const M56_STORAGE_PROOF: u64 = 0x4d35_3650_0003_0004;
#[cfg(all(
    feature = "storage-server-repeated-recovery-runtime",
    not(feature = "storage-server-async-recovery-runtime")
))]
const M57_STORAGE_READY_PREFIX: u64 = 0x4d35_3747_0000_0000;
#[cfg(all(
    feature = "storage-server-repeated-recovery-runtime",
    not(feature = "storage-server-async-recovery-runtime")
))]
const M57_STORAGE_PROOF: u64 = 0x4d35_3750_0006_0007;
#[cfg(all(
    feature = "storage-server-async-recovery-runtime",
    not(feature = "storage-server-fault-policy-runtime")
))]
const M58_STORAGE_READY_PREFIX: u64 = 0x4d35_3847_0000_0000;
#[cfg(all(
    feature = "storage-server-async-recovery-runtime",
    not(feature = "storage-server-fault-policy-runtime")
))]
const M58_STORAGE_PROOF: u64 = 0x4d35_3850_0006_0007;
#[cfg(feature = "storage-server-fault-policy-runtime")]
const M60_STORAGE_READY_PREFIX: u64 = 0x4d36_3047_0000_0000;
#[cfg(feature = "storage-server-fault-policy-runtime")]
const M60_STORAGE_PROOF: u64 = 0x4d36_3050_0007_0008;
#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
const M65_STORAGE_READY_PREFIX: u64 = 0x4d36_3547_0000_0000;
#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
const M65_STORAGE_PROOF: u64 = 0x4d36_3550_0002_0003;
#[cfg(feature = "resident-platform-shutdown-runtime")]
const M66_STORAGE_READY_PREFIX: u64 = 0x4d36_3647_0000_0000;
#[cfg(feature = "resident-platform-shutdown-runtime")]
const M66_STORAGE_PROOF: u64 = 0x4d36_3650_080a_0301;
#[cfg(all(
    feature = "unified-product-runtime",
    not(feature = "unified-product-liveness-runtime")
))]
const M67_STORAGE_READY_PREFIX: u64 = 0x4d36_3747_0000_0000;
#[cfg(all(
    feature = "unified-product-runtime",
    not(feature = "unified-product-liveness-runtime")
))]
const M67_STORAGE_PROOF: u64 = 0x4d36_3750_080f_0301;
#[cfg(all(
    feature = "unified-product-liveness-runtime",
    not(feature = "unified-product-multiservice-liveness-runtime")
))]
const M68_STORAGE_READY_PREFIX: u64 = 0x4d36_3847_0000_0000;
#[cfg(all(
    feature = "unified-product-liveness-runtime",
    not(feature = "unified-product-multiservice-liveness-runtime")
))]
const M68_STORAGE_PROOF: u64 = 0x4d36_3850_0302_0101;
#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
const M69_STORAGE_READY_PREFIX: u64 = 0x4d36_3947_0000_0000;
#[cfg(feature = "unified-product-multiservice-liveness-runtime")]
const M69_STORAGE_PROOF: u64 = 0x4d36_3950_0807_0201;
#[cfg(feature = "unified-product-psci-shutdown-runtime")]
const M70_STORAGE_READY_PREFIX: u64 = 0x4d37_3047_0000_0000;
#[cfg(feature = "unified-product-psci-shutdown-runtime")]
const M70_STORAGE_PROOF: u64 = 0x4d37_3050_0807_0201;
#[cfg(feature = "unified-product-continuous-supervision-runtime")]
const M71_STORAGE_READY_PREFIX: u64 = 0x4d37_3147_0000_0000;
#[cfg(feature = "unified-product-continuous-supervision-runtime")]
const M71_STORAGE_PROOF: u64 = 0x4d37_3150_1505_0401;
#[cfg(feature = "unified-product-manifest-supervision-runtime")]
const M72_STORAGE_READY_PREFIX: u64 = 0x4d37_3247_0000_0000;
#[cfg(feature = "unified-product-manifest-supervision-runtime")]
const M72_STORAGE_PROOF: u64 = 0x4d37_3250_0105_0401;
#[cfg(feature = "unified-product-event-supervision-runtime")]
const M73_STORAGE_READY_PREFIX: u64 = 0x4d37_3347_0000_0000;
#[cfg(feature = "unified-product-event-supervision-runtime")]
const M73_STORAGE_PROOF: u64 = 0x4d37_3350_0205_0401;
pub const EXPECTED_TAG: u64 = 0x1234;
pub const EXPECTED_PAYLOAD: u64 = 0x0123_4567_89ab_cdef;
pub const USER_REGISTER_SENTINEL: u64 = 0xd15c_a11e_0bad_c0de;
pub const USER_STACK_SENTINEL: u64 = 0x5a5a_c3c3_f00d_baad;
#[cfg(feature = "process-terminate-self-test")]
const PROCESS_TERMINATE_SELF_TEST_READY_1: u64 = 0x5054_5354_5244_5931;
#[cfg(feature = "process-terminate-self-test")]
const PROCESS_TERMINATE_SELF_TEST_READY_2: u64 = 0x5054_5354_5244_5932;
const M14_SERVING_ACK_TAG: u64 = 0x5352_565f_4143_4b21;
pub(crate) const M14_SERVING_DONE_TAG: u64 = 0x5352_565f_444f_4e45;
const M14_ROUND_ONE_ACK: u64 = (0x101_u64 << 32) | 1;
const M14_ROUND_TWO_ACK: u64 = (0x102_u64 << 32) | 2;
const M14_MANAGER_DONE: u64 = (2_u64 << 32) | 3;
pub(crate) const M14_PROVIDER_DONE: u64 = (2_u64 << 32) | 1;
pub(crate) const M15_SERVICE_COMMIT_TAG: u64 = 0x4431_3543_4f4d_4d54;
pub(crate) const M15_SERVICE_DONE_TAG: u64 = 0x4431_3544_4f4e_4521;
const M15_SERVICE_PHASE_COUNT: u8 = 10;
const M15_SERVICE_TXID_COUNT: usize = 8;
pub(crate) const M16_SERVICE_CYCLE_COMMIT_TAG_PREFIX: u64 = 0x4431_3643_0000_0000;
pub(crate) const M16_SERVICE_CYCLE_DONE_TAG_PREFIX: u64 = 0x4431_3644_0000_0000;
pub(crate) const M17_PROVIDER_IDENTITY_DONE_TAG: u64 = 0x4d31_3749_4445_4e54;
pub(crate) const M17_CLIENT_IDENTITY_DONE_TAG: u64 = 0x4d31_3749_4443_4c54;
pub(crate) const M17_SERVICE_ACL_DONE_TAG: u64 = 0x4d31_3741_434c_4f4b;
pub(crate) const M17_SERVICE_ACL_TXID: u32 = 0x401;
pub(crate) const M17_SERVICE_MALFORMED_REJECTIONS: u16 = 4;
pub(crate) const M18_CLIENT_READY_TAG: u64 = 0x4d31_3852_4541_4459;
pub(crate) const M18_CLIENT_QUEUED_TAG: u64 = 0x4d31_3851_5545_5545;
pub(crate) const M18_CLIENT_DONE_TAG: u64 = 0x4d31_3843_444f_4e45;
pub(crate) const M18_PROVIDER_READY_TAG: u64 = 0x4d31_3850_5244_5921;
pub(crate) const M18_PROVIDER_DONE_TAG: u64 = 0x4d31_3850_444f_4e45;
pub(crate) const M18_MANAGER_DONE_TAG: u64 = 0x4d31_384d_444f_4e45;
pub(crate) const M18_PRIMARY_TXID: u32 = 0x501;
pub(crate) const M18_SECONDARY_TXID: u32 = 0x502;
pub(crate) const M18_SERVICE_INSTANCE: u32 = 0x0002_0001;
const M18_CLIENT_COUNT: u64 = 2;
pub(crate) const M20_MANAGER_EVENT_TAG: u64 = 0x4d32_304d_4556_4e54;
pub(crate) const M20_CLIENT_EVENT_TAG: u64 = 0x4d32_3043_4556_4e54;
pub(crate) const M20_PROVIDER_EVENT_TAG: u64 = 0x4d32_3050_4556_4e54;
pub(crate) const M20_STALLED_SECONDARY_TXID: u32 = 0x601;
pub(crate) const M20_PRIMARY_STALLED_PROGRESS_TXID: u32 = 0x602;
pub(crate) const M20_PRIMARY_DETACHED_PROGRESS_TXID: u32 = 0x603;
pub(crate) const M20_SECONDARY_REATTACHED_TXID: u32 = 0x604;
#[cfg(not(feature = "el0-fault-containment-self-test"))]
pub(crate) const M20_TRANSCRIPT_PHASE_COUNT: u8 = 23;
const M16_SERVICE_CYCLE_TAG_PREFIX_MASK: u64 = 0xffff_ffff_0000_0000;
const M16_SERVICE_CYCLE_ROUND_MASK: u64 = 0x00ff_ffff;
const M16_SERVICE_CYCLE_TXID_COUNT: usize = 4;

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static HEAP_FREE_BASELINE: AtomicUsize = AtomicUsize::new(0);
static CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "mobile-ui-runtime")]
static SYSTEM_CLOCK_LAST_LOGGED_MINUTE: AtomicU64 = AtomicU64::new(u64::MAX);
static SUCCESSES: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);
static UNKNOWN: AtomicU64 = AtomicU64::new(0);
static CHANNELS_CREATED: AtomicU64 = AtomicU64::new(0);
static WRITES: AtomicU64 = AtomicU64::new(0);
static READS: AtomicU64 = AtomicU64::new(0);
static CHANNEL_PEEKS: AtomicU64 = AtomicU64::new(0);
static CHANNEL_PEEK_SCALAR: AtomicU64 = AtomicU64::new(0);
static CHANNEL_PEEK_BYTES: AtomicU64 = AtomicU64::new(0);
static CHANNEL_PEEK_TRANSFER: AtomicU64 = AtomicU64::new(0);
static CHANNEL_ENVELOPE_READS: AtomicU64 = AtomicU64::new(0);
static CHANNEL_ENVELOPE_SCALAR: AtomicU64 = AtomicU64::new(0);
static CHANNEL_ENVELOPE_BYTES: AtomicU64 = AtomicU64::new(0);
static CHANNEL_ENVELOPE_TRANSFER: AtomicU64 = AtomicU64::new(0);
static CHANNEL_ENVELOPE_PRESERVED_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static CHANNEL_ENVELOPE_BUFFER_TOO_SMALL: AtomicU64 = AtomicU64::new(0);
static CHANNEL_ENVELOPE_BAD_ADDRESS: AtomicU64 = AtomicU64::new(0);
static CHANNEL_ENVELOPE_TABLE_FULL: AtomicU64 = AtomicU64::new(0);
static DUPLICATES: AtomicU64 = AtomicU64::new(0);
static CLOSES: AtomicU64 = AtomicU64::new(0);
static STALE_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static RIGHTS_DENIALS: AtomicU64 = AtomicU64::new(0);
static LAST_TAG: AtomicU64 = AtomicU64::new(0);
static LAST_PAYLOAD: AtomicU64 = AtomicU64::new(0);
static INIT_READY: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "storage-server-runtime")]
static M55_STORAGE_GENERATION: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-runtime")]
static M55_STORAGE_STAGES: AtomicU64 = AtomicU64::new(0);
static INIT_FAILURE: AtomicU64 = AtomicU64::new(0);
static PAN_FAILURES: AtomicU64 = AtomicU64::new(0);
static PRIVATE_SVC_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static SHOULD_WAIT_RETURNS: AtomicU64 = AtomicU64::new(0);
static REGISTER_PRESERVED: AtomicBool = AtomicBool::new(false);
static STACK_ROUND_TRIP: AtomicBool = AtomicBool::new(false);
static USER_ASID: AtomicU64 = AtomicU64::new(0);
static BYTE_WRITES: AtomicU64 = AtomicU64::new(0);
static BYTE_READS: AtomicU64 = AtomicU64::new(0);
static BYTE_READ_ROLLBACKS: AtomicU64 = AtomicU64::new(0);
static BYTE_BUFFER_TOO_SMALL: AtomicU64 = AtomicU64::new(0);
static BYTE_ZERO_LENGTH: AtomicU64 = AtomicU64::new(0);
static TRANSFER_WRITES: AtomicU64 = AtomicU64::new(0);
static TRANSFER_READS: AtomicU64 = AtomicU64::new(0);
static TRANSFER_READ_ROLLBACKS: AtomicU64 = AtomicU64::new(0);
static TRANSFER_SELF_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static TRANSFER_ORDER_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static OBJECT_WAIT_CALLS: AtomicU64 = AtomicU64::new(0);
static OBJECT_WAIT_IMMEDIATE: AtomicU64 = AtomicU64::new(0);
static WAIT_MANY_CALLS: AtomicU64 = AtomicU64::new(0);
static WAIT_MANY_IMMEDIATE: AtomicU64 = AtomicU64::new(0);
static WAIT_MANY_POLL_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_CALLS: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_IMMEDIATE: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_POLL_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_INVALID_COUNTS: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_BAD_ADDRESSES: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_VALIDATION_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_MAX_ITEMS_OBSERVED: AtomicU64 = AtomicU64::new(0);
static WAIT_ARRAY_LOWEST_MULTI_READY: AtomicU64 = AtomicU64::new(0);
static EVENTS_CREATED: AtomicU64 = AtomicU64::new(0);
static EVENT_SIGNAL_CALLS: AtomicU64 = AtomicU64::new(0);
static EVENT_SIGNAL_EDGES: AtomicU64 = AtomicU64::new(0);
static EVENT_CLEAR_CALLS: AtomicU64 = AtomicU64::new(0);
static EVENT_CLEAR_EDGES: AtomicU64 = AtomicU64::new(0);
static EVENT_SIGNAL_DENIALS: AtomicU64 = AtomicU64::new(0);
static EVENT_WAIT_CALLS: AtomicU64 = AtomicU64::new(0);
static EVENT_WAIT_BLOCKS: AtomicU64 = AtomicU64::new(0);
static EVENT_WAIT_WAKES: AtomicU64 = AtomicU64::new(0);
static EVENT_TRANSFER_WRITES: AtomicU64 = AtomicU64::new(0);
static EVENT_TRANSFER_READS: AtomicU64 = AtomicU64::new(0);
static FILE_OPEN_CALLS: AtomicU64 = AtomicU64::new(0);
static FILE_OPEN_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static VMO_READ_CALLS: AtomicU64 = AtomicU64::new(0);
static VMO_READ_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static VMO_READ_BYTES: AtomicU64 = AtomicU64::new(0);
static VMO_TRANSFER_WRITES: AtomicU64 = AtomicU64::new(0);
static VMO_TRANSFER_READS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-manifest-supervision-runtime")]
static SERVICE_MANIFEST_OPEN_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-manifest-supervision-runtime")]
static SERVICE_MANIFEST_OPEN_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-manifest-supervision-runtime")]
static SERVICE_MANIFEST_OPEN_ARGUMENT_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-manifest-supervision-runtime")]
static SERVICE_MANIFEST_OPEN_PERMISSION_DENIALS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-verified-manifest-runtime")]
static VERIFIED_MANIFEST_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-verified-manifest-runtime")]
static VERIFIED_MANIFEST_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-verified-manifest-runtime")]
static VERIFIED_MANIFEST_SIGNATURE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-verified-manifest-runtime")]
static VERIFIED_MANIFEST_SIGNATURE_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-verified-manifest-runtime")]
static VERIFIED_MANIFEST_ROLLBACK_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-verified-manifest-runtime")]
static VERIFIED_MANIFEST_FORMAT_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-verified-manifest-runtime")]
static VERIFIED_MANIFEST_ROLLBACK_INDEX: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-verified-manifest-runtime")]
static VERIFIED_MANIFEST_DIGEST_0: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-verified-manifest-runtime")]
static VERIFIED_MANIFEST_DIGEST_1: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-verified-manifest-runtime")]
static VERIFIED_MANIFEST_DIGEST_2: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-verified-manifest-runtime")]
static VERIFIED_MANIFEST_DIGEST_3: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-persistent-rollback-runtime")]
static PERSISTENT_MANIFEST_LEDGER_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-persistent-rollback-runtime")]
static PERSISTENT_MANIFEST_LEDGER_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-persistent-rollback-runtime")]
static PERSISTENT_MANIFEST_LEDGER_ROLLBACK_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-persistent-rollback-runtime")]
static PERSISTENT_MANIFEST_LEDGER_FAILURES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-key-rotation-runtime")]
static KEY_ROTATION_RETIRED_KEY_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_AUTHORIZATION_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_AUTHORIZATION_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_AUTHORIZATION_SIGNATURE_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_AUTHORIZATION_BINDING_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_AUDIT_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_AUDIT_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_AUDIT_REPLAY_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_AUDIT_FAILURES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_SESSION_OPEN_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_SESSION_OPEN_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_SESSION_OPEN_ARGUMENT_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_SESSION_OPEN_PERMISSION_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_SESSION_OPEN_STATE_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_SESSION_OPENED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_REPORT_GATE_DENIALS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
static SIGNED_MAINTENANCE_PLAN_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
static SIGNED_MAINTENANCE_PLAN_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
static SIGNED_MAINTENANCE_PLAN_SIGNATURE_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
static SIGNED_MAINTENANCE_PLAN_BINDING_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
static SIGNED_MAINTENANCE_PLAN_PROGRAM_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
static SIGNED_MAINTENANCE_PLAN_PERSISTENCE_FAILURES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "unified-product-verified-manifest-runtime")]
static VERIFIED_MANIFEST_ARTIFACT: &[u8; bndr_sm::verified_manifest::SIGNED_SERVICE_MANIFEST_SIZE] =
    include_bytes!(env!("BNDR_VERIFIED_MANIFEST_ARTIFACT"));
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_AUTHORIZATION_ARTIFACT:
    &[u8; bndr_sm::maintenance_authorization::MAINTENANCE_AUTHORIZATION_SIZE] =
    include_bytes!(env!("BNDR_MAINTENANCE_AUTHORIZATION_ARTIFACT"));
#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
static MAINTENANCE_PLAN_ARTIFACT: &[u8; bndr_sm::maintenance_plan::MAINTENANCE_PLAN_SIZE] =
    include_bytes!(env!("BNDR_MAINTENANCE_PLAN_ARTIFACT"));

#[cfg(feature = "unified-product-persistent-rollback-runtime")]
struct PersistentManifestPreparation {
    prepared: AtomicBool,
    evidence: UnsafeCell<Option<bndroid_kernel::persist::ManifestRollbackEvidence>>,
}

#[cfg(feature = "unified-product-persistent-rollback-runtime")]
impl PersistentManifestPreparation {
    const fn new() -> Self {
        Self {
            prepared: AtomicBool::new(false),
            evidence: UnsafeCell::new(None),
        }
    }

    fn publish(&self, evidence: bndroid_kernel::persist::ManifestRollbackEvidence) {
        if self.prepared.load(Ordering::Acquire) {
            panic!("persistent manifest preparation published twice");
        }
        // SAFETY: the pre-EL0 boot monitor is the only writer. The following
        // Release store publishes the completed Copy value to later readers.
        unsafe {
            *self.evidence.get() = Some(evidence);
        }
        self.prepared.store(true, Ordering::Release);
    }

    fn snapshot(&self) -> Option<bndroid_kernel::persist::ManifestRollbackEvidence> {
        if !self.prepared.load(Ordering::Acquire) {
            return None;
        }
        // SAFETY: Acquire observed the one-time Release publication, and no
        // writer mutates the evidence after that publication.
        unsafe { *self.evidence.get() }
    }
}

// SAFETY: the only mutation happens on the pre-EL0 boot monitor and is
// published exactly once through `prepared`; later accesses are read-only.
#[cfg(feature = "unified-product-persistent-rollback-runtime")]
unsafe impl Sync for PersistentManifestPreparation {}

#[cfg(feature = "unified-product-persistent-rollback-runtime")]
static PERSISTENT_MANIFEST_PREPARATION: PersistentManifestPreparation =
    PersistentManifestPreparation::new();

#[cfg(feature = "unified-product-key-rotation-runtime")]
struct KeyRotationManifestPreparation {
    prepared: AtomicBool,
    evidence: UnsafeCell<Option<bndroid_kernel::persist::ManifestKeyRotationEvidence>>,
}

#[cfg(feature = "unified-product-key-rotation-runtime")]
impl KeyRotationManifestPreparation {
    const fn new() -> Self {
        Self {
            prepared: AtomicBool::new(false),
            evidence: UnsafeCell::new(None),
        }
    }

    fn publish(&self, evidence: bndroid_kernel::persist::ManifestKeyRotationEvidence) {
        if self.prepared.load(Ordering::Acquire) {
            panic!("key-rotation manifest preparation published twice");
        }
        // SAFETY: the pre-EL0 monitor is the sole writer and the Release store
        // publishes this Copy evidence before any reader can enter EL0.
        unsafe {
            *self.evidence.get() = Some(evidence);
        }
        self.prepared.store(true, Ordering::Release);
    }

    fn snapshot(&self) -> Option<bndroid_kernel::persist::ManifestKeyRotationEvidence> {
        if !self.prepared.load(Ordering::Acquire) {
            return None;
        }
        // SAFETY: Acquire observes the one-time Release publication.
        unsafe { *self.evidence.get() }
    }
}

// SAFETY: publication is one-shot on the pre-EL0 monitor and later accesses
// are read-only.
#[cfg(feature = "unified-product-key-rotation-runtime")]
unsafe impl Sync for KeyRotationManifestPreparation {}

#[cfg(feature = "unified-product-key-rotation-runtime")]
static KEY_ROTATION_MANIFEST_PREPARATION: KeyRotationManifestPreparation =
    KeyRotationManifestPreparation::new();

#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
struct MaintenanceAuthorizationPreparation {
    prepared: AtomicBool,
    evidence: UnsafeCell<Option<bndroid_kernel::persist::MaintenanceAuditEvidence>>,
}

#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
impl MaintenanceAuthorizationPreparation {
    const fn new() -> Self {
        Self {
            prepared: AtomicBool::new(false),
            evidence: UnsafeCell::new(None),
        }
    }

    fn publish(&self, evidence: bndroid_kernel::persist::MaintenanceAuditEvidence) {
        if self.prepared.load(Ordering::Acquire) {
            panic!("maintenance authorization preparation published twice");
        }
        // SAFETY: the pre-EL0 monitor is the sole writer. The Release store
        // publishes this Copy evidence before any userspace context can run.
        unsafe {
            *self.evidence.get() = Some(evidence);
        }
        self.prepared.store(true, Ordering::Release);
    }

    fn snapshot(&self) -> Option<bndroid_kernel::persist::MaintenanceAuditEvidence> {
        if !self.prepared.load(Ordering::Acquire) {
            return None;
        }
        // SAFETY: Acquire observes the one-time Release publication, after
        // which the evidence remains immutable.
        unsafe { *self.evidence.get() }
    }
}

// SAFETY: publication is one-shot on the pre-EL0 monitor and all later
// accesses are read-only.
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
unsafe impl Sync for MaintenanceAuthorizationPreparation {}

#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
static MAINTENANCE_AUTHORIZATION_PREPARATION: MaintenanceAuthorizationPreparation =
    MaintenanceAuthorizationPreparation::new();

#[cfg(feature = "unified-product-maintenance-plan-runtime")]
struct MaintenancePlanAdmissionPreparation {
    prepared: AtomicBool,
    evidence: UnsafeCell<Option<bndroid_kernel::persist::MaintenancePlanAdmissionEvidence>>,
}

#[cfg(feature = "unified-product-maintenance-plan-runtime")]
impl MaintenancePlanAdmissionPreparation {
    const fn new() -> Self {
        Self {
            prepared: AtomicBool::new(false),
            evidence: UnsafeCell::new(None),
        }
    }

    fn publish(&self, evidence: bndroid_kernel::persist::MaintenancePlanAdmissionEvidence) {
        if self.prepared.load(Ordering::Acquire) {
            panic!("maintenance plan admission published twice");
        }
        // SAFETY: the pre-EL0 monitor is the sole writer. The Release store
        // publishes this immutable admission before syscall activation.
        unsafe {
            *self.evidence.get() = Some(evidence);
        }
        self.prepared.store(true, Ordering::Release);
    }

    fn snapshot(&self) -> Option<bndroid_kernel::persist::MaintenancePlanAdmissionEvidence> {
        if !self.prepared.load(Ordering::Acquire) {
            return None;
        }
        // SAFETY: Acquire observes the one-time Release publication.
        unsafe { *self.evidence.get() }
    }
}

// SAFETY: publication is one-shot on the pre-EL0 monitor and all later
// accesses are read-only.
#[cfg(feature = "unified-product-maintenance-plan-runtime")]
unsafe impl Sync for MaintenancePlanAdmissionPreparation {}

#[cfg(feature = "unified-product-maintenance-plan-runtime")]
static MAINTENANCE_PLAN_ADMISSION_PREPARATION: MaintenancePlanAdmissionPreparation =
    MaintenancePlanAdmissionPreparation::new();

#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignedMaintenancePlanPreparationSnapshot {
    pub verified: bndr_sm::maintenance_plan::VerifiedMaintenancePlan,
    pub evidence: bndroid_kernel::persist::SignedMaintenancePlanEvidence,
}

#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
struct SignedMaintenancePlanPreparation {
    prepared: AtomicBool,
    snapshot: UnsafeCell<Option<SignedMaintenancePlanPreparationSnapshot>>,
}

#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
impl SignedMaintenancePlanPreparation {
    const fn new() -> Self {
        Self {
            prepared: AtomicBool::new(false),
            snapshot: UnsafeCell::new(None),
        }
    }

    fn publish(&self, snapshot: SignedMaintenancePlanPreparationSnapshot) {
        if self.prepared.load(Ordering::Acquire) {
            panic!("signed maintenance plan preparation published twice");
        }
        // SAFETY: the pre-EL0 monitor is the sole writer and the Release store
        // publishes both immutable Copy values before syscall activation.
        unsafe {
            *self.snapshot.get() = Some(snapshot);
        }
        self.prepared.store(true, Ordering::Release);
    }

    fn snapshot(&self) -> Option<SignedMaintenancePlanPreparationSnapshot> {
        if !self.prepared.load(Ordering::Acquire) {
            return None;
        }
        // SAFETY: Acquire observes the one-shot immutable publication.
        unsafe { *self.snapshot.get() }
    }
}

// SAFETY: publication is one-shot on the pre-EL0 monitor and all later
// accesses are read-only.
#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
unsafe impl Sync for SignedMaintenancePlanPreparation {}

#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
static SIGNED_MAINTENANCE_PLAN_PREPARATION: SignedMaintenancePlanPreparation =
    SignedMaintenancePlanPreparation::new();
#[cfg(feature = "app-data-runtime")]
static APP_DATA_ROOT_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_ROOT_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_FILE_OPEN_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_FILE_OPEN_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_REPLACE_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_REPLACE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_DIRECTORY_CREATE_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_DIRECTORY_CREATE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_UNLINK_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_UNLINK_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_DIRECTORY_READ_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_DIRECTORY_READ_SUCCESSES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_DIRECTORY_READ_EOF: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_PERMISSION_DENIALS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_VALIDATION_REJECTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_SHOULD_WAITS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_CONFLICTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_OUTCOME_UNKNOWN: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_CORRUPTION_ERRORS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "app-data-runtime")]
static APP_DATA_NO_SPACE: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFERS_CREATED: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_CREATE_EXHAUSTIONS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_WRITE_CALLS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_WRITE_BYTES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "mobile-ui-runtime")]
static GRAPHICS_BUFFER_WRITE_SCRATCH: GraphicsBufferWriteScratch =
    GraphicsBufferWriteScratch::new();
#[cfg(feature = "androidbox-multipackage4")]
static ANDROID_PACKAGE_DIRECTORY_WIRE_SCRATCH: AndroidPackageDirectoryWireScratch =
    AndroidPackageDirectoryWireScratch::new();
static GRAPHICS_BUFFER_PRESENTS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_MAPPABLE_CREATED: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_MAP_CALLS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_MAP_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_UNMAP_CALLS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_UNMAP_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_QUEUE_CALLS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_QUEUE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_ACQUIRE_CALLS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_ACQUIRE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_RELEASE_CALLS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_RELEASE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_RELEASES: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_MAPPED_PRESENTS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_VALIDATED_PIXELS: AtomicU64 = AtomicU64::new(0);
static GRAPHICS_BUFFER_VALIDATED_BYTES: AtomicU64 = AtomicU64::new(0);
static SURFACE_REACQUIRES: AtomicU64 = AtomicU64::new(0);
static SURFACE_REACQUIRE_OLD_PID: AtomicU64 = AtomicU64::new(0);
static SURFACE_REACQUIRE_NEW_PID: AtomicU64 = AtomicU64::new(0);
static SURFACE_REACQUIRE_OLD_SESSION: AtomicU64 = AtomicU64::new(0);
static SURFACE_REACQUIRE_NEW_SESSION: AtomicU64 = AtomicU64::new(0);
static SURFACE_REACQUIRE_DISCARDED_INPUT: AtomicU64 = AtomicU64::new(0);
static SURFACE_REACQUIRE_DISCARDED_KEYS: AtomicU64 = AtomicU64::new(0);
static SURFACE_REACQUIRE_FROZEN_CURSOR_X: AtomicU64 = AtomicU64::new(0);
static SURFACE_REACQUIRE_FROZEN_CURSOR_Y: AtomicU64 = AtomicU64::new(0);
static SURFACE_REACQUIRE_FROZEN_CURSOR_PIXELS: AtomicU64 = AtomicU64::new(0);
static SURFACE_REACQUIRE_FROZEN_CURSOR_VISIBLE: AtomicBool = AtomicBool::new(false);
static SURFACE_REACQUIRE_FROZEN_CURSOR_PRESSED: AtomicBool = AtomicBool::new(false);
static SURFACE_REACQUIRE_FROZEN_SCANOUT_VALID: AtomicBool = AtomicBool::new(false);
static SURFACE_FRAME_ACQUIRE_CALLS: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "mobile-ui-runtime")]
struct GraphicsBufferWriteScratch {
    borrowed: AtomicBool,
    bytes: UnsafeCell<[u8; GRAPHICS_BUFFER_WRITE_MAX_BYTES]>,
}

#[cfg(feature = "mobile-ui-runtime")]
impl GraphicsBufferWriteScratch {
    const fn new() -> Self {
        Self {
            borrowed: AtomicBool::new(false),
            bytes: UnsafeCell::new([0; GRAPHICS_BUFFER_WRITE_MAX_BYTES]),
        }
    }

    fn acquire(&'static self) -> GraphicsBufferWriteScratchGuard {
        if !crate::arch::aarch64::irq_is_masked() {
            panic!("graphics-buffer write scratch acquired with IRQ enabled");
        }
        if self
            .borrowed
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            panic!("graphics-buffer write scratch was re-entered");
        }
        GraphicsBufferWriteScratchGuard { scratch: self }
    }
}

// SAFETY: GraphicsBufferWrite runs in the single-core IRQ-masked syscall
// domain. The atomic borrow guard detects accidental re-entry before the
// UnsafeCell can be dereferenced a second time.
#[cfg(feature = "mobile-ui-runtime")]
unsafe impl Sync for GraphicsBufferWriteScratch {}

#[cfg(feature = "mobile-ui-runtime")]
struct GraphicsBufferWriteScratchGuard {
    scratch: &'static GraphicsBufferWriteScratch,
}

#[cfg(feature = "mobile-ui-runtime")]
impl GraphicsBufferWriteScratchGuard {
    fn bytes(&mut self) -> &mut [u8; GRAPHICS_BUFFER_WRITE_MAX_BYTES] {
        unsafe { &mut *self.scratch.bytes.get() }
    }
}

#[cfg(feature = "mobile-ui-runtime")]
impl Drop for GraphicsBufferWriteScratchGuard {
    fn drop(&mut self) {
        if !self.scratch.borrowed.swap(false, Ordering::Release) {
            panic!("graphics-buffer write scratch guard lost its ownership");
        }
    }
}

#[cfg(feature = "androidbox-multipackage4")]
struct AndroidPackageDirectoryWireScratch {
    borrowed: AtomicBool,
    bytes: UnsafeCell<[u8; bndr_abi::ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE]>,
}

#[cfg(feature = "androidbox-multipackage4")]
impl AndroidPackageDirectoryWireScratch {
    const fn new() -> Self {
        Self {
            borrowed: AtomicBool::new(false),
            bytes: UnsafeCell::new([0; bndr_abi::ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE]),
        }
    }

    fn acquire(&'static self) -> AndroidPackageDirectoryWireScratchGuard {
        if !crate::arch::aarch64::irq_is_masked() {
            panic!("Android package directory scratch acquired with IRQ enabled");
        }
        if self
            .borrowed
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            panic!("Android package directory scratch was re-entered");
        }
        AndroidPackageDirectoryWireScratchGuard { scratch: self }
    }
}

// SAFETY: this scratch is borrowed only in the single-core IRQ-masked syscall
// domain. The atomic lease detects accidental nested use before dereferencing
// the UnsafeCell.
#[cfg(feature = "androidbox-multipackage4")]
unsafe impl Sync for AndroidPackageDirectoryWireScratch {}

#[cfg(feature = "androidbox-multipackage4")]
struct AndroidPackageDirectoryWireScratchGuard {
    scratch: &'static AndroidPackageDirectoryWireScratch,
}

#[cfg(feature = "androidbox-multipackage4")]
impl AndroidPackageDirectoryWireScratchGuard {
    fn bytes(&mut self) -> &mut [u8; bndr_abi::ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE] {
        // SAFETY: `acquire` granted this guard the sole lease and Drop releases
        // it before another caller may borrow the backing array.
        unsafe { &mut *self.scratch.bytes.get() }
    }
}

#[cfg(feature = "androidbox-multipackage4")]
impl Drop for AndroidPackageDirectoryWireScratchGuard {
    fn drop(&mut self) {
        if !self.scratch.borrowed.swap(false, Ordering::Release) {
            panic!("Android package directory scratch guard lost its ownership");
        }
    }
}
static SURFACE_FRAME_ACQUIRE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static SURFACE_FRAME_ACQUIRE_SHOULD_WAIT: AtomicU64 = AtomicU64::new(0);
static SURFACE_FRAME_ACQUIRE_INVALID_STATE: AtomicU64 = AtomicU64::new(0);
static SURFACE_KEY_READ_CALLS: AtomicU64 = AtomicU64::new(0);
static SURFACE_KEY_READ_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static SURFACE_KEY_READ_SHOULD_WAIT: AtomicU64 = AtomicU64::new(0);
static SURFACE_KEY_READ_PEER_CLOSED: AtomicU64 = AtomicU64::new(0);
static SURFACE_KEY_READ_INVALID_ARGUMENT: AtomicU64 = AtomicU64::new(0);
static SURFACE_KEY_READ_PERMISSION_DENIED: AtomicU64 = AtomicU64::new(0);
static SURFACE_KEY_READ_INVALID_STATE: AtomicU64 = AtomicU64::new(0);
static INPUT_ACQUIRE_CALLS: AtomicU64 = AtomicU64::new(0);
static INPUT_ACQUIRE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static INPUT_ACQUIRE_PERMISSION_DENIED: AtomicU64 = AtomicU64::new(0);
static INPUT_SESSION_INFO_CALLS: AtomicU64 = AtomicU64::new(0);
static INPUT_SESSION_INFO_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static INPUT_SESSION_INFO_PERMISSION_DENIED: AtomicU64 = AtomicU64::new(0);
static PROCESS_TERMINATE_CALLS: AtomicU64 = AtomicU64::new(0);
static PROCESS_TERMINATE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static PROCESS_TERMINATE_NOT_FOUND: AtomicU64 = AtomicU64::new(0);
static PROCESS_TERMINATE_INVALID_ARGUMENT: AtomicU64 = AtomicU64::new(0);
static PROCESS_TERMINATE_INVALID_STATE: AtomicU64 = AtomicU64::new(0);
static PROCESS_TERMINATE_PERMISSION_DENIED: AtomicU64 = AtomicU64::new(0);
static NEXT_SURFACE_SESSION_ID: AtomicU64 = AtomicU64::new(1);
static SERVICE_ACK_READS: AtomicU64 = AtomicU64::new(0);
static SERVICE_ACK_BITMAP: AtomicU64 = AtomicU64::new(0);
static SERVICE_DONE_READS: AtomicU64 = AtomicU64::new(0);
static SERVICE_DONE_BITMAP: AtomicU64 = AtomicU64::new(0);
static SERVICE_TRANSCRIPT_ERRORS: AtomicU64 = AtomicU64::new(0);
static M15_SERVICE_PHASE: AtomicU64 = AtomicU64::new(0);
static M15_SERVICE_ERRORS: AtomicU64 = AtomicU64::new(0);
static M15_SERVICE_OLD_INSTANCE: AtomicU64 = AtomicU64::new(0);
static M15_SERVICE_NEW_INSTANCE: AtomicU64 = AtomicU64::new(0);
static M15_SERVICE_TXIDS: [AtomicU64; M15_SERVICE_TXID_COUNT] =
    [const { AtomicU64::new(0) }; M15_SERVICE_TXID_COUNT];
static M15_SERVICE_DONE_READS: AtomicU64 = AtomicU64::new(0);
static M15_SERVICE_DONE_BITMAP: AtomicU64 = AtomicU64::new(0);
static M16_SERVICE_CYCLE_ROUND: AtomicU64 = AtomicU64::new(0);
static M16_SERVICE_CYCLE_STEP: AtomicU64 = AtomicU64::new(0);
static M16_SERVICE_CYCLE_COMPLETED_ROUNDS: AtomicU64 = AtomicU64::new(0);
static M16_SERVICE_CYCLE_ERRORS: AtomicU64 = AtomicU64::new(0);
static M16_SERVICE_CYCLE_INSTANCE: AtomicU64 = AtomicU64::new(0);
static M16_SERVICE_CYCLE_TXIDS: [AtomicU64; M16_SERVICE_CYCLE_TXID_COUNT] =
    [const { AtomicU64::new(0) }; M16_SERVICE_CYCLE_TXID_COUNT];
static M16_SERVICE_CYCLE_DONE_READS: AtomicU64 = AtomicU64::new(0);
static M16_SERVICE_CYCLE_DONE_BITMAP: AtomicU64 = AtomicU64::new(0);
static M17_IDENTITY_DONE_READS: AtomicU64 = AtomicU64::new(0);
static M17_ACL_DONE_READS: AtomicU64 = AtomicU64::new(0);
static M17_IDENTITY_ERRORS: AtomicU64 = AtomicU64::new(0);
static M17_PROVIDER_PID: AtomicU64 = AtomicU64::new(0);
static M17_CLIENT_PID: AtomicU64 = AtomicU64::new(0);
static M17_ACL_PAYLOAD: AtomicU64 = AtomicU64::new(0);
static M18_READY_BITMAP: AtomicU64 = AtomicU64::new(0);
static M18_QUEUED_BITMAP: AtomicU64 = AtomicU64::new(0);
static M18_DONE_BITMAP: AtomicU64 = AtomicU64::new(0);
static M18_ERRORS: AtomicU64 = AtomicU64::new(0);
static M18_SECONDARY_CLIENT_PID: AtomicU64 = AtomicU64::new(0);
static M18_REQUEST_ORDER: AtomicU64 = AtomicU64::new(0);
static M20_PHASE: AtomicU64 = AtomicU64::new(0);
static M20_ERRORS: AtomicU64 = AtomicU64::new(0);
static M20_SECONDARY_CLIENT_PID: AtomicU64 = AtomicU64::new(0);
static M20_MANAGER_ATTACHES: AtomicU64 = AtomicU64::new(0);
static M20_CLIENT_ATTACHES: AtomicU64 = AtomicU64::new(0);
static M20_REVOKE_BITMAP: AtomicU64 = AtomicU64::new(0);
static M20_STALE_REVOKE_BITMAP: AtomicU64 = AtomicU64::new(0);
static M20_PRIMARY_PROGRESS_BITMAP: AtomicU64 = AtomicU64::new(0);
static M20_PROVIDER_ACCEPTS: AtomicU64 = AtomicU64::new(0);
static M20_PROVIDER_ECHOES: AtomicU64 = AtomicU64::new(0);
static M20_PROVIDER_ABORTS: AtomicU64 = AtomicU64::new(0);
static M20_SECONDARY_ECHOES: AtomicU64 = AtomicU64::new(0);
static M20_FINAL_IDLE_BITMAP: AtomicU64 = AtomicU64::new(0);

const APP_LIFECYCLE_ACTION_COUNT: usize = 5;
const UI_SUPERVISOR_OPERATION_COUNT: usize = 4;

#[derive(Clone, Copy)]
struct AppLifecycleTraceState {
    tracker: AppLifecycleTracker,
    messages: u64,
    errors: u64,
    decode_errors: u64,
    authentication_errors: u64,
    tracker_errors: u64,
    byte_messages: u64,
    transfer_messages: u64,
    requests: u64,
    state_changes: u64,
    crashes: u64,
    commands: u64,
    acks: u64,
    request_actions: [u64; APP_LIFECYCLE_ACTION_COUNT],
    command_actions: [u64; APP_LIFECYCLE_ACTION_COUNT],
    ack_actions: [u64; APP_LIFECYCLE_ACTION_COUNT],
    transactions: u64,
    completed: u64,
    first_transaction_id: u64,
    last_transaction_id: u64,
    first_instance_id: u64,
    first_app_pid: u64,
    last_instance_id: u64,
    last_app_pid: u64,
    first_sender_pid: u64,
    last_sender_pid: u64,
}

impl AppLifecycleTraceState {
    const fn new() -> Self {
        Self {
            tracker: AppLifecycleTracker::new(),
            messages: 0,
            errors: 0,
            decode_errors: 0,
            authentication_errors: 0,
            tracker_errors: 0,
            byte_messages: 0,
            transfer_messages: 0,
            requests: 0,
            state_changes: 0,
            crashes: 0,
            commands: 0,
            acks: 0,
            request_actions: [0; APP_LIFECYCLE_ACTION_COUNT],
            command_actions: [0; APP_LIFECYCLE_ACTION_COUNT],
            ack_actions: [0; APP_LIFECYCLE_ACTION_COUNT],
            transactions: 0,
            completed: 0,
            first_transaction_id: 0,
            last_transaction_id: 0,
            first_instance_id: 0,
            first_app_pid: 0,
            last_instance_id: 0,
            last_app_pid: 0,
            first_sender_pid: 0,
            last_sender_pid: 0,
        }
    }
}

struct AppLifecycleTraceStorage(UnsafeCell<AppLifecycleTraceState>);

// The trace and process table share one deliberately single-core lifecycle
// domain: every mutation and snapshot runs on the boot CPU with local IRQ
// masked. No two accesses can overlap, so the UnsafeCell has one live borrow.
unsafe impl Sync for AppLifecycleTraceStorage {}

static APP_LIFECYCLE_TRACE: AppLifecycleTraceStorage =
    AppLifecycleTraceStorage(UnsafeCell::new(AppLifecycleTraceState::new()));

#[cfg(all(
    feature = "androidbox-process0",
    not(feature = "androidbox-scene-rpc2")
))]
const ANDROID_APP_FIRST_LABEL_BYTES: u32 = 26;
#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
const ANDROID_APP_FIRST_LABEL_BYTES: u32 = 15;
#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-apk-envelope4")
))]
const ANDROID_APP_FIRST_LABEL_BYTES: u32 = 14;
#[cfg(all(
    feature = "androidbox-apk-envelope4",
    not(feature = "androidbox-manifest-catalog3")
))]
const ANDROID_APP_FIRST_LABEL_BYTES: u32 = 15;
#[cfg(feature = "androidbox-manifest-catalog3")]
const ANDROID_APP_FIRST_LABEL_BYTES: u32 = 18;
#[cfg(all(
    feature = "androidbox-process0",
    not(feature = "androidbox-scene-rpc2")
))]
const ANDROID_APP_FIRST_BUTTON_BYTES: u32 = 11;
#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
const ANDROID_APP_FIRST_BUTTON_BYTES: u32 = 14;
#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-manifest-catalog3")
))]
const ANDROID_APP_FIRST_BUTTON_BYTES: u32 = 7;
#[cfg(feature = "androidbox-manifest-catalog3")]
const ANDROID_APP_FIRST_BUTTON_BYTES: u32 = 15;
#[cfg(feature = "androidbox-icon-resources5")]
const ANDROID_APP_RESOURCE_ID_BASE: u32 = 0x7f02_0000;
#[cfg(not(feature = "androidbox-icon-resources5"))]
const ANDROID_APP_RESOURCE_ID_BASE: u32 = 0x7f01_0000;
#[cfg(all(
    feature = "androidbox-process0",
    not(feature = "androidbox-multiaction3")
))]
const ANDROID_APP_FIRST_UPDATE_BYTES: u32 = 24;
#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-manifest-catalog3")
))]
const ANDROID_APP_FIRST_UPDATE_BYTES: u32 = 18;
#[cfg(feature = "androidbox-manifest-catalog3")]
const ANDROID_APP_FIRST_UPDATE_BYTES: u32 = 24;
#[cfg(not(feature = "androidbox-scene-rpc2"))]
const ANDROID_APP_OPEN_TRANSCRIPT_MESSAGES: u64 = 6;
#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
const ANDROID_APP_OPEN_TRANSCRIPT_MESSAGES: u64 = 16;
#[cfg(feature = "androidbox-layout-row14")]
const ANDROID_APP_OPEN_TRANSCRIPT_MESSAGES: u64 = 19;
#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-layout-row14")
))]
const ANDROID_APP_OPEN_TRANSCRIPT_MESSAGES: u64 = 17;
#[cfg(not(feature = "androidbox-scene-rpc2"))]
const ANDROID_APP_OPEN_LAST_REQUEST_ID: u64 = 1;
#[cfg(feature = "androidbox-layout-row14")]
const ANDROID_APP_OPEN_LAST_REQUEST_ID: u64 = 7;
#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-layout-row14")
))]
const ANDROID_APP_OPEN_LAST_REQUEST_ID: u64 = 6;
#[cfg(not(feature = "androidbox-scene-rpc2"))]
const ANDROID_APP_CRASH_REQUEST_ID: u64 = 2;
#[cfg(feature = "androidbox-layout-row14")]
const ANDROID_APP_CRASH_REQUEST_ID: u64 = 8;
#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-layout-row14")
))]
const ANDROID_APP_CRASH_REQUEST_ID: u64 = 7;
#[cfg(not(feature = "androidbox-scene-rpc2"))]
const ANDROID_APP_CLICK_PRECEDING_MESSAGES: u64 = 6;
#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
const ANDROID_APP_CLICK_PRECEDING_MESSAGES: u64 = 16;
#[cfg(feature = "androidbox-layout-row14")]
const ANDROID_APP_CLICK_PRECEDING_MESSAGES: u64 = 19;
#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-layout-row14")
))]
const ANDROID_APP_CLICK_PRECEDING_MESSAGES: u64 = 17;
#[cfg(not(feature = "androidbox-scene-rpc2"))]
const ANDROID_APP_UPDATED_PRECEDING_MESSAGES: u64 = 7;
#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
const ANDROID_APP_UPDATED_PRECEDING_MESSAGES: u64 = 17;
#[cfg(feature = "androidbox-layout-row14")]
const ANDROID_APP_UPDATED_PRECEDING_MESSAGES: u64 = 20;
#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-layout-row14")
))]
const ANDROID_APP_UPDATED_PRECEDING_MESSAGES: u64 = 18;
#[cfg(not(feature = "androidbox-scene-rpc2"))]
const ANDROID_APP_UPDATE_CHUNK_PRECEDING_MESSAGES: u64 = 8;
#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
const ANDROID_APP_UPDATE_CHUNK_PRECEDING_MESSAGES: u64 = 18;
#[cfg(feature = "androidbox-layout-row14")]
const ANDROID_APP_UPDATE_CHUNK_PRECEDING_MESSAGES: u64 = 21;
#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-layout-row14")
))]
const ANDROID_APP_UPDATE_CHUNK_PRECEDING_MESSAGES: u64 = 19;
#[cfg(not(feature = "androidbox-scene-rpc2"))]
const ANDROID_APP_CLOSE_PRECEDING_MESSAGES: u64 = 9;
#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
const ANDROID_APP_CLOSE_PRECEDING_MESSAGES: u64 = 19;
#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-manifest-catalog3")
))]
const ANDROID_APP_CLOSE_PRECEDING_MESSAGES: u64 = 23;
#[cfg(all(
    feature = "androidbox-manifest-catalog3",
    not(feature = "androidbox-string-builder13")
))]
const ANDROID_APP_CLOSE_PRECEDING_MESSAGES: u64 = 24;
#[cfg(feature = "androidbox-layout-row14")]
const ANDROID_APP_CLOSE_PRECEDING_MESSAGES: u64 = 25;
#[cfg(all(
    feature = "androidbox-string-builder13",
    not(feature = "androidbox-layout-row14")
))]
const ANDROID_APP_CLOSE_PRECEDING_MESSAGES: u64 = 23;
#[cfg(not(feature = "androidbox-scene-rpc2"))]
const ANDROID_APP_CLOSED_PRECEDING_MESSAGES: u64 = 10;
#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
const ANDROID_APP_CLOSED_PRECEDING_MESSAGES: u64 = 20;
#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-manifest-catalog3")
))]
const ANDROID_APP_CLOSED_PRECEDING_MESSAGES: u64 = 24;
#[cfg(all(
    feature = "androidbox-manifest-catalog3",
    not(feature = "androidbox-string-builder13")
))]
const ANDROID_APP_CLOSED_PRECEDING_MESSAGES: u64 = 25;
#[cfg(feature = "androidbox-layout-row14")]
const ANDROID_APP_CLOSED_PRECEDING_MESSAGES: u64 = 26;
#[cfg(all(
    feature = "androidbox-string-builder13",
    not(feature = "androidbox-layout-row14")
))]
const ANDROID_APP_CLOSED_PRECEDING_MESSAGES: u64 = 24;
#[cfg(not(feature = "androidbox-scene-rpc2"))]
const ANDROID_APP_FINAL_REQUEST_ID: u64 = 3;
#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
const ANDROID_APP_FINAL_REQUEST_ID: u64 = 8;
#[cfg(feature = "androidbox-layout-row14")]
const ANDROID_APP_FINAL_REQUEST_ID: u64 = 10;
#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-layout-row14")
))]
const ANDROID_APP_FINAL_REQUEST_ID: u64 = 9;
#[cfg(feature = "androidbox-multiaction3")]
const ANDROID_APP_MULTIACTION_CALLBACKS: usize = 2;
#[cfg(feature = "androidbox-scene-rpc2")]
const ANDROID_APP_TRACKED_CALLBACKS: usize = 4;

#[cfg(feature = "androidbox-process0")]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AndroidAppRpcPhase {
    AwaitReady = 0,
    Idle = 1,
    AwaitOpened = 2,
    LabelChunks = 3,
    ButtonChunks = 4,
    Active = 5,
    AwaitUpdated = 6,
    UpdateChunks = 7,
    AwaitClosed = 8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    AwaitNodeRequest = 9,
    #[cfg(feature = "androidbox-scene-rpc2")]
    AwaitNode = 10,
    #[cfg(feature = "androidbox-scene-rpc2")]
    NodeChunks = 11,
}

#[cfg(feature = "androidbox-process0")]
impl AndroidAppRpcPhase {
    const fn raw(self) -> u8 {
        self as u8
    }

    const fn name(self) -> &'static str {
        match self {
            Self::AwaitReady => "await-ready",
            Self::Idle => "idle",
            Self::AwaitOpened => "await-opened",
            Self::LabelChunks => "label-chunks",
            Self::ButtonChunks => "button-chunks",
            Self::Active => "active",
            Self::AwaitUpdated => "await-updated",
            Self::UpdateChunks => "update-chunks",
            Self::AwaitClosed => "await-closed",
            #[cfg(feature = "androidbox-scene-rpc2")]
            Self::AwaitNodeRequest => "await-node-request",
            #[cfg(feature = "androidbox-scene-rpc2")]
            Self::AwaitNode => "await-node",
            #[cfg(feature = "androidbox-scene-rpc2")]
            Self::NodeChunks => "node-chunks",
        }
    }
}

#[cfg(feature = "androidbox-process0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AndroidAppRpcDirection {
    AppToAndroidApp,
    AndroidAppToApp,
}

#[cfg(feature = "androidbox-process0")]
impl AndroidAppRpcDirection {
    const fn sender_name(self) -> &'static str {
        match self {
            Self::AppToAndroidApp => "app",
            Self::AndroidAppToApp => "android-app",
        }
    }

    const fn receiver_name(self) -> &'static str {
        match self {
            Self::AppToAndroidApp => "android-app",
            Self::AndroidAppToApp => "app",
        }
    }
}

/// Kernel-authenticated evidence for ABI 47's private App ↔ AndroidApp RPC.
///
/// All `*_reads`, byte totals, identities, and request counters below belong
/// to the first completed round and stop changing once `complete` is set.
/// `reads`, validation errors, legal remote Error frames, post-completion
/// traffic, the current phase, and the number of completed rounds remain live.
#[cfg(feature = "androidbox-process0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidAppRpcSnapshot {
    pub reads: u64,
    pub errors: u64,
    pub decode_errors: u64,
    pub authentication_errors: u64,
    pub direction_errors: u64,
    pub protocol_errors: u64,
    pub error_messages: u64,
    pub first_round_errors: u64,
    pub first_round_messages: u64,
    pub post_complete_messages: u64,
    pub requests: u64,
    pub responses: u64,
    pub chunks: u64,
    pub app_to_android_reads: u64,
    pub android_to_app_reads: u64,
    pub ready_reads: u64,
    pub open_reads: u64,
    pub opened_reads: u64,
    pub label_chunk_reads: u64,
    pub button_chunk_reads: u64,
    pub click_reads: u64,
    pub updated_reads: u64,
    pub update_text_chunk_reads: u64,
    pub close_reads: u64,
    pub closed_reads: u64,
    pub error_reads: u64,
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub scene_opened_reads: u64,
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub describe_node_reads: u64,
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub node_reads: u64,
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub node_text_chunk_reads: u64,
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub scene_node_count: u8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub scene_text_bytes: u32,
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub scene_text_view_count: u8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub scene_callback_count: u8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub update_view_id: u32,
    #[cfg(feature = "androidbox-multiaction3")]
    pub callback_button_ids: [u32; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-multiaction3")]
    pub callback_button_text_bytes: [u32; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-multiaction3")]
    pub clicked_button_ids: [u32; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-multiaction3")]
    pub update_view_ids: [u32; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-multiaction3")]
    pub update_text_bytes_by_click: [u32; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-dex-methods8")]
    pub app_defined_call_counts: [u8; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-dex-instance9")]
    pub app_defined_instance_call_counts: [u8; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-activity-fields10")]
    pub activity_field_read_counts: [u8; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-string-text12")]
    pub direct_string_texts: [bool; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-string-builder13")]
    pub dynamic_string_texts: [bool; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-activity-state11")]
    pub activity_int_state_values: [u8; ANDROID_APP_MULTIACTION_CALLBACKS],
    pub app_pid: u64,
    pub android_app_pid: u64,
    pub session_id: u64,
    pub package_generation: u64,
    pub label_id: u32,
    pub button_id: u32,
    pub revision: u32,
    pub label_bytes: u32,
    pub button_bytes: u32,
    pub update_bytes: u32,
    pub last_request_id: u64,
    pub current_last_request_id: u64,
    pub completed_rounds: u64,
    pub phase: u8,
    pub complete: bool,
}

#[cfg(feature = "androidbox-process0")]
#[derive(Clone, Copy)]
struct AndroidAppRpcTraceState {
    phase: AndroidAppRpcPhase,
    reads: u64,
    errors: u64,
    decode_errors: u64,
    authentication_errors: u64,
    direction_errors: u64,
    protocol_errors: u64,
    error_messages: u64,
    first_round_errors: u64,
    first_round_messages: u64,
    post_complete_messages: u64,
    requests: u64,
    responses: u64,
    chunks: u64,
    app_to_android_reads: u64,
    android_to_app_reads: u64,
    ready_reads: u64,
    open_reads: u64,
    opened_reads: u64,
    label_chunk_reads: u64,
    button_chunk_reads: u64,
    click_reads: u64,
    updated_reads: u64,
    update_text_chunk_reads: u64,
    close_reads: u64,
    closed_reads: u64,
    error_reads: u64,
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_opened_reads: u64,
    #[cfg(feature = "androidbox-scene-rpc2")]
    describe_node_reads: u64,
    #[cfg(feature = "androidbox-scene-rpc2")]
    node_reads: u64,
    #[cfg(feature = "androidbox-scene-rpc2")]
    node_text_chunk_reads: u64,
    app_pid: u64,
    android_app_pid: u64,
    first_session_id: u64,
    first_package_generation: u64,
    first_label_id: u32,
    first_button_id: u32,
    first_revision: u32,
    first_label_bytes: u32,
    first_button_bytes: u32,
    first_update_bytes: u32,
    #[cfg(feature = "androidbox-scene-rpc2")]
    first_scene_node_count: u8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    first_scene_text_bytes: u32,
    #[cfg(feature = "androidbox-scene-rpc2")]
    first_scene_text_view_count: u8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    first_scene_callback_count: u8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    first_update_view_id: u32,
    #[cfg(feature = "androidbox-multiaction3")]
    first_callback_button_ids: [u32; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-multiaction3")]
    first_callback_button_text_bytes: [u32; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-multiaction3")]
    first_clicked_button_ids: [u32; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-multiaction3")]
    first_update_view_ids: [u32; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-multiaction3")]
    first_update_text_bytes_by_click: [u32; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-dex-methods8")]
    first_app_defined_call_counts: [u8; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-dex-instance9")]
    first_app_defined_instance_call_counts: [u8; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-activity-fields10")]
    first_activity_field_read_counts: [u8; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-string-text12")]
    first_direct_string_texts: [bool; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-string-builder13")]
    first_dynamic_string_texts: [bool; ANDROID_APP_MULTIACTION_CALLBACKS],
    #[cfg(feature = "androidbox-activity-state11")]
    first_activity_int_state_values: [u8; ANDROID_APP_MULTIACTION_CALLBACKS],
    first_last_request_id: u64,
    completed_rounds: u64,
    session_id: u64,
    package_generation: u64,
    label_id: u32,
    button_id: u32,
    revision: u32,
    last_request_id: u64,
    pending_request_id: u64,
    label_total: u32,
    label_received: u32,
    button_total: u32,
    button_received: u32,
    update_total: u32,
    update_received: u32,
    #[cfg(feature = "androidbox-scene-rpc2")]
    update_view_id: u32,
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_node_count: u8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_next_index: u8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_current_index: u8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_current_id: u32,
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_text_total: u32,
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_text_received: u32,
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_text_bytes: u32,
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_text_view_ids: [u32; ANDROID_APP_SCENE_MAX_NODES],
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_text_view_count: u8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_node_ids: [u32; ANDROID_APP_SCENE_MAX_NODES],
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_node_kinds: [u8; ANDROID_APP_SCENE_MAX_NODES],
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_callback_count: u8,
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_callback_ids: [u32; ANDROID_APP_TRACKED_CALLBACKS],
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_callback_text_bytes: [u32; ANDROID_APP_TRACKED_CALLBACKS],
    first_round_matches: bool,
    first_round_finished: bool,
    complete: bool,
}

#[cfg(feature = "androidbox-process0")]
impl AndroidAppRpcTraceState {
    const fn new() -> Self {
        Self {
            phase: AndroidAppRpcPhase::AwaitReady,
            reads: 0,
            errors: 0,
            decode_errors: 0,
            authentication_errors: 0,
            direction_errors: 0,
            protocol_errors: 0,
            error_messages: 0,
            first_round_errors: 0,
            first_round_messages: 0,
            post_complete_messages: 0,
            requests: 0,
            responses: 0,
            chunks: 0,
            app_to_android_reads: 0,
            android_to_app_reads: 0,
            ready_reads: 0,
            open_reads: 0,
            opened_reads: 0,
            label_chunk_reads: 0,
            button_chunk_reads: 0,
            click_reads: 0,
            updated_reads: 0,
            update_text_chunk_reads: 0,
            close_reads: 0,
            closed_reads: 0,
            error_reads: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_opened_reads: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            describe_node_reads: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            node_reads: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            node_text_chunk_reads: 0,
            app_pid: 0,
            android_app_pid: 0,
            first_session_id: 0,
            first_package_generation: 0,
            first_label_id: 0,
            first_button_id: 0,
            first_revision: 0,
            first_label_bytes: 0,
            first_button_bytes: 0,
            first_update_bytes: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            first_scene_node_count: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            first_scene_text_bytes: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            first_scene_text_view_count: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            first_scene_callback_count: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            first_update_view_id: 0,
            #[cfg(feature = "androidbox-multiaction3")]
            first_callback_button_ids: [0; ANDROID_APP_MULTIACTION_CALLBACKS],
            #[cfg(feature = "androidbox-multiaction3")]
            first_callback_button_text_bytes: [0; ANDROID_APP_MULTIACTION_CALLBACKS],
            #[cfg(feature = "androidbox-multiaction3")]
            first_clicked_button_ids: [0; ANDROID_APP_MULTIACTION_CALLBACKS],
            #[cfg(feature = "androidbox-multiaction3")]
            first_update_view_ids: [0; ANDROID_APP_MULTIACTION_CALLBACKS],
            #[cfg(feature = "androidbox-multiaction3")]
            first_update_text_bytes_by_click: [0; ANDROID_APP_MULTIACTION_CALLBACKS],
            #[cfg(feature = "androidbox-dex-methods8")]
            first_app_defined_call_counts: [0; ANDROID_APP_MULTIACTION_CALLBACKS],
            #[cfg(feature = "androidbox-dex-instance9")]
            first_app_defined_instance_call_counts: [0; ANDROID_APP_MULTIACTION_CALLBACKS],
            #[cfg(feature = "androidbox-activity-fields10")]
            first_activity_field_read_counts: [0; ANDROID_APP_MULTIACTION_CALLBACKS],
            #[cfg(feature = "androidbox-string-text12")]
            first_direct_string_texts: [false; ANDROID_APP_MULTIACTION_CALLBACKS],
            #[cfg(feature = "androidbox-string-builder13")]
            first_dynamic_string_texts: [false; ANDROID_APP_MULTIACTION_CALLBACKS],
            #[cfg(feature = "androidbox-activity-state11")]
            first_activity_int_state_values: [0; ANDROID_APP_MULTIACTION_CALLBACKS],
            first_last_request_id: 0,
            completed_rounds: 0,
            session_id: 0,
            package_generation: 0,
            label_id: 0,
            button_id: 0,
            revision: 0,
            last_request_id: 0,
            pending_request_id: 0,
            label_total: 0,
            label_received: 0,
            button_total: 0,
            button_received: 0,
            update_total: 0,
            update_received: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            update_view_id: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_node_count: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_next_index: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_current_index: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_current_id: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_text_total: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_text_received: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_text_bytes: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_text_view_ids: [0; ANDROID_APP_SCENE_MAX_NODES],
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_text_view_count: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_node_ids: [0; ANDROID_APP_SCENE_MAX_NODES],
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_node_kinds: [0; ANDROID_APP_SCENE_MAX_NODES],
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_callback_count: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_callback_ids: [0; ANDROID_APP_TRACKED_CALLBACKS],
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_callback_text_bytes: [0; ANDROID_APP_TRACKED_CALLBACKS],
            first_round_matches: true,
            first_round_finished: false,
            complete: false,
        }
    }

    fn snapshot(self) -> AndroidAppRpcSnapshot {
        AndroidAppRpcSnapshot {
            reads: self.reads,
            errors: self.errors,
            decode_errors: self.decode_errors,
            authentication_errors: self.authentication_errors,
            direction_errors: self.direction_errors,
            protocol_errors: self.protocol_errors,
            error_messages: self.error_messages,
            first_round_errors: self.first_round_errors,
            first_round_messages: self.first_round_messages,
            post_complete_messages: self.post_complete_messages,
            requests: self.requests,
            responses: self.responses,
            chunks: self.chunks,
            app_to_android_reads: self.app_to_android_reads,
            android_to_app_reads: self.android_to_app_reads,
            ready_reads: self.ready_reads,
            open_reads: self.open_reads,
            opened_reads: self.opened_reads,
            label_chunk_reads: self.label_chunk_reads,
            button_chunk_reads: self.button_chunk_reads,
            click_reads: self.click_reads,
            updated_reads: self.updated_reads,
            update_text_chunk_reads: self.update_text_chunk_reads,
            close_reads: self.close_reads,
            closed_reads: self.closed_reads,
            error_reads: self.error_reads,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_opened_reads: self.scene_opened_reads,
            #[cfg(feature = "androidbox-scene-rpc2")]
            describe_node_reads: self.describe_node_reads,
            #[cfg(feature = "androidbox-scene-rpc2")]
            node_reads: self.node_reads,
            #[cfg(feature = "androidbox-scene-rpc2")]
            node_text_chunk_reads: self.node_text_chunk_reads,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_node_count: self.first_scene_node_count,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_text_bytes: self.first_scene_text_bytes,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_text_view_count: self.first_scene_text_view_count,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_callback_count: self.first_scene_callback_count,
            #[cfg(feature = "androidbox-scene-rpc2")]
            update_view_id: self.first_update_view_id,
            #[cfg(feature = "androidbox-multiaction3")]
            callback_button_ids: self.first_callback_button_ids,
            #[cfg(feature = "androidbox-multiaction3")]
            callback_button_text_bytes: self.first_callback_button_text_bytes,
            #[cfg(feature = "androidbox-multiaction3")]
            clicked_button_ids: self.first_clicked_button_ids,
            #[cfg(feature = "androidbox-multiaction3")]
            update_view_ids: self.first_update_view_ids,
            #[cfg(feature = "androidbox-multiaction3")]
            update_text_bytes_by_click: self.first_update_text_bytes_by_click,
            #[cfg(feature = "androidbox-dex-methods8")]
            app_defined_call_counts: self.first_app_defined_call_counts,
            #[cfg(feature = "androidbox-dex-instance9")]
            app_defined_instance_call_counts: self.first_app_defined_instance_call_counts,
            #[cfg(feature = "androidbox-activity-fields10")]
            activity_field_read_counts: self.first_activity_field_read_counts,
            #[cfg(feature = "androidbox-string-text12")]
            direct_string_texts: self.first_direct_string_texts,
            #[cfg(feature = "androidbox-string-builder13")]
            dynamic_string_texts: self.first_dynamic_string_texts,
            #[cfg(feature = "androidbox-activity-state11")]
            activity_int_state_values: self.first_activity_int_state_values,
            app_pid: self.app_pid,
            android_app_pid: self.android_app_pid,
            session_id: self.first_session_id,
            package_generation: self.first_package_generation,
            label_id: self.first_label_id,
            button_id: self.first_button_id,
            revision: self.first_revision,
            label_bytes: self.first_label_bytes,
            button_bytes: self.first_button_bytes,
            update_bytes: self.first_update_bytes,
            last_request_id: self.first_last_request_id,
            current_last_request_id: self.last_request_id,
            completed_rounds: self.completed_rounds,
            phase: self.phase.raw(),
            complete: self.complete,
        }
    }
}

#[cfg(feature = "androidbox-process0")]
struct AndroidAppRpcTraceStorage(UnsafeCell<AndroidAppRpcTraceState>);

// ABI 47 RPC reads and snapshots use the kernel's single-core IRQ-masked
// process/trace domain, so this cell has exactly one live access at a time.
#[cfg(feature = "androidbox-process0")]
unsafe impl Sync for AndroidAppRpcTraceStorage {}

#[cfg(feature = "androidbox-process0")]
static ANDROID_APP_RPC_TRACE: AndroidAppRpcTraceStorage =
    AndroidAppRpcTraceStorage(UnsafeCell::new(AndroidAppRpcTraceState::new()));

#[cfg(feature = "androidbox-restart0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidAppRestartSnapshot {
    pub errors: u64,
    pub crash_reads: u64,
    pub fault_reaps: u64,
    pub rebind_reads: u64,
    pub replacement_ready_reads: u64,
    pub grant_reissues: u64,
    pub reopen_completions: u64,
    pub reopen_layered_commits: u64,
    pub rebound_reads: u64,
    pub app_pid: u64,
    pub init_pid: u64,
    pub old_worker_pid: u64,
    pub new_worker_pid: u64,
    pub session_id: u64,
    pub package_generation: u64,
    pub crash_request_id: u64,
    pub epoch: u64,
    pub phase: u8,
}

#[cfg(feature = "androidbox-restart0")]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AndroidAppRestartPhase {
    AwaitCrash = 0,
    AwaitRebind = 1,
    AwaitReplacementReady = 2,
    AwaitRebound = 3,
    Complete = 4,
}

#[cfg(feature = "androidbox-restart0")]
#[derive(Clone, Copy)]
struct AndroidAppRestartTraceState {
    errors: u64,
    crash_reads: u64,
    fault_reaps: u64,
    rebind_reads: u64,
    replacement_ready_reads: u64,
    grant_reissues: u64,
    reopen_completions: u64,
    reopen_layered_commits: u64,
    rebound_reads: u64,
    app_pid: u64,
    init_pid: u64,
    old_worker_pid: u64,
    new_worker_pid: u64,
    session_id: u64,
    package_generation: u64,
    crash_request_id: u64,
    epoch: u64,
    phase: AndroidAppRestartPhase,
}

#[cfg(feature = "androidbox-restart0")]
impl AndroidAppRestartTraceState {
    const fn new() -> Self {
        Self {
            errors: 0,
            crash_reads: 0,
            fault_reaps: 0,
            rebind_reads: 0,
            replacement_ready_reads: 0,
            grant_reissues: 0,
            reopen_completions: 0,
            reopen_layered_commits: 0,
            rebound_reads: 0,
            app_pid: 0,
            init_pid: 0,
            old_worker_pid: 0,
            new_worker_pid: 0,
            session_id: 0,
            package_generation: 0,
            crash_request_id: 0,
            epoch: 0,
            phase: AndroidAppRestartPhase::AwaitCrash,
        }
    }

    const fn snapshot(self) -> AndroidAppRestartSnapshot {
        AndroidAppRestartSnapshot {
            errors: self.errors,
            crash_reads: self.crash_reads,
            fault_reaps: self.fault_reaps,
            rebind_reads: self.rebind_reads,
            replacement_ready_reads: self.replacement_ready_reads,
            grant_reissues: self.grant_reissues,
            reopen_completions: self.reopen_completions,
            reopen_layered_commits: self.reopen_layered_commits,
            rebound_reads: self.rebound_reads,
            app_pid: self.app_pid,
            init_pid: self.init_pid,
            old_worker_pid: self.old_worker_pid,
            new_worker_pid: self.new_worker_pid,
            session_id: self.session_id,
            package_generation: self.package_generation,
            crash_request_id: self.crash_request_id,
            epoch: self.epoch,
            phase: self.phase as u8,
        }
    }
}

#[cfg(feature = "androidbox-restart0")]
struct AndroidAppRestartTraceStorage(UnsafeCell<AndroidAppRestartTraceState>);

#[cfg(feature = "androidbox-restart0")]
unsafe impl Sync for AndroidAppRestartTraceStorage {}

#[cfg(feature = "androidbox-restart0")]
static ANDROID_APP_RESTART_TRACE: AndroidAppRestartTraceStorage =
    AndroidAppRestartTraceStorage(UnsafeCell::new(AndroidAppRestartTraceState::new()));

#[derive(Clone, Copy)]
struct UiSupervisorTraceState {
    tracker: UiSupervisorTracker,
    messages: u64,
    errors: u64,
    decode_errors: u64,
    authentication_errors: u64,
    tracker_errors: u64,
    byte_messages: u64,
    transfer_messages: u64,
    commands: u64,
    acks: u64,
    owner_deaths: u64,
    operations: [u64; UI_SUPERVISOR_OPERATION_COUNT],
    transactions: u64,
    completed: u64,
    first_transaction_id: u64,
    last_transaction_id: u64,
    first_sender_pid: u64,
    last_sender_pid: u64,
}

impl UiSupervisorTraceState {
    const fn new() -> Self {
        Self {
            tracker: UiSupervisorTracker::new(),
            messages: 0,
            errors: 0,
            decode_errors: 0,
            authentication_errors: 0,
            tracker_errors: 0,
            byte_messages: 0,
            transfer_messages: 0,
            commands: 0,
            acks: 0,
            owner_deaths: 0,
            operations: [0; UI_SUPERVISOR_OPERATION_COUNT],
            transactions: 0,
            completed: 0,
            first_transaction_id: 0,
            last_transaction_id: 0,
            first_sender_pid: 0,
            last_sender_pid: 0,
        }
    }
}

struct UiSupervisorTraceStorage(UnsafeCell<UiSupervisorTraceState>);

// USC1 committed writes share the same single-core, IRQ-masked lifecycle
// domain as ALC1 and the process table. Consequently this UnsafeCell also has
// exactly one live borrow at a time.
unsafe impl Sync for UiSupervisorTraceStorage {}

static UI_SUPERVISOR_TRACE: UiSupervisorTraceStorage =
    UiSupervisorTraceStorage(UnsafeCell::new(UiSupervisorTraceState::new()));

/// Kernel-validated M15 control transcript. Instance and transaction values
/// remain opaque to the kernel: it validates only their ordering, equality,
/// uniqueness, and source-role relationships.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M15ServiceTranscriptSnapshot {
    pub phase: u8,
    pub errors: u64,
    pub old_instance: u32,
    pub new_instance: u32,
    pub transaction_ids: [u32; M15_SERVICE_TXID_COUNT],
    pub done_reads: u64,
    pub done_bitmap: u8,
}

/// Constant-space, round-indexed post-M15 service-cycle evidence. Only the
/// current or most recently completed round is retained; `completed_rounds`
/// is the monotonic proof that the same five-step recorder was reused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceCycleTranscriptSnapshot {
    pub round: u32,
    pub step: u8,
    pub completed_rounds: u32,
    pub errors: u64,
    pub instance: u32,
    pub transaction_ids: [u32; M16_SERVICE_CYCLE_TXID_COUNT],
    pub done_reads: u64,
    pub done_bitmap: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityTranscriptSnapshot {
    pub identity_done_reads: u64,
    pub acl_done_reads: u64,
    pub errors: u64,
    pub provider_pid: u64,
    pub client_pid: u64,
    pub acl_payload: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MultiClientTranscriptSnapshot {
    pub ready_bitmap: u8,
    pub queued_bitmap: u8,
    pub done_bitmap: u8,
    pub errors: u64,
    pub secondary_client_pid: u64,
    pub request_order: u64,
}

/// Kernel-authenticated evidence for the bounded M20 two-session lifecycle.
/// The phase is advanced only by scalar envelopes that init reads from the
/// exact generation-qualified child startup channel expected at that point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MultiSessionTranscriptSnapshot {
    pub phase: u8,
    pub errors: u64,
    pub secondary_client_pid: u64,
    pub manager_attaches: u8,
    pub client_attaches: u8,
    pub revoke_bitmap: u8,
    pub stale_revoke_bitmap: u8,
    pub primary_progress_bitmap: u8,
    pub provider_accepts: u8,
    pub provider_echoes: u8,
    pub provider_aborts: u8,
    pub secondary_echoes: u8,
    pub final_idle_bitmap: u8,
}

/// Committed-write evidence for the canonical M33 ALC1 protocol.
///
/// `messages` counts only successfully enqueued byte or transfer messages
/// whose first four payload bytes are `ALC1`. Each such message contributes
/// exactly once to either one accepted direction or `errors`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppLifecycleTraceSnapshot {
    pub messages: u64,
    pub errors: u64,
    pub decode_errors: u64,
    pub authentication_errors: u64,
    pub tracker_errors: u64,
    pub byte_messages: u64,
    pub transfer_messages: u64,
    pub requests: u64,
    pub state_changes: u64,
    /// Accepted spontaneous Crashed/ProcessExited state notifications.
    pub crashes: u64,
    pub commands: u64,
    pub acks: u64,
    /// Launch, Activate, Suspend, Resume, Terminate in wire-order.
    pub request_actions: [u64; APP_LIFECYCLE_ACTION_COUNT],
    /// Launch, Activate, Suspend, Resume, Terminate in wire-order.
    pub command_actions: [u64; APP_LIFECYCLE_ACTION_COUNT],
    /// Launch, Activate, Suspend, Resume, Terminate in wire-order.
    pub ack_actions: [u64; APP_LIFECYCLE_ACTION_COUNT],
    pub transactions: u64,
    pub completed: u64,
    pub first_transaction_id: u64,
    pub last_transaction_id: u64,
    pub first_instance_id: u64,
    pub first_app_pid: u64,
    pub last_instance_id: u64,
    pub last_app_pid: u64,
    pub first_sender_pid: u64,
    pub last_sender_pid: u64,
}

/// Committed-write evidence for the canonical M33 USC1 control protocol.
///
/// `messages` counts successfully enqueued byte or transfer messages whose
/// first four payload bytes are `USC1`. `operations` counts accepted commands
/// (not their acknowledgements), so every completed transaction contributes
/// exactly once in Install, Activate, Show, Retire wire-order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiSupervisorTraceSnapshot {
    pub messages: u64,
    pub errors: u64,
    pub decode_errors: u64,
    pub authentication_errors: u64,
    pub tracker_errors: u64,
    pub byte_messages: u64,
    pub transfer_messages: u64,
    pub commands: u64,
    pub acks: u64,
    /// Spontaneous SurfaceServer-to-Init endpoint owner-death notifications.
    pub owner_deaths: u64,
    /// InstallAppEndpoint, ActivateApp, ShowLauncher, RetireAppEndpoint.
    pub operations: [u64; UI_SUPERVISOR_OPERATION_COUNT],
    pub transactions: u64,
    pub completed: u64,
    pub first_transaction_id: u64,
    pub last_transaction_id: u64,
    pub first_sender_pid: u64,
    pub last_sender_pid: u64,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SyscallSnapshot {
    pub calls: u64,
    pub successes: u64,
    pub errors: u64,
    pub unknown: u64,
    pub channels_created: u64,
    pub writes: u64,
    pub reads: u64,
    pub channel_peeks: u64,
    pub channel_peek_scalar: u64,
    pub channel_peek_bytes: u64,
    pub channel_peek_transfer: u64,
    pub channel_envelope_reads: u64,
    pub channel_envelope_scalar: u64,
    pub channel_envelope_bytes: u64,
    pub channel_envelope_transfer: u64,
    pub channel_envelope_preserved_rejections: u64,
    pub channel_envelope_buffer_too_small: u64,
    pub channel_envelope_bad_address: u64,
    pub channel_envelope_table_full: u64,
    pub duplicates: u64,
    pub closes: u64,
    pub stale_rejections: u64,
    pub rights_denials: u64,
    pub last_tag: u64,
    pub last_payload: u64,
    pub ready: bool,
    #[cfg(feature = "storage-server-runtime")]
    pub storage_generation: u64,
    #[cfg(feature = "storage-server-runtime")]
    pub storage_stages: u64,
    pub failure: u64,
    pub pan_failures: u64,
    pub private_svc_rejections: u64,
    pub should_wait_returns: u64,
    pub register_preserved: bool,
    pub stack_round_trip: bool,
    pub user_asid: u8,
    pub byte_writes: u64,
    pub byte_reads: u64,
    pub byte_read_rollbacks: u64,
    pub byte_buffer_too_small: u64,
    pub byte_zero_length: u64,
    pub transfer_writes: u64,
    pub transfer_reads: u64,
    pub transfer_read_rollbacks: u64,
    pub transfer_self_rejections: u64,
    pub transfer_order_rejections: u64,
    pub object_wait_calls: u64,
    pub object_wait_immediate: u64,
    pub wait_many_calls: u64,
    pub wait_many_immediate: u64,
    pub wait_many_poll_timeouts: u64,
    pub wait_array_calls: u64,
    pub wait_array_immediate: u64,
    pub wait_array_poll_timeouts: u64,
    pub wait_array_invalid_counts: u64,
    pub wait_array_bad_addresses: u64,
    pub wait_array_validation_rejections: u64,
    pub wait_array_max_items_observed: u64,
    pub wait_array_lowest_multi_ready: u64,
    pub events_created: u64,
    pub event_signal_calls: u64,
    pub event_signal_edges: u64,
    pub event_clear_calls: u64,
    pub event_clear_edges: u64,
    pub event_signal_denials: u64,
    pub event_wait_calls: u64,
    pub event_wait_blocks: u64,
    pub event_wait_wakes: u64,
    pub event_transfer_writes: u64,
    pub event_transfer_reads: u64,
    pub file_open_calls: u64,
    pub file_open_successes: u64,
    pub vmo_read_calls: u64,
    pub vmo_read_successes: u64,
    pub vmo_read_bytes: u64,
    pub vmo_transfer_writes: u64,
    pub vmo_transfer_reads: u64,
    #[cfg(feature = "unified-product-manifest-supervision-runtime")]
    pub service_manifest_open_calls: u64,
    #[cfg(feature = "unified-product-manifest-supervision-runtime")]
    pub service_manifest_open_successes: u64,
    #[cfg(feature = "unified-product-manifest-supervision-runtime")]
    pub service_manifest_open_argument_rejections: u64,
    #[cfg(feature = "unified-product-manifest-supervision-runtime")]
    pub service_manifest_open_permission_denials: u64,
    #[cfg(feature = "unified-product-verified-manifest-runtime")]
    pub verified_manifest_attempts: u64,
    #[cfg(feature = "unified-product-verified-manifest-runtime")]
    pub verified_manifest_successes: u64,
    #[cfg(feature = "unified-product-verified-manifest-runtime")]
    pub verified_manifest_signature_successes: u64,
    #[cfg(feature = "unified-product-verified-manifest-runtime")]
    pub verified_manifest_signature_rejections: u64,
    #[cfg(feature = "unified-product-verified-manifest-runtime")]
    pub verified_manifest_rollback_rejections: u64,
    #[cfg(feature = "unified-product-verified-manifest-runtime")]
    pub verified_manifest_format_rejections: u64,
    #[cfg(feature = "unified-product-verified-manifest-runtime")]
    pub verified_manifest_rollback_index: u64,
    #[cfg(feature = "unified-product-verified-manifest-runtime")]
    pub verified_manifest_digest: [u64; 4],
    pub graphics_buffers_created: u64,
    pub graphics_buffer_create_exhaustions: u64,
    pub graphics_buffer_write_calls: u64,
    pub graphics_buffer_write_bytes: u64,
    pub graphics_buffer_presents: u64,
    pub graphics_buffer_mappable_created: u64,
    pub graphics_buffer_map_calls: u64,
    pub graphics_buffer_map_successes: u64,
    pub graphics_buffer_unmap_calls: u64,
    pub graphics_buffer_unmap_successes: u64,
    pub graphics_buffer_queue_calls: u64,
    pub graphics_buffer_queue_successes: u64,
    pub graphics_buffer_acquire_calls: u64,
    pub graphics_buffer_acquire_successes: u64,
    pub graphics_buffer_release_calls: u64,
    pub graphics_buffer_release_successes: u64,
    pub graphics_buffer_releases: u64,
    pub graphics_buffer_mapped_presents: u64,
    pub graphics_buffer_validated_pixels: u64,
    pub graphics_buffer_validated_bytes: u64,
    pub surface_reacquires: u64,
    pub surface_reacquire_old_pid: u64,
    pub surface_reacquire_new_pid: u64,
    pub surface_reacquire_old_session: u64,
    pub surface_reacquire_new_session: u64,
    pub surface_reacquire_discarded_input: u64,
    pub surface_reacquire_discarded_keys: u64,
    pub surface_reacquire_frozen_cursor_x: u64,
    pub surface_reacquire_frozen_cursor_y: u64,
    pub surface_reacquire_frozen_cursor_pixels: u64,
    pub surface_reacquire_frozen_cursor_visible: bool,
    pub surface_reacquire_frozen_cursor_pressed: bool,
    pub surface_reacquire_frozen_scanout_valid: bool,
    pub surface_frame_acquire_calls: u64,
    pub surface_frame_acquire_successes: u64,
    pub surface_frame_acquire_should_wait: u64,
    pub surface_frame_acquire_invalid_state: u64,
    pub surface_key_read_calls: u64,
    pub surface_key_read_successes: u64,
    pub surface_key_read_should_wait: u64,
    pub surface_key_read_peer_closed: u64,
    pub surface_key_read_invalid_argument: u64,
    pub surface_key_read_permission_denied: u64,
    pub surface_key_read_invalid_state: u64,
    pub input_acquire_calls: u64,
    pub input_acquire_successes: u64,
    pub input_acquire_permission_denied: u64,
    pub input_session_info_calls: u64,
    pub input_session_info_successes: u64,
    pub input_session_info_permission_denied: u64,
    pub process_terminate_calls: u64,
    pub process_terminate_successes: u64,
    pub process_terminate_not_found: u64,
    pub process_terminate_invalid_argument: u64,
    pub process_terminate_invalid_state: u64,
    pub process_terminate_permission_denied: u64,
    pub app_lifecycle_trace: AppLifecycleTraceSnapshot,
    pub ui_supervisor_trace: UiSupervisorTraceSnapshot,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    pub service_ack_reads: u64,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    pub service_ack_bitmap: u64,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    pub service_done_reads: u64,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    pub service_done_bitmap: u64,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    pub service_transcript_errors: u64,
    #[allow(dead_code)]
    pub m15_service_transcript: M15ServiceTranscriptSnapshot,
    #[allow(dead_code)]
    pub service_cycle_transcript: ServiceCycleTranscriptSnapshot,
    #[allow(dead_code)]
    pub identity_transcript: IdentityTranscriptSnapshot,
    #[allow(dead_code)]
    pub multi_client_transcript: MultiClientTranscriptSnapshot,
    #[allow(dead_code)]
    pub multi_session_transcript: MultiSessionTranscriptSnapshot,
    pub handles_left: usize,
    pub heap_free_baseline: usize,
    pub heap_free_now: usize,
}

#[cfg(feature = "app-data-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppDataSyscallSnapshot {
    pub root_calls: u64,
    pub root_successes: u64,
    pub file_open_calls: u64,
    pub file_open_successes: u64,
    pub replace_calls: u64,
    pub replace_successes: u64,
    pub directory_create_calls: u64,
    pub directory_create_successes: u64,
    pub unlink_calls: u64,
    pub unlink_successes: u64,
    pub directory_read_calls: u64,
    pub directory_read_successes: u64,
    pub directory_read_eof: u64,
    pub permission_denials: u64,
    pub validation_rejections: u64,
    pub should_waits: u64,
    pub conflicts: u64,
    pub outcome_unknown: u64,
    pub corruption_errors: u64,
    pub no_space: u64,
}

#[cfg(feature = "unified-product-persistent-rollback-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistentManifestPrepareError {
    SyscallsAlreadyInitialized,
    IrqMasked,
    AlreadyPrepared,
    Signed(bndr_sm::verified_manifest::SignedManifestError),
    UnsupportedManifestGeneration,
    Ledger(crate::storage_persist::StoragePersistError),
}

#[cfg(feature = "unified-product-persistent-rollback-runtime")]
impl PersistentManifestPrepareError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SyscallsAlreadyInitialized => {
                "persistent manifest preparation must finish before syscall activation"
            }
            Self::IrqMasked => {
                "persistent manifest preparation requires the pre-EL0 IRQ-enabled monitor"
            }
            Self::AlreadyPrepared => "persistent manifest preparation was attempted twice",
            Self::Signed(_) => "persistent manifest signature or envelope validation failed",
            Self::UnsupportedManifestGeneration => {
                "persistent manifest generation is not supported by this product image"
            }
            Self::Ledger(error) => error.as_str(),
        }
    }
}

#[cfg(feature = "unified-product-persistent-rollback-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PersistentManifestRollbackSnapshot {
    pub prepared: bool,
    pub ledger_attempts: u64,
    pub ledger_successes: u64,
    pub ledger_rollback_rejections: u64,
    pub ledger_failures: u64,
    pub evidence: Option<bndroid_kernel::persist::ManifestRollbackEvidence>,
}

/// Verifies M75's complete BMS1 artifact and commits its rollback index before
/// EL0 can request the immutable manifest VMO.
///
/// `process::init` has reset process ownership, but `userboot::start` has not
/// yet initialized syscalls or made any EL0 context runnable. Running here
/// lets the bounded virtio-blk transaction receive hard IRQ completions
/// without ever unmasking interrupts inside an SVC handler. The later syscall
/// initialization recognizes and preserves this one-time preparation record.
#[cfg(feature = "unified-product-persistent-rollback-runtime")]
pub fn prepare_persistent_verified_manifest(
    counter_frequency: u64,
) -> Result<bndroid_kernel::persist::ManifestRollbackEvidence, PersistentManifestPrepareError> {
    use bndr_sm::manifest::PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION;
    use bndr_sm::verified_manifest::{
        PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR, PERSISTENT_MANIFEST_RSA2048_KEY2,
        SignedManifestError, sha256, verify_signed_service_manifest,
    };
    use bndroid_kernel::persist::{ManifestRollbackBinding, ManifestRollbackError};

    if INITIALIZED.load(Ordering::Acquire) {
        return Err(PersistentManifestPrepareError::SyscallsAlreadyInitialized);
    }
    if crate::arch::aarch64::irq_is_masked() {
        return Err(PersistentManifestPrepareError::IrqMasked);
    }
    if PERSISTENT_MANIFEST_PREPARATION.snapshot().is_some() {
        return Err(PersistentManifestPrepareError::AlreadyPrepared);
    }

    VERIFIED_MANIFEST_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    let verified = match verify_signed_service_manifest(
        VERIFIED_MANIFEST_ARTIFACT,
        &PERSISTENT_MANIFEST_RSA2048_KEY2,
        PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
    ) {
        Ok(verified) => verified,
        Err(error @ SignedManifestError::Signature) => {
            VERIFIED_MANIFEST_SIGNATURE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "PERSISTENT_ROLLBACK_REJECTED format=1 reason=signature key_id=2 bootstrap_floor=2 signature_valid=0 manifest_published=0 init_ready=0"
            );
            return Err(PersistentManifestPrepareError::Signed(error));
        }
        Err(error @ SignedManifestError::Rollback { actual, minimum }) => {
            VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            VERIFIED_MANIFEST_ROLLBACK_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "PERSISTENT_ROLLBACK_REJECTED format=1 reason=bootstrap-rollback key_id=2 artifact_index={} bootstrap_floor={} signature_valid=1 manifest_published=0 init_ready=0",
                actual,
                minimum,
            );
            return Err(PersistentManifestPrepareError::Signed(error));
        }
        Err(
            error @ (SignedManifestError::Manifest(_)
            | SignedManifestError::GenerationMismatch { .. }),
        ) => {
            VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            VERIFIED_MANIFEST_FORMAT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "PERSISTENT_ROLLBACK_REJECTED format=1 reason=payload key_id=2 bootstrap_floor=2 signature_valid=1 manifest_published=0 init_ready=0"
            );
            return Err(PersistentManifestPrepareError::Signed(error));
        }
        Err(error) => {
            VERIFIED_MANIFEST_FORMAT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "PERSISTENT_ROLLBACK_REJECTED format=1 reason=envelope key_id=2 bootstrap_floor=2 signature_valid=0 manifest_published=0 init_ready=0"
            );
            return Err(PersistentManifestPrepareError::Signed(error));
        }
    };
    VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.fetch_add(1, Ordering::Relaxed);

    // Generation two is intentionally admitted only as the signed rollback
    // fixture. Any other unexpected generation is rejected before it can
    // raise the durable floor and strand this product image.
    if !matches!(
        verified.manifest().generation(),
        2 | PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION
    ) {
        VERIFIED_MANIFEST_FORMAT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        crate::kprintln!(
            "PERSISTENT_ROLLBACK_REJECTED format=1 reason=unsupported-generation key_id=2 artifact_index={} bootstrap_floor=2 signature_valid=1 manifest_published=0 init_ready=0",
            verified.rollback_index(),
        );
        return Err(PersistentManifestPrepareError::UnsupportedManifestGeneration);
    }

    let binding = ManifestRollbackBinding {
        key_id: PERSISTENT_MANIFEST_RSA2048_KEY2.key_id(),
        trust_anchor_sha256: sha256(PERSISTENT_MANIFEST_RSA2048_KEY2.modulus()),
    };
    PERSISTENT_MANIFEST_LEDGER_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    let evidence = match crate::storage_persist::enforce_verified_manifest_rollback(
        counter_frequency,
        crate::storage::DATA_PARTITION_FIRST_LBA,
        crate::storage::DATA_PARTITION_SECTORS,
        crate::storage::DATA_PARTITION_FORMAT_EPOCH,
        PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
        verified.rollback_index(),
        binding,
    ) {
        Ok(evidence) => evidence,
        Err(
            error @ crate::storage_persist::StoragePersistError::ManifestRollbackTransaction(
                ManifestRollbackError::Rollback { actual, minimum },
            ),
        ) => {
            PERSISTENT_MANIFEST_LEDGER_ROLLBACK_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            VERIFIED_MANIFEST_ROLLBACK_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "PERSISTENT_ROLLBACK_REJECTED format=1 reason=rollback key_id=2 artifact_index={} persisted_floor={} signature_valid=1 manifest_published=0 init_ready=0",
                actual,
                minimum,
            );
            return Err(PersistentManifestPrepareError::Ledger(error));
        }
        Err(error) => {
            PERSISTENT_MANIFEST_LEDGER_FAILURES.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "PERSISTENT_ROLLBACK_REJECTED format=1 reason=ledger key_id=2 artifact_index={} bootstrap_floor=2 signature_valid=1 manifest_published=0 init_ready=0",
                verified.rollback_index(),
            );
            crate::kprintln!("PERSISTENT_ROLLBACK_LEDGER_DIAG error={}", error.as_str());
            return Err(PersistentManifestPrepareError::Ledger(error));
        }
    };

    if verified.manifest().generation() != PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION {
        VERIFIED_MANIFEST_FORMAT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        crate::kprintln!(
            "PERSISTENT_ROLLBACK_REJECTED format=1 reason=product-generation key_id=2 artifact_index={} committed_floor={} signature_valid=1 manifest_published=0 init_ready=0",
            verified.rollback_index(),
            evidence.committed_floor,
        );
        return Err(PersistentManifestPrepareError::UnsupportedManifestGeneration);
    }

    PERSISTENT_MANIFEST_LEDGER_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    VERIFIED_MANIFEST_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    VERIFIED_MANIFEST_ROLLBACK_INDEX.store(u64::from(verified.rollback_index()), Ordering::Release);
    store_verified_manifest_digest(verified.signed_region_sha256());
    PERSISTENT_MANIFEST_PREPARATION.publish(evidence);
    crate::kprintln!(
        "PERSISTENT_ROLLBACK_LEDGER_OK format=1 state_version=1 authority=kernel-pre-el0-qemu-disk slots=2 relative_lbas=3/4 key_id={} bootstrap_floor={} provisioned_before={} persisted_floor_before={} effective_floor={} artifact_index={} committed_floor={} initial_generation={} committed_generation={} initial_slot={} committed_slot={} initial_valid_slots={} initial_rejected_slots={} committed_valid_slots={} committed_rejected_slots={} floor_advanced={} record_written={} redundancy_repaired={} reads={} writes={} flushes={} write_flush_readback=1 manifest_published=0 init_ready=0 host_rollback_resistance=0 erase_resistance=0 rpmb=0 efuse=0 hardware_powercut_claim=0 emulator_only=1",
        evidence.binding.key_id,
        evidence.bootstrap_floor,
        u8::from(evidence.provisioned_before),
        evidence.persisted_floor_before,
        evidence.effective_floor,
        evidence.artifact_index,
        evidence.committed_floor,
        evidence.initial_generation,
        evidence.committed_generation,
        evidence.initial_slot,
        evidence.committed_slot,
        evidence.initial_valid_slots,
        evidence.initial_rejected_slots,
        evidence.committed_valid_slots,
        evidence.committed_rejected_slots,
        u8::from(evidence.floor_advanced),
        u8::from(evidence.record_written),
        u8::from(evidence.redundancy_repaired),
        evidence.reads,
        evidence.writes,
        evidence.flushes,
    );
    Ok(evidence)
}

#[cfg(feature = "unified-product-persistent-rollback-runtime")]
pub fn persistent_manifest_rollback_snapshot() -> PersistentManifestRollbackSnapshot {
    let evidence = PERSISTENT_MANIFEST_PREPARATION.snapshot();
    PersistentManifestRollbackSnapshot {
        prepared: evidence.is_some(),
        ledger_attempts: PERSISTENT_MANIFEST_LEDGER_ATTEMPTS.load(Ordering::Acquire),
        ledger_successes: PERSISTENT_MANIFEST_LEDGER_SUCCESSES.load(Ordering::Acquire),
        ledger_rollback_rejections: PERSISTENT_MANIFEST_LEDGER_ROLLBACK_REJECTIONS
            .load(Ordering::Acquire),
        ledger_failures: PERSISTENT_MANIFEST_LEDGER_FAILURES.load(Ordering::Acquire),
        evidence,
    }
}

#[cfg(feature = "unified-product-key-rotation-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyRotationManifestSnapshot {
    pub prepared: bool,
    pub ledger_attempts: u64,
    pub ledger_successes: u64,
    pub ledger_rollback_rejections: u64,
    pub ledger_failures: u64,
    pub retired_key_rejections: u64,
    pub evidence: Option<bndroid_kernel::persist::ManifestKeyRotationEvidence>,
}

/// Verifies an M76 BMS1 with the ordered public keyring, then commits its key
/// epoch and rollback floor before any EL0 context can run.
#[cfg(feature = "unified-product-key-rotation-runtime")]
pub fn prepare_key_rotation_verified_manifest(
    counter_frequency: u64,
) -> Result<bndroid_kernel::persist::ManifestKeyRotationEvidence, PersistentManifestPrepareError> {
    use bndr_sm::manifest::{
        KEY_ROTATION_RETIRED_KEY_FIXTURE_GENERATION, KEY_ROTATION_SERVICE_MANIFEST_GENERATION,
        KEY_ROTATION_TRANSITION_SERVICE_MANIFEST_GENERATION,
    };
    use bndr_sm::verified_manifest::{
        KEY_ROTATION_ACTIVE_KEY_EPOCH, KEY_ROTATION_ACTIVE_KEY_ID, KEY_ROTATION_MANIFEST_KEYRING,
        KEY_ROTATION_POLICY_SHA256, KEY_ROTATION_TRANSITION_KEY_EPOCH,
        KEY_ROTATION_TRANSITION_KEY_ID, PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
        PERSISTENT_MANIFEST_RSA2048_KEY2, SignedManifestError, rsa2048_keyring_sha256, sha256,
        verify_signed_service_manifest_with_keyring,
    };
    use bndroid_kernel::persist::{
        ManifestKeyPolicyBinding, ManifestKeyRotationError, ManifestKeyRotationRequest,
        ManifestRollbackBinding,
    };

    if INITIALIZED.load(Ordering::Acquire) {
        return Err(PersistentManifestPrepareError::SyscallsAlreadyInitialized);
    }
    if crate::arch::aarch64::irq_is_masked() {
        return Err(PersistentManifestPrepareError::IrqMasked);
    }
    if KEY_ROTATION_MANIFEST_PREPARATION.snapshot().is_some() {
        return Err(PersistentManifestPrepareError::AlreadyPrepared);
    }

    let artifact_key_id = VERIFIED_MANIFEST_ARTIFACT[6];
    VERIFIED_MANIFEST_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    let verified = match verify_signed_service_manifest_with_keyring(
        VERIFIED_MANIFEST_ARTIFACT,
        &KEY_ROTATION_MANIFEST_KEYRING,
        PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
    ) {
        Ok(verified) => verified,
        Err(error @ SignedManifestError::Signature) => {
            VERIFIED_MANIFEST_SIGNATURE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "KEY_ROTATION_REJECTED format=1 reason=signature key_id={} signature_valid=0 manifest_published=0 init_ready=0",
                artifact_key_id,
            );
            return Err(PersistentManifestPrepareError::Signed(error));
        }
        Err(error @ SignedManifestError::Rollback { actual, minimum }) => {
            VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            VERIFIED_MANIFEST_ROLLBACK_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "KEY_ROTATION_REJECTED format=1 reason=bootstrap-rollback key_id={} artifact_index={} bootstrap_floor={} signature_valid=1 manifest_published=0 init_ready=0",
                artifact_key_id,
                actual,
                minimum,
            );
            return Err(PersistentManifestPrepareError::Signed(error));
        }
        Err(
            error @ (SignedManifestError::Manifest(_)
            | SignedManifestError::GenerationMismatch { .. }),
        ) => {
            VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            VERIFIED_MANIFEST_FORMAT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "KEY_ROTATION_REJECTED format=1 reason=payload key_id={} signature_valid=1 manifest_published=0 init_ready=0",
                artifact_key_id,
            );
            return Err(PersistentManifestPrepareError::Signed(error));
        }
        Err(error) => {
            VERIFIED_MANIFEST_FORMAT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "KEY_ROTATION_REJECTED format=1 reason=envelope key_id={} signature_valid=0 manifest_published=0 init_ready=0",
                artifact_key_id,
            );
            return Err(PersistentManifestPrepareError::Signed(error));
        }
    };
    VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.fetch_add(1, Ordering::Relaxed);

    let supported_product = matches!(
        (verified.key_id(), verified.manifest().generation()),
        (
            KEY_ROTATION_TRANSITION_KEY_ID,
            KEY_ROTATION_TRANSITION_SERVICE_MANIFEST_GENERATION
                | KEY_ROTATION_RETIRED_KEY_FIXTURE_GENERATION
        ) | (
            KEY_ROTATION_ACTIVE_KEY_ID,
            KEY_ROTATION_SERVICE_MANIFEST_GENERATION
        )
    );
    if !supported_product {
        VERIFIED_MANIFEST_FORMAT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        crate::kprintln!(
            "KEY_ROTATION_REJECTED format=1 reason=unsupported-key-generation key_id={} artifact_index={} signature_valid=1 manifest_published=0 init_ready=0",
            verified.key_id(),
            verified.rollback_index(),
        );
        return Err(PersistentManifestPrepareError::UnsupportedManifestGeneration);
    }

    let key_epoch = match verified.key_id() {
        KEY_ROTATION_TRANSITION_KEY_ID => KEY_ROTATION_TRANSITION_KEY_EPOCH,
        KEY_ROTATION_ACTIVE_KEY_ID => KEY_ROTATION_ACTIVE_KEY_EPOCH,
        _ => return Err(PersistentManifestPrepareError::UnsupportedManifestGeneration),
    };
    let selected_key = KEY_ROTATION_MANIFEST_KEYRING
        .iter()
        .find(|key| key.key_id() == verified.key_id())
        .expect("verified keyring artifact lost its selected public key");
    let policy_sha256 = rsa2048_keyring_sha256(&KEY_ROTATION_MANIFEST_KEYRING)
        .map_err(PersistentManifestPrepareError::Signed)?;
    if policy_sha256 != KEY_ROTATION_POLICY_SHA256 {
        panic!("compiled M76 keyring digest changed without a policy update");
    }
    let binding = ManifestKeyPolicyBinding {
        key_id: verified.key_id(),
        key_epoch,
        trust_anchor_sha256: sha256(selected_key.modulus()),
        policy_sha256,
    };
    let legacy_predecessor = ManifestRollbackBinding {
        key_id: PERSISTENT_MANIFEST_RSA2048_KEY2.key_id(),
        trust_anchor_sha256: sha256(PERSISTENT_MANIFEST_RSA2048_KEY2.modulus()),
    };

    PERSISTENT_MANIFEST_LEDGER_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    let evidence = match crate::storage_persist::enforce_verified_manifest_key_rotation(
        counter_frequency,
        ManifestKeyRotationRequest {
            partition_first_lba: crate::storage::DATA_PARTITION_FIRST_LBA,
            partition_sectors: crate::storage::DATA_PARTITION_SECTORS,
            expected_format_epoch: crate::storage::DATA_PARTITION_FORMAT_EPOCH,
            bootstrap_floor: PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
            artifact_index: verified.rollback_index(),
            artifact_binding: binding,
            legacy_predecessor,
            legacy_predecessor_epoch: 2,
        },
    ) {
        Ok(evidence) => evidence,
        Err(
            error @ crate::storage_persist::StoragePersistError::ManifestKeyRotationTransaction(
                ManifestKeyRotationError::RetiredKey {
                    artifact_epoch,
                    active_epoch,
                },
            ),
        ) => {
            KEY_ROTATION_RETIRED_KEY_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "KEY_ROTATION_REJECTED format=1 reason=retired-key key_id={} key_epoch={} active_epoch={} artifact_index={} signature_valid=1 manifest_published=0 init_ready=0",
                verified.key_id(),
                artifact_epoch,
                active_epoch,
                verified.rollback_index(),
            );
            return Err(PersistentManifestPrepareError::Ledger(error));
        }
        Err(
            error @ crate::storage_persist::StoragePersistError::ManifestKeyRotationTransaction(
                ManifestKeyRotationError::Rollback { actual, minimum },
            ),
        ) => {
            PERSISTENT_MANIFEST_LEDGER_ROLLBACK_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            VERIFIED_MANIFEST_ROLLBACK_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "KEY_ROTATION_REJECTED format=1 reason=rollback key_id={} artifact_index={} persisted_floor={} signature_valid=1 manifest_published=0 init_ready=0",
                verified.key_id(),
                actual,
                minimum,
            );
            return Err(PersistentManifestPrepareError::Ledger(error));
        }
        Err(
            error @ crate::storage_persist::StoragePersistError::ManifestKeyRotationTransaction(
                ManifestKeyRotationError::SkippedKeyEpoch {
                    artifact_epoch,
                    active_epoch,
                },
            ),
        ) => {
            PERSISTENT_MANIFEST_LEDGER_FAILURES.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "KEY_ROTATION_REJECTED format=1 reason=skipped-key-epoch key_id={} key_epoch={} active_epoch={} artifact_index={} signature_valid=1 manifest_published=0 init_ready=0",
                verified.key_id(),
                artifact_epoch,
                active_epoch,
                verified.rollback_index(),
            );
            return Err(PersistentManifestPrepareError::Ledger(error));
        }
        Err(error) => {
            PERSISTENT_MANIFEST_LEDGER_FAILURES.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "KEY_ROTATION_REJECTED format=1 reason=ledger key_id={} artifact_index={} signature_valid=1 manifest_published=0 init_ready=0",
                verified.key_id(),
                verified.rollback_index(),
            );
            crate::kprintln!("KEY_ROTATION_LEDGER_DIAG error={}", error.as_str());
            return Err(PersistentManifestPrepareError::Ledger(error));
        }
    };

    PERSISTENT_MANIFEST_LEDGER_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    VERIFIED_MANIFEST_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    VERIFIED_MANIFEST_ROLLBACK_INDEX.store(u64::from(verified.rollback_index()), Ordering::Release);
    store_verified_manifest_digest(verified.signed_region_sha256());
    KEY_ROTATION_MANIFEST_PREPARATION.publish(evidence);
    crate::kprintln!(
        "KEY_ROTATION_POLICY_OK format=1 state=BNDRKEY1 state_version=1 authority=kernel-pre-el0-qemu-disk keyring_keys=3 policy_sha256=30676f357683b6c8d2c5d67755e48beb93bdfe8864eccc67c716a1785e935e01 previous_key_id={} previous_key_epoch={} active_key_id={} active_key_epoch={} legacy_migrated={} key_transition={} bootstrap_floor={} persisted_floor_before={} effective_floor={} artifact_index={} committed_floor={} initial_generation={} committed_generation={} initial_slot={} committed_slot={} initial_valid_slots={} initial_rejected_slots={} initial_active_policy_slots={} committed_active_policy_slots={} floor_advanced={} record_written={} redundancy_repaired={} reads={} writes={} flushes={} manifest_published=0 init_ready=0 staged_upgrade_required=1 fixture_keys=1 production_key_claim=0 hsm_claim=0 rpmb_claim=0 efuse_claim=0 host_rollback_resistance=0 hardware_powercut_claim=0 emulator_only=1",
        evidence.previous_key_id,
        evidence.previous_key_epoch,
        evidence.active_key_id,
        evidence.active_key_epoch,
        u8::from(evidence.legacy_migrated),
        u8::from(evidence.key_transition),
        evidence.bootstrap_floor,
        evidence.persisted_floor_before,
        evidence.effective_floor,
        evidence.artifact_index,
        evidence.committed_floor,
        evidence.initial_generation,
        evidence.committed_generation,
        evidence.initial_slot,
        evidence.committed_slot,
        evidence.initial_valid_slots,
        evidence.initial_rejected_slots,
        evidence.initial_active_policy_slots,
        evidence.committed_active_policy_slots,
        u8::from(evidence.floor_advanced),
        u8::from(evidence.record_written),
        u8::from(evidence.redundancy_repaired),
        evidence.reads,
        evidence.writes,
        evidence.flushes,
    );
    Ok(evidence)
}

#[cfg(feature = "unified-product-key-rotation-runtime")]
pub fn key_rotation_manifest_snapshot() -> KeyRotationManifestSnapshot {
    let evidence = KEY_ROTATION_MANIFEST_PREPARATION.snapshot();
    KeyRotationManifestSnapshot {
        prepared: evidence.is_some(),
        ledger_attempts: PERSISTENT_MANIFEST_LEDGER_ATTEMPTS.load(Ordering::Acquire),
        ledger_successes: PERSISTENT_MANIFEST_LEDGER_SUCCESSES.load(Ordering::Acquire),
        ledger_rollback_rejections: PERSISTENT_MANIFEST_LEDGER_ROLLBACK_REJECTIONS
            .load(Ordering::Acquire),
        ledger_failures: PERSISTENT_MANIFEST_LEDGER_FAILURES.load(Ordering::Acquire),
        retired_key_rejections: KEY_ROTATION_RETIRED_KEY_REJECTIONS.load(Ordering::Acquire),
        evidence,
    }
}

#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenanceAuthorizationPrepareError {
    SyscallsAlreadyInitialized,
    IrqMasked,
    ManifestPrerequisite,
    AlreadyPrepared,
    Authorization(bndr_sm::maintenance_authorization::MaintenanceAuthorizationError),
    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    SignedPlan(bndr_sm::maintenance_plan::MaintenancePlanError),
    Audit(crate::storage_persist::StoragePersistError),
}

#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
impl MaintenanceAuthorizationPrepareError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SyscallsAlreadyInitialized => {
                "maintenance authorization must be prepared before syscall activation"
            }
            Self::IrqMasked => {
                "maintenance audit preparation requires the pre-EL0 IRQ-enabled monitor"
            }
            Self::ManifestPrerequisite => {
                "maintenance authorization does not match the prepared product manifest policy"
            }
            Self::AlreadyPrepared => "maintenance authorization was prepared twice",
            Self::Authorization(_) => {
                "maintenance authorization signature, envelope, or binding validation failed"
            }
            #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
            Self::SignedPlan(_) => {
                "signed maintenance-plan signature, binding, or program validation failed"
            }
            Self::Audit(error) => error.as_str(),
        }
    }
}

#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceAuthorizationSnapshot {
    pub prepared: bool,
    pub authorization_attempts: u64,
    pub authorization_successes: u64,
    pub signature_rejections: u64,
    pub binding_rejections: u64,
    pub audit_attempts: u64,
    pub audit_successes: u64,
    pub audit_replay_rejections: u64,
    pub audit_failures: u64,
    pub session_open_calls: u64,
    pub session_open_successes: u64,
    pub session_open_argument_rejections: u64,
    pub session_open_permission_rejections: u64,
    pub session_open_state_rejections: u64,
    pub session_opened: bool,
    pub report_gate_denials: u64,
    pub evidence: Option<bndroid_kernel::persist::MaintenanceAuditEvidence>,
    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    pub plan_evidence: Option<bndroid_kernel::persist::MaintenancePlanAdmissionEvidence>,
}

/// Verifies one M77 BMA1 against a trust root separate from the product
/// manifest keyring, then consumes its exact sequence in the durable audit
/// chain before any EL0 context can run.
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
pub fn prepare_maintenance_authorization(
    counter_frequency: u64,
) -> Result<bndroid_kernel::persist::MaintenanceAuditEvidence, MaintenanceAuthorizationPrepareError>
{
    use bndr_sm::maintenance_authorization::{
        FIXTURE_MAINTENANCE_BINDING, MAINTENANCE_AUTHORIZATION_ROOT_SHA256,
        MAINTENANCE_AUTHORIZATION_RSA2048_KEY1, MAINTENANCE_DEVICE_BINDING_SHA256,
        MAINTENANCE_POLICY_SHA256, MAINTENANCE_PRODUCT_KEY_POLICY_SHA256,
        MAINTENANCE_PRODUCT_MANIFEST_SHA256, MaintenanceAuthorizationError,
        verify_maintenance_authorization,
    };
    use bndr_sm::verified_manifest::{
        KEY_ROTATION_ACTIVE_KEY_EPOCH, KEY_ROTATION_ACTIVE_KEY_ID, KEY_ROTATION_POLICY_SHA256,
        sha256,
    };
    #[cfg(not(feature = "unified-product-maintenance-execution-runtime"))]
    use bndroid_kernel::persist::MaintenanceAuditError;
    #[cfg(feature = "unified-product-maintenance-execution-runtime")]
    use bndroid_kernel::persist::MaintenanceExecutionError;
    use bndroid_kernel::persist::{MaintenanceAuditRequest, MaintenanceAuthorizationAuditBinding};

    if INITIALIZED.load(Ordering::Acquire) {
        return Err(MaintenanceAuthorizationPrepareError::SyscallsAlreadyInitialized);
    }
    if crate::arch::aarch64::irq_is_masked() {
        return Err(MaintenanceAuthorizationPrepareError::IrqMasked);
    }
    if MAINTENANCE_AUTHORIZATION_PREPARATION.snapshot().is_some() {
        return Err(MaintenanceAuthorizationPrepareError::AlreadyPrepared);
    }

    let rotation = KEY_ROTATION_MANIFEST_PREPARATION
        .snapshot()
        .ok_or(MaintenanceAuthorizationPrepareError::ManifestPrerequisite)?;
    let manifest_digest_words = sha256_words(&MAINTENANCE_PRODUCT_MANIFEST_SHA256);
    let stored_manifest_digest_words = [
        VERIFIED_MANIFEST_DIGEST_0.load(Ordering::Acquire),
        VERIFIED_MANIFEST_DIGEST_1.load(Ordering::Acquire),
        VERIFIED_MANIFEST_DIGEST_2.load(Ordering::Acquire),
        VERIFIED_MANIFEST_DIGEST_3.load(Ordering::Acquire),
    ];
    if rotation.active_key_id != KEY_ROTATION_ACTIVE_KEY_ID
        || rotation.active_key_epoch != KEY_ROTATION_ACTIVE_KEY_EPOCH
        || rotation.artifact_index != FIXTURE_MAINTENANCE_BINDING.manifest_generation
        || rotation.committed_floor < rotation.artifact_index
        || rotation.binding.policy_sha256 != KEY_ROTATION_POLICY_SHA256
        || MAINTENANCE_PRODUCT_KEY_POLICY_SHA256 != KEY_ROTATION_POLICY_SHA256
        || stored_manifest_digest_words != manifest_digest_words
        || sha256(MAINTENANCE_AUTHORIZATION_RSA2048_KEY1.modulus())
            != MAINTENANCE_AUTHORIZATION_ROOT_SHA256
    {
        return Err(MaintenanceAuthorizationPrepareError::ManifestPrerequisite);
    }

    MAINTENANCE_AUTHORIZATION_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    let verified = match verify_maintenance_authorization(
        MAINTENANCE_AUTHORIZATION_ARTIFACT,
        &MAINTENANCE_AUTHORIZATION_RSA2048_KEY1,
        FIXTURE_MAINTENANCE_BINDING,
    ) {
        Ok(verified) => verified,
        Err(error @ MaintenanceAuthorizationError::Signature) => {
            MAINTENANCE_AUTHORIZATION_SIGNATURE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "MAINTENANCE_AUTHORIZATION_REJECTED format=1 reason=signature signature_valid=0 binding_valid=0 audit_attempted=0 audit_mutations=0 manifest_published=0 init_ready=0"
            );
            return Err(MaintenanceAuthorizationPrepareError::Authorization(error));
        }
        Err(
            error @ (MaintenanceAuthorizationError::Binding
            | MaintenanceAuthorizationError::AuthorizationId),
        ) => {
            MAINTENANCE_AUTHORIZATION_BINDING_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "MAINTENANCE_AUTHORIZATION_REJECTED format=1 reason=binding signature_valid=1 binding_valid=0 audit_attempted=0 audit_mutations=0 manifest_published=0 init_ready=0"
            );
            return Err(MaintenanceAuthorizationPrepareError::Authorization(error));
        }
        Err(error) => {
            crate::kprintln!(
                "MAINTENANCE_AUTHORIZATION_REJECTED format=1 reason=envelope signature_valid=0 binding_valid=0 audit_attempted=0 audit_mutations=0 manifest_published=0 init_ready=0"
            );
            return Err(MaintenanceAuthorizationPrepareError::Authorization(error));
        }
    };

    let audit_binding = MaintenanceAuthorizationAuditBinding {
        sequence: verified.sequence(),
        operations: verified.operations(),
        max_uses: verified.max_uses(),
        authorization_sha256: *verified.signed_region_sha256(),
        authorization_id: *verified.authorization_id(),
        manifest_sha256: MAINTENANCE_PRODUCT_MANIFEST_SHA256,
        policy_sha256: MAINTENANCE_POLICY_SHA256,
        root_sha256: MAINTENANCE_AUTHORIZATION_ROOT_SHA256,
        device_binding_sha256: MAINTENANCE_DEVICE_BINDING_SHA256,
    };
    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    let (verified_signed_plan, signed_plan_binding) = {
        use bndr_sm::maintenance_plan::{
            MAINTENANCE_PLAN_ROOT_SHA256, MAINTENANCE_PLAN_RSA2048_KEY1, MaintenancePlanBinding,
            MaintenancePlanError, verify_maintenance_plan,
        };
        use bndroid_kernel::persist::SignedMaintenancePlanBinding;

        if sha256(MAINTENANCE_PLAN_RSA2048_KEY1.modulus()) != MAINTENANCE_PLAN_ROOT_SHA256 {
            return Err(MaintenanceAuthorizationPrepareError::ManifestPrerequisite);
        }
        SIGNED_MAINTENANCE_PLAN_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        let expected = MaintenancePlanBinding {
            authorization_sequence: verified.sequence(),
            authorization_sha256: *verified.signed_region_sha256(),
            authorization_id: *verified.authorization_id(),
            device_binding_sha256: MAINTENANCE_DEVICE_BINDING_SHA256,
            plan_id_sha256: bndroid_kernel::persist::maintenance_plan_id_sha256(audit_binding),
        };
        let plan = match verify_maintenance_plan(
            MAINTENANCE_PLAN_ARTIFACT,
            &MAINTENANCE_PLAN_RSA2048_KEY1,
            expected,
        ) {
            Ok(plan) => plan,
            Err(error @ MaintenancePlanError::Signature) => {
                SIGNED_MAINTENANCE_PLAN_SIGNATURE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                crate::kprintln!(
                    "SIGNED_MAINTENANCE_PLAN_REJECTED format=1 reason=signature signature_valid=0 binding_valid=0 program_valid=0 program_preflight_attempted=0 program_mutations=0 audit_attempted=0 audit_mutations=0 manifest_published=0 init_ready=0"
                );
                return Err(MaintenanceAuthorizationPrepareError::SignedPlan(error));
            }
            Err(error @ (MaintenancePlanError::Binding | MaintenancePlanError::PlanId)) => {
                SIGNED_MAINTENANCE_PLAN_BINDING_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                crate::kprintln!(
                    "SIGNED_MAINTENANCE_PLAN_REJECTED format=1 reason=binding signature_valid=1 binding_valid=0 program_valid=0 program_preflight_attempted=0 program_mutations=0 audit_attempted=0 audit_mutations=0 manifest_published=0 init_ready=0"
                );
                return Err(MaintenanceAuthorizationPrepareError::SignedPlan(error));
            }
            Err(
                error @ (MaintenancePlanError::InvalidOperationCount(_)
                | MaintenancePlanError::InvalidTransitionCount(_)
                | MaintenancePlanError::InvalidProgramVersion(_)
                | MaintenancePlanError::InvalidPlanGeneration(_)
                | MaintenancePlanError::InvalidProgram),
            ) => {
                SIGNED_MAINTENANCE_PLAN_PROGRAM_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                crate::kprintln!(
                    "SIGNED_MAINTENANCE_PLAN_REJECTED format=1 reason=program signature_valid=1 binding_valid=1 program_valid=0 program_preflight_attempted=0 program_mutations=0 audit_attempted=0 audit_mutations=0 manifest_published=0 init_ready=0"
                );
                return Err(MaintenanceAuthorizationPrepareError::SignedPlan(error));
            }
            Err(error) => {
                crate::kprintln!(
                    "SIGNED_MAINTENANCE_PLAN_REJECTED format=1 reason=envelope signature_valid=0 binding_valid=0 program_valid=0 program_preflight_attempted=0 program_mutations=0 audit_attempted=0 audit_mutations=0 manifest_published=0 init_ready=0"
                );
                return Err(MaintenanceAuthorizationPrepareError::SignedPlan(error));
            }
        };
        let binding = SignedMaintenancePlanBinding {
            authorization_sequence: plan.authorization_sequence(),
            operation_count: plan.operation_count(),
            transition_count: plan.transition_count(),
            program_version: plan.program_version(),
            preapply_cancel_mask: plan.preapply_cancel_mask(),
            authorization_sha256: *plan.authorization_sha256(),
            authorization_id: *plan.authorization_id(),
            program_sha256: *plan.signed_region_sha256(),
            plan_id_sha256: *plan.plan_id_sha256(),
            descriptor_sha256: *plan.descriptor_sha256(),
            root_sha256: MAINTENANCE_PLAN_ROOT_SHA256,
            device_binding_sha256: *plan.device_binding_sha256(),
        };
        (plan, binding)
    };
    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    let signed_plan_request = bndroid_kernel::persist::SignedMaintenancePlanRequest {
        partition_first_lba: crate::storage::DATA_PARTITION_FIRST_LBA,
        partition_sectors: crate::storage::DATA_PARTITION_SECTORS,
        expected_format_epoch: crate::storage::DATA_PARTITION_FORMAT_EPOCH,
        authorization: audit_binding,
        binding: signed_plan_binding,
    };
    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    let signed_plan_preflight =
        match crate::storage_persist::preflight_verified_signed_maintenance_plan(
            counter_frequency,
            signed_plan_request,
        ) {
            Ok(evidence) => evidence,
            Err(error) => {
                SIGNED_MAINTENANCE_PLAN_PERSISTENCE_FAILURES.fetch_add(1, Ordering::Relaxed);
                crate::kprintln!(
                    "SIGNED_MAINTENANCE_PLAN_REJECTED format=1 reason=program-ledger sequence={} signature_valid=1 binding_valid=1 program_valid=1 program_preflight_attempted=1 program_mutations=0 audit_attempted=0 audit_mutations=0 manifest_published=0 init_ready=0",
                    verified.sequence(),
                );
                crate::kprintln!("SIGNED_MAINTENANCE_PLAN_DIAG error={}", error.as_str());
                return Err(MaintenanceAuthorizationPrepareError::Audit(error));
            }
        };
    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    let signed_plan_evidence = match crate::storage_persist::commit_verified_signed_maintenance_plan(
        counter_frequency,
        signed_plan_request,
    ) {
        Ok(evidence) => evidence,
        Err(error) => {
            SIGNED_MAINTENANCE_PLAN_PERSISTENCE_FAILURES.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "SIGNED_MAINTENANCE_PLAN_REJECTED format=1 reason=program-commit sequence={} signature_valid=1 binding_valid=1 program_valid=1 program_preflight_attempted=1 program_mutations=unknown audit_attempted=0 audit_mutations=0 manifest_published=0 init_ready=0",
                verified.sequence(),
            );
            crate::kprintln!("SIGNED_MAINTENANCE_PLAN_DIAG error={}", error.as_str());
            return Err(MaintenanceAuthorizationPrepareError::Audit(error));
        }
    };
    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    SIGNED_MAINTENANCE_PLAN_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    {
        let mode = option_env!("BNDROID_M81_TEST_MODE").unwrap_or("normal");
        if !matches!(mode, "normal" | "cut-binding") {
            panic!("BNDROID_M81_TEST_MODE is not a bounded emulator mode");
        }
        if mode == "cut-binding" && !signed_plan_evidence.replayed {
            crate::kprintln!(
                "SIGNED_MAINTENANCE_PLAN_TEST_PAUSE format=1 mode=cut-binding boundary=program-bound-before-audit sequence={} generation={} slot={} reads={} writes={} flushes={} program_mutations=1 audit_attempted=0 audit_mutations=0 host_termination_required=1 qemu_disk_durable=1 hardware_powercut_claim=0 emulator_only=1 real_phone_claim=0",
                signed_plan_evidence.committed_sequence,
                signed_plan_evidence.committed_generation,
                signed_plan_evidence.committed_slot,
                signed_plan_evidence.reads,
                signed_plan_evidence.writes,
                signed_plan_evidence.flushes,
            );
            crate::arch::aarch64::halt();
        }
    }
    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    let plan_admission = match crate::storage_persist::preflight_verified_maintenance_plan(
        counter_frequency,
        MaintenanceAuditRequest {
            partition_first_lba: crate::storage::DATA_PARTITION_FIRST_LBA,
            partition_sectors: crate::storage::DATA_PARTITION_SECTORS,
            expected_format_epoch: crate::storage::DATA_PARTITION_FORMAT_EPOCH,
            binding: audit_binding,
        },
    ) {
        Ok(evidence) => evidence,
        Err(error) => {
            crate::kprintln!(
                "MAINTENANCE_AUTHORIZATION_REJECTED format=4 reason=plan-journal sequence={} signature_valid=1 binding_valid=1 plan_preflight_attempted=1 audit_attempted=0 audit_mutations=0 execution_mutations=0 step_mutations=0 plan_mutations=0 manifest_published=0 init_ready=0",
                verified.sequence(),
            );
            crate::kprintln!("MAINTENANCE_PLAN_ADMISSION_DIAG error={}", error.as_str());
            return Err(MaintenanceAuthorizationPrepareError::Audit(error));
        }
    };
    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    let step_admission = plan_admission.step;
    #[cfg(all(
        feature = "unified-product-maintenance-step-runtime",
        not(feature = "unified-product-maintenance-plan-runtime")
    ))]
    let step_admission = match crate::storage_persist::preflight_verified_maintenance_steps(
        counter_frequency,
        MaintenanceAuditRequest {
            partition_first_lba: crate::storage::DATA_PARTITION_FIRST_LBA,
            partition_sectors: crate::storage::DATA_PARTITION_SECTORS,
            expected_format_epoch: crate::storage::DATA_PARTITION_FORMAT_EPOCH,
            binding: audit_binding,
        },
    ) {
        Ok(evidence) => evidence,
        Err(error) => {
            crate::kprintln!(
                "MAINTENANCE_AUTHORIZATION_REJECTED format=3 reason=step-journal sequence={} signature_valid=1 binding_valid=1 step_preflight_attempted=1 audit_attempted=0 audit_mutations=0 execution_mutations=0 step_mutations=0 manifest_published=0 init_ready=0",
                verified.sequence(),
            );
            crate::kprintln!("MAINTENANCE_STEP_ADMISSION_DIAG error={}", error.as_str());
            return Err(MaintenanceAuthorizationPrepareError::Audit(error));
        }
    };
    MAINTENANCE_AUDIT_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    #[cfg(feature = "unified-product-maintenance-execution-runtime")]
    let audit_result = crate::storage_persist::admit_verified_maintenance_authorization(
        counter_frequency,
        MaintenanceAuditRequest {
            partition_first_lba: crate::storage::DATA_PARTITION_FIRST_LBA,
            partition_sectors: crate::storage::DATA_PARTITION_SECTORS,
            expected_format_epoch: crate::storage::DATA_PARTITION_FORMAT_EPOCH,
            binding: audit_binding,
        },
    );
    #[cfg(not(feature = "unified-product-maintenance-execution-runtime"))]
    let audit_result = crate::storage_persist::commit_verified_maintenance_authorization(
        counter_frequency,
        MaintenanceAuditRequest {
            partition_first_lba: crate::storage::DATA_PARTITION_FIRST_LBA,
            partition_sectors: crate::storage::DATA_PARTITION_SECTORS,
            expected_format_epoch: crate::storage::DATA_PARTITION_FORMAT_EPOCH,
            binding: audit_binding,
        },
    );
    let evidence = match audit_result {
        Ok(evidence) => evidence,
        #[cfg(feature = "unified-product-maintenance-execution-runtime")]
        Err(
            error @ crate::storage_persist::StoragePersistError::MaintenanceExecutionTransaction(
                MaintenanceExecutionError::Replay { actual, minimum }
                | MaintenanceExecutionError::CompletedReplay { actual, minimum },
            ),
        ) => {
            MAINTENANCE_AUDIT_REPLAY_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            #[cfg(feature = "unified-product-maintenance-step-runtime")]
            crate::kprintln!(
                "MAINTENANCE_AUTHORIZATION_REJECTED format=2 reason=completed-replay sequence={} minimum={} signature_valid=1 binding_valid=1 audit_attempted=1 audit_mutations=0 execution_mutations=0 step_mutations=0 manifest_published=0 init_ready=0",
                actual,
                minimum,
            );
            #[cfg(not(feature = "unified-product-maintenance-step-runtime"))]
            crate::kprintln!(
                "MAINTENANCE_AUTHORIZATION_REJECTED format=2 reason=completed-replay sequence={} minimum={} signature_valid=1 binding_valid=1 audit_attempted=1 audit_mutations=0 execution_mutations=0 manifest_published=0 init_ready=0",
                actual,
                minimum,
            );
            return Err(MaintenanceAuthorizationPrepareError::Audit(error));
        }
        #[cfg(not(feature = "unified-product-maintenance-execution-runtime"))]
        Err(
            error @ crate::storage_persist::StoragePersistError::MaintenanceAuditTransaction(
                MaintenanceAuditError::Replay { actual, minimum },
            ),
        ) => {
            MAINTENANCE_AUDIT_REPLAY_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "MAINTENANCE_AUTHORIZATION_REJECTED format=1 reason=replay sequence={} minimum={} signature_valid=1 binding_valid=1 audit_attempted=1 audit_mutations=0 manifest_published=0 init_ready=0",
                actual,
                minimum,
            );
            return Err(MaintenanceAuthorizationPrepareError::Audit(error));
        }
        Err(error) => {
            MAINTENANCE_AUDIT_FAILURES.fetch_add(1, Ordering::Relaxed);
            crate::kprintln!(
                "MAINTENANCE_AUTHORIZATION_REJECTED format=1 reason=audit sequence={} signature_valid=1 binding_valid=1 audit_attempted=1 manifest_published=0 init_ready=0",
                verified.sequence(),
            );
            crate::kprintln!("MAINTENANCE_AUDIT_DIAG error={}", error.as_str());
            return Err(MaintenanceAuthorizationPrepareError::Audit(error));
        }
    };

    #[cfg(feature = "unified-product-maintenance-step-runtime")]
    let evidence = {
        let mut evidence = evidence;
        evidence.step_preflight_reads = step_admission.reads;
        evidence.step_provisioned_before = step_admission.provisioned_before;
        evidence.step_legacy_anchor = step_admission.legacy_anchor;
        evidence.step_sequence_before = step_admission.step_sequence;
        evidence.step_base_sequence_before = step_admission.step_base_sequence;
        evidence.step_entry_count_before = step_admission.step_entry_count;
        evidence.step_ordinal_before = step_admission.step_ordinal;
        evidence.step_initial_generation = step_admission.step_generation;
        evidence.step_initial_slot = step_admission.step_slot;
        evidence.step_initial_valid_slots = step_admission.step_valid_slots;
        evidence.step_initial_rejected_slots = step_admission.step_rejected_slots;
        evidence
    };
    MAINTENANCE_AUDIT_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    MAINTENANCE_AUTHORIZATION_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    MAINTENANCE_PLAN_ADMISSION_PREPARATION.publish(plan_admission);
    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    SIGNED_MAINTENANCE_PLAN_PREPARATION.publish(SignedMaintenancePlanPreparationSnapshot {
        verified: verified_signed_plan,
        evidence: signed_plan_evidence,
    });
    MAINTENANCE_AUTHORIZATION_PREPARATION.publish(evidence);
    let authorization_words = sha256_words(&evidence.binding.authorization_sha256);
    let authorization_id_words = sha256_words(&evidence.binding.authorization_id);
    let chain_words = sha256_words(&evidence.committed_chain_sha256);
    crate::kprintln!(
        "MAINTENANCE_AUDIT_OK format=1 state=BNDRMAU1 state_version=1 authority=kernel-pre-el0-qemu-disk slots=2 relative_lbas=5/6 sequence={} operations={} max_uses={} provisioned_before={} previous_sequence={} committed_sequence={} previous_accepted_count={} committed_accepted_count={} initial_generation={} committed_generation={} initial_slot={} committed_slot={} initial_valid_slots={} initial_rejected_slots={} committed_valid_slots={} committed_rejected_slots={} reads={} writes={} flushes={} authorization_sha256={:016x}/{:016x}/{:016x}/{:016x} authorization_id={:016x}/{:016x}/{:016x}/{:016x} chain_sha256={:016x}/{:016x}/{:016x}/{:016x} signature_valid=1 binding_valid=1 audit_mutations={} write_flush_readback={} manifest_published=0 init_ready=0 trusted_monotonic_backend=0 rpmb_claim=0 efuse_claim=0 hsm_claim=0 host_rollback_resistance=0 hardware_powercut_claim=0 emulator_only=1",
        evidence.binding.sequence,
        evidence.binding.operations,
        evidence.binding.max_uses,
        u8::from(evidence.provisioned_before),
        evidence.previous_sequence,
        evidence.committed_sequence,
        evidence.previous_accepted_count,
        evidence.committed_accepted_count,
        evidence.initial_generation,
        evidence.committed_generation,
        evidence.initial_slot,
        evidence.committed_slot,
        evidence.initial_valid_slots,
        evidence.initial_rejected_slots,
        evidence.committed_valid_slots,
        evidence.committed_rejected_slots,
        evidence.reads,
        evidence.writes,
        evidence.flushes,
        authorization_words[0],
        authorization_words[1],
        authorization_words[2],
        authorization_words[3],
        authorization_id_words[0],
        authorization_id_words[1],
        authorization_id_words[2],
        authorization_id_words[3],
        chain_words[0],
        chain_words[1],
        chain_words[2],
        chain_words[3],
        u8::from(evidence.writes != 0),
        u8::from(evidence.writes == 1 && evidence.flushes == 1 && evidence.reads == 4),
    );
    #[cfg(feature = "unified-product-maintenance-execution-runtime")]
    crate::kprintln!(
        "MAINTENANCE_EXECUTION_ADMISSION_OK format=1 audit_state=BNDRMAU1 execution_state=BNDRMEX1 audit_relative_lbas=5/6 execution_relative_lbas=7/8 sequence={} resumed={} completed_sequence_before={} preflight_reads={} audit_reads={} audit_writes={} audit_flushes={} execution_initial_generation={} execution_initial_slot={} execution_initial_valid_slots={} execution_initial_rejected_slots={} exact_binding=1 predecessor_complete={} completed_replay=0 audit_write_flush_readback={} signature_valid=1 binding_valid=1 manifest_published=0 init_ready=0 exactly_once_claim=0 trusted_monotonic_backend=0 rpmb_claim=0 efuse_claim=0 hsm_claim=0 host_rollback_resistance=0 hardware_powercut_claim=0 emulator_only=1",
        evidence.binding.sequence,
        u8::from(evidence.resumed),
        evidence.completed_sequence_before,
        evidence.preflight_reads,
        evidence.reads,
        evidence.writes,
        evidence.flushes,
        evidence.execution_initial_generation,
        evidence.execution_initial_slot,
        evidence.execution_initial_valid_slots,
        evidence.execution_initial_rejected_slots,
        u8::from(
            evidence.binding.sequence == 1
                || evidence.completed_sequence_before.checked_add(1)
                    == Some(evidence.binding.sequence)
        ),
        u8::from(evidence.writes == 1 && evidence.flushes == 1 && evidence.reads == 4),
    );
    #[cfg(feature = "unified-product-maintenance-step-runtime")]
    crate::kprintln!(
        "MAINTENANCE_STEP_ADMISSION_OK format=1 state=BNDRMST1 state_version=1 authority=kernel-pre-el0-three-ledger-preflight slots=2 relative_lbas=9/10 sequence={} audit_sequence_before={} execution_sequence_before={} step_provisioned_before={} step_legacy_anchor={} resumes_current_sequence={} step_sequence_before={} step_base_sequence_before={} step_entry_count_before={} step_ordinal_before={} step_initial_generation={} step_initial_slot={} step_initial_valid_slots={} step_initial_rejected_slots={} reads={} writes=0 flushes=0 exact_authorization=1 predecessor_terminal_or_legacy=1 preflight_before_audit_mutation=1 replay_safe_from_boot_start=1 arbitrary_resume_claim=0 exactly_once_claim=0 trusted_monotonic_backend=0 hardware_powercut_claim=0 emulator_only=1 real_phone_claim=0",
        evidence.binding.sequence,
        step_admission.audit_sequence,
        step_admission.execution_sequence,
        u8::from(step_admission.provisioned_before),
        u8::from(step_admission.legacy_anchor),
        u8::from(step_admission.resumes_current_sequence),
        step_admission.step_sequence,
        step_admission.step_base_sequence,
        step_admission.step_entry_count,
        step_admission.step_ordinal,
        step_admission.step_generation,
        step_admission.step_slot,
        step_admission.step_valid_slots,
        step_admission.step_rejected_slots,
        step_admission.reads,
    );
    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    {
        let digest_words = |digest: &[u8; 32]| {
            [
                u64::from_be_bytes(digest[0..8].try_into().unwrap()),
                u64::from_be_bytes(digest[8..16].try_into().unwrap()),
                u64::from_be_bytes(digest[16..24].try_into().unwrap()),
                u64::from_be_bytes(digest[24..32].try_into().unwrap()),
            ]
        };
        let plan_id = digest_words(&bndroid_kernel::persist::maintenance_plan_id_sha256(
            plan_admission.binding,
        ));
        let chain = digest_words(&plan_admission.chain_sha256);
        crate::kprintln!(
            "MAINTENANCE_PLAN_ADMISSION_OK format=1 state=BNDRMPL1 state_version=1 authority=kernel-pre-el0-four-ledger-preflight slots=2 relative_lbas=11/12 sequence={} plan_provisioned_before={} plan_legacy_anchor={} resumes_current_sequence={} plan_sequence_before={} plan_base_sequence_before={} transition_count_before={} operation_before={} phase_before={} generation={} slot={} valid_slots={} rejected_slots={} reads={} writes=0 flushes=0 result_unknown={} effect_observed_unconfirmed={} plan_id_sha256={:016x}/{:016x}/{:016x}/{:016x} chain_sha256={:016x}/{:016x}/{:016x}/{:016x} exact_authorization=1 operation_instance_bound=1 idempotency_key_bound=1 preflight_before_audit_mutation=1 external_effect_exactly_once_claim=0 arbitrary_resume_claim=0 trusted_monotonic_backend=0 hardware_powercut_claim=0 emulator_only=1 real_phone_claim=0",
            plan_admission.binding.sequence,
            u8::from(plan_admission.provisioned_before),
            u8::from(plan_admission.legacy_anchor),
            u8::from(plan_admission.resumes_current_sequence),
            plan_admission.sequence,
            plan_admission.base_sequence,
            plan_admission.transition_count,
            plan_admission.operation_ordinal,
            plan_admission.phase,
            plan_admission.generation,
            plan_admission.slot,
            plan_admission.valid_slots,
            plan_admission.rejected_slots,
            plan_admission.reads,
            u8::from(plan_admission.result_unknown),
            u8::from(plan_admission.effect_observed_unconfirmed),
            plan_id[0],
            plan_id[1],
            plan_id[2],
            plan_id[3],
            chain[0],
            chain[1],
            chain[2],
            chain[3],
        );
    }
    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    {
        let digest_words = |digest: &[u8; 32]| {
            [
                u64::from_be_bytes(digest[0..8].try_into().unwrap()),
                u64::from_be_bytes(digest[8..16].try_into().unwrap()),
                u64::from_be_bytes(digest[16..24].try_into().unwrap()),
                u64::from_be_bytes(digest[24..32].try_into().unwrap()),
            ]
        };
        let program = digest_words(&signed_plan_evidence.binding.program_sha256);
        let descriptors = digest_words(&signed_plan_evidence.binding.descriptor_sha256);
        let chain = digest_words(&signed_plan_evidence.committed_chain_sha256);
        crate::kprintln!(
            "SIGNED_MAINTENANCE_PLAN_OK format=1 artifact=BMP1 state=BNDRMPB1 state_version=1 authority=kernel-pre-el0-five-ledger-binding slots=2 relative_lbas=13/14 authorization_sequence={} operation_count={} transition_count={} program_version={} plan_generation={} preapply_cancel_mask={} signature_valid=1 binding_valid=1 program_valid=1 plan_preflight_reads={} program_provisioned_before={} program_exact_replay_before={} predecessor_complete={} initial_generation={} committed_generation={} initial_slot={} committed_slot={} initial_valid_slots={} initial_rejected_slots={} committed_valid_slots={} committed_rejected_slots={} reads={} writes={} flushes={} program_sha256={:016x}/{:016x}/{:016x}/{:016x} descriptor_sha256={:016x}/{:016x}/{:016x}/{:016x} chain_sha256={:016x}/{:016x}/{:016x}/{:016x} write_flush_readback={} old_slot_preserved=1 preflight_before_program_mutation=1 program_bound_before_audit_mutation=1 descriptor_driven=1 arbitrary_program_claim=0 external_effect_exactly_once_claim=0 trusted_monotonic_backend=0 hardware_powercut_claim=0 emulator_only=1 real_phone_claim=0 manifest_published=0 init_ready=0",
            signed_plan_evidence.binding.authorization_sequence,
            signed_plan_evidence.binding.operation_count,
            signed_plan_evidence.binding.transition_count,
            signed_plan_evidence.binding.program_version,
            verified_signed_plan.plan_generation(),
            signed_plan_evidence.binding.preapply_cancel_mask,
            signed_plan_preflight.reads,
            u8::from(signed_plan_evidence.provisioned_before),
            u8::from(signed_plan_preflight.exact_replay),
            u8::from(signed_plan_evidence.predecessor_complete),
            signed_plan_evidence.initial_generation,
            signed_plan_evidence.committed_generation,
            signed_plan_evidence.initial_slot,
            signed_plan_evidence.committed_slot,
            signed_plan_evidence.initial_valid_slots,
            signed_plan_evidence.initial_rejected_slots,
            signed_plan_evidence.committed_valid_slots,
            signed_plan_evidence.committed_rejected_slots,
            signed_plan_evidence.reads,
            signed_plan_evidence.writes,
            signed_plan_evidence.flushes,
            program[0],
            program[1],
            program[2],
            program[3],
            descriptors[0],
            descriptors[1],
            descriptors[2],
            descriptors[3],
            chain[0],
            chain[1],
            chain[2],
            chain[3],
            u8::from(
                signed_plan_evidence.writes == 1
                    && signed_plan_evidence.flushes == 1
                    && signed_plan_evidence.reads == 14
                    || signed_plan_evidence.writes == 0
                        && signed_plan_evidence.flushes == 0
                        && signed_plan_evidence.replayed
            ),
        );
    }
    Ok(evidence)
}

#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
pub fn maintenance_authorization_snapshot() -> MaintenanceAuthorizationSnapshot {
    let evidence = MAINTENANCE_AUTHORIZATION_PREPARATION.snapshot();
    MaintenanceAuthorizationSnapshot {
        prepared: evidence.is_some(),
        authorization_attempts: MAINTENANCE_AUTHORIZATION_ATTEMPTS.load(Ordering::Acquire),
        authorization_successes: MAINTENANCE_AUTHORIZATION_SUCCESSES.load(Ordering::Acquire),
        signature_rejections: MAINTENANCE_AUTHORIZATION_SIGNATURE_REJECTIONS
            .load(Ordering::Acquire),
        binding_rejections: MAINTENANCE_AUTHORIZATION_BINDING_REJECTIONS.load(Ordering::Acquire),
        audit_attempts: MAINTENANCE_AUDIT_ATTEMPTS.load(Ordering::Acquire),
        audit_successes: MAINTENANCE_AUDIT_SUCCESSES.load(Ordering::Acquire),
        audit_replay_rejections: MAINTENANCE_AUDIT_REPLAY_REJECTIONS.load(Ordering::Acquire),
        audit_failures: MAINTENANCE_AUDIT_FAILURES.load(Ordering::Acquire),
        session_open_calls: MAINTENANCE_SESSION_OPEN_CALLS.load(Ordering::Acquire),
        session_open_successes: MAINTENANCE_SESSION_OPEN_SUCCESSES.load(Ordering::Acquire),
        session_open_argument_rejections: MAINTENANCE_SESSION_OPEN_ARGUMENT_REJECTIONS
            .load(Ordering::Acquire),
        session_open_permission_rejections: MAINTENANCE_SESSION_OPEN_PERMISSION_REJECTIONS
            .load(Ordering::Acquire),
        session_open_state_rejections: MAINTENANCE_SESSION_OPEN_STATE_REJECTIONS
            .load(Ordering::Acquire),
        session_opened: MAINTENANCE_SESSION_OPENED.load(Ordering::Acquire),
        report_gate_denials: MAINTENANCE_REPORT_GATE_DENIALS.load(Ordering::Acquire),
        evidence,
        #[cfg(feature = "unified-product-maintenance-plan-runtime")]
        plan_evidence: MAINTENANCE_PLAN_ADMISSION_PREPARATION.snapshot(),
    }
}

#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignedMaintenancePlanRuntimeSnapshot {
    pub attempts: u64,
    pub successes: u64,
    pub signature_rejections: u64,
    pub binding_rejections: u64,
    pub program_rejections: u64,
    pub persistence_failures: u64,
    pub preparation: Option<SignedMaintenancePlanPreparationSnapshot>,
}

#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
pub fn signed_maintenance_plan_snapshot() -> SignedMaintenancePlanRuntimeSnapshot {
    SignedMaintenancePlanRuntimeSnapshot {
        attempts: SIGNED_MAINTENANCE_PLAN_ATTEMPTS.load(Ordering::Acquire),
        successes: SIGNED_MAINTENANCE_PLAN_SUCCESSES.load(Ordering::Acquire),
        signature_rejections: SIGNED_MAINTENANCE_PLAN_SIGNATURE_REJECTIONS.load(Ordering::Acquire),
        binding_rejections: SIGNED_MAINTENANCE_PLAN_BINDING_REJECTIONS.load(Ordering::Acquire),
        program_rejections: SIGNED_MAINTENANCE_PLAN_PROGRAM_REJECTIONS.load(Ordering::Acquire),
        persistence_failures: SIGNED_MAINTENANCE_PLAN_PERSISTENCE_FAILURES.load(Ordering::Acquire),
        preparation: SIGNED_MAINTENANCE_PLAN_PREPARATION.snapshot(),
    }
}

pub fn init() {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    if INITIALIZED.swap(true, Ordering::AcqRel) {
        panic!("init process syscall state initialized twice");
    }
    reset_counters();
    let heap_free = crate::kernel_heap::stats()
        .unwrap_or_else(|| panic!("kernel heap missing during syscall initialization"))
        .free_bytes;
    HEAP_FREE_BASELINE.store(heap_free, Ordering::Release);
    crate::arch::aarch64::restore_daif(saved_daif);
}

/// Dispatches one register or bounded byte-buffer syscall from the lower-EL
/// AArch64 SVC vector.
pub fn dispatch(frame: *mut TrapFrame) -> *mut TrapFrame {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("lower-EL syscall dispatcher entered with IRQ enabled");
    }
    if !crate::scheduler::validate_current_user_frame(frame) {
        return crate::scheduler::fault_current_user(
            frame,
            crate::scheduler::UserFaultReason::InvalidSyscallContext,
            0,
            0,
        );
    }
    if crate::arch::aarch64::mmu::pan_supported() && !crate::arch::aarch64::mmu::pan_is_enabled() {
        PAN_FAILURES.fetch_add(1, Ordering::Relaxed);
        panic!("lower-EL syscall dispatcher entered with PAN disabled");
    }
    record_user_asid();
    record_call();
    let (raw_number, arg0, arg1, arg2) =
        unsafe { ((*frame).x[8], (*frame).x[0], (*frame).x[1], (*frame).x[2]) };
    let Some(number) = SyscallNumber::from_raw(raw_number) else {
        UNKNOWN.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::Unsupported, 0, 0);
    };
    #[cfg(feature = "androidbox-process0")]
    if crate::process::current_live_user_image_id() == Some(UserImageId::AndroidApp)
        && !android_app_syscall_allowed(number)
    {
        let owner = crate::process::current_live_user_process_id().unwrap_or(0);
        crate::kprintln!(
            "ANDROID_APP_SYSCALL_DENIED_OK owner={} number={} authority_unchanged=1",
            owner,
            raw_number,
        );
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    match number {
        SyscallNumber::AbiVersion => complete(frame, Status::Ok, ABI_VERSION, 0),
        SyscallNumber::ChannelCreate => channel_create(frame),
        SyscallNumber::ChannelWrite => channel_write(frame, arg0, arg1, arg2),
        SyscallNumber::ChannelRead => channel_read(frame, arg0),
        SyscallNumber::HandleDuplicate => handle_duplicate(frame, arg0, arg1),
        SyscallNumber::HandleClose => handle_close(frame, arg0),
        SyscallNumber::InitReady => init_ready(frame, arg0, arg1, arg2),
        SyscallNumber::InitFailed => {
            if crate::process::current_is_init() {
                INIT_FAILURE.store(arg0.max(1), Ordering::Release);
                complete(frame, Status::Ok, 0, 0)
            } else {
                complete(frame, Status::PermissionDenied, 0, 0)
            }
        }
        SyscallNumber::ChannelWriteBytes => channel_write_bytes(frame, arg0, arg1, arg2),
        SyscallNumber::ChannelReadBytes => channel_read_bytes(frame, arg0, arg1, arg2),
        SyscallNumber::ProcessSpawn => process_spawn(frame, arg0, arg1, arg2),
        SyscallNumber::ThreadExit => thread_exit(frame, arg0, arg1, arg2),
        SyscallNumber::ProcessWait => process_wait(frame, arg0, arg1, arg2),
        SyscallNumber::ChannelWriteTransfer => channel_write_transfer(frame, arg0, arg1, arg2),
        SyscallNumber::ChannelReadTransfer => channel_read_transfer(frame, arg0, arg1, arg2),
        SyscallNumber::ObjectWait => object_wait(frame, arg0, arg1, arg2),
        SyscallNumber::EventCreate => event_create(frame, arg0, arg1, arg2),
        SyscallNumber::EventSignal => event_signal(frame, arg0, arg1, arg2),
        SyscallNumber::EventClear => event_clear(frame, arg0, arg1, arg2),
        SyscallNumber::ObjectWaitMany => object_wait_many(frame, arg0, arg1, arg2),
        SyscallNumber::ChannelPeek => channel_peek(frame, arg0, arg1, arg2),
        SyscallNumber::ChannelReadEnvelope => channel_read_envelope(frame, arg0, arg1, arg2),
        SyscallNumber::ObjectWaitManyArray => object_wait_many_array(frame, arg0, arg1, arg2),
        SyscallNumber::FileOpenAt => file_open_at(frame, arg0, arg1, arg2),
        SyscallNumber::VmoRead => vmo_read(frame, arg0, arg1, arg2),
        SyscallNumber::SurfaceAcquire => surface_acquire(frame, arg0, arg1, arg2),
        SyscallNumber::SurfacePresent => surface_present(frame, arg0, arg1, arg2),
        SyscallNumber::SurfaceReadInput => surface_read_input(frame, arg0, arg1, arg2),
        SyscallNumber::GraphicsBufferCreate => graphics_buffer_create(frame, arg0, arg1, arg2),
        SyscallNumber::GraphicsBufferWrite => graphics_buffer_write(frame, arg0, arg1, arg2),
        SyscallNumber::SurfacePresentBuffer => surface_present_buffer(frame, arg0, arg1, arg2),
        #[cfg(feature = "androidbox-interactive0")]
        SyscallNumber::SurfacePresentBufferLayers => {
            surface_present_buffer_layers(frame, arg0, arg1, arg2)
        }
        SyscallNumber::ProcessTerminate => process_terminate(frame, arg0, arg1, arg2),
        SyscallNumber::GraphicsBufferMap => graphics_buffer_map(frame, arg0, arg1, arg2),
        SyscallNumber::GraphicsBufferUnmap => graphics_buffer_unmap(frame, arg0, arg1, arg2),
        SyscallNumber::GraphicsBufferQueue => graphics_buffer_queue(frame, arg0, arg1, arg2),
        SyscallNumber::GraphicsBufferAcquire => graphics_buffer_acquire(frame, arg0, arg1, arg2),
        SyscallNumber::GraphicsBufferRelease => graphics_buffer_release(frame, arg0, arg1, arg2),
        SyscallNumber::SurfaceFrameAcquire => surface_frame_acquire(frame, arg0, arg1, arg2),
        SyscallNumber::SurfaceReadKey => surface_read_key(frame, arg0, arg1, arg2),
        SyscallNumber::InputAcquire => input_acquire(frame, arg0, arg1, arg2),
        SyscallNumber::InputReadEvent => input_read_event(frame, arg0, arg1, arg2),
        SyscallNumber::InputSessionInfo => input_session_info(frame, arg0, arg1, arg2),
        #[cfg(all(feature = "app-data-runtime", not(feature = "storage-server-runtime")))]
        SyscallNumber::AppDataRootOpen => app_data_root_open(frame, arg0, arg1, arg2),
        #[cfg(all(feature = "app-data-runtime", not(feature = "storage-server-runtime")))]
        SyscallNumber::FileReplaceAt => file_replace_at(frame, arg0, arg1, arg2),
        #[cfg(all(feature = "app-data-runtime", not(feature = "storage-server-runtime")))]
        SyscallNumber::DirectoryCreateAt => directory_create_at(frame, arg0, arg1, arg2),
        #[cfg(all(feature = "app-data-runtime", not(feature = "storage-server-runtime")))]
        SyscallNumber::UnlinkAt => unlink_at(frame, arg0, arg1, arg2),
        #[cfg(all(feature = "app-data-runtime", not(feature = "storage-server-runtime")))]
        SyscallNumber::DirectoryReadAt => directory_read_at(frame, arg0, arg1, arg2),
        #[cfg(any(not(feature = "app-data-runtime"), feature = "storage-server-runtime"))]
        SyscallNumber::AppDataRootOpen
        | SyscallNumber::FileReplaceAt
        | SyscallNumber::DirectoryCreateAt
        | SyscallNumber::UnlinkAt
        | SyscallNumber::DirectoryReadAt => complete(frame, Status::Unsupported, 0, 0),
        #[cfg(feature = "storage-server-runtime")]
        SyscallNumber::IpcBufferCreate => ipc_buffer_create(frame, arg0, arg1, arg2),
        #[cfg(feature = "storage-server-runtime")]
        SyscallNumber::StorageAcquire => storage_acquire(frame, arg0, arg1, arg2),
        #[cfg(feature = "storage-server-runtime")]
        SyscallNumber::StorageSubmit => storage_submit(frame, arg0, arg1, arg2),
        #[cfg(feature = "storage-server-runtime")]
        SyscallNumber::StorageTake => storage_take(frame, arg0, arg1, arg2),
        #[cfg(feature = "storage-server-runtime")]
        SyscallNumber::StorageConnect => storage_connect(frame, arg0, arg1, arg2),
        #[cfg(feature = "storage-server-runtime")]
        SyscallNumber::StorageAccept => storage_accept(frame, arg0, arg1, arg2),
        #[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
        SyscallNumber::SystemShutdown => system_shutdown(frame, arg0, arg1, arg2),
        #[cfg(not(feature = "storage-server-shutdown-orchestration-runtime"))]
        SyscallNumber::SystemShutdown => complete(frame, Status::Unsupported, 0, 0),
        #[cfg(feature = "resident-platform-shutdown-runtime")]
        SyscallNumber::ServiceShutdown => service_shutdown(frame, arg0, arg1, arg2),
        #[cfg(feature = "unified-product-manifest-supervision-runtime")]
        SyscallNumber::ServiceManifestOpen => service_manifest_open(frame, arg0, arg1, arg2),
        #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
        SyscallNumber::MaintenanceSessionOpen => maintenance_session_open(frame, arg0, arg1, arg2),
        #[cfg(feature = "unified-product-event-supervision-runtime")]
        SyscallNumber::ServiceSupervisorReport => {
            service_supervisor_report(frame, arg0, arg1, arg2)
        }
        #[cfg(feature = "mobile-ui-runtime")]
        SyscallNumber::SystemClockRead => system_clock_read(frame, arg0, arg1, arg2),
        #[cfg(feature = "androidbox-apk-install0")]
        SyscallNumber::AndroidPackageSnapshotRead => {
            android_package_snapshot_read(frame, arg0, arg1, arg2)
        }
        #[cfg(feature = "androidbox-apk-install0")]
        SyscallNumber::AndroidPackageRelaunch => android_package_relaunch(frame, arg0, arg1, arg2),
        #[cfg(feature = "androidbox-el0-runtime0")]
        SyscallNumber::AndroidPackageImageClaim => {
            android_package_image_claim(frame, arg0, arg1, arg2)
        }
        #[cfg(feature = "androidbox-runtime-uninstall1")]
        SyscallNumber::AndroidPackageUninstall => {
            android_package_uninstall(frame, arg0, arg1, arg2)
        }
        #[cfg(feature = "androidbox-runtime-install2")]
        SyscallNumber::AndroidPackageInstallCandidateRead => {
            android_package_install_candidate_read(frame, arg0, arg1, arg2)
        }
        #[cfg(feature = "androidbox-runtime-install2")]
        SyscallNumber::AndroidPackageInstall => android_package_install(frame, arg0, arg1, arg2),
        #[cfg(feature = "androidbox-multipackage4")]
        SyscallNumber::AndroidPackageDirectoryRead => {
            android_package_directory_read(frame, arg0, arg1, arg2)
        }
        #[cfg(feature = "androidbox-icon-resources5")]
        SyscallNumber::AndroidPackageIconRead => android_package_icon_read(frame, arg0, arg1, arg2),
        #[cfg(not(feature = "storage-server-runtime"))]
        SyscallNumber::IpcBufferCreate
        | SyscallNumber::StorageAcquire
        | SyscallNumber::StorageSubmit
        | SyscallNumber::StorageTake
        | SyscallNumber::StorageConnect
        | SyscallNumber::StorageAccept => complete(frame, Status::Unsupported, 0, 0),
    }
}

#[cfg(feature = "androidbox-process0")]
const fn android_app_syscall_allowed(number: SyscallNumber) -> bool {
    matches!(
        number,
        SyscallNumber::AbiVersion
            | SyscallNumber::HandleClose
            | SyscallNumber::ChannelWriteBytes
            | SyscallNumber::ThreadExit
            | SyscallNumber::ObjectWait
            | SyscallNumber::ChannelReadEnvelope
            | SyscallNumber::VmoRead
            | SyscallNumber::AndroidPackageSnapshotRead
            | SyscallNumber::AndroidPackageImageClaim
    )
}

#[cfg(feature = "mobile-ui-runtime")]
const fn system_clock_read_arguments_valid(reserved0: u64, reserved1: u64, reserved2: u64) -> bool {
    reserved0 == SYSTEM_CLOCK_READ_FLAGS_NONE
        && reserved1 == SYSTEM_CLOCK_READ_FLAGS_NONE
        && reserved2 == SYSTEM_CLOCK_READ_FLAGS_NONE
}

#[cfg(feature = "mobile-ui-runtime")]
const fn system_clock_minute(unix_seconds: u64) -> u64 {
    unix_seconds / 60
}

/// Returns one read-only current clock snapshot to the authenticated
/// generation-qualified SurfaceServer.
#[cfg(feature = "mobile-ui-runtime")]
fn system_clock_read(
    frame: *mut TrapFrame,
    reserved0: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    if !system_clock_read_arguments_valid(reserved0, reserved1, reserved2) {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    if !crate::process::current_is_surface_server() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let seconds = crate::platform::qemu_virt::read_pl031_unix_seconds();
    let minute = system_clock_minute(seconds);
    let previous_minute = SYSTEM_CLOCK_LAST_LOGGED_MINUTE.swap(minute, Ordering::AcqRel);
    if previous_minute == u64::MAX {
        crate::kprintln!(
            "MOBILE_UI_CLOCK_READ_OK image=surface-server seconds={} minute={} source=qemu-pl031 snapshot=current phase=initial",
            seconds,
            minute,
        );
    } else if previous_minute != minute {
        crate::kprintln!(
            "MOBILE_UI_CLOCK_ADVANCE_OK image=surface-server seconds={} minute={} source=qemu-pl031 snapshot=current direction={}",
            seconds,
            minute,
            if minute > previous_minute {
                "forward"
            } else {
                "backward"
            },
        );
    }
    complete(frame, Status::Ok, seconds, SYSTEM_CLOCK_SOURCE_QEMU_PL031)
}

/// Copies one authority-free, boot-stable installed-package snapshot to the
/// authenticated package reader. The package bytes and package-store
/// capability remain in EL1; this syscall exposes only already-verified
/// bounded metadata.
#[cfg(feature = "androidbox-process0")]
const fn android_package_snapshot_reader_allowed(image: UserImageId) -> bool {
    matches!(
        image,
        UserImageId::Launcher | UserImageId::App | UserImageId::AndroidApp
    )
}

#[cfg(all(
    feature = "androidbox-apk-install0",
    not(feature = "androidbox-process0")
))]
const fn android_package_snapshot_reader_allowed(image: UserImageId) -> bool {
    matches!(image, UserImageId::Launcher | UserImageId::App)
}

#[cfg(feature = "androidbox-process0")]
const _: () = {
    assert!(android_package_snapshot_reader_allowed(
        UserImageId::AndroidApp
    ));
    assert!(android_package_snapshot_reader_allowed(
        UserImageId::Launcher
    ));
    assert!(android_package_snapshot_reader_allowed(UserImageId::App));
    assert!(!android_package_snapshot_reader_allowed(
        UserImageId::SurfaceServer
    ));
};

#[cfg(feature = "androidbox-runtime-install2")]
const fn android_package_install_caller_allowed(image: UserImageId) -> bool {
    matches!(image, UserImageId::App)
}

#[cfg(feature = "androidbox-runtime-install2")]
const _: () = {
    assert!(android_package_install_caller_allowed(UserImageId::App));
    assert!(!android_package_install_caller_allowed(
        UserImageId::Launcher
    ));
    assert!(!android_package_install_caller_allowed(
        UserImageId::AndroidApp
    ));
    assert!(!android_package_install_caller_allowed(
        UserImageId::SurfaceServer
    ));
};

#[cfg(all(
    feature = "androidbox-apk-install0",
    not(feature = "androidbox-process0")
))]
const _: () = {
    assert!(android_package_snapshot_reader_allowed(
        UserImageId::Launcher
    ));
    assert!(android_package_snapshot_reader_allowed(UserImageId::App));
    assert!(!android_package_snapshot_reader_allowed(
        UserImageId::SurfaceServer
    ));
};

#[cfg(feature = "androidbox-runtime-uninstall1")]
const fn android_package_uninstall_caller_allowed(image: UserImageId) -> bool {
    matches!(image, UserImageId::App)
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
const _: () = {
    assert!(android_package_uninstall_caller_allowed(UserImageId::App));
    assert!(!android_package_uninstall_caller_allowed(
        UserImageId::Launcher
    ));
    assert!(!android_package_uninstall_caller_allowed(
        UserImageId::AndroidApp
    ));
    assert!(!android_package_uninstall_caller_allowed(
        UserImageId::SurfaceServer
    ));
};

#[cfg(feature = "androidbox-apk-install0")]
fn android_package_snapshot_read(
    frame: *mut TrapFrame,
    user_destination: u64,
    raw_length: u64,
    reserved: u64,
) -> *mut TrapFrame {
    use bndr_abi::{
        ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE, AndroidPackageInstalledMetadata,
        AndroidPackageIoCounters, AndroidPackageSnapshot, AndroidPackageSnapshotState,
    };

    if raw_length != ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE as u64
        || reserved != 0
        || user_destination == 0
    {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(image) = crate::process::current_live_user_image_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if !android_package_snapshot_reader_allowed(image) {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if !crate::process::current_user_range_is_writable(
        user_destination,
        ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE,
    ) {
        return complete(frame, Status::BadAddress, 0, 0);
    }

    let Some(catalog) = crate::package_manager::catalog_snapshot() else {
        return complete(frame, Status::Unavailable, 0, 0);
    };
    let state = AndroidPackageSnapshotState::new(
        catalog.formatted,
        crate::package_source::bytes().is_some(),
        catalog.source_used,
        catalog.format_performed,
    );
    let io = AndroidPackageIoCounters::new(
        catalog.boot_io.reads,
        catalog.boot_io.writes,
        catalog.boot_io.flushes,
    );
    let value = match catalog.package {
        Some(package) => AndroidPackageSnapshot::installed(
            state,
            io,
            AndroidPackageInstalledMetadata {
                generation: package.generation,
                version_code: package.version_code,
                apk_length: package.apk_length,
                resources_table_crc32: package.resources_arsc_crc32,
                layout_xml_crc32: package.layout_xml_crc32,
                layout_resource_id: package.layout_resource_id,
                text_resource_id: package.text_resource_id,
                instruction_count: package.launch_instruction_count,
                apk_sha256: package.apk_sha256,
                signer_sha256: package.signer_cert_sha256,
                package_name: package.package.as_bytes(),
                activity_name: package.activity.as_bytes(),
                title: package.launch_title.as_bytes(),
                text: package.launch_text.as_bytes(),
            },
        ),
        None => AndroidPackageSnapshot::empty(state, io),
    };
    let Ok(value) = value else {
        crate::kprintln!(
            "ANDROID_PACKAGE_SNAPSHOT_READ_FAIL image={} reason=kernel_snapshot_noncanonical",
            image.raw(),
        );
        return complete(frame, Status::DataCorrupt, 0, 0);
    };
    let wire = value.encode();
    if crate::arch::aarch64::usercopy::copy_to_user(user_destination, &wire).is_err() {
        panic!("prevalidated Android package snapshot destination faulted");
    }
    crate::kprintln!(
        "ANDROID_PACKAGE_SNAPSHOT_READ_OK image={} bytes={} installed={} generation={} version_code={} package_writes={} authority_granted=0 apk_bytes_exposed=0",
        image.raw(),
        wire.len(),
        u8::from(value.installed_package()),
        value.generation(),
        value.version_code(),
        value.io_counters().writes(),
    );
    complete(frame, Status::Ok, 0, 0)
}

/// Copies ABI 55's complete installed-app directory without granting package
/// bytes, storage identity, or a mutable capability.
#[cfg(feature = "androidbox-multipackage4")]
fn android_package_directory_read(
    frame: *mut TrapFrame,
    user_destination: u64,
    raw_length: u64,
    reserved: u64,
) -> *mut TrapFrame {
    use bndr_abi::{
        ANDROID_PACKAGE_DIRECTORY_CAPACITY, ANDROID_PACKAGE_DIRECTORY_FLAG_FORMATTED,
        ANDROID_PACKAGE_DIRECTORY_MAGIC, ANDROID_PACKAGE_DIRECTORY_VERSION,
        ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE, ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE,
        AndroidPackageInstalledMetadata, AndroidPackageIoCounters, AndroidPackageSnapshot,
        AndroidPackageSnapshotState,
    };

    if raw_length != ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE as u64
        || reserved != 0
        || user_destination == 0
    {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(image) = crate::process::current_live_user_image_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if !matches!(image, UserImageId::Launcher | UserImageId::App) {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if !crate::process::current_user_range_is_writable(
        user_destination,
        ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE,
    ) {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let Some(catalog) = crate::package_manager::directory_snapshot() else {
        return complete(frame, Status::Unavailable, 0, 0);
    };

    if usize::from(catalog.count) > ANDROID_PACKAGE_DIRECTORY_CAPACITY
        || (catalog.formatted && catalog.revision == 0)
        || (!catalog.formatted && (catalog.count != 0 || catalog.revision != 0))
    {
        return complete(frame, Status::DataCorrupt, 0, 0);
    }
    let mut scratch = ANDROID_PACKAGE_DIRECTORY_WIRE_SCRATCH.acquire();
    let wire = scratch.bytes();
    wire.fill(0);
    wire[..8].copy_from_slice(&ANDROID_PACKAGE_DIRECTORY_MAGIC);
    wire[8..12].copy_from_slice(&ANDROID_PACKAGE_DIRECTORY_VERSION.to_le_bytes());
    let flags = if catalog.formatted {
        ANDROID_PACKAGE_DIRECTORY_FLAG_FORMATTED
    } else {
        0
    };
    wire[12..16].copy_from_slice(&flags.to_le_bytes());
    wire[16..24].copy_from_slice(&catalog.revision.to_le_bytes());
    wire[24..26].copy_from_slice(&u16::from(catalog.count).to_le_bytes());
    wire[26..28].copy_from_slice(&(ANDROID_PACKAGE_DIRECTORY_CAPACITY as u16).to_le_bytes());

    for (index, package) in catalog.packages.iter().flatten().enumerate() {
        let value = AndroidPackageSnapshot::installed(
            AndroidPackageSnapshotState::new(true, false, false, false),
            AndroidPackageIoCounters::new(0, 0, 0),
            AndroidPackageInstalledMetadata {
                generation: package.generation,
                version_code: package.version_code,
                apk_length: package.apk_length,
                resources_table_crc32: package.resources_arsc_crc32,
                layout_xml_crc32: package.layout_xml_crc32,
                layout_resource_id: package.layout_resource_id,
                text_resource_id: package.text_resource_id,
                instruction_count: package.launch_instruction_count,
                apk_sha256: package.apk_sha256,
                signer_sha256: package.signer_cert_sha256,
                package_name: package.package.as_bytes(),
                activity_name: package.activity.as_bytes(),
                title: package.launch_title.as_bytes(),
                text: package.launch_text.as_bytes(),
            },
        );
        let Ok(value) = value else {
            return complete(frame, Status::DataCorrupt, 0, 0);
        };
        let entry = value.encode();
        let start = 64 + index * ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE;
        wire[start..start + ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE].copy_from_slice(&entry);
    }
    if crate::arch::aarch64::usercopy::copy_to_user(user_destination, wire).is_err() {
        panic!("prevalidated Android package directory destination faulted");
    }
    crate::kprintln!(
        "ANDROID_PACKAGE_DIRECTORY_READ_OK image={} bytes={} revision={} installed_count={} authority_granted=0 apk_bytes_exposed=0",
        image.raw(),
        wire.len(),
        catalog.revision,
        catalog.count,
    );
    complete(frame, Status::Ok, 0, 0)
}

/// Copies one exact package-bound launcher icon without exposing APK bytes.
#[cfg(feature = "androidbox-icon-resources5")]
#[inline(never)]
fn android_package_icon_read(
    frame: *mut TrapFrame,
    user_destination: u64,
    raw_length: u64,
    raw_selector: u64,
) -> *mut TrapFrame {
    use bndr_abi::{
        ANDROID_PACKAGE_DIRECTORY_CAPACITY, ANDROID_PACKAGE_ICON_FLAG_PRESENT,
        ANDROID_PACKAGE_ICON_HEIGHT, ANDROID_PACKAGE_ICON_MAGIC, ANDROID_PACKAGE_ICON_PIXEL_COUNT,
        ANDROID_PACKAGE_ICON_VERSION, ANDROID_PACKAGE_ICON_WIDTH, ANDROID_PACKAGE_ICON_WIRE_SIZE,
    };
    #[cfg(feature = "androidbox-density-icons7")]
    use bndr_abi::{
        ANDROID_PACKAGE_ICON_FLAG_COLOR_QUANTIZED, ANDROID_PACKAGE_ICON_FLAG_DENSITY_NORMALIZED,
    };

    if raw_length != ANDROID_PACKAGE_ICON_WIRE_SIZE as u64
        || user_destination == 0
        || raw_selector >= ANDROID_PACKAGE_DIRECTORY_CAPACITY as u64
    {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(image) = crate::process::current_live_user_image_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if !matches!(image, UserImageId::Launcher | UserImageId::App) {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if !crate::process::current_user_range_is_writable(
        user_destination,
        ANDROID_PACKAGE_ICON_WIRE_SIZE,
    ) {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let Some(directory) = crate::package_manager::directory_snapshot() else {
        return complete(frame, Status::Unavailable, 0, 0);
    };
    let selector = raw_selector as usize;
    let Some(package) = directory.packages.get(selector).copied().flatten() else {
        return complete(frame, Status::NotFound, 0, 0);
    };
    let mut scratch = ANDROID_PACKAGE_DIRECTORY_WIRE_SCRATCH.acquire();
    let wire = &mut scratch.bytes()[..ANDROID_PACKAGE_ICON_WIRE_SIZE];
    wire.fill(0);
    wire[..8].copy_from_slice(&ANDROID_PACKAGE_ICON_MAGIC);
    wire[8..12].copy_from_slice(&ANDROID_PACKAGE_ICON_VERSION.to_le_bytes());
    wire[16..24].copy_from_slice(&directory.revision.to_le_bytes());
    wire[24..26].copy_from_slice(&(selector as u16).to_le_bytes());
    wire[32..40].copy_from_slice(&package.generation.to_le_bytes());
    wire[40..72].copy_from_slice(&package.apk_sha256);
    if let Some(icon) = package.launcher_icon {
        #[cfg(feature = "androidbox-density-icons7")]
        let flags = ANDROID_PACKAGE_ICON_FLAG_PRESENT
            | if icon.source_density_dpi() != 0 {
                ANDROID_PACKAGE_ICON_FLAG_DENSITY_NORMALIZED
            } else {
                0
            }
            | if icon.color_quantized() {
                ANDROID_PACKAGE_ICON_FLAG_COLOR_QUANTIZED
            } else {
                0
            };
        #[cfg(not(feature = "androidbox-density-icons7"))]
        let flags = ANDROID_PACKAGE_ICON_FLAG_PRESENT;
        wire[12..16].copy_from_slice(&flags.to_le_bytes());
        wire[26..28].copy_from_slice(&(ANDROID_PACKAGE_ICON_WIDTH as u16).to_le_bytes());
        wire[28..30].copy_from_slice(&(ANDROID_PACKAGE_ICON_HEIGHT as u16).to_le_bytes());
        wire[72..76].copy_from_slice(&icon.resource_id().to_le_bytes());
        wire[76..80].copy_from_slice(&icon.png_crc32().to_le_bytes());
        for index in 0..ANDROID_PACKAGE_ICON_PIXEL_COUNT {
            let Some(pixel) = icon.pixel(index) else {
                return complete(frame, Status::DataCorrupt, 0, 0);
            };
            let offset = 80 + index * 4;
            wire[offset..offset + 4].copy_from_slice(&pixel.to_le_bytes());
        }
        #[cfg(feature = "androidbox-density-icons7")]
        {
            wire[1_104..1_106].copy_from_slice(&icon.source_width().to_le_bytes());
            wire[1_106..1_108].copy_from_slice(&icon.source_height().to_le_bytes());
            wire[1_108..1_110].copy_from_slice(&icon.source_density_dpi().to_le_bytes());
        }
    }
    if crate::arch::aarch64::usercopy::copy_to_user(user_destination, wire).is_err() {
        panic!("prevalidated Android package icon destination faulted");
    }
    crate::kprintln!(
        "ANDROID_PACKAGE_ICON_READ_OK image={} selector={} directory_revision={} generation={} present={} resource_id={:#010x} png_crc32={:#010x} bytes={} source_width={} source_height={} source_density_dpi={} color_quantized={} authority_granted=0 apk_bytes_exposed=0",
        image.raw(),
        selector,
        directory.revision,
        package.generation,
        u8::from(package.launcher_icon.is_some()),
        package.launcher_icon.map_or(0, |icon| icon.resource_id()),
        package.launcher_icon.map_or(0, |icon| icon.png_crc32()),
        wire.len(),
        {
            #[cfg(feature = "androidbox-density-icons7")]
            {
                package.launcher_icon.map_or(0, |icon| icon.source_width())
            }
            #[cfg(not(feature = "androidbox-density-icons7"))]
            {
                package.launcher_icon.map_or(0, |_| 16)
            }
        },
        {
            #[cfg(feature = "androidbox-density-icons7")]
            {
                package.launcher_icon.map_or(0, |icon| icon.source_height())
            }
            #[cfg(not(feature = "androidbox-density-icons7"))]
            {
                package.launcher_icon.map_or(0, |_| 16)
            }
        },
        {
            #[cfg(feature = "androidbox-density-icons7")]
            {
                package
                    .launcher_icon
                    .map_or(0, |icon| icon.source_density_dpi())
            }
            #[cfg(not(feature = "androidbox-density-icons7"))]
            {
                0
            }
        },
        {
            #[cfg(feature = "androidbox-density-icons7")]
            {
                u8::from(
                    package
                        .launcher_icon
                        .is_some_and(|icon| icon.color_quantized()),
                )
            }
            #[cfg(not(feature = "androidbox-density-icons7"))]
            {
                0
            }
        },
    );
    complete(frame, Status::Ok, 0, 0)
}

#[cfg(feature = "androidbox-el0-runtime0")]
const fn android_package_relaunch_session_valid(compatible_session_id: u64) -> bool {
    compatible_session_id != 0
}

#[cfg(all(
    feature = "androidbox-apk-install0",
    not(feature = "androidbox-el0-runtime0")
))]
const fn android_package_relaunch_session_valid(compatible_session_id: u64) -> bool {
    compatible_session_id == 0
}

/// Submits or collects one exact, generation-bound durable APK relaunch.
///
/// The syscall itself performs no storage I/O because lower-EL SVC handling
/// runs with IRQ masked. The IRQ-enabled mobile monitor owns that work; this
/// path only copies a canonical request, polls the single slot, and publishes
/// a canonical snapshot after a successful exact retry.
#[cfg(feature = "androidbox-apk-install0")]
fn android_package_relaunch(
    frame: *mut TrapFrame,
    user_exchange: u64,
    raw_length: u64,
    reserved: u64,
) -> *mut TrapFrame {
    use crate::package_manager::{RelaunchError, RelaunchPoll};
    use bndr_abi::{
        ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE, AndroidPackageInstalledMetadata,
        AndroidPackageIoCounters, AndroidPackageRelaunchRequest, AndroidPackageSnapshot,
        AndroidPackageSnapshotState,
    };

    if raw_length != ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE as u64
        || !android_package_relaunch_session_valid(reserved)
        || user_exchange == 0
    {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    if !crate::process::current_is_android_package_launcher() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if !crate::process::current_user_range_is_readable(
        user_exchange,
        ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE,
    ) || !crate::process::current_user_range_is_writable(
        user_exchange,
        ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE,
    ) {
        return complete(frame, Status::BadAddress, 0, 0);
    }

    let mut request_wire = [0_u8; ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE];
    if crate::arch::aarch64::usercopy::copy_from_user(&mut request_wire, user_exchange).is_err() {
        panic!("prevalidated Android package relaunch source faulted");
    }
    let Ok(request) = AndroidPackageRelaunchRequest::decode(&request_wire) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let Some(owner) = crate::scheduler::current_process_id().filter(|owner| *owner != 0) else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let completion = match crate::package_manager::poll_relaunch(owner, reserved, request) {
        RelaunchPoll::Pending => return complete(frame, Status::ShouldWait, 0, 0),
        RelaunchPoll::Ready(Err(error)) => {
            let status = match error {
                RelaunchError::Stale => Status::Conflict,
                RelaunchError::NotInstalled => Status::NotFound,
                RelaunchError::Sequence | RelaunchError::Busy => Status::InvalidState,
                RelaunchError::Corrupt => Status::DataCorrupt,
                RelaunchError::Unsupported => Status::Unsupported,
                RelaunchError::RequiresReset => Status::RequiresReset,
                RelaunchError::OutOfMemory => Status::OutOfMemory,
            };
            return complete(frame, status, 0, 0);
        }
        RelaunchPoll::Ready(Ok(completion)) => completion,
    };
    if completion.request_sequence != request.request_sequence()
        || completion.io.reads == 0
        || completion.io.writes != 0
        || completion.io.flushes != 0
    {
        return complete(frame, Status::DataCorrupt, 0, 0);
    }

    let Some(catalog) = crate::package_manager::catalog_snapshot() else {
        return complete(frame, Status::Unavailable, 0, 0);
    };
    #[cfg(not(feature = "androidbox-multipackage4"))]
    if catalog.package != Some(completion.package) {
        return complete(frame, Status::Conflict, 0, 0);
    }
    #[cfg(feature = "androidbox-multipackage4")]
    {
        let Some(directory) = crate::package_manager::directory_snapshot() else {
            return complete(frame, Status::Unavailable, 0, 0);
        };
        if !directory
            .packages
            .iter()
            .flatten()
            .any(|package| *package == completion.package)
        {
            return complete(frame, Status::Conflict, 0, 0);
        }
    }
    let package = completion.package;
    let state = AndroidPackageSnapshotState::new(
        catalog.formatted,
        crate::package_source::bytes().is_some(),
        catalog.source_used,
        catalog.format_performed,
    );
    let value = AndroidPackageSnapshot::installed(
        state,
        AndroidPackageIoCounters::new(
            completion.io.reads,
            completion.io.writes,
            completion.io.flushes,
        ),
        AndroidPackageInstalledMetadata {
            generation: package.generation,
            version_code: package.version_code,
            apk_length: package.apk_length,
            resources_table_crc32: package.resources_arsc_crc32,
            layout_xml_crc32: package.layout_xml_crc32,
            layout_resource_id: package.layout_resource_id,
            text_resource_id: package.text_resource_id,
            instruction_count: package.launch_instruction_count,
            apk_sha256: package.apk_sha256,
            signer_sha256: package.signer_cert_sha256,
            package_name: package.package.as_bytes(),
            activity_name: package.activity.as_bytes(),
            title: package.launch_title.as_bytes(),
            text: package.launch_text.as_bytes(),
        },
    );
    let Ok(value) = value else {
        return complete(frame, Status::DataCorrupt, 0, 0);
    };
    let response_wire = value.encode();

    #[cfg(feature = "androidbox-el0-runtime0")]
    let execution_owner = {
        use bndr_abi::AndroidPackageImageClaim;

        let Some(execution_owner) =
            crate::process::android_package_execution_process_id_irq_masked()
        else {
            return complete(frame, Status::Unavailable, 0, 0);
        };
        let claim = match AndroidPackageImageClaim::new(
            reserved,
            package.generation,
            package.apk_length,
            package.apk_sha256,
            package.signer_cert_sha256,
        ) {
            Ok(claim) => claim,
            Err(_) => return complete(frame, Status::DataCorrupt, 0, 0),
        };
        if crate::package_manager::publish_image_grant(
            owner,
            execution_owner,
            claim,
            completion.image,
        )
        .is_err()
        {
            return complete(frame, Status::Conflict, 0, 0);
        }
        execution_owner
    };

    if crate::arch::aarch64::usercopy::copy_to_user(user_exchange, &response_wire).is_err() {
        panic!("prevalidated Android package relaunch exchange faulted");
    }
    #[cfg(not(feature = "androidbox-el0-runtime0"))]
    crate::kprintln!(
        "ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner={} request_sequence={} generation={} bytes={} reads={} writes=0 flushes=0 authority_granted=0 apk_bytes_exposed=0",
        owner,
        completion.request_sequence,
        package.generation,
        response_wire.len(),
        completion.io.reads,
    );
    #[cfg(all(
        feature = "androidbox-el0-runtime0",
        not(feature = "androidbox-process0")
    ))]
    crate::kprintln!(
        "ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner={} app_owner={} compatible_session_id={} request_sequence={} generation={} bytes={} reads={} writes=0 flushes=0 authority_granted=app-read-only-vmo apk_bytes_exposed_to_launcher=0 apk_bytes_exposed_to_app=1 storage_authority_granted=0",
        owner,
        execution_owner,
        reserved,
        completion.request_sequence,
        package.generation,
        response_wire.len(),
        completion.io.reads,
    );
    #[cfg(feature = "androidbox-process0")]
    crate::kprintln!(
        "ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner={} android_app_owner={} compatible_session_id={} request_sequence={} generation={} bytes={} reads={} writes=0 flushes=0 authority_granted=android-app-read-only-vmo apk_bytes_exposed_to_launcher=0 apk_bytes_exposed_to_app=0 apk_bytes_exposed_to_android_app=1 storage_authority_granted=0",
        owner,
        execution_owner,
        reserved,
        completion.request_sequence,
        package.generation,
        response_wire.len(),
        completion.io.reads,
    );
    complete(frame, Status::Ok, 0, 0)
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn android_package_uninstall(
    frame: *mut TrapFrame,
    user_exchange: u64,
    raw_length: u64,
    reserved: u64,
) -> *mut TrapFrame {
    use crate::package_manager::{RuntimeUninstallError, RuntimeUninstallPoll};
    use bndr_abi::{
        ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE, AndroidPackageIoCounters,
        AndroidPackageUninstallRequest, AndroidPackageUninstallResult,
    };

    if user_exchange == 0
        || raw_length != ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE as u64
        || reserved != 0
    {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    if !crate::process::current_is_android_package_settings() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if !crate::process::current_user_range_is_readable(
        user_exchange,
        ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE,
    ) || !crate::process::current_user_range_is_writable(
        user_exchange,
        ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE,
    ) {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let mut exchange = [0; ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE];
    if crate::arch::aarch64::usercopy::copy_from_user(&mut exchange, user_exchange).is_err() {
        panic!("prevalidated Android package uninstall exchange faulted");
    }
    let Ok(request) = AndroidPackageUninstallRequest::decode(&exchange) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let Some(owner) = crate::scheduler::current_process_id().filter(|owner| *owner != 0) else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let completion = match crate::package_manager::poll_runtime_uninstall(owner, request) {
        RuntimeUninstallPoll::Pending => return complete(frame, Status::ShouldWait, 0, 0),
        RuntimeUninstallPoll::Ready(Err(error)) => {
            let status = match error {
                RuntimeUninstallError::Stale => Status::Conflict,
                RuntimeUninstallError::NotInstalled => Status::NotFound,
                RuntimeUninstallError::Sequence | RuntimeUninstallError::Busy => {
                    Status::InvalidState
                }
                RuntimeUninstallError::Corrupt => Status::DataCorrupt,
                RuntimeUninstallError::Unsupported => Status::Unsupported,
                RuntimeUninstallError::RequiresReset => Status::RequiresReset,
            };
            return complete(frame, status, 0, 0);
        }
        RuntimeUninstallPoll::Ready(Ok(completion)) => completion,
    };
    if completion.request_sequence != request.request_sequence()
        || completion.operation_id != request.operation_id()
        || completion.last_generation != request.expected_generation()
        || request.expected_generation().checked_add(1) != Some(completion.removal_generation)
    {
        return complete(frame, Status::DataCorrupt, 0, 0);
    }
    let io = AndroidPackageIoCounters::new(
        completion.io.reads,
        completion.io.writes,
        completion.io.flushes,
    );
    let Ok(result) = AndroidPackageUninstallResult::new(
        completion.request_sequence,
        completion.operation_id,
        completion.removal_generation,
        completion.last_generation,
        io,
    ) else {
        return complete(frame, Status::DataCorrupt, 0, 0);
    };
    let wire = result.encode();
    if crate::arch::aarch64::usercopy::copy_to_user(user_exchange, &wire).is_err() {
        panic!("prevalidated Android package uninstall result faulted");
    }
    crate::kprintln!(
        "ANDROID_PACKAGE_RUNTIME_UNINSTALL_COLLECT_OK owner={} request_sequence={} operation_id={} last_generation={} removal_generation={} bytes={} reads={} writes={} flushes={} authority_granted=0 package_store_handle=0 block_handle=0 arbitrary_path=0",
        owner,
        result.request_sequence(),
        result.operation_id(),
        result.last_installed_generation(),
        result.removal_generation(),
        wire.len(),
        result.io_counters().reads(),
        result.io_counters().writes(),
        result.io_counters().flushes(),
    );
    complete(frame, Status::Ok, 0, 0)
}

#[cfg(feature = "androidbox-runtime-install2")]
fn android_package_install_candidate_read(
    frame: *mut TrapFrame,
    user_destination: u64,
    raw_length: u64,
    reserved: u64,
) -> *mut TrapFrame {
    use bndr_abi::{ANDROID_PACKAGE_INSTALL_CANDIDATE_WIRE_SIZE, AndroidPackageInstallCandidate};

    if user_destination == 0
        || raw_length != ANDROID_PACKAGE_INSTALL_CANDIDATE_WIRE_SIZE as u64
        || reserved != 0
    {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(image) = crate::process::current_live_user_image_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if !android_package_install_caller_allowed(image) {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if !crate::process::current_user_range_is_writable(
        user_destination,
        ANDROID_PACKAGE_INSTALL_CANDIDATE_WIRE_SIZE,
    ) {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let candidate = crate::package_manager::install_candidate_snapshot()
        .unwrap_or_else(AndroidPackageInstallCandidate::empty);
    let wire = candidate.encode();
    if crate::arch::aarch64::usercopy::copy_to_user(user_destination, &wire).is_err() {
        panic!("prevalidated Android package install candidate destination faulted");
    }
    crate::kprintln!(
        "ANDROID_PACKAGE_INSTALL_CANDIDATE_READ_OK image={} present={} candidate_id={} action={} expected_generation={} expected_version_code={} version_code={} package={} bytes={} authority_granted=0 apk_bytes_exposed=0 path_exposed=0 package_store_handle=0 block_handle=0",
        image.raw(),
        u8::from(candidate.present()),
        candidate.candidate_id(),
        match candidate.action() {
            Some(bndr_abi::AndroidPackageInstallAction::Install) => "install",
            Some(bndr_abi::AndroidPackageInstallAction::Update) => "update",
            Some(bndr_abi::AndroidPackageInstallAction::Reinstall) => "reinstall",
            None => "none",
        },
        candidate.expected_generation(),
        candidate.expected_version_code(),
        candidate.version_code(),
        if candidate.present() {
            candidate.package_name()
        } else {
            "none"
        },
        wire.len(),
    );
    complete(frame, Status::Ok, 0, 0)
}

#[cfg(feature = "androidbox-runtime-install2")]
fn android_package_install(
    frame: *mut TrapFrame,
    user_exchange: u64,
    raw_length: u64,
    reserved: u64,
) -> *mut TrapFrame {
    use crate::package_manager::{RuntimeInstallError, RuntimeInstallPoll};
    use bndr_abi::{
        ANDROID_PACKAGE_INSTALL_WIRE_SIZE, AndroidPackageInstallRequest,
        AndroidPackageInstallResult, AndroidPackageIoCounters,
    };

    if user_exchange == 0 || raw_length != ANDROID_PACKAGE_INSTALL_WIRE_SIZE as u64 || reserved != 0
    {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(image) = crate::process::current_live_user_image_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if !android_package_install_caller_allowed(image)
        || !crate::process::current_is_android_package_settings()
    {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if !crate::process::current_user_range_is_readable(
        user_exchange,
        ANDROID_PACKAGE_INSTALL_WIRE_SIZE,
    ) || !crate::process::current_user_range_is_writable(
        user_exchange,
        ANDROID_PACKAGE_INSTALL_WIRE_SIZE,
    ) {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let mut exchange = [0; ANDROID_PACKAGE_INSTALL_WIRE_SIZE];
    if crate::arch::aarch64::usercopy::copy_from_user(&mut exchange, user_exchange).is_err() {
        panic!("prevalidated Android package install exchange faulted");
    }
    let Ok(request) = AndroidPackageInstallRequest::decode(&exchange) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let Some(owner) = crate::scheduler::current_process_id().filter(|owner| *owner != 0) else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let completion = match crate::package_manager::poll_runtime_install(owner, request) {
        RuntimeInstallPoll::Pending => return complete(frame, Status::ShouldWait, 0, 0),
        RuntimeInstallPoll::Ready(Err(error)) => {
            let status = match error {
                RuntimeInstallError::Stale => Status::Conflict,
                RuntimeInstallError::NoCandidate => Status::NotFound,
                RuntimeInstallError::Sequence | RuntimeInstallError::Busy => Status::InvalidState,
                RuntimeInstallError::Corrupt => Status::DataCorrupt,
                RuntimeInstallError::Unsupported => Status::Unsupported,
                RuntimeInstallError::RequiresReset => Status::RequiresReset,
            };
            return complete(frame, status, 0, 0);
        }
        RuntimeInstallPoll::Ready(Ok(completion)) => completion,
    };
    if completion.request_sequence != request.request_sequence()
        || completion.operation_id != request.operation_id()
        || completion.candidate_id != request.candidate_id()
        || completion.previous_generation != request.expected_generation()
        || completion.installed_generation
            != request
                .expected_generation()
                .checked_add(1)
                .unwrap_or_default()
        || completion.installed_version_code != request.version_code()
        || completion.apk_length != request.apk_length()
        || Some(completion.action) != request.action()
        || completion.apk_sha256 != *request.apk_sha256()
        || completion.signer_sha256 != *request.signer_sha256()
    {
        return complete(frame, Status::DataCorrupt, 0, 0);
    }
    let io = AndroidPackageIoCounters::new(
        completion.io.reads,
        completion.io.writes,
        completion.io.flushes,
    );
    let Ok(result) = AndroidPackageInstallResult::new(
        completion.request_sequence,
        completion.operation_id,
        completion.candidate_id,
        completion.previous_generation,
        completion.installed_generation,
        completion.installed_version_code,
        completion.apk_length,
        completion.action,
        io,
        completion.apk_sha256,
        completion.signer_sha256,
    ) else {
        return complete(frame, Status::DataCorrupt, 0, 0);
    };
    let wire = result.encode();
    if crate::arch::aarch64::usercopy::copy_to_user(user_exchange, &wire).is_err() {
        panic!("prevalidated Android package install result faulted");
    }
    crate::kprintln!(
        "ANDROID_PACKAGE_RUNTIME_INSTALL_COLLECT_OK owner={} request_sequence={} operation_id={} candidate_id={} previous_generation={} installed_generation={} version_code={} bytes={} reads={} writes={} flushes={} authority_granted=0 apk_bytes_exposed=0 package_store_handle=0 block_handle=0 arbitrary_path=0",
        owner,
        result.request_sequence(),
        result.operation_id(),
        result.candidate_id(),
        result.previous_generation(),
        result.installed_generation(),
        result.installed_version_code(),
        wire.len(),
        result.io_counters().reads(),
        result.io_counters().writes(),
        result.io_counters().flushes(),
    );
    complete(frame, Status::Ok, 0, 0)
}

/// Consumes one exact package-image grant into the authenticated execution
/// process's handle table without creating a transferable capability.
///
/// Before ABI 47 that process is App. Under `androidbox-process0` it is the
/// isolated AndroidApp worker; the App UI host never receives APK bytes.
///
/// Capacity is reserved before the package manager consumes the one-shot grant,
/// so a full handle table leaves the grant intact for an exact retry. Once the
/// grant is consumed, insertion into that reservation is infallible.
#[cfg(feature = "androidbox-el0-runtime0")]
fn android_package_image_claim(
    frame: *mut TrapFrame,
    user_claim: u64,
    raw_length: u64,
    reserved: u64,
) -> *mut TrapFrame {
    use crate::package_manager::AndroidPackageImageGrantError;
    use bndr_abi::{ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE, AndroidPackageImageClaim};

    if raw_length != ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE as u64
        || reserved != 0
        || user_claim == 0
    {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    if !crate::process::current_is_android_package_execution_process() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if !crate::process::current_user_range_is_readable(
        user_claim,
        ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE,
    ) {
        return complete(frame, Status::BadAddress, 0, 0);
    }

    let mut claim_wire = [0_u8; ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE];
    if crate::arch::aarch64::usercopy::copy_from_user(&mut claim_wire, user_claim).is_err() {
        panic!("prevalidated Android package image claim source faulted");
    }
    let Ok(claim) = AndroidPackageImageClaim::decode(&claim_wire) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let Some(owner) = crate::scheduler::current_process_id().filter(|owner| *owner != 0) else {
        return complete(frame, Status::InvalidState, 0, 0);
    };

    let claimed: Result<_, Status> = with_table(|table| {
        let reservation = table.reserve_slot().map_err(handle_error_status)?;
        let image =
            crate::package_manager::take_image_grant(owner, claim).map_err(
                |error| match error {
                    AndroidPackageImageGrantError::Missing => Status::NotFound,
                    AndroidPackageImageGrantError::Mismatch => Status::Conflict,
                    AndroidPackageImageGrantError::StaleOwner => Status::InvalidState,
                    AndroidPackageImageGrantError::Busy => Status::ShouldWait,
                },
            )?;
        let apk_length = image.len();
        if apk_length != claim.apk_length() as usize {
            panic!("validated Android package image grant changed length");
        }
        let handle = reservation.insert_parts(KernelObject::from(image), Rights::READ);
        Ok((handle, apk_length))
    });
    let (handle, apk_length) = match claimed {
        Ok(claimed) => claimed,
        Err(status) => return complete(frame, status, 0, 0),
    };

    #[cfg(not(feature = "androidbox-process0"))]
    crate::kprintln!(
        "ANDROID_PACKAGE_IMAGE_CLAIM_OK owner={} compatible_session_id={} generation={} apk_length={} handle={} rights=READ duplicate=0 transfer=0 map=0 write=0 execute=0 wait=0 storage_authority_granted=0",
        owner,
        claim.compatible_session_id(),
        claim.package_generation(),
        apk_length,
        handle.raw(),
    );
    #[cfg(feature = "androidbox-process0")]
    crate::kprintln!(
        "ANDROID_PACKAGE_IMAGE_CLAIM_OK owner={} compatible_session_id={} generation={} apk_length={} handle={} image=android-app apk_bytes_exposed_to_app=0 apk_bytes_exposed_to_android_app=1 rights=READ duplicate=0 transfer=0 map=0 write=0 execute=0 wait=0 storage_authority_granted=0",
        owner,
        claim.compatible_session_id(),
        claim.package_generation(),
        apk_length,
        handle.raw(),
    );
    complete(
        frame,
        Status::Ok,
        u64::from(handle.raw()),
        apk_length as u64,
    )
}

#[cfg(all(test, feature = "mobile-ui-runtime"))]
mod system_clock_read_tests {
    use super::{system_clock_minute, system_clock_read_arguments_valid};
    use crate::platform::qemu_virt::{PL031_DATA_REGISTER_OFFSET, PL031_RTC_BASE};

    #[test]
    fn system_clock_arguments_are_exactly_three_zero_registers() {
        assert!(system_clock_read_arguments_valid(0, 0, 0));
        for arguments in [(1, 0, 0), (0, 1, 0), (0, 0, 1), (u64::MAX, 0, 0)] {
            assert!(!system_clock_read_arguments_valid(
                arguments.0,
                arguments.1,
                arguments.2
            ));
        }
    }

    #[test]
    fn qemu_virt_pl031_data_register_address_is_stable() {
        assert_eq!(PL031_RTC_BASE, 0x0901_0000);
        assert_eq!(PL031_DATA_REGISTER_OFFSET, 0);
    }

    #[test]
    fn clock_log_epoch_changes_only_at_visible_minute_boundaries() {
        assert_eq!(system_clock_minute(0), 0);
        assert_eq!(system_clock_minute(59), 0);
        assert_eq!(system_clock_minute(60), 1);
        assert_eq!(system_clock_minute(119), 1);
        assert_eq!(system_clock_minute(120), 2);
        assert_ne!(system_clock_minute(u64::MAX), u64::MAX);
    }
}

pub fn reject_svc_immediate(frame: *mut TrapFrame, immediate: u64) -> *mut TrapFrame {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("lower-EL SVC rejection entered with IRQ enabled");
    }
    if !crate::scheduler::validate_current_user_frame(frame) {
        return crate::scheduler::fault_current_user(
            frame,
            crate::scheduler::UserFaultReason::InvalidSyscallContext,
            0,
            0,
        );
    }
    if crate::arch::aarch64::mmu::pan_supported() && !crate::arch::aarch64::mmu::pan_is_enabled() {
        PAN_FAILURES.fetch_add(1, Ordering::Relaxed);
        panic!("lower-EL SVC rejection entered with PAN disabled");
    }
    record_user_asid();
    record_call();
    if immediate == crate::arch::aarch64::exceptions::SVC_KERNEL_SLEEP {
        PRIVATE_SVC_REJECTIONS.fetch_add(1, Ordering::Relaxed);
    }
    complete(frame, Status::Unsupported, 0, 0)
}

/// Records a BufferQueue release committed by process owner-death cleanup.
///
/// This is not a userspace `GraphicsBufferRelease` call, so only the total
/// release-transition ledger advances; call/success counters remain syscall
/// specific.
pub(crate) fn record_graphics_buffer_owner_death_release() {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("graphics owner-death release recorded with IRQ enabled");
    }
    GRAPHICS_BUFFER_RELEASES.fetch_add(1, Ordering::Relaxed);
}

/// Takes a read-only, IRQ-serialized view of ABI 47's authenticated private
/// App ↔ AndroidApp RPC transcript.
#[cfg(feature = "androidbox-process0")]
pub fn android_app_rpc_snapshot() -> AndroidAppRpcSnapshot {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let snapshot = android_app_rpc_trace_mut().snapshot();
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

#[cfg(feature = "androidbox-restart0")]
pub fn android_app_restart_snapshot() -> AndroidAppRestartSnapshot {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let snapshot = android_app_restart_trace_mut().snapshot();
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

/// Records the sole terminal-process witness that may retain the private APK
/// VMO escrow for ABI 48. The Crash RPC must already have armed the exact old
/// generation, and the scheduler/reaper must report a real Faulted exit.
#[cfg(feature = "androidbox-restart0")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn record_android_app_restart_fault_reap(
    process_id: u64,
    exit_code: u64,
    termination_reason: bndr_abi::ProcessTerminationReason,
    fault_reason: u64,
    fault_esr: u64,
    fault_far: u64,
    fault_elr: u64,
    fault_sp_el0: u64,
    code_start: usize,
    code_end: usize,
    stack_start: usize,
    stack_end: usize,
) -> bool {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("AndroidApp restart fault reap recorded with IRQ enabled");
    }
    let restart = android_app_restart_trace_mut();
    let valid = restart.phase == AndroidAppRestartPhase::AwaitRebind
        && restart.errors == 0
        && restart.crash_reads == 1
        && restart.fault_reaps == 0
        && restart.old_worker_pid == process_id
        && exit_code == 0
        && termination_reason == bndr_abi::ProcessTerminationReason::Faulted
        && fault_reason == crate::scheduler::UserFaultReason::SynchronousException as u64
        && fault_esr == 0x0000_0000_9200_0047
        && fault_far == crate::userboot::USER_STACK_GUARD_LOW as u64
        && code_start < code_end
        && fault_elr.is_multiple_of(4)
        && fault_elr >= code_start as u64
        && fault_elr < code_end as u64
        && stack_start == crate::userboot::USER_STACK_START
        && stack_end == crate::userboot::USER_STACK_END
        && fault_sp_el0.is_multiple_of(16)
        && fault_sp_el0 >= stack_start as u64
        && fault_sp_el0 < stack_end as u64;
    if valid {
        restart.fault_reaps = 1;
        crate::kprintln!(
            "ANDROID_APP_FAULT_REAP_OK format=1 abi={} epoch={} old_pid={} exit_code=0 termination_reason=faulted fault_reason={} esr={:#018x} far={:#018x} elr={:#018x} sp={:#018x} witness_valid=1 escrow_retained=1",
            ABI_VERSION,
            restart.epoch,
            process_id,
            fault_reason,
            fault_esr,
            fault_far,
            fault_elr,
            fault_sp_el0,
        );
        true
    } else {
        if restart.phase != AndroidAppRestartPhase::Complete {
            restart.errors = restart.errors.saturating_add(1);
        }
        false
    }
}

pub fn snapshot() -> SyscallSnapshot {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let handles_left = crate::process::init_handle_count();
    let heap_free_now = crate::kernel_heap::stats()
        .unwrap_or_else(|| panic!("kernel heap missing during syscall snapshot"))
        .free_bytes;
    let snapshot = SyscallSnapshot {
        calls: CALLS.load(Ordering::Acquire),
        successes: SUCCESSES.load(Ordering::Acquire),
        errors: ERRORS.load(Ordering::Acquire),
        unknown: UNKNOWN.load(Ordering::Acquire),
        channels_created: CHANNELS_CREATED.load(Ordering::Acquire),
        writes: WRITES.load(Ordering::Acquire),
        reads: READS.load(Ordering::Acquire),
        channel_peeks: CHANNEL_PEEKS.load(Ordering::Acquire),
        channel_peek_scalar: CHANNEL_PEEK_SCALAR.load(Ordering::Acquire),
        channel_peek_bytes: CHANNEL_PEEK_BYTES.load(Ordering::Acquire),
        channel_peek_transfer: CHANNEL_PEEK_TRANSFER.load(Ordering::Acquire),
        channel_envelope_reads: CHANNEL_ENVELOPE_READS.load(Ordering::Acquire),
        channel_envelope_scalar: CHANNEL_ENVELOPE_SCALAR.load(Ordering::Acquire),
        channel_envelope_bytes: CHANNEL_ENVELOPE_BYTES.load(Ordering::Acquire),
        channel_envelope_transfer: CHANNEL_ENVELOPE_TRANSFER.load(Ordering::Acquire),
        channel_envelope_preserved_rejections: CHANNEL_ENVELOPE_PRESERVED_REJECTIONS
            .load(Ordering::Acquire),
        channel_envelope_buffer_too_small: CHANNEL_ENVELOPE_BUFFER_TOO_SMALL
            .load(Ordering::Acquire),
        channel_envelope_bad_address: CHANNEL_ENVELOPE_BAD_ADDRESS.load(Ordering::Acquire),
        channel_envelope_table_full: CHANNEL_ENVELOPE_TABLE_FULL.load(Ordering::Acquire),
        duplicates: DUPLICATES.load(Ordering::Acquire),
        closes: CLOSES.load(Ordering::Acquire),
        stale_rejections: STALE_REJECTIONS.load(Ordering::Acquire),
        rights_denials: RIGHTS_DENIALS.load(Ordering::Acquire),
        last_tag: LAST_TAG.load(Ordering::Acquire),
        last_payload: LAST_PAYLOAD.load(Ordering::Acquire),
        ready: INIT_READY.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-runtime")]
        storage_generation: M55_STORAGE_GENERATION.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-runtime")]
        storage_stages: M55_STORAGE_STAGES.load(Ordering::Acquire),
        failure: INIT_FAILURE.load(Ordering::Acquire),
        pan_failures: PAN_FAILURES.load(Ordering::Acquire),
        private_svc_rejections: PRIVATE_SVC_REJECTIONS.load(Ordering::Acquire),
        should_wait_returns: SHOULD_WAIT_RETURNS.load(Ordering::Acquire),
        register_preserved: REGISTER_PRESERVED.load(Ordering::Acquire),
        stack_round_trip: STACK_ROUND_TRIP.load(Ordering::Acquire),
        user_asid: USER_ASID.load(Ordering::Acquire) as u8,
        byte_writes: BYTE_WRITES.load(Ordering::Acquire),
        byte_reads: BYTE_READS.load(Ordering::Acquire),
        byte_read_rollbacks: BYTE_READ_ROLLBACKS.load(Ordering::Acquire),
        byte_buffer_too_small: BYTE_BUFFER_TOO_SMALL.load(Ordering::Acquire),
        byte_zero_length: BYTE_ZERO_LENGTH.load(Ordering::Acquire),
        transfer_writes: TRANSFER_WRITES.load(Ordering::Acquire),
        transfer_reads: TRANSFER_READS.load(Ordering::Acquire),
        transfer_read_rollbacks: TRANSFER_READ_ROLLBACKS.load(Ordering::Acquire),
        transfer_self_rejections: TRANSFER_SELF_REJECTIONS.load(Ordering::Acquire),
        transfer_order_rejections: TRANSFER_ORDER_REJECTIONS.load(Ordering::Acquire),
        object_wait_calls: OBJECT_WAIT_CALLS.load(Ordering::Acquire),
        object_wait_immediate: OBJECT_WAIT_IMMEDIATE.load(Ordering::Acquire),
        wait_many_calls: WAIT_MANY_CALLS.load(Ordering::Acquire),
        wait_many_immediate: WAIT_MANY_IMMEDIATE.load(Ordering::Acquire),
        wait_many_poll_timeouts: WAIT_MANY_POLL_TIMEOUTS.load(Ordering::Acquire),
        wait_array_calls: WAIT_ARRAY_CALLS.load(Ordering::Acquire),
        wait_array_immediate: WAIT_ARRAY_IMMEDIATE.load(Ordering::Acquire),
        wait_array_poll_timeouts: WAIT_ARRAY_POLL_TIMEOUTS.load(Ordering::Acquire),
        wait_array_invalid_counts: WAIT_ARRAY_INVALID_COUNTS.load(Ordering::Acquire),
        wait_array_bad_addresses: WAIT_ARRAY_BAD_ADDRESSES.load(Ordering::Acquire),
        wait_array_validation_rejections: WAIT_ARRAY_VALIDATION_REJECTIONS.load(Ordering::Acquire),
        wait_array_max_items_observed: WAIT_ARRAY_MAX_ITEMS_OBSERVED.load(Ordering::Acquire),
        wait_array_lowest_multi_ready: WAIT_ARRAY_LOWEST_MULTI_READY.load(Ordering::Acquire),
        events_created: EVENTS_CREATED.load(Ordering::Acquire),
        event_signal_calls: EVENT_SIGNAL_CALLS.load(Ordering::Acquire),
        event_signal_edges: EVENT_SIGNAL_EDGES.load(Ordering::Acquire),
        event_clear_calls: EVENT_CLEAR_CALLS.load(Ordering::Acquire),
        event_clear_edges: EVENT_CLEAR_EDGES.load(Ordering::Acquire),
        event_signal_denials: EVENT_SIGNAL_DENIALS.load(Ordering::Acquire),
        event_wait_calls: EVENT_WAIT_CALLS.load(Ordering::Acquire),
        event_wait_blocks: EVENT_WAIT_BLOCKS.load(Ordering::Acquire),
        event_wait_wakes: EVENT_WAIT_WAKES.load(Ordering::Acquire),
        event_transfer_writes: EVENT_TRANSFER_WRITES.load(Ordering::Acquire),
        event_transfer_reads: EVENT_TRANSFER_READS.load(Ordering::Acquire),
        file_open_calls: FILE_OPEN_CALLS.load(Ordering::Acquire),
        file_open_successes: FILE_OPEN_SUCCESSES.load(Ordering::Acquire),
        vmo_read_calls: VMO_READ_CALLS.load(Ordering::Acquire),
        vmo_read_successes: VMO_READ_SUCCESSES.load(Ordering::Acquire),
        vmo_read_bytes: VMO_READ_BYTES.load(Ordering::Acquire),
        vmo_transfer_writes: VMO_TRANSFER_WRITES.load(Ordering::Acquire),
        vmo_transfer_reads: VMO_TRANSFER_READS.load(Ordering::Acquire),
        #[cfg(feature = "unified-product-manifest-supervision-runtime")]
        service_manifest_open_calls: SERVICE_MANIFEST_OPEN_CALLS.load(Ordering::Acquire),
        #[cfg(feature = "unified-product-manifest-supervision-runtime")]
        service_manifest_open_successes: SERVICE_MANIFEST_OPEN_SUCCESSES.load(Ordering::Acquire),
        #[cfg(feature = "unified-product-manifest-supervision-runtime")]
        service_manifest_open_argument_rejections: SERVICE_MANIFEST_OPEN_ARGUMENT_REJECTIONS
            .load(Ordering::Acquire),
        #[cfg(feature = "unified-product-manifest-supervision-runtime")]
        service_manifest_open_permission_denials: SERVICE_MANIFEST_OPEN_PERMISSION_DENIALS
            .load(Ordering::Acquire),
        #[cfg(feature = "unified-product-verified-manifest-runtime")]
        verified_manifest_attempts: VERIFIED_MANIFEST_ATTEMPTS.load(Ordering::Acquire),
        #[cfg(feature = "unified-product-verified-manifest-runtime")]
        verified_manifest_successes: VERIFIED_MANIFEST_SUCCESSES.load(Ordering::Acquire),
        #[cfg(feature = "unified-product-verified-manifest-runtime")]
        verified_manifest_signature_successes: VERIFIED_MANIFEST_SIGNATURE_SUCCESSES
            .load(Ordering::Acquire),
        #[cfg(feature = "unified-product-verified-manifest-runtime")]
        verified_manifest_signature_rejections: VERIFIED_MANIFEST_SIGNATURE_REJECTIONS
            .load(Ordering::Acquire),
        #[cfg(feature = "unified-product-verified-manifest-runtime")]
        verified_manifest_rollback_rejections: VERIFIED_MANIFEST_ROLLBACK_REJECTIONS
            .load(Ordering::Acquire),
        #[cfg(feature = "unified-product-verified-manifest-runtime")]
        verified_manifest_format_rejections: VERIFIED_MANIFEST_FORMAT_REJECTIONS
            .load(Ordering::Acquire),
        #[cfg(feature = "unified-product-verified-manifest-runtime")]
        verified_manifest_rollback_index: VERIFIED_MANIFEST_ROLLBACK_INDEX.load(Ordering::Acquire),
        #[cfg(feature = "unified-product-verified-manifest-runtime")]
        verified_manifest_digest: [
            VERIFIED_MANIFEST_DIGEST_0.load(Ordering::Acquire),
            VERIFIED_MANIFEST_DIGEST_1.load(Ordering::Acquire),
            VERIFIED_MANIFEST_DIGEST_2.load(Ordering::Acquire),
            VERIFIED_MANIFEST_DIGEST_3.load(Ordering::Acquire),
        ],
        graphics_buffers_created: GRAPHICS_BUFFERS_CREATED.load(Ordering::Acquire),
        graphics_buffer_create_exhaustions: GRAPHICS_BUFFER_CREATE_EXHAUSTIONS
            .load(Ordering::Acquire),
        graphics_buffer_write_calls: GRAPHICS_BUFFER_WRITE_CALLS.load(Ordering::Acquire),
        graphics_buffer_write_bytes: GRAPHICS_BUFFER_WRITE_BYTES.load(Ordering::Acquire),
        graphics_buffer_presents: GRAPHICS_BUFFER_PRESENTS.load(Ordering::Acquire),
        graphics_buffer_mappable_created: GRAPHICS_BUFFER_MAPPABLE_CREATED.load(Ordering::Acquire),
        graphics_buffer_map_calls: GRAPHICS_BUFFER_MAP_CALLS.load(Ordering::Acquire),
        graphics_buffer_map_successes: GRAPHICS_BUFFER_MAP_SUCCESSES.load(Ordering::Acquire),
        graphics_buffer_unmap_calls: GRAPHICS_BUFFER_UNMAP_CALLS.load(Ordering::Acquire),
        graphics_buffer_unmap_successes: GRAPHICS_BUFFER_UNMAP_SUCCESSES.load(Ordering::Acquire),
        graphics_buffer_queue_calls: GRAPHICS_BUFFER_QUEUE_CALLS.load(Ordering::Acquire),
        graphics_buffer_queue_successes: GRAPHICS_BUFFER_QUEUE_SUCCESSES.load(Ordering::Acquire),
        graphics_buffer_acquire_calls: GRAPHICS_BUFFER_ACQUIRE_CALLS.load(Ordering::Acquire),
        graphics_buffer_acquire_successes: GRAPHICS_BUFFER_ACQUIRE_SUCCESSES
            .load(Ordering::Acquire),
        graphics_buffer_release_calls: GRAPHICS_BUFFER_RELEASE_CALLS.load(Ordering::Acquire),
        graphics_buffer_release_successes: GRAPHICS_BUFFER_RELEASE_SUCCESSES
            .load(Ordering::Acquire),
        graphics_buffer_releases: GRAPHICS_BUFFER_RELEASES.load(Ordering::Acquire),
        graphics_buffer_mapped_presents: GRAPHICS_BUFFER_MAPPED_PRESENTS.load(Ordering::Acquire),
        graphics_buffer_validated_pixels: GRAPHICS_BUFFER_VALIDATED_PIXELS.load(Ordering::Acquire),
        graphics_buffer_validated_bytes: GRAPHICS_BUFFER_VALIDATED_BYTES.load(Ordering::Acquire),
        surface_reacquires: SURFACE_REACQUIRES.load(Ordering::Acquire),
        surface_reacquire_old_pid: SURFACE_REACQUIRE_OLD_PID.load(Ordering::Acquire),
        surface_reacquire_new_pid: SURFACE_REACQUIRE_NEW_PID.load(Ordering::Acquire),
        surface_reacquire_old_session: SURFACE_REACQUIRE_OLD_SESSION.load(Ordering::Acquire),
        surface_reacquire_new_session: SURFACE_REACQUIRE_NEW_SESSION.load(Ordering::Acquire),
        surface_reacquire_discarded_input: SURFACE_REACQUIRE_DISCARDED_INPUT
            .load(Ordering::Acquire),
        surface_reacquire_discarded_keys: SURFACE_REACQUIRE_DISCARDED_KEYS.load(Ordering::Acquire),
        surface_reacquire_frozen_cursor_x: SURFACE_REACQUIRE_FROZEN_CURSOR_X
            .load(Ordering::Acquire),
        surface_reacquire_frozen_cursor_y: SURFACE_REACQUIRE_FROZEN_CURSOR_Y
            .load(Ordering::Acquire),
        surface_reacquire_frozen_cursor_pixels: SURFACE_REACQUIRE_FROZEN_CURSOR_PIXELS
            .load(Ordering::Acquire),
        surface_reacquire_frozen_cursor_visible: SURFACE_REACQUIRE_FROZEN_CURSOR_VISIBLE
            .load(Ordering::Acquire),
        surface_reacquire_frozen_cursor_pressed: SURFACE_REACQUIRE_FROZEN_CURSOR_PRESSED
            .load(Ordering::Acquire),
        surface_reacquire_frozen_scanout_valid: SURFACE_REACQUIRE_FROZEN_SCANOUT_VALID
            .load(Ordering::Acquire),
        surface_frame_acquire_calls: SURFACE_FRAME_ACQUIRE_CALLS.load(Ordering::Acquire),
        surface_frame_acquire_successes: SURFACE_FRAME_ACQUIRE_SUCCESSES.load(Ordering::Acquire),
        surface_frame_acquire_should_wait: SURFACE_FRAME_ACQUIRE_SHOULD_WAIT
            .load(Ordering::Acquire),
        surface_frame_acquire_invalid_state: SURFACE_FRAME_ACQUIRE_INVALID_STATE
            .load(Ordering::Acquire),
        surface_key_read_calls: SURFACE_KEY_READ_CALLS.load(Ordering::Acquire),
        surface_key_read_successes: SURFACE_KEY_READ_SUCCESSES.load(Ordering::Acquire),
        surface_key_read_should_wait: SURFACE_KEY_READ_SHOULD_WAIT.load(Ordering::Acquire),
        surface_key_read_peer_closed: SURFACE_KEY_READ_PEER_CLOSED.load(Ordering::Acquire),
        surface_key_read_invalid_argument: SURFACE_KEY_READ_INVALID_ARGUMENT
            .load(Ordering::Acquire),
        surface_key_read_permission_denied: SURFACE_KEY_READ_PERMISSION_DENIED
            .load(Ordering::Acquire),
        surface_key_read_invalid_state: SURFACE_KEY_READ_INVALID_STATE.load(Ordering::Acquire),
        input_acquire_calls: INPUT_ACQUIRE_CALLS.load(Ordering::Acquire),
        input_acquire_successes: INPUT_ACQUIRE_SUCCESSES.load(Ordering::Acquire),
        input_acquire_permission_denied: INPUT_ACQUIRE_PERMISSION_DENIED.load(Ordering::Acquire),
        input_session_info_calls: INPUT_SESSION_INFO_CALLS.load(Ordering::Acquire),
        input_session_info_successes: INPUT_SESSION_INFO_SUCCESSES.load(Ordering::Acquire),
        input_session_info_permission_denied: INPUT_SESSION_INFO_PERMISSION_DENIED
            .load(Ordering::Acquire),
        process_terminate_calls: PROCESS_TERMINATE_CALLS.load(Ordering::Acquire),
        process_terminate_successes: PROCESS_TERMINATE_SUCCESSES.load(Ordering::Acquire),
        process_terminate_not_found: PROCESS_TERMINATE_NOT_FOUND.load(Ordering::Acquire),
        process_terminate_invalid_argument: PROCESS_TERMINATE_INVALID_ARGUMENT
            .load(Ordering::Acquire),
        process_terminate_invalid_state: PROCESS_TERMINATE_INVALID_STATE.load(Ordering::Acquire),
        process_terminate_permission_denied: PROCESS_TERMINATE_PERMISSION_DENIED
            .load(Ordering::Acquire),
        app_lifecycle_trace: app_lifecycle_trace_snapshot(),
        ui_supervisor_trace: ui_supervisor_trace_snapshot(),
        service_ack_reads: SERVICE_ACK_READS.load(Ordering::Acquire),
        service_ack_bitmap: SERVICE_ACK_BITMAP.load(Ordering::Acquire),
        service_done_reads: SERVICE_DONE_READS.load(Ordering::Acquire),
        service_done_bitmap: SERVICE_DONE_BITMAP.load(Ordering::Acquire),
        service_transcript_errors: SERVICE_TRANSCRIPT_ERRORS.load(Ordering::Acquire),
        m15_service_transcript: M15ServiceTranscriptSnapshot {
            phase: M15_SERVICE_PHASE.load(Ordering::Acquire) as u8,
            errors: M15_SERVICE_ERRORS.load(Ordering::Acquire),
            old_instance: M15_SERVICE_OLD_INSTANCE.load(Ordering::Acquire) as u32,
            new_instance: M15_SERVICE_NEW_INSTANCE.load(Ordering::Acquire) as u32,
            transaction_ids: core::array::from_fn(|index| {
                M15_SERVICE_TXIDS[index].load(Ordering::Acquire) as u32
            }),
            done_reads: M15_SERVICE_DONE_READS.load(Ordering::Acquire),
            done_bitmap: M15_SERVICE_DONE_BITMAP.load(Ordering::Acquire) as u8,
        },
        service_cycle_transcript: ServiceCycleTranscriptSnapshot {
            round: M16_SERVICE_CYCLE_ROUND.load(Ordering::Acquire) as u32,
            step: M16_SERVICE_CYCLE_STEP.load(Ordering::Acquire) as u8,
            completed_rounds: M16_SERVICE_CYCLE_COMPLETED_ROUNDS.load(Ordering::Acquire) as u32,
            errors: M16_SERVICE_CYCLE_ERRORS.load(Ordering::Acquire),
            instance: M16_SERVICE_CYCLE_INSTANCE.load(Ordering::Acquire) as u32,
            transaction_ids: core::array::from_fn(|index| {
                M16_SERVICE_CYCLE_TXIDS[index].load(Ordering::Acquire) as u32
            }),
            done_reads: M16_SERVICE_CYCLE_DONE_READS.load(Ordering::Acquire),
            done_bitmap: M16_SERVICE_CYCLE_DONE_BITMAP.load(Ordering::Acquire) as u8,
        },
        identity_transcript: IdentityTranscriptSnapshot {
            identity_done_reads: M17_IDENTITY_DONE_READS.load(Ordering::Acquire),
            acl_done_reads: M17_ACL_DONE_READS.load(Ordering::Acquire),
            errors: M17_IDENTITY_ERRORS.load(Ordering::Acquire),
            provider_pid: M17_PROVIDER_PID.load(Ordering::Acquire),
            client_pid: M17_CLIENT_PID.load(Ordering::Acquire),
            acl_payload: M17_ACL_PAYLOAD.load(Ordering::Acquire),
        },
        multi_client_transcript: MultiClientTranscriptSnapshot {
            ready_bitmap: M18_READY_BITMAP.load(Ordering::Acquire) as u8,
            queued_bitmap: M18_QUEUED_BITMAP.load(Ordering::Acquire) as u8,
            done_bitmap: M18_DONE_BITMAP.load(Ordering::Acquire) as u8,
            errors: M18_ERRORS.load(Ordering::Acquire),
            secondary_client_pid: M18_SECONDARY_CLIENT_PID.load(Ordering::Acquire),
            request_order: M18_REQUEST_ORDER.load(Ordering::Acquire),
        },
        multi_session_transcript: MultiSessionTranscriptSnapshot {
            phase: M20_PHASE.load(Ordering::Acquire) as u8,
            errors: M20_ERRORS.load(Ordering::Acquire),
            secondary_client_pid: M20_SECONDARY_CLIENT_PID.load(Ordering::Acquire),
            manager_attaches: M20_MANAGER_ATTACHES.load(Ordering::Acquire) as u8,
            client_attaches: M20_CLIENT_ATTACHES.load(Ordering::Acquire) as u8,
            revoke_bitmap: M20_REVOKE_BITMAP.load(Ordering::Acquire) as u8,
            stale_revoke_bitmap: M20_STALE_REVOKE_BITMAP.load(Ordering::Acquire) as u8,
            primary_progress_bitmap: M20_PRIMARY_PROGRESS_BITMAP.load(Ordering::Acquire) as u8,
            provider_accepts: M20_PROVIDER_ACCEPTS.load(Ordering::Acquire) as u8,
            provider_echoes: M20_PROVIDER_ECHOES.load(Ordering::Acquire) as u8,
            provider_aborts: M20_PROVIDER_ABORTS.load(Ordering::Acquire) as u8,
            secondary_echoes: M20_SECONDARY_ECHOES.load(Ordering::Acquire) as u8,
            final_idle_bitmap: M20_FINAL_IDLE_BITMAP.load(Ordering::Acquire) as u8,
        },
        handles_left,
        heap_free_baseline: HEAP_FREE_BASELINE.load(Ordering::Acquire),
        heap_free_now,
    };
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

#[cfg(feature = "app-data-runtime")]
pub fn app_data_snapshot() -> AppDataSyscallSnapshot {
    AppDataSyscallSnapshot {
        root_calls: APP_DATA_ROOT_CALLS.load(Ordering::Acquire),
        root_successes: APP_DATA_ROOT_SUCCESSES.load(Ordering::Acquire),
        file_open_calls: APP_DATA_FILE_OPEN_CALLS.load(Ordering::Acquire),
        file_open_successes: APP_DATA_FILE_OPEN_SUCCESSES.load(Ordering::Acquire),
        replace_calls: APP_DATA_REPLACE_CALLS.load(Ordering::Acquire),
        replace_successes: APP_DATA_REPLACE_SUCCESSES.load(Ordering::Acquire),
        directory_create_calls: APP_DATA_DIRECTORY_CREATE_CALLS.load(Ordering::Acquire),
        directory_create_successes: APP_DATA_DIRECTORY_CREATE_SUCCESSES.load(Ordering::Acquire),
        unlink_calls: APP_DATA_UNLINK_CALLS.load(Ordering::Acquire),
        unlink_successes: APP_DATA_UNLINK_SUCCESSES.load(Ordering::Acquire),
        directory_read_calls: APP_DATA_DIRECTORY_READ_CALLS.load(Ordering::Acquire),
        directory_read_successes: APP_DATA_DIRECTORY_READ_SUCCESSES.load(Ordering::Acquire),
        directory_read_eof: APP_DATA_DIRECTORY_READ_EOF.load(Ordering::Acquire),
        permission_denials: APP_DATA_PERMISSION_DENIALS.load(Ordering::Acquire),
        validation_rejections: APP_DATA_VALIDATION_REJECTIONS.load(Ordering::Acquire),
        should_waits: APP_DATA_SHOULD_WAITS.load(Ordering::Acquire),
        conflicts: APP_DATA_CONFLICTS.load(Ordering::Acquire),
        outcome_unknown: APP_DATA_OUTCOME_UNKNOWN.load(Ordering::Acquire),
        corruption_errors: APP_DATA_CORRUPTION_ERRORS.load(Ordering::Acquire),
        no_space: APP_DATA_NO_SPACE.load(Ordering::Acquire),
    }
}

fn process_spawn(
    frame: *mut TrapFrame,
    raw_startup_handle: u64,
    raw_image_id: u64,
    flags: u64,
) -> *mut TrapFrame {
    if !crate::process::current_is_init() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    #[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
    if bndroid_kernel::shutdown::admission_closed() {
        bndroid_kernel::shutdown::record_spawn_rejection();
        return complete(frame, Status::InvalidState, 0, 0);
    }
    if flags != PROCESS_SPAWN_FLAGS_NONE {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(image_id) = UserImageId::from_raw(raw_image_id) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    if !image_id.is_spawnable() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    #[cfg(not(feature = "input-server-runtime"))]
    if image_id == UserImageId::InputServer {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    #[cfg(not(feature = "storage-server-runtime"))]
    if image_id == UserImageId::StorageServer {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let Some(startup_handle) = parse_handle(raw_startup_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    match crate::process::spawn_image(startup_handle, image_id) {
        Ok(id) => complete(frame, Status::Ok, id.raw(), 0),
        Err(crate::process::SpawnError::ShouldWait) => complete(frame, Status::ShouldWait, 0, 0),
        Err(crate::process::SpawnError::OutOfMemory) => complete(frame, Status::OutOfMemory, 0, 0),
        Err(crate::process::SpawnError::NotFound) => complete(frame, Status::NotFound, 0, 0),
        Err(crate::process::SpawnError::PermissionDenied) => {
            complete(frame, Status::PermissionDenied, 0, 0)
        }
        Err(crate::process::SpawnError::InvalidState) => {
            complete(frame, Status::InvalidState, 0, 0)
        }
        Err(crate::process::SpawnError::Image(
            crate::userboot::UserBootError::Frame(
                bndroid_kernel::memory::AllocateError::OutOfMemory,
            )
            | crate::userboot::UserBootError::AddressSpace(
                crate::arch::aarch64::mmu::AddressSpaceError::OutOfMemory,
            ),
        )) => complete(frame, Status::OutOfMemory, 0, 0),
        Err(crate::process::SpawnError::Image(error)) => {
            let _reason = error.as_str();
            complete(frame, Status::InvalidState, 0, 0)
        }
    }
}

fn thread_exit(
    frame: *mut TrapFrame,
    exit_code: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    if crate::process::current_is_init() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if reserved1 != 0 || reserved2 != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    #[cfg(feature = "service-supervisor-runtime")]
    if exit_code & 0xffff_0000_0000_0000 == 0xfa11_0000_0000_0000 {
        crate::kprintln!(
            "SERVICE_SUPERVISOR_CHILD_DIAG pid={} failure={}",
            crate::process::current_live_user_process_id().unwrap_or(0),
            exit_code & u64::from(u32::MAX),
        );
    }
    #[cfg(all(
        feature = "mobile-ui-runtime",
        not(feature = "service-supervisor-runtime")
    ))]
    if exit_code & 0xffff_0000_0000_0000 == 0xfa11_0000_0000_0000 {
        crate::kprintln!(
            "MOBILE_UI_CHILD_DIAG pid={} failure={}",
            crate::process::current_live_user_process_id().unwrap_or(0),
            exit_code & u64::from(u32::MAX),
        );
    }
    crate::scheduler::exit_current_user(frame, exit_code)
}

fn process_wait(
    frame: *mut TrapFrame,
    process_id: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    if !crate::process::current_is_init() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if process_id == 0 || reserved1 != 0 || reserved2 != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    match crate::process::wait_child(process_id) {
        Ok(crate::process::ChildWait::Completed { exit_code, reason }) => {
            complete(frame, Status::Ok, exit_code, reason.raw())
        }
        Ok(crate::process::ChildWait::Live) => {
            crate::scheduler::wait_current_user_process(frame, process_id)
        }
        Err(crate::process::ChildWaitError::NotFound) => complete(frame, Status::NotFound, 0, 0),
        Err(crate::process::ChildWaitError::InvalidState) => {
            complete(frame, Status::InvalidState, 0, 0)
        }
    }
}

/// Completes the deferred half of an accepted blocking `ProcessWait` only
/// after monitor-side teardown has produced the final exit result.
pub(crate) fn complete_process_wait(
    frame: *mut TrapFrame,
    exit_code: u64,
    termination_reason: u64,
) {
    if !crate::arch::aarch64::irq_is_masked()
        || frame.is_null()
        || bndr_abi::ProcessTerminationReason::from_raw(termination_reason).is_none()
    {
        panic!("deferred process-wait completion invariants failed");
    }
    SUCCESSES.fetch_add(1, Ordering::Relaxed);
    unsafe {
        (*frame).x[0] = Status::Ok.raw();
        (*frame).x[1] = exit_code;
        (*frame).x[2] = termination_reason;
    }
}

fn process_terminate(
    frame: *mut TrapFrame,
    process_id: u64,
    flags: u64,
    reserved: u64,
) -> *mut TrapFrame {
    PROCESS_TERMINATE_CALLS.fetch_add(1, Ordering::Relaxed);
    if !crate::process::current_is_init() {
        PROCESS_TERMINATE_PERMISSION_DENIED.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if process_id == 0 || flags != PROCESS_TERMINATE_FLAGS_NONE || reserved != 0 {
        PROCESS_TERMINATE_INVALID_ARGUMENT.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    match crate::process::terminate_child(process_id) {
        Ok(()) => {
            PROCESS_TERMINATE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            complete(frame, Status::Ok, 0, 0)
        }
        Err(crate::process::TerminateChildError::NotFound) => {
            PROCESS_TERMINATE_NOT_FOUND.fetch_add(1, Ordering::Relaxed);
            complete(frame, Status::NotFound, 0, 0)
        }
        Err(crate::process::TerminateChildError::InvalidState) => {
            PROCESS_TERMINATE_INVALID_STATE.fetch_add(1, Ordering::Relaxed);
            complete(frame, Status::InvalidState, 0, 0)
        }
    }
}

#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
fn system_shutdown(
    frame: *mut TrapFrame,
    operation: u64,
    generation: u64,
    flags: u64,
) -> *mut TrapFrame {
    let prepare = operation == SYSTEM_SHUTDOWN_PREPARE;
    let commit = operation == SYSTEM_SHUTDOWN_COMMIT;
    if (!prepare && !commit)
        || generation == 0
        || generation > bndroid_kernel::shutdown::MAX_GENERATION
        || flags != SYSTEM_SHUTDOWN_FLAGS_NONE
    {
        bndroid_kernel::shutdown::record_invalid_argument();
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    if !crate::process::current_is_init() {
        bndroid_kernel::shutdown::record_call(prepare);
        bndroid_kernel::shutdown::record_permission_denied();
        return complete(frame, Status::PermissionDenied, 0, 0);
    }

    if prepare {
        bndroid_kernel::shutdown::record_call(true);
        #[cfg(feature = "unified-product-runtime")]
        let prepare_ready = m67_shutdown_prepare_ready(generation);
        #[cfg(all(
            feature = "resident-platform-shutdown-runtime",
            not(feature = "unified-product-runtime")
        ))]
        let prepare_ready = m66_shutdown_prepare_ready(generation);
        #[cfg(not(feature = "resident-platform-shutdown-runtime"))]
        let prepare_ready = m65_shutdown_prepare_ready(generation);
        if !prepare_ready {
            bndroid_kernel::shutdown::record_not_ready();
            return complete(frame, Status::InvalidState, 0, 0);
        }
        return match bndroid_kernel::shutdown::prepare(generation) {
            Ok(()) => complete(frame, Status::Ok, generation, 0),
            Err(_) => {
                bndroid_kernel::shutdown::record_replay();
                complete(frame, Status::InvalidState, 0, 0)
            }
        };
    }

    if !INIT_READY.load(Ordering::Acquire)
        || M55_STORAGE_GENERATION.load(Ordering::Acquire) & M55_STORAGE_PAYLOAD_MASK != generation
        || {
            #[cfg(feature = "unified-product-runtime")]
            {
                !m67_storage_ready_proof_valid(generation)
            }
            #[cfg(all(
                feature = "resident-platform-shutdown-runtime",
                not(feature = "unified-product-runtime")
            ))]
            {
                !m66_storage_ready_proof_valid(generation)
            }
            #[cfg(not(feature = "resident-platform-shutdown-runtime"))]
            {
                !m65_storage_ready_proof_valid(generation)
            }
        }
    {
        bndroid_kernel::shutdown::record_call(false);
        bndroid_kernel::shutdown::record_not_ready();
        return complete(frame, Status::InvalidState, 0, 0);
    }
    bndroid_kernel::shutdown::record_call(false);
    match bndroid_kernel::shutdown::commit(generation) {
        Ok(()) => complete(frame, Status::Ok, generation, 0),
        Err(_) => {
            bndroid_kernel::shutdown::record_replay();
            complete(frame, Status::InvalidState, 0, 0)
        }
    }
}

#[cfg(feature = "resident-platform-shutdown-runtime")]
fn service_shutdown(
    frame: *mut TrapFrame,
    operation: u64,
    raw_node: u64,
    argument: u64,
) -> *mut TrapFrame {
    let register = operation == SERVICE_SHUTDOWN_REGISTER;
    let quiesce = operation == SERVICE_SHUTDOWN_QUIESCE;
    let Some(node) = ShutdownServiceNode::from_raw(raw_node) else {
        bndroid_kernel::service_shutdown::record_invalid_argument();
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    if (!register && !quiesce)
        || (register && argument != node.dependency_mask())
        || (quiesce && argument != SERVICE_SHUTDOWN_FLAGS_NONE)
    {
        bndroid_kernel::service_shutdown::record_invalid_argument();
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    bndroid_kernel::service_shutdown::record_call(register);

    let Some(pid) = crate::process::current_live_user_process_id() else {
        bndroid_kernel::service_shutdown::record_identity_rejection();
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let Some(image) = crate::process::current_live_user_image_id() else {
        bndroid_kernel::service_shutdown::record_identity_rejection();
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if crate::process::current_is_init()
        || image == UserImageId::StorageServer
        || !crate::process::resident_shutdown_node_identity_valid(pid, node)
    {
        bndroid_kernel::service_shutdown::record_permission_denied();
        return complete(frame, Status::PermissionDenied, 0, 0);
    }

    let phase = bndroid_kernel::shutdown::snapshot().phase;
    if register {
        return match bndroid_kernel::service_shutdown::register(node, pid, image, argument, phase) {
            Ok(mask) => complete(frame, Status::Ok, mask, node.dependency_mask()),
            Err(bndroid_kernel::service_shutdown::RegisterError::InvalidDependency) => {
                complete(frame, Status::InvalidArgument, 0, 0)
            }
            Err(
                bndroid_kernel::service_shutdown::RegisterError::InvalidIdentity
                | bndroid_kernel::service_shutdown::RegisterError::InvalidPhase
                | bndroid_kernel::service_shutdown::RegisterError::Duplicate,
            ) => complete(frame, Status::InvalidState, 0, 0),
        };
    }

    match bndroid_kernel::service_shutdown::quiesce(node, pid, image, phase) {
        Ok(mask) => complete(frame, Status::Ok, mask, node.shutdown_wave()),
        Err(_) => complete(frame, Status::InvalidState, 0, 0),
    }
}

#[cfg(feature = "unified-product-manifest-supervision-runtime")]
fn service_manifest_open(
    frame: *mut TrapFrame,
    flags: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    use bndr_sm::manifest::{
        PRODUCT_SERVICE_MANIFEST_DEPENDENCY_COUNT, PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT,
        PRODUCT_SERVICE_MANIFEST_SIZE,
    };
    use bndroid_kernel::vmo::{Vmo, VmoCreateError};

    SERVICE_MANIFEST_OPEN_CALLS.fetch_add(1, Ordering::Relaxed);
    if !crate::process::current_is_init() {
        SERVICE_MANIFEST_OPEN_PERMISSION_DENIALS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if flags != SERVICE_MANIFEST_OPEN_FLAGS_NONE || reserved1 != 0 || reserved2 != 0 {
        SERVICE_MANIFEST_OPEN_ARGUMENT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::InvalidArgument, 0, 0);
    }

    #[cfg(feature = "unified-product-key-rotation-runtime")]
    let (manifest, manifest_bytes) = {
        use bndr_sm::manifest::{
            KEY_ROTATION_RETIRED_KEY_FIXTURE_GENERATION, KEY_ROTATION_SERVICE_MANIFEST_GENERATION,
            KEY_ROTATION_TRANSITION_SERVICE_MANIFEST_GENERATION,
        };
        use bndr_sm::verified_manifest::{
            KEY_ROTATION_MANIFEST_KEYRING, KEY_ROTATION_POLICY_SHA256,
            PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR, SIGNED_SERVICE_MANIFEST_HEADER_SIZE,
            SIGNED_SERVICE_MANIFEST_SIGNED_SIZE, rsa2048_keyring_sha256,
            verify_signed_service_manifest_with_keyring,
        };

        let preparation = KEY_ROTATION_MANIFEST_PREPARATION
            .snapshot()
            .unwrap_or_else(|| panic!("key-rotation BMS1 requested before durable preparation"));
        let verified = verify_signed_service_manifest_with_keyring(
            VERIFIED_MANIFEST_ARTIFACT,
            &KEY_ROTATION_MANIFEST_KEYRING,
            PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
        )
        .unwrap_or_else(|_| panic!("prepared key-rotation BMS1 failed repeat validation"));
        if !matches!(
            verified.manifest().generation(),
            KEY_ROTATION_TRANSITION_SERVICE_MANIFEST_GENERATION
                | KEY_ROTATION_SERVICE_MANIFEST_GENERATION
                | KEY_ROTATION_RETIRED_KEY_FIXTURE_GENERATION
        ) || preparation.artifact_index != verified.rollback_index()
            || preparation.committed_floor < verified.rollback_index()
            || preparation.binding.key_id != verified.key_id()
            || preparation.binding.policy_sha256 != KEY_ROTATION_POLICY_SHA256
            || rsa2048_keyring_sha256(&KEY_ROTATION_MANIFEST_KEYRING)
                != Ok(KEY_ROTATION_POLICY_SHA256)
            || VERIFIED_MANIFEST_ROLLBACK_INDEX.load(Ordering::Acquire)
                != u64::from(verified.rollback_index())
        {
            panic!("prepared key-rotation BMS1 authorization changed before VMO publication");
        }
        let manifest = *verified.manifest();
        (
            manifest,
            &VERIFIED_MANIFEST_ARTIFACT
                [SIGNED_SERVICE_MANIFEST_HEADER_SIZE..SIGNED_SERVICE_MANIFEST_SIGNED_SIZE],
        )
    };
    #[cfg(all(
        feature = "unified-product-persistent-rollback-runtime",
        not(feature = "unified-product-key-rotation-runtime")
    ))]
    let (manifest, manifest_bytes) = {
        use bndr_sm::manifest::PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION;
        use bndr_sm::verified_manifest::{
            PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR, PERSISTENT_MANIFEST_RSA2048_KEY2,
            SIGNED_SERVICE_MANIFEST_HEADER_SIZE, SIGNED_SERVICE_MANIFEST_SIGNED_SIZE,
            verify_signed_service_manifest,
        };

        let preparation = PERSISTENT_MANIFEST_PREPARATION
            .snapshot()
            .unwrap_or_else(|| panic!("persistent BMS1 requested before durable preparation"));
        let verified = verify_signed_service_manifest(
            VERIFIED_MANIFEST_ARTIFACT,
            &PERSISTENT_MANIFEST_RSA2048_KEY2,
            PERSISTENT_MANIFEST_BOOTSTRAP_ROLLBACK_FLOOR,
        )
        .unwrap_or_else(|_| panic!("prepared immutable BMS1 failed repeat validation"));
        if verified.manifest().generation() != PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION
            || preparation.artifact_index != verified.rollback_index()
            || preparation.committed_floor < verified.rollback_index()
            || preparation.binding.key_id != verified.key_id()
            || VERIFIED_MANIFEST_ROLLBACK_INDEX.load(Ordering::Acquire)
                != u64::from(verified.rollback_index())
        {
            panic!("prepared persistent BMS1 authorization changed before VMO publication");
        }
        let manifest = *verified.manifest();
        (
            manifest,
            &VERIFIED_MANIFEST_ARTIFACT
                [SIGNED_SERVICE_MANIFEST_HEADER_SIZE..SIGNED_SERVICE_MANIFEST_SIGNED_SIZE],
        )
    };
    #[cfg(all(
        feature = "unified-product-verified-manifest-runtime",
        not(feature = "unified-product-persistent-rollback-runtime")
    ))]
    let (manifest, manifest_bytes) = {
        use bndr_sm::manifest::VERIFIED_SERVICE_MANIFEST_GENERATION;
        use bndr_sm::verified_manifest::{
            PRODUCT_MANIFEST_ROLLBACK_FLOOR, PRODUCT_MANIFEST_RSA2048_KEY1,
            SIGNED_SERVICE_MANIFEST_HEADER_SIZE, SIGNED_SERVICE_MANIFEST_SIGNED_SIZE,
            SignedManifestError, verify_signed_service_manifest,
        };

        VERIFIED_MANIFEST_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        let verified = match verify_signed_service_manifest(
            VERIFIED_MANIFEST_ARTIFACT,
            &PRODUCT_MANIFEST_RSA2048_KEY1,
            PRODUCT_MANIFEST_ROLLBACK_FLOOR,
        ) {
            Ok(verified) => verified,
            Err(SignedManifestError::Signature) => {
                VERIFIED_MANIFEST_SIGNATURE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                crate::kprintln!(
                    "VERIFIED_MANIFEST_REJECTED format=1 reason=signature key_id=1 rollback_floor=2 init_ready=0 manifest_published=0"
                );
                return complete(frame, Status::DataCorrupt, 0, 0);
            }
            Err(SignedManifestError::Rollback { actual, minimum }) => {
                VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
                VERIFIED_MANIFEST_ROLLBACK_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                crate::kprintln!(
                    "VERIFIED_MANIFEST_REJECTED format=1 reason=rollback key_id=1 rollback_index={} rollback_floor={} signature_valid=1 init_ready=0 manifest_published=0",
                    actual,
                    minimum,
                );
                return complete(frame, Status::DataCorrupt, 0, 0);
            }
            Err(
                SignedManifestError::Manifest(_) | SignedManifestError::GenerationMismatch { .. },
            ) => {
                VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
                VERIFIED_MANIFEST_FORMAT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                crate::kprintln!(
                    "VERIFIED_MANIFEST_REJECTED format=1 reason=payload key_id=1 rollback_floor=2 signature_valid=1 init_ready=0 manifest_published=0"
                );
                return complete(frame, Status::DataCorrupt, 0, 0);
            }
            Err(_) => {
                VERIFIED_MANIFEST_FORMAT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                crate::kprintln!(
                    "VERIFIED_MANIFEST_REJECTED format=1 reason=envelope key_id=1 rollback_floor=2 init_ready=0 manifest_published=0"
                );
                return complete(frame, Status::DataCorrupt, 0, 0);
            }
        };
        VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
        VERIFIED_MANIFEST_SUCCESSES.fetch_add(1, Ordering::Relaxed);
        VERIFIED_MANIFEST_ROLLBACK_INDEX
            .store(u64::from(verified.rollback_index()), Ordering::Release);
        store_verified_manifest_digest(verified.signed_region_sha256());
        let manifest = *verified.manifest();
        if manifest.generation() != VERIFIED_SERVICE_MANIFEST_GENERATION {
            panic!("verified BMF1 generation changed after BMS1 validation");
        }
        (
            manifest,
            &VERIFIED_MANIFEST_ARTIFACT
                [SIGNED_SERVICE_MANIFEST_HEADER_SIZE..SIGNED_SERVICE_MANIFEST_SIGNED_SIZE],
        )
    };
    #[cfg(not(feature = "unified-product-verified-manifest-runtime"))]
    let (manifest, manifest_bytes) = {
        use bndr_sm::manifest::{
            PRODUCT_SERVICE_MANIFEST_BYTES, SERVICE_MANIFEST_GENERATION, ServiceManifest,
        };

        let manifest = ServiceManifest::decode(&PRODUCT_SERVICE_MANIFEST_BYTES)
            .unwrap_or_else(|_| panic!("kernel-owned BMF1 service manifest is invalid"));
        if manifest.generation() != SERVICE_MANIFEST_GENERATION {
            panic!("kernel-owned BMF1 service manifest generation changed");
        }
        (manifest, PRODUCT_SERVICE_MANIFEST_BYTES.as_slice())
    };

    if manifest.wire_size() != PRODUCT_SERVICE_MANIFEST_SIZE
        || manifest.service_count() != PRODUCT_SERVICE_MANIFEST_SERVICE_COUNT
        || manifest.dependency_count() != PRODUCT_SERVICE_MANIFEST_DEPENDENCY_COUNT
    {
        panic!("published BMF1 service manifest contract changed");
    }
    let vmo = match Vmo::try_from_slice(manifest_bytes) {
        Ok(vmo) => vmo,
        Err(VmoCreateError::OutOfMemory) => {
            return complete(frame, Status::OutOfMemory, 0, 0);
        }
        Err(VmoCreateError::TooLarge) => {
            panic!("bounded BMF1 service manifest exceeds the immutable VMO limit")
        }
    };
    match with_table(|table| table.insert(KernelObject::from(vmo), Rights::VMO_DEFAULT)) {
        Ok(handle) => {
            SERVICE_MANIFEST_OPEN_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            complete(
                frame,
                Status::Ok,
                u64::from(handle.raw()),
                PRODUCT_SERVICE_MANIFEST_SIZE as u64,
            )
        }
        Err(error) => complete(frame, handle_error_status(error), 0, 0),
    }
}

#[cfg(feature = "unified-product-verified-manifest-runtime")]
fn store_verified_manifest_digest(digest: &[u8; 32]) {
    let words = sha256_words(digest);
    VERIFIED_MANIFEST_DIGEST_0.store(words[0], Ordering::Release);
    VERIFIED_MANIFEST_DIGEST_1.store(words[1], Ordering::Release);
    VERIFIED_MANIFEST_DIGEST_2.store(words[2], Ordering::Release);
    VERIFIED_MANIFEST_DIGEST_3.store(words[3], Ordering::Release);
}

#[cfg(feature = "unified-product-verified-manifest-runtime")]
fn sha256_words(digest: &[u8; 32]) -> [u64; 4] {
    [
        u64::from_be_bytes(digest[0..8].try_into().unwrap()),
        u64::from_be_bytes(digest[8..16].try_into().unwrap()),
        u64::from_be_bytes(digest[16..24].try_into().unwrap()),
        u64::from_be_bytes(digest[24..32].try_into().unwrap()),
    ]
}

#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
fn maintenance_session_open(
    frame: *mut TrapFrame,
    flags: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    MAINTENANCE_SESSION_OPEN_CALLS.fetch_add(1, Ordering::Relaxed);
    if !crate::process::current_is_init() {
        MAINTENANCE_SESSION_OPEN_PERMISSION_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if flags != bndr_abi::MAINTENANCE_SESSION_OPEN_FLAGS_NONE || reserved1 != 0 || reserved2 != 0 {
        MAINTENANCE_SESSION_OPEN_ARGUMENT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(evidence) = MAINTENANCE_AUTHORIZATION_PREPARATION.snapshot() else {
        MAINTENANCE_SESSION_OPEN_STATE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if MAINTENANCE_SESSION_OPENED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        MAINTENANCE_SESSION_OPEN_STATE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::InvalidState, 0, 0);
    }
    MAINTENANCE_SESSION_OPEN_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    complete(
        frame,
        Status::Ok,
        evidence.binding.sequence,
        evidence.binding.operations,
    )
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn service_supervisor_report(
    frame: *mut TrapFrame,
    operation: u64,
    argument1: u64,
    argument2: u64,
) -> *mut TrapFrame {
    use bndroid_kernel::event_supervision_trace::{ProcessContext, ReportError};

    if !crate::process::current_is_init() {
        bndroid_kernel::event_supervision_trace::record_permission_denied();
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if operation == bndr_abi::SERVICE_SUPERVISOR_REPORT_UI_CONVERGENCE_QUERY {
        if argument1 != 0 || argument2 != 0 {
            return complete(frame, Status::InvalidArgument, 0, 0);
        }
        let converged = bndroid_kernel::unified_product::ui_converged();
        bndroid_kernel::event_supervision_trace::record_ui_query(converged);
        return complete(
            frame,
            if converged {
                Status::Ok
            } else {
                Status::ShouldWait
            },
            0,
            0,
        );
    }
    #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
    if !MAINTENANCE_SESSION_OPENED.load(Ordering::Acquire) {
        MAINTENANCE_REPORT_GATE_DENIALS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let processes = crate::process::snapshot();
    let current_storage_pid =
        crate::process::unique_live_process_id_for_image(UserImageId::StorageServer).unwrap_or(0);
    let context = ProcessContext {
        current_storage_pid,
        created: processes.created,
        exited: processes.exited,
        reaped: processes.reaped,
        terminated_exited: processes.terminated_exited,
        terminated_faulted: processes.terminated_faulted,
        terminated_killed: processes.terminated_killed,
        live: processes.live,
        storage_server_live: processes.storage_server_live,
    };
    match bndroid_kernel::event_supervision_trace::report(operation, argument1, argument2, context)
    {
        Ok(()) => complete(frame, Status::Ok, 0, 0),
        Err(ReportError::InvalidArgument) => complete(frame, Status::InvalidArgument, 0, 0),
        Err(ReportError::InvalidState) => complete(frame, Status::InvalidState, 0, 0),
    }
}

fn file_open_at(
    frame: *mut TrapFrame,
    raw_root: u64,
    user_path: u64,
    raw_path_length: u64,
) -> *mut TrapFrame {
    FILE_OPEN_CALLS.fetch_add(1, Ordering::Relaxed);
    let Some(root) = parse_handle(raw_root) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let root_principal: Result<Option<u64>, Status> = with_table(|table| {
        let object = table.get(root, Rights::READ).map_err(|error| {
            if error == HandleError::AccessDenied {
                RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                #[cfg(feature = "app-data-runtime")]
                APP_DATA_PERMISSION_DENIALS.fetch_add(1, Ordering::Relaxed);
            }
            handle_error_status(error)
        })?;
        if object.is_system_directory() {
            return Ok(None);
        }
        #[cfg(feature = "app-data-runtime")]
        if let Some(directory) = object.as_app_data_directory() {
            return Ok(Some(directory.principal().raw()));
        }
        Err(Status::InvalidState)
    });
    let root_principal = match root_principal {
        Ok(principal) => principal,
        Err(status) => return complete(frame, status, 0, 0),
    };
    if raw_path_length == 0 || raw_path_length > SYSTEM_FILE_PATH_MAX_BYTES as u64 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let path_length = raw_path_length as usize;
    let mut path_bytes = [0_u8; SYSTEM_FILE_PATH_MAX_BYTES];
    if crate::arch::aarch64::usercopy::copy_from_user(&mut path_bytes[..path_length], user_path)
        .is_err()
    {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let path = match core::str::from_utf8(&path_bytes[..path_length]) {
        Ok(path) => path,
        Err(_) => return complete(frame, Status::InvalidArgument, 0, 0),
    };
    #[cfg(feature = "app-data-runtime")]
    if let Some(principal) = root_principal {
        APP_DATA_FILE_OPEN_CALLS.fetch_add(1, Ordering::Relaxed);
        let path = match validate_app_data_path(path.as_bytes()) {
            Ok(path) => path,
            Err(_) => {
                APP_DATA_VALIDATION_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                return complete_app_data(frame, Status::InvalidArgument, 0, 0);
            }
        };
        return file_open_app_data(frame, principal, path);
    }
    #[cfg(not(feature = "app-data-runtime"))]
    let _ = root_principal;
    let (vmo, file_size) = match bndroid_kernel::system_files::open(path) {
        Ok(opened) => opened,
        Err(bndroid_kernel::system_files::OpenError::NotReady) => {
            return complete(frame, Status::Unavailable, 0, 0);
        }
        Err(bndroid_kernel::system_files::OpenError::InvalidPath) => {
            return complete(frame, Status::InvalidArgument, 0, 0);
        }
        Err(bndroid_kernel::system_files::OpenError::NotFound) => {
            return complete(frame, Status::NotFound, 0, 0);
        }
    };
    match with_table(|table| table.insert(KernelObject::from(vmo), Rights::VMO_DEFAULT)) {
        Ok(handle) => {
            FILE_OPEN_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            complete(frame, Status::Ok, u64::from(handle.raw()), file_size as u64)
        }
        Err(error) => complete(frame, handle_error_status(error), 0, 0),
    }
}

#[cfg(feature = "app-data-runtime")]
fn file_open_app_data(frame: *mut TrapFrame, principal: u64, path: &str) -> *mut TrapFrame {
    use bndroid_kernel::vmo::{Vmo, VmoCreateError};

    let owner = authenticated_sender_pid();
    let polled =
        crate::app_data::poll_read(owner, principal, path, |completion| match completion {
            Err(error) => {
                crate::app_data::ReadDisposition::Consume(Err(app_data_error_status(error)))
            }
            Ok((bytes, read)) => match Vmo::try_from_slice(bytes) {
                Err(VmoCreateError::OutOfMemory) => {
                    crate::app_data::ReadDisposition::Preserve(Err(Status::OutOfMemory))
                }
                Err(VmoCreateError::TooLarge) => {
                    crate::app_data::ReadDisposition::Consume(Err(Status::DataCorrupt))
                }
                Ok(vmo) => match with_table(|table| {
                    table.insert(KernelObject::from(vmo), Rights::VMO_DEFAULT)
                }) {
                    Ok(handle) => {
                        crate::app_data::ReadDisposition::Consume(Ok((handle, read.bytes)))
                    }
                    Err(error) => {
                        crate::app_data::ReadDisposition::Preserve(Err(handle_error_status(error)))
                    }
                },
            },
        });
    match polled {
        Ok(None) => complete_app_data(frame, Status::ShouldWait, 0, 0),
        Ok(Some(Ok((handle, bytes)))) => {
            FILE_OPEN_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            APP_DATA_FILE_OPEN_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            complete_app_data(frame, Status::Ok, u64::from(handle.raw()), bytes as u64)
        }
        Ok(Some(Err(status))) => complete_app_data(frame, status, 0, 0),
        Err(error) => complete_app_data(frame, app_data_error_status(error), 0, 0),
    }
}

#[cfg(feature = "app-data-runtime")]
fn app_data_root_open(
    frame: *mut TrapFrame,
    reserved0: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    if crate::process::current_live_user_image_id() != Some(UserImageId::App) {
        APP_DATA_ROOT_CALLS.fetch_add(1, Ordering::Relaxed);
        return complete_app_data(frame, Status::PermissionDenied, 0, 0);
    }
    if reserved0 != 0 || reserved1 != 0 || reserved2 != 0 {
        APP_DATA_ROOT_CALLS.fetch_add(1, Ordering::Relaxed);
        return complete_app_data(frame, Status::InvalidArgument, 0, 0);
    }
    if !crate::app_data::snapshot().initialized {
        APP_DATA_ROOT_CALLS.fetch_add(1, Ordering::Relaxed);
        return complete_app_data(frame, Status::Unavailable, 0, 0);
    }
    if !crate::app_data::runtime_ready() {
        return complete_app_data(frame, Status::ShouldWait, 0, 0);
    }
    APP_DATA_ROOT_CALLS.fetch_add(1, Ordering::Relaxed);
    let principal = AppDataPrincipal::PRIMARY_APP;
    match with_table(|table| {
        table.insert(
            KernelObject::from(AppDataDirectoryCapability::new(principal)),
            Rights::APP_DATA_ROOT_DEFAULT,
        )
    }) {
        Ok(handle) => {
            APP_DATA_ROOT_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            complete_app_data(frame, Status::Ok, u64::from(handle.raw()), principal.raw())
        }
        Err(error) => complete_app_data(frame, handle_error_status(error), 0, 0),
    }
}

#[cfg(feature = "app-data-runtime")]
fn file_replace_at(
    frame: *mut TrapFrame,
    raw_root: u64,
    user_request: u64,
    raw_request_length: u64,
) -> *mut TrapFrame {
    APP_DATA_REPLACE_CALLS.fetch_add(1, Ordering::Relaxed);
    let principal = match app_data_root_principal(raw_root, Rights::WRITE) {
        Ok(principal) => principal,
        Err(status) => return complete_app_data(frame, status, 0, 0),
    };
    if raw_request_length != FILE_REPLACE_REQUEST_WIRE_SIZE as u64 {
        return complete_app_data(frame, Status::InvalidArgument, 0, 0);
    }
    let mut wire = [0_u8; FILE_REPLACE_REQUEST_WIRE_SIZE];
    if crate::arch::aarch64::usercopy::copy_from_user(&mut wire, user_request).is_err() {
        return complete_app_data(frame, Status::BadAddress, 0, 0);
    }
    let request = match FileReplaceRequest::decode(&wire) {
        Ok(request) => request,
        Err(_) => return complete_app_data(frame, Status::InvalidArgument, 0, 0),
    };
    let value_length = request.value_length();
    if value_length > APP_DATA_FILE_MAX_BYTES {
        return complete_app_data(frame, Status::InvalidArgument, 0, 0);
    }
    let mut value = [0_u8; APP_DATA_FILE_MAX_BYTES];
    if value_length != 0
        && (!crate::process::current_user_range_is_readable(request.value_pointer(), value_length)
            || crate::arch::aarch64::usercopy::copy_from_user(
                &mut value[..value_length],
                request.value_pointer(),
            )
            .is_err())
    {
        return complete_app_data(frame, Status::BadAddress, 0, 0);
    }
    let condition = match request.cas() {
        FileReplaceCas::Any => ReplaceCondition::Any,
        FileReplaceCas::CreateOnly => ReplaceCondition::Absent,
        FileReplaceCas::Exact(generation) => ReplaceCondition::Generation(generation),
    };
    let owner = authenticated_sender_pid();
    match crate::app_data::poll_replace(
        owner,
        principal,
        request.path(),
        &value[..value_length],
        condition,
    ) {
        crate::app_data::Poll::Pending => complete_app_data(frame, Status::ShouldWait, 0, 0),
        crate::app_data::Poll::Ready(Ok(mutation)) => {
            let generation = mutation
                .entry_revision
                .unwrap_or_else(|| panic!("successful AppData replace omitted its file revision"));
            if generation == 0 || generation != mutation.committed_generation {
                panic!("successful AppData replace published inconsistent generations");
            }
            APP_DATA_REPLACE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            complete_app_data(frame, Status::Ok, generation, 0)
        }
        crate::app_data::Poll::Ready(Err(error)) => {
            complete_app_data(frame, app_data_error_status(error), 0, 0)
        }
    }
}

#[cfg(feature = "app-data-runtime")]
fn directory_create_at(
    frame: *mut TrapFrame,
    raw_root: u64,
    user_path: u64,
    raw_path_length: u64,
) -> *mut TrapFrame {
    APP_DATA_DIRECTORY_CREATE_CALLS.fetch_add(1, Ordering::Relaxed);
    let principal = match app_data_root_principal(raw_root, Rights::WRITE) {
        Ok(principal) => principal,
        Err(status) => return complete_app_data(frame, status, 0, 0),
    };
    let (path_bytes, path_length) = match copy_app_data_path(user_path, raw_path_length) {
        Ok(path) => path,
        Err(status) => return complete_app_data(frame, status, 0, 0),
    };
    let path = core::str::from_utf8(&path_bytes[..path_length])
        .unwrap_or_else(|_| panic!("validated AppData directory path lost UTF-8"));
    match crate::app_data::poll_create_directory(authenticated_sender_pid(), principal, path) {
        crate::app_data::Poll::Pending => complete_app_data(frame, Status::ShouldWait, 0, 0),
        crate::app_data::Poll::Ready(Ok(_)) => {
            APP_DATA_DIRECTORY_CREATE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            complete_app_data(frame, Status::Ok, 0, 0)
        }
        crate::app_data::Poll::Ready(Err(error)) => {
            complete_app_data(frame, app_data_error_status(error), 0, 0)
        }
    }
}

#[cfg(feature = "app-data-runtime")]
fn unlink_at(
    frame: *mut TrapFrame,
    raw_root: u64,
    user_path: u64,
    raw_path_length: u64,
) -> *mut TrapFrame {
    APP_DATA_UNLINK_CALLS.fetch_add(1, Ordering::Relaxed);
    let principal = match app_data_root_principal(raw_root, Rights::WRITE) {
        Ok(principal) => principal,
        Err(status) => return complete_app_data(frame, status, 0, 0),
    };
    let (path_bytes, path_length) = match copy_app_data_path(user_path, raw_path_length) {
        Ok(path) => path,
        Err(status) => return complete_app_data(frame, status, 0, 0),
    };
    let path = core::str::from_utf8(&path_bytes[..path_length])
        .unwrap_or_else(|_| panic!("validated AppData unlink path lost UTF-8"));
    match crate::app_data::poll_unlink(authenticated_sender_pid(), principal, path) {
        crate::app_data::Poll::Pending => complete_app_data(frame, Status::ShouldWait, 0, 0),
        crate::app_data::Poll::Ready(Ok(_)) => {
            APP_DATA_UNLINK_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            complete_app_data(frame, Status::Ok, 0, 0)
        }
        crate::app_data::Poll::Ready(Err(error)) => {
            complete_app_data(frame, app_data_error_status(error), 0, 0)
        }
    }
}

#[cfg(feature = "app-data-runtime")]
fn directory_read_at(
    frame: *mut TrapFrame,
    raw_root: u64,
    raw_cursor: u64,
    user_destination: u64,
) -> *mut TrapFrame {
    APP_DATA_DIRECTORY_READ_CALLS.fetch_add(1, Ordering::Relaxed);
    let principal = match app_data_root_principal(raw_root, Rights::READ) {
        Ok(principal) => principal,
        Err(status) => return complete_app_data(frame, status, 0, 0),
    };
    let Ok(cursor) = u32::try_from(raw_cursor) else {
        return complete_app_data(frame, Status::InvalidArgument, 0, 0);
    };
    if cursor as usize > APP_DATA_DIRECTORY_MAX_ENTRIES {
        return complete_app_data(frame, Status::InvalidArgument, 0, 0);
    }
    if !crate::process::current_user_range_is_writable(
        user_destination,
        APP_DATA_DIRECTORY_ENTRY_WIRE_SIZE,
    ) {
        return complete_app_data(frame, Status::BadAddress, 0, 0);
    }
    match crate::app_data::poll_directory(authenticated_sender_pid(), principal, cursor) {
        crate::app_data::Poll::Pending => complete_app_data(frame, Status::ShouldWait, 0, 0),
        crate::app_data::Poll::Ready(Err(AppDataError::NotFound)) => {
            APP_DATA_DIRECTORY_READ_EOF.fetch_add(1, Ordering::Relaxed);
            complete_app_data(frame, Status::NotFound, 0, 0)
        }
        crate::app_data::Poll::Ready(Err(error)) => {
            complete_app_data(frame, app_data_error_status(error), 0, 0)
        }
        crate::app_data::Poll::Ready(Ok(completion)) => {
            let core_entry = completion.entry;
            let entry = match core_entry.kind {
                bndr_appdata::EntryKind::File => AppDataDirectoryEntry::file(
                    core_entry.name().as_bytes(),
                    core_entry.revision,
                    core_entry.length as usize,
                ),
                bndr_appdata::EntryKind::Directory => {
                    AppDataDirectoryEntry::directory(core_entry.name().as_bytes())
                }
            }
            .unwrap_or_else(|_| panic!("AppData core emitted a non-canonical directory entry"));
            let wire = entry.encode();
            if crate::arch::aarch64::usercopy::copy_to_user(user_destination, &wire).is_err() {
                panic!("prevalidated AppData directory destination faulted");
            }
            APP_DATA_DIRECTORY_READ_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            complete_app_data(frame, Status::Ok, u64::from(completion.next_cursor), 0)
        }
    }
}

#[cfg(feature = "app-data-runtime")]
fn app_data_root_principal(raw_root: u64, required: Rights) -> Result<u64, Status> {
    let root = parse_handle(raw_root).ok_or(Status::InvalidArgument)?;
    with_table(|table| {
        let object = table.get(root, required).map_err(|error| {
            if error == HandleError::AccessDenied {
                RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
            }
            handle_error_status(error)
        })?;
        object
            .as_app_data_directory()
            .map(|directory| directory.principal().raw())
            .ok_or(Status::InvalidState)
    })
}

#[cfg(feature = "app-data-runtime")]
fn copy_app_data_path(
    user_path: u64,
    raw_path_length: u64,
) -> Result<([u8; APP_DATA_PATH_MAX_BYTES], usize), Status> {
    let path_length = usize::try_from(raw_path_length)
        .ok()
        .filter(|length| *length != 0 && *length <= APP_DATA_PATH_MAX_BYTES)
        .ok_or(Status::InvalidArgument)?;
    let mut bytes = [0_u8; APP_DATA_PATH_MAX_BYTES];
    if crate::arch::aarch64::usercopy::copy_from_user(&mut bytes[..path_length], user_path).is_err()
    {
        return Err(Status::BadAddress);
    }
    validate_app_data_path(&bytes[..path_length]).map_err(|_| Status::InvalidArgument)?;
    Ok((bytes, path_length))
}

#[cfg(feature = "app-data-runtime")]
fn app_data_error_status(error: AppDataError) -> Status {
    match error {
        AppDataError::InvalidPrincipal
        | AppDataError::InvalidPath(_)
        | AppDataError::FileTooLarge => Status::InvalidArgument,
        AppDataError::NotFound => Status::NotFound,
        AppDataError::AlreadyExists | AppDataError::Conflict { .. } => Status::Conflict,
        AppDataError::NotDirectory | AppDataError::IsDirectory | AppDataError::NotEmpty => {
            Status::InvalidState
        }
        AppDataError::BufferTooSmall { .. } => Status::BufferTooSmall,
        AppDataError::OutOfSpace | AppDataError::GenerationExhausted => Status::NoSpace,
        AppDataError::OutcomeUnknown | AppDataError::Io(AppDataIoError::OutcomeUnknown) => {
            Status::OutcomeUnknown
        }
        AppDataError::RequiresReset | AppDataError::Io(AppDataIoError::RequiresReset) => {
            Status::Unavailable
        }
        AppDataError::Corrupt(_)
        | AppDataError::VerificationFailed
        | AppDataError::Unformatted
        | AppDataError::NotVirgin
        | AppDataError::AlreadyFormatted
        | AppDataError::VolumeBounds
        | AppDataError::Io(AppDataIoError::OutOfBounds) => Status::DataCorrupt,
        AppDataError::Io(AppDataIoError::Device) => Status::Unavailable,
    }
}

#[cfg(feature = "app-data-runtime")]
fn complete_app_data(
    frame: *mut TrapFrame,
    status: Status,
    out1: u64,
    out2: u64,
) -> *mut TrapFrame {
    match status {
        Status::PermissionDenied => {
            APP_DATA_PERMISSION_DENIALS.fetch_add(1, Ordering::Relaxed);
        }
        Status::InvalidArgument | Status::BadAddress => {
            APP_DATA_VALIDATION_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        }
        Status::ShouldWait => {
            APP_DATA_SHOULD_WAITS.fetch_add(1, Ordering::Relaxed);
            // The AppData syscall path is still IRQ-masked here, so the
            // current generation-qualified process is the only authority
            // allowed to contribute to a frozen request's post-recovery EL0
            // progress window. The coordinator ignores ordinary waits and
            // waits from every non-owner.
            crate::app_data::record_authenticated_recovery_wait(authenticated_sender_pid());
        }
        Status::Conflict => {
            APP_DATA_CONFLICTS.fetch_add(1, Ordering::Relaxed);
        }
        Status::OutcomeUnknown => {
            APP_DATA_OUTCOME_UNKNOWN.fetch_add(1, Ordering::Relaxed);
        }
        Status::DataCorrupt => {
            APP_DATA_CORRUPTION_ERRORS.fetch_add(1, Ordering::Relaxed);
        }
        Status::NoSpace => {
            APP_DATA_NO_SPACE.fetch_add(1, Ordering::Relaxed);
        }
        _ => {}
    }
    complete(frame, status, out1, out2)
}

fn vmo_read(
    frame: *mut TrapFrame,
    raw_handle: u64,
    user_destination: u64,
    packed_range: u64,
) -> *mut TrapFrame {
    VMO_READ_CALLS.fetch_add(1, Ordering::Relaxed);
    let Some(handle) = parse_handle(raw_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let (offset, requested_length) = unpack_vmo_read(packed_range);
    if requested_length as usize > VMO_READ_MAX_BYTES {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let vmo = match with_table(|table| {
        table
            .get(handle, Rights::READ)
            .map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                handle_error_status(error)
            })?
            .as_vmo()
            .cloned()
            .ok_or(Status::InvalidState)
    }) {
        Ok(vmo) => vmo,
        Err(status) => return complete(frame, status, 0, 0),
    };
    let bytes = match vmo.read_range(offset as usize, requested_length as usize) {
        Ok(bytes) => bytes,
        Err(_) => return complete(frame, Status::InvalidArgument, 0, 0),
    };
    if !bytes.is_empty() {
        if !crate::process::current_user_range_is_writable(user_destination, bytes.len()) {
            return complete(frame, Status::BadAddress, 0, 0);
        }
        if crate::arch::aarch64::usercopy::copy_to_user(user_destination, bytes).is_err() {
            panic!("prevalidated VMO destination faulted during copy-to-user");
        }
    }
    VMO_READ_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    VMO_READ_BYTES.fetch_add(bytes.len() as u64, Ordering::Relaxed);
    complete(frame, Status::Ok, bytes.len() as u64, vmo.len() as u64)
}

#[cfg(feature = "storage-server-runtime")]
fn ipc_buffer_create(
    frame: *mut TrapFrame,
    user_source: u64,
    raw_length: u64,
    flags: u64,
) -> *mut TrapFrame {
    use bndroid_kernel::vmo::{Vmo, VmoCreateError};

    let Some(length) = usize::try_from(raw_length)
        .ok()
        .filter(|length| *length <= IPC_BUFFER_PAYLOAD_MAX_BYTES)
    else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    if flags != IPC_BUFFER_CREATE_FLAGS_NONE
        || (length == 0 && user_source != 0)
        || (length != 0 && user_source == 0)
    {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let mut bytes = [0_u8; IPC_BUFFER_PAYLOAD_MAX_BYTES];
    if length != 0
        && (!crate::process::current_user_range_is_readable(user_source, length)
            || crate::arch::aarch64::usercopy::copy_from_user(&mut bytes[..length], user_source)
                .is_err())
    {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let vmo = match Vmo::try_from_slice(&bytes[..length]) {
        Ok(vmo) => vmo,
        Err(VmoCreateError::OutOfMemory) => {
            return complete(frame, Status::OutOfMemory, 0, 0);
        }
        Err(VmoCreateError::TooLarge) => {
            return complete(frame, Status::InvalidArgument, 0, 0);
        }
    };
    match with_table(|table| table.insert(KernelObject::from(vmo), Rights::VMO_DEFAULT)) {
        Ok(handle) => complete(frame, Status::Ok, u64::from(handle.raw()), length as u64),
        Err(error) => complete(frame, handle_error_status(error), 0, 0),
    }
}

#[cfg(feature = "storage-server-runtime")]
fn storage_acquire(
    frame: *mut TrapFrame,
    reserved0: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    if reserved0 != 0 || reserved1 != 0 || reserved2 != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(owner_pid) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let Some(image_id) = crate::process::current_live_user_image_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if image_id != UserImageId::StorageServer {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    #[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
    if bndroid_kernel::shutdown::admission_closed() {
        return complete(frame, Status::Unavailable, 0, 0);
    }
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    if bndroid_kernel::storage_broker::snapshot().device_offline {
        // Let the broker authenticate and count this one terminal denial.
        // The image check above remains first so unrelated EL0 processes
        // cannot distinguish Offline from an ordinary authority rejection.
        let error = bndroid_kernel::storage_broker::acquire(owner_pid, image_id)
            .expect_err("an offline storage broker unexpectedly issued a capability");
        return complete(frame, storage_acquire_error_status(error), 0, 0);
    }
    if crate::storage::recovery_admission_closed() {
        #[cfg(feature = "storage-server-async-recovery-runtime")]
        crate::storage_server_io::record_async_acquire_wait();
        return complete(frame, Status::ShouldWait, 0, 0);
    }
    let result = with_table(|table| {
        let reservation = table.reserve_slot().map_err(handle_error_status)?;
        let capability = bndroid_kernel::storage_broker::acquire(owner_pid, image_id)
            .map_err(storage_acquire_error_status)?;
        Ok::<_, Status>(reservation.insert_parts(
            KernelObject::from(capability),
            Rights::STORAGE_VOLUME_DEFAULT,
        ))
    });
    match result {
        Ok(handle) => complete(
            frame,
            Status::Ok,
            u64::from(handle.raw()),
            bndr_abi::APP_DATA_VOLUME_SECTORS,
        ),
        Err(status) => complete(frame, status, 0, 0),
    }
}

#[cfg(feature = "storage-server-runtime")]
fn storage_submit(
    frame: *mut TrapFrame,
    raw_capability: u64,
    user_request: u64,
    raw_request_length: u64,
) -> *mut TrapFrame {
    if raw_request_length != STORAGE_BLOCK_REQUEST_WIRE_SIZE as u64 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let capability = match storage_volume_handle(raw_capability, Rights::WRITE) {
        Ok(capability) => capability,
        Err(status) => return complete(frame, status, 0, 0),
    };
    let mut wire = [0_u8; STORAGE_BLOCK_REQUEST_WIRE_SIZE];
    if !crate::process::current_user_range_is_readable(user_request, wire.len())
        || crate::arch::aarch64::usercopy::copy_from_user(&mut wire, user_request).is_err()
    {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let request = match StorageBlockRequest::decode_submission(&wire) {
        Ok(request) => request,
        Err(_) => return complete(frame, Status::InvalidArgument, 0, 0),
    };
    // SVC entry keeps local IRQ masked, so the physical recovery latch cannot
    // change between this gate and broker submission. A bound server must see
    // a session-fatal result and exit; no new Pending request may be admitted
    // while the device route is already known to require reset.
    if crate::storage::recovery_admission_closed() {
        return complete(frame, Status::RequiresReset, 0, 0);
    }
    let owner_pid = authenticated_sender_pid();
    match bndroid_kernel::storage_broker::submit(capability, owner_pid, request) {
        Ok(token) => {
            crate::scheduler::wake_object_waiters();
            complete(frame, Status::Ok, token, 0)
        }
        Err(error) => complete(frame, storage_request_error_status(error), 0, 0),
    }
}

#[cfg(feature = "storage-server-runtime")]
fn storage_take(
    frame: *mut TrapFrame,
    raw_capability: u64,
    token: u64,
    user_read_destination: u64,
) -> *mut TrapFrame {
    if token == 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let capability = match storage_volume_handle(raw_capability, Rights::READ) {
        Ok(capability) => capability,
        Err(status) => return complete(frame, status, 0, 0),
    };
    let owner_pid = authenticated_sender_pid();
    let info = match bndroid_kernel::storage_broker::completion_info(capability, owner_pid, token) {
        Ok(info) => info,
        Err(error) => return complete(frame, storage_request_error_status(error), 0, 0),
    };
    let requested_bytes = info.sector_count() * STORAGE_SECTOR_SIZE;
    match info.operation() {
        StorageBlockOperation::Read => {
            if user_read_destination == 0
                || !crate::process::current_user_range_is_writable(
                    user_read_destination,
                    requested_bytes,
                )
            {
                return complete(frame, Status::BadAddress, 0, 0);
            }
            if info.code() == bndroid_kernel::storage_broker::CompletionCode::Ok {
                let mut read_data = [0_u8; STORAGE_BLOCK_DATA_MAX_BYTES];
                let copied = bndroid_kernel::storage_broker::copy_completion_data(
                    capability,
                    owner_pid,
                    token,
                    &mut read_data,
                )
                .unwrap_or_else(|_| panic!("peeked storage completion changed before copy"));
                if copied != requested_bytes || copied != info.data_bytes() {
                    panic!("storage completion exposed a noncanonical batch length");
                }
                if crate::arch::aarch64::usercopy::copy_to_user(
                    user_read_destination,
                    &read_data[..copied],
                )
                .is_err()
                {
                    panic!("prevalidated storage read destination faulted");
                }
            }
        }
        StorageBlockOperation::Write | StorageBlockOperation::Flush => {
            if user_read_destination != 0 {
                return complete(frame, Status::InvalidArgument, 0, 0);
            }
        }
    }
    bndroid_kernel::storage_broker::acknowledge_completion(capability, owner_pid, token)
        .unwrap_or_else(|_| panic!("peeked storage completion changed before acknowledge"));
    crate::scheduler::wake_object_waiters();
    let status = storage_completion_status(info.code());
    let bytes = if status == Status::Ok && info.operation() == StorageBlockOperation::Read {
        info.data_bytes() as u64
    } else {
        0
    };
    complete(frame, status, bytes, 0)
}

#[cfg(feature = "storage-server-runtime")]
fn storage_connect(
    frame: *mut TrapFrame,
    raw_rights: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    let Ok(bits) = u32::try_from(raw_rights) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let Some(rights) = Rights::from_bits(bits) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    if reserved1 != 0
        || reserved2 != 0
        || bits == 0
        || bits & !bndr_abi::STORAGE_CONNECT_RIGHTS_MASK != 0
    {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(client_pid) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let Some(image_id) = crate::process::current_live_user_image_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if bndroid_kernel::storage_broker::principal_for_image(image_id).is_none() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    #[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
    if bndroid_kernel::shutdown::admission_closed() {
        bndroid_kernel::shutdown::record_connect_rejection();
        return complete(frame, Status::Unavailable, 0, 0);
    }
    // Authenticate the client before exposing the recovery gate. No new
    // queued session may appear after a fatal completion closes admission.
    if crate::storage::recovery_admission_closed() {
        return complete(frame, Status::RequiresReset, 0, 0);
    }
    let result = with_table(|table| {
        let reservation = table.reserve_slot().map_err(handle_error_status)?;
        let endpoint = bndroid_kernel::storage_broker::connect(client_pid, image_id, rights)
            .map_err(storage_session_error_status)?;
        Ok::<_, Status>(reservation.insert_parts(
            KernelObject::from(endpoint),
            Rights::STORAGE_SESSION_DEFAULT,
        ))
    });
    match result {
        Ok(handle) => {
            crate::scheduler::wake_object_waiters();
            complete(frame, Status::Ok, u64::from(handle.raw()), 0)
        }
        Err(status) => complete(frame, status, 0, 0),
    }
}

#[cfg(feature = "storage-server-runtime")]
fn storage_accept(
    frame: *mut TrapFrame,
    raw_capability: u64,
    user_binding: u64,
    raw_binding_length: u64,
) -> *mut TrapFrame {
    if raw_binding_length != STORAGE_SESSION_BINDING_WIRE_SIZE as u64 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    if !crate::process::current_user_range_is_writable(
        user_binding,
        STORAGE_SESSION_BINDING_WIRE_SIZE,
    ) {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let capability = match storage_volume_handle(raw_capability, Rights::READ) {
        Ok(capability) => capability,
        Err(status) => return complete(frame, status, 0, 0),
    };
    let owner_pid = authenticated_sender_pid();
    #[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
    if bndroid_kernel::shutdown::admission_closed() {
        return complete(frame, Status::Unavailable, 0, 0);
    }
    // StorageTake remains available for the one terminal completion, but new
    // accepted endpoints are forbidden once physical recovery is latched.
    if crate::storage::recovery_admission_closed() {
        return complete(frame, Status::RequiresReset, 0, 0);
    }
    let result = with_table(|table| {
        let reservation = table.reserve_slot().map_err(handle_error_status)?;
        let expected = bndroid_kernel::storage_broker::peek_session_binding(capability, owner_pid)
            .map_err(storage_session_error_status)?;
        let wire = expected.encode();
        if crate::arch::aarch64::usercopy::copy_to_user(user_binding, &wire).is_err() {
            return Err(Status::BadAddress);
        }
        let (endpoint, accepted) = bndroid_kernel::storage_broker::accept(capability, owner_pid)
            .map_err(storage_session_error_status)?;
        if accepted != expected {
            panic!("storage accept queue changed during IRQ-masked transaction");
        }
        Ok::<_, Status>(reservation.insert_parts(
            KernelObject::from(endpoint),
            Rights::STORAGE_SESSION_DEFAULT,
        ))
    });
    match result {
        Ok(handle) => complete(frame, Status::Ok, u64::from(handle.raw()), 0),
        Err(status) => complete(frame, status, 0, 0),
    }
}

#[cfg(feature = "storage-server-runtime")]
fn storage_volume_handle(
    raw_capability: u64,
    required: Rights,
) -> Result<bndroid_kernel::storage_broker::StorageVolumeCapability, Status> {
    let handle = parse_handle(raw_capability).ok_or(Status::InvalidArgument)?;
    with_table(|table| {
        table
            .get(handle, required)
            .map_err(handle_error_status)?
            .as_storage_volume()
            .copied()
            .ok_or(Status::InvalidState)
    })
}

#[cfg(feature = "storage-server-runtime")]
fn storage_acquire_error_status(error: bndroid_kernel::storage_broker::AcquireError) -> Status {
    match error {
        bndroid_kernel::storage_broker::AcquireError::InvalidOwner => Status::PermissionDenied,
        bndroid_kernel::storage_broker::AcquireError::DeviceOffline => Status::Unavailable,
        bndroid_kernel::storage_broker::AcquireError::AlreadyBound => Status::ShouldWait,
        bndroid_kernel::storage_broker::AcquireError::EpochExhausted => Status::NoSpace,
    }
}

#[cfg(feature = "storage-server-runtime")]
fn storage_request_error_status(error: bndroid_kernel::storage_broker::RequestError) -> Status {
    match error {
        bndroid_kernel::storage_broker::RequestError::CapabilityMismatch => {
            Status::PermissionDenied
        }
        bndroid_kernel::storage_broker::RequestError::RecoveryRequired => Status::RequiresReset,
        bndroid_kernel::storage_broker::RequestError::ServiceAbandoned => Status::InvalidState,
        bndroid_kernel::storage_broker::RequestError::Busy => Status::ShouldWait,
        bndroid_kernel::storage_broker::RequestError::TokenExhausted => Status::NoSpace,
        bndroid_kernel::storage_broker::RequestError::NoPendingRequest
        | bndroid_kernel::storage_broker::RequestError::TokenMismatch => Status::NotFound,
        bndroid_kernel::storage_broker::RequestError::InvalidTransition => Status::InvalidState,
    }
}

#[cfg(feature = "storage-server-runtime")]
fn storage_session_error_status(error: bndroid_kernel::storage_broker::SessionError) -> Status {
    match error {
        bndroid_kernel::storage_broker::SessionError::Unavailable => Status::ShouldWait,
        bndroid_kernel::storage_broker::SessionError::RecoveryRequired => Status::RequiresReset,
        bndroid_kernel::storage_broker::SessionError::PermissionDenied
        | bndroid_kernel::storage_broker::SessionError::CapabilityMismatch => {
            Status::PermissionDenied
        }
        bndroid_kernel::storage_broker::SessionError::InvalidRights => Status::InvalidArgument,
        bndroid_kernel::storage_broker::SessionError::QueueFull => Status::ShouldWait,
        bndroid_kernel::storage_broker::SessionError::OutOfMemory => Status::OutOfMemory,
        bndroid_kernel::storage_broker::SessionError::SessionIdExhausted => Status::NoSpace,
    }
}

#[cfg(feature = "storage-server-runtime")]
fn storage_completion_status(code: bndroid_kernel::storage_broker::CompletionCode) -> Status {
    match code {
        bndroid_kernel::storage_broker::CompletionCode::Ok => Status::Ok,
        bndroid_kernel::storage_broker::CompletionCode::Unavailable => Status::Unavailable,
        bndroid_kernel::storage_broker::CompletionCode::OutcomeUnknown => Status::OutcomeUnknown,
        bndroid_kernel::storage_broker::CompletionCode::RequiresReset => Status::RequiresReset,
    }
}

/// Creates one fixed XRGB8888 buffer for an authenticated UI producer.
///
/// Reserving the handle slot first makes publication transactional: a full
/// handle table cannot consume one of the two static graphics-buffer slots.
/// M41 also admits the unique SurfaceServer as the producer of its two
/// mappable compositor-output slots; non-mappable creation remains denied.
fn graphics_buffer_create(
    frame: *mut TrapFrame,
    raw_format: u64,
    packed_geometry: u64,
    raw_flags: u64,
) -> *mut TrapFrame {
    let (width, height) = unpack_graphics_buffer_geometry(packed_geometry);
    if raw_format != GRAPHICS_BUFFER_FORMAT_XRGB8888
        || width != GRAPHICS_BUFFER_WIDTH
        || height != GRAPHICS_BUFFER_HEIGHT
        || !matches!(
            raw_flags,
            GRAPHICS_BUFFER_CREATE_FLAGS_NONE | GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE
        )
    {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let legacy_producer = !MULTI_WINDOW_RUNTIME_ACTIVE
        && matches!(
            crate::process::current_live_user_image_id(),
            Some(UserImageId::Launcher | UserImageId::App)
        );
    let surface_output_producer = MULTI_WINDOW_RUNTIME_ACTIVE
        && raw_flags == GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE
        && crate::process::current_is_surface_server();
    #[cfg(feature = "androidbox-interactive0")]
    let system_chrome_producer = raw_flags == GRAPHICS_BUFFER_CREATE_FLAGS_NONE
        && crate::process::current_is_surface_server();
    #[cfg(not(feature = "androidbox-interactive0"))]
    let system_chrome_producer = false;
    if !legacy_producer && !surface_output_producer && !system_chrome_producer {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let Some(producer_pid) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let mappable = raw_flags == GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE;
    let rights = if mappable {
        Rights::GRAPHICS_BUFFER_MAPPED_PRODUCER
    } else {
        Rights::GRAPHICS_BUFFER_DEFAULT
    };
    let created = with_table(|table| {
        let reservation = table.reserve_slot().map_err(handle_error_status)?;
        let buffer = if mappable {
            GraphicsBuffer::try_new_mappable(producer_pid)
        } else {
            GraphicsBuffer::try_new(producer_pid)
        };
        let buffer = match buffer {
            Ok(buffer) => buffer,
            Err(error) => {
                if error == GraphicsBufferError::Exhausted {
                    GRAPHICS_BUFFER_CREATE_EXHAUSTIONS.fetch_add(1, Ordering::Relaxed);
                }
                return Err(graphics_buffer_error_status(error));
            }
        };
        let slot = buffer.slot();
        let slot_generation = buffer.slot_generation();
        let handle = reservation.insert_parts(KernelObject::from(buffer), rights);
        Ok::<_, Status>((handle, slot, slot_generation))
    });
    let (handle, slot, slot_generation) = match created {
        Ok(created) => created,
        Err(status) => return complete(frame, status, 0, 0),
    };
    GRAPHICS_BUFFERS_CREATED.fetch_add(1, Ordering::Relaxed);
    if mappable {
        GRAPHICS_BUFFER_MAPPABLE_CREATED.fetch_add(1, Ordering::Relaxed);
    }
    crate::kprintln!(
        "GRAPHICS_BUFFER_CREATE_OK producer_pid={} handle={} slot={} slot_generation={} format=xrgb8888 width={} height={} logical_bytes={} backing_bytes={} rights={:#010x} mapped={}",
        producer_pid,
        handle.raw(),
        slot,
        slot_generation,
        GRAPHICS_BUFFER_WIDTH,
        GRAPHICS_BUFFER_HEIGHT,
        GRAPHICS_BUFFER_LOGICAL_BYTES,
        bndr_abi::GRAPHICS_BUFFER_BACKING_BYTES,
        rights.bits(),
        u8::from(mappable),
    );
    complete(
        frame,
        Status::Ok,
        u64::from(handle.raw()),
        GRAPHICS_BUFFER_LOGICAL_BYTES as u64,
    )
}

/// Copies one prevalidated, four-byte-aligned producer range into a buffer.
/// A write generation advances exactly once after the complete user copy and
/// XRGB canonicalization succeed.
fn graphics_buffer_write(
    frame: *mut TrapFrame,
    raw_handle: u64,
    user_source: u64,
    packed_range: u64,
) -> *mut TrapFrame {
    let Some(handle) = parse_handle(raw_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let (offset, length) = unpack_graphics_buffer_write(packed_range);
    let offset = offset as usize;
    let length = length as usize;
    if length == 0
        || length > GRAPHICS_BUFFER_WRITE_MAX_BYTES
        || !offset.is_multiple_of(size_of::<u32>())
        || !length.is_multiple_of(size_of::<u32>())
        || offset
            .checked_add(length)
            .is_none_or(|end| end > GRAPHICS_BUFFER_LOGICAL_BYTES)
    {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(producer_pid) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let buffer = match with_table(|table| {
        table
            .get(handle, Rights::WRITE)
            .map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                handle_error_status(error)
            })?
            .as_graphics_buffer()
            .cloned()
            .ok_or(Status::InvalidState)
    }) {
        Ok(buffer) => buffer,
        Err(status) => return complete(frame, status, 0, 0),
    };
    if buffer.producer_pid() != producer_pid {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if !crate::process::current_user_range_is_readable(user_source, length) {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    #[cfg(not(feature = "mobile-ui-runtime"))]
    let scratch = {
        let mut scratch = [0_u8; GRAPHICS_BUFFER_WRITE_MAX_BYTES];
        if crate::arch::aarch64::usercopy::copy_from_user(&mut scratch[..length], user_source)
            .is_err()
        {
            panic!("prevalidated graphics-buffer source faulted during copy-from-user");
        }
        scratch
    };
    #[cfg(feature = "mobile-ui-runtime")]
    let mut scratch_guard = GRAPHICS_BUFFER_WRITE_SCRATCH.acquire();
    #[cfg(feature = "mobile-ui-runtime")]
    let scratch = {
        // Keep the kernel exception stack small even though one mobile ABI
        // write carries twenty-two rows. The complete user range was checked
        // above; each inner copy retains the global user-copy bound and any
        // post-validation fault remains a fatal invariant violation, exactly
        // like the historical one-page path. A fixed, guarded staging buffer
        // avoids a fallible 63 KiB heap allocation while preserving one
        // transactional GraphicsBuffer::write and one generation advance.
        let scratch = scratch_guard.bytes();
        let mut copied = 0_usize;
        while copied < length {
            let chunk_length =
                (length - copied).min(crate::arch::aarch64::usercopy::MAX_USER_COPY_BYTES);
            let source = user_source
                .checked_add(copied as u64)
                .unwrap_or_else(|| panic!("prevalidated graphics-buffer source overflowed"));
            if crate::arch::aarch64::usercopy::copy_from_user(
                &mut scratch[copied..copied + chunk_length],
                source,
            )
            .is_err()
            {
                panic!("prevalidated graphics-buffer source faulted during copy-from-user");
            }
            copied += chunk_length;
        }
        scratch
    };
    let write_generation = match buffer.write(offset, &scratch[..length]) {
        Ok(generation) => generation,
        Err(error) => return complete(frame, graphics_buffer_error_status(error), 0, 0),
    };
    GRAPHICS_BUFFER_WRITE_CALLS.fetch_add(1, Ordering::Relaxed);
    GRAPHICS_BUFFER_WRITE_BYTES.fetch_add(length as u64, Ordering::Relaxed);
    complete(frame, Status::Ok, length as u64, write_generation)
}

fn graphics_buffer_map(
    frame: *mut TrapFrame,
    raw_handle: u64,
    raw_role: u64,
    raw_flags: u64,
) -> *mut TrapFrame {
    GRAPHICS_BUFFER_MAP_CALLS.fetch_add(1, Ordering::Relaxed);
    if raw_flags != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(handle) = parse_handle(raw_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let (role, expected_rights) = match raw_role {
        GRAPHICS_BUFFER_MAP_PRODUCER_RW => (
            GraphicsMappingRole::Producer,
            Rights::GRAPHICS_BUFFER_MAPPED_PRODUCER,
        ),
        GRAPHICS_BUFFER_MAP_CONSUMER_RO => (
            GraphicsMappingRole::Consumer,
            Rights::GRAPHICS_BUFFER_MAPPED_SERVER,
        ),
        _ => return complete(frame, Status::InvalidArgument, 0, 0),
    };
    let (buffer, rights) = match graphics_buffer_handle(handle, Rights::MAP) {
        Ok(buffer) => buffer,
        Err(status) => return complete(frame, status, 0, 0),
    };
    if rights != expected_rights {
        RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let Some(current_pid) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let role_valid = match role {
        GraphicsMappingRole::Producer => {
            (!MULTI_WINDOW_RUNTIME_ACTIVE
                && matches!(
                    crate::process::current_live_user_image_id(),
                    Some(UserImageId::Launcher | UserImageId::App)
                )
                && buffer.producer_pid() == current_pid)
                || (MULTI_WINDOW_RUNTIME_ACTIVE
                    && crate::process::current_is_surface_server()
                    && buffer.producer_pid() == current_pid)
        }
        GraphicsMappingRole::Consumer => crate::process::current_is_surface_server(),
    };
    if !role_valid {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let mapping = match crate::process::map_current_graphics_buffer(buffer, role) {
        Ok(mapping) => mapping,
        Err(error) => return complete(frame, process_graphics_mapping_error_status(error), 0, 0),
    };
    GRAPHICS_BUFFER_MAP_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    crate::kprintln!(
        "GRAPHICS_BUFFER_MAP_OK pid={} role={} address={:#018x} backing={:#018x} pages={} slot={} slot_generation={} access={}",
        current_pid,
        if role == GraphicsMappingRole::Producer {
            "producer"
        } else {
            "consumer"
        },
        mapping.address,
        mapping.backing_address,
        mapping.pages,
        mapping.identity.slot(),
        mapping.identity.generation(),
        if mapping.access == GraphicsMappingAccess::ReadWrite {
            "rw"
        } else {
            "ro"
        },
    );
    complete(
        frame,
        Status::Ok,
        mapping.address as u64,
        bndr_abi::GRAPHICS_BUFFER_BACKING_BYTES as u64,
    )
}

fn graphics_buffer_unmap(
    frame: *mut TrapFrame,
    raw_handle: u64,
    raw_address: u64,
    raw_flags: u64,
) -> *mut TrapFrame {
    GRAPHICS_BUFFER_UNMAP_CALLS.fetch_add(1, Ordering::Relaxed);
    if raw_flags != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(handle) = parse_handle(raw_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let Ok(address) = usize::try_from(raw_address) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let (buffer, rights) = match graphics_buffer_handle(handle, Rights::MAP) {
        Ok(buffer) => buffer,
        Err(status) => return complete(frame, status, 0, 0),
    };
    if !matches!(
        rights,
        Rights::GRAPHICS_BUFFER_MAPPED_PRODUCER | Rights::GRAPHICS_BUFFER_MAPPED_SERVER
    ) {
        RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let mapping = match crate::process::unmap_current_graphics_buffer(&buffer, address) {
        Ok(mapping) => mapping,
        Err(error) => return complete(frame, process_graphics_mapping_error_status(error), 0, 0),
    };
    GRAPHICS_BUFFER_UNMAP_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    crate::kprintln!(
        "GRAPHICS_BUFFER_UNMAP_OK pid={} address={:#018x} pages={} slot={} slot_generation={} role={}",
        crate::process::current_live_user_process_id().unwrap_or(0),
        mapping.address,
        mapping.pages,
        mapping.identity.slot(),
        mapping.identity.generation(),
        if mapping.role == GraphicsMappingRole::Producer {
            "producer"
        } else {
            "consumer"
        },
    );
    complete(frame, Status::Ok, 0, 0)
}

fn graphics_buffer_queue(
    frame: *mut TrapFrame,
    raw_handle: u64,
    expected_generation: u64,
    raw_flags: u64,
) -> *mut TrapFrame {
    GRAPHICS_BUFFER_QUEUE_CALLS.fetch_add(1, Ordering::Relaxed);
    if raw_flags != GRAPHICS_BUFFER_QUEUE_FLAGS_NONE {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(handle) = parse_handle(raw_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let (buffer, rights) = match graphics_buffer_handle(handle, Rights::WRITE) {
        Ok(buffer) => buffer,
        Err(status) => return complete(frame, status, 0, 0),
    };
    let Some(producer_pid) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if rights != Rights::GRAPHICS_BUFFER_MAPPED_PRODUCER || buffer.producer_pid() != producer_pid {
        RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    if let Err(error) = crate::process::protect_current_graphics_buffer_mapping(
        &buffer,
        GraphicsMappingAccess::ReadWrite,
        GraphicsMappingAccess::ReadOnly,
    ) {
        return complete(frame, process_graphics_mapping_error_status(error), 0, 0);
    }
    let generation = match buffer.queue(expected_generation) {
        Ok(generation) => generation,
        Err(error) => {
            crate::process::protect_current_graphics_buffer_mapping(
                &buffer,
                GraphicsMappingAccess::ReadOnly,
                GraphicsMappingAccess::ReadWrite,
            )
            .unwrap_or_else(|rollback| {
                panic!(
                    "failed graphics queue could not restore producer mapping: {}",
                    rollback.as_str()
                )
            });
            return complete(frame, graphics_buffer_error_status(error), 0, 0);
        }
    };
    GRAPHICS_BUFFER_QUEUE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    GRAPHICS_BUFFER_VALIDATED_PIXELS.fetch_add(
        bndr_abi::GRAPHICS_BUFFER_PIXEL_COUNT as u64,
        Ordering::Relaxed,
    );
    GRAPHICS_BUFFER_VALIDATED_BYTES.fetch_add(
        bndr_abi::GRAPHICS_BUFFER_LOGICAL_BYTES as u64,
        Ordering::Relaxed,
    );
    crate::scheduler::wake_object_waiters();
    complete(frame, Status::Ok, generation, 0)
}

fn graphics_buffer_acquire(
    frame: *mut TrapFrame,
    raw_handle: u64,
    expected_generation: u64,
    raw_flags: u64,
) -> *mut TrapFrame {
    GRAPHICS_BUFFER_ACQUIRE_CALLS.fetch_add(1, Ordering::Relaxed);
    if raw_flags != GRAPHICS_BUFFER_ACQUIRE_FLAGS_NONE
        || !crate::process::current_is_surface_server()
    {
        let status = if raw_flags != GRAPHICS_BUFFER_ACQUIRE_FLAGS_NONE {
            Status::InvalidArgument
        } else {
            Status::PermissionDenied
        };
        return complete(frame, status, 0, 0);
    }
    let Some(handle) = parse_handle(raw_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let (buffer, rights) = match graphics_buffer_handle(handle, Rights::READ) {
        Ok(buffer) => buffer,
        Err(status) => return complete(frame, status, 0, 0),
    };
    if rights != Rights::GRAPHICS_BUFFER_MAPPED_SERVER {
        RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let Some(consumer_pid) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let Some(mapping) = crate::process::current_graphics_buffer_mapping(&buffer) else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let transferred_consumer = mapping.role == GraphicsMappingRole::Consumer
        && mapping.access == GraphicsMappingAccess::ReadOnly;
    let self_owned_surface_output = MULTI_WINDOW_RUNTIME_ACTIVE
        && buffer.producer_pid() == consumer_pid
        && mapping.role == GraphicsMappingRole::Producer
        && mapping.access == GraphicsMappingAccess::ReadOnly;
    if !transferred_consumer && !self_owned_surface_output {
        return complete(frame, Status::InvalidState, 0, 0);
    }
    let generation = match buffer.acquire(expected_generation, consumer_pid) {
        Ok(generation) => generation,
        Err(error) => return complete(frame, graphics_buffer_error_status(error), 0, 0),
    };
    GRAPHICS_BUFFER_ACQUIRE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    complete(frame, Status::Ok, generation, 0)
}

fn graphics_buffer_release(
    frame: *mut TrapFrame,
    raw_handle: u64,
    expected_generation: u64,
    raw_flags: u64,
) -> *mut TrapFrame {
    GRAPHICS_BUFFER_RELEASE_CALLS.fetch_add(1, Ordering::Relaxed);
    if raw_flags != GRAPHICS_BUFFER_RELEASE_FLAGS_NONE
        || !crate::process::current_is_surface_server()
    {
        let status = if raw_flags != GRAPHICS_BUFFER_RELEASE_FLAGS_NONE {
            Status::InvalidArgument
        } else {
            Status::PermissionDenied
        };
        return complete(frame, status, 0, 0);
    }
    let Some(handle) = parse_handle(raw_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let (buffer, rights) = match graphics_buffer_handle(handle, Rights::READ) {
        Ok(buffer) => buffer,
        Err(status) => return complete(frame, status, 0, 0),
    };
    if rights != Rights::GRAPHICS_BUFFER_MAPPED_SERVER {
        RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let Some(consumer_pid) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if let Err(error) = buffer.can_release(expected_generation, consumer_pid) {
        return complete(frame, graphics_buffer_error_status(error), 0, 0);
    }
    match crate::process::restore_graphics_buffer_producer_mapping(buffer.producer_pid(), &buffer) {
        Ok(_) => {}
        // A generation-qualified producer that has exited has no address
        // space left to restore. Releasing the acquisition is still required
        // so SurfaceServer can unmap and close the orphaned view instead of
        // leaving the pinned backing permanently Acquired.
        Err(crate::process::ProcessGraphicsMappingError::NotFound) => {}
        Err(error) => {
            return complete(frame, process_graphics_mapping_error_status(error), 0, 0);
        }
    }
    let generation = buffer
        .release(expected_generation, consumer_pid)
        .unwrap_or_else(|error| panic!("prevalidated graphics release failed: {}", error.as_str()));
    GRAPHICS_BUFFER_RELEASE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    GRAPHICS_BUFFER_RELEASES.fetch_add(1, Ordering::Relaxed);
    crate::scheduler::wake_object_waiters();
    complete(frame, Status::Ok, generation, 0)
}

fn graphics_buffer_handle(
    handle: HandleValue,
    required: Rights,
) -> Result<(GraphicsBuffer, Rights), Status> {
    with_table(|table| {
        let rights = table.rights(handle).map_err(handle_error_status)?;
        let buffer = table
            .get(handle, required)
            .map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                handle_error_status(error)
            })?
            .as_graphics_buffer()
            .cloned()
            .ok_or(Status::InvalidState)?;
        Ok((buffer, rights))
    })
}

/// Creates the unique generation-qualified InputServer read capability.
fn input_acquire(
    frame: *mut TrapFrame,
    reserved0: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    INPUT_ACQUIRE_CALLS.fetch_add(1, Ordering::Relaxed);
    if reserved0 != 0 || reserved1 != 0 || reserved2 != 0 {
        return complete_input_acquire(frame, Status::InvalidArgument, 0, 0);
    }
    #[cfg(not(feature = "input-server-runtime"))]
    {
        complete_input_acquire(frame, Status::Unsupported, 0, 0)
    }
    #[cfg(feature = "input-server-runtime")]
    {
        use bndroid_kernel::input_broker::InputAcquireError;

        if !crate::process::current_is_input_server() {
            #[cfg(all(
                feature = "input-server-restart-runtime",
                not(feature = "service-dependency-runtime")
            ))]
            {
                let caller_pid = crate::process::current_live_user_process_id().unwrap_or(0);
                let caller_image = crate::process::current_live_user_image_id();
                if caller_image == Some(UserImageId::SurfaceServer) {
                    let handles_before = with_table(|table| table.len());
                    let next_session_before = crate::input_stream::snapshot().next_session_id;
                    let handles_after = with_table(|table| table.len());
                    let next_session_after = crate::input_stream::snapshot().next_session_id;
                    // Publish the syscall counter before the M47 trace bit. The
                    // monitor uses the trace's Release/Acquire edge as the
                    // commit point for a cross-subsystem snapshot, so exposing
                    // the bit first could let a timer interrupt observe the
                    // audit with the old permission-denial count.
                    INPUT_ACQUIRE_PERMISSION_DENIED.fetch_add(1, Ordering::Relaxed);
                    if !bndroid_kernel::input_server_restart_trace::record_input_acquire_permission_denial(
                        caller_pid,
                        UserImageId::SurfaceServer,
                        handles_before,
                        handles_after,
                        next_session_before,
                        next_session_after,
                    ) {
                        return complete_input_acquire(frame, Status::InvalidState, 0, 0);
                    }
                    return complete(frame, Status::PermissionDenied, 0, 0);
                }
            }
            return complete_input_acquire(frame, Status::PermissionDenied, 0, 0);
        }
        let Some(process_id) = crate::process::current_live_user_process_id() else {
            return complete_input_acquire(frame, Status::InvalidState, 0, 0);
        };
        if crate::process::unique_live_process_id_for_image(UserImageId::InputServer)
            != Some(process_id)
        {
            return complete_input_acquire(frame, Status::PermissionDenied, 0, 0);
        }
        let acquired = with_table(|table| {
            let reservation = table.reserve_slot().map_err(handle_error_status)?;
            let capability =
                crate::input_stream::acquire(process_id).map_err(|error| match error {
                    InputAcquireError::AlreadyBound => Status::AlreadyExists,
                    InputAcquireError::OutOfMemory => Status::OutOfMemory,
                    InputAcquireError::InvalidProcess | InputAcquireError::SessionExhausted => {
                        Status::InvalidState
                    }
                })?;
            let session_id = capability.session_id();
            let handle =
                reservation.insert_parts(KernelObject::from(capability), Rights::INPUT_DEFAULT);
            Ok::<_, Status>((handle, session_id))
        });
        let (handle, session_id) = match acquired {
            Ok(acquired) => acquired,
            Err(status) => return complete_input_acquire(frame, status, 0, 0),
        };
        crate::kprintln!(
            "INPUT_ACQUIRE_OK pid={} session={} handle={} rights={:#010x} capacity={} unique=1 duplicate=0 transferable=0",
            process_id,
            session_id,
            handle.raw(),
            Rights::INPUT_DEFAULT.bits(),
            bndroid_kernel::input_broker::INPUT_EVENT_QUEUE_CAPACITY,
        );
        complete_input_acquire(frame, Status::Ok, u64::from(handle.raw()), session_id)
    }
}

fn complete_input_acquire(
    frame: *mut TrapFrame,
    status: Status,
    result1: u64,
    result2: u64,
) -> *mut TrapFrame {
    if status == Status::Ok {
        INPUT_ACQUIRE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    } else if status == Status::PermissionDenied {
        INPUT_ACQUIRE_PERMISSION_DENIED.fetch_add(1, Ordering::Relaxed);
    }
    complete(frame, status, result1, result2)
}

fn input_read_event(
    frame: *mut TrapFrame,
    raw_handle: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    if reserved1 != 0 || reserved2 != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    #[cfg(not(feature = "input-server-runtime"))]
    {
        let _ = raw_handle;
        complete(frame, Status::Unsupported, 0, 0)
    }
    #[cfg(feature = "input-server-runtime")]
    {
        use bndroid_kernel::input_broker::InputBrokerError;

        if !crate::process::current_is_input_server() {
            return complete(frame, Status::PermissionDenied, 0, 0);
        }
        let Some(handle) = parse_handle(raw_handle) else {
            return complete(frame, Status::InvalidArgument, 0, 0);
        };
        let capability = match with_table(|table| {
            table
                .get(handle, Rights::READ)
                .map_err(|error| {
                    if error == HandleError::AccessDenied {
                        RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                    }
                    handle_error_status(error)
                })?
                .as_input()
                .cloned()
                .ok_or(Status::InvalidState)
        }) {
            Ok(capability) => capability,
            Err(status) => return complete(frame, status, 0, 0),
        };
        if capability.signals().intersects(ObjectSignals::PEER_CLOSED) {
            return complete(frame, Status::PeerClosed, 0, 0);
        }
        let Some(process_id) = crate::process::current_live_user_process_id() else {
            return complete(frame, Status::InvalidState, 0, 0);
        };
        let event = match crate::input_stream::read(&capability, process_id) {
            Ok(Some(event)) => event,
            Ok(None) => return complete(frame, Status::ShouldWait, 0, 0),
            Err(InputBrokerError::PeerClosed | InputBrokerError::Unbound) => {
                return complete(frame, Status::PeerClosed, 0, 0);
            }
            Err(InputBrokerError::CapabilityMismatch) => {
                return complete(frame, Status::PermissionDenied, 0, 0);
            }
            Err(
                InputBrokerError::InvalidEvent(_)
                | InputBrokerError::SequenceExhausted
                | InputBrokerError::Full
                | InputBrokerError::CounterExhausted,
            ) => return complete(frame, Status::InvalidState, 0, 0),
        };
        let (state, sequence) = event.encode_registers();
        complete(frame, Status::Ok, state, sequence)
    }
}

/// Returns the immutable session identity and acquisition-time sequence floor
/// for the caller's currently bound physical-input capability.
fn input_session_info(
    frame: *mut TrapFrame,
    raw_handle: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    INPUT_SESSION_INFO_CALLS.fetch_add(1, Ordering::Relaxed);
    if reserved1 != 0 || reserved2 != 0 {
        return complete_input_session_info(frame, Status::InvalidArgument, 0, 0);
    }
    #[cfg(not(feature = "input-server-runtime"))]
    {
        let _ = raw_handle;
        complete_input_session_info(frame, Status::Unsupported, 0, 0)
    }
    #[cfg(feature = "input-server-runtime")]
    {
        use bndroid_kernel::input_broker::InputBrokerError;

        if !crate::process::current_is_input_server() {
            return complete_input_session_info(frame, Status::PermissionDenied, 0, 0);
        }
        let Some(process_id) = crate::process::current_live_user_process_id() else {
            return complete_input_session_info(frame, Status::InvalidState, 0, 0);
        };
        if crate::process::unique_live_process_id_for_image(UserImageId::InputServer)
            != Some(process_id)
        {
            return complete_input_session_info(frame, Status::PermissionDenied, 0, 0);
        }
        let Some(handle) = parse_handle(raw_handle) else {
            return complete_input_session_info(frame, Status::InvalidArgument, 0, 0);
        };
        let capability = match with_table(|table| {
            table
                .get(handle, Rights::READ)
                .map_err(|error| {
                    if error == HandleError::AccessDenied {
                        RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                    }
                    handle_error_status(error)
                })?
                .as_input()
                .cloned()
                .ok_or(Status::InvalidState)
        }) {
            Ok(capability) => capability,
            Err(status) => return complete_input_session_info(frame, status, 0, 0),
        };
        if capability.signals().intersects(ObjectSignals::PEER_CLOSED) {
            return complete_input_session_info(frame, Status::PeerClosed, 0, 0);
        }
        let info = match crate::input_stream::session_info(&capability, process_id) {
            Ok(info) => info,
            Err(InputBrokerError::PeerClosed | InputBrokerError::Unbound) => {
                return complete_input_session_info(frame, Status::PeerClosed, 0, 0);
            }
            Err(InputBrokerError::CapabilityMismatch) => {
                return complete_input_session_info(frame, Status::PermissionDenied, 0, 0);
            }
            Err(
                InputBrokerError::InvalidEvent(_)
                | InputBrokerError::SequenceExhausted
                | InputBrokerError::Full
                | InputBrokerError::CounterExhausted,
            ) => return complete_input_session_info(frame, Status::InvalidState, 0, 0),
        };
        if info.process_id != process_id
            || info.session_id != capability.session_id()
            || info.acquisition_floor != capability.acquisition_floor()
        {
            panic!("input session query returned inconsistent capability evidence");
        }
        crate::kprintln!(
            "INPUT_SESSION_INFO_OK pid={} session={} acquisition_floor={}",
            info.process_id,
            info.session_id,
            info.acquisition_floor,
        );
        complete_input_session_info(frame, Status::Ok, info.session_id, info.acquisition_floor)
    }
}

fn complete_input_session_info(
    frame: *mut TrapFrame,
    status: Status,
    result1: u64,
    result2: u64,
) -> *mut TrapFrame {
    if status == Status::Ok {
        INPUT_SESSION_INFO_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    } else if status == Status::PermissionDenied {
        INPUT_SESSION_INFO_PERMISSION_DENIED.fetch_add(1, Ordering::Relaxed);
    }
    complete(frame, status, result1, result2)
}

/// Creates the one generation-qualified SurfaceServer capability.
///
/// The handle slot is reserved before either event allocation or display
/// binding. Once `acquire_surface` succeeds, inserting into that reservation
/// is infallible, so no failure can strand a permanently bound session without
/// a userspace owner.
fn surface_acquire(
    frame: *mut TrapFrame,
    reserved0: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    if reserved0 != 0 || reserved1 != 0 || reserved2 != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    if !crate::process::current_is_surface_server() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let Some(process_id) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if crate::process::unique_live_process_id_for_image(UserImageId::SurfaceServer)
        != Some(process_id)
    {
        return complete(frame, Status::InvalidState, 0, 0);
    }
    let Some(session_id) = next_surface_session_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };

    let acquired = with_table(|table| {
        let reservation = table.reserve_slot().map_err(handle_error_status)?;
        let capability = SurfaceCapability::try_new(session_id).map_err(|_| Status::OutOfMemory)?;
        let evidence = crate::display::acquire_surface(&capability, process_id)
            .map_err(surface_display_error_status)?;
        let handle =
            reservation.insert_parts(KernelObject::from(capability), Rights::SURFACE_DEFAULT);
        Ok::<_, Status>((handle, evidence))
    });
    let (handle, evidence) = match acquired {
        Ok(acquired) => acquired,
        Err(status) => return complete(frame, status, 0, 0),
    };
    if evidence.owner != SurfaceOwner::KernelFallback
        || evidence.session_id != session_id
        || evidence.process_id != process_id
        || evidence.input.pending != 0
        || evidence.key_input.pending != 0
        || (!evidence.recovered
            && (evidence.previous_session_id != 0
                || evidence.previous_process_id != 0
                || evidence.discarded_input != 0
                || evidence.discarded_keys != 0
                || evidence.frozen_scene_digest != 0
                || evidence.frozen_scanout_digest != 0
                || evidence.frozen_cursor != bndroid_kernel::compositor::CursorState::hidden()
                || evidence.frozen_cursor_pixels != 0
                || evidence.frozen_scanout_valid))
        || (evidence.recovered
            && (evidence.previous_session_id == 0
                || evidence.previous_process_id == 0
                || evidence.previous_session_id == session_id
                || evidence.previous_process_id == process_id
                || evidence.frozen_scene_digest == 0
                || evidence.frozen_scanout_digest == 0
                || !evidence.frozen_scanout_valid
                || (!evidence.frozen_cursor.visible
                    && (evidence.frozen_cursor
                        != bndroid_kernel::compositor::CursorState::hidden()
                        || evidence.frozen_cursor_pixels != 0
                        || evidence.frozen_scene_digest != evidence.frozen_scanout_digest))
                || (evidence.frozen_cursor.visible
                    && (evidence.frozen_cursor.x >= bndroid_kernel::framebuffer::WIDTH
                        || evidence.frozen_cursor.y >= bndroid_kernel::framebuffer::HEIGHT
                        || evidence.frozen_cursor_pixels == 0))))
    {
        panic!("surface acquire published inconsistent ownership evidence");
    }
    // Slot reservation, event allocation, and display binding have all
    // committed. Advance the global namespace only now so a failed acquire
    // cannot make a later first successful server masquerade as recovery.
    commit_surface_session_id(session_id);
    if evidence.recovered {
        SURFACE_REACQUIRE_OLD_PID.store(evidence.previous_process_id, Ordering::Release);
        SURFACE_REACQUIRE_NEW_PID.store(process_id, Ordering::Release);
        SURFACE_REACQUIRE_OLD_SESSION.store(evidence.previous_session_id, Ordering::Release);
        SURFACE_REACQUIRE_NEW_SESSION.store(session_id, Ordering::Release);
        SURFACE_REACQUIRE_DISCARDED_INPUT.store(
            u64::try_from(evidence.discarded_input)
                .unwrap_or_else(|_| panic!("discarded surface input count did not fit u64")),
            Ordering::Release,
        );
        SURFACE_REACQUIRE_DISCARDED_KEYS.store(
            u64::try_from(evidence.discarded_keys)
                .unwrap_or_else(|_| panic!("discarded surface key count did not fit u64")),
            Ordering::Release,
        );
        SURFACE_REACQUIRE_FROZEN_CURSOR_X.store(
            u64::try_from(evidence.frozen_cursor.x)
                .unwrap_or_else(|_| panic!("frozen cursor x did not fit u64")),
            Ordering::Release,
        );
        SURFACE_REACQUIRE_FROZEN_CURSOR_Y.store(
            u64::try_from(evidence.frozen_cursor.y)
                .unwrap_or_else(|_| panic!("frozen cursor y did not fit u64")),
            Ordering::Release,
        );
        SURFACE_REACQUIRE_FROZEN_CURSOR_PIXELS.store(
            u64::try_from(evidence.frozen_cursor_pixels)
                .unwrap_or_else(|_| panic!("frozen cursor pixel count did not fit u64")),
            Ordering::Release,
        );
        SURFACE_REACQUIRE_FROZEN_CURSOR_VISIBLE
            .store(evidence.frozen_cursor.visible, Ordering::Release);
        SURFACE_REACQUIRE_FROZEN_CURSOR_PRESSED
            .store(evidence.frozen_cursor.pressed, Ordering::Release);
        SURFACE_REACQUIRE_FROZEN_SCANOUT_VALID
            .store(evidence.frozen_scanout_valid, Ordering::Release);
        SURFACE_REACQUIRES.fetch_add(1, Ordering::AcqRel);
        crate::kprintln!(
            "SURFACE_REACQUIRE_OK old_pid={} new_pid={} old_session={} new_session={} discarded_input={} frozen_scene={:#018x} frozen_scanout={:#018x} session_reset=1",
            evidence.previous_process_id,
            process_id,
            evidence.previous_session_id,
            session_id,
            evidence.discarded_input,
            evidence.frozen_scene_digest,
            evidence.frozen_scanout_digest,
        );
    }
    crate::kprintln!(
        "SURFACE_ACQUIRE_OK owner=kernel-fallback pid={} session={} handle={} rights={:#010x} unique=1 duplicate=0 transferable=0 input_capacity={}",
        process_id,
        session_id,
        handle.raw(),
        Rights::SURFACE_DEFAULT.bits(),
        SURFACE_INPUT_QUEUE_CAPACITY,
    );
    complete(frame, Status::Ok, u64::from(handle.raw()), session_id)
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn surface_frame_acquire(
    frame: *mut TrapFrame,
    raw_handle: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    SURFACE_FRAME_ACQUIRE_CALLS.fetch_add(1, Ordering::Relaxed);
    if reserved1 != 0 || reserved2 != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    if !crate::process::current_is_surface_server() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let Some(handle) = parse_handle(raw_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let capability = match with_table(|table| {
        table
            .get(handle, Rights::WRITE)
            .map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                handle_error_status(error)
            })?
            .as_surface()
            .cloned()
            .ok_or(Status::InvalidState)
    }) {
        Ok(capability) => capability,
        Err(status) => return complete(frame, status, 0, 0),
    };
    let Some(process_id) = crate::process::current_live_user_process_id() else {
        SURFACE_FRAME_ACQUIRE_INVALID_STATE.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let evidence = match crate::display::acquire_surface_frame(&capability, process_id) {
        Ok(evidence) => evidence,
        Err(error) => {
            let status = surface_display_error_status(error);
            if status == Status::ShouldWait {
                SURFACE_FRAME_ACQUIRE_SHOULD_WAIT.fetch_add(1, Ordering::Relaxed);
            } else if status == Status::InvalidState {
                SURFACE_FRAME_ACQUIRE_INVALID_STATE.fetch_add(1, Ordering::Relaxed);
            }
            return complete(frame, status, 0, 0);
        }
    };
    if evidence.session_id != capability.session_id()
        || evidence.process_id != process_id
        || evidence.grant.epoch == 0
    {
        panic!("surface frame acquire published inconsistent grant evidence");
    }
    SURFACE_FRAME_ACQUIRE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    crate::kprintln!(
        "SURFACE_FRAME_ACQUIRE_OK pid={} session={} epoch={} boundary={}",
        process_id,
        evidence.session_id,
        evidence.grant.epoch,
        evidence.grant.boundary_tick,
    );
    complete(
        frame,
        Status::Ok,
        evidence.grant.epoch,
        evidence.grant.boundary_tick,
    )
}

#[cfg(not(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
)))]
fn surface_frame_acquire(
    frame: *mut TrapFrame,
    _raw_handle: u64,
    _reserved1: u64,
    _reserved2: u64,
) -> *mut TrapFrame {
    complete(frame, Status::Unsupported, 0, 0)
}

/// Copies and decodes exactly one canonical 64-byte Present command before
/// entering the display critical section. A successful return is the frame's
/// serialization point and reports its monotonic frame/commit identifiers.
fn surface_present(
    frame: *mut TrapFrame,
    raw_handle: u64,
    user_present: u64,
    raw_length: u64,
) -> *mut TrapFrame {
    if !crate::process::current_is_surface_server() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let Some(handle) = parse_handle(raw_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    if raw_length != PRESENT_WIRE_SIZE as u64 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let capability = match with_table(|table| {
        table
            .get(handle, Rights::WRITE)
            .map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                handle_error_status(error)
            })?
            .as_surface()
            .cloned()
            .ok_or(Status::InvalidState)
    }) {
        Ok(capability) => capability,
        Err(status) => return complete(frame, status, 0, 0),
    };
    let mut wire = [0_u8; PRESENT_WIRE_SIZE];
    if crate::arch::aarch64::usercopy::copy_from_user(&mut wire, user_present).is_err() {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let present = match PresentFrame::decode(&wire) {
        Ok(present) => present,
        Err(_) => return complete(frame, Status::InvalidArgument, 0, 0),
    };
    let Some(process_id) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let evidence = match crate::display::present_surface(&capability, process_id, &present) {
        Ok(evidence) => evidence,
        Err(error) => return complete(frame, surface_display_error_status(error), 0, 0),
    };
    if evidence.session_id != capability.session_id()
        || evidence.process_id != process_id
        || evidence.surface.current_owner != SurfaceOwner::UserspaceBound
    {
        panic!("surface present published inconsistent display evidence");
    }
    let mode = match evidence.surface.mode {
        PresentMode::Full => "full",
        PresentMode::Damage => "damage",
    };
    if evidence.surface.previous_owner == SurfaceOwner::KernelFallback {
        crate::kprintln!(
            "SURFACE_HANDOFF_OK from=kernel-fallback to=userspace-bound caller=el0 pid={} session={} frame_id={} scene_digest={:#018x} scanout_digest={:#018x}",
            process_id,
            evidence.session_id,
            evidence.surface.frame_id,
            evidence.composition.scene_digest,
            evidence.composition.scanout_digest,
        );
    }
    let local = evidence.surface.local_damage;
    let global = evidence.surface.global_damage;
    let composition = evidence.composition.composition;
    if evidence.surface.frame_id == 1 || cfg!(feature = "surface-trace-evidence") {
        crate::kprintln!(
            "USER_SURFACE_COMMIT_OK owner=userspace pid={} session={} frame_id={} commit={} mode={} rects={} local_damage={}/{}/{}/{} global_damage={}/{}/{}/{} raster_writes={} composition={}/{}/{}/{} restored={} blended={} scene_digest={:#018x} scanout_digest={:#018x} input_enqueued={} input_dequeued={} input_pending={} input_coalesced={} cursor_preserved=1 dma_barrier=1",
            process_id,
            evidence.session_id,
            evidence.surface.frame_id,
            evidence.surface.commits,
            mode,
            present.rect_count(),
            local.x,
            local.y,
            local.width,
            local.height,
            global.x,
            global.y,
            global.width,
            global.height,
            evidence.surface.raster_writes,
            composition.x,
            composition.y,
            composition.width,
            composition.height,
            evidence.composition.restored_pixels,
            evidence.composition.blended_pixels,
            evidence.composition.scene_digest,
            evidence.composition.scanout_digest,
            evidence.input.enqueued,
            evidence.input.dequeued,
            evidence.input.pending,
            evidence.input.coalesced,
        );
    }
    complete(
        frame,
        Status::Ok,
        u64::from(evidence.surface.frame_id),
        evidence.surface.commits,
    )
}

/// Presents one canonical full-surface graphics-buffer transaction.
///
/// SurfaceServer must hold the unique Surface write capability and an
/// independently attenuated read-only buffer handle. Buffer generation
/// validation remains pinned through the complete display copy.
fn surface_present_buffer(
    frame: *mut TrapFrame,
    raw_surface: u64,
    raw_buffer: u64,
    user_present: u64,
) -> *mut TrapFrame {
    if !crate::process::current_is_surface_server() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let (Some(surface_handle), Some(buffer_handle)) =
        (parse_handle(raw_surface), parse_handle(raw_buffer))
    else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    if surface_handle == buffer_handle {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let objects = with_table(|table| {
        let capability = table
            .get(surface_handle, Rights::WRITE)
            .map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                handle_error_status(error)
            })?
            .as_surface()
            .cloned()
            .ok_or(Status::InvalidState)?;
        let buffer_rights = table.rights(buffer_handle).map_err(handle_error_status)?;
        if !matches!(
            buffer_rights,
            Rights::GRAPHICS_BUFFER_SERVER | Rights::GRAPHICS_BUFFER_MAPPED_SERVER
        ) {
            RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
            return Err(Status::PermissionDenied);
        }
        let buffer = table
            .get(buffer_handle, Rights::READ)
            .map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                handle_error_status(error)
            })?
            .as_graphics_buffer()
            .cloned()
            .ok_or(Status::InvalidState)?;
        Ok::<_, Status>((capability, buffer))
    });
    let (capability, buffer) = match objects {
        Ok(objects) => objects,
        Err(status) => return complete(frame, status, 0, 0),
    };
    let mut wire = [0_u8; BUFFER_PRESENT_WIRE_SIZE];
    if crate::arch::aarch64::usercopy::copy_from_user(&mut wire, user_present).is_err() {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let present = match BufferPresent::decode(&wire) {
        Ok(present) => present,
        Err(_) => return complete(frame, Status::InvalidArgument, 0, 0),
    };
    let Some(process_id) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    if buffer.is_mappable() {
        if let Err(error) = buffer.can_release(present.buffer_generation(), process_id) {
            return complete(frame, graphics_buffer_error_status(error), 0, 0);
        }
        // Validate the producer alias before the first display byte changes.
        // The syscall path is single-core with IRQ masked, so this exact live
        // Process and RO mapping cannot disappear before post-commit restore.
        let producer_mapping = match crate::process::graphics_buffer_mapping_snapshot(
            buffer.producer_pid(),
            &buffer,
        ) {
            Ok(mapping) => mapping,
            Err(error) => {
                return complete(frame, process_graphics_mapping_error_status(error), 0, 0);
            }
        };
        if producer_mapping.role != GraphicsMappingRole::Producer
            || producer_mapping.access != GraphicsMappingAccess::ReadOnly
        {
            return complete(frame, Status::InvalidState, 0, 0);
        }
    }
    #[cfg(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "multi-window-runtime"),
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    let swapchain_token = match bndroid_kernel::swapchain::preflight(
        buffer.identity(),
        present.buffer_generation(),
    ) {
        Ok(token) => token,
        Err(_) => return complete(frame, Status::InvalidState, 0, 0),
    };
    let evidence =
        match crate::display::present_surface_buffer(&capability, process_id, &buffer, &present) {
            Ok(evidence) => evidence,
            Err(error) => return complete(frame, surface_display_error_status(error), 0, 0),
        };
    if evidence.session_id != capability.session_id()
        || evidence.process_id != process_id
        || evidence.surface.current_owner != SurfaceOwner::UserspaceBound
        || evidence.surface.frame_id != present.global_frame_id()
    {
        panic!("graphics-buffer present published inconsistent display evidence");
    }
    #[cfg(any(
        feature = "input-server-restart-runtime",
        all(
            feature = "multi-window-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime"),
            not(feature = "el0-fault-containment-self-test"),
            not(feature = "process-terminate-self-test")
        )
    ))]
    {
        let slot = usize::from(buffer.slot());
        let allocation_generation = u32::try_from(buffer.slot_generation())
            .unwrap_or_else(|_| panic!("graphics allocation generation exceeds u32"));
        #[cfg(all(
            feature = "input-server-restart-runtime",
            not(feature = "service-supervisor-runtime"),
            not(feature = "service-dependency-runtime")
        ))]
        let accepted = if bndroid_kernel::window_trace::snapshot().output_commits < 6 {
            bndroid_kernel::window_trace::record_output_commit(
                slot,
                allocation_generation,
                present.buffer_generation(),
                evidence.surface.frame_id,
            )
        } else {
            bndroid_kernel::input_server_restart_trace::record_output_commit(
                process_id,
                evidence.session_id,
                slot,
                allocation_generation,
                present.buffer_generation(),
                evidence.surface.frame_id,
            )
        };
        #[cfg(all(
            feature = "service-supervisor-runtime",
            not(feature = "service-dependency-runtime")
        ))]
        let accepted = if bndroid_kernel::window_trace::snapshot().output_commits < 6 {
            bndroid_kernel::window_trace::record_output_commit(
                slot,
                allocation_generation,
                present.buffer_generation(),
                evidence.surface.frame_id,
            )
        } else if bndroid_kernel::input_server_restart_trace::snapshot().output_commits < 2 {
            bndroid_kernel::input_server_restart_trace::record_output_commit(
                process_id,
                evidence.session_id,
                slot,
                allocation_generation,
                present.buffer_generation(),
                evidence.surface.frame_id,
            )
        } else {
            bndroid_kernel::service_supervisor_trace::record_degraded_output(
                process_id,
                evidence.session_id,
                slot,
                allocation_generation,
                present.buffer_generation(),
                evidence.surface.frame_id,
            )
        };
        #[cfg(feature = "service-dependency-runtime")]
        let accepted = if bndroid_kernel::window_trace::snapshot().output_commits < 6 {
            bndroid_kernel::window_trace::record_output_commit(
                slot,
                allocation_generation,
                present.buffer_generation(),
                evidence.surface.frame_id,
            )
        } else if !bndroid_kernel::input_surface_recovery_trace::snapshot().complete {
            bndroid_kernel::input_surface_recovery_trace::record_output_commit(
                process_id,
                evidence.session_id,
                slot,
                allocation_generation,
                present.buffer_generation(),
                evidence.surface.frame_id,
            )
        } else {
            #[cfg(feature = "post-recovery-interaction-runtime")]
            {
                if bndroid_kernel::service_dependency_trace::snapshot().complete {
                    #[cfg(feature = "post-recovery-focus-runtime")]
                    {
                        if bndroid_kernel::post_recovery_interaction_trace::snapshot().complete {
                            #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
                            {
                                if bndroid_kernel::post_recovery_focus_trace::snapshot().complete {
                                    bndroid_kernel::post_recovery_focus_roundtrip_trace::record_output_commit(
                                        process_id,
                                        evidence.session_id,
                                        slot,
                                        allocation_generation,
                                        present.buffer_generation(),
                                        evidence.surface.frame_id,
                                    )
                                } else {
                                    bndroid_kernel::post_recovery_focus_trace::record_output_commit(
                                        process_id,
                                        evidence.session_id,
                                        slot,
                                        allocation_generation,
                                        present.buffer_generation(),
                                        evidence.surface.frame_id,
                                    )
                                }
                            }
                            #[cfg(not(feature = "post-recovery-focus-roundtrip-runtime"))]
                            {
                                bndroid_kernel::post_recovery_focus_trace::record_output_commit(
                                    process_id,
                                    evidence.session_id,
                                    slot,
                                    allocation_generation,
                                    present.buffer_generation(),
                                    evidence.surface.frame_id,
                                )
                            }
                        } else {
                            bndroid_kernel::post_recovery_interaction_trace::record_output_commit(
                                process_id,
                                evidence.session_id,
                                slot,
                                allocation_generation,
                                present.buffer_generation(),
                                evidence.surface.frame_id,
                            )
                        }
                    }
                    #[cfg(not(feature = "post-recovery-focus-runtime"))]
                    {
                        bndroid_kernel::post_recovery_interaction_trace::record_output_commit(
                            process_id,
                            evidence.session_id,
                            slot,
                            allocation_generation,
                            present.buffer_generation(),
                            evidence.surface.frame_id,
                        )
                    }
                } else {
                    bndroid_kernel::service_dependency_trace::record_output_commit(
                        process_id,
                        evidence.session_id,
                        slot,
                        allocation_generation,
                        present.buffer_generation(),
                        evidence.surface.frame_id,
                    )
                }
            }
            #[cfg(not(feature = "post-recovery-interaction-runtime"))]
            {
                bndroid_kernel::service_dependency_trace::record_output_commit(
                    process_id,
                    evidence.session_id,
                    slot,
                    allocation_generation,
                    present.buffer_generation(),
                    evidence.surface.frame_id,
                )
            }
        };
        #[cfg(all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime"),
            not(feature = "service-dependency-runtime")
        ))]
        let accepted = if bndroid_kernel::window_trace::snapshot().output_commits < 6 {
            bndroid_kernel::window_trace::record_output_commit(
                slot,
                allocation_generation,
                present.buffer_generation(),
                evidence.surface.frame_id,
            )
        } else {
            bndroid_kernel::input_surface_recovery_trace::record_output_commit(
                process_id,
                evidence.session_id,
                slot,
                allocation_generation,
                present.buffer_generation(),
                evidence.surface.frame_id,
            )
        };
        #[cfg(all(
            feature = "persistent-window-runtime",
            not(feature = "input-server-surface-restart-runtime"),
            not(feature = "input-server-restart-runtime"),
            not(feature = "service-dependency-runtime")
        ))]
        let accepted = bndroid_kernel::persistent_window_trace::record_output_commit(
            slot,
            allocation_generation,
            present.buffer_generation(),
            evidence.surface.frame_id,
        );
        #[cfg(all(
            not(feature = "persistent-window-runtime"),
            not(feature = "input-server-surface-restart-runtime"),
            not(feature = "input-server-restart-runtime"),
            not(feature = "service-dependency-runtime")
        ))]
        let accepted = bndroid_kernel::window_trace::record_output_commit(
            slot,
            allocation_generation,
            present.buffer_generation(),
            evidence.surface.frame_id,
        );
        #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
        let _ = bndroid_kernel::post_recovery_lifecycle_focus_trace::record_output_commit(
            process_id,
            evidence.session_id,
            slot,
            allocation_generation,
            present.buffer_generation(),
            evidence.surface.frame_id,
        );
        if !accepted {
            panic!(
                "userspace compositor output commit diverged from its authenticated window transcript"
            );
        }
        #[cfg(all(
            feature = "text-input-runtime",
            not(feature = "input-server-surface-restart-runtime"),
            not(feature = "input-server-restart-runtime"),
            not(feature = "service-dependency-runtime")
        ))]
        let text_output_accepted = {
            #[cfg(feature = "soft-keyboard-runtime")]
            if bndroid_kernel::text_input_trace::snapshot().final_complete {
                bndroid_kernel::soft_keyboard_trace::record_output_commit(
                    slot,
                    allocation_generation,
                    present.buffer_generation(),
                    evidence.surface.frame_id,
                )
            } else {
                bndroid_kernel::text_input_trace::record_output_commit(
                    slot,
                    allocation_generation,
                    present.buffer_generation(),
                    evidence.surface.frame_id,
                )
            }
            #[cfg(not(feature = "soft-keyboard-runtime"))]
            bndroid_kernel::text_input_trace::record_output_commit(
                slot,
                allocation_generation,
                present.buffer_generation(),
                evidence.surface.frame_id,
            )
        };
        #[cfg(all(
            feature = "text-input-runtime",
            not(feature = "input-server-surface-restart-runtime"),
            not(feature = "input-server-restart-runtime"),
            not(feature = "service-dependency-runtime")
        ))]
        if !text_output_accepted {
            panic!("text-input render output diverged from its authenticated state acknowledgment");
        }
    }
    #[cfg(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "multi-window-runtime"),
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    bndroid_kernel::swapchain::commit(swapchain_token)
        .unwrap_or_else(|error| panic!("preflighted swapchain commit failed: {error:?}"));
    if buffer.is_mappable() {
        crate::process::restore_graphics_buffer_producer_mapping(buffer.producer_pid(), &buffer)
            .unwrap_or_else(|error| {
                panic!(
                    "committed mapped present could not restore producer mapping: {}",
                    error.as_str()
                )
            });
        buffer
            .release(present.buffer_generation(), process_id)
            .unwrap_or_else(|error| {
                panic!(
                    "committed mapped present could not release queue: {}",
                    error.as_str()
                )
            });
        GRAPHICS_BUFFER_RELEASES.fetch_add(1, Ordering::Relaxed);
        GRAPHICS_BUFFER_MAPPED_PRESENTS.fetch_add(1, Ordering::Relaxed);
        crate::scheduler::wake_object_waiters();
    }
    GRAPHICS_BUFFER_PRESENTS.fetch_add(1, Ordering::Relaxed);
    let global = evidence.surface.global_damage;
    let composition = evidence.composition.composition;
    crate::kprintln!(
        "USER_SURFACE_BUFFER_COMMIT_OK owner=userspace pid={} session={} producer_pid={} client_frame_id={} frame_id={} commit={} mode=full buffer_generation={} format=xrgb8888 width={} height={} global_damage={}/{}/{}/{} raster_writes={} composition={}/{}/{}/{} scene_digest={:#018x} scanout_digest={:#018x} cursor_preserved=1 dma_barrier=1",
        process_id,
        evidence.session_id,
        buffer.producer_pid(),
        present.client_frame_id(),
        evidence.surface.frame_id,
        evidence.surface.commits,
        present.buffer_generation(),
        GRAPHICS_BUFFER_WIDTH,
        GRAPHICS_BUFFER_HEIGHT,
        global.x,
        global.y,
        global.width,
        global.height,
        evidence.surface.raster_writes,
        composition.x,
        composition.y,
        composition.width,
        composition.height,
        evidence.composition.scene_digest,
        evidence.composition.scanout_digest,
    );
    #[cfg(all(
        feature = "graphics-frame-clock-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    {
        let pacing = crate::display::frame_clock_snapshot()
            .unwrap_or_else(|| panic!("paced buffer commit lost its frame-clock binding"));
        if pacing.session_id != evidence.session_id
            || pacing.process_id != process_id
            || pacing.clock.last_presented_epoch == 0
            || pacing.clock.presented != evidence.surface.commits
        {
            panic!("paced buffer commit published inconsistent clock evidence");
        }
        crate::kprintln!(
            "SURFACE_FRAME_COMMIT_OK pid={} session={} epoch={} frame_id={} buffer_generation={}",
            process_id,
            evidence.session_id,
            pacing.clock.last_presented_epoch,
            evidence.surface.frame_id,
            present.buffer_generation(),
        );
    }
    #[cfg(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "multi-window-runtime"),
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    crate::kprintln!(
        "GRAPHICS_SWAPCHAIN_COMMIT_OK frame={} slot={} allocation_generation={} buffer_generation={} release=post-copy",
        evidence.surface.frame_id,
        buffer.slot(),
        buffer.slot_generation(),
        present.buffer_generation(),
    );
    complete(
        frame,
        Status::Ok,
        u64::from(evidence.surface.frame_id),
        evidence.surface.commits,
    )
}

/// Presents one authenticated client content layer beneath SurfaceServer-owned
/// system chrome.
///
/// The two exact read handles, producer identities, wire, and generations are
/// validated before `display` changes the scene. This syscall is deliberately
/// separate from the predecessor full-surface path so ABI 45 remains byte-for-
/// byte compatible and cannot gain implicit chrome authority.
#[cfg(feature = "androidbox-interactive0")]
fn surface_present_buffer_layers(
    frame: *mut TrapFrame,
    raw_surface: u64,
    packed_buffers: u64,
    user_present: u64,
) -> *mut TrapFrame {
    if !crate::process::current_is_surface_server() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let Some(surface_handle) = parse_handle(raw_surface) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let (content_handle, chrome_handle) = unpack_surface_layer_handles(packed_buffers);
    if content_handle.raw() == 0 || chrome_handle.raw() == 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    if let Err(error) =
        validate_layered_present_preflight(LayeredPresentPreflight::HandleTopology {
            surface_content_and_chrome_are_distinct: surface_handle != content_handle
                && surface_handle != chrome_handle
                && content_handle != chrome_handle,
        })
    {
        return complete(frame, layered_present_preflight_error_status(error), 0, 0);
    }
    let objects = with_table(|table| {
        let capability = table
            .get(surface_handle, Rights::WRITE)
            .map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                handle_error_status(error)
            })?
            .as_surface()
            .cloned()
            .ok_or(Status::InvalidState)?;
        let content_rights = table.rights(content_handle).map_err(handle_error_status)?;
        let chrome_rights = table.rights(chrome_handle).map_err(handle_error_status)?;
        if let Err(error) =
            validate_layered_present_preflight(LayeredPresentPreflight::BufferRights {
                content: content_rights,
                chrome: chrome_rights,
            })
        {
            RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
            return Err(layered_present_preflight_error_status(error));
        }
        let content = table
            .get(content_handle, Rights::READ)
            .map_err(handle_error_status)?
            .as_graphics_buffer()
            .cloned()
            .ok_or(Status::InvalidState)?;
        let chrome = table
            .get(chrome_handle, Rights::READ)
            .map_err(handle_error_status)?
            .as_graphics_buffer()
            .cloned()
            .ok_or(Status::InvalidState)?;
        Ok::<_, Status>((capability, content, chrome))
    });
    let (capability, content, chrome) = match objects {
        Ok(objects) => objects,
        Err(status) => return complete(frame, status, 0, 0),
    };
    let Some(process_id) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let launcher_pid = crate::process::unique_live_process_id_for_image(UserImageId::Launcher);
    let app_pid = crate::process::unique_live_process_id_for_image(UserImageId::App);

    let mut wire = [0_u8; BUFFER_PRESENT_WIRE_SIZE];
    if crate::arch::aarch64::usercopy::copy_from_user(&mut wire, user_present).is_err() {
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let present = match BufferPresent::decode(&wire) {
        Ok(present) => present,
        Err(_) => return complete(frame, Status::InvalidArgument, 0, 0),
    };
    if let Err(error) = validate_layered_present_preflight(LayeredPresentPreflight::Submission(
        LayeredPresentSubmissionPreflight {
            content_and_chrome_are_same_buffer: content.same_buffer(&chrome),
            content_is_mappable: content.is_mappable(),
            chrome_is_mappable: chrome.is_mappable(),
            content_producer_pid: content.producer_pid(),
            launcher_pid,
            app_pid,
            chrome_producer_pid: chrome.producer_pid(),
            current_surface_server_pid: process_id,
            requested_content_generation: present.buffer_generation(),
            current_content_generation: content.write_generation(),
            requested_chrome_generation: present.system_chrome_generation(),
            current_chrome_generation: chrome.write_generation(),
        },
    )) {
        return complete(frame, layered_present_preflight_error_status(error), 0, 0);
    }
    // `with_pixels_pair` repeats both generation checks atomically while
    // pinning the source slices. Every other user-controlled rejection has
    // completed here, before display can change the scene.
    let evidence = match crate::display::present_surface_buffer_layers(
        &capability,
        process_id,
        &content,
        &chrome,
        &present,
    ) {
        Ok(evidence) => evidence,
        Err(error) => return complete(frame, surface_display_error_status(error), 0, 0),
    };
    if evidence.session_id != capability.session_id()
        || evidence.process_id != process_id
        || evidence.surface.current_owner != SurfaceOwner::UserspaceBound
        || evidence.surface.frame_id != present.global_frame_id()
    {
        panic!("layered graphics-buffer present published inconsistent display evidence");
    }
    #[cfg(feature = "androidbox-restart0")]
    {
        let restart = android_app_restart_trace_mut();
        if restart.phase == AndroidAppRestartPhase::Complete
            && restart.errors == 0
            && restart.reopen_completions == 1
            && content.producer_pid() == restart.app_pid
            && restart.reopen_layered_commits < 5
        {
            restart.reopen_layered_commits += 1;
        }
    }
    GRAPHICS_BUFFER_PRESENTS.fetch_add(1, Ordering::Relaxed);
    let global = evidence.surface.global_damage;
    let composition = evidence.composition.composition;
    crate::kprintln!(
        "USER_SURFACE_LAYERED_COMMIT_OK owner=surface-server pid={} session={} content_producer_pid={} chrome_producer_pid={} client_frame_id={} frame_id={} commit={} mode=content-plus-system-chrome content_generation={} chrome_generation={} format=xrgb8888 surface={}x{} viewport={}/{}/{}/{} chrome_regions=0-64/1512-1600 global_damage={}/{}/{}/{} raster_writes={} composition={}/{}/{}/{} scene_digest={:#018x} scanout_digest={:#018x} atomic_sources=2 system_chrome_owner=surface-server",
        process_id,
        evidence.session_id,
        content.producer_pid(),
        chrome.producer_pid(),
        present.client_frame_id(),
        evidence.surface.frame_id,
        evidence.surface.commits,
        present.buffer_generation(),
        present.system_chrome_generation(),
        GRAPHICS_BUFFER_WIDTH,
        GRAPHICS_BUFFER_HEIGHT,
        bndr_ui::MOBILE_CONTENT_VIEWPORT_X,
        bndr_ui::MOBILE_CONTENT_VIEWPORT_Y,
        bndr_ui::MOBILE_CONTENT_VIEWPORT_WIDTH,
        bndr_ui::MOBILE_CONTENT_VIEWPORT_HEIGHT,
        global.x,
        global.y,
        global.width,
        global.height,
        evidence.surface.raster_writes,
        composition.x,
        composition.y,
        composition.width,
        composition.height,
        evidence.composition.scene_digest,
        evidence.composition.scanout_digest,
    );
    complete(
        frame,
        Status::Ok,
        u64::from(evidence.surface.frame_id),
        evidence.surface.commits,
    )
}

fn surface_read_input(
    frame: *mut TrapFrame,
    raw_handle: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    if reserved1 != 0 || reserved2 != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    if !crate::process::current_is_surface_server() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    let Some(handle) = parse_handle(raw_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let capability = match with_table(|table| {
        table
            .get(handle, Rights::READ)
            .map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                handle_error_status(error)
            })?
            .as_surface()
            .cloned()
            .ok_or(Status::InvalidState)
    }) {
        Ok(capability) => capability,
        Err(status) => return complete(frame, status, 0, 0),
    };
    if capability.signals().intersects(ObjectSignals::PEER_CLOSED) {
        return complete(frame, Status::PeerClosed, 0, 0);
    }
    let Some(process_id) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    let evidence = match crate::display::read_surface_input(&capability, process_id) {
        Ok(evidence) => evidence,
        Err(error) => return complete(frame, surface_display_error_status(error), 0, 0),
    };
    match evidence.owner {
        SurfaceOwner::UserspaceBound => {}
        SurfaceOwner::Degraded => return complete(frame, Status::PeerClosed, 0, 0),
        SurfaceOwner::KernelFallback => {
            return complete(frame, Status::InvalidState, 0, 0);
        }
    }
    let Some(sample) = evidence.sample else {
        let status = if capability.signals().intersects(ObjectSignals::PEER_CLOSED) {
            Status::PeerClosed
        } else {
            Status::ShouldWait
        };
        return complete(frame, status, 0, 0);
    };
    let (state, sequence) = sample.encode_registers();
    #[cfg(feature = "surface-trace-evidence")]
    crate::kprintln!(
        "USER_INPUT_READ_OK owner=userspace pid={} session={} sequence={} x={} y={} pressed={} pending={} enqueued={} dequeued={} high_water={} coalesced={}",
        process_id,
        capability.session_id(),
        sample.sequence(),
        sample.x(),
        sample.y(),
        u8::from(sample.pressed()),
        evidence.input.pending,
        evidence.input.enqueued,
        evidence.input.dequeued,
        evidence.input.high_watermark,
        evidence.input.coalesced,
    );
    complete(frame, Status::Ok, state, sequence)
}

fn surface_read_key(
    frame: *mut TrapFrame,
    raw_handle: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    SURFACE_KEY_READ_CALLS.fetch_add(1, Ordering::Relaxed);
    if reserved1 != 0 || reserved2 != 0 {
        return complete_surface_key_read(frame, Status::InvalidArgument, 0, 0);
    }
    if !crate::process::current_is_surface_server() {
        return complete_surface_key_read(frame, Status::PermissionDenied, 0, 0);
    }
    let Some(handle) = parse_handle(raw_handle) else {
        return complete_surface_key_read(frame, Status::InvalidArgument, 0, 0);
    };
    let capability = match with_table(|table| {
        table
            .get(handle, Rights::READ)
            .map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                handle_error_status(error)
            })?
            .as_surface()
            .cloned()
            .ok_or(Status::InvalidState)
    }) {
        Ok(capability) => capability,
        Err(status) => return complete_surface_key_read(frame, status, 0, 0),
    };
    if capability.signals().intersects(ObjectSignals::PEER_CLOSED) {
        return complete_surface_key_read(frame, Status::PeerClosed, 0, 0);
    }
    let Some(process_id) = crate::process::current_live_user_process_id() else {
        return complete_surface_key_read(frame, Status::InvalidState, 0, 0);
    };
    let evidence = match crate::display::read_surface_key(&capability, process_id) {
        Ok(evidence) => evidence,
        Err(error) => {
            return complete_surface_key_read(frame, surface_display_error_status(error), 0, 0);
        }
    };
    match evidence.owner {
        SurfaceOwner::UserspaceBound => {}
        SurfaceOwner::Degraded => {
            return complete_surface_key_read(frame, Status::PeerClosed, 0, 0);
        }
        SurfaceOwner::KernelFallback => {
            return complete_surface_key_read(frame, Status::InvalidState, 0, 0);
        }
    }
    let Some(sample) = evidence.sample else {
        let status = if capability.signals().intersects(ObjectSignals::PEER_CLOSED) {
            Status::PeerClosed
        } else {
            Status::ShouldWait
        };
        return complete_surface_key_read(frame, status, 0, 0);
    };
    let (state, sequence) = sample.encode_registers();
    complete_surface_key_read(frame, Status::Ok, state, sequence)
}

fn complete_surface_key_read(
    frame: *mut TrapFrame,
    status: Status,
    result1: u64,
    result2: u64,
) -> *mut TrapFrame {
    match status {
        Status::Ok => SURFACE_KEY_READ_SUCCESSES.fetch_add(1, Ordering::Relaxed),
        Status::ShouldWait => SURFACE_KEY_READ_SHOULD_WAIT.fetch_add(1, Ordering::Relaxed),
        Status::PeerClosed => SURFACE_KEY_READ_PEER_CLOSED.fetch_add(1, Ordering::Relaxed),
        Status::InvalidArgument => {
            SURFACE_KEY_READ_INVALID_ARGUMENT.fetch_add(1, Ordering::Relaxed)
        }
        Status::PermissionDenied => {
            SURFACE_KEY_READ_PERMISSION_DENIED.fetch_add(1, Ordering::Relaxed)
        }
        Status::InvalidState => SURFACE_KEY_READ_INVALID_STATE.fetch_add(1, Ordering::Relaxed),
        _ => 0,
    };
    complete(frame, status, result1, result2)
}

fn next_surface_session_id() -> Option<u64> {
    let current = NEXT_SURFACE_SESSION_ID.load(Ordering::Acquire);
    (current != 0 && current.checked_add(1).is_some()).then_some(current)
}

fn commit_surface_session_id(session_id: u64) {
    let next = session_id
        .checked_add(1)
        .unwrap_or_else(|| panic!("prevalidated surface session ID overflowed"));
    NEXT_SURFACE_SESSION_ID
        .compare_exchange(session_id, next, Ordering::AcqRel, Ordering::Acquire)
        .unwrap_or_else(|_| panic!("surface session namespace changed inside one syscall"));
}

fn graphics_buffer_error_status(error: GraphicsBufferError) -> Status {
    match error {
        GraphicsBufferError::InvalidProducer
        | GraphicsBufferError::EmptyWrite
        | GraphicsBufferError::Unaligned
        | GraphicsBufferError::OutOfRange
        | GraphicsBufferError::NonCanonicalPixel
        | GraphicsBufferError::NonZeroPadding => Status::InvalidArgument,
        GraphicsBufferError::Exhausted => Status::OutOfMemory,
        GraphicsBufferError::InvalidConsumer => Status::PermissionDenied,
        GraphicsBufferError::NotMappable
        | GraphicsBufferError::MappableWriteDenied
        | GraphicsBufferError::InvalidQueueState
        | GraphicsBufferError::WriteGenerationExhausted
        | GraphicsBufferError::StaleGeneration => Status::InvalidState,
    }
}

#[cfg(feature = "androidbox-interactive0")]
fn layered_present_preflight_error_status(error: LayeredPresentPreflightError) -> Status {
    match error {
        LayeredPresentPreflightError::AliasedHandles
        | LayeredPresentPreflightError::ZeroChromeGeneration => Status::InvalidArgument,
        LayeredPresentPreflightError::InvalidContentRights
        | LayeredPresentPreflightError::InvalidChromeRights
        | LayeredPresentPreflightError::InvalidContentProducer
        | LayeredPresentPreflightError::InvalidChromeProducer => Status::PermissionDenied,
        LayeredPresentPreflightError::AliasedBuffers
        | LayeredPresentPreflightError::MappableContent
        | LayeredPresentPreflightError::MappableChrome
        | LayeredPresentPreflightError::StaleContentGeneration
        | LayeredPresentPreflightError::StaleChromeGeneration => Status::InvalidState,
    }
}

fn process_graphics_mapping_error_status(
    error: crate::process::ProcessGraphicsMappingError,
) -> Status {
    use crate::arch::aarch64::mmu::GraphicsMappingError;
    use crate::process::ProcessGraphicsMappingError;
    match error {
        ProcessGraphicsMappingError::InvalidState => Status::InvalidState,
        ProcessGraphicsMappingError::NotFound => Status::NotFound,
        ProcessGraphicsMappingError::IdentityMismatch => Status::PermissionDenied,
        ProcessGraphicsMappingError::AddressSpace(mapping) => match mapping {
            GraphicsMappingError::OutOfMemory => Status::OutOfMemory,
            GraphicsMappingError::InvalidBuffer | GraphicsMappingError::InvalidAddress => {
                Status::InvalidArgument
            }
            GraphicsMappingError::AlreadyMapped => Status::AlreadyExists,
            GraphicsMappingError::NotMapped => Status::NotFound,
            GraphicsMappingError::IdentityMismatch | GraphicsMappingError::WrongRole => {
                Status::PermissionDenied
            }
            GraphicsMappingError::WrongAccess | GraphicsMappingError::HierarchyCorrupt => {
                Status::InvalidState
            }
            GraphicsMappingError::MmuNotEnabled | GraphicsMappingError::Busy => Status::Unavailable,
        },
    }
}

fn surface_display_error_status(error: crate::display::DisplayError) -> Status {
    use crate::display::DisplayError;
    match error {
        DisplayError::NotReady
        | DisplayError::FwCfg(_)
        | DisplayError::Directory(_)
        | DisplayError::TooManyFiles
        | DisplayError::InvalidFramebufferAddress
        | DisplayError::Render(_)
        | DisplayError::SceneMismatch
        | DisplayError::AlreadyInitialized => Status::Unavailable,
        DisplayError::InvalidCursorCoordinate
        | DisplayError::InvalidPointerSample
        | DisplayError::Compose(_) => Status::InvalidArgument,
        DisplayError::CounterExhausted
        | DisplayError::SurfaceNotAcquired
        | DisplayError::FrameGrantAlreadyOutstanding
        | DisplayError::FrameGrantRequired
        | DisplayError::SceneWriterOwnedByUserspace => Status::InvalidState,
        DisplayError::FrameOpportunityUnavailable => Status::ShouldWait,
        DisplayError::SurfaceAlreadyAcquired => Status::AlreadyExists,
        DisplayError::SurfaceCapabilityMismatch | DisplayError::SurfaceProcessMismatch => {
            Status::PermissionDenied
        }
        DisplayError::Surface(SurfaceError::Protocol(protocol)) => match protocol {
            ProtocolError::FirstFrameMustBeFull
            | ProtocolError::FirstFrameIdMustBeOne
            | ProtocolError::FrameIdExhausted
            | ProtocolError::FrameReplay
            | ProtocolError::FrameGap => Status::InvalidState,
            _ => Status::InvalidArgument,
        },
        DisplayError::Surface(SurfaceError::WrongScenePixelCount) => Status::InvalidState,
        DisplayError::Surface(
            SurfaceError::WrongBufferPixelCount | SurfaceError::NonCanonicalBufferPixel,
        ) => Status::InvalidArgument,
        DisplayError::Surface(SurfaceError::CommitCounterExhausted) => Status::InvalidState,
        DisplayError::Surface(SurfaceError::Degraded) => Status::PeerClosed,
        DisplayError::GraphicsBuffer(error) => graphics_buffer_error_status(error),
        DisplayError::SurfaceInput(SurfaceInputError::InvalidSample(_)) => Status::InvalidArgument,
        DisplayError::SurfaceInput(
            SurfaceInputError::SequenceExhausted | SurfaceInputError::CounterExhausted,
        ) => Status::InvalidState,
        DisplayError::SurfaceKey(SurfaceKeyError::InvalidSample(_)) => Status::InvalidArgument,
        DisplayError::SurfaceKey(SurfaceKeyError::Full) => Status::ShouldWait,
        DisplayError::SurfaceKey(
            SurfaceKeyError::SequenceExhausted | SurfaceKeyError::CounterExhausted,
        ) => Status::InvalidState,
    }
}

#[cfg(feature = "input-server-runtime")]
fn input_broker_error_status(error: bndroid_kernel::input_broker::InputBrokerError) -> Status {
    use bndroid_kernel::input_broker::InputBrokerError;
    match error {
        InputBrokerError::Unbound | InputBrokerError::PeerClosed => Status::PeerClosed,
        InputBrokerError::CapabilityMismatch => Status::PermissionDenied,
        InputBrokerError::InvalidEvent(_) => Status::InvalidArgument,
        InputBrokerError::SequenceExhausted
        | InputBrokerError::Full
        | InputBrokerError::CounterExhausted => Status::InvalidState,
    }
}

fn object_wait(
    frame: *mut TrapFrame,
    raw_handle: u64,
    raw_requested: u64,
    reserved: u64,
) -> *mut TrapFrame {
    let Some(handle) = parse_handle(raw_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let Ok(requested_bits) = u32::try_from(raw_requested) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let Some(requested) = ObjectSignals::from_bits(requested_bits) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    if requested.bits() == 0 || reserved != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let observed = with_table(|table| {
        let object = table.get(handle, Rights::WAIT).map_err(|error| {
            if error == HandleError::AccessDenied {
                RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
            }
            handle_error_status(error)
        })?;
        if !object.signal_mask().contains(requested) {
            return Err(Status::InvalidArgument);
        }
        Ok::<_, Status>((object.signals(), object.as_event().is_some()))
    });
    let (observed, is_event) = match observed {
        Ok(observed) => observed,
        Err(status) => return complete(frame, status, 0, 0),
    };
    OBJECT_WAIT_CALLS.fetch_add(1, Ordering::Relaxed);
    if is_event {
        EVENT_WAIT_CALLS.fetch_add(1, Ordering::Relaxed);
    }
    if observed.intersects(requested) {
        OBJECT_WAIT_IMMEDIATE.fetch_add(1, Ordering::Relaxed);
        complete(frame, Status::Ok, u64::from(observed.bits()), 0)
    } else {
        if is_event {
            EVENT_WAIT_BLOCKS.fetch_add(1, Ordering::Relaxed);
        }
        crate::scheduler::wait_current_user_object(frame, raw_handle, requested)
    }
}

fn object_wait_many(
    frame: *mut TrapFrame,
    raw_first: u64,
    raw_second: u64,
    timeout_ns: u64,
) -> *mut TrapFrame {
    let packed = [raw_first, raw_second];
    let mut handles = [HandleValue::INVALID; OBJECT_WAIT_MANY_ITEM_COUNT];
    let mut requested = [ObjectSignals::NONE; OBJECT_WAIT_MANY_ITEM_COUNT];
    for (index, raw_item) in packed.into_iter().enumerate() {
        let (handle, raw_signals) = unpack_wait_item(raw_item);
        let Some(signals) = ObjectSignals::from_bits(raw_signals) else {
            return complete(frame, Status::InvalidArgument, 0, 0);
        };
        if !handle.is_valid() || signals.bits() == 0 {
            return complete(frame, Status::InvalidArgument, 0, 0);
        }
        handles[index] = handle;
        requested[index] = signals;
    }

    let deadline = if timeout_ns == OBJECT_WAIT_TIMEOUT_INFINITE {
        None
    } else if timeout_ns == OBJECT_WAIT_TIMEOUT_POLL {
        Some(crate::arch::aarch64::timer::counter_value())
    } else {
        let now = crate::arch::aarch64::timer::counter_value();
        match bndroid_kernel::time::counter_deadline_after_nanoseconds(
            now,
            crate::arch::aarch64::timer::frequency_hz(),
            timeout_ns,
        ) {
            Ok(deadline) => Some(deadline),
            Err(_) => return complete(frame, Status::InvalidArgument, 0, 0),
        }
    };

    // Validate the entire set before considering readiness. A ready first
    // item must not hide an invalid or unauthorized second item.
    let observed = with_table(|table| {
        let mut observed = [ObjectSignals::NONE; OBJECT_WAIT_MANY_ITEM_COUNT];
        for index in 0..OBJECT_WAIT_MANY_ITEM_COUNT {
            let object = table.get(handles[index], Rights::WAIT).map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                handle_error_status(error)
            })?;
            if !object.signal_mask().contains(requested[index]) {
                return Err(Status::InvalidArgument);
            }
            observed[index] = object.signals();
        }
        Ok::<_, Status>(observed)
    });
    let observed = match observed {
        Ok(observed) => observed,
        Err(status) => return complete(frame, status, 0, 0),
    };

    WAIT_MANY_CALLS.fetch_add(1, Ordering::Relaxed);
    for index in 0..OBJECT_WAIT_MANY_ITEM_COUNT {
        if observed[index].intersects(requested[index]) {
            WAIT_MANY_IMMEDIATE.fetch_add(1, Ordering::Relaxed);
            return complete(
                frame,
                Status::Ok,
                index as u64,
                u64::from(observed[index].bits()),
            );
        }
    }

    if timeout_ns == OBJECT_WAIT_TIMEOUT_POLL
        || deadline.is_some_and(|deadline| {
            bndroid_kernel::time::deadline_reached(
                crate::arch::aarch64::timer::counter_value(),
                deadline,
            )
        })
    {
        WAIT_MANY_POLL_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::Timeout, u64::MAX, 0);
    }

    let items = [
        ObjectWaitItem::new(u64::from(handles[0].raw()), requested[0]),
        ObjectWaitItem::new(u64::from(handles[1].raw()), requested[1]),
    ];
    crate::scheduler::wait_current_user_objects(frame, &items, deadline)
}

/// Waits on one to eight generation-qualified object handles supplied through
/// a canonical little-endian user array.
///
/// The complete array is copied and validated before readiness is considered,
/// so a ready early item cannot hide a malformed, stale, unauthorized, or
/// object-incompatible later item. The bounded 64-byte copy keeps the current
/// single-core PAN-on usercopy contract while removing the two-register shape
/// as a service-runtime limitation.
fn object_wait_many_array(
    frame: *mut TrapFrame,
    user_items: u64,
    raw_count: u64,
    timeout_ns: u64,
) -> *mut TrapFrame {
    let Ok(count) = usize::try_from(raw_count) else {
        WAIT_ARRAY_INVALID_COUNTS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    if count == 0 || count > OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS {
        WAIT_ARRAY_INVALID_COUNTS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::InvalidArgument, 0, 0);
    }

    let deadline = if timeout_ns == OBJECT_WAIT_TIMEOUT_INFINITE {
        None
    } else if timeout_ns == OBJECT_WAIT_TIMEOUT_POLL {
        Some(crate::arch::aarch64::timer::counter_value())
    } else {
        let now = crate::arch::aarch64::timer::counter_value();
        match bndroid_kernel::time::counter_deadline_after_nanoseconds(
            now,
            crate::arch::aarch64::timer::frequency_hz(),
            timeout_ns,
        ) {
            Ok(deadline) => Some(deadline),
            Err(_) => {
                WAIT_ARRAY_VALIDATION_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                return complete(frame, Status::InvalidArgument, 0, 0);
            }
        }
    };

    let wire_length = count
        .checked_mul(OBJECT_WAIT_MANY_ARRAY_ITEM_WIRE_SIZE)
        .filter(|length| *length <= OBJECT_WAIT_MANY_ARRAY_MAX_WIRE_SIZE)
        .unwrap_or_else(|| panic!("validated wait-array length exceeded its ABI bound"));
    let mut wire = [0_u8; OBJECT_WAIT_MANY_ARRAY_MAX_WIRE_SIZE];
    if crate::arch::aarch64::usercopy::copy_from_user(&mut wire[..wire_length], user_items).is_err()
    {
        WAIT_ARRAY_BAD_ADDRESSES.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::BadAddress, 0, 0);
    }

    let mut handles = [HandleValue::INVALID; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    let mut requested = [ObjectSignals::NONE; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    let mut items = [ObjectWaitItem::new(0, ObjectSignals::NONE); OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    for index in 0..count {
        let offset = index * OBJECT_WAIT_MANY_ARRAY_ITEM_WIRE_SIZE;
        let mut packed_bytes = [0_u8; OBJECT_WAIT_MANY_ARRAY_ITEM_WIRE_SIZE];
        packed_bytes.copy_from_slice(&wire[offset..offset + OBJECT_WAIT_MANY_ARRAY_ITEM_WIRE_SIZE]);
        let (handle, raw_signals) = unpack_wait_item(u64::from_le_bytes(packed_bytes));
        let Some(signals) = ObjectSignals::from_bits(raw_signals) else {
            WAIT_ARRAY_VALIDATION_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            return complete(frame, Status::InvalidArgument, 0, 0);
        };
        if !handle.is_valid() || signals.bits() == 0 {
            WAIT_ARRAY_VALIDATION_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            return complete(frame, Status::InvalidArgument, 0, 0);
        }
        handles[index] = handle;
        requested[index] = signals;
        items[index] = ObjectWaitItem::new(u64::from(handle.raw()), signals);
    }

    // Validate the entire copied set while holding the owning HandleTable lock
    // before sampling any readiness result.
    let observed = with_table(|table| {
        let mut observed = [ObjectSignals::NONE; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        for index in 0..count {
            let object = table.get(handles[index], Rights::WAIT).map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                handle_error_status(error)
            })?;
            if !object.signal_mask().contains(requested[index]) {
                return Err(Status::InvalidArgument);
            }
            observed[index] = object.signals();
        }
        Ok::<_, Status>(observed)
    });
    let observed = match observed {
        Ok(observed) => observed,
        Err(status) => {
            WAIT_ARRAY_VALIDATION_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            return complete(frame, status, 0, 0);
        }
    };

    WAIT_ARRAY_CALLS.fetch_add(1, Ordering::Relaxed);
    WAIT_ARRAY_MAX_ITEMS_OBSERVED.fetch_max(count as u64, Ordering::Relaxed);
    let mut first_ready = None;
    let mut ready_count = 0_u64;
    for index in 0..count {
        if observed[index].intersects(requested[index]) {
            ready_count += 1;
            if first_ready.is_none() {
                first_ready = Some(index);
            }
        }
    }
    if let Some(index) = first_ready {
        WAIT_ARRAY_IMMEDIATE.fetch_add(1, Ordering::Relaxed);
        if ready_count > 1 {
            WAIT_ARRAY_LOWEST_MULTI_READY.fetch_add(1, Ordering::Relaxed);
        }
        return complete(
            frame,
            Status::Ok,
            index as u64,
            u64::from(observed[index].bits()),
        );
    }

    if timeout_ns == OBJECT_WAIT_TIMEOUT_POLL
        || deadline.is_some_and(|deadline| {
            bndroid_kernel::time::deadline_reached(
                crate::arch::aarch64::timer::counter_value(),
                deadline,
            )
        })
    {
        WAIT_ARRAY_POLL_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::Timeout, u64::MAX, 0);
    }

    #[cfg(all(
        feature = "service-supervisor-runtime",
        not(feature = "service-dependency-runtime")
    ))]
    if crate::process::current_is_init()
        && timeout_ns == bndroid_kernel::service_supervisor_trace::SERVICE_WATCHDOG_TIMEOUT_NS
    {
        let health = bndroid_kernel::service_supervisor_trace::snapshot();
        if health.probe_messages == 2
            && health.watchdog_requests == 0
            && health.watchdog_timeouts == 0
        {
            let gap = crate::process::input_server_restart_gap_evidence();
            let timeout_wakes = crate::scheduler::wait_array_snapshot().timeout_wakes;
            let init_pid = crate::process::current_live_user_process_id().unwrap_or(0);
            let stable_input_peer = count == 1
                && crate::process::current_channel_peer_is_unique_live_image(
                    u64::from(handles[0].raw()),
                    UserImageId::InputServer,
                );
            let accepted = bndroid_kernel::service_supervisor_trace::record_watchdog_request(
                init_pid,
                timeout_ns,
                stable_input_peer,
                gap.created,
                gap.exited,
                gap.reaped,
                gap.live,
                timeout_wakes,
            );
            if count != 1 || !accepted {
                return complete(frame, Status::InvalidState, 0, 0);
            }
        }
    }

    #[cfg(all(
        feature = "input-server-restart-runtime",
        not(feature = "service-dependency-runtime")
    ))]
    if crate::process::current_is_init()
        && timeout_ns == bndroid_kernel::input_server_restart_trace::INPUT_RESTART_BACKOFF_NS
    {
        let gap = crate::process::input_server_restart_gap_evidence();
        let timeout_wakes = crate::scheduler::wait_array_snapshot().timeout_wakes;
        let init_pid = crate::process::current_live_user_process_id().unwrap_or(0);
        #[cfg(feature = "service-supervisor-runtime")]
        let trace_result = {
            let restart = bndroid_kernel::input_server_restart_trace::snapshot();
            if restart.backoff_requests == 0 {
                let stable_surface_peer = count == 1
                    && crate::process::current_channel_peer_is_unique_live_image(
                        u64::from(handles[0].raw()),
                        UserImageId::SurfaceServer,
                    );
                let accepted = bndroid_kernel::input_server_restart_trace::record_backoff_request(
                    init_pid,
                    timeout_ns,
                    stable_surface_peer,
                    gap.created,
                    gap.exited,
                    gap.reaped,
                    gap.live,
                    gap.replacement_spawn_pid,
                    timeout_wakes,
                );
                (true, accepted)
            } else {
                // M48's one-shot M47 restart backoff is distinct from its
                // scheduler-aware health watchdog and must not be replayed.
                (false, true)
            }
        };
        #[cfg(not(feature = "service-supervisor-runtime"))]
        let trace_result = {
            let stable_surface_peer = count == 1
                && crate::process::current_channel_peer_is_unique_live_image(
                    u64::from(handles[0].raw()),
                    UserImageId::SurfaceServer,
                );
            (
                true,
                bndroid_kernel::input_server_restart_trace::record_backoff_request(
                    init_pid,
                    timeout_ns,
                    stable_surface_peer,
                    gap.created,
                    gap.exited,
                    gap.reaped,
                    gap.live,
                    gap.replacement_spawn_pid,
                    timeout_wakes,
                ),
            )
        };
        if trace_result.0 && (count != 1 || !trace_result.1) {
            return complete(frame, Status::InvalidState, 0, 0);
        }
    }

    #[cfg(feature = "service-dependency-runtime")]
    let service_dependency_wait_trace_complete = {
        #[cfg(feature = "post-recovery-interaction-runtime")]
        {
            bndroid_kernel::service_dependency_trace::snapshot().complete
        }
        #[cfg(not(feature = "post-recovery-interaction-runtime"))]
        {
            false
        }
    };
    #[cfg(feature = "service-dependency-runtime")]
    if !service_dependency_wait_trace_complete
        && crate::process::current_is_init()
        && (timeout_ns == bndroid_kernel::service_dependency_trace::SERVICE_DEPENDENCY_TIMEOUT_NS
            || timeout_ns
                == bndroid_kernel::service_dependency_trace::SERVICE_DEPENDENCY_BACKOFF_NS)
        && bndroid_kernel::input_surface_recovery_trace::snapshot().complete
    {
        let expected_peer = count == 1
            && (crate::process::current_channel_peer_is_unique_live_image(
                u64::from(handles[0].raw()),
                UserImageId::SurfaceServer,
            ) || crate::process::current_channel_peer_is_unique_live_image(
                u64::from(handles[0].raw()),
                UserImageId::InputServer,
            ));
        let init_pid = crate::process::current_live_user_process_id().unwrap_or(0);
        if count != 1
            || !bndroid_kernel::service_dependency_trace::record_wait_request(
                init_pid,
                timeout_ns,
                expected_peer,
            )
        {
            return complete(frame, Status::InvalidState, 0, 0);
        }
    }

    crate::scheduler::wait_current_user_object_array(frame, &items[..count], deadline)
}

pub(crate) fn complete_object_wait(
    frame: *mut TrapFrame,
    process_id: u64,
    status: Status,
    observed: ObjectSignals,
) {
    if !crate::arch::aarch64::irq_is_masked() || frame.is_null() || process_id == 0 {
        panic!("deferred object-wait completion invariants failed");
    }
    if crate::process::is_init_process_id(process_id) {
        if status == Status::Ok {
            SUCCESSES.fetch_add(1, Ordering::Relaxed);
        } else {
            ERRORS.fetch_add(1, Ordering::Relaxed);
        }
    }
    // SIGNALED belongs exclusively to Event objects, and this callback is
    // reached only after the scheduler atomically consumes the exact wait
    // token. Consequently each successful Event block contributes one wake,
    // while cancelled waits and Channel completions cannot inflate it.
    if status == Status::Ok && observed.intersects(ObjectSignals::SIGNALED) {
        EVENT_WAIT_WAKES.fetch_add(1, Ordering::Relaxed);
    }
    unsafe {
        (*frame).x[0] = status.raw();
        (*frame).x[1] = u64::from(observed.bits());
        (*frame).x[2] = 0;
    }
}

pub(crate) fn complete_object_wait_many(
    frame: *mut TrapFrame,
    process_id: u64,
    status: Status,
    index: u64,
    observed: ObjectSignals,
) {
    if !crate::arch::aarch64::irq_is_masked()
        || frame.is_null()
        || process_id == 0
        || (status == Status::Ok && index >= OBJECT_WAIT_MANY_ITEM_COUNT as u64)
        || (status == Status::Timeout && (index != u64::MAX || observed.bits() != 0))
        || (status == Status::NotFound && index >= OBJECT_WAIT_MANY_ITEM_COUNT as u64)
        || !matches!(status, Status::Ok | Status::Timeout | Status::NotFound)
    {
        panic!("deferred wait-many completion invariants failed");
    }
    if crate::process::is_init_process_id(process_id) {
        if status == Status::Ok {
            SUCCESSES.fetch_add(1, Ordering::Relaxed);
        } else {
            ERRORS.fetch_add(1, Ordering::Relaxed);
        }
    }
    unsafe {
        (*frame).x[0] = status.raw();
        (*frame).x[1] = index;
        (*frame).x[2] = u64::from(observed.bits());
    }
}

pub(crate) fn complete_object_wait_many_array(
    frame: *mut TrapFrame,
    process_id: u64,
    status: Status,
    index: u64,
    observed: ObjectSignals,
) {
    if !crate::arch::aarch64::irq_is_masked()
        || frame.is_null()
        || process_id == 0
        || (status == Status::Ok && index >= OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS as u64)
        || (status == Status::Timeout && (index != u64::MAX || observed.bits() != 0))
        || (status == Status::NotFound && index >= OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS as u64)
        || !matches!(status, Status::Ok | Status::Timeout | Status::NotFound)
    {
        panic!("deferred wait-array completion invariants failed");
    }
    if crate::process::is_init_process_id(process_id) {
        if status == Status::Ok {
            SUCCESSES.fetch_add(1, Ordering::Relaxed);
        } else {
            ERRORS.fetch_add(1, Ordering::Relaxed);
        }
    }
    #[cfg(all(
        feature = "input-server-restart-runtime",
        not(feature = "service-dependency-runtime")
    ))]
    if status == Status::Timeout && crate::process::is_init_process_id(process_id) {
        let restart = bndroid_kernel::input_server_restart_trace::snapshot();
        if restart.backoff_requests == 1 && restart.backoff_timeouts == 0 {
            let gap = crate::process::input_server_restart_gap_evidence();
            let timeout_wakes = crate::scheduler::wait_array_snapshot().timeout_wakes;
            let _ = bndroid_kernel::input_server_restart_trace::record_backoff_timeout(
                process_id,
                gap.created,
                gap.exited,
                gap.reaped,
                gap.live,
                gap.replacement_spawn_pid,
                timeout_wakes,
            );
        }
        #[cfg(feature = "service-supervisor-runtime")]
        {
            let health = bndroid_kernel::service_supervisor_trace::snapshot();
            if health.watchdog_requests == 1 && health.watchdog_timeouts == 0 {
                let gap = crate::process::input_server_restart_gap_evidence();
                let timeout_wakes = crate::scheduler::wait_array_snapshot().timeout_wakes;
                let _ = bndroid_kernel::service_supervisor_trace::record_watchdog_timeout(
                    process_id,
                    gap.created,
                    gap.exited,
                    gap.reaped,
                    gap.live,
                    timeout_wakes,
                );
            }
        }
    }
    #[cfg(feature = "service-dependency-runtime")]
    if status == Status::Timeout && crate::process::is_init_process_id(process_id) {
        let _ = bndroid_kernel::service_dependency_trace::record_wait_timeout(process_id);
    }
    unsafe {
        (*frame).x[0] = status.raw();
        (*frame).x[1] = index;
        (*frame).x[2] = u64::from(observed.bits());
    }
}

fn event_create(
    frame: *mut TrapFrame,
    flags: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    if flags & !EVENT_CREATE_SIGNALED != 0 || reserved1 != 0 || reserved2 != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    if with_table(|table| table.available()) == 0 {
        return complete(frame, Status::OutOfMemory, 0, 0);
    }
    let Ok(event) = Event::try_new() else {
        return complete(frame, Status::OutOfMemory, 0, 0);
    };
    if flags & EVENT_CREATE_SIGNALED != 0 && !event.signal() {
        panic!("new event was already signaled");
    }
    match with_table(|table| table.insert(KernelObject::Event(event), Rights::EVENT_DEFAULT)) {
        Ok(handle) => {
            EVENTS_CREATED.fetch_add(1, Ordering::Relaxed);
            complete(frame, Status::Ok, u64::from(handle.raw()), 0)
        }
        Err(error) => complete(frame, handle_error_status(error), 0, 0),
    }
}

fn event_signal(
    frame: *mut TrapFrame,
    raw_handle: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    event_set_state(frame, raw_handle, reserved1, reserved2, true)
}

fn event_clear(
    frame: *mut TrapFrame,
    raw_handle: u64,
    reserved1: u64,
    reserved2: u64,
) -> *mut TrapFrame {
    event_set_state(frame, raw_handle, reserved1, reserved2, false)
}

fn event_set_state(
    frame: *mut TrapFrame,
    raw_handle: u64,
    reserved1: u64,
    reserved2: u64,
    signaled: bool,
) -> *mut TrapFrame {
    if reserved1 != 0 || reserved2 != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(handle) = parse_handle(raw_handle) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let result = with_table(|table| {
        let object = table.get(handle, Rights::SIGNAL).map_err(|error| {
            if error == HandleError::AccessDenied {
                RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                if signaled {
                    EVENT_SIGNAL_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
            }
            handle_error_status(error)
        })?;
        let event = object.as_event().ok_or(Status::InvalidState)?;
        Ok::<_, Status>(if signaled {
            event.signal()
        } else {
            event.clear()
        })
    });
    match result {
        Ok(changed) => {
            if signaled {
                EVENT_SIGNAL_CALLS.fetch_add(1, Ordering::Relaxed);
                if changed {
                    EVENT_SIGNAL_EDGES.fetch_add(1, Ordering::Relaxed);
                    crate::scheduler::wake_object_waiters();
                }
            } else {
                EVENT_CLEAR_CALLS.fetch_add(1, Ordering::Relaxed);
                if changed {
                    EVENT_CLEAR_EDGES.fetch_add(1, Ordering::Relaxed);
                }
            }
            complete(frame, Status::Ok, u64::from(changed), 0)
        }
        Err(status) => complete(frame, status, 0, 0),
    }
}

fn channel_create(frame: *mut TrapFrame) -> *mut TrapFrame {
    // Capacity is stable for the whole syscall because the single-core vector
    // keeps IRQ masked. Avoid allocating an object that cannot be committed.
    if with_table(|table| table.available()) < 2 {
        return complete(frame, Status::OutOfMemory, 0, 0);
    }
    let Ok((left, right)) = ChannelEndpoint::try_pair() else {
        return complete(frame, Status::OutOfMemory, 0, 0);
    };
    match with_table(|table| {
        table.insert_pair(
            KernelObject::Channel(left),
            KernelObject::Channel(right),
            Rights::CHANNEL_DEFAULT,
        )
    }) {
        Ok((left, right)) => {
            CHANNELS_CREATED.fetch_add(1, Ordering::Relaxed);
            complete(
                frame,
                Status::Ok,
                u64::from(left.raw()),
                u64::from(right.raw()),
            )
        }
        Err(error) => complete(frame, handle_error_status(error), 0, 0),
    }
}

fn channel_write(frame: *mut TrapFrame, raw: u64, tag: u64, payload: u64) -> *mut TrapFrame {
    let Some(handle) = parse_handle(raw) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let sender_pid = authenticated_sender_pid();
    let result = with_table(|table| {
        let object = table.get(handle, Rights::WRITE).map_err(|error| {
            if error == HandleError::AccessDenied {
                RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
            }
            handle_error_status(error)
        })?;
        let endpoint = object.as_channel().ok_or(Status::InvalidState)?;
        endpoint
            .write(
                Message::scalar(tag, payload)
                    .with_sender(sender_pid)
                    .unwrap_or_else(|| panic!("fresh scalar message rejected sender stamp")),
            )
            .map_err(channel_error_status)
    });
    match result {
        Ok(()) => {
            WRITES.fetch_add(1, Ordering::Relaxed);
            crate::scheduler::wake_object_waiters();
            complete(frame, Status::Ok, 0, 0)
        }
        Err(status) => complete(frame, status, 0, 0),
    }
}

fn channel_read(frame: *mut TrapFrame, raw: u64) -> *mut TrapFrame {
    let Some(handle) = parse_handle(raw) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let result = with_table(|table| {
        let object = table.get(handle, Rights::READ).map_err(|error| {
            if error == HandleError::AccessDenied {
                RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
            }
            handle_error_status(error)
        })?;
        let endpoint = object.as_channel().ok_or(Status::InvalidState)?;
        endpoint
            .read_with(|message| {
                let sender_pid = message.sender_pid().ok_or(Status::InvalidState)?;
                let (tag, payload) = message.scalar_payload().ok_or(Status::InvalidState)?;
                Ok((sender_pid, tag, payload))
            })
            .map_err(|error| match error {
                ChannelReadFailure::Channel(error) => channel_error_status(error),
                ChannelReadFailure::Rejected(status) => status,
            })
    });
    match result {
        Ok((sender_pid, tag, payload)) => {
            READS.fetch_add(1, Ordering::Relaxed);
            LAST_TAG.store(tag, Ordering::Relaxed);
            LAST_PAYLOAD.store(payload, Ordering::Relaxed);
            record_m14_service_transcript(handle, sender_pid, tag, payload);
            record_m15_service_transcript(handle, sender_pid, tag, payload);
            record_service_cycle_transcript(handle, sender_pid, tag, payload);
            record_m17_identity_transcript(handle, sender_pid, tag, payload);
            crate::scheduler::wake_object_waiters();
            complete(frame, Status::Ok, tag, payload)
        }
        Err(status) => complete(frame, status, 0, 0),
    }
}

fn channel_peek(frame: *mut TrapFrame, raw: u64, reserved0: u64, reserved1: u64) -> *mut TrapFrame {
    if reserved0 != 0 || reserved1 != 0 {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let Some(handle) = parse_handle(raw) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let result = with_table(|table| {
        let object = table.get(handle, Rights::READ).map_err(|error| {
            if error == HandleError::AccessDenied {
                RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
            }
            handle_error_status(error)
        })?;
        let endpoint = object.as_channel().ok_or(Status::InvalidState)?;
        endpoint.peek().map_err(channel_error_status)
    });
    match result {
        Ok((kind, length)) => {
            CHANNEL_PEEKS.fetch_add(1, Ordering::Relaxed);
            match kind {
                bndr_abi::ChannelMessageKind::Scalar => {
                    CHANNEL_PEEK_SCALAR.fetch_add(1, Ordering::Relaxed);
                }
                bndr_abi::ChannelMessageKind::Bytes => {
                    CHANNEL_PEEK_BYTES.fetch_add(1, Ordering::Relaxed);
                }
                bndr_abi::ChannelMessageKind::Transfer => {
                    CHANNEL_PEEK_TRANSFER.fetch_add(1, Ordering::Relaxed);
                }
            }
            complete(frame, Status::Ok, kind.raw(), length as u64)
        }
        Err(status) => complete(frame, status, 0, 0),
    }
}

#[derive(Clone, Copy)]
struct EnvelopeReadResult {
    sender_pid: u64,
    kind: ChannelMessageKind,
    scalar: Option<(u64, u64)>,
    logical_length: usize,
    is_event: bool,
    #[cfg(feature = "surface-trace-evidence")]
    trace_bytes: Option<[u8; CHANNEL_MESSAGE_MAX_BYTES]>,
}

enum EnvelopeReadFailure {
    Status(Status),
    TableFull,
}

#[cfg(all(
    feature = "service-supervisor-runtime",
    not(feature = "service-dependency-runtime")
))]
fn current_service_supervisor_transport(
    raw_handle: u64,
    owner_pid: u64,
) -> bndroid_kernel::service_supervisor_trace::ChannelTransportEvidence {
    let trace = bndroid_kernel::service_supervisor_trace::snapshot();
    bndroid_kernel::service_supervisor_trace::ChannelTransportEvidence {
        owner_pid,
        peer_pid: crate::process::current_channel_unique_live_peer_pid(raw_handle),
        raw_handle,
        surface_binding_match: crate::process::current_channel_matches_live_binding(
            raw_handle,
            trace.surface_transport_owner_pid,
            trace.surface_transport_handle,
        ),
        input_binding_match: crate::process::current_channel_matches_live_binding(
            raw_handle,
            trace.input_transport_owner_pid,
            trace.input_transport_handle,
        ),
    }
}

/// Atomically consumes the FIFO head, returns its immutable kernel sender
/// identity and canonical payload, and installs a transferred handle when
/// applicable. Every fallible step precedes commit; rejected reads restore the
/// exact message, object, rights, sender stamp, and handle-table generation.
fn channel_read_envelope(
    frame: *mut TrapFrame,
    raw_transport: u64,
    user_destination: u64,
    capacity: u64,
) -> *mut TrapFrame {
    let Some(transport_handle) = parse_handle(raw_transport) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    #[cfg(all(
        feature = "service-supervisor-runtime",
        not(feature = "service-dependency-runtime")
    ))]
    let supervisor_reader_pid = crate::process::current_live_user_process_id()
        .unwrap_or_else(|| panic!("health envelope read lacked an authenticated PID"));
    #[cfg(all(
        feature = "service-supervisor-runtime",
        not(feature = "service-dependency-runtime")
    ))]
    let supervisor_transport =
        current_service_supervisor_transport(raw_transport, supervisor_reader_pid);
    if capacity < CHANNEL_READ_ENVELOPE_SIZE as u64 {
        CHANNEL_ENVELOPE_PRESERVED_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        CHANNEL_ENVELOPE_BUFFER_TOO_SMALL.fetch_add(1, Ordering::Relaxed);
        return complete(
            frame,
            Status::BufferTooSmall,
            CHANNEL_READ_ENVELOPE_SIZE as u64,
            0,
        );
    }
    if !crate::process::current_user_range_is_writable(user_destination, CHANNEL_READ_ENVELOPE_SIZE)
    {
        CHANNEL_ENVELOPE_PRESERVED_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        CHANNEL_ENVELOPE_BAD_ADDRESS.fetch_add(1, Ordering::Relaxed);
        return complete(frame, Status::BadAddress, 0, 0);
    }
    let result = with_table(|table| {
        let transport = table
            .get(transport_handle, Rights::READ)
            .map_err(|error| {
                if error == HandleError::AccessDenied {
                    RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                }
                EnvelopeReadFailure::Status(handle_error_status(error))
            })?
            .as_channel()
            .ok_or(EnvelopeReadFailure::Status(Status::InvalidState))?
            .clone();
        transport
            .read_owned_with(|message| {
                let Some(sender_pid) = message.sender_pid() else {
                    return Err((EnvelopeReadFailure::Status(Status::InvalidState), message));
                };
                let Some((kind, _)) = message.metadata() else {
                    return Err((EnvelopeReadFailure::Status(Status::InvalidState), message));
                };
                match kind {
                    ChannelMessageKind::Scalar => {
                        let Some((tag, payload)) = message.scalar_payload() else {
                            return Err((
                                EnvelopeReadFailure::Status(Status::InvalidState),
                                message,
                            ));
                        };
                        let envelope = ChannelReadEnvelope::scalar(sender_pid, tag, payload)
                            .unwrap_or_else(|_| {
                                panic!("valid scalar message rejected by ABI codec")
                            });
                        crate::arch::aarch64::usercopy::copy_to_user(
                            user_destination,
                            &envelope.encode(),
                        )
                        .unwrap_or_else(|_| {
                            panic!("prevalidated scalar envelope destination faulted")
                        });
                        Ok(EnvelopeReadResult {
                            sender_pid,
                            kind,
                            scalar: Some((tag, payload)),
                            logical_length: 16,
                            is_event: false,
                            #[cfg(feature = "surface-trace-evidence")]
                            trace_bytes: None,
                        })
                    }
                    ChannelMessageKind::Bytes => {
                        let Some(bytes) = message.bytes_payload() else {
                            return Err((
                                EnvelopeReadFailure::Status(Status::InvalidState),
                                message,
                            ));
                        };
                        let logical_length = bytes.len();
                        let mut data = [0; CHANNEL_MESSAGE_MAX_BYTES];
                        data[..logical_length].copy_from_slice(bytes);
                        let envelope = ChannelReadEnvelope::bytes(sender_pid, data, logical_length)
                            .unwrap_or_else(|_| panic!("valid byte message rejected by ABI codec"));
                        crate::arch::aarch64::usercopy::copy_to_user(
                            user_destination,
                            &envelope.encode(),
                        )
                        .unwrap_or_else(|_| {
                            panic!("prevalidated byte envelope destination faulted")
                        });
                        Ok(EnvelopeReadResult {
                            sender_pid,
                            kind,
                            scalar: None,
                            logical_length,
                            is_event: false,
                            #[cfg(feature = "surface-trace-evidence")]
                            trace_bytes: Some(data),
                        })
                    }
                    ChannelMessageKind::Transfer => {
                        let (data, logical_length, object, rights) = match message.into_transfer() {
                            Ok(parts) => parts,
                            Err(message) => {
                                return Err((
                                    EnvelopeReadFailure::Status(Status::InvalidState),
                                    message,
                                ));
                            }
                        };
                        let reservation = match table.reserve_slot() {
                            Ok(reservation) => reservation,
                            Err(HandleError::TableFull) => {
                                return Err((
                                    EnvelopeReadFailure::TableFull,
                                    rebuild_transfer(
                                        sender_pid,
                                        data,
                                        logical_length,
                                        object,
                                        rights,
                                    ),
                                ));
                            }
                            Err(error) => panic!("empty-slot reservation failed: {error:?}"),
                        };
                        let prospective = reservation.prospective_handle();
                        let envelope = ChannelReadEnvelope::transfer(
                            sender_pid,
                            prospective,
                            data,
                            logical_length,
                        )
                        .unwrap_or_else(|_| panic!("valid transfer rejected by ABI codec"));
                        crate::arch::aarch64::usercopy::copy_to_user(
                            user_destination,
                            &envelope.encode(),
                        )
                        .unwrap_or_else(|_| {
                            panic!("prevalidated transfer envelope destination faulted")
                        });
                        let is_event = object.as_event().is_some();
                        let received = reservation.insert_parts(object, rights);
                        assert_eq!(received, prospective);
                        Ok(EnvelopeReadResult {
                            sender_pid,
                            kind,
                            scalar: None,
                            logical_length,
                            is_event,
                            #[cfg(feature = "surface-trace-evidence")]
                            trace_bytes: Some(data),
                        })
                    }
                }
            })
            .map_err(|error| match error {
                ChannelReadFailure::Channel(error) => {
                    EnvelopeReadFailure::Status(channel_error_status(error))
                }
                ChannelReadFailure::Rejected(error) => error,
            })
    });

    match result {
        Ok(result) => {
            #[cfg(all(
                feature = "service-supervisor-runtime",
                not(feature = "service-dependency-runtime")
            ))]
            if matches!(
                result.kind,
                ChannelMessageKind::Bytes | ChannelMessageKind::Transfer
            ) {
                let reader_image =
                    crate::process::current_live_user_image_id().unwrap_or_else(|| {
                        panic!("health envelope read lacked an authenticated image")
                    });
                let read_kind = match result.kind {
                    ChannelMessageKind::Bytes => {
                        bndroid_kernel::service_supervisor_trace::ChannelWriteKind::Bytes
                    }
                    ChannelMessageKind::Transfer => {
                        bndroid_kernel::service_supervisor_trace::ChannelWriteKind::Transfer
                    }
                    ChannelMessageKind::Scalar => unreachable!(),
                };
                let wire = result
                    .trace_bytes
                    .as_ref()
                    .unwrap_or_else(|| panic!("health envelope read lost its byte payload"));
                let _ = bndroid_kernel::service_supervisor_trace::record_channel_read(
                    supervisor_reader_pid,
                    reader_image,
                    supervisor_transport,
                    result.sender_pid,
                    &wire[..result.logical_length],
                    read_kind,
                );
            }
            #[cfg(feature = "service-dependency-runtime")]
            if matches!(
                result.kind,
                ChannelMessageKind::Bytes | ChannelMessageKind::Transfer
            ) {
                let reader_pid = crate::process::current_live_user_process_id()
                    .unwrap_or_else(|| panic!("M49 envelope read lacked an authenticated PID"));
                let reader_image = crate::process::current_live_user_image_id()
                    .unwrap_or_else(|| panic!("M49 envelope read lacked an authenticated image"));
                let read_kind = match result.kind {
                    ChannelMessageKind::Bytes => {
                        bndroid_kernel::service_dependency_trace::ChannelWriteKind::Bytes
                    }
                    ChannelMessageKind::Transfer => {
                        bndroid_kernel::service_dependency_trace::ChannelWriteKind::Transfer
                    }
                    ChannelMessageKind::Scalar => unreachable!(),
                };
                let wire = result
                    .trace_bytes
                    .as_ref()
                    .unwrap_or_else(|| panic!("M49 envelope read lost its byte payload"));
                #[cfg(feature = "post-recovery-interaction-runtime")]
                let _ = if bndroid_kernel::service_dependency_trace::snapshot().complete {
                    #[cfg(feature = "post-recovery-focus-runtime")]
                    {
                        if bndroid_kernel::post_recovery_interaction_trace::snapshot().complete {
                            #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
                            {
                                if bndroid_kernel::post_recovery_focus_trace::snapshot().complete {
                                    bndroid_kernel::post_recovery_focus_roundtrip_trace::record_channel_read(
                                        reader_pid,
                                        reader_image,
                                        result.sender_pid,
                                        &wire[..result.logical_length],
                                        read_kind,
                                    )
                                } else {
                                    bndroid_kernel::post_recovery_focus_trace::record_channel_read(
                                        reader_pid,
                                        reader_image,
                                        result.sender_pid,
                                        &wire[..result.logical_length],
                                        read_kind,
                                    )
                                }
                            }
                            #[cfg(not(feature = "post-recovery-focus-roundtrip-runtime"))]
                            {
                                bndroid_kernel::post_recovery_focus_trace::record_channel_read(
                                    reader_pid,
                                    reader_image,
                                    result.sender_pid,
                                    &wire[..result.logical_length],
                                    read_kind,
                                )
                            }
                        } else {
                            bndroid_kernel::post_recovery_interaction_trace::record_channel_read(
                                reader_pid,
                                reader_image,
                                result.sender_pid,
                                &wire[..result.logical_length],
                                read_kind,
                            )
                        }
                    }
                    #[cfg(not(feature = "post-recovery-focus-runtime"))]
                    {
                        bndroid_kernel::post_recovery_interaction_trace::record_channel_read(
                            reader_pid,
                            reader_image,
                            result.sender_pid,
                            &wire[..result.logical_length],
                            read_kind,
                        )
                    }
                } else {
                    bndroid_kernel::service_dependency_trace::record_channel_read(
                        reader_pid,
                        reader_image,
                        result.sender_pid,
                        &wire[..result.logical_length],
                        read_kind,
                    )
                };
                #[cfg(not(feature = "post-recovery-interaction-runtime"))]
                let _ = bndroid_kernel::service_dependency_trace::record_channel_read(
                    reader_pid,
                    reader_image,
                    result.sender_pid,
                    &wire[..result.logical_length],
                    read_kind,
                );
                #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
                let _ = bndroid_kernel::post_recovery_lifecycle_focus_trace::record_channel_read(
                    reader_pid,
                    reader_image,
                    result.sender_pid,
                    &wire[..result.logical_length],
                    read_kind,
                );
            }
            CHANNEL_ENVELOPE_READS.fetch_add(1, Ordering::Relaxed);
            match result.kind {
                ChannelMessageKind::Scalar => {
                    CHANNEL_ENVELOPE_SCALAR.fetch_add(1, Ordering::Relaxed);
                    READS.fetch_add(1, Ordering::Relaxed);
                    let (tag, payload) = result
                        .scalar
                        .unwrap_or_else(|| panic!("scalar envelope lost its register payload"));
                    LAST_TAG.store(tag, Ordering::Relaxed);
                    LAST_PAYLOAD.store(payload, Ordering::Relaxed);
                    record_m14_service_transcript(
                        transport_handle,
                        result.sender_pid,
                        tag,
                        payload,
                    );
                    record_m15_service_transcript(
                        transport_handle,
                        result.sender_pid,
                        tag,
                        payload,
                    );
                    record_service_cycle_transcript(
                        transport_handle,
                        result.sender_pid,
                        tag,
                        payload,
                    );
                    record_m17_identity_transcript(
                        transport_handle,
                        result.sender_pid,
                        tag,
                        payload,
                    );
                    record_m18_multi_client_transcript(
                        transport_handle,
                        result.sender_pid,
                        tag,
                        payload,
                    );
                    record_m20_multi_session_transcript(
                        transport_handle,
                        result.sender_pid,
                        tag,
                        payload,
                    );
                }
                ChannelMessageKind::Bytes => {
                    CHANNEL_ENVELOPE_BYTES.fetch_add(1, Ordering::Relaxed);
                    BYTE_READS.fetch_add(1, Ordering::Relaxed);
                    if result.logical_length == 0 {
                        BYTE_ZERO_LENGTH.fetch_add(1, Ordering::Relaxed);
                    }
                }
                ChannelMessageKind::Transfer => {
                    CHANNEL_ENVELOPE_TRANSFER.fetch_add(1, Ordering::Relaxed);
                    TRANSFER_READS.fetch_add(1, Ordering::Relaxed);
                    if result.is_event {
                        EVENT_TRANSFER_READS.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            #[cfg(feature = "androidbox-process0")]
            trace_android_app_rpc_read(&result);
            #[cfg(feature = "surface-trace-evidence")]
            trace_ui_channel_read(&result);
            crate::scheduler::wake_object_waiters();
            complete(frame, Status::Ok, result.sender_pid, result.kind.raw())
        }
        Err(EnvelopeReadFailure::TableFull) => {
            CHANNEL_ENVELOPE_PRESERVED_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            CHANNEL_ENVELOPE_TABLE_FULL.fetch_add(1, Ordering::Relaxed);
            complete(frame, Status::OutOfMemory, 0, 0)
        }
        Err(EnvelopeReadFailure::Status(status)) => {
            if status == Status::InvalidState {
                CHANNEL_ENVELOPE_PRESERVED_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            }
            complete(frame, status, 0, 0)
        }
    }
}

#[cfg(feature = "androidbox-process0")]
#[derive(Clone, Copy)]
enum AndroidAppRpcValidationError {
    Decode,
    Authentication,
    Direction,
    Protocol,
}

#[cfg(feature = "androidbox-process0")]
fn trace_android_app_rpc_read(result: &EnvelopeReadResult) {
    let Some(wire) = result.trace_bytes.as_ref() else {
        return;
    };
    #[cfg(feature = "androidbox-restart0")]
    if result.logical_length >= b"BNDARS01".len() && &wire[..b"BNDARS01".len()] == b"BNDARS01" {
        trace_android_app_supervisor_read(result, wire);
        return;
    }
    #[cfg(not(feature = "androidbox-scene-rpc2"))]
    let rpc_magic = b"BNDAPC01";
    #[cfg(all(
        feature = "androidbox-scene-rpc2",
        not(feature = "androidbox-multiaction3")
    ))]
    let rpc_magic = b"BNDAPC02";
    #[cfg(feature = "androidbox-layout-size18")]
    let rpc_magic = b"BNDAPC14";
    #[cfg(all(
        feature = "androidbox-layout-directional17",
        not(feature = "androidbox-layout-size18")
    ))]
    let rpc_magic = b"BNDAPC13";
    #[cfg(all(
        feature = "androidbox-layout-spacing16",
        not(feature = "androidbox-layout-directional17")
    ))]
    let rpc_magic = b"BNDAPC12";
    #[cfg(all(
        feature = "androidbox-layout-weight15",
        not(feature = "androidbox-layout-spacing16")
    ))]
    let rpc_magic = b"BNDAPC11";
    #[cfg(all(
        feature = "androidbox-layout-row14",
        not(feature = "androidbox-layout-weight15")
    ))]
    let rpc_magic = b"BNDAPC10";
    #[cfg(all(
        feature = "androidbox-string-builder13",
        not(feature = "androidbox-layout-row14")
    ))]
    let rpc_magic = b"BNDAPC09";
    #[cfg(all(
        feature = "androidbox-string-text12",
        not(feature = "androidbox-string-builder13")
    ))]
    let rpc_magic = b"BNDAPC08";
    #[cfg(all(
        feature = "androidbox-activity-state11",
        not(feature = "androidbox-string-text12")
    ))]
    let rpc_magic = b"BNDAPC07";
    #[cfg(all(
        feature = "androidbox-activity-fields10",
        not(feature = "androidbox-activity-state11")
    ))]
    let rpc_magic = b"BNDAPC06";
    #[cfg(all(
        feature = "androidbox-dex-instance9",
        not(feature = "androidbox-activity-fields10")
    ))]
    let rpc_magic = b"BNDAPC05";
    #[cfg(all(
        feature = "androidbox-dex-methods8",
        not(feature = "androidbox-dex-instance9")
    ))]
    let rpc_magic = b"BNDAPC04";
    #[cfg(all(
        feature = "androidbox-multiaction3",
        not(feature = "androidbox-dex-methods8")
    ))]
    let rpc_magic = b"BNDAPC03";
    if result.logical_length < rpc_magic.len() || &wire[..rpc_magic.len()] != rpc_magic {
        return;
    }
    if result.logical_length != ANDROID_APP_MESSAGE_WIRE_SIZE {
        android_app_rpc_record_validation_error(AndroidAppRpcValidationError::Decode);
        return;
    }
    let message = match AndroidAppMessage::decode(wire) {
        Ok(message) => message,
        Err(_) => {
            android_app_rpc_record_validation_error(AndroidAppRpcValidationError::Decode);
            return;
        }
    };
    if result.kind != ChannelMessageKind::Bytes {
        android_app_rpc_record_validation_error(AndroidAppRpcValidationError::Protocol);
        return;
    }

    let Some(receiver_pid) = crate::process::current_live_user_process_id() else {
        android_app_rpc_record_validation_error(AndroidAppRpcValidationError::Authentication);
        return;
    };
    let Some(app_pid) = crate::process::unique_live_process_id_for_image(UserImageId::App) else {
        android_app_rpc_record_validation_error(AndroidAppRpcValidationError::Authentication);
        return;
    };
    let Some(android_app_pid) =
        crate::process::unique_live_process_id_for_image(UserImageId::AndroidApp)
    else {
        android_app_rpc_record_validation_error(AndroidAppRpcValidationError::Authentication);
        return;
    };
    let direction = if result.sender_pid == app_pid && receiver_pid == android_app_pid {
        AndroidAppRpcDirection::AppToAndroidApp
    } else if result.sender_pid == android_app_pid && receiver_pid == app_pid {
        AndroidAppRpcDirection::AndroidAppToApp
    } else {
        android_app_rpc_record_validation_error(AndroidAppRpcValidationError::Authentication);
        return;
    };
    if !android_app_rpc_kind_has_direction(message.kind(), direction) {
        android_app_rpc_record_validation_error(AndroidAppRpcValidationError::Direction);
        return;
    }

    let trace = android_app_rpc_trace_mut();
    #[cfg(feature = "androidbox-restart0")]
    if message.kind() == AndroidAppMessageKind::Crash {
        let restart = android_app_restart_trace_mut();
        let valid = restart.phase == AndroidAppRestartPhase::AwaitCrash
            && direction == AndroidAppRpcDirection::AppToAndroidApp
            && trace.phase == AndroidAppRpcPhase::Active
            && trace.first_round_errors == 0
            && trace.first_round_messages == ANDROID_APP_OPEN_TRANSCRIPT_MESSAGES
            && trace.first_round_matches
            && trace.last_request_id == ANDROID_APP_OPEN_LAST_REQUEST_ID
            && message.request_id() == ANDROID_APP_CRASH_REQUEST_ID
            && message.arg0() == trace.session_id
            && message.arg1() == trace.package_generation
            && message.payload().is_empty();
        if !valid {
            restart.errors = restart.errors.saturating_add(1);
            android_app_rpc_apply_validation_error(trace, AndroidAppRpcValidationError::Protocol);
            return;
        }
        restart.crash_reads = 1;
        restart.app_pid = app_pid;
        restart.old_worker_pid = android_app_pid;
        restart.session_id = message.arg0();
        restart.package_generation = message.arg1();
        restart.crash_request_id = message.request_id();
        restart.epoch = 1;
        restart.phase = AndroidAppRestartPhase::AwaitRebind;
        crate::kprintln!(
            "ANDROID_APP_RESTART_ARMED format=1 abi={} app_pid={} old_pid={} session_id={} package_generation={} request_id={} fault_point=after-open-before-ui-commit restart_budget=1",
            ABI_VERSION,
            app_pid,
            android_app_pid,
            message.arg0(),
            message.arg1(),
            message.request_id(),
        );
        *trace = AndroidAppRpcTraceState::new();
        return;
    }
    if (trace.app_pid != 0 && trace.app_pid != app_pid)
        || (trace.android_app_pid != 0 && trace.android_app_pid != android_app_pid)
    {
        android_app_rpc_apply_validation_error(trace, AndroidAppRpcValidationError::Authentication);
        return;
    }
    let first_round = !trace.first_round_finished;
    let mut next = *trace;
    if next.app_pid == 0 {
        next.app_pid = app_pid;
        next.android_app_pid = android_app_pid;
    }
    #[cfg(feature = "androidbox-restart0")]
    if message.kind() == AndroidAppMessageKind::Ready
        && android_app_restart_trace_mut().phase != AndroidAppRestartPhase::AwaitCrash
    {
        let restart = android_app_restart_trace_mut();
        if restart.phase != AndroidAppRestartPhase::AwaitReplacementReady
            || restart.app_pid != app_pid
            || restart.new_worker_pid != android_app_pid
            || restart.old_worker_pid == android_app_pid
            || restart.fault_reaps != 1
            || restart.grant_reissues != 0
            || restart.reopen_completions != 0
            || trace.phase != AndroidAppRpcPhase::AwaitReady
            || trace.errors != 0
            || trace.first_round_errors != 0
            || trace.first_round_messages != 0
            || !trace.first_round_matches
            || trace.first_round_finished
            || trace.last_request_id != 0
            || trace.pending_request_id != 0
        {
            restart.errors = restart.errors.saturating_add(1);
            android_app_rpc_apply_validation_error(
                trace,
                AndroidAppRpcValidationError::Authentication,
            );
            return;
        }
        restart.replacement_ready_reads = 1;
        restart.phase = AndroidAppRestartPhase::AwaitRebound;
        crate::kprintln!(
            "ANDROID_APP_REPLACEMENT_READY_OK format=1 abi={} epoch={} app_pid={} old_pid={} new_pid={} request_reset=1 ready_request_id=0",
            ABI_VERSION,
            restart.epoch,
            app_pid,
            restart.old_worker_pid,
            android_app_pid,
        );
    }
    let Some(completed_round) = android_app_rpc_accept(&mut next, message) else {
        android_app_rpc_apply_validation_error(trace, AndroidAppRpcValidationError::Protocol);
        return;
    };
    #[cfg(feature = "androidbox-restart0")]
    if message.kind() == AndroidAppMessageKind::Open
        && android_app_restart_trace_mut().phase != AndroidAppRestartPhase::AwaitCrash
        && android_app_restart_trace_mut().reopen_completions == 0
    {
        let restart = android_app_restart_trace_mut();
        let valid = matches!(
            restart.phase,
            AndroidAppRestartPhase::AwaitRebound | AndroidAppRestartPhase::Complete
        ) && restart.errors == 0
            && restart.crash_reads == 1
            && restart.fault_reaps == 1
            && restart.rebind_reads == 1
            && restart.replacement_ready_reads == 1
            && restart.grant_reissues == 0
            && restart.reopen_completions == 0
            && restart.app_pid == app_pid
            && restart.new_worker_pid == android_app_pid
            && first_round
            && trace.phase == AndroidAppRpcPhase::Idle
            && trace.errors == 0
            && trace.first_round_errors == 0
            && trace.first_round_matches
            && trace.first_round_messages == 1
            && trace.ready_reads == 1
            && trace.last_request_id == 0
            && message.request_id() == 1
            && message.arg0() == restart.session_id
            && message.arg1() == restart.package_generation
            && message.payload().is_empty()
            && next.phase == AndroidAppRpcPhase::AwaitOpened
            && next.last_request_id == 1
            && next.pending_request_id == 1
            && next.session_id == restart.session_id
            && next.package_generation == restart.package_generation;
        if !valid
            || crate::package_manager::reissue_image_grant_for_restart(
                restart.old_worker_pid,
                android_app_pid,
                restart.session_id,
                restart.package_generation,
            )
            .is_err()
        {
            restart.errors = restart.errors.saturating_add(1);
            android_app_rpc_apply_validation_error(
                trace,
                AndroidAppRpcValidationError::Authentication,
            );
            return;
        }
        restart.grant_reissues = 1;
        crate::kprintln!(
            "ANDROID_PACKAGE_IMAGE_GRANT_REBIND_OK format=1 abi={} epoch={} old_owner={} new_owner={} compatible_session_id={} package_generation={} request_id=1 trigger=authenticated-replacement-open immutable_vmo=1 same_vmo=1 same_slot=1 generation_step=1 reissue=1/1 apk_bytes_exposed_to_init=0 apk_bytes_exposed_to_app=0",
            ABI_VERSION,
            restart.epoch,
            restart.old_worker_pid,
            android_app_pid,
            restart.session_id,
            restart.package_generation,
        );
    }
    android_app_rpc_record_accepted(&mut next, direction, message, first_round, completed_round);
    #[cfg(feature = "androidbox-restart0")]
    if android_app_rpc_reopen_trigger(message, &next)
        && android_app_restart_trace_mut().phase != AndroidAppRestartPhase::AwaitCrash
        && android_app_restart_trace_mut().reopen_completions == 0
    {
        let restart = android_app_restart_trace_mut();
        let valid = matches!(
            restart.phase,
            AndroidAppRestartPhase::AwaitRebound | AndroidAppRestartPhase::Complete
        ) && restart.errors == 0
            && restart.grant_reissues == 1
            && restart.reopen_completions == 0
            && restart.app_pid == app_pid
            && restart.new_worker_pid == android_app_pid
            && first_round
            && next.errors == 0
            && next.first_round_errors == 0
            && next.first_round_matches
            && android_app_rpc_reopen_shape_is_complete(&next)
            && next.first_session_id == restart.session_id
            && next.first_package_generation == restart.package_generation
            && next.phase == AndroidAppRpcPhase::Active;
        if !valid {
            restart.errors = restart.errors.saturating_add(1);
            android_app_rpc_apply_validation_error(trace, AndroidAppRpcValidationError::Protocol);
            return;
        }
        restart.reopen_completions = 1;
        #[cfg(not(feature = "androidbox-scene-rpc2"))]
        crate::kprintln!(
            "ANDROID_APP_REOPEN_OK format=1 abi={} epoch={} app_pid={} new_pid={} session_id={} package_generation={} request_id=1 ready=1 open=1 opened=1 label_chunks=2 label_bytes={} button_chunks=1 button_bytes={} phase=active completed_open_replayed=1 ambiguous_request_replay=0",
            ABI_VERSION,
            restart.epoch,
            app_pid,
            android_app_pid,
            restart.session_id,
            restart.package_generation,
            next.first_label_bytes,
            next.first_button_bytes,
        );
        #[cfg(all(
            feature = "androidbox-scene-rpc2",
            not(feature = "androidbox-multiaction3")
        ))]
        crate::kprintln!(
            "ANDROID_APP_SCENE_REOPEN_OK format=1 abi={} epoch={} app_pid={} new_pid={} session_id={} package_generation={} open_request_id=1 node_count={} describe_requests={} node_responses={} node_text_chunks={} scene_text_bytes={} last_request_id={} phase=active completed_open_replayed=1 ambiguous_request_replay=0",
            ABI_VERSION,
            restart.epoch,
            app_pid,
            android_app_pid,
            restart.session_id,
            restart.package_generation,
            next.first_scene_node_count,
            next.describe_node_reads,
            next.node_reads,
            next.node_text_chunk_reads,
            next.first_scene_text_bytes,
            next.first_last_request_id,
        );
        #[cfg(feature = "androidbox-multiaction3")]
        crate::kprintln!(
            "ANDROID_APP_MULTIACTION_REOPEN_OK format=1 abi={} epoch={} app_pid={} new_pid={} session_id={} package_generation={} open_request_id=1 node_count={} describe_requests={} node_responses={} node_text_chunks={} scene_text_bytes={} text_views={} callback_buttons={} first_button_id={} second_button_id={} first_button_bytes={} second_button_bytes={} last_request_id={} phase=active completed_open_replayed=1 ambiguous_request_replay=0",
            ABI_VERSION,
            restart.epoch,
            app_pid,
            android_app_pid,
            restart.session_id,
            restart.package_generation,
            next.first_scene_node_count,
            next.describe_node_reads,
            next.node_reads,
            next.node_text_chunk_reads,
            next.first_scene_text_bytes,
            next.first_scene_text_view_count,
            next.first_scene_callback_count,
            next.first_callback_button_ids[0],
            next.first_callback_button_ids[1],
            next.first_callback_button_text_bytes[0],
            next.first_callback_button_text_bytes[1],
            next.first_last_request_id,
        );
    }
    *trace = next;

    crate::kprintln!(
        "ANDROID_APP_RPC_READ_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} kind={} request_id={} arg0={} arg1={} payload_bytes={} chunk_offset={} chunk_total={} first_round={} reads={} errors={} phase={} complete={}",
        direction.sender_name(),
        result.sender_pid,
        direction.receiver_name(),
        receiver_pid,
        android_app_rpc_kind_name(message.kind()),
        message.request_id(),
        message.arg0(),
        message.arg1(),
        message.payload().len(),
        message.chunk_offset(),
        message.chunk_total(),
        u8::from(first_round),
        next.reads,
        next.errors,
        next.phase.name(),
        u8::from(next.complete),
    );
}

#[cfg(feature = "androidbox-restart0")]
fn trace_android_app_supervisor_read(
    result: &EnvelopeReadResult,
    wire: &[u8; CHANNEL_MESSAGE_MAX_BYTES],
) {
    if result.logical_length != ANDROID_APP_SUPERVISOR_WIRE_SIZE {
        record_android_app_restart_error();
        return;
    }
    let Some(message) = AndroidAppSupervisorMessage::decode(wire) else {
        record_android_app_restart_error();
        return;
    };
    let Some(receiver_pid) = crate::process::current_live_user_process_id() else {
        record_android_app_restart_error();
        return;
    };
    let Some(app_pid) = crate::process::unique_live_process_id_for_image(UserImageId::App) else {
        record_android_app_restart_error();
        return;
    };
    let Some(init_pid) = crate::process::unique_live_process_id_for_image(UserImageId::Init) else {
        record_android_app_restart_error();
        return;
    };
    let restart = android_app_restart_trace_mut();
    let fields_match = message.epoch() == restart.epoch
        && message.old_pid() == restart.old_worker_pid
        && message.exit_code() == 0
        && message.termination_reason() == bndr_abi::ProcessTerminationReason::Faulted.raw();
    match message.kind() {
        AndroidAppSupervisorMessageKind::Rebind => {
            let current_worker =
                crate::process::unique_live_process_id_for_image(UserImageId::AndroidApp);
            let old_slot = message.old_pid() as u32;
            let new_slot = message.new_pid() as u32;
            let old_generation = (message.old_pid() >> 32) as u32;
            let new_generation = (message.new_pid() >> 32) as u32;
            if restart.phase != AndroidAppRestartPhase::AwaitRebind
                || result.kind != ChannelMessageKind::Transfer
                || result.sender_pid != init_pid
                || receiver_pid != app_pid
                || restart.app_pid != app_pid
                || !fields_match
                || restart.fault_reaps != 1
                || restart.grant_reissues != 0
                || message.new_pid() == message.old_pid()
                || current_worker != Some(message.new_pid())
                || old_slot != new_slot
                || old_generation.checked_add(1) != Some(new_generation)
            {
                restart.errors = restart.errors.saturating_add(1);
                return;
            }
            restart.rebind_reads = 1;
            restart.init_pid = init_pid;
            restart.new_worker_pid = message.new_pid();
            restart.phase = AndroidAppRestartPhase::AwaitReplacementReady;
            crate::kprintln!(
                "ANDROID_APP_REBIND_TRANSFER_OK format=1 abi={} epoch={} init_pid={} app_pid={} old_pid={} new_pid={} exit_code={} termination_reason={} same_slot=1 generation_step=1 endpoint_transfer=1",
                ABI_VERSION,
                message.epoch(),
                init_pid,
                app_pid,
                message.old_pid(),
                message.new_pid(),
                message.exit_code(),
                message.termination_reason(),
            );
        }
        AndroidAppSupervisorMessageKind::Rebound => {
            if restart.phase != AndroidAppRestartPhase::AwaitRebound
                || result.kind != ChannelMessageKind::Bytes
                || result.sender_pid != app_pid
                || receiver_pid != init_pid
                || restart.init_pid != init_pid
                || restart.new_worker_pid != message.new_pid()
                || !fields_match
            {
                restart.errors = restart.errors.saturating_add(1);
                return;
            }
            restart.rebound_reads = 1;
            restart.phase = AndroidAppRestartPhase::Complete;
            crate::kprintln!(
                "ANDROID_APP_REBOUND_ACK_OK format=1 abi={} epoch={} init_pid={} app_pid={} old_pid={} new_pid={} authenticated=1",
                ABI_VERSION,
                message.epoch(),
                init_pid,
                app_pid,
                message.old_pid(),
                message.new_pid(),
            );
        }
    }
}

#[cfg(feature = "androidbox-restart0")]
fn record_android_app_restart_error() {
    let restart = android_app_restart_trace_mut();
    restart.errors = restart.errors.saturating_add(1);
}

#[cfg(feature = "androidbox-process0")]
fn android_app_rpc_record_validation_error(error: AndroidAppRpcValidationError) {
    let trace = android_app_rpc_trace_mut();
    android_app_rpc_apply_validation_error(trace, error);
}

#[cfg(feature = "androidbox-process0")]
fn android_app_rpc_apply_validation_error(
    trace: &mut AndroidAppRpcTraceState,
    error: AndroidAppRpcValidationError,
) {
    trace.errors = trace.errors.saturating_add(1);
    if !trace.first_round_finished {
        trace.first_round_errors = trace.first_round_errors.saturating_add(1);
        trace.first_round_matches = false;
    }
    match error {
        AndroidAppRpcValidationError::Decode => {
            trace.decode_errors = trace.decode_errors.saturating_add(1);
        }
        AndroidAppRpcValidationError::Authentication => {
            trace.authentication_errors = trace.authentication_errors.saturating_add(1);
        }
        AndroidAppRpcValidationError::Direction => {
            trace.direction_errors = trace.direction_errors.saturating_add(1);
        }
        AndroidAppRpcValidationError::Protocol => {
            trace.protocol_errors = trace.protocol_errors.saturating_add(1);
        }
    }
}

#[cfg(feature = "androidbox-process0")]
const fn android_app_rpc_kind_has_direction(
    kind: AndroidAppMessageKind,
    direction: AndroidAppRpcDirection,
) -> bool {
    match direction {
        AndroidAppRpcDirection::AppToAndroidApp => match kind {
            AndroidAppMessageKind::Open
            | AndroidAppMessageKind::Click
            | AndroidAppMessageKind::Close => true,
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::DescribeNode => true,
            #[cfg(feature = "androidbox-restart0")]
            AndroidAppMessageKind::Crash => true,
            _ => false,
        },
        AndroidAppRpcDirection::AndroidAppToApp => match kind {
            AndroidAppMessageKind::Ready
            | AndroidAppMessageKind::Opened
            | AndroidAppMessageKind::LabelChunk
            | AndroidAppMessageKind::ButtonChunk
            | AndroidAppMessageKind::Updated
            | AndroidAppMessageKind::UpdateTextChunk
            | AndroidAppMessageKind::Closed
            | AndroidAppMessageKind::Error => true,
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::SceneOpened
            | AndroidAppMessageKind::Node
            | AndroidAppMessageKind::NodeTextChunk => true,
            _ => false,
        },
    }
}

#[cfg(all(
    feature = "androidbox-restart0",
    not(feature = "androidbox-scene-rpc2")
))]
fn android_app_rpc_reopen_trigger(
    message: AndroidAppMessage,
    _trace: &AndroidAppRpcTraceState,
) -> bool {
    message.kind() == AndroidAppMessageKind::ButtonChunk
}

#[cfg(feature = "androidbox-scene-rpc2")]
fn android_app_rpc_reopen_trigger(
    message: AndroidAppMessage,
    trace: &AndroidAppRpcTraceState,
) -> bool {
    message.kind() == AndroidAppMessageKind::NodeTextChunk
        && trace.phase == AndroidAppRpcPhase::Active
        && trace.scene_next_index == trace.scene_node_count
}

#[cfg(all(
    feature = "androidbox-restart0",
    not(feature = "androidbox-scene-rpc2")
))]
fn android_app_rpc_reopen_shape_is_complete(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_round_messages == 6
        && trace.reads == 6
        && trace.requests == 1
        && trace.responses == 2
        && trace.chunks == 3
        && trace.app_to_android_reads == 1
        && trace.android_to_app_reads == 5
        && trace.ready_reads == 1
        && trace.open_reads == 1
        && trace.opened_reads == 1
        && trace.label_chunk_reads == 2
        && trace.button_chunk_reads == 1
        && trace.first_label_bytes == ANDROID_APP_FIRST_LABEL_BYTES
        && trace.first_button_bytes == ANDROID_APP_FIRST_BUTTON_BYTES
        && trace.first_last_request_id == 1
        && trace.last_request_id == 1
}

#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
fn android_app_rpc_reopen_shape_is_complete(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_round_messages == ANDROID_APP_OPEN_TRANSCRIPT_MESSAGES
        && trace.reads == ANDROID_APP_OPEN_TRANSCRIPT_MESSAGES
        && trace.requests == 6
        && trace.responses == 7
        && trace.chunks == 3
        && trace.app_to_android_reads == 6
        && trace.android_to_app_reads == 10
        && trace.ready_reads == 1
        && trace.open_reads == 1
        && trace.opened_reads == 0
        && trace.scene_opened_reads == 1
        && trace.describe_node_reads == 5
        && trace.node_reads == 5
        && trace.node_text_chunk_reads == 3
        && trace.first_scene_node_count == 5
        && trace.first_scene_text_bytes == 52
        && trace.first_label_bytes == ANDROID_APP_FIRST_LABEL_BYTES
        && trace.first_button_bytes == ANDROID_APP_FIRST_BUTTON_BYTES
        && trace.first_last_request_id == ANDROID_APP_OPEN_LAST_REQUEST_ID
        && trace.last_request_id == ANDROID_APP_OPEN_LAST_REQUEST_ID
}

#[cfg(feature = "androidbox-multiaction3")]
fn android_app_rpc_reopen_shape_is_complete(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_round_messages == ANDROID_APP_OPEN_TRANSCRIPT_MESSAGES
        && trace.reads == ANDROID_APP_OPEN_TRANSCRIPT_MESSAGES
        && trace.requests
            == if cfg!(feature = "androidbox-layout-row14") {
                7
            } else {
                6
            }
        && trace.responses
            == if cfg!(feature = "androidbox-layout-row14") {
                8
            } else {
                7
            }
        && trace.chunks == 4
        && trace.app_to_android_reads
            == if cfg!(feature = "androidbox-layout-row14") {
                7
            } else {
                6
            }
        && trace.android_to_app_reads
            == if cfg!(feature = "androidbox-layout-row14") {
                12
            } else {
                11
            }
        && trace.ready_reads == 1
        && trace.open_reads == 1
        && trace.opened_reads == 0
        && trace.scene_opened_reads == 1
        && trace.describe_node_reads
            == if cfg!(feature = "androidbox-layout-row14") {
                6
            } else {
                5
            }
        && trace.node_reads
            == if cfg!(feature = "androidbox-layout-row14") {
                6
            } else {
                5
            }
        && trace.node_text_chunk_reads == 4
        && trace.first_scene_node_count
            == if cfg!(feature = "androidbox-layout-row14") {
                6
            } else {
                5
            }
        && trace.first_scene_text_bytes
            == if cfg!(feature = "androidbox-manifest-catalog3") {
                62
            } else if cfg!(feature = "androidbox-apk-envelope4") {
                45
            } else {
                44
            }
        && trace.first_scene_text_view_count == 2
        && trace.first_scene_callback_count == 2
        && trace.first_label_bytes == ANDROID_APP_FIRST_LABEL_BYTES
        && trace.first_button_bytes == ANDROID_APP_FIRST_BUTTON_BYTES
        && trace.first_callback_button_ids[0] == ANDROID_APP_RESOURCE_ID_BASE
        && trace.first_callback_button_ids[1] == ANDROID_APP_RESOURCE_ID_BASE + 1
        && trace.first_callback_button_text_bytes
            == if cfg!(feature = "androidbox-manifest-catalog3") {
                [15, 16]
            } else {
                [7, 6]
            }
        && trace.first_last_request_id == ANDROID_APP_OPEN_LAST_REQUEST_ID
        && trace.last_request_id == ANDROID_APP_OPEN_LAST_REQUEST_ID
}

#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
const fn android_app_rpc_scene_callback_count_is_admitted(count: u8) -> bool {
    count == 1
}

#[cfg(feature = "androidbox-multiaction3")]
const fn android_app_rpc_scene_callback_count_is_admitted(count: u8) -> bool {
    count >= 1 && count <= ANDROID_APP_TRACKED_CALLBACKS as u8
}

#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-layout-row14")
))]
fn android_app_rpc_multiaction_descriptor_matches(
    index: u8,
    descriptor: AndroidAppSceneNodeDescriptor,
) -> bool {
    match index {
        0 => {
            descriptor.kind() == AndroidAppSceneNodeKind::LinearLayout
                && descriptor.parent().is_none()
                && descriptor.id() == 0
                && descriptor.text_len() == 0
                && !descriptor.callback()
        }
        1 => {
            descriptor.kind() == AndroidAppSceneNodeKind::TextView
                && descriptor.parent() == Some(0)
                && descriptor.id() == ANDROID_APP_RESOURCE_ID_BASE + 3
                && descriptor.text_len()
                    == if cfg!(feature = "androidbox-manifest-catalog3") {
                        18
                    } else if cfg!(feature = "androidbox-apk-envelope4") {
                        15
                    } else {
                        14
                    }
                && !descriptor.callback()
        }
        2 => {
            descriptor.kind() == AndroidAppSceneNodeKind::TextView
                && descriptor.parent() == Some(0)
                && descriptor.id() == ANDROID_APP_RESOURCE_ID_BASE + 2
                && descriptor.text_len()
                    == if cfg!(feature = "androidbox-manifest-catalog3") {
                        13
                    } else {
                        17
                    }
                && !descriptor.callback()
        }
        3 => {
            descriptor.kind() == AndroidAppSceneNodeKind::Button
                && descriptor.parent() == Some(0)
                && descriptor.id() == ANDROID_APP_RESOURCE_ID_BASE
                && descriptor.text_len()
                    == if cfg!(feature = "androidbox-manifest-catalog3") {
                        15
                    } else {
                        7
                    }
                && descriptor.callback()
        }
        4 => {
            descriptor.kind() == AndroidAppSceneNodeKind::Button
                && descriptor.parent() == Some(0)
                && descriptor.id() == ANDROID_APP_RESOURCE_ID_BASE + 1
                && descriptor.text_len()
                    == if cfg!(feature = "androidbox-manifest-catalog3") {
                        16
                    } else {
                        6
                    }
                && descriptor.callback()
        }
        _ => false,
    }
}

#[cfg(feature = "androidbox-layout-directional17")]
fn android_app_rpc_directional_spacing_matches(
    descriptor: AndroidAppSceneNodeDescriptor,
    margins: [u8; 4],
    padding: [u8; 4],
) -> bool {
    [
        descriptor.layout_margin_left_dp(),
        descriptor.layout_margin_top_dp(),
        descriptor.layout_margin_right_dp(),
        descriptor.layout_margin_bottom_dp(),
    ] == margins
        && [
            descriptor.padding_left_dp(),
            descriptor.padding_top_dp(),
            descriptor.padding_right_dp(),
            descriptor.padding_bottom_dp(),
        ] == padding
}

#[cfg(feature = "androidbox-layout-size18")]
fn android_app_rpc_exact_size_matches(
    descriptor: AndroidAppSceneNodeDescriptor,
    width: AndroidAppSceneDimension,
    height: AndroidAppSceneDimension,
    exact_width_dp: u8,
    exact_height_dp: u8,
) -> bool {
    descriptor.width() == width
        && descriptor.height() == height
        && descriptor.exact_width_dp() == exact_width_dp
        && descriptor.exact_height_dp() == exact_height_dp
}

#[cfg(feature = "androidbox-layout-row14")]
fn android_app_rpc_multiaction_descriptor_matches(
    index: u8,
    descriptor: AndroidAppSceneNodeDescriptor,
) -> bool {
    match index {
        0 => {
            descriptor.kind() == AndroidAppSceneNodeKind::LinearLayout
                && descriptor.parent().is_none()
                && descriptor.id() == 0
                && descriptor.width() == AndroidAppSceneDimension::MatchParent
                && descriptor.height() == AndroidAppSceneDimension::MatchParent
                && descriptor.orientation() == AndroidAppSceneOrientation::Vertical
                && descriptor.text_len() == 0
                && !descriptor.callback()
                && {
                    #[cfg(feature = "androidbox-layout-size18")]
                    {
                        android_app_rpc_exact_size_matches(
                            descriptor,
                            AndroidAppSceneDimension::MatchParent,
                            AndroidAppSceneDimension::MatchParent,
                            0,
                            0,
                        )
                    }
                    #[cfg(not(feature = "androidbox-layout-size18"))]
                    {
                        true
                    }
                }
                && {
                    #[cfg(feature = "androidbox-layout-directional17")]
                    {
                        android_app_rpc_directional_spacing_matches(
                            descriptor,
                            [0, 0, 0, 0],
                            [0, 0, 0, 0],
                        )
                    }
                    #[cfg(all(
                        feature = "androidbox-layout-spacing16",
                        not(feature = "androidbox-layout-directional17")
                    ))]
                    {
                        descriptor.layout_margin_dp() == 0 && descriptor.padding_dp() == 0
                    }
                    #[cfg(not(feature = "androidbox-layout-spacing16"))]
                    {
                        true
                    }
                }
        }
        1 => {
            descriptor.kind() == AndroidAppSceneNodeKind::TextView
                && descriptor.parent() == Some(0)
                && descriptor.id() == ANDROID_APP_RESOURCE_ID_BASE + 3
                && descriptor.text_len() == 18
                && !descriptor.callback()
                && {
                    #[cfg(feature = "androidbox-layout-size18")]
                    {
                        android_app_rpc_exact_size_matches(
                            descriptor,
                            AndroidAppSceneDimension::Exact,
                            AndroidAppSceneDimension::WrapContent,
                            240,
                            0,
                        )
                    }
                    #[cfg(not(feature = "androidbox-layout-size18"))]
                    {
                        true
                    }
                }
                && {
                    #[cfg(feature = "androidbox-layout-directional17")]
                    {
                        android_app_rpc_directional_spacing_matches(
                            descriptor,
                            [0, 0, 0, 0],
                            [0, 0, 0, 0],
                        )
                    }
                    #[cfg(all(
                        feature = "androidbox-layout-spacing16",
                        not(feature = "androidbox-layout-directional17")
                    ))]
                    {
                        descriptor.layout_margin_dp() == 0 && descriptor.padding_dp() == 0
                    }
                    #[cfg(not(feature = "androidbox-layout-spacing16"))]
                    {
                        true
                    }
                }
        }
        2 => {
            descriptor.kind() == AndroidAppSceneNodeKind::TextView
                && descriptor.parent() == Some(0)
                && descriptor.id() == ANDROID_APP_RESOURCE_ID_BASE + 2
                && descriptor.text_len() == 13
                && !descriptor.callback()
                && {
                    #[cfg(feature = "androidbox-layout-size18")]
                    {
                        android_app_rpc_exact_size_matches(
                            descriptor,
                            AndroidAppSceneDimension::MatchParent,
                            AndroidAppSceneDimension::WrapContent,
                            0,
                            0,
                        )
                    }
                    #[cfg(not(feature = "androidbox-layout-size18"))]
                    {
                        true
                    }
                }
                && {
                    #[cfg(feature = "androidbox-layout-directional17")]
                    {
                        android_app_rpc_directional_spacing_matches(
                            descriptor,
                            [0, 0, 0, 0],
                            [0, 0, 0, 0],
                        )
                    }
                    #[cfg(all(
                        feature = "androidbox-layout-spacing16",
                        not(feature = "androidbox-layout-directional17")
                    ))]
                    {
                        descriptor.layout_margin_dp() == 0 && descriptor.padding_dp() == 0
                    }
                    #[cfg(not(feature = "androidbox-layout-spacing16"))]
                    {
                        true
                    }
                }
        }
        3 => {
            descriptor.kind() == AndroidAppSceneNodeKind::LinearLayout
                && descriptor.parent() == Some(0)
                && descriptor.id() == 0
                && {
                    #[cfg(feature = "androidbox-layout-size18")]
                    {
                        android_app_rpc_exact_size_matches(
                            descriptor,
                            AndroidAppSceneDimension::MatchParent,
                            AndroidAppSceneDimension::Exact,
                            0,
                            120,
                        )
                    }
                    #[cfg(not(feature = "androidbox-layout-size18"))]
                    {
                        descriptor.width() == AndroidAppSceneDimension::MatchParent
                            && descriptor.height() == AndroidAppSceneDimension::WrapContent
                    }
                }
                && descriptor.orientation() == AndroidAppSceneOrientation::Horizontal
                && descriptor.text_len() == 0
                && !descriptor.callback()
                && {
                    #[cfg(feature = "androidbox-layout-directional17")]
                    {
                        android_app_rpc_directional_spacing_matches(
                            descriptor,
                            [0, 0, 0, 0],
                            [6, 4, 2, 8],
                        )
                    }
                    #[cfg(all(
                        feature = "androidbox-layout-spacing16",
                        not(feature = "androidbox-layout-directional17")
                    ))]
                    {
                        descriptor.layout_margin_dp() == 0 && descriptor.padding_dp() == 4
                    }
                    #[cfg(not(feature = "androidbox-layout-spacing16"))]
                    {
                        true
                    }
                }
        }
        4 => {
            descriptor.kind() == AndroidAppSceneNodeKind::Button
                && descriptor.parent() == Some(3)
                && descriptor.id() == ANDROID_APP_RESOURCE_ID_BASE
                && {
                    #[cfg(feature = "androidbox-layout-mixed19")]
                    {
                        android_app_rpc_exact_size_matches(
                            descriptor,
                            AndroidAppSceneDimension::Exact,
                            AndroidAppSceneDimension::Exact,
                            132,
                            64,
                        )
                    }
                    #[cfg(all(
                        feature = "androidbox-layout-size18",
                        not(feature = "androidbox-layout-mixed19")
                    ))]
                    {
                        android_app_rpc_exact_size_matches(
                            descriptor,
                            AndroidAppSceneDimension::Zero,
                            AndroidAppSceneDimension::Exact,
                            0,
                            64,
                        )
                    }
                    #[cfg(not(feature = "androidbox-layout-size18"))]
                    {
                        descriptor.height() == AndroidAppSceneDimension::WrapContent
                    }
                }
                && {
                    #[cfg(feature = "androidbox-layout-mixed19")]
                    {
                        descriptor.width() == AndroidAppSceneDimension::Exact
                            && descriptor.layout_weight() == 0
                    }
                    #[cfg(all(
                        feature = "androidbox-layout-weight15",
                        not(feature = "androidbox-layout-mixed19")
                    ))]
                    {
                        descriptor.width() == AndroidAppSceneDimension::Zero
                            && descriptor.layout_weight() == 1
                    }
                    #[cfg(not(feature = "androidbox-layout-weight15"))]
                    {
                        descriptor.width() == AndroidAppSceneDimension::WrapContent
                            && descriptor.layout_weight() == 0
                    }
                }
                && descriptor.text_len() == 15
                && descriptor.callback()
                && {
                    #[cfg(feature = "androidbox-layout-directional17")]
                    {
                        android_app_rpc_directional_spacing_matches(
                            descriptor,
                            [2, 1, 4, 3],
                            [0, 0, 0, 0],
                        )
                    }
                    #[cfg(all(
                        feature = "androidbox-layout-spacing16",
                        not(feature = "androidbox-layout-directional17")
                    ))]
                    {
                        descriptor.layout_margin_dp() == 2 && descriptor.padding_dp() == 0
                    }
                    #[cfg(not(feature = "androidbox-layout-spacing16"))]
                    {
                        true
                    }
                }
        }
        5 => {
            descriptor.kind() == AndroidAppSceneNodeKind::Button
                && descriptor.parent() == Some(3)
                && descriptor.id() == ANDROID_APP_RESOURCE_ID_BASE + 1
                && {
                    #[cfg(feature = "androidbox-layout-size18")]
                    {
                        android_app_rpc_exact_size_matches(
                            descriptor,
                            AndroidAppSceneDimension::Zero,
                            AndroidAppSceneDimension::Exact,
                            0,
                            56,
                        )
                    }
                    #[cfg(not(feature = "androidbox-layout-size18"))]
                    {
                        descriptor.height() == AndroidAppSceneDimension::WrapContent
                    }
                }
                && {
                    #[cfg(feature = "androidbox-layout-weight15")]
                    {
                        descriptor.width() == AndroidAppSceneDimension::Zero
                            && descriptor.layout_weight() == 1
                    }
                    #[cfg(not(feature = "androidbox-layout-weight15"))]
                    {
                        descriptor.width() == AndroidAppSceneDimension::WrapContent
                            && descriptor.layout_weight() == 0
                    }
                }
                && descriptor.text_len() == 16
                && descriptor.callback()
                && {
                    #[cfg(feature = "androidbox-layout-directional17")]
                    {
                        android_app_rpc_directional_spacing_matches(
                            descriptor,
                            [6, 5, 2, 1],
                            [0, 0, 0, 0],
                        )
                    }
                    #[cfg(all(
                        feature = "androidbox-layout-spacing16",
                        not(feature = "androidbox-layout-directional17")
                    ))]
                    {
                        descriptor.layout_margin_dp() == 2 && descriptor.padding_dp() == 0
                    }
                    #[cfg(not(feature = "androidbox-layout-spacing16"))]
                    {
                        true
                    }
                }
        }
        _ => false,
    }
}

#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-layout-row14")
))]
fn android_app_rpc_multiaction_node_text_matches(index: u8, payload: &[u8]) -> bool {
    match index {
        1 => {
            payload
                == if cfg!(feature = "androidbox-manifest-catalog3") {
                    b"Android components".as_slice()
                } else if cfg!(feature = "androidbox-apk-envelope4") {
                    b"Envelope review".as_slice()
                } else {
                    b"Review request".as_slice()
                }
        }
        2 => {
            payload
                == if cfg!(feature = "androidbox-manifest-catalog3") {
                    b"Catalog ready".as_slice()
                } else {
                    b"Decision: pending".as_slice()
                }
        }
        3 => {
            payload
                == if cfg!(feature = "androidbox-manifest-catalog3") {
                    b"Show components".as_slice()
                } else {
                    b"Approve".as_slice()
                }
        }
        4 => {
            payload
                == if cfg!(feature = "androidbox-manifest-catalog3") {
                    b"Show permissions".as_slice()
                } else {
                    b"Reject".as_slice()
                }
        }
        _ => false,
    }
}

#[cfg(feature = "androidbox-layout-row14")]
fn android_app_rpc_multiaction_node_text_matches(index: u8, payload: &[u8]) -> bool {
    match index {
        1 => payload == b"Android components",
        2 => payload == b"Catalog ready",
        4 => payload == b"Show components",
        5 => payload == b"Show permissions",
        _ => false,
    }
}

#[cfg(all(
    feature = "androidbox-process0",
    not(feature = "androidbox-multiaction3")
))]
fn android_app_rpc_first_click_matches(
    trace: &AndroidAppRpcTraceState,
    request_id: u64,
    revision: u32,
) -> bool {
    trace.first_round_messages == ANDROID_APP_CLICK_PRECEDING_MESSAGES
        && request_id == ANDROID_APP_CRASH_REQUEST_ID
        && revision == 0
}

#[cfg(feature = "androidbox-multiaction3")]
fn android_app_rpc_first_click_matches(
    trace: &AndroidAppRpcTraceState,
    request_id: u64,
    revision: u32,
) -> bool {
    match revision {
        0 => {
            trace.first_round_messages == ANDROID_APP_CLICK_PRECEDING_MESSAGES
                && request_id == ANDROID_APP_CRASH_REQUEST_ID
        }
        1 => {
            trace.first_round_messages
                == if cfg!(feature = "androidbox-layout-row14") {
                    22
                } else {
                    20
                }
                && request_id
                    == if cfg!(feature = "androidbox-layout-row14") {
                        9
                    } else {
                        8
                    }
        }
        _ => false,
    }
}

#[cfg(all(
    feature = "androidbox-process0",
    not(feature = "androidbox-multiaction3")
))]
fn android_app_rpc_first_update_matches(
    trace: &AndroidAppRpcTraceState,
    request_id: u64,
    revision: u32,
    update_total: u32,
) -> bool {
    trace.first_round_messages == ANDROID_APP_UPDATED_PRECEDING_MESSAGES
        && request_id == ANDROID_APP_CRASH_REQUEST_ID
        && revision == 1
        && update_total == ANDROID_APP_FIRST_UPDATE_BYTES
}

#[cfg(feature = "androidbox-multiaction3")]
fn android_app_rpc_first_update_matches(
    trace: &AndroidAppRpcTraceState,
    request_id: u64,
    revision: u32,
    update_total: u32,
) -> bool {
    update_total
        == if cfg!(feature = "androidbox-string-builder13") {
            match revision {
                1 | 2 => 15,
                _ => 0,
            }
        } else if cfg!(feature = "androidbox-string-text12") {
            match revision {
                1 => 21,
                2 => 27,
                _ => 0,
            }
        } else if cfg!(feature = "androidbox-manifest-catalog3") && revision == 2 {
            26
        } else {
            ANDROID_APP_FIRST_UPDATE_BYTES
        }
        && match revision {
            1 => {
                trace.first_round_messages == ANDROID_APP_UPDATED_PRECEDING_MESSAGES
                    && request_id == ANDROID_APP_CRASH_REQUEST_ID
            }
            2 => {
                trace.first_round_messages
                    == if cfg!(feature = "androidbox-layout-row14") {
                        23
                    } else {
                        21
                    }
                    && request_id
                        == if cfg!(feature = "androidbox-layout-row14") {
                            9
                        } else {
                            8
                        }
            }
            _ => false,
        }
}

#[cfg(all(
    feature = "androidbox-process0",
    not(feature = "androidbox-multiaction3")
))]
fn android_app_rpc_first_update_chunk_matches(
    trace: &AndroidAppRpcTraceState,
    message: AndroidAppMessage,
) -> bool {
    trace.first_round_messages == ANDROID_APP_UPDATE_CHUNK_PRECEDING_MESSAGES
        && message.chunk_total() == ANDROID_APP_FIRST_UPDATE_BYTES
        && message.payload().len() == ANDROID_APP_FIRST_UPDATE_BYTES as usize
}

#[cfg(feature = "androidbox-multiaction3")]
fn android_app_rpc_first_update_chunk_matches(
    trace: &AndroidAppRpcTraceState,
    message: AndroidAppMessage,
) -> bool {
    if cfg!(feature = "androidbox-string-builder13") {
        return match trace.revision {
            1 => {
                trace.first_round_messages == ANDROID_APP_UPDATE_CHUNK_PRECEDING_MESSAGES
                    && message.request_id() == ANDROID_APP_CRASH_REQUEST_ID
                    && message.chunk_offset() == 0
                    && message.chunk_total() == 15
                    && message.payload() == b"Review count: 1"
            }
            2 => {
                trace.first_round_messages
                    == if cfg!(feature = "androidbox-layout-row14") {
                        24
                    } else {
                        22
                    }
                    && message.request_id()
                        == if cfg!(feature = "androidbox-layout-row14") {
                            9
                        } else {
                            8
                        }
                    && message.chunk_offset() == 0
                    && message.chunk_total() == 15
                    && message.payload() == b"Review count: 2"
            }
            _ => false,
        };
    }
    if cfg!(feature = "androidbox-string-text12") {
        return match trace.revision {
            1 => {
                trace.first_round_messages == ANDROID_APP_UPDATE_CHUNK_PRECEDING_MESSAGES
                    && message.request_id() == ANDROID_APP_CRASH_REQUEST_ID
                    && message.chunk_offset() == 0
                    && message.chunk_total() == 21
                    && message.payload() == b"First review recorded"
            }
            2 => {
                message.request_id() == 8
                    && message.chunk_total() == 27
                    && match message.chunk_offset() {
                        0 => {
                            trace.first_round_messages == 22
                                && message.payload() == b"Review state advanced ag"
                        }
                        24 => trace.first_round_messages == 23 && message.payload() == b"ain",
                        _ => false,
                    }
            }
            _ => false,
        };
    }
    if cfg!(feature = "androidbox-manifest-catalog3") {
        return match trace.revision {
            1 => {
                trace.first_round_messages == ANDROID_APP_UPDATE_CHUNK_PRECEDING_MESSAGES
                    && message.request_id() == ANDROID_APP_CRASH_REQUEST_ID
                    && message.chunk_offset() == 0
                    && message.chunk_total() == 24
                    && message.payload() == b"Components: 6 discovered"
            }
            2 => {
                message.request_id() == 8
                    && message.chunk_total() == 26
                    && match message.chunk_offset() {
                        0 => {
                            trace.first_round_messages == 22
                                && message.payload() == b"Permissions: declared on"
                        }
                        24 => trace.first_round_messages == 23 && message.payload() == b"ly",
                        _ => false,
                    }
            }
            _ => false,
        };
    }
    message.chunk_total() == ANDROID_APP_FIRST_UPDATE_BYTES
        && message.payload().len() == ANDROID_APP_FIRST_UPDATE_BYTES as usize
        && match trace.revision {
            1 => {
                trace.first_round_messages == ANDROID_APP_UPDATE_CHUNK_PRECEDING_MESSAGES
                    && message.request_id() == ANDROID_APP_CRASH_REQUEST_ID
                    && message.payload() == b"Decision: approved"
            }
            2 => {
                trace.first_round_messages == 22
                    && message.request_id() == 8
                    && message.payload() == b"Decision: rejected"
            }
            _ => false,
        }
}

#[cfg(feature = "androidbox-process0")]
fn android_app_rpc_accept(
    trace: &mut AndroidAppRpcTraceState,
    message: AndroidAppMessage,
) -> Option<bool> {
    use AndroidAppMessageKind::{
        ButtonChunk, Click, Close, Closed, Error, LabelChunk, Open, Opened, Ready, UpdateTextChunk,
        Updated,
    };

    if message.kind() == Error {
        let error_code_valid = match trace.phase {
            AndroidAppRpcPhase::AwaitOpened => matches!(message.arg0(), 1 | 4),
            AndroidAppRpcPhase::AwaitUpdated => message.arg0() == 2,
            AndroidAppRpcPhase::AwaitClosed => message.arg0() == 3,
            AndroidAppRpcPhase::AwaitReady
            | AndroidAppRpcPhase::Idle
            | AndroidAppRpcPhase::LabelChunks
            | AndroidAppRpcPhase::ButtonChunks
            | AndroidAppRpcPhase::Active
            | AndroidAppRpcPhase::UpdateChunks => false,
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppRpcPhase::AwaitNodeRequest
            | AndroidAppRpcPhase::AwaitNode
            | AndroidAppRpcPhase::NodeChunks => false,
        };
        if message.request_id() != trace.pending_request_id || !error_code_valid {
            return None;
        }
        android_app_rpc_first_matches(false, trace);
        android_app_rpc_clear_active(trace);
        trace.phase = AndroidAppRpcPhase::Idle;
        return Some(false);
    }

    match (trace.phase, message.kind()) {
        (AndroidAppRpcPhase::AwaitReady, Ready) => {
            android_app_rpc_first_matches(trace.first_round_messages == 0, trace);
            trace.phase = AndroidAppRpcPhase::Idle;
        }
        (AndroidAppRpcPhase::Idle, Open) => {
            if trace.last_request_id.checked_add(1) != Some(message.request_id()) {
                return None;
            }
            android_app_rpc_first_matches(
                trace.first_round_messages == 1 && message.request_id() == 1,
                trace,
            );
            trace.last_request_id = message.request_id();
            trace.pending_request_id = message.request_id();
            trace.session_id = message.arg0();
            trace.package_generation = message.arg1();
            trace.label_id = 0;
            trace.button_id = 0;
            trace.revision = 0;
            trace.phase = AndroidAppRpcPhase::AwaitOpened;
        }
        (AndroidAppRpcPhase::AwaitOpened, Opened) => {
            if message.request_id() != trace.pending_request_id {
                return None;
            }
            let label_id = (message.arg0() >> 32) as u32;
            let button_id = message.arg0() as u32;
            let revision = (message.arg1() >> 32) as u32;
            let label_total = ((message.arg1() >> 16) & 0xffff) as u32;
            let button_total = (message.arg1() & 0xffff) as u32;
            android_app_rpc_first_matches(
                trace.first_round_messages == 2
                    && message.request_id() == 1
                    && revision == 0
                    && label_total == ANDROID_APP_FIRST_LABEL_BYTES
                    && button_total == ANDROID_APP_FIRST_BUTTON_BYTES,
                trace,
            );
            trace.label_id = label_id;
            trace.button_id = button_id;
            trace.revision = revision;
            trace.label_total = label_total;
            trace.label_received = 0;
            trace.button_total = button_total;
            trace.button_received = 0;
            trace.phase = AndroidAppRpcPhase::LabelChunks;
        }
        #[cfg(feature = "androidbox-scene-rpc2")]
        (AndroidAppRpcPhase::AwaitOpened, AndroidAppMessageKind::SceneOpened) => {
            if message.request_id() != trace.pending_request_id
                || message.arg1() != 0
                || message.arg0()
                    != if cfg!(feature = "androidbox-layout-row14") {
                        6
                    } else {
                        5
                    }
            {
                return None;
            }
            android_app_rpc_first_matches(
                trace.first_round_messages == 2 && message.request_id() == 1,
                trace,
            );
            trace.scene_node_count = message.arg0() as u8;
            trace.scene_next_index = 0;
            trace.scene_current_index = 0;
            trace.scene_current_id = 0;
            trace.scene_text_total = 0;
            trace.scene_text_received = 0;
            trace.phase = AndroidAppRpcPhase::AwaitNodeRequest;
        }
        #[cfg(feature = "androidbox-scene-rpc2")]
        (AndroidAppRpcPhase::AwaitNodeRequest, AndroidAppMessageKind::DescribeNode) => {
            let revision = (message.arg1() >> 32) as u32;
            let index = message.arg1() as u32 as usize;
            if trace.last_request_id.checked_add(1) != Some(message.request_id())
                || message.arg0() != trace.session_id
                || revision != trace.revision
                || index != usize::from(trace.scene_next_index)
                || index >= usize::from(trace.scene_node_count)
            {
                return None;
            }
            android_app_rpc_first_matches(message.request_id() == index as u64 + 2, trace);
            trace.last_request_id = message.request_id();
            trace.pending_request_id = message.request_id();
            trace.scene_current_index = trace.scene_next_index;
            trace.phase = AndroidAppRpcPhase::AwaitNode;
        }
        #[cfg(feature = "androidbox-scene-rpc2")]
        (AndroidAppRpcPhase::AwaitNode, AndroidAppMessageKind::Node) => {
            if message.request_id() != trace.pending_request_id
                || message.arg0() != u64::from(trace.scene_current_index)
                || message.arg1() != u64::from(trace.revision)
                || message.payload().len() != ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE
            {
                return None;
            }
            let mut wire = [0; ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE];
            wire.copy_from_slice(message.payload());
            let descriptor = AndroidAppSceneNodeDescriptor::decode(&wire)?;
            let index = usize::from(trace.scene_current_index);
            if !descriptor.is_canonical_for_index(trace.scene_current_index)
                || descriptor.parent().is_some_and(|parent| {
                    trace.scene_node_kinds[usize::from(parent)]
                        != AndroidAppSceneNodeKind::LinearLayout.raw()
                })
                || (descriptor.id() != 0
                    && trace.scene_node_ids[..index]
                        .iter()
                        .any(|id| *id == descriptor.id()))
            {
                return None;
            }
            trace.scene_node_ids[index] = descriptor.id();
            trace.scene_node_kinds[index] = descriptor.kind().raw();
            trace.scene_current_id = descriptor.id();
            trace.scene_text_total = u32::from(descriptor.text_len());
            trace.scene_text_received = 0;
            #[cfg(feature = "androidbox-multiaction3")]
            android_app_rpc_first_matches(
                android_app_rpc_multiaction_descriptor_matches(
                    trace.scene_current_index,
                    descriptor,
                ),
                trace,
            );
            match descriptor.kind() {
                AndroidAppSceneNodeKind::LinearLayout => {}
                AndroidAppSceneNodeKind::TextView => {
                    let text_index = usize::from(trace.scene_text_view_count);
                    if text_index >= trace.scene_text_view_ids.len() {
                        return None;
                    }
                    trace.scene_text_view_ids[text_index] = descriptor.id();
                    trace.scene_text_view_count += 1;
                    if trace.label_id == 0 {
                        trace.label_id = descriptor.id();
                        trace.label_total = u32::from(descriptor.text_len());
                    }
                }
                AndroidAppSceneNodeKind::Button => {
                    if descriptor.callback() {
                        let callback_index = usize::from(trace.scene_callback_count);
                        if callback_index >= trace.scene_callback_ids.len() {
                            return None;
                        }
                        trace.scene_callback_ids[callback_index] = descriptor.id();
                        trace.scene_callback_count = trace.scene_callback_count.checked_add(1)?;
                        if trace.button_id == 0 {
                            trace.button_id = descriptor.id();
                            trace.button_total = u32::from(descriptor.text_len());
                        }
                    }
                }
            }
            android_app_rpc_first_matches(true, trace);
            if trace.scene_text_total == 0 {
                trace.scene_next_index = trace.scene_next_index.checked_add(1)?;
                trace.phase = if trace.scene_next_index == trace.scene_node_count {
                    if trace.scene_text_view_count == 0
                        || !android_app_rpc_scene_callback_count_is_admitted(
                            trace.scene_callback_count,
                        )
                    {
                        return None;
                    }
                    AndroidAppRpcPhase::Active
                } else {
                    AndroidAppRpcPhase::AwaitNodeRequest
                };
            } else {
                trace.phase = AndroidAppRpcPhase::NodeChunks;
            }
        }
        #[cfg(feature = "androidbox-scene-rpc2")]
        (AndroidAppRpcPhase::NodeChunks, AndroidAppMessageKind::NodeTextChunk) => {
            if message.request_id() != trace.pending_request_id
                || message.arg1() != u64::from(trace.scene_current_index)
                || !android_app_rpc_chunk_matches(
                    message,
                    trace.scene_text_total,
                    trace.scene_text_received,
                )
            {
                return None;
            }
            trace.scene_text_received += message.payload().len() as u32;
            trace.scene_text_bytes += message.payload().len() as u32;
            if trace.scene_current_id == trace.label_id {
                trace.label_received += message.payload().len() as u32;
            }
            if trace.scene_current_id == trace.button_id {
                trace.button_received += message.payload().len() as u32;
            }
            if let Some(callback_index) = trace.scene_callback_ids
                [..usize::from(trace.scene_callback_count)]
                .iter()
                .position(|id| *id == trace.scene_current_id)
            {
                trace.scene_callback_text_bytes[callback_index] = trace.scene_callback_text_bytes
                    [callback_index]
                    .saturating_add(message.payload().len() as u32);
            }
            #[cfg(feature = "androidbox-multiaction3")]
            android_app_rpc_first_matches(
                android_app_rpc_multiaction_node_text_matches(
                    trace.scene_current_index,
                    message.payload(),
                ),
                trace,
            );
            android_app_rpc_first_matches(true, trace);
            if trace.scene_text_received == trace.scene_text_total {
                trace.scene_next_index = trace.scene_next_index.checked_add(1)?;
                trace.phase = if trace.scene_next_index == trace.scene_node_count {
                    if trace.scene_text_view_count == 0
                        || !android_app_rpc_scene_callback_count_is_admitted(
                            trace.scene_callback_count,
                        )
                        || trace.label_received != trace.label_total
                        || trace.button_received != trace.button_total
                    {
                        return None;
                    }
                    AndroidAppRpcPhase::Active
                } else {
                    AndroidAppRpcPhase::AwaitNodeRequest
                };
            }
        }
        (AndroidAppRpcPhase::LabelChunks, LabelChunk) => {
            if message.request_id() != trace.pending_request_id
                || !android_app_rpc_chunk_matches(message, trace.label_total, trace.label_received)
            {
                return None;
            }
            android_app_rpc_first_matches(
                match trace.label_received {
                    0 => {
                        trace.first_round_messages == 3
                            && message.chunk_total() == ANDROID_APP_FIRST_LABEL_BYTES
                            && message.payload().len() == ANDROID_APP_MESSAGE_PAYLOAD_BYTES
                    }
                    24 => {
                        trace.first_round_messages == 4
                            && message.chunk_total() == ANDROID_APP_FIRST_LABEL_BYTES
                            && message.payload().len() == 2
                    }
                    _ => false,
                },
                trace,
            );
            trace.label_received += message.payload().len() as u32;
            if trace.label_received == trace.label_total {
                trace.phase = AndroidAppRpcPhase::ButtonChunks;
            }
        }
        (AndroidAppRpcPhase::ButtonChunks, ButtonChunk) => {
            if message.request_id() != trace.pending_request_id
                || !android_app_rpc_chunk_matches(
                    message,
                    trace.button_total,
                    trace.button_received,
                )
            {
                return None;
            }
            android_app_rpc_first_matches(
                trace.first_round_messages == 5
                    && message.chunk_total() == ANDROID_APP_FIRST_BUTTON_BYTES
                    && message.payload().len() == ANDROID_APP_FIRST_BUTTON_BYTES as usize,
                trace,
            );
            trace.button_received += message.payload().len() as u32;
            if trace.button_received == trace.button_total {
                trace.phase = AndroidAppRpcPhase::Active;
            }
        }
        (AndroidAppRpcPhase::Active, Click) => {
            let view_id = (message.arg1() >> 32) as u32;
            let revision = message.arg1() as u32;
            #[cfg(not(feature = "androidbox-multiaction3"))]
            let callback_matches = view_id == trace.button_id;
            #[cfg(feature = "androidbox-multiaction3")]
            let callback_matches = usize::try_from(revision)
                .ok()
                .filter(|index| *index < ANDROID_APP_MULTIACTION_CALLBACKS)
                .is_some_and(|index| trace.scene_callback_ids[index] == view_id);
            if trace.last_request_id.checked_add(1) != Some(message.request_id())
                || message.arg0() != trace.session_id
                || !callback_matches
                || revision != trace.revision
            {
                return None;
            }
            android_app_rpc_first_matches(
                android_app_rpc_first_click_matches(trace, message.request_id(), revision),
                trace,
            );
            trace.last_request_id = message.request_id();
            trace.pending_request_id = message.request_id();
            trace.phase = AndroidAppRpcPhase::AwaitUpdated;
        }
        (AndroidAppRpcPhase::AwaitUpdated, Updated) => {
            let revision = (message.arg1() >> 32) as u32;
            let update_total = message.arg1() as u32;
            #[cfg(not(feature = "androidbox-scene-rpc2"))]
            let update_target_matches = message.arg0() == u64::from(trace.label_id);
            #[cfg(feature = "androidbox-scene-rpc2")]
            let update_target_matches = {
                let update_id = message.arg0() as u32;
                let known_text_view = trace.scene_text_view_ids
                    [..usize::from(trace.scene_text_view_count)]
                    .contains(&update_id);
                #[cfg(not(feature = "androidbox-multiaction3"))]
                {
                    known_text_view
                        && (trace.update_view_id == 0 || trace.update_view_id == update_id)
                }
                #[cfg(feature = "androidbox-multiaction3")]
                {
                    known_text_view
                }
            };
            if message.request_id() != trace.pending_request_id
                || !update_target_matches
                || trace.revision.checked_add(1) != Some(revision)
            {
                return None;
            }
            android_app_rpc_first_matches(
                android_app_rpc_first_update_matches(
                    trace,
                    message.request_id(),
                    revision,
                    update_total,
                ),
                trace,
            );
            #[cfg(feature = "androidbox-scene-rpc2")]
            {
                trace.update_view_id = message.arg0() as u32;
            }
            trace.revision = revision;
            trace.update_total = update_total;
            trace.update_received = 0;
            trace.phase = AndroidAppRpcPhase::UpdateChunks;
        }
        (AndroidAppRpcPhase::UpdateChunks, UpdateTextChunk) => {
            if message.request_id() != trace.pending_request_id
                || !android_app_rpc_chunk_matches(
                    message,
                    trace.update_total,
                    trace.update_received,
                )
            {
                return None;
            }
            android_app_rpc_first_matches(
                android_app_rpc_first_update_chunk_matches(trace, message),
                trace,
            );
            trace.update_received += message.payload().len() as u32;
            if trace.update_received == trace.update_total {
                trace.phase = AndroidAppRpcPhase::Active;
            }
        }
        (AndroidAppRpcPhase::Active, Close) => {
            if trace.last_request_id.checked_add(1) != Some(message.request_id())
                || message.arg0() != trace.session_id
                || message.arg1() != trace.package_generation
            {
                return None;
            }
            android_app_rpc_first_matches(
                trace.first_round_messages == ANDROID_APP_CLOSE_PRECEDING_MESSAGES
                    && message.request_id() == ANDROID_APP_FINAL_REQUEST_ID,
                trace,
            );
            trace.last_request_id = message.request_id();
            trace.pending_request_id = message.request_id();
            trace.phase = AndroidAppRpcPhase::AwaitClosed;
        }
        (AndroidAppRpcPhase::AwaitClosed, Closed) => {
            if message.request_id() != trace.pending_request_id {
                return None;
            }
            android_app_rpc_first_matches(
                trace.first_round_messages == ANDROID_APP_CLOSED_PRECEDING_MESSAGES
                    && message.request_id() == ANDROID_APP_FINAL_REQUEST_ID,
                trace,
            );
            android_app_rpc_clear_active(trace);
            trace.phase = AndroidAppRpcPhase::Idle;
            return Some(true);
        }
        (
            AndroidAppRpcPhase::AwaitReady
            | AndroidAppRpcPhase::Idle
            | AndroidAppRpcPhase::AwaitOpened
            | AndroidAppRpcPhase::LabelChunks
            | AndroidAppRpcPhase::ButtonChunks
            | AndroidAppRpcPhase::Active
            | AndroidAppRpcPhase::AwaitUpdated
            | AndroidAppRpcPhase::UpdateChunks
            | AndroidAppRpcPhase::AwaitClosed,
            Ready | Open | Opened | LabelChunk | ButtonChunk | Click | Updated | UpdateTextChunk
            | Close | Closed | Error,
        ) => return None,
        #[cfg(feature = "androidbox-scene-rpc2")]
        (
            AndroidAppRpcPhase::AwaitNodeRequest
            | AndroidAppRpcPhase::AwaitNode
            | AndroidAppRpcPhase::NodeChunks,
            Ready | Open | Opened | LabelChunk | ButtonChunk | Click | Updated | UpdateTextChunk
            | Close | Closed | Error,
        ) => return None,
        #[cfg(feature = "androidbox-scene-rpc2")]
        (
            _,
            AndroidAppMessageKind::SceneOpened
            | AndroidAppMessageKind::DescribeNode
            | AndroidAppMessageKind::Node
            | AndroidAppMessageKind::NodeTextChunk,
        ) => return None,
        #[cfg(feature = "androidbox-restart0")]
        (_, AndroidAppMessageKind::Crash) => return None,
    }
    Some(false)
}

#[cfg(feature = "androidbox-process0")]
fn android_app_rpc_first_matches(matches: bool, trace: &mut AndroidAppRpcTraceState) {
    if !trace.first_round_finished {
        trace.first_round_matches &= matches;
    }
}

#[cfg(feature = "androidbox-process0")]
fn android_app_rpc_chunk_matches(message: AndroidAppMessage, total: u32, received: u32) -> bool {
    let remaining = total.saturating_sub(received) as usize;
    message.chunk_offset() == received
        && message.chunk_total() == total
        && message.payload().len() == remaining.min(ANDROID_APP_MESSAGE_PAYLOAD_BYTES)
}

#[cfg(feature = "androidbox-process0")]
fn android_app_rpc_clear_active(trace: &mut AndroidAppRpcTraceState) {
    trace.pending_request_id = 0;
    trace.session_id = 0;
    trace.package_generation = 0;
    trace.label_id = 0;
    trace.button_id = 0;
    trace.revision = 0;
    trace.label_total = 0;
    trace.label_received = 0;
    trace.button_total = 0;
    trace.button_received = 0;
    trace.update_total = 0;
    trace.update_received = 0;
    #[cfg(feature = "androidbox-scene-rpc2")]
    {
        trace.update_view_id = 0;
        trace.scene_node_count = 0;
        trace.scene_next_index = 0;
        trace.scene_current_index = 0;
        trace.scene_current_id = 0;
        trace.scene_text_total = 0;
        trace.scene_text_received = 0;
        trace.scene_text_bytes = 0;
        trace.scene_text_view_ids.fill(0);
        trace.scene_text_view_count = 0;
        trace.scene_node_ids.fill(0);
        trace.scene_node_kinds.fill(0);
        trace.scene_callback_count = 0;
        trace.scene_callback_ids.fill(0);
        trace.scene_callback_text_bytes.fill(0);
    }
}

#[cfg(feature = "androidbox-process0")]
fn android_app_rpc_record_accepted(
    trace: &mut AndroidAppRpcTraceState,
    direction: AndroidAppRpcDirection,
    message: AndroidAppMessage,
    first_round: bool,
    completed_round: bool,
) {
    trace.reads = trace.reads.saturating_add(1);
    if message.kind() == AndroidAppMessageKind::Error {
        trace.error_messages = trace.error_messages.saturating_add(1);
    }
    if first_round {
        trace.first_round_messages = trace.first_round_messages.saturating_add(1);
        match direction {
            AndroidAppRpcDirection::AppToAndroidApp => {
                trace.app_to_android_reads = trace.app_to_android_reads.saturating_add(1);
                trace.requests = trace.requests.saturating_add(1);
                trace.first_last_request_id = message.request_id();
            }
            AndroidAppRpcDirection::AndroidAppToApp => {
                trace.android_to_app_reads = trace.android_to_app_reads.saturating_add(1);
                let is_chunk = matches!(
                    message.kind(),
                    AndroidAppMessageKind::LabelChunk
                        | AndroidAppMessageKind::ButtonChunk
                        | AndroidAppMessageKind::UpdateTextChunk
                );
                #[cfg(feature = "androidbox-scene-rpc2")]
                let is_chunk = is_chunk || message.kind() == AndroidAppMessageKind::NodeTextChunk;
                if is_chunk {
                    trace.chunks = trace.chunks.saturating_add(1);
                } else {
                    trace.responses = trace.responses.saturating_add(1);
                }
            }
        }
        match message.kind() {
            AndroidAppMessageKind::Ready => {
                trace.ready_reads = trace.ready_reads.saturating_add(1);
            }
            AndroidAppMessageKind::Open => {
                trace.open_reads = trace.open_reads.saturating_add(1);
                trace.first_session_id = message.arg0();
                trace.first_package_generation = message.arg1();
            }
            AndroidAppMessageKind::Opened => {
                trace.opened_reads = trace.opened_reads.saturating_add(1);
                trace.first_label_id = (message.arg0() >> 32) as u32;
                trace.first_button_id = message.arg0() as u32;
                trace.first_revision = (message.arg1() >> 32) as u32;
            }
            AndroidAppMessageKind::LabelChunk => {
                trace.label_chunk_reads = trace.label_chunk_reads.saturating_add(1);
                trace.first_label_bytes = trace
                    .first_label_bytes
                    .saturating_add(message.payload().len() as u32);
            }
            AndroidAppMessageKind::ButtonChunk => {
                trace.button_chunk_reads = trace.button_chunk_reads.saturating_add(1);
                trace.first_button_bytes = trace
                    .first_button_bytes
                    .saturating_add(message.payload().len() as u32);
            }
            AndroidAppMessageKind::Click => {
                trace.click_reads = trace.click_reads.saturating_add(1);
                #[cfg(feature = "androidbox-multiaction3")]
                {
                    let click_index = usize::try_from(trace.revision).unwrap_or(usize::MAX);
                    if click_index < ANDROID_APP_MULTIACTION_CALLBACKS {
                        trace.first_clicked_button_ids[click_index] = (message.arg1() >> 32) as u32;
                    } else {
                        trace.first_round_matches = false;
                    }
                }
            }
            AndroidAppMessageKind::Updated => {
                trace.updated_reads = trace.updated_reads.saturating_add(1);
                trace.first_revision = (message.arg1() >> 32) as u32;
                #[cfg(feature = "androidbox-scene-rpc2")]
                {
                    trace.first_update_view_id = message.arg0() as u32;
                }
                #[cfg(feature = "androidbox-multiaction3")]
                {
                    let revision = (message.arg1() >> 32) as u32;
                    let update_index =
                        usize::try_from(revision.saturating_sub(1)).unwrap_or(usize::MAX);
                    if update_index < ANDROID_APP_MULTIACTION_CALLBACKS {
                        trace.first_update_view_ids[update_index] = message.arg0() as u32;
                        #[cfg(feature = "androidbox-dex-methods8")]
                        {
                            trace.first_app_defined_call_counts[update_index] =
                                ((message.arg0() >> 32) & 0xff) as u8;
                        }
                        #[cfg(feature = "androidbox-dex-instance9")]
                        {
                            trace.first_app_defined_instance_call_counts[update_index] =
                                ((message.arg0() >> 40) & 0xff) as u8;
                        }
                        #[cfg(feature = "androidbox-activity-fields10")]
                        {
                            let field_byte = ((message.arg0() >> 48) & 0xff) as u8;
                            #[cfg(feature = "androidbox-string-builder13")]
                            {
                                trace.first_activity_field_read_counts[update_index] =
                                    field_byte & 0x3f;
                                trace.first_dynamic_string_texts[update_index] =
                                    field_byte & 0x40 != 0;
                                trace.first_direct_string_texts[update_index] =
                                    field_byte & 0x80 != 0;
                            }
                            #[cfg(all(
                                feature = "androidbox-string-text12",
                                not(feature = "androidbox-string-builder13")
                            ))]
                            {
                                trace.first_activity_field_read_counts[update_index] =
                                    field_byte & 0x7f;
                                trace.first_direct_string_texts[update_index] =
                                    field_byte & 0x80 != 0;
                            }
                            #[cfg(not(feature = "androidbox-string-text12"))]
                            {
                                trace.first_activity_field_read_counts[update_index] = field_byte;
                            }
                        }
                        #[cfg(feature = "androidbox-activity-state11")]
                        {
                            trace.first_activity_int_state_values[update_index] =
                                (message.arg0() >> 56) as u8;
                        }
                    } else {
                        trace.first_round_matches = false;
                    }
                }
            }
            AndroidAppMessageKind::UpdateTextChunk => {
                trace.update_text_chunk_reads = trace.update_text_chunk_reads.saturating_add(1);
                trace.first_update_bytes = trace
                    .first_update_bytes
                    .saturating_add(message.payload().len() as u32);
                #[cfg(feature = "androidbox-multiaction3")]
                {
                    let update_index =
                        usize::try_from(trace.revision.saturating_sub(1)).unwrap_or(usize::MAX);
                    if update_index < ANDROID_APP_MULTIACTION_CALLBACKS {
                        trace.first_update_text_bytes_by_click[update_index] = trace
                            .first_update_text_bytes_by_click[update_index]
                            .saturating_add(message.payload().len() as u32);
                    } else {
                        trace.first_round_matches = false;
                    }
                }
            }
            AndroidAppMessageKind::Close => {
                trace.close_reads = trace.close_reads.saturating_add(1);
            }
            AndroidAppMessageKind::Closed => {
                trace.closed_reads = trace.closed_reads.saturating_add(1);
            }
            AndroidAppMessageKind::Error => {
                trace.error_reads = trace.error_reads.saturating_add(1);
                trace.first_round_matches = false;
            }
            #[cfg(feature = "androidbox-restart0")]
            AndroidAppMessageKind::Crash => {
                trace.first_round_matches = false;
            }
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::SceneOpened => {
                trace.scene_opened_reads = trace.scene_opened_reads.saturating_add(1);
                trace.first_scene_node_count = message.arg0() as u8;
            }
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::DescribeNode => {
                trace.describe_node_reads = trace.describe_node_reads.saturating_add(1);
            }
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::Node => {
                trace.node_reads = trace.node_reads.saturating_add(1);
                trace.first_label_id = trace.label_id;
                trace.first_button_id = trace.button_id;
                trace.first_scene_text_view_count = trace.scene_text_view_count;
                trace.first_scene_callback_count = trace.scene_callback_count;
                #[cfg(feature = "androidbox-multiaction3")]
                {
                    trace.first_callback_button_ids.copy_from_slice(
                        &trace.scene_callback_ids[..ANDROID_APP_MULTIACTION_CALLBACKS],
                    );
                }
            }
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::NodeTextChunk => {
                trace.node_text_chunk_reads = trace.node_text_chunk_reads.saturating_add(1);
                trace.first_label_bytes = trace.label_received;
                trace.first_button_bytes = trace.button_received;
                trace.first_scene_text_bytes = trace.scene_text_bytes;
                #[cfg(feature = "androidbox-multiaction3")]
                {
                    trace.first_callback_button_text_bytes.copy_from_slice(
                        &trace.scene_callback_text_bytes[..ANDROID_APP_MULTIACTION_CALLBACKS],
                    );
                }
            }
        }
    } else {
        trace.post_complete_messages = trace.post_complete_messages.saturating_add(1);
    }

    if completed_round {
        trace.completed_rounds = trace.completed_rounds.saturating_add(1);
        if first_round {
            trace.first_round_finished = true;
            trace.complete = android_app_rpc_first_round_is_complete(trace);
        }
    }
}

#[cfg(all(
    feature = "androidbox-process0",
    not(feature = "androidbox-scene-rpc2")
))]
fn android_app_rpc_first_round_is_complete(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_round_matches
        && trace.first_round_errors == 0
        && trace.first_round_messages == 11
        && trace.requests == 3
        && trace.responses == 4
        && trace.chunks == 4
        && trace.app_to_android_reads == 3
        && trace.android_to_app_reads == 8
        && trace.ready_reads == 1
        && trace.open_reads == 1
        && trace.opened_reads == 1
        && trace.label_chunk_reads == 2
        && trace.button_chunk_reads == 1
        && trace.click_reads == 1
        && trace.updated_reads == 1
        && trace.update_text_chunk_reads == 1
        && trace.close_reads == 1
        && trace.closed_reads == 1
        && trace.error_reads == 0
        && trace.app_pid != 0
        && trace.android_app_pid != 0
        && trace.first_session_id != 0
        && trace.first_package_generation != 0
        && trace.first_label_id != 0
        && trace.first_button_id != 0
        && trace.first_label_id != trace.first_button_id
        && trace.first_revision == 1
        && trace.first_label_bytes == ANDROID_APP_FIRST_LABEL_BYTES
        && trace.first_button_bytes == ANDROID_APP_FIRST_BUTTON_BYTES
        && trace.first_update_bytes == ANDROID_APP_FIRST_UPDATE_BYTES
        && trace.first_last_request_id == ANDROID_APP_FINAL_REQUEST_ID
}

#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
fn android_app_rpc_first_round_is_complete(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_round_matches
        && trace.first_round_errors == 0
        && trace.first_round_messages == 21
        && trace.requests == 8
        && trace.responses == 9
        && trace.chunks == 4
        && trace.app_to_android_reads == 8
        && trace.android_to_app_reads == 13
        && trace.ready_reads == 1
        && trace.open_reads == 1
        && trace.opened_reads == 0
        && trace.label_chunk_reads == 0
        && trace.button_chunk_reads == 0
        && trace.scene_opened_reads == 1
        && trace.describe_node_reads == 5
        && trace.node_reads == 5
        && trace.node_text_chunk_reads == 3
        && trace.click_reads == 1
        && trace.updated_reads == 1
        && trace.update_text_chunk_reads == 1
        && trace.close_reads == 1
        && trace.closed_reads == 1
        && trace.error_reads == 0
        && trace.app_pid != 0
        && trace.android_app_pid != 0
        && trace.first_session_id != 0
        && trace.first_package_generation != 0
        && trace.first_label_id != 0
        && trace.first_button_id != 0
        && trace.first_label_id != trace.first_button_id
        && trace.first_revision == 1
        && trace.first_label_bytes == ANDROID_APP_FIRST_LABEL_BYTES
        && trace.first_button_bytes == ANDROID_APP_FIRST_BUTTON_BYTES
        && trace.first_update_bytes == ANDROID_APP_FIRST_UPDATE_BYTES
        && trace.first_scene_node_count == 5
        && trace.first_scene_text_bytes == 52
        && trace.first_scene_text_view_count == 2
        && trace.first_scene_callback_count == 1
        && trace.first_update_view_id != 0
        && trace.first_update_view_id != trace.first_label_id
        && trace.first_update_view_id != trace.first_button_id
        && trace.first_last_request_id == ANDROID_APP_FINAL_REQUEST_ID
}

#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-manifest-catalog3")
))]
fn android_app_rpc_first_round_is_complete(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_round_matches
        && trace.first_round_errors == 0
        && trace.first_round_messages == 25
        && trace.requests == 9
        && trace.responses == 10
        && trace.chunks == 6
        && trace.app_to_android_reads == 9
        && trace.android_to_app_reads == 16
        && trace.ready_reads == 1
        && trace.open_reads == 1
        && trace.opened_reads == 0
        && trace.label_chunk_reads == 0
        && trace.button_chunk_reads == 0
        && trace.scene_opened_reads == 1
        && trace.describe_node_reads == 5
        && trace.node_reads == 5
        && trace.node_text_chunk_reads == 4
        && trace.click_reads == 2
        && trace.updated_reads == 2
        && trace.update_text_chunk_reads == 2
        && trace.close_reads == 1
        && trace.closed_reads == 1
        && trace.error_reads == 0
        && trace.app_pid != 0
        && trace.android_app_pid != 0
        && trace.first_session_id != 0
        && trace.first_package_generation != 0
        && trace.first_label_id == ANDROID_APP_RESOURCE_ID_BASE + 3
        && trace.first_button_id == ANDROID_APP_RESOURCE_ID_BASE
        && trace.first_revision == 2
        && trace.first_label_bytes == ANDROID_APP_FIRST_LABEL_BYTES
        && trace.first_button_bytes == ANDROID_APP_FIRST_BUTTON_BYTES
        && trace.first_update_bytes == ANDROID_APP_FIRST_UPDATE_BYTES * 2
        && trace.first_scene_node_count == 5
        && trace.first_scene_text_bytes
            == if cfg!(feature = "androidbox-apk-envelope4") {
                45
            } else {
                44
            }
        && trace.first_scene_text_view_count == 2
        && trace.first_scene_callback_count == 2
        && trace.first_callback_button_ids
            == [
                ANDROID_APP_RESOURCE_ID_BASE,
                ANDROID_APP_RESOURCE_ID_BASE + 1,
            ]
        && trace.first_callback_button_text_bytes == [7, 6]
        && trace.first_clicked_button_ids == trace.first_callback_button_ids
        && trace.first_update_view_ids
            == [
                ANDROID_APP_RESOURCE_ID_BASE + 2,
                ANDROID_APP_RESOURCE_ID_BASE + 2,
            ]
        && trace.first_update_text_bytes_by_click == [18, 18]
        && android_app_rpc_app_defined_calls_match(trace)
        && trace.first_update_view_id == ANDROID_APP_RESOURCE_ID_BASE + 2
        && trace.first_last_request_id == ANDROID_APP_FINAL_REQUEST_ID
}

#[cfg(feature = "androidbox-manifest-catalog3")]
fn android_app_rpc_first_round_is_complete(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_round_matches
        && trace.first_round_errors == 0
        && trace.first_round_messages
            == if cfg!(feature = "androidbox-layout-row14") {
                27
            } else if cfg!(feature = "androidbox-string-builder13") {
                25
            } else {
                26
            }
        && trace.requests
            == if cfg!(feature = "androidbox-layout-row14") {
                10
            } else {
                9
            }
        && trace.responses
            == if cfg!(feature = "androidbox-layout-row14") {
                11
            } else {
                10
            }
        && trace.chunks
            == if cfg!(feature = "androidbox-string-builder13") {
                6
            } else {
                7
            }
        && trace.app_to_android_reads
            == if cfg!(feature = "androidbox-layout-row14") {
                10
            } else {
                9
            }
        && trace.android_to_app_reads
            == if cfg!(feature = "androidbox-layout-row14") {
                17
            } else if cfg!(feature = "androidbox-string-builder13") {
                16
            } else {
                17
            }
        && trace.ready_reads == 1
        && trace.open_reads == 1
        && trace.opened_reads == 0
        && trace.label_chunk_reads == 0
        && trace.button_chunk_reads == 0
        && trace.scene_opened_reads == 1
        && trace.describe_node_reads
            == if cfg!(feature = "androidbox-layout-row14") {
                6
            } else {
                5
            }
        && trace.node_reads
            == if cfg!(feature = "androidbox-layout-row14") {
                6
            } else {
                5
            }
        && trace.node_text_chunk_reads == 4
        && trace.click_reads == 2
        && trace.updated_reads == 2
        && trace.update_text_chunk_reads
            == if cfg!(feature = "androidbox-string-builder13") {
                2
            } else {
                3
            }
        && trace.close_reads == 1
        && trace.closed_reads == 1
        && trace.error_reads == 0
        && trace.app_pid != 0
        && trace.android_app_pid != 0
        && trace.first_session_id != 0
        && trace.first_package_generation != 0
        && trace.first_label_id == ANDROID_APP_RESOURCE_ID_BASE + 3
        && trace.first_button_id == ANDROID_APP_RESOURCE_ID_BASE
        && trace.first_revision == 2
        && trace.first_label_bytes == ANDROID_APP_FIRST_LABEL_BYTES
        && trace.first_button_bytes == ANDROID_APP_FIRST_BUTTON_BYTES
        && trace.first_update_bytes
            == if cfg!(feature = "androidbox-string-builder13") {
                30
            } else if cfg!(feature = "androidbox-string-text12") {
                48
            } else {
                50
            }
        && trace.first_scene_node_count
            == if cfg!(feature = "androidbox-layout-row14") {
                6
            } else {
                5
            }
        && trace.first_scene_text_bytes == 62
        && trace.first_scene_text_view_count == 2
        && trace.first_scene_callback_count == 2
        && trace.first_callback_button_ids
            == [
                ANDROID_APP_RESOURCE_ID_BASE,
                ANDROID_APP_RESOURCE_ID_BASE + 1,
            ]
        && trace.first_callback_button_text_bytes == [15, 16]
        && trace.first_clicked_button_ids == trace.first_callback_button_ids
        && trace.first_update_view_ids
            == [
                ANDROID_APP_RESOURCE_ID_BASE + 2,
                ANDROID_APP_RESOURCE_ID_BASE + 2,
            ]
        && {
            #[cfg(feature = "androidbox-string-builder13")]
            {
                trace.first_update_text_bytes_by_click == [15, 15]
            }
            #[cfg(all(
                feature = "androidbox-string-text12",
                not(feature = "androidbox-string-builder13")
            ))]
            {
                trace.first_update_text_bytes_by_click == [21, 27]
            }
            #[cfg(not(feature = "androidbox-string-text12"))]
            {
                trace.first_update_text_bytes_by_click == [24, 26]
            }
        }
        && android_app_rpc_app_defined_calls_match(trace)
        && trace.first_update_view_id == ANDROID_APP_RESOURCE_ID_BASE + 2
        && trace.first_last_request_id == ANDROID_APP_FINAL_REQUEST_ID
}

#[cfg(feature = "androidbox-string-text12")]
fn android_app_rpc_app_defined_calls_match(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_app_defined_call_counts == [0, 0]
        && android_app_rpc_instance_calls_match(trace)
        && android_app_rpc_activity_field_reads_match(trace)
}

#[cfg(all(
    feature = "androidbox-dex-methods8",
    not(feature = "androidbox-string-text12")
))]
fn android_app_rpc_app_defined_calls_match(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_app_defined_call_counts == [1, 1]
        && android_app_rpc_instance_calls_match(trace)
        && android_app_rpc_activity_field_reads_match(trace)
}

#[cfg(feature = "androidbox-string-text12")]
fn android_app_rpc_instance_calls_match(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_app_defined_instance_call_counts == [0, 0]
}

#[cfg(all(
    feature = "androidbox-dex-instance9",
    not(feature = "androidbox-string-text12")
))]
fn android_app_rpc_instance_calls_match(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_app_defined_instance_call_counts == [1, 1]
}

#[cfg(feature = "androidbox-string-text12")]
fn android_app_rpc_activity_field_reads_match(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_activity_field_read_counts == [2, 2]
        && trace.first_direct_string_texts == [true, true]
        && {
            #[cfg(feature = "androidbox-string-builder13")]
            {
                trace.first_dynamic_string_texts == [true, true]
            }
            #[cfg(not(feature = "androidbox-string-builder13"))]
            {
                true
            }
        }
        && trace.first_activity_int_state_values == [1, 2]
}

#[cfg(all(
    feature = "androidbox-activity-state11",
    not(feature = "androidbox-string-text12")
))]
fn android_app_rpc_activity_field_reads_match(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_activity_field_read_counts == [2, 2]
        && trace.first_activity_int_state_values == [1, 2]
}

#[cfg(all(
    feature = "androidbox-activity-fields10",
    not(feature = "androidbox-activity-state11")
))]
fn android_app_rpc_activity_field_reads_match(trace: &AndroidAppRpcTraceState) -> bool {
    trace.first_activity_field_read_counts == [1, 1]
}

#[cfg(all(
    feature = "androidbox-dex-methods8",
    not(feature = "androidbox-activity-fields10")
))]
const fn android_app_rpc_activity_field_reads_match(_trace: &AndroidAppRpcTraceState) -> bool {
    true
}

#[cfg(all(
    feature = "androidbox-dex-methods8",
    not(feature = "androidbox-dex-instance9")
))]
const fn android_app_rpc_instance_calls_match(_trace: &AndroidAppRpcTraceState) -> bool {
    true
}

#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-dex-methods8")
))]
const fn android_app_rpc_app_defined_calls_match(_trace: &AndroidAppRpcTraceState) -> bool {
    true
}

#[cfg(feature = "androidbox-process0")]
const fn android_app_rpc_kind_name(kind: AndroidAppMessageKind) -> &'static str {
    match kind {
        AndroidAppMessageKind::Ready => "ready",
        AndroidAppMessageKind::Open => "open",
        AndroidAppMessageKind::Opened => "opened",
        AndroidAppMessageKind::LabelChunk => "label-chunk",
        AndroidAppMessageKind::ButtonChunk => "button-chunk",
        AndroidAppMessageKind::Click => "click",
        AndroidAppMessageKind::Updated => "updated",
        AndroidAppMessageKind::UpdateTextChunk => "update-text-chunk",
        AndroidAppMessageKind::Close => "close",
        AndroidAppMessageKind::Closed => "closed",
        AndroidAppMessageKind::Error => "error",
        #[cfg(feature = "androidbox-restart0")]
        AndroidAppMessageKind::Crash => "crash",
        #[cfg(feature = "androidbox-scene-rpc2")]
        AndroidAppMessageKind::SceneOpened => "scene-opened",
        #[cfg(feature = "androidbox-scene-rpc2")]
        AndroidAppMessageKind::DescribeNode => "describe-node",
        #[cfg(feature = "androidbox-scene-rpc2")]
        AndroidAppMessageKind::Node => "node",
        #[cfg(feature = "androidbox-scene-rpc2")]
        AndroidAppMessageKind::NodeTextChunk => "node-text-chunk",
    }
}

#[cfg(feature = "surface-trace-evidence")]
fn trace_ui_channel_read(result: &EnvelopeReadResult) {
    if !matches!(
        result.kind,
        ChannelMessageKind::Bytes | ChannelMessageKind::Transfer
    ) || result.logical_length != CHANNEL_MESSAGE_MAX_BYTES
    {
        return;
    }
    let Some(wire) = result.trace_bytes.as_ref() else {
        return;
    };
    let Some(receiver_pid) = crate::process::current_live_user_process_id() else {
        return;
    };
    let Some(sender_role) = unique_ui_trace_role(result.sender_pid) else {
        return;
    };
    let Some(receiver_role) = unique_ui_trace_role(receiver_pid) else {
        return;
    };
    let Some(trace) = decode_authenticated_ui_channel_read(sender_role, receiver_role, wire) else {
        #[cfg(feature = "text-input-runtime")]
        if MULTI_WINDOW_RUNTIME_ACTIVE && (wire.starts_with(b"BTI1") || wire.starts_with(b"BTE1")) {
            let owner_error = if wire.starts_with(b"BTI1") {
                sender_role != UiTraceRole::App || receiver_role != UiTraceRole::SurfaceServer
            } else {
                sender_role != UiTraceRole::SurfaceServer || receiver_role != UiTraceRole::App
            };
            #[cfg(feature = "soft-keyboard-runtime")]
            let accepted = if bndroid_kernel::text_input_trace::snapshot().final_complete {
                bndroid_kernel::soft_keyboard_trace::record_invalid_wire(owner_error)
            } else {
                bndroid_kernel::text_input_trace::record_invalid_wire(owner_error)
            };
            #[cfg(not(feature = "soft-keyboard-runtime"))]
            let accepted = bndroid_kernel::text_input_trace::record_invalid_wire(owner_error);
            log_text_input_trace_step(accepted);
        }
        #[cfg(feature = "multi-window-runtime")]
        if MULTI_WINDOW_RUNTIME_ACTIVE && (wire.starts_with(b"BWC1") || wire.starts_with(b"BWE1")) {
            let owner_error = if wire.starts_with(b"BWC1") {
                !matches!(sender_role, UiTraceRole::Launcher | UiTraceRole::App)
                    || receiver_role != UiTraceRole::SurfaceServer
            } else {
                sender_role != UiTraceRole::SurfaceServer
                    || !matches!(receiver_role, UiTraceRole::Launcher | UiTraceRole::App)
            };
            #[cfg(feature = "persistent-window-runtime")]
            let accepted =
                bndroid_kernel::persistent_window_trace::record_invalid_wire(owner_error);
            #[cfg(not(feature = "persistent-window-runtime"))]
            let accepted = bndroid_kernel::window_trace::record_invalid_wire(owner_error);
            log_window_trace_step(accepted);
        }
        return;
    };
    let trace = match trace {
        AuthenticatedUiChannelTrace::Legacy(trace) => trace,
        AuthenticatedUiChannelTrace::Window(trace) => {
            #[cfg(feature = "multi-window-runtime")]
            if MULTI_WINDOW_RUNTIME_ACTIVE {
                #[cfg(feature = "post-recovery-interaction-runtime")]
                if bndroid_kernel::service_dependency_trace::snapshot().complete {
                    // M41/M42 remain immutable prefixes. The generic channel
                    // hook above already authenticated this M50 BWC1/BWE1
                    // frame for the dedicated post-recovery trace.
                    return;
                }
                #[cfg(feature = "persistent-window-runtime")]
                let accepted = bndroid_kernel::persistent_window_trace::record(trace);
                #[cfg(not(feature = "persistent-window-runtime"))]
                let accepted = bndroid_kernel::window_trace::record(trace);
                log_window_trace_step(accepted);
            }
            #[cfg(not(feature = "multi-window-runtime"))]
            let _ = trace;
            return;
        }
        AuthenticatedUiChannelTrace::Text(trace) => {
            #[cfg(feature = "text-input-runtime")]
            if MULTI_WINDOW_RUNTIME_ACTIVE {
                #[cfg(feature = "soft-keyboard-runtime")]
                let accepted = if bndroid_kernel::text_input_trace::snapshot().final_complete {
                    bndroid_kernel::soft_keyboard_trace::record(trace)
                } else {
                    bndroid_kernel::text_input_trace::record(trace)
                };
                #[cfg(not(feature = "soft-keyboard-runtime"))]
                let accepted = bndroid_kernel::text_input_trace::record(trace);
                log_text_input_trace_step(accepted);
            }
            #[cfg(not(feature = "text-input-runtime"))]
            let _ = trace;
            return;
        }
    };

    match trace {
        UiChannelTrace::Input { session_id, sample } => crate::kprintln!(
            "UI_ROUTE_INPUT_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} session={} sequence={} x={} y={} pressed={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            session_id,
            sample.sequence(),
            sample.x(),
            sample.y(),
            u8::from(sample.pressed()),
        ),
        UiChannelTrace::FocusChanged {
            session_id,
            active_client,
            app,
            focus_generation,
        } => crate::kprintln!(
            "UI_ROUTE_FOCUS_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} session={} active_client={} app={} focus_generation={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            session_id,
            ui_client_name(active_client),
            ui_app_name(app),
            focus_generation,
        ),
        UiChannelTrace::PresentRead {
            frame_id,
            focus_generation,
            mode,
        } => crate::kprintln!(
            "UI_ROUTE_PRESENT_READ_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} frame_id={} focus_generation={} mode={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            frame_id,
            focus_generation,
            present_mode_name(mode),
        ),
        UiChannelTrace::PresentCancelled {
            session_id,
            frame_id,
            focus_generation,
        } => crate::kprintln!(
            "UI_ROUTE_PRESENT_CANCELLED_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} session={} frame_id={} focus_generation={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            session_id,
            frame_id,
            focus_generation,
        ),
        UiChannelTrace::Presented {
            session_id,
            frame_id,
            commit,
        } => crate::kprintln!(
            "UI_ROUTE_PRESENTED_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} session={} frame_id={} commit={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            session_id,
            frame_id,
            commit,
        ),
        UiChannelTrace::AppearanceRequest { action, request_id } => crate::kprintln!(
            "UI_APPEARANCE_REQUEST_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} action={} request_id={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            ui_appearance_action_name(action),
            request_id,
        ),
        UiChannelTrace::AppearanceChanged {
            session_id,
            appearance,
            revision,
        } => crate::kprintln!(
            "UI_APPEARANCE_CHANGED_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} session={} revision={} theme={} accent={} software_dimming={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            session_id,
            revision,
            if appearance.dark_theme() {
                "dark"
            } else {
                "light"
            },
            if appearance.alternate_accent() {
                "violet"
            } else {
                "ocean"
            },
            ui_software_dimming_percent(appearance.software_dimming()),
        ),
        UiChannelTrace::BootNotificationDismissRequest { request_id } => crate::kprintln!(
            "UI_BOOT_NOTIFICATION_DISMISS_REQUEST_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} request_id={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            request_id,
        ),
        UiChannelTrace::BootNotificationChanged {
            session_id,
            visible,
            revision,
        } => crate::kprintln!(
            "UI_BOOT_NOTIFICATION_CHANGED_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} session={} visible={} revision={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            session_id,
            u8::from(visible),
            revision,
        ),
        UiChannelTrace::SystemUiRequest {
            action,
            recent,
            request_id,
            observed_revision,
        } => crate::kprintln!(
            "UI_SYSTEM_UI_REQUEST_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} action={} app={} request_id={} observed_revision={} recent_kind={} compatible_session={} package_generation={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            ui_system_ui_action_name(action),
            ui_recent_name(recent),
            request_id,
            observed_revision,
            ui_recent_kind_name(recent),
            ui_recent_compatible_session(recent),
            ui_recent_package_generation(recent),
        ),
        UiChannelTrace::SystemUiChanged {
            session_id,
            mode,
            recent,
            nav_pressed,
            nav_reveal_px,
            revision,
        } => crate::kprintln!(
            "UI_SYSTEM_UI_CHANGED_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} session={} mode={} recent_app={} nav_pressed={} nav_reveal_px={} revision={} recent_kind={} compatible_session={} package_generation={} activity_pixels=0 thumbnail=0 live_preview=0 background_execution=0",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            session_id,
            ui_system_ui_mode_name(mode),
            ui_recent_name(recent),
            u8::from(nav_pressed),
            nav_reveal_px,
            revision,
            ui_recent_kind_name(recent),
            ui_recent_compatible_session(recent),
            ui_recent_package_generation(recent),
        ),
        UiChannelTrace::SystemUiRequestCompleted {
            session_id,
            action,
            status,
            request_id,
            revision,
            compatible_identity,
            reservation_origin,
        } => crate::kprintln!(
            "UI_SYSTEM_UI_REQUEST_COMPLETED_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} session={} action={} status={} request_id={} revision={} recent_kind={} compatible_session={} package_generation={} reservation_origin={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            session_id,
            ui_system_ui_action_name(action),
            ui_system_ui_request_status_name(status),
            request_id,
            revision,
            ui_compatible_identity_kind_name(compatible_identity),
            ui_compatible_identity_session(compatible_identity),
            ui_compatible_identity_package_generation(compatible_identity),
            ui_compatible_activity_reservation_origin_name(reservation_origin),
        ),
        UiChannelTrace::AndroidBoxDexExecuted {
            request_id,
            kind,
            dex_crc32,
            dex_adler32,
            result: execution_result,
            instruction_count,
            tap_count,
        } => crate::kprintln!(
            "ANDROIDBOX_DEX_EXECUTED_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} request={} kind={} crc={} adler={} result={} instructions={} tap_count={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            request_id,
            ui_androidbox_execution_kind_name(kind),
            dex_crc32,
            dex_adler32,
            execution_result,
            instruction_count,
            tap_count,
        ),
        UiChannelTrace::AndroidBoxActivityExecuted {
            request_id,
            lifecycle,
            manifest_crc32,
            dex_crc32,
            dex_adler32,
            activity_descriptor_crc32,
            on_create_method_index,
            on_create_code_offset,
            instruction_count,
            view_text_label,
            view_text_length,
        } => crate::kprintln!(
            "ANDROIDBOX_ACTIVITY_EXECUTED_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} request={} lifecycle={} manifest_verified=1 manifest_crc={} dex_crc={} dex_adler={} activity=org.bndroid.demo.MainActivity activity_descriptor_crc={} on_create_method={} on_create_code_offset={} instructions={} view_text_id=resource-string view_text_label={} view_text_length={} activitythread=0 framework=0 general_apk_claim=0 android_compatibility_claim=0",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            request_id,
            ui_androidbox_activity_lifecycle_name(lifecycle),
            manifest_crc32,
            dex_crc32,
            dex_adler32,
            activity_descriptor_crc32,
            on_create_method_index,
            on_create_code_offset,
            instruction_count,
            view_text_label,
            view_text_length,
        ),
        UiChannelTrace::AndroidBoxResourcesResolved {
            request_id,
            resources_arsc_crc32,
            layout_xml_crc32,
            layout_resource_id,
            string_resource_id,
            view_text_label,
            view_text_length,
            success_flags,
        } => crate::kprintln!(
            "ANDROIDBOX_RESOURCES_RESOLVED_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} request={} resources_arsc_crc={} layout_xml_crc={} layout_resource_id={} string_resource_id={} view_text_id=resource-string view_text_label={} view_text_length={} success_flags={} table_parsed=1 layout_entry_resolved=1 binary_xml_parsed=1 text_view_verified=1 string_reference_resolved=1 resource_manager=0 arbitrary_layout_claim=0 general_apk_claim=0 android_compatibility_claim=0",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            request_id,
            resources_arsc_crc32,
            layout_xml_crc32,
            layout_resource_id,
            string_resource_id,
            view_text_label,
            view_text_length,
            success_flags,
        ),
        UiChannelTrace::ClockChanged {
            session_id,
            unix_seconds,
            revision,
        } => crate::kprintln!(
            "UI_CLOCK_CHANGED_OK sender_image={} sender_pid={} receiver_image={} receiver_pid={} session={} unix_seconds={} revision={}",
            sender_role.image_name(),
            result.sender_pid,
            receiver_role.image_name(),
            receiver_pid,
            session_id,
            unix_seconds,
            revision,
        ),
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_androidbox_execution_kind_name(
    kind: bndr_ui::UiAndroidBoxExecutionKind,
) -> &'static str {
    match kind {
        bndr_ui::UiAndroidBoxExecutionKind::Boot => "boot",
        bndr_ui::UiAndroidBoxExecutionKind::Tap => "tap",
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_androidbox_activity_lifecycle_name(
    lifecycle: bndr_ui::UiAndroidBoxActivityLifecycleKind,
) -> &'static str {
    match lifecycle {
        bndr_ui::UiAndroidBoxActivityLifecycleKind::OnCreateComplete => "on-create-complete",
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_appearance_action_name(action: bndr_ui::UiAppearanceAction) -> &'static str {
    match action {
        bndr_ui::UiAppearanceAction::ToggleTheme => "toggle-theme",
        bndr_ui::UiAppearanceAction::ToggleAccent => "toggle-accent",
        bndr_ui::UiAppearanceAction::SetSoftwareDimmingOff => "set-software-dimming-100",
        bndr_ui::UiAppearanceAction::SetSoftwareDimmingLight => "set-software-dimming-80",
        bndr_ui::UiAppearanceAction::SetSoftwareDimmingMedium => "set-software-dimming-60",
        bndr_ui::UiAppearanceAction::SetSoftwareDimmingStrong => "set-software-dimming-40",
        bndr_ui::UiAppearanceAction::SetSoftwareDimmingMaximum => "set-software-dimming-20",
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_system_ui_action_name(action: bndr_ui::UiSystemUiAction) -> &'static str {
    match action {
        bndr_ui::UiSystemUiAction::Unlock => "unlock",
        bndr_ui::UiSystemUiAction::CloseOverview => "close-overview",
        bndr_ui::UiSystemUiAction::ActivateRecent => "activate-recent",
        bndr_ui::UiSystemUiAction::PresentCompatibleActivity => "present-compatible",
        bndr_ui::UiSystemUiAction::HomeCompatibleActivity => "home-compatible",
        bndr_ui::UiSystemUiAction::FinishCompatibleActivity => "finish-compatible",
        bndr_ui::UiSystemUiAction::ReserveCompatibleActivity => "reserve-compatible",
        bndr_ui::UiSystemUiAction::AbortCompatibleActivityVerification => "abort-compatible",
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_system_ui_request_status_name(
    status: bndr_ui::UiSystemUiRequestStatus,
) -> &'static str {
    match status {
        bndr_ui::UiSystemUiRequestStatus::Accepted => "accepted",
        bndr_ui::UiSystemUiRequestStatus::Conflict => "conflict",
        bndr_ui::UiSystemUiRequestStatus::CaptureBusy => "capture-busy",
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_compatible_activity_reservation_origin_name(
    origin: Option<bndr_ui::UiCompatibleActivityReservationOrigin>,
) -> &'static str {
    match origin {
        None => "none",
        Some(bndr_ui::UiCompatibleActivityReservationOrigin::Home) => "home",
        Some(bndr_ui::UiCompatibleActivityReservationOrigin::Overview) => "overview",
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_system_ui_mode_name(mode: bndr_ui::UiSystemUiMode) -> &'static str {
    match mode {
        bndr_ui::UiSystemUiMode::Locked => "locked",
        bndr_ui::UiSystemUiMode::Home => "home",
        bndr_ui::UiSystemUiMode::Foreground => "foreground",
        bndr_ui::UiSystemUiMode::Overview => "overview",
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_software_dimming_percent(software_dimming: bndr_ui::UiSoftwareDimming) -> &'static str {
    match software_dimming {
        bndr_ui::UiSoftwareDimming::Off => "100",
        bndr_ui::UiSoftwareDimming::Light => "80",
        bndr_ui::UiSoftwareDimming::Medium => "60",
        bndr_ui::UiSoftwareDimming::Strong => "40",
        bndr_ui::UiSoftwareDimming::Maximum => "20",
    }
}

#[cfg(all(test, feature = "surface-trace-evidence"))]
mod ui_appearance_action_name_tests {
    use super::{
        ui_androidbox_activity_lifecycle_name, ui_androidbox_execution_kind_name,
        ui_appearance_action_name, ui_compatible_activity_reservation_origin_name,
        ui_compatible_identity_kind_name, ui_compatible_identity_package_generation,
        ui_compatible_identity_session, ui_software_dimming_percent, ui_system_ui_action_name,
        ui_system_ui_mode_name, ui_system_ui_request_status_name,
    };
    use bndr_ui::{
        UiAndroidBoxActivityLifecycleKind, UiAndroidBoxExecutionKind, UiAppearanceAction,
        UiCompatibleActivityIdentity, UiCompatibleActivityReservationOrigin, UiSoftwareDimming,
        UiSystemUiAction, UiSystemUiMode, UiSystemUiRequestStatus,
    };

    #[test]
    fn software_dimming_action_names_are_exact_and_unambiguous() {
        for (action, expected) in [
            (
                UiAppearanceAction::SetSoftwareDimmingOff,
                "set-software-dimming-100",
            ),
            (
                UiAppearanceAction::SetSoftwareDimmingLight,
                "set-software-dimming-80",
            ),
            (
                UiAppearanceAction::SetSoftwareDimmingMedium,
                "set-software-dimming-60",
            ),
            (
                UiAppearanceAction::SetSoftwareDimmingStrong,
                "set-software-dimming-40",
            ),
            (
                UiAppearanceAction::SetSoftwareDimmingMaximum,
                "set-software-dimming-20",
            ),
        ] {
            assert_eq!(ui_appearance_action_name(action), expected);
        }
    }

    #[test]
    fn software_dimming_state_percentages_match_the_action_names() {
        for (software_dimming, expected) in [
            (UiSoftwareDimming::Off, "100"),
            (UiSoftwareDimming::Light, "80"),
            (UiSoftwareDimming::Medium, "60"),
            (UiSoftwareDimming::Strong, "40"),
            (UiSoftwareDimming::Maximum, "20"),
        ] {
            assert_eq!(ui_software_dimming_percent(software_dimming), expected);
            let action = UiAppearanceAction::set_software_dimming(software_dimming);
            assert_eq!(
                ui_appearance_action_name(action),
                match expected {
                    "100" => "set-software-dimming-100",
                    "80" => "set-software-dimming-80",
                    "60" => "set-software-dimming-60",
                    "40" => "set-software-dimming-40",
                    "20" => "set-software-dimming-20",
                    _ => unreachable!(),
                }
            );
        }
    }

    #[test]
    fn system_ui_trace_names_cover_the_closed_action_and_mode_domains() {
        for (action, expected) in [
            (UiSystemUiAction::Unlock, "unlock"),
            (UiSystemUiAction::CloseOverview, "close-overview"),
            (UiSystemUiAction::ActivateRecent, "activate-recent"),
            (
                UiSystemUiAction::PresentCompatibleActivity,
                "present-compatible",
            ),
            (UiSystemUiAction::HomeCompatibleActivity, "home-compatible"),
            (
                UiSystemUiAction::FinishCompatibleActivity,
                "finish-compatible",
            ),
            (
                UiSystemUiAction::ReserveCompatibleActivity,
                "reserve-compatible",
            ),
            (
                UiSystemUiAction::AbortCompatibleActivityVerification,
                "abort-compatible",
            ),
        ] {
            assert_eq!(ui_system_ui_action_name(action), expected);
        }
        for (mode, expected) in [
            (UiSystemUiMode::Locked, "locked"),
            (UiSystemUiMode::Home, "home"),
            (UiSystemUiMode::Foreground, "foreground"),
            (UiSystemUiMode::Overview, "overview"),
        ] {
            assert_eq!(ui_system_ui_mode_name(mode), expected);
        }
    }

    #[test]
    fn system_ui_completion_trace_names_and_identity_fields_are_exact() {
        for (status, expected) in [
            (UiSystemUiRequestStatus::Accepted, "accepted"),
            (UiSystemUiRequestStatus::Conflict, "conflict"),
            (UiSystemUiRequestStatus::CaptureBusy, "capture-busy"),
        ] {
            assert_eq!(ui_system_ui_request_status_name(status), expected);
        }
        for (origin, expected) in [
            (None, "none"),
            (Some(UiCompatibleActivityReservationOrigin::Home), "home"),
            (
                Some(UiCompatibleActivityReservationOrigin::Overview),
                "overview",
            ),
        ] {
            assert_eq!(
                ui_compatible_activity_reservation_origin_name(origin),
                expected
            );
        }

        let identity = UiCompatibleActivityIdentity::new(17, 23).unwrap();
        assert_eq!(ui_compatible_identity_kind_name(None), "none");
        assert_eq!(ui_compatible_identity_session(None), 0);
        assert_eq!(ui_compatible_identity_package_generation(None), 0);
        assert_eq!(
            ui_compatible_identity_kind_name(Some(identity)),
            "compatible-activity"
        );
        assert_eq!(ui_compatible_identity_session(Some(identity)), 17);
        assert_eq!(
            ui_compatible_identity_package_generation(Some(identity)),
            23
        );
    }

    #[test]
    fn androidbox_execution_kind_names_are_stable() {
        assert_eq!(
            ui_androidbox_execution_kind_name(UiAndroidBoxExecutionKind::Boot),
            "boot"
        );
        assert_eq!(
            ui_androidbox_execution_kind_name(UiAndroidBoxExecutionKind::Tap),
            "tap"
        );
        assert_eq!(
            ui_androidbox_activity_lifecycle_name(
                UiAndroidBoxActivityLifecycleKind::OnCreateComplete
            ),
            "on-create-complete"
        );
    }
}

#[cfg(all(feature = "surface-trace-evidence", feature = "text-input-runtime"))]
fn log_text_input_trace_step(accepted: bool) {
    #[cfg(feature = "soft-keyboard-runtime")]
    if bndroid_kernel::text_input_trace::snapshot().final_complete {
        let state = bndroid_kernel::soft_keyboard_trace::snapshot();
        crate::kprintln!(
            "SOFT_KEYBOARD_TRACE_STEP_OK accepted={} messages={} commands={} events={} output={} editor={} shows={} hides={} errors={} phase={:?}",
            u8::from(accepted),
            state.messages,
            state.commands,
            state.events,
            state.output_commits,
            state.editor_outputs,
            state.overlay_shows,
            state.overlay_hides,
            state.errors,
            state.phase,
        );
        return;
    }
    let state = bndroid_kernel::text_input_trace::snapshot();
    crate::kprintln!(
        "TEXT_TRACE_STEP_OK accepted={} messages={} commands={} events={} renders={} output={} errors={} phase={:?}",
        u8::from(accepted),
        state.messages,
        state.commands,
        state.events,
        state.rendered_events,
        state.output_commits,
        state.errors,
        state.phase,
    );
}

#[cfg(feature = "multi-window-runtime")]
fn log_window_trace_step(accepted: bool) {
    let snapshot = bndroid_kernel::window_trace::snapshot();
    #[cfg(feature = "persistent-window-runtime")]
    let extension = bndroid_kernel::persistent_window_trace::snapshot();
    #[cfg(feature = "persistent-window-runtime")]
    crate::kprintln!(
        "WINDOW_TRACE_STEP_OK accepted={} traces={}/{} commands={}/{} events={}/{} inputs={}/{} output={}/{} errors={}/{} phase={:?}/{:?}",
        u8::from(accepted),
        snapshot.traces,
        extension.traces,
        snapshot.commands,
        extension.commands,
        snapshot.events,
        extension.events,
        snapshot.input_route_count,
        extension.input_events,
        snapshot.output_commits,
        extension.output_commits,
        snapshot.errors,
        extension.errors,
        snapshot.phase,
        extension.phase,
    );
    #[cfg(not(feature = "persistent-window-runtime"))]
    crate::kprintln!(
        "WINDOW_TRACE_STEP_OK accepted={} traces={} commands={} events={} inputs={} output={} errors={} phase={:?}",
        u8::from(accepted),
        snapshot.traces,
        snapshot.commands,
        snapshot.events,
        snapshot.input_route_count,
        snapshot.output_commits,
        snapshot.errors,
        snapshot.phase,
    );
}

#[cfg(feature = "surface-trace-evidence")]
fn unique_ui_trace_role(process_id: u64) -> Option<UiTraceRole> {
    [
        (UserImageId::SurfaceServer, UiTraceRole::SurfaceServer),
        (UserImageId::Launcher, UiTraceRole::Launcher),
        (UserImageId::App, UiTraceRole::App),
    ]
    .into_iter()
    .find_map(|(image_id, role)| {
        (crate::process::unique_live_process_id_for_image(image_id) == Some(process_id))
            .then_some(role)
    })
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_client_name(client: UiClientId) -> &'static str {
    match client {
        UiClientId::Launcher => "launcher",
        UiClientId::App => "app",
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_app_name(app: Option<ShellAppId>) -> &'static str {
    match app {
        None => "none",
        Some(ShellAppId::Phone) => "phone",
        Some(ShellAppId::Messages) => "messages",
        Some(ShellAppId::Settings) => "settings",
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_recent_name(recent: Option<bndr_ui::UiRecentIdentity>) -> &'static str {
    match recent {
        None => "none",
        Some(bndr_ui::UiRecentIdentity::Shell(app)) => ui_app_name(Some(app)),
        Some(bndr_ui::UiRecentIdentity::CompatibleAndroid(_)) => "android-compatible",
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_recent_kind_name(recent: Option<bndr_ui::UiRecentIdentity>) -> &'static str {
    match recent {
        None => "none",
        Some(bndr_ui::UiRecentIdentity::Shell(_)) => "shell",
        Some(bndr_ui::UiRecentIdentity::CompatibleAndroid(_)) => "compatible-activity",
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_recent_compatible_session(recent: Option<bndr_ui::UiRecentIdentity>) -> u64 {
    match recent {
        Some(bndr_ui::UiRecentIdentity::CompatibleAndroid(identity)) => identity.session_id(),
        None | Some(bndr_ui::UiRecentIdentity::Shell(_)) => 0,
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_recent_package_generation(recent: Option<bndr_ui::UiRecentIdentity>) -> u64 {
    match recent {
        Some(bndr_ui::UiRecentIdentity::CompatibleAndroid(identity)) => {
            identity.package_generation()
        }
        None | Some(bndr_ui::UiRecentIdentity::Shell(_)) => 0,
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_compatible_identity_kind_name(
    identity: Option<bndr_ui::UiCompatibleActivityIdentity>,
) -> &'static str {
    match identity {
        None => "none",
        Some(_) => "compatible-activity",
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_compatible_identity_session(
    identity: Option<bndr_ui::UiCompatibleActivityIdentity>,
) -> u64 {
    match identity {
        None => 0,
        Some(identity) => identity.session_id(),
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn ui_compatible_identity_package_generation(
    identity: Option<bndr_ui::UiCompatibleActivityIdentity>,
) -> u64 {
    match identity {
        None => 0,
        Some(identity) => identity.package_generation(),
    }
}

#[cfg(feature = "surface-trace-evidence")]
const fn present_mode_name(mode: PresentMode) -> &'static str {
    match mode {
        PresentMode::Full => "full",
        PresentMode::Damage => "damage",
    }
}

/// Records only init-observed commits on its private child-control channels.
/// The trusted, digest-attested client emits an ACK only after validating the
/// matching lookup reply and direct provider echo; manager/provider DONE
/// messages independently commit the same two-round service prefix.
fn record_m14_service_transcript(handle: HandleValue, sender_pid: u64, tag: u64, payload: u64) {
    if !crate::process::current_is_init() {
        return;
    }
    let (reads, bitmap, commit) = match tag {
        M14_SERVING_ACK_TAG => (
            &SERVICE_ACK_READS,
            &SERVICE_ACK_BITMAP,
            match payload {
                M14_ROUND_ONE_ACK => Some((1_u64 << 0, UserImageId::Client)),
                M14_ROUND_TWO_ACK => Some((1_u64 << 1, UserImageId::Client)),
                _ => None,
            },
        ),
        M14_SERVING_DONE_TAG => (
            &SERVICE_DONE_READS,
            &SERVICE_DONE_BITMAP,
            match payload {
                M14_MANAGER_DONE => Some((1_u64 << 0, UserImageId::ServiceManager)),
                M14_PROVIDER_DONE => Some((1_u64 << 1, UserImageId::Provider)),
                _ => None,
            },
        ),
        _ => return,
    };
    reads.fetch_add(1, Ordering::Relaxed);
    let Some((bit, image_id)) = commit else {
        SERVICE_TRANSCRIPT_ERRORS.fetch_add(1, Ordering::Relaxed);
        return;
    };
    if !authenticated_startup_sender(handle, sender_pid, image_id) {
        SERVICE_TRANSCRIPT_ERRORS.fetch_add(1, Ordering::Relaxed);
        return;
    }
    if bitmap.fetch_or(bit, Ordering::Relaxed) & bit != 0 {
        SERVICE_TRANSCRIPT_ERRORS.fetch_add(1, Ordering::Relaxed);
    }
}

/// Records the M15 service lifecycle without decoding a BSM1 frame. Init can
/// commit a phase only by consuming it from the generation-qualified startup
/// peer of the expected resident image. The opaque transaction/instance values
/// are checked relationally, so kernel acceptance is not tied to fixed IDs.
fn record_m15_service_transcript(handle: HandleValue, sender_pid: u64, tag: u64, payload: u64) {
    if !crate::process::current_is_init() {
        return;
    }
    match tag {
        M15_SERVICE_COMMIT_TAG => record_m15_service_commit(handle, sender_pid, payload),
        M15_SERVICE_DONE_TAG => record_m15_service_done(handle, sender_pid, payload),
        _ => {}
    }
}

fn record_m15_service_commit(handle: HandleValue, sender_pid: u64, payload: u64) {
    if !INIT_READY.load(Ordering::Acquire) {
        record_m15_service_error();
        return;
    }

    let phase = (payload >> 56) as u8;
    let transaction_id = ((payload >> 32) & 0x00ff_ffff) as u32;
    let instance = payload as u32;
    let current_phase = M15_SERVICE_PHASE.load(Ordering::Acquire) as u8;
    if phase != current_phase.saturating_add(1) || phase > M15_SERVICE_PHASE_COUNT {
        record_m15_service_error();
        return;
    }

    let expected_image = match phase {
        1 | 3 | 4 | 6 | 7 | 9 | 10 => UserImageId::Provider,
        2 | 5 | 8 => UserImageId::Client,
        _ => {
            record_m15_service_error();
            return;
        }
    };
    if !authenticated_startup_sender(handle, sender_pid, expected_image) {
        record_m15_service_error();
        return;
    }

    let old_instance = M15_SERVICE_OLD_INSTANCE.load(Ordering::Acquire) as u32;
    let new_instance = M15_SERVICE_NEW_INSTANCE.load(Ordering::Acquire) as u32;
    let valid_values = match phase {
        1 => instance != 0,
        2 => old_instance != 0 && instance == old_instance,
        3 => old_instance != 0 && instance == old_instance && transaction_id == m15_service_txid(1),
        4 => old_instance != 0 && instance == old_instance,
        5 => old_instance != 0 && instance == 0,
        6 => old_instance != 0 && instance > old_instance,
        7 => new_instance > old_instance && instance == old_instance,
        8 => new_instance > old_instance && instance == new_instance,
        9 => {
            new_instance > old_instance
                && instance == new_instance
                && transaction_id == m15_service_txid(6)
        }
        10 => new_instance > old_instance && instance == new_instance,
        _ => false,
    };
    if !valid_values {
        record_m15_service_error();
        return;
    }

    let transaction_slot = match phase {
        1 => Some(0),
        2 => Some(1),
        3 => None,
        4 => Some(2),
        5 => Some(3),
        6 => Some(4),
        7 => Some(5),
        8 => Some(6),
        9 => None,
        10 => Some(7),
        _ => None,
    };
    if transaction_slot.is_some() && !m15_service_txid_is_new(transaction_id) {
        record_m15_service_error();
        return;
    }

    if phase == 1 {
        M15_SERVICE_OLD_INSTANCE.store(u64::from(instance), Ordering::Relaxed);
    } else if phase == 6 {
        M15_SERVICE_NEW_INSTANCE.store(u64::from(instance), Ordering::Relaxed);
    }
    if let Some(slot) = transaction_slot {
        M15_SERVICE_TXIDS[slot].store(u64::from(transaction_id), Ordering::Relaxed);
    }
    M15_SERVICE_PHASE.store(u64::from(phase), Ordering::Release);
}

fn record_m15_service_done(handle: HandleValue, sender_pid: u64, payload: u64) {
    M15_SERVICE_DONE_READS.fetch_add(1, Ordering::Relaxed);
    if !INIT_READY.load(Ordering::Acquire)
        || M15_SERVICE_PHASE.load(Ordering::Acquire) != u64::from(M15_SERVICE_PHASE_COUNT)
    {
        record_m15_service_error();
        return;
    }

    let role = payload as u32;
    if payload >> 32 != u64::from(M15_SERVICE_PHASE_COUNT) {
        record_m15_service_error();
        return;
    }
    let (bit, image_id) = match role {
        1 => (1_u64 << 0, UserImageId::Provider),
        2 => (1_u64 << 1, UserImageId::Client),
        3 => (1_u64 << 2, UserImageId::ServiceManager),
        _ => {
            record_m15_service_error();
            return;
        }
    };
    if !authenticated_startup_sender(handle, sender_pid, image_id)
        || M15_SERVICE_DONE_BITMAP.load(Ordering::Acquire) & bit != 0
    {
        record_m15_service_error();
        return;
    }
    M15_SERVICE_DONE_BITMAP.fetch_or(bit, Ordering::Release);
}

fn m15_service_txid(index: usize) -> u32 {
    M15_SERVICE_TXIDS[index].load(Ordering::Acquire) as u32
}

fn m15_service_txid_is_new(transaction_id: u32) -> bool {
    transaction_id != 0
        && M15_SERVICE_TXIDS
            .iter()
            .all(|stored| stored.load(Ordering::Acquire) as u32 != transaction_id)
}

fn record_m15_service_error() {
    M15_SERVICE_ERRORS.fetch_add(1, Ordering::Relaxed);
}

/// Adapts authenticated init reads into the pure, allocation-free service
/// cycle reducer. The message's immutable kernel sender stamp and the startup
/// endpoint must independently identify the same unique live resident.
fn record_service_cycle_transcript(handle: HandleValue, sender_pid: u64, tag: u64, payload: u64) {
    use bndroid_kernel::service_cycle::{Event, Gate, Role};

    if !crate::process::current_is_init() {
        return;
    }
    let source = authenticated_service_role(handle, sender_pid);
    let event = match tag & M16_SERVICE_CYCLE_TAG_PREFIX_MASK {
        M16_SERVICE_CYCLE_COMMIT_TAG_PREFIX => Event::commit(
            ((tag >> 8) & M16_SERVICE_CYCLE_ROUND_MASK) as u32,
            (tag & 0xff) as u8,
            source,
            (payload >> 32) as u32,
            payload as u32,
        ),
        M16_SERVICE_CYCLE_DONE_TAG_PREFIX => {
            Event::done(tag as u32, Role::from_raw(payload), source)
        }
        _ => return,
    };
    let gate = if INIT_READY.load(Ordering::Acquire) && m15_service_transcript_is_complete() {
        Gate::open(
            M15_SERVICE_NEW_INSTANCE.load(Ordering::Acquire) as u32,
            m15_service_transaction_high_water(),
        )
    } else {
        Gate::closed()
    };
    let transition =
        bndroid_kernel::service_cycle::reduce(load_service_cycle_reducer_snapshot(), gate, event);
    store_service_cycle_reducer_snapshot(transition.snapshot);
}

fn load_service_cycle_reducer_snapshot() -> bndroid_kernel::service_cycle::Snapshot {
    bndroid_kernel::service_cycle::Snapshot {
        round: M16_SERVICE_CYCLE_ROUND.load(Ordering::Acquire) as u32,
        step: M16_SERVICE_CYCLE_STEP.load(Ordering::Acquire) as u8,
        completed_rounds: M16_SERVICE_CYCLE_COMPLETED_ROUNDS.load(Ordering::Acquire) as u32,
        errors: M16_SERVICE_CYCLE_ERRORS.load(Ordering::Acquire),
        instance: M16_SERVICE_CYCLE_INSTANCE.load(Ordering::Acquire) as u32,
        transaction_ids: core::array::from_fn(|index| {
            M16_SERVICE_CYCLE_TXIDS[index].load(Ordering::Acquire) as u32
        }),
        done_reads: M16_SERVICE_CYCLE_DONE_READS.load(Ordering::Acquire),
        done_bitmap: M16_SERVICE_CYCLE_DONE_BITMAP.load(Ordering::Acquire) as u8,
    }
}

fn store_service_cycle_reducer_snapshot(snapshot: bndroid_kernel::service_cycle::Snapshot) {
    M16_SERVICE_CYCLE_ROUND.store(u64::from(snapshot.round), Ordering::Relaxed);
    M16_SERVICE_CYCLE_STEP.store(u64::from(snapshot.step), Ordering::Relaxed);
    M16_SERVICE_CYCLE_COMPLETED_ROUNDS
        .store(u64::from(snapshot.completed_rounds), Ordering::Relaxed);
    M16_SERVICE_CYCLE_ERRORS.store(snapshot.errors, Ordering::Relaxed);
    M16_SERVICE_CYCLE_INSTANCE.store(u64::from(snapshot.instance), Ordering::Relaxed);
    for (stored, value) in M16_SERVICE_CYCLE_TXIDS.iter().zip(snapshot.transaction_ids) {
        stored.store(u64::from(value), Ordering::Relaxed);
    }
    M16_SERVICE_CYCLE_DONE_READS.store(snapshot.done_reads, Ordering::Relaxed);
    M16_SERVICE_CYCLE_DONE_BITMAP.store(u64::from(snapshot.done_bitmap), Ordering::Release);
}

fn authenticated_service_role(
    handle: HandleValue,
    sender_pid: u64,
) -> Option<bndroid_kernel::service_cycle::Role> {
    use bndroid_kernel::service_cycle::Role;

    [
        (Role::Provider, UserImageId::Provider),
        (Role::Client, UserImageId::Client),
        (Role::ServiceManager, UserImageId::ServiceManager),
    ]
    .into_iter()
    .find_map(|(role, image)| {
        authenticated_startup_sender(handle, sender_pid, image).then_some(role)
    })
}

fn authenticated_startup_sender(
    handle: HandleValue,
    sender_pid: u64,
    image_id: UserImageId,
) -> bool {
    crate::process::init_handle_is_startup_peer_for_process(handle, sender_pid, image_id)
}

fn record_m17_identity_transcript(handle: HandleValue, sender_pid: u64, tag: u64, payload: u64) {
    if !crate::process::current_is_init()
        || !matches!(
            tag,
            M17_PROVIDER_IDENTITY_DONE_TAG
                | M17_CLIENT_IDENTITY_DONE_TAG
                | M17_SERVICE_ACL_DONE_TAG
        )
    {
        return;
    }
    let cycle = load_service_cycle_reducer_snapshot();
    let source_valid =
        authenticated_startup_sender(handle, sender_pid, UserImageId::ServiceManager);
    if !INIT_READY.load(Ordering::Acquire)
        || !m15_service_transcript_is_complete()
        || !cycle.is_round_complete()
        || cycle.errors != 0
        || !source_valid
    {
        M17_IDENTITY_ERRORS.fetch_add(1, Ordering::Relaxed);
        return;
    }

    match tag {
        M17_PROVIDER_IDENTITY_DONE_TAG => {
            let expected = crate::process::unique_live_process_id_for_image(UserImageId::Provider);
            if M17_IDENTITY_DONE_READS.load(Ordering::Acquire) != 0
                || M17_ACL_DONE_READS.load(Ordering::Acquire) != 0
                || expected != Some(payload)
            {
                M17_IDENTITY_ERRORS.fetch_add(1, Ordering::Relaxed);
                return;
            }
            M17_PROVIDER_PID.store(payload, Ordering::Relaxed);
            M17_IDENTITY_DONE_READS.store(1, Ordering::Release);
        }
        M17_CLIENT_IDENTITY_DONE_TAG => {
            let expected = crate::process::unique_live_process_id_for_image(UserImageId::Client);
            if M17_IDENTITY_DONE_READS.load(Ordering::Acquire) != 1
                || M17_ACL_DONE_READS.load(Ordering::Acquire) != 0
                || M17_PROVIDER_PID.load(Ordering::Acquire) == 0
                || expected != Some(payload)
            {
                M17_IDENTITY_ERRORS.fetch_add(1, Ordering::Relaxed);
                return;
            }
            M17_CLIENT_PID.store(payload, Ordering::Relaxed);
            M17_IDENTITY_DONE_READS.store(2, Ordering::Release);
        }
        M17_SERVICE_ACL_DONE_TAG => {
            let expected_payload = (u64::from(M17_SERVICE_ACL_TXID) << 32)
                | (u64::from(M17_SERVICE_MALFORMED_REJECTIONS) << 16)
                | 1;
            if M17_IDENTITY_DONE_READS.load(Ordering::Acquire) != 2
                || M17_ACL_DONE_READS.load(Ordering::Acquire) != 0
                || M17_PROVIDER_PID.load(Ordering::Acquire) == 0
                || M17_CLIENT_PID.load(Ordering::Acquire) == 0
                || payload != expected_payload
            {
                M17_IDENTITY_ERRORS.fetch_add(1, Ordering::Relaxed);
                return;
            }
            M17_ACL_PAYLOAD.store(payload, Ordering::Relaxed);
            M17_ACL_DONE_READS.store(1, Ordering::Release);
        }
        _ => unreachable!(),
    }
}

fn record_m18_multi_client_transcript(
    handle: HandleValue,
    sender_pid: u64,
    tag: u64,
    payload: u64,
) {
    const READY_MANAGER: u64 = 1 << 0;
    const READY_PRIMARY: u64 = 1 << 1;
    const READY_SECONDARY: u64 = 1 << 2;
    const READY_PROVIDER: u64 = 1 << 3;
    const QUEUED_PRIMARY: u64 = 1 << 0;
    const QUEUED_SECONDARY: u64 = 1 << 1;
    const DONE_MANAGER: u64 = 1 << 0;
    const DONE_PROVIDER: u64 = 1 << 1;
    const DONE_PRIMARY: u64 = 1 << 2;
    const DONE_SECONDARY: u64 = 1 << 3;

    if !crate::process::current_is_init()
        || !matches!(
            tag,
            M18_CLIENT_READY_TAG
                | M18_CLIENT_QUEUED_TAG
                | M18_CLIENT_DONE_TAG
                | M18_PROVIDER_READY_TAG
                | M18_PROVIDER_DONE_TAG
                | M18_MANAGER_DONE_TAG
        )
    {
        return;
    }
    if M17_IDENTITY_DONE_READS.load(Ordering::Acquire) != 2
        || M17_ACL_DONE_READS.load(Ordering::Acquire) != 1
        || M17_IDENTITY_ERRORS.load(Ordering::Acquire) != 0
    {
        M18_ERRORS.fetch_add(1, Ordering::Relaxed);
        return;
    }

    let ready = M18_READY_BITMAP.load(Ordering::Acquire);
    let queued = M18_QUEUED_BITMAP.load(Ordering::Acquire);
    let done = M18_DONE_BITMAP.load(Ordering::Acquire);
    let primary_pid = M17_CLIENT_PID.load(Ordering::Acquire);
    let secondary_pid = M18_SECONDARY_CLIENT_PID.load(Ordering::Acquire);
    let provider_pid = M17_PROVIDER_PID.load(Ordering::Acquire);
    let manager_source =
        authenticated_startup_sender(handle, sender_pid, UserImageId::ServiceManager);
    let provider_source = sender_pid == provider_pid
        && authenticated_startup_sender(handle, sender_pid, UserImageId::Provider);
    let client_source = authenticated_startup_sender(handle, sender_pid, UserImageId::Client);

    let valid = match tag {
        M18_CLIENT_READY_TAG if manager_source => {
            if ready == 0 && queued == 0 && done == 0 && payload == M18_CLIENT_COUNT {
                M18_READY_BITMAP.store(READY_MANAGER, Ordering::Release);
                true
            } else {
                false
            }
        }
        M18_CLIENT_READY_TAG if client_source && sender_pid == primary_pid => {
            if ready == READY_MANAGER && queued == 0 && done == 0 && payload == sender_pid {
                M18_READY_BITMAP.store(READY_MANAGER | READY_PRIMARY, Ordering::Release);
                true
            } else {
                false
            }
        }
        M18_CLIENT_READY_TAG if client_source => {
            if ready == READY_MANAGER | READY_PRIMARY
                && queued == 0
                && done == 0
                && sender_pid != 0
                && sender_pid != primary_pid
                && payload == sender_pid
            {
                M18_SECONDARY_CLIENT_PID.store(sender_pid, Ordering::Relaxed);
                M18_READY_BITMAP.store(
                    READY_MANAGER | READY_PRIMARY | READY_SECONDARY,
                    Ordering::Release,
                );
                true
            } else {
                false
            }
        }
        M18_PROVIDER_READY_TAG => {
            if provider_source
                && ready == READY_MANAGER | READY_PRIMARY | READY_SECONDARY
                && queued == 0
                && done == 0
                && payload == M18_CLIENT_COUNT
            {
                M18_READY_BITMAP.store(
                    READY_MANAGER | READY_PRIMARY | READY_SECONDARY | READY_PROVIDER,
                    Ordering::Release,
                );
                true
            } else {
                false
            }
        }
        M18_CLIENT_QUEUED_TAG if sender_pid == primary_pid => {
            if client_source
                && ready == 0b1111
                && queued == 0
                && done == 0
                && payload == u64::from(M18_PRIMARY_TXID)
            {
                M18_QUEUED_BITMAP.store(QUEUED_PRIMARY, Ordering::Release);
                true
            } else {
                false
            }
        }
        M18_CLIENT_QUEUED_TAG => {
            if client_source
                && sender_pid == secondary_pid
                && ready == 0b1111
                && queued == QUEUED_PRIMARY
                && done == 0
                && payload == u64::from(M18_SECONDARY_TXID)
            {
                M18_QUEUED_BITMAP.store(QUEUED_PRIMARY | QUEUED_SECONDARY, Ordering::Release);
                true
            } else {
                false
            }
        }
        M18_MANAGER_DONE_TAG => {
            if manager_source
                && ready == 0b1111
                && queued == 0b11
                && done == 0
                && m18_order_valid(payload)
            {
                M18_REQUEST_ORDER.store(payload, Ordering::Relaxed);
                M18_DONE_BITMAP.store(DONE_MANAGER, Ordering::Release);
                true
            } else {
                false
            }
        }
        M18_PROVIDER_DONE_TAG => {
            if provider_source
                && ready == 0b1111
                && queued == 0b11
                && done == DONE_MANAGER
                && payload == M18_REQUEST_ORDER.load(Ordering::Acquire)
            {
                M18_DONE_BITMAP.store(DONE_MANAGER | DONE_PROVIDER, Ordering::Release);
                true
            } else {
                false
            }
        }
        M18_CLIENT_DONE_TAG if sender_pid == primary_pid => {
            if client_source
                && ready == 0b1111
                && queued == 0b11
                && done == DONE_MANAGER | DONE_PROVIDER
                && payload == m18_result_payload(M18_PRIMARY_TXID)
            {
                M18_DONE_BITMAP.store(
                    DONE_MANAGER | DONE_PROVIDER | DONE_PRIMARY,
                    Ordering::Release,
                );
                true
            } else {
                false
            }
        }
        M18_CLIENT_DONE_TAG => {
            if client_source
                && sender_pid == secondary_pid
                && ready == 0b1111
                && queued == 0b11
                && done == DONE_MANAGER | DONE_PROVIDER | DONE_PRIMARY
                && payload == m18_result_payload(M18_SECONDARY_TXID)
            {
                M18_DONE_BITMAP.store(
                    DONE_MANAGER | DONE_PROVIDER | DONE_PRIMARY | DONE_SECONDARY,
                    Ordering::Release,
                );
                true
            } else {
                false
            }
        }
        _ => false,
    };
    if !valid {
        M18_ERRORS.fetch_add(1, Ordering::Relaxed);
    }
}

const fn m18_result_payload(transaction_id: u32) -> u64 {
    ((transaction_id as u64) << 32) | M18_SERVICE_INSTANCE as u64
}

const fn m18_order_valid(payload: u64) -> bool {
    let first = (payload >> 32) as u32;
    let second = payload as u32;
    (first == M18_PRIMARY_TXID && second == M18_SECONDARY_TXID)
        || (first == M18_SECONDARY_TXID && second == M18_PRIMARY_TXID)
}

fn record_m20_multi_session_transcript(
    handle: HandleValue,
    sender_pid: u64,
    tag: u64,
    payload: u64,
) {
    if !crate::process::current_is_init()
        || !matches!(
            tag,
            M20_MANAGER_EVENT_TAG | M20_CLIENT_EVENT_TAG | M20_PROVIDER_EVENT_TAG
        )
    {
        return;
    }
    if M17_IDENTITY_DONE_READS.load(Ordering::Acquire) != 2
        || M17_ACL_DONE_READS.load(Ordering::Acquire) != 1
        || M17_IDENTITY_ERRORS.load(Ordering::Acquire) != 0
    {
        M20_ERRORS.fetch_add(1, Ordering::Relaxed);
        return;
    }

    let phase = M20_PHASE.load(Ordering::Acquire) as u8;
    let primary_pid = M17_CLIENT_PID.load(Ordering::Acquire);
    let provider_pid = M17_PROVIDER_PID.load(Ordering::Acquire);
    let secondary_pid = M20_SECONDARY_CLIENT_PID.load(Ordering::Acquire);
    let manager_source =
        authenticated_startup_sender(handle, sender_pid, UserImageId::ServiceManager);
    let provider_source = sender_pid == provider_pid
        && authenticated_startup_sender(handle, sender_pid, UserImageId::Provider);
    let primary_source = sender_pid == primary_pid
        && authenticated_startup_sender(handle, sender_pid, UserImageId::Client);
    let secondary_source = sender_pid == secondary_pid
        && authenticated_startup_sender(handle, sender_pid, UserImageId::Client);

    let valid = match phase {
        0 if tag == M20_MANAGER_EVENT_TAG
            && manager_source
            && payload == m20_event_payload(1, 1, 0) =>
        {
            M20_MANAGER_ATTACHES.store(1, Ordering::Relaxed);
            true
        }
        1 if tag == M20_CLIENT_EVENT_TAG
            && sender_pid != 0
            && sender_pid != primary_pid
            && sender_pid != provider_pid
            && authenticated_startup_sender(handle, sender_pid, UserImageId::Client)
            && payload == m20_event_payload(1, 1, 0) =>
        {
            M20_SECONDARY_CLIENT_PID.store(sender_pid, Ordering::Relaxed);
            M20_CLIENT_ATTACHES.store(1, Ordering::Relaxed);
            true
        }
        2 if tag == M20_PROVIDER_EVENT_TAG
            && provider_source
            && payload == m20_event_payload(1, 0, 0) =>
        {
            true
        }
        3 if tag == M20_PROVIDER_EVENT_TAG
            && provider_source
            && payload == m20_event_payload(2, 1, M20_STALLED_SECONDARY_TXID) =>
        {
            M20_PROVIDER_ACCEPTS.store(1, Ordering::Relaxed);
            true
        }
        4 if tag == M20_CLIENT_EVENT_TAG
            && secondary_source
            && payload == m20_event_payload(2, 1, M20_STALLED_SECONDARY_TXID) =>
        {
            true
        }
        5 if tag == M20_PROVIDER_EVENT_TAG
            && provider_source
            && payload == m20_event_payload(2, 1, M20_PRIMARY_STALLED_PROGRESS_TXID) =>
        {
            M20_PROVIDER_ACCEPTS.store(2, Ordering::Relaxed);
            true
        }
        6 if tag == M20_PROVIDER_EVENT_TAG
            && provider_source
            && payload == m20_event_payload(3, 1, M20_PRIMARY_STALLED_PROGRESS_TXID) =>
        {
            M20_PROVIDER_ECHOES.store(1, Ordering::Relaxed);
            true
        }
        7 if tag == M20_CLIENT_EVENT_TAG
            && primary_source
            && payload == m20_event_payload(3, 1, M20_PRIMARY_STALLED_PROGRESS_TXID) =>
        {
            M20_PRIMARY_PROGRESS_BITMAP.store(1, Ordering::Relaxed);
            true
        }
        8 if tag == M20_MANAGER_EVENT_TAG
            && manager_source
            && payload == m20_event_payload(2, 1, 0) =>
        {
            M20_REVOKE_BITMAP.store(1, Ordering::Relaxed);
            true
        }
        9 if tag == M20_CLIENT_EVENT_TAG
            && secondary_source
            && payload == m20_event_payload(4, 1, 0) =>
        {
            M20_REVOKE_BITMAP.store(0b11, Ordering::Relaxed);
            true
        }
        10 if tag == M20_PROVIDER_EVENT_TAG
            && provider_source
            && payload == m20_event_payload(4, 1, M20_STALLED_SECONDARY_TXID) =>
        {
            M20_PROVIDER_ABORTS.store(1, Ordering::Relaxed);
            true
        }
        11 if tag == M20_PROVIDER_EVENT_TAG
            && provider_source
            && payload == m20_event_payload(2, 1, M20_PRIMARY_DETACHED_PROGRESS_TXID) =>
        {
            M20_PROVIDER_ACCEPTS.store(3, Ordering::Relaxed);
            true
        }
        12 if tag == M20_PROVIDER_EVENT_TAG
            && provider_source
            && payload == m20_event_payload(3, 1, M20_PRIMARY_DETACHED_PROGRESS_TXID) =>
        {
            M20_PROVIDER_ECHOES.store(2, Ordering::Relaxed);
            true
        }
        13 if tag == M20_CLIENT_EVENT_TAG
            && primary_source
            && payload == m20_event_payload(3, 1, M20_PRIMARY_DETACHED_PROGRESS_TXID) =>
        {
            M20_PRIMARY_PROGRESS_BITMAP.store(0b11, Ordering::Relaxed);
            true
        }
        14 if tag == M20_MANAGER_EVENT_TAG
            && manager_source
            && payload == m20_event_payload(1, 2, 0) =>
        {
            M20_MANAGER_ATTACHES.store(2, Ordering::Relaxed);
            true
        }
        15 if tag == M20_CLIENT_EVENT_TAG
            && secondary_source
            && payload == m20_event_payload(1, 2, 0) =>
        {
            M20_CLIENT_ATTACHES.store(2, Ordering::Relaxed);
            true
        }
        16 if tag == M20_MANAGER_EVENT_TAG
            && manager_source
            && payload == m20_event_payload(3, 1, 0) =>
        {
            M20_STALE_REVOKE_BITMAP.store(1, Ordering::Relaxed);
            true
        }
        17 if tag == M20_CLIENT_EVENT_TAG
            && secondary_source
            && payload == m20_event_payload(5, 1, 0) =>
        {
            M20_STALE_REVOKE_BITMAP.store(0b11, Ordering::Relaxed);
            true
        }
        18 if tag == M20_PROVIDER_EVENT_TAG
            && provider_source
            && payload == m20_event_payload(2, 2, M20_SECONDARY_REATTACHED_TXID) =>
        {
            M20_PROVIDER_ACCEPTS.store(4, Ordering::Relaxed);
            true
        }
        19 if tag == M20_PROVIDER_EVENT_TAG
            && provider_source
            && payload == m20_event_payload(3, 2, M20_SECONDARY_REATTACHED_TXID) =>
        {
            M20_PROVIDER_ECHOES.store(3, Ordering::Relaxed);
            true
        }
        20 if tag == M20_CLIENT_EVENT_TAG
            && secondary_source
            && payload == m20_event_payload(3, 2, M20_SECONDARY_REATTACHED_TXID) =>
        {
            M20_SECONDARY_ECHOES.store(1, Ordering::Relaxed);
            true
        }
        21 if tag == M20_MANAGER_EVENT_TAG
            && manager_source
            && payload == m20_event_payload(4, 2, 0) =>
        {
            M20_FINAL_IDLE_BITMAP.store(1, Ordering::Relaxed);
            true
        }
        22 if tag == M20_PROVIDER_EVENT_TAG
            && provider_source
            && payload == m20_event_payload(5, 2, 0) =>
        {
            M20_FINAL_IDLE_BITMAP.store(0b11, Ordering::Relaxed);
            true
        }
        _ => false,
    };
    if valid {
        M20_PHASE.store(u64::from(phase + 1), Ordering::Release);
    } else {
        M20_ERRORS.fetch_add(1, Ordering::Relaxed);
    }
}

const fn m20_event_payload(phase: u8, lease: u16, transaction_id: u32) -> u64 {
    ((phase as u64) << 56) | ((lease as u64) << 32) | transaction_id as u64
}

fn m15_service_transcript_is_complete() -> bool {
    M15_SERVICE_PHASE.load(Ordering::Acquire) as u8 == M15_SERVICE_PHASE_COUNT
        && M15_SERVICE_ERRORS.load(Ordering::Acquire) == 0
        && M15_SERVICE_OLD_INSTANCE.load(Ordering::Acquire) != 0
        && M15_SERVICE_NEW_INSTANCE.load(Ordering::Acquire)
            > M15_SERVICE_OLD_INSTANCE.load(Ordering::Acquire)
        && M15_SERVICE_DONE_READS.load(Ordering::Acquire) == 3
        && M15_SERVICE_DONE_BITMAP.load(Ordering::Acquire) == 0b111
        && M15_SERVICE_TXIDS.iter().enumerate().all(|(index, stored)| {
            let transaction_id = stored.load(Ordering::Acquire);
            transaction_id != 0
                && M15_SERVICE_TXIDS[index + 1..]
                    .iter()
                    .all(|other| other.load(Ordering::Acquire) != transaction_id)
        })
}

fn m15_service_transaction_high_water() -> u32 {
    M15_SERVICE_TXIDS
        .iter()
        .map(|stored| stored.load(Ordering::Acquire) as u32)
        .max()
        .unwrap_or(0)
}

#[derive(Clone, Copy)]
enum AppLifecycleWriteKind {
    Bytes,
    Transfer,
}

fn capture_app_lifecycle_candidate(
    payload: &[u8],
    wire: &mut [u8; APP_LIFECYCLE_WIRE_SIZE],
) -> bool {
    if payload.len() < size_of::<u32>()
        || payload[..size_of::<u32>()] != APP_LIFECYCLE_MAGIC.to_le_bytes()
    {
        return false;
    }
    let captured = payload.len().min(APP_LIFECYCLE_WIRE_SIZE);
    wire[..captured].copy_from_slice(&payload[..captured]);
    true
}

fn record_committed_app_lifecycle(
    sender_pid: u64,
    wire: &[u8; APP_LIFECYCLE_WIRE_SIZE],
    payload_length: usize,
    write_kind: AppLifecycleWriteKind,
) {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("app lifecycle trace recorded with IRQ enabled");
    }

    let decoded = if payload_length == APP_LIFECYCLE_WIRE_SIZE {
        AppLifecycleMessage::decode(wire).ok()
    } else {
        None
    };
    let Some(message) = decoded else {
        let trace = app_lifecycle_trace_mut();
        trace.messages += 1;
        trace.errors += 1;
        trace.decode_errors += 1;
        record_app_lifecycle_write_kind(trace, write_kind);
        return;
    };

    let authenticated = match message.payload() {
        AppLifecyclePayload::Request { .. } => {
            crate::process::unique_live_process_id_for_image(UserImageId::Launcher)
                == Some(sender_pid)
        }
        AppLifecyclePayload::StateChanged { .. } | AppLifecyclePayload::Command { .. } => {
            crate::process::unique_live_process_id_for_image(UserImageId::Init) == Some(sender_pid)
        }
        AppLifecyclePayload::Ack { identity, .. } => {
            app_identity_process_id(identity) == sender_pid
                && crate::process::unique_live_process_id_for_image(UserImageId::App)
                    == Some(sender_pid)
        }
    };

    let trace = app_lifecycle_trace_mut();
    trace.messages += 1;
    record_app_lifecycle_write_kind(trace, write_kind);
    if !authenticated {
        trace.errors += 1;
        trace.authentication_errors += 1;
        return;
    }

    let before = trace.tracker.transaction();
    if trace.tracker.accept(message).is_err() {
        trace.errors += 1;
        trace.tracker_errors += 1;
        return;
    }
    let after = trace.tracker.transaction();
    let new_transaction = matches!(message.payload(), AppLifecyclePayload::Request { .. })
        || (before.pending_transaction_id().is_none()
            && before.last_transaction_id() != Some(message.transaction_id())
            && after.last_transaction_id() == Some(message.transaction_id()));
    if new_transaction {
        trace.transactions += 1;
        if trace.first_transaction_id == 0 {
            trace.first_transaction_id = message.transaction_id();
        }
    }
    if before.last_transaction_id() != Some(message.transaction_id())
        && after.last_transaction_id() == Some(message.transaction_id())
    {
        trace.completed += 1;
    }
    trace.last_transaction_id = message.transaction_id();
    if trace.first_sender_pid == 0 {
        trace.first_sender_pid = sender_pid;
    }
    trace.last_sender_pid = sender_pid;

    match message.payload() {
        AppLifecyclePayload::Request {
            action, identity, ..
        } => {
            trace.requests += 1;
            trace.request_actions[app_lifecycle_action_index(action)] += 1;
            record_app_lifecycle_identity(trace, identity);
        }
        AppLifecyclePayload::StateChanged {
            state,
            reason,
            identity,
            ..
        } => {
            trace.state_changes += 1;
            if state == AppLifecycleState::Crashed && reason == AppLifecycleReason::ProcessExited {
                trace.crashes += 1;
            }
            record_app_lifecycle_identity(trace, identity);
        }
        AppLifecyclePayload::Command {
            action, identity, ..
        } => {
            trace.commands += 1;
            trace.command_actions[app_lifecycle_action_index(action)] += 1;
            record_app_lifecycle_identity(trace, Some(identity));
        }
        AppLifecyclePayload::Ack {
            action, identity, ..
        } => {
            trace.acks += 1;
            trace.ack_actions[app_lifecycle_action_index(action)] += 1;
            record_app_lifecycle_identity(trace, Some(identity));
        }
    }
}

fn record_app_lifecycle_write_kind(
    trace: &mut AppLifecycleTraceState,
    write_kind: AppLifecycleWriteKind,
) {
    match write_kind {
        AppLifecycleWriteKind::Bytes => trace.byte_messages += 1,
        AppLifecycleWriteKind::Transfer => trace.transfer_messages += 1,
    }
}

fn record_app_lifecycle_identity(
    trace: &mut AppLifecycleTraceState,
    identity: Option<AppInstanceIdentity>,
) {
    let Some(identity) = identity else {
        return;
    };
    let process_id = app_identity_process_id(identity);
    if trace.first_instance_id == 0 {
        trace.first_instance_id = identity.instance_id();
        trace.first_app_pid = process_id;
    }
    trace.last_instance_id = identity.instance_id();
    trace.last_app_pid = process_id;
}

const fn app_identity_process_id(identity: AppInstanceIdentity) -> u64 {
    (identity.process().generation() as u64) << 32 | identity.process().pid() as u64
}

const fn app_lifecycle_action_index(action: AppLifecycleAction) -> usize {
    action.raw() as usize - 1
}

#[derive(Clone, Copy)]
enum UiSupervisorWriteKind {
    Bytes,
    Transfer,
}

fn capture_ui_supervisor_candidate(
    payload: &[u8],
    wire: &mut [u8; UI_SUPERVISOR_CONTROL_WIRE_SIZE],
) -> bool {
    if payload.len() < size_of::<u32>()
        || payload[..size_of::<u32>()] != UI_SUPERVISOR_CONTROL_MAGIC.to_le_bytes()
    {
        return false;
    }
    let captured = payload.len().min(UI_SUPERVISOR_CONTROL_WIRE_SIZE);
    wire[..captured].copy_from_slice(&payload[..captured]);
    true
}

fn record_committed_ui_supervisor(
    sender_pid: u64,
    wire: &[u8; UI_SUPERVISOR_CONTROL_WIRE_SIZE],
    payload_length: usize,
    write_kind: UiSupervisorWriteKind,
) {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("UI supervisor trace recorded with IRQ enabled");
    }

    let decoded = if payload_length == UI_SUPERVISOR_CONTROL_WIRE_SIZE {
        UiSupervisorControlMessage::decode(wire).ok()
    } else {
        None
    };
    let Some(message) = decoded else {
        let trace = ui_supervisor_trace_mut();
        trace.messages += 1;
        trace.errors += 1;
        trace.decode_errors += 1;
        record_ui_supervisor_write_kind(trace, write_kind);
        return;
    };

    let authenticated = match message.payload() {
        UiSupervisorControlPayload::Command { .. } => {
            crate::process::unique_live_process_id_for_image(UserImageId::Init) == Some(sender_pid)
        }
        UiSupervisorControlPayload::Ack { .. } | UiSupervisorControlPayload::OwnerDied { .. } => {
            crate::process::unique_live_process_id_for_image(UserImageId::SurfaceServer)
                == Some(sender_pid)
        }
    };

    let trace = ui_supervisor_trace_mut();
    trace.messages += 1;
    record_ui_supervisor_write_kind(trace, write_kind);
    if !authenticated {
        trace.errors += 1;
        trace.authentication_errors += 1;
        return;
    }
    if trace.tracker.accept(message).is_err() {
        trace.errors += 1;
        trace.tracker_errors += 1;
        return;
    }

    if trace.first_transaction_id == 0 {
        trace.first_transaction_id = message.transaction_id();
    }
    trace.last_transaction_id = message.transaction_id();
    if trace.first_sender_pid == 0 {
        trace.first_sender_pid = sender_pid;
    }
    trace.last_sender_pid = sender_pid;

    match message.payload() {
        UiSupervisorControlPayload::Command { operation, .. } => {
            trace.commands += 1;
            trace.transactions += 1;
            trace.operations[ui_supervisor_operation_index(operation)] += 1;
        }
        UiSupervisorControlPayload::Ack { .. } => {
            trace.acks += 1;
            trace.completed += 1;
        }
        UiSupervisorControlPayload::OwnerDied { .. } => {
            trace.owner_deaths += 1;
            trace.transactions += 1;
            trace.completed += 1;
        }
    }
}

fn record_ui_supervisor_write_kind(
    trace: &mut UiSupervisorTraceState,
    write_kind: UiSupervisorWriteKind,
) {
    match write_kind {
        UiSupervisorWriteKind::Bytes => trace.byte_messages += 1,
        UiSupervisorWriteKind::Transfer => trace.transfer_messages += 1,
    }
}

const fn ui_supervisor_operation_index(operation: UiSupervisorOperation) -> usize {
    operation.raw() as usize - 1
}

fn channel_write_bytes(
    frame: *mut TrapFrame,
    raw: u64,
    user_source: u64,
    raw_length: u64,
) -> *mut TrapFrame {
    let Some(handle) = parse_handle(raw) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let sender_pid = authenticated_sender_pid();
    #[cfg(all(
        feature = "service-supervisor-runtime",
        not(feature = "service-dependency-runtime")
    ))]
    let supervisor_transport = current_service_supervisor_transport(raw, sender_pid);
    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    let sender_image = crate::process::current_live_user_image_id()
        .unwrap_or_else(|| panic!("recovery channel write lacked an authenticated image"));
    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    let mut recovery_wire = [0_u8; CHANNEL_MESSAGE_MAX_BYTES];
    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    let mut recovery_length = 0_usize;
    let mut lifecycle_wire = [0_u8; APP_LIFECYCLE_WIRE_SIZE];
    let mut lifecycle_candidate = false;
    let mut supervisor_wire = [0_u8; UI_SUPERVISOR_CONTROL_WIRE_SIZE];
    let mut supervisor_candidate = false;
    let result = with_table(|table| {
        let object = table.get(handle, Rights::WRITE).map_err(|error| {
            if error == HandleError::AccessDenied {
                RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
            }
            handle_error_status(error)
        })?;
        if raw_length > CHANNEL_MESSAGE_MAX_BYTES as u64 {
            return Err(Status::InvalidArgument);
        }
        let length = raw_length as usize;
        let endpoint = object.as_channel().ok_or(Status::InvalidState)?;
        endpoint
            .write_with(|| {
                let mut data = [0_u8; CHANNEL_MESSAGE_MAX_BYTES];
                crate::arch::aarch64::usercopy::copy_from_user(&mut data[..length], user_source)
                    .map_err(|_| Status::BadAddress)?;
                #[cfg(any(
                    feature = "input-server-surface-restart-runtime",
                    feature = "input-server-restart-runtime",
                    feature = "service-dependency-runtime"
                ))]
                {
                    recovery_wire[..length].copy_from_slice(&data[..length]);
                    recovery_length = length;
                }
                lifecycle_candidate =
                    capture_app_lifecycle_candidate(&data[..length], &mut lifecycle_wire);
                supervisor_candidate =
                    capture_ui_supervisor_candidate(&data[..length], &mut supervisor_wire);
                Message::bytes(data, length)
                    .and_then(|message| message.with_sender(sender_pid))
                    .ok_or(Status::InvalidArgument)
            })
            .map_err(|error| match error {
                ChannelWriteFailure::Channel(error) => channel_error_status(error),
                ChannelWriteFailure::Rejected(status) => status,
            })
    });
    match result {
        Ok(()) => {
            BYTE_WRITES.fetch_add(1, Ordering::Relaxed);
            if raw_length == 0 {
                BYTE_ZERO_LENGTH.fetch_add(1, Ordering::Relaxed);
            }
            if lifecycle_candidate {
                record_committed_app_lifecycle(
                    sender_pid,
                    &lifecycle_wire,
                    raw_length as usize,
                    AppLifecycleWriteKind::Bytes,
                );
            }
            if supervisor_candidate {
                record_committed_ui_supervisor(
                    sender_pid,
                    &supervisor_wire,
                    raw_length as usize,
                    UiSupervisorWriteKind::Bytes,
                );
            }
            #[cfg(all(
                feature = "input-server-restart-runtime",
                not(feature = "service-dependency-runtime")
            ))]
            let _ = bndroid_kernel::input_server_restart_trace::record_channel_write(
                sender_pid,
                sender_image,
                &recovery_wire[..recovery_length],
                bndroid_kernel::input_server_restart_trace::ChannelWriteKind::Bytes,
            );
            #[cfg(all(
                feature = "service-supervisor-runtime",
                not(feature = "service-dependency-runtime")
            ))]
            let _ = bndroid_kernel::service_supervisor_trace::record_channel_write(
                sender_pid,
                sender_image,
                supervisor_transport,
                &recovery_wire[..recovery_length],
                bndroid_kernel::service_supervisor_trace::ChannelWriteKind::Bytes,
            );
            #[cfg(feature = "service-dependency-runtime")]
            let _ = if bndroid_kernel::input_surface_recovery_trace::snapshot().complete {
                #[cfg(feature = "post-recovery-interaction-runtime")]
                {
                    if bndroid_kernel::service_dependency_trace::snapshot().complete {
                        #[cfg(feature = "post-recovery-focus-runtime")]
                        {
                            if bndroid_kernel::post_recovery_interaction_trace::snapshot().complete
                            {
                                #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
                                {
                                    if bndroid_kernel::post_recovery_focus_trace::snapshot()
                                        .complete
                                    {
                                        bndroid_kernel::post_recovery_focus_roundtrip_trace::record_channel_write(
                                            sender_pid,
                                            sender_image,
                                            &recovery_wire[..recovery_length],
                                            bndroid_kernel::service_dependency_trace::ChannelWriteKind::Bytes,
                                        )
                                    } else {
                                        bndroid_kernel::post_recovery_focus_trace::record_channel_write(
                                            sender_pid,
                                            sender_image,
                                            &recovery_wire[..recovery_length],
                                            bndroid_kernel::service_dependency_trace::ChannelWriteKind::Bytes,
                                        )
                                    }
                                }
                                #[cfg(not(feature = "post-recovery-focus-roundtrip-runtime"))]
                                {
                                    bndroid_kernel::post_recovery_focus_trace::record_channel_write(
                                        sender_pid,
                                        sender_image,
                                        &recovery_wire[..recovery_length],
                                        bndroid_kernel::service_dependency_trace::ChannelWriteKind::Bytes,
                                    )
                                }
                            } else {
                                bndroid_kernel::post_recovery_interaction_trace::record_channel_write(
                                    sender_pid,
                                    sender_image,
                                    &recovery_wire[..recovery_length],
                                    bndroid_kernel::service_dependency_trace::ChannelWriteKind::Bytes,
                                )
                            }
                        }
                        #[cfg(not(feature = "post-recovery-focus-runtime"))]
                        {
                            bndroid_kernel::post_recovery_interaction_trace::record_channel_write(
                                sender_pid,
                                sender_image,
                                &recovery_wire[..recovery_length],
                                bndroid_kernel::service_dependency_trace::ChannelWriteKind::Bytes,
                            )
                        }
                    } else {
                        bndroid_kernel::service_dependency_trace::record_channel_write(
                            sender_pid,
                            sender_image,
                            &recovery_wire[..recovery_length],
                            bndroid_kernel::service_dependency_trace::ChannelWriteKind::Bytes,
                        )
                    }
                }
                #[cfg(not(feature = "post-recovery-interaction-runtime"))]
                {
                    bndroid_kernel::service_dependency_trace::record_channel_write(
                        sender_pid,
                        sender_image,
                        &recovery_wire[..recovery_length],
                        bndroid_kernel::service_dependency_trace::ChannelWriteKind::Bytes,
                    )
                }
            } else {
                bndroid_kernel::input_surface_recovery_trace::record_channel_write(
                    sender_pid,
                    sender_image,
                    &recovery_wire[..recovery_length],
                    bndroid_kernel::input_surface_recovery_trace::ChannelWriteKind::Bytes,
                )
            };
            #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
            let _ = bndroid_kernel::post_recovery_lifecycle_focus_trace::record_channel_write(
                sender_pid,
                sender_image,
                &recovery_wire[..recovery_length],
                bndroid_kernel::service_dependency_trace::ChannelWriteKind::Bytes,
            );
            #[cfg(all(
                feature = "input-server-surface-restart-runtime",
                not(feature = "input-server-restart-runtime"),
                not(feature = "service-dependency-runtime")
            ))]
            bndroid_kernel::input_surface_recovery_trace::record_channel_write(
                sender_pid,
                sender_image,
                &recovery_wire[..recovery_length],
                bndroid_kernel::input_surface_recovery_trace::ChannelWriteKind::Bytes,
            );
            crate::scheduler::wake_object_waiters();
            complete(frame, Status::Ok, raw_length, 0)
        }
        Err(status) => complete(frame, status, 0, 0),
    }
}

enum ByteReadFailure {
    Status(Status),
    BufferTooSmall(usize),
    BadAddress,
}

fn channel_read_bytes(
    frame: *mut TrapFrame,
    raw: u64,
    user_destination: u64,
    capacity: u64,
) -> *mut TrapFrame {
    let Some(handle) = parse_handle(raw) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let result = with_table(|table| {
        let object = table.get(handle, Rights::READ).map_err(|error| {
            if error == HandleError::AccessDenied {
                RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
            }
            ByteReadFailure::Status(handle_error_status(error))
        })?;
        let endpoint = object
            .as_channel()
            .ok_or(ByteReadFailure::Status(Status::InvalidState))?;
        endpoint
            .read_with(|message| {
                let bytes = message
                    .bytes_payload()
                    .ok_or(ByteReadFailure::Status(Status::InvalidState))?;
                if capacity < bytes.len() as u64 {
                    return Err(ByteReadFailure::BufferTooSmall(bytes.len()));
                }
                crate::arch::aarch64::usercopy::copy_to_user(user_destination, bytes)
                    .map_err(|_| ByteReadFailure::BadAddress)?;
                Ok(bytes.len())
            })
            .map_err(|error| match error {
                ChannelReadFailure::Channel(error) => {
                    ByteReadFailure::Status(channel_error_status(error))
                }
                ChannelReadFailure::Rejected(error) => error,
            })
    });
    match result {
        Ok(length) => {
            BYTE_READS.fetch_add(1, Ordering::Relaxed);
            if length == 0 {
                BYTE_ZERO_LENGTH.fetch_add(1, Ordering::Relaxed);
            }
            crate::scheduler::wake_object_waiters();
            complete(frame, Status::Ok, length as u64, 0)
        }
        Err(ByteReadFailure::BufferTooSmall(required)) => {
            BYTE_READ_ROLLBACKS.fetch_add(1, Ordering::Relaxed);
            BYTE_BUFFER_TOO_SMALL.fetch_add(1, Ordering::Relaxed);
            complete(frame, Status::BufferTooSmall, required as u64, 0)
        }
        Err(ByteReadFailure::BadAddress) => {
            BYTE_READ_ROLLBACKS.fetch_add(1, Ordering::Relaxed);
            complete(frame, Status::BadAddress, 0, 0)
        }
        Err(ByteReadFailure::Status(status)) => complete(frame, status, 0, 0),
    }
}

fn channel_write_transfer(
    frame: *mut TrapFrame,
    raw_transport: u64,
    user_source: u64,
    packed: u64,
) -> *mut TrapFrame {
    let Some(transport_handle) = parse_handle(raw_transport) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let (source_handle, raw_length) = unpack_transfer(packed);
    if !source_handle.is_valid() || raw_length as usize > CHANNEL_MESSAGE_MAX_BYTES {
        return complete(frame, Status::InvalidArgument, 0, 0);
    }
    let sender_pid = authenticated_sender_pid();
    #[cfg(all(
        feature = "service-supervisor-runtime",
        not(feature = "service-dependency-runtime")
    ))]
    let supervisor_transport = current_service_supervisor_transport(raw_transport, sender_pid);
    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    let sender_image = crate::process::current_live_user_image_id()
        .unwrap_or_else(|| panic!("recovery transfer write lacked an authenticated image"));
    let length = raw_length as usize;
    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    let mut recovery_wire = [0_u8; CHANNEL_MESSAGE_MAX_BYTES];
    let mut lifecycle_wire = [0_u8; APP_LIFECYCLE_WIRE_SIZE];
    let mut lifecycle_candidate = false;
    let mut supervisor_wire = [0_u8; UI_SUPERVISOR_CONTROL_WIRE_SIZE];
    let mut supervisor_candidate = false;
    let result = with_table(|table| {
        let transport = table
            .get(transport_handle, Rights::WRITE)
            .map_err(handle_error_status)?
            .as_channel()
            .ok_or(Status::InvalidState)?
            .clone();
        transport
            .write_with(|| {
                let source = table
                    .get(source_handle, Rights::TRANSFER)
                    .map_err(|error| {
                        if error == HandleError::AccessDenied {
                            RIGHTS_DENIALS.fetch_add(1, Ordering::Relaxed);
                        }
                        handle_error_status(error)
                    })?;
                if let Some(source_endpoint) = source.as_channel() {
                    if transport.same_channel(source_endpoint) {
                        TRANSFER_SELF_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                        return Err(Status::InvalidArgument);
                    }
                    if !transport.can_own_channel(source_endpoint) {
                        TRANSFER_ORDER_REJECTIONS.fetch_add(1, Ordering::Relaxed);
                        return Err(Status::InvalidArgument);
                    }
                }

                let mut data = [0_u8; CHANNEL_MESSAGE_MAX_BYTES];
                crate::arch::aarch64::usercopy::copy_from_user(&mut data[..length], user_source)
                    .map_err(|_| Status::BadAddress)?;
                #[cfg(any(
                    feature = "input-server-surface-restart-runtime",
                    feature = "input-server-restart-runtime",
                    feature = "service-dependency-runtime"
                ))]
                recovery_wire[..length].copy_from_slice(&data[..length]);
                lifecycle_candidate =
                    capture_app_lifecycle_candidate(&data[..length], &mut lifecycle_wire);
                supervisor_candidate =
                    capture_ui_supervisor_candidate(&data[..length], &mut supervisor_wire);
                let payload = TransferPayload::new(data, length)
                    .unwrap_or_else(|| panic!("validated transfer payload length was rejected"));

                // This is the final fallible operation in the locked write
                // transaction. A successful take is followed only by an
                // infallible inline Message construction and prechecked push.
                let owned = table
                    .take_owned(source_handle, Rights::TRANSFER)
                    .map_err(handle_error_status)?;
                let (object, rights) = owned.into_parts();
                if object.as_event().is_some() {
                    EVENT_TRANSFER_WRITES.fetch_add(1, Ordering::Relaxed);
                }
                if object.as_vmo().is_some() {
                    VMO_TRANSFER_WRITES.fetch_add(1, Ordering::Relaxed);
                }
                Ok(Message::transfer(payload, object, rights)
                    .with_sender(sender_pid)
                    .unwrap_or_else(|| panic!("fresh transfer message rejected sender stamp")))
            })
            .map_err(|error| match error {
                ChannelWriteFailure::Channel(error) => channel_error_status(error),
                ChannelWriteFailure::Rejected(status) => status,
            })
    });
    match result {
        Ok(()) => {
            TRANSFER_WRITES.fetch_add(1, Ordering::Relaxed);
            if lifecycle_candidate {
                record_committed_app_lifecycle(
                    sender_pid,
                    &lifecycle_wire,
                    length,
                    AppLifecycleWriteKind::Transfer,
                );
            }
            if supervisor_candidate {
                record_committed_ui_supervisor(
                    sender_pid,
                    &supervisor_wire,
                    length,
                    UiSupervisorWriteKind::Transfer,
                );
            }
            #[cfg(all(
                feature = "input-server-restart-runtime",
                not(feature = "service-dependency-runtime")
            ))]
            let _ = bndroid_kernel::input_server_restart_trace::record_channel_write(
                sender_pid,
                sender_image,
                &recovery_wire[..length],
                bndroid_kernel::input_server_restart_trace::ChannelWriteKind::Transfer,
            );
            #[cfg(all(
                feature = "service-supervisor-runtime",
                not(feature = "service-dependency-runtime")
            ))]
            let _ = bndroid_kernel::service_supervisor_trace::record_channel_write(
                sender_pid,
                sender_image,
                supervisor_transport,
                &recovery_wire[..length],
                bndroid_kernel::service_supervisor_trace::ChannelWriteKind::Transfer,
            );
            #[cfg(feature = "service-dependency-runtime")]
            let _ = if bndroid_kernel::input_surface_recovery_trace::snapshot().complete {
                #[cfg(feature = "post-recovery-interaction-runtime")]
                {
                    if bndroid_kernel::service_dependency_trace::snapshot().complete {
                        #[cfg(feature = "post-recovery-focus-runtime")]
                        {
                            if bndroid_kernel::post_recovery_interaction_trace::snapshot().complete
                            {
                                #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
                                {
                                    if bndroid_kernel::post_recovery_focus_trace::snapshot()
                                        .complete
                                    {
                                        bndroid_kernel::post_recovery_focus_roundtrip_trace::record_channel_write(
                                            sender_pid,
                                            sender_image,
                                            &recovery_wire[..length],
                                            bndroid_kernel::service_dependency_trace::ChannelWriteKind::Transfer,
                                        )
                                    } else {
                                        bndroid_kernel::post_recovery_focus_trace::record_channel_write(
                                            sender_pid,
                                            sender_image,
                                            &recovery_wire[..length],
                                            bndroid_kernel::service_dependency_trace::ChannelWriteKind::Transfer,
                                        )
                                    }
                                }
                                #[cfg(not(feature = "post-recovery-focus-roundtrip-runtime"))]
                                {
                                    bndroid_kernel::post_recovery_focus_trace::record_channel_write(
                                        sender_pid,
                                        sender_image,
                                        &recovery_wire[..length],
                                        bndroid_kernel::service_dependency_trace::ChannelWriteKind::Transfer,
                                    )
                                }
                            } else {
                                bndroid_kernel::post_recovery_interaction_trace::record_channel_write(
                                    sender_pid,
                                    sender_image,
                                    &recovery_wire[..length],
                                    bndroid_kernel::service_dependency_trace::ChannelWriteKind::Transfer,
                                )
                            }
                        }
                        #[cfg(not(feature = "post-recovery-focus-runtime"))]
                        {
                            bndroid_kernel::post_recovery_interaction_trace::record_channel_write(
                                sender_pid,
                                sender_image,
                                &recovery_wire[..length],
                                bndroid_kernel::service_dependency_trace::ChannelWriteKind::Transfer,
                            )
                        }
                    } else {
                        bndroid_kernel::service_dependency_trace::record_channel_write(
                            sender_pid,
                            sender_image,
                            &recovery_wire[..length],
                            bndroid_kernel::service_dependency_trace::ChannelWriteKind::Transfer,
                        )
                    }
                }
                #[cfg(not(feature = "post-recovery-interaction-runtime"))]
                {
                    bndroid_kernel::service_dependency_trace::record_channel_write(
                        sender_pid,
                        sender_image,
                        &recovery_wire[..length],
                        bndroid_kernel::service_dependency_trace::ChannelWriteKind::Transfer,
                    )
                }
            } else {
                bndroid_kernel::input_surface_recovery_trace::record_channel_write(
                    sender_pid,
                    sender_image,
                    &recovery_wire[..length],
                    bndroid_kernel::input_surface_recovery_trace::ChannelWriteKind::Transfer,
                )
            };
            #[cfg(feature = "post-recovery-lifecycle-focus-runtime")]
            let _ = bndroid_kernel::post_recovery_lifecycle_focus_trace::record_channel_write(
                sender_pid,
                sender_image,
                &recovery_wire[..length],
                bndroid_kernel::service_dependency_trace::ChannelWriteKind::Transfer,
            );
            #[cfg(all(
                feature = "input-server-surface-restart-runtime",
                not(feature = "input-server-restart-runtime"),
                not(feature = "service-dependency-runtime")
            ))]
            bndroid_kernel::input_surface_recovery_trace::record_channel_write(
                sender_pid,
                sender_image,
                &recovery_wire[..length],
                bndroid_kernel::input_surface_recovery_trace::ChannelWriteKind::Transfer,
            );
            crate::scheduler::wake_object_waiters();
            complete(frame, Status::Ok, raw_length as u64, 0)
        }
        Err(status) => complete(frame, status, 0, 0),
    }
}

enum TransferReadFailure {
    Status(Status),
    BufferTooSmall(usize),
}

fn channel_read_transfer(
    frame: *mut TrapFrame,
    raw_transport: u64,
    user_destination: u64,
    capacity: u64,
) -> *mut TrapFrame {
    let Some(transport_handle) = parse_handle(raw_transport) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let result = with_table(|table| {
        let transport = table
            .get(transport_handle, Rights::READ)
            .map_err(|error| TransferReadFailure::Status(handle_error_status(error)))?
            .as_channel()
            .ok_or(TransferReadFailure::Status(Status::InvalidState))?
            .clone();
        transport
            .read_owned_with(|message| {
                let Some(sender_pid) = message.sender_pid() else {
                    return Err((TransferReadFailure::Status(Status::InvalidState), message));
                };
                let (data, length, object, rights) = match message.into_transfer() {
                    Ok(parts) => parts,
                    Err(message) => {
                        return Err((TransferReadFailure::Status(Status::InvalidState), message));
                    }
                };
                if capacity < length as u64 {
                    return Err((
                        TransferReadFailure::BufferTooSmall(length),
                        rebuild_transfer(sender_pid, data, length, object, rights),
                    ));
                }
                if table.available() == 0 {
                    return Err((
                        TransferReadFailure::Status(Status::OutOfMemory),
                        rebuild_transfer(sender_pid, data, length, object, rights),
                    ));
                }
                if crate::arch::aarch64::usercopy::copy_to_user(user_destination, &data[..length])
                    .is_err()
                {
                    return Err((
                        TransferReadFailure::Status(Status::BadAddress),
                        rebuild_transfer(sender_pid, data, length, object, rights),
                    ));
                }
                let is_event = object.as_event().is_some();
                let is_vmo = object.as_vmo().is_some();
                let owned = OwnedHandle::new(object, rights);
                match table.try_insert_owned(owned) {
                    Ok(received) => Ok((length, received, is_event, is_vmo)),
                    Err(owned) => {
                        let (object, rights) = owned.into_parts();
                        Err((
                            TransferReadFailure::Status(Status::OutOfMemory),
                            rebuild_transfer(sender_pid, data, length, object, rights),
                        ))
                    }
                }
            })
            .map_err(|error| match error {
                ChannelReadFailure::Channel(error) => {
                    TransferReadFailure::Status(channel_error_status(error))
                }
                ChannelReadFailure::Rejected(error) => error,
            })
    });
    match result {
        Ok((length, received, is_event, is_vmo)) => {
            TRANSFER_READS.fetch_add(1, Ordering::Relaxed);
            if is_event {
                EVENT_TRANSFER_READS.fetch_add(1, Ordering::Relaxed);
            }
            if is_vmo {
                VMO_TRANSFER_READS.fetch_add(1, Ordering::Relaxed);
            }
            crate::scheduler::wake_object_waiters();
            complete(frame, Status::Ok, length as u64, u64::from(received.raw()))
        }
        Err(TransferReadFailure::BufferTooSmall(required)) => {
            TRANSFER_READ_ROLLBACKS.fetch_add(1, Ordering::Relaxed);
            complete(frame, Status::BufferTooSmall, required as u64, 0)
        }
        Err(TransferReadFailure::Status(status)) => {
            if matches!(
                status,
                Status::InvalidState | Status::BadAddress | Status::OutOfMemory
            ) {
                TRANSFER_READ_ROLLBACKS.fetch_add(1, Ordering::Relaxed);
            }
            complete(frame, status, 0, 0)
        }
    }
}

fn rebuild_transfer(
    sender_pid: u64,
    data: [u8; CHANNEL_MESSAGE_MAX_BYTES],
    length: usize,
    object: KernelObject,
    rights: Rights,
) -> Message {
    let payload = TransferPayload::new(data, length)
        .unwrap_or_else(|| panic!("queued transfer payload length became invalid"));
    Message::transfer(payload, object, rights)
        .with_sender(sender_pid)
        .unwrap_or_else(|| panic!("rollback transfer rejected its authenticated sender"))
}

fn handle_duplicate(frame: *mut TrapFrame, raw: u64, raw_rights: u64) -> *mut TrapFrame {
    let Some(handle) = parse_handle(raw) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let Ok(bits) = u32::try_from(raw_rights) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let Some(rights) = Rights::from_bits(bits) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    match with_table(|table| table.duplicate(handle, rights)) {
        Ok(duplicate) => {
            DUPLICATES.fetch_add(1, Ordering::Relaxed);
            complete(frame, Status::Ok, u64::from(duplicate.raw()), 0)
        }
        Err(error) => complete(frame, handle_error_status(error), 0, 0),
    }
}

fn handle_close(frame: *mut TrapFrame, raw: u64) -> *mut TrapFrame {
    enum CloseFailure {
        Handle(HandleError),
        Display(crate::display::DisplayError),
        #[cfg(feature = "input-server-runtime")]
        Input(bndroid_kernel::input_broker::InputBrokerError),
        #[cfg(feature = "storage-server-runtime")]
        Storage,
    }

    let Some(handle) = parse_handle(raw) else {
        return complete(frame, Status::InvalidArgument, 0, 0);
    };
    let Some(process_id) = crate::process::current_live_user_process_id() else {
        return complete(frame, Status::InvalidState, 0, 0);
    };
    // A mapped buffer is pinned independently by the address space. Allowing
    // its last usable handle to close would strand a live process with an
    // alias it can no longer unmap or release. This read-only clone ends the
    // handle-table borrow before process mapping metadata is queried; local
    // IRQ masking makes the two-stage preflight atomic on the boot CPU.
    let mapped_graphics = with_table(|table| {
        table
            .get(handle, Rights::NONE)
            .ok()
            .and_then(KernelObject::as_graphics_buffer)
            .cloned()
    });
    if mapped_graphics
        .as_ref()
        .is_some_and(|buffer| crate::process::current_graphics_buffer_mapping(buffer).is_some())
    {
        return complete(frame, Status::InvalidState, 0, 0);
    }
    let closed = with_table(|table| {
        let pending = table.begin_close(handle).map_err(CloseFailure::Handle)?;
        let evidence = pending
            .object()
            .as_surface()
            .map(|capability| crate::display::degrade_surface(capability, process_id))
            .transpose()
            .map_err(CloseFailure::Display)?;
        #[cfg(feature = "input-server-runtime")]
        let input_evidence = pending
            .object()
            .as_input()
            .map(|capability| crate::input_stream::release(capability, process_id))
            .transpose()
            .map_err(CloseFailure::Input)?;
        #[cfg(not(feature = "input-server-runtime"))]
        let input_evidence: Option<()> = None;
        #[cfg(feature = "storage-server-owner-liveness-runtime")]
        if pending.object().as_storage_volume().is_some() {
            // A live StorageServer may already own accepted session endpoints
            // which are absent from the broker queue. Only process teardown
            // can atomically retire that complete authority set. M61 therefore
            // makes the volume capability process-lifetime; older proof
            // profiles retain their historical explicit-close ABI below.
            return Err(CloseFailure::Storage);
        }
        #[cfg(all(
            feature = "storage-server-runtime",
            not(feature = "storage-server-owner-liveness-runtime")
        ))]
        let storage_released = if pending.object().as_storage_volume().is_some() {
            if !bndroid_kernel::storage_broker::release_process(process_id) {
                return Err(CloseFailure::Storage);
            }
            true
        } else {
            false
        };
        #[cfg(any(
            not(feature = "storage-server-runtime"),
            feature = "storage-server-owner-liveness-runtime"
        ))]
        let storage_released = false;
        Ok::<_, CloseFailure>((pending.commit(), evidence, input_evidence, storage_released))
    });
    match closed {
        Ok((object, surface_evidence, input_evidence, storage_released)) => {
            if let Some(evidence) = surface_evidence {
                let previous_owner = match evidence.previous_owner {
                    SurfaceOwner::KernelFallback => "kernel-fallback",
                    SurfaceOwner::UserspaceBound => "userspace-bound",
                    SurfaceOwner::Degraded => "degraded",
                };
                crate::kprintln!(
                    "SURFACE_DEGRADED from={} to=degraded reason=handle-close pid={} session={} last_frame={} commits={} pending={} coalesced={} peer_closed_edge={} scene_digest={:#018x} scanout_digest={:#018x}",
                    previous_owner,
                    evidence.process_id,
                    evidence.session_id,
                    evidence.last_frame_id.unwrap_or(0),
                    evidence.commits,
                    evidence.input.pending,
                    evidence.input.coalesced,
                    u8::from(evidence.peer_closed_edge),
                    evidence.scene_digest,
                    evidence.scanout_digest,
                );
            }
            #[cfg(feature = "input-server-runtime")]
            if let Some(evidence) = input_evidence {
                crate::kprintln!(
                    "INPUT_RELEASE_OK reason=handle-close pid={} session={} acquisition_floor={} release_floor={} discarded={} failed={} peer_closed_edge={}",
                    evidence.process_id,
                    evidence.session_id,
                    evidence.acquisition_floor,
                    evidence.release_floor,
                    evidence.discarded,
                    u8::from(evidence.failed),
                    u8::from(evidence.peer_closed_edge),
                );
            }
            #[cfg(not(feature = "input-server-runtime"))]
            let _ = input_evidence;
            let _ = storage_released;
            drop(object);
            CLOSES.fetch_add(1, Ordering::Relaxed);
            crate::scheduler::wake_object_waiters();
            complete(frame, Status::Ok, 0, 0)
        }
        Err(CloseFailure::Handle(HandleError::InvalidHandle)) => {
            STALE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            complete(frame, Status::NotFound, 0, 0)
        }
        Err(CloseFailure::Handle(error)) => complete(frame, handle_error_status(error), 0, 0),
        Err(CloseFailure::Display(error)) => {
            complete(frame, surface_display_error_status(error), 0, 0)
        }
        #[cfg(feature = "input-server-runtime")]
        Err(CloseFailure::Input(error)) => complete(frame, input_broker_error_status(error), 0, 0),
        #[cfg(feature = "storage-server-runtime")]
        Err(CloseFailure::Storage) => complete(frame, Status::InvalidState, 0, 0),
    }
}

#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
fn m65_storage_device_quiescent() -> bool {
    let stats = match crate::storage::stats() {
        Ok(stats) => stats,
        Err(_) => return false,
    };
    let irq = crate::storage::irq_snapshot();
    let terminal = match crate::storage::terminal_dma_snapshot() {
        Ok(terminal) => terminal,
        Err(_) => return false,
    };
    let recovery = crate::storage::async_recovery_snapshot();
    stats.requests != 0
        && stats.requests == stats.completions
        && stats.completions == stats.interrupt_completions
        && stats.timeouts == 0
        && stats.resets == 0
        && irq.armed
        && !irq.failed
        && !irq.rearm_prepared
        && !irq.recovery_required
        && !crate::storage::recovery_required()
        && !crate::storage::recovery_admission_closed()
        && !recovery.active
        && terminal.driver_state == 1
        && terminal.in_flight == 0
        && !terminal.recovery_active
        && terminal.recovery_scratch_clear
        && terminal.requests_terminal
}

#[cfg(feature = "storage-server-shutdown-orchestration-runtime")]
fn m65_storage_io_ledger_valid(
    broker: bndroid_kernel::storage_broker::BrokerSnapshot,
    io: crate::storage_server_io::Snapshot,
) -> bool {
    let classified = io
        .reads
        .checked_add(io.writes)
        .and_then(|value| value.checked_add(io.flushes))
        .and_then(|value| value.checked_add(io.errors));
    broker.submissions != 0
        && broker.submissions == broker.completions
        && broker.completions == broker.retrievals
        && broker.submitted_sectors == broker.completed_sectors
        && broker.read_sectors.checked_add(broker.write_sectors) == Some(broker.completed_sectors)
        && io.reads != 0
        && io.flushes != 0
        && io.errors == 0
        && io.abandoned_services == 0
        && io.recovery_attempts == 0
        && io.recovery_successes == 0
        && io.recovery_failures == 0
        && io.recovery_rearm_aborts == 0
        && io.recovery_fail_closed_retries == 0
        && !io.recovery_retry_pending
        && io.async_recovery_starts == 0
        && io.async_physical_completions == 0
        && io.async_physical_failures == 0
        && !io.async_coordinator_active
        && io.async_active_attempt == 0
        && io.fault_control_sequences == 0
        && io.fault_sequence == 0
        && io.fault_injected_reads == 0
        && io.fault_injected_writes == 0
        && io.fault_injected_flushes == 0
        && io.kernel_permanent_owner_requests == 0
        && io.kernel_permanent_fault_arms == 0
        && io.kernel_permanent_reads == 0
        && !io.terminal_quarantine_active
        && io.terminal_quarantine_starts == 0
        && io.completions == broker.completions
        && classified == Some(io.completions)
}

#[cfg(all(
    feature = "storage-server-shutdown-orchestration-runtime",
    not(feature = "resident-platform-shutdown-runtime")
))]
fn m65_shutdown_prepare_ready(generation: u64) -> bool {
    let Some(server_pid) =
        crate::process::unique_live_process_id_for_image(UserImageId::StorageServer)
    else {
        return false;
    };
    let processes = crate::process::snapshot();
    let broker = bndroid_kernel::storage_broker::snapshot();
    let io = crate::storage_server_io::snapshot();
    let shutdown = bndroid_kernel::shutdown::snapshot();
    !INIT_READY.load(Ordering::Acquire)
        && shutdown.phase == bndroid_kernel::shutdown::Phase::Open
        && shutdown.generation == 0
        && shutdown.calls == 4
        && shutdown.prepare_calls == 4
        && shutdown.prepares == 0
        && shutdown.commit_calls == 0
        && shutdown.commits == 0
        && shutdown.permission_denied == 2
        && shutdown.invalid_arguments == 0
        && shutdown.not_ready == 1
        && shutdown.replays == 0
        && generation != 0
        && processes.created == 4
        && processes.exited == 2
        && processes.reaped == 2
        && processes.terminated_exited == 2
        && processes.terminated_faulted == 0
        && processes.terminated_killed == 0
        && processes.live == 2
        && processes.peak_live == 3
        && !processes.wait_pending
        && !processes.supervisor_wait_pending
        && !processes.completion_pending
        && processes.storage_server_live == 1
        && processes.storage_server_handles == 2
        && processes.total_handles == 3
        && processes.child_spawn_images[..3]
            == [
                UserImageId::StorageServer.raw(),
                UserImageId::Launcher.raw(),
                UserImageId::App.raw(),
            ]
        && processes.child_spawn_images[3..]
            .iter()
            .all(|image| *image == 0)
        && processes.child_spawn_pids[..3].iter().all(|pid| *pid != 0)
        && processes.child_spawn_pids[3..].iter().all(|pid| *pid == 0)
        && processes.child_spawn_pids[0] == server_pid
        && with_table(|table| table.len() == 1)
        && broker.bound
        && broker.owner_pid == server_pid
        && broker.epoch == 1
        && broker.next_epoch == 2
        && broker.next_token == broker.submissions.checked_add(1).unwrap_or(0)
        && broker.next_session_id == 3
        && broker.pending_sessions == 0
        && broker.state == 0
        && broker.releases == 0
        && broker.abandoned == 0
        && !broker.abandoned_running
        && !broker.device_offline
        && m65_storage_io_ledger_valid(broker, io)
        && m65_storage_device_quiescent()
}

#[cfg(all(
    feature = "storage-server-shutdown-orchestration-runtime",
    not(feature = "resident-platform-shutdown-runtime")
))]
fn m65_storage_ready_proof_valid(generation: u64) -> bool {
    let processes = crate::process::snapshot();
    let broker = bndroid_kernel::storage_broker::snapshot();
    let io = crate::storage_server_io::snapshot();
    let shutdown = bndroid_kernel::shutdown::snapshot();
    shutdown.phase == bndroid_kernel::shutdown::Phase::Quiescing
        && shutdown.generation == generation
        && shutdown.calls == 4
        && shutdown.prepare_calls == 4
        && shutdown.prepares == 1
        && shutdown.commit_calls == 0
        && shutdown.commits == 0
        && shutdown.permission_denied == 2
        && shutdown.invalid_arguments == 0
        && shutdown.not_ready == 1
        && shutdown.replays == 0
        && shutdown.spawn_rejections == 1
        && shutdown.connect_rejections == 0
        && shutdown.invariant_errors == 0
        && processes.created == 4
        && processes.exited == 3
        && processes.reaped == 3
        && processes.terminated_exited == 3
        && processes.terminated_faulted == 0
        && processes.terminated_killed == 0
        && processes.live == 1
        && processes.peak_live == 3
        && !processes.wait_pending
        && !processes.supervisor_wait_pending
        && !processes.completion_pending
        && processes.storage_server_live == 0
        && processes.storage_server_handles == 0
        && processes.total_handles == 0
        && processes.child_spawn_images[..3]
            == [
                UserImageId::StorageServer.raw(),
                UserImageId::Launcher.raw(),
                UserImageId::App.raw(),
            ]
        && processes.child_spawn_images[3..]
            .iter()
            .all(|image| *image == 0)
        && processes.child_spawn_pids[..3].iter().all(|pid| *pid != 0)
        && processes.child_spawn_pids[3..].iter().all(|pid| *pid == 0)
        && with_table(|table| table.is_empty())
        && !broker.bound
        && broker.owner_pid == 0
        && broker.epoch == 0
        && broker.next_epoch == 2
        && broker.next_token == broker.submissions.checked_add(1).unwrap_or(0)
        && broker.next_session_id == 3
        && broker.pending_sessions == 0
        && broker.state == 0
        && broker.releases == 1
        && broker.abandoned == 0
        && !broker.abandoned_running
        && !broker.device_offline
        && m65_storage_io_ledger_valid(broker, io)
        && m65_storage_device_quiescent()
}

#[cfg(feature = "unified-product-runtime")]
const M67_PRODUCT_IMAGES: [u64; 10] = [
    UserImageId::ServiceManager.raw(),
    UserImageId::Provider.raw(),
    UserImageId::Client.raw(),
    UserImageId::ServiceManager.raw(),
    UserImageId::Client.raw(),
    UserImageId::InputServer.raw(),
    UserImageId::SurfaceServer.raw(),
    UserImageId::Launcher.raw(),
    UserImageId::App.raw(),
    UserImageId::StorageServer.raw(),
];

#[cfg(feature = "unified-product-runtime")]
fn m67_service_pids_match_product_spawns(
    services: bndroid_kernel::service_shutdown::Snapshot,
    processes: crate::process::ProcessSnapshot,
) -> bool {
    let expected = [
        processes.child_spawn_pids[3],
        processes.child_spawn_pids[1],
        processes.child_spawn_pids[2],
        processes.child_spawn_pids[4],
        processes.child_spawn_pids[6],
        processes.child_spawn_pids[5],
        processes.child_spawn_pids[7],
        processes.child_spawn_pids[8],
    ];
    services.pids == expected && expected.iter().all(|pid| *pid != 0)
}

#[cfg(feature = "unified-product-liveness-runtime")]
fn m68_storage_replacement_matches(processes: crate::process::ProcessSnapshot) -> bool {
    let previous = processes.child_spawn_pids[9];
    let replacement = processes.unified_product_storage_replacement_pid;
    previous != 0
        && replacement != 0
        && processes.unified_product_storage_replacement_image == UserImageId::StorageServer.raw()
        && previous as u32 == replacement as u32
        && (previous >> 32).checked_add(1) == Some(replacement >> 32)
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn m73_storage_rotation_chain_matches(processes: crate::process::ProcessSnapshot) -> bool {
    let initial = processes.child_spawn_pids[9];
    let first = processes.unified_product_storage_replacement_pid;
    let second = processes.unified_product_storage_second_replacement_pid;
    initial != 0
        && first != 0
        && second != 0
        && processes.unified_product_storage_replacement_image == UserImageId::StorageServer.raw()
        && processes.unified_product_storage_second_replacement_image
            == UserImageId::StorageServer.raw()
        && initial as u32 == first as u32
        && first as u32 == second as u32
        && (initial >> 32).checked_add(1) == Some(first >> 32)
        && (first >> 32).checked_add(1) == Some(second >> 32)
}

#[cfg(feature = "unified-product-manifest-supervision-runtime")]
fn service_manifest_access_proof_valid() -> bool {
    let base = SERVICE_MANIFEST_OPEN_CALLS.load(Ordering::Acquire) == 2
        && SERVICE_MANIFEST_OPEN_SUCCESSES.load(Ordering::Acquire) == 1
        && SERVICE_MANIFEST_OPEN_ARGUMENT_REJECTIONS.load(Ordering::Acquire) == 1
        && SERVICE_MANIFEST_OPEN_PERMISSION_DENIALS.load(Ordering::Acquire) == 0;
    #[cfg(feature = "unified-product-verified-manifest-runtime")]
    {
        #[cfg(feature = "unified-product-key-rotation-runtime")]
        let expected_rollback_index = u64::from(u32::from_le_bytes([
            VERIFIED_MANIFEST_ARTIFACT[24],
            VERIFIED_MANIFEST_ARTIFACT[25],
            VERIFIED_MANIFEST_ARTIFACT[26],
            VERIFIED_MANIFEST_ARTIFACT[27],
        ]));
        #[cfg(all(
            feature = "unified-product-persistent-rollback-runtime",
            not(feature = "unified-product-key-rotation-runtime")
        ))]
        let expected_rollback_index =
            u64::from(bndr_sm::manifest::PERSISTENT_ROLLBACK_SERVICE_MANIFEST_GENERATION);
        #[cfg(not(feature = "unified-product-persistent-rollback-runtime"))]
        let expected_rollback_index =
            u64::from(bndr_sm::verified_manifest::PRODUCT_MANIFEST_ROLLBACK_FLOOR);
        let verified = base
            && VERIFIED_MANIFEST_ATTEMPTS.load(Ordering::Acquire) == 1
            && VERIFIED_MANIFEST_SUCCESSES.load(Ordering::Acquire) == 1
            && VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.load(Ordering::Acquire) == 1
            && VERIFIED_MANIFEST_SIGNATURE_REJECTIONS.load(Ordering::Acquire) == 0
            && VERIFIED_MANIFEST_ROLLBACK_REJECTIONS.load(Ordering::Acquire) == 0
            && VERIFIED_MANIFEST_FORMAT_REJECTIONS.load(Ordering::Acquire) == 0
            && VERIFIED_MANIFEST_ROLLBACK_INDEX.load(Ordering::Acquire) == expected_rollback_index
            && VERIFIED_MANIFEST_DIGEST_0.load(Ordering::Acquire) != 0
            && VERIFIED_MANIFEST_DIGEST_1.load(Ordering::Acquire) != 0
            && VERIFIED_MANIFEST_DIGEST_2.load(Ordering::Acquire) != 0
            && VERIFIED_MANIFEST_DIGEST_3.load(Ordering::Acquire) != 0;
        #[cfg(feature = "unified-product-key-rotation-runtime")]
        {
            let rotation = key_rotation_manifest_snapshot();
            verified
                && rotation.prepared
                && rotation.ledger_attempts == 1
                && rotation.ledger_successes == 1
                && rotation.ledger_rollback_rejections == 0
                && rotation.ledger_failures == 0
                && rotation.retired_key_rejections == 0
                && rotation.evidence.is_some_and(|evidence| {
                    evidence.artifact_index == expected_rollback_index as u32
                        && evidence.committed_floor >= evidence.artifact_index
                        && evidence.binding.key_id == VERIFIED_MANIFEST_ARTIFACT[6]
                })
        }
        #[cfg(all(
            feature = "unified-product-persistent-rollback-runtime",
            not(feature = "unified-product-key-rotation-runtime")
        ))]
        {
            let persistent = persistent_manifest_rollback_snapshot();
            verified
                && persistent.prepared
                && persistent.ledger_attempts == 1
                && persistent.ledger_successes == 1
                && persistent.ledger_rollback_rejections == 0
                && persistent.ledger_failures == 0
                && persistent.evidence.is_some_and(|evidence| {
                    evidence.artifact_index == expected_rollback_index as u32
                        && evidence.committed_floor >= evidence.artifact_index
                        && evidence.binding.key_id
                            == bndr_sm::verified_manifest::PERSISTENT_MANIFEST_TRUSTED_KEY_ID
                })
        }
        #[cfg(not(feature = "unified-product-persistent-rollback-runtime"))]
        {
            verified
        }
    }
    #[cfg(not(feature = "unified-product-verified-manifest-runtime"))]
    {
        base
    }
}

#[cfg(all(
    feature = "unified-product-runtime",
    not(feature = "unified-product-manifest-supervision-runtime")
))]
const fn service_manifest_access_proof_valid() -> bool {
    true
}

#[cfg(feature = "unified-product-event-supervision-runtime")]
fn event_supervision_proof_valid() -> bool {
    bndroid_kernel::event_supervision_trace::snapshot().complete
}

#[cfg(all(
    feature = "unified-product-runtime",
    not(feature = "unified-product-event-supervision-runtime")
))]
const fn event_supervision_proof_valid() -> bool {
    true
}

#[cfg(feature = "unified-product-runtime")]
fn m67_live_product_proof_valid(generation: u64, expect_ready: bool) -> bool {
    let Some(server_pid) =
        crate::process::unique_live_process_id_for_image(UserImageId::StorageServer)
    else {
        return false;
    };
    let processes = crate::process::snapshot();
    let broker = bndroid_kernel::storage_broker::snapshot();
    let io = crate::storage_server_io::snapshot();
    let shutdown = bndroid_kernel::shutdown::snapshot();
    let services = bndroid_kernel::service_shutdown::snapshot();
    let ready = INIT_READY.load(Ordering::Acquire);
    let expected_calls = if expect_ready { 4 } else { 3 };
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    let process_profile_valid = processes.process_capacity == 10
        && processes.dynamic_capacity == 9
        && processes.created == 13
        && processes.exited == 3
        && processes.reaped == 3
        && processes.terminated_exited == 3
        && processes.terminated_faulted == 0
        && processes.terminated_killed == 0
        && processes.live == 10
        && processes.peak_live == 10
        && processes.storage_server_live == 1
        && processes.storage_server_handles == 2
        && processes.input_server_live == 1
        && processes.input_server_handles == 3
        && processes.total_handles == 39
        && processes.child_spawn_images == M67_PRODUCT_IMAGES
        && processes.child_spawn_pids.iter().all(|pid| *pid != 0)
        && m73_storage_rotation_chain_matches(processes)
        && processes.unified_product_storage_second_replacement_pid == server_pid
        && PROCESS_TERMINATE_CALLS.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_SUCCESSES.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_NOT_FOUND.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_INVALID_ARGUMENT.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_INVALID_STATE.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_PERMISSION_DENIED.load(Ordering::Acquire) == 0;
    #[cfg(all(
        feature = "unified-product-liveness-runtime",
        not(feature = "unified-product-event-supervision-runtime")
    ))]
    let process_profile_valid = processes.process_capacity == 10
        && processes.dynamic_capacity == 9
        && processes.created == 12
        && processes.exited == 2
        && processes.reaped == 2
        && processes.terminated_exited == 1
        && processes.terminated_faulted == 0
        && processes.terminated_killed == 1
        && processes.live == 10
        && processes.peak_live == 10
        && processes.storage_server_live == 1
        && processes.storage_server_handles == 2
        && processes.input_server_live == 1
        && processes.input_server_handles == 3
        && processes.total_handles == 39
        && processes.child_spawn_images == M67_PRODUCT_IMAGES
        && processes.child_spawn_pids.iter().all(|pid| *pid != 0)
        && m68_storage_replacement_matches(processes)
        && processes.unified_product_storage_replacement_pid == server_pid
        && PROCESS_TERMINATE_CALLS.load(Ordering::Acquire) == 1
        && PROCESS_TERMINATE_SUCCESSES.load(Ordering::Acquire) == 1
        && PROCESS_TERMINATE_NOT_FOUND.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_INVALID_ARGUMENT.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_INVALID_STATE.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_PERMISSION_DENIED.load(Ordering::Acquire) == 0;
    #[cfg(not(feature = "unified-product-liveness-runtime"))]
    let process_profile_valid = processes.process_capacity == 10
        && processes.dynamic_capacity == 9
        && processes.created == 11
        && processes.exited == 1
        && processes.reaped == 1
        && processes.terminated_exited == 1
        && processes.terminated_faulted == 0
        && processes.terminated_killed == 0
        && processes.live == 10
        && processes.peak_live == 10
        && processes.storage_server_live == 1
        && processes.storage_server_handles == 2
        && processes.input_server_live == 1
        && processes.input_server_handles == 3
        && processes.total_handles == 39
        && processes.child_spawn_images == M67_PRODUCT_IMAGES
        && processes.child_spawn_pids.iter().all(|pid| *pid != 0)
        && processes.child_spawn_pids[9] == server_pid
        && PROCESS_TERMINATE_CALLS.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_SUCCESSES.load(Ordering::Acquire) == 0;
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    let broker_profile_valid = broker.epoch == 3 && broker.next_epoch == 4 && broker.releases == 2;
    #[cfg(all(
        feature = "unified-product-liveness-runtime",
        not(feature = "unified-product-event-supervision-runtime")
    ))]
    let broker_profile_valid = broker.epoch == 2 && broker.next_epoch == 3 && broker.releases == 1;
    #[cfg(not(feature = "unified-product-liveness-runtime"))]
    let broker_profile_valid = broker.epoch == 1 && broker.next_epoch == 2 && broker.releases == 0;

    ready == expect_ready
        && bndroid_kernel::unified_product::ui_converged()
        && shutdown.phase == bndroid_kernel::shutdown::Phase::Open
        && shutdown.generation == 0
        && shutdown.calls == expected_calls
        && shutdown.prepare_calls == expected_calls
        && shutdown.prepares == 0
        && shutdown.commit_calls == 0
        && shutdown.commits == 0
        && shutdown.permission_denied == 2
        && shutdown.invalid_arguments == 0
        && shutdown.not_ready == 1
        && shutdown.replays == 0
        && shutdown.spawn_rejections == 0
        && shutdown.connect_rejections == 0
        && generation != 0
        && services.calls == 9
        && services.register_calls == 9
        && services.registrations == 8
        && services.quiesce_calls == 0
        && services.quiesces == 0
        && services.permission_denied == 1
        && services.invalid_arguments == 0
        && services.identity_rejections == 0
        && services.phase_rejections == 0
        && services.order_rejections == 0
        && services.replays == 0
        && services.invariant_errors == 0
        && services.registered_mask == bndr_abi::SHUTDOWN_SERVICE_ALL_MASK
        && services.quiesced_mask == 0
        && process_profile_valid
        && !processes.wait_pending
        && !processes.supervisor_wait_pending
        && !processes.completion_pending
        && m67_service_pids_match_product_spawns(services, processes)
        && service_manifest_access_proof_valid()
        && event_supervision_proof_valid()
        && with_table(|table| table.len() == 9)
        && broker.bound
        && broker.owner_pid == server_pid
        && broker_profile_valid
        && broker.next_token == broker.submissions.checked_add(1).unwrap_or(0)
        && broker.next_session_id == 3
        && broker.pending_sessions == 0
        && broker.state == 0
        && broker.abandoned == 0
        && !broker.abandoned_running
        && !broker.device_offline
        && m65_storage_io_ledger_valid(broker, io)
        && m65_storage_device_quiescent()
}

#[cfg(feature = "unified-product-runtime")]
fn m67_init_ready_proof_valid(generation: u64) -> bool {
    m67_live_product_proof_valid(generation, false)
}

#[cfg(feature = "unified-product-runtime")]
fn m67_shutdown_prepare_ready(generation: u64) -> bool {
    let stored_generation = M55_STORAGE_GENERATION.load(Ordering::Acquire);
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    let (ready_prefix, proof) = (M73_STORAGE_READY_PREFIX, M73_STORAGE_PROOF);
    #[cfg(all(
        feature = "unified-product-manifest-supervision-runtime",
        not(feature = "unified-product-event-supervision-runtime")
    ))]
    let (ready_prefix, proof) = (M72_STORAGE_READY_PREFIX, M72_STORAGE_PROOF);
    #[cfg(all(
        feature = "unified-product-continuous-supervision-runtime",
        not(feature = "unified-product-manifest-supervision-runtime")
    ))]
    let (ready_prefix, proof) = (M71_STORAGE_READY_PREFIX, M71_STORAGE_PROOF);
    #[cfg(all(
        feature = "unified-product-psci-shutdown-runtime",
        not(feature = "unified-product-continuous-supervision-runtime")
    ))]
    let (ready_prefix, proof) = (M70_STORAGE_READY_PREFIX, M70_STORAGE_PROOF);
    #[cfg(all(
        feature = "unified-product-multiservice-liveness-runtime",
        not(feature = "unified-product-psci-shutdown-runtime")
    ))]
    let (ready_prefix, proof) = (M69_STORAGE_READY_PREFIX, M69_STORAGE_PROOF);
    #[cfg(all(
        feature = "unified-product-liveness-runtime",
        not(feature = "unified-product-multiservice-liveness-runtime")
    ))]
    let (ready_prefix, proof) = (M68_STORAGE_READY_PREFIX, M68_STORAGE_PROOF);
    #[cfg(not(feature = "unified-product-liveness-runtime"))]
    let (ready_prefix, proof) = (M67_STORAGE_READY_PREFIX, M67_STORAGE_PROOF);
    stored_generation & M55_STORAGE_PREFIX_MASK == ready_prefix
        && stored_generation & M55_STORAGE_PAYLOAD_MASK == generation
        && M55_STORAGE_STAGES.load(Ordering::Acquire) == proof
        && m67_live_product_proof_valid(generation, true)
}

#[cfg(feature = "unified-product-runtime")]
fn m67_storage_ready_proof_valid(generation: u64) -> bool {
    let processes = crate::process::snapshot();
    let broker = bndroid_kernel::storage_broker::snapshot();
    let io = crate::storage_server_io::snapshot();
    let shutdown = bndroid_kernel::shutdown::snapshot();
    let services = bndroid_kernel::service_shutdown::snapshot();
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    let process_profile_valid = processes.process_capacity == 10
        && processes.dynamic_capacity == 9
        && processes.created == 13
        && processes.exited == 12
        && processes.reaped == 12
        && processes.terminated_exited == 12
        && processes.terminated_faulted == 0
        && processes.terminated_killed == 0
        && processes.live == 1
        && processes.peak_live == 10
        && processes.storage_server_live == 0
        && processes.storage_server_handles == 0
        && processes.input_server_live == 0
        && processes.input_server_handles == 0
        && processes.total_handles == 0
        && processes.child_spawn_images == M67_PRODUCT_IMAGES
        && processes.child_spawn_pids.iter().all(|pid| *pid != 0)
        && m73_storage_rotation_chain_matches(processes)
        && PROCESS_TERMINATE_CALLS.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_SUCCESSES.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_NOT_FOUND.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_INVALID_ARGUMENT.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_INVALID_STATE.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_PERMISSION_DENIED.load(Ordering::Acquire) == 0;
    #[cfg(all(
        feature = "unified-product-liveness-runtime",
        not(feature = "unified-product-event-supervision-runtime")
    ))]
    let process_profile_valid = processes.process_capacity == 10
        && processes.dynamic_capacity == 9
        && processes.created == 12
        && processes.exited == 11
        && processes.reaped == 11
        && processes.terminated_exited == 10
        && processes.terminated_faulted == 0
        && processes.terminated_killed == 1
        && processes.live == 1
        && processes.peak_live == 10
        && processes.storage_server_live == 0
        && processes.storage_server_handles == 0
        && processes.input_server_live == 0
        && processes.input_server_handles == 0
        && processes.total_handles == 0
        && processes.child_spawn_images == M67_PRODUCT_IMAGES
        && processes.child_spawn_pids.iter().all(|pid| *pid != 0)
        && m68_storage_replacement_matches(processes)
        && PROCESS_TERMINATE_CALLS.load(Ordering::Acquire) == 1
        && PROCESS_TERMINATE_SUCCESSES.load(Ordering::Acquire) == 1
        && PROCESS_TERMINATE_NOT_FOUND.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_INVALID_ARGUMENT.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_INVALID_STATE.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_PERMISSION_DENIED.load(Ordering::Acquire) == 0;
    #[cfg(not(feature = "unified-product-liveness-runtime"))]
    let process_profile_valid = processes.process_capacity == 10
        && processes.dynamic_capacity == 9
        && processes.created == 11
        && processes.exited == 10
        && processes.reaped == 10
        && processes.terminated_exited == 10
        && processes.terminated_faulted == 0
        && processes.terminated_killed == 0
        && processes.live == 1
        && processes.peak_live == 10
        && processes.storage_server_live == 0
        && processes.storage_server_handles == 0
        && processes.input_server_live == 0
        && processes.input_server_handles == 0
        && processes.total_handles == 0
        && processes.child_spawn_images == M67_PRODUCT_IMAGES
        && processes.child_spawn_pids.iter().all(|pid| *pid != 0)
        && PROCESS_TERMINATE_CALLS.load(Ordering::Acquire) == 0
        && PROCESS_TERMINATE_SUCCESSES.load(Ordering::Acquire) == 0;
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    let broker_profile_valid = broker.next_epoch == 4 && broker.releases == 3;
    #[cfg(all(
        feature = "unified-product-liveness-runtime",
        not(feature = "unified-product-event-supervision-runtime")
    ))]
    let broker_profile_valid = broker.next_epoch == 3 && broker.releases == 2;
    #[cfg(not(feature = "unified-product-liveness-runtime"))]
    let broker_profile_valid = broker.next_epoch == 2 && broker.releases == 1;

    bndroid_kernel::unified_product::ui_converged()
        && shutdown.phase == bndroid_kernel::shutdown::Phase::Quiescing
        && shutdown.generation == generation
        && shutdown.calls == 4
        && shutdown.prepare_calls == 4
        && shutdown.prepares == 1
        && shutdown.commit_calls == 0
        && shutdown.commits == 0
        && shutdown.permission_denied == 2
        && shutdown.invalid_arguments == 0
        && shutdown.not_ready == 1
        && shutdown.replays == 0
        && shutdown.spawn_rejections == 1
        && shutdown.connect_rejections == 2
        && shutdown.invariant_errors == 0
        && services.calls == 18
        && services.register_calls == 9
        && services.registrations == 8
        && services.quiesce_calls == 9
        && services.quiesces == 8
        && services.permission_denied == 1
        && services.invalid_arguments == 0
        && services.identity_rejections == 0
        && services.phase_rejections == 0
        && services.order_rejections == 1
        && services.replays == 0
        && services.invariant_errors == 0
        && services.complete()
        && process_profile_valid
        && !processes.wait_pending
        && !processes.supervisor_wait_pending
        && !processes.completion_pending
        && m67_service_pids_match_product_spawns(services, processes)
        && event_supervision_proof_valid()
        && with_table(|table| table.is_empty())
        && !broker.bound
        && broker.owner_pid == 0
        && broker.epoch == 0
        && broker_profile_valid
        && broker.next_token == broker.submissions.checked_add(1).unwrap_or(0)
        && broker.next_session_id == 3
        && broker.pending_sessions == 0
        && broker.state == 0
        && broker.abandoned == 0
        && !broker.abandoned_running
        && !broker.device_offline
        && m65_storage_io_ledger_valid(broker, io)
        && m65_storage_device_quiescent()
}

#[cfg(all(
    feature = "resident-platform-shutdown-runtime",
    not(feature = "unified-product-runtime")
))]
fn m66_shutdown_prepare_ready(generation: u64) -> bool {
    let Some(server_pid) =
        crate::process::unique_live_process_id_for_image(UserImageId::StorageServer)
    else {
        return false;
    };
    let processes = crate::process::snapshot();
    let topology = crate::process::resident_shutdown_topology_snapshot();
    let broker = bndroid_kernel::storage_broker::snapshot();
    let io = crate::storage_server_io::snapshot();
    let shutdown = bndroid_kernel::shutdown::snapshot();
    let services = bndroid_kernel::service_shutdown::snapshot();
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

    !INIT_READY.load(Ordering::Acquire)
        && shutdown.phase == bndroid_kernel::shutdown::Phase::Open
        && shutdown.generation == 0
        && shutdown.calls == 4
        && shutdown.prepare_calls == 4
        && shutdown.prepares == 0
        && shutdown.commit_calls == 0
        && shutdown.commits == 0
        && shutdown.permission_denied == 2
        && shutdown.invalid_arguments == 0
        && shutdown.not_ready == 1
        && shutdown.replays == 0
        && shutdown.spawn_rejections == 0
        && shutdown.connect_rejections == 0
        && generation != 0
        && services.calls == 9
        && services.register_calls == 9
        && services.registrations == 8
        && services.quiesce_calls == 0
        && services.quiesces == 0
        && services.permission_denied == 1
        && services.invalid_arguments == 0
        && services.identity_rejections == 0
        && services.phase_rejections == 0
        && services.order_rejections == 0
        && services.replays == 0
        && services.invariant_errors == 0
        && services.registered_mask == bndr_abi::SHUTDOWN_SERVICE_ALL_MASK
        && services.quiesced_mask == 0
        && topology.valid
        && topology.processes == 10
        && topology.service_nodes == bndr_abi::SHUTDOWN_SERVICE_NODE_COUNT
        && topology.control_pairs == 9
        && topology.dependency_pairs == bndr_abi::SHUTDOWN_SERVICE_EDGE_COUNT
        && topology.channel_pairs == 19
        && topology.endpoints == 38
        && topology.total_handles == 39
        && processes.created == 10
        && processes.exited == 0
        && processes.reaped == 0
        && processes.terminated_exited == 0
        && processes.terminated_faulted == 0
        && processes.terminated_killed == 0
        && processes.live == 10
        && processes.peak_live == 10
        && !processes.wait_pending
        && !processes.supervisor_wait_pending
        && !processes.completion_pending
        && processes.storage_server_live == 1
        && processes.storage_server_handles == 2
        && processes.input_server_live == 1
        && processes.input_server_handles == 4
        && processes.total_handles == 39
        && processes.child_spawn_images[..9] == expected_images
        && processes.child_spawn_images[9] == 0
        && processes.child_spawn_pids[..9].iter().all(|pid| *pid != 0)
        && processes.child_spawn_pids[9] == 0
        && processes.child_spawn_pids[0] == server_pid
        && with_table(|table| table.len() == 9)
        && broker.bound
        && broker.owner_pid == server_pid
        && broker.epoch == 1
        && broker.next_epoch == 2
        && broker.next_token == broker.submissions.checked_add(1).unwrap_or(0)
        && broker.next_session_id == 3
        && broker.pending_sessions == 0
        && broker.state == 0
        && broker.releases == 0
        && broker.abandoned == 0
        && !broker.abandoned_running
        && !broker.device_offline
        && m65_storage_io_ledger_valid(broker, io)
        && m65_storage_device_quiescent()
}

#[cfg(all(
    feature = "resident-platform-shutdown-runtime",
    not(feature = "unified-product-runtime")
))]
fn m66_storage_ready_proof_valid(generation: u64) -> bool {
    let processes = crate::process::snapshot();
    let broker = bndroid_kernel::storage_broker::snapshot();
    let io = crate::storage_server_io::snapshot();
    let shutdown = bndroid_kernel::shutdown::snapshot();
    let services = bndroid_kernel::service_shutdown::snapshot();
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

    shutdown.phase == bndroid_kernel::shutdown::Phase::Quiescing
        && shutdown.generation == generation
        && shutdown.calls == 4
        && shutdown.prepare_calls == 4
        && shutdown.prepares == 1
        && shutdown.commit_calls == 0
        && shutdown.commits == 0
        && shutdown.permission_denied == 2
        && shutdown.invalid_arguments == 0
        && shutdown.not_ready == 1
        && shutdown.replays == 0
        && shutdown.spawn_rejections == 1
        && shutdown.connect_rejections == 2
        && shutdown.invariant_errors == 0
        && services.calls == 18
        && services.register_calls == 9
        && services.registrations == 8
        && services.quiesce_calls == 9
        && services.quiesces == 8
        && services.permission_denied == 1
        && services.invalid_arguments == 0
        && services.identity_rejections == 0
        && services.phase_rejections == 0
        && services.order_rejections == 1
        && services.replays == 0
        && services.invariant_errors == 0
        && services.complete()
        && processes.created == 10
        && processes.exited == 9
        && processes.reaped == 9
        && processes.terminated_exited == 9
        && processes.terminated_faulted == 0
        && processes.terminated_killed == 0
        && processes.live == 1
        && processes.peak_live == 10
        && !processes.wait_pending
        && !processes.supervisor_wait_pending
        && !processes.completion_pending
        && processes.storage_server_live == 0
        && processes.storage_server_handles == 0
        && processes.input_server_live == 0
        && processes.input_server_handles == 0
        && processes.total_handles == 0
        && processes.child_spawn_images[..9] == expected_images
        && processes.child_spawn_images[9] == 0
        && processes.child_spawn_pids[..9].iter().all(|pid| *pid != 0)
        && processes.child_spawn_pids[9] == 0
        && services
            .pids
            .iter()
            .enumerate()
            .all(|(index, pid)| *pid == processes.child_spawn_pids[index + 1])
        && with_table(|table| table.is_empty())
        && !broker.bound
        && broker.owner_pid == 0
        && broker.epoch == 0
        && broker.next_epoch == 2
        && broker.next_token == broker.submissions.checked_add(1).unwrap_or(0)
        && broker.next_session_id == 3
        && broker.pending_sessions == 0
        && broker.state == 0
        && broker.releases == 1
        && broker.abandoned == 0
        && !broker.abandoned_running
        && !broker.device_offline
        && m65_storage_io_ledger_valid(broker, io)
        && m65_storage_device_quiescent()
}

fn init_ready(
    frame: *mut TrapFrame,
    magic: u64,
    register_sentinel: u64,
    stack_sentinel: u64,
) -> *mut TrapFrame {
    if !crate::process::current_is_init() {
        return complete(frame, Status::PermissionDenied, 0, 0);
    }
    #[cfg(feature = "process-terminate-self-test")]
    if magic == INIT_READY_MAGIC
        && register_sentinel == PROCESS_TERMINATE_SELF_TEST_READY_1
        && stack_sentinel == PROCESS_TERMINATE_SELF_TEST_READY_2
        && with_table(|table| table.is_empty())
    {
        REGISTER_PRESERVED.store(true, Ordering::Relaxed);
        STACK_ROUND_TRIP.store(true, Ordering::Relaxed);
        INIT_READY.store(true, Ordering::Release);
        return complete(frame, Status::Ok, 0, 0);
    }

    #[cfg(all(
        feature = "storage-server-runtime",
        not(feature = "storage-server-recovery-runtime")
    ))]
    {
        let generation = register_sentinel & M55_STORAGE_PAYLOAD_MASK;
        let stages = stack_sentinel & M55_STORAGE_PAYLOAD_MASK;
        let launcher_stage = (stages >> 8) & 0xff;
        let app_stage = stages & 0xff;
        if magic != INIT_READY_MAGIC
            || register_sentinel & M55_STORAGE_PREFIX_MASK != M55_STORAGE_READY_PREFIX
            || stack_sentinel & M55_STORAGE_PREFIX_MASK != M55_STORAGE_PROOF_PREFIX
            || generation == 0
            || launcher_stage != 2
            || !matches!(app_stage, 1 | 2)
            || stages & !0xffff != 0
            || with_table(|table| table.len() != 1)
            || !crate::process::storage_server_ready_topology_valid()
            || !m55_storage_ready_proof_valid()
        {
            return complete(frame, Status::InvalidState, 0, 0);
        }
        REGISTER_PRESERVED.store(true, Ordering::Relaxed);
        STACK_ROUND_TRIP.store(true, Ordering::Relaxed);
        M55_STORAGE_GENERATION.store(generation, Ordering::Relaxed);
        M55_STORAGE_STAGES.store(stages, Ordering::Relaxed);
        INIT_READY.store(true, Ordering::Release);
        complete(frame, Status::Ok, 0, 0)
    }

    #[cfg(all(
        feature = "storage-server-recovery-runtime",
        not(feature = "storage-server-repeated-recovery-runtime")
    ))]
    {
        let generation = register_sentinel & M55_STORAGE_PAYLOAD_MASK;
        if magic != INIT_READY_MAGIC
            || register_sentinel & M55_STORAGE_PREFIX_MASK != M56_STORAGE_READY_PREFIX
            || stack_sentinel != M56_STORAGE_PROOF
            || generation == 0
            || with_table(|table| table.len() != 1)
            || !m56_storage_ready_proof_valid()
        {
            return complete(frame, Status::InvalidState, 0, 0);
        }
        REGISTER_PRESERVED.store(true, Ordering::Relaxed);
        STACK_ROUND_TRIP.store(true, Ordering::Relaxed);
        M55_STORAGE_GENERATION.store(register_sentinel, Ordering::Relaxed);
        M55_STORAGE_STAGES.store(stack_sentinel, Ordering::Relaxed);
        INIT_READY.store(true, Ordering::Release);
        complete(frame, Status::Ok, 0, 0)
    }

    #[cfg(all(
        feature = "storage-server-repeated-recovery-runtime",
        not(feature = "storage-server-async-recovery-runtime")
    ))]
    {
        let generation = register_sentinel & M55_STORAGE_PAYLOAD_MASK;
        if magic != INIT_READY_MAGIC
            || register_sentinel & M55_STORAGE_PREFIX_MASK != M57_STORAGE_READY_PREFIX
            || stack_sentinel != M57_STORAGE_PROOF
            || generation == 0
            || with_table(|table| table.len() != 1)
            || !m57_storage_ready_proof_valid()
        {
            return complete(frame, Status::InvalidState, 0, 0);
        }
        REGISTER_PRESERVED.store(true, Ordering::Relaxed);
        STACK_ROUND_TRIP.store(true, Ordering::Relaxed);
        M55_STORAGE_GENERATION.store(register_sentinel, Ordering::Relaxed);
        M55_STORAGE_STAGES.store(stack_sentinel, Ordering::Relaxed);
        INIT_READY.store(true, Ordering::Release);
        complete(frame, Status::Ok, 0, 0)
    }

    #[cfg(all(
        feature = "storage-server-async-recovery-runtime",
        not(feature = "storage-server-fault-policy-runtime")
    ))]
    {
        let generation = register_sentinel & M55_STORAGE_PAYLOAD_MASK;
        if magic != INIT_READY_MAGIC
            || register_sentinel & M55_STORAGE_PREFIX_MASK != M58_STORAGE_READY_PREFIX
            || stack_sentinel != M58_STORAGE_PROOF
            || generation == 0
            || with_table(|table| table.len() != 1)
            || !m58_storage_ready_proof_valid()
        {
            return complete(frame, Status::InvalidState, 0, 0);
        }
        REGISTER_PRESERVED.store(true, Ordering::Relaxed);
        STACK_ROUND_TRIP.store(true, Ordering::Relaxed);
        M55_STORAGE_GENERATION.store(register_sentinel, Ordering::Relaxed);
        M55_STORAGE_STAGES.store(stack_sentinel, Ordering::Relaxed);
        INIT_READY.store(true, Ordering::Release);
        complete(frame, Status::Ok, 0, 0)
    }

    #[cfg(all(
        feature = "storage-server-shutdown-orchestration-runtime",
        not(feature = "resident-platform-shutdown-runtime")
    ))]
    {
        let generation = register_sentinel & M55_STORAGE_PAYLOAD_MASK;
        if magic != INIT_READY_MAGIC
            || register_sentinel & M55_STORAGE_PREFIX_MASK != M65_STORAGE_READY_PREFIX
            || stack_sentinel != M65_STORAGE_PROOF
            || generation == 0
            || !with_table(|table| table.is_empty())
            || !m65_storage_ready_proof_valid(generation)
        {
            return complete(frame, Status::InvalidState, 0, 0);
        }
        REGISTER_PRESERVED.store(true, Ordering::Relaxed);
        STACK_ROUND_TRIP.store(true, Ordering::Relaxed);
        M55_STORAGE_GENERATION.store(register_sentinel, Ordering::Relaxed);
        M55_STORAGE_STAGES.store(stack_sentinel, Ordering::Relaxed);
        INIT_READY.store(true, Ordering::Release);
        complete(frame, Status::Ok, 0, 0)
    }

    #[cfg(feature = "unified-product-runtime")]
    {
        let generation = register_sentinel & M55_STORAGE_PAYLOAD_MASK;
        #[cfg(feature = "unified-product-event-supervision-runtime")]
        let (ready_prefix, proof) = (M73_STORAGE_READY_PREFIX, M73_STORAGE_PROOF);
        #[cfg(all(
            feature = "unified-product-manifest-supervision-runtime",
            not(feature = "unified-product-event-supervision-runtime")
        ))]
        let (ready_prefix, proof) = (M72_STORAGE_READY_PREFIX, M72_STORAGE_PROOF);
        #[cfg(all(
            feature = "unified-product-continuous-supervision-runtime",
            not(feature = "unified-product-manifest-supervision-runtime")
        ))]
        let (ready_prefix, proof) = (M71_STORAGE_READY_PREFIX, M71_STORAGE_PROOF);
        #[cfg(all(
            feature = "unified-product-psci-shutdown-runtime",
            not(feature = "unified-product-continuous-supervision-runtime")
        ))]
        let (ready_prefix, proof) = (M70_STORAGE_READY_PREFIX, M70_STORAGE_PROOF);
        #[cfg(all(
            feature = "unified-product-multiservice-liveness-runtime",
            not(feature = "unified-product-psci-shutdown-runtime")
        ))]
        let (ready_prefix, proof) = (M69_STORAGE_READY_PREFIX, M69_STORAGE_PROOF);
        #[cfg(all(
            feature = "unified-product-liveness-runtime",
            not(feature = "unified-product-multiservice-liveness-runtime")
        ))]
        let (ready_prefix, proof) = (M68_STORAGE_READY_PREFIX, M68_STORAGE_PROOF);
        #[cfg(not(feature = "unified-product-liveness-runtime"))]
        let (ready_prefix, proof) = (M67_STORAGE_READY_PREFIX, M67_STORAGE_PROOF);
        if magic != INIT_READY_MAGIC
            || register_sentinel & M55_STORAGE_PREFIX_MASK != ready_prefix
            || stack_sentinel != proof
            || generation == 0
            || !m67_init_ready_proof_valid(generation)
        {
            return complete(frame, Status::InvalidState, 0, 0);
        }
        REGISTER_PRESERVED.store(true, Ordering::Relaxed);
        STACK_ROUND_TRIP.store(true, Ordering::Relaxed);
        M55_STORAGE_GENERATION.store(register_sentinel, Ordering::Relaxed);
        M55_STORAGE_STAGES.store(stack_sentinel, Ordering::Relaxed);
        INIT_READY.store(true, Ordering::Release);
        complete(frame, Status::Ok, 0, 0)
    }

    #[cfg(all(
        feature = "resident-platform-shutdown-runtime",
        not(feature = "unified-product-runtime")
    ))]
    {
        let generation = register_sentinel & M55_STORAGE_PAYLOAD_MASK;
        if magic != INIT_READY_MAGIC
            || register_sentinel & M55_STORAGE_PREFIX_MASK != M66_STORAGE_READY_PREFIX
            || stack_sentinel != M66_STORAGE_PROOF
            || generation == 0
            || !with_table(|table| table.is_empty())
            || !m66_storage_ready_proof_valid(generation)
        {
            return complete(frame, Status::InvalidState, 0, 0);
        }
        REGISTER_PRESERVED.store(true, Ordering::Relaxed);
        STACK_ROUND_TRIP.store(true, Ordering::Relaxed);
        M55_STORAGE_GENERATION.store(register_sentinel, Ordering::Relaxed);
        M55_STORAGE_STAGES.store(stack_sentinel, Ordering::Relaxed);
        INIT_READY.store(true, Ordering::Release);
        complete(frame, Status::Ok, 0, 0)
    }

    #[cfg(all(
        feature = "storage-server-fault-policy-runtime",
        not(feature = "storage-server-shutdown-orchestration-runtime")
    ))]
    {
        let generation = register_sentinel & M55_STORAGE_PAYLOAD_MASK;
        if magic != INIT_READY_MAGIC
            || register_sentinel & M55_STORAGE_PREFIX_MASK != M60_STORAGE_READY_PREFIX
            || stack_sentinel != M60_STORAGE_PROOF
            || generation == 0
            || !with_table(|table| table.is_empty())
            || !m60_storage_ready_proof_valid()
        {
            return complete(frame, Status::InvalidState, 0, 0);
        }
        REGISTER_PRESERVED.store(true, Ordering::Relaxed);
        STACK_ROUND_TRIP.store(true, Ordering::Relaxed);
        M55_STORAGE_GENERATION.store(register_sentinel, Ordering::Relaxed);
        M55_STORAGE_STAGES.store(stack_sentinel, Ordering::Relaxed);
        INIT_READY.store(true, Ordering::Release);
        complete(frame, Status::Ok, 0, 0)
    }

    #[cfg(not(feature = "storage-server-runtime"))]
    {
        // M13 keeps one init-owned lifetime endpoint for each resident core
        // service.  Requiring an empty table here would turn healthy daemons into
        // apparent leaks and force init to kill them before publishing readiness.
        // The kernel's post-ready convergence proof separately requires exactly
        // three resident child waits and four distinct live image identities.
        if magic != INIT_READY_MAGIC
            || register_sentinel != USER_REGISTER_SENTINEL
            || stack_sentinel != USER_STACK_SENTINEL
            || with_table(|table| table.len() != 3)
            || !crate::process::resident_control_topology_valid()
            || LAST_TAG.load(Ordering::Relaxed) != EXPECTED_TAG
            || LAST_PAYLOAD.load(Ordering::Relaxed) != EXPECTED_PAYLOAD
        {
            return complete(frame, Status::InvalidState, 0, 0);
        }
        REGISTER_PRESERVED.store(true, Ordering::Relaxed);
        STACK_ROUND_TRIP.store(true, Ordering::Relaxed);
        INIT_READY.store(true, Ordering::Release);
        complete(frame, Status::Ok, 0, 0)
    }
}

#[cfg(all(
    feature = "storage-server-runtime",
    not(feature = "storage-server-recovery-runtime")
))]
fn m55_storage_ready_proof_valid() -> bool {
    let Some(server_pid) =
        crate::process::unique_live_process_id_for_image(UserImageId::StorageServer)
    else {
        return false;
    };
    let broker = bndroid_kernel::storage_broker::snapshot();
    let io = crate::storage_server_io::snapshot();
    let classified_completions = io
        .reads
        .checked_add(io.writes)
        .and_then(|total| total.checked_add(io.flushes))
        .and_then(|total| total.checked_add(io.errors));

    broker.bound
        && broker.owner_pid == server_pid
        && broker.epoch == 2
        && broker.next_epoch == 3
        && broker.next_token == broker.submissions.checked_add(1).unwrap_or(0)
        && broker.next_session_id == 5
        && broker.pending_sessions == 0
        && broker.state == 0
        && broker.submissions != 0
        && broker.submissions == broker.completions
        && broker.completions == broker.retrievals
        && broker.releases == 1
        && broker.abandoned == 0
        && !broker.abandoned_running
        && io.reads != 0
        && io.errors == 0
        && io.abandoned_services == 0
        && io.completions == broker.completions
        && classified_completions == Some(io.completions)
}

#[cfg(all(
    feature = "storage-server-recovery-runtime",
    not(feature = "storage-server-repeated-recovery-runtime")
))]
fn m56_storage_ready_proof_valid() -> bool {
    let Some(server_pid) =
        crate::process::unique_live_process_id_for_image(UserImageId::StorageServer)
    else {
        return false;
    };
    let processes = crate::process::snapshot();
    let broker = bndroid_kernel::storage_broker::snapshot();
    let io = crate::storage_server_io::snapshot();
    let classified_completions = io
        .reads
        .checked_add(io.writes)
        .and_then(|total| total.checked_add(io.flushes))
        .and_then(|total| total.checked_add(io.errors));

    processes.created == 5
        && processes.exited == 3
        && processes.reaped == 3
        && processes.live == 2
        && processes.peak_live == 2
        && !processes.completion_pending
        && processes.storage_server_live == 1
        && processes.storage_server_handles == 2
        && processes.total_handles == 3
        && processes.child_spawn_images[..4] == [UserImageId::StorageServer.raw(); 4]
        && processes.child_spawn_images[4..]
            .iter()
            .all(|image| *image == 0)
        && processes.child_spawn_pids[3] == server_pid
        && broker.bound
        && broker.owner_pid == server_pid
        && broker.epoch == 4
        && broker.next_epoch == 5
        && broker.next_token == broker.submissions.checked_add(1).unwrap_or(0)
        && broker.next_session_id == 1
        && broker.pending_sessions == 0
        && broker.state == 0
        && broker.submissions == broker.completions
        && broker.completions == broker.retrievals
        && broker.releases == 3
        && broker.abandoned == 3
        && !broker.abandoned_running
        && io.errors == 3
        && io.abandoned_services == 0
        && io.outcome_unknown_completions == 2
        && io.requires_reset_completions == 1
        && io.fault_control_sequences == 3
        && io.fault_control_state == 0
        && io.fault_control_epoch == 0
        && io.fault_sequence == 0x1b
        && io.fault_injected_reads == 1
        && io.fault_injected_writes == 1
        && io.fault_injected_flushes == 1
        && io.recovery_attempts == 3
        && io.recovery_successes == 3
        && io.recovery_failures == 0
        && io.recovery_rearm_aborts == 0
        && io.recovery_fail_closed_retries == 0
        && !io.recovery_retry_pending
        && io.completions == broker.completions
        && classified_completions == Some(io.completions)
}

#[cfg(all(
    feature = "storage-server-repeated-recovery-runtime",
    not(feature = "storage-server-async-recovery-runtime")
))]
fn m57_storage_ready_proof_valid() -> bool {
    let Some(server_pid) =
        crate::process::unique_live_process_id_for_image(UserImageId::StorageServer)
    else {
        return false;
    };
    let processes = crate::process::snapshot();
    let broker = bndroid_kernel::storage_broker::snapshot();
    let io = crate::storage_server_io::snapshot();
    let classified_completions = io
        .reads
        .checked_add(io.writes)
        .and_then(|total| total.checked_add(io.flushes))
        .and_then(|total| total.checked_add(io.errors));

    processes.created == 8
        && processes.exited == 6
        && processes.reaped == 6
        && processes.live == 2
        && processes.peak_live == 2
        && !processes.completion_pending
        && processes.storage_server_live == 1
        && processes.storage_server_handles == 2
        && processes.total_handles == 3
        && processes.child_spawn_images[..7] == [UserImageId::StorageServer.raw(); 7]
        && processes.child_spawn_images[7..]
            .iter()
            .all(|image| *image == 0)
        && processes.child_spawn_pids[6] == server_pid
        && broker.bound
        && broker.owner_pid == server_pid
        && broker.epoch == 7
        && broker.next_epoch == 8
        && broker.next_token == broker.submissions.checked_add(1).unwrap_or(0)
        && broker.next_session_id == 1
        && broker.pending_sessions == 0
        && broker.state == 0
        && broker.submissions == broker.completions
        && broker.completions == broker.retrievals
        && broker.releases == 6
        && broker.abandoned == 6
        && !broker.abandoned_running
        && io.errors == 6
        && io.abandoned_services == 0
        && io.outcome_unknown_completions == 4
        && io.requires_reset_completions == 2
        && io.fault_control_sequences == 6
        && io.fault_control_state == 0
        && io.fault_control_epoch == 0
        && io.fault_sequence == 0x6db
        && io.fault_injected_reads == 2
        && io.fault_injected_writes == 2
        && io.fault_injected_flushes == 2
        && io.recovery_attempts == 7
        && io.recovery_successes == 6
        && io.recovery_failures == 1
        && io.recovery_rearm_aborts == 1
        && io.recovery_fail_closed_retries == 1
        && !io.recovery_retry_pending
        && io.completions == broker.completions
        && classified_completions == Some(io.completions)
}

#[cfg(all(
    feature = "storage-server-async-recovery-runtime",
    not(feature = "storage-server-fault-policy-runtime")
))]
fn m58_storage_ready_proof_valid() -> bool {
    let Some(server_pid) =
        crate::process::unique_live_process_id_for_image(UserImageId::StorageServer)
    else {
        return false;
    };
    let processes = crate::process::snapshot();
    let broker = bndroid_kernel::storage_broker::snapshot();
    let io = crate::storage_server_io::snapshot();
    let recovery = crate::storage::async_recovery_snapshot();
    let scheduler = crate::scheduler::snapshot();
    let classified_completions = io
        .reads
        .checked_add(io.writes)
        .and_then(|total| total.checked_add(io.flushes))
        .and_then(|total| total.checked_add(io.errors));

    processes.created == 8
        && processes.exited == 6
        && processes.reaped == 6
        && processes.live == 2
        && processes.peak_live == 2
        && !processes.completion_pending
        && processes.storage_server_live == 1
        && processes.storage_server_handles == 2
        && processes.total_handles == 3
        && processes.child_spawn_images[..7] == [UserImageId::StorageServer.raw(); 7]
        && processes.child_spawn_images[7..]
            .iter()
            .all(|image| *image == 0)
        && processes.child_spawn_pids[6] == server_pid
        && broker.bound
        && broker.owner_pid == server_pid
        && broker.epoch == 7
        && broker.next_epoch == 8
        && broker.next_token == broker.submissions.checked_add(1).unwrap_or(0)
        && broker.next_session_id == 1
        && broker.pending_sessions == 0
        && broker.state == 0
        && broker.submissions == broker.completions
        && broker.completions == broker.retrievals
        && broker.releases == 6
        && broker.abandoned == 6
        && !broker.abandoned_running
        && io.errors == 6
        && io.abandoned_services == 0
        && io.outcome_unknown_completions == 4
        && io.requires_reset_completions == 2
        && io.fault_control_sequences == 6
        && io.fault_control_state == 0
        && io.fault_control_epoch == 0
        && io.fault_sequence == 0x6db
        && io.fault_injected_reads == 2
        && io.fault_injected_writes == 2
        && io.fault_injected_flushes == 2
        && io.recovery_attempts == 7
        && io.recovery_successes == 6
        && io.recovery_failures == 1
        && io.recovery_rearm_aborts == 1
        && io.recovery_fail_closed_retries == 1
        && !io.recovery_retry_pending
        && io.async_recovery_starts == 7
        && io.async_physical_completions == 7
        && !io.async_coordinator_active
        && io.async_active_attempt == 0
        && io.async_timer_progress_windows == 7
        && io.async_worker_progress_windows == 7
        && io.async_el0_progress_windows == 7
        && io.async_acquire_waits >= 14
        && io.async_acquire_dispatch_changes >= 14
        && !recovery.active
        && recovery.phase == 0
        && recovery.steps
            == recovery
                .pending_returns
                .checked_add(io.async_physical_completions)
                .unwrap_or(0)
        && recovery.pending_returns >= 21
        && recovery.masked_poll_iterations == 0
        && scheduler.timer_period != 0
        && recovery.max_masked_counter_ticks < scheduler.timer_period
        && io.async_max_control_masked_ticks < scheduler.timer_period
        && !crate::storage::recovery_admission_closed()
        && io.completions == broker.completions
        && classified_completions == Some(io.completions)
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
fn m60_storage_ready_proof_valid() -> bool {
    use bndroid_kernel::storage_recovery_policy::RecoveryState;

    let processes = crate::process::snapshot();
    let broker = bndroid_kernel::storage_broker::snapshot();
    let io = crate::storage_server_io::snapshot();
    let policy = crate::storage_server_io::fault_policy_snapshot();
    let recovery = crate::storage::async_recovery_snapshot();
    let persistent = crate::storage::persistent_rebuild_fault_snapshot()
        .unwrap_or_else(|_| panic!("M60 could not inspect its persistent fault latch"));
    let terminal = crate::storage::terminal_dma_snapshot()
        .unwrap_or_else(|_| panic!("M60 could not inspect its terminal DMA state"));
    let irq = crate::storage::irq_snapshot();
    #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
    let terminal_proof = crate::storage_server_io::terminal_proof_gate_snapshot();
    #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
    let terminal_quarantine_valid = !io.terminal_quarantine_active
        && io.terminal_quarantine_starts == 1
        && io.terminal_quarantine_completions == 0
        && io.terminal_quarantine_physical_errors == 1
        && io.terminal_quarantine_steps == 3
        && io.terminal_quarantine_pending_returns == 2
        && io.terminal_quarantine_timer_progress_windows == 1
        && io.terminal_quarantine_worker_progress_windows == 1
        && io.terminal_irq_rollbacks == 2
        && io.terminal_dma_verifications == 1
        && terminal_proof.phase == bndroid_kernel::storage_terminal_quarantine::Phase::Consumed
        && terminal_proof.arms == 1
        && terminal_proof.proof_deferrals == 1
        && terminal_proof.unsafe_observations == 0
        && terminal_proof.publications == 1
        && terminal_proof.invariant_errors == 0;
    #[cfg(not(feature = "storage-server-terminal-quarantine-runtime"))]
    let terminal_quarantine_valid = !io.terminal_quarantine_active
        && io.terminal_quarantine_starts == 0
        && io.terminal_quarantine_completions == 0
        && io.terminal_quarantine_physical_errors == 0
        && io.terminal_irq_rollbacks == 1
        && io.terminal_dma_verifications == 1;
    #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
    let terminal_async_outcomes = io
        .terminal_quarantine_completions
        .checked_add(io.terminal_quarantine_physical_errors)
        .unwrap_or(0);
    #[cfg(not(feature = "storage-server-terminal-quarantine-runtime"))]
    let terminal_async_outcomes = 0;
    let classified_completions = io
        .reads
        .checked_add(io.writes)
        .and_then(|total| total.checked_add(io.flushes))
        .and_then(|total| total.checked_add(io.errors));
    let spawn_images_valid = processes.child_spawn_images[..8]
        == [UserImageId::StorageServer.raw(); 8]
        && processes.child_spawn_images[8..]
            .iter()
            .all(|image| *image == 0)
        && processes.child_spawn_pids[..8].iter().all(|pid| *pid != 0)
        && processes.child_spawn_pids[8..].iter().all(|pid| *pid == 0);
    let generation_chain_valid = processes.child_spawn_pids[..8].windows(2).all(|pair| {
        pair[0] as u32 == pair[1] as u32
            && (pair[1] >> 32) == (pair[0] >> 32).checked_add(1).unwrap_or(0)
    });

    spawn_images_valid
        && generation_chain_valid
        && processes.created == 9
        && processes.exited == 8
        && processes.reaped == 8
        && processes.live == 1
        && processes.peak_live == 2
        && !processes.completion_pending
        && processes.storage_server_live == 0
        && processes.storage_server_handles == 0
        && processes.total_handles == 0
        && !broker.bound
        && broker.owner_pid == 0
        && broker.epoch == 0
        && broker.next_epoch == 8
        && broker.next_token == broker.submissions.checked_add(1).unwrap_or(0)
        && broker.next_session_id == 1
        && broker.pending_sessions == 0
        && broker.state == 4
        && broker.submissions == broker.completions
        && broker.completions == broker.retrievals
        && broker.releases == 7
        && broker.abandoned == 7
        && !broker.abandoned_running
        && broker.device_offline
        && broker.offline_transitions == 1
        && broker.offline_acquire_denials == 1
        && io.errors == 7
        && io.abandoned_services == 0
        && io.outcome_unknown_completions == 4
        && io.requires_reset_completions == 3
        && io.fault_control_sequences == 6
        && io.fault_control_state == 0
        && io.fault_control_epoch == 0
        && io.fault_sequence == 0x1b6e
        && io.fault_injected_reads == 3
        && io.fault_injected_writes == 2
        && io.fault_injected_flushes == 2
        && io.kernel_permanent_owner_requests == 1
        && io.kernel_permanent_fault_arms == 1
        && io.kernel_permanent_reads == 1
        && io.probation_io_failures == 1
        && io.recovery_attempts == 9
        && io.recovery_successes == 6
        && io.recovery_failures == 3
        && io.recovery_rearm_aborts == 1
        && io.recovery_fail_closed_retries == 1
        && !io.recovery_retry_pending
        && io.async_recovery_starts == 9
        && io.async_physical_completions == 7
        && io.async_physical_failures == 2
        && !io.async_coordinator_active
        && io.async_active_attempt == 0
        && terminal_quarantine_valid
        && policy.state == RecoveryState::Offline
        && policy.active_attempt_generation == 0
        && policy.active_attempt_ordinal == 0
        && policy.consecutive_failures == 3
        && policy.base_backoff_ticks == 2
        && policy.backoff_multiplier == 2
        && policy.attempt_limit == 3
        && policy.attempts_started == 9
        && policy.physical_successes == 7
        && policy.physical_failures == 2
        && policy.attempt_failures == 4
        && policy.backoffs_scheduled == 3
        && policy.backoffs_completed == 3
        && policy.total_backoff_ticks == 8
        && policy.probation_entries == 7
        && policy.probation_successes == 5
        && policy.probation_failures == 2
        && policy.healthy_transitions == 5
        && policy.offline_transitions == 1
        && policy.stale_ticket_rejections == 0
        && policy.invalid_transition_rejections == 0
        && policy.generation_exhaustions == 0
        && persistent.armed
        && persistent.hits
            == if cfg!(feature = "storage-server-terminal-quarantine-runtime") {
                3
            } else {
                2
            }
        && terminal.driver_state == 2
        && terminal.transport_status == 0
        && terminal.in_flight == 0
        && !terminal.recovery_active
        && terminal.recovery_scratch_clear
        && terminal.requests_terminal
        && !recovery.active
        && recovery.phase == 0
        && recovery.steps
            == recovery
                .pending_returns
                .checked_add(io.async_physical_completions)
                .and_then(|steps| steps.checked_add(io.async_physical_failures))
                .and_then(|steps| steps.checked_add(terminal_async_outcomes))
                .unwrap_or(0)
        && recovery.masked_poll_iterations == 0
        && !irq.armed
        && irq.failed
        && !irq.rearm_prepared
        && irq.recovery_required
        && crate::storage::recovery_admission_closed()
        && io.completions == broker.completions
        && classified_completions == Some(io.completions)
}

fn complete(frame: *mut TrapFrame, status: Status, out1: u64, out2: u64) -> *mut TrapFrame {
    if status == Status::ShouldWait {
        SHOULD_WAIT_RETURNS.fetch_add(1, Ordering::Relaxed);
    }
    if crate::process::current_is_init() {
        if status == Status::Ok {
            SUCCESSES.fetch_add(1, Ordering::Relaxed);
        } else {
            ERRORS.fetch_add(1, Ordering::Relaxed);
        }
    }
    unsafe {
        (*frame).x[0] = status.raw();
        (*frame).x[1] = out1;
        (*frame).x[2] = out2;
    }
    frame
}

fn record_user_asid() {
    let current = crate::arch::aarch64::mmu::current_translation_context();
    let expected = crate::scheduler::current_user_translation()
        .unwrap_or_else(|| panic!("EL0 syscall entered without a scheduler translation"));
    if current != expected {
        panic!("EL0 syscall entered under the wrong translation context");
    }
    if crate::process::current_is_init() {
        USER_ASID.store(u64::from(current.asid()), Ordering::Relaxed);
    }
}

fn record_call() {
    if crate::process::current_is_init() {
        CALLS.fetch_add(1, Ordering::Relaxed);
    } else {
        crate::process::record_child_syscall();
    }
}

fn authenticated_sender_pid() -> u64 {
    crate::process::current_live_user_process_id()
        .unwrap_or_else(|| panic!("EL0 channel write lacked a live generation-qualified process"))
}

fn parse_handle(raw: u64) -> Option<HandleValue> {
    let raw = u32::try_from(raw).ok()?;
    let handle = HandleValue::from_raw(raw);
    handle.is_valid().then_some(handle)
}

fn handle_error_status(error: HandleError) -> Status {
    match error {
        HandleError::InvalidHandle => Status::NotFound,
        HandleError::AccessDenied | HandleError::RightsEscalation => Status::PermissionDenied,
        HandleError::TableFull => Status::OutOfMemory,
    }
}

fn channel_error_status(error: ChannelError) -> Status {
    match error {
        ChannelError::ShouldWait => Status::ShouldWait,
        ChannelError::PeerClosed => Status::PeerClosed,
    }
}

fn with_table<R>(
    operation: impl FnOnce(&mut HandleTable<KernelObject, HANDLE_CAPACITY>) -> R,
) -> R {
    assert!(crate::arch::aarch64::irq_is_masked());
    crate::process::with_current_handles(operation)
}

fn app_lifecycle_trace_mut() -> &'static mut AppLifecycleTraceState {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("app lifecycle trace accessed with IRQ enabled");
    }
    unsafe { &mut *APP_LIFECYCLE_TRACE.0.get() }
}

#[cfg(feature = "androidbox-process0")]
fn android_app_rpc_trace_mut() -> &'static mut AndroidAppRpcTraceState {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("AndroidApp RPC trace accessed with IRQ enabled");
    }
    unsafe { &mut *ANDROID_APP_RPC_TRACE.0.get() }
}

#[cfg(feature = "androidbox-restart0")]
fn android_app_restart_trace_mut() -> &'static mut AndroidAppRestartTraceState {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("AndroidApp restart trace accessed with IRQ enabled");
    }
    unsafe { &mut *ANDROID_APP_RESTART_TRACE.0.get() }
}

fn app_lifecycle_trace_snapshot() -> AppLifecycleTraceSnapshot {
    let trace = app_lifecycle_trace_mut();
    AppLifecycleTraceSnapshot {
        messages: trace.messages,
        errors: trace.errors,
        decode_errors: trace.decode_errors,
        authentication_errors: trace.authentication_errors,
        tracker_errors: trace.tracker_errors,
        byte_messages: trace.byte_messages,
        transfer_messages: trace.transfer_messages,
        requests: trace.requests,
        state_changes: trace.state_changes,
        crashes: trace.crashes,
        commands: trace.commands,
        acks: trace.acks,
        request_actions: trace.request_actions,
        command_actions: trace.command_actions,
        ack_actions: trace.ack_actions,
        transactions: trace.transactions,
        completed: trace.completed,
        first_transaction_id: trace.first_transaction_id,
        last_transaction_id: trace.last_transaction_id,
        first_instance_id: trace.first_instance_id,
        first_app_pid: trace.first_app_pid,
        last_instance_id: trace.last_instance_id,
        last_app_pid: trace.last_app_pid,
        first_sender_pid: trace.first_sender_pid,
        last_sender_pid: trace.last_sender_pid,
    }
}

fn ui_supervisor_trace_mut() -> &'static mut UiSupervisorTraceState {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("UI supervisor trace accessed with IRQ enabled");
    }
    unsafe { &mut *UI_SUPERVISOR_TRACE.0.get() }
}

fn ui_supervisor_trace_snapshot() -> UiSupervisorTraceSnapshot {
    let trace = ui_supervisor_trace_mut();
    UiSupervisorTraceSnapshot {
        messages: trace.messages,
        errors: trace.errors,
        decode_errors: trace.decode_errors,
        authentication_errors: trace.authentication_errors,
        tracker_errors: trace.tracker_errors,
        byte_messages: trace.byte_messages,
        transfer_messages: trace.transfer_messages,
        commands: trace.commands,
        acks: trace.acks,
        owner_deaths: trace.owner_deaths,
        operations: trace.operations,
        transactions: trace.transactions,
        completed: trace.completed,
        first_transaction_id: trace.first_transaction_id,
        last_transaction_id: trace.last_transaction_id,
        first_sender_pid: trace.first_sender_pid,
        last_sender_pid: trace.last_sender_pid,
    }
}

fn reset_counters() {
    CALLS.store(0, Ordering::Relaxed);
    SUCCESSES.store(0, Ordering::Relaxed);
    ERRORS.store(0, Ordering::Relaxed);
    UNKNOWN.store(0, Ordering::Relaxed);
    CHANNELS_CREATED.store(0, Ordering::Relaxed);
    WRITES.store(0, Ordering::Relaxed);
    READS.store(0, Ordering::Relaxed);
    CHANNEL_PEEKS.store(0, Ordering::Relaxed);
    CHANNEL_PEEK_SCALAR.store(0, Ordering::Relaxed);
    CHANNEL_PEEK_BYTES.store(0, Ordering::Relaxed);
    CHANNEL_PEEK_TRANSFER.store(0, Ordering::Relaxed);
    CHANNEL_ENVELOPE_READS.store(0, Ordering::Relaxed);
    CHANNEL_ENVELOPE_SCALAR.store(0, Ordering::Relaxed);
    CHANNEL_ENVELOPE_BYTES.store(0, Ordering::Relaxed);
    CHANNEL_ENVELOPE_TRANSFER.store(0, Ordering::Relaxed);
    CHANNEL_ENVELOPE_PRESERVED_REJECTIONS.store(0, Ordering::Relaxed);
    CHANNEL_ENVELOPE_BUFFER_TOO_SMALL.store(0, Ordering::Relaxed);
    CHANNEL_ENVELOPE_BAD_ADDRESS.store(0, Ordering::Relaxed);
    CHANNEL_ENVELOPE_TABLE_FULL.store(0, Ordering::Relaxed);
    DUPLICATES.store(0, Ordering::Relaxed);
    CLOSES.store(0, Ordering::Relaxed);
    STALE_REJECTIONS.store(0, Ordering::Relaxed);
    RIGHTS_DENIALS.store(0, Ordering::Relaxed);
    LAST_TAG.store(0, Ordering::Relaxed);
    LAST_PAYLOAD.store(0, Ordering::Relaxed);
    INIT_READY.store(false, Ordering::Relaxed);
    INIT_FAILURE.store(0, Ordering::Relaxed);
    PAN_FAILURES.store(0, Ordering::Relaxed);
    PRIVATE_SVC_REJECTIONS.store(0, Ordering::Relaxed);
    SHOULD_WAIT_RETURNS.store(0, Ordering::Relaxed);
    REGISTER_PRESERVED.store(false, Ordering::Relaxed);
    STACK_ROUND_TRIP.store(false, Ordering::Relaxed);
    USER_ASID.store(0, Ordering::Relaxed);
    BYTE_WRITES.store(0, Ordering::Relaxed);
    BYTE_READS.store(0, Ordering::Relaxed);
    BYTE_READ_ROLLBACKS.store(0, Ordering::Relaxed);
    BYTE_BUFFER_TOO_SMALL.store(0, Ordering::Relaxed);
    BYTE_ZERO_LENGTH.store(0, Ordering::Relaxed);
    TRANSFER_WRITES.store(0, Ordering::Relaxed);
    TRANSFER_READS.store(0, Ordering::Relaxed);
    TRANSFER_READ_ROLLBACKS.store(0, Ordering::Relaxed);
    TRANSFER_SELF_REJECTIONS.store(0, Ordering::Relaxed);
    TRANSFER_ORDER_REJECTIONS.store(0, Ordering::Relaxed);
    OBJECT_WAIT_CALLS.store(0, Ordering::Relaxed);
    OBJECT_WAIT_IMMEDIATE.store(0, Ordering::Relaxed);
    WAIT_MANY_CALLS.store(0, Ordering::Relaxed);
    WAIT_MANY_IMMEDIATE.store(0, Ordering::Relaxed);
    WAIT_MANY_POLL_TIMEOUTS.store(0, Ordering::Relaxed);
    WAIT_ARRAY_CALLS.store(0, Ordering::Relaxed);
    WAIT_ARRAY_IMMEDIATE.store(0, Ordering::Relaxed);
    WAIT_ARRAY_POLL_TIMEOUTS.store(0, Ordering::Relaxed);
    WAIT_ARRAY_INVALID_COUNTS.store(0, Ordering::Relaxed);
    WAIT_ARRAY_BAD_ADDRESSES.store(0, Ordering::Relaxed);
    WAIT_ARRAY_VALIDATION_REJECTIONS.store(0, Ordering::Relaxed);
    WAIT_ARRAY_MAX_ITEMS_OBSERVED.store(0, Ordering::Relaxed);
    WAIT_ARRAY_LOWEST_MULTI_READY.store(0, Ordering::Relaxed);
    EVENTS_CREATED.store(0, Ordering::Relaxed);
    EVENT_SIGNAL_CALLS.store(0, Ordering::Relaxed);
    EVENT_SIGNAL_EDGES.store(0, Ordering::Relaxed);
    EVENT_CLEAR_CALLS.store(0, Ordering::Relaxed);
    EVENT_CLEAR_EDGES.store(0, Ordering::Relaxed);
    EVENT_SIGNAL_DENIALS.store(0, Ordering::Relaxed);
    EVENT_WAIT_CALLS.store(0, Ordering::Relaxed);
    EVENT_WAIT_BLOCKS.store(0, Ordering::Relaxed);
    EVENT_WAIT_WAKES.store(0, Ordering::Relaxed);
    EVENT_TRANSFER_WRITES.store(0, Ordering::Relaxed);
    EVENT_TRANSFER_READS.store(0, Ordering::Relaxed);
    FILE_OPEN_CALLS.store(0, Ordering::Relaxed);
    FILE_OPEN_SUCCESSES.store(0, Ordering::Relaxed);
    VMO_READ_CALLS.store(0, Ordering::Relaxed);
    VMO_READ_SUCCESSES.store(0, Ordering::Relaxed);
    VMO_READ_BYTES.store(0, Ordering::Relaxed);
    VMO_TRANSFER_WRITES.store(0, Ordering::Relaxed);
    VMO_TRANSFER_READS.store(0, Ordering::Relaxed);
    #[cfg(feature = "unified-product-manifest-supervision-runtime")]
    {
        SERVICE_MANIFEST_OPEN_CALLS.store(0, Ordering::Relaxed);
        SERVICE_MANIFEST_OPEN_SUCCESSES.store(0, Ordering::Relaxed);
        SERVICE_MANIFEST_OPEN_ARGUMENT_REJECTIONS.store(0, Ordering::Relaxed);
        SERVICE_MANIFEST_OPEN_PERMISSION_DENIALS.store(0, Ordering::Relaxed);
    }
    #[cfg(all(
        feature = "unified-product-verified-manifest-runtime",
        not(feature = "unified-product-persistent-rollback-runtime")
    ))]
    {
        VERIFIED_MANIFEST_ATTEMPTS.store(0, Ordering::Relaxed);
        VERIFIED_MANIFEST_SUCCESSES.store(0, Ordering::Relaxed);
        VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.store(0, Ordering::Relaxed);
        VERIFIED_MANIFEST_SIGNATURE_REJECTIONS.store(0, Ordering::Relaxed);
        VERIFIED_MANIFEST_ROLLBACK_REJECTIONS.store(0, Ordering::Relaxed);
        VERIFIED_MANIFEST_FORMAT_REJECTIONS.store(0, Ordering::Relaxed);
        VERIFIED_MANIFEST_ROLLBACK_INDEX.store(0, Ordering::Relaxed);
        VERIFIED_MANIFEST_DIGEST_0.store(0, Ordering::Relaxed);
        VERIFIED_MANIFEST_DIGEST_1.store(0, Ordering::Relaxed);
        VERIFIED_MANIFEST_DIGEST_2.store(0, Ordering::Relaxed);
        VERIFIED_MANIFEST_DIGEST_3.store(0, Ordering::Relaxed);
    }
    #[cfg(feature = "unified-product-key-rotation-runtime")]
    {
        let rotation = key_rotation_manifest_snapshot();
        if !rotation.prepared
            || rotation.ledger_attempts != 1
            || rotation.ledger_successes != 1
            || rotation.ledger_rollback_rejections != 0
            || rotation.ledger_failures != 0
            || rotation.retired_key_rejections != 0
            || VERIFIED_MANIFEST_ATTEMPTS.load(Ordering::Acquire) != 1
            || VERIFIED_MANIFEST_SUCCESSES.load(Ordering::Acquire) != 1
            || VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.load(Ordering::Acquire) != 1
            || VERIFIED_MANIFEST_SIGNATURE_REJECTIONS.load(Ordering::Acquire) != 0
            || VERIFIED_MANIFEST_ROLLBACK_REJECTIONS.load(Ordering::Acquire) != 0
            || VERIFIED_MANIFEST_FORMAT_REJECTIONS.load(Ordering::Acquire) != 0
        {
            panic!("syscall activation preceded successful key-rotation manifest preparation");
        }
    }
    #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
    {
        let maintenance = maintenance_authorization_snapshot();
        if !maintenance.prepared
            || maintenance.authorization_attempts != 1
            || maintenance.authorization_successes != 1
            || maintenance.signature_rejections != 0
            || maintenance.binding_rejections != 0
            || maintenance.audit_attempts != 1
            || maintenance.audit_successes != 1
            || maintenance.audit_replay_rejections != 0
            || maintenance.audit_failures != 0
        {
            panic!("syscall activation preceded successful maintenance authorization audit");
        }
        MAINTENANCE_SESSION_OPEN_CALLS.store(0, Ordering::Relaxed);
        MAINTENANCE_SESSION_OPEN_SUCCESSES.store(0, Ordering::Relaxed);
        MAINTENANCE_SESSION_OPEN_ARGUMENT_REJECTIONS.store(0, Ordering::Relaxed);
        MAINTENANCE_SESSION_OPEN_PERMISSION_REJECTIONS.store(0, Ordering::Relaxed);
        MAINTENANCE_SESSION_OPEN_STATE_REJECTIONS.store(0, Ordering::Relaxed);
        MAINTENANCE_SESSION_OPENED.store(false, Ordering::Relaxed);
        MAINTENANCE_REPORT_GATE_DENIALS.store(0, Ordering::Relaxed);
    }
    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    {
        let signed = signed_maintenance_plan_snapshot();
        if signed.attempts != 1
            || signed.successes != 1
            || signed.signature_rejections != 0
            || signed.binding_rejections != 0
            || signed.program_rejections != 0
            || signed.persistence_failures != 0
            || signed.preparation.is_none()
        {
            panic!("syscall activation preceded successful signed maintenance-plan preparation");
        }
    }
    #[cfg(all(
        feature = "unified-product-persistent-rollback-runtime",
        not(feature = "unified-product-key-rotation-runtime")
    ))]
    {
        let persistent = persistent_manifest_rollback_snapshot();
        if !persistent.prepared
            || persistent.ledger_attempts != 1
            || persistent.ledger_successes != 1
            || persistent.ledger_rollback_rejections != 0
            || persistent.ledger_failures != 0
            || VERIFIED_MANIFEST_ATTEMPTS.load(Ordering::Acquire) != 1
            || VERIFIED_MANIFEST_SUCCESSES.load(Ordering::Acquire) != 1
            || VERIFIED_MANIFEST_SIGNATURE_SUCCESSES.load(Ordering::Acquire) != 1
            || VERIFIED_MANIFEST_SIGNATURE_REJECTIONS.load(Ordering::Acquire) != 0
            || VERIFIED_MANIFEST_ROLLBACK_REJECTIONS.load(Ordering::Acquire) != 0
            || VERIFIED_MANIFEST_FORMAT_REJECTIONS.load(Ordering::Acquire) != 0
        {
            panic!("syscall activation preceded successful persistent manifest preparation");
        }
    }
    #[cfg(feature = "app-data-runtime")]
    {
        APP_DATA_ROOT_CALLS.store(0, Ordering::Relaxed);
        APP_DATA_ROOT_SUCCESSES.store(0, Ordering::Relaxed);
        APP_DATA_FILE_OPEN_CALLS.store(0, Ordering::Relaxed);
        APP_DATA_FILE_OPEN_SUCCESSES.store(0, Ordering::Relaxed);
        APP_DATA_REPLACE_CALLS.store(0, Ordering::Relaxed);
        APP_DATA_REPLACE_SUCCESSES.store(0, Ordering::Relaxed);
        APP_DATA_DIRECTORY_CREATE_CALLS.store(0, Ordering::Relaxed);
        APP_DATA_DIRECTORY_CREATE_SUCCESSES.store(0, Ordering::Relaxed);
        APP_DATA_UNLINK_CALLS.store(0, Ordering::Relaxed);
        APP_DATA_UNLINK_SUCCESSES.store(0, Ordering::Relaxed);
        APP_DATA_DIRECTORY_READ_CALLS.store(0, Ordering::Relaxed);
        APP_DATA_DIRECTORY_READ_SUCCESSES.store(0, Ordering::Relaxed);
        APP_DATA_DIRECTORY_READ_EOF.store(0, Ordering::Relaxed);
        APP_DATA_PERMISSION_DENIALS.store(0, Ordering::Relaxed);
        APP_DATA_VALIDATION_REJECTIONS.store(0, Ordering::Relaxed);
        APP_DATA_SHOULD_WAITS.store(0, Ordering::Relaxed);
        APP_DATA_CONFLICTS.store(0, Ordering::Relaxed);
        APP_DATA_OUTCOME_UNKNOWN.store(0, Ordering::Relaxed);
        APP_DATA_CORRUPTION_ERRORS.store(0, Ordering::Relaxed);
        APP_DATA_NO_SPACE.store(0, Ordering::Relaxed);
    }
    GRAPHICS_BUFFERS_CREATED.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_CREATE_EXHAUSTIONS.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_WRITE_CALLS.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_WRITE_BYTES.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_PRESENTS.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_MAPPABLE_CREATED.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_MAP_CALLS.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_MAP_SUCCESSES.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_UNMAP_CALLS.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_UNMAP_SUCCESSES.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_QUEUE_CALLS.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_QUEUE_SUCCESSES.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_ACQUIRE_CALLS.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_ACQUIRE_SUCCESSES.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_RELEASE_CALLS.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_RELEASE_SUCCESSES.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_RELEASES.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_MAPPED_PRESENTS.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_VALIDATED_PIXELS.store(0, Ordering::Relaxed);
    GRAPHICS_BUFFER_VALIDATED_BYTES.store(0, Ordering::Relaxed);
    SURFACE_REACQUIRES.store(0, Ordering::Relaxed);
    SURFACE_REACQUIRE_OLD_PID.store(0, Ordering::Relaxed);
    SURFACE_REACQUIRE_NEW_PID.store(0, Ordering::Relaxed);
    SURFACE_REACQUIRE_OLD_SESSION.store(0, Ordering::Relaxed);
    SURFACE_REACQUIRE_NEW_SESSION.store(0, Ordering::Relaxed);
    SURFACE_REACQUIRE_DISCARDED_INPUT.store(0, Ordering::Relaxed);
    SURFACE_REACQUIRE_DISCARDED_KEYS.store(0, Ordering::Relaxed);
    SURFACE_REACQUIRE_FROZEN_CURSOR_X.store(0, Ordering::Relaxed);
    SURFACE_REACQUIRE_FROZEN_CURSOR_Y.store(0, Ordering::Relaxed);
    SURFACE_REACQUIRE_FROZEN_CURSOR_PIXELS.store(0, Ordering::Relaxed);
    SURFACE_REACQUIRE_FROZEN_CURSOR_VISIBLE.store(false, Ordering::Relaxed);
    SURFACE_REACQUIRE_FROZEN_CURSOR_PRESSED.store(false, Ordering::Relaxed);
    SURFACE_REACQUIRE_FROZEN_SCANOUT_VALID.store(false, Ordering::Relaxed);
    SURFACE_FRAME_ACQUIRE_CALLS.store(0, Ordering::Relaxed);
    SURFACE_FRAME_ACQUIRE_SUCCESSES.store(0, Ordering::Relaxed);
    SURFACE_FRAME_ACQUIRE_SHOULD_WAIT.store(0, Ordering::Relaxed);
    SURFACE_FRAME_ACQUIRE_INVALID_STATE.store(0, Ordering::Relaxed);
    SURFACE_KEY_READ_CALLS.store(0, Ordering::Relaxed);
    SURFACE_KEY_READ_SUCCESSES.store(0, Ordering::Relaxed);
    SURFACE_KEY_READ_SHOULD_WAIT.store(0, Ordering::Relaxed);
    SURFACE_KEY_READ_PEER_CLOSED.store(0, Ordering::Relaxed);
    SURFACE_KEY_READ_INVALID_ARGUMENT.store(0, Ordering::Relaxed);
    SURFACE_KEY_READ_PERMISSION_DENIED.store(0, Ordering::Relaxed);
    SURFACE_KEY_READ_INVALID_STATE.store(0, Ordering::Relaxed);
    INPUT_ACQUIRE_CALLS.store(0, Ordering::Relaxed);
    INPUT_ACQUIRE_SUCCESSES.store(0, Ordering::Relaxed);
    INPUT_ACQUIRE_PERMISSION_DENIED.store(0, Ordering::Relaxed);
    INPUT_SESSION_INFO_CALLS.store(0, Ordering::Relaxed);
    INPUT_SESSION_INFO_SUCCESSES.store(0, Ordering::Relaxed);
    INPUT_SESSION_INFO_PERMISSION_DENIED.store(0, Ordering::Relaxed);
    PROCESS_TERMINATE_CALLS.store(0, Ordering::Relaxed);
    PROCESS_TERMINATE_SUCCESSES.store(0, Ordering::Relaxed);
    PROCESS_TERMINATE_NOT_FOUND.store(0, Ordering::Relaxed);
    PROCESS_TERMINATE_INVALID_ARGUMENT.store(0, Ordering::Relaxed);
    PROCESS_TERMINATE_INVALID_STATE.store(0, Ordering::Relaxed);
    PROCESS_TERMINATE_PERMISSION_DENIED.store(0, Ordering::Relaxed);
    *app_lifecycle_trace_mut() = AppLifecycleTraceState::new();
    #[cfg(feature = "androidbox-process0")]
    {
        *android_app_rpc_trace_mut() = AndroidAppRpcTraceState::new();
    }
    #[cfg(feature = "androidbox-restart0")]
    {
        *android_app_restart_trace_mut() = AndroidAppRestartTraceState::new();
    }
    *ui_supervisor_trace_mut() = UiSupervisorTraceState::new();
    NEXT_SURFACE_SESSION_ID.store(1, Ordering::Relaxed);
    SERVICE_ACK_READS.store(0, Ordering::Relaxed);
    SERVICE_ACK_BITMAP.store(0, Ordering::Relaxed);
    SERVICE_DONE_READS.store(0, Ordering::Relaxed);
    SERVICE_DONE_BITMAP.store(0, Ordering::Relaxed);
    SERVICE_TRANSCRIPT_ERRORS.store(0, Ordering::Relaxed);
    M15_SERVICE_PHASE.store(0, Ordering::Relaxed);
    M15_SERVICE_ERRORS.store(0, Ordering::Relaxed);
    M15_SERVICE_OLD_INSTANCE.store(0, Ordering::Relaxed);
    M15_SERVICE_NEW_INSTANCE.store(0, Ordering::Relaxed);
    for transaction_id in &M15_SERVICE_TXIDS {
        transaction_id.store(0, Ordering::Relaxed);
    }
    M15_SERVICE_DONE_READS.store(0, Ordering::Relaxed);
    M15_SERVICE_DONE_BITMAP.store(0, Ordering::Relaxed);
    M16_SERVICE_CYCLE_ROUND.store(0, Ordering::Relaxed);
    M16_SERVICE_CYCLE_STEP.store(0, Ordering::Relaxed);
    M16_SERVICE_CYCLE_COMPLETED_ROUNDS.store(0, Ordering::Relaxed);
    M16_SERVICE_CYCLE_ERRORS.store(0, Ordering::Relaxed);
    M16_SERVICE_CYCLE_INSTANCE.store(0, Ordering::Relaxed);
    for transaction_id in &M16_SERVICE_CYCLE_TXIDS {
        transaction_id.store(0, Ordering::Relaxed);
    }
    M16_SERVICE_CYCLE_DONE_READS.store(0, Ordering::Relaxed);
    M16_SERVICE_CYCLE_DONE_BITMAP.store(0, Ordering::Relaxed);
    M17_IDENTITY_DONE_READS.store(0, Ordering::Relaxed);
    M17_ACL_DONE_READS.store(0, Ordering::Relaxed);
    M17_IDENTITY_ERRORS.store(0, Ordering::Relaxed);
    M17_PROVIDER_PID.store(0, Ordering::Relaxed);
    M17_CLIENT_PID.store(0, Ordering::Relaxed);
    M17_ACL_PAYLOAD.store(0, Ordering::Relaxed);
    M18_READY_BITMAP.store(0, Ordering::Relaxed);
    M18_QUEUED_BITMAP.store(0, Ordering::Relaxed);
    M18_DONE_BITMAP.store(0, Ordering::Relaxed);
    M18_ERRORS.store(0, Ordering::Relaxed);
    M18_SECONDARY_CLIENT_PID.store(0, Ordering::Relaxed);
    M18_REQUEST_ORDER.store(0, Ordering::Relaxed);
    M20_PHASE.store(0, Ordering::Relaxed);
    M20_ERRORS.store(0, Ordering::Relaxed);
    M20_SECONDARY_CLIENT_PID.store(0, Ordering::Relaxed);
    M20_MANAGER_ATTACHES.store(0, Ordering::Relaxed);
    M20_CLIENT_ATTACHES.store(0, Ordering::Relaxed);
    M20_REVOKE_BITMAP.store(0, Ordering::Relaxed);
    M20_STALE_REVOKE_BITMAP.store(0, Ordering::Relaxed);
    M20_PRIMARY_PROGRESS_BITMAP.store(0, Ordering::Relaxed);
    M20_PROVIDER_ACCEPTS.store(0, Ordering::Relaxed);
    M20_PROVIDER_ECHOES.store(0, Ordering::Relaxed);
    M20_PROVIDER_ABORTS.store(0, Ordering::Relaxed);
    M20_SECONDARY_ECHOES.store(0, Ordering::Relaxed);
    M20_FINAL_IDLE_BITMAP.store(0, Ordering::Relaxed);
}
