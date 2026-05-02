pub use clm_core::event::{
    DispatchDescriptor, Event, EventKind, EventResult, Plugin, PluginId, PluginRegistrar,
    PropertyKey, RawEventHandler, SortKey, Subscription,
};
pub use clm_core::registry::{
    RawMutServiceHandler, RawServiceHandler, Resolver, Service, add_plugin, dispatch_next,
    emit_event, execute_command, park_until_event, query_service, quit, register_command,
    register_resolver,
};
pub use clm_core::runtime::{emit_event_async, init_async_runtime, sleep, spawn_async};
pub use clm_core::value::{Value, ValueConvertError, from_value, to_value};

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

pub fn query_set_mode(mode: Mode) {
    let _ = query_service("modal.set_mode", &[to_value(&mode).unwrap()]);
}
pub fn query_cursor_move(cursor_move: &CursorMove) {
    let _ = query_service("modal.cursor_move", &[to_value(cursor_move).unwrap()]);
}
pub fn query_edit(edit: &EditAction) {
    let _ = query_service("modal.edit", &[to_value(edit).unwrap()]);
}
pub fn query_command_line(cmd_action: &CommandLineAction) {
    let _ = query_service(
        "modal.command_line_action",
        &[to_value(cmd_action).unwrap()],
    );
}
