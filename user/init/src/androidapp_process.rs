//! ABI 47's capability-minimal Android compatibility worker and its trusted
//! App-side RPC client.
//!
//! The worker owns the immutable APK copy, signature/digest re-verification,
//! bounded interpreter, and retained Activity session. It never receives a
//! Surface, graphics buffer, input stream, storage handle, or system channel.
//! The ordinary App process remains the trusted UI host and turns only
//! authenticated, canonical scene results into pixels.

use bndr_abi::{
    ABI_VERSION, ANDROID_APP_BUTTON_MAX_BYTES, ANDROID_APP_LABEL_MAX_BYTES,
    ANDROID_APP_MESSAGE_PAYLOAD_BYTES, ANDROID_APP_MESSAGE_WIRE_SIZE,
    ANDROID_PACKAGE_IMAGE_CLAIM_WIRE_SIZE, AndroidAppMessage, AndroidAppMessageKind,
    AndroidPackageImageClaim, ChannelMessageKind, GRAPHICS_BUFFER_CREATE_FLAGS_NONE,
    GRAPHICS_BUFFER_FORMAT_XRGB8888, GRAPHICS_BUFFER_HEIGHT, GRAPHICS_BUFFER_WIDTH, Status,
    SyscallNumber, VMO_READ_MAX_BYTES, pack_graphics_buffer_geometry, pack_vmo_read,
};
#[cfg(feature = "androidbox-scene-rpc2")]
use bndr_abi::{
    ANDROID_APP_SCENE_MAX_NODES, ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE,
    ANDROID_APP_SCENE_NODE_TEXT_MAX_BYTES, AndroidAppSceneDimension, AndroidAppSceneNodeDescriptor,
    AndroidAppSceneNodeKind, AndroidAppSceneOrientation,
};
#[cfg(feature = "androidbox-restart0")]
use bndr_abi::{
    AndroidAppSupervisorMessage, AndroidAppSupervisorMessageKind, ObjectSignals,
    ProcessTerminationReason,
};
use bndr_ui::UiCompatibleActivityIdentity;
#[cfg(feature = "androidbox-scene-rpc2")]
use bndr_ui::android_scene::{
    AndroidInstalledActivitySceneNode, AndroidInstalledActivitySceneState, AndroidSceneLayoutSize,
    AndroidSceneOrientation, AndroidSceneViewKind,
};
use bndr_ui::mobile::AndroidInstalledAppStatus;

#[cfg(feature = "androidbox-scene-rpc2")]
use crate::androidbox_installed_runtime::InstalledInteractiveSceneNode;
use crate::androidbox_installed_runtime::{
    InstalledInteractiveActivityLease, InstalledInteractiveActivitySnapshot,
    InstalledInteractiveActivityUpdate, InstalledInteractiveAscii,
};

use super::{
    FAIL_SURFACE_PROTOCOL, OwnedUserHandle, close_owned, read_android_installed_app_status,
    read_channel_envelope, syscall, wait_writable,
};
#[cfg(feature = "androidbox-restart0")]
use super::{
    object_wait, read_android_app_supervisor_transfer, write_android_app_supervisor_bytes,
};

const ERROR_OPEN_REJECTED: u32 = 1;
const ERROR_CLICK_REJECTED: u32 = 2;
const ERROR_CLOSE_REJECTED: u32 = 3;
const ERROR_BUSY: u32 = 4;

// The kernel Channel has eight slots per direction. One maximum-size Open
// response is exactly one header, four label chunks, and three button chunks.
// Requiring every non-final chunk to use the complete payload is therefore a
// protocol invariant, not merely an encoding preference. The synchronous
// `&mut AndroidAppClient` API consumes a complete response before another
// request can be issued, so the trusted side has exactly one request in flight.
const ANDROID_APP_CHANNEL_QUEUE_CAPACITY: usize = 8;
const ANDROID_APP_MAX_OUTSTANDING_REQUESTS: usize = 1;
#[cfg(not(feature = "androidbox-scene-rpc2"))]
const ANDROID_APP_OPEN_RESPONSE_MAX_MESSAGES: usize = 1
    + text_chunk_count(ANDROID_APP_LABEL_MAX_BYTES)
    + text_chunk_count(ANDROID_APP_BUTTON_MAX_BYTES);
#[cfg(feature = "androidbox-scene-rpc2")]
const ANDROID_APP_NODE_RESPONSE_MAX_MESSAGES: usize =
    1 + text_chunk_count(ANDROID_APP_SCENE_NODE_TEXT_MAX_BYTES);
const ANDROID_APP_UPDATE_RESPONSE_MAX_MESSAGES: usize =
    1 + text_chunk_count(ANDROID_APP_LABEL_MAX_BYTES);

const _: () = {
    assert!(ANDROID_APP_MESSAGE_PAYLOAD_BYTES != 0);
    assert!(ANDROID_APP_MAX_OUTSTANDING_REQUESTS == 1);
    assert!(ANDROID_APP_UPDATE_RESPONSE_MAX_MESSAGES <= ANDROID_APP_CHANNEL_QUEUE_CAPACITY);
};
#[cfg(not(feature = "androidbox-scene-rpc2"))]
const _: () = assert!(ANDROID_APP_OPEN_RESPONSE_MAX_MESSAGES == ANDROID_APP_CHANNEL_QUEUE_CAPACITY);
#[cfg(feature = "androidbox-scene-rpc2")]
const _: () = assert!(ANDROID_APP_NODE_RESPONSE_MAX_MESSAGES <= ANDROID_APP_CHANNEL_QUEUE_CAPACITY);

