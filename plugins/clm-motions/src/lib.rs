use std::collections::HashMap;

use clm_plugin_api::core::*;
use clm_plugin_api::input::*;

#[derive(Debug, Default)]
pub struct MotionPlugin();
impl MotionPlugin {
    pub fn new() -> Self {
        Self()
    }
}
#[clm_plugin_api::clm_handlers]
impl MotionPlugin {
    fn key_input(
        &mut self,
        data: &EventData,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::Key(key) = data
        else {
            return EventResult::Propagate;
        };
        let mode = query_service("modal.mode", &[])
            .and_then(|mode| mode.try_into().ok())
            .unwrap_or(Mode::Normal);
        if key.state.is_pressed() {
            match mode {
                Mode::Normal => match &key.logical_key {
                    LogicalKey::Character(c) => match c.as_str() {
                        "w" => {
                            emit_cursor_move(CursorMove::Up(1));
                        }
                        "a" => {
                            emit_cursor_move(CursorMove::Left(1));
                        }
                        "s" => {
                            emit_cursor_move(CursorMove::Down(1));
                        }
                        "d" => {
                            emit_cursor_move(CursorMove::Right(1));
                        }
                        "j" => {
                            emit_set_mode(Mode::Insert);
                        }
                        "k" => {
                            emit_set_mode(Mode::Insert);
                            emit_cursor_move(CursorMove::Right(1));
                        }
                        ";" => {
                            emit_set_mode(Mode::Command);
                        }
                        _ => return EventResult::Propagate,
                    },
                    LogicalKey::Named(named) => match named {
                        NamedKey::ArrowUp => {
                            emit_cursor_move(CursorMove::Up(1));
                        }
                        NamedKey::ArrowLeft => {
                            emit_cursor_move(CursorMove::Left(1));
                        }
                        NamedKey::ArrowDown => {
                            emit_cursor_move(CursorMove::Down(1));
                        }
                        NamedKey::ArrowRight => {
                            emit_cursor_move(CursorMove::Right(1));
                        }
                        NamedKey::Escape => {
                            ctx.quit();
                        }
                        _ => return EventResult::Propagate,
                    },
                    _ => return EventResult::Propagate,
                },
                Mode::Insert => match &key.logical_key {
                    LogicalKey::Character(c) => {
                        emit_edit(EditAction::InsertText(c.clone()));
                    }
                    LogicalKey::Named(named) => match named {
                        NamedKey::Enter => {
                            emit_edit(EditAction::NewLine);
                        }
                        NamedKey::ArrowUp => {
                            emit_cursor_move(CursorMove::Up(1));
                        }
                        NamedKey::ArrowLeft => {
                            emit_cursor_move(CursorMove::Left(1));
                        }
                        NamedKey::ArrowDown => {
                            emit_cursor_move(CursorMove::Down(1));
                        }
                        NamedKey::ArrowRight => {
                            emit_cursor_move(CursorMove::Right(1));
                        }
                        NamedKey::Backspace => {
                            emit_edit(EditAction::DeleteCharBackward);
                        }
                        NamedKey::Delete => {
                            emit_edit(EditAction::DeleteCharForward);
                        }
                        NamedKey::Escape => {
                            emit_set_mode(Mode::Normal);
                        }
                        _ => return EventResult::Propagate,
                    },
                    _ => return EventResult::Propagate,
                },
                Mode::Command => match &key.logical_key {
                    LogicalKey::Character(c) => {
                        for c in c.chars() {
                            emit_command_line(CommandLineAction::AddChar(c));
                        }
                    }
                    LogicalKey::Named(named) => match named {
                        NamedKey::Enter => {
                            emit_command_line(CommandLineAction::Execute);
                        }
                        NamedKey::Escape => {
                            emit_command_line(CommandLineAction::Clear);
                            emit_set_mode(Mode::Normal);
                        }
                        NamedKey::Backspace => {
                            emit_command_line(CommandLineAction::Backspace);
                        }
                        _ => return EventResult::Propagate,
                    },
                    _ => return EventResult::Propagate,
                },
            }
        }
        EventResult::Handled
    }
}
impl Plugin for MotionPlugin {
    fn init(&mut self, plugin_id: PluginId) {
        subscribe(Subscription {
            plugin_id,
            kind: EventKind("key_input".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            handler: Self::KEY_INPUT,
        });
    }
}

fn emit_set_mode(mode: Mode) {
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
fn emit_cursor_move(event: CursorMove) {
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
fn emit_edit(edit: EditAction) {
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
fn emit_command_line(cmd_action: CommandLineAction) {
    emit_event(
        Event {
            kind: EventKind("command_line".to_string()),
            data: EventData::CommandLine(cmd_action),
        },
        DispatchDescriptor {
            consumable: true,
            sort_keys: vec![SortKey("priority".to_string())],
        },
    );
}
