//! Feature-gated M33c resident application lifecycle runtime.
//!
//! The default M32 proof remains in `main.rs`.  This module owns the alternate
//! startup graph so changing endpoint authority cannot perturb the certified
//! default ledger.

use super::*;
use bndr_abi::CHILD_EXIT_MAGIC;
#[cfg(any(
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime",
    all(
        feature = "mapped-graphics-runtime",
        not(feature = "app-crash-recovery-runtime")
    )
))]
use bndr_abi::{
    GRAPHICS_BUFFER_ACQUIRE_FLAGS_NONE, GRAPHICS_BUFFER_BACKING_BYTES,
    GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE, GRAPHICS_BUFFER_MAP_ADDRESS,
    GRAPHICS_BUFFER_MAP_PRODUCER_RW, GRAPHICS_BUFFER_MAP_STRIDE, GRAPHICS_BUFFER_PIXEL_COUNT,
    GRAPHICS_BUFFER_QUEUE_FLAGS_NONE,
};
#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
use bndr_abi::{GRAPHICS_BUFFER_MAP_CONSUMER_RO, GRAPHICS_BUFFER_RELEASE_FLAGS_NONE};
#[cfg(feature = "service-supervisor-runtime")]
use bndr_input::InputQuarantineReason;
#[cfg(all(
    feature = "input-server-surface-restart-runtime",
    not(feature = "input-server-restart-runtime"),
    not(feature = "service-dependency-runtime")
))]
use bndr_input::InputRouteGapMetrics;
#[cfg(feature = "service-dependency-runtime")]
use bndr_input::InputRouteGapMetrics;
#[cfg(any(
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
use bndr_input::{
    INPUT_RESTART_BACKOFF_NS, InputServiceFocus, InputServiceRecoveryMessage,
    InputServiceRecoveryPayload, InputServiceRecoveryPhase, InputServiceRecoverySequenceTracker,
};
#[cfg(feature = "input-server-restart-runtime")]
use bndr_input::{InputRestartPolicy, InputRestartState, InputServiceGeneration};
#[cfg(any(
    feature = "input-server-surface-restart-runtime",
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
use bndr_input::{InputRouteControl, InputRouteControlPayload, OwnerPid};
#[cfg(feature = "service-dependency-runtime")]
use bndr_sm::health::{DependencyImpact, DependencyKind};
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
use bndr_sm::health::{
    FaultClass, FaultDisposition, HealthFrame, HealthOpcode, ServiceIdentity, ServiceKind,
    ServicePhase, ServicePolicy, ServiceSupervisor,
};
use bndr_ui::{
    APP_LIFECYCLE_WIRE_SIZE, AppInstanceIdentity, AppLifecycleAction, AppLifecycleMessage,
    AppLifecyclePayload, AppLifecycleReason, AppLifecycleState, AppLifecycleStateMachine,
    AppLifecycleStatus, AppLifecycleTracker, BUFFER_PRESENT_WIRE_SIZE, UI_BOOTSTRAP_WIRE_SIZE,
    UI_SUPERVISOR_CONTROL_WIRE_SIZE, UiBootstrapEndpointKind, UiBootstrapMessage, UiClientId,
    UiServerEvent, UiServerEventPayload, UiSupervisorControlMessage, UiSupervisorControlPayload,
    UiSupervisorOperation, UiSupervisorStatus, UiSupervisorTracker,
};
#[cfg(all(
    feature = "input-server-surface-restart-runtime",
    not(feature = "input-server-restart-runtime"),
    not(feature = "service-dependency-runtime")
))]
use bndr_ui::{
    RecoveryCancelKind, SurfaceRecoveryCheckpoint, SurfaceRecoveryMessage, SurfaceRecoveryPayload,
    SurfaceRecoveryPhase,
};
#[cfg(feature = "service-dependency-runtime")]
use bndr_ui::{
    RecoveryCancelKind, SurfaceRecoveryCheckpoint, SurfaceRecoveryMessage, SurfaceRecoveryPayload,
    SurfaceRecoveryPhase,
};

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
mod multi_window_runtime;

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
pub(super) fn multi_window_surface_runtime(startup: u64) -> ! {
    multi_window_runtime::surface_runtime(startup)
}

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
pub(super) fn multi_window_launcher_runtime(startup: u64) -> ! {
    multi_window_runtime::launcher_runtime(startup)
}

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
pub(super) fn multi_window_app_runtime(startup: u64) -> ! {
    multi_window_runtime::app_runtime(startup)
}

const FAIL_RUNTIME: u64 = 220;
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
const FAIL_SERVICE_PROBE_ARM: u64 = 221;
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
const FAIL_SERVICE_PROBE_FRAME: u64 = 222;
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
const FAIL_SERVICE_PROBE_WRITE: u64 = 223;
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
const FAIL_SERVICE_HEALTH_WAIT: u64 = 224;
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
const FAIL_SERVICE_HEALTH_OUTCOME: u64 = 225;
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
const FAIL_SERVICE_CADENCE_WAIT: u64 = 226;
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
const FAIL_SERVICE_CADENCE_OUTCOME: u64 = 227;
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
const FAIL_SERVICE_CADENCE_CLOCK: u64 = 228;
#[cfg(feature = "service-dependency-runtime")]
const FAIL_DEPENDENCY_TRANSITION: u64 = 229;
#[cfg(feature = "service-dependency-runtime")]
const FAIL_DEPENDENCY_SURFACE_BACKOFF: u64 = 230;
#[cfg(feature = "service-dependency-runtime")]
const FAIL_DEPENDENCY_DEGRADED_HOLD: u64 = 231;
#[cfg(feature = "service-dependency-runtime")]
const FAIL_DEPENDENCY_INPUT_BACKOFF: u64 = 232;
#[cfg(feature = "service-dependency-runtime")]
const FAIL_DEPENDENCY_FINAL_CADENCE: u64 = 233;
#[cfg(feature = "service-dependency-runtime")]
const FAIL_DEPENDENCY_SURFACE_RECOVERED_HOLD: u64 = 234;
#[cfg(feature = "service-dependency-runtime")]
const SERVICE_HEALTH_TIMEOUT_NS: u64 = 100_000_000;
#[cfg(all(
    feature = "service-supervisor-runtime",
    not(feature = "service-dependency-runtime")
))]
const SERVICE_HEALTH_TIMEOUT_NS: u64 = 200_000_000;
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
const SERVICE_RECOVERED_CADENCE_NS: u64 = 2_000_000_000;
#[cfg(all(
    feature = "input-server-surface-restart-runtime",
    not(feature = "input-server-restart-runtime"),
    not(feature = "service-dependency-runtime")
))]
const INPUT_SURFACE_RESTART_READY_MAGIC: u64 = 0x4d34_315f_5749_4e44;
#[cfg(feature = "service-dependency-runtime")]
const INPUT_SURFACE_RESTART_READY_MAGIC: u64 = 0x4d34_315f_5749_4e44;
#[cfg(feature = "post-recovery-interaction-runtime")]
const POST_RECOVERY_INTERACTION_READY_MAGIC: u64 = 0x4d35_305f_504f_5354;
#[cfg(feature = "post-recovery-interaction-runtime")]
const POST_RECOVERY_APP_ACK_MAGIC: u64 = 0x4d35_305f_4150_504f;
#[cfg(feature = "input-server-runtime")]
const FAIL_INPUT_SURFACE_SPAWN_WAIT: u64 = 250;
#[cfg(feature = "input-server-runtime")]
const FAIL_INPUT_SURFACE_SPAWN_MEMORY: u64 = 251;
#[cfg(feature = "input-server-runtime")]
const FAIL_INPUT_SURFACE_SPAWN_NOT_FOUND: u64 = 252;
#[cfg(feature = "input-server-runtime")]
const FAIL_INPUT_SURFACE_SPAWN_DENIED: u64 = 253;
#[cfg(feature = "input-server-runtime")]
const FAIL_INPUT_SURFACE_SPAWN_STATE: u64 = 254;
#[cfg(feature = "input-server-runtime")]
const FAIL_INPUT_SURFACE_SPAWN_OTHER: u64 = 255;
const APP: ShellAppId = ShellAppId::Phone;
#[cfg(all(
    feature = "graphics-owner-death-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
const GRAPHICS_OWNER_DEATH_READY_MAGIC: u64 = 0x4d33_365f_5257_4f4b;
#[cfg(feature = "graphics-surface-restart-runtime")]
const GRAPHICS_SURFACE_RESTART_READY_MAGIC: u64 = 0x4d33_375f_5352_564b;
#[cfg(feature = "graphics-surface-restart-runtime")]
const GRAPHICS_SURFACE_REBOUND_MAGIC: u64 = 0x4d33_375f_5549_4f4b;
#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const GRAPHICS_PRODUCER_ORPHAN_READY_MAGIC: u64 = 0x4d33_385f_4f52_4f4b;
#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const GRAPHICS_PRODUCER_REUSE_REQUEST_MAGIC: u64 = 0x4d33_385f_5251_4f4b;
#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const GRAPHICS_PRODUCER_REUSE_DONE_MAGIC: u64 = 0x4d33_385f_5244_4f4b;
#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const FRAME_CLOCK_LATE_TIMEOUT_NS: u64 = 30_000_000;
#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-swapchain-runtime"),
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const FRAME_CLOCK_BACKPRESSURE_MAGIC: u64 = 0x4d33_395f_4250_0000;
#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const SWAPCHAIN_INITIAL_BATCH_MAGIC: u64 = 0x4d34_305f_494e_0003;
#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const SWAPCHAIN_ACTIVE_BATCH_MAGIC: u64 = 0x4d34_305f_4143_0003;
#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const SWAPCHAIN_REFILL_A_MAGIC: u64 = 0x4d34_305f_5241_0001;
#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const SWAPCHAIN_REFILL_B_MAGIC: u64 = 0x4d34_305f_5242_0002;
#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const SWAPCHAIN_FINAL_MIXED_MAGIC: u64 = 0x4d34_305f_464e_0002;

// `--all-features` remains a supported workspace lint configuration. M47 is
// the newest leaf runtime, so it owns the shared lifecycle and mapped-graphics
// path whenever its feature is selected. Real QEMU profiles still select one
// leaf feature, while this precedence keeps mechanical aggregate builds
// behaviorally equivalent to the dedicated M47 profile.

#[derive(Clone, Copy)]
enum LifecycleStep {
    Action(AppLifecycleAction),
    #[cfg(feature = "app-crash-recovery-runtime")]
    Crash,
}

#[cfg(feature = "input-server-restart-runtime")]
const LIFECYCLE_STEPS: [LifecycleStep; 2] = [
    LifecycleStep::Action(AppLifecycleAction::Launch),
    LifecycleStep::Action(AppLifecycleAction::Activate),
];

#[cfg(all(
    not(feature = "input-server-restart-runtime"),
    not(feature = "app-crash-recovery-runtime"),
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "graphics-frame-clock-runtime")
))]
const LIFECYCLE_STEPS: [LifecycleStep; 7] = [
    LifecycleStep::Action(AppLifecycleAction::Launch),
    LifecycleStep::Action(AppLifecycleAction::Activate),
    LifecycleStep::Action(AppLifecycleAction::Suspend),
    LifecycleStep::Action(AppLifecycleAction::Resume),
    LifecycleStep::Action(AppLifecycleAction::Terminate),
    LifecycleStep::Action(AppLifecycleAction::Launch),
    LifecycleStep::Action(AppLifecycleAction::Activate),
];

#[cfg(all(
    not(feature = "input-server-restart-runtime"),
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const LIFECYCLE_STEPS: [LifecycleStep; 2] = [
    LifecycleStep::Action(AppLifecycleAction::Launch),
    LifecycleStep::Action(AppLifecycleAction::Activate),
];

#[cfg(all(
    not(feature = "input-server-restart-runtime"),
    feature = "graphics-owner-death-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const LIFECYCLE_STEPS: [LifecycleStep; 1] = [LifecycleStep::Action(AppLifecycleAction::Launch)];

#[cfg(all(
    not(feature = "input-server-restart-runtime"),
    feature = "graphics-surface-restart-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
const LIFECYCLE_STEPS: [LifecycleStep; 2] = [
    LifecycleStep::Action(AppLifecycleAction::Launch),
    LifecycleStep::Action(AppLifecycleAction::Activate),
];

#[cfg(all(
    feature = "app-crash-recovery-runtime",
    not(feature = "input-server-restart-runtime")
))]
const LIFECYCLE_STEPS: [LifecycleStep; 10] = [
    LifecycleStep::Action(AppLifecycleAction::Launch),
    LifecycleStep::Action(AppLifecycleAction::Activate),
    LifecycleStep::Action(AppLifecycleAction::Suspend),
    LifecycleStep::Action(AppLifecycleAction::Resume),
    LifecycleStep::Action(AppLifecycleAction::Terminate),
    LifecycleStep::Action(AppLifecycleAction::Launch),
    LifecycleStep::Action(AppLifecycleAction::Activate),
    LifecycleStep::Crash,
    LifecycleStep::Action(AppLifecycleAction::Launch),
    LifecycleStep::Action(AppLifecycleAction::Activate),
];

const TRANSACTION_COUNT: u64 = LIFECYCLE_STEPS.len() as u64;

struct InitAppGeneration {
    lifecycle: OwnedUserHandle,
    app_ui: Option<OwnedUserHandle>,
    surface_ui: Option<OwnedUserHandle>,
    pid: u64,
    identity: AppInstanceIdentity,
}

pub(super) fn init_runtime(system_root: u64) -> ! {
    let mut children = bootstrap_core_runtime(system_root);
    #[cfg(not(feature = "input-server-runtime"))]
    let (mut surface_control, launcher_lifecycle, mut surface_pid, launcher_pid) =
        spawn_window_supervisors();
    #[cfg(all(
        feature = "input-server-runtime",
        not(feature = "input-server-surface-restart-runtime"),
        not(feature = "input-server-restart-runtime"),
        not(feature = "service-dependency-runtime")
    ))]
    let (
        mut surface_control,
        launcher_lifecycle,
        mut surface_pid,
        launcher_pid,
        input_server_control,
        input_server_pid,
    ) = spawn_window_supervisors();
    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    let (
        mut surface_control,
        launcher_lifecycle,
        mut surface_pid,
        launcher_pid,
        input_server_control,
        input_server_pid,
        input_server_session,
    ) = spawn_window_supervisors();
    children.surface_server_pid = surface_pid;
    children.launcher_pid = launcher_pid;
    #[cfg(feature = "input-server-runtime")]
    {
        if input_server_pid == 0
            || input_server_pid == surface_pid
            || input_server_pid == launcher_pid
        {
            fail(FAIL_RUNTIME);
        }
        children.input_server_pid = input_server_pid;
        children.input_server_control = Some(input_server_control);
        #[cfg(any(
            feature = "input-server-surface-restart-runtime",
            feature = "input-server-restart-runtime",
            feature = "service-dependency-runtime"
        ))]
        {
            if input_server_session == 0 {
                fail(FAIL_RUNTIME);
            }
            children.input_server_session = input_server_session;
            children.input_route_epoch = 1;
        }
    }

    let mut lifecycle = AppLifecycleTracker::new();
    let mut machine = AppLifecycleStateMachine::new(APP);
    let mut supervisor = UiSupervisorTracker::new();
    let mut resident_app: Option<InitAppGeneration> = None;
    let mut first_app_pid = 0;
    let mut capacity_probed = false;
    let mut launcher_request_sequence = 0;
    let mut launcher_state_sequence = 0;
    let mut supervisor_command_sequence = 0;
    let mut supervisor_response_sequence = 0;

    for (index, step) in LIFECYCLE_STEPS.into_iter().enumerate() {
        let transaction_id = index as u64 + 1;
        match step {
            LifecycleStep::Action(action) => execute_lifecycle_action(
                transaction_id,
                action,
                &mut lifecycle,
                &mut machine,
                &mut supervisor,
                &mut resident_app,
                &mut children,
                launcher_lifecycle.raw(),
                launcher_pid,
                &mut surface_control,
                &mut surface_pid,
                &mut first_app_pid,
                &mut capacity_probed,
                &mut launcher_request_sequence,
                &mut launcher_state_sequence,
                &mut supervisor_command_sequence,
                &mut supervisor_response_sequence,
            ),
            #[cfg(feature = "app-crash-recovery-runtime")]
            LifecycleStep::Crash => recover_crashed_app(
                transaction_id,
                &mut lifecycle,
                &mut machine,
                &mut supervisor,
                &mut resident_app,
                launcher_lifecycle.raw(),
                surface_control.raw(),
                surface_pid,
                &mut launcher_state_sequence,
                &mut supervisor_response_sequence,
            ),
        }
    }

    #[cfg(all(
        feature = "graphics-owner-death-runtime",
        not(feature = "graphics-surface-restart-runtime"),
        not(feature = "graphics-producer-orphan-runtime"),
        not(feature = "input-server-restart-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    {
        if machine.state() != AppLifecycleState::Inactive
            || machine
                .identity()
                .is_none_or(|identity| identity.instance_id() != 1)
            || machine.last_transaction_id() != Some(TRANSACTION_COUNT)
            || resident_app.is_some()
            || !capacity_probed
        {
            fail(FAIL_RUNTIME);
        }
        close_owned(surface_control);
        loop {
            core::hint::spin_loop();
        }
    }
    #[cfg(all(
        feature = "graphics-producer-orphan-runtime",
        not(feature = "graphics-surface-restart-runtime"),
        not(feature = "input-server-restart-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    finish_graphics_producer_orphan_runtime(
        &mut machine,
        &mut resident_app,
        &mut children,
        surface_control,
        launcher_lifecycle,
        surface_pid,
        launcher_pid,
        capacity_probed,
    );
    #[cfg(all(
        feature = "graphics-surface-restart-runtime",
        not(feature = "input-server-restart-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    {
        let final_app = resident_app.unwrap_or_else(|| fail(FAIL_RUNTIME));
        if machine.state() != AppLifecycleState::Active
            || machine.identity() != Some(final_app.identity)
            || machine.last_transaction_id() != Some(TRANSACTION_COUNT)
            || !capacity_probed
            || children.surface_server_pid != surface_pid
        {
            fail(FAIL_RUNTIME);
        }
        supervise_runtime(
            &children,
            surface_control,
            launcher_lifecycle,
            final_app.lifecycle,
        )
    }
    #[cfg(not(all(
        feature = "graphics-owner-death-runtime",
        not(feature = "input-server-restart-runtime"),
        not(feature = "app-crash-recovery-runtime")
    )))]
    #[cfg(not(all(
        feature = "graphics-surface-restart-runtime",
        not(feature = "input-server-restart-runtime"),
        not(feature = "app-crash-recovery-runtime")
    )))]
    {
        let final_app = resident_app.unwrap_or_else(|| fail(FAIL_RUNTIME));
        if machine.state() != AppLifecycleState::Active
            || machine.identity() != Some(final_app.identity)
            || machine.last_transaction_id() != Some(TRANSACTION_COUNT)
            || !capacity_probed
        {
            fail(FAIL_RUNTIME);
        }
        #[cfg(feature = "service-dependency-runtime")]
        finish_service_dependency_runtime(
            &mut children,
            surface_control,
            surface_pid,
            launcher_lifecycle,
            launcher_pid,
            final_app,
        );
        #[cfg(all(
            feature = "input-server-restart-runtime",
            not(feature = "service-dependency-runtime")
        ))]
        finish_input_server_restart_runtime(
            &mut children,
            surface_control,
            surface_pid,
            launcher_lifecycle,
            launcher_pid,
            final_app,
        );
        #[cfg(all(
            feature = "input-server-surface-restart-runtime",
            not(feature = "input-server-restart-runtime"),
            not(feature = "service-dependency-runtime")
        ))]
        finish_input_server_surface_restart_runtime(
            &mut children,
            surface_control,
            surface_pid,
            launcher_lifecycle,
            launcher_pid,
            final_app,
        );
        #[cfg(all(
            feature = "multi-window-runtime",
            not(feature = "input-server-surface-restart-runtime"),
            not(feature = "input-server-restart-runtime"),
            not(feature = "service-dependency-runtime"),
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        ))]
        read_authenticated_magic(
            surface_control.raw(),
            surface_pid,
            multi_window_runtime::READY_MAGIC,
        );
        #[cfg(not(any(
            feature = "input-server-surface-restart-runtime",
            feature = "input-server-restart-runtime",
            feature = "service-dependency-runtime"
        )))]
        supervise_runtime(
            &children,
            surface_control,
            launcher_lifecycle,
            final_app.lifecycle,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_lifecycle_action(
    transaction_id: u64,
    action: AppLifecycleAction,
    lifecycle: &mut AppLifecycleTracker,
    machine: &mut AppLifecycleStateMachine,
    supervisor: &mut UiSupervisorTracker,
    resident_app: &mut Option<InitAppGeneration>,
    children: &mut ResidentChildren,
    launcher_lifecycle: u64,
    launcher_pid: u64,
    surface_control: &mut OwnedUserHandle,
    surface_pid: &mut u64,
    first_app_pid: &mut u64,
    capacity_probed: &mut bool,
    launcher_request_sequence: &mut u64,
    launcher_state_sequence: &mut u64,
    supervisor_command_sequence: &mut u64,
    supervisor_response_sequence: &mut u64,
) {
    let request = read_lifecycle_message(launcher_lifecycle, launcher_pid, false).0;
    let request_identity = resident_app.as_ref().map(|app| app.identity);
    if request.sender_sequence() != next_sequence(launcher_request_sequence)
        || request.transaction_id() != transaction_id
        || request.payload()
            != (AppLifecyclePayload::Request {
                app: APP,
                action,
                identity: if action == AppLifecycleAction::Launch {
                    None
                } else {
                    request_identity
                },
            })
        || lifecycle.accept(request).is_err()
    {
        fail(FAIL_RUNTIME);
    }

    let begin_identity = if action == AppLifecycleAction::Launch {
        None
    } else {
        request_identity
    };
    let intermediate = machine
        .begin(transaction_id, action, begin_identity)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));

    if action == AppLifecycleAction::Launch {
        let instance_id = launch_instance_id(transaction_id);
        let app = spawn_app_generation(instance_id);
        if transaction_id == 1 {
            *first_app_pid = app.pid;
        } else {
            let generation_delta =
                u32::try_from(instance_id - 1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
            if process_slot(app.pid) != process_slot(*first_app_pid)
                || process_generation(app.pid)
                    != process_generation(*first_app_pid)
                        .checked_add(generation_delta)
                        .unwrap_or_else(|| fail(FAIL_RUNTIME))
            {
                fail(FAIL_RUNTIME);
            }
        }
        machine
            .bind_identity(transaction_id, app.identity)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        children.app_pid = app.pid;
        *resident_app = Some(app);
        if !*capacity_probed {
            verify_process_capacity_is_stable();
            *capacity_probed = true;
        }
    }

    let identity = resident_app
        .as_ref()
        .map(|app| app.identity)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let intermediate_message = AppLifecycleMessage::state_changed(
        next_sequence(launcher_state_sequence),
        transaction_id,
        APP,
        intermediate,
        AppLifecycleReason::Requested,
        Some(identity),
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    accept_and_write_lifecycle(lifecycle, launcher_lifecycle, intermediate_message, None);

    let command = AppLifecycleMessage::command(
        app_endpoint_sequence(transaction_id),
        transaction_id,
        APP,
        action,
        identity,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let app = resident_app.as_mut().unwrap_or_else(|| fail(FAIL_RUNTIME));
    let transfer = if action == AppLifecycleAction::Launch {
        Some(app.app_ui.take().unwrap_or_else(|| fail(FAIL_RUNTIME)))
    } else {
        None
    };
    accept_and_write_lifecycle(lifecycle, app.lifecycle.raw(), command, transfer);

    let ack = read_lifecycle_message(app.lifecycle.raw(), app.pid, false).0;
    if ack.sender_sequence() != app_endpoint_sequence(transaction_id)
        || ack.transaction_id() != transaction_id
        || ack.payload()
            != (AppLifecyclePayload::Ack {
                app: APP,
                action,
                status: AppLifecycleStatus::Applied,
                identity,
            })
        || lifecycle.accept(ack).is_err()
    {
        fail(FAIL_RUNTIME);
    }

    let (operation, supervisor_app, supervisor_identity) = match action {
        AppLifecycleAction::Launch => (
            UiSupervisorOperation::InstallAppEndpoint,
            Some(APP),
            Some(identity),
        ),
        AppLifecycleAction::Activate | AppLifecycleAction::Resume => (
            UiSupervisorOperation::ActivateApp,
            Some(APP),
            Some(identity),
        ),
        AppLifecycleAction::Suspend => (UiSupervisorOperation::ShowLauncher, None, None),
        AppLifecycleAction::Terminate => (
            UiSupervisorOperation::RetireAppEndpoint,
            Some(APP),
            Some(identity),
        ),
    };
    let supervisor_command = UiSupervisorControlMessage::command(
        next_sequence(supervisor_command_sequence),
        transaction_id,
        operation,
        supervisor_app,
        supervisor_identity,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let supervisor_transfer = if action == AppLifecycleAction::Launch {
        resident_app.as_mut().and_then(|app| app.surface_ui.take())
    } else {
        None
    };
    accept_and_write_supervisor(
        supervisor,
        surface_control.raw(),
        supervisor_command,
        supervisor_transfer,
    );
    let supervisor_ack = read_supervisor_message(surface_control.raw(), *surface_pid).0;
    if supervisor_ack.sender_sequence() != next_sequence(supervisor_response_sequence)
        || supervisor_ack.transaction_id() != transaction_id
        || supervisor_ack.payload()
            != (UiSupervisorControlPayload::Ack {
                operation,
                status: UiSupervisorStatus::Applied,
                app: supervisor_app,
                identity: supervisor_identity,
            })
        || supervisor.accept(supervisor_ack).is_err()
    {
        fail(FAIL_RUNTIME);
    }

    #[cfg(all(
        feature = "graphics-owner-death-runtime",
        not(feature = "input-server-restart-runtime"),
        not(feature = "app-crash-recovery-runtime"),
        any(
            not(feature = "graphics-producer-orphan-runtime"),
            feature = "graphics-surface-restart-runtime"
        )
    ))]
    if transaction_id == 1 && action == AppLifecycleAction::Launch {
        let terminated = syscall(
            SyscallNumber::ProcessTerminate,
            *surface_pid,
            PROCESS_TERMINATE_FLAGS_NONE,
            0,
        );
        if terminated.status != Status::Ok.raw() || terminated.out1 != 0 || terminated.out2 != 0 {
            fail(FAIL_RUNTIME);
        }
        let waited = syscall(SyscallNumber::ProcessWait, *surface_pid, 0, 0);
        if waited.status != Status::Ok.raw()
            || waited.out1 != PROCESS_KILLED_EXIT_CODE
            || waited.out2 != ProcessTerminationReason::Killed.raw()
        {
            fail(FAIL_RUNTIME);
        }

        // The App emits this only after its WRITABLE wait returns, it has
        // volatile-written every page of the restored producer alias, and it
        // has re-read canonical samples. Reaper metadata alone therefore
        // cannot satisfy this proof.
        let app = resident_app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
        let recovered_pid = app.pid;
        let envelope = read_channel_envelope(app.lifecycle.raw());
        if envelope.kind() != ChannelMessageKind::Bytes
            || envelope.logical_length() != size_of::<u64>()
            || envelope.sender_pid() != app.pid
            || envelope.received_handle().is_valid()
            || envelope.data()[..size_of::<u64>()] != GRAPHICS_OWNER_DEATH_READY_MAGIC.to_le_bytes()
        {
            fail(FAIL_RUNTIME);
        }
        #[cfg(not(feature = "graphics-surface-restart-runtime"))]
        {
            let app_waited = syscall(SyscallNumber::ProcessWait, recovered_pid, 0, 0);
            if app_waited.status != Status::Ok.raw()
                || app_waited.out1 != CHILD_EXIT_MAGIC
                || app_waited.out2 != ProcessTerminationReason::Exited.raw()
            {
                fail(FAIL_RUNTIME);
            }
            let recovered = resident_app.take().unwrap_or_else(|| fail(FAIL_RUNTIME));
            if recovered.pid != recovered_pid
                || recovered.app_ui.is_some()
                || recovered.surface_ui.is_some()
            {
                fail(FAIL_RUNTIME);
            }
            close_owned(recovered.lifecycle);
        }
        #[cfg(feature = "graphics-surface-restart-runtime")]
        {
            let app = resident_app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
            if app.pid != recovered_pid {
                fail(FAIL_RUNTIME);
            }
            restart_surface_graph(
                surface_control,
                surface_pid,
                children,
                launcher_lifecycle,
                launcher_pid,
                app.lifecycle.raw(),
                app.pid,
            );
        }
    }

    #[cfg(all(
        feature = "graphics-surface-restart-runtime",
        not(feature = "input-server-restart-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    if transaction_id == 2 && action == AppLifecycleAction::Activate {
        let app = resident_app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
        let envelope = read_channel_envelope(app.lifecycle.raw());
        if envelope.kind() != ChannelMessageKind::Bytes
            || envelope.logical_length() != size_of::<u64>()
            || envelope.sender_pid() != app.pid
            || envelope.received_handle().is_valid()
            || envelope.data()[..size_of::<u64>()] != GRAPHICS_SURFACE_REBOUND_MAGIC.to_le_bytes()
        {
            fail(FAIL_RUNTIME);
        }
    }

    if action == AppLifecycleAction::Terminate {
        let retired = resident_app.take().unwrap_or_else(|| fail(FAIL_RUNTIME));
        close_owned(retired.lifecycle);
        wait_for_child_exit(retired.pid, CHILD_EXIT_MAGIC, FAIL_RUNTIME);
    }

    let completed = machine
        .complete(transaction_id, identity)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let final_message = AppLifecycleMessage::state_changed(
        next_sequence(launcher_state_sequence),
        transaction_id,
        APP,
        completed,
        AppLifecycleReason::Completed,
        Some(identity),
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    accept_and_write_lifecycle(lifecycle, launcher_lifecycle, final_message, None);
}

#[cfg(feature = "app-crash-recovery-runtime")]
#[allow(clippy::too_many_arguments)]
fn recover_crashed_app(
    transaction_id: u64,
    lifecycle: &mut AppLifecycleTracker,
    machine: &mut AppLifecycleStateMachine,
    supervisor: &mut UiSupervisorTracker,
    resident_app: &mut Option<InitAppGeneration>,
    launcher_lifecycle: u64,
    surface_control: u64,
    surface_pid: u64,
    launcher_state_sequence: &mut u64,
    supervisor_response_sequence: &mut u64,
) {
    let crashed = resident_app.take().unwrap_or_else(|| fail(FAIL_RUNTIME));
    let identity = crashed.identity;

    // The final Activate acknowledgement makes init runnable before App2 has
    // necessarily returned to its next ObjectWaitManyArray call.  Cross one
    // genuine lower-EL timer scheduling boundary so round-robin execution can
    // republish that wait token.  The kernel's exact abandonment ledger below
    // remains the fail-closed proof that the target was actually waiting.
    wait_for_lower_el_timer_preemption();

    let terminated = syscall(
        SyscallNumber::ProcessTerminate,
        crashed.pid,
        PROCESS_TERMINATE_FLAGS_NONE,
        0,
    );
    if terminated.status != Status::Ok.raw() || terminated.out1 != 0 || terminated.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    let waited = syscall(SyscallNumber::ProcessWait, crashed.pid, 0, 0);
    if waited.status != Status::Ok.raw()
        || waited.out1 != PROCESS_KILLED_EXIT_CODE
        || waited.out2 != ProcessTerminationReason::Killed.raw()
    {
        fail(FAIL_RUNTIME);
    }

    let owner_died = read_supervisor_message(surface_control, surface_pid).0;
    if owner_died.sender_sequence() != next_sequence(supervisor_response_sequence)
        || owner_died.transaction_id() != transaction_id
        || owner_died.payload() != (UiSupervisorControlPayload::OwnerDied { app: APP, identity })
        || supervisor.accept(owner_died).is_err()
    {
        fail(FAIL_RUNTIME);
    }
    close_owned(crashed.lifecycle);

    let crashed_state = machine
        .crash(transaction_id, identity)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let notification = AppLifecycleMessage::state_changed(
        next_sequence(launcher_state_sequence),
        transaction_id,
        APP,
        crashed_state,
        AppLifecycleReason::ProcessExited,
        Some(identity),
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    accept_and_write_lifecycle(lifecycle, launcher_lifecycle, notification, None);
}

const fn launch_instance_id(transaction_id: u64) -> u64 {
    match transaction_id {
        1 => 1,
        6 => 2,
        #[cfg(feature = "app-crash-recovery-runtime")]
        9 => 3,
        _ => 0,
    }
}

fn next_sequence(sequence: &mut u64) -> u64 {
    *sequence = sequence
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    *sequence
}

fn bootstrap_core_runtime(system_root: u64) -> ResidentChildren {
    if stack_round_trip() != STACK_SENTINEL
        || private_sleep_probe().status != Status::Unsupported.raw()
    {
        fail(FAIL_RUNTIME);
    }
    let abi = syscall(SyscallNumber::AbiVersion, 0, 0, 0);
    if abi.status != Status::Ok.raw()
        || abi.out1 != ABI_VERSION
        || raw_syscall(0xffff, 0, 0, 0).status != Status::Unsupported.raw()
    {
        fail(FAIL_RUNTIME);
    }
    if system_root != 0 {
        exercise_el0_storage(system_root);
    }
    exercise_object_wait_many_array();

    let channel = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    if channel.status != Status::Ok.raw()
        || channel.out1 == 0
        || channel.out2 == 0
        || channel.out1 == channel.out2
    {
        fail(FAIL_RUNTIME);
    }
    let left = channel.out1;
    let right = channel.out2;
    write_scalar(left, EXPECTED_TAG, EXPECTED_PAYLOAD);
    if read_scalar(right) != (EXPECTED_TAG, EXPECTED_PAYLOAD) {
        fail(FAIL_RUNTIME);
    }
    exercise_byte_channel(left, right);
    let duplicate = syscall(
        SyscallNumber::HandleDuplicate,
        left,
        u64::from(Rights::WRITE.bits()),
        0,
    );
    if duplicate.status != Status::Ok.raw() || duplicate.out1 == 0 {
        fail(FAIL_RUNTIME);
    }
    let write_only = duplicate.out1;
    if syscall(SyscallNumber::ChannelRead, write_only, 0, 0).status
        != Status::PermissionDenied.raw()
    {
        fail(FAIL_RUNTIME);
    }

    let mut children = run_supervised_service_manager_rounds();
    write_scalar(left, EXPECTED_TAG, EXPECTED_PAYLOAD);
    if read_scalar(right) != (EXPECTED_TAG, EXPECTED_PAYLOAD) {
        fail(FAIL_RUNTIME);
    }
    for (handle, expected) in [
        (write_only, Status::Ok),
        (left, Status::Ok),
        (left, Status::NotFound),
        (right, Status::Ok),
    ] {
        if syscall(SyscallNumber::HandleClose, handle, 0, 0).status != expected.raw() {
            fail(FAIL_RUNTIME);
        }
    }
    #[cfg(not(feature = "unified-product-runtime"))]
    {
        let ready = syscall(
            SyscallNumber::InitReady,
            INIT_READY_MAGIC,
            REGISTER_SENTINEL,
            stack_round_trip(),
        );
        if ready.status != Status::Ok.raw() {
            fail(FAIL_RUNTIME);
        }
    }
    drive_post_ready_echo_rounds(&children);
    let replacement = drive_dynamic_registration_lifecycle(&children);
    drive_post_cleanup_reuse_round(&children, replacement);
    drive_delegated_endpoint_acl_round(&children);
    drive_multi_client_lookup_round(&mut children);
    children
}

#[cfg(not(feature = "input-server-runtime"))]
fn spawn_window_supervisors() -> (OwnedUserHandle, OwnedUserHandle, u64, u64) {
    // Channel transfer ownership is strictly old-to-new.  Create the
    // Init--Launcher lifecycle transport before the Surface--Launcher UI
    // channel that it bootstraps, then create the still-newer supervisor
    // control channel carried by that UI channel.
    let (launcher_lifecycle, launcher_startup) = create_channel_owned();
    let (surface_startup, launcher_ui) = create_channel_owned();
    let (surface_control, surface_control_child) = create_channel_owned();
    write_bootstrap_transfer(
        launcher_ui.raw(),
        UiBootstrapEndpointKind::SurfaceSupervisor,
        surface_control_child,
    );
    let surface_pid = spawn_owned(surface_startup, UserImageId::SurfaceServer);

    write_bootstrap_transfer(
        launcher_lifecycle.raw(),
        UiBootstrapEndpointKind::LauncherUi,
        launcher_ui,
    );
    let launcher_pid = spawn_owned(launcher_startup, UserImageId::Launcher);
    if surface_pid == launcher_pid {
        fail(FAIL_RUNTIME);
    }
    (
        surface_control,
        launcher_lifecycle,
        surface_pid,
        launcher_pid,
    )
}

#[cfg(all(
    feature = "input-server-runtime",
    not(feature = "input-server-surface-restart-runtime"),
    not(feature = "input-server-restart-runtime"),
    not(feature = "service-dependency-runtime")
))]
fn spawn_window_supervisors() -> (
    OwnedUserHandle,
    OwnedUserHandle,
    u64,
    u64,
    OwnedUserHandle,
    u64,
) {
    // InputServer is a resident authority, not a helper hidden inside
    // SurfaceServer.  Queue its route endpoint on the startup/control pair,
    // then require an authenticated Ready reply before publishing the other
    // route endpoint to SurfaceServer.  Init retains the control peer for the
    // lifetime of the process and therefore observes both protocol messages
    // and peer death.
    let (launcher_lifecycle, launcher_startup) = create_channel_owned();
    let (input_server_control, input_server_startup) = create_channel_owned();
    // Create both transports before the route pair. The kernel's ownership
    // DAG permits only newer handles to move over older channels; both the
    // Init--InputServer control transport and Surface startup transport must
    // therefore predate the two route endpoints they carry.
    let (surface_startup, launcher_ui) = create_channel_owned();
    let (input_route_server, input_route_surface) = create_channel_owned();
    write_wire_transfer(
        input_server_control.raw(),
        &INPUT_ROUTE_BOOTSTRAP_MAGIC.to_le_bytes(),
        input_route_server,
    );
    let input_server_pid = spawn_owned(input_server_startup, UserImageId::InputServer);
    read_authenticated_magic(
        input_server_control.raw(),
        input_server_pid,
        INPUT_SERVER_READY_MAGIC,
    );

    // SurfaceServer receives the established supervisor endpoint first and
    // the input-route endpoint second. Both transferred handles are newer
    // than the startup transport and are queued before the process can run.
    let (surface_control, surface_control_child) = create_channel_owned();
    write_bootstrap_transfer(
        launcher_ui.raw(),
        UiBootstrapEndpointKind::SurfaceSupervisor,
        surface_control_child,
    );
    write_wire_transfer(
        launcher_ui.raw(),
        &INPUT_ROUTE_BOOTSTRAP_MAGIC.to_le_bytes(),
        input_route_surface,
    );
    let surface_startup_raw = surface_startup.raw();
    let spawned_surface = syscall(
        SyscallNumber::ProcessSpawn,
        surface_startup_raw,
        UserImageId::SurfaceServer.raw(),
        PROCESS_SPAWN_FLAGS_NONE,
    );
    if spawned_surface.status != Status::Ok.raw()
        || spawned_surface.out1 == 0
        || spawned_surface.out2 != 0
    {
        let failure = if spawned_surface.status == Status::ShouldWait.raw() {
            FAIL_INPUT_SURFACE_SPAWN_WAIT
        } else if spawned_surface.status == Status::OutOfMemory.raw() {
            FAIL_INPUT_SURFACE_SPAWN_MEMORY
        } else if spawned_surface.status == Status::NotFound.raw() {
            FAIL_INPUT_SURFACE_SPAWN_NOT_FOUND
        } else if spawned_surface.status == Status::PermissionDenied.raw() {
            FAIL_INPUT_SURFACE_SPAWN_DENIED
        } else if spawned_surface.status == Status::InvalidState.raw() {
            FAIL_INPUT_SURFACE_SPAWN_STATE
        } else {
            FAIL_INPUT_SURFACE_SPAWN_OTHER
        };
        fail(failure);
    }
    assert_stale_handle(surface_startup_raw);
    let surface_pid = spawned_surface.out1;

    write_bootstrap_transfer(
        launcher_lifecycle.raw(),
        UiBootstrapEndpointKind::LauncherUi,
        launcher_ui,
    );
    let launcher_pid = spawn_owned(launcher_startup, UserImageId::Launcher);
    if surface_pid == launcher_pid
        || input_server_pid == surface_pid
        || input_server_pid == launcher_pid
    {
        fail(FAIL_RUNTIME);
    }
    (
        surface_control,
        launcher_lifecycle,
        surface_pid,
        launcher_pid,
        input_server_control,
        input_server_pid,
    )
}

