//! Feature-gated decoding for authenticated userspace UI channel evidence.
//!
//! The syscall layer resolves both generation-qualified process identities
//! before calling this module. This decoder then admits only canonical wire
//! payloads travelling in one of the two expected UI directions. It does not
//! mutate protocol state or participate in message delivery.

use bndr_ui::{
    BufferPresent, InputSample, PresentFrame, PresentMode, ShellAppId, ShellRect, TextInputCommand,
    TextInputEvent, UiAndroidBoxActivityLifecycleKind, UiAndroidBoxExecutionKind, UiAppearance,
    UiAppearanceAction, UiClientControl, UiClientControlPayload, UiClientId,
    UiCompatibleActivityIdentity, UiCompatibleActivityReservationOrigin, UiRecentIdentity,
    UiServerEvent, UiServerEventPayload, UiSystemUiAction, UiSystemUiMode, UiSystemUiRequestStatus,
    WindowCommand, WindowCommandOpcode, WindowCommandPayload, WindowEvent, WindowEventPayload,
    WindowId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTraceRole {
    SurfaceServer,
    Launcher,
    App,
}

impl UiTraceRole {
    pub const fn image_name(self) -> &'static str {
        match self {
            Self::SurfaceServer => "surface-server",
            Self::Launcher => "launcher",
            Self::App => "app",
        }
    }

    const fn is_client(self) -> bool {
        matches!(self, Self::Launcher | Self::App)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiChannelTrace {
    Input {
        session_id: u64,
        sample: InputSample,
    },
    FocusChanged {
        session_id: u64,
        active_client: UiClientId,
        app: Option<ShellAppId>,
        focus_generation: u64,
    },
    PresentRead {
        frame_id: u32,
        focus_generation: u32,
        mode: PresentMode,
    },
    PresentCancelled {
        session_id: u64,
        frame_id: u32,
        focus_generation: u64,
    },
    Presented {
        session_id: u64,
        frame_id: u32,
        commit: u64,
    },
    AppearanceRequest {
        action: UiAppearanceAction,
        request_id: u64,
    },
    AppearanceChanged {
        session_id: u64,
        appearance: UiAppearance,
        revision: u64,
    },
    BootNotificationDismissRequest {
        request_id: u64,
    },
    BootNotificationChanged {
        session_id: u64,
        visible: bool,
        revision: u64,
    },
    SystemUiRequest {
        action: UiSystemUiAction,
        recent: Option<UiRecentIdentity>,
        request_id: u64,
        observed_revision: u64,
    },
    SystemUiChanged {
        session_id: u64,
        mode: UiSystemUiMode,
        recent: Option<UiRecentIdentity>,
        nav_pressed: bool,
        nav_reveal_px: u16,
        revision: u64,
    },
    SystemUiRequestCompleted {
        session_id: u64,
        action: UiSystemUiAction,
        status: UiSystemUiRequestStatus,
        request_id: u64,
        revision: u64,
        compatible_identity: Option<UiCompatibleActivityIdentity>,
        reservation_origin: Option<UiCompatibleActivityReservationOrigin>,
    },
    AndroidBoxDexExecuted {
        request_id: u64,
        kind: UiAndroidBoxExecutionKind,
        dex_crc32: u32,
        dex_adler32: u32,
        result: i32,
        instruction_count: u8,
        tap_count: u16,
    },
    AndroidBoxActivityExecuted {
        request_id: u32,
        lifecycle: UiAndroidBoxActivityLifecycleKind,
        manifest_crc32: u32,
        dex_crc32: u32,
        dex_adler32: u32,
        activity_descriptor_crc32: u32,
        on_create_method_index: u16,
        on_create_code_offset: u32,
        instruction_count: u8,
        view_text_label: u16,
        view_text_length: u8,
    },
    AndroidBoxResourcesResolved {
        request_id: u32,
        resources_arsc_crc32: u32,
        layout_xml_crc32: u32,
        layout_resource_id: u32,
        string_resource_id: u32,
        view_text_label: u16,
        view_text_length: u8,
        success_flags: u8,
    },
    ClockChanged {
        session_id: u64,
        unix_seconds: u64,
        revision: u64,
    },
}

/// Authenticated M41 window traffic decoded from one canonical channel read.
///
/// Commands retain a uniform field set so trace consumers can log one stable
/// schema across all four opcodes. Fields unused by an opcode are `None`; the
/// canonical wire decoder has already proved that their encoded bytes were
/// zero. Events use dedicated variants because their evidence is materially
/// different, especially signed local input coordinates and capture state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowChannelTrace {
    Command {
        sender_role: UiTraceRole,
        sequence: u64,
        opcode: WindowCommandOpcode,
        window_id: Option<WindowId>,
        bounds: Option<ShellRect>,
        damage: Option<ShellRect>,
        frame_id: Option<u32>,
        color: Option<u32>,
    },
    Created {
        sender_role: UiTraceRole,
        receiver_role: UiTraceRole,
        command_sequence: u64,
        scene_frame: u32,
        window_id: WindowId,
        bounds: ShellRect,
    },
    Presented {
        sender_role: UiTraceRole,
        receiver_role: UiTraceRole,
        command_sequence: u64,
        scene_frame: u32,
        window_id: WindowId,
        frame_id: u32,
    },
    Raised {
        sender_role: UiTraceRole,
        receiver_role: UiTraceRole,
        command_sequence: u64,
        scene_frame: u32,
        window_id: WindowId,
    },
    InputRoute {
        sender_role: UiTraceRole,
        receiver_role: UiTraceRole,
        command_sequence: u64,
        scene_frame: u32,
        window_id: WindowId,
        global_x: u16,
        global_y: u16,
        local_x: i32,
        local_y: i32,
        pressed: bool,
        captured: bool,
        focus_generation: u64,
    },
    Destroyed {
        sender_role: UiTraceRole,
        receiver_role: UiTraceRole,
        command_sequence: u64,
        scene_frame: u32,
        window_id: WindowId,
    },
}

/// Authenticated ABI-21 text-input traffic. The fixed BTI1/BTE1 decoder has
/// already rejected every non-canonical reserved byte, UTF-8 tail, context,
/// sequence, and opcode representation before this value is constructed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextChannelTrace {
    Command {
        sender_role: UiTraceRole,
        receiver_role: UiTraceRole,
        command: TextInputCommand,
    },
    Event {
        sender_role: UiTraceRole,
        receiver_role: UiTraceRole,
        event: TextInputEvent,
    },
}

