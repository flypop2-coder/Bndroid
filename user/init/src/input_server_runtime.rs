//! M45 independent system input service.
//!
//! The kernel grants the only global physical-input capability to this image.
//! SurfaceServer supplies an authenticated, generation-qualified scene mirror
//! over BIC1 and receives routed/semantic BIE1 events in return.  The input
//! method therefore no longer lives in SurfaceServer and raw device reports
//! never enter an application-facing process.

use super::*;

use bndr_abi::{InputEvent as PhysicalInputEvent, InputEventPayload as PhysicalInputPayload};
use bndr_input::{
    CommandStream, InputAction, InputCommand, InputCommandPayload, InputEvent as RoutedInputEvent,
    InputKey, InputMethodEngine, InputRouter, TextContext,
};
#[cfg(any(
    feature = "input-server-surface-restart-runtime",
    feature = "service-dependency-runtime"
))]
use bndr_input::{InputGapQueue, PointerDeviceState, RoutedPointer};
#[cfg(any(
    feature = "input-server-surface-restart-runtime",
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
use bndr_input::{InputRouteControl, InputRouteControlPayload, OwnerPid};
#[cfg(any(
    feature = "service-supervisor-runtime",
    feature = "service-dependency-runtime"
))]
use bndr_sm::health::{HealthFrame, HealthOpcode, ServiceIdentity, ServiceKind};
use bndr_ui::InputSample;

const FAIL_INPUT_SERVER_RUNTIME: u64 = 245;
#[cfg(feature = "service-dependency-runtime")]
const SERVICE_HEALTH_TIMEOUT_NS: u64 = 100_000_000;
#[cfg(all(
    feature = "service-supervisor-runtime",
    not(feature = "service-dependency-runtime")
))]
const SERVICE_HEALTH_TIMEOUT_NS: u64 = 200_000_000;
const INPUT_DEVICE_KEY_BACKSPACE: u16 = 14;
const INPUT_DEVICE_KEY_ENTER: u16 = 28;
const INPUT_DEVICE_KEY_A: u16 = 30;

struct InputServerRuntime {
    control: OwnedUserHandle,
    route: OwnedUserHandle,
    capability: OwnedUserHandle,
    init_pid: u64,
    surface_pid: u64,
    router: InputRouter,
    ime: InputMethodEngine,
    ime_started: bool,
    current_context: Option<TextContext>,
    semantic_revision: u32,
    event_sequence: u64,
    scene_ready: bool,
    scene_update_active: bool,
    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    scene_commits: u64,
    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    input_session_id: u64,
    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    route_epoch: u64,
    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    status_sequence: u64,
    #[cfg(any(
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    resync_required: bool,
    #[cfg(any(
        feature = "service-supervisor-runtime",
        feature = "service-dependency-runtime"
    ))]
    health_identity: Option<ServiceIdentity>,
    #[cfg(any(
        feature = "service-supervisor-runtime",
        feature = "service-dependency-runtime"
    ))]
    health_probe_count: u8,
    #[cfg(feature = "service-dependency-runtime")]
    dependency_snapshot_step: u8,
}

pub(super) fn run(startup: u64) -> ! {
    let control = OwnedUserHandle::new(startup).unwrap_or_else(|| input_fail());
    let envelope = read_channel_envelope(control.raw());
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != core::mem::size_of::<u64>()
        || envelope.sender_pid() == 0
        || envelope.data()[..core::mem::size_of::<u64>()]
            != INPUT_ROUTE_BOOTSTRAP_MAGIC.to_le_bytes()
    {
        if let Some(endpoint) = OwnedUserHandle::new(u64::from(envelope.received_handle().raw())) {
            close_owned(endpoint);
        }
        input_fail();
    }
    let init_pid = envelope.sender_pid();
    let route = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| input_fail());

    let acquired = syscall(SyscallNumber::InputAcquire, 0, 0, 0);
    if acquired.status != Status::Ok.raw() || acquired.out1 == 0 || acquired.out2 == 0 {
        input_fail();
    }
    let capability = OwnedUserHandle::new(acquired.out1).unwrap_or_else(|| input_fail());
    let mut runtime = InputServerRuntime {
        control,
        route,
        capability,
        init_pid,
        surface_pid: 0,
        router: InputRouter::new(),
        ime: InputMethodEngine::new(),
        ime_started: false,
        current_context: None,
        semantic_revision: 0,
        event_sequence: 0,
        scene_ready: false,
        scene_update_active: false,
        #[cfg(any(
            feature = "input-server-surface-restart-runtime",
            feature = "service-dependency-runtime"
        ))]
        scene_commits: 0,
        #[cfg(any(
            feature = "input-server-surface-restart-runtime",
            feature = "input-server-restart-runtime",
            feature = "service-dependency-runtime"
        ))]
        input_session_id: acquired.out2,
        #[cfg(any(
            feature = "input-server-surface-restart-runtime",
            feature = "input-server-restart-runtime",
            feature = "service-dependency-runtime"
        ))]
        route_epoch: 0,
        #[cfg(any(
            feature = "input-server-surface-restart-runtime",
            feature = "input-server-restart-runtime",
            feature = "service-dependency-runtime"
        ))]
        status_sequence: 0,
        #[cfg(any(
            feature = "input-server-restart-runtime",
            feature = "service-dependency-runtime"
        ))]
        resync_required: false,
        #[cfg(any(
            feature = "service-supervisor-runtime",
            feature = "service-dependency-runtime"
        ))]
        health_identity: None,
        #[cfg(any(
            feature = "service-supervisor-runtime",
            feature = "service-dependency-runtime"
        ))]
        health_probe_count: 0,
        #[cfg(feature = "service-dependency-runtime")]
        dependency_snapshot_step: 0,
    };

    let ready_sequence = runtime.next_event_sequence();
    runtime.send_event(RoutedInputEvent::ready(ready_sequence).unwrap_or_else(|_| input_fail()));
    write_input_bytes(
        runtime.control.raw(),
        &INPUT_SERVER_READY_MAGIC.to_le_bytes(),
    );
    #[cfg(feature = "unified-product-runtime")]
    super::product_runtime::arm(
        runtime.control.raw(),
        ShutdownServiceNode::InputServer,
        runtime.init_pid,
    );

    let channel_requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    loop {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(runtime.route.raw(), channel_requested);
        let mut count = 1_usize;
        let capability_index = runtime.scene_ready.then_some(count);
        if runtime.scene_ready {
            items[count] = pack_user_wait_item(runtime.capability.raw(), channel_requested);
            count += 1;
        }
        let control_index = count;
        items[count] = pack_user_wait_item(runtime.control.raw(), channel_requested);
        count += 1;
        let ready = object_wait_many_array(&items, count, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw()
            || usize::try_from(ready.out1).unwrap_or_else(|_| input_fail()) >= count
        {
            input_fail();
        }
        let ready_index = usize::try_from(ready.out1).unwrap_or_else(|_| input_fail());
        let observed = ready.out2;
        if ready_index == 0 && observed & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
            runtime.replace_surface_route();
            continue;
        }
        if observed & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
            input_fail();
        }
        match ready_index {
            0 if observed & u64::from(ObjectSignals::READABLE.bits()) != 0 => {
                runtime.process_command()
            }
            index
                if capability_index == Some(index)
                    && observed & u64::from(ObjectSignals::READABLE.bits()) != 0 =>
            {
                runtime.process_physical_event()
            }
            index
                if index == control_index
                    && observed & u64::from(ObjectSignals::READABLE.bits()) != 0 =>
            {
                runtime.process_control_message()
            }
            _ => input_fail(),
        }
    }
}

