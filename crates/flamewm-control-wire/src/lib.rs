//! Canonical Flame Control wire schema.
//!
//! This crate is intentionally D-Bus-library-free. It maps the native
//! `com.arkflame.FlameWM1` object ABI to typed `ControlRequest`/`ControlResponse` values. A
//! libdbus adapter only has to translate D-Bus primitive/container values to/from `WireValue`.
//! Product validation remains in Platform services.

use std::collections::BTreeMap;

use flamewm_api::display::{DisplayMode, DisplaySnapshot, OutputSnapshot, PendingModeChange};
use flamewm_api::panels::{PanelSnapshot, PanelsSnapshot, TaskEntry, TaskEntryKind};
use flamewm_api::session::{SessionAction, SessionCapabilities};
use flamewm_api::settings::{SettingValue, SettingsChange, SettingsSnapshot, SettingsTransaction};
use flamewm_api::shortcuts::{KeyBinding, ShortcutSnapshot};
use flamewm_api::workspace::WorkspaceSnapshot;
use flamewm_api::{
    DesktopAppId, ErrorCode, ModeId, OutputId, PanelEdge, Rect, Size, TaskEntryId, TransactionId,
    WindowRef,
};
use flamewm_control_core::{
    ControlError, ControlRequest, ControlResponse, IFACE_DISPLAYS, IFACE_PANELS, IFACE_ROOT,
    IFACE_SESSION, IFACE_SETTINGS, IFACE_SHORTCUTS, IFACE_WORKSPACES, Version,
};

pub type WireDict = BTreeMap<String, WireValue>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireValue {
    Bool(bool),
    U32(u32),
    U64(u64),
    I32(i32),
    I64(i64),
    String(String),
    StringArray(Vec<String>),
    Array(Vec<WireValue>),
    Dict(WireDict),
}

impl WireValue {
    fn as_u64(&self) -> Option<u64> {
        match self {
            Self::U64(value) => Some(*value),
            Self::U32(value) => Some(u64::from(*value)),
            _ => None,
        }
    }