/// Compatibility-preserving union used by authenticated channel tracing.
/// Existing callers may continue using [`decode_ui_channel_read`] and its
/// unchanged return type; new callers can opt into both legacy and M41 window
/// evidence without weakening either decoder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticatedUiChannelTrace {
    Legacy(UiChannelTrace),
    Window(WindowChannelTrace),
    Text(TextChannelTrace),
}

/// Decodes one already-committed `ChannelReadEnvelope` byte payload.
///
/// Role checks happen before wire decoding so a protocol-shaped payload on an
/// unrelated service channel cannot produce UI evidence. Ready and Degraded
/// events are intentionally silent because they do not prove routing, focus,
/// or presentation behavior.
pub fn decode_ui_channel_read(
    sender: UiTraceRole,
    receiver: UiTraceRole,
    wire: &[u8],
) -> Option<UiChannelTrace> {
    if sender == UiTraceRole::SurfaceServer && receiver.is_client() {
        let event = UiServerEvent::decode(wire).ok()?;
        return match event.payload() {
            UiServerEventPayload::Input(sample) => Some(UiChannelTrace::Input {
                session_id: event.session_id(),
                sample,
            }),
            UiServerEventPayload::FocusChanged {
                active_client,
                app,
                focus_generation,
            } => Some(UiChannelTrace::FocusChanged {
                session_id: event.session_id(),
                active_client,
                app,
                focus_generation,
            }),
            UiServerEventPayload::PresentCancelled {
                frame_id,
                focus_generation,
            } => Some(UiChannelTrace::PresentCancelled {
                session_id: event.session_id(),
                frame_id,
                focus_generation,
            }),
            UiServerEventPayload::Presented { frame_id, commit } => {
                Some(UiChannelTrace::Presented {
                    session_id: event.session_id(),
                    frame_id,
                    commit,
                })
            }
            UiServerEventPayload::AppearanceChanged {
                appearance,
                revision,
            } => Some(UiChannelTrace::AppearanceChanged {
                session_id: event.session_id(),
                appearance,
                revision,
            }),
            UiServerEventPayload::ClockChanged {
                unix_seconds,
                revision,
            } => Some(UiChannelTrace::ClockChanged {
                session_id: event.session_id(),
                unix_seconds,
                revision,
            }),
            UiServerEventPayload::BootNotificationChanged { visible, revision } => {
                Some(UiChannelTrace::BootNotificationChanged {
                    session_id: event.session_id(),
                    visible,
                    revision,
                })
            }
            UiServerEventPayload::SystemUiChanged {
                mode,
                recent,
                nav_pressed,
                nav_reveal_px,
                revision,
            } => Some(UiChannelTrace::SystemUiChanged {
                session_id: event.session_id(),
                mode,
                recent,
                nav_pressed,
                nav_reveal_px,
                revision,
            }),
            UiServerEventPayload::SystemUiRequestCompleted {
                action,
                status,
                request_id,
                revision,
                compatible_identity,
                reservation_origin,
            } => Some(UiChannelTrace::SystemUiRequestCompleted {
                session_id: event.session_id(),
                action,
                status,
                request_id,
                revision,
                compatible_identity,
                reservation_origin,
            }),
            UiServerEventPayload::Ready | UiServerEventPayload::Degraded { .. } => None,
        };
    }

    if sender.is_client() && receiver == UiTraceRole::SurfaceServer {
        if let Ok(control) = UiClientControl::decode(wire) {
            match control.payload() {
                UiClientControlPayload::UpdateAppearance { action, request_id } => {
                    return Some(UiChannelTrace::AppearanceRequest { action, request_id });
                }
                UiClientControlPayload::DismissBootNotification { request_id } => {
                    return Some(UiChannelTrace::BootNotificationDismissRequest { request_id });
                }
                UiClientControlPayload::UpdateSystemUi {
                    action,
                    recent,
                    request_id,
                    observed_revision,
                } => {
                    if sender != UiTraceRole::Launcher {
                        return None;
                    }
                    return Some(UiChannelTrace::SystemUiRequest {
                        action,
                        recent,
                        request_id,
                        observed_revision,
                    });
                }
                UiClientControlPayload::ReportAndroidBoxExecution { report } => {
                    if sender != UiTraceRole::Launcher {
                        return None;
                    }
                    return Some(UiChannelTrace::AndroidBoxDexExecuted {
                        request_id: report.request_id(),
                        kind: report.kind(),
                        dex_crc32: report.dex_crc32(),
                        dex_adler32: report.dex_adler32(),
                        result: report.result(),
                        instruction_count: report.instruction_count(),
                        tap_count: report.tap_count(),
                    });
                }
                UiClientControlPayload::ReportAndroidBoxActivityExecution { report } => {
                    if sender != UiTraceRole::Launcher {
                        return None;
                    }
                    return Some(UiChannelTrace::AndroidBoxActivityExecuted {
                        request_id: report.request_id(),
                        lifecycle: report.lifecycle(),
                        manifest_crc32: report.manifest_crc32(),
                        dex_crc32: report.dex_crc32(),
                        dex_adler32: report.dex_adler32(),
                        activity_descriptor_crc32: report.activity_descriptor_crc32(),
                        on_create_method_index: report.on_create_method_index(),
                        on_create_code_offset: report.on_create_code_offset(),
                        instruction_count: report.instruction_count(),
                        view_text_label: report.view_text_label(),
                        view_text_length: report.view_text_length(),
                    });
                }
                UiClientControlPayload::ReportAndroidBoxResourceExecution { report } => {
                    if sender != UiTraceRole::Launcher {
                        return None;
                    }
                    return Some(UiChannelTrace::AndroidBoxResourcesResolved {
                        request_id: report.request_id(),
                        resources_arsc_crc32: report.resources_arsc_crc32(),
                        layout_xml_crc32: report.layout_xml_crc32(),
                        layout_resource_id: report.layout_resource_id(),
                        string_resource_id: report.string_resource_id(),
                        view_text_label: report.view_text_label(),
                        view_text_length: report.view_text_length(),
                        success_flags: report.success_flags(),
                    });
                }
                UiClientControlPayload::AttachAppEndpoint { .. }
                | UiClientControlPayload::SetFocus { .. } => {}
            }
        }
        if let Ok(frame) = PresentFrame::decode(wire) {
            return Some(UiChannelTrace::PresentRead {
                frame_id: frame.frame_id(),
                focus_generation: frame.focus_generation(),
                mode: frame.mode(),
            });
        }
        let frame = BufferPresent::decode(wire).ok()?;
        return Some(UiChannelTrace::PresentRead {
            frame_id: frame.client_frame_id(),
            focus_generation: frame.focus_generation(),
            mode: PresentMode::Full,
        });
    }

    None
}