/// M49 supervises the resident SurfaceServer and InputServer as two
/// independently budgeted services. Surface replacement reuses M46's BSR1
/// graph repair while Input replacement reuses M47's BIR1/BIP1 snapshot gate;
/// BSH1 remains the only health protocol and no kernel ABI is added.
#[cfg(feature = "service-dependency-runtime")]
#[allow(clippy::too_many_arguments)]
fn finish_service_dependency_runtime(
    children: &mut ResidentChildren,
    mut surface_control: OwnedUserHandle,
    surface_pid: u64,
    launcher_lifecycle: OwnedUserHandle,
    launcher_pid: u64,
    final_app: InitAppGeneration,
) -> ! {
    let old_surface_pid = surface_pid;
    let old_input_pid = children.input_server_pid;
    let old_input_session = children.input_server_session;
    let old_input_control = children
        .input_server_control
        .take()
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if old_surface_pid == 0
        || old_surface_pid != children.surface_server_pid
        || launcher_pid == 0
        || launcher_pid != children.launcher_pid
        || final_app.pid == 0
        || final_app.pid != children.app_pid
        || old_input_pid == 0
        || old_input_session != 1
        || children.input_route_epoch != 1
    {
        fail(FAIL_RUNTIME);
    }

    let old_surface_identity = service_identity(ServiceKind::SurfaceServer, old_surface_pid);
    let old_input_identity = service_identity(ServiceKind::InputServer, old_input_pid);
    let policy = ServicePolicy::new(1, INPUT_RESTART_BACKOFF_NS, SERVICE_HEALTH_TIMEOUT_NS)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let mut service_supervisor = ServiceSupervisor::<2>::new();
    service_supervisor
        .register(old_surface_identity, policy)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    service_supervisor
        .register(old_input_identity, policy)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    service_supervisor
        .add_dependency(
            ServiceKind::InputServer,
            ServiceKind::SurfaceServer,
            DependencyKind::Soft,
        )
        .unwrap_or_else(|_| fail(FAIL_DEPENDENCY_TRANSITION));

    let armed = read_surface_recovery(surface_control.raw(), old_surface_pid);
    if armed.sender_sequence() != 1
        || armed.surface_session() != 1
        || armed.route_epoch() != 1
        || armed.payload()
            != (SurfaceRecoveryPayload::SurfaceStatus {
                checkpoint: SurfaceRecoveryCheckpoint::MultiWindowInteractive,
                related_physical_sequence: 0,
                phase: SurfaceRecoveryPhase::Armed,
            })
    {
        fail(FAIL_RUNTIME);
    }
    let requested = read_surface_recovery(surface_control.raw(), old_surface_pid);
    if requested.sender_sequence() != 2
        || requested.surface_session() != 1
        || requested.route_epoch() != 1
        || requested.payload()
            != (SurfaceRecoveryPayload::SurfaceStatus {
                checkpoint: SurfaceRecoveryCheckpoint::MultiWindowInteractive,
                related_physical_sequence: 2,
                phase: SurfaceRecoveryPhase::RestartRequested,
            })
    {
        fail(FAIL_RUNTIME);
    }

    terminate_and_wait_killed(old_surface_pid);
    let surface_fault = service_supervisor
        .classify_fault(old_surface_identity, FaultClass::ProcessExit)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if surface_fault
        != (FaultDisposition::RestartAllowed {
            attempt: 1,
            budget: 1,
            class: FaultClass::ProcessExit,
        })
    {
        fail(FAIL_RUNTIME);
    }
    let surface_impacts = service_supervisor
        .dependency_fault(old_surface_identity)
        .unwrap_or_else(|_| fail(FAIL_DEPENDENCY_TRANSITION));
    expect_dependency_transition(
        &surface_impacts,
        0,
        old_surface_identity,
        DependencyImpact::Unaffected,
        DependencyImpact::HardBlocked,
    );
    if surface_impacts.len() != 1 {
        fail(FAIL_DEPENDENCY_TRANSITION);
    }
    let surface_backoff_deadline = service_supervisor
        .begin_backoff(old_surface_identity, 0)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if surface_backoff_deadline != INPUT_RESTART_BACKOFF_NS {
        fail(FAIL_RUNTIME);
    }

    let lost = read_input_route_control(old_input_control.raw(), old_input_pid);
    let InputRouteControlPayload::RouteLost {
        physical_sequence_floor,
        capture,
    } = lost.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    let capture = capture.unwrap_or_else(|| fail(FAIL_RUNTIME));
    if lost.sequence() != 3
        || lost.route_epoch() != 1
        || lost.input_session_id() != old_input_session
        || physical_sequence_floor != 2
        || capture.owner_pid().get() != final_app.pid
        || capture.target().window_id().get() != 2
        || capture.target().generation() != 1
        || capture.trusted_overlay()
    {
        fail(FAIL_RUNTIME);
    }
    let queued = read_input_route_control(old_input_control.raw(), old_input_pid);
    let InputRouteControlPayload::GapStatus {
        physical_sequence_floor: queued_floor,
        metrics: queued_metrics,
    } = queued.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if queued.sequence() != 4
        || queued.route_epoch() != 2
        || queued.input_session_id() != old_input_session
        || queued_floor != 2
        || queued_metrics
            != InputRouteGapMetrics::try_new(1, 0, 1, 1, 0).unwrap_or_else(|_| fail(FAIL_RUNTIME))
    {
        fail(FAIL_RUNTIME);
    }
    require_service_timeout(
        &[old_input_control.raw()],
        INPUT_RESTART_BACKOFF_NS,
        FAIL_DEPENDENCY_SURFACE_BACKOFF,
    );
    service_supervisor
        .begin_replacement(old_surface_identity, surface_backoff_deadline)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));

    let (surface_startup, launcher_ui) = create_channel_owned();
    let (replacement_surface_control, replacement_surface_control_child) = create_channel_owned();
    let (surface_app_ui, app_ui) = create_channel_owned();
    write_bootstrap_transfer(
        launcher_ui.raw(),
        UiBootstrapEndpointKind::SurfaceSupervisor,
        replacement_surface_control_child,
    );
    let replacement_surface_pid = spawn_owned(surface_startup, UserImageId::SurfaceServer);
    require_next_process_generation(old_surface_pid, replacement_surface_pid);
    let replacement_surface_identity = service_supervisor
        .install_replacement(
            old_surface_identity,
            process_generation(replacement_surface_pid),
            replacement_surface_pid,
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));

    let (input_route_server, input_route_surface) = create_channel_owned();
    let surface_bind = InputRouteControl::bind(
        2,
        2,
        old_input_session,
        OwnerPid::try_new(replacement_surface_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        2,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire_transfer(
        old_input_control.raw(),
        &surface_bind.encode(),
        input_route_server,
    );
    let surface_ready = read_input_route_control(old_input_control.raw(), old_input_pid);
    let InputRouteControlPayload::Ready {
        bound_surface_pid,
        physical_sequence_floor: ready_floor,
    } = surface_ready.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if surface_ready.sequence() != 5
        || surface_ready.route_epoch() != 2
        || surface_ready.input_session_id() != old_input_session
        || bound_surface_pid.get() != replacement_surface_pid
        || ready_floor != 2
    {
        fail(FAIL_RUNTIME);
    }
    write_wire_transfer(
        launcher_ui.raw(),
        &INPUT_ROUTE_BOOTSTRAP_MAGIC.to_le_bytes(),
        input_route_surface,
    );
    write_bootstrap_transfer(
        launcher_ui.raw(),
        UiBootstrapEndpointKind::AppUi,
        surface_app_ui,
    );
    let bootstrap = SurfaceRecoveryMessage::bootstrap(
        1,
        2,
        2,
        old_input_pid,
        old_input_session,
        2,
        SurfaceRecoveryCheckpoint::MultiWindowInteractive,
        1,
        RecoveryCancelKind::ClientPointer,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(launcher_ui.raw(), &bootstrap.encode());

    let graph_prepared =
        read_surface_recovery(replacement_surface_control.raw(), replacement_surface_pid);
    if graph_prepared.sender_sequence() != 1
        || graph_prepared.surface_session() != 2
        || graph_prepared.route_epoch() != 2
        || graph_prepared.payload()
            != (SurfaceRecoveryPayload::SurfaceStatus {
                checkpoint: SurfaceRecoveryCheckpoint::MultiWindowInteractive,
                related_physical_sequence: 2,
                phase: SurfaceRecoveryPhase::GraphPrepared,
            })
    {
        fail(FAIL_RUNTIME);
    }
    write_bootstrap_transfer(
        launcher_lifecycle.raw(),
        UiBootstrapEndpointKind::LauncherUi,
        launcher_ui,
    );
    write_bootstrap_transfer(
        final_app.lifecycle.raw(),
        UiBootstrapEndpointKind::AppUi,
        app_ui,
    );

    let drained = read_input_route_control(old_input_control.raw(), old_input_pid);
    let InputRouteControlPayload::GapStatus {
        physical_sequence_floor: drained_floor,
        metrics: drained_metrics,
    } = drained.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if drained.sequence() != 6
        || drained.route_epoch() != 2
        || drained.input_session_id() != old_input_session
        || drained_floor != 3
        || drained_metrics
            != InputRouteGapMetrics::try_new(1, 1, 0, 1, 0).unwrap_or_else(|_| fail(FAIL_RUNTIME))
    {
        fail(FAIL_RUNTIME);
    }
    let active = read_surface_recovery(replacement_surface_control.raw(), replacement_surface_pid);
    if active.sender_sequence() != 2
        || active.surface_session() != 2
        || active.route_epoch() != 2
        || active.payload()
            != (SurfaceRecoveryPayload::SurfaceStatus {
                checkpoint: SurfaceRecoveryCheckpoint::MultiWindowInteractive,
                related_physical_sequence: 3,
                phase: SurfaceRecoveryPhase::Active,
            })
    {
        fail(FAIL_RUNTIME);
    }
    read_authenticated_magic(
        replacement_surface_control.raw(),
        replacement_surface_pid,
        INPUT_SURFACE_RESTART_READY_MAGIC,
    );
    let retired_surface_control =
        core::mem::replace(&mut surface_control, replacement_surface_control);
    close_owned(retired_surface_control);
    children.surface_server_pid = replacement_surface_pid;
    children.input_route_epoch = 2;

    // Keep the first recovered SurfaceServer frame observable as a distinct
    // stable boundary before the autonomous InputServer watchdog advances the
    // dependency graph. This is a real finite wait on the live SurfaceServer
    // control channel, not a kernel-side test delay.
    require_service_timeout(
        &[surface_control.raw()],
        SERVICE_RECOVERED_CADENCE_NS,
        FAIL_DEPENDENCY_SURFACE_RECOVERED_HOLD,
    );
    require_service_healthy(
        &mut service_supervisor,
        replacement_surface_identity,
        surface_control.raw(),
        replacement_surface_pid,
        surface_backoff_deadline,
    );
    let surface_recovered = service_supervisor
        .dependency_recovered(replacement_surface_identity)
        .unwrap_or_else(|_| fail(FAIL_DEPENDENCY_TRANSITION));
    expect_dependency_transition(
        &surface_recovered,
        0,
        replacement_surface_identity,
        DependencyImpact::HardBlocked,
        DependencyImpact::Unaffected,
    );
    if surface_recovered.len() != 1 {
        fail(FAIL_DEPENDENCY_TRANSITION);
    }

    let input_probe_now = surface_backoff_deadline;
    let input_probe = service_supervisor
        .arm_probe(old_input_identity, input_probe_now)
        .unwrap_or_else(|_| fail(FAIL_SERVICE_PROBE_ARM));
    validate_service_probe(input_probe, old_input_identity);
    if !write_service_probe(
        old_input_control.raw(),
        &input_probe.encode(),
        FAIL_SERVICE_PROBE_WRITE,
    ) {
        fail(FAIL_SERVICE_PROBE_WRITE);
    }
    require_service_timeout(
        &[old_input_control.raw()],
        SERVICE_HEALTH_TIMEOUT_NS,
        FAIL_SERVICE_HEALTH_WAIT,
    );
    let input_fault_now = input_probe_now
        .checked_add(SERVICE_HEALTH_TIMEOUT_NS)
        .unwrap_or_else(|| fail(FAIL_SERVICE_CADENCE_CLOCK));
    let input_fault = service_supervisor
        .check_health_timeout(old_input_identity, input_fault_now)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if input_fault
        != (FaultDisposition::RestartAllowed {
            attempt: 1,
            budget: 1,
            class: FaultClass::HealthTimeout,
        })
    {
        fail(FAIL_RUNTIME);
    }
    let input_impacts = service_supervisor
        .dependency_fault(old_input_identity)
        .unwrap_or_else(|_| fail(FAIL_DEPENDENCY_TRANSITION));
    expect_dependency_transition(
        &input_impacts,
        0,
        old_input_identity,
        DependencyImpact::Unaffected,
        DependencyImpact::HardBlocked,
    );
    expect_dependency_transition(
        &input_impacts,
        1,
        replacement_surface_identity,
        DependencyImpact::Unaffected,
        DependencyImpact::SoftDegraded,
    );
    if input_impacts.len() != 2 {
        fail(FAIL_DEPENDENCY_TRANSITION);
    }

    terminate_and_wait_killed(old_input_pid);
    let input_peer_closed = object_wait(old_input_control.raw(), ObjectSignals::PEER_CLOSED);
    if input_peer_closed.status != Status::Ok.raw()
        || input_peer_closed.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
        || input_peer_closed.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
    close_owned(old_input_control);

    let mut recovery_sequences = InputServiceRecoverySequenceTracker::new();
    let degraded = read_input_service_status(
        surface_control.raw(),
        replacement_surface_pid,
        &mut recovery_sequences,
    );
    expect_dependency_input_status(
        degraded,
        1,
        2,
        old_input_pid,
        1,
        3,
        InputServiceRecoveryPhase::RouteLost,
    );
    require_service_timeout(
        &[surface_control.raw()],
        SERVICE_RECOVERED_CADENCE_NS,
        FAIL_DEPENDENCY_DEGRADED_HOLD,
    );
    let input_backoff_now = input_fault_now
        .checked_add(SERVICE_RECOVERED_CADENCE_NS)
        .unwrap_or_else(|| fail(FAIL_SERVICE_CADENCE_CLOCK));
    let input_backoff_deadline = service_supervisor
        .begin_backoff(old_input_identity, input_backoff_now)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if input_backoff_deadline
        != input_backoff_now
            .checked_add(INPUT_RESTART_BACKOFF_NS)
            .unwrap_or_else(|| fail(FAIL_SERVICE_CADENCE_CLOCK))
    {
        fail(FAIL_RUNTIME);
    }
    require_service_timeout(
        &[surface_control.raw()],
        INPUT_RESTART_BACKOFF_NS,
        FAIL_DEPENDENCY_INPUT_BACKOFF,
    );
    service_supervisor
        .begin_replacement(old_input_identity, input_backoff_deadline)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));

    let (replacement_input_control, replacement_input_startup) = create_channel_owned();
    let replacement_input_pid = spawn_owned(replacement_input_startup, UserImageId::InputServer);
    require_next_process_generation(old_input_pid, replacement_input_pid);
    let acquired = read_input_route_control(replacement_input_control.raw(), replacement_input_pid);
    let InputRouteControlPayload::Acquired {
        physical_sequence_floor,
    } = acquired.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if acquired.sequence() != 1
        || acquired.route_epoch() != 0
        || acquired.input_session_id() != 2
        || physical_sequence_floor != 3
    {
        fail(FAIL_RUNTIME);
    }
    let replacement_input_identity = service_supervisor
        .install_replacement(
            old_input_identity,
            process_generation(replacement_input_pid),
            replacement_input_pid,
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));

    let (replacement_route_server, replacement_route_surface) = create_channel_owned();
    let input_bind = InputRouteControl::bind(
        2,
        3,
        2,
        OwnerPid::try_new(replacement_surface_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        3,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire_transfer(
        replacement_input_control.raw(),
        &input_bind.encode(),
        replacement_route_server,
    );
    let input_ready =
        read_input_route_control(replacement_input_control.raw(), replacement_input_pid);
    let InputRouteControlPayload::Ready {
        bound_surface_pid,
        physical_sequence_floor: input_ready_floor,
    } = input_ready.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if input_ready.sequence() != 2
        || input_ready.route_epoch() != 3
        || input_ready.input_session_id() != 2
        || bound_surface_pid.get() != replacement_surface_pid
        || input_ready_floor != 3
    {
        fail(FAIL_RUNTIME);
    }

    let offer = InputServiceRecoveryMessage::rebind_offer(
        1,
        2,
        3,
        OwnerPid::try_new(old_input_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        OwnerPid::try_new(replacement_input_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        2,
        3,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    recovery_sequences
        .accept(offer)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire_transfer(
        surface_control.raw(),
        &offer.encode(),
        replacement_route_surface,
    );

    let resync_prepared = read_input_service_status(
        surface_control.raw(),
        replacement_surface_pid,
        &mut recovery_sequences,
    );
    expect_dependency_input_status(
        resync_prepared,
        2,
        3,
        replacement_input_pid,
        2,
        3,
        InputServiceRecoveryPhase::ResyncPrepared,
    );
    let recovered = read_input_service_status(
        surface_control.raw(),
        replacement_surface_pid,
        &mut recovery_sequences,
    );
    expect_dependency_input_status(
        recovered,
        3,
        3,
        replacement_input_pid,
        2,
        3,
        InputServiceRecoveryPhase::Active,
    );

    require_service_healthy(
        &mut service_supervisor,
        replacement_input_identity,
        replacement_input_control.raw(),
        replacement_input_pid,
        input_backoff_deadline,
    );
    let input_recovered = service_supervisor
        .dependency_recovered(replacement_input_identity)
        .unwrap_or_else(|_| fail(FAIL_DEPENDENCY_TRANSITION));
    expect_dependency_transition(
        &input_recovered,
        0,
        replacement_input_identity,
        DependencyImpact::HardBlocked,
        DependencyImpact::Unaffected,
    );
    expect_dependency_transition(
        &input_recovered,
        1,
        replacement_surface_identity,
        DependencyImpact::SoftDegraded,
        DependencyImpact::Unaffected,
    );
    if input_recovered.len() != 2
        || service_supervisor
            .dependency_impact(ServiceKind::InputServer)
            .unwrap_or_else(|_| fail(FAIL_DEPENDENCY_TRANSITION))
            != DependencyImpact::Unaffected
        || service_supervisor
            .dependency_impact(ServiceKind::SurfaceServer)
            .unwrap_or_else(|_| fail(FAIL_DEPENDENCY_TRANSITION))
            != DependencyImpact::Unaffected
    {
        fail(FAIL_DEPENDENCY_TRANSITION);
    }
    require_dependency_service_snapshot(
        &service_supervisor,
        ServiceKind::SurfaceServer,
        replacement_surface_identity,
        FaultClass::ProcessExit,
        1,
    );
    require_dependency_service_snapshot(
        &service_supervisor,
        ServiceKind::InputServer,
        replacement_input_identity,
        FaultClass::HealthTimeout,
        1,
    );

    #[cfg(feature = "post-recovery-interaction-runtime")]
    {
        read_authenticated_magic(
            surface_control.raw(),
            replacement_surface_pid,
            POST_RECOVERY_INTERACTION_READY_MAGIC,
        );
        let post_recovery_probe_now = input_backoff_deadline
            .checked_add(SERVICE_HEALTH_TIMEOUT_NS)
            .unwrap_or_else(|| fail(FAIL_SERVICE_CADENCE_CLOCK));
        require_service_healthy(
            &mut service_supervisor,
            replacement_surface_identity,
            surface_control.raw(),
            replacement_surface_pid,
            post_recovery_probe_now,
        );
        require_service_healthy(
            &mut service_supervisor,
            replacement_input_identity,
            replacement_input_control.raw(),
            replacement_input_pid,
            post_recovery_probe_now,
        );
        require_dependency_service_snapshot(
            &service_supervisor,
            ServiceKind::SurfaceServer,
            replacement_surface_identity,
            FaultClass::ProcessExit,
            2,
        );
        require_dependency_service_snapshot(
            &service_supervisor,
            ServiceKind::InputServer,
            replacement_input_identity,
            FaultClass::HealthTimeout,
            2,
        );
    }
    #[cfg(not(feature = "post-recovery-interaction-runtime"))]
    require_service_timeout(
        &[surface_control.raw(), replacement_input_control.raw()],
        SERVICE_RECOVERED_CADENCE_NS,
        FAIL_DEPENDENCY_FINAL_CADENCE,
    );
    children.input_server_pid = replacement_input_pid;
    children.input_server_session = 2;
    children.input_route_epoch = 3;
    children.input_server_control = Some(replacement_input_control);
    supervise_runtime(
        children,
        surface_control,
        launcher_lifecycle,
        final_app.lifecycle,
    )
}

#[cfg(feature = "service-dependency-runtime")]
fn service_identity(kind: ServiceKind, pid: u64) -> ServiceIdentity {
    ServiceIdentity::new(kind, process_generation(pid), pid).unwrap_or_else(|_| fail(FAIL_RUNTIME))
}

#[cfg(feature = "service-dependency-runtime")]
fn require_next_process_generation(previous: u64, replacement: u64) {
    if process_slot(replacement) != process_slot(previous)
        || process_generation(replacement)
            != process_generation(previous)
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_RUNTIME))
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(feature = "service-dependency-runtime")]
fn expect_dependency_transition(
    transitions: &bndr_sm::health::DependencyTransitions<2>,
    index: usize,
    service: ServiceIdentity,
    previous: DependencyImpact,
    current: DependencyImpact,
) {
    let transition = transitions
        .get(index)
        .unwrap_or_else(|| fail(FAIL_DEPENDENCY_TRANSITION));
    if transition.service != service
        || transition.previous != previous
        || transition.current != current
    {
        fail(FAIL_DEPENDENCY_TRANSITION);
    }
}

#[cfg(feature = "service-dependency-runtime")]
fn require_service_timeout(handles: &[u64], timeout_ns: u64, failure: u64) {
    if !matches!(
        wait_service_controls(handles, timeout_ns, failure),
        ServiceWaitOutcome::Timeout
    ) {
        fail(failure);
    }
}

#[cfg(feature = "service-dependency-runtime")]
fn validate_service_probe(probe: HealthFrame, identity: ServiceIdentity) {
    if probe.opcode() != HealthOpcode::Probe
        || probe.identity() != identity
        || probe.interval_ns() != SERVICE_HEALTH_TIMEOUT_NS
    {
        fail(FAIL_SERVICE_PROBE_FRAME);
    }
}

#[cfg(feature = "service-dependency-runtime")]
fn require_service_healthy(
    supervisor: &mut ServiceSupervisor<2>,
    identity: ServiceIdentity,
    transport: u64,
    expected_sender: u64,
    now_ns: u64,
) {
    let probe = supervisor
        .arm_probe(identity, now_ns)
        .unwrap_or_else(|_| fail(FAIL_SERVICE_PROBE_ARM));
    validate_service_probe(probe, identity);
    if !write_service_probe(transport, &probe.encode(), FAIL_SERVICE_PROBE_WRITE) {
        fail(FAIL_SERVICE_PROBE_WRITE);
    }
    let response = wait_service_controls(
        &[transport],
        SERVICE_HEALTH_TIMEOUT_NS,
        FAIL_SERVICE_HEALTH_WAIT,
    );
    // Resolve the inclusive deadline race without extending the health
    // budget. A responder can publish Healthy on the same timer edge that
    // completes the finite wait; in that case the timeout result and a queued
    // message are both truthful. One zero-duration poll accepts only the
    // response already visible at that boundary. A genuinely late or missing
    // response still fails immediately.
    let response = match response {
        ServiceWaitOutcome::Ready { index, signals } => {
            ServiceWaitOutcome::Ready { index, signals }
        }
        ServiceWaitOutcome::Timeout => {
            wait_service_controls(&[transport], 0, FAIL_SERVICE_HEALTH_WAIT)
        }
    };
    let ServiceWaitOutcome::Ready { index: 0, signals } = response else {
        fail(FAIL_SERVICE_HEALTH_OUTCOME);
    };
    if signals & u64::from(ObjectSignals::READABLE.bits()) == 0
        || signals & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
    {
        fail(FAIL_SERVICE_HEALTH_OUTCOME);
    }
    let healthy = read_service_health(transport, expected_sender);
    if healthy.opcode() != HealthOpcode::Healthy
        || healthy.sequence() != probe.sequence()
        || healthy.identity() != identity
        || healthy.fault_class().is_some()
    {
        fail(FAIL_SERVICE_HEALTH_OUTCOME);
    }
    supervisor
        .record_healthy(healthy)
        .unwrap_or_else(|_| fail(FAIL_SERVICE_HEALTH_OUTCOME));
}

#[cfg(feature = "service-dependency-runtime")]
#[allow(clippy::too_many_arguments)]
fn expect_dependency_input_status(
    message: InputServiceRecoveryMessage,
    sequence: u64,
    route_epoch: u64,
    input_pid: u64,
    input_session_id: u64,
    physical_sequence_floor: u64,
    phase: InputServiceRecoveryPhase,
) {
    if message.sequence() != sequence
        || message.surface_session() != 2
        || message.route_epoch() != route_epoch
        || message.payload()
            != (InputServiceRecoveryPayload::SurfaceStatus {
                input_pid: OwnerPid::try_new(input_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                input_session_id,
                physical_sequence_floor,
                phase,
                route_count: 2,
                focus: InputServiceFocus::App,
                capture_active: false,
                text_active: false,
            })
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(feature = "service-dependency-runtime")]
fn require_dependency_service_snapshot(
    supervisor: &ServiceSupervisor<2>,
    kind: ServiceKind,
    identity: ServiceIdentity,
    last_fault: FaultClass,
    expected_health_sequence: u64,
) {
    let snapshot = supervisor
        .service(kind)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if snapshot.identity != identity
        || snapshot.phase != ServicePhase::Healthy
        || snapshot.restarts_used != 1
        || snapshot.probe_deadline_ns.is_some()
        || snapshot.backoff_deadline_ns.is_some()
        || snapshot.last_fault != Some(last_fault)
        || snapshot.last_outbound_sequence != expected_health_sequence
        || snapshot.last_inbound_sequence != expected_health_sequence
        || snapshot.policy.restart_budget() != 1
        || snapshot.policy.restart_backoff_ns() != INPUT_RESTART_BACKOFF_NS
        || snapshot.policy.health_timeout_ns() != SERVICE_HEALTH_TIMEOUT_NS
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(all(
    feature = "input-server-surface-restart-runtime",
    not(feature = "input-server-restart-runtime"),
    not(feature = "service-dependency-runtime")
))]
fn finish_input_server_surface_restart_runtime(
    children: &mut ResidentChildren,
    mut surface_control: OwnedUserHandle,
    surface_pid: u64,
    launcher_lifecycle: OwnedUserHandle,
    launcher_pid: u64,
    final_app: InitAppGeneration,
) -> ! {
    let old_surface_pid = surface_pid;
    let input_server_pid = children.input_server_pid;
    let input_server_session = children.input_server_session;
    let input_control = children
        .input_server_control
        .as_ref()
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if old_surface_pid == 0
        || launcher_pid == 0
        || final_app.pid == 0
        || input_server_pid == 0
        || input_server_session == 0
        || children.input_route_epoch != 1
    {
        fail(FAIL_RUNTIME);
    }

    let armed = read_surface_recovery(surface_control.raw(), old_surface_pid);
    if armed.sender_sequence() != 1
        || armed.surface_session() != 1
        || armed.route_epoch() != 1
        || armed.payload()
            != (SurfaceRecoveryPayload::SurfaceStatus {
                checkpoint: SurfaceRecoveryCheckpoint::MultiWindowInteractive,
                related_physical_sequence: 0,
                phase: SurfaceRecoveryPhase::Armed,
            })
    {
        fail(FAIL_RUNTIME);
    }

    // The checker moves the tablet to the retained App point (physical #1)
    // and presses it (physical #2). Surface publishes RestartRequested only
    // after InputServer and its compositor both hold the App capture.
    let requested = read_surface_recovery(surface_control.raw(), old_surface_pid);
    if requested.sender_sequence() != 2
        || requested.surface_session() != 1
        || requested.route_epoch() != 1
        || requested.payload()
            != (SurfaceRecoveryPayload::SurfaceStatus {
                checkpoint: SurfaceRecoveryCheckpoint::MultiWindowInteractive,
                related_physical_sequence: 2,
                phase: SurfaceRecoveryPhase::RestartRequested,
            })
    {
        fail(FAIL_RUNTIME);
    }

    terminate_and_wait_killed(old_surface_pid);
    let lost = read_input_route_control(input_control.raw(), input_server_pid);
    let InputRouteControlPayload::RouteLost {
        physical_sequence_floor,
        capture,
    } = lost.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    let capture = capture.unwrap_or_else(|| fail(FAIL_RUNTIME));
    if lost.sequence() != 3
        || lost.route_epoch() != 1
        || lost.input_session_id() != input_server_session
        || physical_sequence_floor != 2
        || capture.owner_pid().get() != final_app.pid
        || capture.target().window_id().get() != 2
        || capture.target().generation() != 1
        || capture.trusted_overlay()
    {
        fail(FAIL_RUNTIME);
    }

    // Init intentionally does not create the replacement until the checker
    // observes the old-generation death and injects the release. This makes
    // the route gap an externally witnessed state, not a scheduling accident.
    let queued = read_input_route_control(input_control.raw(), input_server_pid);
    let InputRouteControlPayload::GapStatus {
        physical_sequence_floor: queued_floor,
        metrics: queued_metrics,
    } = queued.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if queued.sequence() != 4
        || queued.route_epoch() != 2
        || queued.input_session_id() != input_server_session
        || queued_floor != 2
        || queued_metrics
            != InputRouteGapMetrics::try_new(1, 0, 1, 1, 0).unwrap_or_else(|_| fail(FAIL_RUNTIME))
    {
        fail(FAIL_RUNTIME);
    }

    let (surface_startup, launcher_ui) = create_channel_owned();
    let (replacement_control, replacement_control_child) = create_channel_owned();
    let (surface_app_ui, app_ui) = create_channel_owned();
    write_bootstrap_transfer(
        launcher_ui.raw(),
        UiBootstrapEndpointKind::SurfaceSupervisor,
        replacement_control_child,
    );
    let replacement_pid = spawn_owned(surface_startup, UserImageId::SurfaceServer);
    if process_slot(replacement_pid) != process_slot(old_surface_pid)
        || process_generation(replacement_pid)
            != process_generation(old_surface_pid)
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_RUNTIME))
    {
        fail(FAIL_RUNTIME);
    }

    let (input_route_server, input_route_surface) = create_channel_owned();
    let bind = InputRouteControl::bind(
        2,
        2,
        input_server_session,
        OwnerPid::try_new(replacement_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        2,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire_transfer(input_control.raw(), &bind.encode(), input_route_server);
    let ready = read_input_route_control(input_control.raw(), input_server_pid);
    let InputRouteControlPayload::Ready {
        bound_surface_pid,
        physical_sequence_floor: ready_floor,
    } = ready.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if ready.sequence() != 5
        || ready.route_epoch() != 2
        || ready.input_session_id() != input_server_session
        || bound_surface_pid.get() != replacement_pid
        || ready_floor != 2
    {
        fail(FAIL_RUNTIME);
    }
    write_wire_transfer(
        launcher_ui.raw(),
        &INPUT_ROUTE_BOOTSTRAP_MAGIC.to_le_bytes(),
        input_route_surface,
    );
    write_bootstrap_transfer(
        launcher_ui.raw(),
        UiBootstrapEndpointKind::AppUi,
        surface_app_ui,
    );
    let bootstrap = SurfaceRecoveryMessage::bootstrap(
        1,
        2,
        2,
        input_server_pid,
        input_server_session,
        2,
        SurfaceRecoveryCheckpoint::MultiWindowInteractive,
        1,
        RecoveryCancelKind::ClientPointer,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(launcher_ui.raw(), &bootstrap.encode());

    let graph_prepared = read_surface_recovery(replacement_control.raw(), replacement_pid);
    if graph_prepared.sender_sequence() != 1
        || graph_prepared.surface_session() != 2
        || graph_prepared.route_epoch() != 2
        || graph_prepared.payload()
            != (SurfaceRecoveryPayload::SurfaceStatus {
                checkpoint: SurfaceRecoveryCheckpoint::MultiWindowInteractive,
                related_physical_sequence: 2,
                phase: SurfaceRecoveryPhase::GraphPrepared,
            })
    {
        fail(FAIL_RUNTIME);
    }

    write_bootstrap_transfer(
        launcher_lifecycle.raw(),
        UiBootstrapEndpointKind::LauncherUi,
        launcher_ui,
    );
    write_bootstrap_transfer(
        final_app.lifecycle.raw(),
        UiBootstrapEndpointKind::AppUi,
        app_ui,
    );

    let drained = read_input_route_control(input_control.raw(), input_server_pid);
    let InputRouteControlPayload::GapStatus {
        physical_sequence_floor: drained_floor,
        metrics: drained_metrics,
    } = drained.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if drained.sequence() != 6
        || drained.route_epoch() != 2
        || drained.input_session_id() != input_server_session
        || drained_floor != 3
        || drained_metrics
            != InputRouteGapMetrics::try_new(1, 1, 0, 1, 0).unwrap_or_else(|_| fail(FAIL_RUNTIME))
    {
        fail(FAIL_RUNTIME);
    }
    let active = read_surface_recovery(replacement_control.raw(), replacement_pid);
    if active.sender_sequence() != 2
        || active.surface_session() != 2
        || active.route_epoch() != 2
        || active.payload()
            != (SurfaceRecoveryPayload::SurfaceStatus {
                checkpoint: SurfaceRecoveryCheckpoint::MultiWindowInteractive,
                related_physical_sequence: 3,
                phase: SurfaceRecoveryPhase::Active,
            })
    {
        fail(FAIL_RUNTIME);
    }

    let old_control = core::mem::replace(&mut surface_control, replacement_control);
    close_owned(old_control);
    children.surface_server_pid = replacement_pid;
    children.input_route_epoch = 2;

    read_authenticated_magic(
        surface_control.raw(),
        replacement_pid,
        INPUT_SURFACE_RESTART_READY_MAGIC,
    );
    supervise_runtime(
        children,
        surface_control,
        launcher_lifecycle,
        final_app.lifecycle,
    )
}

#[cfg(all(
    feature = "input-server-restart-runtime",
    not(feature = "service-dependency-runtime")
))]
fn finish_input_server_restart_runtime(
    children: &mut ResidentChildren,
    surface_control: OwnedUserHandle,
    surface_pid: u64,
    launcher_lifecycle: OwnedUserHandle,
    launcher_pid: u64,
    final_app: InitAppGeneration,
) -> ! {
    let old_input_pid = children.input_server_pid;
    let old_input_session = children.input_server_session;
    let old_input_control = children
        .input_server_control
        .take()
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if surface_pid == 0
        || surface_pid != children.surface_server_pid
        || launcher_pid == 0
        || launcher_pid != children.launcher_pid
        || final_app.pid == 0
        || final_app.pid != children.app_pid
        || old_input_pid == 0
        || old_input_session != 1
        || children.input_route_epoch != 1
    {
        fail(FAIL_RUNTIME);
    }

    #[cfg(feature = "service-supervisor-runtime")]
    let old_service_identity = ServiceIdentity::new(
        ServiceKind::InputServer,
        process_generation(old_input_pid),
        old_input_pid,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    #[cfg(feature = "service-supervisor-runtime")]
    let service_policy = ServicePolicy::new(1, INPUT_RESTART_BACKOFF_NS, SERVICE_HEALTH_TIMEOUT_NS)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    #[cfg(feature = "service-supervisor-runtime")]
    let mut service_supervisor = {
        let mut supervisor = ServiceSupervisor::<1>::new();
        supervisor
            .register(old_service_identity, service_policy)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        supervisor
    };

    let mut recovery_sequences = InputServiceRecoverySequenceTracker::new();
    let armed =
        read_input_service_status(surface_control.raw(), surface_pid, &mut recovery_sequences);
    expect_input_service_status(
        armed,
        1,
        1,
        old_input_pid,
        1,
        0,
        InputServiceRecoveryPhase::Armed,
        InputServiceFocus::Launcher,
    );

    let restart_requested =
        read_input_service_status(surface_control.raw(), surface_pid, &mut recovery_sequences);
    expect_input_service_status(
        restart_requested,
        2,
        1,
        old_input_pid,
        1,
        1,
        InputServiceRecoveryPhase::RestartRequested,
        InputServiceFocus::Launcher,
    );

    let old_generation = InputServiceGeneration::try_new(
        OwnerPid::try_new(old_input_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        1,
        1,
        1,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let mut restart_policy =
        InputRestartPolicy::try_new(old_generation).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    terminate_and_wait_killed(old_input_pid);
    let peer_closed = object_wait(old_input_control.raw(), ObjectSignals::PEER_CLOSED);
    if peer_closed.status != Status::Ok.raw()
        || peer_closed.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
        || peer_closed.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
    restart_policy
        .active_fault(old_generation)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if restart_policy.snapshot().state() != InputRestartState::Backoff
        || restart_policy.snapshot().attempts_used() != 1
    {
        fail(FAIL_RUNTIME);
    }
    close_owned(old_input_control);

    let route_lost =
        read_input_service_status(surface_control.raw(), surface_pid, &mut recovery_sequences);
    expect_input_service_status(
        route_lost,
        3,
        1,
        old_input_pid,
        1,
        1,
        InputServiceRecoveryPhase::RouteLost,
        InputServiceFocus::Launcher,
    );

    #[cfg(feature = "service-supervisor-runtime")]
    {
        let fault = read_service_health(surface_control.raw(), surface_pid);
        if fault.opcode() != HealthOpcode::Fault
            || fault.sequence() != 1
            || fault.identity() != old_service_identity
            || fault.fault_class() != Some(FaultClass::ProcessExit)
        {
            fail(FAIL_RUNTIME);
        }
        let disposition = service_supervisor
            .record_fault(fault)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        if disposition
            != (FaultDisposition::RestartAllowed {
                attempt: 1,
                budget: 1,
                class: FaultClass::ProcessExit,
            })
            || service_supervisor
                .begin_backoff(old_service_identity, 0)
                .unwrap_or_else(|_| fail(FAIL_RUNTIME))
                != INPUT_RESTART_BACKOFF_NS
        {
            fail(FAIL_RUNTIME);
        }
    }

    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    items[0] = pack_user_wait_item(surface_control.raw(), requested);
    let backoff = object_wait_many_array(&items, 1, INPUT_RESTART_BACKOFF_NS);
    if backoff.status != Status::Timeout.raw() || backoff.out1 != u64::MAX || backoff.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    restart_policy
        .backoff_elapsed(INPUT_RESTART_BACKOFF_NS)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    #[cfg(feature = "service-supervisor-runtime")]
    service_supervisor
        .begin_replacement(old_service_identity, INPUT_RESTART_BACKOFF_NS)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let authorization = restart_policy
        .replacement_authorization()
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if authorization.attempt() != 1
        || authorization.failed_generation() != old_generation
        || authorization.required_input_session_id() != 2
        || authorization.required_route_epoch() != 2
        || authorization.minimum_physical_sequence_floor() != 1
    {
        fail(FAIL_RUNTIME);
    }

    let (replacement_control, replacement_startup) = create_channel_owned();
    let replacement_pid = spawn_owned(replacement_startup, UserImageId::InputServer);
    if process_slot(replacement_pid) != process_slot(old_input_pid)
        || process_generation(replacement_pid)
            != process_generation(old_input_pid)
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_RUNTIME))
    {
        fail(FAIL_RUNTIME);
    }
    #[cfg(feature = "service-supervisor-runtime")]
    let replacement_service_identity = service_supervisor
        .install_replacement(
            old_service_identity,
            process_generation(replacement_pid),
            replacement_pid,
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    #[cfg(feature = "service-supervisor-runtime")]
    if replacement_service_identity
        != ServiceIdentity::new(
            ServiceKind::InputServer,
            process_generation(replacement_pid),
            replacement_pid,
        )
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
    {
        fail(FAIL_RUNTIME);
    }
    let acquired = read_input_route_control(replacement_control.raw(), replacement_pid);
    let InputRouteControlPayload::Acquired {
        physical_sequence_floor,
    } = acquired.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if acquired.sequence() != 1
        || acquired.route_epoch() != 0
        || acquired.input_session_id() != 2
        || physical_sequence_floor != 1
    {
        fail(FAIL_RUNTIME);
    }
    let replacement_generation = InputServiceGeneration::try_new(
        OwnerPid::try_new(replacement_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        2,
        2,
        1,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    restart_policy
        .begin_replacement(replacement_generation)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));

    let (route_server, route_surface) = create_channel_owned();
    let bind = InputRouteControl::bind(
        2,
        2,
        2,
        OwnerPid::try_new(surface_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        1,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire_transfer(replacement_control.raw(), &bind.encode(), route_server);
    let ready = read_input_route_control(replacement_control.raw(), replacement_pid);
    let InputRouteControlPayload::Ready {
        bound_surface_pid,
        physical_sequence_floor: ready_floor,
    } = ready.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if ready.sequence() != 2
        || ready.route_epoch() != 2
        || ready.input_session_id() != 2
        || bound_surface_pid.get() != surface_pid
        || ready_floor != 1
    {
        fail(FAIL_RUNTIME);
    }

    let offer = InputServiceRecoveryMessage::rebind_offer(
        1,
        1,
        2,
        OwnerPid::try_new(old_input_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        OwnerPid::try_new(replacement_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        2,
        1,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    recovery_sequences
        .accept(offer)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire_transfer(surface_control.raw(), &offer.encode(), route_surface);

    let resync_prepared =
        read_input_service_status(surface_control.raw(), surface_pid, &mut recovery_sequences);
    expect_input_service_status(
        resync_prepared,
        4,
        2,
        replacement_pid,
        2,
        1,
        InputServiceRecoveryPhase::ResyncPrepared,
        InputServiceFocus::Launcher,
    );
    restart_policy
        .resync_succeeded(replacement_generation)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));

    let active =
        read_input_service_status(surface_control.raw(), surface_pid, &mut recovery_sequences);
    expect_input_service_status(
        active,
        5,
        2,
        replacement_pid,
        2,
        3,
        InputServiceRecoveryPhase::Active,
        InputServiceFocus::App,
    );
    let final_policy = restart_policy.snapshot();
    if final_policy.state() != InputRestartState::Active
        || final_policy.current() != replacement_generation
        || final_policy.candidate().is_some()
        || final_policy.attempts_used() != 1
        || final_policy.budget_remaining() != 0
        || final_policy.required_backoff_ns() != 0
        || final_policy.authorization().is_some()
        || final_policy.quarantine_reason().is_some()
    {
        fail(FAIL_RUNTIME);
    }

    #[cfg(feature = "service-supervisor-runtime")]
    finish_service_supervisor_runtime(
        children,
        surface_control,
        surface_pid,
        launcher_lifecycle,
        final_app,
        replacement_control,
        replacement_pid,
        replacement_generation,
        replacement_service_identity,
        restart_policy,
        service_supervisor,
    );

    #[cfg(not(feature = "service-supervisor-runtime"))]
    {
        children.input_server_pid = replacement_pid;
        children.input_server_session = 2;
        children.input_route_epoch = 2;
        children.input_server_control = Some(replacement_control);
        supervise_runtime(
            children,
            surface_control,
            launcher_lifecycle,
            final_app.lifecycle,
        )
    }
}

#[cfg(feature = "service-supervisor-runtime")]
#[allow(clippy::too_many_arguments)]
fn finish_service_supervisor_runtime(
    children: &mut ResidentChildren,
    surface_control: OwnedUserHandle,
    surface_pid: u64,
    launcher_lifecycle: OwnedUserHandle,
    final_app: InitAppGeneration,
    replacement_control: OwnedUserHandle,
    replacement_pid: u64,
    replacement_generation: InputServiceGeneration,
    replacement_identity: ServiceIdentity,
    input_restart_policy: InputRestartPolicy,
    mut supervisor: ServiceSupervisor<1>,
) -> ! {
    let mut probe_now_ns = 0_u64;
    loop {
        let probe = supervisor
            .arm_probe(replacement_identity, probe_now_ns)
            .unwrap_or_else(|_| fail(FAIL_SERVICE_PROBE_ARM));
        if probe.opcode() != HealthOpcode::Probe
            || probe.identity() != replacement_identity
            || probe.interval_ns() != SERVICE_HEALTH_TIMEOUT_NS
        {
            fail(FAIL_SERVICE_PROBE_FRAME);
        }
        if !write_service_probe(
            replacement_control.raw(),
            &probe.encode(),
            FAIL_SERVICE_PROBE_WRITE,
        ) {
            require_terminal_service_fault(
                supervisor
                    .classify_fault(replacement_identity, FaultClass::ProcessExit)
                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                FaultClass::ProcessExit,
            );
            finish_service_quarantine(
                children,
                surface_control,
                surface_pid,
                launcher_lifecycle,
                final_app,
                replacement_control,
                replacement_pid,
                replacement_generation,
                replacement_identity,
                input_restart_policy,
                supervisor,
                FaultClass::ProcessExit,
            );
        }

        match wait_service_controls(
            &[replacement_control.raw()],
            SERVICE_HEALTH_TIMEOUT_NS,
            FAIL_SERVICE_HEALTH_WAIT,
        ) {
            ServiceWaitOutcome::Timeout => {
                let deadline = probe_now_ns
                    .checked_add(SERVICE_HEALTH_TIMEOUT_NS)
                    .unwrap_or_else(|| fail(FAIL_RUNTIME));
                let disposition = supervisor
                    .check_health_timeout(replacement_identity, deadline)
                    .unwrap_or_else(|_| fail(FAIL_RUNTIME))
                    .unwrap_or_else(|| fail(FAIL_RUNTIME));
                require_terminal_service_fault(disposition, FaultClass::HealthTimeout);
                finish_service_quarantine(
                    children,
                    surface_control,
                    surface_pid,
                    launcher_lifecycle,
                    final_app,
                    replacement_control,
                    replacement_pid,
                    replacement_generation,
                    replacement_identity,
                    input_restart_policy,
                    supervisor,
                    FaultClass::HealthTimeout,
                );
            }
            ServiceWaitOutcome::Ready { index: 0, signals }
                if signals & u64::from(ObjectSignals::READABLE.bits()) != 0 =>
            {
                let accepted = try_read_service_health(replacement_control.raw(), replacement_pid)
                    .is_some_and(|healthy| {
                        healthy.opcode() == HealthOpcode::Healthy
                            && healthy.sequence() == probe.sequence()
                            && healthy.identity() == replacement_identity
                            && healthy.fault_class().is_none()
                            && supervisor.record_healthy(healthy).is_ok()
                    });
                if !accepted {
                    require_terminal_service_fault(
                        supervisor
                            .classify_fault(replacement_identity, FaultClass::ProtocolViolation)
                            .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                        FaultClass::ProtocolViolation,
                    );
                    finish_service_quarantine(
                        children,
                        surface_control,
                        surface_pid,
                        launcher_lifecycle,
                        final_app,
                        replacement_control,
                        replacement_pid,
                        replacement_generation,
                        replacement_identity,
                        input_restart_policy,
                        supervisor,
                        FaultClass::ProtocolViolation,
                    );
                }
            }
            ServiceWaitOutcome::Ready { index: 0, signals }
                if signals & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 =>
            {
                require_terminal_service_fault(
                    supervisor
                        .classify_fault(replacement_identity, FaultClass::ProcessExit)
                        .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                    FaultClass::ProcessExit,
                );
                finish_service_quarantine(
                    children,
                    surface_control,
                    surface_pid,
                    launcher_lifecycle,
                    final_app,
                    replacement_control,
                    replacement_pid,
                    replacement_generation,
                    replacement_identity,
                    input_restart_policy,
                    supervisor,
                    FaultClass::ProcessExit,
                );
            }
            _ => fail(FAIL_SERVICE_HEALTH_OUTCOME),
        }

        match wait_service_controls(
            &[surface_control.raw(), replacement_control.raw()],
            SERVICE_RECOVERED_CADENCE_NS,
            FAIL_SERVICE_CADENCE_WAIT,
        ) {
            ServiceWaitOutcome::Timeout => {}
            ServiceWaitOutcome::Ready { index: 1, signals } => {
                let class = if signals & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
                    FaultClass::ProcessExit
                } else {
                    FaultClass::ProtocolViolation
                };
                require_terminal_service_fault(
                    supervisor
                        .classify_fault(replacement_identity, class)
                        .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                    class,
                );
                finish_service_quarantine(
                    children,
                    surface_control,
                    surface_pid,
                    launcher_lifecycle,
                    final_app,
                    replacement_control,
                    replacement_pid,
                    replacement_generation,
                    replacement_identity,
                    input_restart_policy,
                    supervisor,
                    class,
                );
            }
            _ => fail(FAIL_SERVICE_CADENCE_OUTCOME),
        }
        probe_now_ns = probe_now_ns
            .checked_add(SERVICE_RECOVERED_CADENCE_NS)
            .unwrap_or_else(|| fail(FAIL_SERVICE_CADENCE_CLOCK));
    }
}

#[cfg(feature = "service-supervisor-runtime")]
#[allow(clippy::too_many_arguments)]
fn finish_service_quarantine(
    children: &mut ResidentChildren,
    surface_control: OwnedUserHandle,
    surface_pid: u64,
    launcher_lifecycle: OwnedUserHandle,
    final_app: InitAppGeneration,
    replacement_control: OwnedUserHandle,
    replacement_pid: u64,
    replacement_generation: InputServiceGeneration,
    replacement_identity: ServiceIdentity,
    mut input_restart_policy: InputRestartPolicy,
    mut supervisor: ServiceSupervisor<1>,
    expected_fault: FaultClass,
) -> ! {
    let pending = supervisor
        .service(ServiceKind::InputServer)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if pending.identity != replacement_identity
        || pending.phase != ServicePhase::QuarantinePending
        || pending.restarts_used != 1
        || pending.probe_deadline_ns.is_some()
        || pending.backoff_deadline_ns.is_some()
        || pending.last_fault != Some(expected_fault)
    {
        fail(FAIL_RUNTIME);
    }

    terminate_and_reap_supervised_process(replacement_pid);
    input_restart_policy
        .active_fault(replacement_generation)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let quarantined_input = input_restart_policy.snapshot();
    if quarantined_input.state() != InputRestartState::Quarantined
        || quarantined_input.current() != replacement_generation
        || quarantined_input.candidate().is_some()
        || quarantined_input.attempts_used() != 1
        || quarantined_input.budget_remaining() != 0
        || quarantined_input.required_backoff_ns() != 0
        || quarantined_input.authorization().is_some()
        || quarantined_input.quarantine_reason()
            != Some(InputQuarantineReason::ActiveFaultBudgetExhausted)
    {
        fail(FAIL_RUNTIME);
    }
    let peer_closed = object_wait(replacement_control.raw(), ObjectSignals::PEER_CLOSED);
    if peer_closed.status != Status::Ok.raw()
        || peer_closed.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
        || peer_closed.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
    close_owned(replacement_control);

    let quarantine = supervisor
        .quarantine(replacement_identity)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if quarantine.opcode() != HealthOpcode::Quarantine
        || quarantine.sequence() != 3
        || quarantine.identity() != replacement_identity
        || quarantine.fault_class() != Some(FaultClass::RestartBudgetExhausted)
        || quarantine.restart_attempt() != 2
        || quarantine.restart_budget() != 1
        || quarantine.sequence() < 2
        || quarantine.sequence() > 3
    {
        fail(FAIL_RUNTIME);
    }
    write_wire(surface_control.raw(), &quarantine.encode());

    let degraded = read_service_health(surface_control.raw(), surface_pid);
    if degraded.opcode() != HealthOpcode::DegradedAck
        || degraded.sequence() != 2
        || degraded.identity() != replacement_identity
        || degraded.fault_class().is_some()
    {
        fail(FAIL_RUNTIME);
    }
    supervisor
        .acknowledge_degraded(degraded)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let final_snapshot = supervisor
        .service(ServiceKind::InputServer)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if final_snapshot.identity != replacement_identity
        || final_snapshot.phase != ServicePhase::Degraded
        || final_snapshot.restarts_used != 1
        || final_snapshot.probe_deadline_ns.is_some()
        || final_snapshot.backoff_deadline_ns.is_some()
        || final_snapshot.last_fault != Some(expected_fault)
        || final_snapshot.last_outbound_sequence != quarantine.sequence()
        || final_snapshot.last_inbound_sequence != 2
        || final_snapshot.policy.restart_budget() != 1
        || final_snapshot.policy.restart_backoff_ns() != INPUT_RESTART_BACKOFF_NS
        || final_snapshot.policy.health_timeout_ns() != SERVICE_HEALTH_TIMEOUT_NS
    {
        fail(FAIL_RUNTIME);
    }

    children.input_server_pid = 0;
    children.input_server_session = 2;
    children.input_route_epoch = 2;
    if children.input_server_control.is_some() {
        fail(FAIL_RUNTIME);
    }
    supervise_runtime(
        children,
        surface_control,
        launcher_lifecycle,
        final_app.lifecycle,
    )
}

#[cfg(feature = "service-supervisor-runtime")]
fn require_terminal_service_fault(disposition: FaultDisposition, class: FaultClass) {
    if disposition
        != (FaultDisposition::QuarantineRequired {
            attempt: 2,
            budget: 1,
            class,
        })
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
enum ServiceWaitOutcome {
    Timeout,
    Ready { index: usize, signals: u64 },
}

#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
fn wait_service_controls(handles: &[u64], timeout_ns: u64, failure: u64) -> ServiceWaitOutcome {
    if handles.is_empty() || handles.len() > OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS {
        fail(failure);
    }
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let requested_bits = u64::from(requested.bits());
    let channel_bits = u64::from(ObjectSignals::CHANNEL_ALL.bits());
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    for (index, handle) in handles.iter().copied().enumerate() {
        items[index] = pack_user_wait_item(handle, requested);
    }
    let waited = object_wait_many_array(&items, handles.len(), timeout_ns);
    if waited.status == Status::Timeout.raw() {
        if waited.out1 != u64::MAX || waited.out2 != 0 {
            fail(failure);
        }
        return ServiceWaitOutcome::Timeout;
    }
    let index = usize::try_from(waited.out1).unwrap_or_else(|_| fail(failure));
    if waited.status != Status::Ok.raw()
        || index >= handles.len()
        || waited.out2 & requested_bits == 0
        || waited.out2 & !channel_bits != 0
    {
        fail(failure);
    }
    ServiceWaitOutcome::Ready {
        index,
        // ObjectWaitManyArray reports the object's complete observed signal
        // state, not only the requested subset. A live Channel can therefore
        // be READABLE and WRITABLE at the same time; classify only the two
        // level-triggered signals requested by this supervisor.
        signals: waited.out2 & requested_bits,
    }
}

#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
fn write_service_probe(
    transport: u64,
    wire: &[u8; bndr_sm::health::HEALTH_FRAME_SIZE],
    failure: u64,
) -> bool {
    loop {
        let result = syscall(
            SyscallNumber::ChannelWriteBytes,
            transport,
            wire.as_ptr() as u64,
            wire.len() as u64,
        );
        if result.status == Status::Ok.raw() {
            if result.out1 != wire.len() as u64 || result.out2 != 0 {
                fail(failure);
            }
            return true;
        }
        if matches!(result.status, status if status == Status::PeerClosed.raw() || status == Status::NotFound.raw())
        {
            return false;
        }
        if result.status != Status::ShouldWait.raw() || result.out1 != 0 || result.out2 != 0 {
            fail(failure);
        }
        let waited = object_wait(
            transport,
            signal_union(ObjectSignals::WRITABLE, ObjectSignals::PEER_CLOSED),
        );
        if waited.status != Status::Ok.raw() || waited.out2 != 0 {
            fail(failure);
        }
        if waited.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
            return false;
        }
        if waited.out1 & u64::from(ObjectSignals::WRITABLE.bits()) == 0 {
            fail(failure);
        }
    }
}

#[cfg(feature = "service-supervisor-runtime")]
fn terminate_and_reap_supervised_process(pid: u64) {
    let terminated = syscall(
        SyscallNumber::ProcessTerminate,
        pid,
        PROCESS_TERMINATE_FLAGS_NONE,
        0,
    );
    if !matches!(
        terminated.status,
        status if status == Status::Ok.raw()
            || status == Status::NotFound.raw()
            || status == Status::InvalidState.raw()
    ) || terminated.out1 != 0
        || terminated.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
    let waited = syscall(SyscallNumber::ProcessWait, pid, 0, 0);
    let reason =
        ProcessTerminationReason::from_raw(waited.out2).unwrap_or_else(|| fail(FAIL_RUNTIME));
    if waited.status != Status::Ok.raw()
        || (terminated.status == Status::Ok.raw()
            && (waited.out1 != PROCESS_KILLED_EXIT_CODE
                || reason != ProcessTerminationReason::Killed))
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
fn read_service_health(transport: u64, expected_sender: u64) -> HealthFrame {
    try_read_service_health(transport, expected_sender).unwrap_or_else(|| fail(FAIL_RUNTIME))
}

#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
fn try_read_service_health(transport: u64, expected_sender: u64) -> Option<HealthFrame> {
    let envelope = read_channel_envelope(transport);
    if envelope.received_handle().is_valid() {
        let received = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))?;
        close_owned(received);
        return None;
    }
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != bndr_sm::health::HEALTH_FRAME_SIZE
        || envelope.sender_pid() != expected_sender
    {
        return None;
    }
    HealthFrame::decode(envelope.data()).ok()
}

#[cfg(any(
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
fn read_input_service_status(
    transport: u64,
    expected_sender: u64,
    sequences: &mut InputServiceRecoverySequenceTracker,
) -> InputServiceRecoveryMessage {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != bndr_input::INPUT_SERVICE_RECOVERY_WIRE_SIZE
        || envelope.sender_pid() != expected_sender
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_RUNTIME);
    }
    let message = InputServiceRecoveryMessage::decode(
        &envelope.data()[..bndr_input::INPUT_SERVICE_RECOVERY_WIRE_SIZE],
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    sequences
        .accept(message)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    message
}

#[cfg(feature = "input-server-restart-runtime")]
#[allow(clippy::too_many_arguments)]
fn expect_input_service_status(
    message: InputServiceRecoveryMessage,
    sequence: u64,
    route_epoch: u64,
    input_pid: u64,
    input_session_id: u64,
    physical_sequence_floor: u64,
    phase: InputServiceRecoveryPhase,
    focus: InputServiceFocus,
) {
    if message.sequence() != sequence
        || message.surface_session() != 1
        || message.route_epoch() != route_epoch
        || message.payload()
            != (InputServiceRecoveryPayload::SurfaceStatus {
                input_pid: OwnerPid::try_new(input_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                input_session_id,
                physical_sequence_floor,
                phase,
                route_count: 2,
                focus,
                capture_active: false,
                text_active: false,
            })
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(all(
    feature = "input-server-surface-restart-runtime",
    not(feature = "input-server-restart-runtime"),
    not(feature = "service-dependency-runtime")
))]
fn read_surface_recovery(transport: u64, expected_sender: u64) -> SurfaceRecoveryMessage {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != bndr_ui::SURFACE_RECOVERY_WIRE_SIZE
        || envelope.sender_pid() != expected_sender
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_RUNTIME);
    }
    SurfaceRecoveryMessage::decode(&envelope.data()[..bndr_ui::SURFACE_RECOVERY_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
}

#[cfg(feature = "service-dependency-runtime")]
fn read_surface_recovery(transport: u64, expected_sender: u64) -> SurfaceRecoveryMessage {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != bndr_ui::SURFACE_RECOVERY_WIRE_SIZE
        || envelope.sender_pid() != expected_sender
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_RUNTIME);
    }
    SurfaceRecoveryMessage::decode(&envelope.data()[..bndr_ui::SURFACE_RECOVERY_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
}

#[cfg(any(
    feature = "input-server-surface-restart-runtime",
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
fn spawn_window_supervisors() -> (
    OwnedUserHandle,
    OwnedUserHandle,
    u64,
    u64,
    OwnedUserHandle,
    u64,
    u64,
) {
    // BIR1 removes the M45 trust-on-first-command bootstrap. InputServer first
    // publishes its kernel-issued session; SurfaceServer is then spawned in a
    // blocked state so Init can bind the route to its exact PID before either
    // endpoint becomes usable.
    let (launcher_lifecycle, launcher_startup) = create_channel_owned();
    let (input_server_control, input_server_startup) = create_channel_owned();
    let (surface_startup, launcher_ui) = create_channel_owned();
    let (surface_control, surface_control_child) = create_channel_owned();

    let input_server_pid = spawn_owned(input_server_startup, UserImageId::InputServer);
    let acquired = read_input_route_control(input_server_control.raw(), input_server_pid);
    let InputRouteControlPayload::Acquired {
        physical_sequence_floor,
    } = acquired.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if acquired.sequence() != 1 || acquired.route_epoch() != 0 || physical_sequence_floor != 0 {
        fail(FAIL_RUNTIME);
    }
    let input_server_session = acquired.input_session_id();
    #[cfg(any(
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    if input_server_session != 1 {
        fail(FAIL_RUNTIME);
    }

    write_bootstrap_transfer(
        launcher_ui.raw(),
        UiBootstrapEndpointKind::SurfaceSupervisor,
        surface_control_child,
    );
    let surface_pid = spawn_owned(surface_startup, UserImageId::SurfaceServer);
    if surface_pid == input_server_pid {
        fail(FAIL_RUNTIME);
    }

    let (input_route_server, input_route_surface) = create_channel_owned();
    let bind = InputRouteControl::bind(
        1,
        1,
        input_server_session,
        OwnerPid::try_new(surface_pid).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        0,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire_transfer(
        input_server_control.raw(),
        &bind.encode(),
        input_route_server,
    );
    let ready = read_input_route_control(input_server_control.raw(), input_server_pid);
    let InputRouteControlPayload::Ready {
        bound_surface_pid,
        physical_sequence_floor: ready_floor,
    } = ready.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if ready.sequence() != 2
        || ready.route_epoch() != 1
        || ready.input_session_id() != input_server_session
        || bound_surface_pid.get() != surface_pid
        || ready_floor != 0
    {
        fail(FAIL_RUNTIME);
    }
    write_wire_transfer(
        launcher_ui.raw(),
        &INPUT_ROUTE_BOOTSTRAP_MAGIC.to_le_bytes(),
        input_route_surface,
    );

    write_bootstrap_transfer(
        launcher_lifecycle.raw(),
        UiBootstrapEndpointKind::LauncherUi,
        launcher_ui,
    );
    let launcher_pid = spawn_owned(launcher_startup, UserImageId::Launcher);
    if launcher_pid == surface_pid || launcher_pid == input_server_pid {
        fail(FAIL_RUNTIME);
    }
    (
        surface_control,
        launcher_lifecycle,
        surface_pid,
        launcher_pid,
        input_server_control,
        input_server_pid,
        input_server_session,
    )
}

#[cfg(all(
    feature = "graphics-surface-restart-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
fn restart_surface_graph(
    surface_control: &mut OwnedUserHandle,
    surface_pid: &mut u64,
    children: &mut ResidentChildren,
    launcher_lifecycle: u64,
    launcher_pid: u64,
    app_lifecycle: u64,
    app_pid: u64,
) {
    let old_surface_pid = *surface_pid;
    if old_surface_pid == 0
        || launcher_pid == 0
        || app_pid == 0
        || old_surface_pid == launcher_pid
        || old_surface_pid == app_pid
        || launcher_pid == app_pid
    {
        fail(FAIL_RUNTIME);
    }

    // Queue the complete replacement authority graph before spawning.  The
    // startup endpoint doubles as Surface--Launcher UI transport, while the
    // two UBP1 transfers carry only the Init control and Surface--App peers.
    let (surface_startup, launcher_ui) = create_channel_owned();
    let (replacement_control, replacement_control_child) = create_channel_owned();
    write_bootstrap_transfer(
        launcher_ui.raw(),
        UiBootstrapEndpointKind::SurfaceSupervisor,
        replacement_control_child,
    );
    #[cfg(feature = "input-server-runtime")]
    {
        // InputServer outlives SurfaceServer. Replace only the authenticated
        // route session, wait until the resident authority has reset its
        // scene/focus/IME epoch, then queue the ready peer ahead of AppUi for
        // the replacement SurfaceServer's deterministic bootstrap order.
        let input_control = children
            .input_server_control
            .as_ref()
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        let input_pid = children.input_server_pid;
        if input_pid == 0 || input_pid == old_surface_pid || input_pid == launcher_pid {
            fail(FAIL_RUNTIME);
        }
        let (input_route_server, input_route_surface) = create_channel_owned();
        write_wire_transfer(
            input_control.raw(),
            &INPUT_ROUTE_BOOTSTRAP_MAGIC.to_le_bytes(),
            input_route_server,
        );
        read_authenticated_magic(input_control.raw(), input_pid, INPUT_SERVER_READY_MAGIC);
        write_wire_transfer(
            launcher_ui.raw(),
            &INPUT_ROUTE_BOOTSTRAP_MAGIC.to_le_bytes(),
            input_route_surface,
        );
    }
    let (surface_app_ui, app_ui) = create_channel_owned();
    write_bootstrap_transfer(
        launcher_ui.raw(),
        UiBootstrapEndpointKind::AppUi,
        surface_app_ui,
    );
    let replacement_pid = spawn_owned(surface_startup, UserImageId::SurfaceServer);
    if process_slot(replacement_pid) != process_slot(old_surface_pid)
        || process_generation(replacement_pid)
            != process_generation(old_surface_pid)
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_RUNTIME))
    {
        fail(FAIL_RUNTIME);
    }

    // Keep both client peers in Init until the replacement has consumed its
    // complete bootstrap graph and queued the initial Ready/FocusChanged
    // events.  This makes the supervisor reply a real bootstrap barrier: a
    // client cannot race the replacement by closing or using its new endpoint
    // while SurfaceServer is still reading UBP1 transfers from `startup`.
    let ready = read_channel_envelope(replacement_control.raw());
    if ready.kind() != ChannelMessageKind::Bytes
        || ready.logical_length() != size_of::<u64>()
        || ready.sender_pid() != replacement_pid
        || ready.received_handle().is_valid()
        || ready.data()[..size_of::<u64>()] != GRAPHICS_SURFACE_RESTART_READY_MAGIC.to_le_bytes()
    {
        fail(FAIL_RUNTIME);
    }

    // Existing lifecycle channels are the stable recovery roots.  Once the
    // bootstrap barrier has completed, each live client atomically installs
    // the corresponding fresh session endpoint and drains its queued events.
    write_bootstrap_transfer(
        launcher_lifecycle,
        UiBootstrapEndpointKind::LauncherUi,
        launcher_ui,
    );
    write_bootstrap_transfer(app_lifecycle, UiBootstrapEndpointKind::AppUi, app_ui);

    let old_control = core::mem::replace(surface_control, replacement_control);
    close_owned(old_control);
    *surface_pid = replacement_pid;
    children.surface_server_pid = replacement_pid;
}

#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
#[allow(clippy::too_many_arguments)]
fn finish_graphics_producer_orphan_runtime(
    machine: &mut AppLifecycleStateMachine,
    resident_app: &mut Option<InitAppGeneration>,
    children: &mut ResidentChildren,
    surface_control: OwnedUserHandle,
    launcher_lifecycle: OwnedUserHandle,
    surface_pid: u64,
    launcher_pid: u64,
    capacity_probed: bool,
) -> ! {
    let app = resident_app.take().unwrap_or_else(|| fail(FAIL_RUNTIME));
    if machine.state() != AppLifecycleState::Inactive
        || machine.identity() != Some(app.identity)
        || machine.last_transaction_id() != Some(TRANSACTION_COUNT)
        || TRANSACTION_COUNT != 1
        || app.app_ui.is_some()
        || app.surface_ui.is_some()
        || !capacity_probed
        || app.pid == 0
        || surface_pid == 0
        || launcher_pid == 0
        || app.pid == surface_pid
        || app.pid == launcher_pid
        || surface_pid == launcher_pid
    {
        fail(FAIL_RUNTIME);
    }

    terminate_and_wait_killed(app.pid);
    children.app_pid = 0;
    close_owned(app.lifecycle);

    // SurfaceServer publishes this only from the authenticated AppUi
    // PEER_CLOSED edge after proving that both mapped consumer aliases and
    // both server handles are still resident. Slot 0 remains Acquired while
    // slot 1 remains Writable.
    read_authenticated_magic(
        surface_control.raw(),
        surface_pid,
        GRAPHICS_PRODUCER_ORPHAN_READY_MAGIC,
    );

    terminate_and_wait_killed(surface_pid);
    children.surface_server_pid = 0;
    close_owned(surface_control);

    // The Init--Launcher lifecycle pair is deliberately the only UI recovery
    // root that survives both deaths. Launcher accepts this one-shot request
    // only after its old Surface endpoint has observed PEER_CLOSED.
    write_wire(
        launcher_lifecycle.raw(),
        &GRAPHICS_PRODUCER_REUSE_REQUEST_MAGIC.to_le_bytes(),
    );
    read_authenticated_magic(
        launcher_lifecycle.raw(),
        launcher_pid,
        GRAPHICS_PRODUCER_REUSE_DONE_MAGIC,
    );

    let _ = (children, launcher_lifecycle, machine);
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(any(
    feature = "input-server-surface-restart-runtime",
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime",
    all(
        feature = "graphics-producer-orphan-runtime",
        not(feature = "graphics-surface-restart-runtime"),
        not(feature = "app-crash-recovery-runtime")
    )
))]
fn terminate_and_wait_killed(pid: u64) {
    let terminated = syscall(
        SyscallNumber::ProcessTerminate,
        pid,
        PROCESS_TERMINATE_FLAGS_NONE,
        0,
    );
    if terminated.status != Status::Ok.raw() || terminated.out1 != 0 || terminated.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    let waited = syscall(SyscallNumber::ProcessWait, pid, 0, 0);
    if waited.status != Status::Ok.raw()
        || waited.out1 != PROCESS_KILLED_EXIT_CODE
        || waited.out2 != ProcessTerminationReason::Killed.raw()
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(any(
    feature = "input-server-runtime",
    all(
        feature = "graphics-producer-orphan-runtime",
        not(feature = "graphics-surface-restart-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ),
    all(
        feature = "multi-window-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    )
))]
fn read_authenticated_magic(transport: u64, expected_sender: u64, expected_magic: u64) {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != size_of::<u64>()
        || envelope.sender_pid() != expected_sender
        || envelope.received_handle().is_valid()
        || envelope.data()[..size_of::<u64>()] != expected_magic.to_le_bytes()
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(any(
    feature = "input-server-surface-restart-runtime",
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
fn read_input_route_control(transport: u64, expected_sender: u64) -> InputRouteControl {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != bndr_input::INPUT_ROUTE_CONTROL_WIRE_SIZE
        || envelope.sender_pid() != expected_sender
        || envelope.received_handle().is_valid()
    {
        fail(FAIL_RUNTIME);
    }
    InputRouteControl::decode(&envelope.data()[..bndr_input::INPUT_ROUTE_CONTROL_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME))
}

fn spawn_app_generation(instance_id: u64) -> InitAppGeneration {
    let (lifecycle, startup) = create_channel_owned();
    let (surface_ui, app_ui) = create_channel_owned();
    let pid = spawn_owned(startup, UserImageId::App);
    let identity =
        AppInstanceIdentity::try_new(instance_id, process_slot(pid), process_generation(pid))
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    InitAppGeneration {
        lifecycle,
        app_ui: Some(app_ui),
        surface_ui: Some(surface_ui),
        pid,
        identity,
    }
}

fn spawn_owned(startup: OwnedUserHandle, image: UserImageId) -> u64 {
    let raw = startup.raw();
    let spawned = syscall(
        SyscallNumber::ProcessSpawn,
        raw,
        image.raw(),
        PROCESS_SPAWN_FLAGS_NONE,
    );
    if spawned.status != Status::Ok.raw() || spawned.out1 == 0 || spawned.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    assert_stale_handle(raw);
    spawned.out1
}

#[cfg(feature = "unified-product-runtime")]
fn supervise_runtime(
    children: &ResidentChildren,
    surface: OwnedUserHandle,
    launcher: OwnedUserHandle,
    app: OwnedUserHandle,
) -> ! {
    super::product_runtime::complete_runtime(children, surface, launcher, app)
}

#[cfg(not(feature = "unified-product-runtime"))]
fn supervise_runtime(
    children: &ResidentChildren,
    surface: OwnedUserHandle,
    launcher: OwnedUserHandle,
    app: OwnedUserHandle,
) -> ! {
    let secondary = children
        .secondary_control
        .as_ref()
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    #[cfg(all(
        feature = "input-server-runtime",
        any(
            feature = "service-dependency-runtime",
            not(feature = "service-supervisor-runtime")
        )
    ))]
    let input_server = children
        .input_server_control
        .as_ref()
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    #[cfg(all(
        feature = "input-server-runtime",
        any(
            feature = "service-dependency-runtime",
            not(feature = "service-supervisor-runtime")
        )
    ))]
    if children.input_server_pid == 0
        || children.input_server_pid == children.surface_server_pid
        || children.input_server_pid == children.launcher_pid
        || children.input_server_pid == children.app_pid
        || input_server.raw() == 0
    {
        fail(FAIL_RUNTIME);
    }
    #[cfg(any(
        not(feature = "input-server-runtime"),
        all(
            feature = "service-supervisor-runtime",
            not(feature = "service-dependency-runtime")
        )
    ))]
    let handles = [
        children.manager_control.raw(),
        children.provider_control.raw(),
        children.client_control.raw(),
        secondary.raw(),
        surface.raw(),
        launcher.raw(),
        app.raw(),
    ];
    #[cfg(all(
        feature = "input-server-runtime",
        any(
            feature = "service-dependency-runtime",
            not(feature = "service-supervisor-runtime")
        )
    ))]
    let handles = [
        children.manager_control.raw(),
        children.provider_control.raw(),
        children.client_control.raw(),
        secondary.raw(),
        surface.raw(),
        launcher.raw(),
        app.raw(),
        input_server.raw(),
    ];
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    for (item, handle) in items.iter_mut().zip(handles) {
        *item = pack_user_wait_item(handle, requested);
    }
    let ready = object_wait_many_array(&items, handles.len(), OBJECT_WAIT_TIMEOUT_INFINITE);
    let _ = (surface, launcher, app, ready);
    fail(FAIL_RUNTIME)
}