const fn text_chunk_count(length: usize) -> usize {
    length.div_ceil(ANDROID_APP_MESSAGE_PAYLOAD_BYTES)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AndroidAppClientError {
    RemoteRejected,
    InvalidState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AndroidAppRemoteSnapshot {
    pub(super) label_id: u32,
    pub(super) button_id: u32,
    pub(super) label_text: InstalledInteractiveAscii,
    pub(super) button_text: InstalledInteractiveAscii,
    pub(super) revision: u32,
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub(super) scene_nodes: [AndroidInstalledActivitySceneNode; ANDROID_APP_SCENE_MAX_NODES],
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub(super) scene_node_count: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AndroidAppRemoteUpdate {
    pub(super) label_id: u32,
    pub(super) label_text: InstalledInteractiveAscii,
    pub(super) revision: u32,
    #[cfg(feature = "androidbox-dex-methods8")]
    pub(super) app_defined_call_count: u8,
    #[cfg(feature = "androidbox-dex-instance9")]
    pub(super) app_defined_instance_call_count: u8,
    #[cfg(feature = "androidbox-activity-fields10")]
    pub(super) activity_field_read_count: u8,
    #[cfg(feature = "androidbox-string-text12")]
    pub(super) direct_string_text: bool,
    #[cfg(feature = "androidbox-string-builder13")]
    pub(super) dynamic_string_text: bool,
    #[cfg(feature = "androidbox-activity-state11")]
    pub(super) activity_int_state_value: u8,
}

pub(super) struct AndroidAppClient {
    channel: OwnedUserHandle,
    worker_pid: u64,
    next_request_id: u64,
    active_session_id: u64,
    active_generation: u64,
    label_id: u32,
    button_id: u32,
    remote_revision: u32,
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_nodes: [AndroidInstalledActivitySceneNode; ANDROID_APP_SCENE_MAX_NODES],
    #[cfg(feature = "androidbox-scene-rpc2")]
    scene_node_count: u8,
    #[cfg(feature = "androidbox-restart0")]
    supervisor: OwnedUserHandle,
    #[cfg(feature = "androidbox-restart0")]
    init_pid: u64,
    #[cfg(feature = "androidbox-restart0")]
    restart_epoch: u64,
}

impl AndroidAppClient {
    #[cfg(not(feature = "androidbox-restart0"))]
    pub(super) fn connect(channel: OwnedUserHandle) -> Self {
        let worker_pid = read_worker_ready(channel.raw(), None);
        Self {
            channel,
            worker_pid,
            next_request_id: 1,
            active_session_id: 0,
            active_generation: 0,
            label_id: 0,
            button_id: 0,
            remote_revision: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_nodes: [AndroidInstalledActivitySceneNode::empty(); ANDROID_APP_SCENE_MAX_NODES],
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_node_count: 0,
        }
    }

    #[cfg(feature = "androidbox-restart0")]
    pub(super) fn connect(
        channel: OwnedUserHandle,
        supervisor: OwnedUserHandle,
        init_pid: u64,
    ) -> Self {
        if init_pid == 0 {
            super::fail(FAIL_SURFACE_PROTOCOL);
        }
        let worker_pid = read_worker_ready(channel.raw(), None);
        if worker_pid == init_pid {
            super::fail(FAIL_SURFACE_PROTOCOL);
        }
        Self {
            channel,
            worker_pid,
            next_request_id: 1,
            active_session_id: 0,
            active_generation: 0,
            label_id: 0,
            button_id: 0,
            remote_revision: 0,
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_nodes: [AndroidInstalledActivitySceneNode::empty(); ANDROID_APP_SCENE_MAX_NODES],
            #[cfg(feature = "androidbox-scene-rpc2")]
            scene_node_count: 0,
            supervisor,
            init_pid,
            restart_epoch: 0,
        }
    }

    pub(super) fn open(
        &mut self,
        identity: UiCompatibleActivityIdentity,
    ) -> Result<AndroidAppRemoteSnapshot, AndroidAppClientError> {
        if self.active_session_id != 0 {
            return Err(AndroidAppClientError::InvalidState);
        }
        let request_id = self.take_request_id();
        self.write(
            AndroidAppMessage::new(
                AndroidAppMessageKind::Open,
                request_id,
                identity.session_id(),
                identity.package_generation(),
                &[],
            )
            .map_err(|_| AndroidAppClientError::InvalidState)?,
        );
        let opened = self.read_for_request(request_id)?;
        if opened.kind() == AndroidAppMessageKind::Error {
            if opened.arg0() != u64::from(ERROR_OPEN_REJECTED) {
                self.protocol_fail();
            }
            self.clear_active();
            return Err(AndroidAppClientError::RemoteRejected);
        }
        #[cfg(not(feature = "androidbox-scene-rpc2"))]
        {
            if opened.kind() != AndroidAppMessageKind::Opened {
                self.protocol_fail();
            }
            let label_id = (opened.arg0() >> 32) as u32;
            let button_id = opened.arg0() as u32;
            let revision = (opened.arg1() >> 32) as u32;
            let label_len = ((opened.arg1() >> 16) & 0xffff) as usize;
            let button_len = (opened.arg1() & 0xffff) as usize;
            if label_id == 0
                || button_id == 0
                || label_id == button_id
                || revision != 0
                || !(1..=ANDROID_APP_LABEL_MAX_BYTES).contains(&label_len)
                || !(1..=ANDROID_APP_BUTTON_MAX_BYTES).contains(&button_len)
            {
                self.protocol_fail();
            }
            let label_text =
                self.read_text(request_id, AndroidAppMessageKind::LabelChunk, label_len);
            let button_text =
                self.read_text(request_id, AndroidAppMessageKind::ButtonChunk, button_len);
            self.active_session_id = identity.session_id();
            self.active_generation = identity.package_generation();
            self.label_id = label_id;
            self.button_id = button_id;
            self.remote_revision = revision;
            let snapshot = AndroidAppRemoteSnapshot {
                label_id,
                button_id,
                label_text,
                button_text,
                revision,
            };
            #[cfg(feature = "androidbox-restart0")]
            if self.restart_epoch == 0 {
                return self.controlled_crash_and_reopen(identity);
            }
            Ok(snapshot)
        }
        #[cfg(feature = "androidbox-scene-rpc2")]
        {
            self.open_scene(identity, opened)
        }
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    fn open_scene(
        &mut self,
        identity: UiCompatibleActivityIdentity,
        opened: AndroidAppMessage,
    ) -> Result<AndroidAppRemoteSnapshot, AndroidAppClientError> {
        if opened.kind() != AndroidAppMessageKind::SceneOpened
            || opened.arg1() != 0
            || !(1..=ANDROID_APP_SCENE_MAX_NODES as u64).contains(&opened.arg0())
        {
            self.protocol_fail();
        }
        let node_count = u8::try_from(opened.arg0()).unwrap_or_else(|_| self.protocol_fail());
        self.active_session_id = identity.session_id();
        self.active_generation = identity.package_generation();
        self.remote_revision = 0;

        let mut nodes = [AndroidInstalledActivitySceneNode::empty(); ANDROID_APP_SCENE_MAX_NODES];
        for index in 0..node_count {
            nodes[usize::from(index)] = self.describe_scene_node(index);
        }
        let validated =
            AndroidInstalledActivitySceneState::try_new(&nodes[..usize::from(node_count)], 1)
                .unwrap_or_else(|| self.protocol_fail());
        let Some(button_id) = validated.callback_button_id() else {
            self.protocol_fail();
        };
        let Some(label) = validated
            .nodes()
            .iter()
            .find(|node| node.kind() == AndroidSceneViewKind::TextView)
        else {
            self.protocol_fail();
        };
        let label_id = label.id();
        let label_text = InstalledInteractiveAscii::from_ascii(label.text().as_bytes())
            .unwrap_or_else(|| self.protocol_fail());
        let (_, button) = validated
            .find_by_id(button_id)
            .unwrap_or_else(|| self.protocol_fail());
        let button_text = InstalledInteractiveAscii::from_ascii(button.text().as_bytes())
            .unwrap_or_else(|| self.protocol_fail());

        self.label_id = 0;
        self.button_id = button_id;
        self.scene_nodes = nodes;
        self.scene_node_count = node_count;
        let snapshot = AndroidAppRemoteSnapshot {
            label_id,
            button_id,
            label_text,
            button_text,
            revision: 0,
            scene_nodes: nodes,
            scene_node_count: node_count,
        };
        if self.restart_epoch == 0 {
            return self.controlled_crash_and_reopen(identity);
        }
        Ok(snapshot)
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    fn describe_scene_node(&mut self, index: u8) -> AndroidInstalledActivitySceneNode {
        let request_id = self.take_request_id();
        let message = AndroidAppMessage::new(
            AndroidAppMessageKind::DescribeNode,
            request_id,
            self.active_session_id,
            (u64::from(self.remote_revision) << 32) | u64::from(index),
            &[],
        )
        .unwrap_or_else(|_| self.protocol_fail());
        self.write(message);
        let node = self
            .read_for_request(request_id)
            .unwrap_or_else(|_| self.protocol_fail());
        if node.kind() != AndroidAppMessageKind::Node
            || node.arg0() != u64::from(index)
            || node.arg1() != u64::from(self.remote_revision)
            || node.payload().len() != ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE
        {
            self.protocol_fail();
        }
        let mut wire = [0; ANDROID_APP_SCENE_NODE_DESCRIPTOR_WIRE_SIZE];
        wire.copy_from_slice(node.payload());
        let descriptor =
            AndroidAppSceneNodeDescriptor::decode(&wire).unwrap_or_else(|| self.protocol_fail());
        if !descriptor.is_canonical_for_index(index) {
            self.protocol_fail();
        }
        let text_len = usize::from(descriptor.text_len());
        let text = if text_len == 0 {
            InstalledInteractiveAscii::empty()
        } else {
            self.read_scene_text(request_id, index, text_len)
        };
        #[cfg(feature = "androidbox-layout-size18")]
        let scene_node = AndroidInstalledActivitySceneNode::try_new_sized(
            scene_view_kind_from_wire(descriptor.kind()),
            descriptor.parent(),
            descriptor.id(),
            scene_layout_size_from_wire(descriptor.width()),
            scene_layout_size_from_wire(descriptor.height()),
            scene_orientation_from_wire(descriptor.orientation()),
            text.as_str(),
            descriptor.callback(),
            descriptor.layout_weight(),
            descriptor.layout_margin_left_dp(),
            descriptor.layout_margin_top_dp(),
            descriptor.layout_margin_right_dp(),
            descriptor.layout_margin_bottom_dp(),
            descriptor.padding_left_dp(),
            descriptor.padding_top_dp(),
            descriptor.padding_right_dp(),
            descriptor.padding_bottom_dp(),
            descriptor.exact_width_dp(),
            descriptor.exact_height_dp(),
        );
        #[cfg(all(
            feature = "androidbox-layout-directional17",
            not(feature = "androidbox-layout-size18")
        ))]
        let scene_node = AndroidInstalledActivitySceneNode::try_new_directional(
            scene_view_kind_from_wire(descriptor.kind()),
            descriptor.parent(),
            descriptor.id(),
            scene_layout_size_from_wire(descriptor.width()),
            scene_layout_size_from_wire(descriptor.height()),
            scene_orientation_from_wire(descriptor.orientation()),
            text.as_str(),
            descriptor.callback(),
            descriptor.layout_weight(),
            descriptor.layout_margin_left_dp(),
            descriptor.layout_margin_top_dp(),
            descriptor.layout_margin_right_dp(),
            descriptor.layout_margin_bottom_dp(),
            descriptor.padding_left_dp(),
            descriptor.padding_top_dp(),
            descriptor.padding_right_dp(),
            descriptor.padding_bottom_dp(),
        );
        #[cfg(all(
            feature = "androidbox-layout-spacing16",
            not(feature = "androidbox-layout-directional17")
        ))]
        let scene_node = AndroidInstalledActivitySceneNode::try_new_spaced(
            scene_view_kind_from_wire(descriptor.kind()),
            descriptor.parent(),
            descriptor.id(),
            scene_layout_size_from_wire(descriptor.width()),
            scene_layout_size_from_wire(descriptor.height()),
            scene_orientation_from_wire(descriptor.orientation()),
            text.as_str(),
            descriptor.callback(),
            descriptor.layout_weight(),
            descriptor.layout_margin_dp(),
            descriptor.padding_dp(),
        );
        #[cfg(all(
            feature = "androidbox-layout-weight15",
            not(feature = "androidbox-layout-spacing16")
        ))]
        let scene_node = AndroidInstalledActivitySceneNode::try_new_weighted(
            scene_view_kind_from_wire(descriptor.kind()),
            descriptor.parent(),
            descriptor.id(),
            scene_layout_size_from_wire(descriptor.width()),
            scene_layout_size_from_wire(descriptor.height()),
            scene_orientation_from_wire(descriptor.orientation()),
            text.as_str(),
            descriptor.callback(),
            descriptor.layout_weight(),
        );
        #[cfg(not(feature = "androidbox-layout-weight15"))]
        let scene_node = AndroidInstalledActivitySceneNode::try_new(
            scene_view_kind_from_wire(descriptor.kind()),
            descriptor.parent(),
            descriptor.id(),
            scene_layout_size_from_wire(descriptor.width()),
            scene_layout_size_from_wire(descriptor.height()),
            scene_orientation_from_wire(descriptor.orientation()),
            text.as_str(),
            descriptor.callback(),
        );
        scene_node.unwrap_or_else(|| self.protocol_fail())
    }

    pub(super) fn dispatch_button_click(
        &mut self,
        view_id: u32,
    ) -> Result<AndroidAppRemoteUpdate, AndroidAppClientError> {
        #[cfg(not(feature = "androidbox-multiaction3"))]
        let callback_matches = view_id == self.button_id;
        #[cfg(feature = "androidbox-multiaction3")]
        let callback_matches = self.scene_nodes[..usize::from(self.scene_node_count)]
            .iter()
            .any(|node| {
                node.id() == view_id
                    && node.kind() == AndroidSceneViewKind::Button
                    && node.callback_registered()
            });
        if self.active_session_id == 0 || view_id == 0 || !callback_matches {
            return Err(AndroidAppClientError::InvalidState);
        }
        let request_id = self.take_request_id();
        self.write(
            AndroidAppMessage::new(
                AndroidAppMessageKind::Click,
                request_id,
                self.active_session_id,
                (u64::from(view_id) << 32) | u64::from(self.remote_revision),
                &[],
            )
            .map_err(|_| AndroidAppClientError::InvalidState)?,
        );
        let updated = self.read_for_request(request_id)?;
        if updated.kind() == AndroidAppMessageKind::Error {
            if updated.arg0() != u64::from(ERROR_CLICK_REJECTED) {
                self.protocol_fail();
            }
            self.clear_active();
            return Err(AndroidAppClientError::RemoteRejected);
        }
        if updated.kind() != AndroidAppMessageKind::Updated {
            self.protocol_fail();
        }
        let label_id = updated.arg0() as u32;
        #[cfg(feature = "androidbox-dex-methods8")]
        let app_defined_call_count = ((updated.arg0() >> 32) & 0xff) as u8;
        #[cfg(feature = "androidbox-dex-instance9")]
        let app_defined_instance_call_count = ((updated.arg0() >> 40) & 0xff) as u8;
        #[cfg(feature = "androidbox-activity-fields10")]
        let activity_field_byte = ((updated.arg0() >> 48) & 0xff) as u8;
        #[cfg(feature = "androidbox-string-text12")]
        let direct_string_text = activity_field_byte & 0x80 != 0;
        #[cfg(feature = "androidbox-string-builder13")]
        let dynamic_string_text = activity_field_byte & 0x40 != 0;
        #[cfg(feature = "androidbox-string-builder13")]
        let activity_field_read_count = activity_field_byte & 0x3f;
        #[cfg(all(
            feature = "androidbox-string-text12",
            not(feature = "androidbox-string-builder13")
        ))]
        let activity_field_read_count = activity_field_byte & 0x7f;
        #[cfg(all(
            feature = "androidbox-activity-fields10",
            not(feature = "androidbox-string-text12")
        ))]
        let activity_field_read_count = activity_field_byte;
        #[cfg(feature = "androidbox-activity-state11")]
        let activity_int_state_value = (updated.arg0() >> 56) as u8;
        let revision = (updated.arg1() >> 32) as u32;
        let text_len = updated.arg1() as u32 as usize;
        let Some(expected_revision) = self.remote_revision.checked_add(1) else {
            self.protocol_fail();
        };
        #[cfg(not(feature = "androidbox-scene-rpc2"))]
        let label_matches = label_id == self.label_id;
        #[cfg(feature = "androidbox-scene-rpc2")]
        let label_matches = {
            let is_text_view = self.scene_nodes[..usize::from(self.scene_node_count)]
                .iter()
                .any(|node| node.id() == label_id && node.kind() == AndroidSceneViewKind::TextView);
            #[cfg(not(feature = "androidbox-multiaction3"))]
            {
                is_text_view && (self.label_id == 0 || self.label_id == label_id)
            }
            #[cfg(feature = "androidbox-multiaction3")]
            {
                is_text_view
            }
        };
        if !label_matches
            || revision != expected_revision
            || !(1..=ANDROID_APP_LABEL_MAX_BYTES).contains(&text_len)
        {
            self.protocol_fail();
        }
        let label_text =
            self.read_text(request_id, AndroidAppMessageKind::UpdateTextChunk, text_len);
        self.label_id = label_id;
        self.remote_revision = revision;
        Ok(AndroidAppRemoteUpdate {
            label_id,
            label_text,
            revision,
            #[cfg(feature = "androidbox-dex-methods8")]
            app_defined_call_count,
            #[cfg(feature = "androidbox-dex-instance9")]
            app_defined_instance_call_count,
            #[cfg(feature = "androidbox-activity-fields10")]
            activity_field_read_count,
            #[cfg(feature = "androidbox-string-text12")]
            direct_string_text,
            #[cfg(feature = "androidbox-string-builder13")]
            dynamic_string_text,
            #[cfg(feature = "androidbox-activity-state11")]
            activity_int_state_value,
        })
    }

    pub(super) fn close(&mut self) -> Result<(), AndroidAppClientError> {
        if self.active_session_id == 0 || self.active_generation == 0 {
            return Err(AndroidAppClientError::InvalidState);
        }
        let request_id = self.take_request_id();
        self.write(
            AndroidAppMessage::new(
                AndroidAppMessageKind::Close,
                request_id,
                self.active_session_id,
                self.active_generation,
                &[],
            )
            .map_err(|_| AndroidAppClientError::InvalidState)?,
        );
        let closed = self.read_for_request(request_id)?;
        self.clear_active();
        if closed.kind() == AndroidAppMessageKind::Closed {
            Ok(())
        } else if closed.kind() == AndroidAppMessageKind::Error {
            if closed.arg0() != u64::from(ERROR_CLOSE_REJECTED) {
                self.protocol_fail();
            }
            Err(AndroidAppClientError::RemoteRejected)
        } else {
            self.protocol_fail()
        }
    }

    #[cfg(feature = "androidbox-restart0")]
    fn controlled_crash_and_reopen(
        &mut self,
        identity: UiCompatibleActivityIdentity,
    ) -> Result<AndroidAppRemoteSnapshot, AndroidAppClientError> {
        if self.restart_epoch != 0
            || self.active_session_id != identity.session_id()
            || self.active_generation != identity.package_generation()
        {
            self.protocol_fail();
        }
        let old_pid = self.worker_pid;
        let request_id = self.take_request_id();
        self.write(
            AndroidAppMessage::new(
                AndroidAppMessageKind::Crash,
                request_id,
                self.active_session_id,
                self.active_generation,
                &[],
            )
            .map_err(|_| AndroidAppClientError::InvalidState)?,
        );

        let peer_closed = object_wait(self.channel.raw(), ObjectSignals::PEER_CLOSED);
        if peer_closed.status != Status::Ok.raw()
            || peer_closed.out2 != 0
            || peer_closed.out1 & u64::from(ObjectSignals::PEER_CLOSED.bits()) == 0
        {
            self.protocol_fail();
        }
        let drained = syscall(SyscallNumber::ChannelRead, self.channel.raw(), 0, 0);
        if drained.status != Status::PeerClosed.raw() || drained.out1 != 0 || drained.out2 != 0 {
            self.protocol_fail();
        }
        let closed = syscall(SyscallNumber::HandleClose, self.channel.raw(), 0, 0);
        if closed.status != Status::Ok.raw() || closed.out1 != 0 || closed.out2 != 0 {
            self.protocol_fail();
        }

        let (sender_pid, rebind, replacement) =
            read_android_app_supervisor_transfer(self.supervisor.raw());
        if sender_pid != self.init_pid
            || rebind.kind() != AndroidAppSupervisorMessageKind::Rebind
            || rebind.epoch() != 1
            || rebind.old_pid() != old_pid
            || rebind.new_pid() == old_pid
            || rebind.exit_code() != 0
            || rebind.termination_reason() != ProcessTerminationReason::Faulted.raw()
        {
            self.protocol_fail();
        }
        let new_pid = read_worker_ready(replacement.raw(), Some(rebind.new_pid()));
        self.channel = replacement;
        self.worker_pid = new_pid;
        self.next_request_id = 1;
        self.clear_active();
        self.restart_epoch = rebind.epoch();

        let rebound = AndroidAppSupervisorMessage::new(
            AndroidAppSupervisorMessageKind::Rebound,
            rebind.epoch(),
            rebind.old_pid(),
            rebind.new_pid(),
            rebind.exit_code(),
            rebind.termination_reason(),
        )
        .unwrap_or_else(|| self.protocol_fail());
        write_android_app_supervisor_bytes(self.supervisor.raw(), rebound);

        // The old Open completed before the controlled crash, so no
        // ambiguous request is replayed. ABI 48 starts a fresh worker session
        // with request ID 1 and re-verifies/re-claims the persisted APK.
        self.open(identity)
    }

    fn take_request_id(&mut self) -> u64 {
        let request_id = self.next_request_id;
        self.next_request_id = request_id
            .checked_add(1)
            .unwrap_or_else(|| self.protocol_fail());
        request_id
    }

    fn write(&self, message: AndroidAppMessage) {
        write_message(self.channel.raw(), message);
    }

    fn read_for_request(
        &mut self,
        request_id: u64,
    ) -> Result<AndroidAppMessage, AndroidAppClientError> {
        let (sender_pid, message) = read_message(self.channel.raw());
        if sender_pid != self.worker_pid || message.request_id() != request_id {
            self.protocol_fail();
        }
        Ok(message)
    }

    fn read_text(
        &mut self,
        request_id: u64,
        expected_kind: AndroidAppMessageKind,
        total: usize,
    ) -> InstalledInteractiveAscii {
        let maximum = match expected_kind {
            AndroidAppMessageKind::LabelChunk | AndroidAppMessageKind::UpdateTextChunk => {
                ANDROID_APP_LABEL_MAX_BYTES
            }
            AndroidAppMessageKind::ButtonChunk => ANDROID_APP_BUTTON_MAX_BYTES,
            #[cfg(feature = "androidbox-restart0")]
            AndroidAppMessageKind::Crash => self.protocol_fail(),
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::SceneOpened
            | AndroidAppMessageKind::DescribeNode
            | AndroidAppMessageKind::Node
            | AndroidAppMessageKind::NodeTextChunk => self.protocol_fail(),
            AndroidAppMessageKind::Ready
            | AndroidAppMessageKind::Open
            | AndroidAppMessageKind::Opened
            | AndroidAppMessageKind::Click
            | AndroidAppMessageKind::Updated
            | AndroidAppMessageKind::Close
            | AndroidAppMessageKind::Closed
            | AndroidAppMessageKind::Error => self.protocol_fail(),
        };
        if !(1..=maximum).contains(&total)
            || text_chunk_count(total) > ANDROID_APP_CHANNEL_QUEUE_CAPACITY
        {
            self.protocol_fail();
        }
        let mut bytes = [0; ANDROID_APP_LABEL_MAX_BYTES];
        let mut offset = 0usize;
        while offset < total {
            let message = self
                .read_for_request(request_id)
                .unwrap_or_else(|_| self.protocol_fail());
            let expected_payload_len =
                ANDROID_APP_MESSAGE_PAYLOAD_BYTES.min(total.saturating_sub(offset));
            if message.kind() != expected_kind
                || message.chunk_offset() as usize != offset
                || message.chunk_total() as usize != total
                || message.arg1() != 0
                || message.payload().len() != expected_payload_len
            {
                self.protocol_fail();
            }
            let end = offset
                .checked_add(message.payload().len())
                .filter(|end| *end <= total)
                .unwrap_or_else(|| self.protocol_fail());
            bytes[offset..end].copy_from_slice(message.payload());
            offset = end;
        }
        InstalledInteractiveAscii::from_ascii(&bytes[..total])
            .unwrap_or_else(|| self.protocol_fail())
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    fn read_scene_text(
        &mut self,
        request_id: u64,
        index: u8,
        total: usize,
    ) -> InstalledInteractiveAscii {
        if !(1..=ANDROID_APP_SCENE_NODE_TEXT_MAX_BYTES).contains(&total)
            || text_chunk_count(total) >= ANDROID_APP_CHANNEL_QUEUE_CAPACITY
        {
            self.protocol_fail();
        }
        let mut bytes = [0; ANDROID_APP_LABEL_MAX_BYTES];
        let mut offset = 0usize;
        while offset < total {
            let message = self
                .read_for_request(request_id)
                .unwrap_or_else(|_| self.protocol_fail());
            let expected_payload_len =
                ANDROID_APP_MESSAGE_PAYLOAD_BYTES.min(total.saturating_sub(offset));
            if message.kind() != AndroidAppMessageKind::NodeTextChunk
                || message.chunk_offset() as usize != offset
                || message.chunk_total() as usize != total
                || message.arg1() != u64::from(index)
                || message.payload().len() != expected_payload_len
            {
                self.protocol_fail();
            }
            let end = offset
                .checked_add(message.payload().len())
                .filter(|end| *end <= total)
                .unwrap_or_else(|| self.protocol_fail());
            bytes[offset..end].copy_from_slice(message.payload());
            offset = end;
        }
        InstalledInteractiveAscii::from_ascii(&bytes[..total])
            .unwrap_or_else(|| self.protocol_fail())
    }

    #[cfg(not(feature = "androidbox-scene-rpc2"))]
    fn clear_active(&mut self) {
        self.active_session_id = 0;
        self.active_generation = 0;
        self.label_id = 0;
        self.button_id = 0;
        self.remote_revision = 0;
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    fn clear_active(&mut self) {
        self.active_session_id = 0;
        self.active_generation = 0;
        self.label_id = 0;
        self.button_id = 0;
        self.remote_revision = 0;
        self.scene_nodes =
            [AndroidInstalledActivitySceneNode::empty(); ANDROID_APP_SCENE_MAX_NODES];
        self.scene_node_count = 0;
    }

    fn protocol_fail(&mut self) -> ! {
        self.clear_active();
        super::fail(FAIL_SURFACE_PROTOCOL)
    }
}

/// Runs the independent compatibility worker forever on its sole narrowed
/// App RPC endpoint.
pub(super) fn worker_loop(startup: u64) -> ! {
    prove_worker_authority_is_narrow();
    let channel =
        OwnedUserHandle::new(startup).unwrap_or_else(|| super::fail(FAIL_SURFACE_PROTOCOL));
    write_message(
        channel.raw(),
        AndroidAppMessage::new(AndroidAppMessageKind::Ready, 0, ABI_VERSION, 0, &[])
            .unwrap_or_else(|_| super::fail(FAIL_SURFACE_PROTOCOL)),
    );

    let mut peer_pid = 0u64;
    let mut last_request_id = 0u64;
    let mut active_session_id = 0u64;
    let mut active_generation = 0u64;
    let mut active_revision = 0u32;
    let mut active_label_id = 0u32;
    let mut active_button_id = 0u32;
    #[cfg(feature = "androidbox-scene-rpc2")]
    let mut active_scene_snapshot: Option<InstalledInteractiveActivitySnapshot> = None;
    let mut lease = InstalledInteractiveActivityLease::new();

    loop {
        let (sender_pid, message) = read_message(channel.raw());
        let expected_request_id = last_request_id.checked_add(1);
        if sender_pid == 0
            || (peer_pid != 0 && sender_pid != peer_pid)
            || expected_request_id != Some(message.request_id())
        {
            reset_worker_lease(
                &mut lease,
                &mut active_session_id,
                &mut active_generation,
                &mut active_revision,
                &mut active_label_id,
                &mut active_button_id,
            );
            super::fail(FAIL_SURFACE_PROTOCOL);
        }
        peer_pid = sender_pid;
        last_request_id = message.request_id();

        match message.kind() {
            AndroidAppMessageKind::Open => {
                if !worker_activity_is_inactive(
                    active_session_id,
                    active_generation,
                    active_revision,
                    active_label_id,
                    active_button_id,
                ) {
                    reset_worker_lease(
                        &mut lease,
                        &mut active_session_id,
                        &mut active_generation,
                        &mut active_revision,
                        &mut active_label_id,
                        &mut active_button_id,
                    );
                    write_error(channel.raw(), message.request_id(), ERROR_BUSY);
                    continue;
                }
                let identity = UiCompatibleActivityIdentity::new(message.arg0(), message.arg1())
                    .unwrap_or_else(|| super::fail(FAIL_SURFACE_PROTOCOL));
                let expected = read_android_installed_app_status();
                if !expected.installed || expected.generation != identity.package_generation() {
                    reset_worker_lease(
                        &mut lease,
                        &mut active_session_id,
                        &mut active_generation,
                        &mut active_revision,
                        &mut active_label_id,
                        &mut active_button_id,
                    );
                    write_error(channel.raw(), message.request_id(), ERROR_OPEN_REJECTED);
                    continue;
                }
                let Some(snapshot) = claim_and_open(identity, expected, &mut lease) else {
                    reset_worker_lease(
                        &mut lease,
                        &mut active_session_id,
                        &mut active_generation,
                        &mut active_revision,
                        &mut active_label_id,
                        &mut active_button_id,
                    );
                    write_error(channel.raw(), message.request_id(), ERROR_OPEN_REJECTED);
                    continue;
                };
                if !snapshot_is_wire_canonical(&snapshot) {
                    reset_worker_lease(
                        &mut lease,
                        &mut active_session_id,
                        &mut active_generation,
                        &mut active_revision,
                        &mut active_label_id,
                        &mut active_button_id,
                    );
                    write_error(channel.raw(), message.request_id(), ERROR_OPEN_REJECTED);
                    continue;
                }
                active_session_id = identity.session_id();
                active_generation = identity.package_generation();
                active_revision = snapshot.revision;
                active_button_id = snapshot.button_id;
                #[cfg(not(feature = "androidbox-scene-rpc2"))]
                {
                    active_label_id = snapshot.label_id;
                    write_opened(channel.raw(), message.request_id(), snapshot);
                }
                #[cfg(feature = "androidbox-scene-rpc2")]
                {
                    active_label_id = snapshot.callback_target_id;
                    active_scene_snapshot = Some(snapshot);
                    write_scene_opened(channel.raw(), message.request_id(), snapshot);
                }
            }
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::DescribeNode => {
                let expected_revision = (message.arg1() >> 32) as u32;
                let index = message.arg1() as u32 as usize;
                let current = read_android_installed_app_status();
                let Some(snapshot) = active_scene_snapshot else {
                    super::fail(FAIL_SURFACE_PROTOCOL);
                };
                if active_session_id == 0
                    || active_generation == 0
                    || message.arg0() != active_session_id
                    || expected_revision != active_revision
                    || index >= usize::from(snapshot.scene_node_count)
                    || !current.installed
                    || current.generation != active_generation
                {
                    reset_worker_lease(
                        &mut lease,
                        &mut active_session_id,
                        &mut active_generation,
                        &mut active_revision,
                        &mut active_label_id,
                        &mut active_button_id,
                    );
                    write_error(channel.raw(), message.request_id(), ERROR_OPEN_REJECTED);
                    continue;
                }
                write_scene_node(
                    channel.raw(),
                    message.request_id(),
                    u8::try_from(index).unwrap_or_else(|_| super::fail(FAIL_SURFACE_PROTOCOL)),
                    snapshot.scene_nodes[index],
                    active_revision,
                );
            }
            AndroidAppMessageKind::Click => {
                let view_id = (message.arg1() >> 32) as u32;
                let expected_revision = message.arg1() as u32;
                let current = read_android_installed_app_status();
                #[cfg(not(feature = "androidbox-multiaction3"))]
                let callback_matches = view_id == active_button_id;
                #[cfg(feature = "androidbox-multiaction3")]
                let callback_matches = active_scene_snapshot.is_some_and(|snapshot| {
                    snapshot.scene_nodes[..usize::from(snapshot.scene_node_count)]
                        .iter()
                        .any(|node| {
                            node.id == view_id
                                && node.kind == bndr_androidbox::ViewKind::Button
                                && node.callback_registered
                        })
                });
                if active_session_id == 0
                    || active_generation == 0
                    || message.arg0() != active_session_id
                    || expected_revision != active_revision
                    || view_id == 0
                    || !callback_matches
                    || active_label_id == 0
                    || !current.installed
                    || current.generation != active_generation
                {
                    reset_worker_lease(
                        &mut lease,
                        &mut active_session_id,
                        &mut active_generation,
                        &mut active_revision,
                        &mut active_label_id,
                        &mut active_button_id,
                    );
                    write_error(channel.raw(), message.request_id(), ERROR_CLICK_REJECTED);
                    continue;
                }
                let Some(next_revision) = active_revision.checked_add(1) else {
                    reset_worker_lease(
                        &mut lease,
                        &mut active_session_id,
                        &mut active_generation,
                        &mut active_revision,
                        &mut active_label_id,
                        &mut active_button_id,
                    );
                    write_error(channel.raw(), message.request_id(), ERROR_CLICK_REJECTED);
                    continue;
                };
                match lease.dispatch_button_click(view_id) {
                    Ok(update) => {
                        #[cfg(not(feature = "androidbox-multiaction3"))]
                        let update_canonical =
                            update_is_wire_canonical(&update, active_label_id, next_revision);
                        #[cfg(feature = "androidbox-multiaction3")]
                        let update_canonical = active_scene_snapshot.is_some_and(|snapshot| {
                            update_is_wire_canonical_for_scene(&update, &snapshot, next_revision)
                        });
                        if !update_canonical {
                            reset_worker_lease(
                                &mut lease,
                                &mut active_session_id,
                                &mut active_generation,
                                &mut active_revision,
                                &mut active_label_id,
                                &mut active_button_id,
                            );
                            write_error(channel.raw(), message.request_id(), ERROR_CLICK_REJECTED);
                            continue;
                        }
                        write_updated(channel.raw(), message.request_id(), update);
                        active_revision = next_revision;
                    }
                    Err(_) => {
                        reset_worker_lease(
                            &mut lease,
                            &mut active_session_id,
                            &mut active_generation,
                            &mut active_revision,
                            &mut active_label_id,
                            &mut active_button_id,
                        );
                        write_error(channel.raw(), message.request_id(), ERROR_CLICK_REJECTED);
                    }
                }
            }
            AndroidAppMessageKind::Close => {
                if active_session_id == 0
                    || active_generation == 0
                    || message.arg0() != active_session_id
                    || message.arg1() != active_generation
                {
                    reset_worker_lease(
                        &mut lease,
                        &mut active_session_id,
                        &mut active_generation,
                        &mut active_revision,
                        &mut active_label_id,
                        &mut active_button_id,
                    );
                    write_error(channel.raw(), message.request_id(), ERROR_CLOSE_REJECTED);
                    continue;
                }
                if lease.close().is_err() {
                    reset_worker_lease(
                        &mut lease,
                        &mut active_session_id,
                        &mut active_generation,
                        &mut active_revision,
                        &mut active_label_id,
                        &mut active_button_id,
                    );
                    write_error(channel.raw(), message.request_id(), ERROR_CLOSE_REJECTED);
                    continue;
                }
                clear_worker_activity(
                    &mut active_session_id,
                    &mut active_generation,
                    &mut active_revision,
                    &mut active_label_id,
                    &mut active_button_id,
                );
                write_message(
                    channel.raw(),
                    AndroidAppMessage::new(
                        AndroidAppMessageKind::Closed,
                        message.request_id(),
                        0,
                        0,
                        &[],
                    )
                    .unwrap_or_else(|_| super::fail(FAIL_SURFACE_PROTOCOL)),
                );
            }
            #[cfg(feature = "androidbox-restart0")]
            AndroidAppMessageKind::Crash => {
                if active_session_id == 0
                    || active_generation == 0
                    || message.arg0() != active_session_id
                    || message.arg1() != active_generation
                {
                    reset_worker_lease(
                        &mut lease,
                        &mut active_session_id,
                        &mut active_generation,
                        &mut active_revision,
                        &mut active_label_id,
                        &mut active_button_id,
                    );
                    super::fail(FAIL_SURFACE_PROTOCOL);
                }
                reset_worker_lease(
                    &mut lease,
                    &mut active_session_id,
                    &mut active_generation,
                    &mut active_revision,
                    &mut active_label_id,
                    &mut active_button_id,
                );
                // SAFETY: ABI 48 deliberately performs one volatile write to
                // the image's verified-unmapped lower stack guard. The kernel
                // must contain this lower-EL data abort and report
                // Faulted/exit-code 0 to Init's exact ProcessWait.
                unsafe {
                    core::ptr::write_volatile(super::USER_GUARD_LOW as *mut u8, 0xa5);
                }
                super::fail(FAIL_SURFACE_PROTOCOL)
            }
            AndroidAppMessageKind::Ready
            | AndroidAppMessageKind::Opened
            | AndroidAppMessageKind::LabelChunk
            | AndroidAppMessageKind::ButtonChunk
            | AndroidAppMessageKind::Updated
            | AndroidAppMessageKind::UpdateTextChunk
            | AndroidAppMessageKind::Closed
            | AndroidAppMessageKind::Error => {
                reset_worker_lease(
                    &mut lease,
                    &mut active_session_id,
                    &mut active_generation,
                    &mut active_revision,
                    &mut active_label_id,
                    &mut active_button_id,
                );
                super::fail(FAIL_SURFACE_PROTOCOL)
            }
            #[cfg(feature = "androidbox-scene-rpc2")]
            AndroidAppMessageKind::SceneOpened
            | AndroidAppMessageKind::Node
            | AndroidAppMessageKind::NodeTextChunk => {
                reset_worker_lease(
                    &mut lease,
                    &mut active_session_id,
                    &mut active_generation,
                    &mut active_revision,
                    &mut active_label_id,
                    &mut active_button_id,
                );
                super::fail(FAIL_SURFACE_PROTOCOL)
            }
        }
    }
}

fn prove_worker_authority_is_narrow() {
    let forbidden = [
        syscall(SyscallNumber::ChannelCreate, 0, 0, 0),
        syscall(SyscallNumber::HandleDuplicate, 1, u64::MAX, 0),
        syscall(SyscallNumber::ChannelWriteTransfer, 1, 0, 0),
        syscall(SyscallNumber::ProcessSpawn, 1, 1, 0),
        syscall(SyscallNumber::ProcessTerminate, 1, 0, 0),
        syscall(SyscallNumber::SurfaceAcquire, 0, 0, 0),
        syscall(SyscallNumber::SurfacePresent, 1, 0, 0),
        syscall(SyscallNumber::SurfaceReadInput, 1, 0, 0),
        syscall(
            SyscallNumber::GraphicsBufferCreate,
            GRAPHICS_BUFFER_FORMAT_XRGB8888,
            pack_graphics_buffer_geometry(GRAPHICS_BUFFER_WIDTH, GRAPHICS_BUFFER_HEIGHT),
            GRAPHICS_BUFFER_CREATE_FLAGS_NONE,
        ),
        syscall(SyscallNumber::GraphicsBufferWrite, 1, 0, 0),
        syscall(SyscallNumber::SurfacePresentBuffer, 1, 1, 0),
        syscall(SyscallNumber::GraphicsBufferMap, 1, 0, 0),
        syscall(SyscallNumber::GraphicsBufferQueue, 1, 0, 0),
        syscall(SyscallNumber::InputAcquire, 0, 0, 0),
        syscall(SyscallNumber::InputReadEvent, 1, 0, 0),
        syscall(SyscallNumber::InputSessionInfo, 1, 0, 0),
        syscall(SyscallNumber::AppDataRootOpen, 0, 0, 0),
        syscall(SyscallNumber::StorageAcquire, 0, 0, 0),
        syscall(SyscallNumber::StorageConnect, 1, 0, 0),
        syscall(SyscallNumber::SystemClockRead, 0, 0, 0),
        syscall(SyscallNumber::AndroidPackageRelaunch, 1, 1, 1),
        #[cfg(feature = "androidbox-icon-resources5")]
        syscall(SyscallNumber::AndroidPackageIconRead, 1, 1, 0),
    ];
    if forbidden.iter().any(|result| {
        result.status != Status::PermissionDenied.raw() || result.out1 != 0 || result.out2 != 0
    }) {
        super::fail(FAIL_SURFACE_PROTOCOL);
    }
}

fn claim_and_open(
    identity: UiCompatibleActivityIdentity,
    expected: AndroidInstalledAppStatus,
    lease: &mut InstalledInteractiveActivityLease,
) -> Option<InstalledInteractiveActivitySnapshot> {
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
        if destination.len() > VMO_READ_MAX_BYTES {
            return Err(());
        }
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

fn wire_text_is_canonical(text: &InstalledInteractiveAscii, maximum: usize) -> bool {
    let bytes = text.as_bytes();
    (1..=maximum).contains(&bytes.len()) && bytes.iter().all(|byte| matches!(*byte, 0x20..=0x7e))
}

#[cfg(not(feature = "androidbox-scene-rpc2"))]
fn snapshot_is_wire_canonical(snapshot: &InstalledInteractiveActivitySnapshot) -> bool {
    snapshot.label_id != 0
        && snapshot.button_id != 0
        && snapshot.label_id != snapshot.button_id
        && snapshot.revision == 0
        && wire_text_is_canonical(&snapshot.label_text, ANDROID_APP_LABEL_MAX_BYTES)
        && wire_text_is_canonical(&snapshot.button_text, ANDROID_APP_BUTTON_MAX_BYTES)
        && 1 + text_chunk_count(snapshot.label_text.as_bytes().len())
            + text_chunk_count(snapshot.button_text.as_bytes().len())
            <= ANDROID_APP_CHANNEL_QUEUE_CAPACITY
}

#[cfg(feature = "androidbox-scene-rpc2")]
fn snapshot_is_wire_canonical(snapshot: &InstalledInteractiveActivitySnapshot) -> bool {
    let count = usize::from(snapshot.scene_node_count);
    let listener_count = usize::from(snapshot.execution.listener_button_count);
    #[cfg(not(feature = "androidbox-multiaction3"))]
    let listener_count_is_valid = listener_count == 1;
    #[cfg(feature = "androidbox-multiaction3")]
    let listener_count_is_valid =
        (1..=bndr_androidbox::MAX_INTERACTIVE_ACTIVITY_CALLBACKS).contains(&listener_count);
    let listener_ids = match snapshot.execution.listener_button_ids.get(..listener_count) {
        Some(ids) => ids,
        None => return false,
    };
    if snapshot.revision != 0
        || !(1..=ANDROID_APP_SCENE_MAX_NODES).contains(&count)
        || !listener_count_is_valid
        || snapshot.label_id == 0
        || snapshot.button_id == 0
        || snapshot.callback_target_id == 0
        || snapshot.button_id != snapshot.execution.listener_button_id
        || listener_ids.first().copied() != Some(snapshot.button_id)
        || listener_ids
            .iter()
            .any(|id| *id == 0 || listener_ids.iter().filter(|other| **other == *id).count() != 1)
        || !wire_text_is_canonical(&snapshot.label_text, ANDROID_APP_LABEL_MAX_BYTES)
        || !wire_text_is_canonical(&snapshot.button_text, ANDROID_APP_BUTTON_MAX_BYTES)
    {
        return false;
    }
    let nodes = &snapshot.scene_nodes[..count];
    let mut first_label_seen = false;
    let mut callback_count = 0u8;
    let mut callback_target_seen = false;
    for (index, node) in nodes.iter().copied().enumerate() {
        let Ok(index_u8) = u8::try_from(index) else {
            return false;
        };
        let Some(descriptor) = scene_node_descriptor(node) else {
            return false;
        };
        if !descriptor.is_canonical_for_index(index_u8)
            || (index != 0
                && !node.parent.is_some_and(|parent| {
                    usize::from(parent) < index
                        && nodes[usize::from(parent)].kind
                            == bndr_androidbox::ViewKind::LinearLayout
                }))
            || (node.id != 0 && nodes[..index].iter().any(|earlier| earlier.id == node.id))
        {
            return false;
        }
        match node.kind {
            bndr_androidbox::ViewKind::LinearLayout => {
                if !node.text.is_empty() {
                    return false;
                }
            }
            bndr_androidbox::ViewKind::TextView => {
                if !wire_text_is_canonical(&node.text, ANDROID_APP_SCENE_NODE_TEXT_MAX_BYTES) {
                    return false;
                }
                if !first_label_seen {
                    if node.id != snapshot.label_id || node.text != snapshot.label_text {
                        return false;
                    }
                    first_label_seen = true;
                }
                if node.id == snapshot.callback_target_id {
                    callback_target_seen = true;
                }
            }
            bndr_androidbox::ViewKind::Button => {
                if !wire_text_is_canonical(&node.text, ANDROID_APP_BUTTON_MAX_BYTES) {
                    return false;
                }
                let expected_callback = listener_ids.contains(&node.id);
                if node.callback_registered != expected_callback {
                    return false;
                }
                if node.callback_registered {
                    callback_count = match callback_count.checked_add(1) {
                        Some(value) => value,
                        None => return false,
                    };
                    if callback_count == 1
                        && (node.id != snapshot.button_id || node.text != snapshot.button_text)
                    {
                        return false;
                    }
                }
            }
        }
    }
    first_label_seen && usize::from(callback_count) == listener_count && callback_target_seen
}

#[cfg(feature = "androidbox-string-text12")]
fn string_text_execution_is_wire_canonical(execution: &bndr_androidbox::ActivityUpdate) -> bool {
    let source_valid = if execution.direct_string_text {
        execution.text_resource_id == 0
    } else {
        execution.text_resource_id != 0
    };
    let state_shape_valid = if execution.activity_int_state_value == 0 {
        !execution.direct_string_text
            && execution.activity_field_read_count <= 1
            && execution.activity_field_write_count == 0
    } else if execution.direct_string_text {
        execution.app_defined_call_count == 0
            && execution.app_defined_instance_call_count == 0
            && execution.activity_field_read_count == 2
            && execution.activity_field_write_count == 1
    } else {
        execution.app_defined_call_count == 1
            && execution.app_defined_instance_call_count == 1
            && execution.activity_field_read_count == 2
            && execution.activity_field_write_count == 1
    };
    #[cfg(feature = "androidbox-string-builder13")]
    let dynamic_shape_valid = !execution.dynamic_string_text || execution.direct_string_text;
    #[cfg(not(feature = "androidbox-string-builder13"))]
    let dynamic_shape_valid = true;
    source_valid
        && execution.app_defined_call_count <= 1
        && execution.app_defined_instance_call_count <= execution.app_defined_call_count
        && execution.activity_field_read_count <= 2
        && execution.activity_field_write_count <= 1
        && dynamic_shape_valid
        && state_shape_valid
}

#[cfg(not(feature = "androidbox-multiaction3"))]
fn update_is_wire_canonical(
    update: &InstalledInteractiveActivityUpdate,
    expected_label_id: u32,
    expected_revision: u32,
) -> bool {
    #[cfg(feature = "androidbox-string-text12")]
    let execution_valid = string_text_execution_is_wire_canonical(&update.execution);
    #[cfg(all(
        feature = "androidbox-activity-state11",
        not(feature = "androidbox-string-text12")
    ))]
    let execution_valid = update.execution.app_defined_call_count <= 1
        && update.execution.app_defined_instance_call_count
            <= update.execution.app_defined_call_count
        && update.execution.activity_field_read_count <= 2
        && update.execution.activity_field_write_count <= 1
        && if update.execution.activity_int_state_value == 0 {
            update.execution.activity_field_read_count <= 1
                && update.execution.activity_field_write_count == 0
        } else {
            update.execution.app_defined_call_count == 1
                && update.execution.app_defined_instance_call_count == 1
                && update.execution.activity_field_read_count == 2
                && update.execution.activity_field_write_count == 1
        };
    #[cfg(all(
        feature = "androidbox-activity-fields10",
        not(feature = "androidbox-activity-state11")
    ))]
    let execution_valid = update.execution.app_defined_call_count <= 1
        && update.execution.app_defined_instance_call_count
            <= update.execution.app_defined_call_count
        && update.execution.activity_field_read_count <= 1;
    #[cfg(all(
        feature = "androidbox-dex-instance9",
        not(feature = "androidbox-activity-fields10")
    ))]
    let execution_valid = update.execution.app_defined_call_count <= 1
        && update.execution.app_defined_instance_call_count
            <= update.execution.app_defined_call_count;
    #[cfg(all(
        feature = "androidbox-dex-methods8",
        not(feature = "androidbox-dex-instance9")
    ))]
    let execution_valid = update.execution.app_defined_call_count <= 1;
    #[cfg(not(feature = "androidbox-dex-methods8"))]
    let execution_valid = true;
    expected_label_id != 0
        && expected_revision != 0
        && update.label_id == expected_label_id
        && update.revision == expected_revision
        && wire_text_is_canonical(&update.label_text, ANDROID_APP_LABEL_MAX_BYTES)
        && text_chunk_count(update.label_text.as_bytes().len()) < ANDROID_APP_CHANNEL_QUEUE_CAPACITY
        && execution_valid
}