/// M46 keeps the InputServer process and physical-input session alive while
/// replacing SurfaceServer. BIR1 authenticates both Surface generations by
/// exact PID and carries the one bounded route-gap ledger on Init's stable
/// control channel; BIC1/BIE1 remain byte-for-byte unchanged per route epoch.
#[cfg(all(
    feature = "input-server-surface-restart-runtime",
    not(feature = "input-server-restart-runtime")
))]
pub(super) fn run_surface_restart(startup: u64) -> ! {
    let control = OwnedUserHandle::new(startup).unwrap_or_else(|| input_fail());
    let acquired = syscall(SyscallNumber::InputAcquire, 0, 0, 0);
    if acquired.status != Status::Ok.raw() || acquired.out1 == 0 || acquired.out2 == 0 {
        input_fail();
    }
    let capability = OwnedUserHandle::new(acquired.out1).unwrap_or_else(|| input_fail());
    let input_session_id = acquired.out2;
    let acquired_control =
        InputRouteControl::acquired(1, input_session_id, 0).unwrap_or_else(|_| input_fail());
    write_input_bytes(control.raw(), &acquired_control.encode());

    let (init_pid, route, surface_pid) =
        read_route_bind(control.raw(), None, input_session_id, 1, 1, 0);
    let mut runtime = InputServerRuntime {
        control,
        route,
        capability,
        init_pid,
        surface_pid,
        router: InputRouter::new(),
        ime: InputMethodEngine::new(),
        ime_started: false,
        current_context: None,
        semantic_revision: 0,
        event_sequence: 0,
        scene_ready: false,
        scene_update_active: false,
        scene_commits: 0,
        input_session_id,
        route_epoch: 1,
        status_sequence: 1,
        #[cfg(feature = "service-dependency-runtime")]
        resync_required: false,
        #[cfg(any(
            feature = "service-supervisor-runtime",
            feature = "service-dependency-runtime"
        ))]
        health_identity: None,
        #[cfg(any(
            feature = "service-supervisor-runtime",
            feature = "service-dependency-runtime"
        ))]
        health_probe_count: 0,
        #[cfg(feature = "service-dependency-runtime")]
        dependency_snapshot_step: 0,
    };
    let ready_sequence = runtime.next_event_sequence();
    runtime.send_event(RoutedInputEvent::ready(ready_sequence).unwrap_or_else(|_| input_fail()));
    runtime.send_route_ready();

    let channel_requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut recovery_gap: Option<InputGapQueue> = None;
    loop {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(runtime.route.raw(), channel_requested);
        let mut count = 1_usize;
        let capability_index = runtime.scene_ready.then_some(count);
        if runtime.scene_ready {
            items[count] = pack_user_wait_item(runtime.capability.raw(), channel_requested);
            count += 1;
        }
        let control_index = count;
        items[count] = pack_user_wait_item(runtime.control.raw(), channel_requested);
        count += 1;
        let ready = object_wait_many_array(&items, count, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw()
            || usize::try_from(ready.out1).unwrap_or_else(|_| input_fail()) >= count
        {
            input_fail();
        }
        let ready_index = usize::try_from(ready.out1).unwrap_or_else(|_| input_fail());
        let observed = ready.out2;
        if ready_index == 0 && observed & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
            if recovery_gap.is_some() || runtime.route_epoch != 1 {
                input_fail();
            }
            recovery_gap = Some(runtime.recover_surface_route());
            continue;
        }
        if observed & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
            input_fail();
        }
        match ready_index {
            0 if observed & u64::from(ObjectSignals::READABLE.bits()) != 0 => {
                let previous_commits = runtime.scene_commits;
                runtime.process_command();
                if runtime.scene_commits != previous_commits {
                    let gap = recovery_gap.take().unwrap_or_else(|| input_fail());
                    runtime.drain_recovery_gap(gap);
                }
            }
            index
                if capability_index == Some(index)
                    && observed & u64::from(ObjectSignals::READABLE.bits()) != 0 =>
            {
                runtime.process_physical_event()
            }
            index if index == control_index => input_fail(),
            _ => input_fail(),
        }
    }
}

/// M47 gives each InputServer generation a fresh BIR1/BIC1/BIE1 epoch. The
/// replacement cannot read physical input until one exact six-command
/// snapshot transaction commits.
#[cfg(feature = "input-server-restart-runtime")]
pub(super) fn run_restart(startup: u64) -> ! {
    let control = OwnedUserHandle::new(startup).unwrap_or_else(|| input_fail());
    let acquired = syscall(SyscallNumber::InputAcquire, 0, 0, 0);
    if acquired.status != Status::Ok.raw() || acquired.out1 == 0 || acquired.out2 == 0 {
        input_fail();
    }
    let capability = OwnedUserHandle::new(acquired.out1).unwrap_or_else(|| input_fail());
    let session_info = syscall(SyscallNumber::InputSessionInfo, capability.raw(), 0, 0);
    if session_info.status != Status::Ok.raw()
        || session_info.out1 != acquired.out2
        || !matches!((session_info.out1, session_info.out2), (1, 0) | (2, 1))
    {
        input_fail();
    }
    let input_session_id = session_info.out1;
    let physical_floor = session_info.out2;
    let route_epoch = input_session_id;
    let acquired_control = InputRouteControl::acquired(1, input_session_id, physical_floor)
        .unwrap_or_else(|_| input_fail());
    write_input_bytes(control.raw(), &acquired_control.encode());

    let (init_pid, route, surface_pid) = read_route_bind(
        control.raw(),
        None,
        input_session_id,
        route_epoch,
        route_epoch,
        physical_floor,
    );
    let mut runtime = InputServerRuntime {
        control,
        route,
        capability,
        init_pid,
        surface_pid,
        router: InputRouter::with_physical_floor(physical_floor),
        ime: InputMethodEngine::new(),
        ime_started: false,
        current_context: None,
        semantic_revision: 0,
        event_sequence: 0,
        scene_ready: false,
        scene_update_active: false,
        #[cfg(any(
            feature = "input-server-surface-restart-runtime",
            feature = "service-dependency-runtime"
        ))]
        scene_commits: 0,
        input_session_id,
        route_epoch,
        status_sequence: 1,
        resync_required: physical_floor != 0,
        #[cfg(any(
            feature = "service-supervisor-runtime",
            feature = "service-dependency-runtime"
        ))]
        health_identity: None,
        #[cfg(any(
            feature = "service-supervisor-runtime",
            feature = "service-dependency-runtime"
        ))]
        health_probe_count: 0,
        #[cfg(feature = "service-dependency-runtime")]
        dependency_snapshot_step: 0,
    };
    let ready_sequence = runtime.next_event_sequence();
    runtime.send_event(RoutedInputEvent::ready(ready_sequence).unwrap_or_else(|_| input_fail()));
    runtime.send_route_ready();

    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    loop {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(runtime.route.raw(), requested);
        let mut count = 1_usize;
        let capability_index = runtime.scene_ready.then_some(count);
        if runtime.scene_ready {
            items[count] = pack_user_wait_item(runtime.capability.raw(), requested);
            count += 1;
        }
        let control_index = count;
        items[count] = pack_user_wait_item(runtime.control.raw(), requested);
        count += 1;
        let ready = object_wait_many_array(&items, count, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw()
            || usize::try_from(ready.out1).unwrap_or_else(|_| input_fail()) >= count
            || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
            || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
        {
            input_fail();
        }
        let ready_index = usize::try_from(ready.out1).unwrap_or_else(|_| input_fail());
        match ready_index {
            0 => runtime.process_command(),
            index if capability_index == Some(index) => runtime.process_physical_event(),
            index if index == control_index => {
                #[cfg(feature = "service-supervisor-runtime")]
                runtime.process_health_probe();
                #[cfg(not(feature = "service-supervisor-runtime"))]
                input_fail();
            }
            _ => input_fail(),
        }
    }
}