fn write_bootstrap_transfer(
    transport: u64,
    kind: UiBootstrapEndpointKind,
    endpoint: OwnedUserHandle,
) {
    let wire = UiBootstrapMessage::new(kind).encode();
    write_wire_transfer(transport, &wire, endpoint);
}

fn read_bootstrap_transfer(
    transport: u64,
    expected_kind: UiBootstrapEndpointKind,
) -> (u64, OwnedUserHandle) {
    read_bootstrap_transfer_with_failure(transport, expected_kind, FAIL_RUNTIME)
}

fn read_bootstrap_transfer_with_failure(
    transport: u64,
    expected_kind: UiBootstrapEndpointKind,
    failure: u64,
) -> (u64, OwnedUserHandle) {
    let envelope = read_channel_envelope(transport);
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != UI_BOOTSTRAP_WIRE_SIZE
    {
        fail(failure);
    }
    let message = UiBootstrapMessage::decode(&envelope.data()[..UI_BOOTSTRAP_WIRE_SIZE])
        .unwrap_or_else(|_| fail(failure));
    let endpoint = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(failure));
    if message.kind() != expected_kind || envelope.sender_pid() == 0 {
        close_owned(endpoint);
        fail(failure);
    }
    (envelope.sender_pid(), endpoint)
}

