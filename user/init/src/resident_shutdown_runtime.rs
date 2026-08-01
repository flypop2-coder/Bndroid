//! M66 complete resident dependency graph and platform-shutdown userspace.

use core::arch::asm;

use bndr_abi::{
    AppDataPrincipal, ChannelMessageKind, OBJECT_WAIT_TIMEOUT_INFINITE, ObjectSignals,
    PROCESS_SPAWN_FLAGS_NONE, ProcessTerminationReason, SERVICE_SHUTDOWN_FLAGS_NONE,
    SERVICE_SHUTDOWN_QUIESCE, SERVICE_SHUTDOWN_REGISTER, SHUTDOWN_SERVICE_ALL_MASK,
    SHUTDOWN_SERVICE_EDGE_COUNT, SHUTDOWN_SERVICE_NODE_COUNT, SYSTEM_SHUTDOWN_COMMIT,
    SYSTEM_SHUTDOWN_FLAGS_NONE, SYSTEM_SHUTDOWN_PREPARE, ShutdownServiceNode, Status,
    SyscallNumber, UserImageId,
};

use super::storage_server_runtime::{
    SERVER_IDLE_TAG, SERVER_READY_TAG, SERVER_SHUTDOWN_ACK_TAG, SERVER_SHUTDOWN_COMMAND_TAG,
    SERVER_SHUTDOWN_EXIT_CODE, resident_client_work, resident_storage_admission_probe,
};
use super::{
    INIT_READY_MAGIC, assert_stale_handle, exit_child, fail, object_wait,
    read_channel_envelope_now, read_scalar_envelope, syscall, transfer_write, write_scalar,
};

const M66_STORAGE_READY_PREFIX: u64 = 0x4d36_3647_0000_0000;
const M66_STORAGE_PROOF: u64 = 0x4d36_3650_080a_0301;

const GRAPH_CONFIG_TAG: u64 = 0x4d36_3647_434f_4e46;
const GRAPH_EDGE_TAG: u64 = 0x4d36_3647_4544_4745;
const GRAPH_READY_TAG: u64 = 0x4d36_3647_5244_5921;
const GRAPH_COMMAND_TAG: u64 = 0x4d36_3647_434d_4421;
const GRAPH_ACTIVE_TAG: u64 = 0x4d36_3647_4143_5456;
const GRAPH_PROBE_ACK_TAG: u64 = 0x4d36_3647_5052_4f42;
const GRAPH_QUIESCED_TAG: u64 = 0x4d36_3647_5155_4945;
const GRAPH_COMMAND_ACTIVATE: u64 = 1;
const GRAPH_COMMAND_PROBE_QUIESCE: u64 = 2;
const GRAPH_COMMAND_QUIESCE: u64 = 3;
const GRAPH_EDGE_WIRE_BYTES: usize = 16;
const GRAPH_MAX_INCIDENT_EDGES: usize = 3;
const NODE_EXIT_CODE_PREFIX: u64 = 0x4d36_3653_0000_0000;

const FAIL_ABI: u64 = 0x6601;
const FAIL_AUTHORITY: u64 = 0x6602;
const FAIL_CHANNEL: u64 = 0x6603;
const FAIL_SPAWN: u64 = 0x6604;
const FAIL_CONFIG: u64 = 0x6605;
const FAIL_EDGE: u64 = 0x6606;
const FAIL_REGISTER: u64 = 0x6607;
const FAIL_ACTIVE: u64 = 0x6608;
const FAIL_ORDER: u64 = 0x6609;
const FAIL_QUIESCE: u64 = 0x660a;
const FAIL_WAIT: u64 = 0x660b;
const FAIL_STORAGE: u64 = 0x660c;
const FAIL_READY: u64 = 0x660d;

#[derive(Clone, Copy)]
struct GraphEdge {
    consumer: ShutdownServiceNode,
    provider: ShutdownServiceNode,
}