/// M49 keeps one strict dependency transcript across a SurfaceServer restart
/// followed by an InputServer restart. The kernel-issued input session is the
/// generation discriminator: session 1 retains the M46 route-gap machinery,
/// while session 2 accepts one exact six-command authoritative snapshot.
#[cfg(feature = "service-dependency-runtime")]
pub(super) fn run_dependency(startup: u64) -> ! {
    let control = OwnedUserHandle::new(startup).unwrap_or_else(|| input_fail());
    let acquired = syscall(SyscallNumber::InputAcquire, 0, 0, 0);
    if acquired.status != Status::Ok.raw() || acquired.out1 == 0 || acquired.out2 == 0 {
        input_fail();
    }
    let capability = OwnedUserHandle::new(acquired.out1).unwrap_or_else(|| input_fail());
    let session_info = syscall(SyscallNumber::InputSessionInfo, capability.raw(), 0, 0);
    if session_info.status != Status::Ok.raw()
        || session_info.out1 != acquired.out2
        || !matches!((session_info.out1, session_info.out2), (1, 0) | (2, 3))
    {
        input_fail();
    }
    let input_session_id = session_info.out1;
    let physical_floor = session_info.out2;
    let (bind_sequence, route_epoch) = match input_session_id {
        1 => (1, 1),
        2 => (2, 3),
        _ => input_fail(),
    };
    let acquired_control = InputRouteControl::acquired(1, input_session_id, physical_floor)
        .unwrap_or_else(|_| input_fail());
    write_input_bytes(control.raw(), &acquired_control.encode());

    let (init_pid, route, surface_pid) = read_route_bind(
        control.raw(),
        None,
        input_session_id,
        bind_sequence,
        route_epoch,
        physical_floor,
    );
    let mut runtime = InputServerRuntime {
        control,
        route,
        capability,
        init_pid,
        surface_pid,
        router: InputRouter::with_physical_floor(physical_floor),
        ime: InputMethodEngine::new(),
        ime_started: false,
        current_context: None,
        semantic_revision: 0,
        event_sequence: 0,
        scene_ready: false,
        scene_update_active: false,
        scene_commits: 0,
        input_session_id,
        route_epoch,
        status_sequence: 1,
        resync_required: input_session_id == 2,
        health_identity: None,
        health_probe_count: 0,
        dependency_snapshot_step: 0,
    };
    let ready_sequence = runtime.next_event_sequence();
    runtime.send_event(RoutedInputEvent::ready(ready_sequence).unwrap_or_else(|_| input_fail()));
    runtime.send_route_ready();

    let requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
    let mut recovery_gap: Option<InputGapQueue> = None;
    loop {
        let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
        items[0] = pack_user_wait_item(runtime.route.raw(), requested);
        let mut count = 1_usize;
        let capability_index = runtime.scene_ready.then_some(count);
        if runtime.scene_ready {
            items[count] = pack_user_wait_item(runtime.capability.raw(), requested);
            count += 1;
        }
        let control_index = count;
        items[count] = pack_user_wait_item(runtime.control.raw(), requested);
        count += 1;
        let ready = object_wait_many_array(&items, count, OBJECT_WAIT_TIMEOUT_INFINITE);
        if ready.status != Status::Ok.raw()
            || usize::try_from(ready.out1).unwrap_or_else(|_| input_fail()) >= count
        {
            input_fail();
        }
        let ready_index = usize::try_from(ready.out1).unwrap_or_else(|_| input_fail());
        let observed = ready.out2;
        if ready_index == 0 && observed & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0 {
            if runtime.input_session_id != 1
                || runtime.route_epoch != 1
                || recovery_gap.is_some()
                || runtime.health_probe_count != 0
            {
                input_fail();
            }
            recovery_gap = Some(runtime.recover_surface_route());
            continue;
        }
        if observed & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
            || observed & u64::from(ObjectSignals::READABLE.bits()) == 0
        {
            input_fail();
        }
        match ready_index {
            0 => {
                let previous_commits = runtime.scene_commits;
                runtime.process_command();
                if runtime.scene_commits != previous_commits
                    && let Some(gap) = recovery_gap.take()
                {
                    runtime.drain_recovery_gap(gap);
                }
            }
            index if capability_index == Some(index) => runtime.process_physical_event(),
            index if index == control_index => runtime.process_dependency_health_probe(),
            _ => input_fail(),
        }
    }
}

#[cfg(any(
    feature = "input-server-surface-restart-runtime",
    feature = "input-server-restart-runtime",
    feature = "service-dependency-runtime"
))]
fn read_route_bind(
    control: u64,
    expected_init_pid: Option<u64>,
    input_session_id: u64,
    bind_sequence: u64,
    route_epoch: u64,
    physical_floor: u64,
) -> (u64, OwnedUserHandle, u64) {
    let envelope = read_channel_envelope(control);
    if envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != bndr_input::INPUT_ROUTE_CONTROL_WIRE_SIZE
        || envelope.data()[..core::mem::size_of::<u32>()]
            != bndr_input::INPUT_ROUTE_CONTROL_MAGIC.to_le_bytes()
        || envelope.sender_pid() == 0
        || expected_init_pid.is_some_and(|pid| envelope.sender_pid() != pid)
    {
        if let Some(endpoint) = OwnedUserHandle::new(u64::from(envelope.received_handle().raw())) {
            close_owned(endpoint);
        }
        input_fail();
    }
    let message =
        InputRouteControl::decode(&envelope.data()[..bndr_input::INPUT_ROUTE_CONTROL_WIRE_SIZE])
            .unwrap_or_else(|_| input_fail());
    let InputRouteControlPayload::Bind {
        expected_surface_pid,
        expected_physical_floor,
    } = message.payload()
    else {
        if let Some(endpoint) = OwnedUserHandle::new(u64::from(envelope.received_handle().raw())) {
            close_owned(endpoint);
        }
        input_fail();
    };
    if message.sequence() != bind_sequence
        || message.route_epoch() != route_epoch
        || message.input_session_id() != input_session_id
        || expected_physical_floor != physical_floor
    {
        if let Some(endpoint) = OwnedUserHandle::new(u64::from(envelope.received_handle().raw())) {
            close_owned(endpoint);
        }
        input_fail();
    }
    let route = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
        .unwrap_or_else(|| input_fail());
    (envelope.sender_pid(), route, expected_surface_pid.get())
}