#[cfg(feature = "androidbox-multiaction3")]
fn update_is_wire_canonical_for_scene(
    update: &InstalledInteractiveActivityUpdate,
    snapshot: &InstalledInteractiveActivitySnapshot,
    expected_revision: u32,
) -> bool {
    let count = usize::from(snapshot.scene_node_count);
    #[cfg(feature = "androidbox-string-text12")]
    let execution_valid = string_text_execution_is_wire_canonical(&update.execution);
    #[cfg(all(
        feature = "androidbox-activity-state11",
        not(feature = "androidbox-string-text12")
    ))]
    let execution_valid = update.execution.app_defined_call_count <= 1
        && update.execution.app_defined_instance_call_count
            <= update.execution.app_defined_call_count
        && update.execution.activity_field_read_count <= 2
        && update.execution.activity_field_write_count <= 1
        && if update.execution.activity_int_state_value == 0 {
            update.execution.activity_field_read_count <= 1
                && update.execution.activity_field_write_count == 0
        } else {
            update.execution.app_defined_call_count == 1
                && update.execution.app_defined_instance_call_count == 1
                && update.execution.activity_field_read_count == 2
                && update.execution.activity_field_write_count == 1
        };
    #[cfg(all(
        feature = "androidbox-activity-fields10",
        not(feature = "androidbox-activity-state11")
    ))]
    let execution_valid = update.execution.app_defined_call_count <= 1
        && update.execution.app_defined_instance_call_count
            <= update.execution.app_defined_call_count
        && update.execution.activity_field_read_count <= 1;
    #[cfg(all(
        feature = "androidbox-dex-instance9",
        not(feature = "androidbox-activity-fields10")
    ))]
    let execution_valid = update.execution.app_defined_call_count <= 1
        && update.execution.app_defined_instance_call_count
            <= update.execution.app_defined_call_count;
    #[cfg(all(
        feature = "androidbox-dex-methods8",
        not(feature = "androidbox-dex-instance9")
    ))]
    let execution_valid = update.execution.app_defined_call_count <= 1;
    #[cfg(not(feature = "androidbox-dex-methods8"))]
    let execution_valid = true;
    expected_revision != 0
        && update.label_id != 0
        && update.revision == expected_revision
        && count <= snapshot.scene_nodes.len()
        && snapshot.scene_nodes[..count].iter().any(|node| {
            node.id == update.label_id && node.kind == bndr_androidbox::ViewKind::TextView
        })
        && wire_text_is_canonical(&update.label_text, ANDROID_APP_LABEL_MAX_BYTES)
        && text_chunk_count(update.label_text.as_bytes().len()) < ANDROID_APP_CHANNEL_QUEUE_CAPACITY
        && execution_valid
}

