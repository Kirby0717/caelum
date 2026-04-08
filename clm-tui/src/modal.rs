use clm_plugin_api::*;
use crossterm::event::{KeyCode, KeyEvent};

pub struct ModalPlugin();
impl ModalPlugin {
    pub fn new() -> Self {
        Self()
    }
}
impl EventHandler for ModalPlugin {
    fn handle(
        &mut self,
        event: &Event,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let Some(key_event) = event.payload.as_ref().downcast_ref::<KeyEvent>()
        else {
            return EventResult::Propagate;
        };
        if key_event.is_press() {
            match ctx.mode() {
                Mode::Normal => match key_event.code {
                    KeyCode::Char(c) => match c {
                        'w' => {
                            ctx.cursor_up(1);
                        }
                        'a' => {
                            ctx.cursor_left(1);
                        }
                        's' => {
                            ctx.cursor_down(1);
                        }
                        'd' => {
                            ctx.cursor_right(1);
                        }
                        'j' => {
                            ctx.set_mode(Mode::Insert);
                        }
                        ';' => {
                            ctx.set_mode(Mode::Command);
                        }
                        _ => {}
                    },
                    KeyCode::Up => {
                        ctx.cursor_up(1);
                    }
                    KeyCode::Left => {
                        ctx.cursor_left(1);
                    }
                    KeyCode::Down => {
                        ctx.cursor_down(1);
                    }
                    KeyCode::Right => {
                        ctx.cursor_right(1);
                    }
                    KeyCode::Esc => {
                        ctx.quit();
                    }
                    _ => {}
                },
                Mode::Insert => match key_event.code {
                    KeyCode::Char(c) => {
                        ctx.buffer_insert_char_at_cursor(c);
                    }
                    KeyCode::Up => {
                        ctx.cursor_up(1);
                    }
                    KeyCode::Left => {
                        ctx.cursor_left(1);
                    }
                    KeyCode::Down => {
                        ctx.cursor_down(1);
                    }
                    KeyCode::Right => {
                        ctx.cursor_right(1);
                    }
                    KeyCode::Backspace => {
                        ctx.buffer_backspace();
                    }
                    KeyCode::Esc => {
                        ctx.set_mode(Mode::Normal);
                    }
                    _ => {}
                },
                Mode::Command => match key_event.code {
                    KeyCode::Char(c) => {
                        ctx.command_add_char(c);
                    }
                    KeyCode::Enter => {
                        ctx.command_execute();
                    }
                    KeyCode::Esc => {
                        ctx.command_clear();
                        ctx.set_mode(Mode::Normal);
                    }
                    KeyCode::Backspace => {
                        ctx.command_backspace();
                    }
                    _ => {}
                },
            }
        }
        EventResult::Handled
    }
}