const GRAPH_EDGES: [GraphEdge; SHUTDOWN_SERVICE_EDGE_COUNT] = [
    GraphEdge {
        consumer: ShutdownServiceNode::Provider,
        provider: ShutdownServiceNode::ServiceManager,
    },
    GraphEdge {
        consumer: ShutdownServiceNode::PrimaryClient,
        provider: ShutdownServiceNode::ServiceManager,
    },
    GraphEdge {
        consumer: ShutdownServiceNode::SecondaryClient,
        provider: ShutdownServiceNode::ServiceManager,
    },
    GraphEdge {
        consumer: ShutdownServiceNode::PrimaryClient,
        provider: ShutdownServiceNode::Provider,
    },
    GraphEdge {
        consumer: ShutdownServiceNode::SecondaryClient,
        provider: ShutdownServiceNode::Provider,
    },
    GraphEdge {
        consumer: ShutdownServiceNode::InputServer,
        provider: ShutdownServiceNode::SurfaceServer,
    },
    GraphEdge {
        consumer: ShutdownServiceNode::Launcher,
        provider: ShutdownServiceNode::SurfaceServer,
    },
    GraphEdge {
        consumer: ShutdownServiceNode::App,
        provider: ShutdownServiceNode::SurfaceServer,
    },
    GraphEdge {
        consumer: ShutdownServiceNode::Launcher,
        provider: ShutdownServiceNode::InputServer,
    },
    GraphEdge {
        consumer: ShutdownServiceNode::App,
        provider: ShutdownServiceNode::InputServer,
    },
];

#[derive(Clone, Copy)]
struct Child {
    control: u64,
    pid: u64,
    node: Option<ShutdownServiceNode>,
}

#[derive(Clone, Copy)]
struct GraphHandle {
    handle: u64,
    peer_is_dependent: bool,
}

impl GraphHandle {
    const EMPTY: Self = Self {
        handle: 0,
        peer_is_dependent: false,
    };
}