#[cfg(not(feature = "androidbox-scene-rpc2"))]
fn write_opened(channel: u64, request_id: u64, snapshot: InstalledInteractiveActivitySnapshot) {
    let label = snapshot.label_text.as_bytes();
    let button = snapshot.button_text.as_bytes();
    if !snapshot_is_wire_canonical(&snapshot) {
        super::fail(FAIL_SURFACE_PROTOCOL);
    }
    let arg0 = (u64::from(snapshot.label_id) << 32) | u64::from(snapshot.button_id);
    let arg1 =
        (u64::from(snapshot.revision) << 32) | ((label.len() as u64) << 16) | button.len() as u64;
    write_message(
        channel,
        AndroidAppMessage::new(AndroidAppMessageKind::Opened, request_id, arg0, arg1, &[])
            .unwrap_or_else(|_| super::fail(FAIL_SURFACE_PROTOCOL)),
    );
    write_text_chunks(
        channel,
        request_id,
        AndroidAppMessageKind::LabelChunk,
        label,
    );
    write_text_chunks(
        channel,
        request_id,
        AndroidAppMessageKind::ButtonChunk,
        button,
    );
    let _ = snapshot.execution;
}

#[cfg(feature = "androidbox-scene-rpc2")]
fn write_scene_opened(
    channel: u64,
    request_id: u64,
    snapshot: InstalledInteractiveActivitySnapshot,
) {
    if !snapshot_is_wire_canonical(&snapshot) {
        super::fail(FAIL_SURFACE_PROTOCOL);
    }
    write_message(
        channel,
        AndroidAppMessage::new(
            AndroidAppMessageKind::SceneOpened,
            request_id,
            u64::from(snapshot.scene_node_count),
            u64::from(snapshot.revision),
            &[],
        )
        .unwrap_or_else(|_| super::fail(FAIL_SURFACE_PROTOCOL)),
    );
    let _ = snapshot.execution;
}