fn accept_and_write_lifecycle(
    tracker: &mut AppLifecycleTracker,
    transport: u64,
    message: AppLifecycleMessage,
    transfer: Option<OwnedUserHandle>,
) {
    if tracker.accept(message).is_err() {
        fail(FAIL_RUNTIME);
    }
    let wire = message.encode();
    match transfer {
        Some(endpoint) => write_wire_transfer(transport, &wire, endpoint),
        None => write_wire(transport, &wire),
    }
}

fn read_lifecycle_message(
    transport: u64,
    expected_sender: u64,
    expect_transfer: bool,
) -> (AppLifecycleMessage, Option<OwnedUserHandle>) {
    let envelope = read_channel_envelope(transport);
    let kind_valid = if expect_transfer {
        envelope.kind() == ChannelMessageKind::Transfer && envelope.received_handle().is_valid()
    } else {
        envelope.kind() == ChannelMessageKind::Bytes && !envelope.received_handle().is_valid()
    };
    if !kind_valid
        || envelope.logical_length() != APP_LIFECYCLE_WIRE_SIZE
        || envelope.sender_pid() != expected_sender
    {
        fail(FAIL_RUNTIME);
    }
    let message = AppLifecycleMessage::decode(&envelope.data()[..APP_LIFECYCLE_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let transferred = if expect_transfer {
        Some(
            OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
                .unwrap_or_else(|| fail(FAIL_RUNTIME)),
        )
    } else {
        None
    };
    (message, transferred)
}

fn accept_and_write_supervisor(
    tracker: &mut UiSupervisorTracker,
    transport: u64,
    message: UiSupervisorControlMessage,
    transfer: Option<OwnedUserHandle>,
) {
    if tracker.accept(message).is_err() {
        fail(FAIL_RUNTIME);
    }
    let wire = message.encode();
    match transfer {
        Some(endpoint) => write_wire_transfer(transport, &wire, endpoint),
        None => write_wire(transport, &wire),
    }
}

fn read_supervisor_message(
    transport: u64,
    expected_sender: u64,
) -> (UiSupervisorControlMessage, Option<OwnedUserHandle>) {
    let envelope = read_channel_envelope(transport);
    if !matches!(
        envelope.kind(),
        ChannelMessageKind::Bytes | ChannelMessageKind::Transfer
    ) || envelope.logical_length() != UI_SUPERVISOR_CONTROL_WIRE_SIZE
        || envelope.sender_pid() != expected_sender
    {
        fail(FAIL_RUNTIME);
    }
    let transferred = if envelope.kind() == ChannelMessageKind::Transfer {
        Some(
            OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
                .unwrap_or_else(|| fail(FAIL_RUNTIME)),
        )
    } else {
        if envelope.received_handle().is_valid() {
            fail(FAIL_RUNTIME);
        }
        None
    };
    let message =
        UiSupervisorControlMessage::decode(&envelope.data()[..UI_SUPERVISOR_CONTROL_WIRE_SIZE])
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    (message, transferred)
}

fn write_wire<const N: usize>(transport: u64, wire: &[u8; N]) {
    wait_writable(transport);
    let result = syscall(
        SyscallNumber::ChannelWriteBytes,
        transport,
        wire.as_ptr() as u64,
        N as u64,
    );
    if result.status != Status::Ok.raw() || result.out1 != N as u64 || result.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
}

fn write_wire_transfer<const N: usize>(transport: u64, wire: &[u8; N], endpoint: OwnedUserHandle) {
    wait_writable(transport);
    let raw = endpoint.raw();
    let result = transfer_write(transport, wire.as_ptr() as u64, raw, N);
    if result.status != Status::Ok.raw() || result.out1 != N as u64 || result.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    assert_stale_handle(raw);
}

struct SurfaceAppSlot {
    endpoint: OwnedUserHandle,
    buffer: OwnedUserHandle,
    #[cfg(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    swapchain_buffer: OwnedUserHandle,
    #[cfg(all(
        feature = "graphics-producer-orphan-runtime",
        not(feature = "graphics-surface-restart-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    standby_buffer: OwnedUserHandle,
    identity: AppInstanceIdentity,
    pending_frame: Option<BufferPresent>,
    last_client_frame: Option<u32>,
    input_sequence: u64,
    #[cfg(all(
        feature = "mapped-graphics-runtime",
        not(feature = "app-crash-recovery-runtime")
    ))]
    consumer_mapping: u64,
    #[cfg(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    swapchain_consumer_mapping: u64,
    #[cfg(all(
        feature = "graphics-producer-orphan-runtime",
        not(feature = "graphics-surface-restart-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    standby_consumer_mapping: u64,
}

struct LifecycleSurface {
    launcher: OwnedUserHandle,
    control: OwnedUserHandle,
    capability: OwnedUserHandle,
    #[cfg(feature = "graphics-surface-restart-runtime")]
    restart_app_endpoint: Option<OwnedUserHandle>,
    init_pid: u64,
    session_id: u64,
    tracker: UiSupervisorTracker,
    app: Option<SurfaceAppSlot>,
    active_identity: Option<AppInstanceIdentity>,
    input_capture: Option<AppInstanceIdentity>,
    pointer_pressed: bool,
    focus_generation: u64,
    global_frame_id: u32,
    #[cfg(all(
        feature = "graphics-frame-clock-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    last_frame_epoch: u64,
    #[cfg(all(
        feature = "graphics-frame-clock-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    last_frame_boundary: u64,
    command_sequence: u64,
    response_sequence: u64,
}

#[derive(Clone, Copy)]
enum SurfaceWaitSource {
    Input,
    Control,
    Launcher,
    App,
}

pub(super) fn surface_runtime(startup: u64) -> ! {
    let acquired = syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0);
    if acquired.status != Status::Ok.raw() || acquired.out1 == 0 || acquired.out2 == 0 {
        fail(FAIL_RUNTIME);
    }
    let capability = OwnedUserHandle::new(acquired.out1).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let (init_pid, control) =
        read_bootstrap_transfer(startup, UiBootstrapEndpointKind::SurfaceSupervisor);
    #[cfg(feature = "graphics-surface-restart-runtime")]
    let restart_app_endpoint = if acquired.out2 > 1 {
        let (app_init_pid, endpoint) =
            read_bootstrap_transfer(startup, UiBootstrapEndpointKind::AppUi);
        if app_init_pid != init_pid {
            close_owned(endpoint);
            fail(FAIL_RUNTIME);
        }
        Some(endpoint)
    } else {
        None
    };
    let launcher = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let mut surface = LifecycleSurface {
        launcher,
        control,
        capability,
        #[cfg(feature = "graphics-surface-restart-runtime")]
        restart_app_endpoint,
        init_pid,
        session_id: acquired.out2,
        tracker: UiSupervisorTracker::new(),
        app: None,
        active_identity: None,
        input_capture: None,
        pointer_pressed: false,
        focus_generation: 1,
        global_frame_id: 0,
        #[cfg(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        ))]
        last_frame_epoch: 0,
        #[cfg(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        ))]
        last_frame_boundary: 0,
        command_sequence: if cfg!(feature = "graphics-surface-restart-runtime") && acquired.out2 > 1
        {
            1
        } else {
            0
        },
        response_sequence: if cfg!(feature = "graphics-surface-restart-runtime")
            && acquired.out2 > 1
        {
            1
        } else {
            0
        },
    };
    send_ui_event(
        surface.launcher.raw(),
        UiServerEvent::ready(surface.session_id).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
    );
    publish_surface_focus(&surface, UiClientId::Launcher, None);
    #[cfg(feature = "graphics-surface-restart-runtime")]
    if let Some(endpoint) = surface.restart_app_endpoint.as_ref() {
        send_ui_event(
            endpoint.raw(),
            UiServerEvent::ready(surface.session_id).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        );
        send_ui_event(
            endpoint.raw(),
            UiServerEvent::focus_changed(
                surface.session_id,
                UiClientId::Launcher,
                None,
                surface.focus_generation,
            )
            .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
        );
        write_wire(
            surface.control.raw(),
            &GRAPHICS_SURFACE_RESTART_READY_MAGIC.to_le_bytes(),
        );
    }

    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    loop {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        let mut sources = [SurfaceWaitSource::Input; 4];
        let mut count = 3;
        sources[0] = SurfaceWaitSource::Input;
        sources[1] = SurfaceWaitSource::Control;
        sources[2] = SurfaceWaitSource::Launcher;
        items[0] = pack_user_wait_item(surface.capability.raw(), requested);
        items[1] = pack_user_wait_item(surface.control.raw(), requested);
        items[2] = pack_user_wait_item(surface.launcher.raw(), requested);
        if let Some(app) = surface.app.as_ref() {
            sources[count] = SurfaceWaitSource::App;
            items[count] = pack_user_wait_item(app.endpoint.raw(), requested);
            count += 1;
        }
        let ready = object_wait_many_array(&items, count, OBJECT_WAIT_TIMEOUT_INFINITE);
        let index = usize::try_from(ready.out1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        if ready.status != Status::Ok.raw()
            || index >= count
            || ready.out2 & u64::from(requested.bits()) == 0
        {
            fail(FAIL_RUNTIME);
        }
        match sources[index] {
            SurfaceWaitSource::Input => consume_surface_input(&mut surface),
            SurfaceWaitSource::Control => dispatch_supervisor_command(&mut surface),
            SurfaceWaitSource::Launcher => fail(FAIL_RUNTIME),
            SurfaceWaitSource::App => {
                #[cfg(all(
                    feature = "graphics-swapchain-runtime",
                    not(feature = "graphics-owner-death-runtime"),
                    not(feature = "app-crash-recovery-runtime")
                ))]
                fail(FAIL_RUNTIME);
                #[cfg(all(
                    feature = "graphics-frame-clock-runtime",
                    not(feature = "graphics-swapchain-runtime"),
                    not(feature = "graphics-owner-death-runtime"),
                    not(feature = "app-crash-recovery-runtime")
                ))]
                handle_frame_clock_app_frame(&mut surface, ready.out2);
                #[cfg(feature = "app-crash-recovery-runtime")]
                handle_app_owner_death(&mut surface, ready.out2);
                #[cfg(all(
                    feature = "graphics-producer-orphan-runtime",
                    not(feature = "graphics-surface-restart-runtime"),
                    not(feature = "app-crash-recovery-runtime")
                ))]
                handle_graphics_producer_orphan(&surface, ready.out2);
                #[cfg(all(
                    not(feature = "app-crash-recovery-runtime"),
                    not(all(
                        feature = "graphics-frame-clock-runtime",
                        not(feature = "graphics-swapchain-runtime"),
                        not(feature = "graphics-owner-death-runtime")
                    )),
                    not(all(
                        feature = "graphics-swapchain-runtime",
                        not(feature = "graphics-owner-death-runtime")
                    )),
                    not(all(
                        feature = "graphics-producer-orphan-runtime",
                        not(feature = "graphics-surface-restart-runtime")
                    ))
                ))]
                fail(FAIL_RUNTIME);
            }
        }
    }
}

