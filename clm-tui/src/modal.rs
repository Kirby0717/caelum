use clm_core::editor::SharedState;
use clm_core::event::{Event, EventHandler, EventResult};
use clm_core::mode::Mode;
use crossterm::event::{KeyCode, KeyEvent};

pub struct ModalPlugin {
    state: SharedState,
}
impl ModalPlugin {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }
}
impl EventHandler for ModalPlugin {
    fn handle(&mut self, event: &Event) -> EventResult {
        let mut state = self.state.borrow_mut();
        let Some(key_event) = event.payload.as_ref().downcast_ref::<KeyEvent>()
        else {
            return EventResult::Propagate;
        };
        if key_event.is_press() {
            match state.mode {
                Mode::Normal => match key_event.code {
                    KeyCode::Char(c) => match c {
                        'w' => {
                            state.cursor.row =
                                state.cursor.row.saturating_sub(1);
                            state.clamp_cursor();
                        }
                        'a' => {
                            state.cursor.col =
                                state.cursor.col.saturating_sub(1);
                            state.clamp_cursor();
                        }
                        's' => {
                            state.cursor.row += 1;
                            state.clamp_cursor();
                        }
                        'd' => {
                            state.cursor.col += 1;
                            state.clamp_cursor();
                        }
                        'j' => {
                            state.mode = Mode::Insert;
                        }
                        ';' => {
                            state.mode = Mode::Command;
                        }
                        _ => {}
                    },
                    KeyCode::Esc => {
                        state.running = false;
                    }
                    _ => {}
                },
                Mode::Insert => match key_event.code {
                    KeyCode::Char(c) => {
                        state.insert_char(c);
                    }
                    KeyCode::Backspace => {
                        state.backspace();
                    }
                    KeyCode::Esc => {
                        state.mode = Mode::Normal;
                    }
                    _ => {}
                },
                Mode::Command => match key_event.code {
                    KeyCode::Char(c) => {
                        state.command_line.push(c);
                    }
                    KeyCode::Enter => {
                        state.execute_command();
                    }
                    KeyCode::Esc => {
                        state.command_line.clear();
                        state.mode = Mode::Normal;
                    }
                    KeyCode::Backspace => {
                        if state.command_line.pop().is_none() {
                            state.mode = Mode::Normal;
                        }
                    }
                    _ => {}
                },
            }
        }
        EventResult::Handled
    }
}