#[cfg(feature = "androidbox-scene-rpc2")]
fn write_scene_node(
    channel: u64,
    request_id: u64,
    index: u8,
    node: InstalledInteractiveSceneNode,
    revision: u32,
) {
    let descriptor =
        scene_node_descriptor(node).unwrap_or_else(|| super::fail(FAIL_SURFACE_PROTOCOL));
    if !descriptor.is_canonical_for_index(index) {
        super::fail(FAIL_SURFACE_PROTOCOL);
    }
    write_message(
        channel,
        AndroidAppMessage::new(
            AndroidAppMessageKind::Node,
            request_id,
            u64::from(index),
            u64::from(revision),
            &descriptor.encode(),
        )
        .unwrap_or_else(|_| super::fail(FAIL_SURFACE_PROTOCOL)),
    );
    if !node.text.is_empty() {
        write_scene_text_chunks(channel, request_id, index, node.text.as_bytes());
    }
}

fn write_updated(channel: u64, request_id: u64, update: InstalledInteractiveActivityUpdate) {
    let text = update.label_text.as_bytes();
    #[cfg(feature = "androidbox-string-text12")]
    let app_defined_calls_valid = string_text_execution_is_wire_canonical(&update.execution);
    #[cfg(all(
        feature = "androidbox-activity-state11",
        not(feature = "androidbox-string-text12")
    ))]
    let app_defined_calls_valid = update.execution.app_defined_call_count <= 1
        && update.execution.app_defined_instance_call_count
            <= update.execution.app_defined_call_count
        && update.execution.activity_field_read_count <= 2
        && update.execution.activity_field_write_count <= 1
        && if update.execution.activity_int_state_value == 0 {
            update.execution.activity_field_read_count <= 1
                && update.execution.activity_field_write_count == 0
        } else {
            update.execution.app_defined_call_count == 1
                && update.execution.app_defined_instance_call_count == 1
                && update.execution.activity_field_read_count == 2
                && update.execution.activity_field_write_count == 1
        };
    #[cfg(all(
        feature = "androidbox-activity-fields10",
        not(feature = "androidbox-activity-state11")
    ))]
    let app_defined_calls_valid = update.execution.app_defined_call_count <= 1
        && update.execution.app_defined_instance_call_count
            <= update.execution.app_defined_call_count
        && update.execution.activity_field_read_count <= 1;
    #[cfg(all(
        feature = "androidbox-dex-instance9",
        not(feature = "androidbox-activity-fields10")
    ))]
    let app_defined_calls_valid = update.execution.app_defined_call_count <= 1
        && update.execution.app_defined_instance_call_count
            <= update.execution.app_defined_call_count;
    #[cfg(all(
        feature = "androidbox-dex-methods8",
        not(feature = "androidbox-dex-instance9")
    ))]
    let app_defined_calls_valid = update.execution.app_defined_call_count <= 1;
    #[cfg(not(feature = "androidbox-dex-methods8"))]
    let app_defined_calls_valid = true;
    if update.label_id == 0
        || update.revision == 0
        || !wire_text_is_canonical(&update.label_text, ANDROID_APP_LABEL_MAX_BYTES)
        || text_chunk_count(text.len()) >= ANDROID_APP_CHANNEL_QUEUE_CAPACITY
        || !app_defined_calls_valid
    {
        super::fail(FAIL_SURFACE_PROTOCOL);
    }
    #[cfg(feature = "androidbox-string-builder13")]
    let update_identity = (u64::from(update.execution.activity_int_state_value) << 56)
        | (u64::from(
            update.execution.activity_field_read_count
                | u8::from(update.execution.dynamic_string_text) << 6
                | u8::from(update.execution.direct_string_text) << 7,
        ) << 48)
        | (u64::from(update.execution.app_defined_instance_call_count) << 40)
        | (u64::from(update.execution.app_defined_call_count) << 32)
        | u64::from(update.label_id);
    #[cfg(all(
        feature = "androidbox-string-text12",
        not(feature = "androidbox-string-builder13")
    ))]
    let update_identity = (u64::from(update.execution.activity_int_state_value) << 56)
        | (u64::from(
            update.execution.activity_field_read_count
                | u8::from(update.execution.direct_string_text) << 7,
        ) << 48)
        | (u64::from(update.execution.app_defined_instance_call_count) << 40)
        | (u64::from(update.execution.app_defined_call_count) << 32)
        | u64::from(update.label_id);
    #[cfg(all(
        feature = "androidbox-activity-state11",
        not(feature = "androidbox-string-text12")
    ))]
    let update_identity = (u64::from(update.execution.activity_int_state_value) << 56)
        | (u64::from(update.execution.activity_field_read_count) << 48)
        | (u64::from(update.execution.app_defined_instance_call_count) << 40)
        | (u64::from(update.execution.app_defined_call_count) << 32)
        | u64::from(update.label_id);
    #[cfg(all(
        feature = "androidbox-activity-fields10",
        not(feature = "androidbox-activity-state11")
    ))]
    let update_identity = (u64::from(update.execution.activity_field_read_count) << 48)
        | (u64::from(update.execution.app_defined_instance_call_count) << 40)
        | (u64::from(update.execution.app_defined_call_count) << 32)
        | u64::from(update.label_id);
    #[cfg(all(
        feature = "androidbox-dex-instance9",
        not(feature = "androidbox-activity-fields10")
    ))]
    let update_identity = (u64::from(update.execution.app_defined_instance_call_count) << 40)
        | (u64::from(update.execution.app_defined_call_count) << 32)
        | u64::from(update.label_id);
    #[cfg(all(
        feature = "androidbox-dex-methods8",
        not(feature = "androidbox-dex-instance9")
    ))]
    let update_identity =
        (u64::from(update.execution.app_defined_call_count) << 32) | u64::from(update.label_id);
    #[cfg(not(feature = "androidbox-dex-methods8"))]
    let update_identity = u64::from(update.label_id);
    write_message(
        channel,
        AndroidAppMessage::new(
            AndroidAppMessageKind::Updated,
            request_id,
            update_identity,
            (u64::from(update.revision) << 32) | text.len() as u64,
            &[],
        )
        .unwrap_or_else(|_| super::fail(FAIL_SURFACE_PROTOCOL)),
    );
    write_text_chunks(
        channel,
        request_id,
        AndroidAppMessageKind::UpdateTextChunk,
        text,
    );
    #[cfg(not(feature = "androidbox-dex-methods8"))]
    let _ = update.execution;
}

