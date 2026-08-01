#![no_std]

#[cfg(all(
    feature = "androidbox-apk-install0",
    any(feature = "app-data-runtime", feature = "storage-server-runtime")
))]
compile_error!(
    "androidbox-apk-install0 is an isolated mobile profile and cannot be combined with AppData, \
     StorageServer, resident-platform, or unified-product runtime profiles"
);

#[cfg(feature = "androidbox-layout-mixed19")]
pub const ABI_VERSION: u64 = 69;
#[cfg(all(
    feature = "androidbox-layout-size18",
    not(feature = "androidbox-layout-mixed19")
))]
pub const ABI_VERSION: u64 = 68;
#[cfg(all(
    feature = "androidbox-layout-directional17",
    not(feature = "androidbox-layout-size18")
))]
pub const ABI_VERSION: u64 = 67;
#[cfg(all(
    feature = "androidbox-layout-spacing16",
    not(feature = "androidbox-layout-directional17")
))]
pub const ABI_VERSION: u64 = 66;
#[cfg(all(
    feature = "androidbox-layout-weight15",
    not(feature = "androidbox-layout-spacing16")
))]
pub const ABI_VERSION: u64 = 65;
#[cfg(all(
    feature = "androidbox-layout-row14",
    not(feature = "androidbox-layout-weight15")
))]
pub const ABI_VERSION: u64 = 64;
#[cfg(all(
    feature = "androidbox-string-builder13",
    not(feature = "androidbox-layout-row14")
))]
pub const ABI_VERSION: u64 = 63;
#[cfg(all(
    feature = "androidbox-string-text12",
    not(feature = "androidbox-string-builder13")
))]
pub const ABI_VERSION: u64 = 62;
#[cfg(all(
    feature = "androidbox-activity-state11",
    not(feature = "androidbox-string-text12")
))]
pub const ABI_VERSION: u64 = 61;
#[cfg(all(
    feature = "androidbox-activity-fields10",
    not(feature = "androidbox-activity-state11")
))]
pub const ABI_VERSION: u64 = 60;
#[cfg(all(
    feature = "androidbox-dex-instance9",
    not(feature = "androidbox-activity-fields10")
))]
pub const ABI_VERSION: u64 = 59;
#[cfg(all(
    feature = "androidbox-dex-methods8",
    not(feature = "androidbox-dex-instance9")
))]
pub const ABI_VERSION: u64 = 58;
#[cfg(all(
    feature = "androidbox-density-icons7",
    not(feature = "androidbox-dex-methods8")
))]
pub const ABI_VERSION: u64 = 57;
#[cfg(all(
    feature = "androidbox-icon-resources5",
    not(feature = "androidbox-density-icons7")
))]
pub const ABI_VERSION: u64 = 56;
#[cfg(all(
    feature = "androidbox-multipackage4",
    not(feature = "androidbox-icon-resources5")
))]
pub const ABI_VERSION: u64 = 55;
#[cfg(all(
    feature = "androidbox-manifest-catalog3",
    not(feature = "androidbox-multipackage4")
))]
pub const ABI_VERSION: u64 = 54;
#[cfg(all(
    feature = "androidbox-runtime-install2",
    not(feature = "androidbox-manifest-catalog3")
))]
pub const ABI_VERSION: u64 = 53;
#[cfg(all(
    feature = "androidbox-runtime-uninstall1",
    not(feature = "androidbox-runtime-install2")
))]
pub const ABI_VERSION: u64 = 52;
#[cfg(all(
    feature = "androidbox-apk-envelope4",
    not(feature = "androidbox-runtime-uninstall1")
))]
pub const ABI_VERSION: u64 = 51;
#[cfg(all(
    feature = "androidbox-multiaction3",
    not(feature = "androidbox-apk-envelope4")
))]
pub const ABI_VERSION: u64 = 50;
#[cfg(all(
    feature = "androidbox-scene-rpc2",
    not(feature = "androidbox-multiaction3")
))]
pub const ABI_VERSION: u64 = 49;
#[cfg(all(
    feature = "androidbox-restart0",
    not(feature = "androidbox-scene-rpc2")
))]
pub const ABI_VERSION: u64 = 48;
#[cfg(all(feature = "androidbox-process0", not(feature = "androidbox-restart0")))]
pub const ABI_VERSION: u64 = 47;
#[cfg(all(
    feature = "androidbox-interactive0",
    not(feature = "androidbox-process0")
))]
pub const ABI_VERSION: u64 = 46;
#[cfg(all(
    feature = "androidbox-el0-runtime0",
    not(feature = "androidbox-interactive0")
))]
pub const ABI_VERSION: u64 = 45;
#[cfg(all(
    feature = "androidbox-apk-install0",
    not(feature = "androidbox-el0-runtime0")
))]
pub const ABI_VERSION: u64 = 44;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-signed-maintenance-plan-runtime"
))]
pub const ABI_VERSION: u64 = 42;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-maintenance-plan-runtime",
    not(feature = "unified-product-signed-maintenance-plan-runtime")
))]
pub const ABI_VERSION: u64 = 41;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-maintenance-step-runtime",
    not(feature = "unified-product-maintenance-plan-runtime")
))]
pub const ABI_VERSION: u64 = 40;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-maintenance-execution-runtime",
    not(feature = "unified-product-maintenance-step-runtime")
))]
pub const ABI_VERSION: u64 = 39;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-maintenance-authorization-runtime",
    not(feature = "unified-product-maintenance-execution-runtime")
))]
pub const ABI_VERSION: u64 = 38;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-key-rotation-runtime",
    not(feature = "unified-product-maintenance-authorization-runtime")
))]
pub const ABI_VERSION: u64 = 37;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-persistent-rollback-runtime",
    not(feature = "unified-product-key-rotation-runtime")
))]
pub const ABI_VERSION: u64 = 36;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-verified-manifest-runtime",
    not(feature = "unified-product-persistent-rollback-runtime")
))]
pub const ABI_VERSION: u64 = 35;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-event-supervision-runtime",
    not(feature = "unified-product-verified-manifest-runtime")
))]
pub const ABI_VERSION: u64 = 34;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-manifest-supervision-runtime",
    not(feature = "unified-product-event-supervision-runtime")
))]
pub const ABI_VERSION: u64 = 33;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-continuous-supervision-runtime",
    not(feature = "unified-product-manifest-supervision-runtime")
))]
pub const ABI_VERSION: u64 = 32;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-psci-shutdown-runtime",
    not(feature = "unified-product-continuous-supervision-runtime")
))]
pub const ABI_VERSION: u64 = 31;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-multiservice-liveness-runtime",
    not(feature = "unified-product-psci-shutdown-runtime")
))]
pub const ABI_VERSION: u64 = 30;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-liveness-runtime",
    not(feature = "unified-product-multiservice-liveness-runtime")
))]
pub const ABI_VERSION: u64 = 29;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "unified-product-runtime",
    not(feature = "unified-product-liveness-runtime")
))]
pub const ABI_VERSION: u64 = 28;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    feature = "resident-platform-shutdown-runtime",
    not(feature = "unified-product-runtime")
))]
pub const ABI_VERSION: u64 = 27;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    not(feature = "resident-platform-shutdown-runtime"),
    feature = "storage-server-shutdown-orchestration-runtime"
))]
pub const ABI_VERSION: u64 = 26;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    not(feature = "storage-server-shutdown-orchestration-runtime"),
    feature = "storage-server-runtime"
))]
pub const ABI_VERSION: u64 = 25;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    not(feature = "storage-server-runtime"),
    feature = "app-data-runtime"
))]
pub const ABI_VERSION: u64 = 24;
#[cfg(all(
    not(feature = "androidbox-apk-install0"),
    not(feature = "storage-server-runtime"),
    not(feature = "app-data-runtime"),
    feature = "mobile-ui-runtime"
))]
pub const ABI_VERSION: u64 = 24;
#[cfg(not(any(
    feature = "androidbox-apk-install0",
    feature = "storage-server-runtime",
    feature = "app-data-runtime",
    feature = "mobile-ui-runtime"
)))]
pub const ABI_VERSION: u64 = 23;
pub const SVC_USER_SYSCALL: u16 = 0;
pub const CHANNEL_MESSAGE_MAX_BYTES: usize = 64;
pub const CHANNEL_READ_ENVELOPE_SIZE: usize = 112;
/// ABI 47+'s authenticated App ↔ AndroidApp control frame fills one Channel
/// byte message so every read can be sender-stamped and traced atomically.
#[cfg(feature = "androidbox-process0")]
pub const ANDROID_APP_MESSAGE_WIRE_SIZE: usize = CHANNEL_MESSAGE_MAX_BYTES;
/// Maximum publisher TextView bytes accepted by the ABI 47+ UI bridge.
#[cfg(feature = "androidbox-process0")]
pub const ANDROID_APP_LABEL_MAX_BYTES: usize = 96;
/// Maximum publisher Button bytes accepted by the ABI 47+ UI bridge.
#[cfg(feature = "androidbox-process0")]
pub const ANDROID_APP_BUTTON_MAX_BYTES: usize = 64;
/// Payload bytes carried by one canonical ABI 47+ text chunk.
#[cfg(feature = "androidbox-process0")]
pub const ANDROID_APP_MESSAGE_PAYLOAD_BYTES: usize = 24;
/// Maximum number of nodes carried by one ABI 49 Scene-RPC-2 scene.
#[cfg(feature = "androidbox-scene-rpc2")]
pub const ANDROID_APP_SCENE_MAX_NODES: usize = 8;
/// Maximum text bytes declared by one ABI 49 Scene-RPC-2 node.
#[cfg(feature = "androidbox-scene-rpc2")]
pub const ANDROID_APP_SCENE_NODE_TEXT_MAX_BYTES: usize = ANDROID_APP_LABEL_MAX_BYTES;
/// Exact payload bytes used by one canonical ABI 49 node descriptor.
#[cfg(feature = "androidbox-scene-rpc2")]
#[cfg(feature = "androidbox-layout-directional17")]
pub const ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE: usize = 24;
#[cfg(not(feature = "androidbox-layout-directional17"))]
pub const ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE: usize = 16;
/// Canonical Init → App transferred-endpoint attachment record.
#[cfg(feature = "androidbox-process0")]
pub const ANDROID_APP_BOOTSTRAP_WIRE_SIZE: usize = 16;
#[cfg(feature = "androidbox-restart0")]
pub const ANDROID_APP_SUPERVISOR_WIRE_SIZE: usize = 64;
pub const CHILD_EXIT_MAGIC: u64 = 0xc11d_e817_baad_f00d;
pub const EVENT_CREATE_SIGNALED: u64 = 1 << 0;
/// Legacy syscall 19 accepts exactly two packed wait items in registers.
pub const OBJECT_WAIT_MANY_ITEM_COUNT: usize = 2;
/// One canonical `ObjectWaitManyArray` item is one little-endian packed `u64`.
pub const OBJECT_WAIT_MANY_ARRAY_ITEM_WIRE_SIZE: usize = 8;
/// Maximum number of wait items accepted by syscall 22.
pub const OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS: usize = 8;
/// Largest canonical wait-item array copied by one syscall 22 invocation.
pub const OBJECT_WAIT_MANY_ARRAY_MAX_WIRE_SIZE: usize =
    OBJECT_WAIT_MANY_ARRAY_ITEM_WIRE_SIZE * OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS;
pub const OBJECT_WAIT_TIMEOUT_POLL: u64 = 0;
pub const OBJECT_WAIT_TIMEOUT_INFINITE: u64 = u64::MAX;
pub const PROCESS_SPAWN_FLAGS_NONE: u64 = 0;
/// Forced process termination (introduced in ABI 18) accepts no optional flags.
pub const PROCESS_TERMINATE_FLAGS_NONE: u64 = 0;
/// Stable exit code reported for a process stopped by `ProcessTerminate`.
pub const PROCESS_KILLED_EXIT_CODE: u64 = 137;
/// M65's init-only two-phase shutdown syscall accepts no optional flags.
pub const SYSTEM_SHUTDOWN_FLAGS_NONE: u64 = 0;
/// The mobile preview's read-only system-clock syscall accepts no flags.
#[cfg(feature = "mobile-ui-runtime")]
pub const SYSTEM_CLOCK_READ_FLAGS_NONE: u64 = 0;
/// The clock value came from QEMU `virt`'s PL031 RTC data register.
#[cfg(feature = "mobile-ui-runtime")]
pub const SYSTEM_CLOCK_SOURCE_QEMU_PL031: u64 = 1;
/// Begins the bounded M65 shutdown admission barrier.
pub const SYSTEM_SHUTDOWN_PREPARE: u64 = 1;
/// Commits a previously prepared, generation-qualified M65 shutdown.
pub const SYSTEM_SHUTDOWN_COMMIT: u64 = 2;
/// M66 service-graph lifecycle calls accept no optional flags.
pub const SERVICE_SHUTDOWN_FLAGS_NONE: u64 = 0;
/// Registers one generation-qualified resident node before global Prepare.
pub const SERVICE_SHUTDOWN_REGISTER: u64 = 1;
/// Quiesces one registered node after global Prepare.
pub const SERVICE_SHUTDOWN_QUIESCE: u64 = 2;
/// ABI-33's kernel-owned service-manifest VMO accepts no caller-selected flags.
pub const SERVICE_MANIFEST_OPEN_FLAGS_NONE: u64 = 0;
/// M73's init-only supervisor-report operations carry no caller-selected flags.
pub const SERVICE_SUPERVISOR_REPORT_FLAGS_NONE: u64 = 0;
/// M77's init-only maintenance-session activation accepts no flags.
pub const MAINTENANCE_SESSION_OPEN_FLAGS_NONE: u64 = 0;
/// Queries the kernel-owned predecessor UI convergence seal without granting authority.
pub const SERVICE_SUPERVISOR_REPORT_UI_CONVERGENCE_QUERY: u64 = 0;
/// Publishes the first catalog-wide healthy runtime boundary.
pub const SERVICE_SUPERVISOR_REPORT_ACTIVE: u64 = 1;
/// Publishes an externally requested StorageServer rotation before shutdown.
pub const SERVICE_SUPERVISOR_REPORT_ROTATION_BEGIN: u64 = 2;
/// Publishes the mounted and health-checked replacement generation.
pub const SERVICE_SUPERVISOR_REPORT_ROTATION_COMPLETE: u64 = 3;
/// Publishes a batch with outstanding responses before the power-key request.
pub const SERVICE_SUPERVISOR_REPORT_CANCEL_WINDOW: u64 = 4;
/// Publishes the authenticated stop request and its outstanding-response count.
pub const SERVICE_SUPERVISOR_REPORT_STOP_REQUESTED: u64 = 5;
/// Publishes that every in-flight response was drained before shutdown begins.
pub const SERVICE_SUPERVISOR_REPORT_STOP_DRAINED: u64 = 6;
/// Number of non-storage nodes in M66's complete resident dependency graph.
pub const SHUTDOWN_SERVICE_NODE_COUNT: usize = 8;
/// Number of real Channel edges in M66's resident dependency graph.
pub const SHUTDOWN_SERVICE_EDGE_COUNT: usize = 10;
/// Number of reverse-topological quiesce waves in M66.
pub const SHUTDOWN_SERVICE_WAVE_COUNT: usize = 3;
/// Exact registered/quiesced bitmap for all M66 non-storage service nodes.
pub const SHUTDOWN_SERVICE_ALL_MASK: u64 = (1_u64 << SHUTDOWN_SERVICE_NODE_COUNT) - 1;
/// Longest canonical root-relative system-file path accepted by syscall 23.
pub const SYSTEM_FILE_PATH_MAX_BYTES: usize = 64;
/// Largest immutable VMO range copied by one syscall 24 invocation.
pub const VMO_READ_MAX_BYTES: usize = 4096;
/// The only graphics-buffer pixel format accepted since ABI 17.
pub const GRAPHICS_BUFFER_FORMAT_XRGB8888: u64 = 1;
/// Legacy copy-backed graphics-buffer creation uses no optional flags.
pub const GRAPHICS_BUFFER_CREATE_FLAGS_NONE: u64 = 0;
/// Opts a graphics buffer into ABI-19 shared mapping and queue semantics.
pub const GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE: u64 = 1 << 0;
/// Fixed EL0 virtual address used by the bounded ABI-19 graphics mapping.
pub const GRAPHICS_BUFFER_MAP_ADDRESS: u64 = 0x0000_0002_0010_0000;
/// Per-backing-slot virtual stride; the final five pages remain unmapped.
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const GRAPHICS_BUFFER_MAP_STRIDE: u64 = 0x0005_0000;
/// Page-aligned stride reserved by the large-screen preview ABI.
///
/// The mobile preview deliberately uses the legacy bounded-copy producer path
/// until the EL0 address-space hierarchy grows beyond one 2 MiB L3 table.
#[cfg(feature = "mobile-ui-runtime")]
pub const GRAPHICS_BUFFER_MAP_STRIDE: u64 = 0x0048_0000;
/// Requests the authenticated producer's read-write mapping.
pub const GRAPHICS_BUFFER_MAP_PRODUCER_RW: u64 = 1;
/// Requests the SurfaceServer consumer's read-only mapping.
pub const GRAPHICS_BUFFER_MAP_CONSUMER_RO: u64 = 2;
/// Queue, acquire, and release currently accept no optional flags.
pub const GRAPHICS_BUFFER_QUEUE_FLAGS_NONE: u64 = 0;
pub const GRAPHICS_BUFFER_ACQUIRE_FLAGS_NONE: u64 = 0;
pub const GRAPHICS_BUFFER_RELEASE_FLAGS_NONE: u64 = 0;
/// Fixed logical width of a graphics buffer in pixels.
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const GRAPHICS_BUFFER_WIDTH: u32 = 208;
/// Physical 20:9 display width used by the opt-in phone preview.
#[cfg(feature = "mobile-ui-runtime")]
pub const GRAPHICS_BUFFER_WIDTH: u32 = 720;
/// Fixed logical height of a graphics buffer in pixels.
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const GRAPHICS_BUFFER_HEIGHT: u32 = 368;
/// Physical 20:9 display height used by the opt-in phone preview.
#[cfg(feature = "mobile-ui-runtime")]
pub const GRAPHICS_BUFFER_HEIGHT: u32 = 1_600;
/// Number of pixels in one fixed graphics buffer.
pub const GRAPHICS_BUFFER_PIXEL_COUNT: usize =
    GRAPHICS_BUFFER_WIDTH as usize * GRAPHICS_BUFFER_HEIGHT as usize;
/// Number of meaningful XRGB8888 bytes in one fixed graphics buffer.
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const GRAPHICS_BUFFER_LOGICAL_BYTES: usize = 306_176;
#[cfg(feature = "mobile-ui-runtime")]
pub const GRAPHICS_BUFFER_LOGICAL_BYTES: usize = 4_608_000;
/// Page-rounded allocation size of one fixed graphics buffer.
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const GRAPHICS_BUFFER_BACKING_BYTES: usize = 307_200;
#[cfg(feature = "mobile-ui-runtime")]
pub const GRAPHICS_BUFFER_BACKING_BYTES: usize = 4_608_000;
/// Largest bounded user-memory copy accepted by one graphics-buffer write.
///
/// The default product keeps the historical one-page bound. The opt-in phone
/// preview accepts exactly twenty-two complete 720-pixel scanlines so one
/// frame needs 73 writes instead of 1,600. The kernel still copies this range
/// through independently bounded user-copy chunks before publishing one write
/// generation.
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const GRAPHICS_BUFFER_WRITE_MAX_BYTES: usize = 4_096;
#[cfg(feature = "mobile-ui-runtime")]
pub const GRAPHICS_BUFFER_WRITE_MAX_BYTES: usize = 63_360;
/// Maximum number of complete physical rows carried by one mobile write.
#[cfg(feature = "mobile-ui-runtime")]
pub const GRAPHICS_BUFFER_WRITE_MAX_ROWS: usize =
    GRAPHICS_BUFFER_WRITE_MAX_BYTES / (GRAPHICS_BUFFER_WIDTH as usize * 4);
/// Exact number of row-aligned writes used for one complete mobile frame.
#[cfg(feature = "mobile-ui-runtime")]
pub const GRAPHICS_BUFFER_FRAME_WRITE_CALLS: usize = GRAPHICS_BUFFER_HEIGHT as usize
    / GRAPHICS_BUFFER_WRITE_MAX_ROWS
    + if (GRAPHICS_BUFFER_HEIGHT as usize).is_multiple_of(GRAPHICS_BUFFER_WRITE_MAX_ROWS) {
        0
    } else {
        1
    };
/// Width of the canonical global input coordinate space.
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const INPUT_GLOBAL_WIDTH: u16 = 320;
#[cfg(feature = "mobile-ui-runtime")]
pub const INPUT_GLOBAL_WIDTH: u16 = 720;
/// Height of the canonical global input coordinate space.
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const INPUT_GLOBAL_HEIGHT: u16 = 480;
#[cfg(feature = "mobile-ui-runtime")]
pub const INPUT_GLOBAL_HEIGHT: u16 = 1_600;

/// Exact byte count copied by the isolated Android package snapshot syscall.
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE: usize = 640;
/// Maximum installed entries exposed by ABI 55's authority-free directory.
#[cfg(feature = "androidbox-multipackage4")]
pub const ANDROID_PACKAGE_DIRECTORY_CAPACITY: usize = 2;
/// Exact byte count copied by the ABI 55 package-directory syscall.
#[cfg(feature = "androidbox-multipackage4")]
pub const ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE: usize =
    64 + ANDROID_PACKAGE_DIRECTORY_CAPACITY * ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE;
/// Canonical package-directory marker `BNDAPD01`.
#[cfg(feature = "androidbox-multipackage4")]
pub const ANDROID_PACKAGE_DIRECTORY_MAGIC: [u8; 8] = *b"BNDAPD01";
#[cfg(feature = "androidbox-multipackage4")]
pub const ANDROID_PACKAGE_DIRECTORY_VERSION: u32 = 1;
#[cfg(feature = "androidbox-multipackage4")]
pub const ANDROID_PACKAGE_DIRECTORY_FLAG_FORMATTED: u32 = 1;
/// Exact byte count copied by ABI 56's package-bound launcher-icon syscall.
#[cfg(feature = "androidbox-icon-resources5")]
pub const ANDROID_PACKAGE_ICON_WIRE_SIZE: usize = 1_152;
/// Canonical launcher-icon marker `BNDAIC01`.
#[cfg(feature = "androidbox-icon-resources5")]
pub const ANDROID_PACKAGE_ICON_MAGIC: [u8; 8] = *b"BNDAIC01";
#[cfg(feature = "androidbox-density-icons7")]
pub const ANDROID_PACKAGE_ICON_VERSION: u32 = 2;
#[cfg(all(
    feature = "androidbox-icon-resources5",
    not(feature = "androidbox-density-icons7")
))]
pub const ANDROID_PACKAGE_ICON_VERSION: u32 = 1;
#[cfg(feature = "androidbox-icon-resources5")]
pub const ANDROID_PACKAGE_ICON_FLAG_PRESENT: u32 = 1;
/// ABI 57 normalized an exact mdpi 48×48 resource to the retained 16×16 wire.
#[cfg(feature = "androidbox-density-icons7")]
pub const ANDROID_PACKAGE_ICON_FLAG_DENSITY_NORMALIZED: u32 = 1 << 1;
/// ABI 57 deterministically reduced more than sixteen decoded colors.
#[cfg(feature = "androidbox-density-icons7")]
pub const ANDROID_PACKAGE_ICON_FLAG_COLOR_QUANTIZED: u32 = 1 << 2;
#[cfg(feature = "androidbox-density-icons7")]
pub const ANDROID_PACKAGE_ICON_SOURCE_MDP_WIDTH: u16 = 48;
#[cfg(feature = "androidbox-density-icons7")]
pub const ANDROID_PACKAGE_ICON_SOURCE_MDP_HEIGHT: u16 = 48;
#[cfg(feature = "androidbox-density-icons7")]
pub const ANDROID_PACKAGE_ICON_SOURCE_MDP_DENSITY_DPI: u16 = 160;
#[cfg(feature = "androidbox-icon-resources5")]
pub const ANDROID_PACKAGE_ICON_WIDTH: usize = 16;
#[cfg(feature = "androidbox-icon-resources5")]
pub const ANDROID_PACKAGE_ICON_HEIGHT: usize = 16;
#[cfg(feature = "androidbox-icon-resources5")]
pub const ANDROID_PACKAGE_ICON_PIXEL_COUNT: usize =
    ANDROID_PACKAGE_ICON_WIDTH * ANDROID_PACKAGE_ICON_HEIGHT;
/// Exact byte count of the Launcher-owned relaunch exchange.
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE: usize = 640;
/// Exact exchange size for ABI 52's runtime uninstall request/result.
#[cfg(feature = "androidbox-runtime-uninstall1")]
pub const ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE: usize = 256;
/// Canonical eight-byte runtime uninstall request marker `BNDURQ01`.
#[cfg(feature = "androidbox-runtime-uninstall1")]
pub const ANDROID_PACKAGE_UNINSTALL_REQUEST_MAGIC: [u8; 8] = *b"BNDURQ01";
/// Canonical eight-byte runtime uninstall completion marker `BNDURT01`.
#[cfg(feature = "androidbox-runtime-uninstall1")]
pub const ANDROID_PACKAGE_UNINSTALL_RESULT_MAGIC: [u8; 8] = *b"BNDURT01";
#[cfg(feature = "androidbox-runtime-uninstall1")]
pub const ANDROID_PACKAGE_UNINSTALL_WIRE_VERSION: u32 = 1;
#[cfg(feature = "androidbox-runtime-uninstall1")]
pub const ANDROID_PACKAGE_UNINSTALL_FLAGS_NONE: u32 = 0;
#[cfg(feature = "androidbox-runtime-uninstall1")]
pub const ANDROID_PACKAGE_UNINSTALL_DATA_KEEP_NONE: u16 = 1;
/// Exact byte count of ABI 53's authority-free install/update candidate.
#[cfg(feature = "androidbox-runtime-install2")]
pub const ANDROID_PACKAGE_INSTALL_CANDIDATE_WIRE_SIZE: usize = 640;
/// Exact exchange size of ABI 53's App-owned runtime install transaction.
#[cfg(feature = "androidbox-runtime-install2")]
pub const ANDROID_PACKAGE_INSTALL_WIRE_SIZE: usize = 256;
/// Canonical read-only install-candidate marker `BNDICS01`.
#[cfg(feature = "androidbox-runtime-install2")]
pub const ANDROID_PACKAGE_INSTALL_CANDIDATE_MAGIC: [u8; 8] = *b"BNDICS01";
/// Canonical runtime-install request marker `BNDIRQ01`.
#[cfg(feature = "androidbox-runtime-install2")]
pub const ANDROID_PACKAGE_INSTALL_REQUEST_MAGIC: [u8; 8] = *b"BNDIRQ01";
/// Canonical runtime-install completion marker `BNDIRT01`.
#[cfg(feature = "androidbox-runtime-install2")]
pub const ANDROID_PACKAGE_INSTALL_RESULT_MAGIC: [u8; 8] = *b"BNDIRT01";
#[cfg(feature = "androidbox-runtime-install2")]
pub const ANDROID_PACKAGE_INSTALL_WIRE_VERSION: u32 = 1;
#[cfg(feature = "androidbox-runtime-install2")]
pub const ANDROID_PACKAGE_INSTALL_FLAGS_NONE: u32 = 0;
/// Exact byte count of the App-owned, one-shot package-image claim.
#[cfg(feature = "androidbox-el0-runtime0")]
pub const ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE: usize = 128;
/// Canonical eight-byte image-claim marker `BNDACM01`.
#[cfg(feature = "androidbox-el0-runtime0")]
pub const ANDROID_PACKAGE_IMAGE_CLAIM_MAGIC: [u8; 8] = *b"BNDACM01";
#[cfg(feature = "androidbox-el0-runtime0")]
pub const ANDROID_PACKAGE_IMAGE_CLAIM_VERSION: u32 = 1;
/// Image claims accept no optional flags.
#[cfg(feature = "androidbox-el0-runtime0")]
pub const ANDROID_PACKAGE_IMAGE_CLAIM_FLAGS_NONE: u32 = 0;
/// Canonical eight-byte relaunch-request marker `BNDARQ01`.
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_RELAUNCH_REQUEST_MAGIC: [u8; 8] = *b"BNDARQ01";
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_RELAUNCH_REQUEST_VERSION: u32 = 1;
/// Relaunch requests accept no optional flags.
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_RELAUNCH_REQUEST_FLAGS_NONE: u32 = 0;
/// Canonical eight-byte wire marker `BNDAPS01`.
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_SNAPSHOT_MAGIC: [u8; 8] = *b"BNDAPS01";
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_SNAPSHOT_VERSION: u32 = 1;
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMATTED: u32 = 1 << 0;
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_SNAPSHOT_FLAG_INSTALLED: u32 = 1 << 1;
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_PRESENT: u32 = 1 << 2;
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_USED: u32 = 1 << 3;
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMAT_PERFORMED: u32 = 1 << 4;
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_SNAPSHOT_KNOWN_FLAGS: u32 = ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMATTED
    | ANDROID_PACKAGE_SNAPSHOT_FLAG_INSTALLED
    | ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_PRESENT
    | ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_USED
    | ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMAT_PERFORMED;
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_APK_MAX_BYTES: u32 = 65_024;
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_NAME_MAX_BYTES: usize = 96;
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_ACTIVITY_MAX_BYTES: usize = 128;
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_TITLE_MAX_BYTES: usize = 128;
#[cfg(feature = "androidbox-apk-install0")]
pub const ANDROID_PACKAGE_TEXT_MAX_BYTES: usize = 128;

#[cfg(feature = "androidbox-apk-install0")]
const _: () = assert!(152 + ANDROID_PACKAGE_NAME_MAX_BYTES == 248);
#[cfg(feature = "androidbox-apk-install0")]
const _: () = assert!(248 + ANDROID_PACKAGE_ACTIVITY_MAX_BYTES == 376);
#[cfg(feature = "androidbox-apk-install0")]
const _: () = assert!(376 + ANDROID_PACKAGE_TITLE_MAX_BYTES == 504);
#[cfg(feature = "androidbox-apk-install0")]
const _: () =
    assert!(504 + ANDROID_PACKAGE_TEXT_MAX_BYTES + 8 == ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE);
#[cfg(feature = "androidbox-apk-install0")]
const _: () = assert!(120 + ANDROID_PACKAGE_NAME_MAX_BYTES == 216);
#[cfg(feature = "androidbox-apk-install0")]
const _: () = assert!(216 + ANDROID_PACKAGE_ACTIVITY_MAX_BYTES == 344);
#[cfg(feature = "androidbox-apk-install0")]
const _: () =
    assert!(ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE == ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE);

/// The compatibility subset that admitted an installed APK.
#[cfg(feature = "androidbox-apk-install0")]
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageCompatibilityProfile {
    Resources1 = 2,
}

#[cfg(feature = "androidbox-apk-install0")]
impl AndroidPackageCompatibilityProfile {
    pub const fn raw(self) -> u16 {
        self as u16
    }

    pub const fn from_raw(raw: u16) -> Option<Self> {
        match raw {
            2 => Some(Self::Resources1),
            _ => None,
        }
    }
}

/// Canonical generation-bound request submitted to syscall 60.
///
/// The value contains only immutable installed-package identity. In
/// particular, title and rendered text are deliberately absent: a successful
/// relaunch must derive presentation output from the re-read durable APK.
#[cfg(feature = "androidbox-apk-install0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageRelaunchRequest {
    request_sequence: u64,
    expected_generation: u64,
    expected_version_code: u64,
    apk_length: u32,
    package_name_length: u16,
    activity_name_length: u16,
    apk_sha256: [u8; 32],
    signer_sha256: [u8; 32],
    package_name: [u8; ANDROID_PACKAGE_NAME_MAX_BYTES],
    activity_name: [u8; ANDROID_PACKAGE_ACTIVITY_MAX_BYTES],
}

#[cfg(feature = "androidbox-apk-install0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageRelaunchRequestError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    NonZeroFlags,
    InvalidRequestSequence,
    InvalidExpectedGeneration,
    InvalidExpectedVersionCode,
    InvalidApkLength,
    InvalidProfile,
    ZeroApkSha256,
    ZeroSignerSha256,
    InvalidPackageNameLength,
    InvalidActivityNameLength,
    NonPrintablePackageName,
    NonPrintableActivityName,
    NonZeroPackageNamePadding,
    NonZeroActivityNamePadding,
    NonZeroReserved,
}

#[cfg(feature = "androidbox-apk-install0")]
impl AndroidPackageRelaunchRequest {
    /// Constructs one canonical Resources-1 relaunch request.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_sequence: u64,
        expected_generation: u64,
        expected_version_code: u64,
        apk_length: u32,
        apk_sha256: [u8; 32],
        signer_sha256: [u8; 32],
        package_name: &[u8],
        activity_name: &[u8],
    ) -> Result<Self, AndroidPackageRelaunchRequestError> {
        if request_sequence == 0 {
            return Err(AndroidPackageRelaunchRequestError::InvalidRequestSequence);
        }
        if expected_generation == 0 {
            return Err(AndroidPackageRelaunchRequestError::InvalidExpectedGeneration);
        }
        if expected_version_code == 0 {
            return Err(AndroidPackageRelaunchRequestError::InvalidExpectedVersionCode);
        }
        if apk_length == 0 || apk_length > ANDROID_PACKAGE_APK_MAX_BYTES {
            return Err(AndroidPackageRelaunchRequestError::InvalidApkLength);
        }
        if apk_sha256.iter().all(|byte| *byte == 0) {
            return Err(AndroidPackageRelaunchRequestError::ZeroApkSha256);
        }
        if signer_sha256.iter().all(|byte| *byte == 0) {
            return Err(AndroidPackageRelaunchRequestError::ZeroSignerSha256);
        }
        validate_android_package_relaunch_text(
            package_name,
            ANDROID_PACKAGE_NAME_MAX_BYTES,
            AndroidPackageRelaunchRequestError::InvalidPackageNameLength,
            AndroidPackageRelaunchRequestError::NonPrintablePackageName,
        )?;
        validate_android_package_relaunch_text(
            activity_name,
            ANDROID_PACKAGE_ACTIVITY_MAX_BYTES,
            AndroidPackageRelaunchRequestError::InvalidActivityNameLength,
            AndroidPackageRelaunchRequestError::NonPrintableActivityName,
        )?;

        let mut canonical_package_name = [0; ANDROID_PACKAGE_NAME_MAX_BYTES];
        canonical_package_name[..package_name.len()].copy_from_slice(package_name);
        let mut canonical_activity_name = [0; ANDROID_PACKAGE_ACTIVITY_MAX_BYTES];
        canonical_activity_name[..activity_name.len()].copy_from_slice(activity_name);
        Ok(Self {
            request_sequence,
            expected_generation,
            expected_version_code,
            apk_length,
            package_name_length: package_name.len() as u16,
            activity_name_length: activity_name.len() as u16,
            apk_sha256,
            signer_sha256,
            package_name: canonical_package_name,
            activity_name: canonical_activity_name,
        })
    }

    pub const fn request_sequence(&self) -> u64 {
        self.request_sequence
    }

    pub const fn expected_generation(&self) -> u64 {
        self.expected_generation
    }

    pub const fn expected_version_code(&self) -> u64 {
        self.expected_version_code
    }

    pub const fn apk_length(&self) -> u32 {
        self.apk_length
    }

    pub const fn profile(&self) -> AndroidPackageCompatibilityProfile {
        AndroidPackageCompatibilityProfile::Resources1
    }

    pub const fn apk_sha256(&self) -> &[u8; 32] {
        &self.apk_sha256
    }

    pub const fn signer_sha256(&self) -> &[u8; 32] {
        &self.signer_sha256
    }

    pub fn package_name(&self) -> &str {
        core::str::from_utf8(self.package_name_bytes()).expect("validated package name")
    }

    pub fn package_name_bytes(&self) -> &[u8] {
        &self.package_name[..self.package_name_length as usize]
    }

    pub fn activity_name(&self) -> &str {
        core::str::from_utf8(self.activity_name_bytes()).expect("validated activity name")
    }

    pub fn activity_name_bytes(&self) -> &[u8] {
        &self.activity_name[..self.activity_name_length as usize]
    }

    /// Encodes the canonical 640-byte little-endian `BNDARQ01` wire.
    pub fn encode(&self) -> [u8; ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE] {
        let mut wire = [0; ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE];
        wire[0..8].copy_from_slice(&ANDROID_PACKAGE_RELAUNCH_REQUEST_MAGIC);
        write_app_data_u32(&mut wire, 8, ANDROID_PACKAGE_RELAUNCH_REQUEST_VERSION);
        write_app_data_u32(&mut wire, 12, ANDROID_PACKAGE_RELAUNCH_REQUEST_FLAGS_NONE);
        write_app_data_u64(&mut wire, 16, self.request_sequence);
        write_app_data_u64(&mut wire, 24, self.expected_generation);
        write_app_data_u64(&mut wire, 32, self.expected_version_code);
        write_app_data_u32(&mut wire, 40, self.apk_length);
        write_app_data_u16(&mut wire, 44, self.package_name_length);
        write_app_data_u16(&mut wire, 46, self.activity_name_length);
        write_app_data_u16(
            &mut wire,
            48,
            AndroidPackageCompatibilityProfile::Resources1.raw(),
        );
        wire[56..88].copy_from_slice(&self.apk_sha256);
        wire[88..120].copy_from_slice(&self.signer_sha256);
        wire[120..216].copy_from_slice(&self.package_name);
        wire[216..344].copy_from_slice(&self.activity_name);
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, AndroidPackageRelaunchRequestError> {
        if wire.len() != ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE {
            return Err(AndroidPackageRelaunchRequestError::InvalidWireLength);
        }
        if wire[0..8] != ANDROID_PACKAGE_RELAUNCH_REQUEST_MAGIC {
            return Err(AndroidPackageRelaunchRequestError::InvalidMagic);
        }
        if read_app_data_u32(wire, 8) != ANDROID_PACKAGE_RELAUNCH_REQUEST_VERSION {
            return Err(AndroidPackageRelaunchRequestError::UnsupportedVersion);
        }
        if read_app_data_u32(wire, 12) != ANDROID_PACKAGE_RELAUNCH_REQUEST_FLAGS_NONE {
            return Err(AndroidPackageRelaunchRequestError::NonZeroFlags);
        }
        if wire[50..56].iter().any(|byte| *byte != 0)
            || wire[344..640].iter().any(|byte| *byte != 0)
        {
            return Err(AndroidPackageRelaunchRequestError::NonZeroReserved);
        }
        if AndroidPackageCompatibilityProfile::from_raw(read_app_data_u16(wire, 48))
            != Some(AndroidPackageCompatibilityProfile::Resources1)
        {
            return Err(AndroidPackageRelaunchRequestError::InvalidProfile);
        }

        let package_name_length = read_app_data_u16(wire, 44) as usize;
        let activity_name_length = read_app_data_u16(wire, 46) as usize;
        validate_android_package_relaunch_wire_text(
            wire,
            120,
            ANDROID_PACKAGE_NAME_MAX_BYTES,
            package_name_length,
            AndroidPackageRelaunchRequestError::InvalidPackageNameLength,
            AndroidPackageRelaunchRequestError::NonZeroPackageNamePadding,
        )?;
        validate_android_package_relaunch_wire_text(
            wire,
            216,
            ANDROID_PACKAGE_ACTIVITY_MAX_BYTES,
            activity_name_length,
            AndroidPackageRelaunchRequestError::InvalidActivityNameLength,
            AndroidPackageRelaunchRequestError::NonZeroActivityNamePadding,
        )?;

        let mut apk_sha256 = [0; 32];
        apk_sha256.copy_from_slice(&wire[56..88]);
        let mut signer_sha256 = [0; 32];
        signer_sha256.copy_from_slice(&wire[88..120]);
        Self::new(
            read_app_data_u64(wire, 16),
            read_app_data_u64(wire, 24),
            read_app_data_u64(wire, 32),
            read_app_data_u32(wire, 40),
            apk_sha256,
            signer_sha256,
            &wire[120..120 + package_name_length],
            &wire[216..216 + activity_name_length],
        )
    }
}

/// Canonical, full-identity request for ABI 52's single-package uninstall.
///
/// The request contains no path, block address, storage handle, or arbitrary
/// data-deletion selector. The kernel must match every identity field against
/// its live installed catalog before it can enqueue the durable tombstone.
#[cfg(feature = "androidbox-runtime-uninstall1")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageUninstallRequest {
    request_sequence: u64,
    operation_id: u64,
    expected_generation: u64,
    expected_version_code: u64,
    expected_apk_length: u32,
    package_name_length: u16,
    expected_apk_sha256: [u8; 32],
    expected_signer_sha256: [u8; 32],
    package_name: [u8; ANDROID_PACKAGE_NAME_MAX_BYTES],
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageUninstallRequestError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    NonZeroFlags,
    InvalidRequestSequence,
    InvalidOperationId,
    InvalidExpectedGeneration,
    InvalidExpectedVersionCode,
    InvalidApkLength,
    InvalidProfile,
    ZeroApkSha256,
    ZeroSignerSha256,
    InvalidPackageNameLength,
    NonPrintablePackageName,
    NonZeroPackageNamePadding,
    NonZeroReserved,
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
impl AndroidPackageUninstallRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_sequence: u64,
        operation_id: u64,
        expected_generation: u64,
        expected_version_code: u64,
        expected_apk_length: u32,
        expected_apk_sha256: [u8; 32],
        expected_signer_sha256: [u8; 32],
        package_name: &[u8],
    ) -> Result<Self, AndroidPackageUninstallRequestError> {
        if request_sequence == 0 {
            return Err(AndroidPackageUninstallRequestError::InvalidRequestSequence);
        }
        if operation_id == 0 {
            return Err(AndroidPackageUninstallRequestError::InvalidOperationId);
        }
        if expected_generation == 0 {
            return Err(AndroidPackageUninstallRequestError::InvalidExpectedGeneration);
        }
        if expected_version_code == 0 {
            return Err(AndroidPackageUninstallRequestError::InvalidExpectedVersionCode);
        }
        if expected_apk_length == 0 || expected_apk_length > ANDROID_PACKAGE_APK_MAX_BYTES {
            return Err(AndroidPackageUninstallRequestError::InvalidApkLength);
        }
        if expected_apk_sha256.iter().all(|byte| *byte == 0) {
            return Err(AndroidPackageUninstallRequestError::ZeroApkSha256);
        }
        if expected_signer_sha256.iter().all(|byte| *byte == 0) {
            return Err(AndroidPackageUninstallRequestError::ZeroSignerSha256);
        }
        if package_name.is_empty() || package_name.len() > ANDROID_PACKAGE_NAME_MAX_BYTES {
            return Err(AndroidPackageUninstallRequestError::InvalidPackageNameLength);
        }
        if package_name
            .iter()
            .any(|byte| !(0x20..=0x7e).contains(byte))
        {
            return Err(AndroidPackageUninstallRequestError::NonPrintablePackageName);
        }
        let mut canonical_package_name = [0; ANDROID_PACKAGE_NAME_MAX_BYTES];
        canonical_package_name[..package_name.len()].copy_from_slice(package_name);
        Ok(Self {
            request_sequence,
            operation_id,
            expected_generation,
            expected_version_code,
            expected_apk_length,
            package_name_length: package_name.len() as u16,
            expected_apk_sha256,
            expected_signer_sha256,
            package_name: canonical_package_name,
        })
    }

    pub const fn request_sequence(&self) -> u64 {
        self.request_sequence
    }

    pub const fn operation_id(&self) -> u64 {
        self.operation_id
    }

    pub const fn expected_generation(&self) -> u64 {
        self.expected_generation
    }

    pub const fn expected_version_code(&self) -> u64 {
        self.expected_version_code
    }

    pub const fn expected_apk_length(&self) -> u32 {
        self.expected_apk_length
    }

    pub const fn profile(&self) -> AndroidPackageCompatibilityProfile {
        AndroidPackageCompatibilityProfile::Resources1
    }

    pub const fn expected_apk_sha256(&self) -> &[u8; 32] {
        &self.expected_apk_sha256
    }

    pub const fn expected_signer_sha256(&self) -> &[u8; 32] {
        &self.expected_signer_sha256
    }

    pub fn package_name(&self) -> &str {
        core::str::from_utf8(self.package_name_bytes()).expect("validated package name")
    }

    pub fn package_name_bytes(&self) -> &[u8] {
        &self.package_name[..self.package_name_length as usize]
    }

    pub fn encode(&self) -> [u8; ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE] {
        let mut wire = [0; ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE];
        wire[..8].copy_from_slice(&ANDROID_PACKAGE_UNINSTALL_REQUEST_MAGIC);
        write_app_data_u32(&mut wire, 8, ANDROID_PACKAGE_UNINSTALL_WIRE_VERSION);
        write_app_data_u32(&mut wire, 12, ANDROID_PACKAGE_UNINSTALL_FLAGS_NONE);
        write_app_data_u64(&mut wire, 16, self.request_sequence);
        write_app_data_u64(&mut wire, 24, self.operation_id);
        write_app_data_u64(&mut wire, 32, self.expected_generation);
        write_app_data_u64(&mut wire, 40, self.expected_version_code);
        write_app_data_u32(&mut wire, 48, self.expected_apk_length);
        write_app_data_u16(&mut wire, 52, self.package_name_length);
        write_app_data_u16(
            &mut wire,
            54,
            AndroidPackageCompatibilityProfile::Resources1.raw(),
        );
        wire[56..88].copy_from_slice(&self.expected_apk_sha256);
        wire[88..120].copy_from_slice(&self.expected_signer_sha256);
        wire[120..216].copy_from_slice(&self.package_name);
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, AndroidPackageUninstallRequestError> {
        if wire.len() != ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE {
            return Err(AndroidPackageUninstallRequestError::InvalidWireLength);
        }
        if wire[..8] != ANDROID_PACKAGE_UNINSTALL_REQUEST_MAGIC {
            return Err(AndroidPackageUninstallRequestError::InvalidMagic);
        }
        if read_app_data_u32(wire, 8) != ANDROID_PACKAGE_UNINSTALL_WIRE_VERSION {
            return Err(AndroidPackageUninstallRequestError::UnsupportedVersion);
        }
        if read_app_data_u32(wire, 12) != ANDROID_PACKAGE_UNINSTALL_FLAGS_NONE {
            return Err(AndroidPackageUninstallRequestError::NonZeroFlags);
        }
        if AndroidPackageCompatibilityProfile::from_raw(read_app_data_u16(wire, 54))
            != Some(AndroidPackageCompatibilityProfile::Resources1)
        {
            return Err(AndroidPackageUninstallRequestError::InvalidProfile);
        }
        if wire[216..].iter().any(|byte| *byte != 0) {
            return Err(AndroidPackageUninstallRequestError::NonZeroReserved);
        }
        let package_name_length = read_app_data_u16(wire, 52) as usize;
        if package_name_length == 0 || package_name_length > ANDROID_PACKAGE_NAME_MAX_BYTES {
            return Err(AndroidPackageUninstallRequestError::InvalidPackageNameLength);
        }
        if wire[120..120 + package_name_length]
            .iter()
            .any(|byte| !(0x20..=0x7e).contains(byte))
        {
            return Err(AndroidPackageUninstallRequestError::NonPrintablePackageName);
        }
        if wire[120 + package_name_length..216]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(AndroidPackageUninstallRequestError::NonZeroPackageNamePadding);
        }
        let mut apk_sha256 = [0; 32];
        apk_sha256.copy_from_slice(&wire[56..88]);
        let mut signer_sha256 = [0; 32];
        signer_sha256.copy_from_slice(&wire[88..120]);
        Self::new(
            read_app_data_u64(wire, 16),
            read_app_data_u64(wire, 24),
            read_app_data_u64(wire, 32),
            read_app_data_u64(wire, 40),
            read_app_data_u32(wire, 48),
            apk_sha256,
            signer_sha256,
            &wire[120..120 + package_name_length],
        )
    }
}

/// Exact identity required to consume one freshly verified package image.
///
/// The claim carries no storage location or mutable authority. A successful
/// syscall 61 consumes a boot-local grant bound to this complete value and
/// returns one non-transferable, non-duplicable, read-only VMO handle.
#[cfg(feature = "androidbox-el0-runtime0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageImageClaim {
    compatible_session_id: u64,
    package_generation: u64,
    apk_length: u32,
    apk_sha256: [u8; 32],
    signer_sha256: [u8; 32],
}

#[cfg(feature = "androidbox-el0-runtime0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageImageClaimError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    NonZeroFlags,
    InvalidCompatibleSessionId,
    InvalidPackageGeneration,
    InvalidApkLength,
    InvalidProfile,
    ZeroApkSha256,
    ZeroSignerSha256,
    NonZeroReserved,
}

#[cfg(feature = "androidbox-el0-runtime0")]
impl AndroidPackageImageClaim {
    pub fn new(
        compatible_session_id: u64,
        package_generation: u64,
        apk_length: u32,
        apk_sha256: [u8; 32],
        signer_sha256: [u8; 32],
    ) -> Result<Self, AndroidPackageImageClaimError> {
        if compatible_session_id == 0 {
            return Err(AndroidPackageImageClaimError::InvalidCompatibleSessionId);
        }
        if package_generation == 0 {
            return Err(AndroidPackageImageClaimError::InvalidPackageGeneration);
        }
        if apk_length == 0 || apk_length > ANDROID_PACKAGE_APK_MAX_BYTES {
            return Err(AndroidPackageImageClaimError::InvalidApkLength);
        }
        if apk_sha256.iter().all(|byte| *byte == 0) {
            return Err(AndroidPackageImageClaimError::ZeroApkSha256);
        }
        if signer_sha256.iter().all(|byte| *byte == 0) {
            return Err(AndroidPackageImageClaimError::ZeroSignerSha256);
        }
        Ok(Self {
            compatible_session_id,
            package_generation,
            apk_length,
            apk_sha256,
            signer_sha256,
        })
    }

    pub const fn compatible_session_id(&self) -> u64 {
        self.compatible_session_id
    }

    pub const fn package_generation(&self) -> u64 {
        self.package_generation
    }

    pub const fn apk_length(&self) -> u32 {
        self.apk_length
    }

    pub const fn profile(&self) -> AndroidPackageCompatibilityProfile {
        AndroidPackageCompatibilityProfile::Resources1
    }

    pub const fn apk_sha256(&self) -> &[u8; 32] {
        &self.apk_sha256
    }

    pub const fn signer_sha256(&self) -> &[u8; 32] {
        &self.signer_sha256
    }

    pub fn encode(&self) -> [u8; ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE] {
        let mut wire = [0; ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE];
        wire[0..8].copy_from_slice(&ANDROID_PACKAGE_IMAGE_CLAIM_MAGIC);
        write_app_data_u32(&mut wire, 8, ANDROID_PACKAGE_IMAGE_CLAIM_VERSION);
        write_app_data_u32(&mut wire, 12, ANDROID_PACKAGE_IMAGE_CLAIM_FLAGS_NONE);
        write_app_data_u64(&mut wire, 16, self.compatible_session_id);
        write_app_data_u64(&mut wire, 24, self.package_generation);
        write_app_data_u32(&mut wire, 32, self.apk_length);
        write_app_data_u16(
            &mut wire,
            36,
            AndroidPackageCompatibilityProfile::Resources1.raw(),
        );
        wire[40..72].copy_from_slice(&self.apk_sha256);
        wire[72..104].copy_from_slice(&self.signer_sha256);
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, AndroidPackageImageClaimError> {
        if wire.len() != ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE {
            return Err(AndroidPackageImageClaimError::InvalidWireLength);
        }
        if wire[0..8] != ANDROID_PACKAGE_IMAGE_CLAIM_MAGIC {
            return Err(AndroidPackageImageClaimError::InvalidMagic);
        }
        if read_app_data_u32(wire, 8) != ANDROID_PACKAGE_IMAGE_CLAIM_VERSION {
            return Err(AndroidPackageImageClaimError::UnsupportedVersion);
        }
        if read_app_data_u32(wire, 12) != ANDROID_PACKAGE_IMAGE_CLAIM_FLAGS_NONE {
            return Err(AndroidPackageImageClaimError::NonZeroFlags);
        }
        if wire[38..40].iter().any(|byte| *byte != 0) || wire[104..].iter().any(|byte| *byte != 0) {
            return Err(AndroidPackageImageClaimError::NonZeroReserved);
        }
        if AndroidPackageCompatibilityProfile::from_raw(read_app_data_u16(wire, 36))
            != Some(AndroidPackageCompatibilityProfile::Resources1)
        {
            return Err(AndroidPackageImageClaimError::InvalidProfile);
        }
        let mut apk_sha256 = [0; 32];
        apk_sha256.copy_from_slice(&wire[40..72]);
        let mut signer_sha256 = [0; 32];
        signer_sha256.copy_from_slice(&wire[72..104]);
        Self::new(
            read_app_data_u64(wire, 16),
            read_app_data_u64(wire, 24),
            read_app_data_u32(wire, 32),
            apk_sha256,
            signer_sha256,
        )
    }
}

/// Package-volume and boot-source state supplied to a snapshot constructor.
#[cfg(feature = "androidbox-apk-install0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageSnapshotState {
    formatted: bool,
    source_present: bool,
    source_used: bool,
    format_performed: bool,
}

#[cfg(feature = "androidbox-apk-install0")]
impl AndroidPackageSnapshotState {
    pub const fn new(
        formatted: bool,
        source_present: bool,
        source_used: bool,
        format_performed: bool,
    ) -> Self {
        Self {
            formatted,
            source_present,
            source_used,
            format_performed,
        }
    }

    pub const fn formatted(self) -> bool {
        self.formatted
    }

    pub const fn source_present(self) -> bool {
        self.source_present
    }

    pub const fn source_used(self) -> bool {
        self.source_used
    }

    pub const fn format_performed(self) -> bool {
        self.format_performed
    }
}

/// Read-only package-volume I/O counters captured with the snapshot.
#[cfg(feature = "androidbox-apk-install0")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AndroidPackageIoCounters {
    reads: u64,
    writes: u64,
    flushes: u64,
}

#[cfg(feature = "androidbox-apk-install0")]
impl AndroidPackageIoCounters {
    pub const fn new(reads: u64, writes: u64, flushes: u64) -> Self {
        Self {
            reads,
            writes,
            flushes,
        }
    }

    pub const fn reads(self) -> u64 {
        self.reads
    }

    pub const fn writes(self) -> u64 {
        self.writes
    }

    pub const fn flushes(self) -> u64 {
        self.flushes
    }
}

/// Canonical terminal result replacing a successful runtime-uninstall request.
#[cfg(feature = "androidbox-runtime-uninstall1")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageUninstallResult {
    request_sequence: u64,
    operation_id: u64,
    removal_generation: u64,
    last_installed_generation: u64,
    io: AndroidPackageIoCounters,
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageUninstallResultError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    NonZeroFlags,
    InvalidRequestSequence,
    InvalidOperationId,
    InvalidRemovalGeneration,
    InvalidLastInstalledGeneration,
    InvalidGenerationTransition,
    InvalidIoEvidence,
    UnsupportedDataDisposition,
    NonZeroReserved,
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
impl AndroidPackageUninstallResult {
    pub fn new(
        request_sequence: u64,
        operation_id: u64,
        removal_generation: u64,
        last_installed_generation: u64,
        io: AndroidPackageIoCounters,
    ) -> Result<Self, AndroidPackageUninstallResultError> {
        if request_sequence == 0 {
            return Err(AndroidPackageUninstallResultError::InvalidRequestSequence);
        }
        if operation_id == 0 {
            return Err(AndroidPackageUninstallResultError::InvalidOperationId);
        }
        if removal_generation == 0 {
            return Err(AndroidPackageUninstallResultError::InvalidRemovalGeneration);
        }
        if last_installed_generation == 0 {
            return Err(AndroidPackageUninstallResultError::InvalidLastInstalledGeneration);
        }
        if last_installed_generation.checked_add(1) != Some(removal_generation) {
            return Err(AndroidPackageUninstallResultError::InvalidGenerationTransition);
        }
        if io.reads() == 0 || io.writes() == 0 || io.flushes() == 0 {
            return Err(AndroidPackageUninstallResultError::InvalidIoEvidence);
        }
        Ok(Self {
            request_sequence,
            operation_id,
            removal_generation,
            last_installed_generation,
            io,
        })
    }

    pub const fn request_sequence(self) -> u64 {
        self.request_sequence
    }

    pub const fn operation_id(self) -> u64 {
        self.operation_id
    }

    pub const fn removal_generation(self) -> u64 {
        self.removal_generation
    }

    pub const fn last_installed_generation(self) -> u64 {
        self.last_installed_generation
    }

    pub const fn io_counters(self) -> AndroidPackageIoCounters {
        self.io
    }

    pub fn encode(self) -> [u8; ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE] {
        let mut wire = [0; ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE];
        wire[..8].copy_from_slice(&ANDROID_PACKAGE_UNINSTALL_RESULT_MAGIC);
        write_app_data_u32(&mut wire, 8, ANDROID_PACKAGE_UNINSTALL_WIRE_VERSION);
        write_app_data_u32(&mut wire, 12, ANDROID_PACKAGE_UNINSTALL_FLAGS_NONE);
        write_app_data_u64(&mut wire, 16, self.request_sequence);
        write_app_data_u64(&mut wire, 24, self.operation_id);
        write_app_data_u64(&mut wire, 32, self.removal_generation);
        write_app_data_u64(&mut wire, 40, self.last_installed_generation);
        write_app_data_u64(&mut wire, 48, self.io.reads());
        write_app_data_u64(&mut wire, 56, self.io.writes());
        write_app_data_u64(&mut wire, 64, self.io.flushes());
        write_app_data_u16(&mut wire, 72, ANDROID_PACKAGE_UNINSTALL_DATA_KEEP_NONE);
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, AndroidPackageUninstallResultError> {
        if wire.len() != ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE {
            return Err(AndroidPackageUninstallResultError::InvalidWireLength);
        }
        if wire[..8] != ANDROID_PACKAGE_UNINSTALL_RESULT_MAGIC {
            return Err(AndroidPackageUninstallResultError::InvalidMagic);
        }
        if read_app_data_u32(wire, 8) != ANDROID_PACKAGE_UNINSTALL_WIRE_VERSION {
            return Err(AndroidPackageUninstallResultError::UnsupportedVersion);
        }
        if read_app_data_u32(wire, 12) != ANDROID_PACKAGE_UNINSTALL_FLAGS_NONE {
            return Err(AndroidPackageUninstallResultError::NonZeroFlags);
        }
        if read_app_data_u16(wire, 72) != ANDROID_PACKAGE_UNINSTALL_DATA_KEEP_NONE {
            return Err(AndroidPackageUninstallResultError::UnsupportedDataDisposition);
        }
        if wire[74..].iter().any(|byte| *byte != 0) {
            return Err(AndroidPackageUninstallResultError::NonZeroReserved);
        }
        Self::new(
            read_app_data_u64(wire, 16),
            read_app_data_u64(wire, 24),
            read_app_data_u64(wire, 32),
            read_app_data_u64(wire, 40),
            AndroidPackageIoCounters::new(
                read_app_data_u64(wire, 48),
                read_app_data_u64(wire, 56),
                read_app_data_u64(wire, 64),
            ),
        )
    }
}

/// ABI 53's only user-selectable runtime package mutation.
#[cfg(feature = "androidbox-runtime-install2")]
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageInstallAction {
    Install = 1,
    Update = 2,
    Reinstall = 3,
}

#[cfg(feature = "androidbox-runtime-install2")]
impl AndroidPackageInstallAction {
    pub const fn raw(self) -> u16 {
        self as u16
    }

    pub const fn from_raw(raw: u16) -> Option<Self> {
        match raw {
            1 => Some(Self::Install),
            2 => Some(Self::Update),
            3 => Some(Self::Reinstall),
            _ => None,
        }
    }
}

/// Borrowed inputs used to construct one authority-free install candidate.
#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageInstallCandidateMetadata<'a> {
    pub candidate_id: u64,
    pub action: AndroidPackageInstallAction,
    pub expected_generation: u64,
    pub expected_version_code: u64,
    pub version_code: u64,
    pub apk_length: u32,
    pub resources_table_crc32: u32,
    pub layout_xml_crc32: u32,
    pub layout_resource_id: u32,
    pub text_resource_id: u32,
    pub instruction_count: u16,
    pub apk_sha256: [u8; 32],
    pub signer_sha256: [u8; 32],
    pub package_name: &'a [u8],
    pub activity_name: &'a [u8],
    pub title: &'a [u8],
    pub text: &'a [u8],
}

/// Canonical semantic value returned by ABI 53 syscall 64.
///
/// It contains display metadata and complete immutable package identity, but
/// no pointer, pathname, storage address, handle, or byte-range authority.
#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageInstallCandidate {
    candidate_id: u64,
    expected_generation: u64,
    expected_version_code: u64,
    version_code: u64,
    apk_length: u32,
    resources_table_crc32: u32,
    layout_xml_crc32: u32,
    layout_resource_id: u32,
    text_resource_id: u32,
    instruction_count: u16,
    profile: u16,
    action: u16,
    package_name_length: u16,
    activity_name_length: u16,
    title_length: u16,
    text_length: u16,
    apk_sha256: [u8; 32],
    signer_sha256: [u8; 32],
    package_name: [u8; ANDROID_PACKAGE_NAME_MAX_BYTES],
    activity_name: [u8; ANDROID_PACKAGE_ACTIVITY_MAX_BYTES],
    title: [u8; ANDROID_PACKAGE_TITLE_MAX_BYTES],
    text: [u8; ANDROID_PACKAGE_TEXT_MAX_BYTES],
}

#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageInstallCandidateError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    NonZeroFlags,
    InvalidCandidateId,
    InvalidAction,
    InvalidExpectedState,
    InvalidVersionCode,
    InvalidApkLength,
    InvalidResourcesTableCrc32,
    InvalidLayoutXmlCrc32,
    InvalidLayoutResourceId,
    InvalidTextResourceId,
    InvalidInstructionCount,
    InvalidProfile,
    ZeroApkSha256,
    ZeroSignerSha256,
    InvalidPackageNameLength,
    InvalidActivityNameLength,
    InvalidTitleLength,
    InvalidTextLength,
    NonPrintablePackageName,
    NonPrintableActivityName,
    NonPrintableTitle,
    NonPrintableText,
    NonZeroPackageNamePadding,
    NonZeroActivityNamePadding,
    NonZeroTitlePadding,
    NonZeroTextPadding,
    NonCanonicalEmptyCandidate,
    NonZeroReserved,
}

#[cfg(feature = "androidbox-runtime-install2")]
impl AndroidPackageInstallCandidate {
    pub const fn empty() -> Self {
        Self {
            candidate_id: 0,
            expected_generation: 0,
            expected_version_code: 0,
            version_code: 0,
            apk_length: 0,
            resources_table_crc32: 0,
            layout_xml_crc32: 0,
            layout_resource_id: 0,
            text_resource_id: 0,
            instruction_count: 0,
            profile: 0,
            action: 0,
            package_name_length: 0,
            activity_name_length: 0,
            title_length: 0,
            text_length: 0,
            apk_sha256: [0; 32],
            signer_sha256: [0; 32],
            package_name: [0; ANDROID_PACKAGE_NAME_MAX_BYTES],
            activity_name: [0; ANDROID_PACKAGE_ACTIVITY_MAX_BYTES],
            title: [0; ANDROID_PACKAGE_TITLE_MAX_BYTES],
            text: [0; ANDROID_PACKAGE_TEXT_MAX_BYTES],
        }
    }

    pub fn new(
        metadata: AndroidPackageInstallCandidateMetadata<'_>,
    ) -> Result<Self, AndroidPackageInstallCandidateError> {
        validate_android_package_install_candidate_metadata(&metadata)?;
        let mut package_name = [0; ANDROID_PACKAGE_NAME_MAX_BYTES];
        package_name[..metadata.package_name.len()].copy_from_slice(metadata.package_name);
        let mut activity_name = [0; ANDROID_PACKAGE_ACTIVITY_MAX_BYTES];
        activity_name[..metadata.activity_name.len()].copy_from_slice(metadata.activity_name);
        let mut title = [0; ANDROID_PACKAGE_TITLE_MAX_BYTES];
        title[..metadata.title.len()].copy_from_slice(metadata.title);
        let mut text = [0; ANDROID_PACKAGE_TEXT_MAX_BYTES];
        text[..metadata.text.len()].copy_from_slice(metadata.text);
        Ok(Self {
            candidate_id: metadata.candidate_id,
            expected_generation: metadata.expected_generation,
            expected_version_code: metadata.expected_version_code,
            version_code: metadata.version_code,
            apk_length: metadata.apk_length,
            resources_table_crc32: metadata.resources_table_crc32,
            layout_xml_crc32: metadata.layout_xml_crc32,
            layout_resource_id: metadata.layout_resource_id,
            text_resource_id: metadata.text_resource_id,
            instruction_count: metadata.instruction_count,
            profile: AndroidPackageCompatibilityProfile::Resources1.raw(),
            action: metadata.action.raw(),
            package_name_length: metadata.package_name.len() as u16,
            activity_name_length: metadata.activity_name.len() as u16,
            title_length: metadata.title.len() as u16,
            text_length: metadata.text.len() as u16,
            apk_sha256: metadata.apk_sha256,
            signer_sha256: metadata.signer_sha256,
            package_name,
            activity_name,
            title,
            text,
        })
    }

    pub const fn present(&self) -> bool {
        self.candidate_id != 0
    }

    pub const fn candidate_id(&self) -> u64 {
        self.candidate_id
    }

    pub const fn action(&self) -> Option<AndroidPackageInstallAction> {
        AndroidPackageInstallAction::from_raw(self.action)
    }

    pub const fn expected_generation(&self) -> u64 {
        self.expected_generation
    }

    pub const fn expected_version_code(&self) -> u64 {
        self.expected_version_code
    }

    pub const fn version_code(&self) -> u64 {
        self.version_code
    }

    pub const fn apk_length(&self) -> u32 {
        self.apk_length
    }

    pub const fn resources_table_crc32(&self) -> u32 {
        self.resources_table_crc32
    }

    pub const fn layout_xml_crc32(&self) -> u32 {
        self.layout_xml_crc32
    }

    pub const fn layout_resource_id(&self) -> u32 {
        self.layout_resource_id
    }

    pub const fn text_resource_id(&self) -> u32 {
        self.text_resource_id
    }

    pub const fn instruction_count(&self) -> u16 {
        self.instruction_count
    }

    pub const fn profile(&self) -> Option<AndroidPackageCompatibilityProfile> {
        AndroidPackageCompatibilityProfile::from_raw(self.profile)
    }

    pub const fn apk_sha256(&self) -> &[u8; 32] {
        &self.apk_sha256
    }

    pub const fn signer_sha256(&self) -> &[u8; 32] {
        &self.signer_sha256
    }

    pub fn package_name(&self) -> &str {
        core::str::from_utf8(self.package_name_bytes()).expect("validated candidate package")
    }

    pub fn package_name_bytes(&self) -> &[u8] {
        &self.package_name[..self.package_name_length as usize]
    }

    pub fn activity_name(&self) -> &str {
        core::str::from_utf8(self.activity_name_bytes()).expect("validated candidate activity")
    }

    pub fn activity_name_bytes(&self) -> &[u8] {
        &self.activity_name[..self.activity_name_length as usize]
    }

    pub fn title(&self) -> &str {
        core::str::from_utf8(self.title_bytes()).expect("validated candidate title")
    }

    pub fn title_bytes(&self) -> &[u8] {
        &self.title[..self.title_length as usize]
    }

    pub fn text(&self) -> &str {
        core::str::from_utf8(self.text_bytes()).expect("validated candidate text")
    }

    pub fn text_bytes(&self) -> &[u8] {
        &self.text[..self.text_length as usize]
    }

    pub fn encode(&self) -> [u8; ANDROID_PACKAGE_INSTALL_CANDIDATE_WIRE_SIZE] {
        let mut wire = [0; ANDROID_PACKAGE_INSTALL_CANDIDATE_WIRE_SIZE];
        wire[..8].copy_from_slice(&ANDROID_PACKAGE_INSTALL_CANDIDATE_MAGIC);
        write_app_data_u32(&mut wire, 8, ANDROID_PACKAGE_INSTALL_WIRE_VERSION);
        write_app_data_u32(&mut wire, 12, ANDROID_PACKAGE_INSTALL_FLAGS_NONE);
        write_app_data_u64(&mut wire, 16, self.candidate_id);
        write_app_data_u64(&mut wire, 24, self.expected_generation);
        write_app_data_u64(&mut wire, 32, self.expected_version_code);
        write_app_data_u64(&mut wire, 40, self.version_code);
        write_app_data_u32(&mut wire, 48, self.apk_length);
        write_app_data_u32(&mut wire, 52, self.resources_table_crc32);
        write_app_data_u32(&mut wire, 56, self.layout_xml_crc32);
        write_app_data_u32(&mut wire, 60, self.layout_resource_id);
        write_app_data_u32(&mut wire, 64, self.text_resource_id);
        write_app_data_u16(&mut wire, 68, self.instruction_count);
        write_app_data_u16(&mut wire, 70, self.profile);
        write_app_data_u16(&mut wire, 72, self.action);
        write_app_data_u16(&mut wire, 74, self.package_name_length);
        write_app_data_u16(&mut wire, 76, self.activity_name_length);
        write_app_data_u16(&mut wire, 78, self.title_length);
        write_app_data_u16(&mut wire, 80, self.text_length);
        wire[88..120].copy_from_slice(&self.apk_sha256);
        wire[120..152].copy_from_slice(&self.signer_sha256);
        wire[152..248].copy_from_slice(&self.package_name);
        wire[248..376].copy_from_slice(&self.activity_name);
        wire[376..504].copy_from_slice(&self.title);
        wire[504..632].copy_from_slice(&self.text);
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, AndroidPackageInstallCandidateError> {
        if wire.len() != ANDROID_PACKAGE_INSTALL_CANDIDATE_WIRE_SIZE {
            return Err(AndroidPackageInstallCandidateError::InvalidWireLength);
        }
        if wire[..8] != ANDROID_PACKAGE_INSTALL_CANDIDATE_MAGIC {
            return Err(AndroidPackageInstallCandidateError::InvalidMagic);
        }
        if read_app_data_u32(wire, 8) != ANDROID_PACKAGE_INSTALL_WIRE_VERSION {
            return Err(AndroidPackageInstallCandidateError::UnsupportedVersion);
        }
        if read_app_data_u32(wire, 12) != ANDROID_PACKAGE_INSTALL_FLAGS_NONE {
            return Err(AndroidPackageInstallCandidateError::NonZeroFlags);
        }
        if wire[82..88].iter().any(|byte| *byte != 0)
            || wire[632..640].iter().any(|byte| *byte != 0)
        {
            return Err(AndroidPackageInstallCandidateError::NonZeroReserved);
        }
        let candidate_id = read_app_data_u64(wire, 16);
        if candidate_id == 0 {
            if wire[16..632].iter().any(|byte| *byte != 0) {
                return Err(AndroidPackageInstallCandidateError::NonCanonicalEmptyCandidate);
            }
            return Ok(Self::empty());
        }
        let package_name_length = read_app_data_u16(wire, 74) as usize;
        let activity_name_length = read_app_data_u16(wire, 76) as usize;
        let title_length = read_app_data_u16(wire, 78) as usize;
        let text_length = read_app_data_u16(wire, 80) as usize;
        validate_android_package_install_wire_text(
            wire,
            152,
            ANDROID_PACKAGE_NAME_MAX_BYTES,
            package_name_length,
            AndroidPackageInstallCandidateError::InvalidPackageNameLength,
            AndroidPackageInstallCandidateError::NonZeroPackageNamePadding,
        )?;
        validate_android_package_install_wire_text(
            wire,
            248,
            ANDROID_PACKAGE_ACTIVITY_MAX_BYTES,
            activity_name_length,
            AndroidPackageInstallCandidateError::InvalidActivityNameLength,
            AndroidPackageInstallCandidateError::NonZeroActivityNamePadding,
        )?;
        validate_android_package_install_wire_text(
            wire,
            376,
            ANDROID_PACKAGE_TITLE_MAX_BYTES,
            title_length,
            AndroidPackageInstallCandidateError::InvalidTitleLength,
            AndroidPackageInstallCandidateError::NonZeroTitlePadding,
        )?;
        validate_android_package_install_wire_text(
            wire,
            504,
            ANDROID_PACKAGE_TEXT_MAX_BYTES,
            text_length,
            AndroidPackageInstallCandidateError::InvalidTextLength,
            AndroidPackageInstallCandidateError::NonZeroTextPadding,
        )?;
        let action = AndroidPackageInstallAction::from_raw(read_app_data_u16(wire, 72))
            .ok_or(AndroidPackageInstallCandidateError::InvalidAction)?;
        if AndroidPackageCompatibilityProfile::from_raw(read_app_data_u16(wire, 70))
            != Some(AndroidPackageCompatibilityProfile::Resources1)
        {
            return Err(AndroidPackageInstallCandidateError::InvalidProfile);
        }
        let mut apk_sha256 = [0; 32];
        apk_sha256.copy_from_slice(&wire[88..120]);
        let mut signer_sha256 = [0; 32];
        signer_sha256.copy_from_slice(&wire[120..152]);
        Self::new(AndroidPackageInstallCandidateMetadata {
            candidate_id,
            action,
            expected_generation: read_app_data_u64(wire, 24),
            expected_version_code: read_app_data_u64(wire, 32),
            version_code: read_app_data_u64(wire, 40),
            apk_length: read_app_data_u32(wire, 48),
            resources_table_crc32: read_app_data_u32(wire, 52),
            layout_xml_crc32: read_app_data_u32(wire, 56),
            layout_resource_id: read_app_data_u32(wire, 60),
            text_resource_id: read_app_data_u32(wire, 64),
            instruction_count: read_app_data_u16(wire, 68),
            apk_sha256,
            signer_sha256,
            package_name: &wire[152..152 + package_name_length],
            activity_name: &wire[248..248 + activity_name_length],
            title: &wire[376..376 + title_length],
            text: &wire[504..504 + text_length],
        })
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
fn validate_android_package_install_candidate_metadata(
    metadata: &AndroidPackageInstallCandidateMetadata<'_>,
) -> Result<(), AndroidPackageInstallCandidateError> {
    if metadata.candidate_id == 0 {
        return Err(AndroidPackageInstallCandidateError::InvalidCandidateId);
    }
    validate_android_package_install_expected_state(
        metadata.action,
        metadata.expected_generation,
        metadata.expected_version_code,
        metadata.version_code,
    )?;
    if metadata.version_code == 0 || metadata.version_code > u64::from(u32::MAX) {
        return Err(AndroidPackageInstallCandidateError::InvalidVersionCode);
    }
    if metadata.apk_length == 0 || metadata.apk_length > ANDROID_PACKAGE_APK_MAX_BYTES {
        return Err(AndroidPackageInstallCandidateError::InvalidApkLength);
    }
    if metadata.resources_table_crc32 == 0 {
        return Err(AndroidPackageInstallCandidateError::InvalidResourcesTableCrc32);
    }
    if metadata.layout_xml_crc32 == 0 {
        return Err(AndroidPackageInstallCandidateError::InvalidLayoutXmlCrc32);
    }
    if metadata.layout_resource_id >> 24 != 0x7f {
        return Err(AndroidPackageInstallCandidateError::InvalidLayoutResourceId);
    }
    if metadata.text_resource_id >> 24 != 0x7f
        || metadata.text_resource_id == metadata.layout_resource_id
    {
        return Err(AndroidPackageInstallCandidateError::InvalidTextResourceId);
    }
    if metadata.instruction_count == 0 {
        return Err(AndroidPackageInstallCandidateError::InvalidInstructionCount);
    }
    if metadata.apk_sha256.iter().all(|byte| *byte == 0) {
        return Err(AndroidPackageInstallCandidateError::ZeroApkSha256);
    }
    if metadata.signer_sha256.iter().all(|byte| *byte == 0) {
        return Err(AndroidPackageInstallCandidateError::ZeroSignerSha256);
    }
    validate_android_package_install_text(
        metadata.package_name,
        ANDROID_PACKAGE_NAME_MAX_BYTES,
        AndroidPackageInstallCandidateError::InvalidPackageNameLength,
        AndroidPackageInstallCandidateError::NonPrintablePackageName,
    )?;
    validate_android_package_install_text(
        metadata.activity_name,
        ANDROID_PACKAGE_ACTIVITY_MAX_BYTES,
        AndroidPackageInstallCandidateError::InvalidActivityNameLength,
        AndroidPackageInstallCandidateError::NonPrintableActivityName,
    )?;
    validate_android_package_install_text(
        metadata.title,
        ANDROID_PACKAGE_TITLE_MAX_BYTES,
        AndroidPackageInstallCandidateError::InvalidTitleLength,
        AndroidPackageInstallCandidateError::NonPrintableTitle,
    )?;
    validate_android_package_install_text(
        metadata.text,
        ANDROID_PACKAGE_TEXT_MAX_BYTES,
        AndroidPackageInstallCandidateError::InvalidTextLength,
        AndroidPackageInstallCandidateError::NonPrintableText,
    )
}

#[cfg(feature = "androidbox-runtime-install2")]
fn validate_android_package_install_expected_state(
    action: AndroidPackageInstallAction,
    expected_generation: u64,
    expected_version_code: u64,
    version_code: u64,
) -> Result<(), AndroidPackageInstallCandidateError> {
    let valid = match action {
        AndroidPackageInstallAction::Install => {
            expected_generation == 0 && expected_version_code == 0
        }
        AndroidPackageInstallAction::Update => {
            expected_generation != 0
                && expected_version_code != 0
                && version_code > expected_version_code
        }
        AndroidPackageInstallAction::Reinstall => {
            expected_generation != 0
                && expected_version_code != 0
                && version_code >= expected_version_code
        }
    };
    if valid {
        Ok(())
    } else {
        Err(AndroidPackageInstallCandidateError::InvalidExpectedState)
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
fn validate_android_package_install_text(
    text: &[u8],
    maximum_length: usize,
    length_error: AndroidPackageInstallCandidateError,
    printable_error: AndroidPackageInstallCandidateError,
) -> Result<(), AndroidPackageInstallCandidateError> {
    if text.is_empty() || text.len() > maximum_length {
        return Err(length_error);
    }
    if text.iter().any(|byte| !(0x20..=0x7e).contains(byte)) {
        return Err(printable_error);
    }
    Ok(())
}

#[cfg(feature = "androidbox-runtime-install2")]
fn validate_android_package_install_wire_text(
    wire: &[u8],
    offset: usize,
    maximum_length: usize,
    length: usize,
    length_error: AndroidPackageInstallCandidateError,
    padding_error: AndroidPackageInstallCandidateError,
) -> Result<(), AndroidPackageInstallCandidateError> {
    if length == 0 || length > maximum_length {
        return Err(length_error);
    }
    if wire[offset + length..offset + maximum_length]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(padding_error);
    }
    Ok(())
}

/// Canonical fixed-size request submitted by the built-in Settings process.
#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageInstallRequest {
    request_sequence: u64,
    operation_id: u64,
    candidate_id: u64,
    expected_generation: u64,
    expected_version_code: u64,
    version_code: u64,
    apk_length: u32,
    action: u16,
    profile: u16,
    package_name_length: u16,
    apk_sha256: [u8; 32],
    signer_sha256: [u8; 32],
    package_name: [u8; ANDROID_PACKAGE_NAME_MAX_BYTES],
}

#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageInstallRequestError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    NonZeroFlags,
    InvalidRequestSequence,
    InvalidOperationId,
    InvalidCandidateId,
    InvalidExpectedState,
    InvalidVersionCode,
    InvalidApkLength,
    InvalidAction,
    InvalidProfile,
    ZeroApkSha256,
    ZeroSignerSha256,
    InvalidPackageNameLength,
    NonPrintablePackageName,
    NonZeroPackageNamePadding,
    NonZeroReserved,
}

#[cfg(feature = "androidbox-runtime-install2")]
impl AndroidPackageInstallRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_sequence: u64,
        operation_id: u64,
        candidate_id: u64,
        action: AndroidPackageInstallAction,
        expected_generation: u64,
        expected_version_code: u64,
        version_code: u64,
        apk_length: u32,
        apk_sha256: [u8; 32],
        signer_sha256: [u8; 32],
        package_name: &[u8],
    ) -> Result<Self, AndroidPackageInstallRequestError> {
        if request_sequence == 0 {
            return Err(AndroidPackageInstallRequestError::InvalidRequestSequence);
        }
        if operation_id == 0 {
            return Err(AndroidPackageInstallRequestError::InvalidOperationId);
        }
        if candidate_id == 0 {
            return Err(AndroidPackageInstallRequestError::InvalidCandidateId);
        }
        validate_android_package_install_request_state(
            action,
            expected_generation,
            expected_version_code,
            version_code,
        )?;
        if version_code == 0 || version_code > u64::from(u32::MAX) {
            return Err(AndroidPackageInstallRequestError::InvalidVersionCode);
        }
        if apk_length == 0 || apk_length > ANDROID_PACKAGE_APK_MAX_BYTES {
            return Err(AndroidPackageInstallRequestError::InvalidApkLength);
        }
        if apk_sha256.iter().all(|byte| *byte == 0) {
            return Err(AndroidPackageInstallRequestError::ZeroApkSha256);
        }
        if signer_sha256.iter().all(|byte| *byte == 0) {
            return Err(AndroidPackageInstallRequestError::ZeroSignerSha256);
        }
        if package_name.is_empty() || package_name.len() > ANDROID_PACKAGE_NAME_MAX_BYTES {
            return Err(AndroidPackageInstallRequestError::InvalidPackageNameLength);
        }
        if package_name
            .iter()
            .any(|byte| !(0x20..=0x7e).contains(byte))
        {
            return Err(AndroidPackageInstallRequestError::NonPrintablePackageName);
        }
        let mut canonical_package_name = [0; ANDROID_PACKAGE_NAME_MAX_BYTES];
        canonical_package_name[..package_name.len()].copy_from_slice(package_name);
        Ok(Self {
            request_sequence,
            operation_id,
            candidate_id,
            expected_generation,
            expected_version_code,
            version_code,
            apk_length,
            action: action.raw(),
            profile: AndroidPackageCompatibilityProfile::Resources1.raw(),
            package_name_length: package_name.len() as u16,
            apk_sha256,
            signer_sha256,
            package_name: canonical_package_name,
        })
    }

    pub const fn request_sequence(&self) -> u64 {
        self.request_sequence
    }

    pub const fn operation_id(&self) -> u64 {
        self.operation_id
    }

    pub const fn candidate_id(&self) -> u64 {
        self.candidate_id
    }

    pub const fn expected_generation(&self) -> u64 {
        self.expected_generation
    }

    pub const fn expected_version_code(&self) -> u64 {
        self.expected_version_code
    }

    pub const fn version_code(&self) -> u64 {
        self.version_code
    }

    pub const fn apk_length(&self) -> u32 {
        self.apk_length
    }

    pub const fn action(&self) -> Option<AndroidPackageInstallAction> {
        AndroidPackageInstallAction::from_raw(self.action)
    }

    pub const fn profile(&self) -> Option<AndroidPackageCompatibilityProfile> {
        AndroidPackageCompatibilityProfile::from_raw(self.profile)
    }

    pub const fn apk_sha256(&self) -> &[u8; 32] {
        &self.apk_sha256
    }

    pub const fn signer_sha256(&self) -> &[u8; 32] {
        &self.signer_sha256
    }

    pub fn package_name(&self) -> &str {
        core::str::from_utf8(self.package_name_bytes()).expect("validated install package")
    }

    pub fn package_name_bytes(&self) -> &[u8] {
        &self.package_name[..self.package_name_length as usize]
    }

    pub fn encode(&self) -> [u8; ANDROID_PACKAGE_INSTALL_WIRE_SIZE] {
        let mut wire = [0; ANDROID_PACKAGE_INSTALL_WIRE_SIZE];
        wire[..8].copy_from_slice(&ANDROID_PACKAGE_INSTALL_REQUEST_MAGIC);
        write_app_data_u32(&mut wire, 8, ANDROID_PACKAGE_INSTALL_WIRE_VERSION);
        write_app_data_u32(&mut wire, 12, ANDROID_PACKAGE_INSTALL_FLAGS_NONE);
        write_app_data_u64(&mut wire, 16, self.request_sequence);
        write_app_data_u64(&mut wire, 24, self.operation_id);
        write_app_data_u64(&mut wire, 32, self.candidate_id);
        write_app_data_u64(&mut wire, 40, self.expected_generation);
        write_app_data_u64(&mut wire, 48, self.expected_version_code);
        write_app_data_u64(&mut wire, 56, self.version_code);
        write_app_data_u32(&mut wire, 64, self.apk_length);
        write_app_data_u16(&mut wire, 68, self.action);
        write_app_data_u16(&mut wire, 70, self.profile);
        write_app_data_u16(&mut wire, 72, self.package_name_length);
        wire[76..108].copy_from_slice(&self.apk_sha256);
        wire[108..140].copy_from_slice(&self.signer_sha256);
        wire[140..236].copy_from_slice(&self.package_name);
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, AndroidPackageInstallRequestError> {
        if wire.len() != ANDROID_PACKAGE_INSTALL_WIRE_SIZE {
            return Err(AndroidPackageInstallRequestError::InvalidWireLength);
        }
        if wire[..8] != ANDROID_PACKAGE_INSTALL_REQUEST_MAGIC {
            return Err(AndroidPackageInstallRequestError::InvalidMagic);
        }
        if read_app_data_u32(wire, 8) != ANDROID_PACKAGE_INSTALL_WIRE_VERSION {
            return Err(AndroidPackageInstallRequestError::UnsupportedVersion);
        }
        if read_app_data_u32(wire, 12) != ANDROID_PACKAGE_INSTALL_FLAGS_NONE {
            return Err(AndroidPackageInstallRequestError::NonZeroFlags);
        }
        if wire[74..76].iter().any(|byte| *byte != 0)
            || wire[236..256].iter().any(|byte| *byte != 0)
        {
            return Err(AndroidPackageInstallRequestError::NonZeroReserved);
        }
        let action = AndroidPackageInstallAction::from_raw(read_app_data_u16(wire, 68))
            .ok_or(AndroidPackageInstallRequestError::InvalidAction)?;
        if AndroidPackageCompatibilityProfile::from_raw(read_app_data_u16(wire, 70))
            != Some(AndroidPackageCompatibilityProfile::Resources1)
        {
            return Err(AndroidPackageInstallRequestError::InvalidProfile);
        }
        let package_name_length = read_app_data_u16(wire, 72) as usize;
        if package_name_length == 0 || package_name_length > ANDROID_PACKAGE_NAME_MAX_BYTES {
            return Err(AndroidPackageInstallRequestError::InvalidPackageNameLength);
        }
        if wire[140..140 + package_name_length]
            .iter()
            .any(|byte| !(0x20..=0x7e).contains(byte))
        {
            return Err(AndroidPackageInstallRequestError::NonPrintablePackageName);
        }
        if wire[140 + package_name_length..236]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(AndroidPackageInstallRequestError::NonZeroPackageNamePadding);
        }
        let mut apk_sha256 = [0; 32];
        apk_sha256.copy_from_slice(&wire[76..108]);
        let mut signer_sha256 = [0; 32];
        signer_sha256.copy_from_slice(&wire[108..140]);
        Self::new(
            read_app_data_u64(wire, 16),
            read_app_data_u64(wire, 24),
            read_app_data_u64(wire, 32),
            action,
            read_app_data_u64(wire, 40),
            read_app_data_u64(wire, 48),
            read_app_data_u64(wire, 56),
            read_app_data_u32(wire, 64),
            apk_sha256,
            signer_sha256,
            &wire[140..140 + package_name_length],
        )
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
fn validate_android_package_install_request_state(
    action: AndroidPackageInstallAction,
    expected_generation: u64,
    expected_version_code: u64,
    version_code: u64,
) -> Result<(), AndroidPackageInstallRequestError> {
    let valid = match action {
        AndroidPackageInstallAction::Install => {
            expected_generation == 0 && expected_version_code == 0
        }
        AndroidPackageInstallAction::Update => {
            expected_generation != 0
                && expected_version_code != 0
                && version_code > expected_version_code
        }
        AndroidPackageInstallAction::Reinstall => {
            expected_generation != 0
                && expected_version_code != 0
                && version_code >= expected_version_code
        }
    };
    if valid {
        Ok(())
    } else {
        Err(AndroidPackageInstallRequestError::InvalidExpectedState)
    }
}

/// Canonical completion written over a successful runtime-install request.
#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageInstallResult {
    request_sequence: u64,
    operation_id: u64,
    candidate_id: u64,
    previous_generation: u64,
    installed_generation: u64,
    installed_version_code: u64,
    apk_length: u32,
    action: u16,
    profile: u16,
    io: AndroidPackageIoCounters,
    apk_sha256: [u8; 32],
    signer_sha256: [u8; 32],
}

#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageInstallResultError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    NonZeroFlags,
    InvalidRequestSequence,
    InvalidOperationId,
    InvalidCandidateId,
    InvalidPreviousGeneration,
    InvalidInstalledGeneration,
    InvalidGenerationTransition,
    InvalidVersionCode,
    InvalidApkLength,
    InvalidAction,
    InvalidProfile,
    InvalidIoEvidence,
    ZeroApkSha256,
    ZeroSignerSha256,
    NonZeroReserved,
}

#[cfg(feature = "androidbox-runtime-install2")]
impl AndroidPackageInstallResult {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_sequence: u64,
        operation_id: u64,
        candidate_id: u64,
        previous_generation: u64,
        installed_generation: u64,
        installed_version_code: u64,
        apk_length: u32,
        action: AndroidPackageInstallAction,
        io: AndroidPackageIoCounters,
        apk_sha256: [u8; 32],
        signer_sha256: [u8; 32],
    ) -> Result<Self, AndroidPackageInstallResultError> {
        if request_sequence == 0 {
            return Err(AndroidPackageInstallResultError::InvalidRequestSequence);
        }
        if operation_id == 0 {
            return Err(AndroidPackageInstallResultError::InvalidOperationId);
        }
        if candidate_id == 0 {
            return Err(AndroidPackageInstallResultError::InvalidCandidateId);
        }
        if matches!(action, AndroidPackageInstallAction::Install) && previous_generation != 0 {
            return Err(AndroidPackageInstallResultError::InvalidPreviousGeneration);
        }
        if !matches!(action, AndroidPackageInstallAction::Install) && previous_generation == 0 {
            return Err(AndroidPackageInstallResultError::InvalidPreviousGeneration);
        }
        if installed_generation == 0 {
            return Err(AndroidPackageInstallResultError::InvalidInstalledGeneration);
        }
        if previous_generation.checked_add(1) != Some(installed_generation) {
            return Err(AndroidPackageInstallResultError::InvalidGenerationTransition);
        }
        if installed_version_code == 0 || installed_version_code > u64::from(u32::MAX) {
            return Err(AndroidPackageInstallResultError::InvalidVersionCode);
        }
        if apk_length == 0 || apk_length > ANDROID_PACKAGE_APK_MAX_BYTES {
            return Err(AndroidPackageInstallResultError::InvalidApkLength);
        }
        if io.reads == 0 || io.writes == 0 || io.flushes == 0 {
            return Err(AndroidPackageInstallResultError::InvalidIoEvidence);
        }
        if apk_sha256.iter().all(|byte| *byte == 0) {
            return Err(AndroidPackageInstallResultError::ZeroApkSha256);
        }
        if signer_sha256.iter().all(|byte| *byte == 0) {
            return Err(AndroidPackageInstallResultError::ZeroSignerSha256);
        }
        Ok(Self {
            request_sequence,
            operation_id,
            candidate_id,
            previous_generation,
            installed_generation,
            installed_version_code,
            apk_length,
            action: action.raw(),
            profile: AndroidPackageCompatibilityProfile::Resources1.raw(),
            io,
            apk_sha256,
            signer_sha256,
        })
    }

    pub const fn request_sequence(&self) -> u64 {
        self.request_sequence
    }

    pub const fn operation_id(&self) -> u64 {
        self.operation_id
    }

    pub const fn candidate_id(&self) -> u64 {
        self.candidate_id
    }

    pub const fn previous_generation(&self) -> u64 {
        self.previous_generation
    }

    pub const fn installed_generation(&self) -> u64 {
        self.installed_generation
    }

    pub const fn installed_version_code(&self) -> u64 {
        self.installed_version_code
    }

    pub const fn apk_length(&self) -> u32 {
        self.apk_length
    }

    pub const fn action(&self) -> Option<AndroidPackageInstallAction> {
        AndroidPackageInstallAction::from_raw(self.action)
    }

    pub const fn io_counters(&self) -> AndroidPackageIoCounters {
        self.io
    }

    pub const fn apk_sha256(&self) -> &[u8; 32] {
        &self.apk_sha256
    }

    pub const fn signer_sha256(&self) -> &[u8; 32] {
        &self.signer_sha256
    }

    pub fn encode(self) -> [u8; ANDROID_PACKAGE_INSTALL_WIRE_SIZE] {
        let mut wire = [0; ANDROID_PACKAGE_INSTALL_WIRE_SIZE];
        wire[..8].copy_from_slice(&ANDROID_PACKAGE_INSTALL_RESULT_MAGIC);
        write_app_data_u32(&mut wire, 8, ANDROID_PACKAGE_INSTALL_WIRE_VERSION);
        write_app_data_u32(&mut wire, 12, ANDROID_PACKAGE_INSTALL_FLAGS_NONE);
        write_app_data_u64(&mut wire, 16, self.request_sequence);
        write_app_data_u64(&mut wire, 24, self.operation_id);
        write_app_data_u64(&mut wire, 32, self.candidate_id);
        write_app_data_u64(&mut wire, 40, self.previous_generation);
        write_app_data_u64(&mut wire, 48, self.installed_generation);
        write_app_data_u64(&mut wire, 56, self.installed_version_code);
        write_app_data_u32(&mut wire, 64, self.apk_length);
        write_app_data_u16(&mut wire, 68, self.action);
        write_app_data_u16(&mut wire, 70, self.profile);
        write_app_data_u64(&mut wire, 72, self.io.reads);
        write_app_data_u64(&mut wire, 80, self.io.writes);
        write_app_data_u64(&mut wire, 88, self.io.flushes);
        wire[96..128].copy_from_slice(&self.apk_sha256);
        wire[128..160].copy_from_slice(&self.signer_sha256);
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, AndroidPackageInstallResultError> {
        if wire.len() != ANDROID_PACKAGE_INSTALL_WIRE_SIZE {
            return Err(AndroidPackageInstallResultError::InvalidWireLength);
        }
        if wire[..8] != ANDROID_PACKAGE_INSTALL_RESULT_MAGIC {
            return Err(AndroidPackageInstallResultError::InvalidMagic);
        }
        if read_app_data_u32(wire, 8) != ANDROID_PACKAGE_INSTALL_WIRE_VERSION {
            return Err(AndroidPackageInstallResultError::UnsupportedVersion);
        }
        if read_app_data_u32(wire, 12) != ANDROID_PACKAGE_INSTALL_FLAGS_NONE {
            return Err(AndroidPackageInstallResultError::NonZeroFlags);
        }
        if wire[160..256].iter().any(|byte| *byte != 0) {
            return Err(AndroidPackageInstallResultError::NonZeroReserved);
        }
        let action = AndroidPackageInstallAction::from_raw(read_app_data_u16(wire, 68))
            .ok_or(AndroidPackageInstallResultError::InvalidAction)?;
        if AndroidPackageCompatibilityProfile::from_raw(read_app_data_u16(wire, 70))
            != Some(AndroidPackageCompatibilityProfile::Resources1)
        {
            return Err(AndroidPackageInstallResultError::InvalidProfile);
        }
        let mut apk_sha256 = [0; 32];
        apk_sha256.copy_from_slice(&wire[96..128]);
        let mut signer_sha256 = [0; 32];
        signer_sha256.copy_from_slice(&wire[128..160]);
        Self::new(
            read_app_data_u64(wire, 16),
            read_app_data_u64(wire, 24),
            read_app_data_u64(wire, 32),
            read_app_data_u64(wire, 40),
            read_app_data_u64(wire, 48),
            read_app_data_u64(wire, 56),
            read_app_data_u32(wire, 64),
            action,
            AndroidPackageIoCounters::new(
                read_app_data_u64(wire, 72),
                read_app_data_u64(wire, 80),
                read_app_data_u64(wire, 88),
            ),
            apk_sha256,
            signer_sha256,
        )
    }
}

/// Borrowed, allocation-free inputs for one installed Resources-1 package.
#[cfg(feature = "androidbox-apk-install0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageInstalledMetadata<'a> {
    pub generation: u64,
    pub version_code: u64,
    pub apk_length: u32,
    pub resources_table_crc32: u32,
    pub layout_xml_crc32: u32,
    pub layout_resource_id: u32,
    pub text_resource_id: u32,
    pub instruction_count: u16,
    pub apk_sha256: [u8; 32],
    pub signer_sha256: [u8; 32],
    pub package_name: &'a [u8],
    pub activity_name: &'a [u8],
    pub title: &'a [u8],
    pub text: &'a [u8],
}

/// Canonical semantic value returned by syscall 59.
///
/// This value is inline and `Copy`: it contains no handle, pointer, allocation,
/// or authority. Encoding always produces exactly
/// [`ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE`] bytes.
#[cfg(feature = "androidbox-apk-install0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageSnapshot {
    flags: u32,
    generation: u64,
    version_code: u64,
    apk_length: u32,
    resources_table_crc32: u32,
    layout_xml_crc32: u32,
    layout_resource_id: u32,
    text_resource_id: u32,
    instruction_count: u16,
    profile: u16,
    package_name_length: u16,
    activity_name_length: u16,
    title_length: u16,
    text_length: u16,
    io: AndroidPackageIoCounters,
    apk_sha256: [u8; 32],
    signer_sha256: [u8; 32],
    package_name: [u8; ANDROID_PACKAGE_NAME_MAX_BYTES],
    activity_name: [u8; ANDROID_PACKAGE_ACTIVITY_MAX_BYTES],
    title: [u8; ANDROID_PACKAGE_TITLE_MAX_BYTES],
    text: [u8; ANDROID_PACKAGE_TEXT_MAX_BYTES],
}

#[cfg(feature = "androidbox-apk-install0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageSnapshotError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    UnknownFlags,
    InconsistentSourceState,
    InvalidFormatPerformedState,
    SourceUsedWithoutInstalledPackage,
    InstalledVolumeNotFormatted,
    InvalidGeneration,
    InvalidVersionCode,
    InvalidApkLength,
    InvalidResourcesTableCrc32,
    InvalidLayoutXmlCrc32,
    InvalidLayoutResourceId,
    InvalidTextResourceId,
    InvalidInstructionCount,
    InvalidProfile,
    ZeroApkSha256,
    ZeroSignerSha256,
    InvalidPackageNameLength,
    InvalidActivityNameLength,
    InvalidTitleLength,
    InvalidTextLength,
    NonPrintablePackageName,
    NonPrintableActivityName,
    NonPrintableTitle,
    NonPrintableText,
    NonZeroPackageNamePadding,
    NonZeroActivityNamePadding,
    NonZeroTitlePadding,
    NonZeroTextPadding,
    NonCanonicalEmptyPackage,
    NonZeroReserved,
}

#[cfg(feature = "androidbox-apk-install0")]
impl AndroidPackageSnapshot {
    /// Constructs a canonical snapshot with no installed package.
    pub fn empty(
        state: AndroidPackageSnapshotState,
        io: AndroidPackageIoCounters,
    ) -> Result<Self, AndroidPackageSnapshotError> {
        let flags = validate_android_package_snapshot_state(state, false)?;
        Ok(Self {
            flags,
            generation: 0,
            version_code: 0,
            apk_length: 0,
            resources_table_crc32: 0,
            layout_xml_crc32: 0,
            layout_resource_id: 0,
            text_resource_id: 0,
            instruction_count: 0,
            profile: 0,
            package_name_length: 0,
            activity_name_length: 0,
            title_length: 0,
            text_length: 0,
            io,
            apk_sha256: [0; 32],
            signer_sha256: [0; 32],
            package_name: [0; ANDROID_PACKAGE_NAME_MAX_BYTES],
            activity_name: [0; ANDROID_PACKAGE_ACTIVITY_MAX_BYTES],
            title: [0; ANDROID_PACKAGE_TITLE_MAX_BYTES],
            text: [0; ANDROID_PACKAGE_TEXT_MAX_BYTES],
        })
    }

    /// Constructs a validated installed Resources-1 package snapshot.
    pub fn installed(
        state: AndroidPackageSnapshotState,
        io: AndroidPackageIoCounters,
        metadata: AndroidPackageInstalledMetadata<'_>,
    ) -> Result<Self, AndroidPackageSnapshotError> {
        let flags = validate_android_package_snapshot_state(state, true)?;
        validate_android_package_metadata(&metadata)?;

        let mut package_name = [0; ANDROID_PACKAGE_NAME_MAX_BYTES];
        package_name[..metadata.package_name.len()].copy_from_slice(metadata.package_name);
        let mut activity_name = [0; ANDROID_PACKAGE_ACTIVITY_MAX_BYTES];
        activity_name[..metadata.activity_name.len()].copy_from_slice(metadata.activity_name);
        let mut title = [0; ANDROID_PACKAGE_TITLE_MAX_BYTES];
        title[..metadata.title.len()].copy_from_slice(metadata.title);
        let mut text = [0; ANDROID_PACKAGE_TEXT_MAX_BYTES];
        text[..metadata.text.len()].copy_from_slice(metadata.text);

        Ok(Self {
            flags,
            generation: metadata.generation,
            version_code: metadata.version_code,
            apk_length: metadata.apk_length,
            resources_table_crc32: metadata.resources_table_crc32,
            layout_xml_crc32: metadata.layout_xml_crc32,
            layout_resource_id: metadata.layout_resource_id,
            text_resource_id: metadata.text_resource_id,
            instruction_count: metadata.instruction_count,
            profile: AndroidPackageCompatibilityProfile::Resources1.raw(),
            package_name_length: metadata.package_name.len() as u16,
            activity_name_length: metadata.activity_name.len() as u16,
            title_length: metadata.title.len() as u16,
            text_length: metadata.text.len() as u16,
            io,
            apk_sha256: metadata.apk_sha256,
            signer_sha256: metadata.signer_sha256,
            package_name,
            activity_name,
            title,
            text,
        })
    }

    pub const fn flags(&self) -> u32 {
        self.flags
    }

    pub const fn formatted(&self) -> bool {
        self.flags & ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMATTED != 0
    }

    pub const fn installed_package(&self) -> bool {
        self.flags & ANDROID_PACKAGE_SNAPSHOT_FLAG_INSTALLED != 0
    }

    pub const fn source_present(&self) -> bool {
        self.flags & ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_PRESENT != 0
    }

    pub const fn source_used(&self) -> bool {
        self.flags & ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_USED != 0
    }

    pub const fn format_performed(&self) -> bool {
        self.flags & ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMAT_PERFORMED != 0
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub const fn version_code(&self) -> u64 {
        self.version_code
    }

    pub const fn apk_length(&self) -> u32 {
        self.apk_length
    }

    pub const fn resources_table_crc32(&self) -> u32 {
        self.resources_table_crc32
    }

    pub const fn layout_xml_crc32(&self) -> u32 {
        self.layout_xml_crc32
    }

    pub const fn layout_resource_id(&self) -> u32 {
        self.layout_resource_id
    }

    pub const fn text_resource_id(&self) -> u32 {
        self.text_resource_id
    }

    pub const fn instruction_count(&self) -> u16 {
        self.instruction_count
    }

    pub const fn profile(&self) -> Option<AndroidPackageCompatibilityProfile> {
        AndroidPackageCompatibilityProfile::from_raw(self.profile)
    }

    pub const fn io_counters(&self) -> AndroidPackageIoCounters {
        self.io
    }

    pub const fn apk_sha256(&self) -> &[u8; 32] {
        &self.apk_sha256
    }

    pub const fn signer_sha256(&self) -> &[u8; 32] {
        &self.signer_sha256
    }

    pub fn package_name(&self) -> &str {
        core::str::from_utf8(self.package_name_bytes()).expect("validated package name")
    }

    pub fn package_name_bytes(&self) -> &[u8] {
        &self.package_name[..self.package_name_length as usize]
    }

    pub fn activity_name(&self) -> &str {
        core::str::from_utf8(self.activity_name_bytes()).expect("validated activity name")
    }

    pub fn activity_name_bytes(&self) -> &[u8] {
        &self.activity_name[..self.activity_name_length as usize]
    }

    pub fn title(&self) -> &str {
        core::str::from_utf8(self.title_bytes()).expect("validated title")
    }

    pub fn title_bytes(&self) -> &[u8] {
        &self.title[..self.title_length as usize]
    }

    pub fn text(&self) -> &str {
        core::str::from_utf8(self.text_bytes()).expect("validated text")
    }

    pub fn text_bytes(&self) -> &[u8] {
        &self.text[..self.text_length as usize]
    }

    /// Encodes the canonical 640-byte little-endian `BNDAPS01` wire.
    pub fn encode(&self) -> [u8; ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE] {
        let mut wire = [0; ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE];
        wire[0..8].copy_from_slice(&ANDROID_PACKAGE_SNAPSHOT_MAGIC);
        write_app_data_u32(&mut wire, 8, ANDROID_PACKAGE_SNAPSHOT_VERSION);
        write_app_data_u32(&mut wire, 12, self.flags);
        write_app_data_u64(&mut wire, 16, self.generation);
        write_app_data_u64(&mut wire, 24, self.version_code);
        write_app_data_u32(&mut wire, 32, self.apk_length);
        write_app_data_u32(&mut wire, 36, self.resources_table_crc32);
        write_app_data_u32(&mut wire, 40, self.layout_xml_crc32);
        write_app_data_u32(&mut wire, 44, self.layout_resource_id);
        write_app_data_u32(&mut wire, 48, self.text_resource_id);
        write_app_data_u16(&mut wire, 52, self.instruction_count);
        write_app_data_u16(&mut wire, 54, self.profile);
        write_app_data_u16(&mut wire, 56, self.package_name_length);
        write_app_data_u16(&mut wire, 58, self.activity_name_length);
        write_app_data_u16(&mut wire, 60, self.title_length);
        write_app_data_u16(&mut wire, 62, self.text_length);
        write_app_data_u64(&mut wire, 64, self.io.reads);
        write_app_data_u64(&mut wire, 72, self.io.writes);
        write_app_data_u64(&mut wire, 80, self.io.flushes);
        wire[88..120].copy_from_slice(&self.apk_sha256);
        wire[120..152].copy_from_slice(&self.signer_sha256);
        wire[152..248].copy_from_slice(&self.package_name);
        wire[248..376].copy_from_slice(&self.activity_name);
        wire[376..504].copy_from_slice(&self.title);
        wire[504..632].copy_from_slice(&self.text);
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, AndroidPackageSnapshotError> {
        if wire.len() != ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE {
            return Err(AndroidPackageSnapshotError::InvalidWireLength);
        }
        if wire[0..8] != ANDROID_PACKAGE_SNAPSHOT_MAGIC {
            return Err(AndroidPackageSnapshotError::InvalidMagic);
        }
        if read_app_data_u32(wire, 8) != ANDROID_PACKAGE_SNAPSHOT_VERSION {
            return Err(AndroidPackageSnapshotError::UnsupportedVersion);
        }
        if wire[632..640].iter().any(|byte| *byte != 0) {
            return Err(AndroidPackageSnapshotError::NonZeroReserved);
        }

        let flags = read_app_data_u32(wire, 12);
        if flags & !ANDROID_PACKAGE_SNAPSHOT_KNOWN_FLAGS != 0 {
            return Err(AndroidPackageSnapshotError::UnknownFlags);
        }
        let installed = flags & ANDROID_PACKAGE_SNAPSHOT_FLAG_INSTALLED != 0;
        let state = AndroidPackageSnapshotState::new(
            flags & ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMATTED != 0,
            flags & ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_PRESENT != 0,
            flags & ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_USED != 0,
            flags & ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMAT_PERFORMED != 0,
        );
        validate_android_package_snapshot_state(state, installed)?;

        let package_name_length = read_app_data_u16(wire, 56) as usize;
        let activity_name_length = read_app_data_u16(wire, 58) as usize;
        let title_length = read_app_data_u16(wire, 60) as usize;
        let text_length = read_app_data_u16(wire, 62) as usize;
        validate_android_package_wire_text(
            wire,
            152,
            ANDROID_PACKAGE_NAME_MAX_BYTES,
            package_name_length,
            AndroidPackageSnapshotError::InvalidPackageNameLength,
            AndroidPackageSnapshotError::NonZeroPackageNamePadding,
        )?;
        validate_android_package_wire_text(
            wire,
            248,
            ANDROID_PACKAGE_ACTIVITY_MAX_BYTES,
            activity_name_length,
            AndroidPackageSnapshotError::InvalidActivityNameLength,
            AndroidPackageSnapshotError::NonZeroActivityNamePadding,
        )?;
        validate_android_package_wire_text(
            wire,
            376,
            ANDROID_PACKAGE_TITLE_MAX_BYTES,
            title_length,
            AndroidPackageSnapshotError::InvalidTitleLength,
            AndroidPackageSnapshotError::NonZeroTitlePadding,
        )?;
        validate_android_package_wire_text(
            wire,
            504,
            ANDROID_PACKAGE_TEXT_MAX_BYTES,
            text_length,
            AndroidPackageSnapshotError::InvalidTextLength,
            AndroidPackageSnapshotError::NonZeroTextPadding,
        )?;

        let io = AndroidPackageIoCounters::new(
            read_app_data_u64(wire, 64),
            read_app_data_u64(wire, 72),
            read_app_data_u64(wire, 80),
        );
        if !installed {
            if wire[16..64].iter().any(|byte| *byte != 0)
                || wire[88..632].iter().any(|byte| *byte != 0)
            {
                return Err(AndroidPackageSnapshotError::NonCanonicalEmptyPackage);
            }
            return Self::empty(state, io);
        }

        if AndroidPackageCompatibilityProfile::from_raw(read_app_data_u16(wire, 54))
            != Some(AndroidPackageCompatibilityProfile::Resources1)
        {
            return Err(AndroidPackageSnapshotError::InvalidProfile);
        }
        let mut apk_sha256 = [0; 32];
        apk_sha256.copy_from_slice(&wire[88..120]);
        let mut signer_sha256 = [0; 32];
        signer_sha256.copy_from_slice(&wire[120..152]);
        Self::installed(
            state,
            io,
            AndroidPackageInstalledMetadata {
                generation: read_app_data_u64(wire, 16),
                version_code: read_app_data_u64(wire, 24),
                apk_length: read_app_data_u32(wire, 32),
                resources_table_crc32: read_app_data_u32(wire, 36),
                layout_xml_crc32: read_app_data_u32(wire, 40),
                layout_resource_id: read_app_data_u32(wire, 44),
                text_resource_id: read_app_data_u32(wire, 48),
                instruction_count: read_app_data_u16(wire, 52),
                apk_sha256,
                signer_sha256,
                package_name: &wire[152..152 + package_name_length],
                activity_name: &wire[248..248 + activity_name_length],
                title: &wire[376..376 + title_length],
                text: &wire[504..504 + text_length],
            },
        )
    }
}

/// Canonical authority-free view of ABI 55's installed package directory.
///
/// Entries are packed in stable volume order and contain no boot-source state,
/// I/O counters, handle, path, pointer, or storage location.
#[cfg(feature = "androidbox-multipackage4")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageDirectory {
    formatted: bool,
    revision: u64,
    count: u8,
    entries: [Option<AndroidPackageSnapshot>; ANDROID_PACKAGE_DIRECTORY_CAPACITY],
}

#[cfg(feature = "androidbox-multipackage4")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageDirectoryError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    UnknownFlags,
    InvalidCapacity,
    InvalidCount,
    InvalidRevision,
    InconsistentUnformattedState,
    NonZeroReserved,
    NonCanonicalUnusedEntry,
    InvalidEntry(AndroidPackageSnapshotError),
    EntryCarriesSourceState,
    EntryCarriesIoCounters,
    DuplicatePackage,
}

#[cfg(feature = "androidbox-multipackage4")]
impl AndroidPackageDirectory {
    pub fn new(
        formatted: bool,
        revision: u64,
        entries: &[AndroidPackageSnapshot],
    ) -> Result<Self, AndroidPackageDirectoryError> {
        if entries.len() > ANDROID_PACKAGE_DIRECTORY_CAPACITY {
            return Err(AndroidPackageDirectoryError::InvalidCount);
        }
        let mut packed = [None; ANDROID_PACKAGE_DIRECTORY_CAPACITY];
        for (index, entry) in entries.iter().copied().enumerate() {
            packed[index] = Some(entry);
        }
        let directory = Self {
            formatted,
            revision,
            count: entries.len() as u8,
            entries: packed,
        };
        directory.validate()?;
        Ok(directory)
    }

    pub const fn formatted(self) -> bool {
        self.formatted
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }

    pub const fn count(self) -> u8 {
        self.count
    }

    pub fn entry(&self, index: usize) -> Option<&AndroidPackageSnapshot> {
        self.entries.get(index).and_then(Option::as_ref)
    }

    pub fn encode(&self) -> [u8; ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE] {
        let mut wire = [0; ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE];
        wire[..8].copy_from_slice(&ANDROID_PACKAGE_DIRECTORY_MAGIC);
        write_app_data_u32(&mut wire, 8, ANDROID_PACKAGE_DIRECTORY_VERSION);
        write_app_data_u32(
            &mut wire,
            12,
            if self.formatted {
                ANDROID_PACKAGE_DIRECTORY_FLAG_FORMATTED
            } else {
                0
            },
        );
        write_app_data_u64(&mut wire, 16, self.revision);
        write_app_data_u16(&mut wire, 24, u16::from(self.count));
        write_app_data_u16(&mut wire, 26, ANDROID_PACKAGE_DIRECTORY_CAPACITY as u16);
        for index in 0..usize::from(self.count) {
            let start = 64 + index * ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE;
            let entry = self.entries[index]
                .as_ref()
                .expect("validated packed package directory")
                .encode();
            wire[start..start + ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE].copy_from_slice(&entry);
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, AndroidPackageDirectoryError> {
        if wire.len() != ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE {
            return Err(AndroidPackageDirectoryError::InvalidWireLength);
        }
        if wire[..8] != ANDROID_PACKAGE_DIRECTORY_MAGIC {
            return Err(AndroidPackageDirectoryError::InvalidMagic);
        }
        if read_app_data_u32(wire, 8) != ANDROID_PACKAGE_DIRECTORY_VERSION {
            return Err(AndroidPackageDirectoryError::UnsupportedVersion);
        }
        let flags = read_app_data_u32(wire, 12);
        if flags & !ANDROID_PACKAGE_DIRECTORY_FLAG_FORMATTED != 0 {
            return Err(AndroidPackageDirectoryError::UnknownFlags);
        }
        if read_app_data_u16(wire, 26) as usize != ANDROID_PACKAGE_DIRECTORY_CAPACITY {
            return Err(AndroidPackageDirectoryError::InvalidCapacity);
        }
        if wire[28..64].iter().any(|byte| *byte != 0) {
            return Err(AndroidPackageDirectoryError::NonZeroReserved);
        }
        let count = read_app_data_u16(wire, 24) as usize;
        if count > ANDROID_PACKAGE_DIRECTORY_CAPACITY {
            return Err(AndroidPackageDirectoryError::InvalidCount);
        }
        let mut entries = [None; ANDROID_PACKAGE_DIRECTORY_CAPACITY];
        for (index, destination) in entries.iter_mut().take(count).enumerate() {
            let start = 64 + index * ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE;
            *destination = Some(
                AndroidPackageSnapshot::decode(
                    &wire[start..start + ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE],
                )
                .map_err(AndroidPackageDirectoryError::InvalidEntry)?,
            );
        }
        let unused_start = 64 + count * ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE;
        if wire[unused_start..].iter().any(|byte| *byte != 0) {
            return Err(AndroidPackageDirectoryError::NonCanonicalUnusedEntry);
        }
        let result = Self {
            formatted: flags & ANDROID_PACKAGE_DIRECTORY_FLAG_FORMATTED != 0,
            revision: read_app_data_u64(wire, 16),
            count: count as u8,
            entries,
        };
        result.validate()?;
        Ok(result)
    }

    fn validate(&self) -> Result<(), AndroidPackageDirectoryError> {
        let count = usize::from(self.count);
        if count > ANDROID_PACKAGE_DIRECTORY_CAPACITY {
            return Err(AndroidPackageDirectoryError::InvalidCount);
        }
        if !self.formatted {
            if self.revision != 0 || count != 0 {
                return Err(AndroidPackageDirectoryError::InconsistentUnformattedState);
            }
        } else if self.revision == 0 {
            return Err(AndroidPackageDirectoryError::InvalidRevision);
        }
        for index in 0..count {
            let entry = self.entries[index]
                .as_ref()
                .ok_or(AndroidPackageDirectoryError::InvalidCount)?;
            if !entry.formatted()
                || !entry.installed_package()
                || entry.source_present()
                || entry.source_used()
                || entry.format_performed()
            {
                return Err(AndroidPackageDirectoryError::EntryCarriesSourceState);
            }
            let io = entry.io_counters();
            if io.reads() != 0 || io.writes() != 0 || io.flushes() != 0 {
                return Err(AndroidPackageDirectoryError::EntryCarriesIoCounters);
            }
            for earlier in 0..index {
                if self.entries[earlier]
                    .as_ref()
                    .ok_or(AndroidPackageDirectoryError::InvalidCount)?
                    .package_name_bytes()
                    == entry.package_name_bytes()
                {
                    return Err(AndroidPackageDirectoryError::DuplicatePackage);
                }
            }
        }
        if self.entries[count..].iter().any(Option::is_some) {
            return Err(AndroidPackageDirectoryError::InvalidCount);
        }
        Ok(())
    }
}

/// ABI 56 metadata for one decoded, signed APK launcher icon.
#[cfg(feature = "androidbox-icon-resources5")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageIconMetadata {
    pub resource_id: u32,
    pub png_crc32: u32,
    pub pixels: [u32; ANDROID_PACKAGE_ICON_PIXEL_COUNT],
    #[cfg(feature = "androidbox-density-icons7")]
    pub source_width: u16,
    #[cfg(feature = "androidbox-density-icons7")]
    pub source_height: u16,
    #[cfg(feature = "androidbox-density-icons7")]
    pub source_density_dpi: u16,
    #[cfg(feature = "androidbox-density-icons7")]
    pub color_quantized: bool,
}

/// Package-bound launcher artwork or canonical absence.
///
/// Identity fields are retained even when `present` is false, so an EL0
/// fallback can still prove that it belongs to the same directory entry.
#[cfg(feature = "androidbox-icon-resources5")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidPackageIcon {
    directory_revision: u64,
    selector: u8,
    generation: u64,
    apk_sha256: [u8; 32],
    icon: Option<AndroidPackageIconMetadata>,
}

#[cfg(feature = "androidbox-icon-resources5")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidPackageIconError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    UnknownFlags,
    InvalidSelector,
    InvalidDirectoryRevision,
    InvalidGeneration,
    ZeroApkSha256,
    InvalidDimensions,
    InvalidResourceId,
    InvalidPngCrc32,
    EmptyPixels,
    NonCanonicalTransparentPixel,
    #[cfg(feature = "androidbox-density-icons7")]
    InvalidSourceMetadata,
    NonCanonicalAbsentIcon,
    NonZeroReserved,
}

#[cfg(feature = "androidbox-icon-resources5")]
impl AndroidPackageIcon {
    pub fn new(
        directory_revision: u64,
        selector: u8,
        generation: u64,
        apk_sha256: [u8; 32],
        icon: Option<AndroidPackageIconMetadata>,
    ) -> Result<Self, AndroidPackageIconError> {
        let value = Self {
            directory_revision,
            selector,
            generation,
            apk_sha256,
            icon,
        };
        value.validate()?;
        Ok(value)
    }

    pub const fn directory_revision(self) -> u64 {
        self.directory_revision
    }

    pub const fn selector(self) -> u8 {
        self.selector
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn apk_sha256(self) -> [u8; 32] {
        self.apk_sha256
    }

    pub const fn present(self) -> bool {
        self.icon.is_some()
    }

    pub const fn icon(self) -> Option<AndroidPackageIconMetadata> {
        self.icon
    }

    pub fn encode(&self) -> [u8; ANDROID_PACKAGE_ICON_WIRE_SIZE] {
        let mut wire = [0; ANDROID_PACKAGE_ICON_WIRE_SIZE];
        wire[..8].copy_from_slice(&ANDROID_PACKAGE_ICON_MAGIC);
        write_app_data_u32(&mut wire, 8, ANDROID_PACKAGE_ICON_VERSION);
        #[cfg(feature = "androidbox-density-icons7")]
        let flags = self.icon.map_or(0, |icon| {
            ANDROID_PACKAGE_ICON_FLAG_PRESENT
                | if icon.source_width == ANDROID_PACKAGE_ICON_SOURCE_MDP_WIDTH {
                    ANDROID_PACKAGE_ICON_FLAG_DENSITY_NORMALIZED
                } else {
                    0
                }
                | if icon.color_quantized {
                    ANDROID_PACKAGE_ICON_FLAG_COLOR_QUANTIZED
                } else {
                    0
                }
        });
        #[cfg(not(feature = "androidbox-density-icons7"))]
        let flags = if self.icon.is_some() {
            ANDROID_PACKAGE_ICON_FLAG_PRESENT
        } else {
            0
        };
        write_app_data_u32(&mut wire, 12, flags);
        write_app_data_u64(&mut wire, 16, self.directory_revision);
        write_app_data_u16(&mut wire, 24, u16::from(self.selector));
        write_app_data_u16(
            &mut wire,
            26,
            if self.icon.is_some() {
                ANDROID_PACKAGE_ICON_WIDTH as u16
            } else {
                0
            },
        );
        write_app_data_u16(
            &mut wire,
            28,
            if self.icon.is_some() {
                ANDROID_PACKAGE_ICON_HEIGHT as u16
            } else {
                0
            },
        );
        write_app_data_u64(&mut wire, 32, self.generation);
        wire[40..72].copy_from_slice(&self.apk_sha256);
        if let Some(icon) = self.icon {
            write_app_data_u32(&mut wire, 72, icon.resource_id);
            write_app_data_u32(&mut wire, 76, icon.png_crc32);
            for (index, pixel) in icon.pixels.iter().copied().enumerate() {
                write_app_data_u32(&mut wire, 80 + index * 4, pixel);
            }
            #[cfg(feature = "androidbox-density-icons7")]
            {
                write_app_data_u16(&mut wire, 1_104, icon.source_width);
                write_app_data_u16(&mut wire, 1_106, icon.source_height);
                write_app_data_u16(&mut wire, 1_108, icon.source_density_dpi);
            }
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, AndroidPackageIconError> {
        if wire.len() != ANDROID_PACKAGE_ICON_WIRE_SIZE {
            return Err(AndroidPackageIconError::InvalidWireLength);
        }
        if wire[..8] != ANDROID_PACKAGE_ICON_MAGIC {
            return Err(AndroidPackageIconError::InvalidMagic);
        }
        if read_app_data_u32(wire, 8) != ANDROID_PACKAGE_ICON_VERSION {
            return Err(AndroidPackageIconError::UnsupportedVersion);
        }
        let flags = read_app_data_u32(wire, 12);
        #[cfg(feature = "androidbox-density-icons7")]
        let known_flags = ANDROID_PACKAGE_ICON_FLAG_PRESENT
            | ANDROID_PACKAGE_ICON_FLAG_DENSITY_NORMALIZED
            | ANDROID_PACKAGE_ICON_FLAG_COLOR_QUANTIZED;
        #[cfg(not(feature = "androidbox-density-icons7"))]
        let known_flags = ANDROID_PACKAGE_ICON_FLAG_PRESENT;
        if flags & !known_flags != 0 {
            return Err(AndroidPackageIconError::UnknownFlags);
        }
        #[cfg(feature = "androidbox-density-icons7")]
        let reserved_nonzero = wire[1_110..].iter().any(|byte| *byte != 0);
        #[cfg(not(feature = "androidbox-density-icons7"))]
        let reserved_nonzero = wire[1_104..].iter().any(|byte| *byte != 0);
        if read_app_data_u16(wire, 30) != 0 || reserved_nonzero {
            return Err(AndroidPackageIconError::NonZeroReserved);
        }
        let present = flags & ANDROID_PACKAGE_ICON_FLAG_PRESENT != 0;
        #[cfg(feature = "androidbox-density-icons7")]
        if !present
            && flags
                & (ANDROID_PACKAGE_ICON_FLAG_DENSITY_NORMALIZED
                    | ANDROID_PACKAGE_ICON_FLAG_COLOR_QUANTIZED)
                != 0
        {
            return Err(AndroidPackageIconError::NonCanonicalAbsentIcon);
        }
        let dimensions = (
            read_app_data_u16(wire, 26) as usize,
            read_app_data_u16(wire, 28) as usize,
        );
        let icon = if present {
            if dimensions != (ANDROID_PACKAGE_ICON_WIDTH, ANDROID_PACKAGE_ICON_HEIGHT) {
                return Err(AndroidPackageIconError::InvalidDimensions);
            }
            let mut pixels = [0; ANDROID_PACKAGE_ICON_PIXEL_COUNT];
            for (index, pixel) in pixels.iter_mut().enumerate() {
                *pixel = read_app_data_u32(wire, 80 + index * 4);
            }
            Some(AndroidPackageIconMetadata {
                resource_id: read_app_data_u32(wire, 72),
                png_crc32: read_app_data_u32(wire, 76),
                pixels,
                #[cfg(feature = "androidbox-density-icons7")]
                source_width: read_app_data_u16(wire, 1_104),
                #[cfg(feature = "androidbox-density-icons7")]
                source_height: read_app_data_u16(wire, 1_106),
                #[cfg(feature = "androidbox-density-icons7")]
                source_density_dpi: read_app_data_u16(wire, 1_108),
                #[cfg(feature = "androidbox-density-icons7")]
                color_quantized: flags & ANDROID_PACKAGE_ICON_FLAG_COLOR_QUANTIZED != 0,
            })
        } else {
            #[cfg(feature = "androidbox-density-icons7")]
            let icon_payload_nonzero = wire[72..1_110].iter().any(|byte| *byte != 0);
            #[cfg(not(feature = "androidbox-density-icons7"))]
            let icon_payload_nonzero = wire[72..1_104].iter().any(|byte| *byte != 0);
            if dimensions != (0, 0) || icon_payload_nonzero {
                return Err(AndroidPackageIconError::NonCanonicalAbsentIcon);
            }
            None
        };
        Self::new(
            read_app_data_u64(wire, 16),
            u8::try_from(read_app_data_u16(wire, 24))
                .map_err(|_| AndroidPackageIconError::InvalidSelector)?,
            read_app_data_u64(wire, 32),
            wire[40..72]
                .try_into()
                .map_err(|_| AndroidPackageIconError::ZeroApkSha256)?,
            icon,
        )
    }

    fn validate(&self) -> Result<(), AndroidPackageIconError> {
        if self.directory_revision == 0 {
            return Err(AndroidPackageIconError::InvalidDirectoryRevision);
        }
        if usize::from(self.selector) >= ANDROID_PACKAGE_DIRECTORY_CAPACITY {
            return Err(AndroidPackageIconError::InvalidSelector);
        }
        if self.generation == 0 {
            return Err(AndroidPackageIconError::InvalidGeneration);
        }
        if self.apk_sha256.iter().all(|byte| *byte == 0) {
            return Err(AndroidPackageIconError::ZeroApkSha256);
        }
        if let Some(icon) = self.icon {
            #[cfg(feature = "androidbox-density-icons7")]
            {
                let density_normalized = icon.source_width == ANDROID_PACKAGE_ICON_SOURCE_MDP_WIDTH
                    && icon.source_height == ANDROID_PACKAGE_ICON_SOURCE_MDP_HEIGHT
                    && icon.source_density_dpi == ANDROID_PACKAGE_ICON_SOURCE_MDP_DENSITY_DPI;
                let default_source = icon.source_width == ANDROID_PACKAGE_ICON_WIDTH as u16
                    && icon.source_height == ANDROID_PACKAGE_ICON_HEIGHT as u16
                    && icon.source_density_dpi == 0;
                if !density_normalized && !default_source {
                    return Err(AndroidPackageIconError::InvalidSourceMetadata);
                }
            }
            if icon.resource_id == 0 {
                return Err(AndroidPackageIconError::InvalidResourceId);
            }
            if icon.png_crc32 == 0 {
                return Err(AndroidPackageIconError::InvalidPngCrc32);
            }
            if !icon.pixels.iter().any(|pixel| pixel >> 24 != 0) {
                return Err(AndroidPackageIconError::EmptyPixels);
            }
            if icon
                .pixels
                .iter()
                .any(|pixel| pixel >> 24 == 0 && pixel & 0x00ff_ffff != 0)
            {
                return Err(AndroidPackageIconError::NonCanonicalTransparentPixel);
            }
        }
        Ok(())
    }
}

#[cfg(feature = "androidbox-apk-install0")]
fn validate_android_package_snapshot_state(
    state: AndroidPackageSnapshotState,
    installed: bool,
) -> Result<u32, AndroidPackageSnapshotError> {
    #[cfg(not(feature = "androidbox-runtime-install2"))]
    if state.source_present != state.source_used {
        return Err(AndroidPackageSnapshotError::InconsistentSourceState);
    }
    #[cfg(feature = "androidbox-runtime-install2")]
    if state.source_used && !state.source_present {
        return Err(AndroidPackageSnapshotError::InconsistentSourceState);
    }
    if state.format_performed && (!state.formatted || !state.source_present) {
        return Err(AndroidPackageSnapshotError::InvalidFormatPerformedState);
    }
    if !installed && state.source_used {
        return Err(AndroidPackageSnapshotError::SourceUsedWithoutInstalledPackage);
    }
    if installed && !state.formatted {
        return Err(AndroidPackageSnapshotError::InstalledVolumeNotFormatted);
    }

    let mut flags = 0;
    if state.formatted {
        flags |= ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMATTED;
    }
    if installed {
        flags |= ANDROID_PACKAGE_SNAPSHOT_FLAG_INSTALLED;
    }
    if state.source_present {
        flags |= ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_PRESENT;
    }
    if state.source_used {
        flags |= ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_USED;
    }
    if state.format_performed {
        flags |= ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMAT_PERFORMED;
    }
    Ok(flags)
}

#[cfg(feature = "androidbox-apk-install0")]
fn validate_android_package_metadata(
    metadata: &AndroidPackageInstalledMetadata<'_>,
) -> Result<(), AndroidPackageSnapshotError> {
    if metadata.generation == 0 {
        return Err(AndroidPackageSnapshotError::InvalidGeneration);
    }
    if metadata.version_code == 0 || metadata.version_code > u64::from(u32::MAX) {
        return Err(AndroidPackageSnapshotError::InvalidVersionCode);
    }
    if metadata.apk_length == 0 || metadata.apk_length > ANDROID_PACKAGE_APK_MAX_BYTES {
        return Err(AndroidPackageSnapshotError::InvalidApkLength);
    }
    if metadata.resources_table_crc32 == 0 {
        return Err(AndroidPackageSnapshotError::InvalidResourcesTableCrc32);
    }
    if metadata.layout_xml_crc32 == 0 {
        return Err(AndroidPackageSnapshotError::InvalidLayoutXmlCrc32);
    }
    if metadata.layout_resource_id >> 24 != 0x7f {
        return Err(AndroidPackageSnapshotError::InvalidLayoutResourceId);
    }
    if metadata.text_resource_id >> 24 != 0x7f
        || metadata.text_resource_id == metadata.layout_resource_id
    {
        return Err(AndroidPackageSnapshotError::InvalidTextResourceId);
    }
    if metadata.instruction_count == 0 {
        return Err(AndroidPackageSnapshotError::InvalidInstructionCount);
    }
    if metadata.apk_sha256.iter().all(|byte| *byte == 0) {
        return Err(AndroidPackageSnapshotError::ZeroApkSha256);
    }
    if metadata.signer_sha256.iter().all(|byte| *byte == 0) {
        return Err(AndroidPackageSnapshotError::ZeroSignerSha256);
    }
    validate_android_package_text(
        metadata.package_name,
        ANDROID_PACKAGE_NAME_MAX_BYTES,
        AndroidPackageSnapshotError::InvalidPackageNameLength,
        AndroidPackageSnapshotError::NonPrintablePackageName,
    )?;
    validate_android_package_text(
        metadata.activity_name,
        ANDROID_PACKAGE_ACTIVITY_MAX_BYTES,
        AndroidPackageSnapshotError::InvalidActivityNameLength,
        AndroidPackageSnapshotError::NonPrintableActivityName,
    )?;
    validate_android_package_text(
        metadata.title,
        ANDROID_PACKAGE_TITLE_MAX_BYTES,
        AndroidPackageSnapshotError::InvalidTitleLength,
        AndroidPackageSnapshotError::NonPrintableTitle,
    )?;
    validate_android_package_text(
        metadata.text,
        ANDROID_PACKAGE_TEXT_MAX_BYTES,
        AndroidPackageSnapshotError::InvalidTextLength,
        AndroidPackageSnapshotError::NonPrintableText,
    )
}

#[cfg(feature = "androidbox-apk-install0")]
fn validate_android_package_text(
    text: &[u8],
    maximum_length: usize,
    length_error: AndroidPackageSnapshotError,
    printable_error: AndroidPackageSnapshotError,
) -> Result<(), AndroidPackageSnapshotError> {
    if text.is_empty() || text.len() > maximum_length {
        return Err(length_error);
    }
    if text.iter().any(|byte| !(0x20..=0x7e).contains(byte)) {
        return Err(printable_error);
    }
    Ok(())
}

#[cfg(feature = "androidbox-apk-install0")]
fn validate_android_package_wire_text(
    wire: &[u8],
    offset: usize,
    maximum_length: usize,
    length: usize,
    length_error: AndroidPackageSnapshotError,
    padding_error: AndroidPackageSnapshotError,
) -> Result<(), AndroidPackageSnapshotError> {
    if length > maximum_length {
        return Err(length_error);
    }
    if wire[offset + length..offset + maximum_length]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(padding_error);
    }
    Ok(())
}

#[cfg(feature = "androidbox-apk-install0")]
fn validate_android_package_relaunch_text(
    text: &[u8],
    maximum_length: usize,
    length_error: AndroidPackageRelaunchRequestError,
    printable_error: AndroidPackageRelaunchRequestError,
) -> Result<(), AndroidPackageRelaunchRequestError> {
    if text.is_empty() || text.len() > maximum_length {
        return Err(length_error);
    }
    if text.iter().any(|byte| !(0x20..=0x7e).contains(byte)) {
        return Err(printable_error);
    }
    Ok(())
}

#[cfg(feature = "androidbox-apk-install0")]
fn validate_android_package_relaunch_wire_text(
    wire: &[u8],
    offset: usize,
    maximum_length: usize,
    length: usize,
    length_error: AndroidPackageRelaunchRequestError,
    padding_error: AndroidPackageRelaunchRequestError,
) -> Result<(), AndroidPackageRelaunchRequestError> {
    if length == 0 || length > maximum_length {
        return Err(length_error);
    }
    if wire[offset + length..offset + maximum_length]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(padding_error);
    }
    Ok(())
}

/// Longest canonical root-relative path accepted by the AppData ABI.
pub const APP_DATA_PATH_MAX_BYTES: usize = 64;
/// Largest number of non-empty components in one canonical AppData path.
pub const APP_DATA_PATH_MAX_DEPTH: usize = 4;
/// Largest file value accepted by one atomic AppData replacement.
pub const APP_DATA_FILE_MAX_BYTES: usize = 4096;
/// Largest number of files and directories in one AppData principal's root.
pub const APP_DATA_DIRECTORY_MAX_ENTRIES: usize = 32;
/// Number of 512-byte sectors addressable relative to the AppData volume.
pub const APP_DATA_VOLUME_SECTORS: u64 = 1_920;

/// Size of one storage-sector payload in bytes.
pub const STORAGE_SECTOR_SIZE: usize = 512;
/// Largest payload copied into one immutable IPC buffer.
pub const IPC_BUFFER_PAYLOAD_MAX_BYTES: usize = 4_160;
/// Immutable IPC-buffer creation accepts no optional flags.
pub const IPC_BUFFER_CREATE_FLAGS_NONE: u64 = 0;

/// Little-endian ASCII `SBRQ` at the start of a [`StorageBlockRequest`] wire.
pub const STORAGE_BLOCK_REQUEST_MAGIC: u32 = u32::from_le_bytes(*b"SBRQ");
pub const STORAGE_BLOCK_REQUEST_VERSION: u16 = 2;
/// Largest sector count carried by one bounded storage-block request.
pub const STORAGE_BLOCK_MAX_SECTORS: usize = 8;
/// Largest data area carried by one bounded storage-block request.
pub const STORAGE_BLOCK_DATA_MAX_BYTES: usize = STORAGE_SECTOR_SIZE * STORAGE_BLOCK_MAX_SECTORS;
/// Canonical request size: a 64-byte header followed by at most eight sectors.
pub const STORAGE_BLOCK_REQUEST_WIRE_SIZE: usize = 64 + STORAGE_BLOCK_DATA_MAX_BYTES;
/// Token value reserved for a request that has not yet entered the kernel.
pub const STORAGE_BLOCK_REQUEST_UNASSIGNED_TOKEN: u64 = 0;

const _: () = assert!(STORAGE_BLOCK_MAX_SECTORS == 8);
const _: () = assert!(STORAGE_BLOCK_DATA_MAX_BYTES == 4_096);
const _: () = assert!(STORAGE_BLOCK_REQUEST_WIRE_SIZE == IPC_BUFFER_PAYLOAD_MAX_BYTES);

/// Little-endian ASCII `SSBN` at the start of a [`StorageSessionBinding`] wire.
pub const STORAGE_SESSION_BINDING_MAGIC: u32 = u32::from_le_bytes(*b"SSBN");
pub const STORAGE_SESSION_BINDING_VERSION: u16 = 1;
/// Exact binding record size written by [`SyscallNumber::StorageAccept`].
pub const STORAGE_SESSION_BINDING_WIRE_SIZE: usize = 64;
/// Only read and write authority may be requested through `StorageConnect`.
pub const STORAGE_CONNECT_RIGHTS_MASK: u32 = (1 << 0) | (1 << 1);

/// Little-endian ASCII `ADFR` at the start of a [`FileReplaceRequest`] wire.
pub const FILE_REPLACE_REQUEST_MAGIC: u32 = u32::from_le_bytes(*b"ADFR");
pub const FILE_REPLACE_REQUEST_VERSION: u16 = 1;
/// Canonical encoded size passed as `x2` to [`SyscallNumber::FileReplaceAt`].
pub const FILE_REPLACE_REQUEST_WIRE_SIZE: usize = 112;
/// Replaces regardless of whether the path is absent or at any generation.
pub const FILE_REPLACE_FLAGS_EXPECT_ANY: u16 = 0;
/// Replaces only when the path does not currently exist.
pub const FILE_REPLACE_FLAG_CREATE_ONLY: u16 = 1 << 0;
/// Replaces only when the path has the exact non-zero expected generation.
pub const FILE_REPLACE_FLAG_EXPECT_EXACT: u16 = 1 << 1;

/// Little-endian ASCII `ADDE` at the start of an AppData directory-entry wire.
pub const APP_DATA_DIRECTORY_ENTRY_MAGIC: u32 = u32::from_le_bytes(*b"ADDE");
pub const APP_DATA_DIRECTORY_ENTRY_VERSION: u16 = 1;
pub const APP_DATA_DIRECTORY_ENTRY_FLAGS_NONE: u32 = 0;
/// Canonical output size written by [`SyscallNumber::DirectoryReadAt`].
pub const APP_DATA_DIRECTORY_ENTRY_WIRE_SIZE: usize = 112;

/// A stable, kernel-derived AppData namespace identity.
///
/// Principals are tied to a trusted image identity rather than a process ID,
/// so restarting an image reopens the same namespace. Zero is permanently
/// invalid and cannot name an AppData namespace.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppDataPrincipal(u64);

impl AppDataPrincipal {
    /// Stable principal assigned to the canonical primary App image.
    pub const PRIMARY_APP: Self = Self(1);
    /// Stable principal assigned to the canonical Launcher image.
    pub const LAUNCHER: Self = Self(2);

    pub const fn from_raw(raw: u64) -> Option<Self> {
        if raw == 0 { None } else { Some(Self(raw)) }
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppDataPathError {
    Empty,
    TooLong,
    InvalidUtf8,
    ContainsNul,
    Absolute,
    TrailingSlash,
    EmptyComponent,
    CurrentDirectoryComponent,
    ParentDirectoryComponent,
    TooDeep,
}

/// Validates one byte-exact, UTF-8, root-relative AppData path.
///
/// Canonical paths contain between one and four non-empty components, have no
/// leading or trailing slash, and contain neither NUL nor `.`/`..` components.
/// Unicode is accepted byte-for-byte; this ABI deliberately does not perform
/// locale-sensitive or Unicode-normalization transforms.
pub fn validate_app_data_path(path: &[u8]) -> Result<&str, AppDataPathError> {
    if path.is_empty() {
        return Err(AppDataPathError::Empty);
    }
    if path.len() > APP_DATA_PATH_MAX_BYTES {
        return Err(AppDataPathError::TooLong);
    }
    let path = core::str::from_utf8(path).map_err(|_| AppDataPathError::InvalidUtf8)?;
    if path.as_bytes().contains(&0) {
        return Err(AppDataPathError::ContainsNul);
    }
    if path.starts_with('/') {
        return Err(AppDataPathError::Absolute);
    }
    if path.ends_with('/') {
        return Err(AppDataPathError::TrailingSlash);
    }

    let mut depth = 0;
    for component in path.split('/') {
        if component.is_empty() {
            return Err(AppDataPathError::EmptyComponent);
        }
        if component == "." {
            return Err(AppDataPathError::CurrentDirectoryComponent);
        }
        if component == ".." {
            return Err(AppDataPathError::ParentDirectoryComponent);
        }
        depth += 1;
        if depth > APP_DATA_PATH_MAX_DEPTH {
            return Err(AppDataPathError::TooDeep);
        }
    }
    Ok(path)
}

/// Compare-and-swap precondition for one atomic file replacement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileReplaceCas {
    /// The existing state is irrelevant.
    Any,
    /// The path must not already exist.
    CreateOnly,
    /// The path must be a file at this exact non-zero generation.
    Exact(u64),
}

impl FileReplaceCas {
    fn encode_fields(self) -> Result<(u16, u64), FileReplaceRequestError> {
        match self {
            Self::Any => Ok((FILE_REPLACE_FLAGS_EXPECT_ANY, 0)),
            Self::CreateOnly => Ok((FILE_REPLACE_FLAG_CREATE_ONLY, 0)),
            Self::Exact(0) => Err(FileReplaceRequestError::InvalidCas),
            Self::Exact(generation) => Ok((FILE_REPLACE_FLAG_EXPECT_EXACT, generation)),
        }
    }

    fn decode_fields(flags: u16, generation: u64) -> Result<Self, FileReplaceRequestError> {
        match flags {
            FILE_REPLACE_FLAGS_EXPECT_ANY if generation == 0 => Ok(Self::Any),
            FILE_REPLACE_FLAG_CREATE_ONLY if generation == 0 => Ok(Self::CreateOnly),
            FILE_REPLACE_FLAG_EXPECT_EXACT if generation != 0 => Ok(Self::Exact(generation)),
            FILE_REPLACE_FLAGS_EXPECT_ANY
            | FILE_REPLACE_FLAG_CREATE_ONLY
            | FILE_REPLACE_FLAG_EXPECT_EXACT => Err(FileReplaceRequestError::InvalidCas),
            _ => Err(FileReplaceRequestError::InvalidFlags),
        }
    }
}

/// A validated semantic representation of the canonical syscall-43 wire.
///
/// The path is inline so the kernel can copy and validate the entire request
/// once. The value remains in caller memory for the duration of the syscall.
/// Its pointer is zero exactly when its length is zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileReplaceRequest {
    path: [u8; APP_DATA_PATH_MAX_BYTES],
    path_length: u32,
    value_pointer: u64,
    value_length: u32,
    cas: FileReplaceCas,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileReplaceRequestError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    NonZeroReserved,
    InvalidFlags,
    InvalidCas,
    Path(AppDataPathError),
    NonZeroPathPadding,
    ValueTooLarge,
    InvalidValuePointer,
    ValueRangeOverflow,
}

impl FileReplaceRequest {
    /// Creates a request after validating path, value range, and CAS mode.
    pub fn new(
        path: &[u8],
        value_pointer: u64,
        value_length: usize,
        cas: FileReplaceCas,
    ) -> Result<Self, FileReplaceRequestError> {
        validate_app_data_path(path).map_err(FileReplaceRequestError::Path)?;
        validate_app_data_value(value_pointer, value_length)?;
        cas.encode_fields()?;

        let mut stored_path = [0; APP_DATA_PATH_MAX_BYTES];
        stored_path[..path.len()].copy_from_slice(path);
        Ok(Self {
            path: stored_path,
            path_length: path.len() as u32,
            value_pointer,
            value_length: value_length as u32,
            cas,
        })
    }

    pub fn path(&self) -> &str {
        // Construction and decoding both validate this exact prefix as UTF-8.
        core::str::from_utf8(self.path_bytes()).expect("validated AppData path")
    }

    pub fn path_bytes(&self) -> &[u8] {
        &self.path[..self.path_length as usize]
    }

    pub const fn value_pointer(&self) -> u64 {
        self.value_pointer
    }

    pub const fn value_length(&self) -> usize {
        self.value_length as usize
    }

    pub const fn cas(&self) -> FileReplaceCas {
        self.cas
    }

    /// Encodes the only canonical 112-byte representation.
    ///
    /// ```text
    /// 0   u32 magic = "ADFR"
    /// 4   u16 version = 1
    /// 6   u16 CAS flags (0 any, 1 create-only, 2 exact)
    /// 8   u32 path length
    /// 12  u32 value length
    /// 16  u64 value pointer
    /// 24  u64 expected generation (non-zero only for exact CAS)
    /// 32  reserved[16] = 0
    /// 48  path[64], followed by zero padding
    /// ```
    pub fn encode(&self) -> [u8; FILE_REPLACE_REQUEST_WIRE_SIZE] {
        let mut wire = [0; FILE_REPLACE_REQUEST_WIRE_SIZE];
        let (flags, generation) = self
            .cas
            .encode_fields()
            .expect("validated FileReplace CAS mode");
        write_app_data_u32(&mut wire, 0, FILE_REPLACE_REQUEST_MAGIC);
        write_app_data_u16(&mut wire, 4, FILE_REPLACE_REQUEST_VERSION);
        write_app_data_u16(&mut wire, 6, flags);
        write_app_data_u32(&mut wire, 8, self.path_length);
        write_app_data_u32(&mut wire, 12, self.value_length);
        write_app_data_u64(&mut wire, 16, self.value_pointer);
        write_app_data_u64(&mut wire, 24, generation);
        wire[48..].copy_from_slice(&self.path);
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, FileReplaceRequestError> {
        if wire.len() != FILE_REPLACE_REQUEST_WIRE_SIZE {
            return Err(FileReplaceRequestError::InvalidWireLength);
        }
        if read_app_data_u32(wire, 0) != FILE_REPLACE_REQUEST_MAGIC {
            return Err(FileReplaceRequestError::InvalidMagic);
        }
        if read_app_data_u16(wire, 4) != FILE_REPLACE_REQUEST_VERSION {
            return Err(FileReplaceRequestError::UnsupportedVersion);
        }
        if wire[32..48].iter().any(|byte| *byte != 0) {
            return Err(FileReplaceRequestError::NonZeroReserved);
        }

        let cas =
            FileReplaceCas::decode_fields(read_app_data_u16(wire, 6), read_app_data_u64(wire, 24))?;
        let path_length = read_app_data_u32(wire, 8) as usize;
        if path_length > APP_DATA_PATH_MAX_BYTES {
            return Err(FileReplaceRequestError::Path(AppDataPathError::TooLong));
        }
        if wire[48 + path_length..].iter().any(|byte| *byte != 0) {
            return Err(FileReplaceRequestError::NonZeroPathPadding);
        }
        let path = &wire[48..48 + path_length];
        validate_app_data_path(path).map_err(FileReplaceRequestError::Path)?;

        let value_pointer = read_app_data_u64(wire, 16);
        let value_length = read_app_data_u32(wire, 12) as usize;
        validate_app_data_value(value_pointer, value_length)?;
        Self::new(path, value_pointer, value_length, cas)
    }
}

fn validate_app_data_value(
    value_pointer: u64,
    value_length: usize,
) -> Result<(), FileReplaceRequestError> {
    if value_length > APP_DATA_FILE_MAX_BYTES {
        return Err(FileReplaceRequestError::ValueTooLarge);
    }
    if (value_length == 0) != (value_pointer == 0) {
        return Err(FileReplaceRequestError::InvalidValuePointer);
    }
    if value_length != 0 && value_pointer.checked_add(value_length as u64).is_none() {
        return Err(FileReplaceRequestError::ValueRangeOverflow);
    }
    Ok(())
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppDataDirectoryEntryKind {
    File = 1,
    Directory = 2,
}

impl AppDataDirectoryEntryKind {
    pub const fn raw(self) -> u16 {
        self as u16
    }

    pub const fn from_raw(raw: u16) -> Option<Self> {
        match raw {
            1 => Some(Self::File),
            2 => Some(Self::Directory),
            _ => None,
        }
    }
}

/// One validated entry returned by the flattened root-relative enumeration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppDataDirectoryEntry {
    kind: AppDataDirectoryEntryKind,
    path: [u8; APP_DATA_PATH_MAX_BYTES],
    path_length: u32,
    generation: u64,
    size: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppDataDirectoryEntryError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    InvalidKind,
    NonZeroFlags,
    NonZeroReserved,
    Path(AppDataPathError),
    NonZeroPathPadding,
    InvalidGeneration,
    InvalidSize,
    NonCanonicalDirectoryMetadata,
}

impl AppDataDirectoryEntry {
    pub fn file(
        path: &[u8],
        generation: u64,
        size: usize,
    ) -> Result<Self, AppDataDirectoryEntryError> {
        if generation == 0 {
            return Err(AppDataDirectoryEntryError::InvalidGeneration);
        }
        if size > APP_DATA_FILE_MAX_BYTES {
            return Err(AppDataDirectoryEntryError::InvalidSize);
        }
        Self::new(
            AppDataDirectoryEntryKind::File,
            path,
            generation,
            size as u64,
        )
    }

    pub fn directory(path: &[u8]) -> Result<Self, AppDataDirectoryEntryError> {
        Self::new(AppDataDirectoryEntryKind::Directory, path, 0, 0)
    }

    fn new(
        kind: AppDataDirectoryEntryKind,
        path: &[u8],
        generation: u64,
        size: u64,
    ) -> Result<Self, AppDataDirectoryEntryError> {
        validate_app_data_path(path).map_err(AppDataDirectoryEntryError::Path)?;
        let mut stored_path = [0; APP_DATA_PATH_MAX_BYTES];
        stored_path[..path.len()].copy_from_slice(path);
        Ok(Self {
            kind,
            path: stored_path,
            path_length: path.len() as u32,
            generation,
            size,
        })
    }

    pub const fn kind(&self) -> AppDataDirectoryEntryKind {
        self.kind
    }

    pub fn path(&self) -> &str {
        core::str::from_utf8(self.path_bytes()).expect("validated AppData entry path")
    }

    pub fn path_bytes(&self) -> &[u8] {
        &self.path[..self.path_length as usize]
    }

    /// File generation, or zero for a directory.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// File size, or zero for a directory.
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// Encodes the only canonical 112-byte representation.
    ///
    /// ```text
    /// 0   u32 magic = "ADDE"
    /// 4   u16 version = 1
    /// 6   u16 kind (1 file, 2 directory)
    /// 8   u32 flags = 0
    /// 12  u32 path length
    /// 16  u64 generation (non-zero file, zero directory)
    /// 24  u64 size (0..=4096 file, zero directory)
    /// 32  reserved[16] = 0
    /// 48  path[64], followed by zero padding
    /// ```
    pub fn encode(&self) -> [u8; APP_DATA_DIRECTORY_ENTRY_WIRE_SIZE] {
        let mut wire = [0; APP_DATA_DIRECTORY_ENTRY_WIRE_SIZE];
        write_app_data_u32(&mut wire, 0, APP_DATA_DIRECTORY_ENTRY_MAGIC);
        write_app_data_u16(&mut wire, 4, APP_DATA_DIRECTORY_ENTRY_VERSION);
        write_app_data_u16(&mut wire, 6, self.kind.raw());
        write_app_data_u32(&mut wire, 8, APP_DATA_DIRECTORY_ENTRY_FLAGS_NONE);
        write_app_data_u32(&mut wire, 12, self.path_length);
        write_app_data_u64(&mut wire, 16, self.generation);
        write_app_data_u64(&mut wire, 24, self.size);
        wire[48..].copy_from_slice(&self.path);
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, AppDataDirectoryEntryError> {
        if wire.len() != APP_DATA_DIRECTORY_ENTRY_WIRE_SIZE {
            return Err(AppDataDirectoryEntryError::InvalidWireLength);
        }
        if read_app_data_u32(wire, 0) != APP_DATA_DIRECTORY_ENTRY_MAGIC {
            return Err(AppDataDirectoryEntryError::InvalidMagic);
        }
        if read_app_data_u16(wire, 4) != APP_DATA_DIRECTORY_ENTRY_VERSION {
            return Err(AppDataDirectoryEntryError::UnsupportedVersion);
        }
        let kind = AppDataDirectoryEntryKind::from_raw(read_app_data_u16(wire, 6))
            .ok_or(AppDataDirectoryEntryError::InvalidKind)?;
        if read_app_data_u32(wire, 8) != APP_DATA_DIRECTORY_ENTRY_FLAGS_NONE {
            return Err(AppDataDirectoryEntryError::NonZeroFlags);
        }
        if wire[32..48].iter().any(|byte| *byte != 0) {
            return Err(AppDataDirectoryEntryError::NonZeroReserved);
        }

        let path_length = read_app_data_u32(wire, 12) as usize;
        if path_length > APP_DATA_PATH_MAX_BYTES {
            return Err(AppDataDirectoryEntryError::Path(AppDataPathError::TooLong));
        }
        if wire[48 + path_length..].iter().any(|byte| *byte != 0) {
            return Err(AppDataDirectoryEntryError::NonZeroPathPadding);
        }
        let path = &wire[48..48 + path_length];
        validate_app_data_path(path).map_err(AppDataDirectoryEntryError::Path)?;

        let generation = read_app_data_u64(wire, 16);
        let size = read_app_data_u64(wire, 24);
        match kind {
            AppDataDirectoryEntryKind::File => {
                if generation == 0 {
                    return Err(AppDataDirectoryEntryError::InvalidGeneration);
                }
                if size > APP_DATA_FILE_MAX_BYTES as u64 {
                    return Err(AppDataDirectoryEntryError::InvalidSize);
                }
                Self::file(path, generation, size as usize)
            }
            AppDataDirectoryEntryKind::Directory => {
                if generation != 0 || size != 0 {
                    return Err(AppDataDirectoryEntryError::NonCanonicalDirectoryMetadata);
                }
                Self::directory(path)
            }
        }
    }
}

fn write_app_data_u16(wire: &mut [u8], offset: usize, value: u16) {
    wire[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_app_data_u32(wire: &mut [u8], offset: usize, value: u32) {
    wire[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_app_data_u64(wire: &mut [u8], offset: usize, value: u64) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_app_data_u16(wire: &[u8], offset: usize) -> u16 {
    let mut bytes = [0; 2];
    bytes.copy_from_slice(&wire[offset..offset + 2]);
    u16::from_le_bytes(bytes)
}

fn read_app_data_u32(wire: &[u8], offset: usize) -> u32 {
    let mut bytes = [0; 4];
    bytes.copy_from_slice(&wire[offset..offset + 4]);
    u32::from_le_bytes(bytes)
}

fn read_app_data_u64(wire: &[u8], offset: usize) -> u64 {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&wire[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}

/// Operation encoded in one canonical [`StorageBlockRequest`].
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageBlockOperation {
    Read = 1,
    Write = 2,
    Flush = 3,
}

impl StorageBlockOperation {
    pub const fn raw(self) -> u16 {
        self as u16
    }

    pub const fn from_raw(raw: u16) -> Option<Self> {
        match raw {
            1 => Some(Self::Read),
            2 => Some(Self::Write),
            3 => Some(Self::Flush),
            _ => None,
        }
    }
}

/// One validated bounded sector batch submitted through `StorageSubmit`.
///
/// Client submissions always carry token zero. After accepting a submission,
/// the kernel assigns a non-zero token before delivering the request to the
/// authenticated StorageServer. Read and flush data areas are all-zero; only a
/// write request carries caller-controlled sector data, with canonical zero
/// padding after its declared sector count.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageBlockRequest {
    operation: StorageBlockOperation,
    relative_lba: u64,
    token: u64,
    sector_count: u16,
    data: [u8; STORAGE_BLOCK_DATA_MAX_BYTES],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageBlockRequestError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    InvalidOperation,
    NonZeroReserved,
    InvalidSectorCount,
    RelativeLbaOutOfRange,
    NonCanonicalFlushLba,
    NonCanonicalFlushSectorCount,
    NonZeroReadData,
    NonZeroWritePadding,
    NonZeroFlushData,
    NonZeroSubmissionToken,
    MissingDeliveryToken,
    TokenAlreadyAssigned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StorageBlockTokenExpectation {
    Submission,
    Delivery,
}

impl StorageBlockRequest {
    pub fn read(relative_lba: u64) -> Result<Self, StorageBlockRequestError> {
        Self::read_batch(relative_lba, 1)
    }

    pub fn read_batch(
        relative_lba: u64,
        sector_count: usize,
    ) -> Result<Self, StorageBlockRequestError> {
        Self::new(
            StorageBlockOperation::Read,
            relative_lba,
            sector_count,
            [0; STORAGE_BLOCK_DATA_MAX_BYTES],
        )
    }

    pub fn write(
        relative_lba: u64,
        data: [u8; STORAGE_SECTOR_SIZE],
    ) -> Result<Self, StorageBlockRequestError> {
        Self::write_batch(relative_lba, core::slice::from_ref(&data))
    }

    pub fn write_batch(
        relative_lba: u64,
        sectors: &[[u8; STORAGE_SECTOR_SIZE]],
    ) -> Result<Self, StorageBlockRequestError> {
        if sectors.is_empty() || sectors.len() > STORAGE_BLOCK_MAX_SECTORS {
            return Err(StorageBlockRequestError::InvalidSectorCount);
        }
        let mut data = [0; STORAGE_BLOCK_DATA_MAX_BYTES];
        for (index, sector) in sectors.iter().enumerate() {
            let start = index * STORAGE_SECTOR_SIZE;
            data[start..start + STORAGE_SECTOR_SIZE].copy_from_slice(sector);
        }
        Self::new(
            StorageBlockOperation::Write,
            relative_lba,
            sectors.len(),
            data,
        )
    }

    pub fn flush() -> Self {
        Self {
            operation: StorageBlockOperation::Flush,
            relative_lba: 0,
            token: STORAGE_BLOCK_REQUEST_UNASSIGNED_TOKEN,
            sector_count: 0,
            data: [0; STORAGE_BLOCK_DATA_MAX_BYTES],
        }
    }

    fn new(
        operation: StorageBlockOperation,
        relative_lba: u64,
        sector_count: usize,
        data: [u8; STORAGE_BLOCK_DATA_MAX_BYTES],
    ) -> Result<Self, StorageBlockRequestError> {
        validate_storage_block_semantics(operation, relative_lba, sector_count, &data)?;
        Ok(Self {
            operation,
            relative_lba,
            token: STORAGE_BLOCK_REQUEST_UNASSIGNED_TOKEN,
            sector_count: sector_count as u16,
            data,
        })
    }

    pub const fn operation(&self) -> StorageBlockOperation {
        self.operation
    }

    pub const fn relative_lba(&self) -> u64 {
        self.relative_lba
    }

    /// Zero in a client submission and non-zero in a kernel delivery.
    pub const fn token(&self) -> u64 {
        self.token
    }

    pub const fn sector_count(&self) -> usize {
        self.sector_count as usize
    }

    pub const fn byte_len(&self) -> usize {
        self.sector_count as usize * STORAGE_SECTOR_SIZE
    }

    pub const fn data(&self) -> &[u8; STORAGE_BLOCK_DATA_MAX_BYTES] {
        &self.data
    }

    /// Assigns the token reserved for the kernel's accepted-request path.
    pub fn with_kernel_token(mut self, token: u64) -> Result<Self, StorageBlockRequestError> {
        if self.token != STORAGE_BLOCK_REQUEST_UNASSIGNED_TOKEN {
            return Err(StorageBlockRequestError::TokenAlreadyAssigned);
        }
        if token == STORAGE_BLOCK_REQUEST_UNASSIGNED_TOKEN {
            return Err(StorageBlockRequestError::MissingDeliveryToken);
        }
        self.token = token;
        Ok(self)
    }

    /// Encodes the only canonical 4160-byte representation.
    ///
    /// ```text
    /// 0   u32 magic = "SBRQ"
    /// 4   u16 version = 2
    /// 6   u16 operation (1 read, 2 write, 3 flush)
    /// 8   u64 AppData-volume-relative LBA (zero for flush)
    /// 16  u64 token (zero from client, non-zero after kernel assignment)
    /// 24  u16 sector count (1..=8 for read/write, zero for flush)
    /// 26  reserved[38] = 0
    /// 64  data[4096] (non-zero bytes permitted only for declared write data)
    /// ```
    pub fn encode(&self) -> [u8; STORAGE_BLOCK_REQUEST_WIRE_SIZE] {
        let mut wire = [0; STORAGE_BLOCK_REQUEST_WIRE_SIZE];
        write_app_data_u32(&mut wire, 0, STORAGE_BLOCK_REQUEST_MAGIC);
        write_app_data_u16(&mut wire, 4, STORAGE_BLOCK_REQUEST_VERSION);
        write_app_data_u16(&mut wire, 6, self.operation.raw());
        write_app_data_u64(&mut wire, 8, self.relative_lba);
        write_app_data_u64(&mut wire, 16, self.token);
        write_app_data_u16(&mut wire, 24, self.sector_count);
        wire[64..].copy_from_slice(&self.data);
        wire
    }

    /// Decodes a client-submitted request and rejects any caller-chosen token.
    pub fn decode(wire: &[u8]) -> Result<Self, StorageBlockRequestError> {
        Self::decode_submission(wire)
    }

    /// Decodes the exact wire accepted by `StorageSubmit`.
    pub fn decode_submission(wire: &[u8]) -> Result<Self, StorageBlockRequestError> {
        Self::decode_with_token_expectation(wire, StorageBlockTokenExpectation::Submission)
    }

    /// Decodes a request delivered by the kernel to the StorageServer.
    pub fn decode_delivery(wire: &[u8]) -> Result<Self, StorageBlockRequestError> {
        Self::decode_with_token_expectation(wire, StorageBlockTokenExpectation::Delivery)
    }

    fn decode_with_token_expectation(
        wire: &[u8],
        token_expectation: StorageBlockTokenExpectation,
    ) -> Result<Self, StorageBlockRequestError> {
        if wire.len() != STORAGE_BLOCK_REQUEST_WIRE_SIZE {
            return Err(StorageBlockRequestError::InvalidWireLength);
        }
        if read_app_data_u32(wire, 0) != STORAGE_BLOCK_REQUEST_MAGIC {
            return Err(StorageBlockRequestError::InvalidMagic);
        }
        if read_app_data_u16(wire, 4) != STORAGE_BLOCK_REQUEST_VERSION {
            return Err(StorageBlockRequestError::UnsupportedVersion);
        }
        let operation = StorageBlockOperation::from_raw(read_app_data_u16(wire, 6))
            .ok_or(StorageBlockRequestError::InvalidOperation)?;
        if wire[26..64].iter().any(|byte| *byte != 0) {
            return Err(StorageBlockRequestError::NonZeroReserved);
        }

        let token = read_app_data_u64(wire, 16);
        match token_expectation {
            StorageBlockTokenExpectation::Submission
                if token != STORAGE_BLOCK_REQUEST_UNASSIGNED_TOKEN =>
            {
                return Err(StorageBlockRequestError::NonZeroSubmissionToken);
            }
            StorageBlockTokenExpectation::Delivery
                if token == STORAGE_BLOCK_REQUEST_UNASSIGNED_TOKEN =>
            {
                return Err(StorageBlockRequestError::MissingDeliveryToken);
            }
            _ => {}
        }

        let relative_lba = read_app_data_u64(wire, 8);
        let sector_count = usize::from(read_app_data_u16(wire, 24));
        let mut data = [0; STORAGE_BLOCK_DATA_MAX_BYTES];
        data.copy_from_slice(&wire[64..]);
        validate_storage_block_semantics(operation, relative_lba, sector_count, &data)?;
        Ok(Self {
            operation,
            relative_lba,
            token,
            sector_count: sector_count as u16,
            data,
        })
    }
}

fn validate_storage_block_semantics(
    operation: StorageBlockOperation,
    relative_lba: u64,
    sector_count: usize,
    data: &[u8; STORAGE_BLOCK_DATA_MAX_BYTES],
) -> Result<(), StorageBlockRequestError> {
    match operation {
        StorageBlockOperation::Read => {
            validate_storage_block_range(relative_lba, sector_count)?;
            if data.iter().any(|byte| *byte != 0) {
                return Err(StorageBlockRequestError::NonZeroReadData);
            }
        }
        StorageBlockOperation::Write => {
            validate_storage_block_range(relative_lba, sector_count)?;
            if data[sector_count * STORAGE_SECTOR_SIZE..]
                .iter()
                .any(|byte| *byte != 0)
            {
                return Err(StorageBlockRequestError::NonZeroWritePadding);
            }
        }
        StorageBlockOperation::Flush => {
            if relative_lba != 0 {
                return Err(StorageBlockRequestError::NonCanonicalFlushLba);
            }
            if sector_count != 0 {
                return Err(StorageBlockRequestError::NonCanonicalFlushSectorCount);
            }
            if data.iter().any(|byte| *byte != 0) {
                return Err(StorageBlockRequestError::NonZeroFlushData);
            }
        }
    }
    Ok(())
}

fn validate_storage_block_range(
    relative_lba: u64,
    sector_count: usize,
) -> Result<(), StorageBlockRequestError> {
    if sector_count == 0 || sector_count > STORAGE_BLOCK_MAX_SECTORS {
        return Err(StorageBlockRequestError::InvalidSectorCount);
    }
    let end = relative_lba
        .checked_add(sector_count as u64)
        .ok_or(StorageBlockRequestError::RelativeLbaOutOfRange)?;
    if end > APP_DATA_VOLUME_SECTORS {
        return Err(StorageBlockRequestError::RelativeLbaOutOfRange);
    }
    Ok(())
}

/// One canonical client/session association returned by `StorageAccept`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageSessionBinding {
    session_id: u64,
    client_pid: u64,
    principal: AppDataPrincipal,
    granted_rights: Rights,
    server_epoch: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageSessionBindingError {
    InvalidWireLength,
    InvalidMagic,
    UnsupportedVersion,
    NonZeroReserved,
    InvalidSessionId,
    InvalidClientPid,
    InvalidPrincipal,
    InvalidGrantedRights,
    InvalidServerEpoch,
}

impl StorageSessionBinding {
    pub fn new(
        session_id: u64,
        client_pid: u64,
        principal: AppDataPrincipal,
        granted_rights: Rights,
        server_epoch: u64,
    ) -> Result<Self, StorageSessionBindingError> {
        if session_id == 0 {
            return Err(StorageSessionBindingError::InvalidSessionId);
        }
        if client_pid == 0 {
            return Err(StorageSessionBindingError::InvalidClientPid);
        }
        if principal.raw() == 0 {
            return Err(StorageSessionBindingError::InvalidPrincipal);
        }
        if !storage_binding_rights_are_valid(granted_rights) {
            return Err(StorageSessionBindingError::InvalidGrantedRights);
        }
        if server_epoch == 0 {
            return Err(StorageSessionBindingError::InvalidServerEpoch);
        }
        Ok(Self {
            session_id,
            client_pid,
            principal,
            granted_rights,
            server_epoch,
        })
    }

    pub const fn session_id(&self) -> u64 {
        self.session_id
    }

    pub const fn client_pid(&self) -> u64 {
        self.client_pid
    }

    pub const fn principal(&self) -> AppDataPrincipal {
        self.principal
    }

    /// Read/write authority requested by the client; wait is implicit in the
    /// returned session capability and is not encoded in this field.
    pub const fn granted_rights(&self) -> Rights {
        self.granted_rights
    }

    pub const fn server_epoch(&self) -> u64 {
        self.server_epoch
    }

    /// Encodes the only canonical 64-byte binding representation.
    ///
    /// ```text
    /// 0   u32 magic = "SSBN"
    /// 4   u16 version = 1
    /// 6   reserved[2] = 0
    /// 8   u64 session ID (non-zero)
    /// 16  u64 client PID (non-zero)
    /// 24  u64 AppData principal (non-zero)
    /// 32  u32 granted READ/WRITE rights (non-empty subset)
    /// 36  reserved[4] = 0
    /// 40  u64 StorageServer epoch (non-zero)
    /// 48  reserved[16] = 0
    /// ```
    pub fn encode(&self) -> [u8; STORAGE_SESSION_BINDING_WIRE_SIZE] {
        let mut wire = [0; STORAGE_SESSION_BINDING_WIRE_SIZE];
        write_app_data_u32(&mut wire, 0, STORAGE_SESSION_BINDING_MAGIC);
        write_app_data_u16(&mut wire, 4, STORAGE_SESSION_BINDING_VERSION);
        write_app_data_u64(&mut wire, 8, self.session_id);
        write_app_data_u64(&mut wire, 16, self.client_pid);
        write_app_data_u64(&mut wire, 24, self.principal.raw());
        write_app_data_u32(&mut wire, 32, self.granted_rights.bits());
        write_app_data_u64(&mut wire, 40, self.server_epoch);
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, StorageSessionBindingError> {
        if wire.len() != STORAGE_SESSION_BINDING_WIRE_SIZE {
            return Err(StorageSessionBindingError::InvalidWireLength);
        }
        if read_app_data_u32(wire, 0) != STORAGE_SESSION_BINDING_MAGIC {
            return Err(StorageSessionBindingError::InvalidMagic);
        }
        if read_app_data_u16(wire, 4) != STORAGE_SESSION_BINDING_VERSION {
            return Err(StorageSessionBindingError::UnsupportedVersion);
        }
        if wire[6..8].iter().any(|byte| *byte != 0)
            || wire[36..40].iter().any(|byte| *byte != 0)
            || wire[48..64].iter().any(|byte| *byte != 0)
        {
            return Err(StorageSessionBindingError::NonZeroReserved);
        }

        let principal = AppDataPrincipal::from_raw(read_app_data_u64(wire, 24))
            .ok_or(StorageSessionBindingError::InvalidPrincipal)?;
        let granted_rights = Rights::from_bits(read_app_data_u32(wire, 32))
            .ok_or(StorageSessionBindingError::InvalidGrantedRights)?;
        Self::new(
            read_app_data_u64(wire, 8),
            read_app_data_u64(wire, 16),
            principal,
            granted_rights,
            read_app_data_u64(wire, 40),
        )
    }
}

fn storage_binding_rights_are_valid(rights: Rights) -> bool {
    let bits = rights.bits();
    bits != 0 && bits & !STORAGE_CONNECT_RIGHTS_MASK == 0
}

#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelMessageKind {
    Scalar = 1,
    Bytes = 2,
    Transfer = 3,
}

impl ChannelMessageKind {
    pub const fn raw(self) -> u64 {
        self as u64
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::Scalar),
            2 => Some(Self::Bytes),
            3 => Some(Self::Transfer),
            _ => None,
        }
    }
}

/// A canonical, fixed-size representation returned by `ChannelReadEnvelope`.
///
/// The wire layout consists of six little-endian `u64` fields followed by 64
/// data bytes:
///
/// ```text
/// 0   sender_pid
/// 8   message kind
/// 16  logical length
/// 24  received handle (low 32 bits, otherwise zero)
/// 32  scalar tag
/// 40  scalar payload
/// 48  data[64]
/// ```
///
/// Constructors and decoding enforce one canonical representation per kind.
/// This keeps reserved/cross-kind fields from becoming a second covert wire
/// format and lets user space reject malformed envelopes without interpreting
/// uninitialized padding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChannelReadEnvelope {
    sender_pid: u64,
    kind: ChannelMessageKind,
    logical_length: u64,
    received_handle: HandleValue,
    tag: u64,
    payload: u64,
    data: [u8; CHANNEL_MESSAGE_MAX_BYTES],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelReadEnvelopeError {
    InvalidWireLength,
    InvalidSenderPid,
    UnknownKind,
    InvalidLogicalLength,
    InvalidReceivedHandle,
    UnexpectedReceivedHandle,
    UnexpectedScalarFields,
    NonZeroScalarData,
    NonZeroDataTail,
}

impl ChannelReadEnvelope {
    pub fn scalar(
        sender_pid: u64,
        tag: u64,
        payload: u64,
    ) -> Result<Self, ChannelReadEnvelopeError> {
        Self::validated(
            sender_pid,
            ChannelMessageKind::Scalar,
            16,
            HandleValue::INVALID,
            tag,
            payload,
            [0; CHANNEL_MESSAGE_MAX_BYTES],
        )
    }

    pub fn bytes(
        sender_pid: u64,
        data: [u8; CHANNEL_MESSAGE_MAX_BYTES],
        logical_length: usize,
    ) -> Result<Self, ChannelReadEnvelopeError> {
        let logical_length = u64::try_from(logical_length)
            .map_err(|_| ChannelReadEnvelopeError::InvalidLogicalLength)?;
        Self::validated(
            sender_pid,
            ChannelMessageKind::Bytes,
            logical_length,
            HandleValue::INVALID,
            0,
            0,
            data,
        )
    }

    pub fn transfer(
        sender_pid: u64,
        received_handle: HandleValue,
        data: [u8; CHANNEL_MESSAGE_MAX_BYTES],
        logical_length: usize,
    ) -> Result<Self, ChannelReadEnvelopeError> {
        let logical_length = u64::try_from(logical_length)
            .map_err(|_| ChannelReadEnvelopeError::InvalidLogicalLength)?;
        Self::validated(
            sender_pid,
            ChannelMessageKind::Transfer,
            logical_length,
            received_handle,
            0,
            0,
            data,
        )
    }

    pub const fn sender_pid(&self) -> u64 {
        self.sender_pid
    }

    pub const fn kind(&self) -> ChannelMessageKind {
        self.kind
    }

    pub const fn logical_length(&self) -> usize {
        self.logical_length as usize
    }

    pub const fn received_handle(&self) -> HandleValue {
        self.received_handle
    }

    pub const fn tag(&self) -> u64 {
        self.tag
    }

    pub const fn payload(&self) -> u64 {
        self.payload
    }

    pub const fn scalar_values(&self) -> Option<(u64, u64)> {
        match self.kind {
            ChannelMessageKind::Scalar => Some((self.tag, self.payload)),
            ChannelMessageKind::Bytes | ChannelMessageKind::Transfer => None,
        }
    }

    pub const fn data(&self) -> &[u8; CHANNEL_MESSAGE_MAX_BYTES] {
        &self.data
    }

    pub fn payload_bytes(&self) -> Option<&[u8]> {
        match self.kind {
            ChannelMessageKind::Scalar => None,
            ChannelMessageKind::Bytes | ChannelMessageKind::Transfer => {
                Some(&self.data[..self.logical_length()])
            }
        }
    }

    pub fn encode(&self) -> [u8; CHANNEL_READ_ENVELOPE_SIZE] {
        let mut wire = [0; CHANNEL_READ_ENVELOPE_SIZE];
        write_envelope_u64(&mut wire, 0, self.sender_pid);
        write_envelope_u64(&mut wire, 8, self.kind.raw());
        write_envelope_u64(&mut wire, 16, self.logical_length);
        write_envelope_u64(&mut wire, 24, u64::from(self.received_handle.raw()));
        write_envelope_u64(&mut wire, 32, self.tag);
        write_envelope_u64(&mut wire, 40, self.payload);
        wire[48..].copy_from_slice(&self.data);
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, ChannelReadEnvelopeError> {
        if wire.len() != CHANNEL_READ_ENVELOPE_SIZE {
            return Err(ChannelReadEnvelopeError::InvalidWireLength);
        }
        let sender_pid = read_envelope_u64(wire, 0);
        let kind = ChannelMessageKind::from_raw(read_envelope_u64(wire, 8))
            .ok_or(ChannelReadEnvelopeError::UnknownKind)?;
        let logical_length = read_envelope_u64(wire, 16);
        let raw_handle = read_envelope_u64(wire, 24);
        let received_handle = u32::try_from(raw_handle)
            .map(HandleValue::from_raw)
            .map_err(|_| ChannelReadEnvelopeError::InvalidReceivedHandle)?;
        let tag = read_envelope_u64(wire, 32);
        let payload = read_envelope_u64(wire, 40);
        let mut data = [0; CHANNEL_MESSAGE_MAX_BYTES];
        data.copy_from_slice(&wire[48..]);
        Self::validated(
            sender_pid,
            kind,
            logical_length,
            received_handle,
            tag,
            payload,
            data,
        )
    }

    fn validated(
        sender_pid: u64,
        kind: ChannelMessageKind,
        logical_length: u64,
        received_handle: HandleValue,
        tag: u64,
        payload: u64,
        data: [u8; CHANNEL_MESSAGE_MAX_BYTES],
    ) -> Result<Self, ChannelReadEnvelopeError> {
        if sender_pid == 0 {
            return Err(ChannelReadEnvelopeError::InvalidSenderPid);
        }
        match kind {
            ChannelMessageKind::Scalar => {
                if logical_length != 16 {
                    return Err(ChannelReadEnvelopeError::InvalidLogicalLength);
                }
                if received_handle.is_valid() {
                    return Err(ChannelReadEnvelopeError::UnexpectedReceivedHandle);
                }
                if data.iter().any(|byte| *byte != 0) {
                    return Err(ChannelReadEnvelopeError::NonZeroScalarData);
                }
            }
            ChannelMessageKind::Bytes | ChannelMessageKind::Transfer => {
                let logical_length = usize::try_from(logical_length)
                    .ok()
                    .filter(|length| *length <= CHANNEL_MESSAGE_MAX_BYTES)
                    .ok_or(ChannelReadEnvelopeError::InvalidLogicalLength)?;
                if tag != 0 || payload != 0 {
                    return Err(ChannelReadEnvelopeError::UnexpectedScalarFields);
                }
                if matches!(kind, ChannelMessageKind::Bytes) && received_handle.is_valid() {
                    return Err(ChannelReadEnvelopeError::UnexpectedReceivedHandle);
                }
                if matches!(kind, ChannelMessageKind::Transfer) && !received_handle.is_valid() {
                    return Err(ChannelReadEnvelopeError::InvalidReceivedHandle);
                }
                if data[logical_length..].iter().any(|byte| *byte != 0) {
                    return Err(ChannelReadEnvelopeError::NonZeroDataTail);
                }
            }
        }
        Ok(Self {
            sender_pid,
            kind,
            logical_length,
            received_handle,
            tag,
            payload,
            data,
        })
    }
}

fn write_envelope_u64(wire: &mut [u8; CHANNEL_READ_ENVELOPE_SIZE], offset: usize, value: u64) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_envelope_u64(wire: &[u8], offset: usize) -> u64 {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&wire[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}

/// Node type carried by ABI 49's allocation-free Scene-RPC-2 protocol.
#[cfg(feature = "androidbox-scene-rpc2")]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidAppSceneNodeKind {
    LinearLayout = 1,
    TextView = 2,
    Button = 3,
}

#[cfg(feature = "androidbox-scene-rpc2")]
impl AndroidAppSceneNodeKind {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::LinearLayout),
            2 => Some(Self::TextView),
            3 => Some(Self::Button),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

/// One canonical Android layout dimension admitted by Scene-RPC-2.
#[cfg(feature = "androidbox-scene-rpc2")]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidAppSceneDimension {
    MatchParent = 1,
    WrapContent = 2,
    #[cfg(feature = "androidbox-layout-weight15")]
    Zero = 3,
    #[cfg(feature = "androidbox-layout-size18")]
    Exact = 4,
}

#[cfg(feature = "androidbox-scene-rpc2")]
impl AndroidAppSceneDimension {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::MatchParent),
            2 => Some(Self::WrapContent),
            #[cfg(feature = "androidbox-layout-weight15")]
            3 => Some(Self::Zero),
            #[cfg(feature = "androidbox-layout-size18")]
            4 => Some(Self::Exact),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

/// One canonical orientation admitted by Scene-RPC-2.
#[cfg(feature = "androidbox-scene-rpc2")]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidAppSceneOrientation {
    None = 0,
    Vertical = 1,
    #[cfg(feature = "androidbox-layout-row14")]
    Horizontal = 2,
}

#[cfg(feature = "androidbox-scene-rpc2")]
impl AndroidAppSceneOrientation {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::None),
            1 => Some(Self::Vertical),
            #[cfg(feature = "androidbox-layout-row14")]
            2 => Some(Self::Horizontal),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }

    const fn is_linear_layout(self) -> bool {
        match self {
            Self::Vertical => true,
            #[cfg(feature = "androidbox-layout-row14")]
            Self::Horizontal => true,
            Self::None => false,
        }
    }
}

/// Allocation-free canonical descriptor for one ABI 49+ Scene-RPC node.
///
/// Its ABI 49..66 encoding is 16 bytes; ABI 67+ uses 24 bytes:
///
/// ```text
/// byte 0      descriptor version
/// byte 1      node kind
/// byte 2      parent index, or 0xff for no parent
/// byte 3      width
/// byte 4      height
/// byte 5      orientation
/// byte 6      callback flag (0 or 1)
/// byte 7      text length
/// bytes 8..12 little-endian Android view ID
/// byte 12     bounded integer layout weight in ABI 65+, otherwise zero
/// byte 13     uniform `layout_margin` dp in ABI 66, otherwise zero
/// byte 14     uniform `padding` dp in ABI 66, otherwise zero
/// byte 15     zero through ABI 66
/// bytes 13..17 ABI 67 margin left/top/right/bottom
/// bytes 17..21 ABI 67 padding left/top/right/bottom
/// bytes 21..23 ABI 68 exact width/height dp, otherwise zero
/// byte 23      ABI 67+ reserved zero
/// ```
///
/// Index zero is the sole root and has no parent. Every later node has one
/// strictly earlier parent. Tree-position validation is performed by
/// [`Self::is_canonical_for_index`] because the index belongs to the enclosing
/// `Node` frame.
#[cfg(feature = "androidbox-scene-rpc2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidAppSceneNodeDescriptor {
    kind: AndroidAppSceneNodeKind,
    parent: Option<u8>,
    id: u32,
    width: AndroidAppSceneDimension,
    height: AndroidAppSceneDimension,
    orientation: AndroidAppSceneOrientation,
    callback: bool,
    text_len: u8,
    layout_weight: u8,
    layout_margin_left_dp: u8,
    layout_margin_top_dp: u8,
    layout_margin_right_dp: u8,
    layout_margin_bottom_dp: u8,
    padding_left_dp: u8,
    padding_top_dp: u8,
    padding_right_dp: u8,
    padding_bottom_dp: u8,
    exact_width_dp: u8,
    exact_height_dp: u8,
}

#[cfg(feature = "androidbox-scene-rpc2")]
impl AndroidAppSceneNodeDescriptor {
    #[cfg(feature = "androidbox-layout-size18")]
    const VERSION: u8 = 5;
    #[cfg(all(
        feature = "androidbox-layout-directional17",
        not(feature = "androidbox-layout-size18")
    ))]
    const VERSION: u8 = 4;
    #[cfg(all(
        feature = "androidbox-layout-spacing16",
        not(feature = "androidbox-layout-directional17")
    ))]
    const VERSION: u8 = 3;
    #[cfg(all(
        feature = "androidbox-layout-weight15",
        not(feature = "androidbox-layout-spacing16")
    ))]
    const VERSION: u8 = 2;
    #[cfg(not(feature = "androidbox-layout-weight15"))]
    const VERSION: u8 = 1;
    const NO_PARENT: u8 = u8::MAX;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: AndroidAppSceneNodeKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidAppSceneDimension,
        height: AndroidAppSceneDimension,
        orientation: AndroidAppSceneOrientation,
        callback: bool,
        text_len: u8,
    ) -> Option<Self> {
        Self::new_with_layout(
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            callback,
            text_len,
            0,
            0,
            0,
        )
    }

    #[cfg(feature = "androidbox-layout-weight15")]
    #[allow(clippy::too_many_arguments)]
    pub fn new_weighted(
        kind: AndroidAppSceneNodeKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidAppSceneDimension,
        height: AndroidAppSceneDimension,
        orientation: AndroidAppSceneOrientation,
        callback: bool,
        text_len: u8,
        layout_weight: u8,
    ) -> Option<Self> {
        Self::new_with_layout(
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            callback,
            text_len,
            layout_weight,
            0,
            0,
        )
    }

    #[cfg(feature = "androidbox-layout-spacing16")]
    #[allow(clippy::too_many_arguments)]
    pub fn new_spaced(
        kind: AndroidAppSceneNodeKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidAppSceneDimension,
        height: AndroidAppSceneDimension,
        orientation: AndroidAppSceneOrientation,
        callback: bool,
        text_len: u8,
        layout_weight: u8,
        layout_margin_dp: u8,
        padding_dp: u8,
    ) -> Option<Self> {
        Self::new_with_layout(
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            callback,
            text_len,
            layout_weight,
            layout_margin_dp,
            padding_dp,
        )
    }

    #[cfg(feature = "androidbox-layout-directional17")]
    #[allow(clippy::too_many_arguments)]
    pub fn new_directional(
        kind: AndroidAppSceneNodeKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidAppSceneDimension,
        height: AndroidAppSceneDimension,
        orientation: AndroidAppSceneOrientation,
        callback: bool,
        text_len: u8,
        layout_weight: u8,
        layout_margin_left_dp: u8,
        layout_margin_top_dp: u8,
        layout_margin_right_dp: u8,
        layout_margin_bottom_dp: u8,
        padding_left_dp: u8,
        padding_top_dp: u8,
        padding_right_dp: u8,
        padding_bottom_dp: u8,
    ) -> Option<Self> {
        Self::new_with_directional_layout(
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            callback,
            text_len,
            layout_weight,
            layout_margin_left_dp,
            layout_margin_top_dp,
            layout_margin_right_dp,
            layout_margin_bottom_dp,
            padding_left_dp,
            padding_top_dp,
            padding_right_dp,
            padding_bottom_dp,
            0,
            0,
        )
    }

    #[cfg(feature = "androidbox-layout-size18")]
    #[allow(clippy::too_many_arguments)]
    pub fn new_sized(
        kind: AndroidAppSceneNodeKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidAppSceneDimension,
        height: AndroidAppSceneDimension,
        orientation: AndroidAppSceneOrientation,
        callback: bool,
        text_len: u8,
        layout_weight: u8,
        layout_margin_left_dp: u8,
        layout_margin_top_dp: u8,
        layout_margin_right_dp: u8,
        layout_margin_bottom_dp: u8,
        padding_left_dp: u8,
        padding_top_dp: u8,
        padding_right_dp: u8,
        padding_bottom_dp: u8,
        exact_width_dp: u8,
        exact_height_dp: u8,
    ) -> Option<Self> {
        Self::new_with_directional_layout(
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            callback,
            text_len,
            layout_weight,
            layout_margin_left_dp,
            layout_margin_top_dp,
            layout_margin_right_dp,
            layout_margin_bottom_dp,
            padding_left_dp,
            padding_top_dp,
            padding_right_dp,
            padding_bottom_dp,
            exact_width_dp,
            exact_height_dp,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_layout(
        kind: AndroidAppSceneNodeKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidAppSceneDimension,
        height: AndroidAppSceneDimension,
        orientation: AndroidAppSceneOrientation,
        callback: bool,
        text_len: u8,
        layout_weight: u8,
        layout_margin_dp: u8,
        padding_dp: u8,
    ) -> Option<Self> {
        Self::new_with_directional_layout(
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            callback,
            text_len,
            layout_weight,
            layout_margin_dp,
            layout_margin_dp,
            layout_margin_dp,
            layout_margin_dp,
            padding_dp,
            padding_dp,
            padding_dp,
            padding_dp,
            0,
            0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_directional_layout(
        kind: AndroidAppSceneNodeKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidAppSceneDimension,
        height: AndroidAppSceneDimension,
        orientation: AndroidAppSceneOrientation,
        callback: bool,
        text_len: u8,
        layout_weight: u8,
        layout_margin_left_dp: u8,
        layout_margin_top_dp: u8,
        layout_margin_right_dp: u8,
        layout_margin_bottom_dp: u8,
        padding_left_dp: u8,
        padding_top_dp: u8,
        padding_right_dp: u8,
        padding_bottom_dp: u8,
        exact_width_dp: u8,
        exact_height_dp: u8,
    ) -> Option<Self> {
        let value = Self {
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            callback,
            text_len,
            layout_weight,
            layout_margin_left_dp,
            layout_margin_top_dp,
            layout_margin_right_dp,
            layout_margin_bottom_dp,
            padding_left_dp,
            padding_top_dp,
            padding_right_dp,
            padding_bottom_dp,
            exact_width_dp,
            exact_height_dp,
        };
        value.is_intrinsically_canonical().then_some(value)
    }

    pub const fn kind(self) -> AndroidAppSceneNodeKind {
        self.kind
    }

    pub const fn parent(self) -> Option<u8> {
        self.parent
    }

    pub const fn id(self) -> u32 {
        self.id
    }

    pub const fn width(self) -> AndroidAppSceneDimension {
        self.width
    }

    pub const fn height(self) -> AndroidAppSceneDimension {
        self.height
    }

    pub const fn orientation(self) -> AndroidAppSceneOrientation {
        self.orientation
    }

    pub const fn callback(self) -> bool {
        self.callback
    }

    pub const fn text_len(self) -> u8 {
        self.text_len
    }

    pub const fn layout_weight(self) -> u8 {
        self.layout_weight
    }

    pub const fn layout_margin_dp(self) -> u8 {
        self.layout_margin_left_dp
    }

    pub const fn padding_dp(self) -> u8 {
        self.padding_left_dp
    }

    pub const fn layout_margin_left_dp(self) -> u8 {
        self.layout_margin_left_dp
    }

    pub const fn layout_margin_top_dp(self) -> u8 {
        self.layout_margin_top_dp
    }

    pub const fn layout_margin_right_dp(self) -> u8 {
        self.layout_margin_right_dp
    }

    pub const fn layout_margin_bottom_dp(self) -> u8 {
        self.layout_margin_bottom_dp
    }

    pub const fn padding_left_dp(self) -> u8 {
        self.padding_left_dp
    }

    pub const fn padding_top_dp(self) -> u8 {
        self.padding_top_dp
    }

    pub const fn padding_right_dp(self) -> u8 {
        self.padding_right_dp
    }

    pub const fn padding_bottom_dp(self) -> u8 {
        self.padding_bottom_dp
    }

    pub const fn exact_width_dp(self) -> u8 {
        self.exact_width_dp
    }

    pub const fn exact_height_dp(self) -> u8 {
        self.exact_height_dp
    }

    pub fn encode(self) -> [u8; ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE] {
        let mut wire = [0; ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE];
        wire[0] = Self::VERSION;
        wire[1] = self.kind.raw();
        wire[2] = self.parent.unwrap_or(Self::NO_PARENT);
        wire[3] = self.width.raw();
        wire[4] = self.height.raw();
        wire[5] = self.orientation.raw();
        wire[6] = u8::from(self.callback);
        wire[7] = self.text_len;
        wire[8..12].copy_from_slice(&self.id.to_le_bytes());
        wire[12] = self.layout_weight;
        #[cfg(feature = "androidbox-layout-directional17")]
        {
            wire[13] = self.layout_margin_left_dp;
            wire[14] = self.layout_margin_top_dp;
            wire[15] = self.layout_margin_right_dp;
            wire[16] = self.layout_margin_bottom_dp;
            wire[17] = self.padding_left_dp;
            wire[18] = self.padding_top_dp;
            wire[19] = self.padding_right_dp;
            wire[20] = self.padding_bottom_dp;
            #[cfg(feature = "androidbox-layout-size18")]
            {
                wire[21] = self.exact_width_dp;
                wire[22] = self.exact_height_dp;
            }
        }
        #[cfg(not(feature = "androidbox-layout-directional17"))]
        {
            wire[13] = self.layout_margin_left_dp;
            wire[14] = self.padding_left_dp;
        }
        wire
    }

    pub fn decode(wire: &[u8; ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE]) -> Option<Self> {
        #[cfg(feature = "androidbox-layout-size18")]
        let reserved = &wire[23..];
        #[cfg(all(
            feature = "androidbox-layout-directional17",
            not(feature = "androidbox-layout-size18")
        ))]
        let reserved = &wire[21..];
        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        let reserved = &wire[15..];
        #[cfg(all(
            feature = "androidbox-layout-weight15",
            not(feature = "androidbox-layout-spacing16")
        ))]
        let reserved = &wire[13..];
        #[cfg(not(feature = "androidbox-layout-weight15"))]
        let reserved = &wire[12..];
        if wire[0] != Self::VERSION || reserved.iter().any(|byte| *byte != 0) {
            return None;
        }
        let parent = match wire[2] {
            Self::NO_PARENT => None,
            parent if usize::from(parent) < ANDROID_APP_SCENE_MAX_NODES => Some(parent),
            _ => return None,
        };
        let callback = match wire[6] {
            0 => false,
            1 => true,
            _ => return None,
        };
        let mut id = [0; 4];
        id.copy_from_slice(&wire[8..12]);
        #[cfg(feature = "androidbox-layout-directional17")]
        {
            Self::new_with_directional_layout(
                AndroidAppSceneNodeKind::from_raw(wire[1])?,
                parent,
                u32::from_le_bytes(id),
                AndroidAppSceneDimension::from_raw(wire[3])?,
                AndroidAppSceneDimension::from_raw(wire[4])?,
                AndroidAppSceneOrientation::from_raw(wire[5])?,
                callback,
                wire[7],
                wire[12],
                wire[13],
                wire[14],
                wire[15],
                wire[16],
                wire[17],
                wire[18],
                wire[19],
                wire[20],
                {
                    #[cfg(feature = "androidbox-layout-size18")]
                    {
                        wire[21]
                    }
                    #[cfg(not(feature = "androidbox-layout-size18"))]
                    {
                        0
                    }
                },
                {
                    #[cfg(feature = "androidbox-layout-size18")]
                    {
                        wire[22]
                    }
                    #[cfg(not(feature = "androidbox-layout-size18"))]
                    {
                        0
                    }
                },
            )
        }
        #[cfg(not(feature = "androidbox-layout-directional17"))]
        {
            Self::new_with_layout(
                AndroidAppSceneNodeKind::from_raw(wire[1])?,
                parent,
                u32::from_le_bytes(id),
                AndroidAppSceneDimension::from_raw(wire[3])?,
                AndroidAppSceneDimension::from_raw(wire[4])?,
                AndroidAppSceneOrientation::from_raw(wire[5])?,
                callback,
                wire[7],
                wire[12],
                wire[13],
                wire[14],
            )
        }
    }

    pub fn is_canonical_for_index(self, index: u8) -> bool {
        if usize::from(index) >= ANDROID_APP_SCENE_MAX_NODES {
            return false;
        }
        let canonical_parent = match (index, self.parent) {
            (0, None) => true,
            (0, Some(_)) | (_, None) => false,
            (_, Some(parent)) => parent < index,
        };
        canonical_parent
            && !(index == 0
                && self.kind == AndroidAppSceneNodeKind::LinearLayout
                && self.orientation != AndroidAppSceneOrientation::Vertical)
            && self.is_intrinsically_canonical()
    }

    fn is_intrinsically_canonical(self) -> bool {
        if self
            .parent
            .is_some_and(|parent| usize::from(parent) >= ANDROID_APP_SCENE_MAX_NODES)
        {
            return false;
        }
        #[cfg(feature = "androidbox-layout-weight15")]
        let weight_is_canonical = match self.layout_weight {
            0 => {
                self.width != AndroidAppSceneDimension::Zero
                    && self.height != AndroidAppSceneDimension::Zero
            }
            1..=8 => {
                self.kind == AndroidAppSceneNodeKind::Button
                    && self.width == AndroidAppSceneDimension::Zero
                    && {
                        #[cfg(feature = "androidbox-layout-size18")]
                        {
                            matches!(
                                self.height,
                                AndroidAppSceneDimension::WrapContent
                                    | AndroidAppSceneDimension::Exact
                            )
                        }
                        #[cfg(not(feature = "androidbox-layout-size18"))]
                        {
                            self.height == AndroidAppSceneDimension::WrapContent
                        }
                    }
            }
            _ => false,
        };
        #[cfg(not(feature = "androidbox-layout-weight15"))]
        let weight_is_canonical = self.layout_weight == 0;
        if !weight_is_canonical {
            return false;
        }
        #[cfg(feature = "androidbox-layout-size18")]
        let exact_size_is_canonical = (self.width == AndroidAppSceneDimension::Exact)
            == (self.exact_width_dp != 0)
            && (self.height == AndroidAppSceneDimension::Exact) == (self.exact_height_dp != 0);
        #[cfg(not(feature = "androidbox-layout-size18"))]
        let exact_size_is_canonical = self.exact_width_dp == 0 && self.exact_height_dp == 0;
        if !exact_size_is_canonical {
            return false;
        }
        #[cfg(feature = "androidbox-layout-spacing16")]
        let spacing_is_canonical = [
            self.layout_margin_left_dp,
            self.layout_margin_top_dp,
            self.layout_margin_right_dp,
            self.layout_margin_bottom_dp,
            self.padding_left_dp,
            self.padding_top_dp,
            self.padding_right_dp,
            self.padding_bottom_dp,
        ]
        .iter()
        .all(|value| *value <= 16)
            && ((self.layout_margin_left_dp
                | self.layout_margin_top_dp
                | self.layout_margin_right_dp
                | self.layout_margin_bottom_dp)
                == 0
                || self.kind == AndroidAppSceneNodeKind::Button)
            && ((self.padding_left_dp
                | self.padding_top_dp
                | self.padding_right_dp
                | self.padding_bottom_dp)
                == 0
                || self.kind == AndroidAppSceneNodeKind::LinearLayout)
            && {
                #[cfg(feature = "androidbox-layout-directional17")]
                {
                    true
                }
                #[cfg(not(feature = "androidbox-layout-directional17"))]
                {
                    self.layout_margin_left_dp == self.layout_margin_top_dp
                        && self.layout_margin_left_dp == self.layout_margin_right_dp
                        && self.layout_margin_left_dp == self.layout_margin_bottom_dp
                        && self.padding_left_dp == self.padding_top_dp
                        && self.padding_left_dp == self.padding_right_dp
                        && self.padding_left_dp == self.padding_bottom_dp
                }
            };
        #[cfg(not(feature = "androidbox-layout-spacing16"))]
        let spacing_is_canonical = (self.layout_margin_left_dp
            | self.layout_margin_top_dp
            | self.layout_margin_right_dp
            | self.layout_margin_bottom_dp
            | self.padding_left_dp
            | self.padding_top_dp
            | self.padding_right_dp
            | self.padding_bottom_dp)
            == 0;
        if !spacing_is_canonical {
            return false;
        }
        match self.kind {
            AndroidAppSceneNodeKind::LinearLayout => {
                self.orientation.is_linear_layout() && !self.callback && self.text_len == 0
            }
            AndroidAppSceneNodeKind::TextView => {
                self.id != 0
                    && self.orientation == AndroidAppSceneOrientation::None
                    && !self.callback
                    && (1..=ANDROID_APP_SCENE_NODE_TEXT_MAX_BYTES)
                        .contains(&usize::from(self.text_len))
            }
            AndroidAppSceneNodeKind::Button => {
                self.id != 0
                    && self.orientation == AndroidAppSceneOrientation::None
                    && (1..=ANDROID_APP_BUTTON_MAX_BYTES).contains(&usize::from(self.text_len))
            }
        }
    }
}

/// Directional operation carried by ABI 47+'s private App ↔ AndroidApp
/// channel. The channel's kernel-stamped sender PID remains the authority;
/// this field only defines the bounded payload shape.
#[cfg(feature = "androidbox-process0")]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidAppMessageKind {
    Ready = 1,
    Open = 2,
    Opened = 3,
    LabelChunk = 4,
    ButtonChunk = 5,
    Click = 6,
    Updated = 7,
    UpdateTextChunk = 8,
    Close = 9,
    Closed = 10,
    Error = 11,
    #[cfg(feature = "androidbox-restart0")]
    Crash = 12,
    #[cfg(feature = "androidbox-scene-rpc2")]
    SceneOpened = 13,
    #[cfg(feature = "androidbox-scene-rpc2")]
    DescribeNode = 14,
    #[cfg(feature = "androidbox-scene-rpc2")]
    Node = 15,
    #[cfg(feature = "androidbox-scene-rpc2")]
    NodeTextChunk = 16,
}

#[cfg(feature = "androidbox-process0")]
impl AndroidAppMessageKind {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Ready),
            2 => Some(Self::Open),
            3 => Some(Self::Opened),
            4 => Some(Self::LabelChunk),
            5 => Some(Self::ButtonChunk),
            6 => Some(Self::Click),
            7 => Some(Self::Updated),
            8 => Some(Self::UpdateTextChunk),
            9 => Some(Self::Close),
            10 => Some(Self::Closed),
            11 => Some(Self::Error),
            #[cfg(feature = "androidbox-restart0")]
            12 => Some(Self::Crash),
            #[cfg(feature = "androidbox-scene-rpc2")]
            13 => Some(Self::SceneOpened),
            #[cfg(feature = "androidbox-scene-rpc2")]
            14 => Some(Self::DescribeNode),
            #[cfg(feature = "androidbox-scene-rpc2")]
            15 => Some(Self::Node),
            #[cfg(feature = "androidbox-scene-rpc2")]
            16 => Some(Self::NodeTextChunk),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

#[cfg(feature = "androidbox-process0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidAppMessageError {
    InvalidMagic,
    InvalidVersion,
    InvalidKind,
    InvalidPayloadLength,
    NonCanonical,
}

/// One canonical fixed-size ABI 47+ App ↔ AndroidApp protocol frame.
///
/// `arg0` and `arg1` have kind-specific meanings. Text chunks encode the
/// byte offset in `arg0[63:32]` and total length in `arg0[31:0]`. Every byte
/// beyond `payload_len` is zero and publisher text is printable ASCII.
#[cfg(feature = "androidbox-process0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidAppMessage {
    kind: AndroidAppMessageKind,
    request_id: u64,
    arg0: u64,
    arg1: u64,
    payload: [u8; ANDROID_APP_MESSAGE_PAYLOAD_BYTES],
    payload_len: u8,
}

#[cfg(feature = "androidbox-process0")]
impl AndroidAppMessage {
    #[cfg(feature = "androidbox-layout-size18")]
    const MAGIC: [u8; 8] = *b"BNDAPC14";
    #[cfg(all(
        feature = "androidbox-layout-directional17",
        not(feature = "androidbox-layout-size18")
    ))]
    const MAGIC: [u8; 8] = *b"BNDAPC13";
    #[cfg(all(
        feature = "androidbox-layout-spacing16",
        not(feature = "androidbox-layout-directional17")
    ))]
    const MAGIC: [u8; 8] = *b"BNDAPC12";
    #[cfg(all(
        feature = "androidbox-layout-weight15",
        not(feature = "androidbox-layout-spacing16")
    ))]
    const MAGIC: [u8; 8] = *b"BNDAPC11";
    #[cfg(all(
        feature = "androidbox-layout-row14",
        not(feature = "androidbox-layout-weight15")
    ))]
    const MAGIC: [u8; 8] = *b"BNDAPC10";
    #[cfg(all(
        feature = "androidbox-string-builder13",
        not(feature = "androidbox-layout-row14")
    ))]
    const MAGIC: [u8; 8] = *b"BNDAPC09";
    #[cfg(all(
        feature = "androidbox-string-text12",
        not(feature = "androidbox-string-builder13")
    ))]
    const MAGIC: [u8; 8] = *b"BNDAPC08";
    #[cfg(all(
        feature = "androidbox-activity-state11",
        not(feature = "androidbox-string-text12")
    ))]
    const MAGIC: [u8; 8] = *b"BNDAPC07";
    #[cfg(all(
        feature = "androidbox-activity-fields10",
        not(feature = "androidbox-activity-state11")
    ))]
    const MAGIC: [u8; 8] = *b"BNDAPC06";
    #[cfg(all(
        feature = "androidbox-dex-instance9",
        not(feature = "androidbox-activity-fields10")
    ))]
    const MAGIC: [u8; 8] = *b"BNDAPC05";
    #[cfg(all(
        feature = "androidbox-dex-methods8",
        not(feature = "androidbox-dex-instance9")
    ))]
    const MAGIC: [u8; 8] = *b"BNDAPC04";
    #[cfg(all(
        feature = "androidbox-multiaction3",
        not(feature = "androidbox-dex-methods8")
    ))]
    const MAGIC: [u8; 8] = *b"BNDAPC03";
    #[cfg(all(
        feature = "androidbox-scene-rpc2",
        not(feature = "androidbox-multiaction3")
    ))]
    const MAGIC: [u8; 8] = *b"BNDAPC02";
    #[cfg(not(feature = "androidbox-scene-rpc2"))]
    const MAGIC: [u8; 8] = *b"BNDAPC01";
    #[cfg(feature = "androidbox-layout-size18")]
    const VERSION: u8 = 14;
    #[cfg(all(
        feature = "androidbox-layout-directional17",
        not(feature = "androidbox-layout-size18")
    ))]
    const VERSION: u8 = 13;
    #[cfg(all(
        feature = "androidbox-layout-spacing16",
        not(feature = "androidbox-layout-directional17")
    ))]
    const VERSION: u8 = 12;
    #[cfg(all(
        feature = "androidbox-layout-weight15",
        not(feature = "androidbox-layout-spacing16")
    ))]
    const VERSION: u8 = 11;
    #[cfg(all(
        feature = "androidbox-layout-row14",
        not(feature = "androidbox-layout-weight15")
    ))]
    const VERSION: u8 = 10;
    #[cfg(all(
        feature = "androidbox-string-builder13",
        not(feature = "androidbox-layout-row14")
    ))]
    const VERSION: u8 = 9;
    #[cfg(all(
        feature = "androidbox-string-text12",
        not(feature = "androidbox-string-builder13")
    ))]
    const VERSION: u8 = 8;
    #[cfg(all(
        feature = "androidbox-activity-state11",
        not(feature = "androidbox-string-text12")
    ))]
    const VERSION: u8 = 7;
    #[cfg(all(
        feature = "androidbox-activity-fields10",
        not(feature = "androidbox-activity-state11")
    ))]
    const VERSION: u8 = 6;
    #[cfg(all(
        feature = "androidbox-dex-instance9",
        not(feature = "androidbox-activity-fields10")
    ))]
    const VERSION: u8 = 5;
    #[cfg(all(
        feature = "androidbox-dex-methods8",
        not(feature = "androidbox-dex-instance9")
    ))]
    const VERSION: u8 = 4;
    #[cfg(all(
        feature = "androidbox-multiaction3",
        not(feature = "androidbox-dex-methods8")
    ))]
    const VERSION: u8 = 3;
    #[cfg(all(
        feature = "androidbox-scene-rpc2",
        not(feature = "androidbox-multiaction3")
    ))]
    const VERSION: u8 = 2;
    #[cfg(not(feature = "androidbox-scene-rpc2"))]
    const VERSION: u8 = 1;
    const HEADER_BYTES: usize = 40;

    pub fn new(
        kind: AndroidAppMessageKind,
        request_id: u64,
        arg0: u64,
        arg1: u64,
        payload: &[u8],
    ) -> Result<Self, AndroidAppMessageError> {
        if payload.len() > ANDROID_APP_MESSAGE_PAYLOAD_BYTES {
            return Err(AndroidAppMessageError::InvalidPayloadLength);
        }
        let mut value = Self {
            kind,
            request_id,
            arg0,
            arg1,
            payload: [0; ANDROID_APP_MESSAGE_PAYLOAD_BYTES],
            payload_len: payload.len() as u8,
        };
        value.payload[..payload.len()].copy_from_slice(payload);
        value.validate()?;
        Ok(value)
    }

    pub const fn kind(self) -> AndroidAppMessageKind {
        self.kind
    }

    pub const fn request_id(self) -> u64 {
        self.request_id
    }

    pub const fn arg0(self) -> u64 {
        self.arg0
    }

    pub const fn arg1(self) -> u64 {
        self.arg1
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload[..usize::from(self.payload_len)]
    }

    pub const fn chunk_offset(self) -> u32 {
        (self.arg0 >> 32) as u32
    }

    pub const fn chunk_total(self) -> u32 {
        self.arg0 as u32
    }

    pub fn encode(self) -> [u8; ANDROID_APP_MESSAGE_WIRE_SIZE] {
        let mut wire = [0; ANDROID_APP_MESSAGE_WIRE_SIZE];
        wire[..8].copy_from_slice(&Self::MAGIC);
        wire[8] = Self::VERSION;
        wire[9] = self.kind.raw();
        wire[10] = self.payload_len;
        android_app_write_u64(&mut wire, 16, self.request_id);
        android_app_write_u64(&mut wire, 24, self.arg0);
        android_app_write_u64(&mut wire, 32, self.arg1);
        wire[Self::HEADER_BYTES..].copy_from_slice(&self.payload);
        wire
    }

    pub fn decode(
        wire: &[u8; ANDROID_APP_MESSAGE_WIRE_SIZE],
    ) -> Result<Self, AndroidAppMessageError> {
        if wire[..8] != Self::MAGIC {
            return Err(AndroidAppMessageError::InvalidMagic);
        }
        if wire[8] != Self::VERSION {
            return Err(AndroidAppMessageError::InvalidVersion);
        }
        let kind =
            AndroidAppMessageKind::from_raw(wire[9]).ok_or(AndroidAppMessageError::InvalidKind)?;
        let payload_len = usize::from(wire[10]);
        if payload_len > ANDROID_APP_MESSAGE_PAYLOAD_BYTES {
            return Err(AndroidAppMessageError::InvalidPayloadLength);
        }
        if wire[11..16].iter().any(|byte| *byte != 0)
            || wire[Self::HEADER_BYTES + payload_len..]
                .iter()
                .any(|byte| *byte != 0)
        {
            return Err(AndroidAppMessageError::NonCanonical);
        }
        Self::new(
            kind,
            android_app_read_u64(wire, 16),
            android_app_read_u64(wire, 24),
            android_app_read_u64(wire, 32),
            &wire[Self::HEADER_BYTES..Self::HEADER_BYTES + payload_len],
        )
    }

    fn validate(&self) -> Result<(), AndroidAppMessageError> {
        use AndroidAppMessageKind::{
            ButtonChunk, Click, Close, Closed, Error, LabelChunk, Open, Opened, Ready,
            UpdateTextChunk, Updated,
        };

        let empty = self.payload_len == 0;
        let valid = match self.kind {
            Ready => self.request_id == 0 && self.arg0 == ABI_VERSION && self.arg1 == 0 && empty,
            Open => self.request_id != 0 && self.arg0 != 0 && self.arg1 != 0 && empty,
            Opened => {
                let label_id = self.arg0 >> 32;
                let button_id = self.arg0 as u32 as u64;
                let revision = self.arg1 >> 32;
                let label_len = ((self.arg1 >> 16) & 0xffff) as usize;
                let button_len = (self.arg1 & 0xffff) as usize;
                self.request_id != 0
                    && label_id != 0
                    && button_id != 0
                    && label_id != button_id
                    && revision == 0
                    && (1..=ANDROID_APP_LABEL_MAX_BYTES).contains(&label_len)
                    && (1..=ANDROID_APP_BUTTON_MAX_BYTES).contains(&button_len)
                    && empty
            }
            LabelChunk | ButtonChunk | UpdateTextChunk => {
                let offset = self.chunk_offset() as usize;
                let total = self.chunk_total() as usize;
                let maximum = if matches!(self.kind, ButtonChunk) {
                    ANDROID_APP_BUTTON_MAX_BYTES
                } else {
                    ANDROID_APP_LABEL_MAX_BYTES
                };
                self.request_id != 0
                    && self.arg1 == 0
                    && total != 0
                    && total <= maximum
                    && self.payload_len != 0
                    && offset < total
                    && offset
                        .checked_add(usize::from(self.payload_len))
                        .is_some_and(|end| end <= total)
                    && self
                        .payload()
                        .iter()
                        .all(|byte| matches!(*byte, 0x20..=0x7e))
            }
            Click => {
                let view_id = self.arg1 >> 32;
                self.request_id != 0
                    && self.arg0 != 0
                    && view_id != 0
                    && view_id <= u32::MAX as u64
                    && empty
            }
            Updated => {
                let revision = self.arg1 >> 32;
                let text_len = self.arg1 as u32 as usize;
                #[cfg(feature = "androidbox-string-builder13")]
                let update_identity_valid = {
                    let provenance = (self.arg0 >> 32) as u32;
                    let total_calls = provenance as u8;
                    let instance_calls = (provenance >> 8) as u8;
                    let field_byte = (provenance >> 16) as u8;
                    let field_reads = field_byte & 0x3f;
                    let dynamic_string_text = field_byte & 0x40 != 0;
                    let direct_string_text = field_byte & 0x80 != 0;
                    let int_state = (provenance >> 24) as u8;
                    let parent_shape = !direct_string_text
                        && !dynamic_string_text
                        && (int_state == 0
                            || (total_calls == 1 && instance_calls == 1 && field_reads == 2));
                    let direct_string_shape = direct_string_text
                        && int_state != 0
                        && total_calls == 0
                        && instance_calls == 0
                        && field_reads == 2;
                    self.arg0 as u32 != 0
                        && total_calls <= 1
                        && instance_calls <= total_calls
                        && field_reads <= 2
                        && (parent_shape || direct_string_shape)
                };
                #[cfg(all(
                    feature = "androidbox-string-text12",
                    not(feature = "androidbox-string-builder13")
                ))]
                let update_identity_valid = {
                    let provenance = (self.arg0 >> 32) as u32;
                    let total_calls = provenance as u8;
                    let instance_calls = (provenance >> 8) as u8;
                    let field_byte = (provenance >> 16) as u8;
                    let field_reads = field_byte & 0x7f;
                    let direct_string_text = field_byte & 0x80 != 0;
                    let int_state = (provenance >> 24) as u8;
                    let parent_shape = !direct_string_text
                        && (int_state == 0
                            || (total_calls == 1 && instance_calls == 1 && field_reads == 2));
                    let direct_string_shape = direct_string_text
                        && int_state != 0
                        && total_calls == 0
                        && instance_calls == 0
                        && field_reads == 2;
                    self.arg0 as u32 != 0
                        && total_calls <= 1
                        && instance_calls <= total_calls
                        && field_reads <= 2
                        && (parent_shape || direct_string_shape)
                };
                #[cfg(all(
                    feature = "androidbox-activity-state11",
                    not(feature = "androidbox-string-text12")
                ))]
                let update_identity_valid = {
                    let provenance = (self.arg0 >> 32) as u32;
                    let total_calls = provenance as u8;
                    let instance_calls = (provenance >> 8) as u8;
                    let field_reads = (provenance >> 16) as u8;
                    let int_state = (provenance >> 24) as u8;
                    self.arg0 as u32 != 0
                        && total_calls <= 1
                        && instance_calls <= total_calls
                        && field_reads <= 2
                        && (int_state == 0
                            || (total_calls == 1 && instance_calls == 1 && field_reads == 2))
                };
                #[cfg(all(
                    feature = "androidbox-activity-fields10",
                    not(feature = "androidbox-activity-state11")
                ))]
                let update_identity_valid = {
                    let provenance = (self.arg0 >> 32) as u32;
                    let total_calls = provenance as u8;
                    let instance_calls = (provenance >> 8) as u8;
                    let field_reads = (provenance >> 16) as u8;
                    self.arg0 as u32 != 0
                        && provenance >> 24 == 0
                        && total_calls <= 1
                        && instance_calls <= total_calls
                        && field_reads <= 1
                };
                #[cfg(all(
                    feature = "androidbox-dex-instance9",
                    not(feature = "androidbox-activity-fields10")
                ))]
                let update_identity_valid = {
                    let provenance = (self.arg0 >> 32) as u32;
                    let total_calls = provenance as u8;
                    let instance_calls = (provenance >> 8) as u8;
                    self.arg0 as u32 != 0
                        && provenance >> 16 == 0
                        && total_calls <= 1
                        && instance_calls <= total_calls
                };
                #[cfg(all(
                    feature = "androidbox-dex-methods8",
                    not(feature = "androidbox-dex-instance9")
                ))]
                let update_identity_valid = self.arg0 as u32 != 0 && self.arg0 >> 32 <= 1;
                #[cfg(not(feature = "androidbox-dex-methods8"))]
                let update_identity_valid = self.arg0 != 0 && self.arg0 <= u32::MAX as u64;
                self.request_id != 0
                    && update_identity_valid
                    && revision != 0
                    && (1..=ANDROID_APP_LABEL_MAX_BYTES).contains(&text_len)
                    && empty
            }
            Close => self.request_id != 0 && self.arg0 != 0 && self.arg1 != 0 && empty,
            Closed => self.request_id != 0 && self.arg0 == 0 && self.arg1 == 0 && empty,
            Error => {
                self.request_id != 0
                    && self.arg0 != 0
                    && self.arg0 <= u32::MAX as u64
                    && self.arg1 == 0
                    && empty
            }
            #[cfg(feature = "androidbox-restart0")]
            AndroidAppMessageKind::Crash => {
                self.request_id != 0 && self.arg0 != 0 && self.arg1 != 0 && empty
            }
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::SceneOpened => {
                self.request_id != 0
                    && (1..=ANDROID_APP_SCENE_MAX_NODES as u64).contains(&self.arg0)
                    && self.arg1 == 0
                    && empty
            }
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::DescribeNode => {
                let index = self.arg1 as u32 as usize;
                self.request_id != 0
                    && self.arg0 != 0
                    && index < ANDROID_APP_SCENE_MAX_NODES
                    && empty
            }
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::Node => {
                if self.request_id == 0
                    || self.arg0 >= ANDROID_APP_SCENE_MAX_NODES as u64
                    || self.arg1 > u32::MAX as u64
                    || usize::from(self.payload_len) != ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE
                {
                    false
                } else {
                    let mut descriptor_wire = [0; ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE];
                    descriptor_wire.copy_from_slice(self.payload());
                    AndroidAppSceneNodeDescriptor::decode(&descriptor_wire).is_some_and(
                        |descriptor| descriptor.is_canonical_for_index(self.arg0 as u8),
                    )
                }
            }
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::NodeTextChunk => {
                let offset = self.chunk_offset() as usize;
                let total = self.chunk_total() as usize;
                self.request_id != 0
                    && self.arg1 < ANDROID_APP_SCENE_MAX_NODES as u64
                    && total != 0
                    && total <= ANDROID_APP_SCENE_NODE_TEXT_MAX_BYTES
                    && self.payload_len != 0
                    && offset < total
                    && offset
                        .checked_add(usize::from(self.payload_len))
                        .is_some_and(|end| end <= total)
                    && self
                        .payload()
                        .iter()
                        .all(|byte| matches!(*byte, 0x20..=0x7e))
            }
        };
        if valid {
            Ok(())
        } else {
            Err(AndroidAppMessageError::NonCanonical)
        }
    }
}

/// Which endpoint Init transfers to the ABI 47 UI-host bootstrap channel.
#[cfg(feature = "androidbox-process0")]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidAppBootstrapKind {
    SurfaceClient = 1,
    RuntimeClient = 2,
}

#[cfg(feature = "androidbox-process0")]
impl AndroidAppBootstrapKind {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::SurfaceClient),
            2 => Some(Self::RuntimeClient),
            _ => None,
        }
    }
}

/// Fixed canonical payload accompanying one moved endpoint from Init to App.
#[cfg(feature = "androidbox-process0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidAppBootstrap {
    kind: AndroidAppBootstrapKind,
}

#[cfg(feature = "androidbox-process0")]
impl AndroidAppBootstrap {
    const MAGIC: [u8; 8] = *b"BNDABP01";
    const VERSION: u8 = 1;

    pub const fn new(kind: AndroidAppBootstrapKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> AndroidAppBootstrapKind {
        self.kind
    }

    pub fn encode(self) -> [u8; ANDROID_APP_BOOTSTRAP_WIRE_SIZE] {
        let mut wire = [0; ANDROID_APP_BOOTSTRAP_WIRE_SIZE];
        wire[..8].copy_from_slice(&Self::MAGIC);
        wire[8] = Self::VERSION;
        wire[9] = self.kind as u8;
        wire
    }

    pub fn decode(wire: &[u8; ANDROID_APP_BOOTSTRAP_WIRE_SIZE]) -> Option<Self> {
        if wire[..8] != Self::MAGIC
            || wire[8] != Self::VERSION
            || wire[10..].iter().any(|byte| *byte != 0)
        {
            return None;
        }
        Some(Self {
            kind: AndroidAppBootstrapKind::from_raw(wire[9])?,
        })
    }
}

/// One-shot ABI 48 Init ↔ App control message used to replace a faulted
/// AndroidApp worker. The bootstrap Channel itself is retained as the
/// supervisor transport, so it is older than every replacement endpoint it
/// can carry.
#[cfg(feature = "androidbox-restart0")]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidAppSupervisorMessageKind {
    Rebind = 1,
    Rebound = 2,
}

#[cfg(feature = "androidbox-restart0")]
impl AndroidAppSupervisorMessageKind {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Rebind),
            2 => Some(Self::Rebound),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

/// Canonical fixed-size metadata for ABI 48's single controlled worker
/// replacement. Sender identity is supplied by the kernel Channel envelope;
/// these fields bind that authority to both worker generations and the exact
/// ProcessWait result.
#[cfg(feature = "androidbox-restart0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidAppSupervisorMessage {
    kind: AndroidAppSupervisorMessageKind,
    epoch: u64,
    old_pid: u64,
    new_pid: u64,
    exit_code: u64,
    termination_reason: u64,
}

#[cfg(feature = "androidbox-restart0")]
impl AndroidAppSupervisorMessage {
    const MAGIC: [u8; 8] = *b"BNDARS01";
    const VERSION: u8 = 1;

    pub fn new(
        kind: AndroidAppSupervisorMessageKind,
        epoch: u64,
        old_pid: u64,
        new_pid: u64,
        exit_code: u64,
        termination_reason: u64,
    ) -> Option<Self> {
        let value = Self {
            kind,
            epoch,
            old_pid,
            new_pid,
            exit_code,
            termination_reason,
        };
        value.is_canonical().then_some(value)
    }

    pub const fn kind(self) -> AndroidAppSupervisorMessageKind {
        self.kind
    }

    pub const fn epoch(self) -> u64 {
        self.epoch
    }

    pub const fn old_pid(self) -> u64 {
        self.old_pid
    }

    pub const fn new_pid(self) -> u64 {
        self.new_pid
    }

    pub const fn exit_code(self) -> u64 {
        self.exit_code
    }

    pub const fn termination_reason(self) -> u64 {
        self.termination_reason
    }

    pub fn encode(self) -> [u8; ANDROID_APP_SUPERVISOR_WIRE_SIZE] {
        let mut wire = [0; ANDROID_APP_SUPERVISOR_WIRE_SIZE];
        wire[..8].copy_from_slice(&Self::MAGIC);
        wire[8] = Self::VERSION;
        wire[9] = self.kind.raw();
        write_supervisor_u64(&mut wire, 16, self.epoch);
        write_supervisor_u64(&mut wire, 24, self.old_pid);
        write_supervisor_u64(&mut wire, 32, self.new_pid);
        write_supervisor_u64(&mut wire, 40, self.exit_code);
        write_supervisor_u64(&mut wire, 48, self.termination_reason);
        wire
    }

    pub fn decode(wire: &[u8; ANDROID_APP_SUPERVISOR_WIRE_SIZE]) -> Option<Self> {
        if wire[..8] != Self::MAGIC
            || wire[8] != Self::VERSION
            || wire[10..16].iter().any(|byte| *byte != 0)
            || wire[56..].iter().any(|byte| *byte != 0)
        {
            return None;
        }
        Self::new(
            AndroidAppSupervisorMessageKind::from_raw(wire[9])?,
            read_supervisor_u64(wire, 16),
            read_supervisor_u64(wire, 24),
            read_supervisor_u64(wire, 32),
            read_supervisor_u64(wire, 40),
            read_supervisor_u64(wire, 48),
        )
    }

    fn is_canonical(self) -> bool {
        self.epoch == 1
            && self.old_pid != 0
            && self.new_pid != 0
            && self.old_pid != self.new_pid
            && self.exit_code == 0
            && self.termination_reason == ProcessTerminationReason::Faulted.raw()
    }
}

#[cfg(feature = "androidbox-restart0")]
fn write_supervisor_u64(
    wire: &mut [u8; ANDROID_APP_SUPERVISOR_WIRE_SIZE],
    offset: usize,
    value: u64,
) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[cfg(feature = "androidbox-restart0")]
fn read_supervisor_u64(wire: &[u8; ANDROID_APP_SUPERVISOR_WIRE_SIZE], offset: usize) -> u64 {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&wire[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}

#[cfg(feature = "androidbox-process0")]
fn android_app_write_u64(
    wire: &mut [u8; ANDROID_APP_MESSAGE_WIRE_SIZE],
    offset: usize,
    value: u64,
) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[cfg(feature = "androidbox-process0")]
fn android_app_read_u64(wire: &[u8; ANDROID_APP_MESSAGE_WIRE_SIZE], offset: usize) -> u64 {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&wire[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}

#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserImageId {
    Init = 1,
    ServiceManager = 2,
    Provider = 3,
    Client = 4,
    SurfaceServer = 5,
    Launcher = 6,
    App = 7,
    InputServer = 8,
    StorageServer = 9,
    /// ABI 47's isolated, capability-minimal Android compatibility worker.
    ///
    /// This image owns APK verification/interpreter state but no Surface,
    /// graphics-buffer, input, storage, or package-manager capability.
    #[cfg(feature = "androidbox-process0")]
    AndroidApp = 10,
}

/// Stable node identities for M66's authenticated resident shutdown graph.
///
/// Dependency masks point from a consumer to the providers it needs while
/// running. Shutdown proceeds in the reverse direction: every dependent must
/// be quiesced before its provider may quiesce.
#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShutdownServiceNode {
    ServiceManager = 0,
    Provider = 1,
    PrimaryClient = 2,
    SecondaryClient = 3,
    SurfaceServer = 4,
    InputServer = 5,
    Launcher = 6,
    App = 7,
}

impl ShutdownServiceNode {
    pub const fn raw(self) -> u64 {
        self as u64
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            0 => Some(Self::ServiceManager),
            1 => Some(Self::Provider),
            2 => Some(Self::PrimaryClient),
            3 => Some(Self::SecondaryClient),
            4 => Some(Self::SurfaceServer),
            5 => Some(Self::InputServer),
            6 => Some(Self::Launcher),
            7 => Some(Self::App),
            _ => None,
        }
    }

    pub const fn bit(self) -> u64 {
        1_u64 << self.raw()
    }

    pub const fn image_id(self) -> UserImageId {
        match self {
            Self::ServiceManager => UserImageId::ServiceManager,
            Self::Provider => UserImageId::Provider,
            Self::PrimaryClient | Self::SecondaryClient => UserImageId::Client,
            Self::SurfaceServer => UserImageId::SurfaceServer,
            Self::InputServer => UserImageId::InputServer,
            Self::Launcher => UserImageId::Launcher,
            Self::App => UserImageId::App,
        }
    }

    pub const fn dependency_mask(self) -> u64 {
        match self {
            Self::ServiceManager | Self::SurfaceServer => 0,
            Self::Provider => Self::ServiceManager.bit(),
            Self::PrimaryClient | Self::SecondaryClient => {
                Self::ServiceManager.bit() | Self::Provider.bit()
            }
            Self::InputServer => Self::SurfaceServer.bit(),
            Self::Launcher | Self::App => Self::SurfaceServer.bit() | Self::InputServer.bit(),
        }
    }

    pub const fn dependent_mask(self) -> u64 {
        match self {
            Self::ServiceManager => {
                Self::Provider.bit() | Self::PrimaryClient.bit() | Self::SecondaryClient.bit()
            }
            Self::Provider => Self::PrimaryClient.bit() | Self::SecondaryClient.bit(),
            Self::PrimaryClient | Self::SecondaryClient | Self::Launcher | Self::App => 0,
            Self::SurfaceServer => Self::InputServer.bit() | Self::Launcher.bit() | Self::App.bit(),
            Self::InputServer => Self::Launcher.bit() | Self::App.bit(),
        }
    }

    pub const fn shutdown_wave(self) -> u64 {
        match self {
            Self::PrimaryClient | Self::SecondaryClient | Self::Launcher | Self::App => 0,
            Self::Provider | Self::InputServer => 1,
            Self::ServiceManager | Self::SurfaceServer => 2,
        }
    }
}

impl UserImageId {
    pub const fn raw(self) -> u64 {
        self as u64
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::Init),
            2 => Some(Self::ServiceManager),
            3 => Some(Self::Provider),
            4 => Some(Self::Client),
            5 => Some(Self::SurfaceServer),
            6 => Some(Self::Launcher),
            7 => Some(Self::App),
            8 => Some(Self::InputServer),
            9 => Some(Self::StorageServer),
            #[cfg(feature = "androidbox-process0")]
            10 => Some(Self::AndroidApp),
            _ => None,
        }
    }

    pub const fn is_spawnable(self) -> bool {
        !matches!(self, Self::Init)
    }
}

/// Why a dynamic process reached its terminal state.
///
/// ABI 19 returns this value in `x2` from a successful `ProcessWait`; `x1`
/// contains the process exit code. The generation-qualified PID remains the
/// wait input and is deliberately not duplicated in the result registers.
#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessTerminationReason {
    Exited = 1,
    Faulted = 2,
    Killed = 3,
}

impl ProcessTerminationReason {
    pub const fn raw(self) -> u64 {
        self as u64
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::Exited),
            2 => Some(Self::Faulted),
            3 => Some(Self::Killed),
            _ => None,
        }
    }
}

#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyscallNumber {
    AbiVersion = 0,
    ChannelCreate = 1,
    ChannelWrite = 2,
    ChannelRead = 3,
    HandleDuplicate = 4,
    HandleClose = 5,
    InitReady = 6,
    InitFailed = 7,
    ChannelWriteBytes = 8,
    ChannelReadBytes = 9,
    ProcessSpawn = 10,
    ThreadExit = 11,
    /// Waits for one generation-qualified dynamic child.
    ///
    /// On ABI-19 success `x1` is the exit code and `x2` is a canonical
    /// [`ProcessTerminationReason`]. Only init may call this operation.
    ProcessWait = 12,
    ChannelWriteTransfer = 13,
    ChannelReadTransfer = 14,
    ObjectWait = 15,
    EventCreate = 16,
    EventSignal = 17,
    EventClear = 18,
    ObjectWaitMany = 19,
    ChannelPeek = 20,
    ChannelReadEnvelope = 21,
    /// Wait on a user array of 1..=8 canonical packed wait items.
    ///
    /// `x0` is the user pointer, `x1` the item count, and `x2` the relative
    /// timeout in nanoseconds. The result follows `ObjectWaitMany`: status is
    /// returned in `x0`, the ready item index in `x1`, and observed signal bits
    /// in `x2`.
    ObjectWaitManyArray = 22,
    /// Opens one canonical path relative to a verified directory capability.
    ///
    /// `x0` is the directory handle, `x1` the user path pointer, and `x2` its
    /// byte length. On success `x1` receives a read-only VMO handle and `x2`
    /// the immutable snapshot size. Boot-directory paths use the ABI-23 rules;
    /// AppData roots use [`validate_app_data_path`].
    FileOpenAt = 23,
    /// Copies one bounded range from a read-only VMO into user memory.
    ///
    /// `x0` is the VMO handle, `x1` the user destination, and `x2` packs a
    /// 32-bit byte offset in its high half and a 32-bit requested length in
    /// its low half. On success `x1` receives bytes copied and `x2` the VMO
    /// size. A read at exact EOF succeeds with zero bytes.
    VmoRead = 24,
    /// Acquires the writable back buffer for the caller's unique UI surface.
    SurfaceAcquire = 25,
    /// Atomically validates and publishes one frame for the caller's surface.
    SurfacePresent = 26,
    /// Reads one queued input sample from the caller's surface session.
    SurfaceReadInput = 27,
    /// Creates one fixed-size writable XRGB8888 graphics buffer.
    ///
    /// `x0` is the format, `x1` packs width in its high half and height in its
    /// low half. `x2` is either [`GRAPHICS_BUFFER_CREATE_FLAGS_NONE`] for the
    /// legacy copy path or [`GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE`] for the
    /// shared BufferQueue path.
    GraphicsBufferCreate = 28,
    /// Copies one bounded user-memory range into a graphics buffer.
    GraphicsBufferWrite = 29,
    /// Presents a read-capable graphics buffer through a UI surface.
    SurfacePresentBuffer = 30,
    /// Forces one generation-qualified dynamic child into a terminal state.
    ///
    /// Only init may call this operation. `x0` is the child PID while `x1`
    /// and `x2` must both be zero. Successful termination is later observed
    /// through `ProcessWait` with reason `Killed`.
    ProcessTerminate = 31,
    /// Maps one opt-in graphics buffer at [`GRAPHICS_BUFFER_MAP_ADDRESS`].
    GraphicsBufferMap = 32,
    /// Removes the caller's generation-pinned graphics-buffer mapping.
    GraphicsBufferUnmap = 33,
    /// Publishes one directly rendered producer frame to the single-slot queue.
    GraphicsBufferQueue = 34,
    /// Acquires the queued generation for the authenticated SurfaceServer.
    GraphicsBufferAcquire = 35,
    /// Cancels an acquired generation and returns it to the producer.
    GraphicsBufferRelease = 36,
    /// Acquires one software-paced presentation opportunity.
    ///
    /// `x0` is the caller's unique surface handle while `x1` and `x2` must
    /// both be zero. On success `x1` receives the monotonically increasing
    /// frame-opportunity epoch and `x2` receives its logical timer boundary.
    SurfaceFrameAcquire = 37,
    /// Reads one queued hardware-key transition from the caller's Surface.
    ///
    /// `x0` is the caller's unique surface handle while `x1` and `x2` must
    /// both be zero. On success `x1` contains the canonical packed key state
    /// and `x2` its monotonically increasing sequence number.
    SurfaceReadKey = 38,
    /// Acquires the system-wide physical-input stream.
    ///
    /// Only the canonical [`UserImageId::InputServer`] process may call this
    /// operation. All input registers must be zero. On success `x1` receives
    /// one move-only capability carrying [`Rights::INPUT_DEFAULT`].
    InputAcquire = 39,
    /// Reads one canonical pointer or key transition from the global stream.
    ///
    /// `x0` is the acquired input capability while `x1` and `x2` must both be
    /// zero. On success `x1` and `x2` contain [`InputEvent::encode_registers`].
    InputReadEvent = 40,
    /// Returns the immutable identity of one acquired physical-input session.
    ///
    /// `x0` is the acquired input capability while `x1` and `x2` must both be
    /// zero. Only the unique live [`UserImageId::InputServer`] may call this
    /// operation. On success `x1` contains the capability session ID and `x2`
    /// contains the global physical-input sequence floor captured atomically
    /// when that session was acquired.
    InputSessionInfo = 41,
    /// Opens the caller image's stable private AppData root.
    ///
    /// `x0`, `x1`, and `x2` must all be zero. The kernel derives a stable
    /// [`AppDataPrincipal`] from the authenticated user-image identity rather
    /// than accepting a caller-selected namespace. On success `x1` is a root
    /// handle carrying [`Rights::APP_DATA_ROOT_DEFAULT`] and `x2` is the
    /// principal's non-zero raw value. The opt-in boot proof may return
    /// [`Status::ShouldWait`] until the predecessor UI topology is sealed; an
    /// exact retry then observes the authority gate without allocating an
    /// early handle.
    AppDataRootOpen = 42,
    /// Atomically creates or replaces one bounded AppData file.
    ///
    /// `x0` is an AppData root, `x1` points to one canonical
    /// [`FileReplaceRequest`] wire, and `x2` must equal
    /// [`FILE_REPLACE_REQUEST_WIRE_SIZE`]. The kernel copies the fixed request,
    /// validates it before copying the value, and returns the newly committed
    /// non-zero generation in `x1` with zero in `x2`. [`Status::Conflict`]
    /// reports a failed CAS without modifying data. [`Status::OutcomeUnknown`]
    /// means durability could not be determined and the caller must reopen or
    /// enumerate before retrying.
    FileReplaceAt = 43,
    /// Creates one AppData directory, including no implicit parent components.
    ///
    /// `x0` is an AppData root, `x1` points to a canonical UTF-8 path, and `x2`
    /// is its exact byte length. Success returns zero in `x1` and `x2`.
    DirectoryCreateAt = 44,
    /// Removes one empty directory or one file from an AppData root.
    ///
    /// `x0` is an AppData root, `x1` points to a canonical UTF-8 path, and `x2`
    /// is its exact byte length. Success returns zero in `x1` and `x2`.
    UnlinkAt = 45,
    /// Reads one flattened, root-relative AppData directory entry.
    ///
    /// `x0` is an AppData root, `x1` is a zero-based cursor no greater than
    /// [`APP_DATA_DIRECTORY_MAX_ENTRIES`], and `x2` points to exactly
    /// [`APP_DATA_DIRECTORY_ENTRY_WIRE_SIZE`] writable bytes. Success writes a
    /// canonical [`AppDataDirectoryEntry`], returns the next cursor in `x1`,
    /// and zero in `x2`. [`Status::NotFound`] marks end of enumeration without
    /// writing the output buffer. Cursors are deliberately weak, bounded
    /// indices rather than snapshot tokens: a caller that mutates concurrently
    /// with enumeration must restart at cursor zero to avoid skips or repeats.
    DirectoryReadAt = 46,
    /// Creates one immutable VMO from a bounded user payload.
    ///
    /// `x0` is the payload pointer, `x1` its length in
    /// `0..=`[`IPC_BUFFER_PAYLOAD_MAX_BYTES`], and `x2` must equal
    /// [`IPC_BUFFER_CREATE_FLAGS_NONE`]. The pointer is zero exactly when the
    /// length is zero. On success `x1` receives the immutable VMO handle and
    /// `x2` receives its payload length.
    IpcBufferCreate = 47,
    /// Acquires the singleton AppData storage-volume capability.
    ///
    /// `x0`, `x1`, and `x2` must all be zero. Only the authenticated
    /// [`UserImageId::StorageServer`] may call this operation. On success `x1`
    /// receives a move-only volume capability with
    /// [`Rights::STORAGE_VOLUME_DEFAULT`].
    StorageAcquire = 48,
    /// Submits one canonical block request through a storage session.
    ///
    /// `x0` is the acquired storage-volume capability, `x1` points to a
    /// [`StorageBlockRequest`], and `x2` must equal
    /// [`STORAGE_BLOCK_REQUEST_WIRE_SIZE`]. The submitted token must be zero;
    /// the kernel returns its generated non-zero token in `x1` on success.
    StorageSubmit = 49,
    /// Takes the completion for one previously submitted block request.
    ///
    /// `x0` is the acquired storage-volume capability and `x1` is the request's
    /// kernel-generated token. For a read, `x2` points to a writable destination
    /// of exactly [`StorageBlockRequest::byte_len`] bytes; it must be zero for a
    /// write or flush. A pending request returns [`Status::ShouldWait`] without
    /// consuming the token.
    StorageTake = 50,
    /// Connects the caller to the StorageServer.
    ///
    /// `x0` is a non-empty subset of [`STORAGE_CONNECT_RIGHTS_MASK`], while
    /// `x1` and `x2` must both be zero. On success `x1` receives a move-only
    /// session capability with [`Rights::STORAGE_SESSION_DEFAULT`].
    StorageConnect = 51,
    /// Accepts one pending client connection on the storage volume.
    ///
    /// `x0` is the acquired volume capability, `x1` points to writable memory
    /// for one [`StorageSessionBinding`], and `x2` must equal
    /// [`STORAGE_SESSION_BINDING_WIRE_SIZE`]. Success writes the binding and
    /// returns a move-only server-side session handle in `x1`.
    StorageAccept = 52,
    /// Advances M65's init-only, two-phase shutdown orchestration.
    ///
    /// `x0` is [`SYSTEM_SHUTDOWN_PREPARE`] or [`SYSTEM_SHUTDOWN_COMMIT`],
    /// `x1` is the exact non-zero AppData generation acknowledged by the
    /// StorageServer, and `x2` must equal [`SYSTEM_SHUTDOWN_FLAGS_NONE`].
    /// Prepare atomically closes new process/storage-session admission only
    /// after all client sessions are drained. Commit is accepted only after
    /// the StorageServer has flushed, read back, exited, and been reaped.
    /// This call does not itself claim hardware poweroff.
    SystemShutdown = 53,
    /// Registers or quiesces one M66 resident service-graph node.
    ///
    /// `x0` is [`SERVICE_SHUTDOWN_REGISTER`] or
    /// [`SERVICE_SHUTDOWN_QUIESCE`], `x1` is a
    /// [`ShutdownServiceNode`], and `x2` is the node's exact dependency mask
    /// for Register or [`SERVICE_SHUTDOWN_FLAGS_NONE`] for Quiesce. The kernel
    /// authenticates the current generation-qualified PID and executable
    /// image. Quiesce is accepted only after global Prepare and only after all
    /// dependents have already quiesced.
    #[cfg(feature = "resident-platform-shutdown-runtime")]
    ServiceShutdown = 54,
    /// Opens the kernel-owned immutable BMF1 service manifest.
    ///
    /// Only init may call this operation. `x0`, `x1`, and `x2` must all be
    /// [`SERVICE_MANIFEST_OPEN_FLAGS_NONE`]. On success `x1` receives a
    /// read-only VMO handle and `x2` its exact wire size. The manifest is
    /// validated by the kernel before publication and is decoded
    /// transactionally by init before any manifest-directed spawn.
    #[cfg(feature = "unified-product-manifest-supervision-runtime")]
    ServiceManifestOpen = 55,
    /// Reports M73 event-supervisor transitions into a kernel-authenticated
    /// evidence ledger.
    ///
    /// Only init may call this operation. `x0` is one canonical
    /// `SERVICE_SUPERVISOR_REPORT_*` operation and `x1`/`x2` carry that
    /// operation's bounded payload. The zero-valued UI-convergence query
    /// requires zero payload and returns [`Status::ShouldWait`] until the
    /// kernel-owned predecessor UI seal is published. The call grants no
    /// object, process, or device authority; the kernel validates report order
    /// against its own generation-qualified process accounting before
    /// committing a transition.
    #[cfg(feature = "unified-product-event-supervision-runtime")]
    ServiceSupervisorReport = 56,
    /// Opens the one boot-local maintenance authorization session.
    ///
    /// Only init may call this operation. `x0`, `x1`, and `x2` must all equal
    /// [`MAINTENANCE_SESSION_OPEN_FLAGS_NONE`]. The kernel accepts the call
    /// exactly once, and only after a BMA1 signature, product/device binding,
    /// and persistent sequence/audit transaction all completed before EL0.
    /// Success returns the consumed authorization sequence in `x1` and its
    /// operation mask in `x2`. The call grants no storage or device handle.
    #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
    MaintenanceSessionOpen = 57,
    /// Reads one current wall-clock snapshot from QEMU `virt`'s PL031 RTC.
    ///
    /// This isolated mobile-preview operation is available only to the
    /// generation-qualified SurfaceServer. `x0`, `x1`, and `x2` must all equal
    /// [`SYSTEM_CLOCK_READ_FLAGS_NONE`]. On success `x1` contains unsigned Unix
    /// seconds and `x2` is [`SYSTEM_CLOCK_SOURCE_QEMU_PL031`]. The operation
    /// grants no device capability and cannot write or configure the RTC.
    #[cfg(feature = "mobile-ui-runtime")]
    SystemClockRead = 58,
    /// Copies one canonical, read-only installed-package snapshot to EL0.
    ///
    /// This isolated AndroidBox install profile operation accepts a writable
    /// 640-byte user destination in `x0`, exactly
    /// [`ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE`] in `x1`, and zero in `x2`.
    /// Success writes one [`AndroidPackageSnapshot`] and returns zero in `x1`
    /// and `x2`. It creates no handle, maps no memory, grants no package-store
    /// or device authority, and cannot mutate install state.
    #[cfg(feature = "androidbox-apk-install0")]
    AndroidPackageSnapshotRead = 59,
    /// Submits or collects one asynchronous, generation-bound APK relaunch.
    ///
    /// Only the authenticated, generation-qualified [`UserImageId::Launcher`]
    /// may call this isolated AndroidBox operation. `x0` points to one readable
    /// and writable 640-byte exchange containing a canonical
    /// [`AndroidPackageRelaunchRequest`], `x1` must equal
    /// [`ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE`]. `x2` must be zero in
    /// Install-0; the EL0Runtime-0 child profile instead supplies the non-zero
    /// compatible-Activity session id being verified.
    /// The first accepted submission, and an exact retry while that same
    /// request remains pending, return [`Status::ShouldWait`] without changing
    /// the exchange. A successful collection replaces all 640 bytes with a
    /// canonical `BNDAPS01` [`AndroidPackageSnapshot`]. Any failed submission
    /// or collection is reported by its [`Status`] and publishes no snapshot.
    /// Every return, including [`Status::ShouldWait`] and success, sets `x1`
    /// and `x2` to zero; only the exchange buffer carries a successful result.
    #[cfg(feature = "androidbox-apk-install0")]
    AndroidPackageRelaunch = 60,
    /// Consumes one freshly verified, session-bound package-image grant.
    ///
    /// Only the authenticated, generation-qualified [`UserImageId::App`] may
    /// call this isolated operation. `x0` points to one readable canonical
    /// [`AndroidPackageImageClaim`], `x1` must equal
    /// [`ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE`], and `x2` must be zero.
    /// Success returns a VMO handle carrying exactly [`Rights::READ`] in `x1`
    /// and the immutable APK byte length in `x2`. The handle cannot be
    /// duplicated, transferred, mapped, written, or used as storage authority.
    /// Each exact grant can be consumed once.
    #[cfg(feature = "androidbox-el0-runtime0")]
    AndroidPackageImageClaim = 61,
    /// Atomically presents one client content buffer beneath a separately
    /// authenticated SurfaceServer-owned system-chrome buffer.
    ///
    /// This AndroidBox Interactive-0 operation is callable only by the
    /// generation-qualified [`UserImageId::SurfaceServer`]. `x0` is its
    /// unique Surface capability, `x1` packs the read-only client-content
    /// handle in the high 32 bits and read-only system-chrome handle in the
    /// low 32 bits, and `x2` points to one canonical
    /// [`BufferPresent`] wire. Both buffers retain full physical-screen
    /// geometry, but only the canonical content viewport is copied from the
    /// client buffer; top and bottom system regions are copied exclusively
    /// from the SurfaceServer buffer. The handles must name distinct live
    /// buffers and both generations are validated before any display byte
    /// changes.
    #[cfg(feature = "androidbox-interactive0")]
    SurfacePresentBufferLayers = 62,
    /// Submits or collects one asynchronous, exact-identity package removal.
    ///
    /// Only the authenticated, generation-qualified built-in
    /// [`UserImageId::App`] may call this ABI 52 operation. `x0` points to one
    /// readable/writable [`ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE`]-byte exchange
    /// containing [`AndroidPackageUninstallRequest`], `x1` is that exact size,
    /// and `x2` is zero. The first accepted call and exact retries return
    /// [`Status::ShouldWait`]. Success atomically replaces the exchange with
    /// [`AndroidPackageUninstallResult`]. The operation grants no handle,
    /// path, package-store, block-device, or general deletion authority.
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    AndroidPackageUninstall = 63,
    /// Reads the current boot-local, fully admitted APK install candidate.
    ///
    /// Only the built-in [`UserImageId::App`] may call this ABI 53 operation.
    /// `x0` points to exactly
    /// [`ANDROID_PACKAGE_INSTALL_CANDIDATE_WIRE_SIZE`] writable bytes, while
    /// `x1` is that size and `x2` is zero. Success writes either a canonical
    /// present candidate or a canonical empty candidate. No APK bytes,
    /// pathname, handle, storage location, or block authority is returned.
    #[cfg(feature = "androidbox-runtime-install2")]
    AndroidPackageInstallCandidateRead = 64,
    /// Submits or collects one asynchronous, exact-candidate install/update.
    ///
    /// Only the built-in [`UserImageId::App`] may call this ABI 53 operation.
    /// `x0` points to one readable/writable
    /// [`ANDROID_PACKAGE_INSTALL_WIRE_SIZE`]-byte exchange containing
    /// [`AndroidPackageInstallRequest`], `x1` is that exact size, and `x2` is
    /// zero. Accepted submissions and exact pending retries return
    /// [`Status::ShouldWait`] without changing the exchange. Success replaces
    /// it with [`AndroidPackageInstallResult`].
    #[cfg(feature = "androidbox-runtime-install2")]
    AndroidPackageInstall = 65,
    /// Reads ABI 55's complete, fixed-capacity installed package directory.
    ///
    /// The authenticated built-in App and Launcher may copy one canonical
    /// [`AndroidPackageDirectory`] into an exact
    /// [`ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE`]-byte writable destination.
    /// The operation is read-only and returns no APK bytes, handles, paths,
    /// pointers, package-store location, or block authority.
    #[cfg(feature = "androidbox-multipackage4")]
    AndroidPackageDirectoryRead = 66,
    /// Reads one package-bound launcher icon from ABI 56's installed directory.
    ///
    /// Built-in App or Launcher supplies an exact writable
    /// [`ANDROID_PACKAGE_ICON_WIRE_SIZE`]-byte destination in `x0`, that size
    /// in `x1`, and a zero-based directory selector in `x2`. The result binds
    /// pixels (or canonical absence) to the directory revision, package
    /// generation, and APK digest. It grants no APK bytes, handle, path,
    /// package-store location, or mutable authority.
    #[cfg(feature = "androidbox-icon-resources5")]
    AndroidPackageIconRead = 67,
}

impl SyscallNumber {
    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            0 => Some(Self::AbiVersion),
            1 => Some(Self::ChannelCreate),
            2 => Some(Self::ChannelWrite),
            3 => Some(Self::ChannelRead),
            4 => Some(Self::HandleDuplicate),
            5 => Some(Self::HandleClose),
            6 => Some(Self::InitReady),
            7 => Some(Self::InitFailed),
            8 => Some(Self::ChannelWriteBytes),
            9 => Some(Self::ChannelReadBytes),
            10 => Some(Self::ProcessSpawn),
            11 => Some(Self::ThreadExit),
            12 => Some(Self::ProcessWait),
            13 => Some(Self::ChannelWriteTransfer),
            14 => Some(Self::ChannelReadTransfer),
            15 => Some(Self::ObjectWait),
            16 => Some(Self::EventCreate),
            17 => Some(Self::EventSignal),
            18 => Some(Self::EventClear),
            19 => Some(Self::ObjectWaitMany),
            20 => Some(Self::ChannelPeek),
            21 => Some(Self::ChannelReadEnvelope),
            22 => Some(Self::ObjectWaitManyArray),
            23 => Some(Self::FileOpenAt),
            24 => Some(Self::VmoRead),
            25 => Some(Self::SurfaceAcquire),
            26 => Some(Self::SurfacePresent),
            27 => Some(Self::SurfaceReadInput),
            28 => Some(Self::GraphicsBufferCreate),
            29 => Some(Self::GraphicsBufferWrite),
            30 => Some(Self::SurfacePresentBuffer),
            31 => Some(Self::ProcessTerminate),
            32 => Some(Self::GraphicsBufferMap),
            33 => Some(Self::GraphicsBufferUnmap),
            34 => Some(Self::GraphicsBufferQueue),
            35 => Some(Self::GraphicsBufferAcquire),
            36 => Some(Self::GraphicsBufferRelease),
            37 => Some(Self::SurfaceFrameAcquire),
            38 => Some(Self::SurfaceReadKey),
            39 => Some(Self::InputAcquire),
            40 => Some(Self::InputReadEvent),
            41 => Some(Self::InputSessionInfo),
            42 => Some(Self::AppDataRootOpen),
            43 => Some(Self::FileReplaceAt),
            44 => Some(Self::DirectoryCreateAt),
            45 => Some(Self::UnlinkAt),
            46 => Some(Self::DirectoryReadAt),
            47 => Some(Self::IpcBufferCreate),
            48 => Some(Self::StorageAcquire),
            49 => Some(Self::StorageSubmit),
            50 => Some(Self::StorageTake),
            51 => Some(Self::StorageConnect),
            52 => Some(Self::StorageAccept),
            53 => Some(Self::SystemShutdown),
            #[cfg(feature = "resident-platform-shutdown-runtime")]
            54 => Some(Self::ServiceShutdown),
            #[cfg(feature = "unified-product-manifest-supervision-runtime")]
            55 => Some(Self::ServiceManifestOpen),
            #[cfg(feature = "unified-product-event-supervision-runtime")]
            56 => Some(Self::ServiceSupervisorReport),
            #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
            57 => Some(Self::MaintenanceSessionOpen),
            #[cfg(feature = "mobile-ui-runtime")]
            58 => Some(Self::SystemClockRead),
            #[cfg(feature = "androidbox-apk-install0")]
            59 => Some(Self::AndroidPackageSnapshotRead),
            #[cfg(feature = "androidbox-apk-install0")]
            60 => Some(Self::AndroidPackageRelaunch),
            #[cfg(feature = "androidbox-el0-runtime0")]
            61 => Some(Self::AndroidPackageImageClaim),
            #[cfg(feature = "androidbox-interactive0")]
            62 => Some(Self::SurfacePresentBufferLayers),
            #[cfg(feature = "androidbox-runtime-uninstall1")]
            63 => Some(Self::AndroidPackageUninstall),
            #[cfg(feature = "androidbox-runtime-install2")]
            64 => Some(Self::AndroidPackageInstallCandidateRead),
            #[cfg(feature = "androidbox-runtime-install2")]
            65 => Some(Self::AndroidPackageInstall),
            #[cfg(feature = "androidbox-multipackage4")]
            66 => Some(Self::AndroidPackageDirectoryRead),
            #[cfg(feature = "androidbox-icon-resources5")]
            67 => Some(Self::AndroidPackageIconRead),
            _ => None,
        }
    }
}

#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Ok = 0,
    InvalidArgument = 1,
    PermissionDenied = 2,
    NotFound = 3,
    AlreadyExists = 4,
    Unavailable = 5,
    Timeout = 6,
    OutOfMemory = 7,
    InvalidState = 8,
    Unsupported = 9,
    ShouldWait = 10,
    PeerClosed = 11,
    BadAddress = 12,
    BufferTooSmall = 13,
    /// A compare-and-swap precondition did not match committed state.
    Conflict = 14,
    /// A durable mutation may or may not have committed; callers must recover.
    OutcomeUnknown = 15,
    /// Persisted metadata or content failed authenticated integrity checks.
    DataCorrupt = 16,
    /// The bounded namespace, entry table, or backing store has no free space.
    NoSpace = 17,
    /// The block transport must be reset before another request is safe.
    RequiresReset = 18,
}

impl Status {
    pub const fn raw(self) -> u64 {
        self as u64
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandleValue(u32);

impl HandleValue {
    pub const INVALID: Self = Self(0);

    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// One canonical hardware-key transition returned entirely in registers.
///
/// The low 16 bits of the state register contain the non-zero Linux input key
/// code and bits 16..=17 contain the value (release, press, or repeat). Every
/// other state bit is reserved and must be zero. The second register is a
/// non-zero, monotonically increasing Surface key-sequence number.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyInputSample {
    code: u16,
    value: u8,
    sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyInputSampleError {
    InvalidCode,
    InvalidValue,
    InvalidSequence,
    NonCanonicalState,
}

impl KeyInputSample {
    pub const VALUE_RELEASE: u8 = 0;
    pub const VALUE_PRESS: u8 = 1;
    pub const VALUE_REPEAT: u8 = 2;
    const CODE_MASK: u64 = u16::MAX as u64;
    const VALUE_SHIFT: u32 = 16;
    const VALUE_MASK: u64 = 0b11 << Self::VALUE_SHIFT;
    const STATE_MASK: u64 = Self::CODE_MASK | Self::VALUE_MASK;

    pub const fn try_new(sequence: u64, code: u16, value: u8) -> Result<Self, KeyInputSampleError> {
        if sequence == 0 {
            return Err(KeyInputSampleError::InvalidSequence);
        }
        if code == 0 {
            return Err(KeyInputSampleError::InvalidCode);
        }
        if value > Self::VALUE_REPEAT {
            return Err(KeyInputSampleError::InvalidValue);
        }
        Ok(Self {
            code,
            value,
            sequence,
        })
    }

    pub const fn code(self) -> u16 {
        self.code
    }

    pub const fn value(self) -> u8 {
        self.value
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub const fn encode_registers(self) -> (u64, u64) {
        (
            self.code as u64 | ((self.value as u64) << Self::VALUE_SHIFT),
            self.sequence,
        )
    }

    pub const fn decode_registers(state: u64, sequence: u64) -> Result<Self, KeyInputSampleError> {
        if state & !Self::STATE_MASK != 0 {
            return Err(KeyInputSampleError::NonCanonicalState);
        }
        let code = (state & Self::CODE_MASK) as u16;
        let value = ((state & Self::VALUE_MASK) >> Self::VALUE_SHIFT) as u8;
        Self::try_new(sequence, code, value)
    }
}

/// Stable discriminants for the system-wide physical-input stream.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEventKind {
    Pointer = 1,
    Key = 2,
}

impl InputEventKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Pointer),
            2 => Some(Self::Key),
            _ => None,
        }
    }
}

/// The validated payload carried by one canonical [`InputEvent`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEventPayload {
    Pointer { x: u16, y: u16, pressed: bool },
    Key { code: u16, value: u8 },
}

/// One canonical system-wide physical-input transition returned in registers.
///
/// The state register uses bits 0..=7 for a non-zero device id, bits 8..=15
/// for [`InputEventKind`], and kind-specific payload bits. A pointer stores its
/// global x coordinate in bits 16..=31, global y in bits 32..=47, and pressed
/// state in bit 48. A key stores its non-zero Linux input code in bits 16..=31
/// and release/press/repeat value in bits 32..=33. All remaining bits are
/// reserved and decoding rejects them. The second register is a non-zero
/// sequence shared by every device and event kind; its global monotonicity is
/// maintained by the input-stream producer.
///
/// Fields are private and construction is validating, so safe callers cannot
/// create an event that has a non-canonical register encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputEvent {
    device_id: u8,
    payload: InputEventPayload,
    sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEventError {
    InvalidDeviceId,
    InvalidKind,
    InvalidSequence,
    CoordinateOutOfBounds,
    InvalidKeyCode,
    InvalidKeyValue,
    NonCanonicalState,
}

impl InputEvent {
    pub const KEY_VALUE_RELEASE: u8 = 0;
    pub const KEY_VALUE_PRESS: u8 = 1;
    pub const KEY_VALUE_REPEAT: u8 = 2;

    const DEVICE_MASK: u64 = u8::MAX as u64;
    const KIND_SHIFT: u32 = 8;
    const KIND_MASK: u64 = (u8::MAX as u64) << Self::KIND_SHIFT;
    const DATA0_SHIFT: u32 = 16;
    const DATA0_MASK: u64 = (u16::MAX as u64) << Self::DATA0_SHIFT;
    const DATA1_SHIFT: u32 = 32;
    const POINTER_DATA1_MASK: u64 = (u16::MAX as u64) << Self::DATA1_SHIFT;
    const POINTER_PRESSED_SHIFT: u32 = 48;
    const POINTER_PRESSED_MASK: u64 = 1 << Self::POINTER_PRESSED_SHIFT;
    const POINTER_STATE_MASK: u64 = Self::DEVICE_MASK
        | Self::KIND_MASK
        | Self::DATA0_MASK
        | Self::POINTER_DATA1_MASK
        | Self::POINTER_PRESSED_MASK;
    const KEY_VALUE_MASK: u64 = 0b11 << Self::DATA1_SHIFT;
    const KEY_STATE_MASK: u64 =
        Self::DEVICE_MASK | Self::KIND_MASK | Self::DATA0_MASK | Self::KEY_VALUE_MASK;

    pub const fn try_pointer(
        sequence: u64,
        device_id: u8,
        x: u16,
        y: u16,
        pressed: bool,
    ) -> Result<Self, InputEventError> {
        if sequence == 0 {
            return Err(InputEventError::InvalidSequence);
        }
        if device_id == 0 {
            return Err(InputEventError::InvalidDeviceId);
        }
        if x >= INPUT_GLOBAL_WIDTH || y >= INPUT_GLOBAL_HEIGHT {
            return Err(InputEventError::CoordinateOutOfBounds);
        }
        Ok(Self {
            device_id,
            payload: InputEventPayload::Pointer { x, y, pressed },
            sequence,
        })
    }

    pub const fn try_key(
        sequence: u64,
        device_id: u8,
        code: u16,
        value: u8,
    ) -> Result<Self, InputEventError> {
        if sequence == 0 {
            return Err(InputEventError::InvalidSequence);
        }
        if device_id == 0 {
            return Err(InputEventError::InvalidDeviceId);
        }
        if code == 0 {
            return Err(InputEventError::InvalidKeyCode);
        }
        if value > Self::KEY_VALUE_REPEAT {
            return Err(InputEventError::InvalidKeyValue);
        }
        Ok(Self {
            device_id,
            payload: InputEventPayload::Key { code, value },
            sequence,
        })
    }

    pub const fn kind(self) -> InputEventKind {
        match self.payload {
            InputEventPayload::Pointer { .. } => InputEventKind::Pointer,
            InputEventPayload::Key { .. } => InputEventKind::Key,
        }
    }

    pub const fn device_id(self) -> u8 {
        self.device_id
    }

    pub const fn payload(self) -> InputEventPayload {
        self.payload
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub const fn pointer(self) -> Option<(u16, u16, bool)> {
        match self.payload {
            InputEventPayload::Pointer { x, y, pressed } => Some((x, y, pressed)),
            InputEventPayload::Key { .. } => None,
        }
    }

    pub const fn key(self) -> Option<(u16, u8)> {
        match self.payload {
            InputEventPayload::Pointer { .. } => None,
            InputEventPayload::Key { code, value } => Some((code, value)),
        }
    }

    pub const fn encode_registers(self) -> (u64, u64) {
        let common = self.device_id as u64 | ((self.kind().raw() as u64) << Self::KIND_SHIFT);
        let payload = match self.payload {
            InputEventPayload::Pointer { x, y, pressed } => {
                ((x as u64) << Self::DATA0_SHIFT)
                    | ((y as u64) << Self::DATA1_SHIFT)
                    | ((pressed as u64) << Self::POINTER_PRESSED_SHIFT)
            }
            InputEventPayload::Key { code, value } => {
                ((code as u64) << Self::DATA0_SHIFT) | ((value as u64) << Self::DATA1_SHIFT)
            }
        };
        (common | payload, self.sequence)
    }

    pub const fn decode_registers(state: u64, sequence: u64) -> Result<Self, InputEventError> {
        let device_id = (state & Self::DEVICE_MASK) as u8;
        let kind = ((state & Self::KIND_MASK) >> Self::KIND_SHIFT) as u8;
        match InputEventKind::from_raw(kind) {
            Some(InputEventKind::Pointer) => {
                if state & !Self::POINTER_STATE_MASK != 0 {
                    return Err(InputEventError::NonCanonicalState);
                }
                Self::try_pointer(
                    sequence,
                    device_id,
                    ((state & Self::DATA0_MASK) >> Self::DATA0_SHIFT) as u16,
                    ((state & Self::POINTER_DATA1_MASK) >> Self::DATA1_SHIFT) as u16,
                    state & Self::POINTER_PRESSED_MASK != 0,
                )
            }
            Some(InputEventKind::Key) => {
                if state & !Self::KEY_STATE_MASK != 0 {
                    return Err(InputEventError::NonCanonicalState);
                }
                Self::try_key(
                    sequence,
                    device_id,
                    ((state & Self::DATA0_MASK) >> Self::DATA0_SHIFT) as u16,
                    ((state & Self::KEY_VALUE_MASK) >> Self::DATA1_SHIFT) as u8,
                )
            }
            None => Err(InputEventError::InvalidKind),
        }
    }
}

/// Packs a transfer message's source handle and byte length into syscall x2.
/// The source handle occupies the high 32 bits; the length occupies the low
/// 32 bits and is validated by the kernel against `CHANNEL_MESSAGE_MAX_BYTES`.
pub const fn pack_transfer(handle: HandleValue, length: u32) -> u64 {
    ((handle.raw() as u64) << 32) | length as u64
}

pub const fn unpack_transfer(packed: u64) -> (HandleValue, u32) {
    (HandleValue::from_raw((packed >> 32) as u32), packed as u32)
}

/// Packs the byte offset and length consumed by [`SyscallNumber::VmoRead`].
pub const fn pack_vmo_read(offset: u32, length: u32) -> u64 {
    ((offset as u64) << 32) | length as u64
}

/// Unpacks a canonical syscall-24 offset/length register pair.
pub const fn unpack_vmo_read(packed: u64) -> (u32, u32) {
    ((packed >> 32) as u32, packed as u32)
}

/// Packs the width and height consumed by [`SyscallNumber::GraphicsBufferCreate`].
pub const fn pack_graphics_buffer_geometry(width: u32, height: u32) -> u64 {
    ((width as u64) << 32) | height as u64
}

/// Unpacks a canonical syscall-28 width/height register pair.
pub const fn unpack_graphics_buffer_geometry(packed: u64) -> (u32, u32) {
    ((packed >> 32) as u32, packed as u32)
}

/// Packs the content and system-chrome handles consumed by
/// [`SyscallNumber::SurfacePresentBufferLayers`].
///
/// The client-content handle occupies the high 32 bits and the
/// SurfaceServer-owned chrome handle occupies the low 32 bits.
#[cfg(feature = "androidbox-interactive0")]
pub const fn pack_surface_layer_handles(content: HandleValue, chrome: HandleValue) -> u64 {
    ((content.raw() as u64) << 32) | chrome.raw() as u64
}

/// Unpacks the two handles consumed by
/// [`SyscallNumber::SurfacePresentBufferLayers`].
#[cfg(feature = "androidbox-interactive0")]
pub const fn unpack_surface_layer_handles(packed: u64) -> (HandleValue, HandleValue) {
    (
        HandleValue::from_raw((packed >> 32) as u32),
        HandleValue::from_raw(packed as u32),
    )
}

/// Packs the byte offset and length consumed by
/// [`SyscallNumber::GraphicsBufferWrite`].
pub const fn pack_graphics_buffer_write(offset: u32, length: u32) -> u64 {
    ((offset as u64) << 32) | length as u64
}

/// Unpacks a canonical syscall-29 graphics-buffer offset/length register pair.
pub const fn unpack_graphics_buffer_write(packed: u64) -> (u32, u32) {
    ((packed >> 32) as u32, packed as u32)
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectSignals(u32);

impl ObjectSignals {
    pub const NONE: Self = Self(0);
    pub const READABLE: Self = Self(1 << 0);
    pub const WRITABLE: Self = Self(1 << 1);
    pub const PEER_CLOSED: Self = Self(1 << 2);
    pub const SIGNALED: Self = Self(1 << 3);
    /// One level-triggered software frame opportunity is ready to acquire.
    pub const FRAME_READY: Self = Self(1 << 4);
    /// One or more hardware-key transitions are queued on a Surface.
    pub const KEY_READY: Self = Self(1 << 5);
    pub const CHANNEL_ALL: Self = Self(Self::READABLE.0 | Self::WRITABLE.0 | Self::PEER_CLOSED.0);
    pub const EVENT_ALL: Self = Self(Self::SIGNALED.0);
    pub const SURFACE_ALL: Self =
        Self(Self::READABLE.0 | Self::PEER_CLOSED.0 | Self::FRAME_READY.0 | Self::KEY_READY.0);
    /// Single-slot BufferQueue acquire/release fence signals.
    pub const GRAPHICS_BUFFER_ALL: Self = Self(Self::READABLE.0 | Self::WRITABLE.0);
    pub const ALL: Self =
        Self(Self::CHANNEL_ALL.0 | Self::EVENT_ALL.0 | Self::FRAME_READY.0 | Self::KEY_READY.0);

    pub const fn from_bits(bits: u32) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub const fn contains(self, required: Self) -> bool {
        self.0 & required.0 == required.0
    }
}

/// Packs one wait item into the canonical `u64` representation.
///
/// The handle occupies the high 32 bits and the requested signal bits occupy
/// the low 32 bits. Legacy `ObjectWaitMany` accepts exactly
/// [`OBJECT_WAIT_MANY_ITEM_COUNT`] such items in registers. Syscall 22,
/// `ObjectWaitManyArray`, reads between one and
/// [`OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS`] contiguous packed items from the user
/// pointer in `x0`; `x1` is the item count and `x2` is the relative timeout in
/// nanoseconds. Each item occupies exactly
/// [`OBJECT_WAIT_MANY_ARRAY_ITEM_WIRE_SIZE`] bytes in little-endian order.
pub const fn pack_wait_item(handle: HandleValue, signals: ObjectSignals) -> u64 {
    ((handle.raw() as u64) << 32) | signals.bits() as u64
}

/// Unpacks one wait item without validating its signal bits.
///
/// The raw bits let the kernel reject unknown signals before applying the
/// object-type-specific signal mask.
pub const fn unpack_wait_item(packed: u64) -> (HandleValue, u32) {
    (HandleValue::from_raw((packed >> 32) as u32), packed as u32)
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rights(u32);

impl Rights {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const DUPLICATE: Self = Self(1 << 2);
    pub const TRANSFER: Self = Self(1 << 3);
    pub const SIGNAL: Self = Self(1 << 4);
    pub const MAP: Self = Self(1 << 5);
    pub const EXECUTE: Self = Self(1 << 6);
    pub const ADMIN: Self = Self(1 << 7);
    pub const WAIT: Self = Self(1 << 8);
    pub const CHANNEL_DEFAULT: Self =
        Self(Self::READ.0 | Self::WRITE.0 | Self::DUPLICATE.0 | Self::TRANSFER.0 | Self::WAIT.0);
    /// ABI 47's AndroidApp startup endpoint is intentionally non-delegable.
    ///
    /// It can exchange bounded bytes and block for peer state, but cannot be
    /// duplicated or transferred to any other process.
    #[cfg(feature = "androidbox-process0")]
    pub const ANDROID_APP_CHANNEL: Self = Self(Self::READ.0 | Self::WRITE.0 | Self::WAIT.0);
    pub const EVENT_DEFAULT: Self =
        Self(Self::DUPLICATE.0 | Self::TRANSFER.0 | Self::SIGNAL.0 | Self::WAIT.0);
    /// Immutable VMO handles can be read, attenuated, and transferred. They
    /// intentionally carry neither WRITE nor MAP until those ABIs exist.
    pub const VMO_DEFAULT: Self = Self(Self::READ.0 | Self::DUPLICATE.0 | Self::TRANSFER.0);
    /// A boot directory is a move-only namespace capability.
    pub const DIRECTORY_DEFAULT: Self = Self(Self::READ.0 | Self::TRANSFER.0);
    /// A principal-bound AppData root may be read, mutated, and attenuated.
    /// It intentionally cannot cross a process boundary through handle transfer.
    pub const APP_DATA_ROOT_DEFAULT: Self = Self(Self::READ.0 | Self::WRITE.0 | Self::DUPLICATE.0);
    /// The singleton StorageServer volume is readable, writable, and waitable.
    /// It cannot be duplicated or transferred to another process.
    pub const STORAGE_VOLUME_DEFAULT: Self = Self(Self::READ.0 | Self::WRITE.0 | Self::WAIT.0);
    /// A client storage session is readable, writable, and waitable.
    /// It cannot be duplicated or transferred to another process.
    pub const STORAGE_SESSION_DEFAULT: Self = Self(Self::READ.0 | Self::WRITE.0 | Self::WAIT.0);
    /// The unique UI producer may present, read input, and wait for reports,
    /// but it cannot duplicate or transfer ownership to another process.
    pub const SURFACE_DEFAULT: Self = Self(Self::READ.0 | Self::WRITE.0 | Self::WAIT.0);
    /// The move-only global-input capability can only be read and waited on.
    pub const INPUT_DEFAULT: Self = Self(Self::READ.0 | Self::WAIT.0);
    /// A graphics-buffer producer can fill, attenuate, and transfer its buffer.
    /// The syscall-only ABI intentionally exposes no memory mapping right.
    pub const GRAPHICS_BUFFER_DEFAULT: Self =
        Self(Self::READ.0 | Self::WRITE.0 | Self::DUPLICATE.0 | Self::TRANSFER.0);
    /// Attenuated graphics-buffer ownership transferred to the surface server.
    /// It can be presented or transferred onward, but cannot be modified.
    pub const GRAPHICS_BUFFER_SERVER: Self = Self(Self::READ.0 | Self::TRANSFER.0);
    /// Opt-in ABI-19 producer rights: direct mapping plus release-fence waits.
    pub const GRAPHICS_BUFFER_MAPPED_PRODUCER: Self = Self(
        Self::READ.0
            | Self::WRITE.0
            | Self::DUPLICATE.0
            | Self::TRANSFER.0
            | Self::MAP.0
            | Self::WAIT.0,
    );
    /// Opt-in ABI-19 SurfaceServer rights: read-only mapping and acquire wait.
    pub const GRAPHICS_BUFFER_MAPPED_SERVER: Self =
        Self(Self::READ.0 | Self::TRANSFER.0 | Self::MAP.0 | Self::WAIT.0);
    pub const ALL: Self = Self(
        Self::READ.0
            | Self::WRITE.0
            | Self::DUPLICATE.0
            | Self::TRANSFER.0
            | Self::SIGNAL.0
            | Self::MAP.0
            | Self::EXECUTE.0
            | Self::ADMIN.0
            | Self::WAIT.0,
    );

    pub const fn from_bits(bits: u32) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, required: Self) -> bool {
        self.0 & required.0 == required.0
    }

    pub const fn is_subset_of(self, parent: Self) -> bool {
        parent.contains(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ABI_VERSION, APP_DATA_DIRECTORY_ENTRY_FLAGS_NONE, APP_DATA_DIRECTORY_ENTRY_MAGIC,
        APP_DATA_DIRECTORY_ENTRY_VERSION, APP_DATA_DIRECTORY_ENTRY_WIRE_SIZE,
        APP_DATA_DIRECTORY_MAX_ENTRIES, APP_DATA_FILE_MAX_BYTES, APP_DATA_PATH_MAX_BYTES,
        APP_DATA_PATH_MAX_DEPTH, APP_DATA_VOLUME_SECTORS, AppDataDirectoryEntry,
        AppDataDirectoryEntryError, AppDataDirectoryEntryKind, AppDataPathError, AppDataPrincipal,
        CHANNEL_MESSAGE_MAX_BYTES, CHANNEL_READ_ENVELOPE_SIZE, CHILD_EXIT_MAGIC,
        ChannelMessageKind, ChannelReadEnvelope, ChannelReadEnvelopeError,
        FILE_REPLACE_FLAG_CREATE_ONLY, FILE_REPLACE_FLAG_EXPECT_EXACT,
        FILE_REPLACE_FLAGS_EXPECT_ANY, FILE_REPLACE_REQUEST_MAGIC, FILE_REPLACE_REQUEST_VERSION,
        FILE_REPLACE_REQUEST_WIRE_SIZE, FileReplaceCas, FileReplaceRequest,
        FileReplaceRequestError, GRAPHICS_BUFFER_ACQUIRE_FLAGS_NONE, GRAPHICS_BUFFER_BACKING_BYTES,
        GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE, GRAPHICS_BUFFER_CREATE_FLAGS_NONE,
        GRAPHICS_BUFFER_FORMAT_XRGB8888, GRAPHICS_BUFFER_HEIGHT, GRAPHICS_BUFFER_LOGICAL_BYTES,
        GRAPHICS_BUFFER_MAP_ADDRESS, GRAPHICS_BUFFER_MAP_CONSUMER_RO,
        GRAPHICS_BUFFER_MAP_PRODUCER_RW, GRAPHICS_BUFFER_MAP_STRIDE, GRAPHICS_BUFFER_PIXEL_COUNT,
        GRAPHICS_BUFFER_QUEUE_FLAGS_NONE, GRAPHICS_BUFFER_RELEASE_FLAGS_NONE,
        GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_WRITE_MAX_BYTES, HandleValue, INPUT_GLOBAL_HEIGHT,
        INPUT_GLOBAL_WIDTH, IPC_BUFFER_CREATE_FLAGS_NONE, IPC_BUFFER_PAYLOAD_MAX_BYTES, InputEvent,
        InputEventError, InputEventKind, InputEventPayload, KeyInputSample, KeyInputSampleError,
        OBJECT_WAIT_MANY_ARRAY_ITEM_WIRE_SIZE, OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS,
        OBJECT_WAIT_MANY_ARRAY_MAX_WIRE_SIZE, OBJECT_WAIT_MANY_ITEM_COUNT,
        OBJECT_WAIT_TIMEOUT_INFINITE, OBJECT_WAIT_TIMEOUT_POLL, ObjectSignals,
        PROCESS_KILLED_EXIT_CODE, PROCESS_SPAWN_FLAGS_NONE, PROCESS_TERMINATE_FLAGS_NONE,
        ProcessTerminationReason, Rights, STORAGE_BLOCK_DATA_MAX_BYTES, STORAGE_BLOCK_MAX_SECTORS,
        STORAGE_BLOCK_REQUEST_MAGIC, STORAGE_BLOCK_REQUEST_UNASSIGNED_TOKEN,
        STORAGE_BLOCK_REQUEST_VERSION, STORAGE_BLOCK_REQUEST_WIRE_SIZE,
        STORAGE_CONNECT_RIGHTS_MASK, STORAGE_SECTOR_SIZE, STORAGE_SESSION_BINDING_MAGIC,
        STORAGE_SESSION_BINDING_VERSION, STORAGE_SESSION_BINDING_WIRE_SIZE,
        SYSTEM_FILE_PATH_MAX_BYTES, Status, StorageBlockOperation, StorageBlockRequest,
        StorageBlockRequestError, StorageSessionBinding, StorageSessionBindingError, SyscallNumber,
        UserImageId, VMO_READ_MAX_BYTES, pack_graphics_buffer_geometry, pack_graphics_buffer_write,
        pack_transfer, pack_vmo_read, pack_wait_item, unpack_graphics_buffer_geometry,
        unpack_graphics_buffer_write, unpack_transfer, unpack_vmo_read, unpack_wait_item,
        validate_app_data_path,
    };
    #[cfg(feature = "androidbox-apk-install0")]
    use super::{
        ANDROID_PACKAGE_ACTIVITY_MAX_BYTES, ANDROID_PACKAGE_APK_MAX_BYTES,
        ANDROID_PACKAGE_NAME_MAX_BYTES, ANDROID_PACKAGE_RELAUNCH_REQUEST_FLAGS_NONE,
        ANDROID_PACKAGE_RELAUNCH_REQUEST_MAGIC, ANDROID_PACKAGE_RELAUNCH_REQUEST_VERSION,
        ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE, ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMAT_PERFORMED,
        ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMATTED, ANDROID_PACKAGE_SNAPSHOT_FLAG_INSTALLED,
        ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_PRESENT, ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_USED,
        ANDROID_PACKAGE_SNAPSHOT_MAGIC, ANDROID_PACKAGE_SNAPSHOT_VERSION,
        ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE, ANDROID_PACKAGE_TEXT_MAX_BYTES,
        ANDROID_PACKAGE_TITLE_MAX_BYTES, AndroidPackageCompatibilityProfile,
        AndroidPackageInstalledMetadata, AndroidPackageIoCounters, AndroidPackageRelaunchRequest,
        AndroidPackageRelaunchRequestError, AndroidPackageSnapshot, AndroidPackageSnapshotError,
        AndroidPackageSnapshotState,
    };
    #[cfg(feature = "androidbox-multipackage4")]
    use super::{
        ANDROID_PACKAGE_DIRECTORY_CAPACITY, ANDROID_PACKAGE_DIRECTORY_FLAG_FORMATTED,
        ANDROID_PACKAGE_DIRECTORY_MAGIC, ANDROID_PACKAGE_DIRECTORY_VERSION,
        ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE, AndroidPackageDirectory, AndroidPackageDirectoryError,
    };
    #[cfg(feature = "androidbox-density-icons7")]
    use super::{
        ANDROID_PACKAGE_ICON_FLAG_COLOR_QUANTIZED, ANDROID_PACKAGE_ICON_FLAG_DENSITY_NORMALIZED,
        ANDROID_PACKAGE_ICON_SOURCE_MDP_DENSITY_DPI, ANDROID_PACKAGE_ICON_SOURCE_MDP_HEIGHT,
        ANDROID_PACKAGE_ICON_SOURCE_MDP_WIDTH,
    };
    #[cfg(feature = "androidbox-icon-resources5")]
    use super::{
        ANDROID_PACKAGE_ICON_FLAG_PRESENT, ANDROID_PACKAGE_ICON_HEIGHT, ANDROID_PACKAGE_ICON_MAGIC,
        ANDROID_PACKAGE_ICON_PIXEL_COUNT, ANDROID_PACKAGE_ICON_VERSION, ANDROID_PACKAGE_ICON_WIDTH,
        ANDROID_PACKAGE_ICON_WIRE_SIZE, AndroidPackageIcon, AndroidPackageIconError,
        AndroidPackageIconMetadata,
    };
    #[cfg(feature = "androidbox-el0-runtime0")]
    use super::{
        ANDROID_PACKAGE_IMAGE_CLAIM_FLAGS_NONE, ANDROID_PACKAGE_IMAGE_CLAIM_MAGIC,
        ANDROID_PACKAGE_IMAGE_CLAIM_VERSION, ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE,
        AndroidPackageImageClaim, AndroidPackageImageClaimError,
    };
    #[cfg(feature = "androidbox-runtime-install2")]
    use super::{
        ANDROID_PACKAGE_INSTALL_CANDIDATE_MAGIC, ANDROID_PACKAGE_INSTALL_CANDIDATE_WIRE_SIZE,
        ANDROID_PACKAGE_INSTALL_REQUEST_MAGIC, ANDROID_PACKAGE_INSTALL_RESULT_MAGIC,
        ANDROID_PACKAGE_INSTALL_WIRE_SIZE, ANDROID_PACKAGE_INSTALL_WIRE_VERSION,
        AndroidPackageInstallAction, AndroidPackageInstallCandidate,
        AndroidPackageInstallCandidateError, AndroidPackageInstallCandidateMetadata,
        AndroidPackageInstallRequest, AndroidPackageInstallRequestError,
        AndroidPackageInstallResult, AndroidPackageInstallResultError,
    };
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    use super::{
        ANDROID_PACKAGE_UNINSTALL_DATA_KEEP_NONE, ANDROID_PACKAGE_UNINSTALL_REQUEST_MAGIC,
        ANDROID_PACKAGE_UNINSTALL_RESULT_MAGIC, ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE,
        ANDROID_PACKAGE_UNINSTALL_WIRE_VERSION, AndroidPackageUninstallRequest,
        AndroidPackageUninstallRequestError, AndroidPackageUninstallResult,
        AndroidPackageUninstallResultError,
    };

    #[test]
    fn syscall_numbers_are_stable_and_unknown_values_are_rejected() {
        #[cfg(feature = "androidbox-layout-mixed19")]
        assert_eq!(ABI_VERSION, 69);
        #[cfg(all(
            feature = "androidbox-layout-size18",
            not(feature = "androidbox-layout-mixed19")
        ))]
        assert_eq!(ABI_VERSION, 68);
        #[cfg(all(
            feature = "androidbox-layout-directional17",
            not(feature = "androidbox-layout-size18")
        ))]
        assert_eq!(ABI_VERSION, 67);
        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        assert_eq!(ABI_VERSION, 66);
        #[cfg(all(
            feature = "androidbox-layout-weight15",
            not(feature = "androidbox-layout-spacing16")
        ))]
        assert_eq!(ABI_VERSION, 65);
        #[cfg(all(
            feature = "androidbox-layout-row14",
            not(feature = "androidbox-layout-weight15")
        ))]
        assert_eq!(ABI_VERSION, 64);
        #[cfg(all(
            feature = "androidbox-string-builder13",
            not(feature = "androidbox-layout-row14")
        ))]
        assert_eq!(ABI_VERSION, 63);
        #[cfg(all(
            feature = "androidbox-string-text12",
            not(feature = "androidbox-string-builder13")
        ))]
        assert_eq!(ABI_VERSION, 62);
        #[cfg(all(
            feature = "androidbox-activity-state11",
            not(feature = "androidbox-string-text12")
        ))]
        assert_eq!(ABI_VERSION, 61);
        #[cfg(all(
            feature = "androidbox-activity-fields10",
            not(feature = "androidbox-activity-state11")
        ))]
        assert_eq!(ABI_VERSION, 60);
        #[cfg(all(
            feature = "androidbox-dex-instance9",
            not(feature = "androidbox-activity-fields10")
        ))]
        assert_eq!(ABI_VERSION, 59);
        #[cfg(all(
            feature = "androidbox-dex-methods8",
            not(feature = "androidbox-dex-instance9")
        ))]
        assert_eq!(ABI_VERSION, 58);
        #[cfg(all(
            feature = "androidbox-density-icons7",
            not(feature = "androidbox-dex-methods8")
        ))]
        assert_eq!(ABI_VERSION, 57);
        #[cfg(all(
            feature = "androidbox-icon-resources5",
            not(feature = "androidbox-density-icons7")
        ))]
        assert_eq!(ABI_VERSION, 56);
        #[cfg(all(
            feature = "androidbox-multipackage4",
            not(feature = "androidbox-icon-resources5")
        ))]
        assert_eq!(ABI_VERSION, 55);
        #[cfg(all(
            feature = "androidbox-manifest-catalog3",
            not(feature = "androidbox-multipackage4")
        ))]
        assert_eq!(ABI_VERSION, 54);
        #[cfg(all(
            feature = "androidbox-runtime-install2",
            not(feature = "androidbox-manifest-catalog3")
        ))]
        assert_eq!(ABI_VERSION, 53);
        #[cfg(all(
            feature = "androidbox-runtime-uninstall1",
            not(feature = "androidbox-runtime-install2")
        ))]
        assert_eq!(ABI_VERSION, 52);
        #[cfg(all(
            feature = "androidbox-apk-envelope4",
            not(feature = "androidbox-runtime-uninstall1")
        ))]
        assert_eq!(ABI_VERSION, 51);
        #[cfg(all(
            feature = "androidbox-multiaction3",
            not(feature = "androidbox-apk-envelope4")
        ))]
        assert_eq!(ABI_VERSION, 50);
        #[cfg(all(
            feature = "androidbox-scene-rpc2",
            not(feature = "androidbox-multiaction3")
        ))]
        assert_eq!(ABI_VERSION, 49);
        #[cfg(all(
            feature = "androidbox-restart0",
            not(feature = "androidbox-scene-rpc2")
        ))]
        assert_eq!(ABI_VERSION, 48);
        #[cfg(all(feature = "androidbox-process0", not(feature = "androidbox-restart0")))]
        assert_eq!(ABI_VERSION, 47);
        #[cfg(all(
            feature = "androidbox-interactive0",
            not(feature = "androidbox-process0")
        ))]
        assert_eq!(ABI_VERSION, 46);
        #[cfg(all(
            feature = "androidbox-el0-runtime0",
            not(feature = "androidbox-interactive0")
        ))]
        assert_eq!(ABI_VERSION, 45);
        #[cfg(all(
            feature = "androidbox-apk-install0",
            not(feature = "androidbox-el0-runtime0")
        ))]
        assert_eq!(ABI_VERSION, 44);
        #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
        assert_eq!(ABI_VERSION, 42);
        #[cfg(all(
            feature = "unified-product-maintenance-plan-runtime",
            not(feature = "unified-product-signed-maintenance-plan-runtime")
        ))]
        assert_eq!(ABI_VERSION, 41);
        #[cfg(all(
            feature = "unified-product-maintenance-step-runtime",
            not(feature = "unified-product-maintenance-plan-runtime")
        ))]
        assert_eq!(ABI_VERSION, 40);
        #[cfg(all(
            feature = "unified-product-maintenance-execution-runtime",
            not(feature = "unified-product-maintenance-step-runtime")
        ))]
        assert_eq!(ABI_VERSION, 39);
        #[cfg(all(
            feature = "unified-product-maintenance-authorization-runtime",
            not(feature = "unified-product-maintenance-execution-runtime")
        ))]
        assert_eq!(ABI_VERSION, 38);
        #[cfg(all(
            feature = "unified-product-key-rotation-runtime",
            not(feature = "unified-product-maintenance-authorization-runtime")
        ))]
        assert_eq!(ABI_VERSION, 37);
        #[cfg(all(
            feature = "unified-product-persistent-rollback-runtime",
            not(feature = "unified-product-key-rotation-runtime")
        ))]
        assert_eq!(ABI_VERSION, 36);
        #[cfg(all(
            feature = "unified-product-verified-manifest-runtime",
            not(feature = "unified-product-persistent-rollback-runtime")
        ))]
        assert_eq!(ABI_VERSION, 35);
        #[cfg(all(
            feature = "unified-product-event-supervision-runtime",
            not(feature = "unified-product-verified-manifest-runtime")
        ))]
        assert_eq!(ABI_VERSION, 34);
        #[cfg(all(
            feature = "unified-product-manifest-supervision-runtime",
            not(feature = "unified-product-event-supervision-runtime")
        ))]
        assert_eq!(ABI_VERSION, 33);
        #[cfg(all(
            feature = "unified-product-continuous-supervision-runtime",
            not(feature = "unified-product-manifest-supervision-runtime")
        ))]
        assert_eq!(ABI_VERSION, 32);
        #[cfg(all(
            feature = "unified-product-psci-shutdown-runtime",
            not(feature = "unified-product-continuous-supervision-runtime")
        ))]
        assert_eq!(ABI_VERSION, 31);
        #[cfg(all(
            feature = "unified-product-multiservice-liveness-runtime",
            not(feature = "unified-product-psci-shutdown-runtime")
        ))]
        assert_eq!(ABI_VERSION, 30);
        #[cfg(all(
            feature = "unified-product-liveness-runtime",
            not(feature = "unified-product-multiservice-liveness-runtime")
        ))]
        assert_eq!(ABI_VERSION, 29);
        #[cfg(all(
            feature = "unified-product-runtime",
            not(feature = "unified-product-liveness-runtime")
        ))]
        assert_eq!(ABI_VERSION, 28);
        #[cfg(all(
            feature = "resident-platform-shutdown-runtime",
            not(feature = "unified-product-runtime")
        ))]
        assert_eq!(ABI_VERSION, 27);
        #[cfg(all(
            not(feature = "resident-platform-shutdown-runtime"),
            feature = "storage-server-shutdown-orchestration-runtime"
        ))]
        assert_eq!(ABI_VERSION, 26);
        #[cfg(all(
            not(feature = "resident-platform-shutdown-runtime"),
            not(feature = "storage-server-shutdown-orchestration-runtime"),
            feature = "storage-server-runtime"
        ))]
        assert_eq!(ABI_VERSION, 25);
        #[cfg(all(not(feature = "storage-server-runtime"), feature = "app-data-runtime"))]
        assert_eq!(ABI_VERSION, 24);
        #[cfg(all(
            not(feature = "androidbox-apk-install0"),
            not(feature = "storage-server-runtime"),
            not(feature = "app-data-runtime"),
            feature = "mobile-ui-runtime"
        ))]
        assert_eq!(ABI_VERSION, 24);
        #[cfg(not(any(
            feature = "storage-server-runtime",
            feature = "app-data-runtime",
            feature = "mobile-ui-runtime"
        )))]
        assert_eq!(ABI_VERSION, 23);
        let stable = [
            SyscallNumber::AbiVersion,
            SyscallNumber::ChannelCreate,
            SyscallNumber::ChannelWrite,
            SyscallNumber::ChannelRead,
            SyscallNumber::HandleDuplicate,
            SyscallNumber::HandleClose,
            SyscallNumber::InitReady,
            SyscallNumber::InitFailed,
            SyscallNumber::ChannelWriteBytes,
            SyscallNumber::ChannelReadBytes,
            SyscallNumber::ProcessSpawn,
            SyscallNumber::ThreadExit,
            SyscallNumber::ProcessWait,
            SyscallNumber::ChannelWriteTransfer,
            SyscallNumber::ChannelReadTransfer,
            SyscallNumber::ObjectWait,
            SyscallNumber::EventCreate,
            SyscallNumber::EventSignal,
            SyscallNumber::EventClear,
            SyscallNumber::ObjectWaitMany,
            SyscallNumber::ChannelPeek,
            SyscallNumber::ChannelReadEnvelope,
            SyscallNumber::ObjectWaitManyArray,
            SyscallNumber::FileOpenAt,
            SyscallNumber::VmoRead,
            SyscallNumber::SurfaceAcquire,
            SyscallNumber::SurfacePresent,
            SyscallNumber::SurfaceReadInput,
            SyscallNumber::GraphicsBufferCreate,
            SyscallNumber::GraphicsBufferWrite,
            SyscallNumber::SurfacePresentBuffer,
            SyscallNumber::ProcessTerminate,
            SyscallNumber::GraphicsBufferMap,
            SyscallNumber::GraphicsBufferUnmap,
            SyscallNumber::GraphicsBufferQueue,
            SyscallNumber::GraphicsBufferAcquire,
            SyscallNumber::GraphicsBufferRelease,
            SyscallNumber::SurfaceFrameAcquire,
            SyscallNumber::SurfaceReadKey,
            SyscallNumber::InputAcquire,
            SyscallNumber::InputReadEvent,
            SyscallNumber::InputSessionInfo,
            SyscallNumber::AppDataRootOpen,
            SyscallNumber::FileReplaceAt,
            SyscallNumber::DirectoryCreateAt,
            SyscallNumber::UnlinkAt,
            SyscallNumber::DirectoryReadAt,
            SyscallNumber::IpcBufferCreate,
            SyscallNumber::StorageAcquire,
            SyscallNumber::StorageSubmit,
            SyscallNumber::StorageTake,
            SyscallNumber::StorageConnect,
            SyscallNumber::StorageAccept,
            SyscallNumber::SystemShutdown,
            #[cfg(feature = "resident-platform-shutdown-runtime")]
            SyscallNumber::ServiceShutdown,
            #[cfg(feature = "unified-product-manifest-supervision-runtime")]
            SyscallNumber::ServiceManifestOpen,
            #[cfg(feature = "unified-product-event-supervision-runtime")]
            SyscallNumber::ServiceSupervisorReport,
            #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
            SyscallNumber::MaintenanceSessionOpen,
        ];
        for (raw, expected) in stable.into_iter().enumerate() {
            assert_eq!(SyscallNumber::from_raw(raw as u64), Some(expected));
            assert_eq!(expected as u64, raw as u64);
        }
        #[cfg(feature = "mobile-ui-runtime")]
        {
            assert_eq!(SyscallNumber::SystemClockRead as u64, 58);
            assert_eq!(
                SyscallNumber::from_raw(58),
                Some(SyscallNumber::SystemClockRead)
            );
        }
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!(SyscallNumber::from_raw(58), None);
        #[cfg(feature = "androidbox-apk-install0")]
        {
            assert_eq!(SyscallNumber::AndroidPackageSnapshotRead as u64, 59);
            assert_eq!(
                SyscallNumber::from_raw(59),
                Some(SyscallNumber::AndroidPackageSnapshotRead)
            );
            assert_eq!(SyscallNumber::AndroidPackageRelaunch as u64, 60);
            assert_eq!(
                SyscallNumber::from_raw(60),
                Some(SyscallNumber::AndroidPackageRelaunch)
            );
        }
        #[cfg(feature = "androidbox-el0-runtime0")]
        {
            assert_eq!(SyscallNumber::AndroidPackageImageClaim as u64, 61);
            assert_eq!(
                SyscallNumber::from_raw(61),
                Some(SyscallNumber::AndroidPackageImageClaim)
            );
        }
        #[cfg(not(feature = "androidbox-el0-runtime0"))]
        assert_eq!(SyscallNumber::from_raw(61), None);
        #[cfg(feature = "androidbox-interactive0")]
        {
            assert_eq!(SyscallNumber::SurfacePresentBufferLayers as u64, 62);
            assert_eq!(
                SyscallNumber::from_raw(62),
                Some(SyscallNumber::SurfacePresentBufferLayers)
            );
        }
        #[cfg(not(feature = "androidbox-interactive0"))]
        assert_eq!(SyscallNumber::from_raw(62), None);
        #[cfg(feature = "androidbox-runtime-uninstall1")]
        {
            assert_eq!(SyscallNumber::AndroidPackageUninstall as u64, 63);
            assert_eq!(
                SyscallNumber::from_raw(63),
                Some(SyscallNumber::AndroidPackageUninstall)
            );
        }
        #[cfg(not(feature = "androidbox-runtime-uninstall1"))]
        assert_eq!(SyscallNumber::from_raw(63), None);
        #[cfg(feature = "androidbox-runtime-install2")]
        {
            assert_eq!(SyscallNumber::AndroidPackageInstallCandidateRead as u64, 64);
            assert_eq!(SyscallNumber::AndroidPackageInstall as u64, 65);
            assert_eq!(
                SyscallNumber::from_raw(64),
                Some(SyscallNumber::AndroidPackageInstallCandidateRead)
            );
            assert_eq!(
                SyscallNumber::from_raw(65),
                Some(SyscallNumber::AndroidPackageInstall)
            );
        }
        #[cfg(not(feature = "androidbox-runtime-install2"))]
        {
            assert_eq!(SyscallNumber::from_raw(64), None);
            assert_eq!(SyscallNumber::from_raw(65), None);
        }
        #[cfg(feature = "androidbox-multipackage4")]
        {
            assert_eq!(SyscallNumber::AndroidPackageDirectoryRead as u64, 66);
            assert_eq!(
                SyscallNumber::from_raw(66),
                Some(SyscallNumber::AndroidPackageDirectoryRead)
            );
        }
        #[cfg(not(feature = "androidbox-multipackage4"))]
        assert_eq!(SyscallNumber::from_raw(66), None);
        #[cfg(not(feature = "androidbox-apk-install0"))]
        {
            assert_eq!(SyscallNumber::from_raw(59), None);
            assert_eq!(SyscallNumber::from_raw(60), None);
        }
        #[cfg(all(
            feature = "unified-product-event-supervision-runtime",
            not(feature = "unified-product-maintenance-authorization-runtime")
        ))]
        assert_eq!(SyscallNumber::from_raw(57), None);
        #[cfg(all(
            feature = "unified-product-manifest-supervision-runtime",
            not(feature = "unified-product-event-supervision-runtime")
        ))]
        assert_eq!(SyscallNumber::from_raw(56), None);
        #[cfg(all(
            feature = "resident-platform-shutdown-runtime",
            not(feature = "unified-product-manifest-supervision-runtime")
        ))]
        assert_eq!(SyscallNumber::from_raw(55), None);
        #[cfg(not(feature = "resident-platform-shutdown-runtime"))]
        assert_eq!(SyscallNumber::from_raw(54), None);
        assert_eq!(SyscallNumber::from_raw(u64::MAX), None);
        assert_eq!(CHILD_EXIT_MAGIC, 0xc11d_e817_baad_f00d);
    }

    #[cfg(feature = "mobile-ui-runtime")]
    #[test]
    fn mobile_system_clock_contract_is_stable_and_read_only() {
        assert_eq!(super::SYSTEM_CLOCK_READ_FLAGS_NONE, 0);
        assert_eq!(super::SYSTEM_CLOCK_SOURCE_QEMU_PL031, 1);
        assert_eq!(SyscallNumber::SystemClockRead as u64, 58);
        assert_eq!(
            SyscallNumber::from_raw(58),
            Some(SyscallNumber::SystemClockRead)
        );
    }

    #[cfg(feature = "androidbox-apk-install0")]
    fn installed_snapshot_metadata() -> AndroidPackageInstalledMetadata<'static> {
        AndroidPackageInstalledMetadata {
            generation: 7,
            version_code: 12,
            apk_length: 12_566,
            resources_table_crc32: 0x1122_3344,
            layout_xml_crc32: 0x5566_7788,
            layout_resource_id: 0x7f02_0000,
            text_resource_id: 0x7f03_0000,
            instruction_count: 4,
            apk_sha256: [0x11; 32],
            signer_sha256: [0x22; 32],
            package_name: b"dev.bndroid.resources",
            activity_name: b"dev.bndroid.resources.MainActivity",
            title: b"AndroidBox Resources",
            text: b"Hello from a signed resource APK",
        }
    }

    #[cfg(feature = "androidbox-apk-install0")]
    fn installed_snapshot() -> AndroidPackageSnapshot {
        AndroidPackageSnapshot::installed(
            AndroidPackageSnapshotState::new(true, true, true, true),
            AndroidPackageIoCounters::new(19, 9, 3),
            installed_snapshot_metadata(),
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-multipackage4")]
    fn directory_snapshot(
        package_name: &'static [u8],
        activity_name: &'static [u8],
        generation: u64,
        hash_byte: u8,
    ) -> AndroidPackageSnapshot {
        AndroidPackageSnapshot::installed(
            AndroidPackageSnapshotState::new(true, false, false, false),
            AndroidPackageIoCounters::new(0, 0, 0),
            AndroidPackageInstalledMetadata {
                generation,
                version_code: generation + 10,
                apk_length: 12_566,
                resources_table_crc32: 0x1122_3344,
                layout_xml_crc32: 0x5566_7788,
                layout_resource_id: 0x7f02_0000,
                text_resource_id: 0x7f03_0000,
                instruction_count: 4,
                apk_sha256: [hash_byte; 32],
                signer_sha256: [0x52; 32],
                package_name,
                activity_name,
                title: b"Installed Android app",
                text: b"Ready to launch",
            },
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-multipackage4")]
    #[test]
    fn android_package_directory_round_trips_two_entries_and_empty_volume() {
        let first = directory_snapshot(
            b"org.bndroid.catalog",
            b"org.bndroid.catalog.HomeActivity",
            1,
            0x11,
        );
        let second = directory_snapshot(
            b"org.bndroid.notes",
            b"org.bndroid.notes.MainActivity",
            3,
            0x22,
        );
        let directory = AndroidPackageDirectory::new(true, 9, &[first, second]).unwrap();
        assert!(directory.formatted());
        assert_eq!(directory.revision(), 9);
        assert_eq!(directory.count(), 2);
        assert_eq!(
            directory.entry(0).unwrap().package_name_bytes(),
            b"org.bndroid.catalog"
        );
        assert_eq!(
            directory.entry(1).unwrap().package_name_bytes(),
            b"org.bndroid.notes"
        );
        assert_eq!(directory.entry(2), None);

        let wire = directory.encode();
        assert_eq!(ANDROID_PACKAGE_DIRECTORY_CAPACITY, 2);
        assert_eq!(ANDROID_PACKAGE_DIRECTORY_WIRE_SIZE, 1_344);
        assert_eq!(&wire[..8], &ANDROID_PACKAGE_DIRECTORY_MAGIC);
        assert_eq!(
            u32::from_le_bytes(wire[8..12].try_into().unwrap()),
            ANDROID_PACKAGE_DIRECTORY_VERSION
        );
        assert_eq!(
            u32::from_le_bytes(wire[12..16].try_into().unwrap()),
            ANDROID_PACKAGE_DIRECTORY_FLAG_FORMATTED
        );
        assert_eq!(AndroidPackageDirectory::decode(&wire), Ok(directory));

        let empty = AndroidPackageDirectory::new(false, 0, &[]).unwrap();
        assert_eq!(AndroidPackageDirectory::decode(&empty.encode()), Ok(empty));
    }

    #[cfg(feature = "androidbox-multipackage4")]
    #[test]
    fn android_package_directory_rejects_capacity_identity_and_transient_state() {
        let first = directory_snapshot(
            b"org.bndroid.catalog",
            b"org.bndroid.catalog.HomeActivity",
            1,
            0x11,
        );
        let second = directory_snapshot(
            b"org.bndroid.notes",
            b"org.bndroid.notes.MainActivity",
            2,
            0x22,
        );
        let third = directory_snapshot(
            b"org.bndroid.clock",
            b"org.bndroid.clock.MainActivity",
            3,
            0x33,
        );
        assert_eq!(
            AndroidPackageDirectory::new(true, 1, &[first, second, third]),
            Err(AndroidPackageDirectoryError::InvalidCount)
        );
        assert_eq!(
            AndroidPackageDirectory::new(true, 1, &[first, first]),
            Err(AndroidPackageDirectoryError::DuplicatePackage)
        );
        assert_eq!(
            AndroidPackageDirectory::new(false, 0, &[first]),
            Err(AndroidPackageDirectoryError::InconsistentUnformattedState)
        );
        assert_eq!(
            AndroidPackageDirectory::new(true, 0, &[]),
            Err(AndroidPackageDirectoryError::InvalidRevision)
        );

        let source_entry = AndroidPackageSnapshot::installed(
            AndroidPackageSnapshotState::new(true, true, false, false),
            AndroidPackageIoCounters::new(0, 0, 0),
            installed_snapshot_metadata(),
        )
        .unwrap();
        assert_eq!(
            AndroidPackageDirectory::new(true, 1, &[source_entry]),
            Err(AndroidPackageDirectoryError::EntryCarriesSourceState)
        );
        let io_entry = AndroidPackageSnapshot::installed(
            AndroidPackageSnapshotState::new(true, false, false, false),
            AndroidPackageIoCounters::new(1, 0, 0),
            installed_snapshot_metadata(),
        )
        .unwrap();
        assert_eq!(
            AndroidPackageDirectory::new(true, 1, &[io_entry]),
            Err(AndroidPackageDirectoryError::EntryCarriesIoCounters)
        );
    }

    #[cfg(feature = "androidbox-multipackage4")]
    #[test]
    fn android_package_directory_decoder_rejects_every_noncanonical_field_class() {
        let first = directory_snapshot(
            b"org.bndroid.catalog",
            b"org.bndroid.catalog.HomeActivity",
            1,
            0x11,
        );
        let canonical = AndroidPackageDirectory::new(true, 4, &[first])
            .unwrap()
            .encode();
        assert_eq!(
            AndroidPackageDirectory::decode(&canonical[..canonical.len() - 1]),
            Err(AndroidPackageDirectoryError::InvalidWireLength)
        );

        for (offset, expected) in [
            (0, AndroidPackageDirectoryError::InvalidMagic),
            (8, AndroidPackageDirectoryError::UnsupportedVersion),
            (12, AndroidPackageDirectoryError::UnknownFlags),
            (26, AndroidPackageDirectoryError::InvalidCapacity),
            (28, AndroidPackageDirectoryError::NonZeroReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0x80;
            assert_eq!(AndroidPackageDirectory::decode(&wire), Err(expected));
        }
        let mut bad_count = canonical;
        bad_count[24..26].copy_from_slice(&3_u16.to_le_bytes());
        assert_eq!(
            AndroidPackageDirectory::decode(&bad_count),
            Err(AndroidPackageDirectoryError::InvalidCount)
        );
        let mut zero_revision = canonical;
        zero_revision[16..24].fill(0);
        assert_eq!(
            AndroidPackageDirectory::decode(&zero_revision),
            Err(AndroidPackageDirectoryError::InvalidRevision)
        );
        let mut unused_entry = canonical;
        unused_entry[64 + super::ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE] = 1;
        assert_eq!(
            AndroidPackageDirectory::decode(&unused_entry),
            Err(AndroidPackageDirectoryError::NonCanonicalUnusedEntry)
        );
        let mut invalid_nested = canonical;
        invalid_nested[64] ^= 0x80;
        assert_eq!(
            AndroidPackageDirectory::decode(&invalid_nested),
            Err(AndroidPackageDirectoryError::InvalidEntry(
                AndroidPackageSnapshotError::InvalidMagic
            ))
        );
    }

    #[cfg(feature = "androidbox-icon-resources5")]
    fn package_icon() -> AndroidPackageIcon {
        let mut pixels = [0; ANDROID_PACKAGE_ICON_PIXEL_COUNT];
        pixels[0] = 0xff0b_57d0;
        pixels[1] = 0xffff_ffff;
        AndroidPackageIcon::new(
            9,
            1,
            3,
            [0x44; 32],
            Some(AndroidPackageIconMetadata {
                resource_id: 0x7f01_0000,
                png_crc32: 0xf63d_0b72,
                pixels,
                #[cfg(feature = "androidbox-density-icons7")]
                source_width: ANDROID_PACKAGE_ICON_WIDTH as u16,
                #[cfg(feature = "androidbox-density-icons7")]
                source_height: ANDROID_PACKAGE_ICON_HEIGHT as u16,
                #[cfg(feature = "androidbox-density-icons7")]
                source_density_dpi: 0,
                #[cfg(feature = "androidbox-density-icons7")]
                color_quantized: false,
            }),
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-icon-resources5")]
    #[test]
    fn android_package_icon_wire_is_exact_and_round_trips_presence_and_absence() {
        let icon = package_icon();
        let wire = icon.encode();
        assert_eq!(ANDROID_PACKAGE_ICON_WIRE_SIZE, 1_152);
        assert_eq!(&wire[..8], &ANDROID_PACKAGE_ICON_MAGIC);
        assert_eq!(
            u32::from_le_bytes(wire[8..12].try_into().unwrap()),
            ANDROID_PACKAGE_ICON_VERSION
        );
        assert_eq!(
            u32::from_le_bytes(wire[12..16].try_into().unwrap()),
            ANDROID_PACKAGE_ICON_FLAG_PRESENT
        );
        assert_eq!(u16::from_le_bytes(wire[26..28].try_into().unwrap()), 16);
        assert_eq!(u16::from_le_bytes(wire[28..30].try_into().unwrap()), 16);
        assert_eq!(AndroidPackageIcon::decode(&wire), Ok(icon));

        let absent = AndroidPackageIcon::new(10, 0, 4, [0x55; 32], None).unwrap();
        let absent_wire = absent.encode();
        #[cfg(feature = "androidbox-density-icons7")]
        assert!(absent_wire[72..1_110].iter().all(|byte| *byte == 0));
        #[cfg(not(feature = "androidbox-density-icons7"))]
        assert!(absent_wire[72..1_104].iter().all(|byte| *byte == 0));
        assert_eq!(AndroidPackageIcon::decode(&absent_wire), Ok(absent));
    }

    #[cfg(feature = "androidbox-density-icons7")]
    #[test]
    fn android_package_icon_v2_carries_normalization_and_quantization_provenance() {
        let mut pixels = [0; ANDROID_PACKAGE_ICON_PIXEL_COUNT];
        pixels.fill(0xff12_3456);
        let icon = AndroidPackageIcon::new(
            11,
            0,
            5,
            [0x66; 32],
            Some(AndroidPackageIconMetadata {
                resource_id: 0x7f01_0000,
                png_crc32: 0xfc4d_4dc6,
                pixels,
                source_width: ANDROID_PACKAGE_ICON_SOURCE_MDP_WIDTH,
                source_height: ANDROID_PACKAGE_ICON_SOURCE_MDP_HEIGHT,
                source_density_dpi: ANDROID_PACKAGE_ICON_SOURCE_MDP_DENSITY_DPI,
                color_quantized: true,
            }),
        )
        .unwrap();
        let wire = icon.encode();
        assert_eq!(
            u32::from_le_bytes(wire[12..16].try_into().unwrap()),
            ANDROID_PACKAGE_ICON_FLAG_PRESENT
                | ANDROID_PACKAGE_ICON_FLAG_DENSITY_NORMALIZED
                | ANDROID_PACKAGE_ICON_FLAG_COLOR_QUANTIZED
        );
        assert_eq!(
            u16::from_le_bytes(wire[1_104..1_106].try_into().unwrap()),
            ANDROID_PACKAGE_ICON_SOURCE_MDP_WIDTH
        );
        assert_eq!(
            u16::from_le_bytes(wire[1_106..1_108].try_into().unwrap()),
            ANDROID_PACKAGE_ICON_SOURCE_MDP_HEIGHT
        );
        assert_eq!(
            u16::from_le_bytes(wire[1_108..1_110].try_into().unwrap()),
            ANDROID_PACKAGE_ICON_SOURCE_MDP_DENSITY_DPI
        );
        assert_eq!(AndroidPackageIcon::decode(&wire), Ok(icon));
    }

    #[cfg(feature = "androidbox-icon-resources5")]
    #[test]
    fn android_package_icon_decoder_rejects_noncanonical_wire_classes() {
        let canonical = package_icon().encode();
        assert_eq!(
            AndroidPackageIcon::decode(&canonical[..canonical.len() - 1]),
            Err(AndroidPackageIconError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, AndroidPackageIconError::InvalidMagic),
            (8, AndroidPackageIconError::UnsupportedVersion),
            (12, AndroidPackageIconError::UnknownFlags),
            (30, AndroidPackageIconError::NonZeroReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0x80;
            assert_eq!(AndroidPackageIcon::decode(&wire), Err(expected));
        }
        #[cfg(feature = "androidbox-density-icons7")]
        {
            let mut wire = canonical;
            wire[1_110] ^= 0x80;
            assert_eq!(
                AndroidPackageIcon::decode(&wire),
                Err(AndroidPackageIconError::NonZeroReserved)
            );
            let mut bad_source = canonical;
            bad_source[1_104..1_106].copy_from_slice(&17_u16.to_le_bytes());
            assert_eq!(
                AndroidPackageIcon::decode(&bad_source),
                Err(AndroidPackageIconError::InvalidSourceMetadata)
            );
        }
        #[cfg(not(feature = "androidbox-density-icons7"))]
        {
            let mut wire = canonical;
            wire[1_104] ^= 0x80;
            assert_eq!(
                AndroidPackageIcon::decode(&wire),
                Err(AndroidPackageIconError::NonZeroReserved)
            );
        }

        for (range, expected) in [
            (16..24, AndroidPackageIconError::InvalidDirectoryRevision),
            (32..40, AndroidPackageIconError::InvalidGeneration),
            (40..72, AndroidPackageIconError::ZeroApkSha256),
            (72..76, AndroidPackageIconError::InvalidResourceId),
            (76..80, AndroidPackageIconError::InvalidPngCrc32),
            (80..1_104, AndroidPackageIconError::EmptyPixels),
        ] {
            let mut wire = canonical;
            wire[range].fill(0);
            assert_eq!(AndroidPackageIcon::decode(&wire), Err(expected));
        }

        let mut bad_selector = canonical;
        bad_selector[24..26].copy_from_slice(&2_u16.to_le_bytes());
        assert_eq!(
            AndroidPackageIcon::decode(&bad_selector),
            Err(AndroidPackageIconError::InvalidSelector)
        );
        let mut bad_dimensions = canonical;
        bad_dimensions[26..28].copy_from_slice(&15_u16.to_le_bytes());
        assert_eq!(
            AndroidPackageIcon::decode(&bad_dimensions),
            Err(AndroidPackageIconError::InvalidDimensions)
        );
        let mut noncanonical_transparent = canonical;
        noncanonical_transparent[80..84].copy_from_slice(&0x0000_0001_u32.to_le_bytes());
        assert_eq!(
            AndroidPackageIcon::decode(&noncanonical_transparent),
            Err(AndroidPackageIconError::NonCanonicalTransparentPixel)
        );

        let mut absent_with_pixels = AndroidPackageIcon::new(10, 0, 4, [0x55; 32], None)
            .unwrap()
            .encode();
        absent_with_pixels[80] = 1;
        assert_eq!(
            AndroidPackageIcon::decode(&absent_with_pixels),
            Err(AndroidPackageIconError::NonCanonicalAbsentIcon)
        );
    }

    #[cfg(feature = "androidbox-apk-install0")]
    fn relaunch_request() -> AndroidPackageRelaunchRequest {
        AndroidPackageRelaunchRequest::new(
            41,
            7,
            12,
            12_566,
            [0x11; 32],
            [0x22; 32],
            b"dev.bndroid.resources",
            b"dev.bndroid.resources.MainActivity",
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-apk-install0")]
    #[test]
    fn android_package_relaunch_request_wire_is_stable_and_round_trips() {
        #[cfg(feature = "androidbox-layout-mixed19")]
        assert_eq!(ABI_VERSION, 69);
        #[cfg(all(
            feature = "androidbox-layout-size18",
            not(feature = "androidbox-layout-mixed19")
        ))]
        assert_eq!(ABI_VERSION, 68);
        #[cfg(all(
            feature = "androidbox-layout-directional17",
            not(feature = "androidbox-layout-size18")
        ))]
        assert_eq!(ABI_VERSION, 67);
        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        assert_eq!(ABI_VERSION, 66);
        #[cfg(all(
            feature = "androidbox-layout-weight15",
            not(feature = "androidbox-layout-spacing16")
        ))]
        assert_eq!(ABI_VERSION, 65);
        #[cfg(all(
            feature = "androidbox-layout-row14",
            not(feature = "androidbox-layout-weight15")
        ))]
        assert_eq!(ABI_VERSION, 64);
        #[cfg(all(
            feature = "androidbox-string-builder13",
            not(feature = "androidbox-layout-row14")
        ))]
        assert_eq!(ABI_VERSION, 63);
        #[cfg(all(
            feature = "androidbox-string-text12",
            not(feature = "androidbox-string-builder13")
        ))]
        assert_eq!(ABI_VERSION, 62);
        #[cfg(all(
            feature = "androidbox-activity-state11",
            not(feature = "androidbox-string-text12")
        ))]
        assert_eq!(ABI_VERSION, 61);
        #[cfg(all(
            feature = "androidbox-activity-fields10",
            not(feature = "androidbox-activity-state11")
        ))]
        assert_eq!(ABI_VERSION, 60);
        #[cfg(all(
            feature = "androidbox-dex-instance9",
            not(feature = "androidbox-activity-fields10")
        ))]
        assert_eq!(ABI_VERSION, 59);
        #[cfg(all(
            feature = "androidbox-dex-methods8",
            not(feature = "androidbox-dex-instance9")
        ))]
        assert_eq!(ABI_VERSION, 58);
        #[cfg(all(
            feature = "androidbox-density-icons7",
            not(feature = "androidbox-dex-methods8")
        ))]
        assert_eq!(ABI_VERSION, 57);
        #[cfg(all(
            feature = "androidbox-icon-resources5",
            not(feature = "androidbox-density-icons7")
        ))]
        assert_eq!(ABI_VERSION, 56);
        #[cfg(all(
            feature = "androidbox-multipackage4",
            not(feature = "androidbox-icon-resources5")
        ))]
        assert_eq!(ABI_VERSION, 55);
        #[cfg(all(
            feature = "androidbox-manifest-catalog3",
            not(feature = "androidbox-multipackage4")
        ))]
        assert_eq!(ABI_VERSION, 54);
        #[cfg(all(
            feature = "androidbox-runtime-install2",
            not(feature = "androidbox-manifest-catalog3")
        ))]
        assert_eq!(ABI_VERSION, 53);
        #[cfg(all(
            feature = "androidbox-runtime-uninstall1",
            not(feature = "androidbox-runtime-install2")
        ))]
        assert_eq!(ABI_VERSION, 52);
        #[cfg(all(
            feature = "androidbox-apk-envelope4",
            not(feature = "androidbox-runtime-uninstall1")
        ))]
        assert_eq!(ABI_VERSION, 51);
        #[cfg(all(
            feature = "androidbox-multiaction3",
            not(feature = "androidbox-apk-envelope4")
        ))]
        assert_eq!(ABI_VERSION, 50);
        #[cfg(all(
            feature = "androidbox-scene-rpc2",
            not(feature = "androidbox-multiaction3")
        ))]
        assert_eq!(ABI_VERSION, 49);
        #[cfg(all(
            feature = "androidbox-restart0",
            not(feature = "androidbox-scene-rpc2")
        ))]
        assert_eq!(ABI_VERSION, 48);
        #[cfg(all(feature = "androidbox-process0", not(feature = "androidbox-restart0")))]
        assert_eq!(ABI_VERSION, 47);
        #[cfg(all(
            feature = "androidbox-interactive0",
            not(feature = "androidbox-process0")
        ))]
        assert_eq!(ABI_VERSION, 46);
        #[cfg(all(
            feature = "androidbox-el0-runtime0",
            not(feature = "androidbox-interactive0")
        ))]
        assert_eq!(ABI_VERSION, 45);
        #[cfg(not(feature = "androidbox-el0-runtime0"))]
        assert_eq!(ABI_VERSION, 44);
        assert_eq!(ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE, 640);
        assert_eq!(
            ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE,
            ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE
        );
        assert_eq!(ANDROID_PACKAGE_RELAUNCH_REQUEST_MAGIC, *b"BNDARQ01");
        assert_eq!(ANDROID_PACKAGE_RELAUNCH_REQUEST_VERSION, 1);
        assert_eq!(ANDROID_PACKAGE_RELAUNCH_REQUEST_FLAGS_NONE, 0);

        let request = relaunch_request();
        assert_eq!(request.request_sequence(), 41);
        assert_eq!(request.expected_generation(), 7);
        assert_eq!(request.expected_version_code(), 12);
        assert_eq!(request.apk_length(), 12_566);
        assert_eq!(
            request.profile(),
            AndroidPackageCompatibilityProfile::Resources1
        );
        assert_eq!(request.apk_sha256(), &[0x11; 32]);
        assert_eq!(request.signer_sha256(), &[0x22; 32]);
        assert_eq!(request.package_name(), "dev.bndroid.resources");
        assert_eq!(
            request.activity_name(),
            "dev.bndroid.resources.MainActivity"
        );
        assert_eq!(request.package_name_bytes(), b"dev.bndroid.resources");
        assert_eq!(
            request.activity_name_bytes(),
            b"dev.bndroid.resources.MainActivity"
        );

        let wire = request.encode();
        assert_eq!(&wire[0..8], b"BNDARQ01");
        assert_eq!(&wire[8..12], &1_u32.to_le_bytes());
        assert_eq!(&wire[12..16], &[0; 4]);
        assert_eq!(&wire[16..24], &41_u64.to_le_bytes());
        assert_eq!(&wire[24..32], &7_u64.to_le_bytes());
        assert_eq!(&wire[32..40], &12_u64.to_le_bytes());
        assert_eq!(&wire[40..44], &12_566_u32.to_le_bytes());
        assert_eq!(
            &wire[44..46],
            &(b"dev.bndroid.resources".len() as u16).to_le_bytes()
        );
        assert_eq!(
            &wire[46..48],
            &(b"dev.bndroid.resources.MainActivity".len() as u16).to_le_bytes()
        );
        assert_eq!(&wire[48..50], &2_u16.to_le_bytes());
        assert_eq!(&wire[50..56], &[0; 6]);
        assert_eq!(&wire[56..88], &[0x11; 32]);
        assert_eq!(&wire[88..120], &[0x22; 32]);
        assert_eq!(
            &wire[120..120 + b"dev.bndroid.resources".len()],
            b"dev.bndroid.resources"
        );
        assert!(
            wire[120 + b"dev.bndroid.resources".len()..216]
                .iter()
                .all(|byte| *byte == 0)
        );
        assert_eq!(
            &wire[216..216 + b"dev.bndroid.resources.MainActivity".len()],
            b"dev.bndroid.resources.MainActivity"
        );
        assert!(
            wire[216 + b"dev.bndroid.resources.MainActivity".len()..344]
                .iter()
                .all(|byte| *byte == 0)
        );
        assert!(wire[344..640].iter().all(|byte| *byte == 0));
        assert_eq!(AndroidPackageRelaunchRequest::decode(&wire), Ok(request));

        let maximum_package = [b'~'; ANDROID_PACKAGE_NAME_MAX_BYTES];
        let maximum_activity = [b' '; ANDROID_PACKAGE_ACTIVITY_MAX_BYTES];
        let maximum = AndroidPackageRelaunchRequest::new(
            u64::MAX,
            u64::MAX,
            u64::MAX,
            ANDROID_PACKAGE_APK_MAX_BYTES,
            [0xff; 32],
            [1; 32],
            &maximum_package,
            &maximum_activity,
        )
        .unwrap();
        assert_eq!(maximum.request_sequence(), u64::MAX);
        assert_eq!(maximum.expected_generation(), u64::MAX);
        assert_eq!(maximum.expected_version_code(), u64::MAX);
        assert_eq!(maximum.apk_length(), ANDROID_PACKAGE_APK_MAX_BYTES);
        assert_eq!(maximum.package_name_bytes(), &maximum_package);
        assert_eq!(maximum.activity_name_bytes(), &maximum_activity);
        assert_eq!(
            AndroidPackageRelaunchRequest::decode(&maximum.encode()),
            Ok(maximum)
        );
    }

    #[cfg(feature = "androidbox-runtime-uninstall1")]
    fn runtime_uninstall_request() -> AndroidPackageUninstallRequest {
        AndroidPackageUninstallRequest::new(
            9,
            0x5255_4e54_494d_4501,
            7,
            12,
            12_566,
            [0x11; 32],
            [0x22; 32],
            b"dev.bndroid.resources",
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-runtime-uninstall1")]
    #[test]
    fn android_package_runtime_uninstall_wires_are_canonical_and_disjoint() {
        #[cfg(feature = "androidbox-layout-mixed19")]
        assert_eq!(ABI_VERSION, 69);
        #[cfg(all(
            feature = "androidbox-layout-size18",
            not(feature = "androidbox-layout-mixed19")
        ))]
        assert_eq!(ABI_VERSION, 68);
        #[cfg(all(
            feature = "androidbox-layout-directional17",
            not(feature = "androidbox-layout-size18")
        ))]
        assert_eq!(ABI_VERSION, 67);
        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        assert_eq!(ABI_VERSION, 66);
        #[cfg(all(
            feature = "androidbox-layout-weight15",
            not(feature = "androidbox-layout-spacing16")
        ))]
        assert_eq!(ABI_VERSION, 65);
        #[cfg(all(
            feature = "androidbox-layout-row14",
            not(feature = "androidbox-layout-weight15")
        ))]
        assert_eq!(ABI_VERSION, 64);
        #[cfg(all(
            feature = "androidbox-string-builder13",
            not(feature = "androidbox-layout-row14")
        ))]
        assert_eq!(ABI_VERSION, 63);
        #[cfg(all(
            feature = "androidbox-string-text12",
            not(feature = "androidbox-string-builder13")
        ))]
        assert_eq!(ABI_VERSION, 62);
        #[cfg(all(
            feature = "androidbox-activity-state11",
            not(feature = "androidbox-string-text12")
        ))]
        assert_eq!(ABI_VERSION, 61);
        #[cfg(all(
            feature = "androidbox-activity-fields10",
            not(feature = "androidbox-activity-state11")
        ))]
        assert_eq!(ABI_VERSION, 60);
        #[cfg(all(
            feature = "androidbox-dex-instance9",
            not(feature = "androidbox-activity-fields10")
        ))]
        assert_eq!(ABI_VERSION, 59);
        #[cfg(all(
            feature = "androidbox-dex-methods8",
            not(feature = "androidbox-dex-instance9")
        ))]
        assert_eq!(ABI_VERSION, 58);
        #[cfg(all(
            feature = "androidbox-density-icons7",
            not(feature = "androidbox-dex-methods8")
        ))]
        assert_eq!(ABI_VERSION, 57);
        #[cfg(all(
            feature = "androidbox-icon-resources5",
            not(feature = "androidbox-density-icons7")
        ))]
        assert_eq!(ABI_VERSION, 56);
        #[cfg(all(
            feature = "androidbox-multipackage4",
            not(feature = "androidbox-icon-resources5")
        ))]
        assert_eq!(ABI_VERSION, 55);
        #[cfg(all(
            feature = "androidbox-manifest-catalog3",
            not(feature = "androidbox-multipackage4")
        ))]
        assert_eq!(ABI_VERSION, 54);
        #[cfg(all(
            feature = "androidbox-runtime-install2",
            not(feature = "androidbox-manifest-catalog3")
        ))]
        assert_eq!(ABI_VERSION, 53);
        #[cfg(not(feature = "androidbox-runtime-install2"))]
        assert_eq!(ABI_VERSION, 52);
        assert_eq!(ANDROID_PACKAGE_UNINSTALL_WIRE_SIZE, 256);
        assert_eq!(ANDROID_PACKAGE_UNINSTALL_REQUEST_MAGIC, *b"BNDURQ01");
        assert_eq!(ANDROID_PACKAGE_UNINSTALL_RESULT_MAGIC, *b"BNDURT01");
        assert_eq!(ANDROID_PACKAGE_UNINSTALL_WIRE_VERSION, 1);
        assert_eq!(ANDROID_PACKAGE_UNINSTALL_DATA_KEEP_NONE, 1);

        let request = runtime_uninstall_request();
        let wire = request.encode();
        assert_eq!(&wire[..8], b"BNDURQ01");
        assert_eq!(&wire[216..], &[0; 40]);
        assert_eq!(AndroidPackageUninstallRequest::decode(&wire), Ok(request));
        assert_eq!(
            AndroidPackageUninstallResult::decode(&wire),
            Err(AndroidPackageUninstallResultError::InvalidMagic)
        );

        let result = AndroidPackageUninstallResult::new(
            request.request_sequence(),
            request.operation_id(),
            8,
            7,
            AndroidPackageIoCounters::new(4, 2, 1),
        )
        .unwrap();
        let result_wire = result.encode();
        assert_eq!(&result_wire[..8], b"BNDURT01");
        assert_eq!(&result_wire[74..], &[0; 182]);
        assert_eq!(
            AndroidPackageUninstallResult::decode(&result_wire),
            Ok(result)
        );
        assert_eq!(
            AndroidPackageUninstallRequest::decode(&result_wire),
            Err(AndroidPackageUninstallRequestError::InvalidMagic)
        );
    }

    #[cfg(feature = "androidbox-runtime-uninstall1")]
    #[test]
    fn android_package_runtime_uninstall_rejects_mutated_identity_and_result() {
        let canonical = runtime_uninstall_request().encode();
        for (offset, expected) in [
            (0, AndroidPackageUninstallRequestError::InvalidMagic),
            (8, AndroidPackageUninstallRequestError::UnsupportedVersion),
            (12, AndroidPackageUninstallRequestError::NonZeroFlags),
            (
                16,
                AndroidPackageUninstallRequestError::InvalidRequestSequence,
            ),
            (24, AndroidPackageUninstallRequestError::InvalidOperationId),
            (
                32,
                AndroidPackageUninstallRequestError::InvalidExpectedGeneration,
            ),
            (
                40,
                AndroidPackageUninstallRequestError::InvalidExpectedVersionCode,
            ),
            (48, AndroidPackageUninstallRequestError::InvalidApkLength),
            (
                52,
                AndroidPackageUninstallRequestError::InvalidPackageNameLength,
            ),
            (54, AndroidPackageUninstallRequestError::InvalidProfile),
            (56, AndroidPackageUninstallRequestError::ZeroApkSha256),
            (88, AndroidPackageUninstallRequestError::ZeroSignerSha256),
            (
                120,
                AndroidPackageUninstallRequestError::NonPrintablePackageName,
            ),
            (216, AndroidPackageUninstallRequestError::NonZeroReserved),
        ] {
            let mut wire = canonical;
            match offset {
                0 => wire[..8].copy_from_slice(b"BADMAGIC"),
                8 | 12 | 48 => wire[offset..offset + 4].fill(0),
                16 | 24 | 32 | 40 => wire[offset..offset + 8].fill(0),
                52 | 54 => wire[offset..offset + 2].fill(0),
                56 | 88 => wire[offset..offset + 32].fill(0),
                120 => wire[offset] = 0,
                216 => wire[offset] = 1,
                _ => unreachable!(),
            }
            if offset == 12 {
                wire[offset] = 1;
            }
            assert_eq!(AndroidPackageUninstallRequest::decode(&wire), Err(expected));
        }

        let result =
            AndroidPackageUninstallResult::new(1, 2, 8, 7, AndroidPackageIoCounters::new(3, 2, 1))
                .unwrap();
        let mut bad_transition = result.encode();
        bad_transition[32..40].copy_from_slice(&9_u64.to_le_bytes());
        assert_eq!(
            AndroidPackageUninstallResult::decode(&bad_transition),
            Err(AndroidPackageUninstallResultError::InvalidGenerationTransition)
        );
        let mut bad_reserved = result.encode();
        bad_reserved[255] = 1;
        assert_eq!(
            AndroidPackageUninstallResult::decode(&bad_reserved),
            Err(AndroidPackageUninstallResultError::NonZeroReserved)
        );
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    fn runtime_install_candidate() -> AndroidPackageInstallCandidate {
        AndroidPackageInstallCandidate::new(AndroidPackageInstallCandidateMetadata {
            candidate_id: 0x4341_4e44_4944_4154,
            action: AndroidPackageInstallAction::Install,
            expected_generation: 0,
            expected_version_code: 0,
            version_code: 2,
            apk_length: 12_566,
            resources_table_crc32: 0x1122_3344,
            layout_xml_crc32: 0x5566_7788,
            layout_resource_id: 0x7f01_0001,
            text_resource_id: 0x7f02_0001,
            instruction_count: 7,
            apk_sha256: [0x11; 32],
            signer_sha256: [0x22; 32],
            package_name: b"dev.bndroid.resources",
            activity_name: b"Ldev/bndroid/resources/MainActivity;",
            title: b"Ready to install",
            text: b"Local Android APK",
        })
        .unwrap()
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    fn runtime_install_request() -> AndroidPackageInstallRequest {
        let candidate = runtime_install_candidate();
        AndroidPackageInstallRequest::new(
            1,
            0x494e_5354_414c_4c01,
            candidate.candidate_id(),
            candidate.action().unwrap(),
            candidate.expected_generation(),
            candidate.expected_version_code(),
            candidate.version_code(),
            candidate.apk_length(),
            *candidate.apk_sha256(),
            *candidate.signer_sha256(),
            candidate.package_name_bytes(),
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    #[test]
    fn android_package_runtime_install_wires_are_canonical_and_disjoint() {
        #[cfg(feature = "androidbox-layout-mixed19")]
        assert_eq!(ABI_VERSION, 69);
        #[cfg(all(
            feature = "androidbox-layout-size18",
            not(feature = "androidbox-layout-mixed19")
        ))]
        assert_eq!(ABI_VERSION, 68);
        #[cfg(all(
            feature = "androidbox-layout-directional17",
            not(feature = "androidbox-layout-size18")
        ))]
        assert_eq!(ABI_VERSION, 67);
        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        assert_eq!(ABI_VERSION, 66);
        #[cfg(all(
            feature = "androidbox-layout-weight15",
            not(feature = "androidbox-layout-spacing16")
        ))]
        assert_eq!(ABI_VERSION, 65);
        #[cfg(all(
            feature = "androidbox-layout-row14",
            not(feature = "androidbox-layout-weight15")
        ))]
        assert_eq!(ABI_VERSION, 64);
        #[cfg(all(
            feature = "androidbox-string-builder13",
            not(feature = "androidbox-layout-row14")
        ))]
        assert_eq!(ABI_VERSION, 63);
        #[cfg(all(
            feature = "androidbox-string-text12",
            not(feature = "androidbox-string-builder13")
        ))]
        assert_eq!(ABI_VERSION, 62);
        #[cfg(all(
            feature = "androidbox-activity-state11",
            not(feature = "androidbox-string-text12")
        ))]
        assert_eq!(ABI_VERSION, 61);
        #[cfg(all(
            feature = "androidbox-activity-fields10",
            not(feature = "androidbox-activity-state11")
        ))]
        assert_eq!(ABI_VERSION, 60);
        #[cfg(all(
            feature = "androidbox-dex-instance9",
            not(feature = "androidbox-activity-fields10")
        ))]
        assert_eq!(ABI_VERSION, 59);
        #[cfg(all(
            feature = "androidbox-dex-methods8",
            not(feature = "androidbox-dex-instance9")
        ))]
        assert_eq!(ABI_VERSION, 58);
        #[cfg(all(
            feature = "androidbox-density-icons7",
            not(feature = "androidbox-dex-methods8")
        ))]
        assert_eq!(ABI_VERSION, 57);
        #[cfg(all(
            feature = "androidbox-icon-resources5",
            not(feature = "androidbox-density-icons7")
        ))]
        assert_eq!(ABI_VERSION, 56);
        #[cfg(all(
            feature = "androidbox-multipackage4",
            not(feature = "androidbox-icon-resources5")
        ))]
        assert_eq!(ABI_VERSION, 55);
        #[cfg(all(
            feature = "androidbox-manifest-catalog3",
            not(feature = "androidbox-multipackage4")
        ))]
        assert_eq!(ABI_VERSION, 54);
        #[cfg(not(feature = "androidbox-manifest-catalog3"))]
        assert_eq!(ABI_VERSION, 53);
        assert_eq!(ANDROID_PACKAGE_INSTALL_CANDIDATE_WIRE_SIZE, 640);
        assert_eq!(ANDROID_PACKAGE_INSTALL_WIRE_SIZE, 256);
        assert_eq!(ANDROID_PACKAGE_INSTALL_CANDIDATE_MAGIC, *b"BNDICS01");
        assert_eq!(ANDROID_PACKAGE_INSTALL_REQUEST_MAGIC, *b"BNDIRQ01");
        assert_eq!(ANDROID_PACKAGE_INSTALL_RESULT_MAGIC, *b"BNDIRT01");
        assert_eq!(ANDROID_PACKAGE_INSTALL_WIRE_VERSION, 1);

        let empty = AndroidPackageInstallCandidate::empty();
        let empty_wire = empty.encode();
        assert_eq!(
            AndroidPackageInstallCandidate::decode(&empty_wire),
            Ok(empty)
        );
        assert!(!empty.present());

        let candidate = runtime_install_candidate();
        let candidate_wire = candidate.encode();
        assert_eq!(&candidate_wire[..8], b"BNDICS01");
        assert_eq!(
            AndroidPackageInstallCandidate::decode(&candidate_wire),
            Ok(candidate)
        );
        assert!(candidate.present());
        assert_eq!(
            candidate.action(),
            Some(AndroidPackageInstallAction::Install)
        );
        assert_eq!(candidate.package_name(), "dev.bndroid.resources");

        let request = runtime_install_request();
        let request_wire = request.encode();
        assert_eq!(&request_wire[..8], b"BNDIRQ01");
        assert_eq!(&request_wire[236..], &[0; 20]);
        assert_eq!(
            AndroidPackageInstallRequest::decode(&request_wire),
            Ok(request)
        );
        assert_eq!(
            AndroidPackageInstallResult::decode(&request_wire),
            Err(AndroidPackageInstallResultError::InvalidMagic)
        );

        let result = AndroidPackageInstallResult::new(
            request.request_sequence(),
            request.operation_id(),
            request.candidate_id(),
            0,
            1,
            request.version_code(),
            request.apk_length(),
            request.action().unwrap(),
            AndroidPackageIoCounters::new(1_032, 130, 3),
            *request.apk_sha256(),
            *request.signer_sha256(),
        )
        .unwrap();
        let result_wire = result.encode();
        assert_eq!(&result_wire[..8], b"BNDIRT01");
        assert_eq!(&result_wire[160..], &[0; 96]);
        assert_eq!(
            AndroidPackageInstallResult::decode(&result_wire),
            Ok(result)
        );
        assert_eq!(
            AndroidPackageInstallRequest::decode(&result_wire),
            Err(AndroidPackageInstallRequestError::InvalidMagic)
        );
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    #[test]
    fn android_package_runtime_install_rejects_stale_or_noncanonical_shapes() {
        let mut bad_candidate = runtime_install_candidate().encode();
        bad_candidate[72..74].fill(0);
        assert_eq!(
            AndroidPackageInstallCandidate::decode(&bad_candidate),
            Err(AndroidPackageInstallCandidateError::InvalidAction)
        );
        let mut bad_empty = AndroidPackageInstallCandidate::empty().encode();
        bad_empty[88] = 1;
        assert_eq!(
            AndroidPackageInstallCandidate::decode(&bad_empty),
            Err(AndroidPackageInstallCandidateError::NonCanonicalEmptyCandidate)
        );
        let mut bad_candidate_reserved = runtime_install_candidate().encode();
        bad_candidate_reserved[82] = 1;
        assert_eq!(
            AndroidPackageInstallCandidate::decode(&bad_candidate_reserved),
            Err(AndroidPackageInstallCandidateError::NonZeroReserved)
        );

        let canonical_request = runtime_install_request().encode();
        let mut bad_request_action = canonical_request;
        bad_request_action[68..70].fill(0);
        assert_eq!(
            AndroidPackageInstallRequest::decode(&bad_request_action),
            Err(AndroidPackageInstallRequestError::InvalidAction)
        );
        let mut bad_request_digest = canonical_request;
        bad_request_digest[76..108].fill(0);
        assert_eq!(
            AndroidPackageInstallRequest::decode(&bad_request_digest),
            Err(AndroidPackageInstallRequestError::ZeroApkSha256)
        );
        let mut bad_request_reserved = canonical_request;
        bad_request_reserved[255] = 1;
        assert_eq!(
            AndroidPackageInstallRequest::decode(&bad_request_reserved),
            Err(AndroidPackageInstallRequestError::NonZeroReserved)
        );

        let result = AndroidPackageInstallResult::new(
            1,
            2,
            3,
            0,
            1,
            2,
            12_566,
            AndroidPackageInstallAction::Install,
            AndroidPackageIoCounters::new(3, 2, 1),
            [0x11; 32],
            [0x22; 32],
        )
        .unwrap();
        let mut bad_transition = result.encode();
        bad_transition[48..56].copy_from_slice(&2_u64.to_le_bytes());
        assert_eq!(
            AndroidPackageInstallResult::decode(&bad_transition),
            Err(AndroidPackageInstallResultError::InvalidGenerationTransition)
        );
        let mut bad_result_reserved = result.encode();
        bad_result_reserved[255] = 1;
        assert_eq!(
            AndroidPackageInstallResult::decode(&bad_result_reserved),
            Err(AndroidPackageInstallResultError::NonZeroReserved)
        );
    }

    #[cfg(feature = "androidbox-el0-runtime0")]
    #[test]
    fn android_package_image_claim_is_canonical_and_rejects_replay_shape_changes() {
        #[cfg(feature = "androidbox-layout-mixed19")]
        assert_eq!(ABI_VERSION, 69);
        #[cfg(all(
            feature = "androidbox-layout-size18",
            not(feature = "androidbox-layout-mixed19")
        ))]
        assert_eq!(ABI_VERSION, 68);
        #[cfg(all(
            feature = "androidbox-layout-directional17",
            not(feature = "androidbox-layout-size18")
        ))]
        assert_eq!(ABI_VERSION, 67);
        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        assert_eq!(ABI_VERSION, 66);
        #[cfg(all(
            feature = "androidbox-layout-weight15",
            not(feature = "androidbox-layout-spacing16")
        ))]
        assert_eq!(ABI_VERSION, 65);
        #[cfg(all(
            feature = "androidbox-layout-row14",
            not(feature = "androidbox-layout-weight15")
        ))]
        assert_eq!(ABI_VERSION, 64);
        #[cfg(all(
            feature = "androidbox-string-builder13",
            not(feature = "androidbox-layout-row14")
        ))]
        assert_eq!(ABI_VERSION, 63);
        #[cfg(all(
            feature = "androidbox-string-text12",
            not(feature = "androidbox-string-builder13")
        ))]
        assert_eq!(ABI_VERSION, 62);
        #[cfg(all(
            feature = "androidbox-activity-state11",
            not(feature = "androidbox-string-text12")
        ))]
        assert_eq!(ABI_VERSION, 61);
        #[cfg(all(
            feature = "androidbox-activity-fields10",
            not(feature = "androidbox-activity-state11")
        ))]
        assert_eq!(ABI_VERSION, 60);
        #[cfg(all(
            feature = "androidbox-dex-instance9",
            not(feature = "androidbox-activity-fields10")
        ))]
        assert_eq!(ABI_VERSION, 59);
        #[cfg(all(
            feature = "androidbox-dex-methods8",
            not(feature = "androidbox-dex-instance9")
        ))]
        assert_eq!(ABI_VERSION, 58);
        #[cfg(all(
            feature = "androidbox-density-icons7",
            not(feature = "androidbox-dex-methods8")
        ))]
        assert_eq!(ABI_VERSION, 57);
        #[cfg(all(
            feature = "androidbox-icon-resources5",
            not(feature = "androidbox-density-icons7")
        ))]
        assert_eq!(ABI_VERSION, 56);
        #[cfg(all(
            feature = "androidbox-multipackage4",
            not(feature = "androidbox-icon-resources5")
        ))]
        assert_eq!(ABI_VERSION, 55);
        #[cfg(all(
            feature = "androidbox-manifest-catalog3",
            not(feature = "androidbox-multipackage4")
        ))]
        assert_eq!(ABI_VERSION, 54);
        #[cfg(all(
            feature = "androidbox-runtime-install2",
            not(feature = "androidbox-manifest-catalog3")
        ))]
        assert_eq!(ABI_VERSION, 53);
        #[cfg(all(
            feature = "androidbox-runtime-uninstall1",
            not(feature = "androidbox-runtime-install2")
        ))]
        assert_eq!(ABI_VERSION, 52);
        #[cfg(all(
            feature = "androidbox-apk-envelope4",
            not(feature = "androidbox-runtime-uninstall1")
        ))]
        assert_eq!(ABI_VERSION, 51);
        #[cfg(all(
            feature = "androidbox-multiaction3",
            not(feature = "androidbox-apk-envelope4")
        ))]
        assert_eq!(ABI_VERSION, 50);
        #[cfg(all(
            feature = "androidbox-scene-rpc2",
            not(feature = "androidbox-multiaction3")
        ))]
        assert_eq!(ABI_VERSION, 49);
        #[cfg(all(
            feature = "androidbox-restart0",
            not(feature = "androidbox-scene-rpc2")
        ))]
        assert_eq!(ABI_VERSION, 48);
        #[cfg(all(feature = "androidbox-process0", not(feature = "androidbox-restart0")))]
        assert_eq!(ABI_VERSION, 47);
        #[cfg(all(
            feature = "androidbox-interactive0",
            not(feature = "androidbox-process0")
        ))]
        assert_eq!(ABI_VERSION, 46);
        #[cfg(not(feature = "androidbox-interactive0"))]
        assert_eq!(ABI_VERSION, 45);
        assert_eq!(ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE, 128);
        assert_eq!(ANDROID_PACKAGE_IMAGE_CLAIM_MAGIC, *b"BNDACM01");
        assert_eq!(ANDROID_PACKAGE_IMAGE_CLAIM_VERSION, 1);
        assert_eq!(ANDROID_PACKAGE_IMAGE_CLAIM_FLAGS_NONE, 0);

        let claim = AndroidPackageImageClaim::new(41, 7, 12_566, [0x11; 32], [0x22; 32]).unwrap();
        assert_eq!(claim.compatible_session_id(), 41);
        assert_eq!(claim.package_generation(), 7);
        assert_eq!(claim.apk_length(), 12_566);
        assert_eq!(claim.apk_sha256(), &[0x11; 32]);
        assert_eq!(claim.signer_sha256(), &[0x22; 32]);
        assert_eq!(
            claim.profile(),
            AndroidPackageCompatibilityProfile::Resources1
        );

        let wire = claim.encode();
        assert_eq!(&wire[0..8], b"BNDACM01");
        assert_eq!(&wire[8..12], &1_u32.to_le_bytes());
        assert_eq!(&wire[12..16], &[0; 4]);
        assert_eq!(&wire[16..24], &41_u64.to_le_bytes());
        assert_eq!(&wire[24..32], &7_u64.to_le_bytes());
        assert_eq!(&wire[32..36], &12_566_u32.to_le_bytes());
        assert_eq!(&wire[36..38], &2_u16.to_le_bytes());
        assert_eq!(&wire[38..40], &[0; 2]);
        assert_eq!(&wire[40..72], &[0x11; 32]);
        assert_eq!(&wire[72..104], &[0x22; 32]);
        assert!(wire[104..].iter().all(|byte| *byte == 0));
        assert_eq!(AndroidPackageImageClaim::decode(&wire), Ok(claim));

        for (offset, error) in [
            (
                16,
                AndroidPackageImageClaimError::InvalidCompatibleSessionId,
            ),
            (24, AndroidPackageImageClaimError::InvalidPackageGeneration),
            (32, AndroidPackageImageClaimError::InvalidApkLength),
        ] {
            let mut malformed = wire;
            malformed[offset..offset + if offset == 32 { 4 } else { 8 }].fill(0);
            assert_eq!(AndroidPackageImageClaim::decode(&malformed), Err(error));
        }
        for (range, error) in [
            (40..72, AndroidPackageImageClaimError::ZeroApkSha256),
            (72..104, AndroidPackageImageClaimError::ZeroSignerSha256),
        ] {
            let mut malformed = wire;
            malformed[range].fill(0);
            assert_eq!(AndroidPackageImageClaim::decode(&malformed), Err(error));
        }
        for offset in [38, 104, 127] {
            let mut malformed = wire;
            malformed[offset] = 1;
            assert_eq!(
                AndroidPackageImageClaim::decode(&malformed),
                Err(AndroidPackageImageClaimError::NonZeroReserved)
            );
        }
    }

    #[cfg(feature = "androidbox-apk-install0")]
    #[test]
    fn android_package_relaunch_request_constructor_rejects_each_invalid_field() {
        let construct = |request_sequence,
                         expected_generation,
                         expected_version_code,
                         apk_length,
                         apk_sha256,
                         signer_sha256,
                         package_name: &[u8],
                         activity_name: &[u8]| {
            AndroidPackageRelaunchRequest::new(
                request_sequence,
                expected_generation,
                expected_version_code,
                apk_length,
                apk_sha256,
                signer_sha256,
                package_name,
                activity_name,
            )
        };
        assert_eq!(
            construct(
                0,
                7,
                12,
                12_566,
                [0x11; 32],
                [0x22; 32],
                b"dev.bndroid.resources",
                b"dev.bndroid.resources.MainActivity"
            ),
            Err(AndroidPackageRelaunchRequestError::InvalidRequestSequence)
        );
        assert_eq!(
            construct(
                41,
                0,
                12,
                12_566,
                [0x11; 32],
                [0x22; 32],
                b"dev.bndroid.resources",
                b"dev.bndroid.resources.MainActivity"
            ),
            Err(AndroidPackageRelaunchRequestError::InvalidExpectedGeneration)
        );
        assert_eq!(
            construct(
                41,
                7,
                0,
                12_566,
                [0x11; 32],
                [0x22; 32],
                b"dev.bndroid.resources",
                b"dev.bndroid.resources.MainActivity"
            ),
            Err(AndroidPackageRelaunchRequestError::InvalidExpectedVersionCode)
        );
        for apk_length in [0, ANDROID_PACKAGE_APK_MAX_BYTES + 1] {
            assert_eq!(
                construct(
                    41,
                    7,
                    12,
                    apk_length,
                    [0x11; 32],
                    [0x22; 32],
                    b"dev.bndroid.resources",
                    b"dev.bndroid.resources.MainActivity"
                ),
                Err(AndroidPackageRelaunchRequestError::InvalidApkLength)
            );
        }
        for (apk_sha256, signer_sha256, error) in [
            (
                [0; 32],
                [0x22; 32],
                AndroidPackageRelaunchRequestError::ZeroApkSha256,
            ),
            (
                [0x11; 32],
                [0; 32],
                AndroidPackageRelaunchRequestError::ZeroSignerSha256,
            ),
        ] {
            assert_eq!(
                construct(
                    41,
                    7,
                    12,
                    12_566,
                    apk_sha256,
                    signer_sha256,
                    b"dev.bndroid.resources",
                    b"dev.bndroid.resources.MainActivity"
                ),
                Err(error)
            );
        }

        let oversized_package = [b'a'; ANDROID_PACKAGE_NAME_MAX_BYTES + 1];
        let oversized_activity = [b'a'; ANDROID_PACKAGE_ACTIVITY_MAX_BYTES + 1];
        for (package_name, activity_name, error) in [
            (
                &b""[..],
                &b"dev.bndroid.resources.MainActivity"[..],
                AndroidPackageRelaunchRequestError::InvalidPackageNameLength,
            ),
            (
                &oversized_package,
                &b"dev.bndroid.resources.MainActivity"[..],
                AndroidPackageRelaunchRequestError::InvalidPackageNameLength,
            ),
            (
                &b"dev.bndroid.resources"[..],
                &b""[..],
                AndroidPackageRelaunchRequestError::InvalidActivityNameLength,
            ),
            (
                &b"dev.bndroid.resources"[..],
                &oversized_activity,
                AndroidPackageRelaunchRequestError::InvalidActivityNameLength,
            ),
            (
                &b"bad\npackage"[..],
                &b"dev.bndroid.resources.MainActivity"[..],
                AndroidPackageRelaunchRequestError::NonPrintablePackageName,
            ),
            (
                &b"dev.bndroid.resources"[..],
                &b"bad\x7factivity"[..],
                AndroidPackageRelaunchRequestError::NonPrintableActivityName,
            ),
        ] {
            assert_eq!(
                construct(
                    41,
                    7,
                    12,
                    12_566,
                    [0x11; 32],
                    [0x22; 32],
                    package_name,
                    activity_name
                ),
                Err(error)
            );
        }
    }

    #[cfg(feature = "androidbox-apk-install0")]
    #[test]
    fn android_package_relaunch_request_decoder_rejects_every_noncanonical_wire_class() {
        let canonical = relaunch_request().encode();
        assert_eq!(
            AndroidPackageRelaunchRequest::decode(&canonical[..639]),
            Err(AndroidPackageRelaunchRequestError::InvalidWireLength)
        );
        let oversized_wire = [0; ANDROID_PACKAGE_RELAUNCH_REQUEST_WIRE_SIZE + 1];
        assert_eq!(
            AndroidPackageRelaunchRequest::decode(&oversized_wire),
            Err(AndroidPackageRelaunchRequestError::InvalidWireLength)
        );

        let mut malformed = canonical;
        malformed[0] = b'X';
        assert_eq!(
            AndroidPackageRelaunchRequest::decode(&malformed),
            Err(AndroidPackageRelaunchRequestError::InvalidMagic)
        );
        let mut malformed = canonical;
        set_test_u32(&mut malformed, 8, 2);
        assert_eq!(
            AndroidPackageRelaunchRequest::decode(&malformed),
            Err(AndroidPackageRelaunchRequestError::UnsupportedVersion)
        );
        let mut malformed = canonical;
        set_test_u32(&mut malformed, 12, 1);
        assert_eq!(
            AndroidPackageRelaunchRequest::decode(&malformed),
            Err(AndroidPackageRelaunchRequestError::NonZeroFlags)
        );

        for (offset, width, error) in [
            (
                16,
                8,
                AndroidPackageRelaunchRequestError::InvalidRequestSequence,
            ),
            (
                24,
                8,
                AndroidPackageRelaunchRequestError::InvalidExpectedGeneration,
            ),
            (
                32,
                8,
                AndroidPackageRelaunchRequestError::InvalidExpectedVersionCode,
            ),
            (40, 4, AndroidPackageRelaunchRequestError::InvalidApkLength),
        ] {
            let mut malformed = canonical;
            malformed[offset..offset + width].fill(0);
            assert_eq!(
                AndroidPackageRelaunchRequest::decode(&malformed),
                Err(error)
            );
        }
        let mut malformed = canonical;
        set_test_u32(&mut malformed, 40, ANDROID_PACKAGE_APK_MAX_BYTES + 1);
        assert_eq!(
            AndroidPackageRelaunchRequest::decode(&malformed),
            Err(AndroidPackageRelaunchRequestError::InvalidApkLength)
        );

        for (offset, maximum, error) in [
            (
                44,
                ANDROID_PACKAGE_NAME_MAX_BYTES,
                AndroidPackageRelaunchRequestError::InvalidPackageNameLength,
            ),
            (
                46,
                ANDROID_PACKAGE_ACTIVITY_MAX_BYTES,
                AndroidPackageRelaunchRequestError::InvalidActivityNameLength,
            ),
        ] {
            let mut malformed = canonical;
            set_test_u16(&mut malformed, offset, 0);
            assert_eq!(
                AndroidPackageRelaunchRequest::decode(&malformed),
                Err(error)
            );
            let mut malformed = canonical;
            set_test_u16(&mut malformed, offset, maximum as u16 + 1);
            assert_eq!(
                AndroidPackageRelaunchRequest::decode(&malformed),
                Err(error)
            );
        }

        for profile in [0, 1, 3, u16::MAX] {
            let mut malformed = canonical;
            set_test_u16(&mut malformed, 48, profile);
            assert_eq!(
                AndroidPackageRelaunchRequest::decode(&malformed),
                Err(AndroidPackageRelaunchRequestError::InvalidProfile)
            );
        }
        for (range, error) in [
            (56..88, AndroidPackageRelaunchRequestError::ZeroApkSha256),
            (
                88..120,
                AndroidPackageRelaunchRequestError::ZeroSignerSha256,
            ),
        ] {
            let mut malformed = canonical;
            malformed[range].fill(0);
            assert_eq!(
                AndroidPackageRelaunchRequest::decode(&malformed),
                Err(error)
            );
        }
        for (offset, error) in [
            (
                120,
                AndroidPackageRelaunchRequestError::NonPrintablePackageName,
            ),
            (
                216,
                AndroidPackageRelaunchRequestError::NonPrintableActivityName,
            ),
        ] {
            for invalid_ascii in [0, 0x1f, 0x7f, 0xff] {
                let mut malformed = canonical;
                malformed[offset] = invalid_ascii;
                assert_eq!(
                    AndroidPackageRelaunchRequest::decode(&malformed),
                    Err(error)
                );
            }
        }
        for (offset, error) in [
            (
                215,
                AndroidPackageRelaunchRequestError::NonZeroPackageNamePadding,
            ),
            (
                343,
                AndroidPackageRelaunchRequestError::NonZeroActivityNamePadding,
            ),
        ] {
            let mut malformed = canonical;
            malformed[offset] = 1;
            assert_eq!(
                AndroidPackageRelaunchRequest::decode(&malformed),
                Err(error)
            );
        }
        for offset in [50, 55, 344, 639] {
            let mut malformed = canonical;
            malformed[offset] = 1;
            assert_eq!(
                AndroidPackageRelaunchRequest::decode(&malformed),
                Err(AndroidPackageRelaunchRequestError::NonZeroReserved)
            );
        }
    }

    #[cfg(feature = "androidbox-apk-install0")]
    #[test]
    fn android_package_snapshot_wire_round_trips_installed_and_empty_states() {
        #[cfg(feature = "androidbox-layout-mixed19")]
        assert_eq!(ABI_VERSION, 69);
        #[cfg(all(
            feature = "androidbox-layout-size18",
            not(feature = "androidbox-layout-mixed19")
        ))]
        assert_eq!(ABI_VERSION, 68);
        #[cfg(all(
            feature = "androidbox-layout-directional17",
            not(feature = "androidbox-layout-size18")
        ))]
        assert_eq!(ABI_VERSION, 67);
        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        assert_eq!(ABI_VERSION, 66);
        #[cfg(all(
            feature = "androidbox-layout-weight15",
            not(feature = "androidbox-layout-spacing16")
        ))]
        assert_eq!(ABI_VERSION, 65);
        #[cfg(all(
            feature = "androidbox-layout-row14",
            not(feature = "androidbox-layout-weight15")
        ))]
        assert_eq!(ABI_VERSION, 64);
        #[cfg(all(
            feature = "androidbox-string-builder13",
            not(feature = "androidbox-layout-row14")
        ))]
        assert_eq!(ABI_VERSION, 63);
        #[cfg(all(
            feature = "androidbox-string-text12",
            not(feature = "androidbox-string-builder13")
        ))]
        assert_eq!(ABI_VERSION, 62);
        #[cfg(all(
            feature = "androidbox-activity-state11",
            not(feature = "androidbox-string-text12")
        ))]
        assert_eq!(ABI_VERSION, 61);
        #[cfg(all(
            feature = "androidbox-activity-fields10",
            not(feature = "androidbox-activity-state11")
        ))]
        assert_eq!(ABI_VERSION, 60);
        #[cfg(all(
            feature = "androidbox-dex-instance9",
            not(feature = "androidbox-activity-fields10")
        ))]
        assert_eq!(ABI_VERSION, 59);
        #[cfg(all(
            feature = "androidbox-dex-methods8",
            not(feature = "androidbox-dex-instance9")
        ))]
        assert_eq!(ABI_VERSION, 58);
        #[cfg(all(
            feature = "androidbox-density-icons7",
            not(feature = "androidbox-dex-methods8")
        ))]
        assert_eq!(ABI_VERSION, 57);
        #[cfg(all(
            feature = "androidbox-icon-resources5",
            not(feature = "androidbox-density-icons7")
        ))]
        assert_eq!(ABI_VERSION, 56);
        #[cfg(all(
            feature = "androidbox-multipackage4",
            not(feature = "androidbox-icon-resources5")
        ))]
        assert_eq!(ABI_VERSION, 55);
        #[cfg(all(
            feature = "androidbox-manifest-catalog3",
            not(feature = "androidbox-multipackage4")
        ))]
        assert_eq!(ABI_VERSION, 54);
        #[cfg(all(
            feature = "androidbox-runtime-install2",
            not(feature = "androidbox-manifest-catalog3")
        ))]
        assert_eq!(ABI_VERSION, 53);
        #[cfg(all(
            feature = "androidbox-runtime-uninstall1",
            not(feature = "androidbox-runtime-install2")
        ))]
        assert_eq!(ABI_VERSION, 52);
        #[cfg(all(
            feature = "androidbox-apk-envelope4",
            not(feature = "androidbox-runtime-uninstall1")
        ))]
        assert_eq!(ABI_VERSION, 51);
        #[cfg(all(
            feature = "androidbox-multiaction3",
            not(feature = "androidbox-apk-envelope4")
        ))]
        assert_eq!(ABI_VERSION, 50);
        #[cfg(all(
            feature = "androidbox-scene-rpc2",
            not(feature = "androidbox-multiaction3")
        ))]
        assert_eq!(ABI_VERSION, 49);
        #[cfg(all(
            feature = "androidbox-restart0",
            not(feature = "androidbox-scene-rpc2")
        ))]
        assert_eq!(ABI_VERSION, 48);
        #[cfg(all(feature = "androidbox-process0", not(feature = "androidbox-restart0")))]
        assert_eq!(ABI_VERSION, 47);
        #[cfg(all(
            feature = "androidbox-interactive0",
            not(feature = "androidbox-process0")
        ))]
        assert_eq!(ABI_VERSION, 46);
        #[cfg(all(
            feature = "androidbox-el0-runtime0",
            not(feature = "androidbox-interactive0")
        ))]
        assert_eq!(ABI_VERSION, 45);
        #[cfg(not(feature = "androidbox-el0-runtime0"))]
        assert_eq!(ABI_VERSION, 44);
        assert_eq!(ANDROID_PACKAGE_SNAPSHOT_WIRE_SIZE, 640);
        assert_eq!(ANDROID_PACKAGE_SNAPSHOT_MAGIC, *b"BNDAPS01");
        assert_eq!(ANDROID_PACKAGE_SNAPSHOT_VERSION, 1);
        assert_eq!(ANDROID_PACKAGE_APK_MAX_BYTES, 65_024);
        assert_eq!(ANDROID_PACKAGE_NAME_MAX_BYTES, 96);
        assert_eq!(ANDROID_PACKAGE_ACTIVITY_MAX_BYTES, 128);
        assert_eq!(ANDROID_PACKAGE_TITLE_MAX_BYTES, 128);
        assert_eq!(ANDROID_PACKAGE_TEXT_MAX_BYTES, 128);
        assert_eq!(AndroidPackageCompatibilityProfile::Resources1.raw(), 2);
        assert_eq!(
            AndroidPackageCompatibilityProfile::from_raw(2),
            Some(AndroidPackageCompatibilityProfile::Resources1)
        );
        assert_eq!(AndroidPackageCompatibilityProfile::from_raw(0), None);
        assert_eq!(AndroidPackageCompatibilityProfile::from_raw(3), None);

        let snapshot = installed_snapshot();
        assert_eq!(
            snapshot.flags(),
            ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMATTED
                | ANDROID_PACKAGE_SNAPSHOT_FLAG_INSTALLED
                | ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_PRESENT
                | ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_USED
                | ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMAT_PERFORMED
        );
        assert!(snapshot.formatted());
        assert!(snapshot.installed_package());
        assert!(snapshot.source_present());
        assert!(snapshot.source_used());
        assert!(snapshot.format_performed());
        assert_eq!(snapshot.generation(), 7);
        assert_eq!(snapshot.version_code(), 12);
        assert_eq!(snapshot.apk_length(), 12_566);
        assert_eq!(snapshot.resources_table_crc32(), 0x1122_3344);
        assert_eq!(snapshot.layout_xml_crc32(), 0x5566_7788);
        assert_eq!(snapshot.layout_resource_id(), 0x7f02_0000);
        assert_eq!(snapshot.text_resource_id(), 0x7f03_0000);
        assert_eq!(snapshot.instruction_count(), 4);
        assert_eq!(
            snapshot.profile(),
            Some(AndroidPackageCompatibilityProfile::Resources1)
        );
        assert_eq!(
            snapshot.io_counters(),
            AndroidPackageIoCounters::new(19, 9, 3)
        );
        assert_eq!(snapshot.io_counters().reads(), 19);
        assert_eq!(snapshot.io_counters().writes(), 9);
        assert_eq!(snapshot.io_counters().flushes(), 3);
        assert_eq!(snapshot.apk_sha256(), &[0x11; 32]);
        assert_eq!(snapshot.signer_sha256(), &[0x22; 32]);
        assert_eq!(snapshot.package_name(), "dev.bndroid.resources");
        assert_eq!(
            snapshot.activity_name(),
            "dev.bndroid.resources.MainActivity"
        );
        assert_eq!(snapshot.title(), "AndroidBox Resources");
        assert_eq!(snapshot.text(), "Hello from a signed resource APK");

        let wire = snapshot.encode();
        assert_eq!(&wire[0..8], b"BNDAPS01");
        assert_eq!(&wire[8..12], &1_u32.to_le_bytes());
        assert_eq!(&wire[16..24], &7_u64.to_le_bytes());
        assert_eq!(&wire[24..32], &12_u64.to_le_bytes());
        assert_eq!(&wire[32..36], &12_566_u32.to_le_bytes());
        assert_eq!(&wire[52..54], &4_u16.to_le_bytes());
        assert_eq!(&wire[54..56], &2_u16.to_le_bytes());
        assert_eq!(&wire[64..72], &19_u64.to_le_bytes());
        assert_eq!(&wire[72..80], &9_u64.to_le_bytes());
        assert_eq!(&wire[80..88], &3_u64.to_le_bytes());
        assert_eq!(&wire[88..120], &[0x11; 32]);
        assert_eq!(&wire[120..152], &[0x22; 32]);
        assert!(wire[632..640].iter().all(|byte| *byte == 0));
        assert_eq!(AndroidPackageSnapshot::decode(&wire), Ok(snapshot));

        let recovered = AndroidPackageSnapshot::installed(
            AndroidPackageSnapshotState::new(true, false, false, false),
            AndroidPackageIoCounters::new(4, 0, 0),
            installed_snapshot_metadata(),
        )
        .unwrap();
        assert_eq!(
            recovered.flags(),
            ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMATTED | ANDROID_PACKAGE_SNAPSHOT_FLAG_INSTALLED
        );
        assert_eq!(
            AndroidPackageSnapshot::decode(&recovered.encode()),
            Ok(recovered)
        );

        let empty = AndroidPackageSnapshot::empty(
            AndroidPackageSnapshotState::new(true, false, false, false),
            AndroidPackageIoCounters::new(5, 2, 1),
        )
        .unwrap();
        assert!(empty.formatted());
        assert!(!empty.installed_package());
        assert_eq!(empty.generation(), 0);
        assert_eq!(empty.profile(), None);
        assert_eq!(empty.package_name(), "");
        assert_eq!(empty.activity_name(), "");
        assert_eq!(empty.title(), "");
        assert_eq!(empty.text(), "");
        let empty_wire = empty.encode();
        assert!(empty_wire[16..64].iter().all(|byte| *byte == 0));
        assert!(empty_wire[88..640].iter().all(|byte| *byte == 0));
        assert_eq!(&empty_wire[64..72], &5_u64.to_le_bytes());
        assert_eq!(&empty_wire[72..80], &2_u64.to_le_bytes());
        assert_eq!(&empty_wire[80..88], &1_u64.to_le_bytes());
        assert_eq!(AndroidPackageSnapshot::decode(&empty_wire), Ok(empty));
    }

    #[cfg(feature = "androidbox-apk-install0")]
    #[test]
    fn android_package_snapshot_constructors_reject_contradictory_state_and_metadata() {
        let io = AndroidPackageIoCounters::default();
        #[cfg(not(feature = "androidbox-runtime-install2"))]
        assert_eq!(
            AndroidPackageSnapshot::empty(
                AndroidPackageSnapshotState::new(false, true, false, false),
                io
            ),
            Err(AndroidPackageSnapshotError::InconsistentSourceState)
        );
        #[cfg(feature = "androidbox-runtime-install2")]
        {
            let candidate_only = AndroidPackageSnapshot::empty(
                AndroidPackageSnapshotState::new(false, true, false, false),
                io,
            )
            .unwrap();
            assert!(!candidate_only.formatted());
            assert!(candidate_only.source_present());
            assert!(!candidate_only.source_used());
            assert!(!candidate_only.installed_package());
            assert_eq!(
                AndroidPackageSnapshot::decode(&candidate_only.encode()),
                Ok(candidate_only)
            );
        }
        assert_eq!(
            AndroidPackageSnapshot::empty(
                AndroidPackageSnapshotState::new(true, true, true, false),
                io
            ),
            Err(AndroidPackageSnapshotError::SourceUsedWithoutInstalledPackage)
        );
        assert_eq!(
            AndroidPackageSnapshot::empty(
                AndroidPackageSnapshotState::new(false, false, false, true),
                io
            ),
            Err(AndroidPackageSnapshotError::InvalidFormatPerformedState)
        );
        assert_eq!(
            AndroidPackageSnapshot::installed(
                AndroidPackageSnapshotState::new(false, false, false, false),
                io,
                installed_snapshot_metadata()
            ),
            Err(AndroidPackageSnapshotError::InstalledVolumeNotFormatted)
        );

        let mut metadata = installed_snapshot_metadata();
        metadata.generation = 0;
        assert_eq!(
            AndroidPackageSnapshot::installed(
                AndroidPackageSnapshotState::new(true, false, false, false),
                io,
                metadata
            ),
            Err(AndroidPackageSnapshotError::InvalidGeneration)
        );
        for version_code in [0, u64::from(u32::MAX) + 1] {
            let mut metadata = installed_snapshot_metadata();
            metadata.version_code = version_code;
            assert_eq!(
                AndroidPackageSnapshot::installed(
                    AndroidPackageSnapshotState::new(true, false, false, false),
                    io,
                    metadata
                ),
                Err(AndroidPackageSnapshotError::InvalidVersionCode)
            );
        }
        for apk_length in [0, ANDROID_PACKAGE_APK_MAX_BYTES + 1] {
            let mut metadata = installed_snapshot_metadata();
            metadata.apk_length = apk_length;
            assert_eq!(
                AndroidPackageSnapshot::installed(
                    AndroidPackageSnapshotState::new(true, false, false, false),
                    io,
                    metadata
                ),
                Err(AndroidPackageSnapshotError::InvalidApkLength)
            );
        }
        let scalar_errors = [
            (
                0,
                0x5566_7788,
                0x7f02_0000,
                0x7f03_0000,
                4,
                AndroidPackageSnapshotError::InvalidResourcesTableCrc32,
            ),
            (
                0x1122_3344,
                0,
                0x7f02_0000,
                0x7f03_0000,
                4,
                AndroidPackageSnapshotError::InvalidLayoutXmlCrc32,
            ),
            (
                0x1122_3344,
                0x5566_7788,
                0x0102_0000,
                0x7f03_0000,
                4,
                AndroidPackageSnapshotError::InvalidLayoutResourceId,
            ),
            (
                0x1122_3344,
                0x5566_7788,
                0x7f02_0000,
                0x7f02_0000,
                4,
                AndroidPackageSnapshotError::InvalidTextResourceId,
            ),
            (
                0x1122_3344,
                0x5566_7788,
                0x7f02_0000,
                0x7f03_0000,
                0,
                AndroidPackageSnapshotError::InvalidInstructionCount,
            ),
        ];
        for (arsc_crc, layout_crc, layout_id, text_id, instructions, error) in scalar_errors {
            let mut metadata = installed_snapshot_metadata();
            metadata.resources_table_crc32 = arsc_crc;
            metadata.layout_xml_crc32 = layout_crc;
            metadata.layout_resource_id = layout_id;
            metadata.text_resource_id = text_id;
            metadata.instruction_count = instructions;
            assert_eq!(
                AndroidPackageSnapshot::installed(
                    AndroidPackageSnapshotState::new(true, false, false, false),
                    io,
                    metadata
                ),
                Err(error)
            );
        }
        for signer in [false, true] {
            let mut metadata = installed_snapshot_metadata();
            if signer {
                metadata.signer_sha256 = [0; 32];
            } else {
                metadata.apk_sha256 = [0; 32];
            }
            assert_eq!(
                AndroidPackageSnapshot::installed(
                    AndroidPackageSnapshotState::new(true, false, false, false),
                    io,
                    metadata
                ),
                Err(if signer {
                    AndroidPackageSnapshotError::ZeroSignerSha256
                } else {
                    AndroidPackageSnapshotError::ZeroApkSha256
                })
            );
        }

        let oversized_package = [b'a'; ANDROID_PACKAGE_NAME_MAX_BYTES + 1];
        let oversized_activity = [b'a'; ANDROID_PACKAGE_ACTIVITY_MAX_BYTES + 1];
        let oversized_title = [b'a'; ANDROID_PACKAGE_TITLE_MAX_BYTES + 1];
        let oversized_text = [b'a'; ANDROID_PACKAGE_TEXT_MAX_BYTES + 1];
        let invalid_texts: [(&[u8], usize, AndroidPackageSnapshotError); 8] = [
            (
                b"",
                0,
                AndroidPackageSnapshotError::InvalidPackageNameLength,
            ),
            (
                &oversized_package,
                0,
                AndroidPackageSnapshotError::InvalidPackageNameLength,
            ),
            (
                b"",
                1,
                AndroidPackageSnapshotError::InvalidActivityNameLength,
            ),
            (
                &oversized_activity,
                1,
                AndroidPackageSnapshotError::InvalidActivityNameLength,
            ),
            (b"", 2, AndroidPackageSnapshotError::InvalidTitleLength),
            (
                &oversized_title,
                2,
                AndroidPackageSnapshotError::InvalidTitleLength,
            ),
            (b"", 3, AndroidPackageSnapshotError::InvalidTextLength),
            (
                &oversized_text,
                3,
                AndroidPackageSnapshotError::InvalidTextLength,
            ),
        ];
        for (value, field, error) in invalid_texts {
            let mut metadata = installed_snapshot_metadata();
            match field {
                0 => metadata.package_name = value,
                1 => metadata.activity_name = value,
                2 => metadata.title = value,
                _ => metadata.text = value,
            }
            assert_eq!(
                AndroidPackageSnapshot::installed(
                    AndroidPackageSnapshotState::new(true, false, false, false),
                    io,
                    metadata
                ),
                Err(error)
            );
        }
        for (field, error) in [
            (0, AndroidPackageSnapshotError::NonPrintablePackageName),
            (1, AndroidPackageSnapshotError::NonPrintableActivityName),
            (2, AndroidPackageSnapshotError::NonPrintableTitle),
            (3, AndroidPackageSnapshotError::NonPrintableText),
        ] {
            let mut metadata = installed_snapshot_metadata();
            match field {
                0 => metadata.package_name = b"bad\npackage",
                1 => metadata.activity_name = b"bad\x7factivity",
                2 => metadata.title = b"\ttitle",
                _ => metadata.text = b"text\0",
            }
            assert_eq!(
                AndroidPackageSnapshot::installed(
                    AndroidPackageSnapshotState::new(true, false, false, false),
                    io,
                    metadata
                ),
                Err(error)
            );
        }
    }

    #[cfg(feature = "androidbox-apk-install0")]
    #[test]
    fn android_package_snapshot_decoder_rejects_every_noncanonical_wire_class() {
        let canonical = installed_snapshot().encode();
        assert_eq!(
            AndroidPackageSnapshot::decode(&canonical[..639]),
            Err(AndroidPackageSnapshotError::InvalidWireLength)
        );

        let mut malformed = canonical;
        malformed[0] = b'X';
        assert_eq!(
            AndroidPackageSnapshot::decode(&malformed),
            Err(AndroidPackageSnapshotError::InvalidMagic)
        );
        let mut malformed = canonical;
        set_test_u32(&mut malformed, 8, 2);
        assert_eq!(
            AndroidPackageSnapshot::decode(&malformed),
            Err(AndroidPackageSnapshotError::UnsupportedVersion)
        );
        let mut malformed = canonical;
        set_test_u32(&mut malformed, 12, 1 << 31);
        assert_eq!(
            AndroidPackageSnapshot::decode(&malformed),
            Err(AndroidPackageSnapshotError::UnknownFlags)
        );
        let mut malformed = canonical;
        set_test_u32(
            &mut malformed,
            12,
            ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMATTED
                | ANDROID_PACKAGE_SNAPSHOT_FLAG_INSTALLED
                | ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_PRESENT,
        );
        #[cfg(not(feature = "androidbox-runtime-install2"))]
        assert_eq!(
            AndroidPackageSnapshot::decode(&malformed),
            Err(AndroidPackageSnapshotError::InconsistentSourceState)
        );
        #[cfg(feature = "androidbox-runtime-install2")]
        {
            let candidate_only = AndroidPackageSnapshot::decode(&malformed).unwrap();
            assert!(candidate_only.formatted());
            assert!(candidate_only.installed_package());
            assert!(candidate_only.source_present());
            assert!(!candidate_only.source_used());
            assert!(!candidate_only.format_performed());
        }
        let mut malformed = canonical;
        set_test_u32(
            &mut malformed,
            12,
            ANDROID_PACKAGE_SNAPSHOT_FLAG_INSTALLED
                | ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_PRESENT
                | ANDROID_PACKAGE_SNAPSHOT_FLAG_SOURCE_USED
                | ANDROID_PACKAGE_SNAPSHOT_FLAG_FORMAT_PERFORMED,
        );
        assert_eq!(
            AndroidPackageSnapshot::decode(&malformed),
            Err(AndroidPackageSnapshotError::InvalidFormatPerformedState)
        );
        let mut malformed = canonical;
        set_test_u32(&mut malformed, 12, ANDROID_PACKAGE_SNAPSHOT_FLAG_INSTALLED);
        assert_eq!(
            AndroidPackageSnapshot::decode(&malformed),
            Err(AndroidPackageSnapshotError::InstalledVolumeNotFormatted)
        );

        for (offset, maximum, error) in [
            (
                56,
                ANDROID_PACKAGE_NAME_MAX_BYTES,
                AndroidPackageSnapshotError::InvalidPackageNameLength,
            ),
            (
                58,
                ANDROID_PACKAGE_ACTIVITY_MAX_BYTES,
                AndroidPackageSnapshotError::InvalidActivityNameLength,
            ),
            (
                60,
                ANDROID_PACKAGE_TITLE_MAX_BYTES,
                AndroidPackageSnapshotError::InvalidTitleLength,
            ),
            (
                62,
                ANDROID_PACKAGE_TEXT_MAX_BYTES,
                AndroidPackageSnapshotError::InvalidTextLength,
            ),
        ] {
            let mut malformed = canonical;
            set_test_u16(&mut malformed, offset, maximum as u16 + 1);
            assert_eq!(AndroidPackageSnapshot::decode(&malformed), Err(error));
        }
        for (offset, error) in [
            (247, AndroidPackageSnapshotError::NonZeroPackageNamePadding),
            (375, AndroidPackageSnapshotError::NonZeroActivityNamePadding),
            (503, AndroidPackageSnapshotError::NonZeroTitlePadding),
            (631, AndroidPackageSnapshotError::NonZeroTextPadding),
        ] {
            let mut malformed = canonical;
            malformed[offset] = 1;
            assert_eq!(AndroidPackageSnapshot::decode(&malformed), Err(error));
        }
        for (offset, error) in [
            (152, AndroidPackageSnapshotError::NonPrintablePackageName),
            (248, AndroidPackageSnapshotError::NonPrintableActivityName),
            (376, AndroidPackageSnapshotError::NonPrintableTitle),
            (504, AndroidPackageSnapshotError::NonPrintableText),
        ] {
            let mut malformed = canonical;
            malformed[offset] = 0x1f;
            assert_eq!(AndroidPackageSnapshot::decode(&malformed), Err(error));
        }
        for (range, error) in [
            (88..120, AndroidPackageSnapshotError::ZeroApkSha256),
            (120..152, AndroidPackageSnapshotError::ZeroSignerSha256),
        ] {
            let mut malformed = canonical;
            malformed[range].fill(0);
            assert_eq!(AndroidPackageSnapshot::decode(&malformed), Err(error));
        }
        for (offset, width, error) in [
            (16, 8, AndroidPackageSnapshotError::InvalidGeneration),
            (24, 8, AndroidPackageSnapshotError::InvalidVersionCode),
            (32, 4, AndroidPackageSnapshotError::InvalidApkLength),
            (
                36,
                4,
                AndroidPackageSnapshotError::InvalidResourcesTableCrc32,
            ),
            (40, 4, AndroidPackageSnapshotError::InvalidLayoutXmlCrc32),
            (44, 4, AndroidPackageSnapshotError::InvalidLayoutResourceId),
            (48, 4, AndroidPackageSnapshotError::InvalidTextResourceId),
            (52, 2, AndroidPackageSnapshotError::InvalidInstructionCount),
            (54, 2, AndroidPackageSnapshotError::InvalidProfile),
        ] {
            let mut malformed = canonical;
            malformed[offset..offset + width].fill(0);
            assert_eq!(AndroidPackageSnapshot::decode(&malformed), Err(error));
        }
        let mut malformed = canonical;
        malformed[639] = 1;
        assert_eq!(
            AndroidPackageSnapshot::decode(&malformed),
            Err(AndroidPackageSnapshotError::NonZeroReserved)
        );

        let empty = AndroidPackageSnapshot::empty(
            AndroidPackageSnapshotState::new(true, false, false, false),
            AndroidPackageIoCounters::new(8, 4, 2),
        )
        .unwrap()
        .encode();
        for offset in [16, 88] {
            let mut malformed = empty;
            malformed[offset] = 1;
            assert_eq!(
                AndroidPackageSnapshot::decode(&malformed),
                Err(AndroidPackageSnapshotError::NonCanonicalEmptyPackage)
            );
        }
        for (length_offset, text_offset) in [(56, 152), (58, 248), (60, 376), (62, 504)] {
            let mut malformed = empty;
            set_test_u16(&mut malformed, length_offset, 1);
            malformed[text_offset] = b'A';
            assert_eq!(
                AndroidPackageSnapshot::decode(&malformed),
                Err(AndroidPackageSnapshotError::NonCanonicalEmptyPackage)
            );
        }
    }

    #[cfg(feature = "resident-platform-shutdown-runtime")]
    #[test]
    fn resident_shutdown_graph_is_stable_acyclic_and_complete() {
        use super::{
            SHUTDOWN_SERVICE_ALL_MASK, SHUTDOWN_SERVICE_EDGE_COUNT, SHUTDOWN_SERVICE_NODE_COUNT,
            SHUTDOWN_SERVICE_WAVE_COUNT, ShutdownServiceNode, UserImageId,
        };

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
        let mut mask = 0_u64;
        let mut edges = 0_u32;
        for (raw, node) in nodes.into_iter().enumerate() {
            assert_eq!(ShutdownServiceNode::from_raw(raw as u64), Some(node));
            assert_eq!(node.raw(), raw as u64);
            assert_eq!(node.bit(), 1_u64 << raw);
            assert_eq!(node.dependency_mask() & node.bit(), 0);
            assert!(node.dependency_mask() & !SHUTDOWN_SERVICE_ALL_MASK == 0);
            assert!(node.dependent_mask() & !SHUTDOWN_SERVICE_ALL_MASK == 0);
            assert!(node.shutdown_wave() < SHUTDOWN_SERVICE_WAVE_COUNT as u64);
            for dependency_raw in 0..SHUTDOWN_SERVICE_NODE_COUNT {
                let dependency = ShutdownServiceNode::from_raw(dependency_raw as u64).unwrap();
                if node.dependency_mask() & dependency.bit() != 0 {
                    assert!(node.shutdown_wave() < dependency.shutdown_wave());
                    assert!(dependency.dependent_mask() & node.bit() != 0);
                    edges += 1;
                }
            }
            mask |= node.bit();
        }
        assert_eq!(mask, SHUTDOWN_SERVICE_ALL_MASK);
        assert_eq!(edges as usize, SHUTDOWN_SERVICE_EDGE_COUNT);
        assert_eq!(ShutdownServiceNode::from_raw(8), None);
        assert_eq!(
            ShutdownServiceNode::PrimaryClient.image_id(),
            UserImageId::Client
        );
        assert_eq!(
            ShutdownServiceNode::SecondaryClient.image_id(),
            UserImageId::Client
        );
    }

    #[test]
    fn app_data_principal_bounds_and_path_contract_are_stable() {
        assert_eq!(AppDataPrincipal::PRIMARY_APP.raw(), 1);
        assert_eq!(AppDataPrincipal::LAUNCHER.raw(), 2);
        assert_eq!(AppDataPrincipal::from_raw(0), None);
        assert_eq!(
            AppDataPrincipal::from_raw(1),
            Some(AppDataPrincipal::PRIMARY_APP)
        );
        assert_eq!(
            AppDataPrincipal::from_raw(2),
            Some(AppDataPrincipal::LAUNCHER)
        );
        assert_eq!(
            AppDataPrincipal::from_raw(u64::MAX).unwrap().raw(),
            u64::MAX
        );

        assert_eq!(APP_DATA_PATH_MAX_BYTES, 64);
        assert_eq!(APP_DATA_PATH_MAX_DEPTH, 4);
        assert_eq!(APP_DATA_FILE_MAX_BYTES, 4096);
        assert_eq!(APP_DATA_DIRECTORY_MAX_ENTRIES, 32);
        assert_eq!(
            validate_app_data_path(b"settings/theme"),
            Ok("settings/theme")
        );
        assert_eq!(
            validate_app_data_path("设置/主题".as_bytes()),
            Ok("设置/主题")
        );
        assert!(validate_app_data_path(&[b'a'; APP_DATA_PATH_MAX_BYTES]).is_ok());
        assert!(validate_app_data_path(b"one/two/three/four").is_ok());

        assert_eq!(validate_app_data_path(b""), Err(AppDataPathError::Empty));
        assert_eq!(
            validate_app_data_path(&[b'a'; APP_DATA_PATH_MAX_BYTES + 1]),
            Err(AppDataPathError::TooLong)
        );
        assert_eq!(
            validate_app_data_path(&[0xff]),
            Err(AppDataPathError::InvalidUtf8)
        );
        assert_eq!(
            validate_app_data_path(b"bad\0name"),
            Err(AppDataPathError::ContainsNul)
        );
        assert_eq!(
            validate_app_data_path(b"/absolute"),
            Err(AppDataPathError::Absolute)
        );
        assert_eq!(
            validate_app_data_path(b"trailing/"),
            Err(AppDataPathError::TrailingSlash)
        );
        assert_eq!(
            validate_app_data_path(b"double//slash"),
            Err(AppDataPathError::EmptyComponent)
        );
        assert_eq!(
            validate_app_data_path(b"one/./two"),
            Err(AppDataPathError::CurrentDirectoryComponent)
        );
        assert_eq!(
            validate_app_data_path(b"one/../two"),
            Err(AppDataPathError::ParentDirectoryComponent)
        );
        assert_eq!(
            validate_app_data_path(b"one/two/three/four/five"),
            Err(AppDataPathError::TooDeep)
        );
    }

    #[test]
    fn file_replace_requests_round_trip_all_cas_modes_and_boundaries() {
        assert_eq!(FILE_REPLACE_REQUEST_MAGIC.to_le_bytes(), *b"ADFR");
        assert_eq!(FILE_REPLACE_REQUEST_VERSION, 1);
        assert_eq!(FILE_REPLACE_REQUEST_WIRE_SIZE, 112);
        assert_eq!(FILE_REPLACE_FLAGS_EXPECT_ANY, 0);
        assert_eq!(FILE_REPLACE_FLAG_CREATE_ONLY, 1);
        assert_eq!(FILE_REPLACE_FLAG_EXPECT_EXACT, 2);

        let cases = [
            FileReplaceRequest::new(b"empty", 0, 0, FileReplaceCas::Any).unwrap(),
            FileReplaceRequest::new(
                b"settings/theme",
                0x0102_0304_0506_0708,
                APP_DATA_FILE_MAX_BYTES,
                FileReplaceCas::CreateOnly,
            )
            .unwrap(),
            FileReplaceRequest::new(
                &[b'x'; APP_DATA_PATH_MAX_BYTES],
                0x1112_1314_1516_1718,
                1,
                FileReplaceCas::Exact(0x2122_2324_2526_2728),
            )
            .unwrap(),
        ];

        for request in cases {
            let wire = request.encode();
            assert_eq!(FileReplaceRequest::decode(&wire), Ok(request));
            assert_eq!(request.path().as_bytes(), request.path_bytes());
            assert_eq!(wire.len(), FILE_REPLACE_REQUEST_WIRE_SIZE);
            assert_eq!(&wire[0..4], b"ADFR");
            assert_eq!(&wire[4..6], &1_u16.to_le_bytes());
            assert_eq!(&wire[32..48], &[0; 16]);
            assert!(
                wire[48 + request.path_bytes().len()..]
                    .iter()
                    .all(|byte| *byte == 0)
            );
        }

        let any = cases[0].encode();
        assert_eq!(&any[6..8], &0_u16.to_le_bytes());
        assert_eq!(&any[12..16], &0_u32.to_le_bytes());
        assert_eq!(&any[16..24], &0_u64.to_le_bytes());
        assert_eq!(&any[24..32], &0_u64.to_le_bytes());

        let create = cases[1].encode();
        assert_eq!(&create[6..8], &1_u16.to_le_bytes());
        assert_eq!(&create[8..12], &14_u32.to_le_bytes());
        assert_eq!(&create[12..16], &4096_u32.to_le_bytes());
        assert_eq!(&create[16..24], &0x0102_0304_0506_0708_u64.to_le_bytes());
        assert_eq!(&create[24..32], &0_u64.to_le_bytes());
        assert_eq!(&create[48..62], b"settings/theme");

        let exact = cases[2].encode();
        assert_eq!(&exact[6..8], &2_u16.to_le_bytes());
        assert_eq!(&exact[8..12], &64_u32.to_le_bytes());
        assert_eq!(cases[2].value_pointer(), 0x1112_1314_1516_1718);
        assert_eq!(cases[2].value_length(), 1);
        assert_eq!(cases[2].cas(), FileReplaceCas::Exact(0x2122_2324_2526_2728));
        assert_eq!(&exact[24..32], &0x2122_2324_2526_2728_u64.to_le_bytes());
    }

    #[test]
    fn file_replace_constructor_rejects_bad_value_ranges_paths_and_cas() {
        assert_eq!(
            FileReplaceRequest::new(b"file", 1, 0, FileReplaceCas::Any),
            Err(FileReplaceRequestError::InvalidValuePointer)
        );
        assert_eq!(
            FileReplaceRequest::new(b"file", 0, 1, FileReplaceCas::Any),
            Err(FileReplaceRequestError::InvalidValuePointer)
        );
        assert_eq!(
            FileReplaceRequest::new(b"file", 1, APP_DATA_FILE_MAX_BYTES + 1, FileReplaceCas::Any,),
            Err(FileReplaceRequestError::ValueTooLarge)
        );
        assert_eq!(
            FileReplaceRequest::new(b"file", u64::MAX, 1, FileReplaceCas::Any),
            Err(FileReplaceRequestError::ValueRangeOverflow)
        );
        assert_eq!(
            FileReplaceRequest::new(b"file", 1, 1, FileReplaceCas::Exact(0)),
            Err(FileReplaceRequestError::InvalidCas)
        );
        assert_eq!(
            FileReplaceRequest::new(b"../escape", 1, 1, FileReplaceCas::Any),
            Err(FileReplaceRequestError::Path(
                AppDataPathError::ParentDirectoryComponent
            ))
        );
    }

    #[test]
    fn file_replace_decoder_rejects_every_noncanonical_field_class() {
        let canonical = FileReplaceRequest::new(b"file", 0x1000, 4, FileReplaceCas::Exact(7))
            .unwrap()
            .encode();
        assert_eq!(
            FileReplaceRequest::decode(&canonical[..canonical.len() - 1]),
            Err(FileReplaceRequestError::InvalidWireLength)
        );

        let mut malformed = canonical;
        set_test_u32(&mut malformed, 0, 0);
        assert_eq!(
            FileReplaceRequest::decode(&malformed),
            Err(FileReplaceRequestError::InvalidMagic)
        );
        let mut malformed = canonical;
        set_test_u16(&mut malformed, 4, 2);
        assert_eq!(
            FileReplaceRequest::decode(&malformed),
            Err(FileReplaceRequestError::UnsupportedVersion)
        );
        for offset in [32, 47] {
            let mut malformed = canonical;
            malformed[offset] = 1;
            assert_eq!(
                FileReplaceRequest::decode(&malformed),
                Err(FileReplaceRequestError::NonZeroReserved)
            );
        }
        for flags in [3, 4, u16::MAX] {
            let mut malformed = canonical;
            set_test_u16(&mut malformed, 6, flags);
            assert_eq!(
                FileReplaceRequest::decode(&malformed),
                Err(FileReplaceRequestError::InvalidFlags)
            );
        }
        for (flags, generation) in [(0, 1), (1, 1), (2, 0)] {
            let mut malformed = canonical;
            set_test_u16(&mut malformed, 6, flags);
            set_test_u64(&mut malformed, 24, generation);
            assert_eq!(
                FileReplaceRequest::decode(&malformed),
                Err(FileReplaceRequestError::InvalidCas)
            );
        }

        let mut malformed = canonical;
        set_test_u32(&mut malformed, 8, 0);
        assert_eq!(
            FileReplaceRequest::decode(&malformed),
            Err(FileReplaceRequestError::NonZeroPathPadding)
        );
        let mut malformed = canonical;
        set_test_u32(&mut malformed, 8, 65);
        assert_eq!(
            FileReplaceRequest::decode(&malformed),
            Err(FileReplaceRequestError::Path(AppDataPathError::TooLong))
        );
        let mut malformed = canonical;
        malformed[48] = 0xff;
        assert_eq!(
            FileReplaceRequest::decode(&malformed),
            Err(FileReplaceRequestError::Path(AppDataPathError::InvalidUtf8))
        );
        let mut malformed = canonical;
        malformed[52] = 1;
        assert_eq!(
            FileReplaceRequest::decode(&malformed),
            Err(FileReplaceRequestError::NonZeroPathPadding)
        );

        let mut malformed = canonical;
        set_test_u32(&mut malformed, 12, 4097);
        assert_eq!(
            FileReplaceRequest::decode(&malformed),
            Err(FileReplaceRequestError::ValueTooLarge)
        );
        let mut malformed = canonical;
        set_test_u64(&mut malformed, 16, 0);
        assert_eq!(
            FileReplaceRequest::decode(&malformed),
            Err(FileReplaceRequestError::InvalidValuePointer)
        );
        let mut malformed = canonical;
        set_test_u64(&mut malformed, 16, u64::MAX - 1);
        assert_eq!(
            FileReplaceRequest::decode(&malformed),
            Err(FileReplaceRequestError::ValueRangeOverflow)
        );
    }

    #[test]
    fn app_data_directory_entries_have_one_canonical_wire_layout() {
        assert_eq!(APP_DATA_DIRECTORY_ENTRY_MAGIC.to_le_bytes(), *b"ADDE");
        assert_eq!(APP_DATA_DIRECTORY_ENTRY_VERSION, 1);
        assert_eq!(APP_DATA_DIRECTORY_ENTRY_FLAGS_NONE, 0);
        assert_eq!(APP_DATA_DIRECTORY_ENTRY_WIRE_SIZE, 112);
        assert_eq!(AppDataDirectoryEntryKind::File.raw(), 1);
        assert_eq!(AppDataDirectoryEntryKind::Directory.raw(), 2);
        assert_eq!(
            AppDataDirectoryEntryKind::from_raw(1),
            Some(AppDataDirectoryEntryKind::File)
        );
        assert_eq!(AppDataDirectoryEntryKind::from_raw(0), None);
        assert_eq!(AppDataDirectoryEntryKind::from_raw(3), None);

        let file = AppDataDirectoryEntry::file(b"settings/theme", 9, 4096).unwrap();
        let wire = file.encode();
        assert_eq!(AppDataDirectoryEntry::decode(&wire), Ok(file));
        assert_eq!(&wire[0..4], b"ADDE");
        assert_eq!(&wire[4..6], &1_u16.to_le_bytes());
        assert_eq!(&wire[6..8], &1_u16.to_le_bytes());
        assert_eq!(&wire[8..12], &0_u32.to_le_bytes());
        assert_eq!(&wire[12..16], &14_u32.to_le_bytes());
        assert_eq!(&wire[16..24], &9_u64.to_le_bytes());
        assert_eq!(&wire[24..32], &4096_u64.to_le_bytes());
        assert_eq!(&wire[32..48], &[0; 16]);
        assert_eq!(&wire[48..62], b"settings/theme");
        assert!(wire[62..].iter().all(|byte| *byte == 0));
        assert_eq!(file.kind(), AppDataDirectoryEntryKind::File);
        assert_eq!(file.path(), "settings/theme");
        assert_eq!(file.path_bytes(), b"settings/theme");
        assert_eq!(file.generation(), 9);
        assert_eq!(file.size(), 4096);

        let directory = AppDataDirectoryEntry::directory(b"settings").unwrap();
        let wire = directory.encode();
        assert_eq!(AppDataDirectoryEntry::decode(&wire), Ok(directory));
        assert_eq!(&wire[6..8], &2_u16.to_le_bytes());
        assert_eq!(&wire[16..32], &[0; 16]);
        assert_eq!(directory.generation(), 0);
        assert_eq!(directory.size(), 0);
    }

    #[test]
    fn app_data_directory_entries_reject_noncanonical_metadata_and_wire_bytes() {
        assert_eq!(
            AppDataDirectoryEntry::file(b"file", 0, 0),
            Err(AppDataDirectoryEntryError::InvalidGeneration)
        );
        assert_eq!(
            AppDataDirectoryEntry::file(b"file", 1, APP_DATA_FILE_MAX_BYTES + 1),
            Err(AppDataDirectoryEntryError::InvalidSize)
        );
        assert_eq!(
            AppDataDirectoryEntry::directory(b"bad//path"),
            Err(AppDataDirectoryEntryError::Path(
                AppDataPathError::EmptyComponent
            ))
        );

        let canonical = AppDataDirectoryEntry::file(b"file", 3, 4).unwrap().encode();
        assert_eq!(
            AppDataDirectoryEntry::decode(&canonical[..111]),
            Err(AppDataDirectoryEntryError::InvalidWireLength)
        );
        let mut malformed = canonical;
        set_test_u32(&mut malformed, 0, 0);
        assert_eq!(
            AppDataDirectoryEntry::decode(&malformed),
            Err(AppDataDirectoryEntryError::InvalidMagic)
        );
        let mut malformed = canonical;
        set_test_u16(&mut malformed, 4, 2);
        assert_eq!(
            AppDataDirectoryEntry::decode(&malformed),
            Err(AppDataDirectoryEntryError::UnsupportedVersion)
        );
        let mut malformed = canonical;
        set_test_u16(&mut malformed, 6, 0);
        assert_eq!(
            AppDataDirectoryEntry::decode(&malformed),
            Err(AppDataDirectoryEntryError::InvalidKind)
        );
        let mut malformed = canonical;
        set_test_u32(&mut malformed, 8, 1);
        assert_eq!(
            AppDataDirectoryEntry::decode(&malformed),
            Err(AppDataDirectoryEntryError::NonZeroFlags)
        );
        for offset in [32, 47] {
            let mut malformed = canonical;
            malformed[offset] = 1;
            assert_eq!(
                AppDataDirectoryEntry::decode(&malformed),
                Err(AppDataDirectoryEntryError::NonZeroReserved)
            );
        }
        let mut malformed = canonical;
        set_test_u32(&mut malformed, 12, 65);
        assert_eq!(
            AppDataDirectoryEntry::decode(&malformed),
            Err(AppDataDirectoryEntryError::Path(AppDataPathError::TooLong))
        );
        let mut malformed = canonical;
        malformed[48] = 0xff;
        assert_eq!(
            AppDataDirectoryEntry::decode(&malformed),
            Err(AppDataDirectoryEntryError::Path(
                AppDataPathError::InvalidUtf8
            ))
        );
        let mut malformed = canonical;
        malformed[52] = 1;
        assert_eq!(
            AppDataDirectoryEntry::decode(&malformed),
            Err(AppDataDirectoryEntryError::NonZeroPathPadding)
        );
        let mut malformed = canonical;
        set_test_u64(&mut malformed, 16, 0);
        assert_eq!(
            AppDataDirectoryEntry::decode(&malformed),
            Err(AppDataDirectoryEntryError::InvalidGeneration)
        );
        let mut malformed = canonical;
        set_test_u64(&mut malformed, 24, 4097);
        assert_eq!(
            AppDataDirectoryEntry::decode(&malformed),
            Err(AppDataDirectoryEntryError::InvalidSize)
        );

        let directory = AppDataDirectoryEntry::directory(b"dir").unwrap().encode();
        for (offset, value) in [(16, 1), (24, 1)] {
            let mut malformed = directory;
            set_test_u64(&mut malformed, offset, value);
            assert_eq!(
                AppDataDirectoryEntry::decode(&malformed),
                Err(AppDataDirectoryEntryError::NonCanonicalDirectoryMetadata)
            );
        }
    }

    #[test]
    fn storage_block_requests_have_one_canonical_wire_and_token_lifecycle() {
        assert_eq!(APP_DATA_VOLUME_SECTORS, 1_920);
        assert_eq!(STORAGE_SECTOR_SIZE, 512);
        assert_eq!(IPC_BUFFER_PAYLOAD_MAX_BYTES, 4_160);
        assert_eq!(IPC_BUFFER_CREATE_FLAGS_NONE, 0);
        assert_eq!(STORAGE_BLOCK_REQUEST_MAGIC, u32::from_le_bytes(*b"SBRQ"));
        assert_eq!(STORAGE_BLOCK_REQUEST_VERSION, 2);
        assert_eq!(STORAGE_BLOCK_MAX_SECTORS, 8);
        assert_eq!(STORAGE_BLOCK_DATA_MAX_BYTES, 4_096);
        assert_eq!(STORAGE_BLOCK_REQUEST_WIRE_SIZE, 4_160);
        assert_eq!(STORAGE_BLOCK_REQUEST_UNASSIGNED_TOKEN, 0);

        let operations = [
            StorageBlockOperation::Read,
            StorageBlockOperation::Write,
            StorageBlockOperation::Flush,
        ];
        for (offset, operation) in operations.into_iter().enumerate() {
            let raw = offset as u16 + 1;
            assert_eq!(operation.raw(), raw);
            assert_eq!(StorageBlockOperation::from_raw(raw), Some(operation));
        }
        assert_eq!(StorageBlockOperation::from_raw(0), None);
        assert_eq!(StorageBlockOperation::from_raw(4), None);
        assert_eq!(StorageBlockOperation::from_raw(u16::MAX), None);

        let read = StorageBlockRequest::read(APP_DATA_VOLUME_SECTORS - 1).unwrap();
        let read_wire = read.encode();
        assert_eq!(&read_wire[0..4], b"SBRQ");
        assert_eq!(&read_wire[4..6], &2_u16.to_le_bytes());
        assert_eq!(&read_wire[6..8], &1_u16.to_le_bytes());
        assert_eq!(
            &read_wire[8..16],
            &(APP_DATA_VOLUME_SECTORS - 1).to_le_bytes()
        );
        assert_eq!(&read_wire[16..24], &[0; 8]);
        assert_eq!(&read_wire[24..26], &1_u16.to_le_bytes());
        assert_eq!(&read_wire[26..64], &[0; 38]);
        assert_eq!(&read_wire[64..], &[0; STORAGE_BLOCK_DATA_MAX_BYTES]);
        assert_eq!(StorageBlockRequest::decode(&read_wire), Ok(read));
        assert_eq!(read.operation(), StorageBlockOperation::Read);
        assert_eq!(read.relative_lba(), APP_DATA_VOLUME_SECTORS - 1);
        assert_eq!(read.token(), 0);
        assert_eq!(read.sector_count(), 1);
        assert_eq!(read.byte_len(), STORAGE_SECTOR_SIZE);
        assert_eq!(read.data(), &[0; STORAGE_BLOCK_DATA_MAX_BYTES]);

        let maximum_read = StorageBlockRequest::read_batch(
            APP_DATA_VOLUME_SECTORS - STORAGE_BLOCK_MAX_SECTORS as u64,
            STORAGE_BLOCK_MAX_SECTORS,
        )
        .unwrap();
        let maximum_read_wire = maximum_read.encode();
        assert_eq!(
            &maximum_read_wire[24..26],
            &(STORAGE_BLOCK_MAX_SECTORS as u16).to_le_bytes()
        );
        assert_eq!(maximum_read.sector_count(), STORAGE_BLOCK_MAX_SECTORS);
        assert_eq!(maximum_read.byte_len(), STORAGE_BLOCK_DATA_MAX_BYTES);
        assert_eq!(
            StorageBlockRequest::decode_submission(&maximum_read_wire),
            Ok(maximum_read)
        );

        let mut data = [0; STORAGE_SECTOR_SIZE];
        for (index, byte) in data.iter_mut().enumerate() {
            *byte = index as u8;
        }
        let write = StorageBlockRequest::write(7, data).unwrap();
        let write_wire = write.encode();
        assert_eq!(&write_wire[6..8], &2_u16.to_le_bytes());
        assert_eq!(&write_wire[24..26], &1_u16.to_le_bytes());
        assert_eq!(&write_wire[64..64 + STORAGE_SECTOR_SIZE], &data);
        assert_eq!(
            &write_wire[64 + STORAGE_SECTOR_SIZE..],
            &[0; STORAGE_BLOCK_DATA_MAX_BYTES - STORAGE_SECTOR_SIZE]
        );
        assert_eq!(&write.data()[..STORAGE_SECTOR_SIZE], &data);
        assert_eq!(write.sector_count(), 1);
        assert_eq!(write.byte_len(), STORAGE_SECTOR_SIZE);
        assert_eq!(
            StorageBlockRequest::decode_submission(&write_wire),
            Ok(write)
        );

        let mut sectors = [[0; STORAGE_SECTOR_SIZE]; 3];
        for (sector_index, sector) in sectors.iter_mut().enumerate() {
            for (byte_index, byte) in sector.iter_mut().enumerate() {
                *byte = (sector_index as u8).wrapping_mul(17) ^ byte_index as u8;
            }
        }
        let write_batch = StorageBlockRequest::write_batch(21, &sectors).unwrap();
        let write_batch_wire = write_batch.encode();
        assert_eq!(write_batch.sector_count(), sectors.len());
        assert_eq!(write_batch.byte_len(), sectors.len() * STORAGE_SECTOR_SIZE);
        for (index, sector) in sectors.iter().enumerate() {
            let start = index * STORAGE_SECTOR_SIZE;
            assert_eq!(
                &write_batch.data()[start..start + STORAGE_SECTOR_SIZE],
                sector
            );
        }
        assert!(
            write_batch.data()[write_batch.byte_len()..]
                .iter()
                .all(|byte| *byte == 0)
        );
        assert_eq!(
            StorageBlockRequest::decode_submission(&write_batch_wire),
            Ok(write_batch)
        );

        let flush = StorageBlockRequest::flush();
        let flush_wire = flush.encode();
        assert_eq!(&flush_wire[6..8], &3_u16.to_le_bytes());
        assert_eq!(&flush_wire[8..], &[0; STORAGE_BLOCK_REQUEST_WIRE_SIZE - 8]);
        assert_eq!(flush.sector_count(), 0);
        assert_eq!(flush.byte_len(), 0);
        assert_eq!(flush.data(), &[0; STORAGE_BLOCK_DATA_MAX_BYTES]);
        assert_eq!(
            StorageBlockRequest::decode_submission(&flush_wire),
            Ok(flush)
        );

        assert_eq!(
            StorageBlockRequest::decode_delivery(&write_wire),
            Err(StorageBlockRequestError::MissingDeliveryToken)
        );
        assert_eq!(
            write.with_kernel_token(0),
            Err(StorageBlockRequestError::MissingDeliveryToken)
        );
        let delivered = write.with_kernel_token(0x0102_0304_0506_0708).unwrap();
        let delivered_wire = delivered.encode();
        assert_eq!(
            &delivered_wire[16..24],
            &0x0102_0304_0506_0708_u64.to_le_bytes()
        );
        assert_eq!(
            StorageBlockRequest::decode_delivery(&delivered_wire),
            Ok(delivered)
        );
        assert_eq!(
            StorageBlockRequest::decode_submission(&delivered_wire),
            Err(StorageBlockRequestError::NonZeroSubmissionToken)
        );
        assert_eq!(
            delivered.with_kernel_token(9),
            Err(StorageBlockRequestError::TokenAlreadyAssigned)
        );
    }

    #[test]
    fn storage_block_request_decode_rejects_every_noncanonical_field_class() {
        assert_eq!(
            StorageBlockRequest::read(APP_DATA_VOLUME_SECTORS),
            Err(StorageBlockRequestError::RelativeLbaOutOfRange)
        );
        assert_eq!(
            StorageBlockRequest::write(APP_DATA_VOLUME_SECTORS, [0; STORAGE_SECTOR_SIZE]),
            Err(StorageBlockRequestError::RelativeLbaOutOfRange)
        );
        for count in [0, STORAGE_BLOCK_MAX_SECTORS + 1] {
            assert_eq!(
                StorageBlockRequest::read_batch(0, count),
                Err(StorageBlockRequestError::InvalidSectorCount)
            );
        }
        assert_eq!(
            StorageBlockRequest::read_batch(
                APP_DATA_VOLUME_SECTORS - STORAGE_BLOCK_MAX_SECTORS as u64,
                STORAGE_BLOCK_MAX_SECTORS,
            )
            .unwrap()
            .byte_len(),
            STORAGE_BLOCK_DATA_MAX_BYTES
        );
        assert_eq!(
            StorageBlockRequest::read_batch(
                APP_DATA_VOLUME_SECTORS - STORAGE_BLOCK_MAX_SECTORS as u64 + 1,
                STORAGE_BLOCK_MAX_SECTORS,
            ),
            Err(StorageBlockRequestError::RelativeLbaOutOfRange)
        );
        assert_eq!(
            StorageBlockRequest::read_batch(u64::MAX, 1),
            Err(StorageBlockRequestError::RelativeLbaOutOfRange)
        );
        assert_eq!(
            StorageBlockRequest::write_batch(0, &[]),
            Err(StorageBlockRequestError::InvalidSectorCount)
        );
        assert_eq!(
            StorageBlockRequest::write_batch(
                0,
                &[[0; STORAGE_SECTOR_SIZE]; STORAGE_BLOCK_MAX_SECTORS + 1],
            ),
            Err(StorageBlockRequestError::InvalidSectorCount)
        );
        assert_eq!(
            StorageBlockRequest::write_batch(
                APP_DATA_VOLUME_SECTORS - 1,
                &[[0; STORAGE_SECTOR_SIZE]; 2],
            ),
            Err(StorageBlockRequestError::RelativeLbaOutOfRange)
        );

        let canonical = StorageBlockRequest::read_batch(1, 2).unwrap().encode();
        assert_eq!(
            StorageBlockRequest::decode(&canonical[..canonical.len() - 1]),
            Err(StorageBlockRequestError::InvalidWireLength)
        );
        let mut malformed = canonical;
        set_test_u32(&mut malformed, 0, 0);
        assert_eq!(
            StorageBlockRequest::decode(&malformed),
            Err(StorageBlockRequestError::InvalidMagic)
        );
        let mut malformed = canonical;
        set_test_u16(&mut malformed, 4, 1);
        assert_eq!(
            StorageBlockRequest::decode(&malformed),
            Err(StorageBlockRequestError::UnsupportedVersion)
        );
        for operation in [0, 4, u16::MAX] {
            let mut malformed = canonical;
            set_test_u16(&mut malformed, 6, operation);
            assert_eq!(
                StorageBlockRequest::decode(&malformed),
                Err(StorageBlockRequestError::InvalidOperation)
            );
        }
        for offset in [26, 63] {
            let mut malformed = canonical;
            malformed[offset] = 1;
            assert_eq!(
                StorageBlockRequest::decode(&malformed),
                Err(StorageBlockRequestError::NonZeroReserved)
            );
        }
        for count in [0, STORAGE_BLOCK_MAX_SECTORS as u16 + 1] {
            let mut malformed = canonical;
            set_test_u16(&mut malformed, 24, count);
            assert_eq!(
                StorageBlockRequest::decode(&malformed),
                Err(StorageBlockRequestError::InvalidSectorCount)
            );
        }
        let mut malformed = canonical;
        set_test_u64(&mut malformed, 8, APP_DATA_VOLUME_SECTORS - 1);
        assert_eq!(
            StorageBlockRequest::decode(&malformed),
            Err(StorageBlockRequestError::RelativeLbaOutOfRange)
        );
        let mut malformed = canonical;
        set_test_u64(&mut malformed, 16, 1);
        assert_eq!(
            StorageBlockRequest::decode(&malformed),
            Err(StorageBlockRequestError::NonZeroSubmissionToken)
        );
        for offset in [64, STORAGE_BLOCK_REQUEST_WIRE_SIZE - 1] {
            let mut malformed = canonical;
            malformed[offset] = 1;
            assert_eq!(
                StorageBlockRequest::decode(&malformed),
                Err(StorageBlockRequestError::NonZeroReadData)
            );
        }

        let canonical_write = StorageBlockRequest::write(3, [7; STORAGE_SECTOR_SIZE])
            .unwrap()
            .encode();
        let mut malformed = canonical_write;
        malformed[64 + STORAGE_SECTOR_SIZE] = 1;
        assert_eq!(
            StorageBlockRequest::decode(&malformed),
            Err(StorageBlockRequestError::NonZeroWritePadding)
        );

        let flush = StorageBlockRequest::flush().encode();
        let mut malformed = flush;
        set_test_u64(&mut malformed, 8, 1);
        assert_eq!(
            StorageBlockRequest::decode(&malformed),
            Err(StorageBlockRequestError::NonCanonicalFlushLba)
        );
        let mut malformed = flush;
        set_test_u16(&mut malformed, 24, 1);
        assert_eq!(
            StorageBlockRequest::decode(&malformed),
            Err(StorageBlockRequestError::NonCanonicalFlushSectorCount)
        );
        let mut malformed = flush;
        malformed[STORAGE_BLOCK_REQUEST_WIRE_SIZE - 1] = 1;
        assert_eq!(
            StorageBlockRequest::decode(&malformed),
            Err(StorageBlockRequestError::NonZeroFlushData)
        );
    }

    #[test]
    fn storage_session_binding_round_trips_exact_identity_and_rights() {
        assert_eq!(STORAGE_SESSION_BINDING_MAGIC, u32::from_le_bytes(*b"SSBN"));
        assert_eq!(STORAGE_SESSION_BINDING_VERSION, 1);
        assert_eq!(STORAGE_SESSION_BINDING_WIRE_SIZE, 64);
        assert_eq!(
            STORAGE_CONNECT_RIGHTS_MASK,
            Rights::READ.bits() | Rights::WRITE.bits()
        );

        let granted = Rights::from_bits(STORAGE_CONNECT_RIGHTS_MASK).unwrap();
        let binding = StorageSessionBinding::new(
            0x0102_0304_0506_0708,
            0x1112_1314_1516_1718,
            AppDataPrincipal::LAUNCHER,
            granted,
            0x2122_2324_2526_2728,
        )
        .unwrap();
        let wire = binding.encode();
        assert_eq!(&wire[0..4], b"SSBN");
        assert_eq!(&wire[4..8], &[1, 0, 0, 0]);
        assert_eq!(&wire[8..16], &0x0102_0304_0506_0708_u64.to_le_bytes());
        assert_eq!(&wire[16..24], &0x1112_1314_1516_1718_u64.to_le_bytes());
        assert_eq!(
            &wire[24..32],
            &AppDataPrincipal::LAUNCHER.raw().to_le_bytes()
        );
        assert_eq!(&wire[32..36], &STORAGE_CONNECT_RIGHTS_MASK.to_le_bytes());
        assert_eq!(&wire[36..40], &[0; 4]);
        assert_eq!(&wire[40..48], &0x2122_2324_2526_2728_u64.to_le_bytes());
        assert_eq!(&wire[48..], &[0; 16]);
        assert_eq!(StorageSessionBinding::decode(&wire), Ok(binding));
        assert_eq!(binding.session_id(), 0x0102_0304_0506_0708);
        assert_eq!(binding.client_pid(), 0x1112_1314_1516_1718);
        assert_eq!(binding.principal(), AppDataPrincipal::LAUNCHER);
        assert_eq!(binding.granted_rights(), granted);
        assert_eq!(binding.server_epoch(), 0x2122_2324_2526_2728);

        assert!(
            StorageSessionBinding::new(1, 2, AppDataPrincipal::PRIMARY_APP, Rights::READ, 3)
                .is_ok()
        );
        assert!(
            StorageSessionBinding::new(1, 2, AppDataPrincipal::PRIMARY_APP, Rights::WRITE, 3)
                .is_ok()
        );
        assert_eq!(
            StorageSessionBinding::new(1, 2, AppDataPrincipal::PRIMARY_APP, Rights::NONE, 3),
            Err(StorageSessionBindingError::InvalidGrantedRights)
        );
        assert_eq!(
            StorageSessionBinding::new(1, 2, AppDataPrincipal::PRIMARY_APP, Rights::WAIT, 3),
            Err(StorageSessionBindingError::InvalidGrantedRights)
        );
    }

    #[test]
    fn storage_session_binding_decode_rejects_every_noncanonical_field_class() {
        let canonical =
            StorageSessionBinding::new(1, 2, AppDataPrincipal::PRIMARY_APP, Rights::READ, 3)
                .unwrap()
                .encode();
        assert_eq!(
            StorageSessionBinding::decode(&canonical[..canonical.len() - 1]),
            Err(StorageSessionBindingError::InvalidWireLength)
        );
        let mut malformed = canonical;
        set_test_u32(&mut malformed, 0, 0);
        assert_eq!(
            StorageSessionBinding::decode(&malformed),
            Err(StorageSessionBindingError::InvalidMagic)
        );
        let mut malformed = canonical;
        set_test_u16(&mut malformed, 4, 2);
        assert_eq!(
            StorageSessionBinding::decode(&malformed),
            Err(StorageSessionBindingError::UnsupportedVersion)
        );
        for offset in [6, 7, 36, 39, 48, 63] {
            let mut malformed = canonical;
            malformed[offset] = 1;
            assert_eq!(
                StorageSessionBinding::decode(&malformed),
                Err(StorageSessionBindingError::NonZeroReserved)
            );
        }
        for (offset, error) in [
            (8, StorageSessionBindingError::InvalidSessionId),
            (16, StorageSessionBindingError::InvalidClientPid),
            (24, StorageSessionBindingError::InvalidPrincipal),
            (40, StorageSessionBindingError::InvalidServerEpoch),
        ] {
            let mut malformed = canonical;
            set_test_u64(&mut malformed, offset, 0);
            assert_eq!(StorageSessionBinding::decode(&malformed), Err(error));
        }
        for rights in [0, Rights::WAIT.bits(), 1 << 31] {
            let mut malformed = canonical;
            set_test_u32(&mut malformed, 32, rights);
            assert_eq!(
                StorageSessionBinding::decode(&malformed),
                Err(StorageSessionBindingError::InvalidGrantedRights)
            );
        }
    }

    fn set_test_u16(wire: &mut [u8], offset: usize, value: u16) {
        wire[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn set_test_u32(wire: &mut [u8], offset: usize, value: u32) {
        wire[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn set_test_u64(wire: &mut [u8], offset: usize, value: u64) {
        wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn process_termination_contract_is_stable_and_strict() {
        let stable = [
            ProcessTerminationReason::Exited,
            ProcessTerminationReason::Faulted,
            ProcessTerminationReason::Killed,
        ];
        for (offset, expected) in stable.into_iter().enumerate() {
            let raw = offset as u64 + 1;
            assert_eq!(expected.raw(), raw);
            assert_eq!(ProcessTerminationReason::from_raw(raw), Some(expected));
        }
        assert_eq!(ProcessTerminationReason::from_raw(0), None);
        assert_eq!(ProcessTerminationReason::from_raw(4), None);
        assert_eq!(ProcessTerminationReason::from_raw(u64::MAX), None);
        assert_eq!(PROCESS_TERMINATE_FLAGS_NONE, 0);
        assert_eq!(PROCESS_KILLED_EXIT_CODE, 137);
    }

    #[test]
    fn channel_message_kinds_are_stable_and_reject_unknown_values() {
        let stable = [
            ChannelMessageKind::Scalar,
            ChannelMessageKind::Bytes,
            ChannelMessageKind::Transfer,
        ];
        for (offset, expected) in stable.into_iter().enumerate() {
            let raw = offset as u64 + 1;
            assert_eq!(expected.raw(), raw);
            assert_eq!(ChannelMessageKind::from_raw(raw), Some(expected));
        }
        assert_eq!(ChannelMessageKind::from_raw(0), None);
        assert_eq!(ChannelMessageKind::from_raw(4), None);
        assert_eq!(ChannelMessageKind::from_raw(u64::MAX), None);
    }

    #[test]
    fn scalar_envelope_has_a_canonical_112_byte_little_endian_wire_format() {
        assert_eq!(CHANNEL_READ_ENVELOPE_SIZE, 112);
        let envelope = ChannelReadEnvelope::scalar(
            0x0102_0304_0506_0708,
            0x1112_1314_1516_1718,
            0x2122_2324_2526_2728,
        )
        .unwrap();
        let wire = envelope.encode();
        assert_eq!(&wire[0..8], &0x0102_0304_0506_0708_u64.to_le_bytes());
        assert_eq!(&wire[8..16], &1_u64.to_le_bytes());
        assert_eq!(&wire[16..24], &16_u64.to_le_bytes());
        assert_eq!(&wire[24..32], &[0; 8]);
        assert_eq!(&wire[32..40], &0x1112_1314_1516_1718_u64.to_le_bytes());
        assert_eq!(&wire[40..48], &0x2122_2324_2526_2728_u64.to_le_bytes());
        assert_eq!(&wire[48..], &[0; CHANNEL_MESSAGE_MAX_BYTES]);
        assert_eq!(ChannelReadEnvelope::decode(&wire), Ok(envelope));
        assert_eq!(envelope.sender_pid(), 0x0102_0304_0506_0708);
        assert_eq!(envelope.kind(), ChannelMessageKind::Scalar);
        assert_eq!(envelope.logical_length(), 16);
        assert_eq!(envelope.received_handle(), HandleValue::INVALID);
        assert_eq!(
            envelope.scalar_values(),
            Some((0x1112_1314_1516_1718, 0x2122_2324_2526_2728))
        );
        assert_eq!(envelope.payload_bytes(), None);
    }

    #[test]
    fn byte_envelopes_round_trip_boundaries_and_reject_noncanonical_tail() {
        let empty = ChannelReadEnvelope::bytes(7, [0; CHANNEL_MESSAGE_MAX_BYTES], 0).unwrap();
        assert_eq!(empty.payload_bytes(), Some(&[][..]));
        assert_eq!(ChannelReadEnvelope::decode(&empty.encode()), Ok(empty));

        let mut full_data = [0; CHANNEL_MESSAGE_MAX_BYTES];
        for (index, byte) in full_data.iter_mut().enumerate() {
            *byte = index as u8;
        }
        let full = ChannelReadEnvelope::bytes(8, full_data, CHANNEL_MESSAGE_MAX_BYTES).unwrap();
        assert_eq!(full.payload_bytes(), Some(&full_data[..]));
        assert_eq!(ChannelReadEnvelope::decode(&full.encode()), Ok(full));

        let mut noncanonical = [0; CHANNEL_MESSAGE_MAX_BYTES];
        noncanonical[4] = 1;
        assert_eq!(
            ChannelReadEnvelope::bytes(9, noncanonical, 4),
            Err(ChannelReadEnvelopeError::NonZeroDataTail)
        );
        assert_eq!(
            ChannelReadEnvelope::bytes(9, [0; CHANNEL_MESSAGE_MAX_BYTES], 65),
            Err(ChannelReadEnvelopeError::InvalidLogicalLength)
        );
    }

    #[test]
    fn transfer_envelopes_require_a_real_received_handle_and_round_trip() {
        let mut data = [0; CHANNEL_MESSAGE_MAX_BYTES];
        data[..4].copy_from_slice(b"move");
        let handle = HandleValue::from_raw(0xfedc_ba98);
        let envelope = ChannelReadEnvelope::transfer(11, handle, data, 4).unwrap();
        assert_eq!(envelope.kind(), ChannelMessageKind::Transfer);
        assert_eq!(envelope.received_handle(), handle);
        assert_eq!(envelope.payload_bytes(), Some(&b"move"[..]));
        assert_eq!(
            ChannelReadEnvelope::decode(&envelope.encode()),
            Ok(envelope)
        );
        assert_eq!(
            ChannelReadEnvelope::transfer(11, HandleValue::INVALID, data, 4),
            Err(ChannelReadEnvelopeError::InvalidReceivedHandle)
        );
    }

    #[test]
    fn envelope_decode_rejects_unknown_and_cross_kind_fields() {
        assert_eq!(
            ChannelReadEnvelope::decode(&[0; CHANNEL_READ_ENVELOPE_SIZE - 1]),
            Err(ChannelReadEnvelopeError::InvalidWireLength)
        );
        let scalar = ChannelReadEnvelope::scalar(12, 13, 14).unwrap().encode();

        let mut malformed = scalar;
        set_wire_u64(&mut malformed, 0, 0);
        assert_eq!(
            ChannelReadEnvelope::decode(&malformed),
            Err(ChannelReadEnvelopeError::InvalidSenderPid)
        );
        let mut malformed = scalar;
        set_wire_u64(&mut malformed, 8, 99);
        assert_eq!(
            ChannelReadEnvelope::decode(&malformed),
            Err(ChannelReadEnvelopeError::UnknownKind)
        );
        let mut malformed = scalar;
        set_wire_u64(&mut malformed, 16, 15);
        assert_eq!(
            ChannelReadEnvelope::decode(&malformed),
            Err(ChannelReadEnvelopeError::InvalidLogicalLength)
        );
        let mut malformed = scalar;
        set_wire_u64(&mut malformed, 24, 1);
        assert_eq!(
            ChannelReadEnvelope::decode(&malformed),
            Err(ChannelReadEnvelopeError::UnexpectedReceivedHandle)
        );
        let mut malformed = scalar;
        malformed[48] = 1;
        assert_eq!(
            ChannelReadEnvelope::decode(&malformed),
            Err(ChannelReadEnvelopeError::NonZeroScalarData)
        );

        let bytes = ChannelReadEnvelope::bytes(15, [0; CHANNEL_MESSAGE_MAX_BYTES], 0)
            .unwrap()
            .encode();
        let mut malformed = bytes;
        set_wire_u64(&mut malformed, 24, 1);
        assert_eq!(
            ChannelReadEnvelope::decode(&malformed),
            Err(ChannelReadEnvelopeError::UnexpectedReceivedHandle)
        );
        let mut malformed = bytes;
        set_wire_u64(&mut malformed, 32, 1);
        assert_eq!(
            ChannelReadEnvelope::decode(&malformed),
            Err(ChannelReadEnvelopeError::UnexpectedScalarFields)
        );
        let mut malformed = bytes;
        set_wire_u64(&mut malformed, 40, 1);
        assert_eq!(
            ChannelReadEnvelope::decode(&malformed),
            Err(ChannelReadEnvelopeError::UnexpectedScalarFields)
        );
        let mut malformed = bytes;
        malformed[111] = 1;
        assert_eq!(
            ChannelReadEnvelope::decode(&malformed),
            Err(ChannelReadEnvelopeError::NonZeroDataTail)
        );

        let transfer = ChannelReadEnvelope::transfer(
            16,
            HandleValue::from_raw(1),
            [0; CHANNEL_MESSAGE_MAX_BYTES],
            0,
        )
        .unwrap()
        .encode();
        let mut malformed = transfer;
        set_wire_u64(&mut malformed, 24, 0);
        assert_eq!(
            ChannelReadEnvelope::decode(&malformed),
            Err(ChannelReadEnvelopeError::InvalidReceivedHandle)
        );
        let mut malformed = transfer;
        set_wire_u64(&mut malformed, 24, u64::from(u32::MAX) + 1);
        assert_eq!(
            ChannelReadEnvelope::decode(&malformed),
            Err(ChannelReadEnvelopeError::InvalidReceivedHandle)
        );
    }

    fn set_wire_u64(wire: &mut [u8; CHANNEL_READ_ENVELOPE_SIZE], offset: usize, value: u64) {
        wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    #[cfg(feature = "androidbox-process0")]
    #[test]
    fn android_app_process_messages_are_fixed_canonical_and_bounded() {
        use super::{
            ABI_VERSION, ANDROID_APP_MESSAGE_PAYLOAD_BYTES, AndroidAppBootstrap,
            AndroidAppBootstrapKind, AndroidAppMessage, AndroidAppMessageKind,
        };

        for kind in [
            AndroidAppBootstrapKind::SurfaceClient,
            AndroidAppBootstrapKind::RuntimeClient,
        ] {
            let bootstrap = AndroidAppBootstrap::new(kind);
            assert_eq!(
                AndroidAppBootstrap::decode(&bootstrap.encode()),
                Some(bootstrap)
            );
        }

        let ready =
            AndroidAppMessage::new(AndroidAppMessageKind::Ready, 0, ABI_VERSION, 0, &[]).unwrap();
        assert_eq!(AndroidAppMessage::decode(&ready.encode()), Ok(ready));

        let open = AndroidAppMessage::new(AndroidAppMessageKind::Open, 7, 41, 3, &[]).unwrap();
        assert_eq!(AndroidAppMessage::decode(&open.encode()), Ok(open));

        let text = [b'A'; ANDROID_APP_MESSAGE_PAYLOAD_BYTES];
        let chunk = AndroidAppMessage::new(
            AndroidAppMessageKind::LabelChunk,
            7,
            (24_u64 << 32) | 48,
            0,
            &text,
        )
        .unwrap();
        assert_eq!(chunk.chunk_offset(), 24);
        assert_eq!(chunk.chunk_total(), 48);
        assert_eq!(chunk.payload(), &text);
        assert_eq!(AndroidAppMessage::decode(&chunk.encode()), Ok(chunk));

        let click = AndroidAppMessage::new(
            AndroidAppMessageKind::Click,
            8,
            41,
            (0x7f01_0000_u64 << 32) | 3,
            &[],
        )
        .unwrap();
        assert_eq!(AndroidAppMessage::decode(&click.encode()), Ok(click));
        let close = AndroidAppMessage::new(AndroidAppMessageKind::Close, 9, 41, 3, &[]).unwrap();
        assert_eq!(AndroidAppMessage::decode(&close.encode()), Ok(close));
        let closed = AndroidAppMessage::new(AndroidAppMessageKind::Closed, 9, 0, 0, &[]).unwrap();
        assert_eq!(AndroidAppMessage::decode(&closed.encode()), Ok(closed));
        let updated = AndroidAppMessage::new(
            AndroidAppMessageKind::Updated,
            8,
            0x7f00_0000,
            (1_u64 << 32) | 24,
            &[],
        )
        .unwrap();
        assert_eq!(AndroidAppMessage::decode(&updated.encode()), Ok(updated));
        #[cfg(feature = "androidbox-dex-methods8")]
        {
            let app_call = AndroidAppMessage::new(
                AndroidAppMessageKind::Updated,
                8,
                (1_u64 << 32) | 0x7f00_0000,
                (1_u64 << 32) | 24,
                &[],
            )
            .unwrap();
            assert_eq!(AndroidAppMessage::decode(&app_call.encode()), Ok(app_call));
            assert!(
                AndroidAppMessage::new(
                    AndroidAppMessageKind::Updated,
                    8,
                    (2_u64 << 32) | 0x7f00_0000,
                    (1_u64 << 32) | 24,
                    &[],
                )
                .is_err()
            );
        }
        #[cfg(all(
            feature = "androidbox-dex-instance9",
            not(feature = "androidbox-activity-fields10")
        ))]
        {
            let instance_call = AndroidAppMessage::new(
                AndroidAppMessageKind::Updated,
                8,
                (1_u64 << 40) | (1_u64 << 32) | 0x7f00_0000,
                (1_u64 << 32) | 24,
                &[],
            )
            .unwrap();
            assert_eq!(
                AndroidAppMessage::decode(&instance_call.encode()),
                Ok(instance_call)
            );
            for invalid_provenance in [1_u64 << 40, 2_u64 << 40 | 1_u64 << 32, 1_u64 << 48] {
                assert!(
                    AndroidAppMessage::new(
                        AndroidAppMessageKind::Updated,
                        8,
                        invalid_provenance | 0x7f00_0000,
                        (1_u64 << 32) | 24,
                        &[],
                    )
                    .is_err()
                );
            }
        }
        #[cfg(all(
            feature = "androidbox-activity-fields10",
            not(feature = "androidbox-activity-state11")
        ))]
        {
            let field_read = AndroidAppMessage::new(
                AndroidAppMessageKind::Updated,
                8,
                (1_u64 << 48) | (1_u64 << 40) | (1_u64 << 32) | 0x7f00_0000,
                (1_u64 << 32) | 24,
                &[],
            )
            .unwrap();
            assert_eq!(
                AndroidAppMessage::decode(&field_read.encode()),
                Ok(field_read)
            );
            for invalid_provenance in [2_u64 << 48, 2_u64 << 40 | 1_u64 << 32, 1_u64 << 56] {
                assert!(
                    AndroidAppMessage::new(
                        AndroidAppMessageKind::Updated,
                        8,
                        invalid_provenance | 0x7f00_0000,
                        (1_u64 << 32) | 24,
                        &[],
                    )
                    .is_err()
                );
            }
        }
        #[cfg(feature = "androidbox-activity-state11")]
        {
            let first_state = AndroidAppMessage::new(
                AndroidAppMessageKind::Updated,
                8,
                (1_u64 << 56) | (2_u64 << 48) | (1_u64 << 40) | (1_u64 << 32) | 0x7f00_0000,
                (1_u64 << 32) | 24,
                &[],
            )
            .unwrap();
            assert_eq!(
                AndroidAppMessage::decode(&first_state.encode()),
                Ok(first_state)
            );
            let second_state = AndroidAppMessage::new(
                AndroidAppMessageKind::Updated,
                8,
                (2_u64 << 56) | (2_u64 << 48) | (1_u64 << 40) | (1_u64 << 32) | 0x7f00_0000,
                (2_u64 << 32) | 24,
                &[],
            )
            .unwrap();
            assert_eq!(
                AndroidAppMessage::decode(&second_state.encode()),
                Ok(second_state)
            );
            for invalid_provenance in [
                3_u64 << 48,
                2_u64 << 40 | 1_u64 << 32,
                1_u64 << 56 | 1_u64 << 48 | 1_u64 << 40 | 1_u64 << 32,
                1_u64 << 56 | 2_u64 << 48 | 1_u64 << 32,
            ] {
                assert!(
                    AndroidAppMessage::new(
                        AndroidAppMessageKind::Updated,
                        8,
                        invalid_provenance | 0x7f00_0000,
                        (1_u64 << 32) | 24,
                        &[],
                    )
                    .is_err()
                );
            }
        }
        #[cfg(feature = "androidbox-string-text12")]
        {
            let direct_string = AndroidAppMessage::new(
                AndroidAppMessageKind::Updated,
                8,
                (1_u64 << 56) | (1_u64 << 55) | (2_u64 << 48) | 0x7f00_0000,
                (1_u64 << 32) | 21,
                &[],
            )
            .unwrap();
            assert_eq!(
                AndroidAppMessage::decode(&direct_string.encode()),
                Ok(direct_string)
            );
            for invalid_provenance in [
                (1_u64 << 55) | (2_u64 << 48),
                (1_u64 << 56) | (1_u64 << 55) | (1_u64 << 48),
                (1_u64 << 56) | (1_u64 << 55) | (2_u64 << 48) | (1_u64 << 32),
                (1_u64 << 56) | (1_u64 << 55) | (2_u64 << 48) | (1_u64 << 40),
            ] {
                assert!(
                    AndroidAppMessage::new(
                        AndroidAppMessageKind::Updated,
                        8,
                        invalid_provenance | 0x7f00_0000,
                        (1_u64 << 32) | 21,
                        &[],
                    )
                    .is_err()
                );
            }
        }
        #[cfg(feature = "androidbox-string-builder13")]
        {
            let dynamic_string = AndroidAppMessage::new(
                AndroidAppMessageKind::Updated,
                9,
                (1_u64 << 56) | (1_u64 << 55) | (1_u64 << 54) | (2_u64 << 48) | 0x7f00_0000,
                (1_u64 << 32) | 15,
                &[],
            )
            .unwrap();
            assert_eq!(
                AndroidAppMessage::decode(&dynamic_string.encode()),
                Ok(dynamic_string)
            );
            assert!(
                AndroidAppMessage::new(
                    AndroidAppMessageKind::Updated,
                    9,
                    (1_u64 << 56) | (1_u64 << 54) | (2_u64 << 48) | 0x7f00_0000,
                    (1_u64 << 32) | 15,
                    &[],
                )
                .is_err()
            );
        }
        let error = AndroidAppMessage::new(AndroidAppMessageKind::Error, 10, 1, 0, &[]).unwrap();
        assert_eq!(AndroidAppMessage::decode(&error.encode()), Ok(error));

        let opened = AndroidAppMessage::new(
            AndroidAppMessageKind::Opened,
            7,
            (0x7f00_0000_u64 << 32) | 0x7f01_0000,
            ((super::ANDROID_APP_LABEL_MAX_BYTES as u64) << 16)
                | super::ANDROID_APP_BUTTON_MAX_BYTES as u64,
            &[],
        )
        .unwrap();
        let mut maximum_open_response_messages = 1_usize;
        for (kind, total) in [
            (
                AndroidAppMessageKind::LabelChunk,
                super::ANDROID_APP_LABEL_MAX_BYTES,
            ),
            (
                AndroidAppMessageKind::ButtonChunk,
                super::ANDROID_APP_BUTTON_MAX_BYTES,
            ),
        ] {
            let mut offset = 0_usize;
            while offset < total {
                let length = (total - offset).min(ANDROID_APP_MESSAGE_PAYLOAD_BYTES);
                let message = AndroidAppMessage::new(
                    kind,
                    7,
                    ((offset as u64) << 32) | total as u64,
                    0,
                    &text[..length],
                )
                .unwrap();
                assert_eq!(AndroidAppMessage::decode(&message.encode()), Ok(message));
                maximum_open_response_messages += 1;
                offset += length;
            }
        }
        assert_eq!(AndroidAppMessage::decode(&opened.encode()), Ok(opened));
        assert_eq!(maximum_open_response_messages, 8);

        assert!(
            AndroidAppMessage::new(AndroidAppMessageKind::Ready, 0, ABI_VERSION - 1, 0, &[],)
                .is_err()
        );
        assert!(
            AndroidAppMessage::new(
                AndroidAppMessageKind::LabelChunk,
                7,
                (48_u64 << 32) | 48,
                0,
                b"X",
            )
            .is_err()
        );
        assert!(
            AndroidAppMessage::new(AndroidAppMessageKind::ButtonChunk, 7, 65, 0, b"X",).is_err()
        );
        assert!(
            AndroidAppMessage::new(AndroidAppMessageKind::UpdateTextChunk, 7, 1, 0, b"\n",)
                .is_err()
        );

        let mut noncanonical = ready.encode();
        noncanonical[15] = 1;
        assert!(AndroidAppMessage::decode(&noncanonical).is_err());
        let mut noncanonical = ready.encode();
        noncanonical[63] = 1;
        assert!(AndroidAppMessage::decode(&noncanonical).is_err());

        let mut bootstrap =
            AndroidAppBootstrap::new(AndroidAppBootstrapKind::SurfaceClient).encode();
        bootstrap[15] = 1;
        assert!(AndroidAppBootstrap::decode(&bootstrap).is_none());
    }

    #[cfg(all(
        feature = "androidbox-restart0",
        not(feature = "androidbox-scene-rpc2")
    ))]
    #[test]
    fn android_app_restart_wire_remains_exactly_bndapc01_version_1() {
        use super::{ABI_VERSION, AndroidAppMessage, AndroidAppMessageKind};

        assert_eq!(ABI_VERSION, 48);
        let ready =
            AndroidAppMessage::new(AndroidAppMessageKind::Ready, 0, ABI_VERSION, 0, &[]).unwrap();
        let wire = ready.encode();
        assert_eq!(&wire[..8], b"BNDAPC01");
        assert_eq!(wire[8], 1);
        assert_eq!(wire[9], AndroidAppMessageKind::Ready.raw());
        assert_eq!(wire[10], 0);
        assert!(wire[11..16].iter().all(|byte| *byte == 0));
        assert_eq!(&wire[16..24], &0_u64.to_le_bytes());
        assert_eq!(&wire[24..32], &48_u64.to_le_bytes());
        assert!(wire[32..].iter().all(|byte| *byte == 0));
        assert_eq!(AndroidAppMessageKind::from_raw(13), None);
        assert_eq!(AndroidAppMessage::decode(&wire), Ok(ready));
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn android_app_scene_rpc2_nodes_round_trip_without_allocation() {
        use super::{
            ABI_VERSION, ANDROID_APP_BUTTON_MAX_BYTES, ANDROID_APP_MESSAGE_PAYLOAD_BYTES,
            ANDROID_APP_SCENE_MAX_NODES, ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE,
            ANDROID_APP_SCENE_NODE_TEXT_MAX_BYTES, AndroidAppMessage, AndroidAppMessageKind,
            AndroidAppSceneDimension, AndroidAppSceneNodeDescriptor, AndroidAppSceneNodeKind,
            AndroidAppSceneOrientation,
        };

        #[cfg(feature = "androidbox-layout-mixed19")]
        assert_eq!(ABI_VERSION, 69);
        #[cfg(all(
            feature = "androidbox-layout-size18",
            not(feature = "androidbox-layout-mixed19")
        ))]
        assert_eq!(ABI_VERSION, 68);
        #[cfg(all(
            feature = "androidbox-layout-directional17",
            not(feature = "androidbox-layout-size18")
        ))]
        assert_eq!(ABI_VERSION, 67);
        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        assert_eq!(ABI_VERSION, 66);
        #[cfg(all(
            feature = "androidbox-layout-weight15",
            not(feature = "androidbox-layout-spacing16")
        ))]
        assert_eq!(ABI_VERSION, 65);
        #[cfg(all(
            feature = "androidbox-layout-row14",
            not(feature = "androidbox-layout-weight15")
        ))]
        assert_eq!(ABI_VERSION, 64);
        #[cfg(all(
            feature = "androidbox-string-builder13",
            not(feature = "androidbox-layout-row14")
        ))]
        assert_eq!(ABI_VERSION, 63);
        #[cfg(all(
            feature = "androidbox-string-text12",
            not(feature = "androidbox-string-builder13")
        ))]
        assert_eq!(ABI_VERSION, 62);
        #[cfg(all(
            feature = "androidbox-activity-state11",
            not(feature = "androidbox-string-text12")
        ))]
        assert_eq!(ABI_VERSION, 61);
        #[cfg(all(
            feature = "androidbox-activity-fields10",
            not(feature = "androidbox-activity-state11")
        ))]
        assert_eq!(ABI_VERSION, 60);
        #[cfg(all(
            feature = "androidbox-dex-instance9",
            not(feature = "androidbox-activity-fields10")
        ))]
        assert_eq!(ABI_VERSION, 59);
        #[cfg(all(
            feature = "androidbox-dex-methods8",
            not(feature = "androidbox-dex-instance9")
        ))]
        assert_eq!(ABI_VERSION, 58);
        #[cfg(all(
            feature = "androidbox-density-icons7",
            not(feature = "androidbox-dex-methods8")
        ))]
        assert_eq!(ABI_VERSION, 57);
        #[cfg(all(
            feature = "androidbox-icon-resources5",
            not(feature = "androidbox-density-icons7")
        ))]
        assert_eq!(ABI_VERSION, 56);
        #[cfg(all(
            feature = "androidbox-multipackage4",
            not(feature = "androidbox-icon-resources5")
        ))]
        assert_eq!(ABI_VERSION, 55);
        #[cfg(all(
            feature = "androidbox-manifest-catalog3",
            not(feature = "androidbox-multipackage4")
        ))]
        assert_eq!(ABI_VERSION, 54);
        #[cfg(all(
            feature = "androidbox-runtime-install2",
            not(feature = "androidbox-manifest-catalog3")
        ))]
        assert_eq!(ABI_VERSION, 53);
        #[cfg(all(
            feature = "androidbox-runtime-uninstall1",
            not(feature = "androidbox-runtime-install2")
        ))]
        assert_eq!(ABI_VERSION, 52);
        #[cfg(all(
            feature = "androidbox-apk-envelope4",
            not(feature = "androidbox-runtime-uninstall1")
        ))]
        assert_eq!(ABI_VERSION, 51);
        #[cfg(all(
            feature = "androidbox-multiaction3",
            not(feature = "androidbox-apk-envelope4")
        ))]
        assert_eq!(ABI_VERSION, 50);
        #[cfg(not(feature = "androidbox-multiaction3"))]
        assert_eq!(ABI_VERSION, 49);
        assert_eq!(ANDROID_APP_SCENE_MAX_NODES, 8);
        assert_eq!(ANDROID_APP_SCENE_NODE_TEXT_MAX_BYTES, 96);
        #[cfg(feature = "androidbox-layout-directional17")]
        assert_eq!(ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE, 24);
        #[cfg(not(feature = "androidbox-layout-directional17"))]
        assert_eq!(ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE, 16);
        assert!(ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE <= ANDROID_APP_MESSAGE_PAYLOAD_BYTES);

        let root = AndroidAppSceneNodeDescriptor::new(
            AndroidAppSceneNodeKind::LinearLayout,
            None,
            0,
            AndroidAppSceneDimension::MatchParent,
            AndroidAppSceneDimension::WrapContent,
            AndroidAppSceneOrientation::Vertical,
            false,
            0,
        )
        .unwrap();
        let label = AndroidAppSceneNodeDescriptor::new(
            AndroidAppSceneNodeKind::TextView,
            Some(0),
            0x7f01_0000,
            AndroidAppSceneDimension::MatchParent,
            AndroidAppSceneDimension::WrapContent,
            AndroidAppSceneOrientation::None,
            false,
            ANDROID_APP_SCENE_NODE_TEXT_MAX_BYTES as u8,
        )
        .unwrap();
        let button = AndroidAppSceneNodeDescriptor::new(
            AndroidAppSceneNodeKind::Button,
            Some(0),
            0x7f01_0001,
            AndroidAppSceneDimension::WrapContent,
            AndroidAppSceneDimension::WrapContent,
            AndroidAppSceneOrientation::None,
            true,
            ANDROID_APP_BUTTON_MAX_BYTES as u8,
        )
        .unwrap();

        for (index, descriptor) in [(0, root), (1, label), (2, button)] {
            assert!(descriptor.is_canonical_for_index(index));
            assert_eq!(
                AndroidAppSceneNodeDescriptor::decode(&descriptor.encode()),
                Some(descriptor)
            );
            let node = AndroidAppMessage::new(
                AndroidAppMessageKind::Node,
                7,
                u64::from(index),
                0,
                &descriptor.encode(),
            )
            .unwrap();
            assert_eq!(AndroidAppMessage::decode(&node.encode()), Ok(node));
        }

        assert_eq!(root.kind(), AndroidAppSceneNodeKind::LinearLayout);
        assert_eq!(root.parent(), None);
        assert_eq!(root.id(), 0);
        assert_eq!(root.width(), AndroidAppSceneDimension::MatchParent);
        assert_eq!(root.height(), AndroidAppSceneDimension::WrapContent);
        assert_eq!(root.orientation(), AndroidAppSceneOrientation::Vertical);
        assert!(!root.callback());
        assert_eq!(root.text_len(), 0);
        assert_eq!(label.parent(), Some(0));
        assert_eq!(label.text_len(), 96);
        assert!(button.callback());

        #[cfg(feature = "androidbox-layout-weight15")]
        {
            let weighted = AndroidAppSceneNodeDescriptor::new_weighted(
                AndroidAppSceneNodeKind::Button,
                Some(1),
                0x7f01_0002,
                AndroidAppSceneDimension::Zero,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::None,
                true,
                12,
                1,
            )
            .expect("bounded 0dp weighted Button");
            let weighted_wire = weighted.encode();
            #[cfg(feature = "androidbox-layout-size18")]
            assert_eq!(weighted_wire[0], 5);
            #[cfg(all(
                feature = "androidbox-layout-directional17",
                not(feature = "androidbox-layout-size18")
            ))]
            assert_eq!(weighted_wire[0], 4);
            #[cfg(all(
                feature = "androidbox-layout-spacing16",
                not(feature = "androidbox-layout-directional17")
            ))]
            assert_eq!(weighted_wire[0], 3);
            #[cfg(not(feature = "androidbox-layout-spacing16"))]
            assert_eq!(weighted_wire[0], 2);
            assert_eq!(weighted_wire[3], AndroidAppSceneDimension::Zero.raw());
            assert_eq!(weighted_wire[12], 1);
            assert!(weighted_wire[13..].iter().all(|byte| *byte == 0));
            assert_eq!(
                AndroidAppSceneNodeDescriptor::decode(&weighted_wire),
                Some(weighted)
            );
            assert_eq!(weighted.layout_weight(), 1);
            assert!(weighted.is_canonical_for_index(2));
            assert!(
                AndroidAppSceneNodeDescriptor::new(
                    AndroidAppSceneNodeKind::Button,
                    Some(1),
                    0x7f01_0002,
                    AndroidAppSceneDimension::Zero,
                    AndroidAppSceneDimension::WrapContent,
                    AndroidAppSceneOrientation::None,
                    true,
                    12,
                )
                .is_none()
            );
            assert!(
                AndroidAppSceneNodeDescriptor::new_weighted(
                    AndroidAppSceneNodeKind::TextView,
                    Some(1),
                    0x7f01_0002,
                    AndroidAppSceneDimension::Zero,
                    AndroidAppSceneDimension::WrapContent,
                    AndroidAppSceneOrientation::None,
                    false,
                    12,
                    1,
                )
                .is_none()
            );
            assert!(
                AndroidAppSceneNodeDescriptor::new_weighted(
                    AndroidAppSceneNodeKind::Button,
                    Some(1),
                    0x7f01_0002,
                    AndroidAppSceneDimension::Zero,
                    AndroidAppSceneDimension::WrapContent,
                    AndroidAppSceneOrientation::None,
                    true,
                    12,
                    9,
                )
                .is_none()
            );
        }

        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        {
            let padded_row = AndroidAppSceneNodeDescriptor::new_spaced(
                AndroidAppSceneNodeKind::LinearLayout,
                Some(0),
                0,
                AndroidAppSceneDimension::MatchParent,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::Horizontal,
                false,
                0,
                0,
                0,
                4,
            )
            .expect("uniformly padded row");
            let spaced_button = AndroidAppSceneNodeDescriptor::new_spaced(
                AndroidAppSceneNodeKind::Button,
                Some(1),
                0x7f01_0002,
                AndroidAppSceneDimension::Zero,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::None,
                true,
                12,
                1,
                2,
                0,
            )
            .expect("weighted Button with uniform margin");
            let padded_wire = padded_row.encode();
            let spaced_wire = spaced_button.encode();
            assert_eq!(padded_wire[0], 3);
            assert_eq!(padded_wire[13], 0);
            assert_eq!(padded_wire[14], 4);
            assert_eq!(padded_wire[15], 0);
            assert_eq!(spaced_wire[12], 1);
            assert_eq!(spaced_wire[13], 2);
            assert_eq!(spaced_wire[14], 0);
            assert_eq!(
                AndroidAppSceneNodeDescriptor::decode(&padded_wire),
                Some(padded_row)
            );
            assert_eq!(
                AndroidAppSceneNodeDescriptor::decode(&spaced_wire),
                Some(spaced_button)
            );
            assert_eq!(padded_row.padding_dp(), 4);
            assert_eq!(spaced_button.layout_margin_dp(), 2);
            assert!(
                AndroidAppSceneNodeDescriptor::new_spaced(
                    AndroidAppSceneNodeKind::TextView,
                    Some(0),
                    1,
                    AndroidAppSceneDimension::MatchParent,
                    AndroidAppSceneDimension::WrapContent,
                    AndroidAppSceneOrientation::None,
                    false,
                    1,
                    0,
                    1,
                    0,
                )
                .is_none()
            );
            assert!(
                AndroidAppSceneNodeDescriptor::new_spaced(
                    AndroidAppSceneNodeKind::Button,
                    Some(1),
                    1,
                    AndroidAppSceneDimension::Zero,
                    AndroidAppSceneDimension::WrapContent,
                    AndroidAppSceneOrientation::None,
                    true,
                    1,
                    1,
                    2,
                    1,
                )
                .is_none()
            );
        }

        #[cfg(all(
            feature = "androidbox-layout-directional17",
            not(feature = "androidbox-layout-size18")
        ))]
        {
            let padded_row = AndroidAppSceneNodeDescriptor::new_directional(
                AndroidAppSceneNodeKind::LinearLayout,
                Some(0),
                0,
                AndroidAppSceneDimension::MatchParent,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::Horizontal,
                false,
                0,
                0,
                0,
                0,
                0,
                0,
                6,
                4,
                2,
                8,
            )
            .expect("directionally padded row");
            let spaced_button = AndroidAppSceneNodeDescriptor::new_directional(
                AndroidAppSceneNodeKind::Button,
                Some(1),
                0x7f01_0002,
                AndroidAppSceneDimension::Zero,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::None,
                true,
                12,
                1,
                2,
                1,
                4,
                3,
                0,
                0,
                0,
                0,
            )
            .expect("weighted Button with directional margins");
            let padded_wire = padded_row.encode();
            let spaced_wire = spaced_button.encode();
            assert_eq!(padded_wire[0], 4);
            assert_eq!(&padded_wire[13..17], &[0, 0, 0, 0]);
            assert_eq!(&padded_wire[17..21], &[6, 4, 2, 8]);
            assert!(padded_wire[21..].iter().all(|byte| *byte == 0));
            assert_eq!(spaced_wire[12], 1);
            assert_eq!(&spaced_wire[13..17], &[2, 1, 4, 3]);
            assert_eq!(&spaced_wire[17..21], &[0, 0, 0, 0]);
            assert!(spaced_wire[21..].iter().all(|byte| *byte == 0));
            assert_eq!(
                AndroidAppSceneNodeDescriptor::decode(&padded_wire),
                Some(padded_row)
            );
            assert_eq!(
                AndroidAppSceneNodeDescriptor::decode(&spaced_wire),
                Some(spaced_button)
            );
            assert_eq!(padded_row.padding_left_dp(), 6);
            assert_eq!(padded_row.padding_top_dp(), 4);
            assert_eq!(padded_row.padding_right_dp(), 2);
            assert_eq!(padded_row.padding_bottom_dp(), 8);
            assert_eq!(spaced_button.layout_margin_left_dp(), 2);
            assert_eq!(spaced_button.layout_margin_top_dp(), 1);
            assert_eq!(spaced_button.layout_margin_right_dp(), 4);
            assert_eq!(spaced_button.layout_margin_bottom_dp(), 3);
            assert!(
                AndroidAppSceneNodeDescriptor::new_directional(
                    AndroidAppSceneNodeKind::LinearLayout,
                    Some(0),
                    0,
                    AndroidAppSceneDimension::MatchParent,
                    AndroidAppSceneDimension::WrapContent,
                    AndroidAppSceneOrientation::Horizontal,
                    false,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    17,
                    0,
                    0,
                    0,
                )
                .is_none()
            );
            assert!(
                AndroidAppSceneNodeDescriptor::new_directional(
                    AndroidAppSceneNodeKind::TextView,
                    Some(0),
                    1,
                    AndroidAppSceneDimension::MatchParent,
                    AndroidAppSceneDimension::WrapContent,
                    AndroidAppSceneOrientation::None,
                    false,
                    1,
                    0,
                    1,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                )
                .is_none()
            );
            assert!(
                AndroidAppSceneNodeDescriptor::new_directional(
                    AndroidAppSceneNodeKind::Button,
                    Some(1),
                    1,
                    AndroidAppSceneDimension::Zero,
                    AndroidAppSceneDimension::WrapContent,
                    AndroidAppSceneOrientation::None,
                    true,
                    1,
                    1,
                    0,
                    0,
                    0,
                    0,
                    1,
                    0,
                    0,
                    0,
                )
                .is_none()
            );
        }

        #[cfg(feature = "androidbox-layout-size18")]
        {
            let exact_title = AndroidAppSceneNodeDescriptor::new_sized(
                AndroidAppSceneNodeKind::TextView,
                Some(0),
                0x7f01_0002,
                AndroidAppSceneDimension::Exact,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::None,
                false,
                12,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                240,
                0,
            )
            .expect("240dp-wide title");
            let exact_button = AndroidAppSceneNodeDescriptor::new_sized(
                AndroidAppSceneNodeKind::Button,
                Some(1),
                0x7f01_0003,
                AndroidAppSceneDimension::Zero,
                AndroidAppSceneDimension::Exact,
                AndroidAppSceneOrientation::None,
                true,
                12,
                1,
                2,
                1,
                4,
                3,
                0,
                0,
                0,
                0,
                0,
                64,
            )
            .expect("weighted Button with exact 64dp height");
            let title_wire = exact_title.encode();
            let button_wire = exact_button.encode();
            assert_eq!(title_wire[0], 5);
            assert_eq!(title_wire[3], AndroidAppSceneDimension::Exact.raw());
            assert_eq!(title_wire[21], 240);
            assert_eq!(title_wire[22], 0);
            assert_eq!(title_wire[23], 0);
            assert_eq!(button_wire[4], AndroidAppSceneDimension::Exact.raw());
            assert_eq!(&button_wire[13..17], &[2, 1, 4, 3]);
            assert_eq!(button_wire[21], 0);
            assert_eq!(button_wire[22], 64);
            assert_eq!(button_wire[23], 0);
            assert_eq!(
                AndroidAppSceneNodeDescriptor::decode(&title_wire),
                Some(exact_title)
            );
            assert_eq!(
                AndroidAppSceneNodeDescriptor::decode(&button_wire),
                Some(exact_button)
            );
            assert_eq!(exact_title.exact_width_dp(), 240);
            assert_eq!(exact_button.exact_height_dp(), 64);

            let mut nonzero_reserved = button_wire;
            nonzero_reserved[23] = 1;
            assert!(AndroidAppSceneNodeDescriptor::decode(&nonzero_reserved).is_none());
            let mut missing_exact_width = title_wire;
            missing_exact_width[21] = 0;
            assert!(AndroidAppSceneNodeDescriptor::decode(&missing_exact_width).is_none());
            let mut unexpected_exact_width = button_wire;
            unexpected_exact_width[21] = 1;
            assert!(AndroidAppSceneNodeDescriptor::decode(&unexpected_exact_width).is_none());
            assert!(
                AndroidAppSceneNodeDescriptor::new_sized(
                    AndroidAppSceneNodeKind::TextView,
                    Some(0),
                    1,
                    AndroidAppSceneDimension::Exact,
                    AndroidAppSceneDimension::WrapContent,
                    AndroidAppSceneOrientation::None,
                    false,
                    1,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                )
                .is_none()
            );
        }

        let ready =
            AndroidAppMessage::new(AndroidAppMessageKind::Ready, 0, ABI_VERSION, 0, &[]).unwrap();
        let wire = ready.encode();
        #[cfg(feature = "androidbox-layout-size18")]
        {
            assert_eq!(&wire[..8], b"BNDAPC14");
            assert_eq!(wire[8], 14);
        }
        #[cfg(all(
            feature = "androidbox-layout-directional17",
            not(feature = "androidbox-layout-size18")
        ))]
        {
            assert_eq!(&wire[..8], b"BNDAPC13");
            assert_eq!(wire[8], 13);
        }
        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        {
            assert_eq!(&wire[..8], b"BNDAPC12");
            assert_eq!(wire[8], 12);
        }
        #[cfg(all(
            feature = "androidbox-layout-weight15",
            not(feature = "androidbox-layout-spacing16")
        ))]
        {
            assert_eq!(&wire[..8], b"BNDAPC11");
            assert_eq!(wire[8], 11);
        }
        #[cfg(all(
            feature = "androidbox-layout-row14",
            not(feature = "androidbox-layout-weight15")
        ))]
        {
            assert_eq!(&wire[..8], b"BNDAPC10");
            assert_eq!(wire[8], 10);
        }
        #[cfg(all(
            feature = "androidbox-string-builder13",
            not(feature = "androidbox-layout-row14")
        ))]
        {
            assert_eq!(&wire[..8], b"BNDAPC09");
            assert_eq!(wire[8], 9);
        }
        #[cfg(all(
            feature = "androidbox-string-text12",
            not(feature = "androidbox-string-builder13")
        ))]
        {
            assert_eq!(&wire[..8], b"BNDAPC08");
            assert_eq!(wire[8], 8);
        }
        #[cfg(all(
            feature = "androidbox-activity-state11",
            not(feature = "androidbox-string-text12")
        ))]
        {
            assert_eq!(&wire[..8], b"BNDAPC07");
            assert_eq!(wire[8], 7);
        }
        #[cfg(all(
            feature = "androidbox-activity-fields10",
            not(feature = "androidbox-activity-state11")
        ))]
        {
            assert_eq!(&wire[..8], b"BNDAPC06");
            assert_eq!(wire[8], 6);
        }
        #[cfg(all(
            feature = "androidbox-dex-instance9",
            not(feature = "androidbox-activity-fields10")
        ))]
        {
            assert_eq!(&wire[..8], b"BNDAPC05");
            assert_eq!(wire[8], 5);
        }
        #[cfg(all(
            feature = "androidbox-dex-methods8",
            not(feature = "androidbox-dex-instance9")
        ))]
        {
            assert_eq!(&wire[..8], b"BNDAPC04");
            assert_eq!(wire[8], 4);
        }
        #[cfg(all(
            feature = "androidbox-multiaction3",
            not(feature = "androidbox-dex-methods8")
        ))]
        {
            assert_eq!(&wire[..8], b"BNDAPC03");
            assert_eq!(wire[8], 3);
        }
        #[cfg(not(feature = "androidbox-multiaction3"))]
        {
            assert_eq!(&wire[..8], b"BNDAPC02");
            assert_eq!(wire[8], 2);
        }
        assert_eq!(AndroidAppMessage::decode(&wire), Ok(ready));

        let scene_opened =
            AndroidAppMessage::new(AndroidAppMessageKind::SceneOpened, 7, 3, 0, &[]).unwrap();
        assert_eq!(
            AndroidAppMessage::decode(&scene_opened.encode()),
            Ok(scene_opened)
        );
        let describe = AndroidAppMessage::new(
            AndroidAppMessageKind::DescribeNode,
            7,
            41,
            (3_u64 << 32) | 2,
            &[],
        )
        .unwrap();
        assert_eq!(describe.arg0(), 41);
        assert_eq!(describe.arg1() >> 32, 3);
        assert_eq!(describe.arg1() as u32, 2);
        assert_eq!(AndroidAppMessage::decode(&describe.encode()), Ok(describe));

        let text = [b'T'; ANDROID_APP_MESSAGE_PAYLOAD_BYTES];
        let chunk = AndroidAppMessage::new(
            AndroidAppMessageKind::NodeTextChunk,
            7,
            (72_u64 << 32) | 96,
            1,
            &text,
        )
        .unwrap();
        assert_eq!(chunk.chunk_offset(), 72);
        assert_eq!(chunk.chunk_total(), 96);
        assert_eq!(chunk.arg1(), 1);
        assert_eq!(AndroidAppMessage::decode(&chunk.encode()), Ok(chunk));

        for (raw, expected) in [
            (13, AndroidAppMessageKind::SceneOpened),
            (14, AndroidAppMessageKind::DescribeNode),
            (15, AndroidAppMessageKind::Node),
            (16, AndroidAppMessageKind::NodeTextChunk),
        ] {
            assert_eq!(AndroidAppMessageKind::from_raw(raw), Some(expected));
            assert_eq!(expected.raw(), raw);
        }
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    #[test]
    fn android_app_scene_rpc2_rejects_noncanonical_nodes_and_frames() {
        use super::{
            ABI_VERSION, ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE, AndroidAppMessage,
            AndroidAppMessageError, AndroidAppMessageKind, AndroidAppSceneDimension,
            AndroidAppSceneNodeDescriptor, AndroidAppSceneNodeKind, AndroidAppSceneOrientation,
        };

        let root = AndroidAppSceneNodeDescriptor::new(
            AndroidAppSceneNodeKind::LinearLayout,
            None,
            0,
            AndroidAppSceneDimension::MatchParent,
            AndroidAppSceneDimension::WrapContent,
            AndroidAppSceneOrientation::Vertical,
            false,
            0,
        )
        .unwrap();
        let child = AndroidAppSceneNodeDescriptor::new(
            AndroidAppSceneNodeKind::TextView,
            Some(0),
            1,
            AndroidAppSceneDimension::MatchParent,
            AndroidAppSceneDimension::WrapContent,
            AndroidAppSceneOrientation::None,
            false,
            1,
        )
        .unwrap();

        assert!(AndroidAppMessage::new(AndroidAppMessageKind::SceneOpened, 7, 0, 0, &[]).is_err());
        assert!(AndroidAppMessage::new(AndroidAppMessageKind::SceneOpened, 7, 9, 0, &[]).is_err());
        assert!(AndroidAppMessage::new(AndroidAppMessageKind::SceneOpened, 7, 1, 1, &[]).is_err());
        assert!(AndroidAppMessage::new(AndroidAppMessageKind::SceneOpened, 7, 1, 0, b"X").is_err());
        assert!(AndroidAppMessage::new(AndroidAppMessageKind::DescribeNode, 7, 0, 0, &[]).is_err());
        assert!(AndroidAppMessage::new(AndroidAppMessageKind::DescribeNode, 7, 1, 8, &[]).is_err());
        assert!(
            AndroidAppMessage::new(AndroidAppMessageKind::DescribeNode, 7, 1, 0, b"X").is_err()
        );

        assert!(
            AndroidAppMessage::new(AndroidAppMessageKind::Node, 7, 8, 0, &root.encode(),).is_err()
        );
        assert!(
            AndroidAppMessage::new(
                AndroidAppMessageKind::Node,
                7,
                0,
                u64::from(u32::MAX) + 1,
                &root.encode(),
            )
            .is_err()
        );
        assert!(
            AndroidAppMessage::new(AndroidAppMessageKind::Node, 7, 1, 0, &root.encode(),).is_err()
        );
        assert!(
            AndroidAppMessage::new(AndroidAppMessageKind::Node, 7, 0, 0, &child.encode(),).is_err()
        );
        assert!(
            AndroidAppMessage::new(
                AndroidAppMessageKind::Node,
                7,
                1,
                0,
                &child.encode()[..ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE - 1],
            )
            .is_err()
        );

        for (arg0, arg1, payload) in [
            (0, 0, b"X".as_slice()),
            (97, 0, b"X".as_slice()),
            (1, 8, b"X".as_slice()),
            (1, 0, b"\n".as_slice()),
            ((1_u64 << 32) | 1, 0, b"XX".as_slice()),
        ] {
            assert!(
                AndroidAppMessage::new(
                    AndroidAppMessageKind::NodeTextChunk,
                    7,
                    arg0,
                    arg1,
                    payload,
                )
                .is_err()
            );
        }

        let invalid_descriptors = [
            AndroidAppSceneNodeDescriptor::new(
                AndroidAppSceneNodeKind::LinearLayout,
                None,
                0,
                AndroidAppSceneDimension::MatchParent,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::None,
                false,
                0,
            ),
            AndroidAppSceneNodeDescriptor::new(
                AndroidAppSceneNodeKind::LinearLayout,
                None,
                0,
                AndroidAppSceneDimension::MatchParent,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::Vertical,
                true,
                0,
            ),
            AndroidAppSceneNodeDescriptor::new(
                AndroidAppSceneNodeKind::TextView,
                Some(0),
                0,
                AndroidAppSceneDimension::MatchParent,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::None,
                false,
                1,
            ),
            AndroidAppSceneNodeDescriptor::new(
                AndroidAppSceneNodeKind::TextView,
                Some(0),
                1,
                AndroidAppSceneDimension::MatchParent,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::None,
                true,
                1,
            ),
            AndroidAppSceneNodeDescriptor::new(
                AndroidAppSceneNodeKind::TextView,
                Some(0),
                1,
                AndroidAppSceneDimension::MatchParent,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::None,
                false,
                0,
            ),
            AndroidAppSceneNodeDescriptor::new(
                AndroidAppSceneNodeKind::TextView,
                Some(0),
                1,
                AndroidAppSceneDimension::MatchParent,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::None,
                false,
                97,
            ),
            AndroidAppSceneNodeDescriptor::new(
                AndroidAppSceneNodeKind::Button,
                Some(0),
                0,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::None,
                true,
                1,
            ),
            AndroidAppSceneNodeDescriptor::new(
                AndroidAppSceneNodeKind::Button,
                Some(0),
                1,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::Vertical,
                true,
                1,
            ),
            AndroidAppSceneNodeDescriptor::new(
                AndroidAppSceneNodeKind::Button,
                Some(0),
                1,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::None,
                true,
                65,
            ),
        ];
        assert!(invalid_descriptors.iter().all(Option::is_none));

        let canonical = root.encode();
        #[cfg(feature = "androidbox-layout-weight15")]
        let invalid_version = 1;
        #[cfg(not(feature = "androidbox-layout-weight15"))]
        let invalid_version = 2;
        for (offset, value) in [
            (0, invalid_version),
            (1, 0),
            (2, 8),
            (3, 0),
            (4, 3),
            (6, 2),
            (12, 1),
        ] {
            let mut malformed = canonical;
            malformed[offset] = value;
            assert_eq!(AndroidAppSceneNodeDescriptor::decode(&malformed), None);
        }
        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        for (offset, value) in [(13, 1), (14, 17), (15, 1)] {
            let mut malformed = canonical;
            malformed[offset] = value;
            assert_eq!(AndroidAppSceneNodeDescriptor::decode(&malformed), None);
        }
        #[cfg(feature = "androidbox-layout-directional17")]
        for (offset, value) in [(13, 1), (17, 17), (21, 1)] {
            let mut malformed = canonical;
            malformed[offset] = value;
            assert_eq!(AndroidAppSceneNodeDescriptor::decode(&malformed), None);
        }
        let mut horizontal_root = canonical;
        horizontal_root[5] = 2;
        #[cfg(not(feature = "androidbox-layout-row14"))]
        assert_eq!(
            AndroidAppSceneNodeDescriptor::decode(&horizontal_root),
            None
        );
        #[cfg(feature = "androidbox-layout-row14")]
        {
            let horizontal =
                AndroidAppSceneNodeDescriptor::decode(&horizontal_root).expect("known orientation");
            assert!(!horizontal.is_canonical_for_index(0));
            let nested = AndroidAppSceneNodeDescriptor::new(
                AndroidAppSceneNodeKind::LinearLayout,
                Some(0),
                0,
                AndroidAppSceneDimension::MatchParent,
                AndroidAppSceneDimension::WrapContent,
                AndroidAppSceneOrientation::Horizontal,
                false,
                0,
            )
            .expect("bounded horizontal row");
            assert!(nested.is_canonical_for_index(1));
            assert_eq!(
                AndroidAppSceneNodeDescriptor::decode(&nested.encode()),
                Some(nested)
            );
        }

        let current_wire =
            AndroidAppMessage::new(AndroidAppMessageKind::Ready, 0, ABI_VERSION, 0, &[])
                .unwrap()
                .encode();
        let mut old_magic = current_wire;
        old_magic[..8].copy_from_slice(b"BNDAPC01");
        assert_eq!(
            AndroidAppMessage::decode(&old_magic),
            Err(AndroidAppMessageError::InvalidMagic)
        );
        let mut old_version = current_wire;
        old_version[8] = 1;
        assert_eq!(
            AndroidAppMessage::decode(&old_version),
            Err(AndroidAppMessageError::InvalidVersion)
        );
    }

    #[cfg(feature = "androidbox-restart0")]
    #[test]
    fn android_app_restart_messages_bind_one_faulted_generation_transition() {
        use super::{
            ABI_VERSION, ANDROID_APP_SUPERVISOR_WIRE_SIZE, AndroidAppMessage,
            AndroidAppMessageKind, AndroidAppSupervisorMessage, AndroidAppSupervisorMessageKind,
            ProcessTerminationReason,
        };

        #[cfg(feature = "androidbox-layout-mixed19")]
        assert_eq!(ABI_VERSION, 69);
        #[cfg(all(
            feature = "androidbox-layout-size18",
            not(feature = "androidbox-layout-mixed19")
        ))]
        assert_eq!(ABI_VERSION, 68);
        #[cfg(all(
            feature = "androidbox-layout-directional17",
            not(feature = "androidbox-layout-size18")
        ))]
        assert_eq!(ABI_VERSION, 67);
        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        assert_eq!(ABI_VERSION, 66);
        #[cfg(all(
            feature = "androidbox-layout-weight15",
            not(feature = "androidbox-layout-spacing16")
        ))]
        assert_eq!(ABI_VERSION, 65);
        #[cfg(all(
            feature = "androidbox-layout-row14",
            not(feature = "androidbox-layout-weight15")
        ))]
        assert_eq!(ABI_VERSION, 64);
        #[cfg(all(
            feature = "androidbox-string-builder13",
            not(feature = "androidbox-layout-row14")
        ))]
        assert_eq!(ABI_VERSION, 63);
        #[cfg(all(
            feature = "androidbox-string-text12",
            not(feature = "androidbox-string-builder13")
        ))]
        assert_eq!(ABI_VERSION, 62);
        #[cfg(all(
            feature = "androidbox-activity-state11",
            not(feature = "androidbox-string-text12")
        ))]
        assert_eq!(ABI_VERSION, 61);
        #[cfg(all(
            feature = "androidbox-activity-fields10",
            not(feature = "androidbox-activity-state11")
        ))]
        assert_eq!(ABI_VERSION, 60);
        #[cfg(all(
            feature = "androidbox-dex-instance9",
            not(feature = "androidbox-activity-fields10")
        ))]
        assert_eq!(ABI_VERSION, 59);
        #[cfg(all(
            feature = "androidbox-dex-methods8",
            not(feature = "androidbox-dex-instance9")
        ))]
        assert_eq!(ABI_VERSION, 58);
        #[cfg(all(
            feature = "androidbox-density-icons7",
            not(feature = "androidbox-dex-methods8")
        ))]
        assert_eq!(ABI_VERSION, 57);
        #[cfg(all(
            feature = "androidbox-icon-resources5",
            not(feature = "androidbox-density-icons7")
        ))]
        assert_eq!(ABI_VERSION, 56);
        #[cfg(all(
            feature = "androidbox-multipackage4",
            not(feature = "androidbox-icon-resources5")
        ))]
        assert_eq!(ABI_VERSION, 55);
        #[cfg(all(
            feature = "androidbox-manifest-catalog3",
            not(feature = "androidbox-multipackage4")
        ))]
        assert_eq!(ABI_VERSION, 54);
        #[cfg(all(
            feature = "androidbox-runtime-install2",
            not(feature = "androidbox-manifest-catalog3")
        ))]
        assert_eq!(ABI_VERSION, 53);
        #[cfg(all(
            feature = "androidbox-runtime-uninstall1",
            not(feature = "androidbox-runtime-install2")
        ))]
        assert_eq!(ABI_VERSION, 52);
        #[cfg(all(
            feature = "androidbox-apk-envelope4",
            not(feature = "androidbox-runtime-uninstall1")
        ))]
        assert_eq!(ABI_VERSION, 51);
        #[cfg(all(
            feature = "androidbox-multiaction3",
            not(feature = "androidbox-apk-envelope4")
        ))]
        assert_eq!(ABI_VERSION, 50);
        #[cfg(all(
            feature = "androidbox-scene-rpc2",
            not(feature = "androidbox-multiaction3")
        ))]
        assert_eq!(ABI_VERSION, 49);
        #[cfg(not(feature = "androidbox-scene-rpc2"))]
        assert_eq!(ABI_VERSION, 48);
        let crash = AndroidAppMessage::new(AndroidAppMessageKind::Crash, 2, 41, 3, &[]).unwrap();
        assert_eq!(AndroidAppMessage::decode(&crash.encode()), Ok(crash));
        assert!(AndroidAppMessage::new(AndroidAppMessageKind::Crash, 0, 41, 3, &[]).is_err());
        assert!(AndroidAppMessage::new(AndroidAppMessageKind::Crash, 2, 0, 3, &[]).is_err());

        let rebind = AndroidAppSupervisorMessage::new(
            AndroidAppSupervisorMessageKind::Rebind,
            1,
            0x0000_0001_0000_0008,
            0x0000_0002_0000_0008,
            0,
            ProcessTerminationReason::Faulted.raw(),
        )
        .unwrap();
        assert_eq!(ANDROID_APP_SUPERVISOR_WIRE_SIZE, 64);
        assert_eq!(
            AndroidAppSupervisorMessage::decode(&rebind.encode()),
            Some(rebind)
        );
        let rebound = AndroidAppSupervisorMessage::new(
            AndroidAppSupervisorMessageKind::Rebound,
            rebind.epoch(),
            rebind.old_pid(),
            rebind.new_pid(),
            rebind.exit_code(),
            rebind.termination_reason(),
        )
        .unwrap();
        assert_eq!(
            AndroidAppSupervisorMessage::decode(&rebound.encode()),
            Some(rebound)
        );
        assert!(
            AndroidAppSupervisorMessage::new(
                AndroidAppSupervisorMessageKind::Rebind,
                2,
                rebind.old_pid(),
                rebind.new_pid(),
                0,
                ProcessTerminationReason::Faulted.raw(),
            )
            .is_none()
        );
        let mut noncanonical = rebind.encode();
        noncanonical[63] = 1;
        assert!(AndroidAppSupervisorMessage::decode(&noncanonical).is_none());
    }

    #[test]
    fn user_image_ids_are_stable_bounded_and_explicitly_spawnable() {
        let stable = [
            UserImageId::Init,
            UserImageId::ServiceManager,
            UserImageId::Provider,
            UserImageId::Client,
            UserImageId::SurfaceServer,
            UserImageId::Launcher,
            UserImageId::App,
            UserImageId::InputServer,
            UserImageId::StorageServer,
        ];
        for (offset, expected) in stable.into_iter().enumerate() {
            let raw = offset as u64 + 1;
            assert_eq!(expected.raw(), raw);
            assert_eq!(UserImageId::from_raw(raw), Some(expected));
        }

        assert_eq!(UserImageId::from_raw(0), None);
        #[cfg(feature = "androidbox-process0")]
        {
            assert_eq!(UserImageId::AndroidApp.raw(), 10);
            assert_eq!(UserImageId::from_raw(10), Some(UserImageId::AndroidApp));
            assert!(UserImageId::AndroidApp.is_spawnable());
            assert_eq!(UserImageId::from_raw(11), None);
        }
        #[cfg(not(feature = "androidbox-process0"))]
        assert_eq!(UserImageId::from_raw(10), None);
        assert_eq!(UserImageId::from_raw(u64::MAX), None);
        assert!(!UserImageId::Init.is_spawnable());
        assert!(UserImageId::ServiceManager.is_spawnable());
        assert!(UserImageId::Provider.is_spawnable());
        assert!(UserImageId::Client.is_spawnable());
        assert!(UserImageId::SurfaceServer.is_spawnable());
        assert!(UserImageId::Launcher.is_spawnable());
        assert!(UserImageId::App.is_spawnable());
        assert!(UserImageId::InputServer.is_spawnable());
        assert!(UserImageId::StorageServer.is_spawnable());
        assert_eq!(PROCESS_SPAWN_FLAGS_NONE, 0);
    }

    #[test]
    fn status_values_are_register_abi_values() {
        let stable = [
            Status::Ok,
            Status::InvalidArgument,
            Status::PermissionDenied,
            Status::NotFound,
            Status::AlreadyExists,
            Status::Unavailable,
            Status::Timeout,
            Status::OutOfMemory,
            Status::InvalidState,
            Status::Unsupported,
            Status::ShouldWait,
            Status::PeerClosed,
            Status::BadAddress,
            Status::BufferTooSmall,
            Status::Conflict,
            Status::OutcomeUnknown,
            Status::DataCorrupt,
            Status::NoSpace,
            Status::RequiresReset,
        ];
        for (raw, status) in stable.into_iter().enumerate() {
            assert_eq!(status.raw(), raw as u64);
        }
    }

    #[test]
    fn rights_only_reduce_and_reject_unknown_bits() {
        let reduced = Rights::from_bits(Rights::READ.bits() | Rights::WRITE.bits()).unwrap();
        assert!(reduced.is_subset_of(Rights::CHANNEL_DEFAULT));
        assert!(Rights::TRANSFER.is_subset_of(Rights::CHANNEL_DEFAULT));
        assert!(Rights::WAIT.is_subset_of(Rights::CHANNEL_DEFAULT));
        #[cfg(feature = "androidbox-process0")]
        {
            assert_eq!(Rights::ANDROID_APP_CHANNEL.bits(), 0x103);
            assert!(Rights::ANDROID_APP_CHANNEL.is_subset_of(Rights::CHANNEL_DEFAULT));
            assert!(Rights::READ.is_subset_of(Rights::ANDROID_APP_CHANNEL));
            assert!(Rights::WRITE.is_subset_of(Rights::ANDROID_APP_CHANNEL));
            assert!(Rights::WAIT.is_subset_of(Rights::ANDROID_APP_CHANNEL));
            assert!(!Rights::DUPLICATE.is_subset_of(Rights::ANDROID_APP_CHANNEL));
            assert!(!Rights::TRANSFER.is_subset_of(Rights::ANDROID_APP_CHANNEL));
        }
        assert!(Rights::SIGNAL.is_subset_of(Rights::EVENT_DEFAULT));
        assert!(!Rights::READ.is_subset_of(Rights::EVENT_DEFAULT));
        assert!(Rights::READ.is_subset_of(Rights::VMO_DEFAULT));
        assert!(Rights::DUPLICATE.is_subset_of(Rights::VMO_DEFAULT));
        assert!(Rights::TRANSFER.is_subset_of(Rights::VMO_DEFAULT));
        assert!(!Rights::WRITE.is_subset_of(Rights::VMO_DEFAULT));
        assert!(!Rights::MAP.is_subset_of(Rights::VMO_DEFAULT));
        assert!(!Rights::WAIT.is_subset_of(Rights::VMO_DEFAULT));
        assert!(Rights::READ.is_subset_of(Rights::DIRECTORY_DEFAULT));
        assert!(Rights::TRANSFER.is_subset_of(Rights::DIRECTORY_DEFAULT));
        assert!(!Rights::DUPLICATE.is_subset_of(Rights::DIRECTORY_DEFAULT));
        assert_eq!(Rights::APP_DATA_ROOT_DEFAULT.bits(), 0x07);
        assert!(Rights::READ.is_subset_of(Rights::APP_DATA_ROOT_DEFAULT));
        assert!(Rights::WRITE.is_subset_of(Rights::APP_DATA_ROOT_DEFAULT));
        assert!(Rights::DUPLICATE.is_subset_of(Rights::APP_DATA_ROOT_DEFAULT));
        assert!(!Rights::TRANSFER.is_subset_of(Rights::APP_DATA_ROOT_DEFAULT));
        assert!(!Rights::WAIT.is_subset_of(Rights::APP_DATA_ROOT_DEFAULT));
        assert_eq!(Rights::STORAGE_VOLUME_DEFAULT.bits(), 0x103);
        assert!(Rights::READ.is_subset_of(Rights::STORAGE_VOLUME_DEFAULT));
        assert!(Rights::WRITE.is_subset_of(Rights::STORAGE_VOLUME_DEFAULT));
        assert!(Rights::WAIT.is_subset_of(Rights::STORAGE_VOLUME_DEFAULT));
        assert!(!Rights::DUPLICATE.is_subset_of(Rights::STORAGE_VOLUME_DEFAULT));
        assert!(!Rights::TRANSFER.is_subset_of(Rights::STORAGE_VOLUME_DEFAULT));
        assert_eq!(Rights::STORAGE_SESSION_DEFAULT.bits(), 0x103);
        assert!(Rights::READ.is_subset_of(Rights::STORAGE_SESSION_DEFAULT));
        assert!(Rights::WRITE.is_subset_of(Rights::STORAGE_SESSION_DEFAULT));
        assert!(Rights::WAIT.is_subset_of(Rights::STORAGE_SESSION_DEFAULT));
        assert!(!Rights::DUPLICATE.is_subset_of(Rights::STORAGE_SESSION_DEFAULT));
        assert!(!Rights::TRANSFER.is_subset_of(Rights::STORAGE_SESSION_DEFAULT));
        assert!(Rights::READ.is_subset_of(Rights::SURFACE_DEFAULT));
        assert!(Rights::WRITE.is_subset_of(Rights::SURFACE_DEFAULT));
        assert!(Rights::WAIT.is_subset_of(Rights::SURFACE_DEFAULT));
        assert!(!Rights::DUPLICATE.is_subset_of(Rights::SURFACE_DEFAULT));
        assert!(!Rights::TRANSFER.is_subset_of(Rights::SURFACE_DEFAULT));
        assert_eq!(Rights::INPUT_DEFAULT.bits(), 0x101);
        assert!(Rights::READ.is_subset_of(Rights::INPUT_DEFAULT));
        assert!(Rights::WAIT.is_subset_of(Rights::INPUT_DEFAULT));
        assert!(!Rights::WRITE.is_subset_of(Rights::INPUT_DEFAULT));
        assert!(!Rights::DUPLICATE.is_subset_of(Rights::INPUT_DEFAULT));
        assert!(!Rights::TRANSFER.is_subset_of(Rights::INPUT_DEFAULT));
        assert_eq!(Rights::GRAPHICS_BUFFER_DEFAULT.bits(), 0x0f);
        assert!(Rights::READ.is_subset_of(Rights::GRAPHICS_BUFFER_DEFAULT));
        assert!(Rights::WRITE.is_subset_of(Rights::GRAPHICS_BUFFER_DEFAULT));
        assert!(Rights::DUPLICATE.is_subset_of(Rights::GRAPHICS_BUFFER_DEFAULT));
        assert!(Rights::TRANSFER.is_subset_of(Rights::GRAPHICS_BUFFER_DEFAULT));
        assert!(!Rights::MAP.is_subset_of(Rights::GRAPHICS_BUFFER_DEFAULT));
        assert!(!Rights::WAIT.is_subset_of(Rights::GRAPHICS_BUFFER_DEFAULT));
        assert!(!Rights::EXECUTE.is_subset_of(Rights::GRAPHICS_BUFFER_DEFAULT));
        assert!(!Rights::SIGNAL.is_subset_of(Rights::GRAPHICS_BUFFER_DEFAULT));
        assert_eq!(Rights::GRAPHICS_BUFFER_SERVER.bits(), 0x09);
        assert!(Rights::GRAPHICS_BUFFER_SERVER.is_subset_of(Rights::GRAPHICS_BUFFER_DEFAULT));
        assert!(Rights::READ.is_subset_of(Rights::GRAPHICS_BUFFER_SERVER));
        assert!(Rights::TRANSFER.is_subset_of(Rights::GRAPHICS_BUFFER_SERVER));
        assert!(!Rights::WRITE.is_subset_of(Rights::GRAPHICS_BUFFER_SERVER));
        assert!(!Rights::DUPLICATE.is_subset_of(Rights::GRAPHICS_BUFFER_SERVER));
        assert!(!Rights::MAP.is_subset_of(Rights::GRAPHICS_BUFFER_SERVER));
        assert!(!Rights::WAIT.is_subset_of(Rights::GRAPHICS_BUFFER_SERVER));
        assert!(!Rights::EXECUTE.is_subset_of(Rights::GRAPHICS_BUFFER_SERVER));
        assert!(!Rights::SIGNAL.is_subset_of(Rights::GRAPHICS_BUFFER_SERVER));
        assert_eq!(Rights::GRAPHICS_BUFFER_MAPPED_PRODUCER.bits(), 0x12f);
        assert!(
            Rights::GRAPHICS_BUFFER_DEFAULT.is_subset_of(Rights::GRAPHICS_BUFFER_MAPPED_PRODUCER)
        );
        assert!(Rights::MAP.is_subset_of(Rights::GRAPHICS_BUFFER_MAPPED_PRODUCER));
        assert!(Rights::WAIT.is_subset_of(Rights::GRAPHICS_BUFFER_MAPPED_PRODUCER));
        assert_eq!(Rights::GRAPHICS_BUFFER_MAPPED_SERVER.bits(), 0x129);
        assert!(Rights::GRAPHICS_BUFFER_SERVER.is_subset_of(Rights::GRAPHICS_BUFFER_MAPPED_SERVER));
        assert!(Rights::MAP.is_subset_of(Rights::GRAPHICS_BUFFER_MAPPED_SERVER));
        assert!(Rights::WAIT.is_subset_of(Rights::GRAPHICS_BUFFER_MAPPED_SERVER));
        assert!(!Rights::WRITE.is_subset_of(Rights::GRAPHICS_BUFFER_MAPPED_SERVER));
        assert!(!Rights::ADMIN.is_subset_of(Rights::CHANNEL_DEFAULT));
        assert_eq!(Rights::from_bits(1 << 31), None);
    }

    #[test]
    fn object_signals_accept_none_and_reject_unknown_bits() {
        assert_eq!(ObjectSignals::from_bits(0), Some(ObjectSignals::NONE));
        assert_eq!(
            ObjectSignals::from_bits(
                ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits()
            ),
            Some(ObjectSignals::from_bits(0b101).unwrap())
        );
        assert!(ObjectSignals::CHANNEL_ALL.contains(ObjectSignals::WRITABLE));
        assert!(ObjectSignals::CHANNEL_ALL.intersects(ObjectSignals::PEER_CLOSED));
        assert!(ObjectSignals::SURFACE_ALL.contains(ObjectSignals::READABLE));
        assert!(ObjectSignals::SURFACE_ALL.contains(ObjectSignals::PEER_CLOSED));
        assert!(ObjectSignals::SURFACE_ALL.contains(ObjectSignals::FRAME_READY));
        assert!(ObjectSignals::SURFACE_ALL.contains(ObjectSignals::KEY_READY));
        assert!(!ObjectSignals::SURFACE_ALL.contains(ObjectSignals::WRITABLE));
        assert!(ObjectSignals::ALL.contains(ObjectSignals::SIGNALED));
        assert!(ObjectSignals::ALL.contains(ObjectSignals::FRAME_READY));
        assert!(ObjectSignals::ALL.contains(ObjectSignals::KEY_READY));
        assert!(!ObjectSignals::READABLE.intersects(ObjectSignals::WRITABLE));
        assert!(!ObjectSignals::READABLE.intersects(ObjectSignals::KEY_READY));
        assert_eq!(ObjectSignals::from_bits(1 << 6), None);
        assert_eq!(ObjectSignals::from_bits(1 << 31), None);
    }

    #[test]
    fn zero_is_the_only_invalid_handle_value() {
        assert!(!HandleValue::INVALID.is_valid());
        assert!(HandleValue::from_raw(1).is_valid());
        assert_eq!(HandleValue::from_raw(0x1234_5678).raw(), 0x1234_5678);
    }

    #[test]
    fn key_input_sample_register_codec_is_canonical_and_strict() {
        for value in [
            KeyInputSample::VALUE_RELEASE,
            KeyInputSample::VALUE_PRESS,
            KeyInputSample::VALUE_REPEAT,
        ] {
            let sample = KeyInputSample::try_new(0x0102_0304_0506_0708, 30, value).unwrap();
            let registers = sample.encode_registers();
            assert_eq!(registers.0, u64::from(30_u16) | (u64::from(value) << 16));
            assert_eq!(registers.1, 0x0102_0304_0506_0708);
            assert_eq!(
                KeyInputSample::decode_registers(registers.0, registers.1),
                Ok(sample)
            );
            assert_eq!(sample.code(), 30);
            assert_eq!(sample.value(), value);
            assert_eq!(sample.sequence(), 0x0102_0304_0506_0708);
        }
        assert_eq!(
            KeyInputSample::try_new(1, 0, KeyInputSample::VALUE_PRESS),
            Err(KeyInputSampleError::InvalidCode)
        );
        assert_eq!(
            KeyInputSample::try_new(1, 30, 3),
            Err(KeyInputSampleError::InvalidValue)
        );
        assert_eq!(
            KeyInputSample::try_new(0, 30, KeyInputSample::VALUE_PRESS),
            Err(KeyInputSampleError::InvalidSequence)
        );
        assert_eq!(
            KeyInputSample::decode_registers(1_u64 << 18, 1),
            Err(KeyInputSampleError::NonCanonicalState)
        );
        assert_eq!(
            KeyInputSample::decode_registers(3_u64 << 16 | 30, 1),
            Err(KeyInputSampleError::InvalidValue)
        );
        assert_eq!(
            KeyInputSample::decode_registers(30, 0),
            Err(KeyInputSampleError::InvalidSequence)
        );
    }

    #[test]
    fn global_input_event_kinds_are_stable_and_strict() {
        assert_eq!(InputEventKind::Pointer.raw(), 1);
        assert_eq!(InputEventKind::Key.raw(), 2);
        assert_eq!(InputEventKind::from_raw(1), Some(InputEventKind::Pointer));
        assert_eq!(InputEventKind::from_raw(2), Some(InputEventKind::Key));
        assert_eq!(InputEventKind::from_raw(0), None);
        assert_eq!(InputEventKind::from_raw(3), None);
        assert_eq!(InputEventKind::from_raw(u8::MAX), None);
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!((INPUT_GLOBAL_WIDTH, INPUT_GLOBAL_HEIGHT), (320, 480));
        #[cfg(feature = "mobile-ui-runtime")]
        assert_eq!((INPUT_GLOBAL_WIDTH, INPUT_GLOBAL_HEIGHT), (720, 1_600));
    }

    #[test]
    fn global_pointer_event_codec_uses_global_coordinates_and_one_sequence() {
        let released = InputEvent::try_pointer(1, 7, 0, 0, false).unwrap();
        assert_eq!(released.kind(), InputEventKind::Pointer);
        assert_eq!(released.device_id(), 7);
        assert_eq!(released.pointer(), Some((0, 0, false)));
        assert_eq!(released.key(), None);
        assert_eq!(released.sequence(), 1);
        assert_eq!(
            released.payload(),
            InputEventPayload::Pointer {
                x: 0,
                y: 0,
                pressed: false
            }
        );
        assert_eq!(released.encode_registers(), (0x0000_0000_0000_0107, 1));
        assert_eq!(
            InputEvent::decode_registers(0x0000_0000_0000_0107, 1),
            Ok(released)
        );

        let pressed = InputEvent::try_pointer(
            0x0102_0304_0506_0708,
            u8::MAX,
            INPUT_GLOBAL_WIDTH - 1,
            INPUT_GLOBAL_HEIGHT - 1,
            true,
        )
        .unwrap();
        let registers = pressed.encode_registers();
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!(registers.0, 0x0001_01df_013f_01ff);
        #[cfg(feature = "mobile-ui-runtime")]
        assert_eq!(registers.0, 0x0001_063f_02cf_01ff);
        assert_eq!(registers.1, 0x0102_0304_0506_0708);
        assert_eq!(
            InputEvent::decode_registers(registers.0, registers.1),
            Ok(pressed)
        );
        assert_eq!(
            pressed.pointer(),
            Some((INPUT_GLOBAL_WIDTH - 1, INPUT_GLOBAL_HEIGHT - 1, true))
        );
    }

    #[test]
    fn global_key_event_codec_accepts_only_canonical_linux_key_transitions() {
        for value in [
            InputEvent::KEY_VALUE_RELEASE,
            InputEvent::KEY_VALUE_PRESS,
            InputEvent::KEY_VALUE_REPEAT,
        ] {
            let event = InputEvent::try_key(99, 2, 30, value).unwrap();
            let registers = event.encode_registers();
            assert_eq!(
                registers.0,
                2 | (2 << 8) | (30 << 16) | (u64::from(value) << 32)
            );
            assert_eq!(registers.1, 99);
            assert_eq!(
                InputEvent::decode_registers(registers.0, registers.1),
                Ok(event)
            );
            assert_eq!(event.kind(), InputEventKind::Key);
            assert_eq!(event.device_id(), 2);
            assert_eq!(event.pointer(), None);
            assert_eq!(event.key(), Some((30, value)));
            assert_eq!(event.payload(), InputEventPayload::Key { code: 30, value });
        }
    }

    #[test]
    fn global_input_event_constructors_reject_invalid_boundaries() {
        assert_eq!(
            InputEvent::try_pointer(0, 1, 0, 0, false),
            Err(InputEventError::InvalidSequence)
        );
        assert_eq!(
            InputEvent::try_key(0, 1, 30, InputEvent::KEY_VALUE_PRESS),
            Err(InputEventError::InvalidSequence)
        );
        assert_eq!(
            InputEvent::try_pointer(1, 0, 0, 0, false),
            Err(InputEventError::InvalidDeviceId)
        );
        assert_eq!(
            InputEvent::try_key(1, 0, 30, InputEvent::KEY_VALUE_PRESS),
            Err(InputEventError::InvalidDeviceId)
        );
        assert_eq!(
            InputEvent::try_pointer(1, 1, INPUT_GLOBAL_WIDTH, 0, false),
            Err(InputEventError::CoordinateOutOfBounds)
        );
        assert_eq!(
            InputEvent::try_pointer(1, 1, 0, INPUT_GLOBAL_HEIGHT, false),
            Err(InputEventError::CoordinateOutOfBounds)
        );
        assert_eq!(
            InputEvent::try_key(1, 1, 0, InputEvent::KEY_VALUE_PRESS),
            Err(InputEventError::InvalidKeyCode)
        );
        assert_eq!(
            InputEvent::try_key(1, 1, 30, 3),
            Err(InputEventError::InvalidKeyValue)
        );
    }

    #[test]
    fn global_input_event_decoder_rejects_every_noncanonical_field_class() {
        assert_eq!(
            InputEvent::decode_registers(0, 1),
            Err(InputEventError::InvalidKind)
        );
        assert_eq!(
            InputEvent::decode_registers(3 << 8 | 1, 1),
            Err(InputEventError::InvalidKind)
        );
        assert_eq!(
            InputEvent::decode_registers(1 << 8, 1),
            Err(InputEventError::InvalidDeviceId)
        );
        assert_eq!(
            InputEvent::decode_registers(1 << 8 | 1, 0),
            Err(InputEventError::InvalidSequence)
        );
        assert_eq!(
            InputEvent::decode_registers(1 << 49 | 1 << 8 | 1, 1),
            Err(InputEventError::NonCanonicalState)
        );
        assert_eq!(
            InputEvent::decode_registers(1 << 48 | 2 << 8 | 1, 1),
            Err(InputEventError::NonCanonicalState)
        );
        assert_eq!(
            InputEvent::decode_registers(1 << 34 | 2 << 8 | 1, 1),
            Err(InputEventError::NonCanonicalState)
        );
        assert_eq!(
            InputEvent::decode_registers(3 << 32 | 30 << 16 | 2 << 8 | 1, 1),
            Err(InputEventError::InvalidKeyValue)
        );
        assert_eq!(
            InputEvent::decode_registers(u64::from(INPUT_GLOBAL_WIDTH) << 16 | 1 << 8 | 1, 1),
            Err(InputEventError::CoordinateOutOfBounds)
        );
        assert_eq!(
            InputEvent::decode_registers(u64::from(INPUT_GLOBAL_HEIGHT) << 32 | 1 << 8 | 1, 1),
            Err(InputEventError::CoordinateOutOfBounds)
        );
    }

    #[test]
    fn transfer_argument_preserves_all_handle_and_length_bits() {
        let handle = HandleValue::from_raw(0xfedc_ba98);
        let packed = pack_transfer(handle, 64);
        assert_eq!(unpack_transfer(packed), (handle, 64));
        assert_eq!(packed, 0xfedc_ba98_0000_0040);
    }

    #[test]
    fn vmo_read_argument_and_system_file_bounds_are_stable() {
        let packed = pack_vmo_read(0xfedc_ba98, 0x7654_3210);
        assert_eq!(packed, 0xfedc_ba98_7654_3210);
        assert_eq!(unpack_vmo_read(packed), (0xfedc_ba98, 0x7654_3210));
        assert_eq!(SYSTEM_FILE_PATH_MAX_BYTES, 64);
        assert_eq!(VMO_READ_MAX_BYTES, 4096);
    }

    #[test]
    fn graphics_buffer_geometry_format_backing_and_write_bounds_are_stable() {
        assert_eq!(GRAPHICS_BUFFER_FORMAT_XRGB8888, 1);
        assert_eq!(GRAPHICS_BUFFER_CREATE_FLAGS_NONE, 0);
        assert_eq!(GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE, 1);
        assert_eq!(GRAPHICS_BUFFER_MAP_ADDRESS, 0x0000_0002_0010_0000);
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!(GRAPHICS_BUFFER_MAP_STRIDE, 0x0005_0000);
        #[cfg(feature = "mobile-ui-runtime")]
        assert_eq!(GRAPHICS_BUFFER_MAP_STRIDE, 0x0048_0000);
        assert_eq!(GRAPHICS_BUFFER_MAP_PRODUCER_RW, 1);
        assert_eq!(GRAPHICS_BUFFER_MAP_CONSUMER_RO, 2);
        assert_eq!(GRAPHICS_BUFFER_QUEUE_FLAGS_NONE, 0);
        assert_eq!(GRAPHICS_BUFFER_ACQUIRE_FLAGS_NONE, 0);
        assert_eq!(GRAPHICS_BUFFER_RELEASE_FLAGS_NONE, 0);
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!(
            (
                GRAPHICS_BUFFER_WIDTH,
                GRAPHICS_BUFFER_HEIGHT,
                GRAPHICS_BUFFER_PIXEL_COUNT,
            ),
            (208, 368, 76_544)
        );
        #[cfg(feature = "mobile-ui-runtime")]
        assert_eq!(
            (
                GRAPHICS_BUFFER_WIDTH,
                GRAPHICS_BUFFER_HEIGHT,
                GRAPHICS_BUFFER_PIXEL_COUNT,
            ),
            (720, 1_600, 1_152_000)
        );
        assert_eq!(
            GRAPHICS_BUFFER_PIXEL_COUNT,
            GRAPHICS_BUFFER_WIDTH as usize * GRAPHICS_BUFFER_HEIGHT as usize
        );
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!(GRAPHICS_BUFFER_LOGICAL_BYTES, 306_176);
        #[cfg(feature = "mobile-ui-runtime")]
        assert_eq!(GRAPHICS_BUFFER_LOGICAL_BYTES, 4_608_000);
        assert_eq!(
            GRAPHICS_BUFFER_LOGICAL_BYTES,
            GRAPHICS_BUFFER_PIXEL_COUNT * 4
        );
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!(GRAPHICS_BUFFER_BACKING_BYTES, 307_200);
        #[cfg(feature = "mobile-ui-runtime")]
        assert_eq!(GRAPHICS_BUFFER_BACKING_BYTES, 4_608_000);
        assert_eq!(GRAPHICS_BUFFER_BACKING_BYTES % 4096, 0);
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!(
            GRAPHICS_BUFFER_BACKING_BYTES - GRAPHICS_BUFFER_LOGICAL_BYTES,
            1024
        );
        #[cfg(feature = "mobile-ui-runtime")]
        assert_eq!(
            GRAPHICS_BUFFER_BACKING_BYTES - GRAPHICS_BUFFER_LOGICAL_BYTES,
            0
        );
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!(GRAPHICS_BUFFER_WRITE_MAX_BYTES, 4_096);
        #[cfg(feature = "mobile-ui-runtime")]
        {
            assert_eq!(GRAPHICS_BUFFER_WRITE_MAX_BYTES, 63_360);
            assert_eq!(super::GRAPHICS_BUFFER_WRITE_MAX_ROWS, 22);
            assert_eq!(super::GRAPHICS_BUFFER_FRAME_WRITE_CALLS, 73);
            assert_eq!(
                GRAPHICS_BUFFER_WRITE_MAX_BYTES,
                super::GRAPHICS_BUFFER_WRITE_MAX_ROWS * GRAPHICS_BUFFER_WIDTH as usize * 4
            );
        }
    }

    #[test]
    fn graphics_buffer_geometry_argument_preserves_width_and_height_boundaries() {
        assert_eq!(pack_graphics_buffer_geometry(0, 0), 0);
        assert_eq!(unpack_graphics_buffer_geometry(0), (0, 0));

        let fixed = pack_graphics_buffer_geometry(GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_HEIGHT);
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!(fixed, 0x0000_00d0_0000_0170);
        #[cfg(feature = "mobile-ui-runtime")]
        assert_eq!(fixed, 0x0000_02d0_0000_0640);
        assert_eq!(
            unpack_graphics_buffer_geometry(fixed),
            (GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_HEIGHT)
        );

        let maximum = pack_graphics_buffer_geometry(u32::MAX, u32::MAX);
        assert_eq!(maximum, u64::MAX);
        assert_eq!(
            unpack_graphics_buffer_geometry(maximum),
            (u32::MAX, u32::MAX)
        );
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn layered_surface_handle_register_is_stable_and_lossless() {
        let content = HandleValue::from_raw(0x1234_5678);
        let chrome = HandleValue::from_raw(0x9abc_def0);
        let packed = super::pack_surface_layer_handles(content, chrome);
        assert_eq!(packed, 0x1234_5678_9abc_def0);
        assert_eq!(
            super::unpack_surface_layer_handles(packed),
            (content, chrome)
        );
    }

    #[test]
    fn graphics_buffer_write_argument_preserves_offset_and_length_boundaries() {
        assert_eq!(pack_graphics_buffer_write(0, 0), 0);
        assert_eq!(unpack_graphics_buffer_write(0), (0, 0));

        let packed = pack_graphics_buffer_write(0xfedc_ba98, 0x7654_3210);
        assert_eq!(packed, 0xfedc_ba98_7654_3210);
        assert_eq!(
            unpack_graphics_buffer_write(packed),
            (0xfedc_ba98, 0x7654_3210)
        );

        let maximum = pack_graphics_buffer_write(u32::MAX, u32::MAX);
        assert_eq!(maximum, u64::MAX);
        assert_eq!(unpack_graphics_buffer_write(maximum), (u32::MAX, u32::MAX));
    }

    #[test]
    fn wait_many_abis_have_stable_bounds_wire_size_and_timeout_sentinels() {
        assert_eq!(OBJECT_WAIT_MANY_ITEM_COUNT, 2);
        assert_eq!(OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS, 8);
        assert_eq!(OBJECT_WAIT_MANY_ARRAY_ITEM_WIRE_SIZE, 8);
        assert_eq!(OBJECT_WAIT_MANY_ARRAY_MAX_WIRE_SIZE, 64);
        assert_eq!(
            OBJECT_WAIT_MANY_ARRAY_ITEM_WIRE_SIZE,
            core::mem::size_of::<u64>()
        );
        assert_eq!(OBJECT_WAIT_TIMEOUT_POLL, 0);
        assert_eq!(OBJECT_WAIT_TIMEOUT_INFINITE, u64::MAX);
        assert_ne!(OBJECT_WAIT_TIMEOUT_POLL, OBJECT_WAIT_TIMEOUT_INFINITE);
    }

    #[test]
    fn wait_item_preserves_all_handle_and_known_signal_bits() {
        let handle = HandleValue::from_raw(0xfedc_ba98);
        let signals = ObjectSignals::from_bits(
            ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits(),
        )
        .unwrap();
        let packed = pack_wait_item(handle, signals);
        assert_eq!(packed, 0xfedc_ba98_0000_0005);
        assert_eq!(unpack_wait_item(packed), (handle, signals.bits()));

        let packed_min = pack_wait_item(HandleValue::INVALID, ObjectSignals::NONE);
        assert_eq!(packed_min, 0);
        assert_eq!(
            unpack_wait_item(packed_min),
            (HandleValue::INVALID, ObjectSignals::NONE.bits())
        );

        let max_handle = HandleValue::from_raw(u32::MAX);
        let packed_max = pack_wait_item(max_handle, ObjectSignals::ALL);
        assert_eq!(packed_max, 0xffff_ffff_0000_003f);
        assert_eq!(
            unpack_wait_item(packed_max),
            (max_handle, ObjectSignals::ALL.bits())
        );
    }

    #[test]
    fn wait_item_unpack_preserves_unknown_signal_bits_for_validation() {
        let packed = 0x1234_5678_8000_0000;
        let (handle, raw_signals) = unpack_wait_item(packed);
        assert_eq!(handle, HandleValue::from_raw(0x1234_5678));
        assert_eq!(raw_signals, 1 << 31);
        assert_eq!(ObjectSignals::from_bits(raw_signals), None);
    }
}