impl InputServerRuntime {
    #[cfg(feature = "service-supervisor-runtime")]
    fn process_health_probe(&mut self) {
        let envelope = read_channel_envelope_now(self.control.raw());
        if envelope.kind() != ChannelMessageKind::Bytes
            || envelope.logical_length() != bndr_sm::health::HEALTH_FRAME_SIZE
            || envelope.sender_pid() != self.init_pid
            || envelope.received_handle().is_valid()
        {
            input_fail();
        }
        let probe = HealthFrame::decode(envelope.data()).unwrap_or_else(|_| input_fail());
        let identity = probe.identity();
        if probe.opcode() != HealthOpcode::Probe
            || probe.interval_ns() != SERVICE_HEALTH_TIMEOUT_NS
            || identity.kind() != ServiceKind::InputServer
            || identity.generation() != process_generation(identity.pid())
            || process_slot(identity.pid()) == 0
            || self.input_session_id != 2
            || self.route_epoch != 2
            || self.surface_pid == 0
            || !self.scene_ready
            || self.scene_update_active
            || self.resync_required
            || self.router.last_input_sequence() != 3
            || self.event_sequence != 9
        {
            input_fail();
        }
        match self.health_probe_count {
            0 => {
                if probe.sequence() != 1 || self.health_identity.is_some() {
                    input_fail();
                }
                self.health_identity = Some(identity);
                self.health_probe_count = 1;
                let healthy = HealthFrame::healthy(1, identity).unwrap_or_else(|_| input_fail());
                write_input_bytes(self.control.raw(), &healthy.encode());
            }
            1 => {
                if probe.sequence() != 2 || self.health_identity != Some(identity) {
                    input_fail();
                }
                self.health_probe_count = 2;
                // The replacement remains alive but intentionally withholds
                // Probe 2 so Init's real finite wait drives the watchdog path.
            }
            _ => input_fail(),
        }
    }

    #[cfg(feature = "service-dependency-runtime")]
    fn process_dependency_health_probe(&mut self) {
        #[cfg(feature = "post-recovery-interaction-runtime")]
        self.process_post_recovery_health_probe();
        #[cfg(not(feature = "post-recovery-interaction-runtime"))]
        self.process_m49_dependency_health_probe();
    }

    #[cfg(feature = "post-recovery-interaction-runtime")]
    fn process_post_recovery_health_probe(&mut self) {
        let envelope = read_channel_envelope_now(self.control.raw());
        if envelope.kind() != ChannelMessageKind::Bytes
            || envelope.logical_length() != bndr_sm::health::HEALTH_FRAME_SIZE
            || envelope.data()[..bndr_sm::health::HEALTH_PROTOCOL_MAGIC.len()]
                != bndr_sm::health::HEALTH_PROTOCOL_MAGIC
            || envelope.sender_pid() != self.init_pid
            || envelope.received_handle().is_valid()
        {
            if let Some(received) =
                OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
            {
                close_owned(received);
            }
            input_fail();
        }
        let probe = HealthFrame::decode(envelope.data()).unwrap_or_else(|_| input_fail());
        let identity = probe.identity();
        if probe.opcode() != HealthOpcode::Probe
            || probe.interval_ns() != SERVICE_HEALTH_TIMEOUT_NS
            || identity.kind() != ServiceKind::InputServer
            || identity.generation() != process_generation(identity.pid())
            || process_slot(identity.pid()) == 0
            || self.surface_pid == 0
            || !self.scene_ready
            || self.scene_update_active
            || self.resync_required
        {
            input_fail();
        }

        match (
            self.input_session_id,
            probe.sequence(),
            self.health_probe_count,
        ) {
            (1, 1, 0) => {
                if self.route_epoch != 2
                    || self.status_sequence != 6
                    || self.scene_commits != 1
                    || self.dependency_snapshot_step != 0
                    || self.router.route_count() != 2
                    || self.router.last_input_sequence() != 3
                    || self.event_sequence != 7
                    || self.router.captured().is_some()
                    || self.health_identity.is_some()
                {
                    input_fail();
                }
                self.health_identity = Some(identity);
                self.health_probe_count = 1;
                // Generation 1 deliberately withholds Healthy so Init's real
                // finite wait remains the M49 dependency-root fault.
            }
            (2, 1, 0) => {
                if self.route_epoch != 3
                    || self.status_sequence != 2
                    || self.scene_commits != 1
                    || self.dependency_snapshot_step != 6
                    || self.router.route_count() != 2
                    || self.router.last_route_sequence() != 4
                    || self.router.last_focus_sequence() != 2
                    || self.router.focused().is_none()
                    || self.router.text_context().is_some()
                    || self.router.last_input_sequence() != 3
                    || self.event_sequence != 7
                    || self.router.captured().is_some()
                    || self.health_identity.is_some()
                {
                    input_fail();
                }
                self.health_identity = Some(identity);
                self.health_probe_count = 1;
                let healthy = HealthFrame::healthy(1, identity).unwrap_or_else(|_| input_fail());
                write_input_bytes(self.control.raw(), &healthy.encode());
            }
            (2, 2, 1) => {
                if self.route_epoch != 3
                    || self.status_sequence != 2
                    || self.scene_commits != 1
                    || self.dependency_snapshot_step != 6
                    || self.router.route_count() != 2
                    || self.router.last_route_sequence() != 4
                    || self.router.last_focus_sequence() != 2
                    || self.router.focused().is_none()
                    || self.router.text_context().is_some()
                    || self.router.last_input_sequence() != 7
                    || self.event_sequence != 11
                    || self.router.captured().is_some()
                    || self.health_identity != Some(identity)
                {
                    input_fail();
                }
                self.health_probe_count = 2;
                let healthy = HealthFrame::healthy(2, identity).unwrap_or_else(|_| input_fail());
                write_input_bytes(self.control.raw(), &healthy.encode());
            }
            _ => input_fail(),
        }
    }