#[cfg(feature = "graphics-surface-restart-runtime")]
fn seed_replacement_supervisor_tracker(
    surface: &mut LifecycleSurface,
    command: UiSupervisorControlMessage,
    operation: UiSupervisorOperation,
    app: Option<ShellAppId>,
    identity: Option<AppInstanceIdentity>,
) {
    if surface.session_id <= 1
        || surface
            .tracker
            .transaction()
            .last_transaction_id()
            .is_some()
    {
        return;
    }
    let identity = identity.unwrap_or_else(|| fail(FAIL_RUNTIME));
    if command.sender_sequence() != 2
        || command.transaction_id() != 2
        || operation != UiSupervisorOperation::ActivateApp
        || app != Some(APP)
        || surface.restart_app_endpoint.is_none()
    {
        fail(FAIL_RUNTIME);
    }

    // The old server committed transaction 1 before dying.  Replaying that
    // already-authenticated state locally gives the replacement the exact
    // global USC1 high-water marks without emitting a duplicate wire message.
    let installed = UiSupervisorControlMessage::command(
        1,
        1,
        UiSupervisorOperation::InstallAppEndpoint,
        Some(APP),
        Some(identity),
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let installed_ack = UiSupervisorControlMessage::ack(
        1,
        1,
        UiSupervisorOperation::InstallAppEndpoint,
        UiSupervisorStatus::Applied,
        Some(APP),
        Some(identity),
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if surface.tracker.accept(installed).is_err() || surface.tracker.accept(installed_ack).is_err()
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(all(
    feature = "graphics-surface-restart-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
fn materialize_rebound_app(
    surface: &mut LifecycleSurface,
    app: Option<ShellAppId>,
    identity: Option<AppInstanceIdentity>,
) {
    let identity = identity.unwrap_or_else(|| fail(FAIL_RUNTIME));
    if app != Some(APP) || surface.app.is_some() || surface.active_identity.is_some() {
        fail(FAIL_RUNTIME);
    }
    let endpoint = surface
        .restart_app_endpoint
        .take()
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let (buffer, frame) =
        receive_rebound_app_buffer(endpoint.raw(), identity, surface.focus_generation);
    let consumer_mapping = map_graphics_buffer(buffer.raw(), GRAPHICS_BUFFER_MAP_CONSUMER_RO);
    wait_graphics_fence(buffer.raw(), ObjectSignals::READABLE);
    acquire_graphics_buffer(buffer.raw(), frame.buffer_generation());
    verify_mapped_frame_samples(
        consumer_mapping,
        identity.instance_id(),
        frame.buffer_generation(),
    );
    surface.app = Some(SurfaceAppSlot {
        endpoint,
        buffer,
        identity,
        pending_frame: Some(frame),
        last_client_frame: None,
        input_sequence: 0,
        consumer_mapping,
    });
}

fn dispatch_supervisor_command(surface: &mut LifecycleSurface) {
    let (command, transferred) = read_supervisor_message(surface.control.raw(), surface.init_pid);
    let UiSupervisorControlPayload::Command {
        operation,
        app,
        identity,
    } = command.payload()
    else {
        if let Some(endpoint) = transferred {
            close_owned(endpoint);
        }
        fail(FAIL_RUNTIME);
    };
    #[cfg(feature = "graphics-surface-restart-runtime")]
    seed_replacement_supervisor_tracker(surface, command, operation, app, identity);
    if command.sender_sequence() != next_sequence(&mut surface.command_sequence)
        || command.transaction_id() > TRANSACTION_COUNT
        || surface.tracker.accept(command).is_err()
    {
        if let Some(endpoint) = transferred {
            close_owned(endpoint);
        }
        fail(FAIL_RUNTIME);
    }

    #[cfg(all(
        feature = "graphics-surface-restart-runtime",
        not(feature = "app-crash-recovery-runtime")
    ))]
    if surface.session_id > 1 && operation == UiSupervisorOperation::ActivateApp {
        materialize_rebound_app(surface, app, identity);
    }

    match operation {
        UiSupervisorOperation::InstallAppEndpoint => {
            let endpoint = transferred.unwrap_or_else(|| fail(FAIL_RUNTIME));
            let identity = identity.unwrap_or_else(|| fail(FAIL_RUNTIME));
            if app != Some(APP) || surface.app.is_some() || surface.active_identity.is_some() {
                close_owned(endpoint);
                fail(FAIL_RUNTIME);
            }
            #[cfg(all(
                feature = "mapped-graphics-runtime",
                not(feature = "app-crash-recovery-runtime")
            ))]
            {
                // The App queues this transfer before Init moves `endpoint`
                // into SurfaceServer.  Receiving and mapping it first lets
                // Ready become the producer's permission to render frame 1;
                // SurfaceServer can therefore block on the real READABLE
                // acquire fence instead of racing an already queued frame.
                let (buffer, frame) =
                    receive_initial_app_buffer(endpoint.raw(), identity, surface.focus_generation);
                let consumer_mapping =
                    map_graphics_buffer(buffer.raw(), GRAPHICS_BUFFER_MAP_CONSUMER_RO);
                #[cfg(all(
                    feature = "graphics-swapchain-runtime",
                    not(feature = "graphics-owner-death-runtime"),
                    not(feature = "app-crash-recovery-runtime")
                ))]
                let (swapchain_buffer, swapchain_frame, swapchain_consumer_mapping) = {
                    let (swapchain_buffer, swapchain_frame) = receive_swapchain_initial_buffer(
                        endpoint.raw(),
                        identity,
                        surface.focus_generation,
                    );
                    let swapchain_consumer_mapping = map_graphics_buffer(
                        swapchain_buffer.raw(),
                        GRAPHICS_BUFFER_MAP_CONSUMER_RO,
                    );
                    if swapchain_buffer.raw() == buffer.raw()
                        || swapchain_consumer_mapping == consumer_mapping
                    {
                        fail(FAIL_RUNTIME);
                    }
                    (
                        swapchain_buffer,
                        swapchain_frame,
                        swapchain_consumer_mapping,
                    )
                };
                #[cfg(all(
                    feature = "graphics-producer-orphan-runtime",
                    not(feature = "graphics-surface-restart-runtime"),
                    not(feature = "app-crash-recovery-runtime")
                ))]
                let (standby_buffer, standby_consumer_mapping) = {
                    let standby_buffer = receive_orphan_standby_buffer(
                        endpoint.raw(),
                        identity,
                        surface.focus_generation,
                    );
                    let standby_consumer_mapping =
                        map_graphics_buffer(standby_buffer.raw(), GRAPHICS_BUFFER_MAP_CONSUMER_RO);
                    if standby_buffer.raw() == buffer.raw()
                        || standby_consumer_mapping == consumer_mapping
                    {
                        fail(FAIL_RUNTIME);
                    }
                    (standby_buffer, standby_consumer_mapping)
                };
                let slot = SurfaceAppSlot {
                    endpoint,
                    buffer,
                    #[cfg(all(
                        feature = "graphics-swapchain-runtime",
                        not(feature = "graphics-owner-death-runtime"),
                        not(feature = "app-crash-recovery-runtime")
                    ))]
                    swapchain_buffer,
                    #[cfg(all(
                        feature = "graphics-producer-orphan-runtime",
                        not(feature = "graphics-surface-restart-runtime"),
                        not(feature = "app-crash-recovery-runtime")
                    ))]
                    standby_buffer,
                    identity,
                    pending_frame: None,
                    last_client_frame: None,
                    input_sequence: 0,
                    consumer_mapping,
                    #[cfg(all(
                        feature = "graphics-swapchain-runtime",
                        not(feature = "graphics-owner-death-runtime"),
                        not(feature = "app-crash-recovery-runtime")
                    ))]
                    swapchain_consumer_mapping,
                    #[cfg(all(
                        feature = "graphics-producer-orphan-runtime",
                        not(feature = "graphics-surface-restart-runtime"),
                        not(feature = "app-crash-recovery-runtime")
                    ))]
                    standby_consumer_mapping,
                };
                send_ui_event(
                    slot.endpoint.raw(),
                    UiServerEvent::ready(surface.session_id).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                );
                send_ui_event(
                    slot.endpoint.raw(),
                    UiServerEvent::focus_changed(
                        surface.session_id,
                        UiClientId::Launcher,
                        None,
                        surface.focus_generation,
                    )
                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                );
                #[cfg(all(
                    feature = "graphics-swapchain-runtime",
                    not(feature = "graphics-owner-death-runtime"),
                    not(feature = "app-crash-recovery-runtime")
                ))]
                {
                    receive_swapchain_magic(
                        slot.endpoint.raw(),
                        slot.identity,
                        SWAPCHAIN_INITIAL_BATCH_MAGIC,
                    );
                    wait_graphics_fence(slot.buffer.raw(), ObjectSignals::READABLE);
                    wait_graphics_fence(slot.swapchain_buffer.raw(), ObjectSignals::READABLE);
                    acquire_graphics_buffer(slot.buffer.raw(), frame.buffer_generation());
                    acquire_graphics_buffer(
                        slot.swapchain_buffer.raw(),
                        swapchain_frame.buffer_generation(),
                    );
                    verify_swapchain_frame_samples(
                        slot.consumer_mapping,
                        slot.identity.instance_id(),
                        0,
                        frame.client_frame_id(),
                        frame.buffer_generation(),
                    );
                    verify_swapchain_frame_samples(
                        slot.swapchain_consumer_mapping,
                        slot.identity.instance_id(),
                        1,
                        swapchain_frame.client_frame_id(),
                        swapchain_frame.buffer_generation(),
                    );
                    send_ui_event(
                        slot.endpoint.raw(),
                        UiServerEvent::present_cancelled(
                            surface.session_id,
                            frame.client_frame_id(),
                            surface.focus_generation,
                        )
                        .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                    );
                    send_ui_event(
                        slot.endpoint.raw(),
                        UiServerEvent::present_cancelled(
                            surface.session_id,
                            swapchain_frame.client_frame_id(),
                            surface.focus_generation,
                        )
                        .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                    );
                    release_graphics_buffer(slot.buffer.raw(), frame.buffer_generation());
                    release_graphics_buffer(
                        slot.swapchain_buffer.raw(),
                        swapchain_frame.buffer_generation(),
                    );
                }
                #[cfg(not(all(
                    feature = "graphics-swapchain-runtime",
                    not(feature = "graphics-owner-death-runtime"),
                    not(feature = "app-crash-recovery-runtime")
                )))]
                {
                    wait_graphics_fence(slot.buffer.raw(), ObjectSignals::READABLE);
                    acquire_graphics_buffer(slot.buffer.raw(), frame.buffer_generation());
                    verify_mapped_frame_samples(
                        slot.consumer_mapping,
                        slot.identity.instance_id(),
                        frame.buffer_generation(),
                    );
                }
                #[cfg(feature = "graphics-owner-death-runtime")]
                {
                    let denied = syscall(
                        SyscallNumber::GraphicsBufferUnmap,
                        slot.buffer.raw(),
                        slot.consumer_mapping,
                        0,
                    );
                    if denied.status != Status::InvalidState.raw()
                        || denied.out1 != 0
                        || denied.out2 != 0
                    {
                        fail(FAIL_RUNTIME);
                    }
                    let close_denied = syscall(SyscallNumber::HandleClose, slot.buffer.raw(), 0, 0);
                    if close_denied.status != Status::InvalidState.raw()
                        || close_denied.out1 != 0
                        || close_denied.out2 != 0
                    {
                        fail(FAIL_RUNTIME);
                    }
                }
                #[cfg(all(
                    not(feature = "graphics-owner-death-runtime"),
                    not(feature = "graphics-swapchain-runtime")
                ))]
                {
                    send_ui_event(
                        slot.endpoint.raw(),
                        UiServerEvent::present_cancelled(
                            surface.session_id,
                            frame.client_frame_id(),
                            surface.focus_generation,
                        )
                        .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                    );
                    release_graphics_buffer(slot.buffer.raw(), frame.buffer_generation());
                }
                surface.app = Some(slot);
            }
            #[cfg(not(all(
                feature = "mapped-graphics-runtime",
                not(feature = "app-crash-recovery-runtime")
            )))]
            {
                let mut slot = SurfaceAppSlot {
                    endpoint,
                    buffer: provision_placeholder_handle(),
                    identity,
                    pending_frame: None,
                    last_client_frame: None,
                    input_sequence: 0,
                };
                // Install becomes visible only after both endpoint and its first
                // generation-qualified buffer have been validated.
                send_ui_event(
                    slot.endpoint.raw(),
                    UiServerEvent::ready(surface.session_id).unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                );
                send_ui_event(
                    slot.endpoint.raw(),
                    UiServerEvent::focus_changed(
                        surface.session_id,
                        UiClientId::Launcher,
                        None,
                        surface.focus_generation,
                    )
                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                );
                let (buffer, frame) = receive_initial_app_buffer(
                    slot.endpoint.raw(),
                    identity,
                    surface.focus_generation,
                );
                close_owned(slot.buffer);
                slot.buffer = buffer;
                slot.pending_frame = Some(frame);
                send_ui_event(
                    slot.endpoint.raw(),
                    UiServerEvent::present_cancelled(
                        surface.session_id,
                        frame.client_frame_id(),
                        surface.focus_generation,
                    )
                    .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
                );
                surface.app = Some(slot);
            }
        }
        UiSupervisorOperation::ActivateApp => {
            if transferred.is_some() || app != Some(APP) || identity.is_none() {
                fail(FAIL_RUNTIME);
            }
            let identity = identity.unwrap_or_else(|| fail(FAIL_RUNTIME));
            if surface.app.as_ref().map(|slot| slot.identity) != Some(identity)
                || surface.active_identity == Some(identity)
            {
                fail(FAIL_RUNTIME);
            }
            surface.active_identity = Some(identity);
            surface.focus_generation = surface
                .focus_generation
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_RUNTIME));
            publish_surface_focus(surface, UiClientId::App, Some(APP));
            present_pending_app_frame(surface);
        }
        UiSupervisorOperation::ShowLauncher => {
            if transferred.is_some()
                || app.is_some()
                || identity.is_some()
                || surface.active_identity.is_none()
            {
                fail(FAIL_RUNTIME);
            }
            surface.active_identity = None;
            surface.focus_generation = surface
                .focus_generation
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_RUNTIME));
            publish_surface_focus(surface, UiClientId::Launcher, None);
        }
        UiSupervisorOperation::RetireAppEndpoint => {
            if transferred.is_some() || app != Some(APP) || identity.is_none() {
                fail(FAIL_RUNTIME);
            }
            let identity = identity.unwrap_or_else(|| fail(FAIL_RUNTIME));
            if surface.app.as_ref().map(|slot| slot.identity) != Some(identity) {
                fail(FAIL_RUNTIME);
            }
            surface.active_identity = None;
            surface.focus_generation = surface
                .focus_generation
                .checked_add(1)
                .unwrap_or_else(|| fail(FAIL_RUNTIME));
            publish_surface_focus(surface, UiClientId::Launcher, None);
            // One commit point clears every identity-coupled resource.  The
            // global compositor frame id deliberately survives replacement.
            let retired = surface.app.take().unwrap_or_else(|| fail(FAIL_RUNTIME));
            #[cfg(all(
                feature = "mapped-graphics-runtime",
                not(feature = "app-crash-recovery-runtime")
            ))]
            unmap_graphics_buffer(retired.buffer.raw(), retired.consumer_mapping);
            #[cfg(all(
                feature = "graphics-swapchain-runtime",
                not(feature = "graphics-owner-death-runtime"),
                not(feature = "app-crash-recovery-runtime")
            ))]
            unmap_graphics_buffer(
                retired.swapchain_buffer.raw(),
                retired.swapchain_consumer_mapping,
            );
            close_owned(retired.buffer);
            #[cfg(all(
                feature = "graphics-swapchain-runtime",
                not(feature = "graphics-owner-death-runtime"),
                not(feature = "app-crash-recovery-runtime")
            ))]
            close_owned(retired.swapchain_buffer);
            close_owned(retired.endpoint);
            surface.input_capture = None;
            surface.pointer_pressed = false;
        }
    }

    let ack = UiSupervisorControlMessage::ack(
        next_sequence(&mut surface.response_sequence),
        command.transaction_id(),
        operation,
        UiSupervisorStatus::Applied,
        app,
        identity,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if surface.tracker.accept(ack).is_err() {
        fail(FAIL_RUNTIME);
    }
    write_wire(surface.control.raw(), &ack.encode());
}