#[cfg(feature = "androidbox-scene-rpc2")]
fn scene_node_descriptor(
    node: InstalledInteractiveSceneNode,
) -> Option<AndroidAppSceneNodeDescriptor> {
    let kind = match node.kind {
        bndr_androidbox::ViewKind::LinearLayout => AndroidAppSceneNodeKind::LinearLayout,
        bndr_androidbox::ViewKind::TextView => AndroidAppSceneNodeKind::TextView,
        bndr_androidbox::ViewKind::Button => AndroidAppSceneNodeKind::Button,
    };
    let width = match node.width {
        bndr_androidbox::LayoutSize::MatchParent => AndroidAppSceneDimension::MatchParent,
        bndr_androidbox::LayoutSize::WrapContent => AndroidAppSceneDimension::WrapContent,
        #[cfg(feature = "androidbox-layout-weight15")]
        bndr_androidbox::LayoutSize::Zero => AndroidAppSceneDimension::Zero,
        #[cfg(feature = "androidbox-layout-size18")]
        bndr_androidbox::LayoutSize::Exact => AndroidAppSceneDimension::Exact,
    };
    let height = match node.height {
        bndr_androidbox::LayoutSize::MatchParent => AndroidAppSceneDimension::MatchParent,
        bndr_androidbox::LayoutSize::WrapContent => AndroidAppSceneDimension::WrapContent,
        #[cfg(feature = "androidbox-layout-weight15")]
        bndr_androidbox::LayoutSize::Zero => AndroidAppSceneDimension::Zero,
        #[cfg(feature = "androidbox-layout-size18")]
        bndr_androidbox::LayoutSize::Exact => AndroidAppSceneDimension::Exact,
    };
    let orientation = match node.orientation {
        bndr_androidbox::LayoutOrientation::None => AndroidAppSceneOrientation::None,
        bndr_androidbox::LayoutOrientation::Vertical => AndroidAppSceneOrientation::Vertical,
        #[cfg(feature = "androidbox-layout-row14")]
        bndr_androidbox::LayoutOrientation::Horizontal => AndroidAppSceneOrientation::Horizontal,
    };
    #[cfg(feature = "androidbox-layout-size18")]
    let descriptor = AndroidAppSceneNodeDescriptor::new_sized(
        kind,
        node.parent,
        node.id,
        width,
        height,
        orientation,
        node.callback_registered,
        u8::try_from(node.text.as_bytes().len()).ok()?,
        node.layout_weight,
        node.layout_margin_left_dp,
        node.layout_margin_top_dp,
        node.layout_margin_right_dp,
        node.layout_margin_bottom_dp,
        node.padding_left_dp,
        node.padding_top_dp,
        node.padding_right_dp,
        node.padding_bottom_dp,
        node.exact_width_dp,
        node.exact_height_dp,
    );
    #[cfg(all(
        feature = "androidbox-layout-directional17",
        not(feature = "androidbox-layout-size18")
    ))]
    let descriptor = AndroidAppSceneNodeDescriptor::new_directional(
        kind,
        node.parent,
        node.id,
        width,
        height,
        orientation,
        node.callback_registered,
        u8::try_from(node.text.as_bytes().len()).ok()?,
        node.layout_weight,
        node.layout_margin_left_dp,
        node.layout_margin_top_dp,
        node.layout_margin_right_dp,
        node.layout_margin_bottom_dp,
        node.padding_left_dp,
        node.padding_top_dp,
        node.padding_right_dp,
        node.padding_bottom_dp,
    );
    #[cfg(all(
        feature = "androidbox-layout-spacing16",
        not(feature = "androidbox-layout-directional17")
    ))]
    let descriptor = AndroidAppSceneNodeDescriptor::new_spaced(
        kind,
        node.parent,
        node.id,
        width,
        height,
        orientation,
        node.callback_registered,
        u8::try_from(node.text.as_bytes().len()).ok()?,
        node.layout_weight,
        node.layout_margin_dp,
        node.padding_dp,
    );
    #[cfg(all(
        feature = "androidbox-layout-weight15",
        not(feature = "androidbox-layout-spacing16")
    ))]
    let descriptor = AndroidAppSceneNodeDescriptor::new_weighted(
        kind,
        node.parent,
        node.id,
        width,
        height,
        orientation,
        node.callback_registered,
        u8::try_from(node.text.as_bytes().len()).ok()?,
        node.layout_weight,
    );
    #[cfg(not(feature = "androidbox-layout-weight15"))]
    let descriptor = AndroidAppSceneNodeDescriptor::new(
        kind,
        node.parent,
        node.id,
        width,
        height,
        orientation,
        node.callback_registered,
        u8::try_from(node.text.as_bytes().len()).ok()?,
    );
    descriptor
}

