pub use clm_core::editor::{CursorState, Mode, PluginContext};
pub use clm_core::event::data::{
    BufferId, CommandLineAction, CursorMove, EditAction, EventData,
};
pub use clm_core::event::{
    DispatchDescriptor, Event, EventKind, EventResult, Plugin, PluginId,
    PluginRegistrar, PropertyKey, RawEventHandler, SortKey, Subscription,
};
pub use clm_core::registry::{
    RawMutServiceHandler, RawServiceHandler, Service, add_plugin, emit_event,
    execute_command, query_service, register_command, register_resolver,
};
pub use clm_core::value::Value;

pub fn emit_set_mode(mode: Mode) {
    emit_event(
        Event {
            kind: EventKind("set_mode".to_string()),
            data: EventData::Mode(mode),
        },
        DispatchDescriptor {
            consumable: true,
            sort_keys: vec![SortKey("priority".to_string())],
        },
    );
}
pub fn emit_cursor_move(event: CursorMove) {
    emit_event(
        Event {
            kind: EventKind("cursor_move".to_string()),
            data: EventData::Motion(event),
        },
        DispatchDescriptor {
            consumable: true,
            sort_keys: vec![SortKey("priority".to_string())],
        },
    );
}
pub fn emit_edit(edit: EditAction) {
    emit_event(
        Event {
            kind: EventKind("edit".to_string()),
            data: EventData::Edit(edit),
        },
        DispatchDescriptor {
            consumable: true,
            sort_keys: vec![SortKey("priority".to_string())],
        },
    );
}
pub fn emit_command_line(cmd_action: CommandLineAction) {
    emit_event(
        Event {
            kind: EventKind("command_line_action".to_string()),
            data: EventData::CommandLine(cmd_action),
        },
        DispatchDescriptor {
            consumable: true,
            sort_keys: vec![SortKey("priority".to_string())],
        },
    );
}