    fn as_u32(&self) -> Option<u32> {
        match self {
            Self::U32(value) => Some(*value),
            Self::U64(value) => u32::try_from(*value).ok(),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    fn as_i32(&self) -> Option<i32> {
        match self {
            Self::I32(value) => Some(*value),
            Self::I64(value) => i32::try_from(*value).ok(),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&[WireValue]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    fn as_string_array(&self) -> Option<&[String]> {
        match self {
            Self::StringArray(value) => Some(value),
            _ => None,
        }
    }

    fn as_dict(&self) -> Option<&WireDict> {
        match self {
            Self::Dict(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireCall {
    pub interface: String,
    pub member: String,
    pub args: Vec<WireValue>,
    /// Monotonic clock supplied by the WM reactor; not part of the D-Bus ABI.
    pub now_ms: u64,
}

impl WireCall {
    #[must_use]
    pub fn new(
        interface: impl Into<String>,
        member: impl Into<String>,
        args: Vec<WireValue>,
    ) -> Self {
        Self {
            interface: interface.into(),
            member: member.into(),
            args,
            now_ms: 0,
        }
    }

    #[must_use]
    pub fn with_now_ms(mut self, now_ms: u64) -> Self {
        self.now_ms = now_ms;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WireReply {
    pub values: Vec<WireValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireError {
    pub name: String,
    pub message: String,
}

impl From<ControlError> for WireError {
    fn from(value: ControlError) -> Self {
        Self {
            name: value.name.to_owned(),
            message: value.message,
        }
    }
}

pub fn decode_call(call: &WireCall) -> Result<ControlRequest, ControlError> {
    let bad = |message: &str| ControlError {
        name: flamewm_control_core::error_name(ErrorCode::InvalidArgument),
        code: ErrorCode::InvalidArgument,
        message: message.to_owned(),
    };

    match (call.interface.as_str(), call.member.as_str()) {
        (IFACE_ROOT, "Ping") => expect_arity(call, 0).map(|()| ControlRequest::Ping),
        (IFACE_ROOT, "GetVersion") => expect_arity(call, 0).map(|()| ControlRequest::GetVersion),
        (IFACE_ROOT, "GetCapabilities") => {
            expect_arity(call, 0).map(|()| ControlRequest::GetCapabilities)
        }
        (IFACE_SETTINGS, "GetSnapshot") => {
            expect_arity(call, 0).map(|()| ControlRequest::GetSettings)
        }
        (IFACE_SETTINGS, "Apply") => {
            expect_arity(call, 2)?;
            let expected_revision = required_u64(&call.args[0], "expectedRevision")?;
            let changes = required_dict(&call.args[1], "changes")?;
            let changes = changes
                .iter()
                .map(|(key, value)| {
                    setting_value_for_key(key, value).map(|value| SettingsChange {
                        key: key.clone(),
                        value: Some(value),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ControlRequest::ApplySettings(SettingsTransaction {
                expected_revision,
                changes,
                reset_section: None,
            }))
        }
        (IFACE_SETTINGS, "ResetSection") => {
            expect_arity(call, 2)?;
            Ok(ControlRequest::ApplySettings(SettingsTransaction {
                expected_revision: required_u64(&call.args[0], "expectedRevision")?,
                changes: Vec::new(),
                reset_section: Some(required_string(&call.args[1], "section")?.to_owned()),
            }))
        }
        (IFACE_WORKSPACES, "GetSnapshot") => {
            expect_arity(call, 0).map(|()| ControlRequest::GetWorkspaces)
        }
        (IFACE_WORKSPACES, "Activate") => {
            workspace_index_request(call, |index, expected_revision| {
                ControlRequest::ActivateWorkspace {
                    index,
                    expected_revision,
                }
            })
        }
        (IFACE_WORKSPACES, "InsertAfter") => {
            workspace_index_request(call, |index, expected_revision| {
                ControlRequest::InsertWorkspaceAfter {
                    index,
                    expected_revision,
                }
            })
        }
        (IFACE_WORKSPACES, "Remove") => {
            workspace_index_request(call, |index, expected_revision| {
                ControlRequest::RemoveWorkspace {
                    index,
                    expected_revision,
                }
            })
        }
        (IFACE_DISPLAYS, "GetSnapshot") => {
            expect_arity(call, 0).map(|()| ControlRequest::GetDisplays)
        }
        (IFACE_DISPLAYS, "BeginModeChange") => {
            expect_arity(call, 3)?;
            Ok(ControlRequest::BeginDisplayMode {
                output: OutputId::new(required_string(&call.args[0], "outputKey")?),
                mode: ModeId(required_u64(&call.args[1], "modeId")?),
                expected_generation: required_u64(&call.args[2], "generation")?,
                now_ms: call.now_ms,
            })
        }
        (IFACE_DISPLAYS, "Keep") => {
            expect_arity(call, 1)?;
            Ok(ControlRequest::KeepDisplayMode {
                transaction: TransactionId(required_u64(&call.args[0], "transaction")?),
            })
        }
        (IFACE_DISPLAYS, "Revert") => {
            expect_arity(call, 1)?;
            Ok(ControlRequest::RevertDisplayMode {
                transaction: TransactionId(required_u64(&call.args[0], "transaction")?),
            })
        }
        (IFACE_DISPLAYS, "SetShellScale") => {
            expect_arity(call, 3)?;
            let percent = required_u32(&call.args[1], "percent")?;
            Ok(ControlRequest::SetShellScale {
                output: OutputId::new(required_string(&call.args[0], "outputKey")?),
                percent: u16::try_from(percent).map_err(|_| bad("percent exceeds u16"))?,
                expected_revision: required_u64(&call.args[2], "expectedRevision")?,
            })
        }
        (IFACE_SHORTCUTS, "GetSnapshot") => {
            expect_arity(call, 0).map(|()| ControlRequest::GetShortcuts)
        }
        (IFACE_SHORTCUTS, "SetBinding") => {
            expect_arity(call, 3)?;
            Ok(ControlRequest::SetShortcut {
                action: required_string(&call.args[0], "action")?.to_owned(),
                binding: required_string(&call.args[1], "binding")?.to_owned(),
                expected_revision: required_u64(&call.args[2], "expectedRevision")?,
            })
        }
        (IFACE_SHORTCUTS, "ClearBinding") => {
            shortcut_action_request(call, |action, expected_revision| {
                ControlRequest::ClearShortcut {
                    action,
                    expected_revision,
                }
            })
        }
        (IFACE_SHORTCUTS, "ResetBinding") => {
            shortcut_action_request(call, |action, expected_revision| {
                ControlRequest::ResetShortcut {
                    action,
                    expected_revision,
                }
            })
        }
        (IFACE_PANELS, "GetSnapshot") => expect_arity(call, 0).map(|()| ControlRequest::GetPanels),
        (IFACE_PANELS, "SetEdge") => {
            expect_arity(call, 3)?;
            Ok(ControlRequest::SetPanelEdge {
                output: OutputId::new(required_string(&call.args[0], "outputKey")?),
                edge: parse_edge(required_string(&call.args[1], "edge")?)?,
                expected_revision: required_u64(&call.args[2], "expectedRevision")?,
            })
        }
        (IFACE_PANELS, "SetSize") => {
            expect_arity(call, 3)?;
            Ok(ControlRequest::SetPanelSize {
                output: OutputId::new(required_string(&call.args[0], "outputKey")?),
                logical_size: u16::try_from(required_u32(&call.args[1], "size")?)
                    .map_err(|_| bad("size exceeds u16"))?,
                expected_revision: required_u64(&call.args[2], "expectedRevision")?,
            })
        }
        (IFACE_PANELS, "Pin") => {
            panel_app_request(call, |app, expected_revision| ControlRequest::PinApp {
                app,
                expected_revision,
            })
        }
        (IFACE_PANELS, "Unpin") => {
            panel_app_request(call, |app, expected_revision| ControlRequest::UnpinApp {
                app,
                expected_revision,
            })
        }
        (IFACE_PANELS, "Reorder") => {
            expect_arity(call, 3)?;
            Ok(ControlRequest::ReorderTask {
                entry_id: TaskEntryId::new(required_string(&call.args[0], "entryId")?),
                index: usize::try_from(required_u32(&call.args[1], "index")?)
                    .map_err(|_| bad("index does not fit usize"))?,
                expected_revision: required_u64(&call.args[2], "expectedRevision")?,
            })
        }
        (IFACE_SESSION, "GetCapabilities") => {
            expect_arity(call, 0).map(|()| ControlRequest::GetSessionCapabilities)
        }
        (IFACE_SESSION, "Lock") => session_action(call, SessionAction::Lock),
        (IFACE_SESSION, "Logout") => session_action(call, SessionAction::Logout),
        (IFACE_SESSION, "Suspend") => session_action(call, SessionAction::Suspend),
        (IFACE_SESSION, "Reboot") => session_action(call, SessionAction::Reboot),
        (IFACE_SESSION, "Shutdown") => session_action(call, SessionAction::Shutdown),
        _ => Err(ControlError {
            name: flamewm_control_core::error_name(ErrorCode::NotFound),
            code: ErrorCode::NotFound,
            message: format!(
                "unknown Flame Control method {}.{}",
                call.interface, call.member
            ),
        }),
    }
}

/// Encode typed client request arguments for the D-Bus adapter.
pub fn encode_call(request: &ControlRequest) -> Result<WireCall, ControlError> {
    let call = match request {
        ControlRequest::Ping => WireCall::new(IFACE_ROOT, "Ping", Vec::new()),
        ControlRequest::GetVersion => WireCall::new(IFACE_ROOT, "GetVersion", Vec::new()),
        ControlRequest::GetCapabilities => WireCall::new(IFACE_ROOT, "GetCapabilities", Vec::new()),
        ControlRequest::GetSettings => WireCall::new(IFACE_SETTINGS, "GetSnapshot", Vec::new()),
        ControlRequest::ApplySettings(transaction) => match &transaction.reset_section {
            Some(section) => WireCall::new(
                IFACE_SETTINGS,
                "ResetSection",
                vec![
                    WireValue::U64(transaction.expected_revision),
                    WireValue::String(section.clone()),
                ],
            ),
            None => WireCall::new(
                IFACE_SETTINGS,
                "Apply",
                vec![
                    WireValue::U64(transaction.expected_revision),
                    WireValue::Dict(
                        transaction
                            .changes
                            .iter()
                            .filter_map(|change| {
                                change
                                    .value
                                    .as_ref()
                                    .map(|value| (change.key.clone(), setting_wire_value(value)))
                            })
                            .collect(),
                    ),
                ],
            ),
        },
        ControlRequest::GetWorkspaces => WireCall::new(IFACE_WORKSPACES, "GetSnapshot", Vec::new()),
        ControlRequest::ActivateWorkspace {
            index,
            expected_revision,
        } => workspace_call("Activate", *index, *expected_revision),
        ControlRequest::InsertWorkspaceAfter {
            index,
            expected_revision,
        } => workspace_call("InsertAfter", *index, *expected_revision),
        ControlRequest::RemoveWorkspace {
            index,
            expected_revision,
        } => workspace_call("Remove", *index, *expected_revision),
        ControlRequest::GetDisplays => WireCall::new(IFACE_DISPLAYS, "GetSnapshot", Vec::new()),
        ControlRequest::BeginDisplayMode {
            output,
            mode,
            expected_generation,
            ..
        } => WireCall::new(
            IFACE_DISPLAYS,
            "BeginModeChange",
            vec![
                WireValue::String(output.as_str().to_owned()),
                WireValue::U64(mode.0),
                WireValue::U64(*expected_generation),
            ],
        ),
        ControlRequest::KeepDisplayMode { transaction } => {
            WireCall::new(IFACE_DISPLAYS, "Keep", vec![WireValue::U64(transaction.0)])
        }
        ControlRequest::RevertDisplayMode { transaction } => WireCall::new(
            IFACE_DISPLAYS,
            "Revert",
            vec![WireValue::U64(transaction.0)],
        ),
        ControlRequest::SetShellScale {
            output,
            percent,
            expected_revision,
        } => WireCall::new(
            IFACE_DISPLAYS,
            "SetShellScale",
            vec![
                WireValue::String(output.as_str().to_owned()),
                WireValue::U32(u32::from(*percent)),
                WireValue::U64(*expected_revision),
            ],
        ),
        ControlRequest::GetShortcuts => WireCall::new(IFACE_SHORTCUTS, "GetSnapshot", Vec::new()),
        ControlRequest::SetShortcut {
            action,
            binding,
            expected_revision,
        } => WireCall::new(
            IFACE_SHORTCUTS,
            "SetBinding",
            vec![
                WireValue::String(action.clone()),
                WireValue::String(binding.clone()),
                WireValue::U64(*expected_revision),
            ],
        ),
        ControlRequest::ClearShortcut {
            action,
            expected_revision,
        } => WireCall::new(
            IFACE_SHORTCUTS,
            "ClearBinding",
            vec![
                WireValue::String(action.clone()),
                WireValue::U64(*expected_revision),
            ],
        ),
        ControlRequest::ResetShortcut {
            action,
            expected_revision,
        } => WireCall::new(
            IFACE_SHORTCUTS,
            "ResetBinding",
            vec![
                WireValue::String(action.clone()),
                WireValue::U64(*expected_revision),
            ],
        ),
        ControlRequest::GetPanels => WireCall::new(IFACE_PANELS, "GetSnapshot", Vec::new()),
        ControlRequest::SetPanelEdge {
            output,
            edge,
            expected_revision,
        } => WireCall::new(
            IFACE_PANELS,
            "SetEdge",
            vec![
                WireValue::String(output.as_str().to_owned()),
                WireValue::String(edge_name(*edge).to_owned()),
                WireValue::U64(*expected_revision),
            ],
        ),
        ControlRequest::SetPanelSize {
            output,
            logical_size,
            expected_revision,
        } => WireCall::new(
            IFACE_PANELS,
            "SetSize",
            vec![
                WireValue::String(output.as_str().to_owned()),
                WireValue::U32(u32::from(*logical_size)),
                WireValue::U64(*expected_revision),
            ],
        ),
        ControlRequest::PinApp {
            app,
            expected_revision,
        } => panel_call("Pin", app.as_str(), *expected_revision),
        ControlRequest::UnpinApp {
            app,
            expected_revision,
        } => panel_call("Unpin", app.as_str(), *expected_revision),
        ControlRequest::ReorderTask {
            entry_id,
            index,
            expected_revision,
        } => WireCall::new(
            IFACE_PANELS,
            "Reorder",
            vec![
                WireValue::String(entry_id.as_str().to_owned()),
                WireValue::U32(u32_saturating(*index)),
                WireValue::U64(*expected_revision),
            ],
        ),
        ControlRequest::GetSessionCapabilities => {
            WireCall::new(IFACE_SESSION, "GetCapabilities", Vec::new())
        }
        ControlRequest::SessionAction(action) => {
            WireCall::new(IFACE_SESSION, session_method(*action), Vec::new())
        }
        ControlRequest::ApplyShortcuts { .. } => {
            return Err(invalid("ApplyShortcuts has no D-Bus method"));
        }
    };
    Ok(call)
}

fn workspace_call(member: &str, index: usize, revision: u64) -> WireCall {
    WireCall::new(
        IFACE_WORKSPACES,
        member,
        vec![
            WireValue::U32(u32_saturating(index)),
            WireValue::U64(revision),
        ],
    )
}

fn panel_call(member: &str, app: &str, revision: u64) -> WireCall {
    WireCall::new(
        IFACE_PANELS,
        member,
        vec![WireValue::String(app.to_owned()), WireValue::U64(revision)],
    )
}

const fn session_method(action: SessionAction) -> &'static str {
    match action {
        SessionAction::Lock => "Lock",
        SessionAction::Logout => "Logout",
        SessionAction::Suspend => "Suspend",
        SessionAction::Reboot => "Reboot",
        SessionAction::Shutdown => "Shutdown",
    }
}

pub fn encode_reply(
    call: &WireCall,
    response: &ControlResponse,
) -> Result<WireReply, ControlError> {
    let values = match (call.interface.as_str(), call.member.as_str(), response) {
        (IFACE_ROOT, "Ping", ControlResponse::Pong) => vec![WireValue::String("pong".to_owned())],
        (IFACE_ROOT, "GetVersion", ControlResponse::Version(version)) => {
            vec![WireValue::String(version.to_string())]
        }
        (IFACE_ROOT, "GetCapabilities", ControlResponse::Capabilities(items)) => {
            vec![WireValue::StringArray(items.clone())]
        }
        (IFACE_SETTINGS, "GetSnapshot", ControlResponse::Settings(snapshot)) => {
            vec![WireValue::Dict(settings_dict(snapshot))]
        }
        (IFACE_SETTINGS, "Apply" | "ResetSection", ControlResponse::Settings(snapshot)) => {
            vec![WireValue::Dict(settings_dict(snapshot))]
        }
        (IFACE_WORKSPACES, "GetSnapshot", ControlResponse::Workspaces(snapshot)) => {
            vec![WireValue::Dict(workspace_dict(snapshot))]
        }
        (IFACE_DISPLAYS, "GetSnapshot", ControlResponse::Displays(snapshot)) => {
            vec![WireValue::Dict(display_dict(snapshot))]
        }
        (IFACE_DISPLAYS, "BeginModeChange", ControlResponse::Transaction(transaction)) => {
            vec![WireValue::U64(transaction.0)]
        }
        (IFACE_SHORTCUTS, "GetSnapshot", ControlResponse::Shortcuts(snapshot)) => {
            vec![WireValue::Dict(shortcut_dict(snapshot))]
        }
        (IFACE_PANELS, "GetSnapshot", ControlResponse::Panels(snapshot)) => {
            vec![WireValue::Dict(panels_dict(snapshot))]
        }
        (IFACE_SESSION, "GetCapabilities", ControlResponse::SessionCapabilities(capabilities)) => {
            vec![WireValue::StringArray(session_capability_names(
                *capabilities,
            ))]
        }
        (_, _, ControlResponse::Unit | ControlResponse::Changed(_)) => Vec::new(),
        _ => {
            return Err(ControlError {
                name: flamewm_control_core::error_name(ErrorCode::InternalFailure),
                code: ErrorCode::InternalFailure,
                message: format!("response does not match {}.{}", call.interface, call.member),
            });
        }
    };
    Ok(WireReply { values })
}

/// Decode the reply for the exact request call. This is the client-side inverse of `encode_reply`.
pub fn decode_reply(call: &WireCall, reply: &WireReply) -> Result<ControlResponse, ControlError> {
    match (call.interface.as_str(), call.member.as_str()) {
        (IFACE_ROOT, "Ping") => {
            expect_reply_arity(reply, 1)?;
            let pong = required_string(&reply.values[0], "reply")?;
            if pong != "pong" {
                return Err(invalid("Ping reply must be pong"));
            }
            Ok(ControlResponse::Pong)
        }
        (IFACE_ROOT, "GetVersion") => {
            expect_reply_arity(reply, 1)?;
            let version = Version::parse(required_string(&reply.values[0], "version")?)
                .ok_or_else(|| invalid("version is not canonical major.minor.patch"))?;
            Ok(ControlResponse::Version(version))
        }
        (IFACE_ROOT, "GetCapabilities") => {
            expect_reply_arity(reply, 1)?;
            Ok(ControlResponse::Capabilities(
                required_string_array(&reply.values[0], "capabilities")?.to_vec(),
            ))
        }
        (IFACE_SETTINGS, "GetSnapshot") => {
            expect_reply_arity(reply, 1)?;
            Ok(ControlResponse::Settings(settings_snapshot(
                required_dict(&reply.values[0], "snapshot")?,
            )?))
        }
        (IFACE_SETTINGS, "Apply" | "ResetSection") => {
            expect_reply_arity(reply, 1)?;
            Ok(ControlResponse::Settings(settings_snapshot(
                required_dict(&reply.values[0], "snapshot")?,
            )?))
        }
        (IFACE_WORKSPACES, "GetSnapshot") => {
            expect_reply_arity(reply, 1)?;
            Ok(ControlResponse::Workspaces(workspace_snapshot(
                required_dict(&reply.values[0], "snapshot")?,
            )?))
        }
        (IFACE_DISPLAYS, "GetSnapshot") => {
            expect_reply_arity(reply, 1)?;
            Ok(ControlResponse::Displays(display_snapshot(required_dict(
                &reply.values[0],
                "snapshot",
            )?)?))
        }
        (IFACE_DISPLAYS, "BeginModeChange") => {
            expect_reply_arity(reply, 1)?;
            Ok(ControlResponse::Transaction(TransactionId(required_u64(
                &reply.values[0],
                "transaction",
            )?)))
        }
        (IFACE_SHORTCUTS, "GetSnapshot") => {
            expect_reply_arity(reply, 1)?;
            Ok(ControlResponse::Shortcuts(shortcut_snapshot(
                required_dict(&reply.values[0], "snapshot")?,
            )?))
        }
        (IFACE_PANELS, "GetSnapshot") => {
            expect_reply_arity(reply, 1)?;
            Ok(ControlResponse::Panels(panels_snapshot(required_dict(
                &reply.values[0],
                "snapshot",
            )?)?))
        }
        (IFACE_SESSION, "GetCapabilities") => {
            expect_reply_arity(reply, 1)?;
            Ok(ControlResponse::SessionCapabilities(session_capabilities(
                required_string_array(&reply.values[0], "capabilities")?,
            )))
        }
        (IFACE_WORKSPACES, "Activate" | "InsertAfter" | "Remove")
        | (IFACE_DISPLAYS, "Keep" | "Revert" | "SetShellScale")
        | (IFACE_SHORTCUTS, "SetBinding" | "ClearBinding" | "ResetBinding")
        | (IFACE_PANELS, "SetEdge" | "SetSize" | "Pin" | "Unpin" | "Reorder")
        | (IFACE_SESSION, "Lock" | "Logout" | "Suspend" | "Reboot" | "Shutdown") => {
            expect_reply_arity(reply, 0)?;
            Ok(ControlResponse::Unit)
        }
        _ => Err(ControlError {
            name: flamewm_control_core::error_name(ErrorCode::NotFound),
            code: ErrorCode::NotFound,
            message: format!(
                "unknown Flame Control reply {}.{}",
                call.interface, call.member
            ),
        }),
    }
}

fn settings_snapshot(dict: &WireDict) -> Result<SettingsSnapshot, ControlError> {
    let revision = dict_u64(dict, "revision")?;
    let mut snapshot = SettingsSnapshot::new(revision);
    for (key, value) in dict {
        if key == "revision" {
            continue;
        }
        let typed = setting_value_for_key(key, value)?;
        snapshot.values.insert(key.clone(), typed);
    }
    Ok(snapshot)
}

fn workspace_snapshot(dict: &WireDict) -> Result<WorkspaceSnapshot, ControlError> {
    let count = usize::try_from(dict_u32(dict, "count")?)
        .map_err(|_| invalid("workspace count does not fit usize"))?;
    let active = usize::try_from(dict_i32(dict, "activeIndex")?)
        .map_err(|_| invalid("active workspace must be nonnegative"))?;
    let last_raw = dict_i32(dict, "lastIndex")?;
    let last_index = if last_raw < 0 {
        None
    } else {
        Some(usize::try_from(last_raw).map_err(|_| invalid("last workspace does not fit usize"))?)
    };
    let names = dict_string_array(dict, "names")?.to_vec();
    let snapshot = WorkspaceSnapshot {
        revision: dict_u64(dict, "revision")?,
        count,
        active_index: active,
        last_index,
        names,
    };
    snapshot.validate().map_err(ControlError::from)?;
    Ok(snapshot)
}

fn display_snapshot(dict: &WireDict) -> Result<DisplaySnapshot, ControlError> {
    let generation = dict_u64(dict, "generation")?;
    let outputs = dict.get("outputs").map_or(Ok(Vec::new()), |value| {
        required_array(value, "outputs")?
            .iter()
            .map(output_snapshot)
            .collect::<Result<Vec<_>, _>>()
    })?;
    let pending = if dict
        .get("hasPending")
        .and_then(WireValue::as_bool)
        .unwrap_or(false)
    {
        Some(PendingModeChange {
            transaction: TransactionId(dict_u64(dict, "pendingTx")?),
            output: OutputId::new(dict_string(dict, "pendingOutput")?),
            mode: ModeId(dict_u64(dict, "pendingMode")?),
            deadline_ms: dict_u64(dict, "pendingDeadlineMs")?,
        })
    } else {
        None
    };
    Ok(DisplaySnapshot {
        generation,
        outputs,
        pending,
    })
}

fn output_snapshot(value: &WireValue) -> Result<OutputSnapshot, ControlError> {
    let dict = required_dict(value, "output")?;
    let modes = dict.get("modes").map_or(Ok(Vec::new()), |modes| {
        required_array(modes, "modes")?
            .iter()
            .map(display_mode)
            .collect::<Result<Vec<_>, _>>()
    })?;
    let scale = dict_u32(dict, "scale")?;
    Ok(OutputSnapshot {
        id: OutputId::new(dict_string(dict, "id")?),
        connector: dict_string(dict, "connector")?.to_owned(),
        edid_identity: dict_string(dict, "edid")?.to_owned(),
        connected: dict_bool(dict, "connected")?,
        primary: dict_bool(dict, "primary")?,
        geometry: Rect::new(
            dict_i32(dict, "x")?,
            dict_i32(dict, "y")?,
            dict_i32(dict, "width")?,
            dict_i32(dict, "height")?,
        ),
        current_mode: ModeId(dict_u64(dict, "mode")?),
        modes,
        shell_scale_percent: u16::try_from(scale).map_err(|_| invalid("scale exceeds u16"))?,
    })
}

fn display_mode(value: &WireValue) -> Result<DisplayMode, ControlError> {
    let dict = required_dict(value, "mode")?;
    Ok(DisplayMode {
        id: ModeId(dict_u64(dict, "id")?),
        resolution: Size::new(dict_i32(dict, "width")?, dict_i32(dict, "height")?),
        refresh_millihz: dict_i32(dict, "refreshMillihz")?,
        preferred: dict_bool(dict, "preferred")?,
    })
}

fn shortcut_snapshot(dict: &WireDict) -> Result<ShortcutSnapshot, ControlError> {
    let revision = dict_u64(dict, "revision")?;
    let mut bindings = BTreeMap::new();
    for (key, value) in dict {
        if key == "revision" {
            continue;
        }
        bindings.insert(
            key.clone(),
            KeyBinding::assigned(required_string(value, key)?),
        );
    }
    Ok(ShortcutSnapshot { revision, bindings })
}

fn panels_snapshot(dict: &WireDict) -> Result<PanelsSnapshot, ControlError> {
    let panels = dict.get("panels").map_or(Ok(Vec::new()), |value| {
        required_array(value, "panels")?
            .iter()
            .map(panel_snapshot)
            .collect::<Result<Vec<_>, _>>()
    })?;
    let tasks = dict.get("tasks").map_or(Ok(Vec::new()), |value| {
        required_array(value, "tasks")?
            .iter()
            .map(task_entry)
            .collect::<Result<Vec<_>, _>>()
    })?;
    let pinned_apps = match dict.get("pinnedApps") {
        Some(value) => required_string_array(value, "pinnedApps")?
            .iter()
            .map(DesktopAppId::new)
            .collect(),
        None => Vec::new(),
    };
    Ok(PanelsSnapshot {
        revision: dict_u64(dict, "revision")?,
        panels,
        tasks,
        pinned_apps,
    })
}

fn panel_snapshot(value: &WireValue) -> Result<PanelSnapshot, ControlError> {
    let dict = required_dict(value, "panel")?;
    let size = dict_u32(dict, "size")?;
    Ok(PanelSnapshot {
        output: OutputId::new(dict_string(dict, "output")?),
        edge: parse_edge(dict_string(dict, "edge")?)?,
        logical_size: u16::try_from(size).map_err(|_| invalid("panel size exceeds u16"))?,
        visible: dict_bool(dict, "visible")?,
        geometry: Rect::new(
            dict_i32(dict, "x")?,
            dict_i32(dict, "y")?,
            dict_i32(dict, "width")?,
            dict_i32(dict, "height")?,
        ),
    })
}

fn task_entry(value: &WireValue) -> Result<TaskEntry, ControlError> {
    let dict = required_dict(value, "task")?;
    let kind = match dict_string(dict, "kind")? {
        "pin" => {
            let window = if dict.contains_key("windowId") {
                Some(WindowRef::new(
                    dict_u64(dict, "windowId")?,
                    dict_u64(dict, "windowGeneration")?,
                ))
            } else {
                None
            };
            TaskEntryKind::PinnedSlot { window }
        }
        "window" => TaskEntryKind::Window {
            window: WindowRef::new(
                dict_u64(dict, "windowId")?,
                dict_u64(dict, "windowGeneration")?,
            ),
        },
        _ => return Err(invalid("task kind must be pin or window")),
    };
    Ok(TaskEntry {
        id: TaskEntryId::new(dict_string(dict, "id")?),
        app_id: DesktopAppId::new(dict_string(dict, "appId")?),
        kind,
        order_index: usize::try_from(dict_u32(dict, "order")?)
            .map_err(|_| invalid("task order does not fit usize"))?,
    })
}

fn session_capabilities(names: &[String]) -> SessionCapabilities {
    SessionCapabilities {
        lock: names.iter().any(|name| name == "lock"),
        logout: names.iter().any(|name| name == "logout"),
        suspend: names.iter().any(|name| name == "suspend"),
        reboot: names.iter().any(|name| name == "reboot"),
        shutdown: names.iter().any(|name| name == "shutdown"),
    }
}

fn expect_reply_arity(reply: &WireReply, expected: usize) -> Result<(), ControlError> {
    if reply.values.len() == expected {
        Ok(())
    } else {
        Err(invalid(&format!("reply expects {expected} values")))
    }
}

fn required_array<'a>(value: &'a WireValue, name: &str) -> Result<&'a [WireValue], ControlError> {
    value
        .as_array()
        .ok_or_else(|| invalid(&format!("{name} must be array")))
}

fn required_string_array<'a>(
    value: &'a WireValue,
    name: &str,
) -> Result<&'a [String], ControlError> {
    value
        .as_string_array()
        .ok_or_else(|| invalid(&format!("{name} must be string array")))
}

fn dict_value<'a>(dict: &'a WireDict, key: &str) -> Result<&'a WireValue, ControlError> {
    dict.get(key)
        .ok_or_else(|| invalid(&format!("snapshot missing {key}")))
}

fn dict_u64(dict: &WireDict, key: &str) -> Result<u64, ControlError> {
    required_u64(dict_value(dict, key)?, key)
}
fn dict_u32(dict: &WireDict, key: &str) -> Result<u32, ControlError> {
    required_u32(dict_value(dict, key)?, key)
}
fn dict_i32(dict: &WireDict, key: &str) -> Result<i32, ControlError> {
    dict_value(dict, key)?
        .as_i32()
        .ok_or_else(|| invalid(&format!("{key} must be int32")))
}
fn dict_bool(dict: &WireDict, key: &str) -> Result<bool, ControlError> {
    dict_value(dict, key)?
        .as_bool()
        .ok_or_else(|| invalid(&format!("{key} must be bool")))
}
fn dict_string<'a>(dict: &'a WireDict, key: &str) -> Result<&'a str, ControlError> {
    required_string(dict_value(dict, key)?, key)
}
fn dict_string_array<'a>(dict: &'a WireDict, key: &str) -> Result<&'a [String], ControlError> {
    required_string_array(dict_value(dict, key)?, key)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlSignal {
    SettingsChanged {
        revision: u64,
        keys: Vec<String>,
    },
    WorkspacesChanged {
        revision: u64,
    },
    TopologyChanged {
        generation: u64,
    },
    DisplayTransactionChanged {
        transaction: u64,
        state: String,
        deadline_ms: u64,
    },
    ShortcutsChanged {
        revision: u64,
    },
    PanelsChanged {
        revision: u64,
    },
}

#[must_use]
pub fn settings_dict(snapshot: &SettingsSnapshot) -> WireDict {
    let mut out = WireDict::new();
    out.insert("revision".to_owned(), WireValue::U64(snapshot.revision));
    for (key, value) in &snapshot.values {
        out.insert(key.clone(), setting_wire_value(value));
    }
    out
}

#[must_use]
pub fn workspace_dict(snapshot: &WorkspaceSnapshot) -> WireDict {
    let mut out = WireDict::new();
    out.insert("revision".to_owned(), WireValue::U64(snapshot.revision));
    out.insert(
        "count".to_owned(),
        WireValue::U32(u32_saturating(snapshot.count)),
    );
    out.insert(
        "activeIndex".to_owned(),
        WireValue::I32(i32_saturating(snapshot.active_index)),
    );
    out.insert(
        "lastIndex".to_owned(),
        WireValue::I32(snapshot.last_index.map_or(-1, i32_saturating)),
    );
    out.insert(
        "names".to_owned(),
        WireValue::StringArray(snapshot.names.clone()),
    );
    out
}

#[must_use]
pub fn display_dict(snapshot: &DisplaySnapshot) -> WireDict {
    let mut out = WireDict::new();
    out.insert("generation".to_owned(), WireValue::U64(snapshot.generation));
    out.insert(
        "outputCount".to_owned(),
        WireValue::U32(u32_saturating(snapshot.outputs.len())),
    );
    out.insert(
        "hasPending".to_owned(),
        WireValue::Bool(snapshot.pending.is_some()),
    );
    if let Some(pending) = &snapshot.pending {
        out.insert(
            "pendingTx".to_owned(),
            WireValue::U64(pending.transaction.0),
        );
        out.insert(
            "pendingDeadlineMs".to_owned(),
            WireValue::U64(pending.deadline_ms),
        );
        out.insert(
            "pendingOutput".to_owned(),
            WireValue::String(pending.output.as_str().to_owned()),
        );
        out.insert("pendingMode".to_owned(), WireValue::U64(pending.mode.0));
    }
    out.insert(
        "outputs".to_owned(),
        WireValue::Array(snapshot.outputs.iter().map(output_wire_value).collect()),
    );
    out
}

#[must_use]
pub fn shortcut_dict(snapshot: &ShortcutSnapshot) -> WireDict {
    let mut out = WireDict::new();
    out.insert("revision".to_owned(), WireValue::U64(snapshot.revision));
    for (action, binding) in &snapshot.bindings {
        out.insert(
            action.clone(),
            WireValue::String(binding.0.clone().unwrap_or_default()),
        );
    }
    out
}

#[must_use]
pub fn panels_dict(snapshot: &PanelsSnapshot) -> WireDict {
    let mut out = WireDict::new();
    out.insert("revision".to_owned(), WireValue::U64(snapshot.revision));
    out.insert(
        "panelCount".to_owned(),
        WireValue::U32(u32_saturating(snapshot.panels.len())),
    );
    out.insert(
        "taskCount".to_owned(),
        WireValue::U32(u32_saturating(snapshot.tasks.len())),
    );
    out.insert(
        "pinnedCount".to_owned(),
        WireValue::U32(u32_saturating(snapshot.pinned_apps.len())),
    );
    out.insert(
        "panels".to_owned(),
        WireValue::Array(snapshot.panels.iter().map(panel_wire_value).collect()),
    );
    out.insert(
        "tasks".to_owned(),
        WireValue::Array(
            snapshot
                .tasks
                .iter()
                .map(|entry| {
                    let mut item = WireDict::new();
                    item.insert(
                        "id".to_owned(),
                        WireValue::String(entry.id.as_str().to_owned()),
                    );
                    item.insert(
                        "appId".to_owned(),
                        WireValue::String(entry.app_id.as_str().to_owned()),
                    );
                    item.insert(
                        "order".to_owned(),
                        WireValue::U32(u32_saturating(entry.order_index)),
                    );
                    match &entry.kind {
                        TaskEntryKind::PinnedSlot { window } => {
                            item.insert("kind".to_owned(), WireValue::String("pin".to_owned()));
                            if let Some(window) = window {
                                item.insert("windowId".to_owned(), WireValue::U64(window.id));
                                item.insert(
                                    "windowGeneration".to_owned(),
                                    WireValue::U64(window.generation),
                                );
                            }
                        }
                        TaskEntryKind::Window { window } => {
                            item.insert("kind".to_owned(), WireValue::String("window".to_owned()));
                            item.insert("windowId".to_owned(), WireValue::U64(window.id));
                            item.insert(
                                "windowGeneration".to_owned(),
                                WireValue::U64(window.generation),
                            );
                        }
                    }
                    WireValue::Dict(item)
                })
                .collect(),
        ),
    );
    out.insert(
        "pinnedApps".to_owned(),
        WireValue::StringArray(
            snapshot
                .pinned_apps
                .iter()
                .map(|app| app.as_str().to_owned())
                .collect(),
        ),
    );
    out
}

#[must_use]
pub fn session_capability_names(capabilities: SessionCapabilities) -> Vec<String> {
    let mut out = Vec::new();
    if capabilities.lock {
        out.push("lock".to_owned());
    }
    if capabilities.logout {
        out.push("logout".to_owned());
    }
    if capabilities.suspend {
        out.push("suspend".to_owned());
    }
    if capabilities.reboot {
        out.push("reboot".to_owned());
    }
    if capabilities.shutdown {
        out.push("shutdown".to_owned());
    }
    out
}

fn output_wire_value(output: &OutputSnapshot) -> WireValue {
    let mut item = WireDict::new();
    item.insert(
        "id".to_owned(),
        WireValue::String(output.id.as_str().to_owned()),
    );
    item.insert(
        "connector".to_owned(),
        WireValue::String(output.connector.clone()),
    );
    item.insert(
        "edid".to_owned(),
        WireValue::String(output.edid_identity.clone()),
    );
    item.insert("connected".to_owned(), WireValue::Bool(output.connected));
    item.insert("primary".to_owned(), WireValue::Bool(output.primary));
    item.insert("x".to_owned(), WireValue::I32(output.geometry.x));
    item.insert("y".to_owned(), WireValue::I32(output.geometry.y));
    item.insert("width".to_owned(), WireValue::I32(output.geometry.width));
    item.insert("height".to_owned(), WireValue::I32(output.geometry.height));
    item.insert("mode".to_owned(), WireValue::U64(output.current_mode.0));
    item.insert(
        "scale".to_owned(),
        WireValue::U32(u32::from(output.shell_scale_percent)),
    );
    item.insert(
        "modes".to_owned(),
        WireValue::Array(
            output
                .modes
                .iter()
                .map(|mode| {
                    let mut value = WireDict::new();
                    value.insert("id".to_owned(), WireValue::U64(mode.id.0));
                    value.insert("width".to_owned(), WireValue::I32(mode.resolution.width));
                    value.insert("height".to_owned(), WireValue::I32(mode.resolution.height));
                    value.insert(
                        "refreshMillihz".to_owned(),
                        WireValue::I32(mode.refresh_millihz),
                    );
                    value.insert("preferred".to_owned(), WireValue::Bool(mode.preferred));
                    WireValue::Dict(value)
                })
                .collect(),
        ),
    );
    WireValue::Dict(item)
}

fn panel_wire_value(panel: &PanelSnapshot) -> WireValue {
    let mut item = WireDict::new();
    item.insert(
        "output".to_owned(),
        WireValue::String(panel.output.as_str().to_owned()),
    );
    item.insert(
        "edge".to_owned(),
        WireValue::String(edge_name(panel.edge).to_owned()),
    );
    item.insert(
        "size".to_owned(),
        WireValue::U32(u32::from(panel.logical_size)),
    );
    item.insert("visible".to_owned(), WireValue::Bool(panel.visible));
    item.insert("x".to_owned(), WireValue::I32(panel.geometry.x));
    item.insert("y".to_owned(), WireValue::I32(panel.geometry.y));
    item.insert("width".to_owned(), WireValue::I32(panel.geometry.width));
    item.insert("height".to_owned(), WireValue::I32(panel.geometry.height));
    WireValue::Dict(item)
}

fn setting_wire_value(value: &SettingValue) -> WireValue {
    match value {
        SettingValue::Boolean(value) => WireValue::Bool(*value),
        SettingValue::Integer(value) => WireValue::I64(*value),
        SettingValue::Percent(value) => WireValue::U32(u32::from(*value)),
        SettingValue::Rgb(value) => WireValue::U32(*value),
        SettingValue::Text(value) => WireValue::String(value.clone()),
    }
}

fn setting_value(value: &WireValue) -> Result<SettingValue, ControlError> {
    let invalid = || ControlError {
        name: flamewm_control_core::error_name(ErrorCode::InvalidArgument),
        code: ErrorCode::InvalidArgument,
        message: "settings change contains unsupported variant type".to_owned(),
    };
    match value {
        WireValue::Bool(value) => Ok(SettingValue::Boolean(*value)),
        WireValue::I64(value) => Ok(SettingValue::Integer(*value)),
        WireValue::I32(value) => Ok(SettingValue::Integer(i64::from(*value))),
        WireValue::U32(value) => Ok(SettingValue::Integer(i64::from(*value))),
        WireValue::String(value) => Ok(SettingValue::Text(value.clone())),
        _ => Err(invalid()),
    }
}

fn setting_value_for_key(key: &str, value: &WireValue) -> Result<SettingValue, ControlError> {
    match key {
        "DesktopSelectionFillOpacity" | "WindowSnapPreviewFillOpacity" | "taskbarOpacity" => {
            let percent = required_u32(value, key)?;
            Ok(SettingValue::Percent(
                u8::try_from(percent).map_err(|_| invalid("percentage exceeds u8"))?,
            ))
        }
        _ => setting_value(value),
    }
}

fn workspace_index_request(
    call: &WireCall,
    build: impl FnOnce(usize, u64) -> ControlRequest,
) -> Result<ControlRequest, ControlError> {
    expect_arity(call, 2)?;
    let index = usize::try_from(required_u32(&call.args[0], "index")?)
        .map_err(|_| invalid("index does not fit usize"))?;
    Ok(build(
        index,
        required_u64(&call.args[1], "expectedRevision")?,
    ))
}

fn shortcut_action_request(
    call: &WireCall,
    build: impl FnOnce(String, u64) -> ControlRequest,
) -> Result<ControlRequest, ControlError> {
    expect_arity(call, 2)?;
    Ok(build(
        required_string(&call.args[0], "action")?.to_owned(),
        required_u64(&call.args[1], "expectedRevision")?,
    ))
}

fn panel_app_request(
    call: &WireCall,
    build: impl FnOnce(DesktopAppId, u64) -> ControlRequest,
) -> Result<ControlRequest, ControlError> {
    expect_arity(call, 2)?;
    Ok(build(
        DesktopAppId::new(required_string(&call.args[0], "appId")?),
        required_u64(&call.args[1], "expectedRevision")?,
    ))
}

fn session_action(call: &WireCall, action: SessionAction) -> Result<ControlRequest, ControlError> {
    expect_arity(call, 0)?;
    Ok(ControlRequest::SessionAction(action))
}

fn expect_arity(call: &WireCall, expected: usize) -> Result<(), ControlError> {
    if call.args.len() == expected {
        Ok(())
    } else {
        Err(invalid(&format!(
            "{} expects {expected} arguments",
            call.member
        )))
    }
}

fn required_u64(value: &WireValue, name: &str) -> Result<u64, ControlError> {
    value
        .as_u64()
        .ok_or_else(|| invalid(&format!("{name} must be uint64")))
}

fn required_u32(value: &WireValue, name: &str) -> Result<u32, ControlError> {
    value
        .as_u32()
        .ok_or_else(|| invalid(&format!("{name} must be uint32")))
}

fn required_string<'a>(value: &'a WireValue, name: &str) -> Result<&'a str, ControlError> {
    value
        .as_str()
        .ok_or_else(|| invalid(&format!("{name} must be string")))
}

fn required_dict<'a>(value: &'a WireValue, name: &str) -> Result<&'a WireDict, ControlError> {
    value
        .as_dict()
        .ok_or_else(|| invalid(&format!("{name} must be a{{sv}}")))
}