#[cfg(feature = "app-crash-recovery-runtime")]
fn handle_app_owner_death(surface: &mut LifecycleSurface, observed: u64) {
    if observed & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0 {
        fail(FAIL_RUNTIME);
    }
    let retired = surface.app.take().unwrap_or_else(|| fail(FAIL_RUNTIME));
    if surface.active_identity != Some(retired.identity) {
        fail(FAIL_RUNTIME);
    }

    // Commit the identity-coupled cleanup before publishing either focus or
    // owner-death notification. The compositor's global frame id deliberately
    // remains monotonic across the restart.
    surface.active_identity = None;
    surface.input_capture = None;
    surface.pointer_pressed = false;
    close_owned(retired.buffer);
    close_owned(retired.endpoint);
    surface.focus_generation = surface
        .focus_generation
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let focus = UiServerEvent::focus_changed(
        surface.session_id,
        UiClientId::Launcher,
        None,
        surface.focus_generation,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    send_ui_event(surface.launcher.raw(), focus);

    let owner_died = UiSupervisorControlMessage::owner_died(
        next_sequence(&mut surface.response_sequence),
        8,
        APP,
        retired.identity,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if surface.tracker.accept(owner_died).is_err() {
        fail(FAIL_RUNTIME);
    }
    write_wire(surface.control.raw(), &owner_died.encode());
}

#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn handle_graphics_producer_orphan(surface: &LifecycleSurface, observed: u64) -> ! {
    if observed & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
        || surface.active_identity.is_some()
        || surface.input_capture.is_some()
        || surface.pointer_pressed
    {
        fail(FAIL_RUNTIME);
    }
    let slot = surface.app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
    if slot.pending_frame.is_some()
        || slot.last_client_frame.is_some()
        || slot.buffer.raw() == slot.standby_buffer.raw()
        || slot.consumer_mapping == slot.standby_consumer_mapping
    {
        fail(FAIL_RUNTIME);
    }

    // The standby allocation never entered Queue, so its live server mapping
    // must still expose the initial WRITABLE state after the producer dies.
    // Slot 0 cannot be queried without mutating its Acquired state; its
    // earlier denied Unmap/Close probes and the completed Install ack are the
    // immutable proof that SurfaceServer still owns that acquisition here.
    wait_graphics_fence(slot.standby_buffer.raw(), ObjectSignals::WRITABLE);
    write_wire(
        surface.control.raw(),
        &GRAPHICS_PRODUCER_ORPHAN_READY_MAGIC.to_le_bytes(),
    );
    loop {
        core::hint::spin_loop();
    }
}

fn provision_placeholder_handle() -> OwnedUserHandle {
    let (left, right) = create_channel_owned();
    close_owned(right);
    left
}

fn receive_initial_app_buffer(
    endpoint: u64,
    identity: AppInstanceIdentity,
    focus_generation: u64,
) -> (OwnedUserHandle, BufferPresent) {
    let envelope = read_channel_envelope(endpoint);
    let expected_pid =
        (u64::from(identity.process().generation()) << 32) | u64::from(identity.process().pid());
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != BUFFER_PRESENT_WIRE_SIZE
        || envelope.sender_pid() != expected_pid
    {
        fail(FAIL_RUNTIME);
    }
    let buffer = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let frame = BufferPresent::decode(&envelope.data()[..BUFFER_PRESENT_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if frame.client_frame_id() != 1
        || frame.global_frame_id() != 1
        || u64::from(frame.focus_generation()) != focus_generation
        || frame.buffer_generation() != 1
    {
        close_owned(buffer);
        fail(FAIL_RUNTIME);
    }
    (buffer, frame)
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn receive_swapchain_initial_buffer(
    endpoint: u64,
    identity: AppInstanceIdentity,
    focus_generation: u64,
) -> (OwnedUserHandle, BufferPresent) {
    let envelope = read_channel_envelope(endpoint);
    let expected_pid =
        (u64::from(identity.process().generation()) << 32) | u64::from(identity.process().pid());
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != BUFFER_PRESENT_WIRE_SIZE
        || envelope.sender_pid() != expected_pid
    {
        fail(FAIL_RUNTIME);
    }
    let buffer = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let frame = BufferPresent::decode(&envelope.data()[..BUFFER_PRESENT_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if frame.client_frame_id() != 2
        || frame.global_frame_id() != 2
        || u64::from(frame.focus_generation()) != focus_generation
        || frame.buffer_generation() != 1
    {
        close_owned(buffer);
        fail(FAIL_RUNTIME);
    }
    (buffer, frame)
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn receive_swapchain_frame(
    endpoint: u64,
    identity: AppInstanceIdentity,
    focus_generation: u64,
    expected_client_frame: u32,
    expected_buffer_generation: u64,
) -> BufferPresent {
    let envelope = read_channel_envelope(endpoint);
    let expected_pid =
        (u64::from(identity.process().generation()) << 32) | u64::from(identity.process().pid());
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != BUFFER_PRESENT_WIRE_SIZE
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() != expected_pid
    {
        fail(FAIL_RUNTIME);
    }
    let frame = BufferPresent::decode(&envelope.data()[..BUFFER_PRESENT_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if frame.client_frame_id() != expected_client_frame
        || frame.global_frame_id() != expected_client_frame
        || u64::from(frame.focus_generation()) != focus_generation
        || frame.buffer_generation() != expected_buffer_generation
    {
        fail(FAIL_RUNTIME);
    }
    frame
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn receive_swapchain_magic(endpoint: u64, identity: AppInstanceIdentity, expected_magic: u64) {
    let envelope = read_channel_envelope(endpoint);
    let expected_pid =
        (u64::from(identity.process().generation()) << 32) | u64::from(identity.process().pid());
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != size_of::<u64>()
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() != expected_pid
        || envelope.data()[..size_of::<u64>()] != expected_magic.to_le_bytes()
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn receive_orphan_standby_buffer(
    endpoint: u64,
    identity: AppInstanceIdentity,
    focus_generation: u64,
) -> OwnedUserHandle {
    let envelope = read_channel_envelope(endpoint);
    let expected_pid =
        (u64::from(identity.process().generation()) << 32) | u64::from(identity.process().pid());
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != BUFFER_PRESENT_WIRE_SIZE
        || envelope.sender_pid() != expected_pid
    {
        fail(FAIL_RUNTIME);
    }
    let buffer = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let descriptor = BufferPresent::decode(&envelope.data()[..BUFFER_PRESENT_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if descriptor.client_frame_id() != 2
        || descriptor.global_frame_id() != 2
        || u64::from(descriptor.focus_generation()) != focus_generation
        || descriptor.buffer_generation() != 1
    {
        close_owned(buffer);
        fail(FAIL_RUNTIME);
    }
    buffer
}

#[cfg(all(
    feature = "graphics-surface-restart-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
fn receive_rebound_app_buffer(
    endpoint: u64,
    identity: AppInstanceIdentity,
    focus_generation: u64,
) -> (OwnedUserHandle, BufferPresent) {
    let envelope = read_channel_envelope(endpoint);
    let expected_pid =
        (u64::from(identity.process().generation()) << 32) | u64::from(identity.process().pid());
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != BUFFER_PRESENT_WIRE_SIZE
        || envelope.sender_pid() != expected_pid
    {
        fail(FAIL_RUNTIME);
    }
    let buffer = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let frame = BufferPresent::decode(&envelope.data()[..BUFFER_PRESENT_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if frame.client_frame_id() != 1
        || frame.global_frame_id() != 1
        || u64::from(frame.focus_generation()) != focus_generation
        || frame.buffer_generation() != 2
    {
        close_owned(buffer);
        fail(FAIL_RUNTIME);
    }
    (buffer, frame)
}

#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
fn receive_followup_app_buffer(
    endpoint: u64,
    identity: AppInstanceIdentity,
    focus_generation: u64,
) -> BufferPresent {
    let envelope = read_channel_envelope(endpoint);
    let expected_pid =
        (u64::from(identity.process().generation()) << 32) | u64::from(identity.process().pid());
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != BUFFER_PRESENT_WIRE_SIZE
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() != expected_pid
    {
        fail(FAIL_RUNTIME);
    }
    let frame = BufferPresent::decode(&envelope.data()[..BUFFER_PRESENT_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    // Cancellation makes client frame 1 retryable.  Queue generation, not a
    // second client id, distinguishes the directly rendered replacement.
    if frame.client_frame_id() != 1
        || frame.global_frame_id() != 1
        || u64::from(frame.focus_generation()) != focus_generation
        || frame.buffer_generation() != 2
    {
        fail(FAIL_RUNTIME);
    }
    frame
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-swapchain-runtime"),
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn receive_frame_clock_app_buffer(
    endpoint: u64,
    identity: AppInstanceIdentity,
    focus_generation: u64,
    expected_client_frame: u32,
    expected_buffer_generation: u64,
) -> BufferPresent {
    let envelope = read_channel_envelope(endpoint);
    let expected_pid =
        (u64::from(identity.process().generation()) << 32) | u64::from(identity.process().pid());
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != BUFFER_PRESENT_WIRE_SIZE
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() != expected_pid
    {
        fail(FAIL_RUNTIME);
    }
    let frame = BufferPresent::decode(&envelope.data()[..BUFFER_PRESENT_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if frame.client_frame_id() != expected_client_frame
        || frame.global_frame_id() != expected_client_frame
        || u64::from(frame.focus_generation()) != focus_generation
        || frame.buffer_generation() != expected_buffer_generation
    {
        fail(FAIL_RUNTIME);
    }
    frame
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-swapchain-runtime"),
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn acquire_frame_clock_app_buffer(
    surface: &mut LifecycleSurface,
    expected_client_frame: u32,
    expected_buffer_generation: u64,
) -> BufferPresent {
    let slot = surface.app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
    if surface.active_identity != Some(slot.identity)
        || surface.focus_generation != active_focus_generation(slot.identity.instance_id())
        || slot.last_client_frame
            != expected_client_frame
                .checked_sub(1)
                .and_then(|previous| (previous != 0).then_some(previous))
    {
        fail(FAIL_RUNTIME);
    }
    let endpoint = slot.endpoint.raw();
    let buffer = slot.buffer.raw();
    let identity = slot.identity;
    let consumer_mapping = slot.consumer_mapping;
    wait_graphics_fence(buffer, ObjectSignals::READABLE);
    let frame = receive_frame_clock_app_buffer(
        endpoint,
        identity,
        surface.focus_generation,
        expected_client_frame,
        expected_buffer_generation,
    );
    acquire_graphics_buffer(buffer, expected_buffer_generation);
    verify_mapped_frame_samples(
        consumer_mapping,
        identity.instance_id(),
        expected_buffer_generation,
    );
    // This authenticated edge can be emitted only after the producer's
    // zero-time WRITABLE poll returned Timeout. SurfaceServer therefore
    // cannot race a fast grant/present ahead of the backpressure proof.
    receive_frame_clock_backpressure(endpoint, identity, expected_client_frame);
    frame
}

#[cfg(any(
    feature = "input-server-restart-runtime",
    all(
        feature = "mapped-graphics-runtime",
        not(feature = "app-crash-recovery-runtime")
    )
))]
fn map_graphics_buffer(handle: u64, role: u64) -> u64 {
    let mapped = syscall(SyscallNumber::GraphicsBufferMap, handle, role, 0);
    let map_limit = GRAPHICS_BUFFER_MAP_ADDRESS
        .checked_add(
            GRAPHICS_BUFFER_MAP_STRIDE
                .checked_mul(2)
                .unwrap_or_else(|| fail(FAIL_RUNTIME)),
        )
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if mapped.status != Status::Ok.raw()
        || mapped.out1 < GRAPHICS_BUFFER_MAP_ADDRESS
        || mapped.out1 >= map_limit
        || !mapped.out1.is_multiple_of(4096)
        || mapped.out2 != GRAPHICS_BUFFER_BACKING_BYTES as u64
    {
        fail(FAIL_RUNTIME);
    }
    mapped.out1
}

#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
fn unmap_graphics_buffer(handle: u64, address: u64) {
    let unmapped = syscall(SyscallNumber::GraphicsBufferUnmap, handle, address, 0);
    if unmapped.status != Status::Ok.raw() || unmapped.out1 != 0 || unmapped.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(any(
    feature = "input-server-restart-runtime",
    all(
        feature = "mapped-graphics-runtime",
        not(feature = "app-crash-recovery-runtime")
    )
))]
fn queue_graphics_buffer(handle: u64, expected_generation: u64) -> u64 {
    let queued = syscall(
        SyscallNumber::GraphicsBufferQueue,
        handle,
        expected_generation,
        GRAPHICS_BUFFER_QUEUE_FLAGS_NONE,
    );
    let next = expected_generation
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if queued.status != Status::Ok.raw() || queued.out1 != next || queued.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    queued.out1
}

#[cfg(any(
    feature = "input-server-restart-runtime",
    all(
        feature = "mapped-graphics-runtime",
        not(feature = "app-crash-recovery-runtime")
    )
))]
fn acquire_graphics_buffer(handle: u64, expected_generation: u64) {
    let acquired = syscall(
        SyscallNumber::GraphicsBufferAcquire,
        handle,
        expected_generation,
        GRAPHICS_BUFFER_ACQUIRE_FLAGS_NONE,
    );
    if acquired.status != Status::Ok.raw()
        || acquired.out1 != expected_generation
        || acquired.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
fn release_graphics_buffer(handle: u64, expected_generation: u64) {
    let released = syscall(
        SyscallNumber::GraphicsBufferRelease,
        handle,
        expected_generation,
        GRAPHICS_BUFFER_RELEASE_FLAGS_NONE,
    );
    if released.status != Status::Ok.raw()
        || released.out1 != expected_generation
        || released.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
fn wait_graphics_fence(handle: u64, signal: ObjectSignals) {
    let waited = object_wait(handle, signal);
    if waited.status != Status::Ok.raw()
        || waited.out1 & u64::from(signal.bits()) == 0
        || waited.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn poll_graphics_backpressure(handle: u64) {
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    items[0] = pack_user_wait_item(handle, ObjectSignals::WRITABLE);
    let polled = object_wait_many_array(&items, 1, OBJECT_WAIT_TIMEOUT_POLL);
    if polled.status != Status::Timeout.raw() || polled.out1 != u64::MAX || polled.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-swapchain-runtime"),
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
const fn frame_clock_backpressure_magic(client_frame_id: u32) -> u64 {
    FRAME_CLOCK_BACKPRESSURE_MAGIC | client_frame_id as u64
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-swapchain-runtime"),
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn receive_frame_clock_backpressure(
    endpoint: u64,
    identity: AppInstanceIdentity,
    client_frame_id: u32,
) {
    let envelope = read_channel_envelope(endpoint);
    let expected_pid =
        (u64::from(identity.process().generation()) << 32) | u64::from(identity.process().pid());
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != size_of::<u64>()
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() != expected_pid
        || envelope.data()[..size_of::<u64>()]
            != frame_clock_backpressure_magic(client_frame_id).to_le_bytes()
    {
        fail(FAIL_RUNTIME);
    }
}

fn publish_surface_focus(surface: &LifecycleSurface, client: UiClientId, app: Option<ShellAppId>) {
    let event =
        UiServerEvent::focus_changed(surface.session_id, client, app, surface.focus_generation)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    send_ui_event(surface.launcher.raw(), event);
    if let Some(slot) = surface.app.as_ref() {
        send_ui_event(slot.endpoint.raw(), event);
    }
}

#[cfg(not(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
)))]
fn present_pending_app_frame(surface: &mut LifecycleSurface) {
    #[cfg(all(
        feature = "graphics-frame-clock-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    let frame = acquire_frame_clock_app_buffer(surface, 1, 2);
    #[cfg(all(
        feature = "mapped-graphics-runtime",
        not(feature = "graphics-surface-restart-runtime"),
        not(feature = "app-crash-recovery-runtime"),
        any(
            not(feature = "graphics-frame-clock-runtime"),
            feature = "graphics-owner-death-runtime"
        )
    ))]
    let frame = {
        let slot = surface.app.as_mut().unwrap_or_else(|| fail(FAIL_RUNTIME));
        // Resume reuses the already committed frame. Only the first Activate
        // of an App generation has a queued follow-up frame to acquire; an
        // unconditional wait here would deadlock against a producer whose
        // phase is already `Presented`.
        if slot.last_client_frame.is_some() {
            return;
        }
        wait_graphics_fence(slot.buffer.raw(), ObjectSignals::READABLE);
        let frame = receive_followup_app_buffer(
            slot.endpoint.raw(),
            slot.identity,
            surface.focus_generation,
        );
        acquire_graphics_buffer(slot.buffer.raw(), frame.buffer_generation());
        verify_mapped_frame_samples(
            slot.consumer_mapping,
            slot.identity.instance_id(),
            frame.buffer_generation(),
        );
        frame
    };
    #[cfg(all(
        feature = "graphics-surface-restart-runtime",
        not(feature = "app-crash-recovery-runtime")
    ))]
    let frame = {
        let slot = surface.app.as_mut().unwrap_or_else(|| fail(FAIL_RUNTIME));
        if slot.last_client_frame.is_some() {
            return;
        }
        slot.pending_frame
            .take()
            .unwrap_or_else(|| fail(FAIL_RUNTIME))
    };
    #[cfg(not(all(
        feature = "mapped-graphics-runtime",
        not(feature = "app-crash-recovery-runtime")
    )))]
    let frame = {
        let slot = surface.app.as_mut().unwrap_or_else(|| fail(FAIL_RUNTIME));
        let Some(frame) = slot.pending_frame.take() else {
            return;
        };
        frame
    };
    commit_app_frame(surface, frame);
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn present_pending_app_frame(surface: &mut LifecycleSurface) {
    present_swapchain_frames(surface);
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn present_swapchain_frames(surface: &mut LifecycleSurface) {
    let (endpoint, identity, buffer_a, buffer_b, mapping_a, mapping_b) = {
        let slot = surface.app.as_ref().unwrap_or_else(|| fail(FAIL_RUNTIME));
        if surface.active_identity != Some(slot.identity)
            || surface.focus_generation != active_focus_generation(slot.identity.instance_id())
            || slot.last_client_frame.is_some()
        {
            fail(FAIL_RUNTIME);
        }
        (
            slot.endpoint.raw(),
            slot.identity,
            slot.buffer.raw(),
            slot.swapchain_buffer.raw(),
            slot.consumer_mapping,
            slot.swapchain_consumer_mapping,
        )
    };

    let frame_1 = receive_swapchain_frame(endpoint, identity, surface.focus_generation, 1, 2);
    let frame_2 = receive_swapchain_frame(endpoint, identity, surface.focus_generation, 2, 2);
    receive_swapchain_magic(endpoint, identity, SWAPCHAIN_ACTIVE_BATCH_MAGIC);
    wait_graphics_fence(buffer_a, ObjectSignals::READABLE);
    wait_graphics_fence(buffer_b, ObjectSignals::READABLE);
    acquire_graphics_buffer(buffer_a, 2);
    verify_swapchain_frame_samples(mapping_a, identity.instance_id(), 0, 1, 2);
    acquire_graphics_buffer(buffer_b, 2);
    verify_swapchain_frame_samples(mapping_b, identity.instance_id(), 1, 2, 2);

    let duplicate = syscall(
        SyscallNumber::GraphicsBufferAcquire,
        buffer_a,
        2,
        GRAPHICS_BUFFER_ACQUIRE_FLAGS_NONE,
    );
    if duplicate.status != Status::InvalidState.raw() || duplicate.out1 != 0 || duplicate.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
    commit_paced_swapchain_frame(surface, buffer_a, frame_1);

    let frame_3 = receive_swapchain_frame(endpoint, identity, surface.focus_generation, 3, 3);
    receive_swapchain_magic(endpoint, identity, SWAPCHAIN_REFILL_A_MAGIC);
    wait_graphics_fence(buffer_a, ObjectSignals::READABLE);
    acquire_graphics_buffer(buffer_a, 3);
    verify_swapchain_frame_samples(mapping_a, identity.instance_id(), 0, 3, 3);
    commit_paced_swapchain_frame(surface, buffer_b, frame_2);

    let frame_4 = receive_swapchain_frame(endpoint, identity, surface.focus_generation, 4, 3);
    receive_swapchain_magic(endpoint, identity, SWAPCHAIN_REFILL_B_MAGIC);
    wait_graphics_fence(buffer_b, ObjectSignals::READABLE);
    acquire_graphics_buffer(buffer_b, 3);
    verify_swapchain_frame_samples(mapping_b, identity.instance_id(), 1, 4, 3);
    commit_paced_swapchain_frame(surface, buffer_a, frame_3);

    let pending = frame_4
        .with_frame_id(4, 4)
        .and_then(|frame| {
            frame.with_focus_generation(
                u32::try_from(surface.focus_generation)
                    .map_err(|_| bndr_ui::BufferPresentError::ZeroFocusGeneration)?,
            )
        })
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let pending_wire = pending.encode();
    let denied = syscall(
        SyscallNumber::SurfacePresentBuffer,
        surface.capability.raw(),
        buffer_b,
        pending_wire.as_ptr() as u64,
    );
    if denied.status != Status::InvalidState.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    receive_swapchain_magic(endpoint, identity, SWAPCHAIN_FINAL_MIXED_MAGIC);
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn commit_paced_swapchain_frame(surface: &mut LifecycleSurface, buffer: u64, frame: BufferPresent) {
    let global_frame = surface
        .global_frame_id
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if frame.client_frame_id() != global_frame || !(1..=3).contains(&global_frame) {
        fail(FAIL_RUNTIME);
    }
    let frame = frame
        .with_frame_id(frame.client_frame_id(), global_frame)
        .and_then(|frame| {
            frame.with_focus_generation(
                u32::try_from(surface.focus_generation)
                    .map_err(|_| bndr_ui::BufferPresentError::ZeroFocusGeneration)?,
            )
        })
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let wire = frame.encode();

    let denied = syscall(
        SyscallNumber::SurfacePresentBuffer,
        surface.capability.raw(),
        buffer,
        wire.as_ptr() as u64,
    );
    if denied.status != Status::InvalidState.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_RUNTIME);
    }

    let ready = object_wait(surface.capability.raw(), ObjectSignals::FRAME_READY);
    if ready.status != Status::Ok.raw()
        || ready.out1 & u64::from(ObjectSignals::FRAME_READY.bits()) == 0
        || ready.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
    let acquired = syscall(
        SyscallNumber::SurfaceFrameAcquire,
        surface.capability.raw(),
        0,
        0,
    );
    let expected_epoch = surface
        .last_frame_epoch
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if acquired.status != Status::Ok.raw()
        || acquired.out1 != expected_epoch
        || acquired.out2 == 0
        || acquired.out2 <= surface.last_frame_boundary
    {
        fail(FAIL_RUNTIME);
    }
    surface.last_frame_epoch = acquired.out1;
    surface.last_frame_boundary = acquired.out2;

    if global_frame == 1 {
        let duplicate = syscall(
            SyscallNumber::SurfaceFrameAcquire,
            surface.capability.raw(),
            0,
            0,
        );
        if duplicate.status != Status::InvalidState.raw()
            || duplicate.out1 != 0
            || duplicate.out2 != 0
        {
            fail(FAIL_RUNTIME);
        }
        let wrong = frame
            .with_frame_id(frame.client_frame_id(), 2)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        let wrong_wire = wrong.encode();
        let rejected = syscall(
            SyscallNumber::SurfacePresentBuffer,
            surface.capability.raw(),
            buffer,
            wrong_wire.as_ptr() as u64,
        );
        if rejected.status != Status::InvalidState.raw() || rejected.out1 != 0 || rejected.out2 != 0
        {
            fail(FAIL_RUNTIME);
        }
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(surface.capability.raw(), ObjectSignals::PEER_CLOSED);
        let late = object_wait_many_array(&items, 1, FRAME_CLOCK_LATE_TIMEOUT_NS);
        if late.status != Status::Timeout.raw() || late.out1 != u64::MAX || late.out2 != 0 {
            fail(FAIL_RUNTIME);
        }
    }

    let presented = syscall(
        SyscallNumber::SurfacePresentBuffer,
        surface.capability.raw(),
        buffer,
        wire.as_ptr() as u64,
    );
    if presented.status != Status::Ok.raw()
        || presented.out1 != u64::from(global_frame)
        || presented.out2 != u64::from(global_frame)
    {
        fail(FAIL_RUNTIME);
    }
    let slot = surface.app.as_mut().unwrap_or_else(|| fail(FAIL_RUNTIME));
    slot.last_client_frame = Some(frame.client_frame_id());
    surface.global_frame_id = global_frame;
    send_ui_event(
        slot.endpoint.raw(),
        UiServerEvent::presented(surface.session_id, frame.client_frame_id(), presented.out2)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
    );
}

#[cfg(not(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
)))]
fn commit_app_frame(surface: &mut LifecycleSurface, frame: BufferPresent) {
    let global_frame = surface
        .global_frame_id
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let frame = frame
        .with_frame_id(frame.client_frame_id(), global_frame)
        .and_then(|frame| {
            frame.with_focus_generation(
                u32::try_from(surface.focus_generation)
                    .map_err(|_| bndr_ui::BufferPresentError::ZeroFocusGeneration)?,
            )
        })
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let wire = frame.encode();
    let slot = surface.app.as_mut().unwrap_or_else(|| fail(FAIL_RUNTIME));
    let presented = syscall(
        SyscallNumber::SurfacePresentBuffer,
        surface.capability.raw(),
        slot.buffer.raw(),
        wire.as_ptr() as u64,
    );
    if presented.status != Status::Ok.raw()
        || presented.out1 != u64::from(global_frame)
        || presented.out2 == 0
    {
        fail(FAIL_RUNTIME);
    }
    slot.last_client_frame = Some(frame.client_frame_id());
    surface.global_frame_id = global_frame;
    send_ui_event(
        slot.endpoint.raw(),
        UiServerEvent::presented(surface.session_id, frame.client_frame_id(), presented.out2)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
    );
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-swapchain-runtime"),
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn commit_app_frame(surface: &mut LifecycleSurface, frame: BufferPresent) {
    let global_frame = surface
        .global_frame_id
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if frame.client_frame_id() != global_frame || !(1..=3).contains(&global_frame) {
        fail(FAIL_RUNTIME);
    }
    let frame = frame
        .with_frame_id(frame.client_frame_id(), global_frame)
        .and_then(|frame| {
            frame.with_focus_generation(
                u32::try_from(surface.focus_generation)
                    .map_err(|_| bndr_ui::BufferPresentError::ZeroFocusGeneration)?,
            )
        })
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let wire = frame.encode();
    let buffer = surface
        .app
        .as_ref()
        .unwrap_or_else(|| fail(FAIL_RUNTIME))
        .buffer
        .raw();

    // Every committed frame first proves that an Acquired buffer alone is
    // insufficient: without a frame-clock grant the syscall must reject the
    // transaction without releasing the producer fence or advancing Surface.
    let denied = syscall(
        SyscallNumber::SurfacePresentBuffer,
        surface.capability.raw(),
        buffer,
        wire.as_ptr() as u64,
    );
    if denied.status != Status::InvalidState.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_RUNTIME);
    }

    let ready = object_wait(surface.capability.raw(), ObjectSignals::FRAME_READY);
    if ready.status != Status::Ok.raw()
        || ready.out1 & u64::from(ObjectSignals::FRAME_READY.bits()) == 0
        || ready.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
    let acquired = syscall(
        SyscallNumber::SurfaceFrameAcquire,
        surface.capability.raw(),
        0,
        0,
    );
    let expected_epoch = surface
        .last_frame_epoch
        .checked_add(1)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    if acquired.status != Status::Ok.raw()
        || acquired.out1 != expected_epoch
        || acquired.out2 == 0
        || acquired.out2 <= surface.last_frame_boundary
    {
        fail(FAIL_RUNTIME);
    }
    surface.last_frame_epoch = acquired.out1;
    surface.last_frame_boundary = acquired.out2;

    if global_frame == 1 {
        // A syntactically valid but out-of-sequence first frame reaches the
        // display transaction and fails. No second FrameAcquire is issued:
        // the correct retry below can succeed only if that failure preserved
        // the exact outstanding grant.
        let wrong = frame
            .with_frame_id(frame.client_frame_id(), 2)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME));
        let wrong_wire = wrong.encode();
        let rejected = syscall(
            SyscallNumber::SurfacePresentBuffer,
            surface.capability.raw(),
            buffer,
            wrong_wire.as_ptr() as u64,
        );
        if rejected.status != Status::InvalidState.raw() || rejected.out1 != 0 || rejected.out2 != 0
        {
            fail(FAIL_RUNTIME);
        }

        // Wait on a signal that cannot become ready in the healthy run. The
        // finite timeout crosses at least one software-clock interval while
        // the grant remains outstanding, proving late intervals are
        // suppressed rather than publishing a second concurrent grant.
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(surface.capability.raw(), ObjectSignals::PEER_CLOSED);
        let late = object_wait_many_array(&items, 1, FRAME_CLOCK_LATE_TIMEOUT_NS);
        if late.status != Status::Timeout.raw() || late.out1 != u64::MAX || late.out2 != 0 {
            fail(FAIL_RUNTIME);
        }
    }

    let presented = syscall(
        SyscallNumber::SurfacePresentBuffer,
        surface.capability.raw(),
        buffer,
        wire.as_ptr() as u64,
    );
    if presented.status != Status::Ok.raw()
        || presented.out1 != u64::from(global_frame)
        || presented.out2 != u64::from(global_frame)
    {
        fail(FAIL_RUNTIME);
    }
    let slot = surface.app.as_mut().unwrap_or_else(|| fail(FAIL_RUNTIME));
    slot.last_client_frame = Some(frame.client_frame_id());
    surface.global_frame_id = global_frame;
    send_ui_event(
        slot.endpoint.raw(),
        UiServerEvent::presented(surface.session_id, frame.client_frame_id(), presented.out2)
            .unwrap_or_else(|_| fail(FAIL_RUNTIME)),
    );
}

