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
                Value::Int(500),
            )]),
            handler: Box::new(cursor_event),
        });
        subscribe(Subscription {
            plugin_id: PluginId(0),
            kind: EventKind("set_mode".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            handler: Box::new(mode_event(mode.clone())),
        });
        {
            let mode = mode.clone();
            register_service(
                "modal.mode",
                Box::new(move |_| Value::Str(mode.borrow().to_string())),
            );
        }
        Self { mode }
    }
}
impl Plugin for ModalPlugin {}

fn cursor_event(event: &Event, ctx: &mut dyn PluginContext) -> EventResult {
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
fn mode_event(mode: Rc<RefCell<Mode>>) -> EventHandler {
    Box::new(move |event, _| {
        let EventPayload::Mode(next_mode) = &event.payload
        else {
            return EventResult::Propagate;
        };
        *mode.borrow_mut() = *next_mode;
        EventResult::Handled
    })
}