fn parse_edge(value: &str) -> Result<PanelEdge, ControlError> {
    match value.to_ascii_lowercase().as_str() {
        "bottom" => Ok(PanelEdge::Bottom),
        "top" => Ok(PanelEdge::Top),
        "left" => Ok(PanelEdge::Left),
        "right" => Ok(PanelEdge::Right),
        _ => Err(invalid("edge must be bottom/top/left/right")),
    }
}

#[must_use]
pub const fn edge_name(edge: PanelEdge) -> &'static str {
    match edge {
        PanelEdge::Bottom => "bottom",
        PanelEdge::Top => "top",
        PanelEdge::Left => "left",
        PanelEdge::Right => "right",
    }
}

fn invalid(message: &str) -> ControlError {
    ControlError {
        name: flamewm_control_core::error_name(ErrorCode::InvalidArgument),
        code: ErrorCode::InvalidArgument,
        message: message.to_owned(),
    }
}

fn u32_saturating(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn i32_saturating(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flamewm_api::settings::SettingValue;

    #[test]
    fn settings_apply_decodes_typed_variants_without_stringly_typed_core() {
        let mut changes = WireDict::new();
        changes.insert("PanelSize".to_owned(), WireValue::I64(44));
        changes.insert("Dark".to_owned(), WireValue::Bool(true));
        changes.insert("taskbarOpacity".to_owned(), WireValue::U32(83));
        let call = WireCall::new(
            IFACE_SETTINGS,
            "Apply",
            vec![WireValue::U64(7), WireValue::Dict(changes)],
        );
        let ControlRequest::ApplySettings(transaction) = decode_call(&call).expect("valid call")
        else {
            panic!("wrong request");
        };
        assert_eq!(transaction.expected_revision, 7);
        assert_eq!(transaction.changes.len(), 3);
        assert!(
            transaction
                .changes
                .iter()
                .any(|change| change.key == "taskbarOpacity"
                    && change.value == Some(SettingValue::Percent(83)))
        );
    }

    #[test]
    fn settings_snapshot_preserves_real_variant_types() {
        let mut snapshot = SettingsSnapshot::new(9);
        snapshot
            .values
            .insert("Dark".to_owned(), SettingValue::Boolean(true));
        snapshot
            .values
            .insert("AccentColor".to_owned(), SettingValue::Rgb(0xEF4048));
        let dict = settings_dict(&snapshot);
        assert_eq!(dict.get("revision"), Some(&WireValue::U64(9)));
        assert_eq!(dict.get("Dark"), Some(&WireValue::Bool(true)));
        assert_eq!(dict.get("AccentColor"), Some(&WireValue::U32(0xEF4048)));
    }

    #[test]
    fn begin_display_mode_uses_reactor_clock_not_untrusted_bus_argument() {
        let call = WireCall::new(
            IFACE_DISPLAYS,
            "BeginModeChange",
            vec![
                WireValue::String("HDMI-1".to_owned()),
                WireValue::U64(42),
                WireValue::U64(5),
            ],
        )
        .with_now_ms(12_345);
        assert_eq!(
            decode_call(&call).expect("valid"),
            ControlRequest::BeginDisplayMode {
                output: OutputId::new("HDMI-1"),
                mode: ModeId(42),
                expected_generation: 5,
                now_ms: 12_345,
            }
        );
    }

    #[test]
    fn v8_reorder_decodes_stable_entry_identity() {
        let call = WireCall::new(
            IFACE_PANELS,
            "Reorder",
            vec![
                WireValue::String("win:99:2".to_owned()),
                WireValue::U32(1),
                WireValue::U64(8),
            ],
        );
        assert_eq!(
            decode_call(&call).expect("valid"),
            ControlRequest::ReorderTask {
                entry_id: TaskEntryId::new("win:99:2"),
                index: 1,
                expected_revision: 8,
            }
        );
    }

    #[test]
    fn reset_section_reuses_atomic_settings_transaction() {
        let call = WireCall::new(
            IFACE_SETTINGS,
            "ResetSection",
            vec![
                WireValue::U64(3),
                WireValue::String("appearance".to_owned()),
            ],
        );
        let ControlRequest::ApplySettings(transaction) = decode_call(&call).expect("valid") else {
            panic!("wrong request");
        };
        assert_eq!(transaction.reset_section.as_deref(), Some("appearance"));
        assert!(transaction.changes.is_empty());
    }

    #[test]
    fn settings_mutation_reply_round_trips_full_authoritative_snapshot() {
        let call = WireCall::new(IFACE_SETTINGS, "Apply", Vec::new());
        let mut source = SettingsSnapshot::new(4);
        source
            .values
            .insert("fontBold".to_owned(), SettingValue::Boolean(true));
        let reply = encode_reply(&call, &ControlResponse::Settings(source.clone()))
            .expect("matching response");
        assert_eq!(
            decode_reply(&call, &reply).expect("decode"),
            ControlResponse::Settings(source)
        );
    }

    #[test]
    fn workspace_snapshot_round_trips_through_client_codec() {
        let call = WireCall::new(IFACE_WORKSPACES, "GetSnapshot", Vec::new());
        let source = WorkspaceSnapshot {
            revision: 12,
            count: 3,
            active_index: 1,
            last_index: Some(0),
            names: vec!["1".to_owned(), "2".to_owned(), "3".to_owned()],
        };
        let reply =
            encode_reply(&call, &ControlResponse::Workspaces(source.clone())).expect("encode");
        assert_eq!(
            decode_reply(&call, &reply).expect("decode"),
            ControlResponse::Workspaces(source)
        );
    }

    #[test]
    fn settings_reply_decoder_restores_percent_semantics_for_known_keys() {
        let call = WireCall::new(IFACE_SETTINGS, "GetSnapshot", Vec::new());
        let mut source = SettingsSnapshot::new(2);
        source
            .values
            .insert("taskbarOpacity".to_owned(), SettingValue::Percent(83));
        let reply =
            encode_reply(&call, &ControlResponse::Settings(source.clone())).expect("encode");
        assert_eq!(
            decode_reply(&call, &reply).expect("decode"),
            ControlResponse::Settings(source)
        );
    }
}