#[cfg(all(
    feature = "graphics-frame-clock-runtime",
    not(feature = "graphics-swapchain-runtime"),
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn handle_frame_clock_app_frame(surface: &mut LifecycleSurface, observed: u64) {
    if observed & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
        || observed & u64::from(ObjectSignals::READABLE.bits()) == 0
    {
        fail(FAIL_RUNTIME);
    }
    let last = surface
        .app
        .as_ref()
        .and_then(|slot| slot.last_client_frame)
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    let expected_client_frame = last.checked_add(1).unwrap_or_else(|| fail(FAIL_RUNTIME));
    if !(2..=3).contains(&expected_client_frame) {
        fail(FAIL_RUNTIME);
    }
    let expected_buffer_generation = u64::from(expected_client_frame) + 1;
    let frame =
        acquire_frame_clock_app_buffer(surface, expected_client_frame, expected_buffer_generation);
    commit_app_frame(surface, frame);
}

fn consume_surface_input(surface: &mut LifecycleSurface) {
    let input = syscall(
        SyscallNumber::SurfaceReadInput,
        surface.capability.raw(),
        0,
        0,
    );
    if input.status == Status::ShouldWait.raw() {
        return;
    }
    if input.status != Status::Ok.raw() {
        fail(FAIL_RUNTIME);
    }
    let sample = InputSample::decode_registers(input.out1, input.out2)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let target_identity = if surface.pointer_pressed {
        surface.input_capture
    } else {
        surface.active_identity
    };
    match (surface.pointer_pressed, sample.pressed()) {
        (false, true) => {
            surface.pointer_pressed = true;
            surface.input_capture = surface.active_identity;
        }
        (true, false) => {
            surface.pointer_pressed = false;
            surface.input_capture = None;
        }
        _ => {}
    }
    let event = UiServerEvent::input_sample(surface.session_id, sample)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if let Some(identity) = target_identity.or(surface.input_capture) {
        let slot = surface.app.as_mut().unwrap_or_else(|| fail(FAIL_RUNTIME));
        if slot.identity != identity || surface.active_identity != Some(identity) {
            fail(FAIL_RUNTIME);
        }
        slot.input_sequence = slot
            .input_sequence
            .checked_add(1)
            .unwrap_or_else(|| fail(FAIL_RUNTIME));
        send_ui_event(slot.endpoint.raw(), event);
    } else {
        send_ui_event(surface.launcher.raw(), event);
    }
}

fn send_ui_event(transport: u64, event: UiServerEvent) {
    write_wire(transport, &event.encode());
}

pub(super) fn launcher_runtime(startup: u64) -> ! {
    let denied = syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0);
    if denied.status != Status::PermissionDenied.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    let lifecycle = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let (init_pid, initial_ui) =
        read_bootstrap_transfer(lifecycle.raw(), UiBootstrapEndpointKind::LauncherUi);
    #[cfg(all(
        feature = "graphics-surface-restart-runtime",
        not(feature = "app-crash-recovery-runtime")
    ))]
    let mut ui = initial_ui;
    #[cfg(not(all(
        feature = "graphics-surface-restart-runtime",
        not(feature = "app-crash-recovery-runtime")
    )))]
    let ui = initial_ui;
    let mut sequence = bndr_ui::AppLifecycleSequenceTracker::new();
    let mut ui_tracker = UiServerEventTracker::new();
    let mut surface_pid = None;
    let mut transaction_id = 1_u64;
    let mut request_sequence = 0;
    let mut state_sequence = 0;
    let mut identity = None;
    let mut intermediate_seen = false;
    send_launcher_request(
        lifecycle.raw(),
        &mut sequence,
        &mut request_sequence,
        transaction_id,
        AppLifecycleAction::Launch,
        identity,
    );

    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    loop {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        // During M37 recovery the old UI endpoint may become PEER_CLOSED at
        // the same time Init queues its replacement UBP on `lifecycle`.
        // ObjectWaitManyArray deliberately returns the lowest ready index, so
        // put UI first in that profile and retire it before decoding anything
        // from the stable recovery root. Other profiles retain their original
        // lifecycle-first arbitration.
        let recovery_ui_first = cfg!(all(
            feature = "graphics-surface-restart-runtime",
            not(feature = "app-crash-recovery-runtime")
        ));
        let (lifecycle_index, ui_index) = if recovery_ui_first { (1, 0) } else { (0, 1) };
        items[lifecycle_index] = pack_user_wait_item(lifecycle.raw(), requested);
        items[ui_index] = pack_user_wait_item(ui.raw(), requested);
        let ready = object_wait_many_array(&items, 2, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw()
            || ready.out1 > 1
            || ready.out2 & u64::from(requested.bits()) == 0
        {
            fail(FAIL_RUNTIME);
        }
        let ready_index = usize::try_from(ready.out1).unwrap_or_else(|_| fail(FAIL_RUNTIME));
        #[cfg(all(
            feature = "graphics-producer-orphan-runtime",
            not(feature = "graphics-surface-restart-runtime"),
            not(feature = "app-crash-recovery-runtime")
        ))]
        if transaction_id > TRANSACTION_COUNT
            && (ready_index == lifecycle_index
                || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0)
        {
            if ready_index == lifecycle_index
                && ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
            {
                fail(FAIL_RUNTIME);
            }
            reuse_orphaned_graphics_slots(lifecycle, ui, init_pid);
        }
        if ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
            #[cfg(all(
                feature = "graphics-surface-restart-runtime",
                not(feature = "app-crash-recovery-runtime")
            ))]
            {
                if ready_index != ui_index {
                    fail(FAIL_RUNTIME);
                }
                close_owned(ui);
                let (sender_pid, rebound) =
                    read_bootstrap_transfer(lifecycle.raw(), UiBootstrapEndpointKind::LauncherUi);
                if sender_pid != init_pid {
                    close_owned(rebound);
                    fail(FAIL_RUNTIME);
                }
                ui = rebound;
                ui_tracker = UiServerEventTracker::new();
                surface_pid = None;
                continue;
            }
            #[cfg(all(
                feature = "graphics-owner-death-runtime",
                not(feature = "graphics-surface-restart-runtime"),
                not(feature = "graphics-producer-orphan-runtime")
            ))]
            loop {
                core::hint::spin_loop();
            }
            #[cfg(not(feature = "graphics-owner-death-runtime"))]
            fail(FAIL_RUNTIME);
        }
        if ready_index == ui_index {
            let envelope = read_channel_envelope_now(ui.raw());
            if envelope.kind() != ChannelMessageKind::Bytes
                || envelope.logical_length() != UI_SERVER_EVENT_WIRE_SIZE
                || envelope.received_handle().is_valid()
                || envelope.sender_pid() == 0
                || surface_pid.is_some_and(|pid| pid != envelope.sender_pid())
            {
                fail(FAIL_RUNTIME);
            }
            surface_pid = Some(envelope.sender_pid());
            let event = UiServerEvent::decode(&envelope.data()[..UI_SERVER_EVENT_WIRE_SIZE])
                .unwrap_or_else(|_| fail(FAIL_RUNTIME));
            if ui_tracker.accept(event).is_err()
                || !matches!(
                    event.payload(),
                    UiServerEventPayload::Ready
                        | UiServerEventPayload::Input(_)
                        | UiServerEventPayload::FocusChanged { .. }
                )
            {
                fail(FAIL_RUNTIME);
            }
            continue;
        }

        if transaction_id > TRANSACTION_COUNT {
            fail(FAIL_RUNTIME);
        }
        let (message, transfer) = read_lifecycle_message(lifecycle.raw(), init_pid, false);
        if transfer.is_some()
            || message.sender_sequence() != next_sequence(&mut state_sequence)
            || message.transaction_id() != transaction_id
            || sequence.accept(message).is_err()
        {
            fail(FAIL_RUNTIME);
        }
        let AppLifecyclePayload::StateChanged {
            app,
            state,
            reason,
            identity: state_identity,
        } = message.payload()
        else {
            fail(FAIL_RUNTIME);
        };
        if app != APP || state_identity.is_none() {
            fail(FAIL_RUNTIME);
        }
        let state_identity = state_identity.unwrap_or_else(|| fail(FAIL_RUNTIME));

        #[cfg(feature = "app-crash-recovery-runtime")]
        if transaction_id == 8 {
            if intermediate_seen
                || identity != Some(state_identity)
                || state != AppLifecycleState::Crashed
                || reason != AppLifecycleReason::ProcessExited
            {
                fail(FAIL_RUNTIME);
            }
            identity = None;
            transaction_id = 9;
            send_launcher_request(
                lifecycle.raw(),
                &mut sequence,
                &mut request_sequence,
                transaction_id,
                AppLifecycleAction::Launch,
                None,
            );
            continue;
        }

        let action =
            lifecycle_action_for_transaction(transaction_id).unwrap_or_else(|| fail(FAIL_RUNTIME));
        if !intermediate_seen {
            if state != intermediate_state_for(action)
                || reason != AppLifecycleReason::Requested
                || (action != AppLifecycleAction::Launch && identity != Some(state_identity))
            {
                fail(FAIL_RUNTIME);
            }
            if action == AppLifecycleAction::Launch {
                identity = Some(state_identity);
            }
            intermediate_seen = true;
            continue;
        }
        if state != completed_state_for(action)
            || reason != AppLifecycleReason::Completed
            || identity != Some(state_identity)
        {
            fail(FAIL_RUNTIME);
        }
        if action == AppLifecycleAction::Terminate {
            identity = None;
        }
        intermediate_seen = false;
        transaction_id += 1;
        if let Some(next) = lifecycle_action_for_transaction(transaction_id) {
            send_launcher_request(
                lifecycle.raw(),
                &mut sequence,
                &mut request_sequence,
                transaction_id,
                next,
                if next == AppLifecycleAction::Launch {
                    None
                } else {
                    identity
                },
            );
        } else if transaction_id > TRANSACTION_COUNT && identity.is_none() {
            fail(FAIL_RUNTIME);
        }
    }
}

#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn reuse_orphaned_graphics_slots(
    lifecycle: OwnedUserHandle,
    old_ui: OwnedUserHandle,
    init_pid: u64,
) -> ! {
    let peer_closed = object_wait(old_ui.raw(), ObjectSignals::PEER_CLOSED);
    if peer_closed.status != Status::Ok.raw()
        || peer_closed.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
        || peer_closed.out2 != 0
    {
        fail(FAIL_RUNTIME);
    }
    read_authenticated_magic(
        lifecycle.raw(),
        init_pid,
        GRAPHICS_PRODUCER_REUSE_REQUEST_MAGIC,
    );
    close_owned(old_ui);

    let (first, first_mapping) = create_reused_graphics_buffer();
    let (second, second_mapping) = create_reused_graphics_buffer();
    if first.raw() == second.raw() || first_mapping == second_mapping {
        fail(FAIL_RUNTIME);
    }
    let exhausted = syscall(
        SyscallNumber::GraphicsBufferCreate,
        GRAPHICS_BUFFER_FORMAT_XRGB8888,
        pack_graphics_buffer_geometry(GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_HEIGHT),
        GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE,
    );
    if exhausted.status != Status::OutOfMemory.raw() || exhausted.out1 != 0 || exhausted.out2 != 0 {
        fail(FAIL_RUNTIME);
    }

    // Recycling is accepted only after a byte-for-byte volatile proof of the
    // complete page-rounded backings, including the 1,024-byte padding tails.
    verify_complete_backing_zero(first_mapping);
    verify_complete_backing_zero(second_mapping);
    render_mapped_frame(first_mapping, 1, 1);
    render_mapped_frame(second_mapping, 2, 2);
    verify_complete_mapped_frame(first_mapping, 1, 1);
    verify_complete_mapped_frame(second_mapping, 2, 2);

    unmap_graphics_buffer(first.raw(), first_mapping);
    unmap_graphics_buffer(second.raw(), second_mapping);
    close_owned(first);
    close_owned(second);
    write_wire(
        lifecycle.raw(),
        &GRAPHICS_PRODUCER_REUSE_DONE_MAGIC.to_le_bytes(),
    );

    let _ = lifecycle;
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn create_reused_graphics_buffer() -> (OwnedUserHandle, u64) {
    let created = syscall(
        SyscallNumber::GraphicsBufferCreate,
        GRAPHICS_BUFFER_FORMAT_XRGB8888,
        pack_graphics_buffer_geometry(GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_HEIGHT),
        GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE,
    );
    if created.status != Status::Ok.raw()
        || created.out1 == 0
        || created.out2 != GRAPHICS_BUFFER_LOGICAL_BYTES as u64
    {
        fail(FAIL_RUNTIME);
    }
    let buffer = OwnedUserHandle::new(created.out1).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let mapping = map_graphics_buffer(buffer.raw(), GRAPHICS_BUFFER_MAP_PRODUCER_RW);
    (buffer, mapping)
}

#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn verify_complete_backing_zero(address: u64) {
    let address = usize::try_from(address).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if !address.is_multiple_of(4096) {
        fail(FAIL_RUNTIME);
    }
    for offset in 0..GRAPHICS_BUFFER_BACKING_BYTES {
        if unsafe { core::ptr::read_volatile((address as *const u8).add(offset)) } != 0 {
            fail(FAIL_RUNTIME);
        }
    }
}

#[cfg(all(
    feature = "graphics-producer-orphan-runtime",
    not(feature = "graphics-surface-restart-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn verify_complete_mapped_frame(address: u64, instance_id: u64, generation: u64) {
    let address = usize::try_from(address).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let instance = u32::try_from(instance_id).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if !address.is_multiple_of(4096) || !matches!(generation, 1 | 2) {
        fail(FAIL_RUNTIME);
    }
    for index in 0..GRAPHICS_BUFFER_PIXEL_COUNT {
        let observed = unsafe { core::ptr::read_volatile((address as *const u32).add(index)) };
        if observed != mapped_frame_pixel(index, instance, generation) || observed >> 24 != 0 {
            fail(FAIL_RUNTIME);
        }
    }
}

fn send_launcher_request(
    lifecycle: u64,
    sequence: &mut bndr_ui::AppLifecycleSequenceTracker,
    request_sequence: &mut u64,
    transaction_id: u64,
    action: AppLifecycleAction,
    identity: Option<AppInstanceIdentity>,
) {
    let request = AppLifecycleMessage::request(
        next_sequence(request_sequence),
        transaction_id,
        APP,
        action,
        identity,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if sequence.accept(request).is_err() {
        fail(FAIL_RUNTIME);
    }
    write_wire(lifecycle, &request.encode());
}

const fn lifecycle_action_for_transaction(transaction_id: u64) -> Option<AppLifecycleAction> {
    if transaction_id > TRANSACTION_COUNT {
        return None;
    }
    match transaction_id {
        1 | 6 => Some(AppLifecycleAction::Launch),
        2 | 7 => Some(AppLifecycleAction::Activate),
        3 => Some(AppLifecycleAction::Suspend),
        4 => Some(AppLifecycleAction::Resume),
        5 => Some(AppLifecycleAction::Terminate),
        #[cfg(feature = "app-crash-recovery-runtime")]
        9 => Some(AppLifecycleAction::Launch),
        #[cfg(feature = "app-crash-recovery-runtime")]
        10 => Some(AppLifecycleAction::Activate),
        _ => None,
    }
}

const fn intermediate_state_for(action: AppLifecycleAction) -> AppLifecycleState {
    match action {
        AppLifecycleAction::Launch => AppLifecycleState::Launching,
        AppLifecycleAction::Activate => AppLifecycleState::Activating,
        AppLifecycleAction::Suspend => AppLifecycleState::Suspending,
        AppLifecycleAction::Resume => AppLifecycleState::Resuming,
        AppLifecycleAction::Terminate => AppLifecycleState::Terminating,
    }
}

const fn completed_state_for(action: AppLifecycleAction) -> AppLifecycleState {
    match action {
        AppLifecycleAction::Launch => AppLifecycleState::Inactive,
        AppLifecycleAction::Activate | AppLifecycleAction::Resume => AppLifecycleState::Active,
        AppLifecycleAction::Suspend => AppLifecycleState::Suspended,
        AppLifecycleAction::Terminate => AppLifecycleState::NotRunning,
    }
}

#[cfg(not(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime")
)))]
fn create_legacy_app_buffer() -> (OwnedUserHandle, Option<OwnedUserHandle>, u64) {
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
        fail(FAIL_RUNTIME);
    }
    let buffer = OwnedUserHandle::new(created.out1).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let duplicated = syscall(
        SyscallNumber::HandleDuplicate,
        buffer.raw(),
        u64::from(Rights::GRAPHICS_BUFFER_SERVER.bits()),
        0,
    );
    if duplicated.status != Status::Ok.raw() || duplicated.out1 == 0 || duplicated.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    let buffer_for_surface =
        Some(OwnedUserHandle::new(duplicated.out1).unwrap_or_else(|| fail(FAIL_RUNTIME)));
    let seed = COLOR_PHONE_SCREEN.to_le_bytes();
    let written = syscall(
        SyscallNumber::GraphicsBufferWrite,
        buffer.raw(),
        seed.as_ptr() as u64,
        pack_graphics_buffer_write(0, seed.len() as u32),
    );
    if written.status != Status::Ok.raw() || written.out1 != seed.len() as u64 || written.out2 != 1
    {
        fail(FAIL_RUNTIME);
    }
    (buffer, buffer_for_surface, written.out2)
}

#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime"),
    not(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "graphics-owner-death-runtime")
    ))
))]
#[derive(Clone, Copy, Eq, PartialEq)]
enum MappedAppPhase {
    Attached,
    FirstReleased,
    Presented,
    #[cfg(feature = "graphics-surface-restart-runtime")]
    ReboundQueued,
    #[cfg(feature = "graphics-surface-restart-runtime")]
    RestartPresented,
}

#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
const fn initial_focus_generation(launch_transaction: u64) -> u32 {
    match launch_transaction {
        1 => 1,
        6 => 5,
        _ => 0,
    }
}

#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
const fn active_focus_generation(instance_id: u64) -> u64 {
    match instance_id {
        1 => 2,
        2 => 6,
        _ => 0,
    }
}

