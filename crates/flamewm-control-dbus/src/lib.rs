//! D-Bus transport glue for the transport-neutral Flame Control protocol.

use std::ffi::CString;
use std::time::Duration;

use dbus::arg::messageitem::{MessageItem, MessageItemArray};
use dbus::strings::ErrorName;
use dbus::{Message, MessageType, Signature};
use flamewm_api::ports::EnginePorts;
use flamewm_api::{ErrorCode, FlameError, FlameResult};
use flamewm_control_core::{ControlError, ControlRequest, Dispatcher, OBJECT_PATH};
use flamewm_control_wire::{
    ControlSignal, WireCall, WireReply, WireValue, decode_call, decode_reply, encode_call,
    encode_reply, encode_signal,
};
use flamewm_dbus_reactor::{BusKind, BusPump, MessageDisposition};
use flamewm_platform::host::PlatformHost;

pub struct ControlServer {
    pump: BusPump,
    dispatcher: Dispatcher,
}

impl ControlServer {
    pub fn connect(pump: BusPump) -> FlameResult<Self> {
        pump.own_name(flamewm_control_core::BUS_NAME)?;
        Ok(Self {
            pump,
            dispatcher: Dispatcher::new(),
        })
    }

    pub fn connect_session() -> FlameResult<Self> {
        Self::connect(BusPump::connect(BusKind::Session)?)
    }

    pub fn on_ready<E: EnginePorts>(
        &self,
        host: &mut PlatformHost<E>,
        now_ms: u64,
    ) -> FlameResult<usize> {
        let pump = &self.pump;
        let dispatcher = &self.dispatcher;
        pump.on_ready(|message| {
            if message.msg_type() != MessageType::MethodCall
                || message.path().map(|path| path.to_string()) != Some(OBJECT_PATH.to_owned())
            {
                return MessageDisposition::Unhandled;
            }
            let result = handle_call(dispatcher, host, message, now_ms).and_then(|reply| {
                if let Some(reply) = reply {
                    pump.send(reply)?;
                }
                Ok(())
            });
            if let Err(error) = result {
                if !message.get_no_reply() {
                    if let Ok(name) = ErrorName::new(error_name(&error)) {
                        if let Ok(text) = CString::new(error.to_string()) {
                            let _ = pump.send(message.error(&name, &text));
                        }
                    }
                }
            }
            MessageDisposition::Handled
        })
    }

    #[must_use]
    pub fn watch(&self) -> flamewm_dbus_reactor::BusWatch {
        self.pump.watch()
    }

    pub fn emit_signal(&self, signal: &ControlSignal) -> FlameResult<()> {
        let signal = encode_signal(signal);
        let mut message = Message::new_signal(OBJECT_PATH, &signal.interface, &signal.member)
            .map_err(|error| invalid(&error))?;
        message.append_items(
            &signal
                .args
                .iter()
                .map(to_item)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| invalid(&error))?,
        );
        self.pump.send(message).map(|_| ())
    }
}

fn handle_call<E: EnginePorts>(
    dispatcher: &Dispatcher,
    host: &mut PlatformHost<E>,
    message: &Message,
    now_ms: u64,
) -> FlameResult<Option<Message>> {
    let interface = message
        .interface()
        .ok_or_else(|| invalid("missing interface"))?
        .to_string();
    let member = message
        .member()
        .ok_or_else(|| invalid("missing member"))?
        .to_string();
    let args = message
        .get_items()
        .into_iter()
        .map(from_item)
        .collect::<Result<Vec<_>, _>>()?;
    let call = WireCall::new(interface, member, args).with_now_ms(now_ms);
    let request = decode_call(&call).map_err(to_flame_error)?;
    let response = dispatcher.dispatch(host, request).map_err(to_flame_error)?;
    if message.get_no_reply() {
        return Ok(None);
    }
    let reply = encode_reply(&call, &response).map_err(to_flame_error)?;
    let mut output = message.method_return();
    output.append_items(
        &reply
            .values
            .iter()
            .map(to_item)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| invalid(&error))?,
    );
    Ok(Some(output))
}

pub struct ControlClient {
    pump: BusPump,
}

impl ControlClient {
    pub fn connect(kind: BusKind) -> FlameResult<Self> {
        Ok(Self {
            pump: BusPump::connect(kind)?,
        })
    }

