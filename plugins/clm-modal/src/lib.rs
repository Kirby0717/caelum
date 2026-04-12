use std::collections::HashMap;

use clm_plugin_api::core::*;

#[derive(Debug, Default)]
pub struct ModalPlugin {
    mode: Mode,
    cursor: CursorState,
    command_line: String,
    // TODO: コマンドの途中経過も管理する
}
impl ModalPlugin {
    pub fn new() -> Self {
        let mode = Mode::Normal;
        let cursor = CursorState::default();
        let command_line = String::new();
        Self {
            mode,
            cursor,
            command_line,
        }
    }

    pub fn clamp_cursor(&mut self, ctx: &mut dyn PluginContext) {
        let cursor = &mut self.cursor;
        let max_row = ctx.buffer_len_lines().saturating_sub(1);
        cursor.row = cursor.row.min(max_row);
        let max_col = match self.mode {
            Mode::Insert => ctx.buffer_line_len_chars(cursor.row),
            _ => ctx.buffer_line_len_chars(cursor.row).saturating_sub(1),
        };
        cursor.col = cursor.col.min(max_col);
    }
}

#[clm_plugin_api::clm_handlers]
impl ModalPlugin {
    fn on_set_mode(
        &mut self,
        data: &EventData,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::Mode(mode) = data
        else {
            return EventResult::Propagate;
        };
        self.mode = *mode;
        self.clamp_cursor(ctx);
        EventResult::Handled
    }
    fn on_quit(
        &mut self,
        _data: &EventData,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        ctx.quit();
        EventResult::Handled
    }
    fn on_cursor_move(
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
                let row = self.cursor.row;
                self.cursor.row = row.saturating_sub(count);
            }
            CursorMove::Down(count) => {
                self.cursor.row += count;
            }
            CursorMove::Left(count) => {
                let col = self.cursor.col;
                self.cursor.col = col.saturating_sub(count);
            }
            CursorMove::Right(count) => {
                self.cursor.col += count;
            }
            _ => return EventResult::Propagate,
        }
        self.clamp_cursor(ctx);
        EventResult::Handled
    }
    fn on_edit(
        &mut self,
        data: &EventData,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::Edit(edit) = data
        else {
            return EventResult::Propagate;
        };
        let cursor = &mut self.cursor;
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
                if cursor.col == 0 {
                    cursor.row = cursor.row.saturating_sub(1);
                    cursor.col = ctx.buffer_line_len_chars(cursor.row);
                }
                else {
                    cursor.col -= 1;
                }
                ctx.buffer_remove((char_idx, char_idx + 1));
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
    fn on_buffer_op(
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
    fn on_command_line_action(
        &mut self,
        data: &EventData,
        _ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::CommandLine(cmd_action) = data
        else {
            return EventResult::Propagate;
        };
        let command_line = &mut self.command_line;
        match cmd_action {
            CommandLineAction::AddChar(c) => {
                command_line.push(*c);
            }
            CommandLineAction::Backspace => {
                command_line.pop();
            }
            CommandLineAction::Execute => {
                execute_command(command_line, &[]);
                command_line.clear();
                self.mode = Mode::Normal;
            }
            CommandLineAction::Clear => {
                command_line.clear();
            }
        }
        EventResult::Handled
    }
    fn mode(&self, _args: &[Value]) -> Value {
        Value::Str(self.mode.to_string())
    }
    fn cursor(&self, _args: &[Value]) -> Value {
        self.cursor.into()
    }
    fn command_line(&self, _args: &[Value]) -> Value {
        Value::Str(self.command_line.clone())
    }
}

impl Plugin for ModalPlugin {
    fn init(&mut self, reg: PluginRegistrar) {
        reg.subscribe(
            "set_mode",
            HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            Self::ON_SET_MODE,
        );
        reg.subscribe(
            "quit",
            HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            Self::ON_QUIT,
        );
        reg.subscribe(
            "cursor_move",
            HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            Self::ON_CURSOR_MOVE,
        );
        reg.subscribe(
            "edit",
            HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            Self::ON_EDIT,
        );
        reg.subscribe(
            "buffer_op",
            HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            Self::ON_BUFFER_OP,
        );
        reg.subscribe(
            "command_line_action",
            HashMap::from([(
                PropertyKey("priority".to_string()),
                Value::Int(500),
            )]),
            Self::ON_COMMAND_LINE_ACTION,
        );
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
        reg.register_service("modal.mode", Self::MODE);
        reg.register_service("modal.cursor", Self::CURSOR);
        reg.register_service("modal.command_line", Self::COMMAND_LINE);
    }
}
