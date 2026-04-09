use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use clm_plugin_api::core::*;
use clm_plugin_api::input::*;

#[derive(Debug, Default)]
pub struct MotionPlugin();
impl MotionPlugin {
    pub fn new() -> Self {
        subscribe(Subscription {
            plugin_id: PluginId(0),
            kind: EventKind("key_input".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Box::new(500) as SubscriptionProperty,
            )]),
            handler: Box::new(MotionEventHandler()),
        });
        Self()
    }
}
impl Plugin for MotionPlugin {}

struct MotionEventHandler();
impl EventHandler for MotionEventHandler {
    fn handle(
        &mut self,
        event: &Event,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventPayload::KeyInput(key_event) = &event.payload
        else {
            return EventResult::Propagate;
        };
        let mode = with_service("modal.mode", |mode: &Rc<RefCell<Mode>>| {
            *mode.borrow()
        })
        .unwrap_or(Mode::Normal);
        if key_event.state.is_pressed() {
            match mode {
                Mode::Normal => match &key_event.logical_key {
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
                Mode::Insert => match &key_event.logical_key {
                    LogicalKey::Character(c) => {
                        for c in c.chars() {
                            ctx.buffer_insert_char_at_cursor(c);
                        }
                    }
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
                        NamedKey::Backspace => {
                            ctx.buffer_backspace();
                        }
                        NamedKey::Escape => {
                            emit_set_mode(Mode::Normal);
                        }
                        _ => return EventResult::Propagate,
                    },
                    _ => return EventResult::Propagate,
                },
                Mode::Command => match &key_event.logical_key {
                    LogicalKey::Character(c) => {
                        for c in c.chars() {
                            ctx.command_add_char(c);
                        }
                    }
                    LogicalKey::Named(named) => match named {
                        NamedKey::Enter => {
                            ctx.command_execute();
                        }
                        NamedKey::Escape => {
                            ctx.command_clear();
                            emit_set_mode(Mode::Normal);
                        }
                        NamedKey::Backspace => {
                            ctx.command_backspace();
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

fn emit_set_mode(mode: Mode) {
    emit_event(
        Event {
            kind: EventKind("set_mode".to_string()),
            payload: EventPayload::Mode(mode),
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
            payload: EventPayload::CursorMove(event),
        },
        DispatchDescriptor {
            consumable: true,
            sort_keys: vec![SortKey("priority".to_string())],
        },
    );
}