#[cfg(feature = "androidbox-scene-rpc2")]
const fn scene_view_kind_from_wire(kind: AndroidAppSceneNodeKind) -> AndroidSceneViewKind {
    match kind {
        AndroidAppSceneNodeKind::LinearLayout => AndroidSceneViewKind::LinearLayout,
        AndroidAppSceneNodeKind::TextView => AndroidSceneViewKind::TextView,
        AndroidAppSceneNodeKind::Button => AndroidSceneViewKind::Button,
    }
}

#[cfg(feature = "androidbox-scene-rpc2")]
const fn scene_layout_size_from_wire(size: AndroidAppSceneDimension) -> AndroidSceneLayoutSize {
    match size {
        AndroidAppSceneDimension::MatchParent => AndroidSceneLayoutSize::MatchParent,
        AndroidAppSceneDimension::WrapContent => AndroidSceneLayoutSize::WrapContent,
        #[cfg(feature = "androidbox-layout-weight15")]
        AndroidAppSceneDimension::Zero => AndroidSceneLayoutSize::Zero,
        #[cfg(feature = "androidbox-layout-size18")]
        AndroidAppSceneDimension::Exact => AndroidSceneLayoutSize::Exact,
    }
}

#[cfg(feature = "androidbox-scene-rpc2")]
const fn scene_orientation_from_wire(
    orientation: AndroidAppSceneOrientation,
) -> AndroidSceneOrientation {
    match orientation {
        AndroidAppSceneOrientation::None => AndroidSceneOrientation::None,
        AndroidAppSceneOrientation::Vertical => AndroidSceneOrientation::Vertical,
        #[cfg(feature = "androidbox-layout-row14")]
        AndroidAppSceneOrientation::Horizontal => AndroidSceneOrientation::Horizontal,
    }
}

