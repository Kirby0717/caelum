use std::collections::HashMap;

use clm_plugin_api::core::*;
use clm_plugin_api::input::*;

pub struct ModalPlugin();
impl ModalPlugin {
    pub fn new() -> Self {
        subscribe(Subscription {
            plugin_id: PluginId(0),
            kind: EventKind("key_input".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Box::new(500) as SubscriptionProperty,
            )]),
            handler: Box::new(ModalEventHandler()),
        });
        Self()
    }
}
impl Plugin for ModalPlugin {}

struct ModalEventHandler();
impl EventHandler for ModalEventHandler {
    fn handle(
        &mut self,
        event: &Event,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let Some(key_event) = event.payload.as_ref().downcast_ref::<KeyEvent>()
        else {
            return EventResult::Propagate;
        };
        if key_event.state.is_pressed() {
            match ctx.mode() {
                Mode::Normal => match &key_event.logical_key {
                    LogicalKey::Character(c) => match c.as_str() {
                        "w" => {
                            ctx.cursor_up(1);
                        }
                        "a" => {
                            ctx.cursor_left(1);
                        }
                        "s" => {
                            ctx.cursor_down(1);
                        }
                        "d" => {
                            ctx.cursor_right(1);
                        }
                        "j" => {
                            ctx.set_mode(Mode::Insert);
                        }
                        "k" => {
                            ctx.set_mode(Mode::Insert);
                            ctx.cursor_right(1);
                        }
                        ";" => {
                            ctx.set_mode(Mode::Command);
                        }
                        _ => {}
                    },
                    LogicalKey::Named(named) => match named {
                        NamedKey::ArrowUp => {
                            ctx.cursor_up(1);
                        }
                        NamedKey::ArrowLeft => {
                            ctx.cursor_left(1);
                        }
                        NamedKey::ArrowDown => {
                            ctx.cursor_down(1);
                        }
                        NamedKey::ArrowRight => {
                            ctx.cursor_right(1);
                        }
                        NamedKey::Escape => {
                            ctx.quit();
                        }
                        _ => {}
                    },
                    _ => {}
                },
                Mode::Insert => match &key_event.logical_key {
                    LogicalKey::Character(c) => {
                        for c in c.chars() {
                            ctx.buffer_insert_char_at_cursor(c);
                        }
                    }
                    LogicalKey::Named(named) => match named {
                        NamedKey::ArrowUp => {
                            ctx.cursor_up(1);
                        }
                        NamedKey::ArrowLeft => {
                            ctx.cursor_left(1);
                        }
                        NamedKey::ArrowDown => {
                            ctx.cursor_down(1);
                        }
                        NamedKey::ArrowRight => {
                            ctx.cursor_right(1);
                        }
                        NamedKey::Backspace => {
                            ctx.buffer_backspace();
                        }
                        NamedKey::Escape => {
                            ctx.set_mode(Mode::Normal);
                        }
                        _ => {}
                    },
                    _ => {}
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
                            ctx.set_mode(Mode::Normal);
                        }
                        NamedKey::Backspace => {
                            ctx.command_backspace();
                        }
                        _ => {}
                    },
                    _ => {}
                },
            }
        }
        EventResult::Handled
    }
}