/// Decodes only canonical M41 window traffic after process identities have
/// already been authenticated by the syscall layer.
///
/// Launcher owns slot 0 and App owns slot 1. Commands with a window id must be
/// sent by that owner; every event must be delivered to that owner. Create is
/// the sole command without an allocated id, so its owner cannot be checked
/// beyond requiring an authenticated client sender.
pub fn decode_window_ui_channel_read(
    sender: UiTraceRole,
    receiver: UiTraceRole,
    wire: &[u8],
) -> Option<WindowChannelTrace> {
    if sender.is_client() && receiver == UiTraceRole::SurfaceServer {
        let command = WindowCommand::decode(wire).ok()?;
        let (window_id, bounds, damage, frame_id, color) = match command.payload() {
            WindowCommandPayload::Create { bounds } => (None, Some(bounds), None, None, None),
            WindowCommandPayload::Present {
                window_id,
                frame_id,
                damage,
                color,
            } => {
                if window_client_role(window_id) != sender {
                    return None;
                }
                (
                    Some(window_id),
                    None,
                    Some(damage),
                    Some(frame_id),
                    Some(color),
                )
            }
            WindowCommandPayload::Raise { window_id }
            | WindowCommandPayload::Destroy { window_id } => {
                if window_client_role(window_id) != sender {
                    return None;
                }
                (Some(window_id), None, None, None, None)
            }
        };
        return Some(WindowChannelTrace::Command {
            sender_role: sender,
            sequence: command.sequence(),
            opcode: command.opcode(),
            window_id,
            bounds,
            damage,
            frame_id,
            color,
        });
    }

    if sender == UiTraceRole::SurfaceServer && receiver.is_client() {
        let event = WindowEvent::decode(wire).ok()?;
        let window_id = event.window_id();
        if window_client_role(window_id) != receiver {
            return None;
        }
        return match event.payload() {
            WindowEventPayload::Created { bounds, .. } => Some(WindowChannelTrace::Created {
                sender_role: sender,
                receiver_role: receiver,
                command_sequence: event.command_sequence(),
                scene_frame: event.scene_frame(),
                window_id,
                bounds,
            }),
            WindowEventPayload::Presented { frame_id, .. } => Some(WindowChannelTrace::Presented {
                sender_role: sender,
                receiver_role: receiver,
                command_sequence: event.command_sequence(),
                scene_frame: event.scene_frame(),
                window_id,
                frame_id,
            }),
            WindowEventPayload::Raised { .. } => Some(WindowChannelTrace::Raised {
                sender_role: sender,
                receiver_role: receiver,
                command_sequence: event.command_sequence(),
                scene_frame: event.scene_frame(),
                window_id,
            }),
            WindowEventPayload::InputRoute {
                global_x,
                global_y,
                local_x,
                local_y,
                pressed,
                captured,
                focus_generation,
                ..
            } => Some(WindowChannelTrace::InputRoute {
                sender_role: sender,
                receiver_role: receiver,
                command_sequence: event.command_sequence(),
                scene_frame: event.scene_frame(),
                window_id,
                global_x,
                global_y,
                local_x,
                local_y,
                pressed,
                captured,
                focus_generation,
            }),
            WindowEventPayload::Destroyed { .. } => Some(WindowChannelTrace::Destroyed {
                sender_role: sender,
                receiver_role: receiver,
                command_sequence: event.command_sequence(),
                scene_frame: event.scene_frame(),
                window_id,
            }),
        };
    }

    None
}

/// Decodes legacy traffic first, preserving its exact historical result, then
/// admits canonical window traffic through the independently versioned wire.
pub fn decode_authenticated_ui_channel_read(
    sender: UiTraceRole,
    receiver: UiTraceRole,
    wire: &[u8],
) -> Option<AuthenticatedUiChannelTrace> {
    if let Some(trace) = decode_ui_channel_read(sender, receiver, wire) {
        return Some(AuthenticatedUiChannelTrace::Legacy(trace));
    }
    if let Some(trace) = decode_window_ui_channel_read(sender, receiver, wire) {
        return Some(AuthenticatedUiChannelTrace::Window(trace));
    }
    decode_text_ui_channel_read(sender, receiver, wire).map(AuthenticatedUiChannelTrace::Text)
}

/// Decodes only the App-owned text session. Launcher can never impersonate a
/// text client, and SurfaceServer is the sole permitted event producer.
pub fn decode_text_ui_channel_read(
    sender: UiTraceRole,
    receiver: UiTraceRole,
    wire: &[u8],
) -> Option<TextChannelTrace> {
    if sender == UiTraceRole::App && receiver == UiTraceRole::SurfaceServer {
        let command = TextInputCommand::decode(wire).ok()?;
        if command.context().window_id().slot() != 1 {
            return None;
        }
        return Some(TextChannelTrace::Command {
            sender_role: sender,
            receiver_role: receiver,
            command,
        });
    }
    if sender == UiTraceRole::SurfaceServer && receiver == UiTraceRole::App {
        let event = TextInputEvent::decode(wire).ok()?;
        if event.context().window_id().slot() != 1 {
            return None;
        }
        return Some(TextChannelTrace::Event {
            sender_role: sender,
            receiver_role: receiver,
            event,
        });
    }
    None
}

