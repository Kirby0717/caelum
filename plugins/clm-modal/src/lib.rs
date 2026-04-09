use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use clm_plugin_api::core::*;

#[derive(Debug, Default)]
pub struct ModalPlugin {
    mode: Rc<RefCell<Mode>>,
}
impl ModalPlugin {
    pub fn new() -> Self {
        let mode = Rc::new(RefCell::new(Mode::Normal));

        subscribe(Subscription {
            plugin_id: PluginId(0),
            kind: EventKind("cursor_move".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Box::new(500) as SubscriptionProperty,
            )]),
            handler: Box::new(CursorEventHandler()),
        });
        subscribe(Subscription {
            plugin_id: PluginId(0),
            kind: EventKind("set_mode".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Box::new(500) as SubscriptionProperty,
            )]),
            handler: Box::new(ModeEventHandler(mode.clone())),
        });
        register_service("modal.mode", mode.clone());
        Self { mode }
    }
}
impl Plugin for ModalPlugin {}

struct CursorEventHandler();
impl EventHandler for CursorEventHandler {
    fn handle(
        &mut self,
        event: &Event,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventPayload::CursorMove(cursor_move) = &event.payload
        else {
            return EventResult::Propagate;
        };
        match cursor_move {
            CursorMove::Up(count) => ctx.cursor_up(*count),
            CursorMove::Down(count) => ctx.cursor_down(*count),
            CursorMove::Left(count) => ctx.cursor_left(*count),
            CursorMove::Right(count) => ctx.cursor_right(*count),
            _ => return EventResult::Propagate,
        }
        EventResult::Handled
    }
}

struct ModeEventHandler(Rc<RefCell<Mode>>);
impl EventHandler for ModeEventHandler {
    fn handle(
        &mut self,
        event: &Event,
        _ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventPayload::Mode(mode) = &event.payload
        else {
            return EventResult::Propagate;
        };
        *self.0.borrow_mut() = *mode;
        EventResult::Handled
    }
}