#[cfg(feature = "androidbox-scene-rpc2")]
fn write_scene_text_chunks(channel: u64, request_id: u64, index: u8, text: &[u8]) {
    if !(1..=ANDROID_APP_SCENE_NODE_TEXT_MAX_BYTES).contains(&text.len())
        || !text.iter().all(|byte| matches!(*byte, 0x20..=0x7e))
        || text_chunk_count(text.len()) >= ANDROID_APP_CHANNEL_QUEUE_CAPACITY
    {
        super::fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut offset = 0usize;
    while offset < text.len() {
        let end = (offset + ANDROID_APP_MESSAGE_PAYLOAD_BYTES).min(text.len());
        write_message(
            channel,
            AndroidAppMessage::new(
                AndroidAppMessageKind::NodeTextChunk,
                request_id,
                ((offset as u64) << 32) | text.len() as u64,
                u64::from(index),
                &text[offset..end],
            )
            .unwrap_or_else(|_| super::fail(FAIL_SURFACE_PROTOCOL)),
        );
        offset = end;
    }
}

fn write_text_chunks(channel: u64, request_id: u64, kind: AndroidAppMessageKind, text: &[u8]) {
    let maximum = match kind {
        AndroidAppMessageKind::LabelChunk | AndroidAppMessageKind::UpdateTextChunk => {
            ANDROID_APP_LABEL_MAX_BYTES
        }
        AndroidAppMessageKind::ButtonChunk => ANDROID_APP_BUTTON_MAX_BYTES,
        #[cfg(feature = "androidbox-restart0")]
        AndroidAppMessageKind::Crash => super::fail(FAIL_SURFACE_PROTOCOL),
        #[cfg(feature = "androidbox-scene-rpc2")]
        AndroidAppMessageKind::SceneOpened
        | AndroidAppMessageKind::DescribeNode
        | AndroidAppMessageKind::Node
        | AndroidAppMessageKind::NodeTextChunk => super::fail(FAIL_SURFACE_PROTOCOL),
        AndroidAppMessageKind::Ready
        | AndroidAppMessageKind::Open
        | AndroidAppMessageKind::Opened
        | AndroidAppMessageKind::Click
        | AndroidAppMessageKind::Updated
        | AndroidAppMessageKind::Close
        | AndroidAppMessageKind::Closed
        | AndroidAppMessageKind::Error => super::fail(FAIL_SURFACE_PROTOCOL),
    };
    if !(1..=maximum).contains(&text.len())
        || !text.iter().all(|byte| matches!(*byte, 0x20..=0x7e))
        || text_chunk_count(text.len()) > ANDROID_APP_CHANNEL_QUEUE_CAPACITY
    {
        super::fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut offset = 0usize;
    while offset < text.len() {
        let end = (offset + ANDROID_APP_MESSAGE_PAYLOAD_BYTES).min(text.len());
        write_message(
            channel,
            AndroidAppMessage::new(
                kind,
                request_id,
                ((offset as u64) << 32) | text.len() as u64,
                0,
                &text[offset..end],
            )
            .unwrap_or_else(|_| super::fail(FAIL_SURFACE_PROTOCOL)),
        );
        offset = end;
    }
}

fn write_error(channel: u64, request_id: u64, error: u32) {
    if !matches!(
        error,
        ERROR_OPEN_REJECTED | ERROR_CLICK_REJECTED | ERROR_CLOSE_REJECTED | ERROR_BUSY
    ) {
        super::fail(FAIL_SURFACE_PROTOCOL);
    }
    write_message(
        channel,
        AndroidAppMessage::new(
            AndroidAppMessageKind::Error,
            request_id,
            u64::from(error),
            0,
            &[],
        )
        .unwrap_or_else(|_| super::fail(FAIL_SURFACE_PROTOCOL)),
    );
}

fn write_message(channel: u64, message: AndroidAppMessage) {
    wait_writable(channel);
    let wire = message.encode();
    let result = syscall(
        SyscallNumber::ChannelWriteBytes,
        channel,
        wire.as_ptr() as u64,
        wire.len() as u64,
    );
    if result.status != Status::Ok.raw()
        || result.out1 != ANDROID_APP_MESSAGE_WIRE_SIZE as u64
        || result.out2 != 0
    {
        super::fail(FAIL_SURFACE_PROTOCOL);
    }
}

fn read_message(channel: u64) -> (u64, AndroidAppMessage) {
    let envelope = read_channel_envelope(channel);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != ANDROID_APP_MESSAGE_WIRE_SIZE
        || envelope.received_handle().is_valid()
    {
        super::fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut wire = [0; ANDROID_APP_MESSAGE_WIRE_SIZE];
    wire.copy_from_slice(&envelope.data()[..ANDROID_APP_MESSAGE_WIRE_SIZE]);
    let message =
        AndroidAppMessage::decode(&wire).unwrap_or_else(|_| super::fail(FAIL_SURFACE_PROTOCOL));
    (envelope.sender_pid(), message)
}

fn read_worker_ready(channel: u64, expected_pid: Option<u64>) -> u64 {
    let envelope = read_channel_envelope(channel);
    if envelope.kind() != ChannelMessageKind::Bytes
        || envelope.logical_length() != ANDROID_APP_MESSAGE_WIRE_SIZE
        || envelope.received_handle().is_valid()
        || envelope.sender_pid() == 0
        || expected_pid.is_some_and(|expected| envelope.sender_pid() != expected)
    {
        super::fail(FAIL_SURFACE_PROTOCOL);
    }
    let mut wire = [0; ANDROID_APP_MESSAGE_WIRE_SIZE];
    wire.copy_from_slice(&envelope.data()[..ANDROID_APP_MESSAGE_WIRE_SIZE]);
    let message =
        AndroidAppMessage::decode(&wire).unwrap_or_else(|_| super::fail(FAIL_SURFACE_PROTOCOL));
    if message.kind() != AndroidAppMessageKind::Ready
        || message.request_id() != 0
        || message.arg0() != ABI_VERSION
        || message.arg1() != 0
    {
        super::fail(FAIL_SURFACE_PROTOCOL);
    }
    envelope.sender_pid()
}

fn worker_activity_is_inactive(
    active_session_id: u64,
    active_generation: u64,
    active_revision: u32,
    active_label_id: u32,
    active_button_id: u32,
) -> bool {
    active_session_id == 0
        && active_generation == 0
        && active_revision == 0
        && active_label_id == 0
        && active_button_id == 0
}

fn clear_worker_activity(
    active_session_id: &mut u64,
    active_generation: &mut u64,
    active_revision: &mut u32,
    active_label_id: &mut u32,
    active_button_id: &mut u32,
) {
    *active_session_id = 0;
    *active_generation = 0;
    *active_revision = 0;
    *active_label_id = 0;
    *active_button_id = 0;
}

fn reset_worker_lease(
    lease: &mut InstalledInteractiveActivityLease,
    active_session_id: &mut u64,
    active_generation: &mut u64,
    active_revision: &mut u32,
    active_label_id: &mut u32,
    active_button_id: &mut u32,
) {
    // A successful lease.open can precede publication of the active identity,
    // so close unconditionally. This also wipes the process-private APK copy.
    let _ = lease.close();
    clear_worker_activity(
        active_session_id,
        active_generation,
        active_revision,
        active_label_id,
        active_button_id,
    );
}
