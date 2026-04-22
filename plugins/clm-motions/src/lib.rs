use clm_plugin_api::core::*;
use clm_plugin_api::data::*;
use clm_plugin_api::input::*;

#[derive(Debug, Default)]
pub struct MotionPlugin();
impl MotionPlugin {
    pub fn new() -> Self {
        Self()
    }
}
#[clm_plugin_api::clm_handlers(name = "motion")]
impl MotionPlugin {
    #[subscribe(priority = 500)]
    fn on_key_input(&mut self, data: &Value, _ctx: &mut dyn PluginContext) -> EventResult {
        let Ok(key) = KeyEvent::try_from(data.clone()) else {
            return EventResult::Propagate;
        };
        let mode = query_service("modal.mode", &[])
            .ok()
            .and_then(|mode| mode.try_into().ok())
            .unwrap_or(Mode::Normal);
        if key.state.is_pressed() {
            match mode {
                Mode::Normal => match &key.logical_key {
                    LogicalKey::Character(c) => match c.as_str() {
                        "w" => {
                            emit_cursor_move(&CursorMove::Up { count: 1 });
                        }
                        "a" => {
                            emit_cursor_move(&CursorMove::Left { count: 1 });
                        }
                        "s" => {
                            emit_cursor_move(&CursorMove::Down { count: 1 });
                        }
                        "d" => {
                            emit_cursor_move(&CursorMove::Right { count: 1 });
                        }
                        "W" => {
                            emit_cursor_move(&CursorMove::Up { count: 5 });
                        }
                        "A" => {
                            emit_cursor_move(&CursorMove::Left { count: 5 });
                        }
                        "S" => {
                            emit_cursor_move(&CursorMove::Down { count: 5 });
                        }
                        "D" => {
                            emit_cursor_move(&CursorMove::Right { count: 5 });
                        }
                        "i" => {
                            emit_edit(&EditAction::Undo);
                        }
                        "I" => {
                            emit_edit(&EditAction::Redo);
                        }
                        "j" => {
                            emit_set_mode(Mode::Insert);
                        }
                        "k" => {
                            emit_set_mode(Mode::Insert);
                            emit_cursor_move(&CursorMove::Right { count: 1 });
                        }
                        ";" => {
                            emit_set_mode(Mode::Command);
                        }
                        _ => return EventResult::Propagate,
                    },
                    LogicalKey::Named(named) => match named {
                        NamedKey::ArrowUp => {
                            emit_cursor_move(&CursorMove::Up { count: 1 });
                        }
                        NamedKey::ArrowLeft => {
                            emit_cursor_move(&CursorMove::Left { count: 1 });
                        }
                        NamedKey::ArrowDown => {
                            emit_cursor_move(&CursorMove::Down { count: 1 });
                        }
                        NamedKey::ArrowRight => {
                            emit_cursor_move(&CursorMove::Right { count: 1 });
                        }
                        _ => return EventResult::Propagate,
                    },
                    _ => return EventResult::Propagate,
                },
                Mode::Insert => match &key.logical_key {
                    LogicalKey::Character(c) => {
                        emit_edit(&EditAction::InsertText(c.clone()));
                    }
                    LogicalKey::Named(named) => match named {
                        NamedKey::Enter => {
                            emit_edit(&EditAction::NewLine);
                        }
                        NamedKey::ArrowUp => {
                            emit_cursor_move(&CursorMove::Up { count: 1 });
                        }
                        NamedKey::ArrowLeft => {
                            emit_cursor_move(&CursorMove::Left { count: 1 });
                        }
                        NamedKey::ArrowDown => {
                            emit_cursor_move(&CursorMove::Down { count: 1 });
                        }
                        NamedKey::ArrowRight => {
                            emit_cursor_move(&CursorMove::Right { count: 1 });
                        }
                        NamedKey::Backspace => {
                            emit_edit(&EditAction::DeleteCharBackward);
                        }
                        NamedKey::Delete => {
                            emit_edit(&EditAction::DeleteCharForward);
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
                            emit_command_line(&CommandLineAction::AddChar(c));
                        }
                    }
                    LogicalKey::Named(named) => match named {
                        NamedKey::Enter => {
                            emit_command_line(&CommandLineAction::Execute);
                        }
                        NamedKey::Escape => {
                            emit_command_line(&CommandLineAction::Clear);
                            emit_set_mode(Mode::Normal);
                        }
                        NamedKey::Backspace => {
                            emit_command_line(&CommandLineAction::Backspace);
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
    fn init(&mut self, reg: PluginRegistrar) {
        Self::register_service_and_subscribe(&reg);
    }
}