    pub fn call(
        &self,
        request: &ControlRequest,
    ) -> Result<flamewm_control_core::ControlResponse, ControlError> {
        let call = encode_call(request)?;
        let message = Message::new_method_call(
            flamewm_control_core::BUS_NAME,
            OBJECT_PATH,
            &call.interface,
            &call.member,
        )
        .map_err(|error| invalid(&error))?;
        let mut message = message;
        message.append_items(
            &call
                .args
                .iter()
                .map(to_item)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| invalid(&error))?,
        );
        let reply = self
            .pump
            .send_with_reply(message, Duration::from_secs(10))
            .map_err(|error| ControlError {
                name: flamewm_control_core::error_name(ErrorCode::Unavailable),
                code: ErrorCode::Unavailable,
                message: error.to_string(),
            })?;
        let values = reply
            .get_items()
            .into_iter()
            .map(from_item)
            .collect::<Result<Vec<_>, _>>()?;
        decode_reply(&call, &WireReply { values })
    }
}

fn to_item(value: &WireValue) -> Result<MessageItem, String> {
    match value {
        WireValue::Bool(v) => Ok(MessageItem::Bool(*v)),
        WireValue::U32(v) => Ok(MessageItem::UInt32(*v)),
        WireValue::U64(v) => Ok(MessageItem::UInt64(*v)),
        WireValue::I32(v) => Ok(MessageItem::Int32(*v)),
        WireValue::I64(v) => Ok(MessageItem::Int64(*v)),
        WireValue::String(v) => Ok(MessageItem::Str(v.clone())),
        WireValue::StringArray(v) => array(
            v.iter().map(|v| MessageItem::Str(v.clone())).collect(),
            "as",
        ),
        WireValue::Array(v) => array(
            v.iter().map(to_item).collect::<Result<Vec<_>, _>>()?,
            "aa{sv}",
        ),
        WireValue::Dict(v) => {
            MessageItem::from_dict(v.iter().map(|(k, v)| to_item(v).map(|v| (k.clone(), v))))
                .map_err(|error| error.to_string())
        }
    }
}

fn array(values: Vec<MessageItem>, signature: &str) -> Result<MessageItem, String> {
    MessageItemArray::new(
        values,
        Signature::new(signature).map_err(|error| error.to_string())?,
    )
    .map(MessageItem::Array)
    .map_err(|error| format!("{error:?}"))
}

fn from_item(item: MessageItem) -> Result<WireValue, FlameError> {
    let signature = item.signature().to_string();
    match item {
        MessageItem::Variant(value) => from_item(*value),
        MessageItem::Bool(v) => Ok(WireValue::Bool(v)),
        MessageItem::UInt32(v) => Ok(WireValue::U32(v)),
        MessageItem::UInt64(v) => Ok(WireValue::U64(v)),
        MessageItem::Int32(v) => Ok(WireValue::I32(v)),
        MessageItem::Int64(v) => Ok(WireValue::I64(v)),
        MessageItem::Str(v) => Ok(WireValue::String(v)),
        MessageItem::Array(values) => {
            let string_array = signature == "as";
            let values = values
                .into_vec()
                .into_iter()
                .map(from_item)
                .collect::<Result<Vec<_>, _>>()?;
            if string_array {
                Ok(WireValue::StringArray(
                    values
                        .into_iter()
                        .map(|v| {
                            if let WireValue::String(v) = v {
                                v
                            } else {
                                unreachable!()
                            }
                        })
                        .collect(),
                ))
            } else {
                Ok(WireValue::Array(values))
            }
        }
        MessageItem::Dict(values) => {
            let mut out = std::collections::BTreeMap::new();
            for (key, value) in values.into_vec() {
                let MessageItem::Str(key) = key else {
                    return Err(invalid("D-Bus dictionary key must be string"));
                };
                out.insert(key, from_item(value)?);
            }
            Ok(WireValue::Dict(out))
        }
        _ => Err(invalid("unsupported D-Bus value")),
    }
}

fn invalid(message: &str) -> FlameError {
    FlameError::new(ErrorCode::InvalidArgument, message)
}
fn to_flame_error(error: ControlError) -> FlameError {
    FlameError::new(error.code, error.message)
}
fn error_name(error: &FlameError) -> &'static str {
    flamewm_control_core::error_name(error.code)
}