fn window_client_role(window_id: WindowId) -> UiTraceRole {
    match window_id.slot() {
        0 => UiTraceRole::Launcher,
        1 => UiTraceRole::App,
        _ => unreachable!("canonical WindowId escaped its fixed two-slot domain"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bndr_ui::{
        SolidRect, UI_ANDROIDBOX_ACTIVITY_DESCRIPTOR_CRC32, UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT,
        UI_ANDROIDBOX_RESOURCE_ARSC_CRC32, UI_ANDROIDBOX_RESOURCE_LAYOUT_ID,
        UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32, UI_ANDROIDBOX_RESOURCE_STRING_ID,
        UI_ANDROIDBOX_RESOURCE_SUCCESS_FLAGS, UI_ANDROIDBOX_RESOURCE_VIEW_TEXT,
        UiServerDegradedReason, androidbox_activity_view_text_label,
    };

    fn rect() -> SolidRect {
        SolidRect::try_new(1, 2, 3, 4, 0x12_3456).unwrap()
    }

    fn launcher_window() -> WindowId {
        WindowId::try_new(0, 7).unwrap()
    }

    fn app_window() -> WindowId {
        WindowId::try_new(1, 11).unwrap()
    }

    #[test]
    fn decodes_all_canonical_client_window_commands_with_one_trace_schema() {
        let launcher = launcher_window();
        let app = app_window();
        let bounds = ShellRect::new(48, 80, 112, 160);
        let damage = ShellRect::new(16, 24, 40, 32);

        let create = WindowCommand::create(1, bounds).unwrap().encode();
        let create_trace = WindowChannelTrace::Command {
            sender_role: UiTraceRole::Launcher,
            sequence: 1,
            opcode: WindowCommandOpcode::Create,
            window_id: None,
            bounds: Some(bounds),
            damage: None,
            frame_id: None,
            color: None,
        };
        assert_eq!(
            decode_window_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &create,
            ),
            Some(create_trace)
        );
        assert_eq!(
            decode_authenticated_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &create,
            ),
            Some(AuthenticatedUiChannelTrace::Window(create_trace))
        );

        let present = WindowCommand::present(2, app, 3, damage, 0x0012_3456)
            .unwrap()
            .encode();
        assert_eq!(
            decode_window_ui_channel_read(UiTraceRole::App, UiTraceRole::SurfaceServer, &present,),
            Some(WindowChannelTrace::Command {
                sender_role: UiTraceRole::App,
                sequence: 2,
                opcode: WindowCommandOpcode::Present,
                window_id: Some(app),
                bounds: None,
                damage: Some(damage),
                frame_id: Some(3),
                color: Some(0x0012_3456),
            })
        );

        let raised = WindowCommand::raise(3, launcher).unwrap().encode();
        assert_eq!(
            decode_window_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &raised,
            ),
            Some(WindowChannelTrace::Command {
                sender_role: UiTraceRole::Launcher,
                sequence: 3,
                opcode: WindowCommandOpcode::Raise,
                window_id: Some(launcher),
                bounds: None,
                damage: None,
                frame_id: None,
                color: None,
            })
        );

        let destroyed = WindowCommand::destroy(4, app).unwrap().encode();
        assert_eq!(
            decode_window_ui_channel_read(UiTraceRole::App, UiTraceRole::SurfaceServer, &destroyed,),
            Some(WindowChannelTrace::Command {
                sender_role: UiTraceRole::App,
                sequence: 4,
                opcode: WindowCommandOpcode::Destroy,
                window_id: Some(app),
                bounds: None,
                damage: None,
                frame_id: None,
                color: None,
            })
        );
    }

    #[test]
    fn decodes_all_canonical_server_window_events_for_the_exact_owner() {
        let launcher = launcher_window();
        let app = app_window();
        let bounds = ShellRect::new(0, 0, 208, 368);

        let created = WindowEvent::created(1, 1, launcher, bounds)
            .unwrap()
            .encode();
        assert_eq!(
            decode_window_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::Launcher,
                &created,
            ),
            Some(WindowChannelTrace::Created {
                sender_role: UiTraceRole::SurfaceServer,
                receiver_role: UiTraceRole::Launcher,
                command_sequence: 1,
                scene_frame: 1,
                window_id: launcher,
                bounds,
            })
        );

        let presented = WindowEvent::presented(2, 2, app, 9).unwrap().encode();
        assert_eq!(
            decode_window_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::App, &presented,),
            Some(WindowChannelTrace::Presented {
                sender_role: UiTraceRole::SurfaceServer,
                receiver_role: UiTraceRole::App,
                command_sequence: 2,
                scene_frame: 2,
                window_id: app,
                frame_id: 9,
            })
        );

        let raised = WindowEvent::raised(3, 3, launcher).unwrap().encode();
        assert_eq!(
            decode_window_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::Launcher,
                &raised,
            ),
            Some(WindowChannelTrace::Raised {
                sender_role: UiTraceRole::SurfaceServer,
                receiver_role: UiTraceRole::Launcher,
                command_sequence: 3,
                scene_frame: 3,
                window_id: launcher,
            })
        );

        let input = WindowEvent::input_route(4, 4, app, 80, 120, 32, 40, true, true, 5)
            .unwrap()
            .encode();
        assert_eq!(
            decode_window_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::App, &input,),
            Some(WindowChannelTrace::InputRoute {
                sender_role: UiTraceRole::SurfaceServer,
                receiver_role: UiTraceRole::App,
                command_sequence: 4,
                scene_frame: 4,
                window_id: app,
                global_x: 80,
                global_y: 120,
                local_x: 32,
                local_y: 40,
                pressed: true,
                captured: true,
                focus_generation: 5,
            })
        );

        let destroyed = WindowEvent::destroyed(5, 5, app).unwrap().encode();
        assert_eq!(
            decode_window_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::App, &destroyed,),
            Some(WindowChannelTrace::Destroyed {
                sender_role: UiTraceRole::SurfaceServer,
                receiver_role: UiTraceRole::App,
                command_sequence: 5,
                scene_frame: 5,
                window_id: app,
            })
        );
    }

    #[test]
    fn rejects_window_commands_from_the_wrong_direction_or_slot_owner() {
        let launcher_present =
            WindowCommand::present(1, launcher_window(), 1, ShellRect::new(0, 0, 1, 1), 0)
                .unwrap()
                .encode();
        let app_present = WindowCommand::present(1, app_window(), 1, ShellRect::new(0, 0, 1, 1), 0)
            .unwrap()
            .encode();
        assert_eq!(
            decode_window_ui_channel_read(
                UiTraceRole::App,
                UiTraceRole::SurfaceServer,
                &launcher_present,
            ),
            None
        );
        assert_eq!(
            decode_window_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &app_present,
            ),
            None
        );
        for (sender, receiver) in [
            (UiTraceRole::SurfaceServer, UiTraceRole::Launcher),
            (UiTraceRole::SurfaceServer, UiTraceRole::SurfaceServer),
            (UiTraceRole::Launcher, UiTraceRole::App),
            (UiTraceRole::App, UiTraceRole::Launcher),
        ] {
            assert_eq!(
                decode_window_ui_channel_read(sender, receiver, &launcher_present),
                None
            );
        }

        let create = WindowCommand::create(1, ShellRect::new(0, 0, 1, 1))
            .unwrap()
            .encode();
        assert!(
            decode_window_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &create,
            )
            .is_some()
        );
        assert!(
            decode_window_ui_channel_read(UiTraceRole::App, UiTraceRole::SurfaceServer, &create,)
                .is_some()
        );
        assert_eq!(
            decode_window_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::Launcher,
                &create,
            ),
            None
        );
    }

    #[test]
    fn rejects_window_events_sent_to_the_wrong_direction_or_slot_owner() {
        let launcher = WindowEvent::raised(1, 1, launcher_window())
            .unwrap()
            .encode();
        let app = WindowEvent::raised(1, 1, app_window()).unwrap().encode();
        assert_eq!(
            decode_window_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::App, &launcher,),
            None
        );
        assert_eq!(
            decode_window_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::Launcher, &app,),
            None
        );
        for (sender, receiver) in [
            (UiTraceRole::Launcher, UiTraceRole::SurfaceServer),
            (UiTraceRole::App, UiTraceRole::SurfaceServer),
            (UiTraceRole::Launcher, UiTraceRole::App),
            (UiTraceRole::SurfaceServer, UiTraceRole::SurfaceServer),
        ] {
            assert_eq!(
                decode_window_ui_channel_read(sender, receiver, &launcher),
                None
            );
        }
    }

    #[test]
    fn rejects_truncated_and_noncanonical_window_wires_without_legacy_fallback() {
        let command = WindowCommand::raise(1, launcher_window()).unwrap().encode();
        let event = WindowEvent::raised(1, 1, launcher_window())
            .unwrap()
            .encode();
        assert_eq!(
            decode_authenticated_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &command[..63],
            ),
            None
        );
        assert_eq!(
            decode_authenticated_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::Launcher,
                &event[..63],
            ),
            None
        );
        for (mut wire, sender, receiver) in [
            (command, UiTraceRole::Launcher, UiTraceRole::SurfaceServer),
            (event, UiTraceRole::SurfaceServer, UiTraceRole::Launcher),
        ] {
            wire[63] = 1;
            assert_eq!(
                decode_authenticated_ui_channel_read(sender, receiver, &wire),
                None
            );
        }

        let mut bad_command_token = command;
        bad_command_token[16..24].fill(0);
        assert_eq!(
            decode_window_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &bad_command_token,
            ),
            None
        );
        let mut bad_event_token = event;
        bad_event_token[24..32].fill(0);
        assert_eq!(
            decode_window_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::Launcher,
                &bad_event_token,
            ),
            None
        );
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::Launcher, UiTraceRole::SurfaceServer, &command,),
            None
        );
    }

    #[test]
    fn input_route_trace_preserves_negative_local_coordinates_and_capture() {
        let id = app_window();
        let input = WindowEvent::input_route(9, 7, id, 16, 32, -32, -48, false, true, 6)
            .unwrap()
            .encode();
        let expected = WindowChannelTrace::InputRoute {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            command_sequence: 9,
            scene_frame: 7,
            window_id: id,
            global_x: 16,
            global_y: 32,
            local_x: -32,
            local_y: -48,
            pressed: false,
            captured: true,
            focus_generation: 6,
        };
        assert_eq!(
            decode_window_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::App, &input,),
            Some(expected)
        );
        assert_eq!(
            decode_authenticated_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::App,
                &input,
            ),
            Some(AuthenticatedUiChannelTrace::Window(expected))
        );
    }

    #[test]
    fn authenticated_decoder_preserves_every_legacy_trace_result() {
        let frame = PresentFrame::damage(12, &[rect()])
            .unwrap()
            .with_focus_generation(8)
            .unwrap()
            .encode();
        let input_sample = InputSample::try_new(3, 160, 460, false).unwrap();
        let input = UiServerEvent::input_sample(9, input_sample)
            .unwrap()
            .encode();
        let legacy_present =
            decode_ui_channel_read(UiTraceRole::Launcher, UiTraceRole::SurfaceServer, &frame)
                .unwrap();
        let legacy_input =
            decode_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::App, &input).unwrap();
        assert_eq!(
            decode_authenticated_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &frame,
            ),
            Some(AuthenticatedUiChannelTrace::Legacy(legacy_present))
        );
        assert_eq!(
            decode_authenticated_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::App,
                &input,
            ),
            Some(AuthenticatedUiChannelTrace::Legacy(legacy_input))
        );

        let ready = UiServerEvent::ready(9).unwrap().encode();
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::App, &ready),
            None
        );
        assert_eq!(
            decode_authenticated_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::App,
                &ready,
            ),
            None
        );
    }

    #[test]
    fn decodes_only_canonical_server_to_client_routing_events() {
        let input = InputSample::try_new(3, 160, 460, false).unwrap();
        let input_event = UiServerEvent::input_sample(9, input).unwrap().encode();
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::App, &input_event,),
            Some(UiChannelTrace::Input {
                session_id: 9,
                sample: input,
            })
        );

        let focus = UiServerEvent::focus_changed(9, UiClientId::App, Some(ShellAppId::Phone), 8)
            .unwrap()
            .encode();
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::Launcher, &focus,),
            Some(UiChannelTrace::FocusChanged {
                session_id: 9,
                active_client: UiClientId::App,
                app: Some(ShellAppId::Phone),
                focus_generation: 8,
            })
        );

        let cancelled = UiServerEvent::present_cancelled(9, 12, 9).unwrap().encode();
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::App, &cancelled,),
            Some(UiChannelTrace::PresentCancelled {
                session_id: 9,
                frame_id: 12,
                focus_generation: 9,
            })
        );

        let presented = UiServerEvent::presented(9, 12, 17).unwrap().encode();
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::App, &presented,),
            Some(UiChannelTrace::Presented {
                session_id: 9,
                frame_id: 12,
                commit: 17,
            })
        );

        let appearance = UiAppearance::new(false, true);
        let appearance_changed = UiServerEvent::appearance_changed(9, appearance, 3)
            .unwrap()
            .encode();
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::Launcher,
                &appearance_changed,
            ),
            Some(UiChannelTrace::AppearanceChanged {
                session_id: 9,
                appearance,
                revision: 3,
            })
        );

        let clock_changed = UiServerEvent::clock_changed(9, 1_785_318_060, 1)
            .unwrap()
            .encode();
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::Launcher,
                &clock_changed,
            ),
            Some(UiChannelTrace::ClockChanged {
                session_id: 9,
                unix_seconds: 1_785_318_060,
                revision: 1,
            })
        );

        let boot_notification_changed = UiServerEvent::boot_notification_changed(9, false, 2)
            .unwrap()
            .encode();
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::App,
                &boot_notification_changed,
            ),
            Some(UiChannelTrace::BootNotificationChanged {
                session_id: 9,
                visible: false,
                revision: 2,
            })
        );

        let system_ui_changed = UiServerEvent::system_ui_changed(
            9,
            UiSystemUiMode::Foreground,
            Some(ShellAppId::Settings),
            true,
            240,
            4,
        )
        .unwrap()
        .encode();
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::Launcher,
                &system_ui_changed,
            ),
            Some(UiChannelTrace::SystemUiChanged {
                session_id: 9,
                mode: UiSystemUiMode::Foreground,
                recent: Some(UiRecentIdentity::Shell(ShellAppId::Settings)),
                nav_pressed: true,
                nav_reveal_px: 240,
                revision: 4,
            })
        );

        let compatible_identity = UiCompatibleActivityIdentity::new(17, 23).unwrap();
        let request_completed = UiServerEvent::system_ui_request_completed(
            9,
            UiSystemUiAction::ReserveCompatibleActivity,
            UiSystemUiRequestStatus::Accepted,
            5,
            6,
            Some(compatible_identity),
            Some(UiCompatibleActivityReservationOrigin::Home),
        )
        .unwrap()
        .encode();
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::Launcher,
                &request_completed,
            ),
            Some(UiChannelTrace::SystemUiRequestCompleted {
                session_id: 9,
                action: UiSystemUiAction::ReserveCompatibleActivity,
                status: UiSystemUiRequestStatus::Accepted,
                request_id: 5,
                revision: 6,
                compatible_identity: Some(compatible_identity),
                reservation_origin: Some(UiCompatibleActivityReservationOrigin::Home),
            })
        );

        let ready = UiServerEvent::ready(9).unwrap().encode();
        let degraded = UiServerEvent::degraded(9, UiServerDegradedReason::KernelDegraded)
            .unwrap()
            .encode();
        for silent in [ready, degraded] {
            assert_eq!(
                decode_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::App, &silent,),
                None
            );
        }
    }

    #[test]
    fn appearance_trace_authenticates_both_client_request_streams() {
        for (sender, action, request_id) in [
            (UiTraceRole::Launcher, UiAppearanceAction::ToggleTheme, 1),
            (UiTraceRole::App, UiAppearanceAction::ToggleAccent, 7),
        ] {
            let wire = UiClientControl::update_appearance(action, request_id)
                .unwrap()
                .encode();
            assert_eq!(
                decode_ui_channel_read(sender, UiTraceRole::SurfaceServer, &wire),
                Some(UiChannelTrace::AppearanceRequest { action, request_id })
            );
            assert_eq!(
                decode_ui_channel_read(UiTraceRole::SurfaceServer, sender, &wire),
                None
            );
        }
    }

    #[test]
    fn boot_notification_trace_authenticates_dismiss_and_changed_directions() {
        for (sender, request_id) in [
            (UiTraceRole::Launcher, 1),
            (UiTraceRole::App, 0x0102_0304_0506_0708),
        ] {
            let wire = UiClientControl::dismiss_boot_notification(request_id)
                .unwrap()
                .encode();
            assert_eq!(
                decode_ui_channel_read(sender, UiTraceRole::SurfaceServer, &wire),
                Some(UiChannelTrace::BootNotificationDismissRequest { request_id })
            );
            assert_eq!(
                decode_ui_channel_read(UiTraceRole::SurfaceServer, sender, &wire),
                None
            );
        }

        for (receiver, visible, revision) in [
            (UiTraceRole::Launcher, true, 1),
            (UiTraceRole::App, false, 2),
        ] {
            let wire = UiServerEvent::boot_notification_changed(9, visible, revision)
                .unwrap()
                .encode();
            assert_eq!(
                decode_ui_channel_read(UiTraceRole::SurfaceServer, receiver, &wire),
                Some(UiChannelTrace::BootNotificationChanged {
                    session_id: 9,
                    visible,
                    revision,
                })
            );
            assert_eq!(
                decode_ui_channel_read(receiver, UiTraceRole::SurfaceServer, &wire),
                None
            );
        }
    }

    #[test]
    fn system_ui_trace_authenticates_launcher_request_and_server_changed_directions() {
        let request = UiClientControl::update_system_ui(
            UiSystemUiAction::ActivateRecent,
            Some(ShellAppId::Messages),
            3,
            7,
        )
        .unwrap()
        .encode();
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::Launcher, UiTraceRole::SurfaceServer, &request,),
            Some(UiChannelTrace::SystemUiRequest {
                action: UiSystemUiAction::ActivateRecent,
                recent: Some(UiRecentIdentity::Shell(ShellAppId::Messages)),
                request_id: 3,
                observed_revision: 7,
            })
        );
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::App, UiTraceRole::SurfaceServer, &request),
            None
        );
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::Launcher, &request,),
            None
        );

        let changed = UiServerEvent::system_ui_changed(
            9,
            UiSystemUiMode::Overview,
            Some(ShellAppId::Messages),
            false,
            0,
            8,
        )
        .unwrap()
        .encode();
        for receiver in [UiTraceRole::Launcher, UiTraceRole::App] {
            assert_eq!(
                decode_ui_channel_read(UiTraceRole::SurfaceServer, receiver, &changed),
                Some(UiChannelTrace::SystemUiChanged {
                    session_id: 9,
                    mode: UiSystemUiMode::Overview,
                    recent: Some(UiRecentIdentity::Shell(ShellAppId::Messages)),
                    nav_pressed: false,
                    nav_reveal_px: 0,
                    revision: 8,
                })
            );
            assert_eq!(
                decode_ui_channel_read(receiver, UiTraceRole::SurfaceServer, &changed),
                None
            );
        }

        let compatible_identity = UiCompatibleActivityIdentity::new(17, 23).unwrap();
        let completed = UiServerEvent::system_ui_request_completed(
            9,
            UiSystemUiAction::AbortCompatibleActivityVerification,
            UiSystemUiRequestStatus::Conflict,
            11,
            8,
            Some(compatible_identity),
            Some(UiCompatibleActivityReservationOrigin::Overview),
        )
        .unwrap()
        .encode();
        for receiver in [UiTraceRole::Launcher, UiTraceRole::App] {
            assert_eq!(
                decode_ui_channel_read(UiTraceRole::SurfaceServer, receiver, &completed),
                Some(UiChannelTrace::SystemUiRequestCompleted {
                    session_id: 9,
                    action: UiSystemUiAction::AbortCompatibleActivityVerification,
                    status: UiSystemUiRequestStatus::Conflict,
                    request_id: 11,
                    revision: 8,
                    compatible_identity: Some(compatible_identity),
                    reservation_origin: Some(UiCompatibleActivityReservationOrigin::Overview),
                })
            );
            assert_eq!(
                decode_ui_channel_read(receiver, UiTraceRole::SurfaceServer, &completed),
                None
            );
        }

        let mut noncanonical_request = request;
        noncanonical_request[63] = 1;
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &noncanonical_request,
            ),
            None
        );
        let mut noncanonical_changed = changed;
        noncanonical_changed[18] |= 0x80;
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::Launcher,
                &noncanonical_changed,
            ),
            None
        );
    }

    #[test]
    fn androidbox_trace_requires_canonical_launcher_to_server_report() {
        let report = UiClientControl::report_androidbox_execution(
            7,
            UiAndroidBoxExecutionKind::Tap,
            0x1122_3344,
            0x5566_7788,
            -29,
            17,
            3,
        )
        .unwrap()
        .encode();
        let expected = UiChannelTrace::AndroidBoxDexExecuted {
            request_id: 7,
            kind: UiAndroidBoxExecutionKind::Tap,
            dex_crc32: 0x1122_3344,
            dex_adler32: 0x5566_7788,
            result: -29,
            instruction_count: 17,
            tap_count: 3,
        };
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::Launcher, UiTraceRole::SurfaceServer, &report,),
            Some(expected)
        );
        assert_eq!(
            decode_authenticated_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &report,
            ),
            Some(AuthenticatedUiChannelTrace::Legacy(expected))
        );
        for (sender, receiver) in [
            (UiTraceRole::App, UiTraceRole::SurfaceServer),
            (UiTraceRole::SurfaceServer, UiTraceRole::Launcher),
            (UiTraceRole::Launcher, UiTraceRole::App),
        ] {
            assert_eq!(decode_ui_channel_read(sender, receiver, &report), None);
        }

        let mut noncanonical = report;
        noncanonical[29] |= 0x80;
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &noncanonical,
            ),
            None
        );
        let mut version_five = report;
        version_five[4..6].copy_from_slice(&5_u16.to_le_bytes());
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &version_five,
            ),
            None
        );
    }

    #[test]
    fn androidbox_activity_trace_requires_canonical_launcher_to_server_report() {
        const VIEW_TEXT: &[u8] = UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT;
        let report = UiClientControl::report_androidbox_activity_execution(
            1,
            UiAndroidBoxActivityLifecycleKind::OnCreateComplete,
            0x1020_3040,
            0x1122_3344,
            0x5566_7788,
            23,
            0x240,
            19,
            VIEW_TEXT,
        )
        .unwrap()
        .encode();
        let expected = UiChannelTrace::AndroidBoxActivityExecuted {
            request_id: 1,
            lifecycle: UiAndroidBoxActivityLifecycleKind::OnCreateComplete,
            manifest_crc32: 0x1020_3040,
            dex_crc32: 0x1122_3344,
            dex_adler32: 0x5566_7788,
            activity_descriptor_crc32: UI_ANDROIDBOX_ACTIVITY_DESCRIPTOR_CRC32,
            on_create_method_index: 23,
            on_create_code_offset: 0x240,
            instruction_count: 19,
            view_text_label: androidbox_activity_view_text_label(VIEW_TEXT).unwrap(),
            view_text_length: VIEW_TEXT.len() as u8,
        };
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::Launcher, UiTraceRole::SurfaceServer, &report,),
            Some(expected)
        );
        assert_eq!(
            decode_authenticated_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &report,
            ),
            Some(AuthenticatedUiChannelTrace::Legacy(expected))
        );
        for (sender, receiver) in [
            (UiTraceRole::App, UiTraceRole::SurfaceServer),
            (UiTraceRole::SurfaceServer, UiTraceRole::Launcher),
            (UiTraceRole::Launcher, UiTraceRole::App),
        ] {
            assert_eq!(decode_ui_channel_read(sender, receiver, &report), None);
        }

        let mut invalid_lifecycle = report;
        invalid_lifecycle[31] = (invalid_lifecycle[31] & 0x3f) | 0x80;
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &invalid_lifecycle,
            ),
            None
        );
        let mut hidden_payload = report;
        hidden_payload[32] = 1;
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &hidden_payload,
            ),
            None
        );
        let mut version_five = report;
        version_five[4..6].copy_from_slice(&5_u16.to_le_bytes());
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &version_five,
            ),
            None
        );
    }

    #[test]
    fn androidbox_resource_trace_requires_canonical_launcher_to_server_report() {
        let report = UiClientControl::report_androidbox_resource_execution(
            1,
            UI_ANDROIDBOX_RESOURCE_ARSC_CRC32,
            UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32,
            UI_ANDROIDBOX_RESOURCE_LAYOUT_ID,
            UI_ANDROIDBOX_RESOURCE_STRING_ID,
            UI_ANDROIDBOX_RESOURCE_VIEW_TEXT,
        )
        .unwrap()
        .encode();
        let expected = UiChannelTrace::AndroidBoxResourcesResolved {
            request_id: 1,
            resources_arsc_crc32: UI_ANDROIDBOX_RESOURCE_ARSC_CRC32,
            layout_xml_crc32: UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32,
            layout_resource_id: UI_ANDROIDBOX_RESOURCE_LAYOUT_ID,
            string_resource_id: UI_ANDROIDBOX_RESOURCE_STRING_ID,
            view_text_label: androidbox_activity_view_text_label(UI_ANDROIDBOX_RESOURCE_VIEW_TEXT)
                .unwrap(),
            view_text_length: UI_ANDROIDBOX_RESOURCE_VIEW_TEXT.len() as u8,
            success_flags: UI_ANDROIDBOX_RESOURCE_SUCCESS_FLAGS,
        };
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::Launcher, UiTraceRole::SurfaceServer, &report),
            Some(expected)
        );
        assert_eq!(
            decode_authenticated_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &report,
            ),
            Some(AuthenticatedUiChannelTrace::Legacy(expected))
        );
        for (sender, receiver) in [
            (UiTraceRole::App, UiTraceRole::SurfaceServer),
            (UiTraceRole::SurfaceServer, UiTraceRole::Launcher),
            (UiTraceRole::Launcher, UiTraceRole::App),
        ] {
            assert_eq!(decode_ui_channel_read(sender, receiver, &report), None);
        }

        let mut reserved_high_nibble = report;
        reserved_high_nibble[31] |= 0x80;
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &reserved_high_nibble,
            ),
            None
        );
        let mut hidden_payload = report;
        hidden_payload[32] = 1;
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &hidden_payload,
            ),
            None
        );
        let mut version_six = report;
        version_six[4..6].copy_from_slice(&6_u16.to_le_bytes());
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &version_six,
            ),
            None
        );
    }

    #[test]
    fn decodes_client_present_with_its_local_generation() {
        let frame = PresentFrame::damage(12, &[rect()])
            .unwrap()
            .with_focus_generation(8)
            .unwrap()
            .encode();
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::App, UiTraceRole::SurfaceServer, &frame,),
            Some(UiChannelTrace::PresentRead {
                frame_id: 12,
                focus_generation: 8,
                mode: PresentMode::Damage,
            })
        );

        let buffered = BufferPresent::client(13, 9, 75).unwrap().encode();
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::App, UiTraceRole::SurfaceServer, &buffered,),
            Some(UiChannelTrace::PresentRead {
                frame_id: 13,
                focus_generation: 9,
                mode: PresentMode::Full,
            })
        );
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::Launcher, UiTraceRole::SurfaceServer, &frame,),
            Some(UiChannelTrace::PresentRead {
                frame_id: 12,
                focus_generation: 8,
                mode: PresentMode::Damage,
            })
        );
    }

    #[test]
    fn text_decoder_authenticates_only_the_app_owned_bti_bte_directions() {
        let context = bndr_ui::TextInputContext::try_new(app_window(), 1, 5).unwrap();
        let command = TextInputCommand::activate(1, context).unwrap();
        let event = TextInputEvent::activated(1, context, 1, 0).unwrap();
        let expected_command = TextChannelTrace::Command {
            sender_role: UiTraceRole::App,
            receiver_role: UiTraceRole::SurfaceServer,
            command,
        };
        let expected_event = TextChannelTrace::Event {
            sender_role: UiTraceRole::SurfaceServer,
            receiver_role: UiTraceRole::App,
            event,
        };

        assert_eq!(
            decode_authenticated_ui_channel_read(
                UiTraceRole::App,
                UiTraceRole::SurfaceServer,
                &command.encode(),
            ),
            Some(AuthenticatedUiChannelTrace::Text(expected_command))
        );
        assert_eq!(
            decode_authenticated_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::App,
                &event.encode(),
            ),
            Some(AuthenticatedUiChannelTrace::Text(expected_event))
        );
        assert_eq!(
            decode_text_ui_channel_read(
                UiTraceRole::Launcher,
                UiTraceRole::SurfaceServer,
                &command.encode(),
            ),
            None
        );
        assert_eq!(
            decode_text_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::Launcher,
                &event.encode(),
            ),
            None
        );

        let launcher_context = bndr_ui::TextInputContext::try_new(launcher_window(), 1, 5).unwrap();
        let launcher_command = TextInputCommand::activate(1, launcher_context)
            .unwrap()
            .encode();
        assert_eq!(
            decode_text_ui_channel_read(
                UiTraceRole::App,
                UiTraceRole::SurfaceServer,
                &launcher_command,
            ),
            None
        );
    }

    #[test]
    fn rejects_wrong_directions_non_ui_payloads_and_noncanonical_wires() {
        let input = UiServerEvent::input_sample(1, InputSample::try_new(1, 10, 20, true).unwrap())
            .unwrap()
            .encode();
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::App, UiTraceRole::Launcher, &input),
            None
        );
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::SurfaceServer,
                &input,
            ),
            None
        );
        assert_eq!(
            decode_ui_channel_read(
                UiTraceRole::SurfaceServer,
                UiTraceRole::App,
                &input[..input.len() - 1],
            ),
            None
        );

        let mut noncanonical = input;
        noncanonical[63] = 1;
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::SurfaceServer, UiTraceRole::App, &noncanonical,),
            None
        );
        assert_eq!(
            decode_ui_channel_read(UiTraceRole::App, UiTraceRole::SurfaceServer, &[0xa5; 64],),
            None
        );
    }
}
