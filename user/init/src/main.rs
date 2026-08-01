use core::arch::asm;
#[cfg(feature = "mobile-ui-runtime")]
use core::cell::UnsafeCell;
use core::ptr;
use core::sync::atomic::{AtomicU64, Ordering};

#[cfg(all(
    feature = "androidbox-el0-runtime0",
    not(feature = "androidbox-interactive0")
))]
use crate::androidbox_installed_runtime::{
    InstalledActivityExecution, read_and_execute_installed_activity,
};
#[cfg(all(
    feature = "androidbox-interactive0",
    not(feature = "androidbox-process0")
))]
use crate::androidbox_installed_runtime::{
    InstalledInteractiveActivityLease, InstalledInteractiveActivitySnapshot,
};
#[cfg(all(feature = "androidbox-dex0", not(feature = "androidbox-apk-install0")))]
use crate::androidbox_runtime::{
    AndroidBoxActivityRun, AndroidBoxActivityViewState, AndroidBoxRun, AndroidBoxRuntime,
    AndroidBoxViewState,
};
#[cfg(feature = "mobile-ui-runtime")]
use bndr_abi::SYSTEM_CLOCK_SOURCE_QEMU_PL031;
#[cfg(feature = "unified-product-runtime")]
use bndr_abi::ShutdownServiceNode;
#[cfg(feature = "androidbox-interactive0")]
use bndr_abi::pack_surface_layer_handles;
use bndr_abi::{
    ABI_VERSION, CHANNEL_MESSAGE_MAX_BYTES, CHANNEL_READ_ENVELOPE_SIZE, ChannelMessageKind,
    ChannelReadEnvelope, GRAPHICS_BUFFER_CREATE_FLAGS_NONE, GRAPHICS_BUFFER_FORMAT_XRGB8888,
    GRAPHICS_BUFFER_HEIGHT, GRAPHICS_BUFFER_LOGICAL_BYTES, GRAPHICS_BUFFER_WIDTH,
    GRAPHICS_BUFFER_WRITE_MAX_BYTES, HandleValue, OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS,
    OBJECT_WAIT_TIMEOUT_INFINITE, OBJECT_WAIT_TIMEOUT_POLL, ObjectSignals,
    PROCESS_KILLED_EXIT_CODE, PROCESS_SPAWN_FLAGS_NONE, PROCESS_TERMINATE_FLAGS_NONE,
    ProcessTerminationReason, Rights, Status, SyscallNumber, UserImageId, VMO_READ_MAX_BYTES,
    pack_graphics_buffer_geometry, pack_graphics_buffer_write, pack_transfer, pack_vmo_read,
    pack_wait_item,
};
#[cfg(feature = "androidbox-process0")]
use bndr_abi::{ANDROID_APP_BOOTSTRAP_WIRE_SIZE, AndroidAppBootstrap, AndroidAppBootstrapKind};
#[cfg(feature = "androidbox-restart0")]
use bndr_abi::{
    ANDROID_APP_SUPERVISOR_WIRE_SIZE, AndroidAppSupervisorMessage, AndroidAppSupervisorMessageKind,
};
#[cfg(feature = "androidbox-multipackage4")]
use bndr_abi::{
    ANDROID_PACKAGE_DIRECTORY_CAPACITY, ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE,
    AndroidPackageDirectory,
};
#[cfg(feature = "androidbox-icon-resources5")]
use bndr_abi::{ANDROID_PACKAGE_ICON_WIRE_SIZE, AndroidPackageIcon};
#[cfg(all(
    feature = "androidbox-el0-runtime0",
    not(feature = "androidbox-process0")
))]
use bndr_abi::{ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE, AndroidPackageImageClaim};
#[cfg(feature = "androidbox-runtime-install2")]
use bndr_abi::{
    ANDROID_PACKAGE_INSTALL_CANDIDATE_WIRE_SIZE, ANDROID_PACKAGE_INSTALL_WIRE_SIZE,
    AndroidPackageInstallCandidate, AndroidPackageInstallRequest, AndroidPackageInstallResult,
};
#[cfg(feature = "androidbox-apk-install0")]
use bndr_abi::{
    ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE, ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE,
    AndroidPackageRelaunchRequest, AndroidPackageSnapshot,
};
#[cfg(feature = "androidbox-runtime-uninstall1")]
use bndr_abi::{
    ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE, AndroidPackageUninstallRequest,
    AndroidPackageUninstallResult,
};
use bndr_sm::{
    FRAME_SIZE, Frame, FrameError, InstanceId, Opcode, RegisterError, Registry, RemoveError, Role,
    SUPERVISOR_ATTACH_FRAME_SIZE, ServiceName, ServiceStatus, SupervisorAttachFrame,
};
#[cfg(all(feature = "androidbox-dex0", not(feature = "androidbox-apk-install0")))]
use bndr_ui::mobile::AndroidBoxResourceStatus;
#[cfg(feature = "androidbox-icon-resources5")]
use bndr_ui::mobile::AndroidInstalledIcon;
#[cfg(feature = "androidbox-runtime-uninstall1")]
use bndr_ui::mobile::AndroidInstalledUninstallFailure;
#[cfg(all(
    feature = "mobile-ui-runtime",
    not(feature = "androidbox-interactive0")
))]
use bndr_ui::mobile::render_rows as render_mobile_rows;
#[cfg(feature = "androidbox-runtime-install2")]
use bndr_ui::mobile::{
    AndroidInstallCandidateAction, AndroidInstallCandidateStatus, AndroidInstallFailure,
};
#[cfg(feature = "androidbox-apk-install0")]
use bndr_ui::mobile::{
    AndroidInstalledAppStatus, AndroidInstalledLaunchFailure, AndroidInstalledLaunchStatus,
};
#[cfg(feature = "mobile-ui-runtime")]
use bndr_ui::mobile::{
    HEIGHT as MOBILE_HEIGHT, MobileAction, MobileModel, MobilePage, MobileTimeSnapshot,
    OVERVIEW_GESTURE_COMMIT_PX, OVERVIEW_HOME_COMMIT_PX, PAGE_TRANSITION_ENTER_OFFSETS,
    PAGE_TRANSITION_EXIT_OFFSETS, TouchController, WIDTH as MOBILE_WIDTH,
    point_inside_visible_display,
};
#[cfg(feature = "androidbox-interactive0")]
use bndr_ui::mobile::{
    MobileSystemChromeState, render_content_rows as render_mobile_rows, render_system_chrome_rows,
};
use bndr_ui::{
    BUFFER_PRESENT_WIRE_SIZE, BufferPresent, InputSample, PRESENT_WIRE_SIZE, PresentFrame,
    PresentMode, ShellAppId, UI_CLIENT_CONTROL_WIRE_SIZE, UI_SERVER_EVENT_WIRE_SIZE,
    UiClientControl, UiClientControlPayload, UiClientControlTracker, UiClientId, UiServerEvent,
    UiServerEventPayload, UiServerEventTracker,
};
#[cfg(not(feature = "mobile-ui-runtime"))]
use bndr_ui::{HOME_TARGET, ShellController, ShellTransition, ShellView, SolidRect};
#[cfg(feature = "androidbox-interactive0")]
use bndr_ui::{MOBILE_CONTENT_BOTTOM, MOBILE_CONTENT_TOP};
#[cfg(feature = "mobile-ui-runtime")]
use bndr_ui::{
    UI_SYSTEM_UI_NAV_REVEAL_MAX_PX, UI_SYSTEM_UI_NAV_REVEAL_QUANTUM_PX, UiAndroidBoxAuditSession,
    UiAndroidBoxResourceAuditSession, UiAppearance, UiAppearanceAction, UiAppearanceSession,
    UiBootNotificationSession, UiCompatibleActivityIdentity, UiCompatibleActivityReservationOrigin,
    UiRecentIdentity, UiSoftwareDimming, UiSystemUiAction, UiSystemUiMode, UiSystemUiRequestStatus,
    UiSystemUiSession, UiSystemUiSessionError, UiSystemUiUpdate,
};

#[cfg(all(
    feature = "mobile-ui-runtime",
    any(feature = "app-lifecycle-runtime", feature = "storage-server-runtime")
))]
compile_error!(
    "mobile-ui-runtime is an isolated preview and cannot be combined with app-lifecycle or storage runtimes"
);
#[cfg(all(feature = "mobile-ui-runtime", feature = "ui-stale-present-evidence"))]
compile_error!("mobile-ui-runtime cannot be combined with ui-stale-present-evidence");

// The kernel build script compiles the embedded userspace ABI as a standalone
// rlib. Keep the feature propagation a compile-time contract so a kernel and
// userspace with different ABI generations cannot be linked silently.
#[cfg(all(feature = "app-data-runtime", not(feature = "storage-server-runtime")))]
const _: [(); 24] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "mobile-ui-runtime",
    not(feature = "androidbox-apk-install0")
))]
const _: [(); 24] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-apk-install0",
    not(feature = "androidbox-el0-runtime0")
))]
const _: [(); 44] = [(); ABI_VERSION as usize];

#[cfg(feature = "androidbox-layout-mixed19")]
const _: [(); 69] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-layout-size18",
    not(feature = "androidbox-layout-mixed19")
))]
const _: [(); 68] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-layout-directional17",
    not(feature = "androidbox-layout-size18")
))]
const _: [(); 67] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-layout-spacing16",
    not(feature = "androidbox-layout-directional17")
))]
const _: [(); 66] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-layout-weight15",
    not(feature = "androidbox-layout-spacing16")
))]
const _: [(); 65] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-layout-row14",
    not(feature = "androidbox-layout-weight15")
))]
const _: [(); 64] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-string-builder13",
    not(feature = "androidbox-layout-row14")
))]
const _: [(); 63] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-string-text12",
    not(feature = "androidbox-string-builder13")
))]
const _: [(); 62] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-activity-state11",
    not(feature = "androidbox-string-text12")
))]
const _: [(); 61] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-activity-fields10",
    not(feature = "androidbox-activity-state11")
))]
const _: [(); 60] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-dex-instance9",
    not(feature = "androidbox-activity-fields10")
))]
const _: [(); 59] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-dex-methods8",
    not(feature = "androidbox-dex-instance9")
))]
const _: [(); 58] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-density-icons7",
    not(feature = "androidbox-dex-methods8")
))]
const _: [(); 57] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-icon-resources5",
    not(feature = "androidbox-density-icons7")
))]
const _: [(); 56] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-multipackage4",
    not(feature = "androidbox-icon-resources5")
))]
const _: [(); 55] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-manifest-catalog3",
    not(feature = "androidbox-multipackage4")
))]
const _: [(); 54] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-runtime-install2",
    not(feature = "androidbox-manifest-catalog3")
))]
const _: [(); 53] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-runtime-uninstall1",
    not(feature = "androidbox-runtime-install2")
))]
const _: [(); 52] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-apk-envelope4",
    not(feature = "androidbox-runtime-uninstall1")
))]
const _: [(); 51] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-apk-envelope4")
))]
const _: [(); 50] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
const _: [(); 49] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-restart0",
    not(feature = "androidbox-scene-rpc2")
))]
const _: [(); 48] = [(); ABI_VERSION as usize];

#[cfg(all(feature = "androidbox-process0", not(feature = "androidbox-restart0")))]
const _: [(); 47] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-interactive0",
    not(feature = "androidbox-process0")
))]
const _: [(); 46] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "androidbox-el0-runtime0",
    not(feature = "androidbox-interactive0")
))]
const _: [(); 45] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "storage-server-runtime",
    not(feature = "storage-server-shutdown-orchestration-runtime")
))]
const _: [(); 25] = [(); ABI_VERSION as usize];

#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
const _: [(); 42] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-maintenance-plan-runtime",
    not(feature = "unified-product-signed-maintenance-plan-runtime")
))]
const _: [(); 41] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-maintenance-step-runtime",
    not(feature = "unified-product-maintenance-plan-runtime")
))]
const _: [(); 40] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-maintenance-execution-runtime",
    not(feature = "unified-product-maintenance-step-runtime")
))]
const _: [(); 39] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-maintenance-authorization-runtime",
    not(feature = "unified-product-maintenance-execution-runtime")
))]
const _: [(); 38] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-key-rotation-runtime",
    not(feature = "unified-product-maintenance-authorization-runtime")
))]
const _: [(); 37] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-persistent-rollback-runtime",
    not(feature = "unified-product-key-rotation-runtime")
))]
const _: [(); 36] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-verified-manifest-runtime",
    not(feature = "unified-product-persistent-rollback-runtime")
))]
const _: [(); 35] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-event-supervision-runtime",
    not(feature = "unified-product-verified-manifest-runtime")
))]
const _: [(); 34] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-manifest-supervision-runtime",
    not(feature = "unified-product-event-supervision-runtime")
))]
const _: [(); 33] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-continuous-supervision-runtime",
    not(feature = "unified-product-manifest-supervision-runtime")
))]
const _: [(); 32] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-psci-shutdown-runtime",
    not(feature = "unified-product-continuous-supervision-runtime")
))]
const _: [(); 31] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-multiservice-liveness-runtime",
    not(feature = "unified-product-psci-shutdown-runtime")
))]
const _: [(); 30] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-liveness-runtime",
    not(feature = "unified-product-multiservice-liveness-runtime")
))]
const _: [(); 29] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "unified-product-runtime",
    not(feature = "unified-product-liveness-runtime")
))]
const _: [(); 28] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "resident-platform-shutdown-runtime",
    not(feature = "unified-product-runtime")
))]
const _: [(); 27] = [(); ABI_VERSION as usize];

#[cfg(all(
    feature = "storage-server-shutdown-orchestration-runtime",
    not(feature = "resident-platform-shutdown-runtime")
))]
const _: [(); 26] = [(); ABI_VERSION as usize];

#[cfg(feature = "input-server-runtime")]
mod input_server_runtime;
#[cfg(feature = "app-lifecycle-runtime")]
#[path = "lifecycle_runtime.rs"]
mod lifecycle_runtime;
#[cfg(feature = "unified-product-runtime")]
mod product_runtime;
#[cfg(feature = "resident-platform-shutdown-runtime")]
mod resident_shutdown_runtime;
#[cfg(feature = "storage-server-runtime")]
mod storage_server_runtime;

#[cfg(feature = "input-server-runtime")]
pub(super) const INPUT_ROUTE_BOOTSTRAP_MAGIC: u64 = 0x4d34_355f_524f_5554;
#[cfg(feature = "input-server-runtime")]
pub(super) const INPUT_SERVER_READY_MAGIC: u64 = 0x4d34_355f_494e_4f4b;

#[cfg(feature = "androidbox-process0")]
mod androidapp_process;

const EXPECTED_TAG: u64 = 0x1234;
const EXPECTED_PAYLOAD: u64 = 0x0123_4567_89ab_cdef;
const INIT_READY_MAGIC: u64 = 0xc0de_1a17_baad_f00d;
const REGISTER_SENTINEL: u64 = 0xd15c_a11e_0bad_c0de;
const STACK_SENTINEL: u64 = 0x5a5a_c3c3_f00d_baad;
const USER_GUARD_HIGH: u64 = 0x0000_0002_001f_f000;
#[cfg(feature = "storage-server-runtime")]
const USER_GUARD_LOW: u64 = 0x0000_0002_001b_e000;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    feature = "androidbox-interactive0"
))]
const USER_GUARD_LOW: u64 = 0x0000_0002_001e_a000;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    not(feature = "androidbox-interactive0"),
    feature = "androidbox-apk-install0"
))]
const USER_GUARD_LOW: u64 = 0x0000_0002_001f_6000;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    not(feature = "androidbox-interactive0"),
    not(feature = "androidbox-apk-install0")
))]
const USER_GUARD_LOW: u64 = 0x0000_0002_001f_a000;
#[cfg(feature = "storage-server-runtime")]
const USER_STACK_START: usize = 0x0000_0002_001b_f000;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    feature = "androidbox-interactive0"
))]
const USER_STACK_START: usize = 0x0000_0002_001e_b000;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    not(feature = "androidbox-interactive0"),
    feature = "androidbox-apk-install0"
))]
const USER_STACK_START: usize = 0x0000_0002_001f_7000;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    not(feature = "androidbox-interactive0"),
    not(feature = "androidbox-apk-install0")
))]
const USER_STACK_START: usize = 0x0000_0002_001f_b000;
#[cfg(feature = "storage-server-runtime")]
const USER_STACK_PAGES: usize = 64;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    feature = "androidbox-interactive0"
))]
const USER_STACK_PAGES: usize = 20;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    not(feature = "androidbox-interactive0"),
    feature = "androidbox-apk-install0"
))]
const USER_STACK_PAGES: usize = 8;
#[cfg(all(
    not(feature = "storage-server-runtime"),
    not(feature = "androidbox-interactive0"),
    not(feature = "androidbox-apk-install0")
))]
const USER_STACK_PAGES: usize = 4;
const USER_PAGE_SIZE: usize = 4096;
const _: () = assert!(USER_STACK_START == USER_GUARD_LOW as usize + USER_PAGE_SIZE);
const _: () =
    assert!(USER_STACK_START + USER_STACK_PAGES * USER_PAGE_SIZE == USER_GUARD_HIGH as usize);
const PROCESS_TERMINATE_SELF_TEST_INIT_ARGUMENT: u64 = 0x5054_5354_494e_4954;
const PROCESS_TERMINATE_SELF_TEST_READY_1: u64 = 0x5054_5354_5244_5931;
const PROCESS_TERMINATE_SELF_TEST_READY_2: u64 = 0x5054_5354_5244_5932;
const SYSTEM_HELLO_PATH: &[u8] = b"HELLO.TXT";
const SYSTEM_BUILD_PATH: &[u8] = b"SYSTEM/BUILD.TXT";
const SYSTEM_TRAVERSAL_PATH: &[u8] = b"../HELLO.TXT";
const SYSTEM_MISSING_PATH: &[u8] = b"MISSING.TXT";
const SYSTEM_HELLO_CONTENT: &[u8] = b"Bndroid M23 FAT16 is alive.\n";
const SYSTEM_BUILD_CONTENT: &[u8] = b"build=m23\nfilesystem=fat16\nvfs=readonly\n";
const SYSTEM_HELLO_DIGEST: u64 = 0xdd2f_7134_2016_eede;
const SYSTEM_BUILD_DIGEST: u64 = 0xe57c_e4ce_9f1b_4ec0;
const DATA_INITIAL: u64 = 0x424e_4452_4441_5441;
const DATA_MUTATED: u64 = 0x6d75_7461_7465_6421;
const BSS_MUTATED: u64 = 0x7a65_726f_2d6f_6b21;
const CHILD_DUPLICATES_TO_FILL: usize = 31;
const ECHO_TAG: u64 = 0x4543_484f;
const ECHO_PAYLOAD: u64 = 0x626e_6472_2d65_6368;
const REGISTER_TXID: u32 = 1;
const DUPLICATE_TXID: u32 = 2;
const UNKNOWN_LOOKUP_TXID: u32 = 1;
const CLIENT_LOOKUP_TXID: u32 = 2;
const FAIL_SM_NAME: u64 = 101;
const FAIL_SM_FRAME: u64 = 102;
const FAIL_SM_WAIT: u64 = 103;
const FAIL_SM_BYTE_WRITE: u64 = 104;
const FAIL_SM_BYTE_READ: u64 = 105;
const FAIL_SM_TRANSFER_WRITE: u64 = 106;
const FAIL_SM_TRANSFER_STALE: u64 = 107;
const FAIL_SM_TRANSFER_READ: u64 = 108;
const FAIL_SM_FRAME_MISMATCH: u64 = 109;
const FAIL_SM_CLOSE: u64 = 110;
const FAIL_SM_REGISTRY: u64 = 111;
const FAIL_SM_ECHO: u64 = 112;
const FAIL_SM_ROLE: u64 = 114;
const FAIL_SM_LIFECYCLE: u64 = 115;
const FAIL_WAIT_MANY_POLL: u64 = 116;
const FAIL_WAIT_MANY_TIMEOUT: u64 = 117;
const FAIL_WAIT_MANY_SIGNAL: u64 = 118;
const FAIL_WAIT_MANY_PRIORITY: u64 = 119;
const FAIL_WAIT_MANY_VALIDATE_ALL: u64 = 120;
const FAIL_CONCURRENT_PREPARE: u64 = 121;
const FAIL_CONCURRENT_CAPACITY: u64 = 122;
const FAIL_GENERIC_CONTINUE: u64 = 123;
const FAIL_PROVIDER_OWNER: u64 = 126;
const FAIL_ABANDONED_TRANSFER: u64 = 128;
const CHILD_FAILURE_EXIT_PREFIX: u64 = 0xfa11_0000_0000_0000;
const WAIT_MANY_FINITE_TIMEOUT_NS: u64 = 50_000_000;
const WAIT_MANY_SIGNAL_TIMEOUT_NS: u64 = 1_000_000_000;
const GENERIC_CONTINUE_TAG: u64 = 0x4745_4e45_5249_4321;
const GENERIC_CONTINUE_PAYLOAD: u64 = 0x636f_6e74_696e_7565;
const ROLE_READY_TAG: u64 = 0x524f_4c45_5244_5921;
const ROLE_DONE_TAG: u64 = 0x524f_4c45_444f_4e45;
const RESIDENT_READY_TAG: u64 = 0x5253_444e_5452_4459;
const SERVING_ROUND_TAG: u64 = 0x5352_5652_4f55_4e44;
const SERVING_ACK_TAG: u64 = 0x5352_565f_4143_4b21;
const SERVING_DONE_TAG: u64 = 0x5352_565f_444f_4e45;
const POST_READY_ECHO_ROUNDS: u32 = 2;
const POST_READY_TXID_BASE: u32 = 0x100;
const DYNAMIC_COMMAND_TAG: u64 = 0x4431_3543_4d44_2121;
const DYNAMIC_COMMIT_TAG: u64 = 0x4431_3543_4f4d_4d54;
const DYNAMIC_DONE_TAG: u64 = 0x4431_3544_4f4e_4521;
const DYNAMIC_REGISTER_FIRST_COMMAND: u64 = 1;
const DYNAMIC_UNREGISTER_FIRST_COMMAND: u64 = 2;
const DYNAMIC_REGISTER_REPLACEMENT_COMMAND: u64 = 3;
const DYNAMIC_CLEANUP_COMMAND: u64 = 4;
const DYNAMIC_ECHO_FIRST_COMMAND: u64 = 5;
const DYNAMIC_LOOKUP_MISSING_COMMAND: u64 = 6;
const DYNAMIC_ECHO_REPLACEMENT_COMMAND: u64 = 7;
const DYNAMIC_CONFIRM_FIRST_ECHO_COMMAND: u64 = 8;
const DYNAMIC_CONFIRM_REPLACEMENT_ECHO_COMMAND: u64 = 9;
const DYNAMIC_REGISTER_FIRST_TXID: u32 = 0x201;
const DYNAMIC_LOOKUP_FIRST_TXID: u32 = 0x202;
const DYNAMIC_UNREGISTER_FIRST_TXID: u32 = 0x203;
const DYNAMIC_LOOKUP_MISSING_TXID: u32 = 0x204;
const DYNAMIC_REGISTER_REPLACEMENT_TXID: u32 = 0x205;
const DYNAMIC_UNREGISTER_STALE_TXID: u32 = 0x206;
const DYNAMIC_LOOKUP_REPLACEMENT_TXID: u32 = 0x207;
const DYNAMIC_UNREGISTER_REPLACEMENT_TXID: u32 = 0x208;
const DYNAMIC_FINAL_PHASE: u8 = 10;
const REUSE_COMMAND_TAG: u64 = 0x4431_3643_4d44_2121;
const REUSE_COMMIT_TAG_PREFIX: u64 = 0x4431_3643_0000_0000;
const REUSE_DONE_TAG_PREFIX: u64 = 0x4431_3644_0000_0000;
const REUSE_ROUND: u8 = 1;
const REUSE_REGISTER_COMMAND: u64 = 1;
const REUSE_CONFIRM_ECHO_COMMAND: u64 = 2;
const REUSE_UNREGISTER_COMMAND: u64 = 3;
const REUSE_ECHO_COMMAND: u64 = 4;
const REUSE_LOOKUP_MISSING_COMMAND: u64 = 5;
const REUSE_REGISTER_TXID: u32 = 0x301;
const REUSE_LOOKUP_TXID: u32 = 0x302;
const REUSE_UNREGISTER_TXID: u32 = 0x303;
const REUSE_LOOKUP_MISSING_TXID: u32 = 0x304;
const IDENTITY_COMMAND_TAG: u64 = 0x4d31_3743_4d44_2121;
const IDENTITY_PROVIDER_RELAY_COMMAND: u64 = 1;
const IDENTITY_CLIENT_ATTACK_TAG: u64 = 0x4d31_3743_4c4e_5421;
const IDENTITY_PROVIDER_COMMIT_TAG: u64 = 0x4d31_3750_524f_5621;
const IDENTITY_CLIENT_COMMIT_TAG: u64 = 0x4d31_3743_4c4e_5423;
const PROVIDER_IDENTITY_DONE_TAG: u64 = 0x4d31_3749_4445_4e54;
const CLIENT_IDENTITY_DONE_TAG: u64 = 0x4d31_3749_4443_4c54;
const SERVICE_ACL_DONE_TAG: u64 = 0x4d31_3741_434c_4f4b;
const IDENTITY_ATTACK_TXID: u32 = 0x401;
const MALFORMED_REQUEST_COUNT: u16 = 4;
const CLIENT_MODE_TAG: u64 = 0x4d31_3843_4d4f_4445;
const CLIENT_MODE_PRIMARY: u64 = 1;
const CLIENT_MODE_SECONDARY: u64 = 2;
const M20_MANAGER_CONTROL_TAG: u64 = 0x4d32_304d_4354_524c;
const M20_MANAGER_EVENT_TAG: u64 = 0x4d32_304d_4556_4e54;
const M20_CLIENT_CONTROL_TAG: u64 = 0x4d32_3043_4354_524c;
const M20_CLIENT_EVENT_TAG: u64 = 0x4d32_3043_4556_4e54;
const M20_CLIENT_MANAGER_PID_TAG: u64 = 0x4d32_3043_4d50_4944;
const M20_PROVIDER_CONTROL_TAG: u64 = 0x4d32_3050_4354_524c;
const M20_PROVIDER_EVENT_TAG: u64 = 0x4d32_3050_4556_4e54;
const M20_PROVIDER_PRIMARY_PID_TAG: u64 = 0x4d32_3050_5049_4421;
const M20_PROVIDER_SECONDARY_PID_TAG: u64 = 0x4d32_3053_5049_4421;
const M20_ECHO_TAG: u64 = 0x4d32_3045_4348_4f21;
const M20_PRIMARY_LEASE: u16 = 1;
const M20_SECONDARY_FIRST_LEASE: u16 = 1;
const M20_SECONDARY_REATTACHED_LEASE: u16 = 2;
const M20_SECONDARY_STALLED_TXID: u32 = 0x601;
const M20_PRIMARY_WHILE_STALLED_TXID: u32 = 0x602;
const M20_PRIMARY_DETACHED_TXID: u32 = 0x603;
const M20_SECONDARY_FINAL_TXID: u32 = 0x604;
const M20_MANAGER_CONTROL_REVOKE: u8 = 1;
const M20_MANAGER_CONTROL_FINALIZE: u8 = 2;
const M20_MANAGER_EVENT_ATTACHED: u8 = 1;
const M20_MANAGER_EVENT_REVOKED: u8 = 2;
const M20_MANAGER_EVENT_STALE_REJECTED: u8 = 3;
const M20_MANAGER_EVENT_IDLE: u8 = 4;
const M20_CLIENT_CONTROL_STALL: u8 = 1;
const M20_CLIENT_CONTROL_ECHO: u8 = 2;
const M20_CLIENT_CONTROL_STALE_REVOKE: u8 = 3;
const M20_CLIENT_EVENT_ATTACHED: u8 = 1;
const M20_CLIENT_EVENT_STALLED: u8 = 2;
const M20_CLIENT_EVENT_ECHO_DONE: u8 = 3;
const M20_CLIENT_EVENT_REVOKED: u8 = 4;
const M20_CLIENT_EVENT_STALE_REJECTED: u8 = 5;
const M20_PROVIDER_CONTROL_ARM: u8 = 1;
const M20_PROVIDER_CONTROL_FINALIZE: u8 = 2;
const M20_PROVIDER_EVENT_ARMED: u8 = 1;
const M20_PROVIDER_EVENT_ACCEPTED: u8 = 2;
const M20_PROVIDER_EVENT_ECHO_DONE: u8 = 3;
const M20_PROVIDER_EVENT_ABORTED: u8 = 4;
const M20_PROVIDER_EVENT_IDLE: u8 = 5;
const MANAGER_INSTANCE_TAG: u64 = 0x4d47_525f_494e_5354;
const MANAGER_LOST_TAG: u64 = 0x4d47_525f_4c4f_5354;
const MANAGER_RESTART_EXIT: u64 = 0x534d_4752_5253_5431;
const FIRST_MANAGER_EPOCH: u16 = 1;
const SECOND_MANAGER_EPOCH: u16 = 2;
const FIRST_EPOCH_INSTANCE_RAW: u32 = 0x0001_0001;
const SECOND_EPOCH_INSTANCE_RAW: u32 = 0x0002_0001;
const FAIL_ATTACH: u64 = 130;
const FAIL_ROLE_READY: u64 = 131;
const FAIL_MANAGER_IDENTITY: u64 = 132;
const FAIL_MANAGER_REPORT: u64 = 133;
const FAIL_MANAGER_RESTART: u64 = 134;
const FAIL_SESSION_OBSERVATION: u64 = 135;
const FAIL_MANAGER_DONE: u64 = 136;
const FAIL_STALE_GENERATION: u64 = 137;
const FAIL_CAPACITY_SOURCE: u64 = 138;
const FAIL_FINAL_WAIT: u64 = 139;
const FAIL_MANAGER_PROTOCOL: u64 = 140;
const FAIL_DYNAMIC_PROTOCOL: u64 = 141;
const FAIL_DYNAMIC_INSTANCE: u64 = 142;
const FAIL_DYNAMIC_COMMIT: u64 = 143;
const FAIL_DYNAMIC_CLEANUP: u64 = 144;
const FAIL_REUSE_PROTOCOL: u64 = 145;
const FAIL_REUSE_INSTANCE: u64 = 146;
const FAIL_REUSE_COMMIT: u64 = 147;
const FAIL_IDENTITY_ENVELOPE: u64 = 148;
const FAIL_IDENTITY_SENDER: u64 = 149;
const FAIL_IDENTITY_PROTOCOL: u64 = 150;
const FAIL_SERVICE_ACL: u64 = 151;
const FAIL_MULTI_CLIENT_CONFIG: u64 = 152;
const FAIL_MULTI_CLIENT_ATTACH: u64 = 153;
const FAIL_MULTI_CLIENT_PROTOCOL: u64 = 154;
const FAIL_MULTI_CLIENT_IDENTITY: u64 = 155;
const FAIL_MULTI_CLIENT_ECHO: u64 = 156;
const FAIL_MULTI_CLIENT_CLEANUP: u64 = 157;
const FAIL_WAIT_MANY_ARRAY_COUNT: u64 = 158;
const FAIL_WAIT_MANY_ARRAY_POLL: u64 = 159;
const FAIL_WAIT_MANY_ARRAY_BAD_POINTER: u64 = 160;
const FAIL_WAIT_MANY_ARRAY_GUARD_ROLLBACK: u64 = 161;
const FAIL_WAIT_MANY_ARRAY_VALIDATE_ALL: u64 = 162;
const FAIL_WAIT_MANY_ARRAY_LOWEST: u64 = 163;
const FAIL_WAIT_MANY_ARRAY_BLOCKING_WAKE: u64 = 164;
const FAIL_STORAGE_ROOT: u64 = 165;
const FAIL_STORAGE_OPEN_INVALID_ROOT: u64 = 166;
const FAIL_STORAGE_OPEN_BAD_ADDRESS: u64 = 167;
const FAIL_STORAGE_OPEN_TRAVERSAL: u64 = 168;
const FAIL_STORAGE_OPEN_MISSING: u64 = 169;
const FAIL_STORAGE_OPEN_HELLO: u64 = 170;
const FAIL_STORAGE_OPEN_BUILD: u64 = 171;
const FAIL_STORAGE_ROOT_DUPLICATE: u64 = 172;
const FAIL_STORAGE_BUILD_ESCALATION: u64 = 173;
const FAIL_STORAGE_BUILD_DUPLICATE: u64 = 174;
const FAIL_STORAGE_BUILD_STALE: u64 = 175;
const FAIL_STORAGE_HELLO_ATTENUATION: u64 = 176;
const FAIL_STORAGE_VMO_WAIT: u64 = 177;
const FAIL_STORAGE_HELLO_READ: u64 = 178;
const FAIL_STORAGE_HELLO_RANGE: u64 = 179;
const FAIL_STORAGE_VMO_EOF: u64 = 180;
const FAIL_STORAGE_VMO_OFFSET: u64 = 181;
const FAIL_STORAGE_VMO_LENGTH: u64 = 182;
const FAIL_STORAGE_VMO_BAD_ADDRESS: u64 = 183;
const FAIL_STORAGE_BUILD_READ: u64 = 184;
const FAIL_STORAGE_CHANNEL: u64 = 185;
const FAIL_STORAGE_TRANSFER: u64 = 186;
const FAIL_STORAGE_TRANSFER_STALE: u64 = 187;
const FAIL_STORAGE_RECEIVE: u64 = 188;
const FAIL_STORAGE_RECEIVED_READ: u64 = 189;
const FAIL_STORAGE_CLEANUP: u64 = 190;
const FAIL_SURFACE_ACQUIRE: u64 = 191;
const FAIL_SURFACE_PRESENT: u64 = 192;
const FAIL_SURFACE_INPUT: u64 = 193;
const FAIL_SURFACE_PROTOCOL: u64 = 194;
#[cfg(feature = "mobile-ui-runtime")]
const MOBILE_DEFERRED_PAYLOAD_QUEUE_CAPACITY: usize = 16;
#[cfg(feature = "mobile-ui-runtime")]
const _: () = assert!(MOBILE_DEFERRED_PAYLOAD_QUEUE_CAPACITY > 0);
#[cfg(feature = "mobile-ui-runtime")]
const MOBILE_CLOCK_SECONDS_PER_MINUTE: u64 = 60;
#[cfg(feature = "mobile-ui-runtime")]
const MOBILE_CLOCK_NANOSECONDS_PER_SECOND: u64 = 1_000_000_000;
#[cfg(feature = "app-data-runtime")]
const FAIL_APP_DATA_AUTHORITY: u64 = 256;
const COLOR_PHONE_SCREEN: u32 = 0x001e_293b;
#[cfg(not(feature = "mobile-ui-runtime"))]
const COLOR_PHONE_HEADER: u32 = 0x001d_4ed8;
#[cfg(not(feature = "mobile-ui-runtime"))]
const COLOR_CARD_CYAN: u32 = 0x0006_b6d4;
#[cfg(not(feature = "mobile-ui-runtime"))]
const COLOR_CARD_PURPLE: u32 = 0x008b_5cf6;
#[cfg(not(feature = "mobile-ui-runtime"))]
const COLOR_CARD_GREEN: u32 = 0x0022_c55e;
#[cfg(not(feature = "mobile-ui-runtime"))]
const COLOR_APP_PANEL: u32 = 0x000f_172a;
#[cfg(not(feature = "mobile-ui-runtime"))]
const COLOR_APP_ROW: u32 = 0x00e2_e8f0;
#[cfg(not(feature = "mobile-ui-runtime"))]
const COLOR_APP_ROW_MUTED: u32 = 0x0064_748b;
#[cfg(not(feature = "mobile-ui-runtime"))]
const COLOR_APP_TOGGLE: u32 = 0x0038_bdf8;
#[cfg_attr(target_os = "none", unsafe(link_section = ".rodata.byte_message"))]
static BYTE_MESSAGE: [u8; 16] = *b"Bndroid copy I/O";
#[used]
#[cfg_attr(target_os = "none", unsafe(link_section = ".data.image_probe"))]
static FILE_BACKED_PROBE: AtomicU64 = AtomicU64::new(DATA_INITIAL);
#[used]
#[cfg_attr(target_os = "none", unsafe(link_section = ".bss.image_probe"))]
static ZERO_FILLED_PROBE: AtomicU64 = AtomicU64::new(0);
static PROCESS_ROLE: AtomicU64 = AtomicU64::new(u64::MAX);

#[derive(Clone, Copy)]
struct SyscallResult {
    status: u64,
    out1: u64,
    out2: u64,
}

struct OwnedUserHandle {
    raw: u64,
}

#[cfg(all(
    feature = "androidbox-interactive0",
    not(feature = "androidbox-process0")
))]
type AndroidInstalledActivityLease = InstalledInteractiveActivityLease;
#[cfg(feature = "androidbox-process0")]
type AndroidInstalledActivityLease = androidapp_process::AndroidAppClient;

#[cfg(feature = "mobile-ui-runtime")]
#[derive(Clone, Copy)]
struct MobileSystemNavCapture {
    start_x: u16,
    start_y: u16,
    rejected: bool,
}

/// SurfaceServer-owned wall-clock publication state.
///
/// The PL031 value may move either forward or backward. The displayed value
/// follows the latest read, while the protocol revision remains strictly
/// monotonic and advances only when the visible minute changes.
#[cfg(feature = "mobile-ui-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MobileClockSession {
    unix_seconds: u64,
    revision: u64,
}

#[cfg(feature = "mobile-ui-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MobileClockUpdate {
    unix_seconds: u64,
    revision: u64,
}

#[cfg(feature = "mobile-ui-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MobileClockSessionError {
    RevisionExhausted,
}

#[cfg(feature = "mobile-ui-runtime")]
impl MobileClockSession {
    const fn new(unix_seconds: u64) -> Self {
        Self {
            unix_seconds,
            revision: 1,
        }
    }

    const fn wait_timeout_ns(self) -> u64 {
        mobile_clock_wait_timeout_ns(self.unix_seconds)
    }

    fn observe(
        &mut self,
        unix_seconds: u64,
    ) -> Result<Option<MobileClockUpdate>, MobileClockSessionError> {
        if mobile_clock_minute(self.unix_seconds) == mobile_clock_minute(unix_seconds) {
            self.unix_seconds = unix_seconds;
            return Ok(None);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(MobileClockSessionError::RevisionExhausted)?;
        self.unix_seconds = unix_seconds;
        self.revision = revision;
        Ok(Some(MobileClockUpdate {
            unix_seconds,
            revision,
        }))
    }
}

#[cfg(feature = "mobile-ui-runtime")]
const fn mobile_clock_minute(unix_seconds: u64) -> u64 {
    unix_seconds / MOBILE_CLOCK_SECONDS_PER_MINUTE
}

#[cfg(feature = "mobile-ui-runtime")]
const fn mobile_clock_wait_timeout_ns(unix_seconds: u64) -> u64 {
    let seconds_to_next_minute =
        MOBILE_CLOCK_SECONDS_PER_MINUTE - unix_seconds % MOBILE_CLOCK_SECONDS_PER_MINUTE;
    seconds_to_next_minute * MOBILE_CLOCK_NANOSECONDS_PER_SECOND
}

#[cfg(feature = "mobile-ui-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompatibleVerificationInputBarrier {
    Open,
    Reservation { last_pressed: Option<bool> },
    Draining { last_pressed: Option<bool> },
    AwaitingRelease,
}

#[cfg(feature = "mobile-ui-runtime")]
const COMPATIBLE_VERIFICATION_INPUT_DRAIN_LIMIT: usize = 64;

#[cfg(feature = "mobile-ui-runtime")]
impl CompatibleVerificationInputBarrier {
    const fn new() -> Self {
        Self::Open
    }

    const fn blocks_reservation(self) -> bool {
        !matches!(self, Self::Open)
    }

    fn begin_reservation(&mut self) -> bool {
        if !matches!(self, Self::Open) {
            return false;
        }
        *self = Self::Reservation { last_pressed: None };
        true
    }

    fn begin_completion_drain(&mut self) -> bool {
        let Self::Reservation { last_pressed } = *self else {
            return false;
        };
        *self = Self::Draining { last_pressed };
        true
    }

    /// Returns true when the sample belongs to the terminal reservation
    /// barrier and must not be routed to either client.
    fn drop_sample(&mut self, pressed: bool) -> bool {
        match *self {
            Self::Open => false,
            Self::Reservation { .. } => {
                *self = Self::Reservation {
                    last_pressed: Some(pressed),
                };
                true
            }
            Self::Draining { .. } => {
                *self = Self::Draining {
                    last_pressed: Some(pressed),
                };
                true
            }
            Self::AwaitingRelease => {
                if !pressed {
                    *self = Self::Open;
                }
                true
            }
        }
    }

    fn finish_completion_drain(&mut self, limit_reached: bool) -> bool {
        let Self::Draining { last_pressed } = *self else {
            return false;
        };
        *self = if limit_reached || last_pressed == Some(true) {
            // A bounded drain or trailing press cannot prove a release.
            Self::AwaitingRelease
        } else {
            // Reserve started only after the launch contact's release was
            // consumed. An empty FIFO preserves that known-released state;
            // an explicit final release establishes it again.
            Self::Open
        };
        true
    }
}

#[cfg(feature = "ui-stale-present-evidence")]
#[derive(Clone, Copy)]
enum UiStalePresentBarrier {
    Awaiting,
    Held(BufferPresent),
    Released,
}

struct SurfaceServerRuntime {
    launcher_channel: OwnedUserHandle,
    app_channel: OwnedUserHandle,
    launcher_pid: Option<u64>,
    app_pid: Option<u64>,
    launcher_buffer: Option<OwnedUserHandle>,
    app_buffer: Option<OwnedUserHandle>,
    capability: OwnedUserHandle,
    session_id: u64,
    active_client: UiClientId,
    active_app: Option<ShellAppId>,
    focus_generation: u64,
    global_frame_id: u32,
    launcher_frame_id: Option<u32>,
    app_frame_id: Option<u32>,
    launcher_input_sequence: u64,
    app_input_sequence: u64,
    input_capture: Option<UiClientId>,
    pointer_pressed: bool,
    focus_controls: UiClientControlTracker,
    wait_rotation: usize,
    #[cfg(feature = "mobile-ui-runtime")]
    appearance: UiAppearanceSession,
    #[cfg(feature = "mobile-ui-runtime")]
    boot_notification: UiBootNotificationSession,
    #[cfg(feature = "mobile-ui-runtime")]
    system_ui: UiSystemUiSession,
    #[cfg(feature = "androidbox-interactive0")]
    system_chrome_buffer: OwnedUserHandle,
    #[cfg(feature = "androidbox-interactive0")]
    system_chrome_buffer_for_present: OwnedUserHandle,
    #[cfg(feature = "androidbox-interactive0")]
    system_chrome_generation: u64,
    #[cfg(feature = "mobile-ui-runtime")]
    clock: MobileClockSession,
    #[cfg(feature = "mobile-ui-runtime")]
    androidbox_audit: UiAndroidBoxAuditSession,
    #[cfg(feature = "mobile-ui-runtime")]
    androidbox_activity_audit: bndr_ui::UiAndroidBoxActivityAuditSession,
    #[cfg(feature = "mobile-ui-runtime")]
    androidbox_resource_audit: UiAndroidBoxResourceAuditSession,
    #[cfg(feature = "mobile-ui-runtime")]
    system_nav_capture: Option<MobileSystemNavCapture>,
    #[cfg(feature = "androidbox-interactive0")]
    system_top_chrome_capture: bool,
    #[cfg(feature = "mobile-ui-runtime")]
    // A terminal compatible-Activity request drains the kernel FIFO and keeps
    // a release barrier only when the bounded observation ends while pressed.
    compatible_verification_input_barrier: CompatibleVerificationInputBarrier,
    #[cfg(feature = "ui-stale-present-evidence")]
    stale_present_barrier: UiStalePresentBarrier,
}

#[cfg(not(feature = "mobile-ui-runtime"))]
struct LauncherRuntime {
    channel: OwnedUserHandle,
    server_pid: u64,
    events: UiServerEventTracker,
    shell: ShellController,
    deferred_transition: Option<ShellTransition>,
}

struct AppRuntime {
    channel: OwnedUserHandle,
    server_pid: u64,
    events: UiServerEventTracker,
    buffer: OwnedUserHandle,
    buffer_for_server: Option<OwnedUserHandle>,
    buffer_generation: u64,
    #[cfg(feature = "mobile-ui-runtime")]
    // These payloads already passed `UiServerEventTracker::accept` while a
    // presentation acknowledgment was outstanding. Consumers must not accept
    // them through the tracker a second time.
    deferred_payloads: MobileDeferredPayloadQueue,
    #[cfg(feature = "mobile-ui-runtime")]
    // The protocol tracker may already contain focus changes deferred above.
    // Input routing must follow only focus events actually delivered to the
    // mobile event loop.
    delivered_active_client: UiClientId,
    #[cfg(feature = "mobile-ui-runtime")]
    delivered_focus_generation: u64,
    #[cfg(feature = "mobile-ui-runtime")]
    // The protocol tracker may be ahead while an appearance broadcast waits
    // in the accepted FIFO. Rendering follows only the delivered revision.
    delivered_appearance: UiAppearance,
    #[cfg(feature = "mobile-ui-runtime")]
    delivered_appearance_revision: u64,
    #[cfg(feature = "mobile-ui-runtime")]
    // The fixed boot-local notice is SurfaceServer session state. Rendering
    // follows only the revision delivered to this client loop.
    delivered_boot_notification_visible: bool,
    #[cfg(feature = "mobile-ui-runtime")]
    delivered_boot_notification_revision: u64,
    #[cfg(feature = "mobile-ui-runtime")]
    // Bottom navigation and Overview are SurfaceServer-owned session state.
    // Clients mirror only the latest payload delivered to their own loop.
    delivered_system_ui_mode: UiSystemUiMode,
    #[cfg(feature = "mobile-ui-runtime")]
    delivered_system_ui_recent: Option<UiRecentIdentity>,
    #[cfg(feature = "mobile-ui-runtime")]
    delivered_system_nav_pressed: bool,
    #[cfg(feature = "mobile-ui-runtime")]
    delivered_system_nav_reveal_px: u16,
    #[cfg(feature = "mobile-ui-runtime")]
    delivered_system_ui_revision: u64,
    #[cfg(feature = "mobile-ui-runtime")]
    // Clock snapshots are accepted by the protocol tracker before delivery
    // when they arrive during a synchronous present. Rendering follows only
    // the latest event delivered to the mobile loop.
    delivered_clock_unix_seconds: u64,
    #[cfg(feature = "mobile-ui-runtime")]
    delivered_clock_revision: u64,
    #[cfg(feature = "mobile-ui-runtime")]
    appearance_request_id: u64,
    #[cfg(feature = "mobile-ui-runtime")]
    boot_notification_request_id: u64,
    #[cfg(feature = "mobile-ui-runtime")]
    system_ui_request_id: u64,
    #[cfg(feature = "mobile-ui-runtime")]
    // Exactly one client SystemUI request may be in flight. The accepted
    // request id is advanced only by an explicit SurfaceServer completion,
    // never when bytes are merely queued.
    system_ui_request_outstanding: Option<MobileSystemUiRequest>,
    #[cfg(feature = "mobile-ui-runtime")]
    // A transition quarantines samples accepted while its frame ack is
    // outstanding. After the final step, ignore input until one unpressed
    // boundary is delivered so a contact that began mid-transition can never
    // arm a target on the stable destination.
    suppress_mobile_input_until_release: bool,
    #[cfg(feature = "mobile-ui-runtime")]
    quarantined_mobile_inputs: u64,
}

#[cfg(feature = "mobile-ui-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MobileSystemUiRequest {
    action: UiSystemUiAction,
    recent: Option<UiRecentIdentity>,
    request_id: u64,
    observed_revision: u64,
}

#[cfg(feature = "mobile-ui-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MobileSystemUiEpoch {
    mode: UiSystemUiMode,
    recent: Option<UiRecentIdentity>,
    nav_pressed: bool,
    nav_reveal_px: u16,
    revision: u64,
}

#[cfg(feature = "mobile-ui-runtime")]
fn tracked_mobile_system_ui_epoch(events: &UiServerEventTracker) -> Option<MobileSystemUiEpoch> {
    Some(MobileSystemUiEpoch {
        mode: events.system_ui_mode()?,
        recent: events.recent(),
        nav_pressed: events.nav_pressed()?,
        nav_reveal_px: events.nav_reveal_px()?,
        revision: events.last_system_ui_revision()?,
    })
}

#[cfg(feature = "androidbox-el0-runtime0")]
fn compatible_app_surface_is_presentable(system_ui: UiSystemUiSession) -> bool {
    system_ui.mode() == UiSystemUiMode::Foreground
        && matches!(
            system_ui.recent(),
            Some(UiRecentIdentity::CompatibleAndroid(_))
        )
        && !system_ui.nav_pressed()
        && system_ui.nav_reveal_px() == 0
        && !system_ui.has_compatible_activity_reservation()
}

#[cfg(feature = "androidbox-el0-runtime0")]
fn finish_compatible_activity_from_app(
    system_ui: &mut UiSystemUiSession,
) -> Option<(UiSystemUiUpdate, UiCompatibleActivityIdentity, u64)> {
    if !compatible_app_surface_is_presentable(*system_ui) {
        return None;
    }
    let identity = system_ui.recent()?.compatible_android()?;
    let request_id = system_ui.last_request_id().checked_add(1)?;
    let update = system_ui
        .apply_client_recent_action(
            UiClientId::Launcher,
            UiSystemUiAction::FinishCompatibleActivity,
            Some(UiRecentIdentity::CompatibleAndroid(identity)),
            request_id,
            system_ui.revision(),
        )
        .ok()?;
    Some((update, identity, request_id))
}

#[cfg(all(test, feature = "mobile-ui-runtime"))]
mod compatible_verification_input_tests {
    use super::*;

    #[test]
    fn mobile_clock_waits_to_the_next_exact_minute_boundary() {
        assert_eq!(
            mobile_clock_wait_timeout_ns(0),
            60 * MOBILE_CLOCK_NANOSECONDS_PER_SECOND
        );
        assert_eq!(
            mobile_clock_wait_timeout_ns(1),
            59 * MOBILE_CLOCK_NANOSECONDS_PER_SECOND
        );
        assert_eq!(
            mobile_clock_wait_timeout_ns(59),
            MOBILE_CLOCK_NANOSECONDS_PER_SECOND
        );
        assert_eq!(
            mobile_clock_wait_timeout_ns(60),
            60 * MOBILE_CLOCK_NANOSECONDS_PER_SECOND
        );
        assert_eq!(
            mobile_clock_wait_timeout_ns(u64::MAX),
            45 * MOBILE_CLOCK_NANOSECONDS_PER_SECOND
        );
    }

    #[test]
    fn mobile_clock_publishes_once_per_visible_minute_in_both_directions() {
        let mut clock = MobileClockSession::new(3_599);
        assert_eq!(clock.wait_timeout_ns(), MOBILE_CLOCK_NANOSECONDS_PER_SECOND);
        assert_eq!(clock.observe(3_540), Ok(None));
        assert_eq!(clock.unix_seconds, 3_540);
        assert_eq!(clock.revision, 1);

        assert_eq!(
            clock.observe(3_600),
            Ok(Some(MobileClockUpdate {
                unix_seconds: 3_600,
                revision: 2,
            }))
        );
        assert_eq!(clock.observe(3_659), Ok(None));
        assert_eq!(clock.unix_seconds, 3_659);
        assert_eq!(clock.revision, 2);

        assert_eq!(
            clock.observe(3_539),
            Ok(Some(MobileClockUpdate {
                unix_seconds: 3_539,
                revision: 3,
            }))
        );
        assert_eq!(clock.unix_seconds, 3_539);
        assert_eq!(clock.revision, 3);
    }

    #[test]
    fn mobile_clock_revision_exhaustion_is_transactional() {
        let mut clock = MobileClockSession {
            unix_seconds: 59,
            revision: u64::MAX,
        };
        let before = clock;
        assert_eq!(
            clock.observe(60),
            Err(MobileClockSessionError::RevisionExhausted)
        );
        assert_eq!(clock, before);
    }

    #[test]
    fn successful_commit_drops_queued_press_and_latches_through_release() {
        let mut barrier = CompatibleVerificationInputBarrier::new();
        assert!(!barrier.blocks_reservation());
        assert!(!barrier.drop_sample(true));
        assert!(barrier.begin_reservation());
        assert!(barrier.begin_completion_drain());
        assert!(barrier.drop_sample(true));
        assert!(barrier.finish_completion_drain(false));
        assert_eq!(barrier, CompatibleVerificationInputBarrier::AwaitingRelease);

        assert!(barrier.drop_sample(true));
        assert!(barrier.blocks_reservation());
        assert!(barrier.drop_sample(false));
        assert_eq!(barrier, CompatibleVerificationInputBarrier::Open);
        assert!(!barrier.drop_sample(true));
    }

    #[test]
    fn failed_verification_abort_drops_the_complete_overlapping_contact() {
        let mut barrier = CompatibleVerificationInputBarrier::new();
        assert!(barrier.begin_reservation());
        assert!(barrier.drop_sample(true));
        assert!(barrier.begin_completion_drain());
        assert!(barrier.finish_completion_drain(false));
        assert_eq!(barrier, CompatibleVerificationInputBarrier::AwaitingRelease);

        for pressed in [true, true, false] {
            assert!(barrier.drop_sample(pressed));
        }
        assert_eq!(barrier, CompatibleVerificationInputBarrier::Open);

        assert!(barrier.begin_reservation());
        assert!(barrier.begin_completion_drain());
        assert!(barrier.drop_sample(true));
        assert!(barrier.drop_sample(false));
        assert!(barrier.finish_completion_drain(false));
        assert_eq!(barrier, CompatibleVerificationInputBarrier::Open);
        assert!(!barrier.drop_sample(true));
    }

    #[test]
    fn empty_terminal_drain_preserves_the_known_released_boundary() {
        let mut barrier = CompatibleVerificationInputBarrier::new();
        assert!(barrier.begin_reservation());
        assert!(barrier.begin_completion_drain());
        assert!(barrier.finish_completion_drain(false));
        assert_eq!(barrier, CompatibleVerificationInputBarrier::Open);
        assert!(!barrier.drop_sample(true));
    }

    #[test]
    fn bounded_terminal_drain_requires_a_fresh_release_boundary() {
        let mut barrier = CompatibleVerificationInputBarrier::new();
        assert!(barrier.begin_reservation());
        assert!(barrier.begin_completion_drain());
        assert!(barrier.drop_sample(false));
        assert!(barrier.finish_completion_drain(true));
        assert_eq!(barrier, CompatibleVerificationInputBarrier::AwaitingRelease);
        assert!(barrier.drop_sample(false));
        assert_eq!(barrier, CompatibleVerificationInputBarrier::Open);
    }

    #[test]
    fn deferred_system_ui_epoch_supersedes_the_next_transition_frame() {
        let mut events = UiServerEventTracker::new();
        events.accept(UiServerEvent::ready(7).unwrap()).unwrap();
        events
            .accept(
                UiServerEvent::system_ui_changed(
                    7,
                    UiSystemUiMode::Foreground,
                    Some(ShellAppId::Phone),
                    false,
                    0,
                    1,
                )
                .unwrap(),
            )
            .unwrap();
        let delivered = tracked_mobile_system_ui_epoch(&events).unwrap();

        events
            .accept(
                UiServerEvent::system_ui_changed(
                    7,
                    UiSystemUiMode::Home,
                    Some(ShellAppId::Phone),
                    false,
                    0,
                    2,
                )
                .unwrap(),
            )
            .unwrap();
        assert_ne!(tracked_mobile_system_ui_epoch(&events), Some(delivered));
    }

    #[test]
    fn mobile_buffer_present_requires_the_exact_current_system_ui_revision() {
        let current = BufferPresent::client_with_system_ui_revision(9, 3, 73, 11).unwrap();
        let stale = BufferPresent::client_with_system_ui_revision(9, 3, 73, 10).unwrap();
        let future = BufferPresent::client_with_system_ui_revision(9, 3, 73, 12).unwrap();
        let unbound = BufferPresent::client(9, 3, 73).unwrap();

        assert!(mobile_buffer_present_revision_is_current(current, 11));
        assert!(!mobile_buffer_present_revision_is_current(stale, 11));
        assert!(!mobile_buffer_present_revision_is_current(future, 11));
        assert!(!mobile_buffer_present_revision_is_current(unbound, 11));
        assert!(!mobile_buffer_present_revision_is_current(current, 0));
    }

    #[test]
    fn transition_quarantine_latches_only_an_unreleased_contact() {
        assert_eq!(
            advance_mobile_transition_input_quarantine(0, true),
            Some((1, true))
        );
        assert_eq!(
            advance_mobile_transition_input_quarantine(1, true),
            Some((2, true))
        );
        assert_eq!(
            advance_mobile_transition_input_quarantine(2, false),
            Some((3, false))
        );
        assert_eq!(
            advance_mobile_transition_input_quarantine(0, false),
            Some((1, false))
        );
        assert_eq!(
            advance_mobile_transition_input_quarantine(u64::MAX, false),
            None
        );
    }

    #[cfg(feature = "androidbox-el0-runtime0")]
    fn compatible_foreground_session() -> (UiSystemUiSession, UiCompatibleActivityIdentity) {
        let identity = UiCompatibleActivityIdentity::new(71, 9).unwrap();
        let recent = Some(UiRecentIdentity::CompatibleAndroid(identity));
        let mut session = UiSystemUiSession::new();
        session
            .apply_client_recent_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 1, 1)
            .unwrap();
        session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                recent,
                2,
                2,
            )
            .unwrap();
        session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::PresentCompatibleActivity,
                recent,
                3,
                3,
            )
            .unwrap();
        (session, identity)
    }

    #[cfg(feature = "androidbox-el0-runtime0")]
    #[test]
    fn compatible_app_present_requires_a_stable_foreground_epoch() {
        let (mut foreground, identity) = compatible_foreground_session();
        assert!(compatible_app_surface_is_presentable(foreground));

        let nav = foreground.begin_nav().unwrap();
        assert!(nav.nav_pressed());
        assert!(!compatible_app_surface_is_presentable(foreground));
        let home = foreground.finish_nav(UiSystemUiMode::Home).unwrap();
        assert_eq!(
            home.recent(),
            Some(UiRecentIdentity::CompatibleAndroid(identity))
        );
        assert!(!compatible_app_surface_is_presentable(foreground));
    }

    #[cfg(feature = "androidbox-el0-runtime0")]
    #[test]
    fn app_back_finishes_compatible_activity_and_advances_request_epoch() {
        let (mut session, identity) = compatible_foreground_session();
        let previous_revision = session.revision();
        let previous_request_id = session.last_request_id();
        let (update, finished, request_id) =
            finish_compatible_activity_from_app(&mut session).unwrap();

        assert_eq!(finished, identity);
        assert_eq!(request_id, previous_request_id + 1);
        assert_eq!(update.revision(), previous_revision + 1);
        assert_eq!(update.mode(), UiSystemUiMode::Home);
        assert_eq!(update.recent(), None);
        assert_eq!(session.last_request_id(), request_id);
        assert!(!compatible_app_surface_is_presentable(session));
        assert!(finish_compatible_activity_from_app(&mut session).is_none());
    }
}

#[cfg(all(feature = "mobile-ui-runtime", feature = "androidbox-apk-install0"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompatibleActivityLaunchOrigin {
    AllApps,
    Overview,
}

#[cfg(all(feature = "mobile-ui-runtime", feature = "androidbox-apk-install0"))]
impl CompatibleActivityLaunchOrigin {
    const fn reservation_origin(self) -> UiCompatibleActivityReservationOrigin {
        match self {
            Self::AllApps => UiCompatibleActivityReservationOrigin::Home,
            Self::Overview => UiCompatibleActivityReservationOrigin::Overview,
        }
    }

    const fn mode(self) -> UiSystemUiMode {
        match self {
            Self::AllApps => UiSystemUiMode::Home,
            Self::Overview => UiSystemUiMode::Overview,
        }
    }

    const fn commit_action(self) -> UiSystemUiAction {
        match self {
            Self::AllApps => UiSystemUiAction::PresentCompatibleActivity,
            Self::Overview => UiSystemUiAction::ActivateRecent,
        }
    }
}

#[cfg(all(feature = "mobile-ui-runtime", feature = "androidbox-apk-install0"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompatibleActivityLaunchPhase {
    ClearRecent {
        request_id: u64,
        stale_identity: UiCompatibleActivityIdentity,
    },
    Reservation {
        request_id: u64,
    },
    Commit {
        request_id: u64,
        reserve_request_id: u64,
        reserve_revision: u64,
    },
    Abort {
        action: UiSystemUiAction,
        request_id: u64,
        reserve_request_id: u64,
        reserve_revision: u64,
    },
}

#[cfg(all(feature = "mobile-ui-runtime", feature = "androidbox-apk-install0"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CompatibleActivityLaunchOperation {
    identity: UiCompatibleActivityIdentity,
    request_sequence: u64,
    origin: CompatibleActivityLaunchOrigin,
    origin_recent: Option<UiRecentIdentity>,
    phase: CompatibleActivityLaunchPhase,
}

#[cfg(feature = "mobile-ui-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MobileDeferredPayloadQueueError {
    Full,
    Invariant,
}

#[cfg(feature = "mobile-ui-runtime")]
struct MobileDeferredPayloadQueue {
    slots: [Option<UiServerEventPayload>; MOBILE_DEFERRED_PAYLOAD_QUEUE_CAPACITY],
    head: usize,
    len: usize,
}

#[cfg(feature = "mobile-ui-runtime")]
impl MobileDeferredPayloadQueue {
    const fn new() -> Self {
        Self {
            slots: [None; MOBILE_DEFERRED_PAYLOAD_QUEUE_CAPACITY],
            head: 0,
            len: 0,
        }
    }

    fn push(
        &mut self,
        payload: UiServerEventPayload,
    ) -> Result<(), MobileDeferredPayloadQueueError> {
        if self.head >= MOBILE_DEFERRED_PAYLOAD_QUEUE_CAPACITY
            || self.len > MOBILE_DEFERRED_PAYLOAD_QUEUE_CAPACITY
        {
            return Err(MobileDeferredPayloadQueueError::Invariant);
        }
        if self.len == MOBILE_DEFERRED_PAYLOAD_QUEUE_CAPACITY {
            return Err(MobileDeferredPayloadQueueError::Full);
        }
        let tail = (self.head + self.len) % MOBILE_DEFERRED_PAYLOAD_QUEUE_CAPACITY;
        if self.slots[tail].is_some() {
            return Err(MobileDeferredPayloadQueueError::Invariant);
        }
        self.slots[tail] = Some(payload);
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Result<Option<UiServerEventPayload>, MobileDeferredPayloadQueueError> {
        if self.head >= MOBILE_DEFERRED_PAYLOAD_QUEUE_CAPACITY
            || self.len > MOBILE_DEFERRED_PAYLOAD_QUEUE_CAPACITY
        {
            return Err(MobileDeferredPayloadQueueError::Invariant);
        }
        if self.len == 0 {
            return Ok(None);
        }
        let payload = self.slots[self.head].ok_or(MobileDeferredPayloadQueueError::Invariant)?;
        self.slots[self.head] = None;
        self.head = (self.head + 1) % MOBILE_DEFERRED_PAYLOAD_QUEUE_CAPACITY;
        self.len -= 1;
        Ok(Some(payload))
    }
}

#[cfg(feature = "mobile-ui-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MobilePresentOutcome {
    Presented,
    Superseded,
}

#[cfg(feature = "mobile-ui-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MobileInputPolicy {
    Defer,
    Quarantine,
}

#[cfg(feature = "mobile-ui-runtime")]
const MOBILE_ROW_BYTES: usize = MOBILE_WIDTH * core::mem::size_of::<u32>();
#[cfg(feature = "mobile-ui-runtime")]
const MOBILE_TRANSFER_ROWS: usize = GRAPHICS_BUFFER_WRITE_MAX_BYTES / MOBILE_ROW_BYTES;
#[cfg(feature = "mobile-ui-runtime")]
const MOBILE_TRANSFER_PIXELS: usize = MOBILE_WIDTH * MOBILE_TRANSFER_ROWS;
#[cfg(feature = "mobile-ui-runtime")]
const _: () = assert!(MOBILE_TRANSFER_ROWS == bndr_abi::GRAPHICS_BUFFER_WRITE_MAX_ROWS);
#[cfg(feature = "mobile-ui-runtime")]
const _: () = assert!(MOBILE_TRANSFER_ROWS > 1);
#[cfg(feature = "mobile-ui-runtime")]
const _: () = assert!(GRAPHICS_BUFFER_WRITE_MAX_BYTES == MOBILE_TRANSFER_PIXELS * 4);

#[cfg(feature = "mobile-ui-runtime")]
#[repr(align(4096))]
struct MobileTransferPixels(UnsafeCell<[u32; MOBILE_TRANSFER_PIXELS]>);

#[cfg(feature = "mobile-ui-runtime")]
unsafe impl Sync for MobileTransferPixels {}

#[cfg(feature = "mobile-ui-runtime")]
static MOBILE_TRANSFER_PIXELS_BUFFER: MobileTransferPixels =
    MobileTransferPixels(UnsafeCell::new([0; MOBILE_TRANSFER_PIXELS]));

#[derive(Clone, Copy, Eq, PartialEq)]
enum UiServerWaitSource {
    Surface,
    Launcher,
    App,
}

enum UiClientMessage {
    Present(PresentFrame),
    BufferPresent {
        frame: BufferPresent,
        buffer: Option<OwnedUserHandle>,
    },
    Control(UiClientControl),
}

impl OwnedUserHandle {
    fn new(raw: u64) -> Option<Self> {
        let encoded = u32::try_from(raw).ok()?;
        HandleValue::from_raw(encoded)
            .is_valid()
            .then_some(Self { raw })
    }

    const fn raw(&self) -> u64 {
        self.raw
    }

    #[cfg(any(
        feature = "service-supervisor-runtime",
        feature = "service-dependency-runtime"
    ))]
    fn retire(&mut self) -> Self {
        let raw = core::mem::replace(&mut self.raw, 0);
        Self::new(raw).unwrap_or_else(|| fail(FAIL_SM_CLOSE))
    }
}

struct RegisteredService {
    connector: OwnedUserHandle,
    owner_pid: u64,
}

struct ChildRound {
    startup_parent: u64,
    startup_child: u64,
    service_parent: u64,
    service_child: u64,
    abandoned_parent: u64,
    abandoned_child: u64,
    event_parent: u64,
    event_child: u64,
    pid: u64,
}

struct ManagerResidentState {
    provider_session: OwnedUserHandle,
    client_session: OwnedUserHandle,
    secondary_session: Option<ResidentClientSession>,
    secondary_pid: u64,
    secondary_lease: u16,
    services: ServiceRegistry,
    provider_pid: u64,
    client_pid: u64,
    supervisor_pid: u64,
    instance: InstanceId,
}

struct ResidentClientSession {
    channel: OwnedUserHandle,
    owner_pid: u64,
    lease: u16,
}

struct PendingProviderEcho {
    endpoint: OwnedUserHandle,
    transaction_id: u32,
    lease: u16,
    expected_client_pid: u64,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ManagerDispatchOutcome {
    Authorized,
    Unauthorized,
    Malformed,
}

struct ProviderResidentState {
    session: OwnedUserHandle,
    connector: OwnedUserHandle,
    instance: InstanceId,
    supervisor_pid: u64,
}

struct ClientResidentState {
    session: OwnedUserHandle,
    instance: InstanceId,
    supervisor_pid: u64,
    client_pid: u64,
}

struct ResidentChildren {
    manager_pid: u64,
    provider_pid: u64,
    client_pid: u64,
    secondary_pid: u64,
    surface_server_pid: u64,
    launcher_pid: u64,
    app_pid: u64,
    #[cfg(feature = "androidbox-process0")]
    android_app_pid: u64,
    #[cfg(feature = "androidbox-restart0")]
    android_app_supervisor: Option<OwnedUserHandle>,
    #[cfg(feature = "input-server-runtime")]
    input_server_pid: u64,
    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    input_server_session: u64,
    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    input_route_epoch: u64,
    manager_control: OwnedUserHandle,
    provider_control: OwnedUserHandle,
    client_control: OwnedUserHandle,
    secondary_control: Option<OwnedUserHandle>,
    #[cfg(feature = "input-server-runtime")]
    input_server_control: Option<OwnedUserHandle>,
}

trait ResidentState {
    fn is_reachable(&self) -> bool;
}

impl ResidentState for ManagerResidentState {
    fn is_reachable(&self) -> bool {
        let Some(entry) = self.services.lookup(&echo_service_name()) else {
            return false;
        };
        !self.services.is_empty()
            && self.services.len() <= self.services.capacity()
            && self.provider_session.raw() != self.client_session.raw()
            && self.secondary_session.as_ref().is_none_or(|secondary| {
                secondary.channel.raw() != self.provider_session.raw()
                    && secondary.channel.raw() != self.client_session.raw()
                    && secondary.owner_pid == self.secondary_pid
                    && secondary.lease == self.secondary_lease
            })
            && ((self.secondary_pid == 0 && self.secondary_lease == 0)
                || (self.secondary_pid != 0
                    && matches!(
                        self.secondary_lease,
                        M20_SECONDARY_FIRST_LEASE | M20_SECONDARY_REATTACHED_LEASE
                    )))
            && self.provider_pid != 0
            && self.client_pid != 0
            && self.supervisor_pid != 0
            && self.provider_pid != self.client_pid
            && self.provider_pid != self.supervisor_pid
            && self.client_pid != self.supervisor_pid
            && entry.resource().owner_pid == self.provider_pid
            && entry.instance() == self.instance
            && entry.resource().connector.raw() != self.provider_session.raw()
            && entry.resource().connector.raw() != self.client_session.raw()
            && self.services.iter().all(|registered| {
                registered.resource().owner_pid == self.provider_pid
                    && registered.resource().connector.raw() != self.provider_session.raw()
                    && registered.resource().connector.raw() != self.client_session.raw()
                    && self.secondary_session.as_ref().is_none_or(|secondary| {
                        registered.resource().connector.raw() != secondary.channel.raw()
                    })
            })
    }
}

impl ResidentState for ProviderResidentState {
    fn is_reachable(&self) -> bool {
        self.session.raw() != self.connector.raw()
            && self.instance.raw() != 0
            && self.supervisor_pid != 0
    }
}

impl ResidentState for ClientResidentState {
    fn is_reachable(&self) -> bool {
        self.session.raw() != 0
            && self.instance.raw() != 0
            && self.supervisor_pid != 0
            && self.client_pid != 0
            && self.client_pid != self.supervisor_pid
    }
}

type ServiceRegistry = Registry<RegisteredService, 4>;

fn prepare_entry(expected: UserImageId, startup_handle: u64, raw_image_id: u64, child: bool) {
    PROCESS_ROLE.store(u64::from(child), Ordering::Relaxed);
    if UserImageId::from_raw(raw_image_id) != Some(expected)
        || (expected != UserImageId::Init && startup_handle == 0)
    {
        fail(29);
    }
    verify_fresh_image();
    verify_stack_pages();
}

#[cfg(all(feature = "app-data-runtime", not(feature = "storage-server-runtime")))]
fn expect_app_data_root_denied() {
    let denied = syscall(SyscallNumber::AppDataRootOpen, 0, 0, 0);
    if denied.status != Status::PermissionDenied.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_APP_DATA_AUTHORITY);
    }
}

pub fn init_entry(startup_handle: u64, raw_image_id: u64) -> ! {
    prepare_entry(UserImageId::Init, startup_handle, raw_image_id, false);
    #[cfg(all(feature = "app-data-runtime", not(feature = "storage-server-runtime")))]
    expect_app_data_root_denied();
    if startup_handle == PROCESS_TERMINATE_SELF_TEST_INIT_ARGUMENT {
        process_terminate_self_test()
    }
    wait_for_lower_el_timer_preemption();
    #[cfg(feature = "unified-product-runtime")]
    product_runtime::init_runtime(startup_handle);
    #[cfg(all(
        feature = "resident-platform-shutdown-runtime",
        not(feature = "unified-product-runtime")
    ))]
    resident_shutdown_runtime::init_runtime(startup_handle);
    #[cfg(all(
        feature = "storage-server-runtime",
        not(feature = "resident-platform-shutdown-runtime"),
        not(feature = "unified-product-runtime")
    ))]
    storage_server_runtime::init_runtime(startup_handle);
    #[cfg(all(
        feature = "app-lifecycle-runtime",
        not(feature = "storage-server-runtime")
    ))]
    lifecycle_runtime::init_runtime(startup_handle);
    #[cfg(not(any(feature = "app-lifecycle-runtime", feature = "storage-server-runtime")))]
    parent_start(startup_handle)
}

fn process_terminate_self_test() -> ! {
    let abi = syscall(SyscallNumber::AbiVersion, 0, 0, 0);
    if abi.status != Status::Ok.raw() || abi.out1 != ABI_VERSION || abi.out2 != 0 {
        fail(200);
    }
    let channel = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    if channel.status != Status::Ok.raw() || channel.out1 == 0 || channel.out2 == 0 {
        fail(201);
    }
    let spawned = syscall(
        SyscallNumber::ProcessSpawn,
        channel.out2,
        UserImageId::Client.raw(),
        PROCESS_SPAWN_FLAGS_NONE,
    );
    if spawned.status != Status::Ok.raw() || spawned.out1 == 0 || spawned.out2 != 0 {
        fail(202);
    }

    // Yield exactly once to the next dynamic context. The Client validates
    // ABI19, then blocks on its empty startup Channel with a single ObjectWait.
    // When init resumes, the target is deterministically Waiting rather than
    // merely Runnable, so the forced-termination token-abandon path is real.
    unsafe {
        asm!("wfi", options(nomem, nostack, preserves_flags));
    }

    let terminated = syscall(
        SyscallNumber::ProcessTerminate,
        spawned.out1,
        PROCESS_TERMINATE_FLAGS_NONE,
        0,
    );
    if terminated.status != Status::Ok.raw() || terminated.out1 != 0 || terminated.out2 != 0 {
        fail(203);
    }
    let waited = syscall(SyscallNumber::ProcessWait, spawned.out1, 0, 0);
    if waited.status != Status::Ok.raw()
        || waited.out1 != PROCESS_KILLED_EXIT_CODE
        || waited.out2 != ProcessTerminationReason::Killed.raw()
    {
        fail(204);
    }
    let peer_closed = object_wait(channel.out1, ObjectSignals::PEER_CLOSED);
    if peer_closed.status != Status::Ok.raw()
        || peer_closed.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
        || peer_closed.out2 != 0
        || syscall(SyscallNumber::HandleClose, channel.out1, 0, 0).status != Status::Ok.raw()
    {
        fail(205);
    }

    for invalid in [
        syscall(SyscallNumber::ProcessTerminate, 0, 0, 0),
        syscall(SyscallNumber::ProcessTerminate, spawned.out1, 1, 0),
        syscall(SyscallNumber::ProcessTerminate, spawned.out1, 0, 1),
    ] {
        if invalid.status != Status::InvalidArgument.raw() || invalid.out1 != 0 || invalid.out2 != 0
        {
            fail(206);
        }
    }
    let init_pid = (1_u64 << 32) | 1;
    for missing in [
        syscall(SyscallNumber::ProcessTerminate, spawned.out1, 0, 0),
        syscall(SyscallNumber::ProcessTerminate, init_pid, 0, 0),
        syscall(SyscallNumber::ProcessWait, spawned.out1, 0, 0),
    ] {
        if missing.status != Status::NotFound.raw() || missing.out1 != 0 || missing.out2 != 0 {
            fail(207);
        }
    }

    let ready = syscall(
        SyscallNumber::InitReady,
        INIT_READY_MAGIC,
        PROCESS_TERMINATE_SELF_TEST_READY_1,
        PROCESS_TERMINATE_SELF_TEST_READY_2,
    );
    if ready.status != Status::Ok.raw() || ready.out1 != 0 || ready.out2 != 0 {
        fail(208);
    }
    loop {
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

fn wait_for_lower_el_timer_preemption() {
    // The kernel sets SCTLR_EL1.nTWI so EL0 WFI is permitted. The periodic
    // physical timer is already armed before init is activated, so this
    // resumes only after a genuine lower-EL IRQ entry. This replaces a
    // build-speed-dependent hope that a timer happens to land between two
    // short syscalls.
    unsafe {
        asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}

pub fn service_manager_entry(startup_handle: u64, raw_image_id: u64) -> ! {
    prepare_entry(
        UserImageId::ServiceManager,
        startup_handle,
        raw_image_id,
        true,
    );
    #[cfg(all(
        feature = "resident-platform-shutdown-runtime",
        not(feature = "unified-product-runtime")
    ))]
    {
        resident_shutdown_runtime::node_runtime(startup_handle, UserImageId::ServiceManager)
    }
    #[cfg(any(
        not(feature = "resident-platform-shutdown-runtime"),
        feature = "unified-product-runtime"
    ))]
    {
        let supervisor_pid = child_prelude(startup_handle, Role::ServiceManager);
        let (epoch, state) = child_service_manager_round(startup_handle, supervisor_pid);
        if epoch == FIRST_MANAGER_EPOCH {
            exit_child(MANAGER_RESTART_EXIT)
        }
        #[cfg(feature = "unified-product-runtime")]
        product_runtime::arm(
            startup_handle,
            ShutdownServiceNode::ServiceManager,
            supervisor_pid,
        );
        manager_resident_loop(startup_handle, epoch, state)
    }
}

pub fn provider_entry(startup_handle: u64, raw_image_id: u64) -> ! {
    prepare_entry(UserImageId::Provider, startup_handle, raw_image_id, true);
    #[cfg(all(
        feature = "resident-platform-shutdown-runtime",
        not(feature = "unified-product-runtime")
    ))]
    {
        resident_shutdown_runtime::node_runtime(startup_handle, UserImageId::Provider)
    }
    #[cfg(any(
        not(feature = "resident-platform-shutdown-runtime"),
        feature = "unified-product-runtime"
    ))]
    {
        let supervisor_pid = child_prelude(startup_handle, Role::Provider);
        let state = child_provider_service_round(startup_handle, supervisor_pid);
        #[cfg(feature = "unified-product-runtime")]
        product_runtime::arm(
            startup_handle,
            ShutdownServiceNode::Provider,
            supervisor_pid,
        );
        provider_resident_loop(startup_handle, SECOND_MANAGER_EPOCH, state)
    }
}

pub fn client_entry(startup_handle: u64, raw_image_id: u64) -> ! {
    prepare_entry(UserImageId::Client, startup_handle, raw_image_id, true);
    #[cfg(all(
        feature = "resident-platform-shutdown-runtime",
        not(feature = "unified-product-runtime")
    ))]
    {
        resident_shutdown_runtime::node_runtime(startup_handle, UserImageId::Client)
    }
    #[cfg(any(
        not(feature = "resident-platform-shutdown-runtime"),
        feature = "unified-product-runtime"
    ))]
    {
        let supervisor_pid = child_prelude(startup_handle, Role::Client);
        let (mode_sender, (mode_tag, mode)) = read_scalar_envelope(startup_handle);
        if mode_sender != supervisor_pid || mode_tag != CLIENT_MODE_TAG {
            fail(FAIL_MULTI_CLIENT_CONFIG);
        }
        match mode {
            CLIENT_MODE_PRIMARY => {
                let state = child_client_service_round(startup_handle, supervisor_pid);
                #[cfg(feature = "unified-product-runtime")]
                product_runtime::arm(
                    startup_handle,
                    ShutdownServiceNode::PrimaryClient,
                    supervisor_pid,
                );
                client_resident_loop(startup_handle, SECOND_MANAGER_EPOCH, state)
            }
            CLIENT_MODE_SECONDARY => {
                #[cfg(feature = "unified-product-runtime")]
                product_runtime::arm(
                    startup_handle,
                    ShutdownServiceNode::SecondaryClient,
                    supervisor_pid,
                );
                secondary_client_loop(startup_handle, supervisor_pid)
            }
            _ => fail(FAIL_MULTI_CLIENT_CONFIG),
        }
    }
}

pub fn surface_server_entry(startup_handle: u64, raw_image_id: u64) -> ! {
    prepare_entry(
        UserImageId::SurfaceServer,
        startup_handle,
        raw_image_id,
        true,
    );
    #[cfg(all(
        feature = "resident-platform-shutdown-runtime",
        not(feature = "unified-product-runtime")
    ))]
    {
        resident_shutdown_runtime::node_runtime(startup_handle, UserImageId::SurfaceServer)
    }
    #[cfg(any(
        not(feature = "resident-platform-shutdown-runtime"),
        feature = "unified-product-runtime"
    ))]
    {
        #[cfg(all(feature = "app-data-runtime", not(feature = "storage-server-runtime")))]
        expect_app_data_root_denied();
        #[cfg(all(
            feature = "multi-window-runtime",
            any(
                feature = "input-server-restart-runtime",
                feature = "service-dependency-runtime",
                all(
                    not(feature = "graphics-owner-death-runtime"),
                    not(feature = "app-crash-recovery-runtime")
                )
            )
        ))]
        lifecycle_runtime::multi_window_surface_runtime(startup_handle);
        #[cfg(all(
            feature = "app-lifecycle-runtime",
            not(all(
                feature = "multi-window-runtime",
                any(
                    feature = "input-server-restart-runtime",
                    feature = "service-dependency-runtime",
                    all(
                        not(feature = "graphics-owner-death-runtime"),
                        not(feature = "app-crash-recovery-runtime")
                    )
                )
            ))
        ))]
        lifecycle_runtime::surface_runtime(startup_handle);
        #[cfg(not(feature = "app-lifecycle-runtime"))]
        surface_server_loop(startup_handle)
    }
}

pub fn input_server_entry(startup_handle: u64, raw_image_id: u64) -> ! {
    prepare_entry(UserImageId::InputServer, startup_handle, raw_image_id, true);
    #[cfg(all(
        feature = "resident-platform-shutdown-runtime",
        not(feature = "unified-product-runtime")
    ))]
    {
        resident_shutdown_runtime::node_runtime(startup_handle, UserImageId::InputServer)
    }
    #[cfg(any(
        not(feature = "resident-platform-shutdown-runtime"),
        feature = "unified-product-runtime"
    ))]
    {
        #[cfg(feature = "service-dependency-runtime")]
        input_server_runtime::run_dependency(startup_handle);
        #[cfg(all(
            feature = "input-server-restart-runtime",
            not(feature = "service-dependency-runtime")
        ))]
        input_server_runtime::run_restart(startup_handle);
        #[cfg(all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime"),
            not(feature = "service-dependency-runtime")
        ))]
        input_server_runtime::run_surface_restart(startup_handle);
        #[cfg(all(
            feature = "input-server-runtime",
            not(feature = "input-server-surface-restart-runtime"),
            not(feature = "input-server-restart-runtime"),
            not(feature = "service-dependency-runtime")
        ))]
        input_server_runtime::run(startup_handle);
        #[cfg(not(feature = "input-server-runtime"))]
        fail(220)
    }
}

pub fn launcher_entry(startup_handle: u64, raw_image_id: u64) -> ! {
    prepare_entry(UserImageId::Launcher, startup_handle, raw_image_id, true);
    #[cfg(all(
        feature = "resident-platform-shutdown-runtime",
        not(feature = "unified-product-runtime")
    ))]
    {
        resident_shutdown_runtime::node_runtime(startup_handle, UserImageId::Launcher)
    }
    #[cfg(any(
        not(feature = "resident-platform-shutdown-runtime"),
        feature = "unified-product-runtime"
    ))]
    {
        #[cfg(all(feature = "app-data-runtime", not(feature = "storage-server-runtime")))]
        expect_app_data_root_denied();
        #[cfg(feature = "unified-product-runtime")]
        lifecycle_runtime::multi_window_launcher_runtime(startup_handle);
        #[cfg(all(
            feature = "storage-server-runtime",
            not(feature = "unified-product-runtime")
        ))]
        storage_server_runtime::client_runtime(
            startup_handle,
            bndr_abi::AppDataPrincipal::LAUNCHER,
        );
        #[cfg(all(
            feature = "multi-window-runtime",
            not(feature = "storage-server-runtime"),
            any(
                feature = "input-server-restart-runtime",
                feature = "service-dependency-runtime",
                all(
                    not(feature = "graphics-owner-death-runtime"),
                    not(feature = "app-crash-recovery-runtime")
                )
            )
        ))]
        lifecycle_runtime::multi_window_launcher_runtime(startup_handle);
        #[cfg(all(
            feature = "app-lifecycle-runtime",
            not(feature = "storage-server-runtime"),
            not(all(
                feature = "multi-window-runtime",
                any(
                    feature = "input-server-restart-runtime",
                    feature = "service-dependency-runtime",
                    all(
                        not(feature = "graphics-owner-death-runtime"),
                        not(feature = "app-crash-recovery-runtime")
                    )
                )
            ))
        ))]
        lifecycle_runtime::launcher_runtime(startup_handle);
        #[cfg(all(
            feature = "mobile-ui-runtime",
            not(any(feature = "app-lifecycle-runtime", feature = "storage-server-runtime"))
        ))]
        mobile_launcher_loop(startup_handle);
        #[cfg(all(
            not(feature = "mobile-ui-runtime"),
            not(any(feature = "app-lifecycle-runtime", feature = "storage-server-runtime"))
        ))]
        launcher_loop(startup_handle)
    }
}

pub fn app_entry(startup_handle: u64, raw_image_id: u64) -> ! {
    prepare_entry(UserImageId::App, startup_handle, raw_image_id, true);
    #[cfg(all(
        feature = "resident-platform-shutdown-runtime",
        not(feature = "unified-product-runtime")
    ))]
    {
        resident_shutdown_runtime::node_runtime(startup_handle, UserImageId::App)
    }
    #[cfg(any(
        not(feature = "resident-platform-shutdown-runtime"),
        feature = "unified-product-runtime"
    ))]
    {
        #[cfg(feature = "unified-product-runtime")]
        lifecycle_runtime::multi_window_app_runtime(startup_handle);
        #[cfg(all(
            feature = "storage-server-runtime",
            not(feature = "unified-product-runtime")
        ))]
        storage_server_runtime::client_runtime(
            startup_handle,
            bndr_abi::AppDataPrincipal::PRIMARY_APP,
        );
        #[cfg(all(
            feature = "multi-window-runtime",
            not(feature = "storage-server-runtime"),
            any(
                feature = "input-server-restart-runtime",
                feature = "service-dependency-runtime",
                all(
                    not(feature = "graphics-owner-death-runtime"),
                    not(feature = "app-crash-recovery-runtime")
                )
            )
        ))]
        lifecycle_runtime::multi_window_app_runtime(startup_handle);
        #[cfg(all(
            feature = "app-lifecycle-runtime",
            not(feature = "storage-server-runtime"),
            not(all(
                feature = "multi-window-runtime",
                any(
                    feature = "input-server-restart-runtime",
                    feature = "service-dependency-runtime",
                    all(
                        not(feature = "graphics-owner-death-runtime"),
                        not(feature = "app-crash-recovery-runtime")
                    )
                )
            ))
        ))]
        lifecycle_runtime::app_runtime(startup_handle);
        #[cfg(all(
            feature = "mobile-ui-runtime",
            not(any(feature = "app-lifecycle-runtime", feature = "storage-server-runtime"))
        ))]
        mobile_app_loop(startup_handle);
        #[cfg(all(
            not(feature = "mobile-ui-runtime"),
            not(any(feature = "app-lifecycle-runtime", feature = "storage-server-runtime"))
        ))]
        app_loop(startup_handle)
    }
}

#[cfg(feature = "androidbox-process0")]
pub fn android_app_entry(startup_handle: u64, raw_image_id: u64) -> ! {
    prepare_entry(UserImageId::AndroidApp, startup_handle, raw_image_id, true);
    androidapp_process::worker_loop(startup_handle)
}

#[cfg(feature = "storage-server-runtime")]
pub fn storage_server_entry(startup_handle: u64, raw_image_id: u64) -> ! {
    prepare_entry(
        UserImageId::StorageServer,
        startup_handle,
        raw_image_id,
        true,
    );
    storage_server_runtime::server_runtime(startup_handle)
}

fn verify_fresh_image() {
    if unsafe { ptr::read_volatile(BYTE_MESSAGE.as_ptr()) } != b'B' {
        fail(40);
    }
    if FILE_BACKED_PROBE
        .compare_exchange(
            DATA_INITIAL,
            DATA_MUTATED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        )
        .is_err()
    {
        fail(37);
    }
    if ZERO_FILLED_PROBE
        .compare_exchange(0, BSS_MUTATED, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        fail(38);
    }
    if FILE_BACKED_PROBE.load(Ordering::SeqCst) != DATA_MUTATED
        || ZERO_FILLED_PROBE.load(Ordering::SeqCst) != BSS_MUTATED
    {
        fail(39);
    }
}

fn verify_stack_pages() {
    for page in 0..USER_STACK_PAGES {
        let value = STACK_SENTINEL ^ page as u64;
        let address = (USER_STACK_START + page * USER_PAGE_SIZE + 8) as *mut u64;
        unsafe {
            ptr::write_volatile(address, value);
            if ptr::read_volatile(address) != value {
                fail(41);
            }
        }
    }
}

fn parent_start(system_root: u64) -> ! {
    let stack_value = stack_round_trip();
    if stack_value != STACK_SENTINEL {
        fail(1);
    }

    if private_sleep_probe().status != Status::Unsupported.raw() {
        fail(2);
    }

    let abi = syscall(SyscallNumber::AbiVersion, 0, 0, 0);
    if abi.status != Status::Ok.raw() {
        fail(3);
    }
    if abi.out1 != ABI_VERSION {
        fail(4);
    }

    if raw_syscall(0xffff, 0, 0, 0).status != Status::Unsupported.raw() {
        fail(5);
    }

    // The storage timeout/reset recovery image deliberately stops at the M22
    // boundary and therefore has no published M24 catalog/root capability.
    // Normal M24 boots always receive a nonzero root and run the full proof.
    if system_root != 0 {
        exercise_el0_storage(system_root);
    }

    exercise_object_wait_many_array();

    let channel = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    if channel.status != Status::Ok.raw() {
        fail(6);
    }
    let left = channel.out1;
    let right = channel.out2;
    if left == 0 || right == 0 || left == right {
        fail(7);
    }

    if syscall(
        SyscallNumber::ChannelWrite,
        left,
        EXPECTED_TAG,
        EXPECTED_PAYLOAD,
    )
    .status
        != Status::Ok.raw()
    {
        fail(8);
    }

    let message = syscall(SyscallNumber::ChannelRead, right, 0, 0);
    if message.status != Status::Ok.raw() {
        fail(9);
    }
    if message.out1 != EXPECTED_TAG {
        fail(10);
    }
    if message.out2 != EXPECTED_PAYLOAD {
        fail(11);
    }

    exercise_byte_channel(left, right);

    let duplicate = syscall(
        SyscallNumber::HandleDuplicate,
        left,
        u64::from(Rights::WRITE.bits()),
        0,
    );
    if duplicate.status != Status::Ok.raw() {
        fail(12);
    }
    let write_only = duplicate.out1;
    if write_only == 0 {
        fail(13);
    }
    if syscall(SyscallNumber::ChannelRead, write_only, 0, 0).status
        != Status::PermissionDenied.raw()
    {
        fail(14);
    }

    let mut resident_children = run_supervised_service_manager_rounds();

    // Concurrent broker traffic deliberately leaves the provider's served
    // acknowledgement as the most recent scalar IPC. Restore the original
    // end-to-end sentinel immediately before InitReady so the kernel can still
    // prove a final write/read round trip after the resident roles completed
    // their second-generation echo.
    write_scalar(left, EXPECTED_TAG, EXPECTED_PAYLOAD);
    if read_scalar(right) != (EXPECTED_TAG, EXPECTED_PAYLOAD) {
        fail(FAIL_SM_ECHO);
    }

    if syscall(SyscallNumber::HandleClose, write_only, 0, 0).status != Status::Ok.raw() {
        fail(15);
    }
    if syscall(SyscallNumber::HandleClose, left, 0, 0).status != Status::Ok.raw() {
        fail(16);
    }
    if syscall(SyscallNumber::HandleClose, left, 0, 0).status != Status::NotFound.raw() {
        fail(17);
    }
    if syscall(SyscallNumber::HandleClose, right, 0, 0).status != Status::Ok.raw() {
        fail(18);
    }

    let stack_value = stack_round_trip();
    let ready = syscall(
        SyscallNumber::InitReady,
        INIT_READY_MAGIC,
        REGISTER_SENTINEL,
        stack_value,
    );
    if ready.status != Status::Ok.raw() {
        fail(19);
    }
    drive_post_ready_echo_rounds(&resident_children);
    let m15_replacement = drive_dynamic_registration_lifecycle(&resident_children);
    drive_post_cleanup_reuse_round(&resident_children, m15_replacement);
    drive_delegated_endpoint_acl_round(&resident_children);
    drive_multi_client_lookup_round(&mut resident_children);
    spawn_ui_runtime(&mut resident_children);
    supervise_resident_children(&mut resident_children)
}

fn exercise_el0_storage(system_root: u64) {
    if system_root == 0 {
        fail(FAIL_STORAGE_ROOT);
    }

    expect_storage_error(
        file_open_at_raw(
            0,
            SYSTEM_HELLO_PATH.as_ptr() as u64,
            SYSTEM_HELLO_PATH.len(),
        ),
        Status::InvalidArgument,
        FAIL_STORAGE_OPEN_INVALID_ROOT,
    );
    expect_storage_error(
        file_open_at_raw(system_root, USER_GUARD_LOW, 1),
        Status::BadAddress,
        FAIL_STORAGE_OPEN_BAD_ADDRESS,
    );
    expect_storage_error(
        file_open_at(system_root, SYSTEM_TRAVERSAL_PATH),
        Status::InvalidArgument,
        FAIL_STORAGE_OPEN_TRAVERSAL,
    );
    expect_storage_error(
        file_open_at(system_root, SYSTEM_MISSING_PATH),
        Status::NotFound,
        FAIL_STORAGE_OPEN_MISSING,
    );

    let hello = file_open_at(system_root, SYSTEM_HELLO_PATH);
    if hello.status != Status::Ok.raw()
        || hello.out1 == 0
        || hello.out2 != SYSTEM_HELLO_CONTENT.len() as u64
    {
        fail(FAIL_STORAGE_OPEN_HELLO);
    }
    let hello = hello.out1;

    let build = file_open_at(system_root, SYSTEM_BUILD_PATH);
    if build.status != Status::Ok.raw()
        || build.out1 == 0
        || build.out1 == hello
        || build.out2 != SYSTEM_BUILD_CONTENT.len() as u64
    {
        fail(FAIL_STORAGE_OPEN_BUILD);
    }
    let build = build.out1;

    expect_storage_error(
        syscall(
            SyscallNumber::HandleDuplicate,
            system_root,
            u64::from(Rights::READ.bits()),
            0,
        ),
        Status::PermissionDenied,
        FAIL_STORAGE_ROOT_DUPLICATE,
    );
    expect_storage_error(
        syscall(
            SyscallNumber::HandleDuplicate,
            build,
            u64::from(Rights::WRITE.bits()),
            0,
        ),
        Status::PermissionDenied,
        FAIL_STORAGE_BUILD_ESCALATION,
    );

    let build_duplicate = syscall(
        SyscallNumber::HandleDuplicate,
        build,
        u64::from(Rights::READ.bits()),
        0,
    );
    if build_duplicate.status != Status::Ok.raw()
        || build_duplicate.out1 == 0
        || build_duplicate.out1 == build
        || build_duplicate.out2 != 0
    {
        fail(FAIL_STORAGE_BUILD_DUPLICATE);
    }
    let build_duplicate = build_duplicate.out1;
    close_storage_handle(build, FAIL_STORAGE_BUILD_DUPLICATE);

    let mut denied_output = [0xa5_u8; 16];
    expect_storage_error(
        vmo_read_raw(build, denied_output.as_mut_ptr() as u64, 0, 1),
        Status::NotFound,
        FAIL_STORAGE_BUILD_STALE,
    );
    if denied_output != [0xa5; 16] {
        fail(FAIL_STORAGE_BUILD_STALE);
    }

    let transfer_only = syscall(
        SyscallNumber::HandleDuplicate,
        hello,
        u64::from(Rights::TRANSFER.bits()),
        0,
    );
    if transfer_only.status != Status::Ok.raw()
        || transfer_only.out1 == 0
        || transfer_only.out1 == hello
        || transfer_only.out2 != 0
    {
        fail(FAIL_STORAGE_HELLO_ATTENUATION);
    }
    let transfer_only = transfer_only.out1;
    expect_storage_error(
        vmo_read_raw(transfer_only, denied_output.as_mut_ptr() as u64, 0, 1),
        Status::PermissionDenied,
        FAIL_STORAGE_HELLO_ATTENUATION,
    );
    if denied_output != [0xa5; 16] {
        fail(FAIL_STORAGE_HELLO_ATTENUATION);
    }
    expect_storage_error(
        object_wait(hello, ObjectSignals::READABLE),
        Status::PermissionDenied,
        FAIL_STORAGE_VMO_WAIT,
    );

    let mut hello_output = [0xa5_u8; 64];
    let hello_read = vmo_read_raw(
        hello,
        hello_output.as_mut_ptr() as u64,
        0,
        SYSTEM_HELLO_CONTENT.len() as u32,
    );
    if hello_read.status != Status::Ok.raw()
        || hello_read.out1 != SYSTEM_HELLO_CONTENT.len() as u64
        || hello_read.out2 != SYSTEM_HELLO_CONTENT.len() as u64
        || &hello_output[..SYSTEM_HELLO_CONTENT.len()] != SYSTEM_HELLO_CONTENT
        || hello_output[SYSTEM_HELLO_CONTENT.len()..]
            .iter()
            .any(|byte| *byte != 0xa5)
        || fnv1a64(&hello_output[..SYSTEM_HELLO_CONTENT.len()]) != SYSTEM_HELLO_DIGEST
    {
        fail(FAIL_STORAGE_HELLO_READ);
    }

    let mut range_output = [0xa5_u8; 16];
    let range_read = vmo_read_raw(hello, range_output.as_mut_ptr() as u64, 8, 9);
    if range_read.status != Status::Ok.raw()
        || range_read.out1 != 9
        || range_read.out2 != SYSTEM_HELLO_CONTENT.len() as u64
        || &range_output[..9] != b"M23 FAT16"
        || range_output[9..].iter().any(|byte| *byte != 0xa5)
    {
        fail(FAIL_STORAGE_HELLO_RANGE);
    }

    let eof = vmo_read_raw(hello, USER_GUARD_LOW, SYSTEM_HELLO_CONTENT.len() as u32, 1);
    if eof.status != Status::Ok.raw()
        || eof.out1 != 0
        || eof.out2 != SYSTEM_HELLO_CONTENT.len() as u64
    {
        fail(FAIL_STORAGE_VMO_EOF);
    }
    expect_storage_error(
        vmo_read_raw(
            hello,
            hello_output.as_mut_ptr() as u64,
            SYSTEM_HELLO_CONTENT.len() as u32 + 1,
            1,
        ),
        Status::InvalidArgument,
        FAIL_STORAGE_VMO_OFFSET,
    );
    expect_storage_error(
        vmo_read_raw(
            hello,
            hello_output.as_mut_ptr() as u64,
            0,
            VMO_READ_MAX_BYTES as u32 + 1,
        ),
        Status::InvalidArgument,
        FAIL_STORAGE_VMO_LENGTH,
    );

    let edge = (USER_GUARD_HIGH - 1) as *mut u8;
    unsafe { ptr::write_volatile(edge, 0xa5) };
    expect_storage_error(
        vmo_read_raw(hello, edge as u64, 0, 2),
        Status::BadAddress,
        FAIL_STORAGE_VMO_BAD_ADDRESS,
    );
    if unsafe { ptr::read_volatile(edge) } != 0xa5 {
        fail(FAIL_STORAGE_VMO_BAD_ADDRESS);
    }

    let mut build_output = [0xa5_u8; 64];
    let build_read = vmo_read_raw(
        build_duplicate,
        build_output.as_mut_ptr() as u64,
        0,
        SYSTEM_BUILD_CONTENT.len() as u32,
    );
    if build_read.status != Status::Ok.raw()
        || build_read.out1 != SYSTEM_BUILD_CONTENT.len() as u64
        || build_read.out2 != SYSTEM_BUILD_CONTENT.len() as u64
        || &build_output[..SYSTEM_BUILD_CONTENT.len()] != SYSTEM_BUILD_CONTENT
        || build_output[SYSTEM_BUILD_CONTENT.len()..]
            .iter()
            .any(|byte| *byte != 0xa5)
        || fnv1a64(&build_output[..SYSTEM_BUILD_CONTENT.len()]) != SYSTEM_BUILD_DIGEST
    {
        fail(FAIL_STORAGE_BUILD_READ);
    }

    close_storage_handle(transfer_only, FAIL_STORAGE_CLEANUP);
    let channel = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    if channel.status != Status::Ok.raw()
        || channel.out1 == 0
        || channel.out2 == 0
        || channel.out1 == channel.out2
    {
        fail(FAIL_STORAGE_CHANNEL);
    }
    let moved = transfer_write(channel.out1, u64::MAX, hello, 0);
    if moved.status != Status::Ok.raw() || moved.out1 != 0 || moved.out2 != 0 {
        fail(FAIL_STORAGE_TRANSFER);
    }
    expect_storage_error(
        syscall(SyscallNumber::HandleClose, hello, 0, 0),
        Status::NotFound,
        FAIL_STORAGE_TRANSFER_STALE,
    );

    let received = syscall(
        SyscallNumber::ChannelReadTransfer,
        channel.out2,
        u64::MAX,
        0,
    );
    if received.status != Status::Ok.raw()
        || received.out1 != 0
        || received.out2 == 0
        || received.out2 == hello
    {
        fail(FAIL_STORAGE_RECEIVE);
    }
    let received = received.out2;
    let mut received_output = [0xa5_u8; 64];
    let received_read = vmo_read_raw(
        received,
        received_output.as_mut_ptr() as u64,
        0,
        SYSTEM_HELLO_CONTENT.len() as u32,
    );
    if received_read.status != Status::Ok.raw()
        || received_read.out1 != SYSTEM_HELLO_CONTENT.len() as u64
        || received_read.out2 != SYSTEM_HELLO_CONTENT.len() as u64
        || &received_output[..SYSTEM_HELLO_CONTENT.len()] != SYSTEM_HELLO_CONTENT
        || received_output[SYSTEM_HELLO_CONTENT.len()..]
            .iter()
            .any(|byte| *byte != 0xa5)
        || fnv1a64(&received_output[..SYSTEM_HELLO_CONTENT.len()]) != SYSTEM_HELLO_DIGEST
    {
        fail(FAIL_STORAGE_RECEIVED_READ);
    }

    for handle in [
        received,
        channel.out1,
        channel.out2,
        build_duplicate,
        system_root,
    ] {
        close_storage_handle(handle, FAIL_STORAGE_CLEANUP);
    }
}

fn file_open_at(root: u64, path: &[u8]) -> SyscallResult {
    file_open_at_raw(root, path.as_ptr() as u64, path.len())
}

fn file_open_at_raw(root: u64, path: u64, length: usize) -> SyscallResult {
    syscall(SyscallNumber::FileOpenAt, root, path, length as u64)
}

fn vmo_read_raw(handle: u64, destination: u64, offset: u32, length: u32) -> SyscallResult {
    syscall(
        SyscallNumber::VmoRead,
        handle,
        destination,
        pack_vmo_read(offset, length),
    )
}

fn expect_storage_error(result: SyscallResult, status: Status, reason: u64) {
    if result.status != status.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(reason);
    }
}

fn close_storage_handle(handle: u64, reason: u64) {
    let result = syscall(SyscallNumber::HandleClose, handle, 0, 0);
    if result.status != Status::Ok.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(reason);
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn drive_post_ready_echo_rounds(children: &ResidentChildren) {
    for round in 1..=POST_READY_ECHO_ROUNDS {
        write_scalar(
            children.client_control.raw(),
            SERVING_ROUND_TAG,
            serving_round_payload(round),
        );
        if read_scalar(children.client_control.raw())
            != (SERVING_ACK_TAG, serving_round_payload(round))
        {
            fail(FAIL_MANAGER_PROTOCOL);
        }
    }

    if read_scalar(children.manager_control.raw())
        != (
            SERVING_DONE_TAG,
            serving_done_payload(Role::ServiceManager, POST_READY_ECHO_ROUNDS),
        )
        || read_scalar(children.provider_control.raw())
            != (
                SERVING_DONE_TAG,
                serving_done_payload(Role::Provider, POST_READY_ECHO_ROUNDS),
            )
    {
        fail(FAIL_MANAGER_DONE);
    }
}

fn drive_dynamic_registration_lifecycle(children: &ResidentChildren) -> InstanceId {
    write_dynamic_command(
        children.provider_control.raw(),
        DYNAMIC_REGISTER_FIRST_COMMAND,
    );
    let first = read_dynamic_commit(
        children.provider_control.raw(),
        1,
        DYNAMIC_REGISTER_FIRST_TXID,
    );
    if first.raw() <= SECOND_EPOCH_INSTANCE_RAW {
        fail(FAIL_DYNAMIC_INSTANCE);
    }

    write_dynamic_command(children.client_control.raw(), DYNAMIC_ECHO_FIRST_COMMAND);
    expect_dynamic_commit(
        children.client_control.raw(),
        2,
        DYNAMIC_LOOKUP_FIRST_TXID,
        first.raw(),
    );
    // A direct echo proves provider -> client delivery, but waking the client
    // does not preempt the provider immediately. This explicit confirmation
    // makes the provider's phase-3 commit causally follow the client's phase-2
    // commit rather than relying only on init's later read order.
    write_dynamic_command(
        children.provider_control.raw(),
        DYNAMIC_CONFIRM_FIRST_ECHO_COMMAND,
    );
    expect_dynamic_commit(
        children.provider_control.raw(),
        3,
        DYNAMIC_LOOKUP_FIRST_TXID,
        first.raw(),
    );

    write_dynamic_command(
        children.provider_control.raw(),
        DYNAMIC_UNREGISTER_FIRST_COMMAND,
    );
    expect_dynamic_commit(
        children.provider_control.raw(),
        4,
        DYNAMIC_UNREGISTER_FIRST_TXID,
        first.raw(),
    );

    write_dynamic_command(
        children.client_control.raw(),
        DYNAMIC_LOOKUP_MISSING_COMMAND,
    );
    expect_dynamic_commit(
        children.client_control.raw(),
        5,
        DYNAMIC_LOOKUP_MISSING_TXID,
        0,
    );

    write_dynamic_command(
        children.provider_control.raw(),
        DYNAMIC_REGISTER_REPLACEMENT_COMMAND,
    );
    let replacement = read_dynamic_commit(
        children.provider_control.raw(),
        6,
        DYNAMIC_REGISTER_REPLACEMENT_TXID,
    );
    if replacement.raw() <= first.raw() {
        fail(FAIL_DYNAMIC_INSTANCE);
    }
    expect_dynamic_commit(
        children.provider_control.raw(),
        7,
        DYNAMIC_UNREGISTER_STALE_TXID,
        first.raw(),
    );

    write_dynamic_command(
        children.client_control.raw(),
        DYNAMIC_ECHO_REPLACEMENT_COMMAND,
    );
    expect_dynamic_commit(
        children.client_control.raw(),
        8,
        DYNAMIC_LOOKUP_REPLACEMENT_TXID,
        replacement.raw(),
    );
    write_dynamic_command(
        children.provider_control.raw(),
        DYNAMIC_CONFIRM_REPLACEMENT_ECHO_COMMAND,
    );
    expect_dynamic_commit(
        children.provider_control.raw(),
        9,
        DYNAMIC_LOOKUP_REPLACEMENT_TXID,
        replacement.raw(),
    );

    write_dynamic_command(children.provider_control.raw(), DYNAMIC_CLEANUP_COMMAND);
    expect_dynamic_commit(
        children.provider_control.raw(),
        10,
        DYNAMIC_UNREGISTER_REPLACEMENT_TXID,
        replacement.raw(),
    );

    for (control, role) in [
        (children.manager_control.raw(), Role::ServiceManager),
        (children.provider_control.raw(), Role::Provider),
        (children.client_control.raw(), Role::Client),
    ] {
        if read_scalar(control) != (DYNAMIC_DONE_TAG, dynamic_done_payload(role)) {
            fail(FAIL_DYNAMIC_COMMIT);
        }
    }
    replacement
}

fn drive_post_cleanup_reuse_round(children: &ResidentChildren, m15_replacement: InstanceId) {
    write_reuse_command(children.provider_control.raw(), REUSE_REGISTER_COMMAND);
    let instance = read_reuse_commit(children.provider_control.raw(), 1, REUSE_REGISTER_TXID);
    if instance.raw() <= m15_replacement.raw() {
        fail(FAIL_REUSE_INSTANCE);
    }

    write_reuse_command(children.client_control.raw(), REUSE_ECHO_COMMAND);
    expect_reuse_commit(
        children.client_control.raw(),
        2,
        REUSE_LOOKUP_TXID,
        instance.raw(),
    );
    // As in M15, the provider may remain runnable after replying to the direct
    // echo. Make its step-3 commit explicitly depend on init observing the
    // client's step-2 commit instead of relying on scheduler ordering.
    write_reuse_command(children.provider_control.raw(), REUSE_CONFIRM_ECHO_COMMAND);
    expect_reuse_commit(
        children.provider_control.raw(),
        3,
        REUSE_LOOKUP_TXID,
        instance.raw(),
    );

    write_reuse_command(children.provider_control.raw(), REUSE_UNREGISTER_COMMAND);
    expect_reuse_commit(
        children.provider_control.raw(),
        4,
        REUSE_UNREGISTER_TXID,
        instance.raw(),
    );

    write_reuse_command(children.client_control.raw(), REUSE_LOOKUP_MISSING_COMMAND);
    expect_reuse_commit(
        children.client_control.raw(),
        5,
        REUSE_LOOKUP_MISSING_TXID,
        0,
    );

    for (control, role) in [
        (children.manager_control.raw(), Role::ServiceManager),
        (children.provider_control.raw(), Role::Provider),
        (children.client_control.raw(), Role::Client),
    ] {
        if read_scalar(control) != (reuse_done_tag(), role.raw() as u64) {
            fail(FAIL_REUSE_COMMIT);
        }
    }
}

fn drive_delegated_endpoint_acl_round(children: &ResidentChildren) {
    if children.manager_pid == 0
        || children.provider_pid == 0
        || children.client_pid == 0
        || children.manager_pid == children.provider_pid
        || children.manager_pid == children.client_pid
        || children.provider_pid == children.client_pid
    {
        fail(FAIL_IDENTITY_SENDER);
    }

    // Ask the provider to duplicate its already-authorized session endpoint.
    // Init relays that capability to the client without changing the endpoint
    // object, which proves that endpoint possession is not treated as writer
    // identity by the ServiceManager.
    write_scalar(
        children.provider_control.raw(),
        IDENTITY_COMMAND_TAG,
        IDENTITY_PROVIDER_RELAY_COMMAND,
    );
    let (relay_sender, delegated_provider_session) =
        read_empty_transfer_envelope(children.provider_control.raw());
    if relay_sender != children.provider_pid {
        close_owned(delegated_provider_session);
        fail(FAIL_IDENTITY_SENDER);
    }
    let (provider_commit_sender, provider_commit) =
        read_scalar_envelope(children.provider_control.raw());
    if provider_commit_sender != children.provider_pid
        || provider_commit
            != (
                IDENTITY_PROVIDER_COMMIT_TAG,
                IDENTITY_PROVIDER_RELAY_COMMAND,
            )
    {
        close_owned(delegated_provider_session);
        fail(FAIL_IDENTITY_PROTOCOL);
    }

    // The manager PID is carried only as an expected reply identity for the
    // client; authorization still comes exclusively from the kernel-stamped
    // sender PID in the manager's atomic receive envelope.
    write_scalar(
        children.client_control.raw(),
        IDENTITY_CLIENT_ATTACK_TAG,
        children.manager_pid,
    );
    write_empty_transfer(children.client_control.raw(), delegated_provider_session);

    let (client_commit_sender, client_commit) = read_scalar_envelope(children.client_control.raw());
    if client_commit_sender != children.client_pid
        || client_commit != (IDENTITY_CLIENT_COMMIT_TAG, u64::from(IDENTITY_ATTACK_TXID))
    {
        fail(FAIL_SERVICE_ACL);
    }

    let (provider_identity_sender, provider_identity_commit) =
        read_scalar_envelope(children.manager_control.raw());
    if provider_identity_sender != children.manager_pid
        || provider_identity_commit != (PROVIDER_IDENTITY_DONE_TAG, children.provider_pid)
    {
        fail(FAIL_IDENTITY_PROTOCOL);
    }
    let (client_identity_sender, client_identity_commit) =
        read_scalar_envelope(children.manager_control.raw());
    if client_identity_sender != children.manager_pid
        || client_identity_commit != (CLIENT_IDENTITY_DONE_TAG, children.client_pid)
    {
        fail(FAIL_IDENTITY_PROTOCOL);
    }
    let (acl_sender, acl_commit) = read_scalar_envelope(children.manager_control.raw());
    if acl_sender != children.manager_pid
        || acl_commit
            != (
                SERVICE_ACL_DONE_TAG,
                service_acl_done_payload(IDENTITY_ATTACK_TXID, MALFORMED_REQUEST_COUNT, 1),
            )
    {
        fail(FAIL_SERVICE_ACL);
    }
}

fn drive_multi_client_lookup_round(children: &mut ResidentChildren) {
    // M20 keeps a second instance of the Client image resident. Its manager
    // transport is a distinct channel, so revoking it cannot disturb the
    // primary client's long-lived session.
    let mut secondary = prepare_child_round();
    spawn_child(&mut secondary, UserImageId::Client);
    let secondary_pid = secondary.pid;
    if secondary_pid == 0
        || secondary_pid == children.client_pid
        || secondary_pid == children.manager_pid
        || secondary_pid == children.provider_pid
    {
        fail(FAIL_MULTI_CLIENT_IDENTITY);
    }
    boot_and_complete_generic(&secondary, Role::Client, WAIT_MANY_SIGNAL_TIMEOUT_NS, false);
    write_scalar(
        secondary.startup_parent,
        CLIENT_MODE_TAG,
        CLIENT_MODE_SECONDARY,
    );

    let secondary_attach = build_attach(SECOND_MANAGER_EPOCH, Role::Client, secondary_pid);
    let (manager_secondary, client_secondary) = create_channel_owned();
    write_attach_transfer(
        children.manager_control.raw(),
        secondary_attach,
        manager_secondary,
    );
    write_attach_transfer(secondary.startup_parent, secondary_attach, client_secondary);
    transfer_abandoned(&secondary);
    expect_m20_event(
        children.manager_control.raw(),
        children.manager_pid,
        M20_MANAGER_EVENT_TAG,
        M20_MANAGER_EVENT_ATTACHED,
        M20_SECONDARY_FIRST_LEASE,
        0,
    );
    expect_m20_event(
        secondary.startup_parent,
        secondary_pid,
        M20_CLIENT_EVENT_TAG,
        M20_CLIENT_EVENT_ATTACHED,
        M20_SECONDARY_FIRST_LEASE,
        0,
    );
    let secondary_control = retain_resident_control(secondary);

    write_scalar(
        children.client_control.raw(),
        M20_CLIENT_MANAGER_PID_TAG,
        children.manager_pid,
    );
    write_scalar(
        secondary_control.raw(),
        M20_CLIENT_MANAGER_PID_TAG,
        children.manager_pid,
    );

    // The provider receives complete kernel PIDs out-of-band before arming.
    // It later authenticates every service-endpoint scalar envelope against
    // the transaction's expected client, including after reattachment.
    write_scalar(
        children.provider_control.raw(),
        M20_PROVIDER_PRIMARY_PID_TAG,
        children.client_pid,
    );
    write_scalar(
        children.provider_control.raw(),
        M20_PROVIDER_SECONDARY_PID_TAG,
        secondary_pid,
    );
    write_m20_control(
        children.provider_control.raw(),
        M20_PROVIDER_CONTROL_TAG,
        M20_PROVIDER_CONTROL_ARM,
        0,
        0,
    );
    expect_m20_event(
        children.provider_control.raw(),
        children.provider_pid,
        M20_PROVIDER_EVENT_TAG,
        M20_PROVIDER_EVENT_ARMED,
        0,
        0,
    );

    // Secondary obtains a valid service endpoint but deliberately leaves it
    // silent. Provider must retain that endpoint while still accepting and
    // completing an independent primary connection.
    write_m20_control(
        secondary_control.raw(),
        M20_CLIENT_CONTROL_TAG,
        M20_CLIENT_CONTROL_STALL,
        M20_SECONDARY_FIRST_LEASE,
        M20_SECONDARY_STALLED_TXID,
    );
    expect_m20_event(
        children.provider_control.raw(),
        children.provider_pid,
        M20_PROVIDER_EVENT_TAG,
        M20_PROVIDER_EVENT_ACCEPTED,
        M20_SECONDARY_FIRST_LEASE,
        M20_SECONDARY_STALLED_TXID,
    );
    expect_m20_event(
        secondary_control.raw(),
        secondary_pid,
        M20_CLIENT_EVENT_TAG,
        M20_CLIENT_EVENT_STALLED,
        M20_SECONDARY_FIRST_LEASE,
        M20_SECONDARY_STALLED_TXID,
    );
    drive_m20_echo(
        children.client_control.raw(),
        children.client_pid,
        children.provider_control.raw(),
        children.provider_pid,
        M20_PRIMARY_LEASE,
        M20_PRIMARY_WHILE_STALLED_TXID,
    );

    // Revocation closes only the secondary manager session. The secondary
    // observes PEER_CLOSED, drops its stalled service endpoint, and the
    // provider removes the abandoned pending operation without blocking.
    write_m20_control(
        children.manager_control.raw(),
        M20_MANAGER_CONTROL_TAG,
        M20_MANAGER_CONTROL_REVOKE,
        M20_SECONDARY_FIRST_LEASE,
        0,
    );
    expect_m20_event(
        children.manager_control.raw(),
        children.manager_pid,
        M20_MANAGER_EVENT_TAG,
        M20_MANAGER_EVENT_REVOKED,
        M20_SECONDARY_FIRST_LEASE,
        0,
    );
    expect_m20_event(
        secondary_control.raw(),
        secondary_pid,
        M20_CLIENT_EVENT_TAG,
        M20_CLIENT_EVENT_REVOKED,
        M20_SECONDARY_FIRST_LEASE,
        0,
    );
    expect_m20_event(
        children.provider_control.raw(),
        children.provider_pid,
        M20_PROVIDER_EVENT_TAG,
        M20_PROVIDER_EVENT_ABORTED,
        M20_SECONDARY_FIRST_LEASE,
        M20_SECONDARY_STALLED_TXID,
    );
    drive_m20_echo(
        children.client_control.raw(),
        children.client_pid,
        children.provider_control.raw(),
        children.provider_pid,
        M20_PRIMARY_LEASE,
        M20_PRIMARY_DETACHED_TXID,
    );

    // Reattach the same generation-qualified secondary PID with a fresh
    // channel pair and a new lease. An old-lease revoke is rejected by both
    // endpoints and must leave the replacement session usable.
    let (reattached_manager, reattached_client) = create_channel_owned();
    write_attach_transfer(
        children.manager_control.raw(),
        secondary_attach,
        reattached_manager,
    );
    write_attach_transfer(secondary_control.raw(), secondary_attach, reattached_client);
    expect_m20_event(
        children.manager_control.raw(),
        children.manager_pid,
        M20_MANAGER_EVENT_TAG,
        M20_MANAGER_EVENT_ATTACHED,
        M20_SECONDARY_REATTACHED_LEASE,
        0,
    );
    expect_m20_event(
        secondary_control.raw(),
        secondary_pid,
        M20_CLIENT_EVENT_TAG,
        M20_CLIENT_EVENT_ATTACHED,
        M20_SECONDARY_REATTACHED_LEASE,
        0,
    );
    write_m20_control(
        children.manager_control.raw(),
        M20_MANAGER_CONTROL_TAG,
        M20_MANAGER_CONTROL_REVOKE,
        M20_SECONDARY_FIRST_LEASE,
        0,
    );
    write_m20_control(
        secondary_control.raw(),
        M20_CLIENT_CONTROL_TAG,
        M20_CLIENT_CONTROL_STALE_REVOKE,
        M20_SECONDARY_FIRST_LEASE,
        0,
    );
    expect_m20_event(
        children.manager_control.raw(),
        children.manager_pid,
        M20_MANAGER_EVENT_TAG,
        M20_MANAGER_EVENT_STALE_REJECTED,
        M20_SECONDARY_FIRST_LEASE,
        0,
    );
    expect_m20_event(
        secondary_control.raw(),
        secondary_pid,
        M20_CLIENT_EVENT_TAG,
        M20_CLIENT_EVENT_STALE_REJECTED,
        M20_SECONDARY_FIRST_LEASE,
        0,
    );
    drive_m20_echo(
        secondary_control.raw(),
        secondary_pid,
        children.provider_control.raw(),
        children.provider_pid,
        M20_SECONDARY_REATTACHED_LEASE,
        M20_SECONDARY_FINAL_TXID,
    );

    write_m20_control(
        children.manager_control.raw(),
        M20_MANAGER_CONTROL_TAG,
        M20_MANAGER_CONTROL_FINALIZE,
        M20_SECONDARY_REATTACHED_LEASE,
        0,
    );
    write_m20_control(
        children.provider_control.raw(),
        M20_PROVIDER_CONTROL_TAG,
        M20_PROVIDER_CONTROL_FINALIZE,
        M20_SECONDARY_REATTACHED_LEASE,
        0,
    );
    expect_m20_event(
        children.manager_control.raw(),
        children.manager_pid,
        M20_MANAGER_EVENT_TAG,
        M20_MANAGER_EVENT_IDLE,
        M20_SECONDARY_REATTACHED_LEASE,
        0,
    );
    expect_m20_event(
        children.provider_control.raw(),
        children.provider_pid,
        M20_PROVIDER_EVENT_TAG,
        M20_PROVIDER_EVENT_IDLE,
        M20_SECONDARY_REATTACHED_LEASE,
        0,
    );

    children.secondary_pid = secondary_pid;
    children.secondary_control = Some(secondary_control);
}

fn spawn_ui_runtime(children: &mut ResidentChildren) {
    let (surface_handle, launcher_handle) = create_channel_owned();
    #[cfg(feature = "androidbox-process0")]
    let (app_bootstrap_parent, app_bootstrap_child) = create_channel_owned();
    // Channel transfers are acyclic: the transport must have an older object
    // ID than every Channel it carries. Create the bootstrap transport before
    // the two endpoints that it transfers to App.
    let (app_server_handle, app_surface_handle) = create_channel_owned();
    #[cfg(feature = "androidbox-process0")]
    let (app_runtime_handle, android_app_handle) = create_channel_owned();
    #[cfg(feature = "androidbox-process0")]
    {
        write_android_app_bootstrap_transfer(
            app_bootstrap_parent.raw(),
            AndroidAppBootstrapKind::SurfaceClient,
            app_surface_handle,
        );
        write_android_app_bootstrap_transfer(
            app_bootstrap_parent.raw(),
            AndroidAppBootstrapKind::RuntimeClient,
            app_runtime_handle,
        );
    }
    let attach = UiClientControl::attach_app_endpoint(UiClientId::App)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    write_ui_control_transfer(launcher_handle.raw(), attach, app_server_handle);

    let raw_surface_handle = surface_handle.raw();
    let surface = syscall(
        SyscallNumber::ProcessSpawn,
        raw_surface_handle,
        UserImageId::SurfaceServer.raw(),
        PROCESS_SPAWN_FLAGS_NONE,
    );
    if surface.status != Status::Ok.raw() || surface.out1 == 0 || surface.out2 != 0 {
        fail(FAIL_SURFACE_ACQUIRE);
    }
    assert_stale_handle(raw_surface_handle);

    let raw_launcher_handle = launcher_handle.raw();
    let launcher = syscall(
        SyscallNumber::ProcessSpawn,
        raw_launcher_handle,
        UserImageId::Launcher.raw(),
        PROCESS_SPAWN_FLAGS_NONE,
    );
    if launcher.status != Status::Ok.raw() || launcher.out1 == 0 || launcher.out2 != 0 {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    assert_stale_handle(raw_launcher_handle);

    #[cfg(not(feature = "androidbox-process0"))]
    let raw_app_handle = app_surface_handle.raw();
    #[cfg(feature = "androidbox-process0")]
    let raw_app_handle = app_bootstrap_child.raw();
    let app = syscall(
        SyscallNumber::ProcessSpawn,
        raw_app_handle,
        UserImageId::App.raw(),
        PROCESS_SPAWN_FLAGS_NONE,
    );
    if app.status != Status::Ok.raw() || app.out1 == 0 || app.out2 != 0 {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    assert_stale_handle(raw_app_handle);

    #[cfg(feature = "androidbox-process0")]
    let android_app = {
        let raw_android_app_handle = android_app_handle.raw();
        let spawned = syscall(
            SyscallNumber::ProcessSpawn,
            raw_android_app_handle,
            UserImageId::AndroidApp.raw(),
            PROCESS_SPAWN_FLAGS_NONE,
        );
        if spawned.status != Status::Ok.raw() || spawned.out1 == 0 || spawned.out2 != 0 {
            let terminated = syscall(
                SyscallNumber::ProcessTerminate,
                app.out1,
                PROCESS_TERMINATE_FLAGS_NONE,
                0,
            );
            if terminated.status == Status::Ok.raw() {
                let _ = syscall(SyscallNumber::ProcessWait, app.out1, 0, 0);
            }
            close_owned(android_app_handle);
            close_owned(app_bootstrap_parent);
            fail(FAIL_SURFACE_PROTOCOL);
        }
        assert_stale_handle(raw_android_app_handle);
        #[cfg(not(feature = "androidbox-restart0"))]
        close_owned(app_bootstrap_parent);
        spawned
    };

    #[cfg(not(feature = "androidbox-process0"))]
    let pids = [
        children.manager_pid,
        children.provider_pid,
        children.client_pid,
        children.secondary_pid,
        surface.out1,
        launcher.out1,
        app.out1,
    ];
    #[cfg(feature = "androidbox-process0")]
    let pids = [
        children.manager_pid,
        children.provider_pid,
        children.client_pid,
        children.secondary_pid,
        surface.out1,
        launcher.out1,
        app.out1,
        android_app.out1,
    ];
    if pids.contains(&0)
        || pids
            .iter()
            .enumerate()
            .any(|(index, pid)| pids[index + 1..].iter().any(|other| other == pid))
    {
        fail(FAIL_MULTI_CLIENT_IDENTITY);
    }
    children.surface_server_pid = surface.out1;
    children.launcher_pid = launcher.out1;
    children.app_pid = app.out1;
    #[cfg(feature = "androidbox-process0")]
    {
        children.android_app_pid = android_app.out1;
    }
    #[cfg(feature = "androidbox-restart0")]
    {
        children.android_app_supervisor = Some(app_bootstrap_parent);
    }
    verify_process_capacity_is_stable();
}

fn drive_m20_echo(
    client_control: u64,
    client_pid: u64,
    provider_control: u64,
    provider_pid: u64,
    lease: u16,
    transaction_id: u32,
) {
    write_m20_control(
        client_control,
        M20_CLIENT_CONTROL_TAG,
        M20_CLIENT_CONTROL_ECHO,
        lease,
        transaction_id,
    );
    expect_m20_event(
        provider_control,
        provider_pid,
        M20_PROVIDER_EVENT_TAG,
        M20_PROVIDER_EVENT_ACCEPTED,
        lease,
        transaction_id,
    );
    expect_m20_event(
        provider_control,
        provider_pid,
        M20_PROVIDER_EVENT_TAG,
        M20_PROVIDER_EVENT_ECHO_DONE,
        lease,
        transaction_id,
    );
    expect_m20_event(
        client_control,
        client_pid,
        M20_CLIENT_EVENT_TAG,
        M20_CLIENT_EVENT_ECHO_DONE,
        lease,
        transaction_id,
    );
}

fn run_supervised_service_manager_rounds() -> ResidentChildren {
    // The first three dynamic slots are deliberately filled in this order.
    // The ServiceManager is the supervised process; provider and client keep
    // their startup transports alive across both manager generations.
    let mut manager = prepare_child_round();
    spawn_child(&mut manager, UserImageId::ServiceManager);
    let first_manager_pid = manager.pid;
    let mut provider = prepare_child_round();
    spawn_child(&mut provider, UserImageId::Provider);
    let mut client = prepare_child_round();
    spawn_child(&mut client, UserImageId::Client);
    if manager.pid == provider.pid
        || manager.pid == client.pid
        || provider.pid == client.pid
        || [manager.pid, provider.pid, client.pid].contains(&0)
    {
        fail(FAIL_SM_LIFECYCLE);
    }
    boot_and_complete_generic(
        &manager,
        Role::ServiceManager,
        WAIT_MANY_SIGNAL_TIMEOUT_NS,
        false,
    );
    boot_and_complete_generic(
        &provider,
        Role::Provider,
        OBJECT_WAIT_TIMEOUT_INFINITE,
        true,
    );
    boot_and_complete_generic(&client, Role::Client, WAIT_MANY_SIGNAL_TIMEOUT_NS, false);
    write_scalar(client.startup_parent, CLIENT_MODE_TAG, CLIENT_MODE_PRIMARY);

    attach_manager_generation(&manager, &provider, &client, FIRST_MANAGER_EPOCH, false);
    transfer_abandoned(&manager);
    expect_manager_instance(&manager, FIRST_EPOCH_INSTANCE_RAW);
    wait_for_child_exit(manager.pid, MANAGER_RESTART_EXIT, FAIL_MANAGER_RESTART);
    if syscall(SyscallNumber::ProcessWait, manager.pid, 0, 0).status != Status::NotFound.raw() {
        fail(FAIL_MANAGER_RESTART);
    }
    expect_manager_loss(&provider, Role::Provider, FIRST_MANAGER_EPOCH);
    expect_manager_loss(&client, Role::Client, FIRST_MANAGER_EPOCH);
    cleanup_child_round(manager);

    let mut restarted_manager = prepare_child_round();
    spawn_child(&mut restarted_manager, UserImageId::ServiceManager);
    if process_slot(restarted_manager.pid) != process_slot(first_manager_pid)
        || process_generation(restarted_manager.pid)
            != process_generation(first_manager_pid)
                .checked_add(1)
                .unwrap_or(0)
    {
        fail(FAIL_MANAGER_IDENTITY);
    }
    boot_and_complete_generic(
        &restarted_manager,
        Role::ServiceManager,
        WAIT_MANY_SIGNAL_TIMEOUT_NS,
        false,
    );

    // Attachments must be queued before abandoned transfers. Provider and
    // client will read this second attachment from the same startup channel;
    // a zero-byte abandoned message ahead of it would be a protocol error.
    attach_manager_generation(
        &restarted_manager,
        &provider,
        &client,
        SECOND_MANAGER_EPOCH,
        true,
    );
    transfer_abandoned(&provider);
    transfer_abandoned(&client);
    transfer_abandoned(&restarted_manager);
    expect_manager_instance(&restarted_manager, SECOND_EPOCH_INSTANCE_RAW);
    expect_resident_ready(
        &restarted_manager,
        Role::ServiceManager,
        SECOND_MANAGER_EPOCH,
    );
    expect_resident_ready(&provider, Role::Provider, SECOND_MANAGER_EPOCH);
    expect_resident_ready(&client, Role::Client, SECOND_MANAGER_EPOCH);

    ResidentChildren {
        manager_pid: restarted_manager.pid,
        provider_pid: provider.pid,
        client_pid: client.pid,
        secondary_pid: 0,
        surface_server_pid: 0,
        launcher_pid: 0,
        app_pid: 0,
        #[cfg(feature = "androidbox-process0")]
        android_app_pid: 0,
        #[cfg(feature = "androidbox-restart0")]
        android_app_supervisor: None,
        #[cfg(feature = "input-server-runtime")]
        input_server_pid: 0,
        #[cfg(any(
            feature = "input-server-surface-restart-runtime",
            feature = "input-server-restart-runtime",
            feature = "service-dependency-runtime"
        ))]
        input_server_session: 0,
        #[cfg(any(
            feature = "input-server-surface-restart-runtime",
            feature = "input-server-restart-runtime",
            feature = "service-dependency-runtime"
        ))]
        input_route_epoch: 0,
        manager_control: retain_resident_control(restarted_manager),
        provider_control: retain_resident_control(provider),
        client_control: retain_resident_control(client),
        secondary_control: None,
        #[cfg(feature = "input-server-runtime")]
        input_server_control: None,
    }
}

fn attach_manager_generation(
    manager: &ChildRound,
    provider: &ChildRound,
    client: &ChildRound,
    epoch: u16,
    restarted: bool,
) {
    let (manager_provider, provider_session) = create_channel_owned();
    let (manager_client, client_session) = create_channel_owned();
    let provider_attach = build_attach(epoch, Role::Provider, provider.pid);
    let client_attach = build_attach(epoch, Role::Client, client.pid);

    write_attach_transfer(provider.startup_parent, provider_attach, provider_session);
    write_attach_transfer(client.startup_parent, client_attach, client_session);
    write_attach_transfer(manager.startup_parent, provider_attach, manager_provider);
    write_attach_transfer(manager.startup_parent, client_attach, manager_client);

    if restarted != (epoch == SECOND_MANAGER_EPOCH) {
        fail(FAIL_ATTACH);
    }
}

fn expect_manager_instance(manager: &ChildRound, expected: u32) {
    if read_scalar(manager.startup_parent) != (MANAGER_INSTANCE_TAG, u64::from(expected)) {
        fail(FAIL_MANAGER_REPORT);
    }
}

fn expect_manager_loss(child: &ChildRound, role: Role, epoch: u16) {
    if read_scalar(child.startup_parent) != (MANAGER_LOST_TAG, role_epoch_payload(role, epoch)) {
        fail(FAIL_SESSION_OBSERVATION);
    }
}

fn expect_resident_ready(child: &ChildRound, role: Role, epoch: u16) {
    if read_scalar(child.startup_parent) != (RESIDENT_READY_TAG, role_epoch_payload(role, epoch)) {
        fail(FAIL_ROLE_READY);
    }
}

fn wait_for_child_exit(pid: u64, expected_exit: u64, reason: u64) {
    let result = syscall(SyscallNumber::ProcessWait, pid, 0, 0);
    if result.status != Status::Ok.raw()
        || result.out1 != expected_exit
        || result.out2 != ProcessTerminationReason::Exited.raw()
    {
        fail(reason);
    }
}

const fn process_slot(pid: u64) -> u32 {
    pid as u32
}

const fn process_generation(pid: u64) -> u32 {
    (pid >> 32) as u32
}

fn prepare_child_round() -> ChildRound {
    let startup = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    let service = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    let abandoned = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    if startup.status != Status::Ok.raw()
        || service.status != Status::Ok.raw()
        || abandoned.status != Status::Ok.raw()
    {
        fail(FAIL_CONCURRENT_PREPARE);
    }
    let event = syscall(SyscallNumber::EventCreate, 0, 0, 0);
    if event.status != Status::Ok.raw() || event.out1 == 0 {
        fail(FAIL_CONCURRENT_PREPARE);
    }
    let event_child = syscall(
        SyscallNumber::HandleDuplicate,
        event.out1,
        u64::from(Rights::EVENT_DEFAULT.bits()),
        0,
    );
    if event_child.status != Status::Ok.raw() || event_child.out1 == 0 {
        fail(FAIL_CONCURRENT_PREPARE);
    }

    if object_wait(event.out1, ObjectSignals::READABLE).status != Status::InvalidArgument.raw()
        || object_wait(startup.out1, ObjectSignals::SIGNALED).status
            != Status::InvalidArgument.raw()
    {
        fail(91);
    }
    let wait_only = syscall(
        SyscallNumber::HandleDuplicate,
        event.out1,
        u64::from(Rights::WAIT.bits()),
        0,
    );
    if wait_only.status != Status::Ok.raw()
        || syscall(SyscallNumber::EventSignal, wait_only.out1, 0, 0).status
            != Status::PermissionDenied.raw()
        || syscall(SyscallNumber::HandleClose, wait_only.out1, 0, 0).status != Status::Ok.raw()
    {
        fail(84);
    }
    if syscall(
        SyscallNumber::ChannelWrite,
        service.out1,
        EXPECTED_TAG,
        EXPECTED_PAYLOAD,
    )
    .status
        != Status::Ok.raw()
    {
        fail(51);
    }

    if transfer_write(startup.out1, u64::MAX, startup.out2, 0).status
        != Status::InvalidArgument.raw()
        || transfer_write(service.out1, u64::MAX, startup.out2, 0).status
            != Status::InvalidArgument.raw()
    {
        fail(52);
    }
    let reduced = syscall(
        SyscallNumber::HandleDuplicate,
        service.out2,
        u64::from(Rights::READ.bits() | Rights::WRITE.bits()),
        0,
    );
    if reduced.status != Status::Ok.raw()
        || transfer_write(startup.out1, u64::MAX, reduced.out1, 0).status
            != Status::PermissionDenied.raw()
        || object_wait(reduced.out1, ObjectSignals::READABLE).status
            != Status::PermissionDenied.raw()
        || syscall(SyscallNumber::HandleClose, reduced.out1, 0, 0).status != Status::Ok.raw()
    {
        fail(53);
    }
    if transfer_write(startup.out1, USER_GUARD_LOW, service.out2, 1).status
        != Status::BadAddress.raw()
        || transfer_write(startup.out1, USER_GUARD_LOW, event_child.out1, 1).status
            != Status::BadAddress.raw()
    {
        fail(54);
    }

    let polled = object_wait_many(
        event.out1,
        ObjectSignals::SIGNALED,
        startup.out1,
        ObjectSignals::READABLE,
        OBJECT_WAIT_TIMEOUT_POLL,
    );
    if polled.status != Status::Timeout.raw() || polled.out1 != u64::MAX || polled.out2 != 0 {
        fail(FAIL_WAIT_MANY_POLL);
    }
    let timed = object_wait_many(
        event.out1,
        ObjectSignals::SIGNALED,
        startup.out1,
        ObjectSignals::READABLE,
        WAIT_MANY_FINITE_TIMEOUT_NS,
    );
    if timed.status != Status::Timeout.raw() || timed.out1 != u64::MAX || timed.out2 != 0 {
        fail(FAIL_WAIT_MANY_TIMEOUT);
    }

    ChildRound {
        startup_parent: startup.out1,
        startup_child: startup.out2,
        service_parent: service.out1,
        service_child: service.out2,
        abandoned_parent: abandoned.out1,
        abandoned_child: abandoned.out2,
        event_parent: event.out1,
        event_child: event_child.out1,
        pid: 0,
    }
}

fn spawn_child(child: &mut ChildRound, image_id: UserImageId) {
    if !image_id.is_spawnable() {
        fail(58);
    }
    let spawned = syscall(
        SyscallNumber::ProcessSpawn,
        child.startup_child,
        image_id.raw(),
        PROCESS_SPAWN_FLAGS_NONE,
    );
    if spawned.status != Status::Ok.raw() || spawned.out1 == 0 || spawned.out2 != 0 {
        fail(58);
    }
    child.pid = spawned.out1;
    assert_stale_handle(child.startup_child);
}

fn verify_process_capacity_is_stable() {
    let test = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    if test.status != Status::Ok.raw() || test.out1 == 0 || test.out2 == 0 {
        fail(FAIL_CONCURRENT_CAPACITY);
    }
    let rejected = [
        syscall(
            SyscallNumber::ProcessSpawn,
            test.out2,
            0,
            PROCESS_SPAWN_FLAGS_NONE,
        ),
        syscall(
            SyscallNumber::ProcessSpawn,
            test.out2,
            UserImageId::Init.raw(),
            PROCESS_SPAWN_FLAGS_NONE,
        ),
        syscall(
            SyscallNumber::ProcessSpawn,
            test.out2,
            UserImageId::ServiceManager.raw(),
            1,
        ),
    ];
    let expected = [
        Status::InvalidArgument.raw(),
        Status::PermissionDenied.raw(),
        Status::InvalidArgument.raw(),
    ];
    for (rejected, expected_status) in rejected.into_iter().zip(expected) {
        if rejected.status != expected_status || rejected.out1 != 0 || rejected.out2 != 0 {
            fail(FAIL_CONCURRENT_CAPACITY);
        }
    }
    #[cfg(not(feature = "unified-product-runtime"))]
    {
        let full = syscall(
            SyscallNumber::ProcessSpawn,
            test.out2,
            UserImageId::ServiceManager.raw(),
            PROCESS_SPAWN_FLAGS_NONE,
        );
        if full.status != Status::ShouldWait.raw() || full.out1 != 0 || full.out2 != 0 {
            fail(FAIL_CONCURRENT_CAPACITY);
        }
    }
    // M67 deliberately reserves the ninth dynamic slot for the StorageServer
    // that is spawned only after authenticated power input. Its kernel-owned
    // UI convergence proof checks both the 10/9 capacity and the still-empty
    // final spawn-evidence slot, so probing with a valid image here would
    // consume the reservation instead of testing a full table.
    write_scalar(test.out1, EXPECTED_TAG, EXPECTED_PAYLOAD);
    if read_scalar(test.out2) != (EXPECTED_TAG, EXPECTED_PAYLOAD) {
        fail(FAIL_CAPACITY_SOURCE);
    }
    if syscall(SyscallNumber::HandleClose, test.out2, 0, 0).status != Status::Ok.raw()
        || syscall(SyscallNumber::HandleClose, test.out1, 0, 0).status != Status::Ok.raw()
    {
        fail(FAIL_CONCURRENT_CAPACITY);
    }
}

fn boot_and_complete_generic(
    child: &ChildRound,
    role: Role,
    startup_timeout: u64,
    validate_all: bool,
) {
    write_frame_bytes(child.startup_parent, build_frame(Frame::boot(role)));
    let moved_service = transfer_write(
        child.startup_parent,
        BYTE_MESSAGE.as_ptr() as u64,
        child.service_child,
        BYTE_MESSAGE.len(),
    );
    let moved_event = transfer_write(
        child.startup_parent,
        BYTE_MESSAGE.as_ptr() as u64,
        child.event_child,
        1,
    );
    if moved_service.status != Status::Ok.raw()
        || moved_service.out1 != BYTE_MESSAGE.len() as u64
        || moved_event.status != Status::Ok.raw()
        || moved_event.out1 != 1
    {
        fail(55);
    }
    assert_stale_handle(child.service_child);
    assert_stale_handle(child.event_child);

    let event_ready = object_wait(child.event_parent, ObjectSignals::SIGNALED);
    if event_ready.status != Status::Ok.raw()
        || event_ready.out1 & u64::from(ObjectSignals::SIGNALED.bits()) == 0
    {
        fail(86);
    }
    if validate_all
        && object_wait_many(
            child.event_parent,
            ObjectSignals::SIGNALED,
            child.startup_parent,
            ObjectSignals::SIGNALED,
            OBJECT_WAIT_TIMEOUT_POLL,
        )
        .status
            != Status::InvalidArgument.raw()
    {
        fail(FAIL_WAIT_MANY_VALIDATE_ALL);
    }
    let lowest = object_wait_many(
        child.event_parent,
        ObjectSignals::SIGNALED,
        child.event_parent,
        ObjectSignals::SIGNALED,
        OBJECT_WAIT_TIMEOUT_POLL,
    );
    if lowest.status != Status::Ok.raw()
        || lowest.out1 != 0
        || lowest.out2 & u64::from(ObjectSignals::SIGNALED.bits()) == 0
    {
        fail(FAIL_WAIT_MANY_PRIORITY);
    }
    if !event_change(SyscallNumber::EventClear, child.event_parent, true)
        || !event_change(SyscallNumber::EventSignal, child.event_parent, true)
        || !event_change(SyscallNumber::EventSignal, child.event_parent, false)
    {
        fail(87);
    }
    let local_ready = object_wait(child.event_parent, ObjectSignals::SIGNALED);
    if local_ready.status != Status::Ok.raw()
        || local_ready.out1 & u64::from(ObjectSignals::SIGNALED.bits()) == 0
        || !event_change(SyscallNumber::EventClear, child.event_parent, true)
        || !event_change(SyscallNumber::EventClear, child.event_parent, false)
    {
        fail(88);
    }

    let reply = read_scalar(child.service_parent);
    if reply != (EXPECTED_TAG, EXPECTED_PAYLOAD) {
        fail(62);
    }
    wait_peer_closed(child.service_parent);
    if syscall(SyscallNumber::ChannelRead, child.service_parent, 0, 0).status
        != Status::PeerClosed.raw()
        || syscall(SyscallNumber::HandleClose, child.service_parent, 0, 0).status
            != Status::Ok.raw()
    {
        fail(63);
    }

    write_scalar(
        child.startup_parent,
        GENERIC_CONTINUE_TAG,
        GENERIC_CONTINUE_PAYLOAD,
    );
    if validate_all {
        wait_for_startup_request_array(
            child.event_parent,
            child.startup_parent,
            child.abandoned_parent,
            startup_timeout,
        );
    } else {
        wait_for_startup_request(child.event_parent, child.startup_parent, startup_timeout);
    }
    if read_scalar(child.startup_parent) != (ROLE_READY_TAG, u64::from(role.raw())) {
        fail(FAIL_ROLE_READY);
    }
    if syscall(SyscallNumber::HandleClose, child.event_parent, 0, 0).status != Status::Ok.raw() {
        fail(65);
    }
}

fn parent_register_provider(
    startup: u64,
    provider_pid: u64,
    services: &mut ServiceRegistry,
) -> InstanceId {
    let echo = echo_service_name();
    let (register_sender, register, connector) = read_frame_transfer_envelope(startup);
    if register_sender != provider_pid {
        close_owned(connector);
        fail(FAIL_IDENTITY_SENDER);
    }
    expect_service_frame(
        register,
        Opcode::Register,
        REGISTER_TXID,
        echo,
        ServiceStatus::Ok,
        0,
        true,
    );
    let instance = register_new_service(services, echo, connector, register_sender);
    write_frame_bytes(
        startup,
        build_frame(Frame::register_reply(
            REGISTER_TXID,
            echo,
            ServiceStatus::Ok,
            instance.raw(),
        )),
    );

    let (duplicate_sender, duplicate, connector) = read_frame_transfer_envelope(startup);
    if duplicate_sender != provider_pid {
        close_owned(connector);
        fail(FAIL_IDENTITY_SENDER);
    }
    expect_service_frame(
        duplicate,
        Opcode::Register,
        DUPLICATE_TXID,
        echo,
        ServiceStatus::Ok,
        0,
        true,
    );
    let duplicate_resource = RegisteredService {
        connector,
        owner_pid: duplicate_sender,
    };
    match services.register(echo, duplicate_resource) {
        Err(RegisterError::AlreadyExists {
            instance: current,
            resource,
        }) if current == instance && resource.owner_pid == provider_pid => {
            close_registered_service(resource)
        }
        Err(error) => {
            close_registered_service(error.into_resource());
            fail(FAIL_SM_REGISTRY);
        }
        Ok(_) => fail(FAIL_SM_REGISTRY),
    }
    if services.len() != 1 || registered_connector(services, echo, instance, provider_pid) == 0 {
        fail(FAIL_PROVIDER_OWNER);
    }
    write_frame_bytes(
        startup,
        build_frame(Frame::register_reply(
            DUPLICATE_TXID,
            echo,
            ServiceStatus::AlreadyExists,
            0,
        )),
    );
    instance
}

fn parent_connect_client(
    startup: u64,
    client_pid: u64,
    provider_pid: u64,
    instance: InstanceId,
    services: &ServiceRegistry,
) {
    let echo = echo_service_name();
    let missing = missing_service_name();
    let (unknown_sender, unknown) = read_frame_bytes_envelope(startup);
    if unknown_sender != client_pid {
        fail(FAIL_IDENTITY_SENDER);
    }
    expect_service_frame(
        unknown,
        Opcode::Lookup,
        UNKNOWN_LOOKUP_TXID,
        missing,
        ServiceStatus::Ok,
        0,
        false,
    );
    if services.lookup(&missing).is_some() {
        fail(FAIL_SM_REGISTRY);
    }
    write_frame_bytes(
        startup,
        build_frame(Frame::lookup_reply(
            UNKNOWN_LOOKUP_TXID,
            missing,
            ServiceStatus::NotFound,
            0,
        )),
    );

    let (lookup_sender, lookup) = read_frame_bytes_envelope(startup);
    if lookup_sender != client_pid {
        fail(FAIL_IDENTITY_SENDER);
    }
    expect_service_frame(
        lookup,
        Opcode::Lookup,
        CLIENT_LOOKUP_TXID,
        echo,
        ServiceStatus::Ok,
        0,
        false,
    );
    let connector = registered_connector(services, echo, instance, provider_pid);
    let (server_endpoint, client_endpoint) = create_channel_owned();
    write_frame_transfer(
        connector,
        build_frame(Frame::connect(CLIENT_LOOKUP_TXID, echo, instance)),
        server_endpoint,
    );
    write_frame_transfer(
        startup,
        build_frame(Frame::lookup_reply(
            CLIENT_LOOKUP_TXID,
            echo,
            ServiceStatus::Ok,
            instance.raw(),
        )),
        client_endpoint,
    );
}

fn transfer_abandoned(child: &ChildRound) {
    let moved = transfer_write(child.startup_parent, u64::MAX, child.abandoned_child, 0);
    if moved.status != Status::Ok.raw() || moved.out1 != 0 || moved.out2 != 0 {
        fail(FAIL_ABANDONED_TRANSFER);
    }
    assert_stale_handle(child.abandoned_child);
}

fn cleanup_child_round(child: ChildRound) {
    if child.pid == 0 {
        fail(FAIL_SM_LIFECYCLE);
    }
    let startup_closed = object_wait(child.startup_parent, ObjectSignals::PEER_CLOSED);
    let abandoned_closed = object_wait(child.abandoned_parent, ObjectSignals::PEER_CLOSED);
    if startup_closed.status != Status::Ok.raw()
        || startup_closed.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
        || abandoned_closed.status != Status::Ok.raw()
        || abandoned_closed.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
        || syscall(SyscallNumber::ChannelRead, child.abandoned_parent, 0, 0).status
            != Status::PeerClosed.raw()
    {
        fail(FAIL_ABANDONED_TRANSFER);
    }
    for handle in [child.startup_parent, child.abandoned_parent] {
        if syscall(SyscallNumber::HandleClose, handle, 0, 0).status != Status::Ok.raw() {
            fail(65);
        }
    }
}

fn retain_resident_control(child: ChildRound) -> OwnedUserHandle {
    if child.pid == 0 || child.startup_parent == 0 {
        fail(FAIL_SM_LIFECYCLE);
    }
    let abandoned_closed = object_wait(child.abandoned_parent, ObjectSignals::PEER_CLOSED);
    if abandoned_closed.status != Status::Ok.raw()
        || abandoned_closed.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
        || syscall(SyscallNumber::ChannelRead, child.abandoned_parent, 0, 0).status
            != Status::PeerClosed.raw()
        || syscall(SyscallNumber::HandleClose, child.abandoned_parent, 0, 0).status
            != Status::Ok.raw()
    {
        fail(FAIL_ABANDONED_TRANSFER);
    }
    // startup_parent deliberately remains owned by init. Return its raw
    // capability as explicit resident state instead of leaving an unreachable
    // entry in the process handle table.
    OwnedUserHandle::new(child.startup_parent).unwrap_or_else(|| fail(FAIL_SM_LIFECYCLE))
}

fn supervise_resident_children(children: &mut ResidentChildren) -> ! {
    #[cfg(feature = "androidbox-process0")]
    let android_app_invalid = children.android_app_pid == 0
        || [
            children.manager_pid,
            children.provider_pid,
            children.client_pid,
            children.secondary_pid,
            children.surface_server_pid,
            children.launcher_pid,
            children.app_pid,
        ]
        .contains(&children.android_app_pid);
    #[cfg(not(feature = "androidbox-process0"))]
    let android_app_invalid = false;
    if children.manager_control.raw() == 0
        || children.provider_control.raw() == 0
        || children.client_control.raw() == 0
        || children.secondary_pid == 0
        || children.surface_server_pid == 0
        || children.launcher_pid == 0
        || children.app_pid == 0
        || children.surface_server_pid == children.launcher_pid
        || children.surface_server_pid == children.app_pid
        || children.launcher_pid == children.app_pid
        || android_app_invalid
        || {
            #[cfg(feature = "androidbox-restart0")]
            {
                children
                    .android_app_supervisor
                    .as_ref()
                    .is_none_or(|handle| handle.raw() == 0)
            }
            #[cfg(not(feature = "androidbox-restart0"))]
            {
                false
            }
        }
        || children
            .secondary_control
            .as_ref()
            .is_none_or(|handle| handle.raw() == 0)
    {
        fail(FAIL_FINAL_WAIT);
    }
    #[cfg(feature = "androidbox-restart0")]
    {
        supervise_one_android_app_restart(children)
    }
    #[cfg(not(feature = "androidbox-restart0"))]
    {
        let result = syscall(SyscallNumber::ProcessWait, children.manager_pid, 0, 0);
        let _ = result;
        fail(FAIL_FINAL_WAIT)
    }
}

#[cfg(feature = "androidbox-restart0")]
fn supervise_one_android_app_restart(children: &mut ResidentChildren) -> ! {
    const RESTART_EPOCH: u64 = 1;

    let old_pid = children.android_app_pid;
    let waited = syscall(SyscallNumber::ProcessWait, old_pid, 0, 0);
    if waited.status != Status::Ok.raw()
        || waited.out1 != 0
        || waited.out2 != ProcessTerminationReason::Faulted.raw()
    {
        fail(FAIL_FINAL_WAIT);
    }

    // The retained bootstrap Channel predates this replacement pair, keeping
    // the kernel's acyclic transfer-order invariant intact.
    let (app_runtime, worker_runtime) = create_channel_owned();
    let raw_worker_runtime = worker_runtime.raw();
    let replacement = syscall(
        SyscallNumber::ProcessSpawn,
        raw_worker_runtime,
        UserImageId::AndroidApp.raw(),
        PROCESS_SPAWN_FLAGS_NONE,
    );
    if replacement.status != Status::Ok.raw()
        || replacement.out1 == 0
        || replacement.out2 != 0
        || replacement.out1 == old_pid
        || [
            children.manager_pid,
            children.provider_pid,
            children.client_pid,
            children.secondary_pid,
            children.surface_server_pid,
            children.launcher_pid,
            children.app_pid,
        ]
        .contains(&replacement.out1)
    {
        close_owned(app_runtime);
        close_owned(worker_runtime);
        fail(FAIL_SURFACE_PROTOCOL);
    }
    assert_stale_handle(raw_worker_runtime);

    let rebind = AndroidAppSupervisorMessage::new(
        AndroidAppSupervisorMessageKind::Rebind,
        RESTART_EPOCH,
        old_pid,
        replacement.out1,
        waited.out1,
        waited.out2,
    )
    .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let supervisor = children
        .android_app_supervisor
        .as_ref()
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    write_android_app_supervisor_transfer(supervisor.raw(), rebind, app_runtime);

    let (sender_pid, rebound) = read_android_app_supervisor_rebound(supervisor.raw());
    if sender_pid != children.app_pid
        || rebound.kind() != AndroidAppSupervisorMessageKind::Rebound
        || rebound.epoch() != rebind.epoch()
        || rebound.old_pid() != rebind.old_pid()
        || rebound.new_pid() != rebind.new_pid()
        || rebound.exit_code() != rebind.exit_code()
        || rebound.termination_reason() != rebind.termination_reason()
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    children.android_app_pid = replacement.out1;

    // ABI 48 deliberately has a one-worker restart budget. Once that exact
    // transition is acknowledged, retain the historical manager supervision
    // boundary; a second AndroidApp failure remains fail-stop.
    let result = syscall(SyscallNumber::ProcessWait, children.manager_pid, 0, 0);
    let _ = result;
    fail(FAIL_FINAL_WAIT)
}

// The child is held at the generic-continuation read until the parent has
// restored the Event to unsignaled. After the continuation write, this wait
// publishes the token before the child sends its first role request. Clients
// keep exercising the legacy two-item ABI; Provider uses the complete
// eight-item array ABI with six duplicate dormant Channel entries and an
// infinite deadline. This drives the maximum-sized token through the real
// scheduler block/wake path rather than proving the bound through polling only.
fn wait_for_startup_request(event: u64, startup: u64, timeout_ns: u64) {
    let ready = object_wait_many(
        event,
        ObjectSignals::SIGNALED,
        startup,
        ObjectSignals::READABLE,
        timeout_ns,
    );
    if ready.status != Status::Ok.raw()
        || ready.out1 != 1
        || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
    {
        fail(FAIL_WAIT_MANY_SIGNAL);
    }
}

fn wait_for_startup_request_array(event: u64, startup: u64, abandoned: u64, timeout_ns: u64) {
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    items[0] = pack_user_wait_item(event, ObjectSignals::SIGNALED);
    items[1] = pack_user_wait_item(startup, ObjectSignals::READABLE);
    items[2..].fill(pack_user_wait_item(abandoned, ObjectSignals::READABLE));
    let ready = object_wait_many_array(&items, OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS, timeout_ns);
    if ready.status != Status::Ok.raw()
        || ready.out1 != 1
        || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
    {
        fail(FAIL_WAIT_MANY_ARRAY_BLOCKING_WAKE);
    }
}

fn child_service_manager_round(startup: u64, supervisor_pid: u64) -> (u16, ManagerResidentState) {
    let (provider_sender, provider_attach, provider_session) = read_attach_envelope(startup);
    let (client_sender, client_attach, client_session) = read_attach_envelope(startup);
    if supervisor_pid == 0
        || provider_sender != supervisor_pid
        || client_sender != supervisor_pid
        || provider_attach.role() != Role::Provider
        || client_attach.role() != Role::Client
        || provider_attach.manager_epoch() != client_attach.manager_epoch()
        || provider_attach.owner_pid() == client_attach.owner_pid()
    {
        fail(FAIL_ATTACH);
    }
    let epoch = provider_attach.manager_epoch();
    if !matches!(epoch, FIRST_MANAGER_EPOCH | SECOND_MANAGER_EPOCH) {
        fail(FAIL_ATTACH);
    }

    let echo = echo_service_name();
    let (impersonation_sender, impersonation, rejected_connector) =
        read_frame_transfer_envelope(client_session.raw());
    if impersonation_sender != client_attach.owner_pid() {
        close_owned(rejected_connector);
        fail(FAIL_IDENTITY_SENDER);
    }
    expect_service_frame(
        impersonation,
        Opcode::Register,
        REGISTER_TXID,
        echo,
        ServiceStatus::Ok,
        0,
        true,
    );
    close_owned(rejected_connector);
    write_frame_bytes(
        client_session.raw(),
        build_frame(Frame::register_reply(
            REGISTER_TXID,
            echo,
            ServiceStatus::PermissionDenied,
            0,
        )),
    );

    let mut services =
        ServiceRegistry::with_epoch(epoch).unwrap_or_else(|| fail(FAIL_MANAGER_PROTOCOL));
    let instance = parent_register_provider(
        provider_session.raw(),
        provider_attach.owner_pid(),
        &mut services,
    );
    let expected_instance = if epoch == FIRST_MANAGER_EPOCH {
        FIRST_EPOCH_INSTANCE_RAW
    } else {
        SECOND_EPOCH_INSTANCE_RAW
    };
    if instance.raw() != expected_instance {
        fail(FAIL_MANAGER_PROTOCOL);
    }

    if epoch == SECOND_MANAGER_EPOCH {
        let stale = InstanceId::new(FIRST_EPOCH_INSTANCE_RAW)
            .unwrap_or_else(|| fail(FAIL_STALE_GENERATION));
        match services.remove_exact(&echo, stale) {
            Err(RemoveError::InstanceMismatch { current }) if current == instance => {}
            Err(_) => fail(FAIL_STALE_GENERATION),
            Ok(entry) => {
                close_registered_service(entry.into_parts().2);
                fail(FAIL_STALE_GENERATION);
            }
        }
    }

    parent_connect_client(
        client_session.raw(),
        client_attach.owner_pid(),
        provider_attach.owner_pid(),
        instance,
        &services,
    );
    let (provider_done_sender, provider_done) = read_scalar_envelope(provider_session.raw());
    let (client_done_sender, client_done) = read_scalar_envelope(client_session.raw());
    if provider_done_sender != provider_attach.owner_pid()
        || client_done_sender != client_attach.owner_pid()
        || provider_done != (ROLE_DONE_TAG, role_epoch_payload(Role::Provider, epoch))
        || client_done != (ROLE_DONE_TAG, role_epoch_payload(Role::Client, epoch))
    {
        fail(FAIL_MANAGER_DONE);
    }
    write_scalar(startup, MANAGER_INSTANCE_TAG, u64::from(instance.raw()));

    // The first generation exits with all service resources live so kernel
    // teardown drives the peer-close recovery observed by both dependents.
    (
        epoch,
        ManagerResidentState {
            provider_session,
            client_session,
            secondary_session: None,
            secondary_pid: 0,
            secondary_lease: 0,
            services,
            provider_pid: provider_attach.owner_pid(),
            client_pid: client_attach.owner_pid(),
            supervisor_pid,
            instance,
        },
    )
}

fn child_provider_service_round(startup: u64, supervisor_pid: u64) -> ProviderResidentState {
    for expected_epoch in [FIRST_MANAGER_EPOCH, SECOND_MANAGER_EPOCH] {
        let (sender_pid, attach, session) = read_attach_envelope(startup);
        if sender_pid != supervisor_pid {
            close_owned(session);
            fail(FAIL_IDENTITY_SENDER);
        }
        expect_child_attach(attach, Role::Provider, expected_epoch);
        let echo = echo_service_name();
        let (provider_connector, registry_connector) = create_channel_owned();
        write_frame_transfer(
            session.raw(),
            build_frame(Frame::register(REGISTER_TXID, echo)),
            registry_connector,
        );
        let registered = read_frame_bytes(session.raw());
        let instance = expect_service_frame(
            registered,
            Opcode::RegisterReply,
            REGISTER_TXID,
            echo,
            ServiceStatus::Ok,
            registered.instance_raw(),
            false,
        )
        .unwrap_or_else(|| fail(FAIL_SM_FRAME_MISMATCH));
        let expected_instance = if expected_epoch == FIRST_MANAGER_EPOCH {
            FIRST_EPOCH_INSTANCE_RAW
        } else {
            SECOND_EPOCH_INSTANCE_RAW
        };
        if instance.raw() != expected_instance {
            fail(FAIL_MANAGER_PROTOCOL);
        }

        let (duplicate_peer, duplicate_connector) = create_channel_owned();
        write_frame_transfer(
            session.raw(),
            build_frame(Frame::register(DUPLICATE_TXID, echo)),
            duplicate_connector,
        );
        let duplicate = read_frame_bytes(session.raw());
        expect_service_frame(
            duplicate,
            Opcode::RegisterReply,
            DUPLICATE_TXID,
            echo,
            ServiceStatus::AlreadyExists,
            0,
            false,
        );
        wait_peer_closed(duplicate_peer.raw());
        close_owned(duplicate_peer);

        let (connect, server_endpoint) = read_frame_transfer(provider_connector.raw());
        expect_service_frame(
            connect,
            Opcode::Connect,
            CLIENT_LOOKUP_TXID,
            echo,
            ServiceStatus::Ok,
            instance.raw(),
            true,
        );
        let echoed = read_scalar(server_endpoint.raw());
        if echoed != (ECHO_TAG, ECHO_PAYLOAD) {
            fail(FAIL_SM_ECHO);
        }
        write_scalar(server_endpoint.raw(), echoed.0, echoed.1);
        close_owned(server_endpoint);
        write_scalar(
            session.raw(),
            ROLE_DONE_TAG,
            role_epoch_payload(Role::Provider, expected_epoch),
        );

        if expected_epoch == SECOND_MANAGER_EPOCH {
            return ProviderResidentState {
                session,
                connector: provider_connector,
                instance,
                supervisor_pid,
            };
        }

        wait_peer_closed(session.raw());
        wait_peer_closed(provider_connector.raw());
        close_owned(session);
        close_owned(provider_connector);
        write_scalar(
            startup,
            MANAGER_LOST_TAG,
            role_epoch_payload(Role::Provider, expected_epoch),
        );
    }
    fail(FAIL_SM_LIFECYCLE)
}

fn child_client_service_round(startup: u64, supervisor_pid: u64) -> ClientResidentState {
    for expected_epoch in [FIRST_MANAGER_EPOCH, SECOND_MANAGER_EPOCH] {
        let (sender_pid, attach, session) = read_attach_envelope(startup);
        if sender_pid != supervisor_pid {
            close_owned(session);
            fail(FAIL_IDENTITY_SENDER);
        }
        expect_child_attach(attach, Role::Client, expected_epoch);
        let echo = echo_service_name();
        let missing = missing_service_name();

        let (rejected_peer, rejected_connector) = create_channel_owned();
        write_frame_transfer(
            session.raw(),
            build_frame(Frame::register(REGISTER_TXID, echo)),
            rejected_connector,
        );
        let rejected = read_frame_bytes(session.raw());
        expect_service_frame(
            rejected,
            Opcode::RegisterReply,
            REGISTER_TXID,
            echo,
            ServiceStatus::PermissionDenied,
            0,
            false,
        );
        wait_peer_closed(rejected_peer.raw());
        close_owned(rejected_peer);

        write_frame_bytes(
            session.raw(),
            build_frame(Frame::lookup(UNKNOWN_LOOKUP_TXID, missing)),
        );
        let unknown = read_frame_bytes(session.raw());
        expect_service_frame(
            unknown,
            Opcode::LookupReply,
            UNKNOWN_LOOKUP_TXID,
            missing,
            ServiceStatus::NotFound,
            0,
            false,
        );

        write_frame_bytes(
            session.raw(),
            build_frame(Frame::lookup(CLIENT_LOOKUP_TXID, echo)),
        );
        let (reply, client_endpoint) = read_frame_transfer(session.raw());
        let instance = expect_service_frame(
            reply,
            Opcode::LookupReply,
            CLIENT_LOOKUP_TXID,
            echo,
            ServiceStatus::Ok,
            reply.instance_raw(),
            true,
        )
        .unwrap_or_else(|| fail(FAIL_SM_FRAME_MISMATCH));
        let expected_instance = if expected_epoch == FIRST_MANAGER_EPOCH {
            FIRST_EPOCH_INSTANCE_RAW
        } else {
            SECOND_EPOCH_INSTANCE_RAW
        };
        if instance.raw() != expected_instance {
            fail(FAIL_MANAGER_PROTOCOL);
        }

        write_scalar(client_endpoint.raw(), ECHO_TAG, ECHO_PAYLOAD);
        if read_scalar(client_endpoint.raw()) != (ECHO_TAG, ECHO_PAYLOAD) {
            fail(FAIL_SM_ECHO);
        }
        close_owned(client_endpoint);
        write_scalar(
            session.raw(),
            ROLE_DONE_TAG,
            role_epoch_payload(Role::Client, expected_epoch),
        );
        if expected_epoch == SECOND_MANAGER_EPOCH {
            return ClientResidentState {
                session,
                instance,
                supervisor_pid,
                client_pid: attach.owner_pid(),
            };
        }
        wait_peer_closed(session.raw());
        close_owned(session);
        write_scalar(
            startup,
            MANAGER_LOST_TAG,
            role_epoch_payload(Role::Client, expected_epoch),
        );
    }
    fail(FAIL_SM_LIFECYCLE)
}

fn expect_child_attach(attach: SupervisorAttachFrame, role: Role, epoch: u16) {
    if attach.role() != role || attach.manager_epoch() != epoch || attach.owner_pid() == 0 {
        fail(FAIL_ATTACH);
    }
}

fn build_attach(epoch: u16, role: Role, owner_pid: u64) -> SupervisorAttachFrame {
    SupervisorAttachFrame::new(epoch, role, owner_pid).unwrap_or_else(|_| fail(FAIL_ATTACH))
}

const fn role_epoch_payload(role: Role, epoch: u16) -> u64 {
    ((epoch as u64) << 32) | role.raw() as u64
}

const fn m20_payload(phase: u8, lease: u16, transaction_id: u32) -> u64 {
    ((phase as u64) << 56) | ((lease as u64) << 32) | transaction_id as u64
}

const fn m20_payload_parts(payload: u64) -> (u8, u16, u32) {
    (
        (payload >> 56) as u8,
        (payload >> 32) as u16,
        payload as u32,
    )
}

fn write_m20_control(transport: u64, tag: u64, phase: u8, lease: u16, transaction_id: u32) {
    write_scalar(transport, tag, m20_payload(phase, lease, transaction_id));
}

fn expect_m20_event(
    transport: u64,
    sender_pid: u64,
    tag: u64,
    phase: u8,
    lease: u16,
    transaction_id: u32,
) {
    let (actual_sender, actual) = read_scalar_envelope(transport);
    if actual_sender != sender_pid || actual != (tag, m20_payload(phase, lease, transaction_id)) {
        fail(FAIL_MULTI_CLIENT_PROTOCOL);
    }
}

const fn post_ready_transaction_id(round: u32) -> u32 {
    POST_READY_TXID_BASE + round
}

const fn serving_round_payload(round: u32) -> u64 {
    ((post_ready_transaction_id(round) as u64) << 32) | round as u64
}

const fn serving_done_payload(role: Role, completed: u32) -> u64 {
    ((completed as u64) << 32) | role.raw() as u64
}

const fn post_ready_echo_payload(round: u32) -> u64 {
    ECHO_PAYLOAD ^ ((round as u64) << 32) ^ post_ready_transaction_id(round) as u64
}

const fn dynamic_commit_payload(phase: u8, transaction_id: u32, instance: u32) -> u64 {
    ((phase as u64) << 56) | (((transaction_id & 0x00ff_ffff) as u64) << 32) | instance as u64
}

const fn dynamic_done_payload(role: Role) -> u64 {
    ((DYNAMIC_FINAL_PHASE as u64) << 32) | role.raw() as u64
}

const fn dynamic_echo_payload(transaction_id: u32, instance: InstanceId) -> u64 {
    ECHO_PAYLOAD ^ ((transaction_id as u64) << 32) ^ instance.raw() as u64
}

const fn reuse_commit_tag(step: u8) -> u64 {
    REUSE_COMMIT_TAG_PREFIX | ((REUSE_ROUND as u64) << 8) | step as u64
}

const fn reuse_commit_payload(transaction_id: u32, instance: u32) -> u64 {
    ((transaction_id as u64) << 32) | instance as u64
}

const fn reuse_done_tag() -> u64 {
    REUSE_DONE_TAG_PREFIX | REUSE_ROUND as u64
}

const fn service_acl_done_payload(
    transaction_id: u32,
    malformed_rejections: u16,
    registry_len: usize,
) -> u64 {
    ((transaction_id as u64) << 32) | ((malformed_rejections as u64) << 16) | registry_len as u64
}

fn echo_service_name() -> ServiceName {
    ServiceName::new(b"bndroid.echo").unwrap_or_else(|_| fail(FAIL_SM_NAME))
}

fn missing_service_name() -> ServiceName {
    ServiceName::new(b"bndroid.missing").unwrap_or_else(|_| fail(FAIL_SM_NAME))
}

fn dynamic_service_name() -> ServiceName {
    ServiceName::new(b"bndroid.echo.dynamic").unwrap_or_else(|_| fail(FAIL_SM_NAME))
}

fn delegated_attack_service_name() -> ServiceName {
    ServiceName::new(b"bndroid.echo.delegated").unwrap_or_else(|_| fail(FAIL_SM_NAME))
}

fn write_dynamic_command(transport: u64, command: u64) {
    write_scalar(transport, DYNAMIC_COMMAND_TAG, command);
}

fn write_dynamic_commit(transport: u64, phase: u8, transaction_id: u32, instance: u32) {
    write_scalar(
        transport,
        DYNAMIC_COMMIT_TAG,
        dynamic_commit_payload(phase, transaction_id, instance),
    );
}

fn read_dynamic_commit(transport: u64, phase: u8, transaction_id: u32) -> InstanceId {
    let (tag, payload) = read_scalar(transport);
    if tag != DYNAMIC_COMMIT_TAG
        || payload >> 32 != dynamic_commit_payload(phase, transaction_id, 0) >> 32
    {
        fail(FAIL_DYNAMIC_COMMIT);
    }
    InstanceId::new(payload as u32).unwrap_or_else(|| fail(FAIL_DYNAMIC_INSTANCE))
}

fn expect_dynamic_commit(transport: u64, phase: u8, transaction_id: u32, instance: u32) {
    if read_scalar(transport)
        != (
            DYNAMIC_COMMIT_TAG,
            dynamic_commit_payload(phase, transaction_id, instance),
        )
    {
        fail(FAIL_DYNAMIC_COMMIT);
    }
}

fn write_reuse_command(transport: u64, command: u64) {
    write_scalar(transport, REUSE_COMMAND_TAG, command);
}

fn write_reuse_commit(transport: u64, step: u8, transaction_id: u32, instance: u32) {
    write_scalar(
        transport,
        reuse_commit_tag(step),
        reuse_commit_payload(transaction_id, instance),
    );
}

fn read_reuse_commit(transport: u64, step: u8, transaction_id: u32) -> InstanceId {
    let (tag, payload) = read_scalar(transport);
    if tag != reuse_commit_tag(step) || payload >> 32 != transaction_id as u64 {
        fail(FAIL_REUSE_COMMIT);
    }
    InstanceId::new(payload as u32).unwrap_or_else(|| fail(FAIL_REUSE_INSTANCE))
}

fn expect_reuse_commit(transport: u64, step: u8, transaction_id: u32, instance: u32) {
    if read_scalar(transport)
        != (
            reuse_commit_tag(step),
            reuse_commit_payload(transaction_id, instance),
        )
    {
        fail(FAIL_REUSE_COMMIT);
    }
}

fn build_frame(result: Result<Frame, FrameError>) -> Frame {
    result.unwrap_or_else(|_| fail(FAIL_SM_FRAME))
}

fn expect_boot_frame(frame: Frame) -> Role {
    if frame.opcode() != Opcode::Boot
        || frame.transaction_id() != 0
        || frame.status() != ServiceStatus::Ok
        || frame.instance_raw() != 0
        || frame.instance().is_some()
        || frame.name().is_some()
        || frame.flags().has_handle()
        || !matches!(
            frame.role(),
            Role::Provider | Role::Client | Role::ServiceManager
        )
    {
        fail(FAIL_SM_FRAME_MISMATCH);
    }
    frame.role()
}

#[allow(clippy::too_many_arguments)]
fn expect_service_frame(
    frame: Frame,
    opcode: Opcode,
    transaction_id: u32,
    name: ServiceName,
    status: ServiceStatus,
    instance: u32,
    has_handle: bool,
) -> Option<InstanceId> {
    if frame.opcode() != opcode
        || frame.role() != Role::Unspecified
        || frame.transaction_id() != transaction_id
        || frame.name() != Some(name)
        || frame.status() != status
        || frame.instance_raw() != instance
        || frame.flags().has_handle() != has_handle
    {
        fail(FAIL_SM_FRAME_MISMATCH);
    }
    frame.instance()
}

fn create_channel_owned() -> (OwnedUserHandle, OwnedUserHandle) {
    let result = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    let left = OwnedUserHandle::new(result.out1);
    let right = OwnedUserHandle::new(result.out2);
    if result.status != Status::Ok.raw()
        || left.is_none()
        || right.is_none()
        || result.out1 == result.out2
    {
        fail(FAIL_SM_LIFECYCLE);
    }
    (
        left.unwrap_or_else(|| fail(FAIL_SM_LIFECYCLE)),
        right.unwrap_or_else(|| fail(FAIL_SM_LIFECYCLE)),
    )
}

fn register_new_service(
    services: &mut ServiceRegistry,
    name: ServiceName,
    connector: OwnedUserHandle,
    owner_pid: u64,
) -> InstanceId {
    let resource = RegisteredService {
        connector,
        owner_pid,
    };
    match services.register(name, resource) {
        Ok(instance) => instance,
        Err(error) => {
            close_registered_service(error.into_resource());
            fail(FAIL_SM_REGISTRY);
        }
    }
}

fn registered_connector(
    services: &ServiceRegistry,
    name: ServiceName,
    instance: InstanceId,
    owner_pid: u64,
) -> u64 {
    let entry = services
        .lookup(&name)
        .unwrap_or_else(|| fail(FAIL_SM_REGISTRY));
    if entry.instance() != instance || entry.resource().owner_pid != owner_pid {
        fail(FAIL_PROVIDER_OWNER);
    }
    entry.resource().connector.raw()
}

fn close_registered_service(resource: RegisteredService) {
    close_owned(resource.connector);
}

fn wait_writable(handle: u64) {
    let requested = signal_union(ObjectSignals::WRITABLE, ObjectSignals::PEER_CLOSED);
    let result = object_wait(handle, requested);
    if result.status != Status::Ok.raw()
        || result.out2 != 0
        || result.out1 & u64::from(ObjectSignals::WRITABLE.bits()) == 0
        || result.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
    {
        fail(FAIL_SM_WAIT);
    }
}

fn wait_readable(handle: u64) {
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let result = object_wait(handle, requested);
    if result.status != Status::Ok.raw()
        || result.out2 != 0
        || result.out1 & u64::from(ObjectSignals::READABLE.bits()) == 0
    {
        fail(FAIL_SM_WAIT);
    }
}

fn wait_peer_closed(handle: u64) {
    let result = object_wait(handle, ObjectSignals::PEER_CLOSED);
    if result.status != Status::Ok.raw()
        || result.out2 != 0
        || result.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
    {
        fail(FAIL_SM_WAIT);
    }
}

fn write_frame_bytes(transport: u64, frame: Frame) {
    wait_writable(transport);
    let bytes = frame.encode();
    let result = syscall(
        SyscallNumber::ChannelWriteBytes,
        transport,
        bytes.as_ptr() as u64,
        FRAME_SIZE as u64,
    );
    if result.status != Status::Ok.raw() || result.out1 != FRAME_SIZE as u64 || result.out2 != 0 {
        fail(FAIL_SM_BYTE_WRITE);
    }
}

fn try_write_ui_event(transport: u64, event: UiServerEvent) -> bool {
    let wire = event.encode();
    let result = syscall(
        SyscallNumber::ChannelWriteBytes,
        transport,
        wire.as_ptr() as u64,
        UI_SERVER_EVENT_WIRE_SIZE as u64,
    );
    if result.status == Status::ShouldWait.raw() && result.out1 == 0 && result.out2 == 0 {
        return false;
    }
    if result.status != Status::Ok.raw()
        || result.out1 != UI_SERVER_EVENT_WIRE_SIZE as u64
        || result.out2 != 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    true
}

fn write_ui_event(transport: u64, event: UiServerEvent) {
    wait_writable(transport);
    if !try_write_ui_event(transport, event) {
        fail(FAIL_SURFACE_PROTOCOL);
    }
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn write_ui_wire(transport: u64, wire: &[u8; PRESENT_WIRE_SIZE]) {
    wait_writable(transport);
    let result = syscall(
        SyscallNumber::ChannelWriteBytes,
        transport,
        wire.as_ptr() as u64,
        PRESENT_WIRE_SIZE as u64,
    );
    if result.status != Status::Ok.raw()
        || result.out1 != PRESENT_WIRE_SIZE as u64
        || result.out2 != 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
}

fn write_buffer_present(transport: u64, present: BufferPresent, transfer: Option<OwnedUserHandle>) {
    wait_writable(transport);
    let wire = present.encode();
    let result = if let Some(source) = transfer {
        let raw_source = source.raw();
        let result = transfer_write(
            transport,
            wire.as_ptr() as u64,
            raw_source,
            BUFFER_PRESENT_WIRE_SIZE,
        );
        if result.status == Status::Ok.raw() {
            assert_stale_handle(raw_source);
        }
        result
    } else {
        syscall(
            SyscallNumber::ChannelWriteBytes,
            transport,
            wire.as_ptr() as u64,
            BUFFER_PRESENT_WIRE_SIZE as u64,
        )
    };
    if result.status != Status::Ok.raw()
        || result.out1 != BUFFER_PRESENT_WIRE_SIZE as u64
        || result.out2 != 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
}

fn write_ui_control(transport: u64, control: UiClientControl) {
    wait_writable(transport);
    let wire = control.encode();
    let result = syscall(
        SyscallNumber::ChannelWriteBytes,
        transport,
        wire.as_ptr() as u64,
        UI_CLIENT_CONTROL_WIRE_SIZE as u64,
    );
    if result.status != Status::Ok.raw()
        || result.out1 != UI_CLIENT_CONTROL_WIRE_SIZE as u64
        || result.out2 != 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
}

fn write_ui_control_transfer(transport: u64, control: UiClientControl, source: OwnedUserHandle) {
    wait_writable(transport);
    let wire = control.encode();
    let raw_source = source.raw();
    let result = transfer_write(
        transport,
        wire.as_ptr() as u64,
        raw_source,
        UI_CLIENT_CONTROL_WIRE_SIZE,
    );
    if result.status != Status::Ok.raw()
        || result.out1 != UI_CLIENT_CONTROL_WIRE_SIZE as u64
        || result.out2 != 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    assert_stale_handle(raw_source);
}

#[cfg(feature = "androidbox-process0")]
fn write_android_app_bootstrap_transfer(
    transport: u64,
    kind: AndroidAppBootstrapKind,
    source: OwnedUserHandle,
) {
    wait_writable(transport);
    let wire = AndroidAppBootstrap::new(kind).encode();
    let raw_source = source.raw();
    let result = transfer_write(
        transport,
        wire.as_ptr() as u64,
        raw_source,
        ANDROID_APP_BOOTSTRAP_WIRE_SIZE,
    );
    if result.status != Status::Ok.raw()
        || result.out1 != ANDROID_APP_BOOTSTRAP_WIRE_SIZE as u64
        || result.out2 != 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    assert_stale_handle(raw_source);
}

#[cfg(feature = "androidbox-restart0")]
fn write_android_app_supervisor_transfer(
    transport: u64,
    message: AndroidAppSupervisorMessage,
    source: OwnedUserHandle,
) {
    wait_writable(transport);
    let wire = message.encode();
    let raw_source = source.raw();
    let result = transfer_write(
        transport,
        wire.as_ptr() as u64,
        raw_source,
        ANDROID_APP_SUPERVISOR_WIRE_SIZE,
    );
    if result.status != Status::Ok.raw()
        || result.out1 != ANDROID_APP_SUPERVISOR_WIRE_SIZE as u64
        || result.out2 != 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    assert_stale_handle(raw_source);
}

#[cfg(feature = "androidbox-restart0")]
fn write_android_app_supervisor_bytes(transport: u64, message: AndroidAppSupervisorMessage) {
    wait_writable(transport);
    let wire = message.encode();
    let result = syscall(
        SyscallNumber::ChannelWriteBytes,
        transport,
        wire.as_ptr() as u64,
        wire.len() as u64,
    );
    if result.status != Status::Ok.raw()
        || result.out1 != ANDROID_APP_SUPERVISOR_WIRE_SIZE as u64
        || result.out2 != 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
}

#[cfg(feature = "androidbox-restart0")]
fn read_android_app_supervisor_transfer(
    transport: u64,
) -> (u64, AndroidAppSupervisorMessage, OwnedUserHandle) {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != ANDROID_APP_SUPERVISOR_WIRE_SIZE
        || !envelope.received_handle().is_valid()
        || envelope.sender_pid() == 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut wire = [0_u8; ANDROID_APP_SUPERVISOR_WIRE_SIZE];
    wire.copy_from_slice(&envelope.data()[..ANDROID_APP_SUPERVISOR_WIRE_SIZE]);
    let message =
        AndroidAppSupervisorMessage::decode(&wire).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    if message.kind() != AndroidAppSupervisorMessageKind::Rebind {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let received = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    (envelope.sender_pid(), message, received)
}

#[cfg(feature = "androidbox-restart0")]
fn read_android_app_supervisor_rebound(transport: u64) -> (u64, AndroidAppSupervisorMessage) {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != ANDROID_APP_SUPERVISOR_WIRE_SIZE
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() == 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut wire = [0_u8; ANDROID_APP_SUPERVISOR_WIRE_SIZE];
    wire.copy_from_slice(&envelope.data()[..ANDROID_APP_SUPERVISOR_WIRE_SIZE]);
    let message =
        AndroidAppSupervisorMessage::decode(&wire).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    if message.kind() != AndroidAppSupervisorMessageKind::Rebound {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    (envelope.sender_pid(), message)
}

#[cfg(all(feature = "androidbox-process0", not(feature = "androidbox-restart0")))]
fn read_android_app_bootstrap_transfer(
    transport: u64,
    expected_kind: AndroidAppBootstrapKind,
) -> (u64, OwnedUserHandle) {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != ANDROID_APP_BOOTSTRAP_WIRE_SIZE
        || !envelope.received_handle().is_valid()
        || envelope.sender_pid() == 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut wire = [0_u8; ANDROID_APP_BOOTSTRAP_WIRE_SIZE];
    wire.copy_from_slice(&envelope.data()[..ANDROID_APP_BOOTSTRAP_WIRE_SIZE]);
    let bootstrap =
        AndroidAppBootstrap::decode(&wire).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    if bootstrap.kind() != expected_kind {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let received = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    (envelope.sender_pid(), received)
}

#[cfg(feature = "androidbox-restart0")]
fn read_android_app_bootstrap_transfer(
    transport: u64,
    expected_kind: AndroidAppBootstrapKind,
) -> (u64, OwnedUserHandle) {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != ANDROID_APP_BOOTSTRAP_WIRE_SIZE
        || !envelope.received_handle().is_valid()
        || envelope.sender_pid() == 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut wire = [0_u8; ANDROID_APP_BOOTSTRAP_WIRE_SIZE];
    wire.copy_from_slice(&envelope.data()[..ANDROID_APP_BOOTSTRAP_WIRE_SIZE]);
    let bootstrap =
        AndroidAppBootstrap::decode(&wire).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    if bootstrap.kind() != expected_kind {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let received = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    (envelope.sender_pid(), received)
}

#[cfg(all(feature = "androidbox-process0", not(feature = "androidbox-restart0")))]
fn read_android_app_bootstrap(startup: u64) -> (OwnedUserHandle, OwnedUserHandle) {
    let bootstrap = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let (surface_sender, surface) = read_android_app_bootstrap_transfer(
        bootstrap.raw(),
        AndroidAppBootstrapKind::SurfaceClient,
    );
    let (runtime_sender, runtime) = read_android_app_bootstrap_transfer(
        bootstrap.raw(),
        AndroidAppBootstrapKind::RuntimeClient,
    );
    if surface_sender != runtime_sender {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    close_owned(bootstrap);
    (surface, runtime)
}

#[cfg(feature = "androidbox-restart0")]
fn read_android_app_bootstrap(
    startup: u64,
) -> (OwnedUserHandle, OwnedUserHandle, OwnedUserHandle, u64) {
    let bootstrap = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let (surface_sender, surface) = read_android_app_bootstrap_transfer(
        bootstrap.raw(),
        AndroidAppBootstrapKind::SurfaceClient,
    );
    let (runtime_sender, runtime) = read_android_app_bootstrap_transfer(
        bootstrap.raw(),
        AndroidAppBootstrapKind::RuntimeClient,
    );
    if surface_sender == 0 || surface_sender != runtime_sender {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    (surface, runtime, bootstrap, surface_sender)
}

fn read_ui_event(transport: u64) -> (u64, UiServerEvent) {
    wait_readable(transport);
    let envelope = read_channel_envelope_now(transport);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != UI_SERVER_EVENT_WIRE_SIZE
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut wire = [0_u8; UI_SERVER_EVENT_WIRE_SIZE];
    wire.copy_from_slice(&envelope.data()[..UI_SERVER_EVENT_WIRE_SIZE]);
    let event = UiServerEvent::decode(&wire).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    (envelope.sender_pid(), event)
}

fn read_ui_provisioning(transport: u64) -> (u64, UiClientControl, OwnedUserHandle) {
    wait_readable(transport);
    let envelope = read_channel_envelope_now(transport);
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != UI_CLIENT_CONTROL_WIRE_SIZE
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let received = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let control = UiClientControl::decode(&envelope.data()[..UI_CLIENT_CONTROL_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    (envelope.sender_pid(), control, received)
}

fn read_ui_client_message_now(transport: u64) -> (u64, UiClientMessage) {
    let envelope = read_channel_envelope_now(transport);
    if envelope.logical_length() != PRESENT_WIRE_SIZE {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let wire = &envelope.data()[..PRESENT_WIRE_SIZE];
    let message = match envelope.kind() {
        ChannelMessageKind::Bytes if !envelope.received_handle().is_valid() => {
            if let Ok(control) = UiClientControl::decode(wire) {
                UiClientMessage::Control(control)
            } else if let Ok(frame) = PresentFrame::decode(wire) {
                UiClientMessage::Present(frame)
            } else if let Ok(frame) = BufferPresent::decode(wire) {
                UiClientMessage::BufferPresent {
                    frame,
                    buffer: None,
                }
            } else {
                fail(FAIL_SURFACE_PROTOCOL)
            }
        }
        ChannelMessageKind::Transfer if envelope.received_handle().is_valid() => {
            let frame = BufferPresent::decode(wire).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
            let buffer = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
                .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
            UiClientMessage::BufferPresent {
                frame,
                buffer: Some(buffer),
            }
        }
        ChannelMessageKind::Scalar | ChannelMessageKind::Bytes | ChannelMessageKind::Transfer => {
            fail(FAIL_SURFACE_PROTOCOL)
        }
    };
    (envelope.sender_pid(), message)
}

fn read_frame_bytes(transport: u64) -> Frame {
    wait_readable(transport);
    read_frame_bytes_now(transport)
}

fn read_channel_envelope_once(transport: u64) -> ChannelReadEnvelope {
    let mut wire = [0xa5_u8; CHANNEL_READ_ENVELOPE_SIZE];
    let result = syscall(
        SyscallNumber::ChannelReadEnvelope,
        transport,
        wire.as_mut_ptr() as u64,
        CHANNEL_READ_ENVELOPE_SIZE as u64,
    );
    if result.status != Status::Ok.raw() || result.out1 == 0 {
        fail(FAIL_IDENTITY_ENVELOPE);
    }
    let envelope =
        ChannelReadEnvelope::decode(&wire).unwrap_or_else(|_| fail(FAIL_IDENTITY_ENVELOPE));
    if envelope.sender_pid() != result.out1 || envelope.kind().raw() != result.out2 {
        fail(FAIL_IDENTITY_ENVELOPE);
    }
    envelope
}

#[cfg(feature = "unified-product-runtime")]
fn read_channel_envelope_now(transport: u64) -> ChannelReadEnvelope {
    loop {
        let envelope = read_channel_envelope_once(transport);
        if product_runtime::dispatch_control(transport, &envelope) {
            wait_readable(transport);
            continue;
        }
        return envelope;
    }
}

#[cfg(not(feature = "unified-product-runtime"))]
fn read_channel_envelope_now(transport: u64) -> ChannelReadEnvelope {
    read_channel_envelope_once(transport)
}

fn read_channel_envelope(transport: u64) -> ChannelReadEnvelope {
    wait_readable(transport);
    read_channel_envelope_now(transport)
}

fn frame_bytes_from_envelope(envelope: ChannelReadEnvelope) -> Frame {
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != FRAME_SIZE
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_IDENTITY_ENVELOPE);
    }
    Frame::decode(envelope.data()).unwrap_or_else(|_| fail(FAIL_SM_FRAME))
}

fn frame_transfer_from_envelope(envelope: ChannelReadEnvelope) -> (Frame, OwnedUserHandle) {
    if envelope.kind() != ChannelMessageKind::Transfer || envelope.logical_length() != FRAME_SIZE {
        fail(FAIL_IDENTITY_ENVELOPE);
    }
    let received = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_IDENTITY_ENVELOPE));
    (
        Frame::decode(envelope.data()).unwrap_or_else(|_| fail(FAIL_SM_FRAME)),
        received,
    )
}

fn read_frame_bytes_envelope(transport: u64) -> (u64, Frame) {
    let envelope = read_channel_envelope(transport);
    (envelope.sender_pid(), frame_bytes_from_envelope(envelope))
}

fn read_frame_bytes_envelope_now(transport: u64) -> (u64, Frame) {
    // Both rejected calls must leave the exact FIFO head in place. Every
    // process executes this against its BOOT frame before the successful read.
    let mut untouched = [0xa5_u8; CHANNEL_READ_ENVELOPE_SIZE];
    let too_small = syscall(
        SyscallNumber::ChannelReadEnvelope,
        transport,
        untouched.as_mut_ptr() as u64,
        (CHANNEL_READ_ENVELOPE_SIZE - 1) as u64,
    );
    if too_small.status != Status::BufferTooSmall.raw()
        || too_small.out1 != CHANNEL_READ_ENVELOPE_SIZE as u64
        || too_small.out2 != 0
        || untouched.iter().any(|byte| *byte != 0xa5)
    {
        fail(FAIL_IDENTITY_ENVELOPE);
    }
    let bad_address = syscall(
        SyscallNumber::ChannelReadEnvelope,
        transport,
        USER_GUARD_HIGH - (CHANNEL_READ_ENVELOPE_SIZE as u64 / 2),
        CHANNEL_READ_ENVELOPE_SIZE as u64,
    );
    if bad_address.status != Status::BadAddress.raw()
        || bad_address.out1 != 0
        || bad_address.out2 != 0
    {
        fail(FAIL_IDENTITY_ENVELOPE);
    }
    let envelope = read_channel_envelope_now(transport);
    (envelope.sender_pid(), frame_bytes_from_envelope(envelope))
}

fn read_frame_transfer_envelope(transport: u64) -> (u64, Frame, OwnedUserHandle) {
    let envelope = read_channel_envelope(transport);
    let sender_pid = envelope.sender_pid();
    let (frame, received) = frame_transfer_from_envelope(envelope);
    (sender_pid, frame, received)
}

fn read_attach_envelope(transport: u64) -> (u64, SupervisorAttachFrame, OwnedUserHandle) {
    let envelope = read_channel_envelope(transport);
    let sender_pid = envelope.sender_pid();
    let (attach, received) = attach_from_envelope(envelope);
    (sender_pid, attach, received)
}

fn attach_from_envelope(envelope: ChannelReadEnvelope) -> (SupervisorAttachFrame, OwnedUserHandle) {
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != SUPERVISOR_ATTACH_FRAME_SIZE
    {
        fail(FAIL_IDENTITY_ENVELOPE);
    }
    let received = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_IDENTITY_ENVELOPE));
    let mut bytes = [0_u8; SUPERVISOR_ATTACH_FRAME_SIZE];
    bytes.copy_from_slice(&envelope.data()[..SUPERVISOR_ATTACH_FRAME_SIZE]);
    let attach = SupervisorAttachFrame::decode(&bytes).unwrap_or_else(|_| fail(FAIL_ATTACH));
    (attach, received)
}

fn read_scalar_envelope(transport: u64) -> (u64, (u64, u64)) {
    let envelope = read_channel_envelope(transport);
    let scalar = envelope
        .scalar_values()
        .unwrap_or_else(|| fail(FAIL_IDENTITY_ENVELOPE));
    (envelope.sender_pid(), scalar)
}

fn read_scalar_envelope_now(transport: u64) -> (u64, (u64, u64)) {
    let envelope = read_channel_envelope_now(transport);
    let scalar = envelope
        .scalar_values()
        .unwrap_or_else(|| fail(FAIL_IDENTITY_ENVELOPE));
    (envelope.sender_pid(), scalar)
}

fn write_empty_transfer(transport: u64, source: OwnedUserHandle) {
    wait_writable(transport);
    let raw_source = source.raw();
    let result = transfer_write(transport, u64::MAX, raw_source, 0);
    if result.status != Status::Ok.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(FAIL_IDENTITY_PROTOCOL);
    }
    assert_stale_handle(raw_source);
}

fn read_empty_transfer_envelope(transport: u64) -> (u64, OwnedUserHandle) {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != 0
        || envelope.data().iter().any(|byte| *byte != 0)
    {
        fail(FAIL_IDENTITY_ENVELOPE);
    }
    let received = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_IDENTITY_ENVELOPE));
    (envelope.sender_pid(), received)
}

fn read_frame_bytes_now(transport: u64) -> Frame {
    let mut bytes = [0xa5_u8; FRAME_SIZE];
    let result = syscall(
        SyscallNumber::ChannelReadBytes,
        transport,
        bytes.as_mut_ptr() as u64,
        FRAME_SIZE as u64,
    );
    if result.status != Status::Ok.raw() || result.out1 != FRAME_SIZE as u64 || result.out2 != 0 {
        fail(FAIL_SM_BYTE_READ);
    }
    Frame::decode(&bytes).unwrap_or_else(|_| fail(FAIL_SM_FRAME))
}

fn write_frame_transfer(transport: u64, frame: Frame, source: OwnedUserHandle) {
    wait_writable(transport);
    let bytes = frame.encode();
    let raw_source = source.raw();
    let result = transfer_write(transport, bytes.as_ptr() as u64, raw_source, FRAME_SIZE);
    if result.status != Status::Ok.raw() || result.out1 != FRAME_SIZE as u64 || result.out2 != 0 {
        fail(FAIL_SM_TRANSFER_WRITE);
    }
    assert_stale_handle(raw_source);
}

fn write_untrusted_bytes(transport: u64, bytes: &[u8]) {
    wait_writable(transport);
    let result = syscall(
        SyscallNumber::ChannelWriteBytes,
        transport,
        bytes.as_ptr() as u64,
        bytes.len() as u64,
    );
    if result.status != Status::Ok.raw() || result.out1 != bytes.len() as u64 || result.out2 != 0 {
        fail(FAIL_SERVICE_ACL);
    }
}

fn write_untrusted_transfer(transport: u64, bytes: &[u8], source: OwnedUserHandle) {
    wait_writable(transport);
    let raw_source = source.raw();
    let result = transfer_write(transport, bytes.as_ptr() as u64, raw_source, bytes.len());
    if result.status != Status::Ok.raw() || result.out1 != bytes.len() as u64 || result.out2 != 0 {
        fail(FAIL_SERVICE_ACL);
    }
    assert_stale_handle(raw_source);
}

fn read_frame_transfer(transport: u64) -> (Frame, OwnedUserHandle) {
    wait_readable(transport);
    read_frame_transfer_now(transport)
}

fn read_frame_transfer_now(transport: u64) -> (Frame, OwnedUserHandle) {
    let mut bytes = [0xa5_u8; FRAME_SIZE];
    let result = syscall(
        SyscallNumber::ChannelReadTransfer,
        transport,
        bytes.as_mut_ptr() as u64,
        FRAME_SIZE as u64,
    );
    let received = OwnedUserHandle::new(result.out2);
    if result.status != Status::Ok.raw() || result.out1 != FRAME_SIZE as u64 || received.is_none() {
        fail(FAIL_SM_TRANSFER_READ);
    }
    (
        Frame::decode(&bytes).unwrap_or_else(|_| fail(FAIL_SM_FRAME)),
        received.unwrap_or_else(|| fail(FAIL_SM_TRANSFER_READ)),
    )
}

fn write_attach_transfer(transport: u64, frame: SupervisorAttachFrame, source: OwnedUserHandle) {
    wait_writable(transport);
    let bytes = frame.encode();
    let raw_source = source.raw();
    let result = transfer_write(
        transport,
        bytes.as_ptr() as u64,
        raw_source,
        SUPERVISOR_ATTACH_FRAME_SIZE,
    );
    if result.status != Status::Ok.raw()
        || result.out1 != SUPERVISOR_ATTACH_FRAME_SIZE as u64
        || result.out2 != 0
    {
        fail(FAIL_ATTACH);
    }
    assert_stale_handle(raw_source);
}

fn write_scalar(transport: u64, tag: u64, payload: u64) {
    wait_writable(transport);
    let result = syscall(SyscallNumber::ChannelWrite, transport, tag, payload);
    if result.status != Status::Ok.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(FAIL_SM_ECHO);
    }
}

fn read_scalar(transport: u64) -> (u64, u64) {
    wait_readable(transport);
    read_scalar_now(transport)
}

fn read_scalar_now(transport: u64) -> (u64, u64) {
    let result = syscall(SyscallNumber::ChannelRead, transport, 0, 0);
    if result.status != Status::Ok.raw() {
        fail(FAIL_SM_ECHO);
    }
    (result.out1, result.out2)
}

fn assert_stale_handle(raw: u64) {
    let result = syscall(SyscallNumber::HandleClose, raw, 0, 0);
    if result.status != Status::NotFound.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(FAIL_SM_TRANSFER_STALE);
    }
}

fn close_owned(handle: OwnedUserHandle) {
    let result = syscall(SyscallNumber::HandleClose, handle.raw(), 0, 0);
    if result.status != Status::Ok.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(FAIL_SM_CLOSE);
    }
}

fn transfer_write(transport: u64, source: u64, handle: u64, length: usize) -> SyscallResult {
    let Ok(raw_handle) = u32::try_from(handle) else {
        fail(67);
    };
    let Ok(raw_length) = u32::try_from(length) else {
        fail(68);
    };
    syscall(
        SyscallNumber::ChannelWriteTransfer,
        transport,
        source,
        pack_transfer(HandleValue::from_raw(raw_handle), raw_length),
    )
}

fn object_wait(handle: u64, requested: ObjectSignals) -> SyscallResult {
    syscall(
        SyscallNumber::ObjectWait,
        handle,
        u64::from(requested.bits()),
        0,
    )
}

fn exercise_object_wait_many_array() {
    let event = syscall(SyscallNumber::EventCreate, 0, 0, 0);
    if event.status != Status::Ok.raw() || event.out1 == 0 || event.out2 != 0 {
        fail(FAIL_WAIT_MANY_ARRAY_POLL);
    }

    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    let event_signaled = pack_user_wait_item(event.out1, ObjectSignals::SIGNALED);
    items.fill(event_signaled);

    for count in [0, OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS + 1] {
        let rejected = object_wait_many_array(&items, count, OBJECT_WAIT_TIMEOUT_INFINITE);
        if rejected.status != Status::InvalidArgument.raw()
            || rejected.out1 != 0
            || rejected.out2 != 0
        {
            fail(FAIL_WAIT_MANY_ARRAY_COUNT);
        }
    }

    // A pointer wholly inside the low guard and a two-item copy crossing from
    // the mapped stack into the high guard must both fail before publishing a
    // scheduler wait token. The latter proves a partial user copy rolls back.
    let bad_pointer = object_wait_many_array_raw(USER_GUARD_LOW, 1, OBJECT_WAIT_TIMEOUT_INFINITE);
    if bad_pointer.status != Status::BadAddress.raw()
        || bad_pointer.out1 != 0
        || bad_pointer.out2 != 0
    {
        fail(FAIL_WAIT_MANY_ARRAY_BAD_POINTER);
    }
    let crossing_guard = object_wait_many_array_raw(
        USER_GUARD_HIGH - core::mem::size_of::<u64>() as u64,
        2,
        OBJECT_WAIT_TIMEOUT_INFINITE,
    );
    if crossing_guard.status != Status::BadAddress.raw()
        || crossing_guard.out1 != 0
        || crossing_guard.out2 != 0
    {
        fail(FAIL_WAIT_MANY_ARRAY_GUARD_ROLLBACK);
    }

    let polled = object_wait_many_array(
        &items,
        OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS,
        OBJECT_WAIT_TIMEOUT_POLL,
    );
    if polled.status != Status::Timeout.raw() || polled.out1 != u64::MAX || polled.out2 != 0 {
        fail(FAIL_WAIT_MANY_ARRAY_POLL);
    }

    if !event_change(SyscallNumber::EventSignal, event.out1, true) {
        fail(FAIL_WAIT_MANY_ARRAY_LOWEST);
    }
    items[2] = pack_user_wait_item(event.out1, ObjectSignals::READABLE);
    let validate_all = object_wait_many_array(&items, 3, OBJECT_WAIT_TIMEOUT_INFINITE);
    if validate_all.status != Status::InvalidArgument.raw()
        || validate_all.out1 != 0
        || validate_all.out2 != 0
    {
        fail(FAIL_WAIT_MANY_ARRAY_VALIDATE_ALL);
    }

    items[2] = event_signaled;
    let lowest = object_wait_many_array(&items, 3, OBJECT_WAIT_TIMEOUT_INFINITE);
    if lowest.status != Status::Ok.raw()
        || lowest.out1 != 0
        || lowest.out2 & u64::from(ObjectSignals::SIGNALED.bits()) == 0
    {
        fail(FAIL_WAIT_MANY_ARRAY_LOWEST);
    }
    if !event_change(SyscallNumber::EventClear, event.out1, true)
        || syscall(SyscallNumber::HandleClose, event.out1, 0, 0).status != Status::Ok.raw()
    {
        fail(FAIL_WAIT_MANY_ARRAY_LOWEST);
    }
}

fn pack_user_wait_item(handle: u64, requested: ObjectSignals) -> u64 {
    let Ok(raw) = u32::try_from(handle) else {
        fail(FAIL_WAIT_MANY_ARRAY_VALIDATE_ALL);
    };
    pack_wait_item(HandleValue::from_raw(raw), requested)
}

fn object_wait_many_array(
    items: &[u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS],
    count: usize,
    timeout_ns: u64,
) -> SyscallResult {
    object_wait_many_array_raw(items.as_ptr() as u64, count, timeout_ns)
}

fn object_wait_many_array_raw(items: u64, count: usize, timeout_ns: u64) -> SyscallResult {
    syscall(
        SyscallNumber::ObjectWaitManyArray,
        items,
        count as u64,
        timeout_ns,
    )
}

fn object_wait_many(
    first_handle: u64,
    first_requested: ObjectSignals,
    second_handle: u64,
    second_requested: ObjectSignals,
    timeout_ns: u64,
) -> SyscallResult {
    let Ok(first_raw) = u32::try_from(first_handle) else {
        fail(FAIL_WAIT_MANY_VALIDATE_ALL);
    };
    let Ok(second_raw) = u32::try_from(second_handle) else {
        fail(FAIL_WAIT_MANY_VALIDATE_ALL);
    };
    syscall(
        SyscallNumber::ObjectWaitMany,
        pack_wait_item(HandleValue::from_raw(first_raw), first_requested),
        pack_wait_item(HandleValue::from_raw(second_raw), second_requested),
        timeout_ns,
    )
}

fn event_change(number: SyscallNumber, handle: u64, changed: bool) -> bool {
    let result = syscall(number, handle, 0, 0);
    result.status == Status::Ok.raw() && result.out1 == u64::from(changed) && result.out2 == 0
}

fn signal_union(left: ObjectSignals, right: ObjectSignals) -> ObjectSignals {
    ObjectSignals::from_bits(left.bits() | right.bits()).unwrap_or_else(|| fail(81))
}

fn child_prelude(startup: u64, expected_role: Role) -> u64 {
    let abi = syscall(SyscallNumber::AbiVersion, 0, 0, 0);
    if abi.status != Status::Ok.raw() || abi.out1 != ABI_VERSION {
        fail(34);
    }

    let startup_ready = object_wait(
        startup,
        signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED),
    );
    if startup_ready.status != Status::Ok.raw()
        || startup_ready.out1 & u64::from(ObjectSignals::READABLE.bits()) == 0
    {
        fail(80);
    }

    // The image identity comes from the kernel startup ABI. BOOT remains an
    // independent protocol cross-check and can never dispatch this executable
    // into another role.
    let (supervisor_pid, boot) = read_frame_bytes_envelope_now(startup);
    let role = expect_boot_frame(boot);
    if supervisor_pid == 0 || role != expected_role {
        fail(FAIL_SM_ROLE);
    }
    child_generic_transfer_round(startup, supervisor_pid);
    write_scalar(startup, ROLE_READY_TAG, u64::from(expected_role.raw()));
    supervisor_pid
}

fn exit_child(exit_code: u64) -> ! {
    let _ = syscall(SyscallNumber::ThreadExit, exit_code, 0, 0);
    fail(36)
}

fn publish_resident_ready<T: ResidentState>(startup: u64, role: Role, epoch: u16, state: &T) {
    if epoch != SECOND_MANAGER_EPOCH || !state.is_reachable() {
        fail(FAIL_SM_LIFECYCLE);
    }
    consume_abandoned_transfer(startup);
    write_scalar(startup, RESIDENT_READY_TAG, role_epoch_payload(role, epoch));
    if !state.is_reachable() {
        fail(FAIL_SM_LIFECYCLE);
    }
}

fn manager_resident_loop(startup: u64, epoch: u16, mut state: ManagerResidentState) -> ! {
    if syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0).status != Status::PermissionDenied.raw() {
        fail(FAIL_SURFACE_ACQUIRE);
    }
    publish_resident_ready(startup, Role::ServiceManager, epoch, &state);
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut provider_requests = 0_u32;
    let mut client_requests = 0_u32;
    let mut dynamic_done = false;
    let mut reuse_done = false;
    let mut acl_reported = false;
    let mut malformed_rejections = 0_u16;

    loop {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(startup, requested);
        items[1] = pack_user_wait_item(state.provider_session.raw(), requested);
        items[2] = pack_user_wait_item(state.client_session.raw(), requested);
        let count = if let Some(secondary) = state.secondary_session.as_ref() {
            items[3] = pack_user_wait_item(secondary.channel.raw(), requested);
            4
        } else {
            3
        };
        let ready = object_wait_many_array(&items, count, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw()
            || ready.out1 >= count as u64
            || ready.out2 & u64::from(requested.bits()) == 0
        {
            fail(FAIL_MANAGER_PROTOCOL);
        }

        if ready.out1 == 0 {
            if ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
                fail(FAIL_MANAGER_PROTOCOL);
            }
            manager_dispatch_m20_control(startup, epoch, &mut state);
        } else if ready.out1 == 1 {
            if ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
                fail(FAIL_MANAGER_PROTOCOL);
            }
            match manager_dispatch_provider_request(&mut state) {
                ManagerDispatchOutcome::Authorized => {
                    provider_requests = provider_requests
                        .checked_add(1)
                        .unwrap_or_else(|| fail(FAIL_DYNAMIC_PROTOCOL));
                }
                ManagerDispatchOutcome::Unauthorized => {
                    if reuse_done
                        && !acl_reported
                        && malformed_rejections >= MALFORMED_REQUEST_COUNT
                        && state.services.len() == 1
                        && state.services.lookup(&echo_service_name()).is_some()
                        && state
                            .services
                            .lookup(&delegated_attack_service_name())
                            .is_none()
                    {
                        write_scalar(startup, PROVIDER_IDENTITY_DONE_TAG, state.provider_pid);
                        write_scalar(startup, CLIENT_IDENTITY_DONE_TAG, state.client_pid);
                        write_scalar(
                            startup,
                            SERVICE_ACL_DONE_TAG,
                            service_acl_done_payload(
                                IDENTITY_ATTACK_TXID,
                                MALFORMED_REQUEST_COUNT,
                                state.services.len(),
                            ),
                        );
                        acl_reported = true;
                    }
                }
                ManagerDispatchOutcome::Malformed => {
                    malformed_rejections = malformed_rejections.saturating_add(1);
                }
            }
        } else if ready.out1 == 2 {
            if ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
                fail(FAIL_MANAGER_PROTOCOL);
            }
            match manager_dispatch_client_lookup(
                &state,
                state.client_session.raw(),
                state.client_pid,
            ) {
                ManagerDispatchOutcome::Authorized => {
                    if state.secondary_pid == 0 {
                        client_requests = client_requests
                            .checked_add(1)
                            .unwrap_or_else(|| fail(FAIL_DYNAMIC_PROTOCOL));
                        if client_requests == POST_READY_ECHO_ROUNDS {
                            write_scalar(
                                startup,
                                SERVING_DONE_TAG,
                                serving_done_payload(Role::ServiceManager, client_requests),
                            );
                        }
                    }
                }
                ManagerDispatchOutcome::Unauthorized => {}
                ManagerDispatchOutcome::Malformed => {
                    malformed_rejections = malformed_rejections.saturating_add(1);
                }
            }
        } else {
            let (secondary_channel, secondary_pid) = state
                .secondary_session
                .as_ref()
                .map(|session| (session.channel.raw(), session.owner_pid))
                .unwrap_or_else(|| fail(FAIL_MULTI_CLIENT_PROTOCOL));
            if ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
                fail(FAIL_MULTI_CLIENT_CLEANUP);
            }
            if manager_dispatch_client_lookup(&state, secondary_channel, secondary_pid)
                != ManagerDispatchOutcome::Authorized
            {
                fail(FAIL_MULTI_CLIENT_PROTOCOL);
            }
        }

        if !dynamic_done
            && provider_requests == 5
            && client_requests == POST_READY_ECHO_ROUNDS + 3
            && state.services.len() == 1
        {
            write_scalar(
                startup,
                DYNAMIC_DONE_TAG,
                dynamic_done_payload(Role::ServiceManager),
            );
            dynamic_done = true;
        }
        if !reuse_done
            && provider_requests == 7
            && client_requests == 7
            && state.services.len() == 1
        {
            write_scalar(startup, reuse_done_tag(), Role::ServiceManager.raw() as u64);
            reuse_done = true;
        }
        if !state.is_reachable() {
            fail(FAIL_SM_LIFECYCLE);
        }
    }
}

#[cfg(feature = "androidbox-interactive0")]
fn create_mobile_system_chrome_buffer() -> (OwnedUserHandle, OwnedUserHandle) {
    let created = syscall(
        SyscallNumber::GraphicsBufferCreate,
        GRAPHICS_BUFFER_FORMAT_XRGB8888,
        pack_graphics_buffer_geometry(GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_HEIGHT),
        GRAPHICS_BUFFER_CREATE_FLAGS_NONE,
    );
    if created.status != Status::Ok.raw()
        || created.out1 == 0
        || created.out2 != GRAPHICS_BUFFER_LOGICAL_BYTES as u64
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let producer =
        OwnedUserHandle::new(created.out1).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let duplicate = syscall(
        SyscallNumber::HandleDuplicate,
        producer.raw(),
        u64::from(Rights::GRAPHICS_BUFFER_SERVER.bits()),
        0,
    );
    if duplicate.status != Status::Ok.raw() || duplicate.out1 == 0 || duplicate.out2 != 0 {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let present =
        OwnedUserHandle::new(duplicate.out1).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    (producer, present)
}

#[cfg(feature = "mobile-ui-runtime")]
fn read_mobile_clock_seconds() -> u64 {
    let clock = syscall(SyscallNumber::SystemClockRead, 0, 0, 0);
    if clock.status != Status::Ok.raw() || clock.out2 != SYSTEM_CLOCK_SOURCE_QEMU_PL031 {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    clock.out1
}

fn surface_server_loop(startup: u64) -> ! {
    let acquired = syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0);
    if acquired.status != Status::Ok.raw() || acquired.out1 == 0 || acquired.out2 == 0 {
        fail(FAIL_SURFACE_ACQUIRE);
    }
    #[cfg(feature = "mobile-ui-runtime")]
    let mobile_clock = MobileClockSession::new(read_mobile_clock_seconds());
    let capability =
        OwnedUserHandle::new(acquired.out1).unwrap_or_else(|| fail(FAIL_SURFACE_ACQUIRE));
    let duplicate = syscall(
        SyscallNumber::HandleDuplicate,
        capability.raw(),
        u64::from(Rights::SURFACE_DEFAULT.bits()),
        0,
    );
    if duplicate.status != Status::PermissionDenied.raw()
        || duplicate.out1 != 0
        || duplicate.out2 != 0
    {
        fail(FAIL_SURFACE_ACQUIRE);
    }

    let (provisioner_pid, attach, app_channel) = read_ui_provisioning(startup);
    if provisioner_pid == 0
        || !matches!(
            attach.payload(),
            UiClientControlPayload::AttachAppEndpoint {
                client: UiClientId::App
            }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    #[cfg(feature = "androidbox-interactive0")]
    let (system_chrome_buffer, system_chrome_buffer_for_present) =
        create_mobile_system_chrome_buffer();
    let mut server = SurfaceServerRuntime {
        launcher_channel: OwnedUserHandle::new(startup)
            .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL)),
        app_channel,
        launcher_pid: None,
        app_pid: None,
        launcher_buffer: None,
        app_buffer: None,
        capability,
        session_id: acquired.out2,
        active_client: UiClientId::Launcher,
        active_app: None,
        focus_generation: 0,
        global_frame_id: 0,
        launcher_frame_id: None,
        app_frame_id: None,
        launcher_input_sequence: 0,
        app_input_sequence: 0,
        input_capture: None,
        pointer_pressed: false,
        focus_controls: UiClientControlTracker::new(),
        wait_rotation: 0,
        #[cfg(feature = "mobile-ui-runtime")]
        appearance: UiAppearanceSession::new(),
        #[cfg(feature = "mobile-ui-runtime")]
        boot_notification: UiBootNotificationSession::new(),
        #[cfg(feature = "mobile-ui-runtime")]
        system_ui: UiSystemUiSession::new(),
        #[cfg(feature = "androidbox-interactive0")]
        system_chrome_buffer,
        #[cfg(feature = "androidbox-interactive0")]
        system_chrome_buffer_for_present,
        #[cfg(feature = "androidbox-interactive0")]
        system_chrome_generation: 0,
        #[cfg(feature = "mobile-ui-runtime")]
        clock: mobile_clock,
        #[cfg(feature = "mobile-ui-runtime")]
        androidbox_audit: UiAndroidBoxAuditSession::new(),
        #[cfg(feature = "mobile-ui-runtime")]
        androidbox_activity_audit: bndr_ui::UiAndroidBoxActivityAuditSession::new(),
        #[cfg(feature = "mobile-ui-runtime")]
        androidbox_resource_audit: UiAndroidBoxResourceAuditSession::new(),
        #[cfg(feature = "mobile-ui-runtime")]
        system_nav_capture: None,
        #[cfg(feature = "androidbox-interactive0")]
        system_top_chrome_capture: false,
        #[cfg(feature = "mobile-ui-runtime")]
        compatible_verification_input_barrier: CompatibleVerificationInputBarrier::new(),
        #[cfg(feature = "ui-stale-present-evidence")]
        stale_present_barrier: UiStalePresentBarrier::Awaiting,
    };
    let ready =
        UiServerEvent::ready(server.session_id).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    write_ui_event(server.launcher_channel.raw(), ready);
    write_ui_event(server.app_channel.raw(), ready);
    #[cfg(feature = "mobile-ui-runtime")]
    publish_ui_appearance(
        &server,
        server.appearance.appearance(),
        server.appearance.revision(),
    );
    #[cfg(feature = "mobile-ui-runtime")]
    publish_ui_clock(&server, server.clock.unix_seconds, server.clock.revision);
    #[cfg(feature = "mobile-ui-runtime")]
    publish_ui_boot_notification(
        &server,
        server.boot_notification.visible(),
        server.boot_notification.revision(),
    );
    #[cfg(feature = "mobile-ui-runtime")]
    publish_ui_system_ui(&server, server.system_ui.snapshot());
    publish_ui_focus(&mut server, UiClientId::Launcher, None);

    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    loop {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        let canonical_sources = [
            UiServerWaitSource::Surface,
            UiServerWaitSource::Launcher,
            UiServerWaitSource::App,
        ];
        let mut sources = [UiServerWaitSource::Surface; 3];
        let source_count = sources.len();
        for (index, source) in sources.iter_mut().enumerate() {
            *source = canonical_sources[(server.wait_rotation + index) % source_count];
            let handle = match *source {
                UiServerWaitSource::Surface => server.capability.raw(),
                UiServerWaitSource::Launcher => server.launcher_channel.raw(),
                UiServerWaitSource::App => server.app_channel.raw(),
            };
            items[index] = pack_user_wait_item(handle, requested);
        }
        #[cfg(feature = "mobile-ui-runtime")]
        let wait_timeout_ns = server.clock.wait_timeout_ns();
        #[cfg(not(feature = "mobile-ui-runtime"))]
        let wait_timeout_ns = OBJECT_WAIT_TIMEOUT_INFINITE;
        let ready = object_wait_many_array(&items, source_count, wait_timeout_ns);
        #[cfg(feature = "mobile-ui-runtime")]
        if ready.status == Status::Timeout.raw() {
            if ready.out1 != u64::MAX || ready.out2 != 0 {
                fail(FAIL_SURFACE_PROTOCOL);
            }
            refresh_mobile_clock(&mut server);
            continue;
        }
        if ready.status != Status::Ok.raw()
            || usize::try_from(ready.out1).map_or(true, |index| index >= source_count)
            || ready.out2 & u64::from(requested.bits()) == 0
        {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        if ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
            && ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
        {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        let ready_index =
            usize::try_from(ready.out1).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
        match sources[ready_index] {
            UiServerWaitSource::Surface => dispatch_surface_input(&mut server),
            UiServerWaitSource::Launcher => {
                dispatch_ui_client_message(&mut server, UiClientId::Launcher)
            }
            UiServerWaitSource::App => dispatch_ui_client_message(&mut server, UiClientId::App),
        }
        server.wait_rotation = (server.wait_rotation + ready_index + 1) % source_count;
        #[cfg(feature = "mobile-ui-runtime")]
        refresh_mobile_clock(&mut server);
    }
}

fn publish_ui_focus(
    server: &mut SurfaceServerRuntime,
    active_client: UiClientId,
    app: Option<ShellAppId>,
) {
    let focus_generation = server
        .focus_generation
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let event =
        UiServerEvent::focus_changed(server.session_id, active_client, app, focus_generation)
            .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    write_ui_event(server.launcher_channel.raw(), event);
    write_ui_event(server.app_channel.raw(), event);
    server.active_client = active_client;
    server.active_app = app;
    server.focus_generation = focus_generation;
}

#[cfg(feature = "mobile-ui-runtime")]
fn publish_ui_clock(server: &SurfaceServerRuntime, unix_seconds: u64, revision: u64) {
    let event = UiServerEvent::clock_changed(server.session_id, unix_seconds, revision)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    // Both clients receive one identical system-session snapshot. This grants
    // neither client RTC nor MMIO authority.
    write_ui_event(server.launcher_channel.raw(), event);
    write_ui_event(server.app_channel.raw(), event);
}

#[cfg(feature = "mobile-ui-runtime")]
fn refresh_mobile_clock(server: &mut SurfaceServerRuntime) {
    let unix_seconds = read_mobile_clock_seconds();
    let update = server
        .clock
        .observe(unix_seconds)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    if let Some(update) = update {
        publish_ui_clock(server, update.unix_seconds, update.revision);
    }
}

#[cfg(feature = "mobile-ui-runtime")]
fn publish_ui_appearance(server: &SurfaceServerRuntime, appearance: UiAppearance, revision: u64) {
    let event = UiServerEvent::appearance_changed(server.session_id, appearance, revision)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    // The order is part of the bounded two-client session contract.
    write_ui_event(server.launcher_channel.raw(), event);
    write_ui_event(server.app_channel.raw(), event);
}

#[cfg(feature = "mobile-ui-runtime")]
fn publish_ui_boot_notification(server: &SurfaceServerRuntime, visible: bool, revision: u64) {
    let event = UiServerEvent::boot_notification_changed(server.session_id, visible, revision)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    // This fixed state is delivered identically to both clients. It grants no
    // authority to post arbitrary content or request delivery from elsewhere.
    write_ui_event(server.launcher_channel.raw(), event);
    write_ui_event(server.app_channel.raw(), event);
}

#[cfg(feature = "mobile-ui-runtime")]
fn publish_ui_system_ui(server: &SurfaceServerRuntime, update: UiSystemUiUpdate) {
    let event = UiServerEvent::system_ui_update(server.session_id, update)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    // Both clients mirror the same SurfaceServer-owned system layer. This
    // snapshot carries no framebuffer, background-task, or hardware authority.
    write_ui_event(server.launcher_channel.raw(), event);
    write_ui_event(server.app_channel.raw(), event);
}

#[cfg(feature = "mobile-ui-runtime")]
const fn compatible_reservation_origin_for_mode(
    mode: UiSystemUiMode,
) -> Option<UiCompatibleActivityReservationOrigin> {
    match mode {
        UiSystemUiMode::Home => Some(UiCompatibleActivityReservationOrigin::Home),
        UiSystemUiMode::Overview => Some(UiCompatibleActivityReservationOrigin::Overview),
        UiSystemUiMode::Locked | UiSystemUiMode::Foreground => None,
    }
}

#[cfg(feature = "mobile-ui-runtime")]
fn publish_ui_system_ui_request_completion(
    server: &SurfaceServerRuntime,
    action: UiSystemUiAction,
    status: UiSystemUiRequestStatus,
    request_id: u64,
    revision: u64,
    compatible_identity: Option<UiCompatibleActivityIdentity>,
    reservation_origin: Option<UiCompatibleActivityReservationOrigin>,
) {
    let event = UiServerEvent::system_ui_request_completed(
        server.session_id,
        action,
        status,
        request_id,
        revision,
        compatible_identity,
        reservation_origin,
    )
    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    // The authenticated Launcher is the sole SystemUI control producer. The
    // state snapshot remains a two-client broadcast; this request result is a
    // one-client acknowledgement and grants no additional capability.
    write_ui_event(server.launcher_channel.raw(), event);
}

#[cfg(feature = "mobile-ui-runtime")]
const MOBILE_SYSTEM_NAV_CAPTURE_TOP_PX: u16 = 1_512;
#[cfg(feature = "mobile-ui-runtime")]
const MOBILE_SYSTEM_NAV_ACTIVATION_PX: u16 = 24;
#[cfg(feature = "mobile-ui-runtime")]
const MOBILE_SYSTEM_NAV_MAX_HORIZONTAL_DRIFT_PX: u16 = 192;

/// Captures every contact that begins in the trusted top chrome.
///
/// Interactive-0 intentionally exposes no client callback for this region:
/// status pixels and their complete pointer stream have the same
/// SurfaceServer owner. A future notification-shade service must be added as
/// an authenticated system endpoint rather than forwarding these samples to
/// whichever app happens to be focused.
#[cfg(feature = "androidbox-interactive0")]
fn dispatch_mobile_top_chrome_input(
    server: &mut SurfaceServerRuntime,
    sample: InputSample,
) -> bool {
    if server.system_top_chrome_capture {
        if !server.pointer_pressed
            || server.input_capture.is_some()
            || server.system_nav_capture.is_some()
        {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        if !sample.pressed() {
            server.system_top_chrome_capture = false;
            server.pointer_pressed = false;
        }
        return true;
    }
    if sample.y() >= MOBILE_CONTENT_TOP {
        return false;
    }
    // A contact that began in content keeps its authenticated client capture
    // even if it crosses into chrome; the matching release must reach that
    // client. Only an otherwise idle pointer stream can begin or hover in
    // system chrome.
    if server.pointer_pressed
        || server.input_capture.is_some()
        || server.system_nav_capture.is_some()
    {
        return false;
    }
    if !sample.pressed() {
        return true;
    }
    if server.system_ui.nav_pressed() {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    server.pointer_pressed = true;
    server.system_top_chrome_capture = true;
    true
}

/// Intercepts only a contact that begins in the bottom system-navigation
/// region. App/Launcher touch controllers never receive these samples, so
/// neither client can reinterpret system navigation as app content.
#[cfg(feature = "mobile-ui-runtime")]
fn dispatch_mobile_system_nav_input(
    server: &mut SurfaceServerRuntime,
    sample: InputSample,
) -> bool {
    if let Some(mut capture) = server.system_nav_capture {
        if !server.pointer_pressed || server.input_capture.is_some() {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        let upward_px = capture.start_y.saturating_sub(sample.y());
        let horizontal_px = capture.start_x.abs_diff(sample.x());
        let vertical_px = capture.start_y.abs_diff(sample.y());
        let movement_px = horizontal_px.max(vertical_px);
        let release_visible = point_inside_visible_display(sample.x(), sample.y());
        let upward_dominant = sample.y() <= capture.start_y
            && upward_px >= horizontal_px.saturating_mul(2)
            && horizontal_px <= MOBILE_SYSTEM_NAV_MAX_HORIZONTAL_DRIFT_PX;

        if sample.pressed() {
            if !capture.rejected
                && movement_px >= MOBILE_SYSTEM_NAV_ACTIVATION_PX
                && (!release_visible || !upward_dominant)
            {
                capture.rejected = true;
            }
            let next_reveal_px = if capture.rejected
                || movement_px < MOBILE_SYSTEM_NAV_ACTIVATION_PX
                || !upward_dominant
            {
                0
            } else {
                let bounded = upward_px.min(UI_SYSTEM_UI_NAV_REVEAL_MAX_PX);
                bounded / UI_SYSTEM_UI_NAV_REVEAL_QUANTUM_PX * UI_SYSTEM_UI_NAV_REVEAL_QUANTUM_PX
            };
            server.system_nav_capture = Some(capture);
            if next_reveal_px != server.system_ui.nav_reveal_px() {
                let update = server
                    .system_ui
                    .update_nav(next_reveal_px)
                    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
                publish_ui_system_ui(server, update);
            }
            return true;
        }

        server.system_nav_capture = None;
        server.pointer_pressed = false;
        let starting_mode = server.system_ui.mode();
        let update =
            if starting_mode == UiSystemUiMode::Locked || capture.rejected || !release_visible {
                server.system_ui.cancel_nav()
            } else if movement_px < MOBILE_SYSTEM_NAV_ACTIVATION_PX {
                server.system_ui.finish_nav(UiSystemUiMode::Home)
            } else if !upward_dominant || upward_px < OVERVIEW_GESTURE_COMMIT_PX {
                server.system_ui.cancel_nav()
            } else if upward_px < OVERVIEW_HOME_COMMIT_PX {
                server.system_ui.finish_nav(UiSystemUiMode::Overview)
            } else {
                server.system_ui.finish_nav(UiSystemUiMode::Home)
            }
            .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
        publish_ui_system_ui(server, update);
        if server.active_client == UiClientId::App
            && matches!(
                update.mode(),
                UiSystemUiMode::Home | UiSystemUiMode::Overview
            )
        {
            publish_ui_focus(server, UiClientId::Launcher, None);
        }
        return true;
    }

    if !sample.pressed() {
        // QEMU tablets emit an unpressed absolute-position sample before a
        // button-down. Keep that hover sample system-owned as well; otherwise
        // an app could observe coordinates outside its declared content
        // viewport before the subsequent navigation capture begins.
        return !server.pointer_pressed && sample.y() >= MOBILE_SYSTEM_NAV_CAPTURE_TOP_PX;
    }
    if server.pointer_pressed {
        return false;
    }
    if sample.y() < MOBILE_SYSTEM_NAV_CAPTURE_TOP_PX {
        return false;
    }
    if server.input_capture.is_some() || server.system_ui.nav_pressed() {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    server.pointer_pressed = true;
    server.system_nav_capture = Some(MobileSystemNavCapture {
        start_x: sample.x(),
        start_y: sample.y(),
        rejected: false,
    });
    let update = server
        .system_ui
        .begin_nav()
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    publish_ui_system_ui(server, update);
    true
}

fn read_surface_input_sample(server: &SurfaceServerRuntime) -> Option<InputSample> {
    let result = syscall(
        SyscallNumber::SurfaceReadInput,
        server.capability.raw(),
        0,
        0,
    );
    if result.status == Status::ShouldWait.raw() {
        return None;
    }
    if result.status != Status::Ok.raw() {
        fail(FAIL_SURFACE_INPUT);
    }
    Some(
        InputSample::decode_registers(result.out1, result.out2)
            .unwrap_or_else(|_| fail(FAIL_SURFACE_INPUT)),
    )
}

#[cfg(feature = "mobile-ui-runtime")]
fn drain_compatible_verification_input_after_reservation(server: &mut SurfaceServerRuntime) {
    if !server
        .compatible_verification_input_barrier
        .begin_completion_drain()
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut drained = 0usize;
    while drained < COMPATIBLE_VERIFICATION_INPUT_DRAIN_LIMIT {
        let Some(sample) = read_surface_input_sample(server) else {
            break;
        };
        if !server
            .compatible_verification_input_barrier
            .drop_sample(sample.pressed())
        {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        drained += 1;
    }
    if !server
        .compatible_verification_input_barrier
        .finish_completion_drain(drained == COMPATIBLE_VERIFICATION_INPUT_DRAIN_LIMIT)
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
}

fn dispatch_surface_input(server: &mut SurfaceServerRuntime) {
    let Some(sample) = read_surface_input_sample(server) else {
        return;
    };
    #[cfg(feature = "mobile-ui-runtime")]
    if server.system_ui.has_compatible_activity_reservation() {
        if server.pointer_pressed
            || server.input_capture.is_some()
            || server.system_nav_capture.is_some()
            || {
                #[cfg(feature = "androidbox-interactive0")]
                {
                    server.system_top_chrome_capture
                }
                #[cfg(not(feature = "androidbox-interactive0"))]
                {
                    false
                }
            }
        {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        if !server
            .compatible_verification_input_barrier
            .drop_sample(sample.pressed())
        {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        return;
    }
    #[cfg(feature = "mobile-ui-runtime")]
    if server
        .compatible_verification_input_barrier
        .drop_sample(sample.pressed())
    {
        return;
    }
    #[cfg(feature = "androidbox-interactive0")]
    if dispatch_mobile_top_chrome_input(server, sample) {
        return;
    }
    #[cfg(feature = "mobile-ui-runtime")]
    if dispatch_mobile_system_nav_input(server, sample) {
        return;
    }
    let target = match (server.pointer_pressed, sample.pressed()) {
        (false, false) => server.active_client,
        (false, true) => {
            #[cfg(feature = "mobile-ui-runtime")]
            let target = server.active_client;
            #[cfg(not(feature = "mobile-ui-runtime"))]
            let target = if HOME_TARGET.contains(sample.x(), sample.y()) {
                UiClientId::Launcher
            } else {
                server.active_client
            };
            server.input_capture = Some(target);
            server.pointer_pressed = true;
            target
        }
        (true, true) => server
            .input_capture
            .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL)),
        (true, false) => {
            let target = server
                .input_capture
                .take()
                .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
            server.pointer_pressed = false;
            target
        }
    };
    let (channel, sequence) = match target {
        UiClientId::Launcher => {
            let sequence = server
                .launcher_input_sequence
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
            server.launcher_input_sequence = sequence;
            (server.launcher_channel.raw(), sequence)
        }
        UiClientId::App => {
            let sequence = server
                .app_input_sequence
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
            server.app_input_sequence = sequence;
            (server.app_channel.raw(), sequence)
        }
    };
    let routed = InputSample::try_new(sequence, sample.x(), sample.y(), sample.pressed())
        .unwrap_or_else(|_| fail(FAIL_SURFACE_INPUT));
    let event = UiServerEvent::input_sample(server.session_id, routed)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_INPUT));
    write_ui_event(channel, event);
}

fn dispatch_ui_client_message(server: &mut SurfaceServerRuntime, client: UiClientId) {
    let channel = match client {
        UiClientId::Launcher => server.launcher_channel.raw(),
        UiClientId::App => server.app_channel.raw(),
    };
    let (sender_pid, message) = read_ui_client_message_now(channel);
    let bound_pid = match client {
        UiClientId::Launcher => &mut server.launcher_pid,
        UiClientId::App => &mut server.app_pid,
    };
    if sender_pid == 0 || bound_pid.is_some_and(|expected| expected != sender_pid) {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    *bound_pid = Some(sender_pid);

    match (client, message) {
        (UiClientId::Launcher, UiClientMessage::Control(control)) => {
            dispatch_client_control(server, UiClientId::Launcher, control)
        }
        (UiClientId::Launcher, UiClientMessage::Present(frame)) => {
            dispatch_client_present(server, UiClientId::Launcher, frame)
        }
        (UiClientId::App, UiClientMessage::Present(frame)) => {
            let _ = frame;
            fail(FAIL_SURFACE_PROTOCOL)
        }
        (UiClientId::App, UiClientMessage::BufferPresent { frame, buffer }) => {
            #[cfg(feature = "ui-stale-present-evidence")]
            if hold_ui_stale_present(server, frame, buffer.is_some()) {
                return;
            }
            dispatch_client_buffer_present(server, UiClientId::App, frame, buffer)
        }
        (UiClientId::Launcher, UiClientMessage::BufferPresent { frame, buffer }) => {
            #[cfg(feature = "mobile-ui-runtime")]
            dispatch_client_buffer_present(server, UiClientId::Launcher, frame, buffer);
            #[cfg(not(feature = "mobile-ui-runtime"))]
            {
                let _ = (frame, buffer);
                fail(FAIL_SURFACE_PROTOCOL)
            }
        }
        (UiClientId::App, UiClientMessage::Control(control)) => {
            #[cfg(feature = "mobile-ui-runtime")]
            dispatch_client_control(server, UiClientId::App, control);
            #[cfg(not(feature = "mobile-ui-runtime"))]
            {
                let _ = control;
                fail(FAIL_SURFACE_PROTOCOL)
            }
        }
    }
}

fn dispatch_client_control(
    server: &mut SurfaceServerRuntime,
    client: UiClientId,
    control: UiClientControl,
) {
    match control.payload() {
        UiClientControlPayload::SetFocus { .. } => dispatch_focus_control(server, client, control),
        UiClientControlPayload::UpdateAppearance { action, request_id } => {
            #[cfg(feature = "mobile-ui-runtime")]
            {
                let update = server
                    .appearance
                    .apply(client, action, request_id)
                    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
                publish_ui_appearance(server, update.appearance(), update.revision());
            }
            #[cfg(not(feature = "mobile-ui-runtime"))]
            {
                let _ = (client, action, request_id);
                fail(FAIL_SURFACE_PROTOCOL);
            }
        }
        UiClientControlPayload::DismissBootNotification { request_id } => {
            #[cfg(feature = "mobile-ui-runtime")]
            {
                let update = server
                    .boot_notification
                    .dismiss(client, request_id)
                    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
                publish_ui_boot_notification(server, update.visible(), update.revision());
            }
            #[cfg(not(feature = "mobile-ui-runtime"))]
            {
                let _ = (client, request_id);
                fail(FAIL_SURFACE_PROTOCOL);
            }
        }
        UiClientControlPayload::UpdateSystemUi {
            action,
            recent,
            request_id,
            observed_revision,
        } => {
            #[cfg(feature = "mobile-ui-runtime")]
            {
                if client != UiClientId::Launcher
                    || server.active_client != UiClientId::Launcher
                    || server.active_app.is_some()
                {
                    fail(FAIL_SURFACE_PROTOCOL);
                }
                let compatible_identity = recent.and_then(UiRecentIdentity::compatible_android);
                let reservation_before = server.system_ui.compatible_activity_reservation();
                let requested_origin = if action == UiSystemUiAction::ReserveCompatibleActivity {
                    compatible_reservation_origin_for_mode(server.system_ui.mode())
                } else {
                    reservation_before.map(|reservation| reservation.origin())
                };
                if action == UiSystemUiAction::ReserveCompatibleActivity
                    && (server.pointer_pressed
                        || server.input_capture.is_some()
                        || server.system_nav_capture.is_some()
                        || server
                            .compatible_verification_input_barrier
                            .blocks_reservation())
                {
                    publish_ui_system_ui_request_completion(
                        server,
                        action,
                        UiSystemUiRequestStatus::CaptureBusy,
                        request_id,
                        server.system_ui.revision(),
                        compatible_identity,
                        requested_origin,
                    );
                    return;
                }
                let update = match server.system_ui.apply_client_recent_action(
                    client,
                    action,
                    recent,
                    request_id,
                    observed_revision,
                ) {
                    Ok(update) => update,
                    Err(
                        UiSystemUiSessionError::ObservedRevisionMismatch
                        | UiSystemUiSessionError::NavigationInProgress,
                    ) => {
                        publish_ui_system_ui_request_completion(
                            server,
                            action,
                            UiSystemUiRequestStatus::Conflict,
                            request_id,
                            server.system_ui.revision(),
                            compatible_identity,
                            requested_origin,
                        );
                        return;
                    }
                    Err(_) => fail(FAIL_SURFACE_PROTOCOL),
                };
                match (
                    reservation_before,
                    server.system_ui.compatible_activity_reservation(),
                ) {
                    (None, Some(_)) => {
                        if !server
                            .compatible_verification_input_barrier
                            .begin_reservation()
                        {
                            fail(FAIL_SURFACE_PROTOCOL);
                        }
                    }
                    (Some(_), None) => {
                        // Session commit/abort already established the semantic
                        // epoch. Drain samples that won the wait race before
                        // publishing it. The fixed bound prevents an input
                        // producer from starving the terminal completion.
                        drain_compatible_verification_input_after_reservation(server);
                    }
                    (None, None) => {}
                    (Some(_), Some(_)) => fail(FAIL_SURFACE_PROTOCOL),
                }
                publish_ui_system_ui(server, update);
                let completion_origin = if action == UiSystemUiAction::ReserveCompatibleActivity {
                    server
                        .system_ui
                        .compatible_activity_reservation()
                        .map(|reservation| reservation.origin())
                } else {
                    requested_origin
                };
                publish_ui_system_ui_request_completion(
                    server,
                    action,
                    UiSystemUiRequestStatus::Accepted,
                    request_id,
                    update.revision(),
                    compatible_identity,
                    completion_origin,
                );
                #[cfg(feature = "androidbox-el0-runtime0")]
                if action == UiSystemUiAction::PresentCompatibleActivity {
                    let identity =
                        compatible_identity.unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                    if server.active_client != UiClientId::Launcher
                        || server.active_app.is_some()
                        || update.mode() != UiSystemUiMode::Foreground
                        || update.recent() != Some(UiRecentIdentity::CompatibleAndroid(identity))
                        || update.nav_pressed()
                        || update.nav_reveal_px() != 0
                        || server.system_ui.has_compatible_activity_reservation()
                    {
                        fail(FAIL_SURFACE_PROTOCOL);
                    }
                    // The shared Foreground epoch and its Launcher-only
                    // completion are queued first. The App then observes an
                    // authenticated compatible focus with no ShellAppId.
                    publish_ui_focus(server, UiClientId::App, None);
                }
                if action == UiSystemUiAction::ActivateRecent {
                    let recent = recent.unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                    match recent {
                        UiRecentIdentity::Shell(app) => {
                            if server.active_client != UiClientId::Launcher
                                || server.active_app.is_some()
                                || update.mode() != UiSystemUiMode::Foreground
                            {
                                fail(FAIL_SURFACE_PROTOCOL);
                            }
                            publish_ui_focus(server, UiClientId::App, Some(app));
                        }
                        UiRecentIdentity::CompatibleAndroid(_) => {
                            if server.active_client != UiClientId::Launcher
                                || server.active_app.is_some()
                                || update.mode() != UiSystemUiMode::Foreground
                            {
                                fail(FAIL_SURFACE_PROTOCOL);
                            }
                            #[cfg(feature = "androidbox-el0-runtime0")]
                            {
                                if update.recent() != Some(recent)
                                    || update.nav_pressed()
                                    || update.nav_reveal_px() != 0
                                    || server.system_ui.has_compatible_activity_reservation()
                                {
                                    fail(FAIL_SURFACE_PROTOCOL);
                                }
                                publish_ui_focus(server, UiClientId::App, None);
                            }
                        }
                    }
                }
            }
            #[cfg(not(feature = "mobile-ui-runtime"))]
            {
                let _ = (client, action, recent, request_id, observed_revision);
                fail(FAIL_SURFACE_PROTOCOL);
            }
        }
        UiClientControlPayload::ReportAndroidBoxExecution { report } => {
            #[cfg(feature = "mobile-ui-runtime")]
            {
                if client != UiClientId::Launcher
                    || server.active_client != UiClientId::Launcher
                    || server.active_app.is_some()
                    || server.system_ui.mode() != UiSystemUiMode::Home
                {
                    fail(FAIL_SURFACE_PROTOCOL);
                }
                server
                    .androidbox_audit
                    .accept(client, report)
                    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
            }
            #[cfg(not(feature = "mobile-ui-runtime"))]
            {
                let _ = (client, report);
                fail(FAIL_SURFACE_PROTOCOL);
            }
        }
        UiClientControlPayload::ReportAndroidBoxActivityExecution { report } => {
            #[cfg(feature = "mobile-ui-runtime")]
            {
                if client != UiClientId::Launcher
                    || server.active_client != UiClientId::Launcher
                    || server.active_app.is_some()
                    || server.system_ui.mode() != UiSystemUiMode::Home
                {
                    fail(FAIL_SURFACE_PROTOCOL);
                }
                let dex_audit = server.androidbox_audit;
                server
                    .androidbox_activity_audit
                    .accept(client, &dex_audit, report)
                    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
            }
            #[cfg(not(feature = "mobile-ui-runtime"))]
            {
                let _ = (client, report);
                fail(FAIL_SURFACE_PROTOCOL);
            }
        }
        UiClientControlPayload::ReportAndroidBoxResourceExecution { report } => {
            #[cfg(feature = "mobile-ui-runtime")]
            {
                if client != UiClientId::Launcher
                    || server.active_client != UiClientId::Launcher
                    || server.active_app.is_some()
                    || server.system_ui.mode() != UiSystemUiMode::Home
                {
                    fail(FAIL_SURFACE_PROTOCOL);
                }
                let dex_audit = server.androidbox_audit;
                let activity_audit = server.androidbox_activity_audit;
                server
                    .androidbox_resource_audit
                    .accept(client, &dex_audit, &activity_audit, report)
                    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
            }
            #[cfg(not(feature = "mobile-ui-runtime"))]
            {
                let _ = (client, report);
                fail(FAIL_SURFACE_PROTOCOL);
            }
        }
        UiClientControlPayload::AttachAppEndpoint { .. } => fail(FAIL_SURFACE_PROTOCOL),
    }
}

fn dispatch_focus_control(
    server: &mut SurfaceServerRuntime,
    requester: UiClientId,
    control: UiClientControl,
) {
    let UiClientControlPayload::SetFocus { target, app, .. } = control.payload() else {
        fail(FAIL_SURFACE_PROTOCOL);
    };
    // Channel binding authenticates `requester`; only the current foreground
    // client may hand focus to the opposite side. A background peer cannot
    // synthesize a valid-looking edge and steal or revoke focus.
    let valid_edge = requester == server.active_client
        && matches!(
            (server.active_client, target, app),
            (UiClientId::Launcher, UiClientId::App, Some(_))
                | (UiClientId::App, UiClientId::Launcher, None)
        );
    if !valid_edge {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut next_tracker = server.focus_controls;
    if next_tracker.accept(control).is_err() {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    server.focus_controls = next_tracker;
    #[cfg(feature = "mobile-ui-runtime")]
    {
        match (server.active_client, target, app) {
            #[cfg(feature = "androidbox-el0-runtime0")]
            (UiClientId::App, UiClientId::Launcher, None)
                if server.active_app.is_none()
                    && compatible_app_surface_is_presentable(server.system_ui) =>
            {
                let (update, identity, request_id) =
                    finish_compatible_activity_from_app(&mut server.system_ui)
                        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                publish_ui_system_ui(server, update);
                // This server-internal completion advances the same contiguous
                // Launcher request epoch used by normal SystemUI controls. It
                // is delivered only to Launcher and cannot grant App control
                // over later SystemUI mutations.
                publish_ui_system_ui_request_completion(
                    server,
                    UiSystemUiAction::FinishCompatibleActivity,
                    UiSystemUiRequestStatus::Accepted,
                    request_id,
                    update.revision(),
                    Some(identity),
                    None,
                );
            }
            (UiClientId::Launcher, UiClientId::App, Some(_))
                if server.system_ui.mode() == UiSystemUiMode::Home
                    && !server.system_ui.nav_pressed() => {}
            (UiClientId::App, UiClientId::Launcher, None)
                if server.system_ui.mode() == UiSystemUiMode::Foreground
                    && !server.system_ui.nav_pressed() =>
            {
                let update = server
                    .system_ui
                    .focus_home()
                    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
                publish_ui_system_ui(server, update);
            }
            _ => fail(FAIL_SURFACE_PROTOCOL),
        }
    }
    #[cfg(feature = "ui-stale-present-evidence")]
    let release_stale_present = should_release_ui_stale_present(server, target, app);
    publish_ui_focus(server, target, app);
    #[cfg(feature = "ui-stale-present-evidence")]
    if release_stale_present {
        release_ui_stale_present(server);
    }
}

#[cfg(feature = "ui-stale-present-evidence")]
fn hold_ui_stale_present(
    server: &mut SurfaceServerRuntime,
    frame: BufferPresent,
    has_transferred_buffer: bool,
) -> bool {
    match server.stale_present_barrier {
        UiStalePresentBarrier::Awaiting if frame.client_frame_id() < 10 => false,
        UiStalePresentBarrier::Awaiting => {
            if frame.client_frame_id() != 10
                || frame.global_frame_id() != 10
                || frame.focus_generation() != 8
                || frame.buffer_generation() != 751
                || has_transferred_buffer
                || server.active_client != UiClientId::App
                || server.active_app != Some(ShellAppId::Phone)
                || server.focus_generation != 8
                || server.global_frame_id != 13
                || server.launcher_frame_id != Some(4)
                || server.app_frame_id != Some(9)
                || server.app_buffer.is_none()
                || server.focus_controls.last_transition_id() != Some(7)
                || server.focus_controls.active_client() != Some(UiClientId::App)
                || server.focus_controls.active_app() != Some(ShellAppId::Phone)
            {
                fail(FAIL_SURFACE_PROTOCOL);
            }
            server.stale_present_barrier = UiStalePresentBarrier::Held(frame);
            true
        }
        UiStalePresentBarrier::Held(_) => fail(FAIL_SURFACE_PROTOCOL),
        UiStalePresentBarrier::Released => false,
    }
}

#[cfg(feature = "ui-stale-present-evidence")]
fn should_release_ui_stale_present(
    server: &SurfaceServerRuntime,
    target: UiClientId,
    app: Option<ShellAppId>,
) -> bool {
    match server.stale_present_barrier {
        UiStalePresentBarrier::Awaiting => {
            if server.focus_generation >= 8 {
                fail(FAIL_SURFACE_PROTOCOL);
            }
            false
        }
        UiStalePresentBarrier::Held(frame) => {
            if target != UiClientId::Launcher
                || app.is_some()
                || server.active_client != UiClientId::App
                || server.active_app != Some(ShellAppId::Phone)
                || server.focus_generation != 8
                || server.global_frame_id != 13
                || server.launcher_frame_id != Some(4)
                || server.app_frame_id != Some(9)
                || server.focus_controls.last_transition_id() != Some(8)
                || server.focus_controls.active_client() != Some(UiClientId::Launcher)
                || server.focus_controls.active_app().is_some()
                || frame.client_frame_id() != 10
                || frame.global_frame_id() != 10
                || frame.focus_generation() != 8
                || frame.buffer_generation() != 751
            {
                fail(FAIL_SURFACE_PROTOCOL);
            }
            true
        }
        UiStalePresentBarrier::Released => false,
    }
}

#[cfg(feature = "ui-stale-present-evidence")]
fn release_ui_stale_present(server: &mut SurfaceServerRuntime) {
    if server.active_client != UiClientId::Launcher
        || server.active_app.is_some()
        || server.focus_generation != 9
        || server.global_frame_id != 13
        || server.launcher_frame_id != Some(4)
        || server.app_frame_id != Some(9)
        || server.app_buffer.is_none()
        || server.focus_controls.last_transition_id() != Some(8)
        || server.focus_controls.active_client() != Some(UiClientId::Launcher)
        || server.focus_controls.active_app().is_some()
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let frame = match core::mem::replace(
        &mut server.stale_present_barrier,
        UiStalePresentBarrier::Released,
    ) {
        UiStalePresentBarrier::Held(frame) => frame,
        UiStalePresentBarrier::Awaiting | UiStalePresentBarrier::Released => {
            fail(FAIL_SURFACE_PROTOCOL)
        }
    };
    dispatch_client_buffer_present(server, UiClientId::App, frame, None);
    if server.active_client != UiClientId::Launcher
        || server.active_app.is_some()
        || server.focus_generation != 9
        || server.global_frame_id != 13
        || server.launcher_frame_id != Some(4)
        || server.app_frame_id != Some(9)
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
}

fn dispatch_client_present(
    server: &mut SurfaceServerRuntime,
    client: UiClientId,
    frame: PresentFrame,
) {
    let previous_local = match client {
        UiClientId::Launcher => server.launcher_frame_id,
        UiClientId::App => server.app_frame_id,
    };
    let expected_local = previous_local
        .map_or(Some(1), |previous| previous.checked_add(1))
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    if frame.frame_id() != expected_local
        || (previous_local.is_none() && frame.mode() != PresentMode::Full)
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let current_focus_generation =
        u32::try_from(server.focus_generation).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    if server.active_client != client || frame.focus_generation() != current_focus_generation {
        let event = UiServerEvent::present_cancelled(
            server.session_id,
            frame.frame_id(),
            server.focus_generation,
        )
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
        let channel = match client {
            UiClientId::Launcher => server.launcher_channel.raw(),
            UiClientId::App => server.app_channel.raw(),
        };
        write_ui_event(channel, event);
        return;
    }
    let global_frame_id = server
        .global_frame_id
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let global_frame = match frame.mode() {
        PresentMode::Full => {
            PresentFrame::full(global_frame_id, frame.clear_color(), frame.rects())
        }
        PresentMode::Damage => PresentFrame::damage(global_frame_id, frame.rects()),
    }
    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let wire = global_frame.encode();
    let result = syscall(
        SyscallNumber::SurfacePresent,
        server.capability.raw(),
        wire.as_ptr() as u64,
        PRESENT_WIRE_SIZE as u64,
    );
    if result.status != Status::Ok.raw()
        || result.out1 != u64::from(global_frame_id)
        || result.out2 == 0
    {
        fail(FAIL_SURFACE_PRESENT);
    }
    let event = UiServerEvent::presented(server.session_id, frame.frame_id(), result.out2)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let channel = match client {
        UiClientId::Launcher => {
            server.launcher_frame_id = Some(frame.frame_id());
            server.launcher_channel.raw()
        }
        UiClientId::App => {
            server.app_frame_id = Some(frame.frame_id());
            server.app_channel.raw()
        }
    };
    server.global_frame_id = global_frame_id;
    write_ui_event(channel, event);
}

fn dispatch_client_buffer_present(
    server: &mut SurfaceServerRuntime,
    client: UiClientId,
    frame: BufferPresent,
    transferred_buffer: Option<OwnedUserHandle>,
) {
    let buffer_slot = match client {
        UiClientId::Launcher => &mut server.launcher_buffer,
        UiClientId::App => &mut server.app_buffer,
    };
    if let Some(buffer) = transferred_buffer {
        if buffer_slot.is_some() {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        *buffer_slot = Some(buffer);
    }
    let buffer_raw = buffer_slot
        .as_ref()
        .map(OwnedUserHandle::raw)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let previous_local = match client {
        UiClientId::Launcher => server.launcher_frame_id,
        UiClientId::App => server.app_frame_id,
    };
    let expected_local = previous_local
        .map_or(Some(1), |previous| previous.checked_add(1))
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    if frame.client_frame_id() != expected_local
        || frame.global_frame_id() != frame.client_frame_id()
        || {
            #[cfg(feature = "androidbox-interactive0")]
            {
                frame.system_chrome_generation() != 0
            }
            #[cfg(not(feature = "androidbox-interactive0"))]
            {
                false
            }
        }
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let current_focus_generation =
        u32::try_from(server.focus_generation).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    #[cfg(feature = "mobile-ui-runtime")]
    let system_ui_revision_matches =
        mobile_buffer_present_revision_is_current(frame, server.system_ui.revision());
    #[cfg(not(feature = "mobile-ui-runtime"))]
    let system_ui_revision_matches = true;
    // This check intentionally precedes SurfacePresentBuffer and every frame
    // counter update. A Home/Overview state change therefore cancels a queued
    // Activity raster rendered from an older System UI snapshot, leaving the
    // same client-local frame id available for a current-state retry.
    if server.active_client != client
        || frame.focus_generation() != current_focus_generation
        || !system_ui_revision_matches
    {
        let event = UiServerEvent::present_cancelled(
            server.session_id,
            frame.client_frame_id(),
            server.focus_generation,
        )
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
        let channel = match client {
            UiClientId::Launcher => server.launcher_channel.raw(),
            UiClientId::App => server.app_channel.raw(),
        };
        write_ui_event(channel, event);
        return;
    }
    #[cfg(feature = "androidbox-el0-runtime0")]
    if client == UiClientId::App
        && server.active_app.is_none()
        && !compatible_app_surface_is_presentable(server.system_ui)
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let global_frame_id = server
        .global_frame_id
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let global = frame
        .with_frame_id(frame.client_frame_id(), global_frame_id)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    #[cfg(feature = "androidbox-interactive0")]
    let global = global
        .with_system_chrome_generation(write_mobile_system_chrome(server))
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let wire = global.encode();
    #[cfg(feature = "androidbox-interactive0")]
    let result = {
        let content = HandleValue::from_raw(
            u32::try_from(buffer_raw).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL)),
        );
        let chrome = HandleValue::from_raw(
            u32::try_from(server.system_chrome_buffer_for_present.raw())
                .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL)),
        );
        syscall(
            SyscallNumber::SurfacePresentBufferLayers,
            server.capability.raw(),
            pack_surface_layer_handles(content, chrome),
            wire.as_ptr() as u64,
        )
    };
    #[cfg(not(feature = "androidbox-interactive0"))]
    let result = syscall(
        SyscallNumber::SurfacePresentBuffer,
        server.capability.raw(),
        buffer_raw,
        wire.as_ptr() as u64,
    );
    if result.status != Status::Ok.raw()
        || result.out1 != u64::from(global_frame_id)
        || result.out2 == 0
    {
        fail(FAIL_SURFACE_PRESENT);
    }
    #[cfg(feature = "mobile-ui-runtime")]
    if client == UiClientId::App {
        match server.active_app {
            Some(app) => match (server.system_ui.mode(), server.system_ui.recent_app()) {
                (UiSystemUiMode::Home, _) => {
                    let update = server
                        .system_ui
                        .focus_app(app)
                        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
                    publish_ui_system_ui(server, update);
                }
                (UiSystemUiMode::Foreground, Some(recent)) if recent == app => {}
                _ => fail(FAIL_SURFACE_PROTOCOL),
            },
            #[cfg(feature = "androidbox-el0-runtime0")]
            None if compatible_app_surface_is_presentable(server.system_ui) => {}
            None => fail(FAIL_SURFACE_PROTOCOL),
        }
    }
    let event = UiServerEvent::presented(server.session_id, frame.client_frame_id(), result.out2)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let channel = match client {
        UiClientId::Launcher => {
            server.launcher_frame_id = Some(frame.client_frame_id());
            server.launcher_channel.raw()
        }
        UiClientId::App => {
            server.app_frame_id = Some(frame.client_frame_id());
            server.app_channel.raw()
        }
    };
    server.global_frame_id = global_frame_id;
    write_ui_event(channel, event);
}

#[cfg(feature = "mobile-ui-runtime")]
fn mobile_buffer_present_revision_is_current(
    frame: BufferPresent,
    current_system_ui_revision: u64,
) -> bool {
    current_system_ui_revision != 0 && frame.system_ui_revision() == current_system_ui_revision
}

#[cfg(feature = "mobile-ui-runtime")]
fn advance_mobile_transition_input_quarantine(
    quarantined_inputs: u64,
    pressed: bool,
) -> Option<(u64, bool)> {
    Some((quarantined_inputs.checked_add(1)?, pressed))
}

#[cfg(feature = "mobile-ui-runtime")]
const _: [(); GRAPHICS_BUFFER_WIDTH as usize] = [(); MOBILE_WIDTH];
#[cfg(feature = "mobile-ui-runtime")]
const _: [(); GRAPHICS_BUFFER_HEIGHT as usize] = [(); MOBILE_HEIGHT];
#[cfg(feature = "mobile-ui-runtime")]
const _: [(); GRAPHICS_BUFFER_LOGICAL_BYTES] =
    [(); MOBILE_WIDTH * MOBILE_HEIGHT * core::mem::size_of::<u32>()];

#[cfg(feature = "mobile-ui-runtime")]
const MOBILE_SHELL_X: u16 = 0;
#[cfg(feature = "mobile-ui-runtime")]
const MOBILE_SHELL_Y: u16 = 0;

#[cfg(feature = "mobile-ui-runtime")]
fn create_mobile_graphics_runtime(
    channel: OwnedUserHandle,
    server_pid: u64,
    events: UiServerEventTracker,
) -> AppRuntime {
    let created = syscall(
        SyscallNumber::GraphicsBufferCreate,
        GRAPHICS_BUFFER_FORMAT_XRGB8888,
        pack_graphics_buffer_geometry(GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_HEIGHT),
        GRAPHICS_BUFFER_CREATE_FLAGS_NONE,
    );
    if created.status != Status::Ok.raw()
        || created.out1 == 0
        || created.out2 != GRAPHICS_BUFFER_LOGICAL_BYTES as u64
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let buffer = OwnedUserHandle::new(created.out1).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let duplicated = syscall(
        SyscallNumber::HandleDuplicate,
        buffer.raw(),
        u64::from(Rights::GRAPHICS_BUFFER_SERVER.bits()),
        0,
    );
    if duplicated.status != Status::Ok.raw() || duplicated.out1 == 0 || duplicated.out2 != 0 {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let buffer_for_server =
        OwnedUserHandle::new(duplicated.out1).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let delivered_active_client = events
        .active_client()
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let delivered_focus_generation = events
        .last_focus_generation()
        .filter(|generation| *generation != 0)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let delivered_appearance = events
        .appearance()
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let delivered_appearance_revision = events
        .last_appearance_revision()
        .filter(|revision| *revision != 0)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let delivered_clock_unix_seconds = events
        .clock_unix_seconds()
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let delivered_clock_revision = events
        .last_clock_revision()
        .filter(|revision| *revision != 0)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let delivered_boot_notification_visible = events
        .boot_notification_visible()
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let delivered_boot_notification_revision = events
        .last_boot_notification_revision()
        .filter(|revision| *revision != 0)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let delivered_system_ui_mode = events
        .system_ui_mode()
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let delivered_system_ui_recent = events.recent();
    let delivered_system_nav_pressed = events
        .nav_pressed()
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let delivered_system_nav_reveal_px = events
        .nav_reveal_px()
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let delivered_system_ui_revision = events
        .last_system_ui_revision()
        .filter(|revision| *revision != 0)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    AppRuntime {
        channel,
        server_pid,
        events,
        buffer,
        buffer_for_server: Some(buffer_for_server),
        buffer_generation: 0,
        deferred_payloads: MobileDeferredPayloadQueue::new(),
        delivered_active_client,
        delivered_focus_generation,
        delivered_appearance,
        delivered_appearance_revision,
        delivered_boot_notification_visible,
        delivered_boot_notification_revision,
        delivered_system_ui_mode,
        delivered_system_ui_recent,
        delivered_system_nav_pressed,
        delivered_system_nav_reveal_px,
        delivered_system_ui_revision,
        delivered_clock_unix_seconds,
        delivered_clock_revision,
        appearance_request_id: 0,
        boot_notification_request_id: 0,
        system_ui_request_id: 0,
        system_ui_request_outstanding: None,
        suppress_mobile_input_until_release: false,
        quarantined_mobile_inputs: 0,
    }
}

#[cfg(feature = "mobile-ui-runtime")]
fn write_mobile_buffer(runtime: &mut AppRuntime, model: MobileModel) {
    let pixels = unsafe { &mut *MOBILE_TRANSFER_PIXELS_BUFFER.0.get() };
    #[cfg(feature = "androidbox-interactive0")]
    let (mut first_row, last_row) = (
        usize::from(MOBILE_CONTENT_TOP),
        usize::from(MOBILE_CONTENT_BOTTOM),
    );
    #[cfg(not(feature = "androidbox-interactive0"))]
    let mut first_row = 0_usize;
    #[cfg(not(feature = "androidbox-interactive0"))]
    let last_row = MOBILE_HEIGHT;
    while first_row < last_row {
        let row_count = (last_row - first_row).min(MOBILE_TRANSFER_ROWS);
        let pixel_count = row_count * MOBILE_WIDTH;
        render_mobile_rows(&mut pixels[..pixel_count], first_row, model)
            .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
        let byte_count = pixel_count * core::mem::size_of::<u32>();
        let bytes =
            unsafe { core::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), byte_count) };
        let offset = first_row * MOBILE_ROW_BYTES;
        if bytes.is_empty() || !bytes.len().is_multiple_of(4) || !offset.is_multiple_of(4) {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        let written = syscall(
            SyscallNumber::GraphicsBufferWrite,
            runtime.buffer.raw(),
            bytes.as_ptr() as u64,
            pack_graphics_buffer_write(offset as u32, byte_count as u32),
        );
        let expected_generation = runtime
            .buffer_generation
            .checked_add(1)
            .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
        if written.status != Status::Ok.raw()
            || written.out1 != byte_count as u64
            || written.out2 != expected_generation
        {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        runtime.buffer_generation = written.out2;
        first_row += row_count;
    }
}

#[cfg(feature = "androidbox-interactive0")]
fn write_mobile_system_chrome(server: &mut SurfaceServerRuntime) -> u64 {
    let appearance = server.appearance.appearance();
    let state = MobileSystemChromeState::new(
        MobileTimeSnapshot::from_unix_seconds(server.clock.unix_seconds),
        appearance.dark_theme(),
        appearance.alternate_accent(),
        appearance.software_dimming(),
        server.system_ui.nav_pressed(),
    );
    let pixels = unsafe { &mut *MOBILE_TRANSFER_PIXELS_BUFFER.0.get() };
    for (region_start, region_end) in [
        (0_usize, usize::from(MOBILE_CONTENT_TOP)),
        (usize::from(MOBILE_CONTENT_BOTTOM), MOBILE_HEIGHT),
    ] {
        let mut first_row = region_start;
        while first_row < region_end {
            let row_count = (region_end - first_row).min(MOBILE_TRANSFER_ROWS);
            let pixel_count = row_count * MOBILE_WIDTH;
            render_system_chrome_rows(&mut pixels[..pixel_count], first_row, state)
                .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
            let byte_count = pixel_count * core::mem::size_of::<u32>();
            let bytes =
                unsafe { core::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), byte_count) };
            let offset = first_row * MOBILE_ROW_BYTES;
            let written = syscall(
                SyscallNumber::GraphicsBufferWrite,
                server.system_chrome_buffer.raw(),
                bytes.as_ptr() as u64,
                pack_graphics_buffer_write(offset as u32, byte_count as u32),
            );
            let expected_generation = server
                .system_chrome_generation
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
            if written.status != Status::Ok.raw()
                || written.out1 != byte_count as u64
                || written.out2 != expected_generation
            {
                fail(FAIL_SURFACE_PROTOCOL);
            }
            server.system_chrome_generation = written.out2;
            first_row += row_count;
        }
    }
    server.system_chrome_generation
}

#[cfg(feature = "mobile-ui-runtime")]
fn accept_mobile_server_event(
    runtime: &mut AppRuntime,
    sender_pid: u64,
    event: UiServerEvent,
) -> UiServerEventPayload {
    if sender_pid != runtime.server_pid || runtime.events.accept(event).is_err() {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    event.payload()
}

#[cfg(feature = "mobile-ui-runtime")]
fn next_mobile_server_payload(runtime: &mut AppRuntime) -> UiServerEventPayload {
    // Preserve server order across a synchronous present: accepted payloads
    // always run before reading any later channel event.
    let deferred = match runtime.deferred_payloads.pop() {
        Ok(payload) => payload,
        Err(_) => fail(FAIL_SURFACE_PROTOCOL),
    };
    if let Some(payload) = deferred {
        deliver_mobile_payload_state(runtime, payload);
        return payload;
    }
    let (sender_pid, event) = read_ui_event(runtime.channel.raw());
    let payload = accept_mobile_server_event(runtime, sender_pid, event);
    deliver_mobile_payload_state(runtime, payload);
    payload
}

#[cfg(feature = "mobile-ui-runtime")]
fn deliver_mobile_payload_state(runtime: &mut AppRuntime, payload: UiServerEventPayload) {
    match payload {
        UiServerEventPayload::FocusChanged {
            active_client,
            focus_generation,
            ..
        } => {
            runtime.delivered_active_client = active_client;
            runtime.delivered_focus_generation = focus_generation;
        }
        UiServerEventPayload::AppearanceChanged {
            appearance,
            revision,
        } => {
            runtime.delivered_appearance = appearance;
            runtime.delivered_appearance_revision = revision;
        }
        UiServerEventPayload::ClockChanged {
            unix_seconds,
            revision,
        } => {
            runtime.delivered_clock_unix_seconds = unix_seconds;
            runtime.delivered_clock_revision = revision;
        }
        UiServerEventPayload::BootNotificationChanged { visible, revision } => {
            runtime.delivered_boot_notification_visible = visible;
            runtime.delivered_boot_notification_revision = revision;
        }
        UiServerEventPayload::SystemUiChanged {
            mode,
            recent,
            nav_pressed,
            nav_reveal_px,
            revision,
        } => {
            runtime.delivered_system_ui_mode = mode;
            runtime.delivered_system_ui_recent = recent;
            runtime.delivered_system_nav_pressed = nav_pressed;
            runtime.delivered_system_nav_reveal_px = nav_reveal_px;
            runtime.delivered_system_ui_revision = revision;
        }
        UiServerEventPayload::Ready
        | UiServerEventPayload::Input(_)
        | UiServerEventPayload::Presented { .. }
        | UiServerEventPayload::Degraded { .. }
        | UiServerEventPayload::SystemUiRequestCompleted { .. }
        | UiServerEventPayload::PresentCancelled { .. } => {}
    }
}

#[cfg(feature = "mobile-ui-runtime")]
fn present_mobile_page(
    runtime: &mut AppRuntime,
    client: UiClientId,
    model: MobileModel,
) -> MobilePresentOutcome {
    present_mobile_page_with_input_policy(runtime, client, model, MobileInputPolicy::Defer)
}

#[cfg(feature = "mobile-ui-runtime")]
fn mobile_system_ui_delivery_is_current(runtime: &AppRuntime) -> bool {
    tracked_mobile_system_ui_epoch(&runtime.events)
        == Some(MobileSystemUiEpoch {
            mode: runtime.delivered_system_ui_mode,
            recent: runtime.delivered_system_ui_recent,
            nav_pressed: runtime.delivered_system_nav_pressed,
            nav_reveal_px: runtime.delivered_system_nav_reveal_px,
            revision: runtime.delivered_system_ui_revision,
        })
}

#[cfg(feature = "mobile-ui-runtime")]
fn present_mobile_page_with_input_policy(
    runtime: &mut AppRuntime,
    client: UiClientId,
    model: MobileModel,
    input_policy: MobileInputPolicy,
) -> MobilePresentOutcome {
    let tracked_focus_generation = runtime
        .events
        .last_focus_generation()
        .filter(|generation| *generation != 0)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    if runtime.delivered_active_client != client
        || runtime.events.active_client() != Some(client)
        || runtime.delivered_focus_generation != tracked_focus_generation
        || runtime.events.appearance() != Some(runtime.delivered_appearance)
        || runtime.events.last_appearance_revision() != Some(runtime.delivered_appearance_revision)
        || runtime.events.boot_notification_visible()
            != Some(runtime.delivered_boot_notification_visible)
        || runtime.events.last_boot_notification_revision()
            != Some(runtime.delivered_boot_notification_revision)
        || runtime.events.clock_unix_seconds() != Some(runtime.delivered_clock_unix_seconds)
        || runtime.events.last_clock_revision() != Some(runtime.delivered_clock_revision)
        || !mobile_system_ui_delivery_is_current(runtime)
    {
        return MobilePresentOutcome::Superseded;
    }
    // Every transition frame is bound to the exact delivered SystemUI epoch.
    // Accepted-but-deferred navigation advances the tracker immediately and
    // therefore supersedes this frame sequence before another transition step.
    let focus_generation =
        u32::try_from(tracked_focus_generation).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let frame_id = next_app_frame_id(runtime);
    if runtime.events.begin_present(frame_id).is_err() {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    write_mobile_buffer(runtime, model);
    let frame = BufferPresent::client_with_system_ui_revision(
        frame_id,
        focus_generation,
        runtime.buffer_generation,
        runtime.delivered_system_ui_revision,
    )
    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let transfer = runtime.buffer_for_server.take();
    write_buffer_present(runtime.channel.raw(), frame, transfer);
    loop {
        let (sender_pid, event) = read_ui_event(runtime.channel.raw());
        let payload = accept_mobile_server_event(runtime, sender_pid, event);
        match payload {
            UiServerEventPayload::Input(_) if input_policy == MobileInputPolicy::Defer => {
                if runtime.deferred_payloads.push(payload).is_err() {
                    fail(FAIL_SURFACE_PROTOCOL);
                }
            }
            UiServerEventPayload::Input(sample) => {
                let (quarantined_inputs, suppress_until_release) =
                    advance_mobile_transition_input_quarantine(
                        runtime.quarantined_mobile_inputs,
                        sample.pressed(),
                    )
                    .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                runtime.quarantined_mobile_inputs = quarantined_inputs;
                // Fence only a contact that is still physically down at this
                // point. If its release is also quarantined, the fence clears
                // here and the destination page's first new tap remains
                // usable.
                runtime.suppress_mobile_input_until_release = suppress_until_release;
            }
            UiServerEventPayload::FocusChanged { .. }
            | UiServerEventPayload::AppearanceChanged { .. }
            | UiServerEventPayload::ClockChanged { .. }
            | UiServerEventPayload::BootNotificationChanged { .. }
            | UiServerEventPayload::SystemUiChanged { .. }
            | UiServerEventPayload::SystemUiRequestCompleted { .. } => {
                if runtime.deferred_payloads.push(payload).is_err() {
                    fail(FAIL_SURFACE_PROTOCOL);
                }
            }
            UiServerEventPayload::Presented {
                frame_id: presented_frame,
                commit: _,
            } if presented_frame == frame_id => {
                if runtime.events.active_client() != Some(client)
                    || runtime.events.last_focus_generation() != Some(u64::from(focus_generation))
                {
                    fail(FAIL_SURFACE_PROTOCOL);
                }
                if !mobile_system_ui_delivery_is_current(runtime) {
                    return MobilePresentOutcome::Superseded;
                }
                return MobilePresentOutcome::Presented;
            }
            UiServerEventPayload::PresentCancelled {
                frame_id: cancelled_frame,
                focus_generation: _,
            } if cancelled_frame == frame_id => {
                return MobilePresentOutcome::Superseded;
            }
            UiServerEventPayload::Ready | UiServerEventPayload::Degraded { .. } => {
                fail(FAIL_SURFACE_PROTOCOL)
            }
            UiServerEventPayload::Presented { .. }
            | UiServerEventPayload::PresentCancelled { .. } => fail(FAIL_SURFACE_PROTOCOL),
        }
    }
}

#[cfg(feature = "mobile-ui-runtime")]
fn mobile_local_coordinates(sample: InputSample) -> Option<(u16, u16)> {
    let x = sample.x().checked_sub(MOBILE_SHELL_X)?;
    let y = sample.y().checked_sub(MOBILE_SHELL_Y)?;
    (usize::from(x) < MOBILE_WIDTH && usize::from(y) < MOBILE_HEIGHT).then_some((x, y))
}

#[cfg(feature = "mobile-ui-runtime")]
fn observe_mobile_sample(
    touch: &mut TouchController,
    model: MobileModel,
    sample: InputSample,
) -> Option<MobileAction> {
    if let Some((x, y)) = mobile_local_coordinates(sample) {
        touch.observe(model, x, y, sample.pressed())
    } else {
        // Leaving the surface is an explicit cancellation. Synthetic extreme
        // coordinates could otherwise look like a maximum-distance gesture.
        touch.cancel_contact(model)
    }
}

#[cfg(feature = "mobile-ui-runtime")]
fn apply_mobile_appearance(model: &mut MobileModel, appearance: UiAppearance) -> bool {
    let changed = model.dark_theme != appearance.dark_theme()
        || model.alternate_accent != appearance.alternate_accent()
        || model.software_dimming != appearance.software_dimming();
    model.dark_theme = appearance.dark_theme();
    model.alternate_accent = appearance.alternate_accent();
    model.software_dimming = appearance.software_dimming();
    changed
}

#[cfg(feature = "mobile-ui-runtime")]
fn apply_mobile_system_ui(model: &mut MobileModel, runtime: &AppRuntime) -> bool {
    model.apply_system_ui_state_recent(
        runtime.delivered_system_ui_mode,
        runtime.delivered_system_ui_recent,
        runtime.delivered_system_nav_pressed,
        runtime.delivered_system_nav_reveal_px,
        runtime.delivered_system_ui_revision,
    )
}

#[cfg(feature = "mobile-ui-runtime")]
fn request_mobile_appearance(runtime: &mut AppRuntime, action: UiAppearanceAction) {
    let request_id = runtime
        .appearance_request_id
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let control = UiClientControl::update_appearance(action, request_id)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    write_ui_control(runtime.channel.raw(), control);
    runtime.appearance_request_id = request_id;
}

#[cfg(feature = "mobile-ui-runtime")]
fn apply_mobile_boot_notification(model: &mut MobileModel, visible: bool) -> bool {
    let clears_transient = !visible
        && (model.boot_notification_expanded
            || model.boot_notification_offset_px != 0
            || model.pressed_target
                == Some(bndr_ui::mobile::MobilePressedTarget::BootNotification));
    let changed = model.boot_notification_visible != visible || clears_transient;
    model.boot_notification_visible = visible;
    if !visible {
        model.boot_notification_expanded = false;
        model.boot_notification_offset_px = 0;
        if model.pressed_target == Some(bndr_ui::mobile::MobilePressedTarget::BootNotification) {
            model.pressed_target = None;
        }
    }
    changed
}

#[cfg(feature = "mobile-ui-runtime")]
fn request_mobile_boot_notification_dismiss(runtime: &mut AppRuntime) {
    let request_id = runtime
        .boot_notification_request_id
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let control = UiClientControl::dismiss_boot_notification(request_id)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    write_ui_control(runtime.channel.raw(), control);
    runtime.boot_notification_request_id = request_id;
}

#[cfg(feature = "mobile-ui-runtime")]
fn request_mobile_system_ui(
    runtime: &mut AppRuntime,
    action: UiSystemUiAction,
    app: Option<ShellAppId>,
) -> MobileSystemUiRequest {
    request_mobile_system_ui_recent(runtime, action, app.map(UiRecentIdentity::Shell))
}

#[cfg(feature = "mobile-ui-runtime")]
fn request_mobile_system_ui_recent(
    runtime: &mut AppRuntime,
    action: UiSystemUiAction,
    recent: Option<UiRecentIdentity>,
) -> MobileSystemUiRequest {
    if runtime.system_ui_request_outstanding.is_some() {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let request_id = runtime
        .system_ui_request_id
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let observed_revision = runtime.delivered_system_ui_revision;
    let control =
        UiClientControl::update_system_ui_recent(action, recent, request_id, observed_revision)
            .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    write_ui_control(runtime.channel.raw(), control);
    let request = MobileSystemUiRequest {
        action,
        recent,
        request_id,
        observed_revision,
    };
    runtime.system_ui_request_outstanding = Some(request);
    request
}

#[cfg(feature = "mobile-ui-runtime")]
const fn software_dimming_action(level: UiSoftwareDimming) -> UiAppearanceAction {
    match level {
        UiSoftwareDimming::Off => UiAppearanceAction::SetSoftwareDimmingOff,
        UiSoftwareDimming::Light => UiAppearanceAction::SetSoftwareDimmingLight,
        UiSoftwareDimming::Medium => UiAppearanceAction::SetSoftwareDimmingMedium,
        UiSoftwareDimming::Strong => UiAppearanceAction::SetSoftwareDimmingStrong,
        UiSoftwareDimming::Maximum => UiAppearanceAction::SetSoftwareDimmingMaximum,
    }
}

#[cfg(feature = "mobile-ui-runtime")]
fn clear_mobile_press_and_present(
    runtime: &mut AppRuntime,
    client: UiClientId,
    model: &mut MobileModel,
) {
    if model.apply(MobileAction::SetPressed(None)) {
        // An earlier authoritative appearance/focus event may already have
        // superseded this visual-only release frame. The semantic input was
        // nevertheless accepted in server order, so a supersede must not
        // discard it; the next authoritative frame also carries no pressed
        // target.
        let _ = present_mobile_page(runtime, client, *model);
    }
}

#[cfg(feature = "mobile-ui-runtime")]
fn present_mobile_enter_transition(
    runtime: &mut AppRuntime,
    client: UiClientId,
    model: &mut MobileModel,
) -> MobilePresentOutcome {
    for offset_px in PAGE_TRANSITION_ENTER_OFFSETS {
        if !model.apply(MobileAction::SetPageTransitionOffset(offset_px)) {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        if present_mobile_page_with_input_policy(
            runtime,
            client,
            *model,
            MobileInputPolicy::Quarantine,
        ) != MobilePresentOutcome::Presented
        {
            let _ = model.apply(MobileAction::SetPageTransitionOffset(0));
            return MobilePresentOutcome::Superseded;
        }
    }
    if !model.apply(MobileAction::SetPageTransitionOffset(0)) {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    present_mobile_page_with_input_policy(runtime, client, *model, MobileInputPolicy::Quarantine)
}

#[cfg(feature = "mobile-ui-runtime")]
fn present_mobile_exit_transition(
    runtime: &mut AppRuntime,
    client: UiClientId,
    model: &mut MobileModel,
) -> MobilePresentOutcome {
    for offset_px in PAGE_TRANSITION_EXIT_OFFSETS {
        if !model.apply(MobileAction::SetPageTransitionOffset(offset_px)) {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        if present_mobile_page_with_input_policy(
            runtime,
            client,
            *model,
            MobileInputPolicy::Quarantine,
        ) != MobilePresentOutcome::Presented
        {
            let _ = model.apply(MobileAction::SetPageTransitionOffset(0));
            return MobilePresentOutcome::Superseded;
        }
    }
    MobilePresentOutcome::Presented
}

#[cfg(feature = "mobile-ui-runtime")]
fn next_mobile_transition_id(events: &UiServerEventTracker) -> u64 {
    events
        .last_focus_generation()
        .filter(|generation| *generation != 0)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL))
}

#[cfg(feature = "mobile-ui-runtime")]
fn shell_app_for_page(page: MobilePage) -> Option<ShellAppId> {
    match page {
        MobilePage::Phone => Some(ShellAppId::Phone),
        MobilePage::Messages => Some(ShellAppId::Messages),
        MobilePage::Settings | MobilePage::Apps | MobilePage::About => Some(ShellAppId::Settings),
        MobilePage::Lock | MobilePage::Home | MobilePage::Calculator | MobilePage::AndroidDemo => {
            None
        }
    }
}

/// Reads the live, authority-free package catalog published by EL1.
///
/// The syscall copies only canonical bounded metadata. APK bytes, a storage
/// handle, and every package-store capability remain unavailable to EL0.
#[cfg(feature = "androidbox-apk-install0")]
fn read_android_installed_app_status() -> AndroidInstalledAppStatus {
    let mut wire = [0_u8; ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE];
    let result = syscall(
        SyscallNumber::AndroidPackageSnapshotRead,
        wire.as_mut_ptr() as u64,
        wire.len() as u64,
        0,
    );
    if result.status != Status::Ok.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let snapshot =
        AndroidPackageSnapshot::decode(&wire).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    android_installed_app_status_from_snapshot(&snapshot)
}

#[cfg(feature = "androidbox-apk-install0")]
fn android_installed_app_status_from_snapshot(
    snapshot: &AndroidPackageSnapshot,
) -> AndroidInstalledAppStatus {
    if !snapshot.installed_package() {
        return AndroidInstalledAppStatus::empty();
    }
    let version_code =
        u32::try_from(snapshot.version_code()).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    AndroidInstalledAppStatus::try_new(
        snapshot.generation(),
        version_code,
        snapshot.apk_length(),
        snapshot.package_name(),
        snapshot.activity_name(),
        snapshot.title(),
        snapshot.text(),
        *snapshot.signer_sha256(),
        *snapshot.apk_sha256(),
    )
    .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL))
}

#[cfg(feature = "androidbox-multipackage4")]
#[derive(Clone, Copy)]
struct AndroidInstalledDirectoryStatus {
    apps: [AndroidInstalledAppStatus; ANDROID_PACKAGE_DIRECTORY_CAPACITY],
    count: u8,
    revision: u64,
}

#[cfg(feature = "androidbox-multipackage4")]
fn read_android_installed_directory_status() -> AndroidInstalledDirectoryStatus {
    let mut wire = [0_u8; ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE];
    let result = syscall(
        SyscallNumber::AndroidPackageDirectoryRead,
        wire.as_mut_ptr() as u64,
        wire.len() as u64,
        0,
    );
    if result.status != Status::Ok.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let directory =
        AndroidPackageDirectory::decode(&wire).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let mut apps = [AndroidInstalledAppStatus::empty(); ANDROID_PACKAGE_DIRECTORY_CAPACITY];
    for (index, destination) in apps
        .iter_mut()
        .take(usize::from(directory.count()))
        .enumerate()
    {
        let snapshot = directory
            .entry(index)
            .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
        *destination = android_installed_app_status_from_snapshot(snapshot);
        #[cfg(feature = "androidbox-icon-resources5")]
        {
            let icon = read_android_installed_icon(&directory, index, snapshot);
            *destination = destination
                .with_icon(icon)
                .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
        }
    }
    AndroidInstalledDirectoryStatus {
        apps,
        count: directory.count(),
        revision: directory.revision(),
    }
}

/// Resolves syscall 59's boot-local Launcher selection back to the exact
/// directory entry consumed by App.
///
/// Launcher and App have independent `MobileModel` mirrors. A compatible
/// Activity focus therefore cannot trust App's previous local selection,
/// especially because generations are package-local and two packages may both
/// be generation 1. The compatibility snapshot identifies the package that
/// actually completed durable relaunch; the directory match restores ABI 55's
/// full package identity and ABI 56's icon without granting APK bytes.
#[cfg(feature = "androidbox-multipackage4")]
fn read_android_active_installed_app_status() -> AndroidInstalledAppStatus {
    let active = read_android_installed_app_status();
    if !active.installed {
        return active;
    }

    let directory = read_android_installed_directory_status();
    let mut matched = None;
    for candidate in directory.apps[..usize::from(directory.count)]
        .iter()
        .copied()
    {
        let identity = {
            #[cfg(feature = "androidbox-icon-resources5")]
            {
                let mut identity = candidate;
                identity.icon = None;
                identity
            }
            #[cfg(not(feature = "androidbox-icon-resources5"))]
            {
                candidate
            }
        };
        if identity == active && matched.replace(candidate).is_some() {
            fail(FAIL_SURFACE_PROTOCOL);
        }
    }
    matched.unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL))
}

#[cfg(feature = "androidbox-icon-resources5")]
fn read_android_installed_icon(
    directory: &AndroidPackageDirectory,
    selector: usize,
    snapshot: &AndroidPackageSnapshot,
) -> Option<AndroidInstalledIcon> {
    let mut wire = [0_u8; ANDROID_PACKAGE_ICON_WIRE_SIZE];
    let result = syscall(
        SyscallNumber::AndroidPackageIconRead,
        wire.as_mut_ptr() as u64,
        wire.len() as u64,
        selector as u64,
    );
    if result.status != Status::Ok.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let value = AndroidPackageIcon::decode(&wire).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    if value.directory_revision() != directory.revision()
        || usize::from(value.selector()) != selector
        || value.generation() != snapshot.generation()
        || value.apk_sha256() != *snapshot.apk_sha256()
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    value.icon().map(|icon| {
        AndroidInstalledIcon::try_new(icon.resource_id, icon.png_crc32, icon.pixels)
            .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL))
    })
}

#[cfg(feature = "androidbox-multipackage4")]
fn refresh_android_installed_catalog(
    model: &mut MobileModel,
    preferred_package: Option<&str>,
) -> bool {
    let directory = read_android_installed_directory_status();
    let count = usize::from(directory.count);
    let selected_index = preferred_package
        .and_then(|preferred| {
            directory.apps[..count]
                .iter()
                .position(|app| app.package.as_str() == preferred)
        })
        .or_else(|| {
            model.android_installed_app.installed.then(|| {
                directory.apps[..count]
                    .iter()
                    .position(|app| {
                        app.package == model.android_installed_app.package
                            && app.apk_digest_sha256
                                == model.android_installed_app.apk_digest_sha256
                    })
                    .unwrap_or(0)
            })
        })
        .unwrap_or(0);
    model.apply_android_installed_directory_status(
        directory.apps,
        directory.count,
        directory.revision,
        selected_index as u8,
    )
}

#[cfg(all(
    feature = "androidbox-apk-install0",
    not(feature = "androidbox-multipackage4")
))]
fn refresh_android_installed_catalog(
    model: &mut MobileModel,
    _preferred_package: Option<&str>,
) -> bool {
    model.apply_android_installed_app_status(read_android_installed_app_status())
}

#[cfg(feature = "androidbox-runtime-install2")]
fn read_android_install_candidate_status() -> AndroidInstallCandidateStatus {
    let mut wire = [0_u8; ANDROID_PACKAGE_INSTALL_CANDIDATE_WIRE_SIZE];
    let result = syscall(
        SyscallNumber::AndroidPackageInstallCandidateRead,
        wire.as_mut_ptr() as u64,
        wire.len() as u64,
        0,
    );
    if result.status != Status::Ok.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let candidate = AndroidPackageInstallCandidate::decode(&wire)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    if !candidate.present() {
        return AndroidInstallCandidateStatus::empty();
    }
    let action = match candidate.action() {
        Some(bndr_abi::AndroidPackageInstallAction::Install) => {
            AndroidInstallCandidateAction::Install
        }
        Some(bndr_abi::AndroidPackageInstallAction::Update) => {
            AndroidInstallCandidateAction::Update
        }
        Some(bndr_abi::AndroidPackageInstallAction::Reinstall) => {
            AndroidInstallCandidateAction::Reinstall
        }
        None => fail(FAIL_SURFACE_PROTOCOL),
    };
    AndroidInstallCandidateStatus::try_new(
        candidate.candidate_id(),
        action,
        candidate.expected_generation(),
        u32::try_from(candidate.expected_version_code())
            .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL)),
        u32::try_from(candidate.version_code()).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL)),
        candidate.apk_length(),
        candidate.package_name(),
        candidate.activity_name(),
        candidate.title(),
        candidate.text(),
        *candidate.signer_sha256(),
        *candidate.apk_sha256(),
    )
    .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL))
}

#[cfg(feature = "androidbox-runtime-install2")]
const ANDROID_PACKAGE_INSTALL_RETRY_LIMIT: usize = 4096;

#[cfg(feature = "androidbox-runtime-install2")]
fn android_package_install_operation_id(request_sequence: u64, candidate_id: u64) -> u64 {
    let mixed =
        0x494e_5354_414c_4c02_u64 ^ candidate_id.rotate_left(19) ^ request_sequence.rotate_left(43);
    if mixed == 0 {
        0x494e_5354_414c_4c02
    } else {
        mixed
    }
}

/// Executes ABI 53's exact-retry Settings install/update exchange.
///
/// The request names only the read-only candidate snapshot. No APK bytes,
/// pathname, storage handle, or block authority enters the App process.
#[cfg(feature = "androidbox-runtime-install2")]
fn request_android_package_install(
    request_sequence: u64,
    expected: AndroidInstallCandidateStatus,
) -> Result<u64, AndroidInstallFailure> {
    if request_sequence == 0 || !expected.present {
        return Err(AndroidInstallFailure::Stale);
    }
    let action = match expected.action {
        AndroidInstallCandidateAction::Install => bndr_abi::AndroidPackageInstallAction::Install,
        AndroidInstallCandidateAction::Update => bndr_abi::AndroidPackageInstallAction::Update,
        AndroidInstallCandidateAction::Reinstall => {
            bndr_abi::AndroidPackageInstallAction::Reinstall
        }
    };
    let operation_id =
        android_package_install_operation_id(request_sequence, expected.candidate_id);
    let request = AndroidPackageInstallRequest::new(
        request_sequence,
        operation_id,
        expected.candidate_id,
        action,
        expected.expected_generation,
        u64::from(expected.expected_version_code),
        u64::from(expected.version_code),
        expected.apk_length,
        expected.apk_digest_sha256,
        expected.signer_digest_sha256,
        expected.package.as_str().as_bytes(),
    )
    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let mut exchange = request.encode();
    let _: &[u8; ANDROID_PACKAGE_INSTALL_WIRE_SIZE] = &exchange;

    for _ in 0..ANDROID_PACKAGE_INSTALL_RETRY_LIMIT {
        let result = syscall(
            SyscallNumber::AndroidPackageInstall,
            exchange.as_mut_ptr() as u64,
            exchange.len() as u64,
            0,
        );
        if result.out1 != 0 || result.out2 != 0 {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        if result.status == Status::ShouldWait.raw() {
            if AndroidPackageInstallRequest::decode(&exchange) != Ok(request) {
                fail(FAIL_SURFACE_PROTOCOL);
            }
            unsafe {
                asm!("wfi", options(nomem, nostack, preserves_flags));
            }
            continue;
        }
        if result.status == Status::Ok.raw() {
            let completion = AndroidPackageInstallResult::decode(&exchange)
                .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
            let io = completion.io_counters();
            if completion.request_sequence() != request_sequence
                || completion.operation_id() != operation_id
                || completion.candidate_id() != expected.candidate_id
                || completion.previous_generation() != expected.expected_generation
                || completion.installed_generation()
                    != expected
                        .expected_generation
                        .checked_add(1)
                        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL))
                || completion.installed_version_code() != u64::from(expected.version_code)
                || completion.apk_length() != expected.apk_length
                || completion.action() != Some(action)
                || completion.apk_sha256() != &expected.apk_digest_sha256
                || completion.signer_sha256() != &expected.signer_digest_sha256
                || io.reads() == 0
                || io.writes() == 0
                || io.flushes() == 0
            {
                fail(FAIL_SURFACE_PROTOCOL);
            }
            return Ok(completion.installed_generation());
        }
        let failure = match result.status {
            raw if raw == Status::Conflict.raw() || raw == Status::NotFound.raw() => {
                AndroidInstallFailure::Stale
            }
            raw if raw == Status::InvalidState.raw()
                || raw == Status::Unavailable.raw()
                || raw == Status::Timeout.raw()
                || raw == Status::OutOfMemory.raw() =>
            {
                AndroidInstallFailure::Busy
            }
            raw if raw == Status::RequiresReset.raw() => AndroidInstallFailure::Storage,
            raw if raw == Status::DataCorrupt.raw() => AndroidInstallFailure::Verification,
            raw if raw == Status::Unsupported.raw() => AndroidInstallFailure::Unsupported,
            _ => fail(FAIL_SURFACE_PROTOCOL),
        };
        if AndroidPackageInstallRequest::decode(&exchange) != Ok(request) {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        return Err(failure);
    }
    Err(AndroidInstallFailure::Busy)
}

#[cfg(feature = "androidbox-apk-install0")]
const ANDROID_PACKAGE_RELAUNCH_RETRY_LIMIT: usize = 4096;

/// Submits one immutable package identity and exact-retries it until the
/// IRQ-enabled kernel monitor publishes a fresh durable readback result.
///
/// Pending and failed calls leave the exchange bytes untouched. A successful
/// call must replace them with a canonical snapshot whose UI-visible identity
/// exactly matches the catalog selected by the user.
#[cfg(feature = "androidbox-apk-install0")]
fn request_android_installed_app_relaunch(
    request_sequence: u64,
    identity: UiCompatibleActivityIdentity,
    expected: AndroidInstalledAppStatus,
) -> Result<AndroidInstalledAppStatus, AndroidInstalledLaunchFailure> {
    if !expected.installed
        || identity.session_id() == 0
        || identity.package_generation() != expected.generation
    {
        return Err(AndroidInstalledLaunchFailure::Stale);
    }
    let request = AndroidPackageRelaunchRequest::new(
        request_sequence,
        expected.generation,
        u64::from(expected.version_code),
        expected.apk_length,
        expected.apk_digest_sha256,
        expected.signer_digest_sha256,
        expected.package.as_str().as_bytes(),
        expected.activity.as_str().as_bytes(),
    )
    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let mut exchange = request.encode();
    let _: &[u8; ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE] = &exchange;

    for _ in 0..ANDROID_PACKAGE_RELAUNCH_RETRY_LIMIT {
        #[cfg(feature = "androidbox-el0-runtime0")]
        let compatible_session_id = identity.session_id();
        #[cfg(not(feature = "androidbox-el0-runtime0"))]
        let compatible_session_id = 0;
        let result = syscall(
            SyscallNumber::AndroidPackageRelaunch,
            exchange.as_mut_ptr() as u64,
            exchange.len() as u64,
            compatible_session_id,
        );
        if result.out1 != 0 || result.out2 != 0 {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        if result.status == Status::ShouldWait.raw() {
            if AndroidPackageRelaunchRequest::decode(&exchange) != Ok(request) {
                fail(FAIL_SURFACE_PROTOCOL);
            }
            // The monitor that performs package I/O runs only outside the
            // IRQ-masked syscall domain. Yield until an interrupt gives it a
            // chance to finish, then retry the exact same request bytes.
            unsafe {
                asm!("wfi", options(nomem, nostack, preserves_flags));
            }
            continue;
        }
        if result.status == Status::Ok.raw() {
            let snapshot = AndroidPackageSnapshot::decode(&exchange)
                .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
            let io = snapshot.io_counters();
            if io.reads() == 0 || io.writes() != 0 || io.flushes() != 0 {
                fail(FAIL_SURFACE_PROTOCOL);
            }
            let actual = android_installed_app_status_from_snapshot(&snapshot);
            #[cfg(feature = "androidbox-icon-resources5")]
            let actual = actual
                .with_icon(expected.icon)
                .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
            if actual != expected {
                fail(FAIL_SURFACE_PROTOCOL);
            }
            return Ok(actual);
        }
        let failure = match result.status {
            raw if raw == Status::Conflict.raw() || raw == Status::NotFound.raw() => {
                AndroidInstalledLaunchFailure::Stale
            }
            raw if raw == Status::DataCorrupt.raw() => AndroidInstalledLaunchFailure::Verification,
            raw if raw == Status::Unsupported.raw() => AndroidInstalledLaunchFailure::Unsupported,
            raw if raw == Status::Unavailable.raw()
                || raw == Status::RequiresReset.raw()
                || raw == Status::InvalidState.raw()
                || raw == Status::OutOfMemory.raw()
                || raw == Status::Timeout.raw() =>
            {
                AndroidInstalledLaunchFailure::Unavailable
            }
            _ => fail(FAIL_SURFACE_PROTOCOL),
        };
        if AndroidPackageRelaunchRequest::decode(&exchange) != Ok(request) {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        return Err(failure);
    }
    Err(AndroidInstalledLaunchFailure::Unavailable)
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
const ANDROID_PACKAGE_UNINSTALL_RETRY_LIMIT: usize = 4096;

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn android_package_uninstall_operation_id(request_sequence: u64, generation: u64) -> u64 {
    let mixed =
        0x5255_4e54_494d_4501_u64 ^ generation.rotate_left(17) ^ request_sequence.rotate_left(41);
    if mixed == 0 {
        0x5255_4e54_494d_4501
    } else {
        mixed
    }
}

/// Executes ABI 52's exact-retry Settings uninstall exchange.
///
/// The built-in App receives no storage object. EL1 validates the complete
/// installed identity and performs the durable tombstone outside the SVC
/// frame; success is accepted only with a canonical terminal result.
#[cfg(feature = "androidbox-runtime-uninstall1")]
fn request_android_installed_app_uninstall(
    request_sequence: u64,
    expected: AndroidInstalledAppStatus,
) -> Result<u64, AndroidInstalledUninstallFailure> {
    if !expected.installed || request_sequence == 0 {
        return Err(AndroidInstalledUninstallFailure::Stale);
    }
    let operation_id =
        android_package_uninstall_operation_id(request_sequence, expected.generation);
    let request = AndroidPackageUninstallRequest::new(
        request_sequence,
        operation_id,
        expected.generation,
        u64::from(expected.version_code),
        expected.apk_length,
        expected.apk_digest_sha256,
        expected.signer_digest_sha256,
        expected.package.as_str().as_bytes(),
    )
    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let mut exchange = request.encode();
    let _: &[u8; ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE] = &exchange;

    for _ in 0..ANDROID_PACKAGE_UNINSTALL_RETRY_LIMIT {
        let result = syscall(
            SyscallNumber::AndroidPackageUninstall,
            exchange.as_mut_ptr() as u64,
            exchange.len() as u64,
            0,
        );
        if result.out1 != 0 || result.out2 != 0 {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        if result.status == Status::ShouldWait.raw() {
            if AndroidPackageUninstallRequest::decode(&exchange) != Ok(request) {
                fail(FAIL_SURFACE_PROTOCOL);
            }
            unsafe {
                asm!("wfi", options(nomem, nostack, preserves_flags));
            }
            continue;
        }
        if result.status == Status::Ok.raw() {
            let completion = AndroidPackageUninstallResult::decode(&exchange)
                .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
            let io = completion.io_counters();
            if completion.request_sequence() != request_sequence
                || completion.operation_id() != operation_id
                || completion.last_installed_generation() != expected.generation
                || expected.generation.checked_add(1) != Some(completion.removal_generation())
                || io.reads() == 0
                || io.writes() == 0
                || io.flushes() == 0
            {
                fail(FAIL_SURFACE_PROTOCOL);
            }
            return Ok(completion.removal_generation());
        }
        if AndroidPackageUninstallRequest::decode(&exchange) != Ok(request) {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        return Err(match result.status {
            raw if raw == Status::Conflict.raw() || raw == Status::NotFound.raw() => {
                AndroidInstalledUninstallFailure::Stale
            }
            raw if raw == Status::InvalidState.raw() => AndroidInstalledUninstallFailure::Busy,
            raw if raw == Status::RequiresReset.raw()
                || raw == Status::Unavailable.raw()
                || raw == Status::Timeout.raw() =>
            {
                AndroidInstalledUninstallFailure::Storage
            }
            raw if raw == Status::DataCorrupt.raw() || raw == Status::Unsupported.raw() => {
                AndroidInstalledUninstallFailure::Verification
            }
            _ => fail(FAIL_SURFACE_PROTOCOL),
        });
    }
    Err(AndroidInstalledUninstallFailure::Storage)
}

/// Claims the one-shot, read-only package image prepared by syscall 60 and
/// executes its first Activity lifecycle from App-owned EL0 memory.
#[cfg(all(
    feature = "androidbox-el0-runtime0",
    not(feature = "androidbox-interactive0")
))]
fn claim_and_execute_android_installed_activity(
    identity: UiCompatibleActivityIdentity,
    expected: AndroidInstalledAppStatus,
) -> Option<InstalledActivityExecution> {
    if !expected.installed || identity.package_generation() != expected.generation {
        return None;
    }
    let claim = AndroidPackageImageClaim::new(
        identity.session_id(),
        expected.generation,
        expected.apk_length,
        expected.apk_digest_sha256,
        expected.signer_digest_sha256,
    )
    .ok()?;
    let wire = claim.encode();
    let _: &[u8; ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE] = &wire;
    let claimed = syscall(
        SyscallNumber::AndroidPackageImageClaim,
        wire.as_ptr() as u64,
        wire.len() as u64,
        0,
    );
    if claimed.status != Status::Ok.raw()
        || claimed.out1 == 0
        || claimed.out2 != u64::from(expected.apk_length)
    {
        return None;
    }
    let image = match OwnedUserHandle::new(claimed.out1) {
        Some(image) => image,
        None => return None,
    };
    let execution = read_and_execute_installed_activity(expected, |offset, destination| {
        let offset = u32::try_from(offset).map_err(|_| ())?;
        let length = u32::try_from(destination.len()).map_err(|_| ())?;
        let read = syscall(
            SyscallNumber::VmoRead,
            image.raw(),
            destination.as_mut_ptr() as u64,
            pack_vmo_read(offset, length),
        );
        if read.status == Status::Ok.raw()
            && read.out1 == destination.len() as u64
            && read.out2 == u64::from(expected.apk_length)
        {
            Ok(())
        } else {
            Err(())
        }
    });
    close_owned(image);
    execution.ok()
}

/// ABI 46 claims one immutable package image and retains its admitted Activity
/// session in App-private memory until focus leaves the compatible Activity.
///
/// The VMO handle is closed immediately after the bounded copy. The retained
/// session has no kernel handles and can only dispatch the one verified Button
/// callback exposed by `InstalledInteractiveActivityLease`.
#[cfg(all(
    feature = "androidbox-interactive0",
    not(feature = "androidbox-process0")
))]
fn claim_and_open_android_installed_activity(
    identity: UiCompatibleActivityIdentity,
    expected: AndroidInstalledAppStatus,
    lease: &mut AndroidInstalledActivityLease,
) -> Option<InstalledInteractiveActivitySnapshot> {
    if !expected.installed || identity.package_generation() != expected.generation {
        return None;
    }
    let claim = AndroidPackageImageClaim::new(
        identity.session_id(),
        expected.generation,
        expected.apk_length,
        expected.apk_digest_sha256,
        expected.signer_digest_sha256,
    )
    .ok()?;
    let wire = claim.encode();
    let _: &[u8; ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE] = &wire;
    let claimed = syscall(
        SyscallNumber::AndroidPackageImageClaim,
        wire.as_ptr() as u64,
        wire.len() as u64,
        0,
    );
    if claimed.status != Status::Ok.raw()
        || claimed.out1 == 0
        || claimed.out2 != u64::from(expected.apk_length)
    {
        return None;
    }
    let image = OwnedUserHandle::new(claimed.out1)?;
    let snapshot = lease.open(expected, |offset, destination| {
        let offset = u32::try_from(offset).map_err(|_| ())?;
        let length = u32::try_from(destination.len()).map_err(|_| ())?;
        let read = syscall(
            SyscallNumber::VmoRead,
            image.raw(),
            destination.as_mut_ptr() as u64,
            pack_vmo_read(offset, length),
        );
        if read.status == Status::Ok.raw()
            && read.out1 == destination.len() as u64
            && read.out2 == u64::from(expected.apk_length)
        {
            Ok(())
        } else {
            Err(())
        }
    });
    close_owned(image);
    snapshot.ok()
}

/// ABI 47 leaves the APK bytes and retained Activity state in the independent,
/// capability-minimal AndroidApp process. The trusted App receives only the
/// canonical scene description needed to rasterize the Activity.
#[cfg(feature = "androidbox-process0")]
fn claim_and_open_android_installed_activity(
    identity: UiCompatibleActivityIdentity,
    expected: AndroidInstalledAppStatus,
    lease: &mut AndroidInstalledActivityLease,
) -> Option<androidapp_process::AndroidAppRemoteSnapshot> {
    if !expected.installed || identity.package_generation() != expected.generation {
        return None;
    }
    lease.open(identity).ok()
}

#[cfg(feature = "androidbox-el0-runtime0")]
fn finish_failed_android_installed_activity(runtime: &mut AppRuntime) {
    let transition_id = next_mobile_transition_id(&runtime.events);
    let control = UiClientControl::set_focus(UiClientId::Launcher, None, transition_id)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    write_ui_control(runtime.channel.raw(), control);
}

#[cfg(all(
    feature = "androidbox-interactive0",
    not(feature = "androidbox-process0")
))]
fn close_android_installed_activity_lease(
    lease: &mut AndroidInstalledActivityLease,
    active: &mut bool,
    model: &mut MobileModel,
) {
    if *active {
        if lease.close().is_err() {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        *active = false;
    }
    let _ = model.clear_android_installed_activity_view();
}

#[cfg(feature = "androidbox-process0")]
fn close_android_installed_activity_lease(
    lease: &mut AndroidInstalledActivityLease,
    active: &mut bool,
    model: &mut MobileModel,
) {
    if *active {
        // A rejected remote Click/Close already wipes the worker lease and
        // clears the client's local session. Treat that cleanup as idempotent;
        // malformed transport still fail-stops inside AndroidAppClient.
        let _ = lease.close();
        *active = false;
    }
    let _ = model.clear_android_installed_activity_view();
}

#[cfg(all(feature = "androidbox-dex0", not(feature = "androidbox-apk-install0")))]
fn apply_androidbox_attempt(
    runtime: &mut AppRuntime,
    model: &mut MobileModel,
    attempt: Result<AndroidBoxRun, AndroidBoxViewState>,
) -> bool {
    let view = match attempt {
        Ok(run) => {
            // The SurfaceServer audit report is sent before the result frame.
            // AndroidBox bytecode receives neither this channel nor any other
            // Bndroid handle.
            write_ui_control(runtime.channel.raw(), run.report);
            run.view
        }
        Err(view) => view,
    };
    model.apply_androidbox_result(
        view.verified,
        view.result,
        view.instruction_count,
        view.tap_count,
        view.error,
    )
}

#[cfg(all(feature = "androidbox-dex0", not(feature = "androidbox-apk-install0")))]
fn apply_androidbox_activity_attempt(
    runtime: &mut AppRuntime,
    model: &mut MobileModel,
    attempt: Result<AndroidBoxActivityRun, AndroidBoxActivityViewState>,
) -> bool {
    let view = match attempt {
        Ok(run) => {
            // Resources-1 preconstructs both independently sequenced reports,
            // then sends Activity before Resource after the DEX Boot report.
            // The model is updated only after both writes, so no frame can
            // present a partially audited resource launch.
            write_ui_control(runtime.channel.raw(), run.activity_report);
            write_ui_control(runtime.channel.raw(), run.resource_report);
            run.view
        }
        Err(view) => view,
    };
    model.apply_androidbox_activity_result(
        view.manifest_verified,
        view.launcher_activity,
        view.on_create_completed,
        view.text_view_content,
        view.instruction_count,
        AndroidBoxResourceStatus {
            resource_table_parsed: view.resource_table_parsed,
            layout_entry_resolved: view.layout_entry_resolved,
            binary_xml_parsed: view.binary_xml_parsed,
            text_view_verified: view.text_view_verified,
            string_reference_resolved: view.string_reference_resolved,
            layout_resource_id: view.layout_resource_id,
            string_resource_id: view.string_resource_id,
        },
        view.error,
    )
}

#[cfg(feature = "mobile-ui-runtime")]
fn page_for_shell_app(app: ShellAppId) -> MobilePage {
    match app {
        ShellAppId::Phone => MobilePage::Phone,
        ShellAppId::Messages => MobilePage::Messages,
        ShellAppId::Settings => MobilePage::Settings,
    }
}

#[cfg(feature = "mobile-ui-runtime")]
fn mobile_launcher_loop(startup: u64) -> ! {
    let denied = syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0);
    if denied.status != Status::PermissionDenied.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_SURFACE_ACQUIRE);
    }
    let channel = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let (server_pid, ready) = read_ui_event(channel.raw());
    let mut events = UiServerEventTracker::new();
    if server_pid == 0
        || !matches!(ready.payload(), UiServerEventPayload::Ready)
        || events.accept(ready).is_err()
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let (appearance_sender, initial_appearance) = read_ui_event(channel.raw());
    if appearance_sender != server_pid
        || events.accept(initial_appearance).is_err()
        || !matches!(
            initial_appearance.payload(),
            UiServerEventPayload::AppearanceChanged { revision: 1, .. }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let (clock_sender, initial_clock) = read_ui_event(channel.raw());
    if clock_sender != server_pid
        || events.accept(initial_clock).is_err()
        || !matches!(
            initial_clock.payload(),
            UiServerEventPayload::ClockChanged { revision: 1, .. }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let (notification_sender, initial_notification) = read_ui_event(channel.raw());
    if notification_sender != server_pid
        || events.accept(initial_notification).is_err()
        || !matches!(
            initial_notification.payload(),
            UiServerEventPayload::BootNotificationChanged {
                visible: true,
                revision: 1
            }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let (system_ui_sender, initial_system_ui) = read_ui_event(channel.raw());
    if system_ui_sender != server_pid
        || events.accept(initial_system_ui).is_err()
        || !matches!(
            initial_system_ui.payload(),
            UiServerEventPayload::SystemUiChanged {
                mode: UiSystemUiMode::Locked,
                recent: None,
                nav_pressed: false,
                nav_reveal_px: 0,
                revision: 1
            }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let (focus_sender, initial_focus) = read_ui_event(channel.raw());
    if focus_sender != server_pid
        || events.accept(initial_focus).is_err()
        || !matches!(
            initial_focus.payload(),
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::Launcher,
                app: None,
                focus_generation: 1
            }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut runtime = create_mobile_graphics_runtime(channel, server_pid, events);
    let mut model = MobileModel::locked();
    #[cfg(feature = "androidbox-apk-install0")]
    let _ = refresh_android_installed_catalog(&mut model, None);
    model.time = MobileTimeSnapshot::from_unix_seconds(runtime.delivered_clock_unix_seconds);
    let _ = apply_mobile_appearance(&mut model, runtime.delivered_appearance);
    let _ = apply_mobile_boot_notification(&mut model, runtime.delivered_boot_notification_visible);
    let _ = apply_mobile_system_ui(&mut model, &runtime);
    let mut touch = TouchController::new();
    #[cfg(all(feature = "androidbox-dex0", not(feature = "androidbox-apk-install0")))]
    let mut androidbox = AndroidBoxRuntime::new();
    #[cfg(feature = "androidbox-apk-install0")]
    let mut android_package_relaunch_sequence = 0_u64;
    #[cfg(feature = "androidbox-apk-install0")]
    let mut compatible_activity_operation: Option<CompatibleActivityLaunchOperation> = None;
    if present_mobile_page(&mut runtime, UiClientId::Launcher, model)
        != MobilePresentOutcome::Presented
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }

    loop {
        match next_mobile_server_payload(&mut runtime) {
            UiServerEventPayload::Input(sample)
                if runtime.delivered_active_client == UiClientId::Launcher =>
            {
                if runtime.system_ui_request_outstanding.is_some() {
                    runtime.quarantined_mobile_inputs = runtime
                        .quarantined_mobile_inputs
                        .checked_add(1)
                        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                    runtime.suppress_mobile_input_until_release = sample.pressed();
                    touch = TouchController::new();
                    continue;
                }
                if runtime.suppress_mobile_input_until_release {
                    if !sample.pressed() {
                        runtime.suppress_mobile_input_until_release = false;
                        touch = TouchController::new();
                    }
                    continue;
                }
                let Some(action) = observe_mobile_sample(&mut touch, model, sample) else {
                    continue;
                };
                match action {
                    #[cfg(feature = "androidbox-apk-install0")]
                    MobileAction::LaunchInstalledAndroid(selection) => {
                        #[cfg(feature = "androidbox-multipackage4")]
                        let generation = {
                            let Some(index) = selection
                                .checked_sub(1)
                                .and_then(|value| u8::try_from(value).ok())
                            else {
                                touch = TouchController::new();
                                continue;
                            };
                            if usize::from(index) >= usize::from(model.android_installed_app_count)
                            {
                                touch = TouchController::new();
                                continue;
                            }
                            let _ = model.select_android_installed_app(index);
                            model.android_installed_app.generation
                        };
                        #[cfg(not(feature = "androidbox-multipackage4"))]
                        let generation = selection;
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        if compatible_activity_operation.is_some()
                            || model.system_ui_mode != UiSystemUiMode::Home
                            || model.system_nav_pressed
                            || !model.android_installed_app.installed
                            || model.android_installed_app.generation != generation
                        {
                            touch = TouchController::new();
                            continue;
                        }
                        let request_sequence = android_package_relaunch_sequence
                            .checked_add(1)
                            .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                        let identity = match model.system_ui_recent {
                            Some(UiRecentIdentity::CompatibleAndroid(identity))
                                if identity.package_generation() == generation
                                    && model.android_installed_session.is_some_and(|session| {
                                        session.identity() == identity
                                            && model.compatible_android_recent_ready()
                                    }) =>
                            {
                                identity
                            }
                            Some(UiRecentIdentity::CompatibleAndroid(stale_identity)) => {
                                let identity =
                                    UiCompatibleActivityIdentity::new(request_sequence, generation)
                                        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                                // ABI 55 permits distinct installed packages
                                // to have the same per-volume generation. A
                                // stale compatible recent identity therefore
                                // cannot be reused or compared by generation
                                // alone. Clear it through the ordinary
                                // authenticated SystemUI transition, then
                                // reserve a fresh session for the selected
                                // package after the exact completion arrives.
                                let clear = request_mobile_system_ui_recent(
                                    &mut runtime,
                                    UiSystemUiAction::FinishCompatibleActivity,
                                    Some(UiRecentIdentity::CompatibleAndroid(stale_identity)),
                                );
                                compatible_activity_operation =
                                    Some(CompatibleActivityLaunchOperation {
                                        identity,
                                        request_sequence,
                                        origin: CompatibleActivityLaunchOrigin::AllApps,
                                        origin_recent: Some(UiRecentIdentity::CompatibleAndroid(
                                            stale_identity,
                                        )),
                                        phase: CompatibleActivityLaunchPhase::ClearRecent {
                                            request_id: clear.request_id,
                                            stale_identity,
                                        },
                                    });
                                touch = TouchController::new();
                                continue;
                            }
                            Some(UiRecentIdentity::Shell(_)) | None => {
                                UiCompatibleActivityIdentity::new(request_sequence, generation)
                                    .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL))
                            }
                        };
                        let request = request_mobile_system_ui_recent(
                            &mut runtime,
                            UiSystemUiAction::ReserveCompatibleActivity,
                            Some(UiRecentIdentity::CompatibleAndroid(identity)),
                        );
                        compatible_activity_operation = Some(CompatibleActivityLaunchOperation {
                            identity,
                            request_sequence,
                            origin: CompatibleActivityLaunchOrigin::AllApps,
                            origin_recent: model.system_ui_recent,
                            phase: CompatibleActivityLaunchPhase::Reservation {
                                request_id: request.request_id,
                            },
                        });
                        touch = TouchController::new();
                    }
                    #[cfg(not(feature = "androidbox-apk-install0"))]
                    MobileAction::LaunchInstalledAndroid(_) => {
                        if model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::Open(MobilePage::Calculator) => {
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        if model.apply(action) {
                            let _ = present_mobile_enter_transition(
                                &mut runtime,
                                UiClientId::Launcher,
                                &mut model,
                            );
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::Open(MobilePage::AndroidDemo) => {
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        if model.apply(action) {
                            #[cfg(all(
                                feature = "androidbox-dex0",
                                not(feature = "androidbox-apk-install0")
                            ))]
                            {
                                let attempt = if androidbox.has_booted() {
                                    Err(androidbox.view())
                                } else {
                                    androidbox.boot()
                                };
                                let _ = apply_androidbox_attempt(&mut runtime, &mut model, attempt);
                                let activity_attempt = if !androidbox.has_booted()
                                    || androidbox.has_attempted_activity()
                                {
                                    Err(androidbox.activity_view())
                                } else {
                                    androidbox.launch_activity()
                                };
                                let _ = apply_androidbox_activity_attempt(
                                    &mut runtime,
                                    &mut model,
                                    activity_attempt,
                                );
                            }
                            let _ = present_mobile_enter_transition(
                                &mut runtime,
                                UiClientId::Launcher,
                                &mut model,
                            );
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::Open(page) => {
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        model.unlock_reveal_px = 0;
                        let app =
                            shell_app_for_page(page).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                        let transition_id = next_mobile_transition_id(&runtime.events);
                        let control =
                            UiClientControl::set_focus(UiClientId::App, Some(app), transition_id)
                                .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
                        write_ui_control(runtime.channel.raw(), control);
                        touch = TouchController::new();
                    }
                    #[cfg(feature = "androidbox-apk-install0")]
                    MobileAction::Home
                        if model.page == MobilePage::AndroidDemo
                            && model.installed_android_foreground_content_ready() =>
                    {
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        let identity = model
                            .android_installed_session
                            .map(|session| session.identity())
                            .filter(|identity| {
                                model.system_ui_mode == UiSystemUiMode::Foreground
                                    && model.system_ui_recent
                                        == Some(UiRecentIdentity::CompatibleAndroid(*identity))
                            })
                            .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                        request_mobile_system_ui_recent(
                            &mut runtime,
                            UiSystemUiAction::HomeCompatibleActivity,
                            Some(UiRecentIdentity::CompatibleAndroid(identity)),
                        );
                        touch = TouchController::new();
                    }
                    #[cfg(feature = "androidbox-apk-install0")]
                    MobileAction::Back
                        if model.page == MobilePage::AndroidDemo
                            && model.installed_android_foreground_content_ready()
                            && !model.shade_open
                            && model.shade_reveal_px == 0 =>
                    {
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        let identity = model
                            .android_installed_session
                            .map(|session| session.identity())
                            .filter(|identity| {
                                model.system_ui_mode == UiSystemUiMode::Foreground
                                    && model.system_ui_recent
                                        == Some(UiRecentIdentity::CompatibleAndroid(*identity))
                            })
                            .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                        request_mobile_system_ui_recent(
                            &mut runtime,
                            UiSystemUiAction::FinishCompatibleActivity,
                            Some(UiRecentIdentity::CompatibleAndroid(identity)),
                        );
                        touch = TouchController::new();
                    }
                    MobileAction::Home
                        if matches!(
                            model.page,
                            MobilePage::Calculator | MobilePage::AndroidDemo
                        ) =>
                    {
                        let _ = present_mobile_exit_transition(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        if model.apply(MobileAction::Home) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::Back
                        if matches!(
                            model.page,
                            MobilePage::Calculator | MobilePage::AndroidDemo
                        ) && !model.shade_open
                            && model.shade_reveal_px == 0 =>
                    {
                        let _ = present_mobile_exit_transition(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        if model.apply(MobileAction::Home) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::Unlock => {
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        request_mobile_system_ui(&mut runtime, UiSystemUiAction::Unlock, None);
                        touch = TouchController::new();
                    }
                    MobileAction::ActivateRecentApp => {
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        let recent = model
                            .system_ui_recent
                            .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                        match recent {
                            UiRecentIdentity::Shell(app) => {
                                request_mobile_system_ui(
                                    &mut runtime,
                                    UiSystemUiAction::ActivateRecent,
                                    Some(app),
                                );
                            }
                            #[cfg(feature = "androidbox-apk-install0")]
                            UiRecentIdentity::CompatibleAndroid(identity) => {
                                if compatible_activity_operation.is_some()
                                    || model.system_ui_mode != UiSystemUiMode::Overview
                                    || model.system_nav_pressed
                                {
                                    touch = TouchController::new();
                                    continue;
                                }
                                model
                                    .android_installed_session
                                    .filter(|session| {
                                        session.identity() == identity
                                            && model.compatible_android_recent_ready()
                                    })
                                    .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                                let request_sequence = android_package_relaunch_sequence
                                    .checked_add(1)
                                    .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                                let request = request_mobile_system_ui_recent(
                                    &mut runtime,
                                    UiSystemUiAction::ReserveCompatibleActivity,
                                    Some(UiRecentIdentity::CompatibleAndroid(identity)),
                                );
                                compatible_activity_operation =
                                    Some(CompatibleActivityLaunchOperation {
                                        identity,
                                        request_sequence,
                                        origin: CompatibleActivityLaunchOrigin::Overview,
                                        origin_recent: Some(UiRecentIdentity::CompatibleAndroid(
                                            identity,
                                        )),
                                        phase: CompatibleActivityLaunchPhase::Reservation {
                                            request_id: request.request_id,
                                        },
                                    });
                            }
                            #[cfg(not(feature = "androidbox-apk-install0"))]
                            UiRecentIdentity::CompatibleAndroid(_) => fail(FAIL_SURFACE_PROTOCOL),
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::CloseOverview | MobileAction::Back
                        if model.overview_open()
                            && !model.shade_open
                            && model.shade_reveal_px == 0 =>
                    {
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        request_mobile_system_ui(
                            &mut runtime,
                            UiSystemUiAction::CloseOverview,
                            None,
                        );
                        touch = TouchController::new();
                    }
                    MobileAction::Home
                    | MobileAction::Back
                    | MobileAction::OpenShade
                    | MobileAction::CloseShade
                    | MobileAction::OpenDrawer
                    | MobileAction::CloseDrawer => {
                        if model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::CloseOverview => {
                        if model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::SetShadeReveal(_)
                    | MobileAction::SetDrawerReveal(_)
                    | MobileAction::SetUnlockReveal(_)
                    | MobileAction::SetBootNotificationOffset(_)
                    | MobileAction::SetPageTransitionOffset(_)
                    | MobileAction::SetPageScroll { .. }
                    | MobileAction::SetBackReveal { .. }
                    | MobileAction::SetPressed(_) => {
                        if model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                        }
                    }
                    MobileAction::ToggleTheme => {
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        request_mobile_appearance(&mut runtime, UiAppearanceAction::ToggleTheme);
                        touch = TouchController::new();
                    }
                    MobileAction::ToggleAccent => {
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        request_mobile_appearance(&mut runtime, UiAppearanceAction::ToggleAccent);
                        touch = TouchController::new();
                    }
                    MobileAction::SetSoftwareDimming(level) => {
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        request_mobile_appearance(&mut runtime, software_dimming_action(level));
                    }
                    MobileAction::ActivateBootNotification => {
                        // A visual Lock is not navigation authority. This
                        // second guard complements the touch-level refusal.
                        if model.page != MobilePage::Lock && model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::DismissBootNotification => {
                        if model.boot_notification_visible {
                            request_mobile_boot_notification_dismiss(&mut runtime);
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::ExecuteAndroidBoxDex => {
                        clear_mobile_press_and_present(
                            &mut runtime,
                            UiClientId::Launcher,
                            &mut model,
                        );
                        #[cfg(all(
                            feature = "androidbox-dex0",
                            not(feature = "androidbox-apk-install0")
                        ))]
                        {
                            let attempt = androidbox.on_tap();
                            if apply_androidbox_attempt(&mut runtime, &mut model, attempt) {
                                let _ =
                                    present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                            }
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::PhoneKey(_)
                    | MobileAction::PhoneBackspace
                    | MobileAction::CalculatorKey(_)
                    | MobileAction::OpenMessage(_) => {
                        if model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                        }
                        touch = TouchController::new();
                    }
                    #[cfg(feature = "androidbox-interactive0")]
                    MobileAction::ActivateInstalledAndroidButton(_) => {
                        // Launcher can never own an installed Activity View
                        // proof, so this runtime-only action is unreachable.
                        // Keep it authority-free if a future model change
                        // accidentally produces it while Launcher is focused.
                        touch = TouchController::new();
                    }
                    #[cfg(feature = "androidbox-multipackage4")]
                    MobileAction::SelectInstalledAndroid(_) => {
                        // The Apps page belongs to the App process. Launcher
                        // never turns this local selector into package
                        // authority if malformed input reaches its endpoint.
                        touch = TouchController::new();
                    }
                    #[cfg(feature = "androidbox-runtime-uninstall1")]
                    MobileAction::BeginInstalledAndroidUninstall(_)
                    | MobileAction::ConfirmInstalledAndroidUninstall(_)
                    | MobileAction::CancelInstalledAndroidUninstall => {
                        // Settings owns the only UI path to syscall 63.
                        touch = TouchController::new();
                    }
                    #[cfg(feature = "androidbox-runtime-install2")]
                    MobileAction::BeginAndroidInstall(_)
                    | MobileAction::ConfirmAndroidInstall(_)
                    | MobileAction::CancelAndroidInstall => {
                        // Settings owns the only UI path to syscalls 64/65.
                        touch = TouchController::new();
                    }
                }
            }
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::Launcher,
                app: None,
                ..
            } => {
                // A focus notification is not unlock authority. Normal app
                // returns settle on Home, while a still-locked Launcher must
                // remain locked even if a redundant focus event is delivered.
                if !matches!(
                    model.page,
                    MobilePage::Lock | MobilePage::Calculator | MobilePage::AndroidDemo
                ) {
                    model.page = MobilePage::Home;
                }
                #[cfg(feature = "androidbox-apk-install0")]
                let _ = refresh_android_installed_catalog(&mut model, None);
                model.shade_open = false;
                model.shade_reveal_px = 0;
                model.drawer_open = false;
                model.drawer_reveal_px = 0;
                model.back_reveal_px = 0;
                model.back_origin_y = 0;
                model.unlock_reveal_px = 0;
                model.page_transition_offset_px = 0;
                model.pressed_target = None;
                touch = TouchController::new();
                let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
            }
            UiServerEventPayload::AppearanceChanged { .. } => {
                if apply_mobile_appearance(&mut model, runtime.delivered_appearance)
                    && runtime.delivered_active_client == UiClientId::Launcher
                {
                    let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                }
            }
            UiServerEventPayload::ClockChanged { .. } => {
                let next =
                    MobileTimeSnapshot::from_unix_seconds(runtime.delivered_clock_unix_seconds);
                if model.time != next {
                    model.time = next;
                    if runtime.delivered_active_client == UiClientId::Launcher {
                        let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                    }
                }
            }
            UiServerEventPayload::BootNotificationChanged { .. } => {
                if apply_mobile_boot_notification(
                    &mut model,
                    runtime.delivered_boot_notification_visible,
                ) && runtime.delivered_active_client == UiClientId::Launcher
                {
                    let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                }
                touch = TouchController::new();
            }
            UiServerEventPayload::SystemUiChanged { .. } => {
                let previous_mode = model.system_ui_mode;
                let changed = apply_mobile_system_ui(&mut model, &runtime);
                let compatible_foreground = matches!(
                    runtime.delivered_system_ui_recent,
                    Some(UiRecentIdentity::CompatibleAndroid(_))
                ) && model.compatible_android_recent_ready();
                #[cfg(not(feature = "androidbox-el0-runtime0"))]
                let launcher_owns_visible_mode = runtime.delivered_active_client
                    == UiClientId::Launcher
                    && (runtime.delivered_system_ui_mode != UiSystemUiMode::Foreground
                        || compatible_foreground);
                #[cfg(feature = "androidbox-el0-runtime0")]
                let launcher_owns_visible_mode = runtime.delivered_active_client
                    == UiClientId::Launcher
                    && runtime.delivered_system_ui_mode != UiSystemUiMode::Foreground;
                #[cfg(feature = "androidbox-el0-runtime0")]
                let _ = compatible_foreground;
                if changed && launcher_owns_visible_mode {
                    if previous_mode == UiSystemUiMode::Foreground
                        && runtime.delivered_system_ui_mode == UiSystemUiMode::Home
                    {
                        model.page_transition_offset_px = 0;
                    }
                    let _ = present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                }
                touch = TouchController::new();
            }
            UiServerEventPayload::SystemUiRequestCompleted {
                action,
                status,
                request_id,
                revision,
                compatible_identity,
                reservation_origin,
            } => {
                #[cfg(feature = "androidbox-el0-runtime0")]
                if runtime.system_ui_request_outstanding.is_none() {
                    let expected_request_id = runtime
                        .system_ui_request_id
                        .checked_add(1)
                        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                    if action != UiSystemUiAction::FinishCompatibleActivity
                        || status != UiSystemUiRequestStatus::Accepted
                        || request_id != expected_request_id
                        || revision != runtime.delivered_system_ui_revision
                        || compatible_identity.is_none()
                        || reservation_origin.is_some()
                        || runtime.delivered_active_client != UiClientId::App
                        || runtime.delivered_system_ui_mode != UiSystemUiMode::Home
                        || runtime.delivered_system_ui_recent.is_some()
                        || runtime.delivered_system_nav_pressed
                        || runtime.delivered_system_nav_reveal_px != 0
                        || compatible_activity_operation.is_some()
                    {
                        fail(FAIL_SURFACE_PROTOCOL);
                    }
                    // App Back is committed by SurfaceServer, but consumes the
                    // shared Launcher request epoch. Mirror that exact accepted
                    // completion so the next Launcher request remains
                    // contiguous rather than looking like a replay.
                    runtime.system_ui_request_id = request_id;
                    touch = TouchController::new();
                    continue;
                }
                let outstanding = runtime
                    .system_ui_request_outstanding
                    .take()
                    .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                if action != outstanding.action
                    || request_id != outstanding.request_id
                    || compatible_identity
                        != outstanding
                            .recent
                            .and_then(UiRecentIdentity::compatible_android)
                    || revision < outstanding.observed_revision
                    || runtime.delivered_system_ui_revision != revision
                {
                    fail(FAIL_SURFACE_PROTOCOL);
                }
                match status {
                    UiSystemUiRequestStatus::Accepted => {
                        if revision
                            != outstanding
                                .observed_revision
                                .checked_add(1)
                                .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL))
                        {
                            fail(FAIL_SURFACE_PROTOCOL);
                        }
                        runtime.system_ui_request_id = request_id;
                    }
                    UiSystemUiRequestStatus::Conflict | UiSystemUiRequestStatus::CaptureBusy => {}
                }

                #[cfg(not(feature = "androidbox-apk-install0"))]
                let _ = reservation_origin;
                #[cfg(feature = "androidbox-apk-install0")]
                if let Some(operation) = compatible_activity_operation {
                    match operation.phase {
                        CompatibleActivityLaunchPhase::ClearRecent {
                            request_id: clear_request_id,
                            stale_identity,
                        } if clear_request_id == request_id => {
                            if status != UiSystemUiRequestStatus::Accepted
                                || action != UiSystemUiAction::FinishCompatibleActivity
                                || compatible_identity != Some(stale_identity)
                                || reservation_origin.is_some()
                                || runtime.delivered_active_client != UiClientId::Launcher
                                || runtime.delivered_system_ui_mode != UiSystemUiMode::Home
                                || runtime.delivered_system_ui_recent.is_some()
                                || runtime.delivered_system_nav_pressed
                                || runtime.delivered_system_nav_reveal_px != 0
                            {
                                fail(FAIL_SURFACE_PROTOCOL);
                            }
                            let reserve = request_mobile_system_ui_recent(
                                &mut runtime,
                                UiSystemUiAction::ReserveCompatibleActivity,
                                Some(UiRecentIdentity::CompatibleAndroid(operation.identity)),
                            );
                            compatible_activity_operation =
                                Some(CompatibleActivityLaunchOperation {
                                    origin_recent: None,
                                    phase: CompatibleActivityLaunchPhase::Reservation {
                                        request_id: reserve.request_id,
                                    },
                                    ..operation
                                });
                            touch = TouchController::new();
                            continue;
                        }
                        CompatibleActivityLaunchPhase::Reservation {
                            request_id: reserve_request_id,
                        } if reserve_request_id == request_id => {
                            if action != UiSystemUiAction::ReserveCompatibleActivity
                                || compatible_identity != Some(operation.identity)
                            {
                                fail(FAIL_SURFACE_PROTOCOL);
                            }
                            if status != UiSystemUiRequestStatus::Accepted {
                                compatible_activity_operation = None;
                                touch = TouchController::new();
                                continue;
                            }
                            if reservation_origin != Some(operation.origin.reservation_origin())
                                || runtime.delivered_active_client != UiClientId::Launcher
                                || runtime.delivered_system_ui_mode != operation.origin.mode()
                                || runtime.delivered_system_ui_recent != operation.origin_recent
                                || runtime.delivered_system_nav_pressed
                                || !model.begin_android_installed_launch(
                                    operation.request_sequence,
                                    operation.identity.package_generation(),
                                )
                            {
                                fail(FAIL_SURFACE_PROTOCOL);
                            }
                            android_package_relaunch_sequence = operation.request_sequence;
                            if present_mobile_page(&mut runtime, UiClientId::Launcher, model)
                                != MobilePresentOutcome::Presented
                            {
                                fail(FAIL_SURFACE_PROTOCOL);
                            }

                            let expected = model.android_installed_app;
                            match request_android_installed_app_relaunch(
                                operation.request_sequence,
                                operation.identity,
                                expected,
                            ) {
                                Ok(actual) => {
                                    if actual != expected {
                                        fail(FAIL_SURFACE_PROTOCOL);
                                    }
                                    let commit = request_mobile_system_ui_recent(
                                        &mut runtime,
                                        operation.origin.commit_action(),
                                        Some(UiRecentIdentity::CompatibleAndroid(
                                            operation.identity,
                                        )),
                                    );
                                    compatible_activity_operation =
                                        Some(CompatibleActivityLaunchOperation {
                                            phase: CompatibleActivityLaunchPhase::Commit {
                                                request_id: commit.request_id,
                                                reserve_request_id,
                                                reserve_revision: revision,
                                            },
                                            ..operation
                                        });
                                }
                                Err(failure) => {
                                    if !model.fail_android_installed_launch(
                                        operation.request_sequence,
                                        operation.identity.package_generation(),
                                        failure,
                                    ) {
                                        fail(FAIL_SURFACE_PROTOCOL);
                                    }
                                    if failure == AndroidInstalledLaunchFailure::Stale {
                                        #[cfg(feature = "androidbox-multipackage4")]
                                        let _ = refresh_android_installed_catalog(&mut model, None);
                                        #[cfg(not(feature = "androidbox-multipackage4"))]
                                        {
                                            let latest = read_android_installed_app_status();
                                            let _ =
                                                model.apply_android_installed_app_status(latest);
                                        }
                                    }
                                    let stale_compatible_recent = failure
                                        == AndroidInstalledLaunchFailure::Stale
                                        && operation.origin_recent
                                            == Some(UiRecentIdentity::CompatibleAndroid(
                                                operation.identity,
                                            ));
                                    let abort_action = if stale_compatible_recent {
                                        UiSystemUiAction::FinishCompatibleActivity
                                    } else {
                                        UiSystemUiAction::AbortCompatibleActivityVerification
                                    };
                                    let abort = request_mobile_system_ui_recent(
                                        &mut runtime,
                                        abort_action,
                                        Some(UiRecentIdentity::CompatibleAndroid(
                                            operation.identity,
                                        )),
                                    );
                                    compatible_activity_operation =
                                        Some(CompatibleActivityLaunchOperation {
                                            phase: CompatibleActivityLaunchPhase::Abort {
                                                action: abort_action,
                                                request_id: abort.request_id,
                                                reserve_request_id,
                                                reserve_revision: revision,
                                            },
                                            ..operation
                                        });
                                }
                            }
                            touch = TouchController::new();
                            continue;
                        }
                        CompatibleActivityLaunchPhase::Commit {
                            request_id: commit_request_id,
                            reserve_request_id,
                            reserve_revision,
                        } if commit_request_id == request_id => {
                            if status != UiSystemUiRequestStatus::Accepted
                                || action != operation.origin.commit_action()
                                || compatible_identity != Some(operation.identity)
                                || reservation_origin != Some(operation.origin.reservation_origin())
                                || reserve_request_id == 0
                                || revision
                                    != reserve_revision
                                        .checked_add(1)
                                        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL))
                                || runtime.delivered_active_client != UiClientId::Launcher
                                || runtime.delivered_system_ui_mode != UiSystemUiMode::Foreground
                                || runtime.delivered_system_ui_recent
                                    != Some(UiRecentIdentity::CompatibleAndroid(operation.identity))
                                || runtime.delivered_system_nav_pressed
                                || !matches!(
                                    model.android_installed_launch,
                                    AndroidInstalledLaunchStatus::Pending {
                                        request_sequence,
                                        generation,
                                        apk_sha256,
                                    } if request_sequence == operation.request_sequence
                                        && generation
                                            == operation.identity.package_generation()
                                        && apk_sha256
                                            == model
                                                .android_installed_app
                                                .apk_digest_sha256
                                )
                                || !model.succeed_android_installed_launch(
                                    operation.request_sequence,
                                    operation.identity.package_generation(),
                                    model.android_installed_app.apk_digest_sha256,
                                )
                                || !model
                                    .bind_android_installed_activity_session(operation.identity)
                            {
                                fail(FAIL_SURFACE_PROTOCOL);
                            }
                            #[cfg(not(feature = "androidbox-el0-runtime0"))]
                            if model.page != MobilePage::AndroidDemo
                                && !model.apply(MobileAction::Open(MobilePage::AndroidDemo))
                            {
                                fail(FAIL_SURFACE_PROTOCOL);
                            }
                            if !model.installed_android_foreground_content_ready() {
                                fail(FAIL_SURFACE_PROTOCOL);
                            }
                            compatible_activity_operation = None;
                            #[cfg(not(feature = "androidbox-el0-runtime0"))]
                            {
                                let _ = present_mobile_enter_transition(
                                    &mut runtime,
                                    UiClientId::Launcher,
                                    &mut model,
                                );
                            }
                            touch = TouchController::new();
                            continue;
                        }
                        CompatibleActivityLaunchPhase::Abort {
                            action: abort_action,
                            request_id: abort_request_id,
                            reserve_request_id,
                            reserve_revision,
                        } if abort_request_id == request_id => {
                            if status != UiSystemUiRequestStatus::Accepted
                                || action != abort_action
                                || compatible_identity != Some(operation.identity)
                                || reservation_origin != Some(operation.origin.reservation_origin())
                                || reserve_request_id == 0
                                || revision
                                    != reserve_revision
                                        .checked_add(1)
                                        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL))
                            {
                                fail(FAIL_SURFACE_PROTOCOL);
                            }
                            compatible_activity_operation = None;
                            if runtime.delivered_active_client == UiClientId::Launcher {
                                let _ =
                                    present_mobile_page(&mut runtime, UiClientId::Launcher, model);
                            }
                            touch = TouchController::new();
                            continue;
                        }
                        _ => fail(FAIL_SURFACE_PROTOCOL),
                    }
                }
                touch = TouchController::new();
            }
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::App,
                app: Some(_),
                ..
            } => {
                model.unlock_reveal_px = 0;
                model.page_transition_offset_px = 0;
                touch = TouchController::new();
            }
            #[cfg(feature = "androidbox-el0-runtime0")]
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::App,
                app: None,
                ..
            } => {
                model.unlock_reveal_px = 0;
                model.page_transition_offset_px = 0;
                touch = TouchController::new();
            }
            UiServerEventPayload::Input(_) => {}
            UiServerEventPayload::Ready
            | UiServerEventPayload::Presented { .. }
            | UiServerEventPayload::PresentCancelled { .. }
            | UiServerEventPayload::Degraded { .. }
            | UiServerEventPayload::FocusChanged { .. } => fail(FAIL_SURFACE_PROTOCOL),
        }
    }
}

#[cfg(feature = "mobile-ui-runtime")]
fn mobile_app_loop(startup: u64) -> ! {
    let denied = syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0);
    if denied.status != Status::PermissionDenied.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_SURFACE_ACQUIRE);
    }
    #[cfg(feature = "androidbox-restart0")]
    let (channel, mut installed_activity_lease) = {
        let (surface, runtime, supervisor, init_pid) = read_android_app_bootstrap(startup);
        (
            surface,
            androidapp_process::AndroidAppClient::connect(runtime, supervisor, init_pid),
        )
    };
    #[cfg(all(feature = "androidbox-process0", not(feature = "androidbox-restart0")))]
    let (channel, mut installed_activity_lease) = {
        let (surface, runtime) = read_android_app_bootstrap(startup);
        (
            surface,
            androidapp_process::AndroidAppClient::connect(runtime),
        )
    };
    #[cfg(not(feature = "androidbox-process0"))]
    let channel = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let (server_pid, ready) = read_ui_event(channel.raw());
    let mut events = UiServerEventTracker::new();
    if server_pid == 0
        || !matches!(ready.payload(), UiServerEventPayload::Ready)
        || events.accept(ready).is_err()
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let (appearance_sender, initial_appearance) = read_ui_event(channel.raw());
    if appearance_sender != server_pid
        || events.accept(initial_appearance).is_err()
        || !matches!(
            initial_appearance.payload(),
            UiServerEventPayload::AppearanceChanged { revision: 1, .. }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let (clock_sender, initial_clock) = read_ui_event(channel.raw());
    if clock_sender != server_pid
        || events.accept(initial_clock).is_err()
        || !matches!(
            initial_clock.payload(),
            UiServerEventPayload::ClockChanged { revision: 1, .. }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let (notification_sender, initial_notification) = read_ui_event(channel.raw());
    if notification_sender != server_pid
        || events.accept(initial_notification).is_err()
        || !matches!(
            initial_notification.payload(),
            UiServerEventPayload::BootNotificationChanged {
                visible: true,
                revision: 1
            }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let (system_ui_sender, initial_system_ui) = read_ui_event(channel.raw());
    if system_ui_sender != server_pid
        || events.accept(initial_system_ui).is_err()
        || !matches!(
            initial_system_ui.payload(),
            UiServerEventPayload::SystemUiChanged {
                mode: UiSystemUiMode::Locked,
                recent: None,
                nav_pressed: false,
                nav_reveal_px: 0,
                revision: 1
            }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let (focus_sender, initial_focus) = read_ui_event(channel.raw());
    if focus_sender != server_pid
        || events.accept(initial_focus).is_err()
        || !matches!(
            initial_focus.payload(),
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::Launcher,
                app: None,
                focus_generation: 1
            }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut runtime = create_mobile_graphics_runtime(channel, server_pid, events);
    provision_app_buffer_while_unfocused(&mut runtime);
    let mut model = MobileModel::for_page(MobilePage::Settings);
    #[cfg(feature = "androidbox-apk-install0")]
    let _ = refresh_android_installed_catalog(&mut model, None);
    #[cfg(feature = "androidbox-runtime-install2")]
    let _ = model.apply_android_install_candidate_status(read_android_install_candidate_status());
    model.time = MobileTimeSnapshot::from_unix_seconds(runtime.delivered_clock_unix_seconds);
    let _ = apply_mobile_appearance(&mut model, runtime.delivered_appearance);
    let _ = apply_mobile_boot_notification(&mut model, runtime.delivered_boot_notification_visible);
    let _ = apply_mobile_system_ui(&mut model, &runtime);
    let mut touch = TouchController::new();
    #[cfg(all(
        feature = "androidbox-interactive0",
        not(feature = "androidbox-process0")
    ))]
    let mut installed_activity_lease = InstalledInteractiveActivityLease::new();
    #[cfg(feature = "androidbox-interactive0")]
    let mut installed_activity_lease_active = false;
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    let mut android_package_uninstall_sequence = 0_u64;
    #[cfg(feature = "androidbox-runtime-install2")]
    let mut android_package_install_sequence = 0_u64;

    loop {
        match next_mobile_server_payload(&mut runtime) {
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::App,
                app: Some(app),
                ..
            } => {
                #[cfg(feature = "androidbox-interactive0")]
                close_android_installed_activity_lease(
                    &mut installed_activity_lease,
                    &mut installed_activity_lease_active,
                    &mut model,
                );
                #[cfg(feature = "androidbox-apk-install0")]
                let _ = refresh_android_installed_catalog(&mut model, None);
                #[cfg(feature = "androidbox-runtime-install2")]
                let _ =
                    model.apply_android_install_candidate_status(
                        read_android_install_candidate_status(),
                    );
                model.page = page_for_shell_app(app);
                model.shade_open = false;
                model.shade_reveal_px = 0;
                model.drawer_open = false;
                model.drawer_reveal_px = 0;
                model.back_reveal_px = 0;
                model.back_origin_y = 0;
                model.unlock_reveal_px = 0;
                model.page_transition_offset_px = 0;
                model.pressed_target = None;
                touch = TouchController::new();
                let _ = present_mobile_enter_transition(&mut runtime, UiClientId::App, &mut model);
            }
            UiServerEventPayload::Input(sample)
                if runtime.delivered_active_client == UiClientId::App =>
            {
                if runtime.suppress_mobile_input_until_release {
                    if !sample.pressed() {
                        runtime.suppress_mobile_input_until_release = false;
                        touch = TouchController::new();
                    }
                    continue;
                }
                let Some(action) = observe_mobile_sample(&mut touch, model, sample) else {
                    continue;
                };
                match action {
                    MobileAction::Home => {
                        #[cfg(feature = "androidbox-interactive0")]
                        close_android_installed_activity_lease(
                            &mut installed_activity_lease,
                            &mut installed_activity_lease_active,
                            &mut model,
                        );
                        clear_mobile_press_and_present(&mut runtime, UiClientId::App, &mut model);
                        model.unlock_reveal_px = 0;
                        let _ = present_mobile_exit_transition(
                            &mut runtime,
                            UiClientId::App,
                            &mut model,
                        );
                        let transition_id = next_mobile_transition_id(&runtime.events);
                        let control =
                            UiClientControl::set_focus(UiClientId::Launcher, None, transition_id)
                                .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
                        write_ui_control(runtime.channel.raw(), control);
                        touch = TouchController::new();
                    }
                    MobileAction::Back if !model.shade_open && !model.has_in_app_back() => {
                        #[cfg(feature = "androidbox-interactive0")]
                        close_android_installed_activity_lease(
                            &mut installed_activity_lease,
                            &mut installed_activity_lease_active,
                            &mut model,
                        );
                        clear_mobile_press_and_present(&mut runtime, UiClientId::App, &mut model);
                        model.unlock_reveal_px = 0;
                        let _ = present_mobile_exit_transition(
                            &mut runtime,
                            UiClientId::App,
                            &mut model,
                        );
                        let transition_id = next_mobile_transition_id(&runtime.events);
                        let control =
                            UiClientControl::set_focus(UiClientId::Launcher, None, transition_id)
                                .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
                        write_ui_control(runtime.channel.raw(), control);
                        touch = TouchController::new();
                    }
                    MobileAction::Open(MobilePage::Phone)
                    | MobileAction::Open(MobilePage::Messages)
                    | MobileAction::Open(MobilePage::Calculator)
                    | MobileAction::Open(MobilePage::AndroidDemo)
                    | MobileAction::Open(MobilePage::Home)
                    | MobileAction::Open(MobilePage::Lock) => {}
                    MobileAction::Open(
                        MobilePage::Settings | MobilePage::Apps | MobilePage::About,
                    )
                    | MobileAction::Back
                    | MobileAction::OpenShade
                    | MobileAction::CloseShade
                    | MobileAction::OpenDrawer
                    | MobileAction::CloseDrawer => {
                        if model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::SetShadeReveal(_)
                    | MobileAction::SetDrawerReveal(_)
                    | MobileAction::SetUnlockReveal(_)
                    | MobileAction::SetBootNotificationOffset(_)
                    | MobileAction::SetPageTransitionOffset(_)
                    | MobileAction::SetPageScroll { .. }
                    | MobileAction::SetBackReveal { .. }
                    | MobileAction::SetPressed(_) => {
                        if model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                        }
                    }
                    MobileAction::ToggleTheme => {
                        clear_mobile_press_and_present(&mut runtime, UiClientId::App, &mut model);
                        request_mobile_appearance(&mut runtime, UiAppearanceAction::ToggleTheme);
                        touch = TouchController::new();
                    }
                    MobileAction::ToggleAccent => {
                        clear_mobile_press_and_present(&mut runtime, UiClientId::App, &mut model);
                        request_mobile_appearance(&mut runtime, UiAppearanceAction::ToggleAccent);
                        touch = TouchController::new();
                    }
                    MobileAction::SetSoftwareDimming(level) => {
                        clear_mobile_press_and_present(&mut runtime, UiClientId::App, &mut model);
                        request_mobile_appearance(&mut runtime, software_dimming_action(level));
                    }
                    MobileAction::ActivateBootNotification => {
                        if model.page != MobilePage::Lock && model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                        }
                        touch = TouchController::new();
                    }
                    MobileAction::DismissBootNotification => {
                        if model.boot_notification_visible {
                            request_mobile_boot_notification_dismiss(&mut runtime);
                        }
                        touch = TouchController::new();
                    }
                    #[cfg(feature = "androidbox-multipackage4")]
                    MobileAction::SelectInstalledAndroid(_) => {
                        if model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                        }
                        touch = TouchController::new();
                    }
                    #[cfg(feature = "androidbox-runtime-install2")]
                    MobileAction::BeginAndroidInstall(_) | MobileAction::CancelAndroidInstall => {
                        if model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                        }
                        touch = TouchController::new();
                    }
                    #[cfg(feature = "androidbox-runtime-install2")]
                    MobileAction::ConfirmAndroidInstall(candidate_id) => {
                        let expected = model.android_install_candidate;
                        let request_sequence = android_package_install_sequence
                            .checked_add(1)
                            .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                        let _ = model.apply(action);
                        if !expected.present
                            || expected.candidate_id != candidate_id
                            || !model.mark_android_install_pending(request_sequence, candidate_id)
                        {
                            fail(FAIL_SURFACE_PROTOCOL);
                        }
                        #[cfg(feature = "androidbox-interactive0")]
                        close_android_installed_activity_lease(
                            &mut installed_activity_lease,
                            &mut installed_activity_lease_active,
                            &mut model,
                        );
                        let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                        android_package_install_sequence = request_sequence;
                        let result = request_android_package_install(request_sequence, expected);
                        #[cfg(feature = "androidbox-multipackage4")]
                        let _ = refresh_android_installed_catalog(
                            &mut model,
                            Some(expected.package.as_str()),
                        );
                        #[cfg(not(feature = "androidbox-multipackage4"))]
                        let current = read_android_installed_app_status();
                        #[cfg(feature = "androidbox-multipackage4")]
                        let current = model.android_installed_app;
                        let candidate = read_android_install_candidate_status();
                        if result.is_ok() && (!current.installed || candidate.present) {
                            fail(FAIL_SURFACE_PROTOCOL);
                        }
                        let _ = model.apply_android_installed_app_status(current);
                        let _ = model.apply_android_install_candidate_status(candidate);
                        if !model.finish_android_install(request_sequence, expected, result) {
                            fail(FAIL_SURFACE_PROTOCOL);
                        }
                        let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                        touch = TouchController::new();
                    }
                    #[cfg(feature = "androidbox-runtime-uninstall1")]
                    MobileAction::BeginInstalledAndroidUninstall(_)
                    | MobileAction::CancelInstalledAndroidUninstall => {
                        if model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                        }
                        touch = TouchController::new();
                    }
                    #[cfg(feature = "androidbox-runtime-uninstall1")]
                    MobileAction::ConfirmInstalledAndroidUninstall(generation) => {
                        let expected = model.android_installed_app;
                        let request_sequence = android_package_uninstall_sequence
                            .checked_add(1)
                            .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
                        let _ = model.apply(action);
                        if !expected.installed
                            || generation != expected.generation
                            || !model.mark_android_installed_uninstall_pending(
                                request_sequence,
                                generation,
                            )
                        {
                            fail(FAIL_SURFACE_PROTOCOL);
                        }
                        #[cfg(feature = "androidbox-interactive0")]
                        close_android_installed_activity_lease(
                            &mut installed_activity_lease,
                            &mut installed_activity_lease_active,
                            &mut model,
                        );
                        let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                        android_package_uninstall_sequence = request_sequence;
                        let result =
                            request_android_installed_app_uninstall(request_sequence, expected);
                        #[cfg(feature = "androidbox-multipackage4")]
                        let _ = refresh_android_installed_catalog(&mut model, None);
                        #[cfg(not(feature = "androidbox-multipackage4"))]
                        let current = read_android_installed_app_status();
                        #[cfg(feature = "androidbox-multipackage4")]
                        let target_still_installed = model.android_installed_apps
                            [..usize::from(model.android_installed_app_count)]
                            .iter()
                            .any(|app| app.package == expected.package);
                        #[cfg(not(feature = "androidbox-multipackage4"))]
                        let target_still_installed = current.installed;
                        if result.is_ok() && target_still_installed {
                            fail(FAIL_SURFACE_PROTOCOL);
                        }
                        #[cfg(not(feature = "androidbox-multipackage4"))]
                        let _ = model.apply_android_installed_app_status(current);
                        if !model.finish_android_installed_uninstall(
                            request_sequence,
                            generation,
                            result,
                        ) {
                            fail(FAIL_SURFACE_PROTOCOL);
                        }
                        let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                        touch = TouchController::new();
                    }
                    MobileAction::PhoneKey(_)
                    | MobileAction::PhoneBackspace
                    | MobileAction::CalculatorKey(_)
                    | MobileAction::OpenMessage(_) => {
                        if model.apply(action) {
                            let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                        }
                        touch = TouchController::new();
                    }
                    #[cfg(feature = "androidbox-interactive0")]
                    MobileAction::ActivateInstalledAndroidButton(view_id) => {
                        #[cfg(not(feature = "androidbox-scene-rpc2"))]
                        let current = model.android_installed_activity_view;
                        #[cfg(all(
                            feature = "androidbox-scene-rpc2",
                            not(feature = "androidbox-multiaction3")
                        ))]
                        let callback_button_matches =
                            model.android_installed_activity_scene.callback_button_id()
                                == Some(view_id);
                        #[cfg(feature = "androidbox-multiaction3")]
                        let callback_button_matches = model
                            .android_installed_activity_scene
                            .is_callback_button(view_id);
                        #[cfg(not(feature = "androidbox-scene-rpc2"))]
                        let callback_button_matches = current.button_view_id() == view_id;
                        if !installed_activity_lease_active
                            || !model.installed_android_interactive_content_ready()
                            || !callback_button_matches
                        {
                            close_android_installed_activity_lease(
                                &mut installed_activity_lease,
                                &mut installed_activity_lease_active,
                                &mut model,
                            );
                            finish_failed_android_installed_activity(&mut runtime);
                            touch = TouchController::new();
                            continue;
                        }
                        let _ = model.apply(action);
                        let update = match installed_activity_lease.dispatch_button_click(view_id) {
                            Ok(update) => update,
                            Err(_) => {
                                close_android_installed_activity_lease(
                                    &mut installed_activity_lease,
                                    &mut installed_activity_lease_active,
                                    &mut model,
                                );
                                finish_failed_android_installed_activity(&mut runtime);
                                touch = TouchController::new();
                                continue;
                            }
                        };
                        #[cfg(feature = "androidbox-dex-methods8")]
                        if update.app_defined_call_count > 1 {
                            close_android_installed_activity_lease(
                                &mut installed_activity_lease,
                                &mut installed_activity_lease_active,
                                &mut model,
                            );
                            finish_failed_android_installed_activity(&mut runtime);
                            touch = TouchController::new();
                            continue;
                        }
                        #[cfg(feature = "androidbox-dex-instance9")]
                        if update.app_defined_instance_call_count > update.app_defined_call_count {
                            close_android_installed_activity_lease(
                                &mut installed_activity_lease,
                                &mut installed_activity_lease_active,
                                &mut model,
                            );
                            finish_failed_android_installed_activity(&mut runtime);
                            touch = TouchController::new();
                            continue;
                        }
                        #[cfg(feature = "androidbox-string-text12")]
                        if update.activity_field_read_count > 2
                            || (update.activity_int_state_value == 0
                                && (update.direct_string_text
                                    || update.activity_field_read_count > 1))
                            || (update.activity_int_state_value != 0
                                && (update.activity_field_read_count != 2
                                    || if update.direct_string_text {
                                        update.app_defined_call_count != 0
                                            || update.app_defined_instance_call_count != 0
                                    } else {
                                        update.app_defined_call_count != 1
                                            || update.app_defined_instance_call_count != 1
                                    }))
                            || {
                                #[cfg(feature = "androidbox-string-builder13")]
                                {
                                    update.dynamic_string_text && !update.direct_string_text
                                }
                                #[cfg(not(feature = "androidbox-string-builder13"))]
                                {
                                    false
                                }
                            }
                        {
                            close_android_installed_activity_lease(
                                &mut installed_activity_lease,
                                &mut installed_activity_lease_active,
                                &mut model,
                            );
                            finish_failed_android_installed_activity(&mut runtime);
                            touch = TouchController::new();
                            continue;
                        }
                        #[cfg(all(
                            feature = "androidbox-activity-state11",
                            not(feature = "androidbox-string-text12")
                        ))]
                        if update.activity_field_read_count > 2
                            || (update.activity_int_state_value == 0
                                && update.activity_field_read_count > 1)
                            || (update.activity_int_state_value != 0
                                && (update.app_defined_call_count != 1
                                    || update.app_defined_instance_call_count != 1
                                    || update.activity_field_read_count != 2))
                        {
                            close_android_installed_activity_lease(
                                &mut installed_activity_lease,
                                &mut installed_activity_lease_active,
                                &mut model,
                            );
                            finish_failed_android_installed_activity(&mut runtime);
                            touch = TouchController::new();
                            continue;
                        }
                        #[cfg(all(
                            feature = "androidbox-activity-fields10",
                            not(feature = "androidbox-activity-state11")
                        ))]
                        if update.activity_field_read_count > 1 {
                            close_android_installed_activity_lease(
                                &mut installed_activity_lease,
                                &mut installed_activity_lease_active,
                                &mut model,
                            );
                            finish_failed_android_installed_activity(&mut runtime);
                            touch = TouchController::new();
                            continue;
                        }
                        let Some(ui_revision) = u64::from(update.revision).checked_add(1) else {
                            close_android_installed_activity_lease(
                                &mut installed_activity_lease,
                                &mut installed_activity_lease_active,
                                &mut model,
                            );
                            finish_failed_android_installed_activity(&mut runtime);
                            touch = TouchController::new();
                            continue;
                        };
                        #[cfg(feature = "androidbox-scene-rpc2")]
                        let updated_model = model.update_android_installed_activity_scene_text(
                            update.label_id,
                            update.label_text.as_str(),
                            ui_revision,
                        );
                        #[cfg(not(feature = "androidbox-scene-rpc2"))]
                        let updated_model = update.label_id == current.label_view_id()
                            && model.set_android_installed_activity_view(
                                update.label_id,
                                update.label_text.as_str(),
                                current.button_view_id(),
                                current.button_text(),
                                true,
                                ui_revision,
                            );
                        if !updated_model {
                            close_android_installed_activity_lease(
                                &mut installed_activity_lease,
                                &mut installed_activity_lease_active,
                                &mut model,
                            );
                            finish_failed_android_installed_activity(&mut runtime);
                            touch = TouchController::new();
                            continue;
                        }
                        #[cfg(not(feature = "androidbox-process0"))]
                        let _ = update.execution;
                        let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                        touch = TouchController::new();
                    }
                    MobileAction::Unlock
                    | MobileAction::ActivateRecentApp
                    | MobileAction::CloseOverview
                    | MobileAction::LaunchInstalledAndroid(_)
                    | MobileAction::ExecuteAndroidBoxDex => {}
                }
            }
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::Launcher,
                app: None,
                ..
            } => {
                #[cfg(feature = "androidbox-interactive0")]
                close_android_installed_activity_lease(
                    &mut installed_activity_lease,
                    &mut installed_activity_lease_active,
                    &mut model,
                );
                model.shade_open = false;
                model.shade_reveal_px = 0;
                model.drawer_open = false;
                model.drawer_reveal_px = 0;
                model.back_reveal_px = 0;
                model.back_origin_y = 0;
                model.unlock_reveal_px = 0;
                model.page_transition_offset_px = 0;
                model.pressed_target = None;
                touch = TouchController::new();
            }
            #[cfg(feature = "androidbox-interactive0")]
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::App,
                app: None,
                ..
            } => {
                close_android_installed_activity_lease(
                    &mut installed_activity_lease,
                    &mut installed_activity_lease_active,
                    &mut model,
                );
                let Some(identity) = runtime
                    .delivered_system_ui_recent
                    .and_then(UiRecentIdentity::compatible_android)
                else {
                    finish_failed_android_installed_activity(&mut runtime);
                    touch = TouchController::new();
                    continue;
                };
                if runtime.delivered_system_ui_mode != UiSystemUiMode::Foreground
                    || runtime.delivered_system_nav_pressed
                    || runtime.delivered_system_nav_reveal_px != 0
                {
                    finish_failed_android_installed_activity(&mut runtime);
                    touch = TouchController::new();
                    continue;
                }
                #[cfg(feature = "androidbox-multipackage4")]
                let catalog = read_android_active_installed_app_status();
                #[cfg(not(feature = "androidbox-multipackage4"))]
                let catalog = read_android_installed_app_status();
                let Some(snapshot) = claim_and_open_android_installed_activity(
                    identity,
                    catalog,
                    &mut installed_activity_lease,
                ) else {
                    finish_failed_android_installed_activity(&mut runtime);
                    touch = TouchController::new();
                    continue;
                };
                installed_activity_lease_active = true;
                let request_token = identity.session_id();
                if !model.apply_android_installed_app_status(catalog)
                    && model.android_installed_app != catalog
                    || !model.begin_android_installed_launch(
                        request_token,
                        identity.package_generation(),
                    )
                    || !model.succeed_android_installed_launch(
                        request_token,
                        identity.package_generation(),
                        catalog.apk_digest_sha256,
                    )
                    || !model.bind_android_installed_activity_session(identity)
                {
                    close_android_installed_activity_lease(
                        &mut installed_activity_lease,
                        &mut installed_activity_lease_active,
                        &mut model,
                    );
                    finish_failed_android_installed_activity(&mut runtime);
                    touch = TouchController::new();
                    continue;
                }
                if model.page != MobilePage::AndroidDemo
                    && !model.apply(MobileAction::Open(MobilePage::AndroidDemo))
                {
                    close_android_installed_activity_lease(
                        &mut installed_activity_lease,
                        &mut installed_activity_lease_active,
                        &mut model,
                    );
                    finish_failed_android_installed_activity(&mut runtime);
                    touch = TouchController::new();
                    continue;
                }
                let Some(ui_revision) = u64::from(snapshot.revision).checked_add(1) else {
                    close_android_installed_activity_lease(
                        &mut installed_activity_lease,
                        &mut installed_activity_lease_active,
                        &mut model,
                    );
                    finish_failed_android_installed_activity(&mut runtime);
                    touch = TouchController::new();
                    continue;
                };
                #[cfg(feature = "androidbox-scene-rpc2")]
                let activity_published = model.set_android_installed_activity_scene(
                    &snapshot.scene_nodes[..usize::from(snapshot.scene_node_count)],
                    ui_revision,
                );
                #[cfg(not(feature = "androidbox-scene-rpc2"))]
                let activity_published = model.set_android_installed_activity_view(
                    snapshot.label_id,
                    snapshot.label_text.as_str(),
                    snapshot.button_id,
                    snapshot.button_text.as_str(),
                    true,
                    ui_revision,
                );
                if !activity_published {
                    close_android_installed_activity_lease(
                        &mut installed_activity_lease,
                        &mut installed_activity_lease_active,
                        &mut model,
                    );
                    finish_failed_android_installed_activity(&mut runtime);
                    touch = TouchController::new();
                    continue;
                }
                model.shade_open = false;
                model.shade_reveal_px = 0;
                model.drawer_open = false;
                model.drawer_reveal_px = 0;
                model.back_reveal_px = 0;
                model.back_origin_y = 0;
                model.unlock_reveal_px = 0;
                model.page_transition_offset_px = 0;
                model.pressed_target = None;
                touch = TouchController::new();
                if !model.installed_android_interactive_content_ready() {
                    close_android_installed_activity_lease(
                        &mut installed_activity_lease,
                        &mut installed_activity_lease_active,
                        &mut model,
                    );
                    finish_failed_android_installed_activity(&mut runtime);
                    continue;
                }
                #[cfg(not(feature = "androidbox-process0"))]
                let _ = snapshot.execution;
                let _ = present_mobile_enter_transition(&mut runtime, UiClientId::App, &mut model);
            }
            #[cfg(all(
                feature = "androidbox-el0-runtime0",
                not(feature = "androidbox-interactive0")
            ))]
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::App,
                app: None,
                ..
            } => {
                let Some(identity) = runtime
                    .delivered_system_ui_recent
                    .and_then(UiRecentIdentity::compatible_android)
                else {
                    finish_failed_android_installed_activity(&mut runtime);
                    touch = TouchController::new();
                    continue;
                };
                if runtime.delivered_system_ui_mode != UiSystemUiMode::Foreground
                    || runtime.delivered_system_nav_pressed
                    || runtime.delivered_system_nav_reveal_px != 0
                {
                    finish_failed_android_installed_activity(&mut runtime);
                    touch = TouchController::new();
                    continue;
                }
                #[cfg(feature = "androidbox-multipackage4")]
                let catalog = read_android_active_installed_app_status();
                #[cfg(not(feature = "androidbox-multipackage4"))]
                let catalog = read_android_installed_app_status();
                let Some(execution) =
                    claim_and_execute_android_installed_activity(identity, catalog)
                else {
                    finish_failed_android_installed_activity(&mut runtime);
                    touch = TouchController::new();
                    continue;
                };
                let request_token = identity.session_id();
                if execution.status != catalog
                    || !model.apply_android_installed_app_status(execution.status)
                        && model.android_installed_app != execution.status
                    || !model.begin_android_installed_launch(
                        request_token,
                        identity.package_generation(),
                    )
                    || !model.succeed_android_installed_launch(
                        request_token,
                        identity.package_generation(),
                        execution.status.apk_digest_sha256,
                    )
                    || !model.bind_android_installed_activity_session(identity)
                {
                    finish_failed_android_installed_activity(&mut runtime);
                    touch = TouchController::new();
                    continue;
                }
                if model.page != MobilePage::AndroidDemo
                    && !model.apply(MobileAction::Open(MobilePage::AndroidDemo))
                {
                    finish_failed_android_installed_activity(&mut runtime);
                    touch = TouchController::new();
                    continue;
                }
                model.shade_open = false;
                model.shade_reveal_px = 0;
                model.drawer_open = false;
                model.drawer_reveal_px = 0;
                model.back_reveal_px = 0;
                model.back_origin_y = 0;
                model.unlock_reveal_px = 0;
                model.page_transition_offset_px = 0;
                model.pressed_target = None;
                touch = TouchController::new();
                if !model.installed_android_foreground_content_ready() {
                    finish_failed_android_installed_activity(&mut runtime);
                    continue;
                }
                let _ = (
                    execution.constructor_instruction_count,
                    execution.on_create_instruction_count,
                    execution.resources_arsc_crc32,
                    execution.layout_xml_crc32,
                    execution.layout_resource_id,
                    execution.text_resource_id,
                );
                let _ = present_mobile_enter_transition(&mut runtime, UiClientId::App, &mut model);
            }
            UiServerEventPayload::AppearanceChanged { .. } => {
                if apply_mobile_appearance(&mut model, runtime.delivered_appearance)
                    && runtime.delivered_active_client == UiClientId::App
                {
                    let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                }
            }
            UiServerEventPayload::ClockChanged { .. } => {
                let next =
                    MobileTimeSnapshot::from_unix_seconds(runtime.delivered_clock_unix_seconds);
                if model.time != next {
                    model.time = next;
                    if runtime.delivered_active_client == UiClientId::App {
                        let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                    }
                }
            }
            UiServerEventPayload::BootNotificationChanged { .. } => {
                if apply_mobile_boot_notification(
                    &mut model,
                    runtime.delivered_boot_notification_visible,
                ) && runtime.delivered_active_client == UiClientId::App
                {
                    let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                }
                touch = TouchController::new();
            }
            UiServerEventPayload::SystemUiChanged { .. } => {
                let was_nav_pressed = model.system_nav_pressed;
                let changed = apply_mobile_system_ui(&mut model, &runtime);
                let app_needs_system_layer_frame = runtime.delivered_active_client
                    == UiClientId::App
                    && runtime.delivered_system_ui_mode == UiSystemUiMode::Foreground
                    && (runtime.delivered_system_nav_pressed || was_nav_pressed)
                    && {
                        #[cfg(feature = "androidbox-el0-runtime0")]
                        {
                            // A compatible Activity has no ShellAppId and may
                            // submit only a stable, reservation-free epoch.
                            // SurfaceServer owns the in-progress system-nav
                            // capture; App redraws once cancellation restores
                            // the stable Foreground state.
                            runtime.events.active_app().is_some()
                                || !runtime.delivered_system_nav_pressed
                        }
                        #[cfg(not(feature = "androidbox-el0-runtime0"))]
                        {
                            true
                        }
                    };
                if changed && app_needs_system_layer_frame {
                    let _ = present_mobile_page(&mut runtime, UiClientId::App, model);
                }
                touch = TouchController::new();
            }
            UiServerEventPayload::Input(_) => {}
            UiServerEventPayload::Ready
            | UiServerEventPayload::Presented { .. }
            | UiServerEventPayload::PresentCancelled { .. }
            | UiServerEventPayload::Degraded { .. }
            | UiServerEventPayload::SystemUiRequestCompleted { .. }
            | UiServerEventPayload::FocusChanged { .. } => fail(FAIL_SURFACE_PROTOCOL),
        }
    }
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn launcher_loop(startup: u64) -> ! {
    let denied = syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0);
    if denied.status != Status::PermissionDenied.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_SURFACE_ACQUIRE);
    }
    let channel = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let (server_pid, ready) = read_ui_event(channel.raw());
    let mut events = UiServerEventTracker::new();
    if server_pid == 0
        || !matches!(ready.payload(), UiServerEventPayload::Ready)
        || events.accept(ready).is_err()
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let (focus_sender, initial_focus) = read_ui_event(channel.raw());
    if focus_sender != server_pid
        || events.accept(initial_focus).is_err()
        || !matches!(
            initial_focus.payload(),
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::Launcher,
                app: None,
                focus_generation: 1
            }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut launcher = LauncherRuntime {
        channel,
        server_pid,
        events,
        shell: ShellController::new(),
        deferred_transition: None,
    };
    present_home(&mut launcher);
    drain_deferred_transitions(&mut launcher);

    loop {
        let (sender_pid, event) = read_ui_event(launcher.channel.raw());
        let payload = event.payload();
        let transition = accept_launcher_event(&mut launcher, sender_pid, event, false);
        if let Some(transition) = transition {
            request_shell_transition(&mut launcher, transition);
        }
        if let UiServerEventPayload::FocusChanged {
            active_client, app, ..
        } = payload
        {
            accept_launcher_focus(&mut launcher, active_client, app);
        }
        drain_deferred_transitions(&mut launcher);
    }
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn app_loop(startup: u64) -> ! {
    let denied = syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0);
    if denied.status != Status::PermissionDenied.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_SURFACE_ACQUIRE);
    }
    let created = syscall(
        SyscallNumber::GraphicsBufferCreate,
        GRAPHICS_BUFFER_FORMAT_XRGB8888,
        pack_graphics_buffer_geometry(GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_HEIGHT),
        GRAPHICS_BUFFER_CREATE_FLAGS_NONE,
    );
    if created.status != Status::Ok.raw()
        || created.out1 == 0
        || created.out2 != GRAPHICS_BUFFER_LOGICAL_BYTES as u64
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let buffer = OwnedUserHandle::new(created.out1).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let duplicated = syscall(
        SyscallNumber::HandleDuplicate,
        buffer.raw(),
        u64::from(Rights::GRAPHICS_BUFFER_SERVER.bits()),
        0,
    );
    if duplicated.status != Status::Ok.raw() || duplicated.out1 == 0 || duplicated.out2 != 0 {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let buffer_for_server =
        OwnedUserHandle::new(duplicated.out1).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let channel = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let (server_pid, ready) = read_ui_event(channel.raw());
    let mut events = UiServerEventTracker::new();
    if server_pid == 0
        || !matches!(ready.payload(), UiServerEventPayload::Ready)
        || events.accept(ready).is_err()
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let (focus_sender, initial_focus) = read_ui_event(channel.raw());
    if focus_sender != server_pid
        || events.accept(initial_focus).is_err()
        || !matches!(
            initial_focus.payload(),
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::Launcher,
                app: None,
                focus_generation: 1
            }
        )
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut app_runtime = AppRuntime {
        channel,
        server_pid,
        events,
        buffer,
        buffer_for_server: Some(buffer_for_server),
        buffer_generation: 0,
    };
    provision_app_buffer_while_unfocused(&mut app_runtime);
    loop {
        let (sender_pid, event) = read_ui_event(app_runtime.channel.raw());
        if sender_pid != app_runtime.server_pid || app_runtime.events.accept(event).is_err() {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        match event.payload() {
            UiServerEventPayload::Input(_) => {}
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::App,
                app: Some(shell_app),
                ..
            } => render_focused_app(&mut app_runtime, shell_app),
            UiServerEventPayload::FocusChanged {
                active_client: UiClientId::Launcher,
                app: None,
                ..
            } => {}
            UiServerEventPayload::Ready
            | UiServerEventPayload::Presented { .. }
            | UiServerEventPayload::PresentCancelled { .. }
            | UiServerEventPayload::Degraded { .. }
            | UiServerEventPayload::AppearanceChanged { .. }
            | UiServerEventPayload::ClockChanged { .. }
            | UiServerEventPayload::BootNotificationChanged { .. }
            | UiServerEventPayload::SystemUiChanged { .. }
            | UiServerEventPayload::SystemUiRequestCompleted { .. }
            | UiServerEventPayload::FocusChanged { .. } => fail(FAIL_SURFACE_PROTOCOL),
        }
    }
}

/// Moves the attenuated graphics-buffer handle to SurfaceServer before the
/// resident-process topology is observed.  The initial focus belongs to the
/// Launcher, so frame 1 is required to take the protocol's transactional
/// cancellation path: neither the client-local nor global frame sequence is
/// consumed, and the same client frame id remains available for the first
/// focused render.
fn provision_app_buffer_while_unfocused(runtime: &mut AppRuntime) {
    if runtime.events.active_client() != Some(UiClientId::Launcher)
        || runtime.events.active_app().is_some()
        || runtime.events.last_focus_generation() != Some(1)
        || runtime.events.last_frame_id().is_some()
        || runtime.events.outstanding_frame_id().is_some()
        || runtime.buffer_generation != 0
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }

    let seed = COLOR_PHONE_SCREEN.to_le_bytes();
    let written = syscall(
        SyscallNumber::GraphicsBufferWrite,
        runtime.buffer.raw(),
        seed.as_ptr() as u64,
        pack_graphics_buffer_write(0, seed.len() as u32),
    );
    if written.status != Status::Ok.raw() || written.out1 != seed.len() as u64 || written.out2 != 1
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    runtime.buffer_generation = written.out2;

    let frame_id = 1;
    if runtime.events.begin_present(frame_id).is_err() {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let frame = BufferPresent::client(frame_id, 1, runtime.buffer_generation)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let transfer = runtime
        .buffer_for_server
        .take()
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    write_buffer_present(runtime.channel.raw(), frame, Some(transfer));

    let (sender_pid, event) = read_ui_event(runtime.channel.raw());
    if sender_pid != runtime.server_pid
        || !matches!(
            event.payload(),
            UiServerEventPayload::PresentCancelled {
                frame_id: 1,
                focus_generation: 1
            }
        )
        || runtime.events.accept(event).is_err()
        || runtime.events.last_frame_id().is_some()
        || runtime.events.last_commit().is_some()
        || runtime.events.outstanding_frame_id().is_some()
        || runtime.events.active_client() != Some(UiClientId::Launcher)
        || runtime.events.last_focus_generation() != Some(1)
        || runtime.buffer_generation != 1
        || runtime.buffer_for_server.is_some()
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn accept_launcher_event(
    launcher: &mut LauncherRuntime,
    sender_pid: u64,
    event: UiServerEvent,
    awaiting_present: bool,
) -> Option<ShellTransition> {
    if sender_pid != launcher.server_pid || launcher.events.accept(event).is_err() {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    match event.payload() {
        UiServerEventPayload::Input(sample) => {
            let transition = launcher
                .shell
                .observe(sample.x(), sample.y(), sample.pressed())
                .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
            if awaiting_present {
                if let Some(transition) = transition
                    && launcher.deferred_transition.replace(transition).is_some()
                {
                    fail(FAIL_SURFACE_PROTOCOL);
                }
                None
            } else {
                transition
            }
        }
        UiServerEventPayload::Presented { .. } if awaiting_present => None,
        UiServerEventPayload::PresentCancelled { .. } if awaiting_present => None,
        UiServerEventPayload::FocusChanged { .. } => None,
        UiServerEventPayload::Ready
        | UiServerEventPayload::Presented { .. }
        | UiServerEventPayload::PresentCancelled { .. }
        | UiServerEventPayload::Degraded { .. }
        | UiServerEventPayload::AppearanceChanged { .. }
        | UiServerEventPayload::ClockChanged { .. }
        | UiServerEventPayload::BootNotificationChanged { .. }
        | UiServerEventPayload::SystemUiChanged { .. }
        | UiServerEventPayload::SystemUiRequestCompleted { .. } => fail(FAIL_SURFACE_PROTOCOL),
    }
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn drain_deferred_transitions(launcher: &mut LauncherRuntime) {
    while let Some(transition) = launcher.deferred_transition.take() {
        request_shell_transition(launcher, transition);
    }
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn request_shell_transition(launcher: &mut LauncherRuntime, transition: ShellTransition) {
    if transition.transition_id != launcher.shell.transitions() {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let control = match transition.to {
        ShellView::Home => {
            UiClientControl::set_focus(UiClientId::Launcher, None, transition.transition_id)
        }
        ShellView::App(app) => {
            UiClientControl::set_focus(UiClientId::App, Some(app), transition.transition_id)
        }
    }
    .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    write_ui_control(launcher.channel.raw(), control);
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn accept_launcher_focus(
    launcher: &mut LauncherRuntime,
    active_client: UiClientId,
    app: Option<ShellAppId>,
) {
    match (active_client, app, launcher.shell.view()) {
        (UiClientId::Launcher, None, ShellView::Home) => present_home(launcher),
        (UiClientId::App, Some(active), ShellView::App(expected)) if active == expected => {}
        _ => fail(FAIL_SURFACE_PROTOCOL),
    }
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn present_home(launcher: &mut LauncherRuntime) {
    let frame_id = next_surface_frame_id(launcher);
    let rects = [
        surface_rect(0, 0, 208, 48, COLOR_PHONE_HEADER),
        surface_rect(16, 68, 176, 72, COLOR_CARD_CYAN),
        surface_rect(16, 156, 176, 72, COLOR_CARD_PURPLE),
        surface_rect(16, 244, 176, 72, COLOR_CARD_GREEN),
    ];
    let frame = PresentFrame::full(frame_id, COLOR_PHONE_SCREEN, &rects)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let _ = present_surface_frame(launcher, &frame);
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn render_focused_app(runtime: &mut AppRuntime, initial_app: ShellAppId) {
    let mut expected_app = initial_app;
    loop {
        if runtime.events.active_client() != Some(UiClientId::App)
            || runtime.events.active_app() != Some(expected_app)
        {
            return;
        }
        if present_app(runtime, expected_app) {
            return;
        }
        let Some(active_app) = runtime.events.active_app() else {
            return;
        };
        expected_app = active_app;
    }
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn present_app(runtime: &mut AppRuntime, app: ShellAppId) -> bool {
    for stage in 1..=3 {
        let frame_id = next_app_frame_id(runtime);
        if !present_app_buffer(runtime, app, stage, frame_id) {
            return false;
        }
    }
    true
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn next_surface_frame_id(launcher: &LauncherRuntime) -> u32 {
    launcher
        .events
        .last_frame_id()
        .unwrap_or(0)
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL))
}

fn next_app_frame_id(runtime: &AppRuntime) -> u32 {
    runtime
        .events
        .last_frame_id()
        .unwrap_or(0)
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL))
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn surface_rect(x: u8, y: u16, width: u8, height: u16, color: u32) -> SolidRect {
    SolidRect::try_new(x, y, width, height, color).unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL))
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn present_surface_frame(launcher: &mut LauncherRuntime, frame: &PresentFrame) -> bool {
    if frame.frame_id() != next_surface_frame_id(launcher)
        || launcher.events.begin_present(frame.frame_id()).is_err()
    {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let focus_generation = launcher
        .events
        .last_focus_generation()
        .and_then(|generation| u32::try_from(generation).ok())
        .filter(|_| launcher.events.active_client() == Some(UiClientId::Launcher))
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let wire = (*frame)
        .with_focus_generation(focus_generation)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL))
        .encode();
    write_ui_wire(launcher.channel.raw(), &wire);
    loop {
        let (sender_pid, event) = read_ui_event(launcher.channel.raw());
        let presented = matches!(
            event.payload(),
            UiServerEventPayload::Presented {
                frame_id,
                commit: _
            } if frame_id == frame.frame_id()
        );
        let cancelled = matches!(
            event.payload(),
            UiServerEventPayload::PresentCancelled {
                frame_id,
                focus_generation: _
            } if frame_id == frame.frame_id()
        );
        let _ = accept_launcher_event(launcher, sender_pid, event, true);
        if presented {
            return true;
        }
        if cancelled {
            return false;
        }
    }
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn present_app_buffer(runtime: &mut AppRuntime, app: ShellAppId, stage: u8, frame_id: u32) -> bool {
    if frame_id != next_app_frame_id(runtime) || runtime.events.begin_present(frame_id).is_err() {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let buffer_generation = write_app_buffer(runtime, app, stage);
    let focus_generation = runtime
        .events
        .last_focus_generation()
        .and_then(|generation| u32::try_from(generation).ok())
        .filter(|_| runtime.events.active_client() == Some(UiClientId::App))
        .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
    let frame = BufferPresent::client(frame_id, focus_generation, buffer_generation)
        .unwrap_or_else(|_| fail(FAIL_SURFACE_PROTOCOL));
    let transfer = runtime.buffer_for_server.take();
    write_buffer_present(runtime.channel.raw(), frame, transfer);
    loop {
        let (sender_pid, event) = read_ui_event(runtime.channel.raw());
        let presented = matches!(
            event.payload(),
            UiServerEventPayload::Presented {
                frame_id: presented_frame,
                commit: _
            } if presented_frame == frame_id
        );
        let cancelled = matches!(
            event.payload(),
            UiServerEventPayload::PresentCancelled {
                frame_id: cancelled_frame,
                focus_generation: _
            } if cancelled_frame == frame_id
        );
        if sender_pid != runtime.server_pid || runtime.events.accept(event).is_err() {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        match event.payload() {
            UiServerEventPayload::Input(_)
            | UiServerEventPayload::Presented { .. }
            | UiServerEventPayload::PresentCancelled { .. }
            | UiServerEventPayload::FocusChanged { .. } => {}
            UiServerEventPayload::Ready
            | UiServerEventPayload::Degraded { .. }
            | UiServerEventPayload::AppearanceChanged { .. }
            | UiServerEventPayload::ClockChanged { .. }
            | UiServerEventPayload::BootNotificationChanged { .. }
            | UiServerEventPayload::SystemUiChanged { .. }
            | UiServerEventPayload::SystemUiRequestCompleted { .. } => fail(FAIL_SURFACE_PROTOCOL),
        }
        if presented {
            return true;
        }
        if cancelled {
            return false;
        }
    }
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn write_app_buffer(runtime: &mut AppRuntime, app: ShellAppId, stage: u8) -> u64 {
    if !(1..=3).contains(&stage) {
        fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut scratch = [0_u8; GRAPHICS_BUFFER_WRITE_MAX_BYTES];
    let mut offset = 0_usize;
    while offset < GRAPHICS_BUFFER_LOGICAL_BYTES {
        let length = (GRAPHICS_BUFFER_LOGICAL_BYTES - offset).min(scratch.len());
        if length == 0 || !length.is_multiple_of(4) || !offset.is_multiple_of(4) {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        for (pixel_offset, bytes) in scratch[..length].chunks_exact_mut(4).enumerate() {
            let pixel_index = offset / 4 + pixel_offset;
            let x = pixel_index % GRAPHICS_BUFFER_WIDTH as usize;
            let y = pixel_index / GRAPHICS_BUFFER_WIDTH as usize;
            bytes.copy_from_slice(&app_buffer_pixel(app, stage, x, y).to_le_bytes());
        }
        let result = syscall(
            SyscallNumber::GraphicsBufferWrite,
            runtime.buffer.raw(),
            scratch.as_ptr() as u64,
            pack_graphics_buffer_write(offset as u32, length as u32),
        );
        let expected_generation = runtime
            .buffer_generation
            .checked_add(1)
            .unwrap_or_else(|| fail(FAIL_SURFACE_PROTOCOL));
        if result.status != Status::Ok.raw()
            || result.out1 != length as u64
            || result.out2 != expected_generation
        {
            fail(FAIL_SURFACE_PROTOCOL);
        }
        runtime.buffer_generation = result.out2;
        offset += length;
    }
    runtime.buffer_generation
}

#[cfg(not(feature = "mobile-ui-runtime"))]
fn app_buffer_pixel(app: ShellAppId, stage: u8, x: usize, y: usize) -> u32 {
    let accent = match app {
        ShellAppId::Phone => COLOR_CARD_CYAN,
        ShellAppId::Messages => COLOR_CARD_PURPLE,
        ShellAppId::Settings => COLOR_CARD_GREEN,
    };
    let mut color = COLOR_PHONE_SCREEN;
    if y < 48 {
        color = accent;
    }
    if contains_local(x, y, 16, 18, 48, 12) {
        color = COLOR_PHONE_HEADER;
    }
    if contains_local(x, y, 16, 64, 176, 248) {
        color = COLOR_APP_PANEL;
    }
    if contains_local(x, y, 72, 80, 64, 64) {
        color = accent;
    }
    // A one-pixel checker uses the same palette but proves that the client can
    // raster arbitrary pixels rather than only four solid protocol rects.
    if contains_local(x, y, 96, 104, 16, 16) && (x + y).is_multiple_of(2) {
        color = COLOR_APP_ROW;
    }
    if stage >= 2 {
        for (rx, ry, width, height, row_color) in [
            (32, 172, 112, 12, COLOR_APP_ROW),
            (32, 188, 144, 6, COLOR_APP_ROW_MUTED),
            (32, 212, 128, 12, COLOR_APP_ROW),
            (32, 228, 112, 6, COLOR_APP_ROW_MUTED),
        ] {
            if contains_local(x, y, rx, ry, width, height) {
                color = row_color;
            }
        }
    }
    if stage >= 3 {
        for (rx, ry, width, height, row_color) in [
            (32, 252, 96, 12, COLOR_APP_ROW),
            (32, 268, 136, 6, COLOR_APP_ROW_MUTED),
            (156, 248, 24, 20, COLOR_APP_TOGGLE),
        ] {
            if contains_local(x, y, rx, ry, width, height) {
                color = row_color;
            }
        }
    }
    color
}

#[cfg(not(feature = "mobile-ui-runtime"))]
const fn contains_local(
    x: usize,
    y: usize,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
) -> bool {
    x >= left && x < left + width && y >= top && y < top + height
}

fn manager_dispatch_m20_control(startup: u64, epoch: u16, state: &mut ManagerResidentState) {
    let envelope = read_channel_envelope_now(startup);
    if envelope.sender_pid() != state.supervisor_pid {
        if envelope.kind() == ChannelMessageKind::Transfer
            && let Some(received) =
                OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        {
            close_owned(received);
        }
        fail(FAIL_IDENTITY_SENDER);
    }
    match envelope.kind() {
        ChannelMessageKind::Transfer => {
            let (attach, channel) = attach_from_envelope(envelope);
            let expected_lease = if state.secondary_pid == 0 {
                M20_SECONDARY_FIRST_LEASE
            } else {
                state
                    .secondary_lease
                    .checked_add(1)
                    .unwrap_or_else(|| fail(FAIL_MULTI_CLIENT_ATTACH))
            };
            if epoch != SECOND_MANAGER_EPOCH
                || state.secondary_session.is_some()
                || attach.manager_epoch() != epoch
                || attach.role() != Role::Client
                || attach.owner_pid() == 0
                || attach.owner_pid() == state.client_pid
                || attach.owner_pid() == state.provider_pid
                || attach.owner_pid() == state.supervisor_pid
                || (state.secondary_pid != 0 && attach.owner_pid() != state.secondary_pid)
                || !matches!(
                    expected_lease,
                    M20_SECONDARY_FIRST_LEASE | M20_SECONDARY_REATTACHED_LEASE
                )
            {
                close_owned(channel);
                fail(FAIL_MULTI_CLIENT_ATTACH);
            }
            state.secondary_pid = attach.owner_pid();
            state.secondary_lease = expected_lease;
            state.secondary_session = Some(ResidentClientSession {
                channel,
                owner_pid: attach.owner_pid(),
                lease: expected_lease,
            });
            write_scalar(
                startup,
                M20_MANAGER_EVENT_TAG,
                m20_payload(M20_MANAGER_EVENT_ATTACHED, expected_lease, 0),
            );
        }
        ChannelMessageKind::Scalar => {
            let (tag, payload) = envelope
                .scalar_values()
                .unwrap_or_else(|| fail(FAIL_MULTI_CLIENT_PROTOCOL));
            let (phase, lease, transaction_id) = m20_payload_parts(payload);
            if tag != M20_MANAGER_CONTROL_TAG
                || transaction_id != 0
                || payload != m20_payload(phase, lease, transaction_id)
            {
                fail(FAIL_MULTI_CLIENT_PROTOCOL);
            }
            match phase {
                M20_MANAGER_CONTROL_REVOKE
                    if lease == state.secondary_lease && state.secondary_session.is_some() =>
                {
                    let session = state
                        .secondary_session
                        .take()
                        .unwrap_or_else(|| fail(FAIL_MULTI_CLIENT_CLEANUP));
                    close_owned(session.channel);
                    write_scalar(
                        startup,
                        M20_MANAGER_EVENT_TAG,
                        m20_payload(M20_MANAGER_EVENT_REVOKED, lease, 0),
                    );
                }
                M20_MANAGER_CONTROL_REVOKE
                    if lease < state.secondary_lease && state.secondary_session.is_some() =>
                {
                    write_scalar(
                        startup,
                        M20_MANAGER_EVENT_TAG,
                        m20_payload(M20_MANAGER_EVENT_STALE_REJECTED, lease, 0),
                    );
                }
                M20_MANAGER_CONTROL_FINALIZE
                    if lease == M20_SECONDARY_REATTACHED_LEASE
                        && lease == state.secondary_lease
                        && state.secondary_session.is_some() =>
                {
                    write_scalar(
                        startup,
                        M20_MANAGER_EVENT_TAG,
                        m20_payload(M20_MANAGER_EVENT_IDLE, lease, 0),
                    );
                }
                _ => fail(FAIL_MULTI_CLIENT_PROTOCOL),
            }
        }
        ChannelMessageKind::Bytes => fail(FAIL_MULTI_CLIENT_PROTOCOL),
    }
}

fn manager_dispatch_provider_request(state: &mut ManagerResidentState) -> ManagerDispatchOutcome {
    let envelope = read_channel_envelope_now(state.provider_session.raw());
    let sender_pid = envelope.sender_pid();
    match envelope.kind() {
        ChannelMessageKind::Transfer => {
            let Some(connector) = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
            else {
                return ManagerDispatchOutcome::Malformed;
            };
            if envelope.logical_length() != FRAME_SIZE {
                close_owned(connector);
                return ManagerDispatchOutcome::Malformed;
            }
            let Ok(register) = Frame::decode(envelope.data()) else {
                close_owned(connector);
                return ManagerDispatchOutcome::Malformed;
            };
            if register.opcode() != Opcode::Register {
                close_owned(connector);
                return ManagerDispatchOutcome::Malformed;
            }
            manager_dispatch_register(state, sender_pid, register, connector)
        }
        ChannelMessageKind::Bytes => {
            if envelope.logical_length() != FRAME_SIZE {
                return ManagerDispatchOutcome::Malformed;
            }
            let Ok(unregister) = Frame::decode(envelope.data()) else {
                return ManagerDispatchOutcome::Malformed;
            };
            if unregister.opcode() != Opcode::Unregister {
                return ManagerDispatchOutcome::Malformed;
            }
            manager_dispatch_unregister(state, sender_pid, unregister)
        }
        ChannelMessageKind::Scalar => ManagerDispatchOutcome::Malformed,
    }
}

fn manager_dispatch_register(
    state: &mut ManagerResidentState,
    sender_pid: u64,
    register: Frame,
    connector: OwnedUserHandle,
) -> ManagerDispatchOutcome {
    let transaction_id = register.transaction_id();
    let Some(name) = register.name() else {
        close_owned(connector);
        return ManagerDispatchOutcome::Malformed;
    };
    if sender_pid != state.provider_pid {
        close_owned(connector);
        write_frame_bytes(
            state.provider_session.raw(),
            build_frame(Frame::register_reply(
                transaction_id,
                name,
                ServiceStatus::PermissionDenied,
                0,
            )),
        );
        return ManagerDispatchOutcome::Unauthorized;
    }
    let resource = RegisteredService {
        connector,
        owner_pid: sender_pid,
    };
    let (status, instance) = match state.services.register(name, resource) {
        Ok(instance) => {
            let _ = registered_connector(&state.services, name, instance, sender_pid);
            (ServiceStatus::Ok, instance.raw())
        }
        Err(RegisterError::AlreadyExists { resource, .. }) => {
            close_registered_service(resource);
            (ServiceStatus::AlreadyExists, 0)
        }
        Err(RegisterError::Full { resource }) => {
            close_registered_service(resource);
            (ServiceStatus::OutOfMemory, 0)
        }
        Err(RegisterError::InstanceExhausted { resource }) => {
            close_registered_service(resource);
            (ServiceStatus::Unavailable, 0)
        }
    };
    write_frame_bytes(
        state.provider_session.raw(),
        build_frame(Frame::register_reply(
            transaction_id,
            name,
            status,
            instance,
        )),
    );
    ManagerDispatchOutcome::Authorized
}

fn manager_dispatch_unregister(
    state: &mut ManagerResidentState,
    sender_pid: u64,
    unregister: Frame,
) -> ManagerDispatchOutcome {
    let transaction_id = unregister.transaction_id();
    let (Some(name), Some(instance)) = (unregister.name(), unregister.instance()) else {
        return ManagerDispatchOutcome::Malformed;
    };
    let authorized = sender_pid == state.provider_pid;
    let (status, reply_instance) = if !authorized
        || name == echo_service_name()
        || state
            .services
            .lookup(&name)
            .is_some_and(|entry| entry.resource().owner_pid != sender_pid)
    {
        (ServiceStatus::PermissionDenied, 0)
    } else {
        match state.services.remove_exact(&name, instance) {
            Ok(removed) => {
                let (removed_name, removed_instance, resource) = removed.into_parts();
                if removed_name != name
                    || removed_instance != instance
                    || resource.owner_pid != sender_pid
                {
                    close_registered_service(resource);
                    fail(FAIL_PROVIDER_OWNER);
                }
                close_registered_service(resource);
                (ServiceStatus::Ok, instance.raw())
            }
            Err(RemoveError::NotFound) => (ServiceStatus::NotFound, 0),
            Err(RemoveError::InstanceMismatch { .. }) => (ServiceStatus::InvalidState, 0),
        }
    };
    write_frame_bytes(
        state.provider_session.raw(),
        build_frame(Frame::unregister_reply(
            transaction_id,
            name,
            status,
            reply_instance,
        )),
    );
    if authorized {
        ManagerDispatchOutcome::Authorized
    } else {
        ManagerDispatchOutcome::Unauthorized
    }
}

fn manager_dispatch_client_lookup(
    state: &ManagerResidentState,
    session: u64,
    expected_client_pid: u64,
) -> ManagerDispatchOutcome {
    let envelope = read_channel_envelope_now(session);
    if envelope.kind() == ChannelMessageKind::Transfer {
        if let Some(received) = OwnedUserHandle::new(u64::from(envelope.received_handle().raw())) {
            close_owned(received);
        }
        return ManagerDispatchOutcome::Malformed;
    }
    if envelope.kind() != ChannelMessageKind::Bytes || envelope.logical_length() != FRAME_SIZE {
        return ManagerDispatchOutcome::Malformed;
    }
    let sender_pid = envelope.sender_pid();
    let Ok(lookup) = Frame::decode(envelope.data()) else {
        return ManagerDispatchOutcome::Malformed;
    };
    if lookup.opcode() != Opcode::Lookup {
        return ManagerDispatchOutcome::Malformed;
    }
    let transaction_id = lookup.transaction_id();
    let Some(name) = lookup.name() else {
        return ManagerDispatchOutcome::Malformed;
    };
    if sender_pid != expected_client_pid {
        write_frame_bytes(
            session,
            build_frame(Frame::lookup_reply(
                transaction_id,
                name,
                ServiceStatus::PermissionDenied,
                0,
            )),
        );
        return ManagerDispatchOutcome::Unauthorized;
    }
    let Some(entry) = state.services.lookup(&name) else {
        write_frame_bytes(
            session,
            build_frame(Frame::lookup_reply(
                transaction_id,
                name,
                ServiceStatus::NotFound,
                0,
            )),
        );
        return ManagerDispatchOutcome::Authorized;
    };
    let instance = entry.instance();
    let connector = registered_connector(&state.services, name, instance, state.provider_pid);
    let (server_endpoint, client_endpoint) = create_channel_owned();
    write_frame_transfer(
        connector,
        build_frame(Frame::connect(transaction_id, name, instance)),
        server_endpoint,
    );
    write_frame_transfer(
        session,
        build_frame(Frame::lookup_reply(
            transaction_id,
            name,
            ServiceStatus::Ok,
            instance.raw(),
        )),
        client_endpoint,
    );
    ManagerDispatchOutcome::Authorized
}

fn provider_resident_loop(startup: u64, epoch: u16, state: ProviderResidentState) -> ! {
    publish_resident_ready(startup, Role::Provider, epoch, &state);
    let mut static_served = 0_u32;

    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut first_instance = None;
    let mut first_connector = None;
    let mut replacement_instance = None;
    let mut replacement_connector = None;
    let mut dynamic_done = false;
    let mut m15_replacement = None;
    let mut reuse_instance = None;
    let mut reuse_connector = None;
    let mut reuse_done = false;
    let mut identity_relayed = false;
    let mut m20_primary_pid = 0_u64;
    let mut m20_secondary_pid = 0_u64;
    let mut m20_armed = false;
    let mut m20_echoes_completed = 0_u8;
    let mut m20_stall_aborted = false;
    let mut m20_pending: [Option<PendingProviderEcho>; 2] = core::array::from_fn(|_| None);

    loop {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        let mut pending_index_by_wait = [usize::MAX; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(startup, requested);
        items[1] = pack_user_wait_item(state.connector.raw(), requested);
        let mut count = 2;
        for (pending_index, pending) in m20_pending.iter().enumerate() {
            if let Some(pending) = pending.as_ref() {
                items[count] = pack_user_wait_item(pending.endpoint.raw(), requested);
                pending_index_by_wait[count] = pending_index;
                count += 1;
            }
        }
        let ready = object_wait_many_array(&items, count, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw()
            || ready.out1 >= count as u64
            || ready.out2 & u64::from(requested.bits()) == 0
        {
            fail(FAIL_DYNAMIC_PROTOCOL);
        }
        if ready.out1 == 1 {
            if ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
                || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
            {
                fail(FAIL_DYNAMIC_PROTOCOL);
            }
            if m20_armed {
                provider_accept_m20_echo(
                    startup,
                    &state,
                    m20_primary_pid,
                    m20_secondary_pid,
                    &mut m20_pending,
                );
            } else {
                static_served = static_served
                    .checked_add(1)
                    .unwrap_or_else(|| fail(FAIL_MANAGER_PROTOCOL));
                provider_serve_static_echo_now(&state, static_served);
                if static_served == POST_READY_ECHO_ROUNDS {
                    write_scalar(
                        startup,
                        SERVING_DONE_TAG,
                        serving_done_payload(Role::Provider, static_served),
                    );
                } else if static_served > POST_READY_ECHO_ROUNDS {
                    fail(FAIL_MANAGER_PROTOCOL);
                }
            }
        } else if ready.out1 == 0 {
            if ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
                || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
            {
                fail(FAIL_DYNAMIC_PROTOCOL);
            }
            let (sender_pid, (tag, command)) = read_scalar_envelope_now(startup);
            if sender_pid != state.supervisor_pid {
                fail(FAIL_IDENTITY_SENDER);
            }
            if tag == DYNAMIC_COMMAND_TAG {
                match command {
                    DYNAMIC_REGISTER_FIRST_COMMAND => {
                        if first_instance.is_some() || first_connector.is_some() {
                            fail(FAIL_DYNAMIC_PROTOCOL);
                        }
                        let (connector, instance) =
                            provider_register_dynamic(&state, DYNAMIC_REGISTER_FIRST_TXID);
                        if instance.raw() <= state.instance.raw() {
                            fail(FAIL_DYNAMIC_INSTANCE);
                        }
                        write_dynamic_commit(
                            startup,
                            1,
                            DYNAMIC_REGISTER_FIRST_TXID,
                            instance.raw(),
                        );
                        provider_serve_dynamic_echo(
                            &connector,
                            DYNAMIC_LOOKUP_FIRST_TXID,
                            instance,
                        );
                        if read_scalar(startup)
                            != (DYNAMIC_COMMAND_TAG, DYNAMIC_CONFIRM_FIRST_ECHO_COMMAND)
                        {
                            fail(FAIL_DYNAMIC_PROTOCOL);
                        }
                        write_dynamic_commit(startup, 3, DYNAMIC_LOOKUP_FIRST_TXID, instance.raw());
                        first_instance = Some(instance);
                        first_connector = Some(connector);
                    }
                    DYNAMIC_UNREGISTER_FIRST_COMMAND => {
                        let instance = first_instance
                            .take()
                            .unwrap_or_else(|| fail(FAIL_DYNAMIC_INSTANCE));
                        let connector = first_connector
                            .take()
                            .unwrap_or_else(|| fail(FAIL_DYNAMIC_INSTANCE));
                        provider_unregister_dynamic(
                            &state,
                            DYNAMIC_UNREGISTER_FIRST_TXID,
                            instance,
                            connector,
                        );
                        write_dynamic_commit(
                            startup,
                            4,
                            DYNAMIC_UNREGISTER_FIRST_TXID,
                            instance.raw(),
                        );
                        first_instance = Some(instance);
                    }
                    DYNAMIC_REGISTER_REPLACEMENT_COMMAND => {
                        if replacement_instance.is_some() || replacement_connector.is_some() {
                            fail(FAIL_DYNAMIC_PROTOCOL);
                        }
                        let stale = first_instance.unwrap_or_else(|| fail(FAIL_DYNAMIC_INSTANCE));
                        let (connector, replacement) =
                            provider_register_dynamic(&state, DYNAMIC_REGISTER_REPLACEMENT_TXID);
                        if replacement.raw() <= stale.raw() {
                            fail(FAIL_DYNAMIC_INSTANCE);
                        }
                        write_dynamic_commit(
                            startup,
                            6,
                            DYNAMIC_REGISTER_REPLACEMENT_TXID,
                            replacement.raw(),
                        );
                        provider_reject_stale_unregister(&state, stale, replacement, &connector);
                        write_dynamic_commit(
                            startup,
                            7,
                            DYNAMIC_UNREGISTER_STALE_TXID,
                            stale.raw(),
                        );
                        provider_serve_dynamic_echo(
                            &connector,
                            DYNAMIC_LOOKUP_REPLACEMENT_TXID,
                            replacement,
                        );
                        if read_scalar(startup)
                            != (
                                DYNAMIC_COMMAND_TAG,
                                DYNAMIC_CONFIRM_REPLACEMENT_ECHO_COMMAND,
                            )
                        {
                            fail(FAIL_DYNAMIC_PROTOCOL);
                        }
                        write_dynamic_commit(
                            startup,
                            9,
                            DYNAMIC_LOOKUP_REPLACEMENT_TXID,
                            replacement.raw(),
                        );
                        replacement_instance = Some(replacement);
                        replacement_connector = Some(connector);
                    }
                    DYNAMIC_CLEANUP_COMMAND => {
                        let instance = replacement_instance
                            .take()
                            .unwrap_or_else(|| fail(FAIL_DYNAMIC_INSTANCE));
                        let connector = replacement_connector
                            .take()
                            .unwrap_or_else(|| fail(FAIL_DYNAMIC_INSTANCE));
                        provider_unregister_dynamic(
                            &state,
                            DYNAMIC_UNREGISTER_REPLACEMENT_TXID,
                            instance,
                            connector,
                        );
                        write_dynamic_commit(
                            startup,
                            10,
                            DYNAMIC_UNREGISTER_REPLACEMENT_TXID,
                            instance.raw(),
                        );
                        m15_replacement = Some(instance);
                        if !dynamic_done {
                            write_scalar(
                                startup,
                                DYNAMIC_DONE_TAG,
                                dynamic_done_payload(Role::Provider),
                            );
                            dynamic_done = true;
                        }
                    }
                    _ => fail(FAIL_DYNAMIC_PROTOCOL),
                }
            } else if tag == REUSE_COMMAND_TAG {
                match command {
                    REUSE_REGISTER_COMMAND => {
                        if !dynamic_done
                            || reuse_done
                            || reuse_instance.is_some()
                            || reuse_connector.is_some()
                        {
                            fail(FAIL_REUSE_PROTOCOL);
                        }
                        let floor = m15_replacement.unwrap_or_else(|| fail(FAIL_REUSE_INSTANCE));
                        let (connector, instance) =
                            provider_register_dynamic(&state, REUSE_REGISTER_TXID);
                        if instance.raw() <= floor.raw() {
                            fail(FAIL_REUSE_INSTANCE);
                        }
                        write_reuse_commit(startup, 1, REUSE_REGISTER_TXID, instance.raw());
                        provider_serve_dynamic_echo(&connector, REUSE_LOOKUP_TXID, instance);
                        if read_scalar(startup) != (REUSE_COMMAND_TAG, REUSE_CONFIRM_ECHO_COMMAND) {
                            fail(FAIL_REUSE_PROTOCOL);
                        }
                        write_reuse_commit(startup, 3, REUSE_LOOKUP_TXID, instance.raw());
                        reuse_instance = Some(instance);
                        reuse_connector = Some(connector);
                    }
                    REUSE_UNREGISTER_COMMAND => {
                        if !dynamic_done || reuse_done {
                            fail(FAIL_REUSE_PROTOCOL);
                        }
                        let instance = reuse_instance
                            .take()
                            .unwrap_or_else(|| fail(FAIL_REUSE_INSTANCE));
                        let connector = reuse_connector
                            .take()
                            .unwrap_or_else(|| fail(FAIL_REUSE_INSTANCE));
                        provider_unregister_dynamic(
                            &state,
                            REUSE_UNREGISTER_TXID,
                            instance,
                            connector,
                        );
                        write_reuse_commit(startup, 4, REUSE_UNREGISTER_TXID, instance.raw());
                        write_scalar(startup, reuse_done_tag(), Role::Provider.raw() as u64);
                        reuse_done = true;
                    }
                    _ => fail(FAIL_REUSE_PROTOCOL),
                }
            } else if tag == IDENTITY_COMMAND_TAG {
                if command != IDENTITY_PROVIDER_RELAY_COMMAND || !reuse_done || identity_relayed {
                    fail(FAIL_IDENTITY_PROTOCOL);
                }
                let duplicate = syscall(
                    SyscallNumber::HandleDuplicate,
                    state.session.raw(),
                    u64::from(Rights::CHANNEL_DEFAULT.bits()),
                    0,
                );
                let duplicate = OwnedUserHandle::new(duplicate.out1);
                if duplicate.is_none()
                    || duplicate
                        .as_ref()
                        .is_some_and(|handle| handle.raw() == state.session.raw())
                {
                    fail(FAIL_IDENTITY_PROTOCOL);
                }
                write_empty_transfer(
                    startup,
                    duplicate.unwrap_or_else(|| fail(FAIL_IDENTITY_PROTOCOL)),
                );
                write_scalar(
                    startup,
                    IDENTITY_PROVIDER_COMMIT_TAG,
                    IDENTITY_PROVIDER_RELAY_COMMAND,
                );
                identity_relayed = true;
            } else if tag == M20_PROVIDER_PRIMARY_PID_TAG {
                if command == 0
                    || command == state.supervisor_pid
                    || m20_primary_pid != 0
                    || m20_armed
                {
                    fail(FAIL_MULTI_CLIENT_IDENTITY);
                }
                m20_primary_pid = command;
            } else if tag == M20_PROVIDER_SECONDARY_PID_TAG {
                if command == 0
                    || command == state.supervisor_pid
                    || command == m20_primary_pid
                    || m20_secondary_pid != 0
                    || m20_armed
                {
                    fail(FAIL_MULTI_CLIENT_IDENTITY);
                }
                m20_secondary_pid = command;
            } else if tag == M20_PROVIDER_CONTROL_TAG {
                let (phase, lease, transaction_id) = m20_payload_parts(command);
                if command != m20_payload(phase, lease, transaction_id) {
                    fail(FAIL_MULTI_CLIENT_PROTOCOL);
                }
                match phase {
                    M20_PROVIDER_CONTROL_ARM
                        if lease == 0
                            && transaction_id == 0
                            && identity_relayed
                            && !m20_armed
                            && m20_primary_pid != 0
                            && m20_secondary_pid != 0
                            && m20_primary_pid != m20_secondary_pid =>
                    {
                        m20_armed = true;
                        write_scalar(
                            startup,
                            M20_PROVIDER_EVENT_TAG,
                            m20_payload(M20_PROVIDER_EVENT_ARMED, 0, 0),
                        );
                    }
                    M20_PROVIDER_CONTROL_FINALIZE
                        if lease == M20_SECONDARY_REATTACHED_LEASE
                            && transaction_id == 0
                            && m20_armed
                            && m20_pending.iter().all(Option::is_none)
                            && m20_echoes_completed == 3
                            && m20_stall_aborted =>
                    {
                        write_scalar(
                            startup,
                            M20_PROVIDER_EVENT_TAG,
                            m20_payload(M20_PROVIDER_EVENT_IDLE, lease, 0),
                        );
                    }
                    _ => fail(FAIL_MULTI_CLIENT_PROTOCOL),
                }
            } else {
                fail(FAIL_REUSE_PROTOCOL);
            }
        } else {
            let wait_index =
                usize::try_from(ready.out1).unwrap_or_else(|_| fail(FAIL_MULTI_CLIENT_PROTOCOL));
            let pending_index = pending_index_by_wait[wait_index];
            if pending_index == usize::MAX {
                fail(FAIL_MULTI_CLIENT_PROTOCOL);
            }
            let completed =
                provider_dispatch_m20_pending(startup, pending_index, ready.out2, &mut m20_pending);
            if completed {
                m20_echoes_completed = m20_echoes_completed
                    .checked_add(1)
                    .unwrap_or_else(|| fail(FAIL_MULTI_CLIENT_PROTOCOL));
            } else {
                if m20_stall_aborted {
                    fail(FAIL_MULTI_CLIENT_PROTOCOL);
                }
                m20_stall_aborted = true;
            }
        }
        if !state.is_reachable() {
            fail(FAIL_SM_LIFECYCLE);
        }
    }
}

fn provider_serve_static_echo_now(state: &ProviderResidentState, round: u32) {
    let (connect, server_endpoint) = read_frame_transfer_now(state.connector.raw());
    provider_finish_static_echo(state, round, connect, server_endpoint);
}

fn provider_finish_static_echo(
    state: &ProviderResidentState,
    round: u32,
    connect: Frame,
    server_endpoint: OwnedUserHandle,
) {
    let transaction_id = post_ready_transaction_id(round);
    let echo = echo_service_name();
    expect_service_frame(
        connect,
        Opcode::Connect,
        transaction_id,
        echo,
        ServiceStatus::Ok,
        state.instance.raw(),
        true,
    );
    let expected = (ECHO_TAG, post_ready_echo_payload(round));
    if read_scalar(server_endpoint.raw()) != expected {
        fail(FAIL_SM_ECHO);
    }
    write_scalar(server_endpoint.raw(), expected.0, expected.1);
    close_owned(server_endpoint);
}

fn provider_register_dynamic(
    state: &ProviderResidentState,
    transaction_id: u32,
) -> (OwnedUserHandle, InstanceId) {
    let name = dynamic_service_name();
    let (provider_connector, registry_connector) = create_channel_owned();
    write_frame_transfer(
        state.session.raw(),
        build_frame(Frame::register(transaction_id, name)),
        registry_connector,
    );
    let reply = read_frame_bytes(state.session.raw());
    let instance = expect_service_frame(
        reply,
        Opcode::RegisterReply,
        transaction_id,
        name,
        ServiceStatus::Ok,
        reply.instance_raw(),
        false,
    )
    .unwrap_or_else(|| fail(FAIL_DYNAMIC_INSTANCE));
    (provider_connector, instance)
}

fn provider_unregister_dynamic(
    state: &ProviderResidentState,
    transaction_id: u32,
    instance: InstanceId,
    connector: OwnedUserHandle,
) {
    let name = dynamic_service_name();
    write_frame_bytes(
        state.session.raw(),
        build_frame(Frame::unregister(transaction_id, name, instance)),
    );
    let reply = read_frame_bytes(state.session.raw());
    expect_service_frame(
        reply,
        Opcode::UnregisterReply,
        transaction_id,
        name,
        ServiceStatus::Ok,
        instance.raw(),
        false,
    );
    wait_peer_closed(connector.raw());
    close_owned(connector);
}

fn provider_reject_stale_unregister(
    state: &ProviderResidentState,
    stale: InstanceId,
    replacement: InstanceId,
    connector: &OwnedUserHandle,
) {
    let name = dynamic_service_name();
    write_frame_bytes(
        state.session.raw(),
        build_frame(Frame::unregister(
            DYNAMIC_UNREGISTER_STALE_TXID,
            name,
            stale,
        )),
    );
    let reply = read_frame_bytes(state.session.raw());
    expect_service_frame(
        reply,
        Opcode::UnregisterReply,
        DYNAMIC_UNREGISTER_STALE_TXID,
        name,
        ServiceStatus::InvalidState,
        0,
        false,
    );
    if replacement.raw() <= stale.raw() {
        fail(FAIL_DYNAMIC_INSTANCE);
    }
    let empty = syscall(SyscallNumber::ChannelRead, connector.raw(), 0, 0);
    if empty.status != Status::ShouldWait.raw() || empty.out1 != 0 || empty.out2 != 0 {
        fail(FAIL_DYNAMIC_CLEANUP);
    }
}

fn provider_serve_dynamic_echo(
    connector: &OwnedUserHandle,
    transaction_id: u32,
    instance: InstanceId,
) {
    let name = dynamic_service_name();
    let (connect, server_endpoint) = read_frame_transfer(connector.raw());
    expect_service_frame(
        connect,
        Opcode::Connect,
        transaction_id,
        name,
        ServiceStatus::Ok,
        instance.raw(),
        true,
    );
    let expected = (ECHO_TAG, dynamic_echo_payload(transaction_id, instance));
    if read_scalar(server_endpoint.raw()) != expected {
        fail(FAIL_SM_ECHO);
    }
    write_scalar(server_endpoint.raw(), expected.0, expected.1);
    close_owned(server_endpoint);
}

fn provider_accept_m20_echo(
    startup: u64,
    state: &ProviderResidentState,
    primary_pid: u64,
    secondary_pid: u64,
    pending: &mut [Option<PendingProviderEcho>; 2],
) {
    let (connect, server_endpoint) = read_frame_transfer(state.connector.raw());
    let transaction_id = connect.transaction_id();
    let (lease, expected_client_pid) = match transaction_id {
        M20_SECONDARY_STALLED_TXID => (M20_SECONDARY_FIRST_LEASE, secondary_pid),
        M20_PRIMARY_WHILE_STALLED_TXID | M20_PRIMARY_DETACHED_TXID => {
            (M20_PRIMARY_LEASE, primary_pid)
        }
        M20_SECONDARY_FINAL_TXID => (M20_SECONDARY_REATTACHED_LEASE, secondary_pid),
        _ => {
            close_owned(server_endpoint);
            fail(FAIL_MULTI_CLIENT_PROTOCOL);
        }
    };
    let name = echo_service_name();
    expect_service_frame(
        connect,
        Opcode::Connect,
        transaction_id,
        name,
        ServiceStatus::Ok,
        state.instance.raw(),
        true,
    );
    let Some(slot_index) = pending.iter().position(Option::is_none) else {
        close_owned(server_endpoint);
        fail(FAIL_MULTI_CLIENT_PROTOCOL)
    };
    pending[slot_index] = Some(PendingProviderEcho {
        endpoint: server_endpoint,
        transaction_id,
        lease,
        expected_client_pid,
    });
    write_scalar(
        startup,
        M20_PROVIDER_EVENT_TAG,
        m20_payload(M20_PROVIDER_EVENT_ACCEPTED, lease, transaction_id),
    );
}

fn provider_dispatch_m20_pending(
    startup: u64,
    pending_index: usize,
    observed_signals: u64,
    pending: &mut [Option<PendingProviderEcho>; 2],
) -> bool {
    let operation = pending[pending_index]
        .take()
        .unwrap_or_else(|| fail(FAIL_MULTI_CLIENT_PROTOCOL));
    if observed_signals & u64::from(ObjectSignals::READABLE.bits()) != 0 {
        let (sender_pid, scalar) = read_scalar_envelope_now(operation.endpoint.raw());
        let expected = (
            M20_ECHO_TAG,
            m20_payload(0, operation.lease, operation.transaction_id),
        );
        if sender_pid != operation.expected_client_pid || scalar != expected {
            close_owned(operation.endpoint);
            fail(FAIL_MULTI_CLIENT_ECHO);
        }
        write_scalar(operation.endpoint.raw(), expected.0, expected.1);
        close_owned(operation.endpoint);
        write_scalar(
            startup,
            M20_PROVIDER_EVENT_TAG,
            m20_payload(
                M20_PROVIDER_EVENT_ECHO_DONE,
                operation.lease,
                operation.transaction_id,
            ),
        );
        true
    } else if observed_signals & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
        && operation.transaction_id == M20_SECONDARY_STALLED_TXID
        && operation.lease == M20_SECONDARY_FIRST_LEASE
    {
        close_owned(operation.endpoint);
        write_scalar(
            startup,
            M20_PROVIDER_EVENT_TAG,
            m20_payload(
                M20_PROVIDER_EVENT_ABORTED,
                operation.lease,
                operation.transaction_id,
            ),
        );
        false
    } else {
        close_owned(operation.endpoint);
        fail(FAIL_MULTI_CLIENT_PROTOCOL)
    }
}

fn client_resident_loop(startup: u64, epoch: u16, state: ClientResidentState) -> ! {
    publish_resident_ready(startup, Role::Client, epoch, &state);
    for round in 1..=POST_READY_ECHO_ROUNDS {
        wait_for_client_startup(startup, state.session.raw());
        if read_scalar_now(startup) != (SERVING_ROUND_TAG, serving_round_payload(round)) {
            fail(FAIL_MANAGER_PROTOCOL);
        }
        client_post_ready_echo_round(&state, round);
        write_scalar(startup, SERVING_ACK_TAG, serving_round_payload(round));
        if !state.is_reachable() {
            fail(FAIL_SM_LIFECYCLE);
        }
    }

    let mut first_instance = None;
    let mut replacement_instance = None;
    let mut dynamic_done = false;
    let mut reuse_instance = None;
    let mut reuse_done = false;
    let mut identity_checked = false;
    let mut m20_manager_pid = 0_u64;
    loop {
        wait_for_client_startup(startup, state.session.raw());
        let (sender_pid, (tag, command)) = read_scalar_envelope_now(startup);
        if sender_pid != state.supervisor_pid {
            fail(FAIL_IDENTITY_SENDER);
        }
        if tag == DYNAMIC_COMMAND_TAG {
            match command {
                DYNAMIC_ECHO_FIRST_COMMAND => {
                    let instance = client_dynamic_echo(&state, DYNAMIC_LOOKUP_FIRST_TXID);
                    write_dynamic_commit(startup, 2, DYNAMIC_LOOKUP_FIRST_TXID, instance.raw());
                    first_instance = Some(instance);
                }
                DYNAMIC_LOOKUP_MISSING_COMMAND => {
                    client_dynamic_lookup_missing(&state, DYNAMIC_LOOKUP_MISSING_TXID);
                    write_dynamic_commit(startup, 5, DYNAMIC_LOOKUP_MISSING_TXID, 0);
                }
                DYNAMIC_ECHO_REPLACEMENT_COMMAND => {
                    let instance = client_dynamic_echo(&state, DYNAMIC_LOOKUP_REPLACEMENT_TXID);
                    if instance.raw()
                        <= first_instance
                            .unwrap_or_else(|| fail(FAIL_DYNAMIC_INSTANCE))
                            .raw()
                    {
                        fail(FAIL_DYNAMIC_INSTANCE);
                    }
                    write_dynamic_commit(
                        startup,
                        8,
                        DYNAMIC_LOOKUP_REPLACEMENT_TXID,
                        instance.raw(),
                    );
                    replacement_instance = Some(instance);
                    if !dynamic_done {
                        write_scalar(
                            startup,
                            DYNAMIC_DONE_TAG,
                            dynamic_done_payload(Role::Client),
                        );
                        dynamic_done = true;
                    }
                }
                _ => fail(FAIL_DYNAMIC_PROTOCOL),
            }
        } else if tag == REUSE_COMMAND_TAG {
            match command {
                REUSE_ECHO_COMMAND => {
                    if !dynamic_done || reuse_done || reuse_instance.is_some() {
                        fail(FAIL_REUSE_PROTOCOL);
                    }
                    let floor = replacement_instance.unwrap_or_else(|| fail(FAIL_REUSE_INSTANCE));
                    let instance = client_dynamic_echo(&state, REUSE_LOOKUP_TXID);
                    if instance.raw() <= floor.raw() {
                        fail(FAIL_REUSE_INSTANCE);
                    }
                    write_reuse_commit(startup, 2, REUSE_LOOKUP_TXID, instance.raw());
                    reuse_instance = Some(instance);
                }
                REUSE_LOOKUP_MISSING_COMMAND => {
                    if !dynamic_done || reuse_done || reuse_instance.is_none() {
                        fail(FAIL_REUSE_PROTOCOL);
                    }
                    client_dynamic_lookup_missing(&state, REUSE_LOOKUP_MISSING_TXID);
                    write_reuse_commit(startup, 5, REUSE_LOOKUP_MISSING_TXID, 0);
                    write_scalar(startup, reuse_done_tag(), Role::Client.raw() as u64);
                    reuse_done = true;
                }
                _ => fail(FAIL_REUSE_PROTOCOL),
            }
        } else if tag == IDENTITY_CLIENT_ATTACK_TAG {
            if command == 0 || !reuse_done || identity_checked {
                fail(FAIL_IDENTITY_PROTOCOL);
            }
            let (relay_sender, delegated_session) = read_empty_transfer_envelope(startup);
            if relay_sender != state.supervisor_pid {
                close_owned(delegated_session);
                fail(FAIL_IDENTITY_SENDER);
            }
            let name = delegated_attack_service_name();

            // Treat every manager ingress byte as hostile. These four FIFO
            // entries cover wrong kind, short payload, undecodable transfer
            // (including owned-handle cleanup), and a valid frame with the
            // wrong opcode. Peer-close and the later PermissionDenied reply
            // prove that the manager consumed all four and kept serving.
            write_scalar(delegated_session.raw(), 0, 0);
            write_untrusted_bytes(delegated_session.raw(), &[0xa5]);
            let (malformed_peer, malformed_connector) = create_channel_owned();
            write_untrusted_transfer(
                delegated_session.raw(),
                &[0; FRAME_SIZE],
                malformed_connector,
            );
            wait_peer_closed(malformed_peer.raw());
            close_owned(malformed_peer);
            write_frame_bytes(
                delegated_session.raw(),
                build_frame(Frame::lookup(IDENTITY_ATTACK_TXID, name)),
            );

            let (rejected_peer, rejected_connector) = create_channel_owned();
            write_frame_transfer(
                delegated_session.raw(),
                build_frame(Frame::register(IDENTITY_ATTACK_TXID, name)),
                rejected_connector,
            );
            let (manager_sender, reply) = read_frame_bytes_envelope(delegated_session.raw());
            if manager_sender != command {
                close_owned(rejected_peer);
                close_owned(delegated_session);
                fail(FAIL_IDENTITY_SENDER);
            }
            expect_service_frame(
                reply,
                Opcode::RegisterReply,
                IDENTITY_ATTACK_TXID,
                name,
                ServiceStatus::PermissionDenied,
                0,
                false,
            );
            wait_peer_closed(rejected_peer.raw());
            close_owned(rejected_peer);
            close_owned(delegated_session);
            write_scalar(
                startup,
                IDENTITY_CLIENT_COMMIT_TAG,
                u64::from(IDENTITY_ATTACK_TXID),
            );
            identity_checked = true;
        } else if tag == M20_CLIENT_MANAGER_PID_TAG {
            if !identity_checked
                || command == 0
                || command == state.supervisor_pid
                || command == state.client_pid
                || m20_manager_pid != 0
            {
                fail(FAIL_MULTI_CLIENT_IDENTITY);
            }
            m20_manager_pid = command;
        } else if tag == M20_CLIENT_CONTROL_TAG {
            let (phase, lease, transaction_id) = m20_payload_parts(command);
            if command != m20_payload(phase, lease, transaction_id)
                || !identity_checked
                || m20_manager_pid == 0
                || phase != M20_CLIENT_CONTROL_ECHO
                || lease != M20_PRIMARY_LEASE
                || !matches!(
                    transaction_id,
                    M20_PRIMARY_WHILE_STALLED_TXID | M20_PRIMARY_DETACHED_TXID
                )
            {
                fail(FAIL_MULTI_CLIENT_PROTOCOL);
            }
            client_m20_echo(
                state.session.raw(),
                state.instance,
                m20_manager_pid,
                lease,
                transaction_id,
            );
            write_scalar(
                startup,
                M20_CLIENT_EVENT_TAG,
                m20_payload(M20_CLIENT_EVENT_ECHO_DONE, lease, transaction_id),
            );
        } else {
            fail(FAIL_REUSE_PROTOCOL);
        }
        if replacement_instance.is_some()
            && replacement_instance
                .is_some_and(|replacement| replacement.raw() <= state.instance.raw())
        {
            fail(FAIL_DYNAMIC_INSTANCE);
        }
        if !state.is_reachable() {
            fail(FAIL_SM_LIFECYCLE);
        }
    }
}

fn secondary_client_loop(startup: u64, supervisor_pid: u64) -> ! {
    let instance = InstanceId::new(SECOND_EPOCH_INSTANCE_RAW)
        .unwrap_or_else(|| fail(FAIL_MULTI_CLIENT_CONFIG));
    let (attach_sender, attach, first_session) = read_attach_envelope(startup);
    let client_pid = attach.owner_pid();
    if attach_sender != supervisor_pid
        || attach.manager_epoch() != SECOND_MANAGER_EPOCH
        || attach.role() != Role::Client
        || client_pid == 0
        || client_pid == supervisor_pid
    {
        close_owned(first_session);
        fail(FAIL_MULTI_CLIENT_ATTACH);
    }
    consume_abandoned_transfer(startup);
    let mut session = Some(first_session);
    let mut lease = M20_SECONDARY_FIRST_LEASE;
    let mut manager_pid = 0_u64;
    let mut stalled_endpoint = None;
    write_scalar(
        startup,
        M20_CLIENT_EVENT_TAG,
        m20_payload(M20_CLIENT_EVENT_ATTACHED, lease, 0),
    );

    loop {
        if let Some(active) = session.as_ref() {
            let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
            let ready = object_wait_many(
                startup,
                requested,
                active.raw(),
                requested,
                OBJECT_WAIT_TIMEOUT_INFINITE,
            );
            if ready.status != Status::Ok.raw() || ready.out1 > 1 {
                fail(FAIL_MULTI_CLIENT_PROTOCOL);
            }
            if ready.out1 == 1 {
                if ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
                    || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) != 0
                    || lease != M20_SECONDARY_FIRST_LEASE
                {
                    fail(FAIL_MULTI_CLIENT_CLEANUP);
                }
                let revoked = session
                    .take()
                    .unwrap_or_else(|| fail(FAIL_MULTI_CLIENT_CLEANUP));
                close_owned(revoked);
                let stalled = stalled_endpoint
                    .take()
                    .unwrap_or_else(|| fail(FAIL_MULTI_CLIENT_CLEANUP));
                close_owned(stalled);
                write_scalar(
                    startup,
                    M20_CLIENT_EVENT_TAG,
                    m20_payload(M20_CLIENT_EVENT_REVOKED, lease, 0),
                );
                continue;
            }
            if ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
                || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
            {
                fail(FAIL_MULTI_CLIENT_PROTOCOL);
            }
        } else {
            wait_readable(startup);
        }

        let envelope = read_channel_envelope_now(startup);
        if envelope.sender_pid() != supervisor_pid {
            if envelope.kind() == ChannelMessageKind::Transfer
                && let Some(received) =
                    OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
            {
                close_owned(received);
            }
            fail(FAIL_IDENTITY_SENDER);
        }
        match envelope.kind() {
            ChannelMessageKind::Transfer => {
                let (attach, reattached) = attach_from_envelope(envelope);
                if session.is_some()
                    || lease != M20_SECONDARY_FIRST_LEASE
                    || attach.manager_epoch() != SECOND_MANAGER_EPOCH
                    || attach.role() != Role::Client
                    || attach.owner_pid() != client_pid
                {
                    close_owned(reattached);
                    fail(FAIL_MULTI_CLIENT_ATTACH);
                }
                lease = M20_SECONDARY_REATTACHED_LEASE;
                session = Some(reattached);
                write_scalar(
                    startup,
                    M20_CLIENT_EVENT_TAG,
                    m20_payload(M20_CLIENT_EVENT_ATTACHED, lease, 0),
                );
            }
            ChannelMessageKind::Scalar => {
                let (tag, payload) = envelope
                    .scalar_values()
                    .unwrap_or_else(|| fail(FAIL_MULTI_CLIENT_PROTOCOL));
                if tag == M20_CLIENT_MANAGER_PID_TAG {
                    if payload == 0
                        || payload == supervisor_pid
                        || payload == client_pid
                        || manager_pid != 0
                    {
                        fail(FAIL_MULTI_CLIENT_IDENTITY);
                    }
                    manager_pid = payload;
                    continue;
                }
                let (phase, requested_lease, transaction_id) = m20_payload_parts(payload);
                if tag != M20_CLIENT_CONTROL_TAG
                    || payload != m20_payload(phase, requested_lease, transaction_id)
                    || manager_pid == 0
                {
                    fail(FAIL_MULTI_CLIENT_PROTOCOL);
                }
                match phase {
                    M20_CLIENT_CONTROL_STALL
                        if requested_lease == M20_SECONDARY_FIRST_LEASE
                            && requested_lease == lease
                            && transaction_id == M20_SECONDARY_STALLED_TXID
                            && stalled_endpoint.is_none() =>
                    {
                        let active = session
                            .as_ref()
                            .unwrap_or_else(|| fail(FAIL_MULTI_CLIENT_PROTOCOL));
                        stalled_endpoint = Some(client_m20_open_endpoint(
                            active.raw(),
                            instance,
                            manager_pid,
                            transaction_id,
                        ));
                        write_scalar(
                            startup,
                            M20_CLIENT_EVENT_TAG,
                            m20_payload(M20_CLIENT_EVENT_STALLED, lease, transaction_id),
                        );
                    }
                    M20_CLIENT_CONTROL_STALE_REVOKE
                        if requested_lease == M20_SECONDARY_FIRST_LEASE
                            && lease == M20_SECONDARY_REATTACHED_LEASE
                            && transaction_id == 0
                            && session.is_some() =>
                    {
                        write_scalar(
                            startup,
                            M20_CLIENT_EVENT_TAG,
                            m20_payload(M20_CLIENT_EVENT_STALE_REJECTED, requested_lease, 0),
                        );
                    }
                    M20_CLIENT_CONTROL_ECHO
                        if requested_lease == M20_SECONDARY_REATTACHED_LEASE
                            && requested_lease == lease
                            && transaction_id == M20_SECONDARY_FINAL_TXID
                            && session.is_some() =>
                    {
                        client_m20_echo(
                            session
                                .as_ref()
                                .unwrap_or_else(|| fail(FAIL_MULTI_CLIENT_PROTOCOL))
                                .raw(),
                            instance,
                            manager_pid,
                            lease,
                            transaction_id,
                        );
                        write_scalar(
                            startup,
                            M20_CLIENT_EVENT_TAG,
                            m20_payload(M20_CLIENT_EVENT_ECHO_DONE, lease, transaction_id),
                        );
                    }
                    _ => fail(FAIL_MULTI_CLIENT_PROTOCOL),
                }
            }
            ChannelMessageKind::Bytes => fail(FAIL_MULTI_CLIENT_PROTOCOL),
        }
    }
}

fn wait_for_client_startup(startup: u64, session: u64) {
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let ready = object_wait_many(
        startup,
        requested,
        session,
        requested,
        OBJECT_WAIT_TIMEOUT_INFINITE,
    );
    if ready.status != Status::Ok.raw()
        || ready.out1 != 0
        || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
        || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
    {
        fail(FAIL_MULTI_CLIENT_PROTOCOL);
    }
}

fn client_m20_open_endpoint(
    session: u64,
    instance: InstanceId,
    manager_pid: u64,
    transaction_id: u32,
) -> OwnedUserHandle {
    write_frame_bytes(
        session,
        build_frame(Frame::lookup(transaction_id, echo_service_name())),
    );
    let (reply_sender, reply, client_endpoint) = read_frame_transfer_envelope(session);
    if reply_sender != manager_pid {
        close_owned(client_endpoint);
        fail(FAIL_MULTI_CLIENT_IDENTITY);
    }
    expect_service_frame(
        reply,
        Opcode::LookupReply,
        transaction_id,
        echo_service_name(),
        ServiceStatus::Ok,
        instance.raw(),
        true,
    );
    client_endpoint
}

fn client_m20_echo(
    session: u64,
    instance: InstanceId,
    manager_pid: u64,
    lease: u16,
    transaction_id: u32,
) {
    let endpoint = client_m20_open_endpoint(session, instance, manager_pid, transaction_id);
    let expected = (M20_ECHO_TAG, m20_payload(0, lease, transaction_id));
    write_scalar(endpoint.raw(), expected.0, expected.1);
    if read_scalar(endpoint.raw()) != expected {
        close_owned(endpoint);
        fail(FAIL_MULTI_CLIENT_ECHO);
    }
    close_owned(endpoint);
}

fn client_post_ready_echo_round(state: &ClientResidentState, round: u32) {
    let echo = echo_service_name();
    let transaction_id = post_ready_transaction_id(round);
    write_frame_bytes(
        state.session.raw(),
        build_frame(Frame::lookup(transaction_id, echo)),
    );
    let (reply, client_endpoint) = read_frame_transfer(state.session.raw());
    expect_service_frame(
        reply,
        Opcode::LookupReply,
        transaction_id,
        echo,
        ServiceStatus::Ok,
        state.instance.raw(),
        true,
    );

    let expected = (ECHO_TAG, post_ready_echo_payload(round));
    write_scalar(client_endpoint.raw(), expected.0, expected.1);
    if read_scalar(client_endpoint.raw()) != expected {
        fail(FAIL_SM_ECHO);
    }
    close_owned(client_endpoint);
}

fn client_dynamic_echo(state: &ClientResidentState, transaction_id: u32) -> InstanceId {
    let name = dynamic_service_name();
    write_frame_bytes(
        state.session.raw(),
        build_frame(Frame::lookup(transaction_id, name)),
    );
    let (reply, client_endpoint) = read_frame_transfer(state.session.raw());
    let instance = expect_service_frame(
        reply,
        Opcode::LookupReply,
        transaction_id,
        name,
        ServiceStatus::Ok,
        reply.instance_raw(),
        true,
    )
    .unwrap_or_else(|| fail(FAIL_DYNAMIC_INSTANCE));
    let expected = (ECHO_TAG, dynamic_echo_payload(transaction_id, instance));
    write_scalar(client_endpoint.raw(), expected.0, expected.1);
    if read_scalar(client_endpoint.raw()) != expected {
        fail(FAIL_SM_ECHO);
    }
    close_owned(client_endpoint);
    instance
}

fn client_dynamic_lookup_missing(state: &ClientResidentState, transaction_id: u32) {
    let name = dynamic_service_name();
    write_frame_bytes(
        state.session.raw(),
        build_frame(Frame::lookup(transaction_id, name)),
    );
    let reply = read_frame_bytes(state.session.raw());
    expect_service_frame(
        reply,
        Opcode::LookupReply,
        transaction_id,
        name,
        ServiceStatus::NotFound,
        0,
        false,
    );
}

fn consume_abandoned_transfer(startup: u64) {
    wait_readable(startup);
    let result = syscall(SyscallNumber::ChannelReadTransfer, startup, u64::MAX, 0);
    let received = OwnedUserHandle::new(result.out2);
    if result.status != Status::Ok.raw() || result.out1 != 0 || received.is_none() {
        fail(FAIL_ABANDONED_TRANSFER);
    }
    close_owned(received.unwrap_or_else(|| fail(FAIL_ABANDONED_TRANSFER)));
}

fn child_generic_transfer_round(startup: u64, supervisor_pid: u64) {
    wait_readable(startup);

    let mut receive = [0xa5_u8; CHANNEL_MESSAGE_MAX_BYTES];
    let too_small = syscall(
        SyscallNumber::ChannelReadTransfer,
        startup,
        receive.as_mut_ptr() as u64,
        (BYTE_MESSAGE.len() - 1) as u64,
    );
    if too_small.status != Status::BufferTooSmall.raw()
        || too_small.out1 != BYTE_MESSAGE.len() as u64
        || too_small.out2 != 0
        || receive.iter().any(|byte| *byte != 0xa5)
    {
        fail(35);
    }
    let bad_address = syscall(
        SyscallNumber::ChannelReadTransfer,
        startup,
        USER_GUARD_HIGH,
        receive.len() as u64,
    );
    if bad_address.status != Status::BadAddress.raw()
        || bad_address.out1 != 0
        || bad_address.out2 != 0
    {
        fail(69);
    }

    let mut duplicates = [0_u64; CHILD_DUPLICATES_TO_FILL];
    for slot in &mut duplicates {
        let duplicate = syscall(
            SyscallNumber::HandleDuplicate,
            startup,
            u64::from(Rights::READ.bits()),
            0,
        );
        if duplicate.status != Status::Ok.raw() || duplicate.out1 == 0 {
            fail(70);
        }
        *slot = duplicate.out1;
    }
    if syscall(SyscallNumber::EventCreate, 0, 0, 0).status != Status::OutOfMemory.raw() {
        fail(95);
    }
    if syscall(
        SyscallNumber::ChannelReadTransfer,
        startup,
        receive.as_mut_ptr() as u64,
        receive.len() as u64,
    )
    .status
        != Status::OutOfMemory.raw()
        || receive.iter().any(|byte| *byte != 0xa5)
    {
        fail(71);
    }
    let mut envelope_wire = [0xa5_u8; CHANNEL_READ_ENVELOPE_SIZE];
    let envelope_table_full = syscall(
        SyscallNumber::ChannelReadEnvelope,
        startup,
        envelope_wire.as_mut_ptr() as u64,
        CHANNEL_READ_ENVELOPE_SIZE as u64,
    );
    if envelope_table_full.status != Status::OutOfMemory.raw()
        || envelope_table_full.out1 != 0
        || envelope_table_full.out2 != 0
        || envelope_wire.iter().any(|byte| *byte != 0xa5)
    {
        fail(FAIL_IDENTITY_ENVELOPE);
    }
    if syscall(SyscallNumber::HandleClose, duplicates[0], 0, 0).status != Status::Ok.raw() {
        fail(72);
    }
    let received = syscall(
        SyscallNumber::ChannelReadEnvelope,
        startup,
        envelope_wire.as_mut_ptr() as u64,
        CHANNEL_READ_ENVELOPE_SIZE as u64,
    );
    let envelope = ChannelReadEnvelope::decode(&envelope_wire)
        .unwrap_or_else(|_| fail(FAIL_IDENTITY_ENVELOPE));
    if received.status != Status::Ok.raw()
        || received.out1 != supervisor_pid
        || received.out2 != ChannelMessageKind::Transfer.raw()
        || envelope.sender_pid() != supervisor_pid
        || envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != BYTE_MESSAGE.len()
        || envelope.payload_bytes() != Some(&BYTE_MESSAGE)
        || !envelope.received_handle().is_valid()
    {
        fail(73);
    }
    let service = u64::from(envelope.received_handle().raw());

    // Asking for the complete default rights proves the transferred rights
    // were preserved rather than silently attenuated in transit.
    if syscall(SyscallNumber::HandleClose, duplicates[1], 0, 0).status != Status::Ok.raw() {
        fail(74);
    }
    let rights_probe = syscall(
        SyscallNumber::HandleDuplicate,
        service,
        u64::from(Rights::CHANNEL_DEFAULT.bits()),
        0,
    );
    if rights_probe.status != Status::Ok.raw()
        || rights_probe.out1 == 0
        || syscall(SyscallNumber::HandleClose, rights_probe.out1, 0, 0).status != Status::Ok.raw()
    {
        fail(75);
    }

    // The parent queues the service and Event transfers in two separate
    // syscalls. A timer tick may run this child after the first transfer, so
    // establish that the Event message is present before deliberately filling
    // the final handle-table slot. Otherwise an empty queue would correctly
    // return ShouldWait instead of exercising the intended TableFull rollback.
    wait_readable(startup);
    let event_table_filler = syscall(
        SyscallNumber::HandleDuplicate,
        startup,
        u64::from(Rights::READ.bits()),
        0,
    );
    if event_table_filler.status != Status::Ok.raw() || event_table_filler.out1 == 0 {
        fail(96);
    }
    receive.fill(0xa5);
    let event_table_full = syscall(
        SyscallNumber::ChannelReadTransfer,
        startup,
        receive.as_mut_ptr() as u64,
        1,
    );
    if event_table_full.status != Status::OutOfMemory.raw()
        || receive[0] != 0xa5
        || syscall(SyscallNumber::HandleClose, event_table_filler.out1, 0, 0).status
            != Status::Ok.raw()
    {
        fail(97);
    }
    let event_too_small = syscall(
        SyscallNumber::ChannelReadTransfer,
        startup,
        receive.as_mut_ptr() as u64,
        0,
    );
    if event_too_small.status != Status::BufferTooSmall.raw()
        || event_too_small.out1 != 1
        || event_too_small.out2 != 0
        || receive[0] != 0xa5
    {
        fail(98);
    }
    let event_bad_address = syscall(
        SyscallNumber::ChannelReadTransfer,
        startup,
        USER_GUARD_HIGH,
        receive.len() as u64,
    );
    if event_bad_address.status != Status::BadAddress.raw()
        || event_bad_address.out1 != 0
        || event_bad_address.out2 != 0
    {
        fail(99);
    }
    let received_event = syscall(
        SyscallNumber::ChannelReadTransfer,
        startup,
        receive.as_mut_ptr() as u64,
        receive.len() as u64,
    );
    if received_event.status != Status::Ok.raw()
        || received_event.out1 != 1
        || received_event.out2 == 0
        || receive[0] != BYTE_MESSAGE[0]
        || syscall(SyscallNumber::HandleClose, duplicates[2], 0, 0).status != Status::Ok.raw()
    {
        fail(100);
    }
    let event = received_event.out2;
    let event_rights_probe = syscall(
        SyscallNumber::HandleDuplicate,
        event,
        u64::from(Rights::EVENT_DEFAULT.bits()),
        0,
    );
    if event_rights_probe.status != Status::Ok.raw()
        || event_rights_probe.out1 == 0
        || syscall(SyscallNumber::HandleClose, event_rights_probe.out1, 0, 0).status
            != Status::Ok.raw()
        || !event_change(SyscallNumber::EventSignal, event, true)
        || syscall(SyscallNumber::HandleClose, event, 0, 0).status != Status::Ok.raw()
    {
        fail(90);
    }

    let request = syscall(SyscallNumber::ChannelRead, service, 0, 0);
    if request.status != Status::Ok.raw()
        || request.out1 != EXPECTED_TAG
        || request.out2 != EXPECTED_PAYLOAD
    {
        fail(76);
    }
    if syscall(
        SyscallNumber::ChannelWrite,
        service,
        EXPECTED_TAG,
        EXPECTED_PAYLOAD,
    )
    .status
        != Status::Ok.raw()
    {
        fail(77);
    }
    if syscall(SyscallNumber::HandleClose, service, 0, 0).status != Status::Ok.raw() {
        fail(FAIL_SM_CLOSE);
    }
    for duplicate in &duplicates[3..] {
        if syscall(SyscallNumber::HandleClose, *duplicate, 0, 0).status != Status::Ok.raw() {
            fail(FAIL_SM_CLOSE);
        }
    }

    let continuation = read_scalar(startup);
    if continuation != (GENERIC_CONTINUE_TAG, GENERIC_CONTINUE_PAYLOAD) {
        fail(FAIL_GENERIC_CONTINUE);
    }
}

fn exercise_byte_channel(left: u64, right: u64) {
    let mut receive = [0xa5_u8; CHANNEL_MESSAGE_MAX_BYTES];
    let write = syscall(
        SyscallNumber::ChannelWriteBytes,
        left,
        BYTE_MESSAGE.as_ptr() as u64,
        BYTE_MESSAGE.len() as u64,
    );
    if write.status != Status::Ok.raw() || write.out1 != BYTE_MESSAGE.len() as u64 {
        fail(20);
    }

    let too_small = syscall(
        SyscallNumber::ChannelReadBytes,
        right,
        receive.as_mut_ptr() as u64,
        (BYTE_MESSAGE.len() - 1) as u64,
    );
    if too_small.status != Status::BufferTooSmall.raw()
        || too_small.out1 != BYTE_MESSAGE.len() as u64
        || receive[..BYTE_MESSAGE.len()]
            .iter()
            .any(|byte| *byte != 0xa5)
    {
        fail(21);
    }

    let read = syscall(
        SyscallNumber::ChannelReadBytes,
        right,
        receive.as_mut_ptr() as u64,
        receive.len() as u64,
    );
    if read.status != Status::Ok.raw()
        || read.out1 != BYTE_MESSAGE.len() as u64
        || receive[..BYTE_MESSAGE.len()] != BYTE_MESSAGE
    {
        fail(22);
    }

    let guard_read = syscall(SyscallNumber::ChannelWriteBytes, left, USER_GUARD_LOW, 1);
    if guard_read.status != Status::BadAddress.raw()
        || guard_read.out1 != 0
        || syscall(
            SyscallNumber::ChannelReadBytes,
            right,
            receive.as_mut_ptr() as u64,
            receive.len() as u64,
        )
        .status
            != Status::ShouldWait.raw()
    {
        fail(23);
    }

    let overflow = syscall(SyscallNumber::ChannelWriteBytes, left, u64::MAX, 1);
    if overflow.status != Status::BadAddress.raw() || overflow.out1 != 0 {
        fail(24);
    }

    if syscall(
        SyscallNumber::ChannelWriteBytes,
        left,
        BYTE_MESSAGE.as_ptr() as u64,
        1,
    )
    .status
        != Status::Ok.raw()
    {
        fail(25);
    }
    let guard_write = syscall(SyscallNumber::ChannelReadBytes, right, USER_GUARD_HIGH, 1);
    if guard_write.status != Status::BadAddress.raw() || guard_write.out1 != 0 {
        fail(26);
    }
    receive[0] = 0;
    let retry = syscall(
        SyscallNumber::ChannelReadBytes,
        right,
        receive.as_mut_ptr() as u64,
        receive.len() as u64,
    );
    if retry.status != Status::Ok.raw() || retry.out1 != 1 || receive[0] != BYTE_MESSAGE[0] {
        fail(27);
    }

    let zero_write = syscall(SyscallNumber::ChannelWriteBytes, left, u64::MAX, 0);
    let zero_read = syscall(SyscallNumber::ChannelReadBytes, right, u64::MAX, 0);
    if zero_write.status != Status::Ok.raw()
        || zero_write.out1 != 0
        || zero_read.status != Status::Ok.raw()
        || zero_read.out1 != 0
    {
        fail(28);
    }
}

#[inline(never)]
fn stack_round_trip() -> u64 {
    let mut slot = 0_u64;
    let address = ptr::addr_of_mut!(slot);
    unsafe {
        ptr::write_volatile(address, STACK_SENTINEL);
        ptr::read_volatile(address)
    }
}

#[inline(always)]
fn syscall(number: SyscallNumber, arg0: u64, arg1: u64, arg2: u64) -> SyscallResult {
    raw_syscall(number as u64, arg0, arg1, arg2)
}

#[inline(always)]
fn raw_syscall(number: u64, arg0: u64, arg1: u64, arg2: u64) -> SyscallResult {
    let mut x0 = arg0;
    let mut x1 = arg1;
    let mut x2 = arg2;
    unsafe {
        asm!(
            "svc #0",
            inlateout("x0") x0,
            inlateout("x1") x1,
            inlateout("x2") x2,
            in("x8") number,
            options(nostack)
        );
    }
    SyscallResult {
        status: x0,
        out1: x1,
        out2: x2,
    }
}

#[inline(always)]
fn private_sleep_probe() -> SyscallResult {
    let mut x0 = 0_u64;
    let mut x1 = 0_u64;
    let mut x2 = 0_u64;
    unsafe {
        asm!(
            "svc #0xb0",
            inlateout("x0") x0,
            inlateout("x1") x1,
            inlateout("x2") x2,
            in("x8") 0_u64,
            options(nostack)
        );
    }
    SyscallResult {
        status: x0,
        out1: x1,
        out2: x2,
    }
}

#[cold]
fn fail(reason: u64) -> ! {
    if PROCESS_ROLE.load(Ordering::Relaxed) == 1 {
        let exit_code = CHILD_FAILURE_EXIT_PREFIX | (reason & u64::from(u32::MAX));
        let _ = syscall(SyscallNumber::ThreadExit, exit_code, 0, 0);
        idle();
    }
    let _ = syscall(SyscallNumber::InitFailed, reason, 0, 0);
    idle()
}

fn idle() -> ! {
    loop {
        unsafe {
            asm!("yield", options(nomem, nostack, preserves_flags));
        }
    }
}

pub fn init_panic() -> ! {
    PROCESS_ROLE.store(0, Ordering::Relaxed);
    fail(63)
}

pub fn child_panic() -> ! {
    PROCESS_ROLE.store(1, Ordering::Relaxed);
    fail(63)
}