    #[cfg(all(
        feature = "service-dependency-runtime",
        not(feature = "post-recovery-interaction-runtime")
    ))]
    fn process_m49_dependency_health_probe(&mut self) {
        let envelope = read_channel_envelope_now(self.control.raw());
        if envelope.kind() != ChannelMessageKind::Bytes
            || envelope.logical_length() != bndr_sm::health::HEALTH_FRAME_SIZE
            || envelope.data()[..bndr_sm::health::HEALTH_PROTOCOL_MAGIC.len()]
                != bndr_sm::health::HEALTH_PROTOCOL_MAGIC
            || envelope.sender_pid() != self.init_pid
            || envelope.received_handle().is_valid()
        {
            if let Some(received) =
                OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
            {
                close_owned(received);
            }
            input_fail();
        }
        let probe = HealthFrame::decode(envelope.data()).unwrap_or_else(|_| input_fail());
        let identity = probe.identity();
        if probe.opcode() != HealthOpcode::Probe
            || probe.sequence() != 1
            || probe.interval_ns() != SERVICE_HEALTH_TIMEOUT_NS
            || identity.kind() != ServiceKind::InputServer
            || identity.generation() != process_generation(identity.pid())
            || process_slot(identity.pid()) == 0
            || self.surface_pid == 0
            || !self.scene_ready
            || self.scene_update_active
            || self.resync_required
            || self.router.last_input_sequence() != 3
            || self.health_identity.is_some()
            || self.health_probe_count != 0
        {
            input_fail();
        }
        match self.input_session_id {
            1 => {
                if self.route_epoch != 2
                    || self.status_sequence != 6
                    || self.scene_commits != 1
                    || self.dependency_snapshot_step != 0
                    || self.router.route_count() != 2
                {
                    input_fail();
                }
            }
            2 => {
                if self.route_epoch != 3
                    || self.status_sequence != 2
                    || self.scene_commits != 1
                    || self.dependency_snapshot_step != 6
                    || self.router.route_count() != 2
                    || self.router.last_route_sequence() != 4
                    || self.router.last_focus_sequence() != 2
                    || self.router.focused().is_none()
                    || self.router.captured().is_some()
                    || self.router.text_context().is_some()
                    || self.event_sequence != 7
                {
                    input_fail();
                }
            }
            _ => input_fail(),
        }
        self.health_identity = Some(identity);
        self.health_probe_count = 1;
        if self.input_session_id == 2 {
            let healthy = HealthFrame::healthy(1, identity).unwrap_or_else(|_| input_fail());
            write_input_bytes(self.control.raw(), &healthy.encode());
        }
        // Session 1 deliberately withholds Healthy so Init's finite 100 ms wait
        // records the dependency-root timeout. The frame was still dequeued,
        // authenticated, decoded, and sequence-checked above.
    }

    #[cfg(feature = "service-dependency-runtime")]
    fn validate_dependency_snapshot_command(&mut self, command: InputCommand) {
        if self.input_session_id == 1 {
            if self.dependency_snapshot_step != 0 || self.resync_required {
                input_fail();
            }
            return;
        }
        if self.input_session_id != 2 {
            input_fail();
        }
        let valid = match (self.dependency_snapshot_step, command.payload()) {
            (0, InputCommandPayload::SnapshotReset) => command.sequence() == 1,
            (1, InputCommandPayload::RouteUpsert(_)) => command.sequence() == 2,
            (2, InputCommandPayload::RouteUpsert(_)) => command.sequence() == 3,
            (3, InputCommandPayload::SetFocus(Some(_))) => command.sequence() == 1,
            (4, InputCommandPayload::SetTextContext(None)) => command.sequence() == 2,
            (5, InputCommandPayload::SceneCommit) => command.sequence() == 4,
            #[cfg(feature = "post-recovery-focus-runtime")]
            (6, InputCommandPayload::SetFocus(Some(target))) => {
                command.sequence() == 3 && target.window_id().get() == 1 && target.generation() == 1
            }
            #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
            (7, InputCommandPayload::SetFocus(Some(target))) => {
                command.sequence() == 4 && target.window_id().get() == 2 && target.generation() == 1
            }
            _ => false,
        };
        if !valid {
            input_fail();
        }
        self.dependency_snapshot_step = self
            .dependency_snapshot_step
            .checked_add(1)
            .unwrap_or_else(|| input_fail());
    }

    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    fn next_status_sequence(&mut self) -> u64 {
        self.status_sequence = self
            .status_sequence
            .checked_add(1)
            .unwrap_or_else(|| input_fail());
        self.status_sequence
    }

    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    fn send_route_control(&self, control: InputRouteControl) {
        write_input_bytes(self.control.raw(), &control.encode());
    }

    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "input-server-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    fn send_route_ready(&mut self) {
        let sequence = self.next_status_sequence();
        let control = InputRouteControl::ready(
            sequence,
            self.route_epoch,
            self.input_session_id,
            OwnerPid::try_new(self.surface_pid).unwrap_or_else(|_| input_fail()),
            self.router.last_input_sequence(),
        )
        .unwrap_or_else(|_| input_fail());
        self.send_route_control(control);
    }

    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    fn recover_surface_route(&mut self) -> InputGapQueue {
        let snapshot = self.router.begin_surface_recovery();
        #[cfg(feature = "service-dependency-runtime")]
        if self.input_session_id != 1
            || self.route_epoch != 1
            || snapshot.physical_sequence_floor() != 2
        {
            input_fail();
        }
        let capture = snapshot.capture();
        let lost_sequence = self.next_status_sequence();
        let lost = InputRouteControl::route_lost(
            lost_sequence,
            self.route_epoch,
            self.input_session_id,
            snapshot.physical_sequence_floor(),
            capture,
        )
        .unwrap_or_else(|_| input_fail());
        self.send_route_control(lost);

        let pointer_state =
            capture.map(|_| PointerDeviceState::try_new(2, true).unwrap_or_else(|_| input_fail()));
        let mut gap = InputGapQueue::new(snapshot.physical_sequence_floor(), pointer_state);
        let channel_requested = signal_union(ObjectSignals::READABLE, ObjectSignals::PEER_CLOSED);
        while gap.is_empty() {
            let mut items = [0_u64; OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS];
            items[0] = pack_user_wait_item(self.capability.raw(), channel_requested);
            items[1] = pack_user_wait_item(self.control.raw(), channel_requested);
            let ready = object_wait_many_array(&items, 2, OBJECT_WAIT_TIMEOUT_INFINITE);
            if ready.status != Status::Ok.raw()
                || ready.out1 > 1
                || ready.out2 & u64::from(ObjectSignals::PEER_CLOSED.bits()) != 0
                || ready.out2 & u64::from(ObjectSignals::READABLE.bits()) == 0
                || ready.out1 != 0
                || !gap.can_read_capability()
            {
                input_fail();
            }
            let event = self.read_physical_event();
            gap.enqueue(event).unwrap_or_else(|_| input_fail());
        }
        let next_epoch = self
            .route_epoch
            .checked_add(1)
            .unwrap_or_else(|| input_fail());
        let queued_sequence = self.next_status_sequence();
        let queued = InputRouteControl::gap_status(
            queued_sequence,
            next_epoch,
            self.input_session_id,
            snapshot.physical_sequence_floor(),
            gap.metrics(),
        )
        .unwrap_or_else(|_| input_fail());
        self.send_route_control(queued);

        #[cfg(feature = "service-dependency-runtime")]
        let old_surface_pid = self.surface_pid;
        let (_, replacement, surface_pid) = read_route_bind(
            self.control.raw(),
            Some(self.init_pid),
            self.input_session_id,
            next_epoch,
            next_epoch,
            snapshot.physical_sequence_floor(),
        );
        #[cfg(feature = "service-dependency-runtime")]
        if surface_pid == old_surface_pid {
            close_owned(replacement);
            input_fail();
        }
        let retired = core::mem::replace(&mut self.route, replacement);
        close_owned(retired);
        self.surface_pid = surface_pid;
        self.route_epoch = next_epoch;
        self.ime = InputMethodEngine::new();
        self.ime_started = false;
        self.current_context = None;
        self.semantic_revision = 0;
        self.event_sequence = 0;
        self.scene_ready = false;
        self.scene_update_active = false;
        self.scene_commits = 0;

        let ready_sequence = self.next_event_sequence();
        self.send_event(RoutedInputEvent::ready(ready_sequence).unwrap_or_else(|_| input_fail()));
        self.send_route_ready();
        gap
    }

    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    fn read_physical_event(&self) -> PhysicalInputEvent {
        let read = syscall(SyscallNumber::InputReadEvent, self.capability.raw(), 0, 0);
        if read.status != Status::Ok.raw() || read.out2 == 0 {
            input_fail();
        }
        PhysicalInputEvent::decode_registers(read.out1, read.out2).unwrap_or_else(|_| input_fail())
    }

    #[cfg(any(
        feature = "input-server-surface-restart-runtime",
        feature = "service-dependency-runtime"
    ))]
    fn drain_recovery_gap(&mut self, mut gap: InputGapQueue) {
        let buffered = gap.pop_front().unwrap_or_else(|| input_fail());
        if !gap.is_empty() || buffered.first_sequence() != buffered.last_sequence() {
            input_fail();
        }
        let event = buffered.event();
        #[cfg(feature = "service-dependency-runtime")]
        if event.sequence() != 3 {
            input_fail();
        }
        let PhysicalInputPayload::Pointer { x, y, pressed } = event.payload() else {
            input_fail();
        };
        if pressed {
            input_fail();
        }
        self.router
            .discard_input(event.sequence())
            .unwrap_or_else(|_| input_fail());
        let sample =
            InputSample::try_new(event.sequence(), x, y, false).unwrap_or_else(|_| input_fail());
        if self.ime.handle(sample) != bndr_input::InputOutcome::IGNORED {
            input_fail();
        }
        let sequence = self.next_event_sequence();
        self.send_event(
            RoutedInputEvent::routed_pointer(
                sequence,
                RoutedPointer {
                    input_sequence: event.sequence(),
                    target: None,
                    x,
                    y,
                    pressed: false,
                    captured: false,
                    trusted_overlay: false,
                },
            )
            .unwrap_or_else(|_| input_fail()),
        );
        let drained_sequence = self.next_status_sequence();
        let drained = InputRouteControl::gap_status(
            drained_sequence,
            self.route_epoch,
            self.input_session_id,
            self.router.last_input_sequence(),
            gap.metrics(),
        )
        .unwrap_or_else(|_| input_fail());
        self.send_route_control(drained);
    }

    fn replace_surface_route(&mut self) {
        let envelope = read_channel_envelope(self.control.raw());
        self.replace_surface_route_from_envelope(envelope);
    }

    fn process_control_message(&mut self) {
        let envelope = super::read_channel_envelope_once(self.control.raw());
        #[cfg(feature = "unified-product-runtime")]
        if super::product_runtime::dispatch_control(self.control.raw(), &envelope) {
            return;
        }
        self.replace_surface_route_from_envelope(envelope);
    }

    fn replace_surface_route_from_envelope(&mut self, envelope: ChannelReadEnvelope) {
        if envelope.kind() != ChannelMessageKind::Transfer
            || envelope.logical_length() != core::mem::size_of::<u64>()
            || envelope.sender_pid() != self.init_pid
            || envelope.data()[..core::mem::size_of::<u64>()]
                != INPUT_ROUTE_BOOTSTRAP_MAGIC.to_le_bytes()
        {
            if let Some(endpoint) =
                OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
            {
                close_owned(endpoint);
            }
            input_fail();
        }
        let replacement = OwnedUserHandle::new(u64::from(envelope.received_handle().raw()))
            .unwrap_or_else(|| input_fail());
        let retired = core::mem::replace(&mut self.route, replacement);
        close_owned(retired);

        // A SurfaceServer generation owns one complete scene/control epoch.
        // Never let focus, capture, tombstones, IME state, or wire sequences
        // from the dead generation leak into its replacement.
        self.surface_pid = 0;
        self.router.reset_surface_epoch();
        self.ime = InputMethodEngine::new();
        self.ime_started = false;
        self.current_context = None;
        self.semantic_revision = 0;
        self.event_sequence = 0;
        self.scene_ready = false;
        self.scene_update_active = false;
        #[cfg(any(
            feature = "input-server-surface-restart-runtime",
            feature = "service-dependency-runtime"
        ))]
        {
            self.scene_commits = 0;
        }
        #[cfg(any(
            feature = "input-server-restart-runtime",
            feature = "service-dependency-runtime"
        ))]
        {
            self.resync_required = false;
        }

        let ready_sequence = self.next_event_sequence();
        self.send_event(RoutedInputEvent::ready(ready_sequence).unwrap_or_else(|_| input_fail()));
        write_input_bytes(self.control.raw(), &INPUT_SERVER_READY_MAGIC.to_le_bytes());
    }

    fn next_event_sequence(&mut self) -> u64 {
        self.event_sequence = self
            .event_sequence
            .checked_add(1)
            .unwrap_or_else(|| input_fail());
        self.event_sequence
    }

    fn send_event(&self, event: RoutedInputEvent) {
        write_input_bytes(self.route.raw(), &event.encode());
    }

    fn send_ack(&mut self, stream: CommandStream, acknowledged_sequence: u64) {
        let sequence = self.next_event_sequence();
        let event = RoutedInputEvent::ack(sequence, stream, acknowledged_sequence)
            .unwrap_or_else(|_| input_fail());
        self.send_event(event);
    }

    fn process_command(&mut self) {
        let envelope = read_channel_envelope_now(self.route.raw());
        if envelope.kind() != ChannelMessageKind::Bytes
            || envelope.logical_length() != bndr_input::INPUT_COMMAND_WIRE_SIZE
            || envelope.data()[..core::mem::size_of::<u32>()]
                != bndr_input::INPUT_COMMAND_MAGIC.to_le_bytes()
            || envelope.received_handle().is_valid()
            || envelope.sender_pid() == 0
        {
            input_fail();
        }
        if self.surface_pid == 0 {
            if envelope.sender_pid() == self.init_pid {
                input_fail();
            }
            self.surface_pid = envelope.sender_pid();
        } else if envelope.sender_pid() != self.surface_pid {
            input_fail();
        }
        let command = InputCommand::decode(&envelope.data()[..bndr_input::INPUT_COMMAND_WIRE_SIZE])
            .unwrap_or_else(|_| input_fail());
        #[cfg(feature = "service-dependency-runtime")]
        self.validate_dependency_snapshot_command(command);
        #[cfg(any(
            feature = "input-server-restart-runtime",
            feature = "service-dependency-runtime"
        ))]
        if self.resync_required && !matches!(command.payload(), InputCommandPayload::SnapshotReset)
        {
            input_fail();
        }
        match command.payload() {
            InputCommandPayload::SceneBegin => {
                if self.scene_update_active || !self.scene_ready {
                    input_fail();
                }
                self.router
                    .accept_scene_barrier(command.sequence())
                    .unwrap_or_else(|_| input_fail());
                self.scene_update_active = true;
                self.scene_ready = false;
                self.send_ack(CommandStream::Route, command.sequence());
            }
            InputCommandPayload::SceneCommit => {
                if !self.scene_update_active || self.scene_ready {
                    input_fail();
                }
                self.router
                    .accept_scene_barrier(command.sequence())
                    .unwrap_or_else(|_| input_fail());
                self.scene_update_active = false;
                self.scene_ready = true;
                #[cfg(any(
                    feature = "input-server-surface-restart-runtime",
                    feature = "service-dependency-runtime"
                ))]
                {
                    self.scene_commits = self
                        .scene_commits
                        .checked_add(1)
                        .unwrap_or_else(|| input_fail());
                }
                self.send_ack(CommandStream::Route, command.sequence());
            }
            InputCommandPayload::SnapshotReset => {
                #[cfg(any(
                    feature = "input-server-restart-runtime",
                    feature = "service-dependency-runtime"
                ))]
                let begins_restart_resync = self.resync_required;
                #[cfg(any(
                    feature = "input-server-restart-runtime",
                    feature = "service-dependency-runtime"
                ))]
                if begins_restart_resync && (self.scene_ready || self.scene_update_active) {
                    input_fail();
                }
                self.router
                    .reset_snapshot(command.sequence())
                    .unwrap_or_else(|_| input_fail());
                self.scene_ready = false;
                #[cfg(any(
                    feature = "input-server-restart-runtime",
                    feature = "service-dependency-runtime"
                ))]
                if begins_restart_resync {
                    self.resync_required = false;
                    self.scene_update_active = true;
                }
                self.send_ack(CommandStream::Route, command.sequence());
                self.reconcile_context();
            }
            InputCommandPayload::RouteUpsert(route) => {
                self.router
                    .upsert_route(command.sequence(), route)
                    .unwrap_or_else(|_| input_fail());
                self.send_ack(CommandStream::Route, command.sequence());
                self.reconcile_context();
            }
            InputCommandPayload::RouteRemove(target) => {
                self.router
                    .remove_route(command.sequence(), target)
                    .unwrap_or_else(|_| input_fail());
                self.send_ack(CommandStream::Route, command.sequence());
                self.reconcile_context();
            }
            InputCommandPayload::SetFocus(target) => {
                self.router
                    .set_focus(command.sequence(), target)
                    .unwrap_or_else(|_| input_fail());
                if !self.scene_update_active {
                    self.scene_ready = true;
                }
                self.send_ack(CommandStream::Focus, command.sequence());
                self.reconcile_context();
            }
            InputCommandPayload::SetTextContext(context) => {
                self.router
                    .set_text_context(command.sequence(), context)
                    .unwrap_or_else(|_| input_fail());
                self.send_ack(CommandStream::Focus, command.sequence());
                self.reconcile_context();
            }
        }
    }

    fn reconcile_context(&mut self) {
        let next = self.router.text_context();
        if next == self.current_context {
            return;
        }
        if let Some(previous) = self.current_context {
            self.ime.hide();
            let sequence = self.next_event_sequence();
            self.send_event(
                RoutedInputEvent::overlay_hidden(sequence, previous.context_id())
                    .unwrap_or_else(|_| input_fail()),
            );
        }
        self.current_context = next;
        self.semantic_revision = 0;
        if let Some(context) = next {
            self.ime.show();
            self.ime_started = true;
            let sequence = self.next_event_sequence();
            self.send_event(
                RoutedInputEvent::overlay_shown(sequence, context.context_id())
                    .unwrap_or_else(|_| input_fail()),
            );
        }
    }

    fn process_physical_event(&mut self) {
        let read = syscall(SyscallNumber::InputReadEvent, self.capability.raw(), 0, 0);
        if read.status != Status::Ok.raw() || read.out2 == 0 {
            input_fail();
        }
        let event = PhysicalInputEvent::decode_registers(read.out1, read.out2)
            .unwrap_or_else(|_| input_fail());
        match event.payload() {
            PhysicalInputPayload::Pointer { x, y, pressed } => {
                self.process_pointer(event.sequence(), x, y, pressed)
            }
            PhysicalInputPayload::Key { code, value } => {
                self.process_key(event.sequence(), code, value)
            }
        }
    }

    fn process_pointer(&mut self, sequence: u64, x: u16, y: u16, pressed: bool) {
        let sample = InputSample::try_new(sequence, x, y, pressed).unwrap_or_else(|_| input_fail());
        let routed = self
            .router
            .route_pointer(sample)
            .unwrap_or_else(|_| input_fail());
        #[cfg(feature = "post-recovery-interaction-runtime")]
        self.validate_post_recovery_pointer(routed);
        let hidden_without_overlay =
            !self.ime.is_visible() && !self.router.has_visible_trusted_overlay();
        let outcome = if self.ime_started && (hidden_without_overlay || routed.trusted_overlay) {
            self.ime.handle(sample)
        } else {
            bndr_input::InputOutcome::IGNORED
        };
        if outcome.consumed() && !routed.trusted_overlay {
            input_fail();
        }
        // Pointer-down may atomically move focus and clear the active text
        // context inside the router. Publish that state transition before the
        // routed pointer so SurfaceServer cannot observe the new focus with a
        // stale visible-overlay state.
        self.reconcile_context();
        let event_sequence = self.next_event_sequence();
        self.send_event(
            RoutedInputEvent::routed_pointer(event_sequence, routed)
                .unwrap_or_else(|_| input_fail()),
        );
        if let Some(action) = outcome.action() {
            if !routed.trusted_overlay {
                input_fail();
            }
            self.send_semantic_action(action);
        }
    }

    #[cfg(feature = "post-recovery-interaction-runtime")]
    fn validate_post_recovery_pointer(&self, routed: RoutedPointer) {
        if self.input_session_id != 2 {
            return;
        }
        #[cfg(feature = "post-recovery-focus-runtime")]
        let valid_runtime_phase = match routed.input_sequence {
            8 => {
                self.health_probe_count == 2
                    && self.event_sequence == 11
                    && self.router.last_focus_sequence() == 2
                    && self.dependency_snapshot_step == 6
            }
            9 => {
                self.health_probe_count == 2
                    && self.event_sequence == 13
                    && self.router.last_focus_sequence() == 3
                    && self.dependency_snapshot_step == 7
            }
            #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
            10 => {
                self.health_probe_count == 2
                    && self.event_sequence == 14
                    && self.router.last_route_sequence() == 4
                    && self.router.last_focus_sequence() == 3
                    && self.router.last_input_sequence() == 10
                    && self.router.route_count() == 2
                    && self.dependency_snapshot_step == 7
                    && self.scene_ready
                    && !self.scene_update_active
                    && self.scene_commits == 1
                    && !self.resync_required
            }
            #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
            11 => {
                self.health_probe_count == 2
                    && self.event_sequence == 16
                    && self.router.last_route_sequence() == 4
                    && self.router.last_focus_sequence() == 4
                    && self.router.last_input_sequence() == 11
                    && self.router.route_count() == 2
                    && self.dependency_snapshot_step == 8
                    && self.scene_ready
                    && !self.scene_update_active
                    && self.scene_commits == 1
                    && !self.resync_required
            }
            _ => {
                self.health_probe_count == 1
                    && self.event_sequence
                        == routed
                            .input_sequence
                            .checked_add(3)
                            .unwrap_or_else(|| input_fail())
            }
        };
        #[cfg(not(feature = "post-recovery-focus-runtime"))]
        let valid_runtime_phase = self.health_probe_count == 1
            && self.event_sequence
                == routed
                    .input_sequence
                    .checked_add(3)
                    .unwrap_or_else(|| input_fail());
        if !valid_runtime_phase || self.health_identity.is_none() {
            input_fail();
        }
        let focused = self.router.focused().unwrap_or_else(|| input_fail());
        let valid = match routed.input_sequence {
            4 => {
                routed.target.is_none()
                    && !routed.captured
                    && !routed.trusted_overlay
                    && (routed.x, routed.y) == (16, 32)
                    && routed.pressed
                    && self.router.captured().is_none()
            }
            5 => {
                routed.target.is_none()
                    && !routed.captured
                    && !routed.trusted_overlay
                    && (routed.x, routed.y) == (16, 32)
                    && !routed.pressed
                    && self.router.captured().is_none()
            }
            6 => {
                routed.target == Some(focused)
                    && routed.captured
                    && !routed.trusted_overlay
                    && (routed.x, routed.y) == (136, 184)
                    && routed.pressed
                    && self.router.captured() == Some(focused)
            }
            7 => {
                routed.target == Some(focused)
                    && routed.captured
                    && !routed.trusted_overlay
                    && (routed.x, routed.y) == (136, 184)
                    && !routed.pressed
                    && self.router.captured().is_none()
            }
            #[cfg(feature = "post-recovery-focus-runtime")]
            8 => {
                focused.window_id().get() == 1
                    && focused.generation() == 1
                    && routed.target == Some(focused)
                    && routed.captured
                    && !routed.trusted_overlay
                    && (routed.x, routed.y) == (80, 96)
                    && routed.pressed
                    && self.router.captured() == Some(focused)
            }
            #[cfg(feature = "post-recovery-focus-runtime")]
            9 => {
                focused.window_id().get() == 1
                    && focused.generation() == 1
                    && routed.target == Some(focused)
                    && routed.captured
                    && !routed.trusted_overlay
                    && (routed.x, routed.y) == (80, 96)
                    && !routed.pressed
                    && self.router.captured().is_none()
            }
            #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
            10 => {
                focused.window_id().get() == 2
                    && focused.generation() == 1
                    && routed.target == Some(focused)
                    && routed.captured
                    && !routed.trusted_overlay
                    && (routed.x, routed.y) == (136, 184)
                    && routed.pressed
                    && self.router.captured() == Some(focused)
            }
            #[cfg(feature = "post-recovery-focus-roundtrip-runtime")]
            11 => {
                focused.window_id().get() == 2
                    && focused.generation() == 1
                    && routed.target == Some(focused)
                    && routed.captured
                    && !routed.trusted_overlay
                    && (routed.x, routed.y) == (136, 184)
                    && !routed.pressed
                    && self.router.captured().is_none()
            }
            _ => false,
        };
        if !valid {
            input_fail();
        }
    }

    fn process_key(&mut self, sequence: u64, code: u16, value: u8) {
        #[cfg(feature = "unified-product-runtime")]
        if code == 116 {
            self.router
                .discard_input(sequence)
                .unwrap_or_else(|_| input_fail());
            super::product_runtime::request_power_key(sequence, value);
            return;
        }
        #[cfg(feature = "post-recovery-interaction-runtime")]
        if self.input_session_id == 2 {
            let _ = (sequence, code, value);
            input_fail();
        }
        let key = match code {
            INPUT_DEVICE_KEY_A => InputKey::A,
            INPUT_DEVICE_KEY_BACKSPACE => InputKey::Backspace,
            INPUT_DEVICE_KEY_ENTER => InputKey::Enter,
            _ => {
                #[cfg(feature = "unified-product-event-supervision-runtime")]
                if code == 63 {
                    self.router
                        .discard_input(sequence)
                        .unwrap_or_else(|_| input_fail());
                    super::product_runtime::request_storage_rotation(sequence, value);
                    return;
                }
                self.router
                    .discard_input(sequence)
                    .unwrap_or_else(|_| input_fail());
                return;
            }
        };
        let pressed = match value {
            PhysicalInputEvent::KEY_VALUE_RELEASE => false,
            PhysicalInputEvent::KEY_VALUE_PRESS => true,
            PhysicalInputEvent::KEY_VALUE_REPEAT => {
                self.router
                    .discard_input(sequence)
                    .unwrap_or_else(|_| input_fail());
                return;
            }
            _ => input_fail(),
        };
        let routed = self
            .router
            .route_key(sequence, key, pressed)
            .unwrap_or_else(|_| input_fail());
        let event_sequence = self.next_event_sequence();
        self.send_event(
            RoutedInputEvent::routed_key(
                event_sequence,
                routed.input_sequence,
                routed.target,
                routed.key,
                routed.pressed,
            )
            .unwrap_or_else(|_| input_fail()),
        );
    }

    fn send_semantic_action(&mut self, action: InputAction) {
        let context = self.current_context.unwrap_or_else(|| input_fail());
        self.semantic_revision = self
            .semantic_revision
            .checked_add(1)
            .unwrap_or_else(|| input_fail());
        let sequence = self.next_event_sequence();
        let event = match action {
            InputAction::Preedit(scalar) => RoutedInputEvent::preedit(
                sequence,
                context.context_id(),
                context.target(),
                self.semantic_revision,
                scalar,
            ),
            InputAction::Commit(text) => RoutedInputEvent::commit(
                sequence,
                context.context_id(),
                context.target(),
                self.semantic_revision,
                text,
            ),
            InputAction::DeleteSurrounding {
                before_scalars,
                after_scalars,
            } => RoutedInputEvent::delete_surrounding(
                sequence,
                context.context_id(),
                context.target(),
                self.semantic_revision,
                before_scalars,
                after_scalars,
            ),
        }
        .unwrap_or_else(|_| input_fail());
        self.send_event(event);
    }
}

fn write_input_bytes<const N: usize>(transport: u64, wire: &[u8; N]) {
    wait_writable(transport);
    let result = syscall(
        SyscallNumber::ChannelWriteBytes,
        transport,
        wire.as_ptr() as u64,
        N as u64,
    );
    if result.status != Status::Ok.raw() || result.out1 != N as u64 || result.out2 != 0 {
        input_fail();
    }
}

fn input_fail() -> ! {
    fail(FAIL_INPUT_SERVER_RUNTIME)
}
