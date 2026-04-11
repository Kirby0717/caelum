use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use clm_plugin_api::core::*;

/*
-- TODO --
PluginContextの整理
    カーソル移動は消してModalへ移行
*/

#[derive(Debug, Default)]
pub struct ModalPlugin {
    mode: Rc<RefCell<Mode>>,
    cursor: Rc<RefCell<CursorState>>,
}
impl ModalPlugin {
    pub fn new() -> Self {
        let mode = Rc::new(RefCell::new(Mode::Normal));
        let cursor = Rc::new(RefCell::new(CursorState::default()));
        Self { mode, cursor }
    }

    pub fn clamp_cursor(&mut self, ctx: &mut dyn PluginContext) {
        let mut cursor = self.cursor.borrow_mut();
        let max_row = ctx.buffer_len_lines().saturating_sub(1);
        cursor.row = cursor.row.min(max_row);
        let max_col = match *self.mode.borrow() {
            Mode::Insert => ctx.buffer_line_len_chars(cursor.row),
            _ => ctx.buffer_line_len_chars(cursor.row).saturating_sub(1),
        };
        cursor.col = cursor.col.min(max_col);
    }
}
impl Plugin for ModalPlugin {
    fn init(&mut self, plugin_id: PluginId) {
        // Modalプラグインは最初に読み込まれるべき
        debug_assert_eq!(plugin_id, PluginId(0));
        subscribe(Subscription {
            plugin_id,
            kind: EventKind("cursor_move".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
        });
        subscribe(Subscription {
            plugin_id,
            kind: EventKind("set_mode".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
        });
        subscribe(Subscription {
            plugin_id,
            kind: EventKind("quit".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
        });
        register_command(
            "q",
            Box::new(|_| {
                vec![(
                    Event {
                        kind: EventKind("quit".to_string()),
                        payload: EventPayload::Exit,
                    },
                    DispatchDescriptor {
                        consumable: false,
                        sort_keys: vec![],
                    },
                )]
            }),
        );
        {
            let mode = self.mode.clone();
            register_service(
                "modal.mode",
                Box::new(move |_| Value::Str(mode.borrow().to_string())),
            );
            let cursor = self.cursor.clone();
            register_service(
                "modal.cursor",
                Box::new(move |_| (*cursor.borrow()).into()),
            );
        }
    }
    fn on_cursor_move(
        &mut self,
        mv: CursorMove,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        match mv {
            CursorMove::Up(count) => {
                let row = self.cursor.borrow().row;
                self.cursor.borrow_mut().row = row.saturating_sub(count);
            }
            CursorMove::Down(count) => {
                self.cursor.borrow_mut().row += count;
            }
            CursorMove::Left(count) => {
                let col = self.cursor.borrow().col;
                self.cursor.borrow_mut().col = col.saturating_sub(count);
            }
            CursorMove::Right(count) => {
                self.cursor.borrow_mut().col += count;
            }
            _ => return EventResult::Propagate,
        }
        self.clamp_cursor(ctx);
        EventResult::Handled
    }
    fn on_mode_change(
        &mut self,
        mode: Mode,
        _ctx: &mut dyn PluginContext,
    ) -> EventResult {
        *self.mode.borrow_mut() = mode;
        EventResult::Handled
    }
    fn on_exit(&mut self, ctx: &mut dyn PluginContext) -> EventResult {
        ctx.quit();
        EventResult::Handled
    }
}
