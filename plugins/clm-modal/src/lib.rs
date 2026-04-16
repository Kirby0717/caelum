use clm_plugin_api::core::*;

#[derive(Debug)]
pub struct ModalPlugin {
    mode: Mode,
    cursor: CursorState,
    command_line: String,
    buffer_id: BufferId,
}
impl ModalPlugin {
    pub fn new(path: Option<&str>) -> Self {
        let mode = Mode::Normal;
        let cursor = CursorState::default();
        let command_line = String::new();
        let id: usize = if let Some(path) = path {
            query_service("buffer.open", &[path.into()])
        }
        else {
            query_service("buffer.create", &[])
        }
        .unwrap()
        .try_into()
        .unwrap();
        Self {
            mode,
            cursor,
            command_line,
            buffer_id: BufferId(id),
        }
    }
    pub fn len_lines(&self) -> usize {
        query_service("buffer.len_lines", &[self.buffer_id.0.into()])
            .unwrap()
            .try_into()
            .unwrap()
    }
    pub fn line(&self, row: usize) -> Option<String> {
        query_service("buffer.line", &[self.buffer_id.0.into(), row.into()])
            .unwrap()
            .try_into()
            .unwrap()
    }
    pub fn line_len_bytes(&self, row: usize) -> usize {
        query_service(
            "buffer.line_len_bytes",
            &[self.buffer_id.0.into(), row.into()],
        )
        .unwrap()
        .try_into()
        .unwrap()
    }

    pub fn clamp_cursor(&mut self) {
        let max_row = self.len_lines().saturating_sub(1);
        let mut cursor = self.cursor;
        cursor.row = cursor.row.min(max_row);
        let max_col = match self.mode {
            Mode::Insert => self.line_len_bytes(cursor.row),
            _ => self.line_len_bytes(cursor.row).saturating_sub(1),
        };
        cursor.byte_col = cursor.byte_col.min(max_col);
        self.cursor = cursor;
    }
}

#[clm_plugin_api::clm_handlers(name = "modal")]
impl ModalPlugin {
    #[subscribe(priority = 500)]
    fn on_set_mode(
        &mut self,
        data: &EventData,
        _ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::Mode(mode) = data
        else {
            return EventResult::Propagate;
        };
        self.mode = *mode;
        self.clamp_cursor();
        EventResult::Handled
    }
    #[subscribe(priority = 500)]
    fn on_quit(
        &mut self,
        _data: &EventData,
        ctx: &mut dyn PluginContext,
    ) -> EventResult {
        ctx.quit();
        EventResult::Handled
    }
    #[subscribe(priority = 500)]
    fn on_cursor_move(
        &mut self,
        data: &EventData,
        _ctx: &mut dyn PluginContext,
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
                if count == 0 {
                    return EventResult::Handled;
                }
                let line = self.line(self.cursor.row).unwrap();
                let left = &line[..self.cursor.byte_col];
                if let Some((i, _)) = left.char_indices().nth_back(count - 1) {
                    self.cursor.byte_col = i;
                }
                else {
                    self.cursor.byte_col = 0;
                }
            }
            CursorMove::Right(count) => {
                let line = self.line(self.cursor.row).unwrap();
                let right = &line[self.cursor.byte_col..];
                if let Some((i, _)) = right.char_indices().nth(count) {
                    self.cursor.byte_col += i;
                }
                else {
                    self.cursor.byte_col = line.len();
                }
            }
            _ => return EventResult::Propagate,
        }
        self.clamp_cursor();
        EventResult::Handled
    }
    #[subscribe(priority = 500)]
    fn on_edit(
        &mut self,
        data: &EventData,
        _ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::Edit(edit) = data
        else {
            return EventResult::Propagate;
        };
        let mut cursor = self.cursor;
        match edit {
            EditAction::InsertChar(c) => {
                emit_buffer_op(BufferOp::Insert {
                    buffer_id: self.buffer_id,
                    line_idx: cursor.row,
                    byte_col_idx: cursor.byte_col,
                    text: c.to_string().clone(),
                });
                cursor.byte_col += c.len_utf8();
            }
            EditAction::InsertText(text) => {
                emit_buffer_op(BufferOp::Insert {
                    buffer_id: self.buffer_id,
                    line_idx: cursor.row,
                    byte_col_idx: cursor.byte_col,
                    text: text.clone(),
                });
                cursor.byte_col += text.len();
            }
            EditAction::DeleteCharForward => {
                let line = self.line(cursor.row).unwrap();
                let right_one = &line[cursor.byte_col..].chars().next();
                if let Some(c) = right_one {
                    let next_size = c.len_utf8();
                    emit_buffer_op(BufferOp::Remove {
                        buffer_id: self.buffer_id,
                        start_line_idx: cursor.row,
                        start_byte_col_idx: cursor.byte_col,
                        end_line_idx: cursor.row,
                        end_byte_col_idx: cursor.byte_col + next_size,
                    });
                }
                else {
                    if let Some(next_line) = self.line(cursor.row + 1)
                        && let Some(first) = next_line.chars().next()
                    {
                        emit_buffer_op(BufferOp::Remove {
                            buffer_id: self.buffer_id,
                            start_line_idx: cursor.row + 1,
                            start_byte_col_idx: 0,
                            end_line_idx: cursor.row + 1,
                            end_byte_col_idx: first.len_utf8(),
                        });
                    }
                }
            }
            EditAction::DeleteCharBackward => {
                if cursor.row == 0 && cursor.byte_col == 0 {
                    return EventResult::Handled;
                }

                let line = self.line(cursor.row).unwrap();
                let left_one = &line[..cursor.byte_col].chars().next_back();
                if let Some(c) = left_one {
                    let prev_size = c.len_utf8();
                    emit_buffer_op(BufferOp::Remove {
                        buffer_id: self.buffer_id,
                        start_line_idx: cursor.row,
                        start_byte_col_idx: cursor.byte_col - prev_size,
                        end_line_idx: cursor.row,
                        end_byte_col_idx: cursor.byte_col,
                    });
                    cursor.byte_col -= prev_size;
                }
                else {
                    if let Some(prev_line) = self.line(cursor.row - 1)
                        && let Some(last) = prev_line.chars().next_back()
                    {
                        emit_buffer_op(BufferOp::Remove {
                            buffer_id: self.buffer_id,
                            start_line_idx: cursor.row - 1,
                            start_byte_col_idx: prev_line.len()
                                - last.len_utf8(),
                            end_line_idx: cursor.row - 1,
                            end_byte_col_idx: prev_line.len(),
                        });
                        cursor.row -= 1;
                        cursor.byte_col = prev_line.len() - last.len_utf8();
                    }
                }
            }
            EditAction::NewLine => {
                emit_buffer_op(BufferOp::Insert {
                    buffer_id: self.buffer_id,
                    line_idx: cursor.row,
                    byte_col_idx: cursor.byte_col,
                    text: "\n".to_string(),
                });
                cursor.row += 1;
                cursor.byte_col = 0;
            }
            _ => return EventResult::Propagate,
        }
        self.cursor = cursor;
        EventResult::Handled
    }
    #[subscribe(priority = 500)]
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
                let parsed = command_line
                    .split_whitespace()
                    .map(String::from)
                    .collect::<Vec<_>>();
                if !parsed.is_empty() {
                    execute_command(&parsed[0], &parsed[1..]);
                }
                command_line.clear();
                self.mode = Mode::Normal;
            }
            CommandLineAction::Clear => {
                command_line.clear();
            }
        }
        EventResult::Handled
    }
    #[subscribe(priority = 500)]
    fn on_switch_buffer(
        &mut self,
        data: &EventData,
        _ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::Custom(Value::Int(buf_id)) = data
        else {
            return EventResult::Propagate;
        };
        self.buffer_id = BufferId(*buf_id as usize);
        self.cursor = CursorState::default();
        self.clamp_cursor();
        EventResult::Handled
    }
    #[service]
    fn mode(&self, _args: &[Value]) -> Value {
        Value::Str(self.mode.to_string())
    }
    #[service]
    fn cursor(&self, _args: &[Value]) -> Value {
        self.cursor.into()
    }
    #[service]
    fn command_line(&self, _args: &[Value]) -> Value {
        Value::Str(self.command_line.clone())
    }
    #[service]
    fn buffer_id(&self, _args: &[Value]) -> Value {
        self.buffer_id.0.into()
    }
}