#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
fn create_mapped_app_buffer(
    ui: u64,
    initial_focus: u32,
    client_frame_id: u32,
) -> (OwnedUserHandle, u64) {
    if initial_focus == 0
        || client_frame_id == 0
        || (client_frame_id != 1
            && !cfg!(all(
                feature = "graphics-producer-orphan-runtime",
                not(feature = "graphics-surface-restart-runtime"),
                not(feature = "app-crash-recovery-runtime")
            ))
            && !cfg!(all(
                feature = "graphics-swapchain-runtime",
                not(feature = "graphics-owner-death-runtime"),
                not(feature = "app-crash-recovery-runtime")
            )))
    {
        fail(FAIL_RUNTIME);
    }
    let created = syscall(
        SyscallNumber::GraphicsBufferCreate,
        GRAPHICS_BUFFER_FORMAT_XRGB8888,
        pack_graphics_buffer_geometry(GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_HEIGHT),
        GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE,
    );
    if created.status != Status::Ok.raw()
        || created.out1 == 0
        || created.out2 != GRAPHICS_BUFFER_LOGICAL_BYTES as u64
    {
        fail(FAIL_RUNTIME);
    }
    let buffer = OwnedUserHandle::new(created.out1).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let duplicated = syscall(
        SyscallNumber::HandleDuplicate,
        buffer.raw(),
        u64::from(Rights::GRAPHICS_BUFFER_MAPPED_SERVER.bits()),
        0,
    );
    if duplicated.status != Status::Ok.raw() || duplicated.out1 == 0 || duplicated.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    let buffer_for_surface =
        OwnedUserHandle::new(duplicated.out1).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let producer_mapping = map_graphics_buffer(buffer.raw(), GRAPHICS_BUFFER_MAP_PRODUCER_RW);

    // This promise deliberately precedes the first Queue operation.  The
    // transferred server view pins the same backing, while generation 1 is
    // authenticated only after SurfaceServer observes READABLE and acquires.
    let frame = BufferPresent::client(client_frame_id, initial_focus, 1)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire_transfer(ui, &frame.encode(), buffer_for_surface);
    (buffer, producer_mapping)
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn assert_swapchain_pool_exhausted() {
    let exhausted = syscall(
        SyscallNumber::GraphicsBufferCreate,
        GRAPHICS_BUFFER_FORMAT_XRGB8888,
        pack_graphics_buffer_geometry(GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_HEIGHT),
        GRAPHICS_BUFFER_CREATE_FLAG_MAPPABLE,
    );
    if exhausted.status != Status::OutOfMemory.raw() || exhausted.out1 != 0 || exhausted.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
#[derive(Clone, Copy, Eq, PartialEq)]
enum SwapchainAppPhase {
    Attached,
    InitialReleased,
    Presented,
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
#[allow(clippy::too_many_arguments)]
fn drive_swapchain_app_frame(
    ui: u64,
    buffer_a: u64,
    mapping_a: u64,
    buffer_b: u64,
    mapping_b: u64,
    identity: AppInstanceIdentity,
    state: AppLifecycleState,
    session_id: Option<u64>,
    focus_generation: Option<u64>,
    phase: &mut SwapchainAppPhase,
) {
    if session_id.is_none() {
        return;
    }
    let Some(focus) = focus_generation else {
        return;
    };
    match *phase {
        SwapchainAppPhase::Attached => {
            if state != AppLifecycleState::Inactive || focus != 1 {
                return;
            }
            render_swapchain_frame(mapping_a, identity.instance_id(), 0, 1, 1);
            render_swapchain_frame(mapping_b, identity.instance_id(), 1, 2, 1);
            if queue_graphics_buffer(buffer_a, 0) != 1 || queue_graphics_buffer(buffer_b, 0) != 1 {
                fail(FAIL_RUNTIME);
            }
            poll_graphics_backpressure(buffer_a);
            poll_graphics_backpressure(buffer_b);
            write_wire(ui, &SWAPCHAIN_INITIAL_BATCH_MAGIC.to_le_bytes());
            wait_graphics_fence(buffer_a, ObjectSignals::WRITABLE);
            wait_graphics_fence(buffer_b, ObjectSignals::WRITABLE);
            verify_swapchain_frame_samples(mapping_a, identity.instance_id(), 0, 1, 1);
            verify_swapchain_frame_samples(mapping_b, identity.instance_id(), 1, 2, 1);
            *phase = SwapchainAppPhase::InitialReleased;
        }
        SwapchainAppPhase::InitialReleased => {
            if state != AppLifecycleState::Active
                || focus != active_focus_generation(identity.instance_id())
            {
                return;
            }
            let focus = u32::try_from(focus).unwrap_or_else(|_| fail(FAIL_RUNTIME));

            render_swapchain_frame(mapping_a, identity.instance_id(), 0, 1, 2);
            if queue_graphics_buffer(buffer_a, 1) != 2 {
                fail(FAIL_RUNTIME);
            }
            let frame_1 = BufferPresent::client(1, focus, 2).unwrap_or_else(|_| fail(FAIL_RUNTIME));
            write_wire(ui, &frame_1.encode());

            render_swapchain_frame(mapping_b, identity.instance_id(), 1, 2, 2);
            if queue_graphics_buffer(buffer_b, 1) != 2 {
                fail(FAIL_RUNTIME);
            }
            let frame_2 = BufferPresent::client(2, focus, 2).unwrap_or_else(|_| fail(FAIL_RUNTIME));
            write_wire(ui, &frame_2.encode());
            poll_graphics_backpressure(buffer_a);
            poll_graphics_backpressure(buffer_b);
            write_wire(ui, &SWAPCHAIN_ACTIVE_BATCH_MAGIC.to_le_bytes());

            wait_graphics_fence(buffer_a, ObjectSignals::WRITABLE);
            verify_swapchain_frame_samples(mapping_a, identity.instance_id(), 0, 1, 2);
            poll_graphics_backpressure(buffer_b);
            assert_swapchain_queue_rejected(buffer_b, 2);
            render_swapchain_frame(mapping_a, identity.instance_id(), 0, 3, 3);
            if queue_graphics_buffer(buffer_a, 2) != 3 {
                fail(FAIL_RUNTIME);
            }
            let frame_3 = BufferPresent::client(3, focus, 3).unwrap_or_else(|_| fail(FAIL_RUNTIME));
            write_wire(ui, &frame_3.encode());
            poll_graphics_backpressure(buffer_a);
            write_wire(ui, &SWAPCHAIN_REFILL_A_MAGIC.to_le_bytes());

            wait_graphics_fence(buffer_b, ObjectSignals::WRITABLE);
            verify_swapchain_frame_samples(mapping_b, identity.instance_id(), 1, 2, 2);
            poll_graphics_backpressure(buffer_a);
            assert_swapchain_queue_rejected(buffer_a, 3);
            render_swapchain_frame(mapping_b, identity.instance_id(), 1, 4, 3);
            if queue_graphics_buffer(buffer_b, 2) != 3 {
                fail(FAIL_RUNTIME);
            }
            let frame_4 = BufferPresent::client(4, focus, 3).unwrap_or_else(|_| fail(FAIL_RUNTIME));
            write_wire(ui, &frame_4.encode());
            poll_graphics_backpressure(buffer_b);
            write_wire(ui, &SWAPCHAIN_REFILL_B_MAGIC.to_le_bytes());

            wait_graphics_fence(buffer_a, ObjectSignals::WRITABLE);
            verify_swapchain_frame_samples(mapping_a, identity.instance_id(), 0, 3, 3);
            poll_graphics_backpressure(buffer_b);
            verify_swapchain_frame_samples(mapping_b, identity.instance_id(), 1, 4, 3);
            write_wire(ui, &SWAPCHAIN_FINAL_MIXED_MAGIC.to_le_bytes());
            *phase = SwapchainAppPhase::Presented;
        }
        SwapchainAppPhase::Presented => {}
    }
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn assert_swapchain_queue_rejected(handle: u64, expected_generation: u64) {
    let denied = syscall(
        SyscallNumber::GraphicsBufferQueue,
        handle,
        expected_generation,
        GRAPHICS_BUFFER_QUEUE_FLAGS_NONE,
    );
    if denied.status != Status::InvalidState.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime"),
    not(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "graphics-owner-death-runtime")
    ))
))]
#[allow(clippy::too_many_arguments)]
fn drive_mapped_app_frame(
    _lifecycle: u64,
    _init_pid: u64,
    ui: u64,
    buffer: u64,
    producer_mapping: u64,
    identity: AppInstanceIdentity,
    state: AppLifecycleState,
    _surface_pid: &mut Option<u64>,
    session_id: &mut Option<u64>,
    focus_generation: &mut Option<u64>,
    phase: &mut MappedAppPhase,
) -> Option<OwnedUserHandle> {
    if session_id.is_none() {
        return None;
    }
    let focus = (*focus_generation)?;
    match *phase {
        MappedAppPhase::Attached => {
            let expected_focus = u64::from(match identity.instance_id() {
                1 => 1_u32,
                2 => 5_u32,
                _ => fail(FAIL_RUNTIME),
            });
            if focus != expected_focus || state != AppLifecycleState::Inactive {
                return None;
            }
            render_mapped_frame(producer_mapping, identity.instance_id(), 1);
            if queue_graphics_buffer(buffer, 0) != 1 {
                fail(FAIL_RUNTIME);
            }
            wait_graphics_fence(buffer, ObjectSignals::WRITABLE);
            #[cfg(feature = "graphics-owner-death-runtime")]
            {
                // This touches every one of the 75 producer PTEs after the
                // reaper's BBM RO->RW restore. Any partial restore faults the
                // App before it can publish the proof magic to init.
                render_mapped_frame(producer_mapping, identity.instance_id(), 2);
                #[cfg(feature = "graphics-surface-restart-runtime")]
                {
                    write_wire(_lifecycle, &GRAPHICS_OWNER_DEATH_READY_MAGIC.to_le_bytes());
                    let rebound = rebind_mapped_app_ui(
                        _lifecycle,
                        _init_pid,
                        ui,
                        buffer,
                        _surface_pid,
                        session_id,
                        focus_generation,
                    );
                    *phase = MappedAppPhase::ReboundQueued;
                    return Some(rebound);
                }
                #[cfg(not(feature = "graphics-surface-restart-runtime"))]
                {
                    unmap_graphics_buffer(buffer, producer_mapping);
                    close_owned(OwnedUserHandle::new(buffer).unwrap_or_else(|| fail(FAIL_RUNTIME)));
                    write_wire(_lifecycle, &GRAPHICS_OWNER_DEATH_READY_MAGIC.to_le_bytes());
                    exit_child(CHILD_EXIT_MAGIC);
                }
            }
            #[cfg(not(feature = "graphics-owner-death-runtime"))]
            {
                *phase = MappedAppPhase::FirstReleased;
            }
        }
        MappedAppPhase::FirstReleased => {
            if state != AppLifecycleState::Active
                || focus != active_focus_generation(identity.instance_id())
            {
                return None;
            }
            #[cfg(all(
                feature = "graphics-frame-clock-runtime",
                not(feature = "graphics-owner-death-runtime"),
                not(feature = "app-crash-recovery-runtime")
            ))]
            {
                let focus = u32::try_from(focus).unwrap_or_else(|_| fail(FAIL_RUNTIME));
                for client_frame_id in 1..=3_u32 {
                    let expected_generation = u64::from(client_frame_id);
                    let buffer_generation = expected_generation
                        .checked_add(1)
                        .unwrap_or_else(|| fail(FAIL_RUNTIME));
                    render_mapped_frame(
                        producer_mapping,
                        identity.instance_id(),
                        buffer_generation,
                    );
                    if queue_graphics_buffer(buffer, expected_generation) != buffer_generation {
                        fail(FAIL_RUNTIME);
                    }
                    let frame = BufferPresent::client(client_frame_id, focus, buffer_generation)
                        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                    write_wire(ui, &frame.encode());
                    // Queue publication must remove WRITABLE before
                    // SurfaceServer can pace and commit this generation.
                    poll_graphics_backpressure(buffer);
                    write_wire(
                        ui,
                        &frame_clock_backpressure_magic(client_frame_id).to_le_bytes(),
                    );
                    wait_graphics_fence(buffer, ObjectSignals::WRITABLE);
                    verify_mapped_frame_samples(
                        producer_mapping,
                        identity.instance_id(),
                        buffer_generation,
                    );
                }
                *phase = MappedAppPhase::Presented;
                return None;
            }
            #[cfg(not(all(
                feature = "graphics-frame-clock-runtime",
                not(feature = "graphics-owner-death-runtime"),
                not(feature = "app-crash-recovery-runtime")
            )))]
            {
                render_mapped_frame(producer_mapping, identity.instance_id(), 2);
                let generation = queue_graphics_buffer(buffer, 1);
                let focus = u32::try_from(focus).unwrap_or_else(|_| fail(FAIL_RUNTIME));
                let frame = BufferPresent::client(1, focus, generation)
                    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                write_wire(ui, &frame.encode());
                // Successful SurfacePresentBuffer is the frame-2 release point.
                wait_graphics_fence(buffer, ObjectSignals::WRITABLE);
                *phase = MappedAppPhase::Presented;
            }
        }
        MappedAppPhase::Presented => {}
        #[cfg(feature = "graphics-surface-restart-runtime")]
        MappedAppPhase::ReboundQueued => {
            if state != AppLifecycleState::Active || focus != 2 {
                return None;
            }
            wait_graphics_fence(buffer, ObjectSignals::WRITABLE);
            read_rebound_presented_event(
                ui,
                _surface_pid.unwrap_or_else(|| fail(FAIL_RUNTIME)),
                session_id.unwrap_or_else(|| fail(FAIL_RUNTIME)),
            );
            verify_mapped_frame_samples(producer_mapping, identity.instance_id(), 2);
            write_wire(_lifecycle, &GRAPHICS_SURFACE_REBOUND_MAGIC.to_le_bytes());
            *phase = MappedAppPhase::RestartPresented;
        }
        #[cfg(feature = "graphics-surface-restart-runtime")]
        MappedAppPhase::RestartPresented => {}
    }
    None
}

#[cfg(all(
    feature = "graphics-surface-restart-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
#[allow(clippy::too_many_arguments)]
fn rebind_mapped_app_ui(
    lifecycle: u64,
    init_pid: u64,
    old_ui: u64,
    buffer: u64,
    surface_pid: &mut Option<u64>,
    session_id: &mut Option<u64>,
    focus_generation: &mut Option<u64>,
) -> OwnedUserHandle {
    close_owned(OwnedUserHandle::new(old_ui).unwrap_or_else(|| fail(FAIL_RUNTIME)));
    let (sender_pid, ui) = read_bootstrap_transfer(lifecycle, UiBootstrapEndpointKind::AppUi);
    if sender_pid != init_pid {
        close_owned(ui);
        fail(FAIL_RUNTIME);
    }

    let duplicated = syscall(
        SyscallNumber::HandleDuplicate,
        buffer,
        u64::from(Rights::GRAPHICS_BUFFER_MAPPED_SERVER.bits()),
        0,
    );
    if duplicated.status != Status::Ok.raw() || duplicated.out1 == 0 || duplicated.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    let server_buffer = OwnedUserHandle::new(duplicated.out1).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let frame = BufferPresent::client(1, 1, 2).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire_transfer(ui.raw(), &frame.encode(), server_buffer);

    *surface_pid = None;
    *session_id = None;
    *focus_generation = None;
    consume_app_ui_event(ui.raw(), surface_pid, session_id, focus_generation);
    consume_app_ui_event(ui.raw(), surface_pid, session_id, focus_generation);
    if session_id.is_none_or(|session| session <= 1) || *focus_generation != Some(1) {
        fail(FAIL_RUNTIME);
    }
    if queue_graphics_buffer(buffer, 1) != 2 {
        fail(FAIL_RUNTIME);
    }
    ui
}

#[cfg(all(
    feature = "graphics-surface-restart-runtime",
    not(feature = "app-crash-recovery-runtime")
))]
fn read_rebound_presented_event(ui: u64, surface_pid: u64, session_id: u64) {
    let envelope = read_channel_envelope(ui);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != UI_SERVER_EVENT_WIRE_SIZE
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() != surface_pid
    {
        fail(FAIL_RUNTIME);
    }
    let event = UiServerEvent::decode(&envelope.data()[..UI_SERVER_EVENT_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if event.session_id() != session_id
        || !matches!(
            event.payload(),
            UiServerEventPayload::Presented { frame_id: 1, .. }
        )
    {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn render_swapchain_frame(
    address: u64,
    instance_id: u64,
    buffer_index: u32,
    client_frame_id: u32,
    generation: u64,
) {
    validate_swapchain_frame_identity(buffer_index, client_frame_id, generation);
    let address = usize::try_from(address).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if !address.is_multiple_of(4096) {
        fail(FAIL_RUNTIME);
    }
    let instance = u32::try_from(instance_id).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    for index in 0..GRAPHICS_BUFFER_PIXEL_COUNT {
        let pixel =
            swapchain_frame_pixel(index, instance, buffer_index, client_frame_id, generation);
        unsafe {
            core::ptr::write_volatile((address as *mut u32).add(index), pixel);
        }
    }
    verify_swapchain_frame_samples(
        address as u64,
        instance_id,
        buffer_index,
        client_frame_id,
        generation,
    );
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn verify_swapchain_frame_samples(
    address: u64,
    instance_id: u64,
    buffer_index: u32,
    client_frame_id: u32,
    generation: u64,
) {
    validate_swapchain_frame_identity(buffer_index, client_frame_id, generation);
    let address = usize::try_from(address).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if !address.is_multiple_of(4096) {
        fail(FAIL_RUNTIME);
    }
    let instance = u32::try_from(instance_id).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    for index in [
        0,
        GRAPHICS_BUFFER_PIXEL_COUNT / 2,
        GRAPHICS_BUFFER_PIXEL_COUNT - 1,
    ] {
        let observed = unsafe { core::ptr::read_volatile((address as *const u32).add(index)) };
        if observed
            != swapchain_frame_pixel(index, instance, buffer_index, client_frame_id, generation)
            || observed >> 24 != 0
        {
            fail(FAIL_RUNTIME);
        }
    }
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn validate_swapchain_frame_identity(buffer_index: u32, client_frame_id: u32, generation: u64) {
    if !matches!(
        (buffer_index, client_frame_id, generation),
        (0, 1, 1) | (1, 2, 1) | (0, 1, 2) | (1, 2, 2) | (0, 3, 3) | (1, 4, 3)
    ) {
        fail(FAIL_RUNTIME);
    }
}

#[cfg(all(
    feature = "graphics-swapchain-runtime",
    not(feature = "graphics-owner-death-runtime"),
    not(feature = "app-crash-recovery-runtime")
))]
fn swapchain_frame_pixel(
    index: usize,
    instance: u32,
    buffer_index: u32,
    client_frame_id: u32,
    generation: u64,
) -> u32 {
    let x = u32::try_from(index % GRAPHICS_BUFFER_WIDTH as usize)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let y = u32::try_from(index / GRAPHICS_BUFFER_WIDTH as usize)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let base = match client_frame_id {
        1 => COLOR_PHONE_SCREEN,
        2 => COLOR_PHONE_HEADER,
        3 => COLOR_CARD_CYAN,
        4 => COLOR_CARD_PURPLE,
        _ => fail(FAIL_RUNTIME),
    };
    (base
        ^ ((instance & 0x3f) << 10)
        ^ ((buffer_index & 1) << 19)
        ^ ((u32::try_from(generation).unwrap_or_else(|_| fail(FAIL_RUNTIME)) & 0x3) << 20)
        ^ ((x & 0x1f) << 3)
        ^ (y & 0x1f))
        & 0x00ff_ffff
}

#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime"),
    not(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "graphics-owner-death-runtime")
    ))
))]
fn render_mapped_frame(address: u64, instance_id: u64, generation: u64) {
    let address = usize::try_from(address).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let generation_valid = matches!(generation, 1 | 2)
        || (cfg!(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        )) && matches!(generation, 3 | 4));
    if !address.is_multiple_of(4096) || !generation_valid {
        fail(FAIL_RUNTIME);
    }
    let instance = u32::try_from(instance_id).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    for index in 0..GRAPHICS_BUFFER_PIXEL_COUNT {
        let pixel = mapped_frame_pixel(index, instance, generation);
        unsafe {
            core::ptr::write_volatile((address as *mut u32).add(index), pixel);
        }
    }
    verify_mapped_frame_samples(address as u64, instance_id, generation);
}

#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime"),
    not(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "graphics-owner-death-runtime")
    ))
))]
fn verify_mapped_frame_samples(address: u64, instance_id: u64, generation: u64) {
    let address = usize::try_from(address).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let generation_valid = matches!(generation, 1 | 2)
        || (cfg!(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        )) && matches!(generation, 3 | 4));
    if !address.is_multiple_of(4096) || !generation_valid {
        fail(FAIL_RUNTIME);
    }
    let instance = u32::try_from(instance_id).unwrap_or_else(|_| fail(FAIL_RUNTIME));
    for index in [
        0,
        GRAPHICS_BUFFER_PIXEL_COUNT / 2,
        GRAPHICS_BUFFER_PIXEL_COUNT - 1,
    ] {
        let observed = unsafe { core::ptr::read_volatile((address as *const u32).add(index)) };
        if observed != mapped_frame_pixel(index, instance, generation) || observed >> 24 != 0 {
            fail(FAIL_RUNTIME);
        }
    }
}

#[cfg(all(
    feature = "mapped-graphics-runtime",
    not(feature = "app-crash-recovery-runtime"),
    not(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "graphics-owner-death-runtime")
    ))
))]
fn mapped_frame_pixel(index: usize, instance: u32, generation: u64) -> u32 {
    let x = u32::try_from(index % GRAPHICS_BUFFER_WIDTH as usize)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let y = u32::try_from(index / GRAPHICS_BUFFER_WIDTH as usize)
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let base = match generation {
        1 => COLOR_PHONE_SCREEN,
        2 => COLOR_PHONE_HEADER,
        3 => COLOR_CARD_CYAN,
        4 => COLOR_CARD_PURPLE,
        _ => fail(FAIL_RUNTIME),
    };
    (base ^ ((instance & 0x3f) << 10) ^ ((x & 0x1f) << 3) ^ (y & 0x1f)) & 0x00ff_ffff
}

pub(super) fn app_runtime(startup: u64) -> ! {
    let denied = syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0);
    if denied.status != Status::PermissionDenied.raw() || denied.out1 != 0 || denied.out2 != 0 {
        fail(FAIL_RUNTIME);
    }
    let lifecycle = OwnedUserHandle::new(startup).unwrap_or_else(|| fail(FAIL_RUNTIME));
    let envelope = read_channel_envelope(lifecycle.raw());
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != APP_LIFECYCLE_WIRE_SIZE
        || envelope.sender_pid() == 0
    {
        fail(FAIL_RUNTIME);
    }
    let init_pid = envelope.sender_pid();
    let initial_ui = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| fail(FAIL_RUNTIME));
    #[cfg(all(
        feature = "mapped-graphics-runtime",
        not(feature = "app-crash-recovery-runtime"),
        not(all(
            feature = "graphics-swapchain-runtime",
            not(feature = "graphics-owner-death-runtime")
        ))
    ))]
    let mut ui = initial_ui;
    #[cfg(any(
        not(all(
            feature = "mapped-graphics-runtime",
            not(feature = "app-crash-recovery-runtime")
        )),
        all(
            feature = "graphics-swapchain-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        )
    ))]
    let ui = initial_ui;
    let launch = AppLifecycleMessage::decode(&envelope.data()[..APP_LIFECYCLE_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    let AppLifecyclePayload::Command {
        app,
        action: AppLifecycleAction::Launch,
        identity,
    } = launch.payload()
    else {
        fail(FAIL_RUNTIME);
    };
    if app != APP
        || launch.sender_sequence() != app_endpoint_sequence(launch.transaction_id())
        || lifecycle_action_for_transaction(launch.transaction_id())
            != Some(AppLifecycleAction::Launch)
        || identity.instance_id() != launch_instance_id(launch.transaction_id())
    {
        fail(FAIL_RUNTIME);
    }

    #[cfg(not(all(
        feature = "mapped-graphics-runtime",
        not(feature = "app-crash-recovery-runtime")
    )))]
    let (_buffer, mut buffer_for_surface, initial_buffer_generation) = create_legacy_app_buffer();
    #[cfg(all(
        feature = "mapped-graphics-runtime",
        not(feature = "app-crash-recovery-runtime")
    ))]
    let (buffer, producer_mapping) = create_mapped_app_buffer(
        ui.raw(),
        initial_focus_generation(launch.transaction_id()),
        1,
    );
    #[cfg(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    let (swapchain_buffer, swapchain_producer_mapping) = create_mapped_app_buffer(
        ui.raw(),
        initial_focus_generation(launch.transaction_id()),
        2,
    );
    #[cfg(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    assert_swapchain_pool_exhausted();
    #[cfg(all(
        feature = "graphics-producer-orphan-runtime",
        not(feature = "graphics-surface-restart-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    let (_standby_buffer, _standby_producer_mapping) = create_mapped_app_buffer(
        ui.raw(),
        initial_focus_generation(launch.transaction_id()),
        2,
    );

    send_app_ack(
        lifecycle.raw(),
        launch.transaction_id(),
        AppLifecycleAction::Launch,
        identity,
    );
    let mut expected_transaction = launch.transaction_id() + 1;
    let terminal_transaction = match launch.transaction_id() {
        1 if cfg!(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        )) =>
        {
            2
        }
        1 => 5,
        6 => 7,
        #[cfg(feature = "app-crash-recovery-runtime")]
        9 => 10,
        _ => fail(FAIL_RUNTIME),
    };
    let mut state = AppLifecycleState::Inactive;
    let mut surface_pid = None;
    let mut session_id = None;
    let mut focus_generation = None;
    #[cfg(not(all(
        feature = "mapped-graphics-runtime",
        not(feature = "app-crash-recovery-runtime")
    )))]
    let mut buffer_sent = false;
    #[cfg(all(
        feature = "mapped-graphics-runtime",
        not(feature = "app-crash-recovery-runtime"),
        not(all(
            feature = "graphics-swapchain-runtime",
            not(feature = "graphics-owner-death-runtime")
        ))
    ))]
    let mut mapped_phase = MappedAppPhase::Attached;
    #[cfg(all(
        feature = "graphics-swapchain-runtime",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    ))]
    let mut swapchain_phase = SwapchainAppPhase::Attached;
    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);

    loop {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(lifecycle.raw(), requested);
        items[1] = pack_user_wait_item(ui.raw(), requested);
        let ready = object_wait_many_array(&items, 2, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw()
            || ready.out1 > 1
            || ready.out2 & u64::from(requested.bits()) == 0
        {
            fail(FAIL_RUNTIME);
        }
        if ready.out1 == 1 {
            consume_app_ui_event(
                ui.raw(),
                &mut surface_pid,
                &mut session_id,
                &mut focus_generation,
            );
            #[cfg(not(all(
                feature = "mapped-graphics-runtime",
                not(feature = "app-crash-recovery-runtime")
            )))]
            if !buffer_sent
                && session_id.is_some()
                && let Some(focus) = focus_generation
            {
                let focus = u32::try_from(focus).unwrap_or_else(|_| fail(FAIL_RUNTIME));
                let frame = BufferPresent::client(1, focus, initial_buffer_generation)
                    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
                let transfer = buffer_for_surface
                    .take()
                    .unwrap_or_else(|| fail(FAIL_RUNTIME));
                write_wire_transfer(ui.raw(), &frame.encode(), transfer);
                buffer_sent = true;
            }
            #[cfg(all(
                feature = "mapped-graphics-runtime",
                not(feature = "app-crash-recovery-runtime"),
                not(all(
                    feature = "graphics-swapchain-runtime",
                    not(feature = "graphics-owner-death-runtime")
                ))
            ))]
            if let Some(rebound_ui) = drive_mapped_app_frame(
                lifecycle.raw(),
                init_pid,
                ui.raw(),
                buffer.raw(),
                producer_mapping,
                identity,
                state,
                &mut surface_pid,
                &mut session_id,
                &mut focus_generation,
                &mut mapped_phase,
            ) {
                ui = rebound_ui;
            }
            #[cfg(all(
                feature = "graphics-swapchain-runtime",
                not(feature = "graphics-owner-death-runtime"),
                not(feature = "app-crash-recovery-runtime")
            ))]
            drive_swapchain_app_frame(
                ui.raw(),
                buffer.raw(),
                producer_mapping,
                swapchain_buffer.raw(),
                swapchain_producer_mapping,
                identity,
                state,
                session_id,
                focus_generation,
                &mut swapchain_phase,
            );
            continue;
        }
        if ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
            || expected_transaction > terminal_transaction
        {
            fail(FAIL_RUNTIME);
        }
        let (command, transfer) = read_lifecycle_message(lifecycle.raw(), init_pid, false);
        let AppLifecyclePayload::Command {
            app,
            action,
            identity: command_identity,
        } = command.payload()
        else {
            fail(FAIL_RUNTIME);
        };
        if transfer.is_some()
            || app != APP
            || command.sender_sequence() != app_endpoint_sequence(expected_transaction)
            || command.transaction_id() != expected_transaction
            || command_identity != identity
        {
            fail(FAIL_RUNTIME);
        }
        state = apply_app_action(state, action);
        #[cfg(all(
            feature = "mapped-graphics-runtime",
            not(feature = "app-crash-recovery-runtime"),
            not(all(
                feature = "graphics-swapchain-runtime",
                not(feature = "graphics-owner-death-runtime")
            ))
        ))]
        if action == AppLifecycleAction::Terminate {
            if mapped_phase != MappedAppPhase::Presented {
                fail(FAIL_RUNTIME);
            }
            unmap_graphics_buffer(buffer.raw(), producer_mapping);
        }
        send_app_ack(lifecycle.raw(), expected_transaction, action, identity);
        expected_transaction += 1;
        if action == AppLifecycleAction::Terminate {
            wait_for_lifecycle_retirement(
                lifecycle.raw(),
                ui.raw(),
                &mut surface_pid,
                &mut session_id,
                &mut focus_generation,
            );
        }
        if expected_transaction > terminal_transaction && state != AppLifecycleState::Active {
            fail(FAIL_RUNTIME);
        }
    }
}

fn apply_app_action(state: AppLifecycleState, action: AppLifecycleAction) -> AppLifecycleState {
    match (state, action) {
        (AppLifecycleState::Inactive, AppLifecycleAction::Activate)
        | (AppLifecycleState::Suspended, AppLifecycleAction::Resume) => AppLifecycleState::Active,
        (AppLifecycleState::Active, AppLifecycleAction::Suspend) => AppLifecycleState::Suspended,
        (
            AppLifecycleState::Inactive | AppLifecycleState::Active | AppLifecycleState::Suspended,
            AppLifecycleAction::Terminate,
        ) => AppLifecycleState::NotRunning,
        _ => fail(FAIL_RUNTIME),
    }
}

fn send_app_ack(
    lifecycle: u64,
    transaction_id: u64,
    action: AppLifecycleAction,
    identity: AppInstanceIdentity,
) {
    let ack = AppLifecycleMessage::ack(
        app_endpoint_sequence(transaction_id),
        transaction_id,
        APP,
        action,
        AppLifecycleStatus::Applied,
        identity,
    )
    .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    write_wire(lifecycle, &ack.encode());
}

const fn app_endpoint_sequence(transaction_id: u64) -> u64 {
    match transaction_id {
        1..=5 => transaction_id,
        6..=7 => transaction_id - 5,
        #[cfg(feature = "app-crash-recovery-runtime")]
        9..=10 => transaction_id - 8,
        _ => 0,
    }
}

fn consume_app_ui_event(
    ui: u64,
    surface_pid: &mut Option<u64>,
    session_id: &mut Option<u64>,
    focus_generation: &mut Option<u64>,
) {
    let envelope = read_channel_envelope_now(ui);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != UI_SERVER_EVENT_WIRE_SIZE
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() == 0
        || surface_pid.is_some_and(|pid| pid != envelope.sender_pid())
    {
        fail(FAIL_RUNTIME);
    }
    *surface_pid = Some(envelope.sender_pid());
    let event = UiServerEvent::decode(&envelope.data()[..UI_SERVER_EVENT_WIRE_SIZE])
        .unwrap_or_else(|_| fail(FAIL_RUNTIME));
    if session_id.is_some_and(|session| session != event.session_id()) {
        fail(FAIL_RUNTIME);
    }
    match event.payload() {
        UiServerEventPayload::Ready if session_id.is_none() => {
            *session_id = Some(event.session_id());
        }
        UiServerEventPayload::FocusChanged {
            focus_generation: next,
            ..
        } if session_id == &Some(event.session_id())
            && focus_generation.is_none_or(|previous| next > previous) =>
        {
            *focus_generation = Some(next);
        }
        UiServerEventPayload::PresentCancelled { frame_id, .. }
            if session_id == &Some(event.session_id())
                && (frame_id == 1
                    || (cfg!(all(
                        feature = "graphics-swapchain-runtime",
                        not(feature = "graphics-owner-death-runtime"),
                        not(feature = "app-crash-recovery-runtime")
                    )) && frame_id == 2)) => {}
        UiServerEventPayload::Presented { frame_id, .. }
            if session_id == &Some(event.session_id())
                && (frame_id == 1
                    || (cfg!(all(
                        feature = "graphics-frame-clock-runtime",
                        not(feature = "graphics-owner-death-runtime"),
                        not(feature = "app-crash-recovery-runtime")
                    )) && matches!(frame_id, 2 | 3))) => {}
        UiServerEventPayload::Input(_) if session_id == &Some(event.session_id()) => {}
        _ => fail(FAIL_RUNTIME),
    }
}

fn wait_for_lifecycle_retirement(
    lifecycle: u64,
    ui: u64,
    surface_pid: &mut Option<u64>,
    session_id: &mut Option<u64>,
    focus_generation: &mut Option<u64>,
) -> ! {
    let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
    items[0] = pack_user_wait_item(lifecycle, ObjectSignals::PEER_CLOSED);
    items[1] = pack_user_wait_item(ui, ObjectSignals::READABLE);
    loop {
        let ready = object_wait_many_array(&items, 2, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw() || ready.out1 > 1 {
            fail(FAIL_RUNTIME);
        }
        if ready.out1 == 0 && ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
            exit_child(CHILD_EXIT_MAGIC);
        }
        if ready.out1 == 1 && ready.out2 & u64::from(ObjectSignals::READABLE.bits()) != 0 {
            consume_app_ui_event(ui, surface_pid, session_id, focus_generation);
        } else {
            fail(FAIL_RUNTIME);
        }
    }
}