pub(super) fn init_runtime(system_root: u64) -> ! {
    expect_abi();
    expect_result(
        syscall(SyscallNumber::StorageAcquire, 0, 0, 0),
        Status::PermissionDenied,
        FAIL_AUTHORITY,
    );
    if system_root != 0 {
        close_handle(system_root, FAIL_CHANNEL);
    }

    let server = spawn(UserImageId::StorageServer, None);
    let boot_generation = expect_server_generation(server, SERVER_READY_TAG);
    if boot_generation == 0 || boot_generation & !u64::from(u32::MAX) != 0 {
        fail(FAIL_STORAGE);
    }
    expect_result(
        syscall(
            SyscallNumber::SystemShutdown,
            SYSTEM_SHUTDOWN_PREPARE,
            boot_generation,
            SYSTEM_SHUTDOWN_FLAGS_NONE,
        ),
        Status::InvalidState,
        FAIL_AUTHORITY,
    );

    let children = [
        spawn(
            UserImageId::ServiceManager,
            Some(ShutdownServiceNode::ServiceManager),
        ),
        spawn(UserImageId::Provider, Some(ShutdownServiceNode::Provider)),
        spawn(
            UserImageId::Client,
            Some(ShutdownServiceNode::PrimaryClient),
        ),
        spawn(
            UserImageId::Client,
            Some(ShutdownServiceNode::SecondaryClient),
        ),
        spawn(
            UserImageId::SurfaceServer,
            Some(ShutdownServiceNode::SurfaceServer),
        ),
        spawn(
            UserImageId::InputServer,
            Some(ShutdownServiceNode::InputServer),
        ),
        spawn(UserImageId::Launcher, Some(ShutdownServiceNode::Launcher)),
        spawn(UserImageId::App, Some(ShutdownServiceNode::App)),
    ];
    for child in children {
        let node = child.node.unwrap_or_else(|| fail(FAIL_CONFIG));
        write_scalar(
            child.control,
            GRAPH_CONFIG_TAG,
            (node.raw() << 32) | incident_edge_count(node) as u64,
        );
    }
    distribute_graph_edges(&children);
    for child in children {
        expect_node_message(child, GRAPH_READY_TAG, |node, payload| {
            payload == node.dependency_mask()
        });
    }

    expect_result(
        syscall(
            SyscallNumber::ServiceShutdown,
            SERVICE_SHUTDOWN_REGISTER,
            ShutdownServiceNode::App.raw(),
            ShutdownServiceNode::App.dependency_mask(),
        ),
        Status::PermissionDenied,
        FAIL_AUTHORITY,
    );

    for child in children {
        write_scalar(child.control, GRAPH_COMMAND_TAG, GRAPH_COMMAND_ACTIVATE);
    }
    let mut launcher_stage = 0_u64;
    let mut app_stage = 0_u64;
    for child in children {
        let stage = expect_node_message(child, GRAPH_ACTIVE_TAG, |_, value| value <= 2);
        match child.node.unwrap_or_else(|| fail(FAIL_ACTIVE)) {
            ShutdownServiceNode::Launcher => launcher_stage = stage,
            ShutdownServiceNode::App => app_stage = stage,
            _ if stage == 0 => {}
            _ => fail(FAIL_ACTIVE),
        }
    }
    let final_generation = expect_server_generation(server, SERVER_IDLE_TAG);
    validate_storage_proof(boot_generation, final_generation, launcher_stage, app_stage);

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
        fail(FAIL_READY);
    }
    exercise_spawn_barrier();

    let surface = children[ShutdownServiceNode::SurfaceServer.raw() as usize];
    write_scalar(
        surface.control,
        GRAPH_COMMAND_TAG,
        GRAPH_COMMAND_PROBE_QUIESCE,
    );
    expect_node_message(surface, GRAPH_PROBE_ACK_TAG, |node, value| {
        node == ShutdownServiceNode::SurfaceServer && value == 0
    });

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
        quiesce_child(children[node.raw() as usize], node, quiesced_mask);
    }
    if quiesced_mask != SHUTDOWN_SERVICE_ALL_MASK {
        fail(FAIL_QUIESCE);
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

    let ready = syscall(
        SyscallNumber::InitReady,
        INIT_READY_MAGIC,
        M66_STORAGE_READY_PREFIX | final_generation,
        M66_STORAGE_PROOF,
    );
    if ready.status != Status::Ok.raw() || ready.out1 != 0 || ready.out2 != 0 {
        fail(FAIL_READY);
    }
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
        fail(FAIL_READY);
    }
    loop {
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

pub(super) fn node_runtime(startup: u64, expected_image: UserImageId) -> ! {
    expect_abi();
    let (init_pid, (tag, configuration)) = read_scalar_envelope(startup);
    let raw_node = configuration >> 32;
    let expected_edges = configuration as u32 as usize;
    let Some(node) = ShutdownServiceNode::from_raw(raw_node) else {
        fail(FAIL_CONFIG);
    };
    if init_pid == 0
        || tag != GRAPH_CONFIG_TAG
        || node.image_id() != expected_image
        || expected_edges != incident_edge_count(node)
        || expected_edges > GRAPH_MAX_INCIDENT_EDGES
    {
        fail(FAIL_CONFIG);
    }

    let mut handles = [GraphHandle::EMPTY; GRAPH_MAX_INCIDENT_EDGES];
    let mut seen_edges = 0_u16;
    for slot in handles.iter_mut().take(expected_edges) {
        let (edge_index, handle) = receive_graph_edge(startup, init_pid, node);
        let bit = 1_u16 << edge_index;
        if seen_edges & bit != 0 {
            close_handle(handle.handle, FAIL_EDGE);
            fail(FAIL_EDGE);
        }
        seen_edges |= bit;
        *slot = handle;
    }
    if seen_edges != incident_edge_mask(node) {
        fail(FAIL_EDGE);
    }

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
    write_scalar(
        startup,
        GRAPH_READY_TAG,
        pack_node_value(node, node.dependency_mask()),
    );

    let (sender, (tag, command)) = read_scalar_envelope(startup);
    if sender != init_pid || tag != GRAPH_COMMAND_TAG || command != GRAPH_COMMAND_ACTIVATE {
        fail(FAIL_ACTIVE);
    }
    let stage = match node {
        ShutdownServiceNode::Launcher => resident_client_work(startup, AppDataPrincipal::LAUNCHER),
        ShutdownServiceNode::App => resident_client_work(startup, AppDataPrincipal::PRIMARY_APP),
        _ => 0,
    };
    write_scalar(startup, GRAPH_ACTIVE_TAG, pack_node_value(node, stage));

    loop {
        let (sender, (tag, command)) = read_scalar_envelope(startup);
        if sender != init_pid || tag != GRAPH_COMMAND_TAG {
            fail(FAIL_QUIESCE);
        }
        if command == GRAPH_COMMAND_PROBE_QUIESCE {
            if node != ShutdownServiceNode::SurfaceServer {
                fail(FAIL_ORDER);
            }
            expect_result(
                syscall(
                    SyscallNumber::ServiceShutdown,
                    SERVICE_SHUTDOWN_QUIESCE,
                    node.raw(),
                    SERVICE_SHUTDOWN_FLAGS_NONE,
                ),
                Status::InvalidState,
                FAIL_ORDER,
            );
            write_scalar(startup, GRAPH_PROBE_ACK_TAG, pack_node_value(node, 0));
            continue;
        }
        if command != GRAPH_COMMAND_QUIESCE {
            fail(FAIL_QUIESCE);
        }

        if matches!(
            node,
            ShutdownServiceNode::Launcher | ShutdownServiceNode::App
        ) {
            resident_storage_admission_probe();
        }
        for edge in handles.iter().take(expected_edges) {
            if edge.peer_is_dependent {
                let waited = object_wait(edge.handle, ObjectSignals::PEER_CLOSED);
                if waited.status != Status::Ok.raw()
                    || waited.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
                    || waited.out2 != 0
                {
                    fail(FAIL_ORDER);
                }
            }
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
        for edge in handles.iter().take(expected_edges) {
            close_handle(edge.handle, FAIL_EDGE);
        }
        write_scalar(
            startup,
            GRAPH_QUIESCED_TAG,
            pack_node_value(node, quiesced.out1),
        );
        exit_child(NODE_EXIT_CODE_PREFIX | (node.raw() + 1));
    }
}

fn distribute_graph_edges(children: &[Child; SHUTDOWN_SERVICE_NODE_COUNT]) {
    for (edge_index, edge) in GRAPH_EDGES.into_iter().enumerate() {
        let channel = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
        if channel.status != Status::Ok.raw()
            || channel.out1 == 0
            || channel.out2 == 0
            || channel.out1 == channel.out2
        {
            fail(FAIL_CHANNEL);
        }
        send_graph_edge(
            children[edge.consumer.raw() as usize],
            channel.out1,
            edge_index,
            false,
        );
        send_graph_edge(
            children[edge.provider.raw() as usize],
            channel.out2,
            edge_index,
            true,
        );
    }
}

fn send_graph_edge(child: Child, handle: u64, edge_index: usize, peer_is_dependent: bool) {
    let mut wire = [0_u8; GRAPH_EDGE_WIRE_BYTES];
    wire[..8].copy_from_slice(&GRAPH_EDGE_TAG.to_le_bytes());
    let metadata = edge_index as u64 | (u64::from(peer_is_dependent) << 8);
    wire[8..].copy_from_slice(&metadata.to_le_bytes());
    let sent = transfer_write(
        child.control,
        wire.as_ptr() as u64,
        handle,
        GRAPH_EDGE_WIRE_BYTES,
    );
    if sent.status != Status::Ok.raw()
        || sent.out1 != GRAPH_EDGE_WIRE_BYTES as u64
        || sent.out2 != 0
    {
        fail(FAIL_EDGE);
    }
    assert_stale_handle(handle);
}

fn receive_graph_edge(
    startup: u64,
    init_pid: u64,
    node: ShutdownServiceNode,
) -> (usize, GraphHandle) {
    let signals = ObjectSignals::from_bits(
        ObjectSignals::READABLE.bits() | ObjectSignals::PEER_CLOSED.bits(),
    )
    .unwrap_or_else(|| fail(FAIL_EDGE));
    let waited = object_wait(startup, signals);
    if waited.status != Status::Ok.raw()
        || waited.out1 & u64::from(ObjectSignals::READABLE.bits()) == 0
        || waited.out2 != 0
    {
        fail(FAIL_EDGE);
    }
    let envelope = read_channel_envelope_now(startup);
    if envelope.sender_pid() != init_pid
        || envelope.kind() != ChannelMessageKind::Transfer
        || envelope.logical_length() != GRAPH_EDGE_WIRE_BYTES
        || !envelope.received_handle().is_valid()
    {
        fail(FAIL_EDGE);
    }
    let data = envelope.data();
    let tag = read_u64(&data[..8]);
    let metadata = read_u64(&data[8..16]);
    let edge_index = metadata as u8 as usize;
    let peer_is_dependent = metadata >> 8 == 1;
    let Some(edge) = GRAPH_EDGES.get(edge_index) else {
        fail(FAIL_EDGE);
    };
    if tag != GRAPH_EDGE_TAG
        || (node == edge.consumer && peer_is_dependent)
        || (node == edge.provider && !peer_is_dependent)
        || (node != edge.consumer && node != edge.provider)
    {
        fail(FAIL_EDGE);
    }
    (
        edge_index,
        GraphHandle {
            handle: u64::from(envelope.received_handle().raw()),
            peer_is_dependent,
        },
    )
}

fn quiesce_child(child: Child, expected_node: ShutdownServiceNode, expected_mask: u64) {
    if child.node != Some(expected_node) {
        fail(FAIL_QUIESCE);
    }
    write_scalar(child.control, GRAPH_COMMAND_TAG, GRAPH_COMMAND_QUIESCE);
    let mask = expect_node_message(child, GRAPH_QUIESCED_TAG, |node, value| {
        node == expected_node && value == expected_mask
    });
    if mask != expected_mask {
        fail(FAIL_QUIESCE);
    }
    wait_for_exit(child, NODE_EXIT_CODE_PREFIX | (expected_node.raw() + 1));
}

fn wait_for_exit(child: Child, expected_exit_code: u64) {
    let waited = syscall(SyscallNumber::ProcessWait, child.pid, 0, 0);
    if waited.status != Status::Ok.raw()
        || waited.out1 != expected_exit_code
        || waited.out2 != ProcessTerminationReason::Exited.raw()
    {
        fail(FAIL_WAIT);
    }
    close_handle(child.control, FAIL_CHANNEL);
}

fn expect_node_message(
    child: Child,
    expected_tag: u64,
    validate: impl FnOnce(ShutdownServiceNode, u64) -> bool,
) -> u64 {
    let (sender, (tag, payload)) = read_scalar_envelope(child.control);
    let raw_node = payload >> 56;
    let value = payload & 0x00ff_ffff_ffff_ffff;
    let Some(node) = ShutdownServiceNode::from_raw(raw_node) else {
        fail(FAIL_CONFIG);
    };
    if sender != child.pid
        || tag != expected_tag
        || child.node != Some(node)
        || !validate(node, value)
    {
        fail(FAIL_CONFIG);
    }
    value
}

fn expect_server_generation(server: Child, expected_tag: u64) -> u64 {
    let (sender, (tag, generation)) = read_scalar_envelope(server.control);
    if sender != server.pid || tag != expected_tag || generation == 0 {
        fail(FAIL_STORAGE);
    }
    generation
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
        fail(FAIL_CHANNEL);
    }
    expect_result(
        syscall(
            SyscallNumber::ProcessSpawn,
            channel.out2,
            UserImageId::Launcher.raw(),
            PROCESS_SPAWN_FLAGS_NONE,
        ),
        Status::InvalidState,
        FAIL_AUTHORITY,
    );
    close_handle(channel.out1, FAIL_CHANNEL);
    close_handle(channel.out2, FAIL_CHANNEL);
}

fn spawn(image: UserImageId, node: Option<ShutdownServiceNode>) -> Child {
    let channel = syscall(SyscallNumber::ChannelCreate, 0, 0, 0);
    if channel.status != Status::Ok.raw()
        || channel.out1 == 0
        || channel.out2 == 0
        || channel.out1 == channel.out2
    {
        fail(FAIL_CHANNEL);
    }
    let spawned = syscall(
        SyscallNumber::ProcessSpawn,
        channel.out2,
        image.raw(),
        PROCESS_SPAWN_FLAGS_NONE,
    );
    if spawned.status != Status::Ok.raw() || spawned.out1 == 0 || spawned.out2 != 0 {
        close_handle(channel.out1, FAIL_CHANNEL);
        close_handle(channel.out2, FAIL_CHANNEL);
        fail(FAIL_SPAWN);
    }
    assert_stale_handle(channel.out2);
    Child {
        control: channel.out1,
        pid: spawned.out1,
        node,
    }
}

fn incident_edge_count(node: ShutdownServiceNode) -> usize {
    GRAPH_EDGES
        .iter()
        .filter(|edge| edge.consumer == node || edge.provider == node)
        .count()
}

fn incident_edge_mask(node: ShutdownServiceNode) -> u16 {
    let mut mask = 0_u16;
    for (index, edge) in GRAPH_EDGES.iter().enumerate() {
        if edge.consumer == node || edge.provider == node {
            mask |= 1_u16 << index;
        }
    }
    mask
}

const fn pack_node_value(node: ShutdownServiceNode, value: u64) -> u64 {
    (node.raw() << 56) | (value & 0x00ff_ffff_ffff_ffff)
}

fn read_u64(bytes: &[u8]) -> u64 {
    let mut encoded = [0_u8; 8];
    encoded.copy_from_slice(bytes);
    u64::from_le_bytes(encoded)
}

fn close_handle(handle: u64, reason: u64) {
    let closed = syscall(SyscallNumber::HandleClose, handle, 0, 0);
    if closed.status != Status::Ok.raw() || closed.out1 != 0 || closed.out2 != 0 {
        fail(reason);
    }
}

fn expect_abi() {
    let abi = syscall(SyscallNumber::AbiVersion, 0, 0, 0);
    if abi.status != Status::Ok.raw() || abi.out1 != bndr_abi::ABI_VERSION || abi.out2 != 0 {
        fail(FAIL_ABI);
    }
}

fn expect_result(result: super::SyscallResult, status: Status, reason: u64) {
    if result.status != status.raw() || result.out1 != 0 || result.out2 != 0 {
        fail(reason);
    }
}

const _: () = assert!(SHUTDOWN_SERVICE_NODE_COUNT == 8);
const _: () = assert!(SHUTDOWN_SERVICE_EDGE_COUNT == 10);
const _: () = assert!(OBJECT_WAIT_TIMEOUT_INFINITE == u64::MAX);
