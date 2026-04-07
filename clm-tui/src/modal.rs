use clm_core::editor::SharedState;
use clm_core::event::{Event, EventHandler, EventResult};
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
            match key_event.code {
                KeyCode::Char(c) => match c {
                    'w' => {
                        state.cursor.row = state.cursor.row.saturating_sub(1);
                        state.clamp_cursor();
                    }
                    'a' => {
                        state.cursor.col = state.cursor.col.saturating_sub(1);
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
                    _ => {}
                },
                KeyCode::Esc => {
                    state.running = false;
                }
                _ => {}
            }
        }
        EventResult::Handled
    }
}
