pub use clm_core::editor::PluginContext;
pub use clm_core::event::{
    DispatchDescriptor, Event, EventKind, EventResult, Plugin, PluginId, PluginRegistrar,
    PropertyKey, RawEventHandler, SortKey, Subscription,
};
pub use clm_core::registry::{
    RawMutServiceHandler, RawServiceHandler, Service, add_plugin, emit_event, execute_command,
    query_service, register_command, register_resolver,
};
pub use clm_core::value::{Value, from_value, to_value};

use crate::data::*;

#[inline(always)]
pub fn get_arg<T: TryFrom<Value>>(args: &[Value], index: usize) -> Result<T, String> {
    let Some(arg) = args.get(index) else {
        return Err("arg not found".to_string());
    };
    arg.clone()
        .try_into()
        .map_err(|_| "invalid arg".to_string())
}

pub fn emit_set_mode(mode: Mode) {
    emit_event(
        Event {
            kind: EventKind("set_mode".to_string()),
            data: to_value(&mode).unwrap(),
        },
        DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
    );
}
pub fn emit_cursor_move(cursor_move: &CursorMove) {
    emit_event(
        Event {
            kind: EventKind("cursor_move".to_string()),
            data: to_value(cursor_move).unwrap(),
        },
        DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
    );
}
pub fn emit_buffer_op(buffer_op: &BufferOp) {
    emit_event(
        Event {
            kind: EventKind("buffer_op".to_string()),
            data: to_value(buffer_op).unwrap(),
        },
        DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
    );
}
pub fn emit_edit(edit: &EditAction) {
    emit_event(
        Event {
            kind: EventKind("edit".to_string()),
            data: to_value(edit).unwrap(),
        },
        DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
    );
}
pub fn emit_command_line(cmd_action: &CommandLineAction) {
    emit_event(
        Event {
            kind: EventKind("command_line_action".to_string()),
            data: to_value(cmd_action).unwrap(),
        },
        DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
    );
}
