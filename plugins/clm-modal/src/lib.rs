use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use clm_plugin_api::core::*;

#[derive(Debug, Default)]
pub struct ModalPlugin {
    mode: Rc<RefCell<Mode>>,
    cursor: Rc<RefCell<CursorState>>,
    command_line: Rc<RefCell<String>>,
    // TODO: コマンドの途中経過も管理する
}
impl ModalPlugin {
    pub fn new() -> Self {
        let mode = Rc::new(RefCell::new(Mode::Normal));
        let cursor = Rc::new(RefCell::new(CursorState::default()));
        let command_line = Rc::new(RefCell::new(String::new()));
        Self {
            mode,
            cursor,
            command_line,
        }
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

#[clm_plugin_api::clm_handlers]
impl ModalPlugin {
    fn set_mode(
        &mut self,
        data: &EventData,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::Mode(mode) = data
        else {
            return EventResult::Propagate;
        };
        *self.mode.borrow_mut() = *mode;
        self.clamp_cursor(ctx);
        EventResult::Handled
    }
    fn quit(
        &mut self,
        _data: &EventData,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        ctx.quit();
        EventResult::Handled
    }
    fn cursor_move(
        &mut self,
        data: &EventData,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::Motion(mv) = data
        else {
            return EventResult::Propagate;
        };
        match *mv {
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
    fn edit(
        &mut self,
        data: &EventData,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::Edit(edit) = data
        else {
            return EventResult::Propagate;
        };
        let mut cursor = self.cursor.borrow_mut();
        match edit {
            EditAction::InsertChar(c) => {
                let char_idx = ctx.buffer_line_to_char(cursor.row) + cursor.col;
                ctx.buffer_insert_char(char_idx, *c);
                cursor.col += 1;
            }
            EditAction::InsertText(text) => {
                let char_idx = ctx.buffer_line_to_char(cursor.row) + cursor.col;
                ctx.buffer_insert(char_idx, text);
                cursor.col += text.chars().count();
            }
            EditAction::DeleteCharForward => {
                let char_idx = ctx.buffer_line_to_char(cursor.row) + cursor.col;
                if ctx.buffer_len_chars() <= char_idx {
                    return EventResult::Handled;
                }
                ctx.buffer_remove((char_idx, char_idx + 1));
            }
            EditAction::DeleteCharBackward => {
                let char_idx = ctx.buffer_line_to_char(cursor.row) + cursor.col;
                if char_idx == 0 {
                    return EventResult::Handled;
                }
                let char_idx = char_idx - 1;
                ctx.buffer_remove((char_idx, char_idx + 1));
                if cursor.col == 0 {
                    cursor.row = cursor.row.saturating_sub(1);
                    cursor.col = ctx.buffer_line_len_chars(cursor.row);
                }
                else {
                    cursor.col -= 1;
                }
            }
            EditAction::NewLine => {
                let char_idx = ctx.buffer_line_to_char(cursor.row) + cursor.col;
                ctx.buffer_insert_char(char_idx, '\n');
                cursor.row += 1;
                cursor.col = 0;
            }
            _ => return EventResult::Propagate,
        }
        EventResult::Handled
    }
    fn buffer(
        &mut self,
        data: &EventData,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::BufferOp(buffer_op) = data
        else {
            return EventResult::Propagate;
        };
        match buffer_op {
            BufferOp::Insert { char_idx, text } => {
                ctx.buffer_insert(*char_idx, text);
            }
            BufferOp::Remove(range) => {
                ctx.buffer_remove(*range);
            }
        }
        EventResult::Handled
    }
    fn command_line(
        &mut self,
        data: &EventData,
        _ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::CommandLine(cmd_action) = data
        else {
            return EventResult::Propagate;
        };
        let mut command_line = self.command_line.borrow_mut();
        match cmd_action {
            CommandLineAction::AddChar(c) => {
                command_line.push(*c);
            }
            CommandLineAction::Backspace => {
                command_line.pop();
            }
            CommandLineAction::Execute => {
                execute_command(&command_line, &[]);
                command_line.clear();
                *self.mode.borrow_mut() = Mode::Normal;
            }
            CommandLineAction::Clear => {
                command_line.clear();
            }
        }
        EventResult::Handled
    }
}

impl Plugin for ModalPlugin {
    fn init(&mut self, plugin_id: PluginId) {
        // Modalプラグインは最初に読み込まれるべき
        debug_assert_eq!(plugin_id, PluginId(0));
        subscribe(Subscription {
            plugin_id,
            kind: EventKind("set_mode".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            handler: Self::SET_MODE,
        });
        subscribe(Subscription {
            plugin_id,
            kind: EventKind("quit".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            handler: Self::QUIT,
        });
        subscribe(Subscription {
            plugin_id,
            kind: EventKind("cursor_move".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            handler: Self::CURSOR_MOVE,
        });
        subscribe(Subscription {
            plugin_id,
            kind: EventKind("edit".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            handler: Self::EDIT,
        });
        subscribe(Subscription {
            plugin_id,
            kind: EventKind("buffer".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            handler: Self::BUFFER,
        });
        subscribe(Subscription {
            plugin_id,
            kind: EventKind("command_line".to_string()),
            properties: HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            handler: Self::COMMAND_LINE,
        });
        register_command(
            "q",
            Box::new(|_| {
                vec![(
                    Event {
                        kind: EventKind("quit".to_string()),
                        data: EventData::None,
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
            let command_line = self.command_line.clone();
            register_service(
                "modal.command_line",
                Box::new(move |_| Value::Str(command_line.borrow().clone())),
            );
        }
    }
}