impl Plugin for ModalPlugin {
    fn init(&mut self, reg: PluginRegistrar) {
        Self::register_service_and_subscribe(&reg);
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
        register_command(
            "w",
            Box::new(|_| {
                vec![(
                    Event {
                        kind: EventKind("buffer_op".to_string()),
                        data: EventData::BufferOp(BufferOp::Save(BufferId(
                            query_service("modal.buffer_id", &[])
                                .unwrap()
                                .try_into()
                                .unwrap(),
                        ))),
                    },
                    DispatchDescriptor {
                        consumable: false,
                        sort_keys: vec![],
                    },
                )]
            }),
        );
        register_command(
            "x",
            Box::new(|_| {
                vec![
                    (
                        Event {
                            kind: EventKind("buffer_op".to_string()),
                            data: EventData::BufferOp(BufferOp::Save(
                                BufferId(
                                    query_service("modal.buffer_id", &[])
                                        .unwrap()
                                        .try_into()
                                        .unwrap(),
                                ),
                            )),
                        },
                        DispatchDescriptor {
                            consumable: false,
                            sort_keys: vec![],
                        },
                    ),
                    (
                        Event {
                            kind: EventKind("quit".to_string()),
                            data: EventData::None,
                        },
                        DispatchDescriptor {
                            consumable: false,
                            sort_keys: vec![],
                        },
                    ),
                ]
            }),
        );
        register_command(
            "e",
            Box::new(|args| {
                let path = if let Some(path) = args.first() {
                    path.clone()
                }
                else {
                    let current_id: usize =
                        query_service("modal.buffer_id", &[])
                            .unwrap()
                            .try_into()
                            .unwrap();
                    let file_path: Option<String> =
                        query_service("buffer.file_path", &[current_id.into()])
                            .unwrap()
                            .try_into()
                            .unwrap();
                    let Some(file_path) = file_path
                    else {
                        return vec![];
                    };
                    file_path
                };

                let buf_id =
                    query_service("buffer.open", &[path.into()]).unwrap();
                vec![(
                    Event {
                        kind: EventKind("switch_buffer".to_string()),
                        data: EventData::Custom(buf_id),
                    },
                    DispatchDescriptor {
                        consumable: true,
                        sort_keys: vec![SortKey("priority".to_string())],
                    },
                )]
            }),
        );
    }
}
