use clm_plugin_api::core::*;
use clm_plugin_api::data::*;
use clm_plugin_api::priority;

#[derive(Debug)]
pub struct ModalPlugin {
    mode: Mode,
    cursor: CursorState,
    view_offset: (usize, usize),
    command_line: String,
    command_line_cursor: usize,
    buffer_id: BufferId,
    key_holder: Option<LockToken>,
}
impl ModalPlugin {
    pub fn new(path: Option<&str>) -> Self {
        let mode = Mode::Normal;
        let cursor = CursorState::default();
        let command_line = String::new();
        let id: usize = if let Some(path) = path {
            query_service("buffer.open", &[path.into()])
        } else {
            query_service("buffer.create", &[])
        }
        .unwrap()
        .try_into()
        .unwrap();
        Self {
            mode,
            cursor,
            view_offset: (0, 0),
            command_line,
            command_line_cursor: 0,
            buffer_id: BufferId(id),
            key_holder: None,
        }
    }
    pub fn len_lines(&self) -> usize {
        query_service("buffer.len_lines", &[self.buffer_id.into()])
            .unwrap()
            .try_into()
            .unwrap()
    }
    pub fn line(&self, row: usize) -> Option<String> {
        query_service("buffer.line", &[self.buffer_id.into(), row.into()])
            .unwrap()
            .try_into()
            .unwrap()
    }
    pub fn line_len_bytes(&self, row: usize) -> usize {
        query_service(
            "buffer.line_len_bytes",
            &[self.buffer_id.into(), row.into()],
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
    #[subscribe(priority = priority::DEFAULT)]
    fn on_set_mode(&mut self, data: &Value) -> EventResult {
        let Ok(mode) = from_value::<Mode>(data.clone()) else {
            return EventResult::Propagate;
        };
        if self.mode != Mode::Insert && mode == Mode::Insert {
            match query_service("buffer.lock", &[self.buffer_id.into()]) {
                Ok(r) => {
                    let key = LockToken::try_from(r).unwrap();
                    self.key_holder = Some(key);
                }
                Err(_e) => {
                    // TODO: エラー出力
                }
            }
        }
        if self.mode == Mode::Insert && mode != Mode::Insert {
            let key = self.key_holder.unwrap();
            if let Err(e) = query_service("buffer.unlock", &[self.buffer_id.into(), key.into()]) {
                panic!("{e}");
            }
        }
        if self.mode == Mode::Command && mode != Mode::Command {
            self.command_line_cursor = 0;
            self.command_line.clear();
        }
        self.mode = mode;
        self.clamp_cursor();
        emit_event(
            Event {
                kind: EventKind("mode_changed".to_string()),
                data: to_value(&mode).unwrap(),
            },
            DispatchDescriptor::Broadcast,
        );
        emit_event(
            Event {
                kind: EventKind("render".to_string()),
                data: Value::Null,
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
        EventResult::Handled
    }
    #[subscribe(priority = priority::DEFAULT)]
    fn on_quit(&mut self, _data: &Value) -> EventResult {
        quit();
        EventResult::Handled
    }
    #[subscribe(priority = priority::DEFAULT)]
    fn on_cursor_move(&mut self, data: &Value) -> EventResult {
        let Ok(mv) = CursorMove::try_from(data.clone()) else {
            return EventResult::Propagate;
        };
        match self.mode {
            Mode::Normal | Mode::Insert => match mv {
                CursorMove::Up { count } => {
                    let row = self.cursor.row;
                    self.cursor.row = row.saturating_sub(count);
                }
                CursorMove::Down { count } => {
                    self.cursor.row += count;
                }
                CursorMove::Left { count } => {
                    if count == 0 {
                        return EventResult::Handled;
                    }
                    let line = self.line(self.cursor.row).unwrap();
                    let left = &line[..self.cursor.byte_col];
                    if let Some((i, _)) = left.char_indices().nth_back(count - 1) {
                        self.cursor.byte_col = i;
                    } else {
                        self.cursor.byte_col = 0;
                    }
                }
                CursorMove::Right { count } => {
                    let line = self.line(self.cursor.row).unwrap();
                    let right = &line[self.cursor.byte_col..];
                    if let Some((i, _)) = right.char_indices().nth(count) {
                        self.cursor.byte_col += i;
                    } else {
                        self.cursor.byte_col = line.len();
                    }
                }
                _ => return EventResult::Propagate,
            },
            Mode::Command => match mv {
                CursorMove::Left { count } => {
                    if count == 0 {
                        return EventResult::Handled;
                    }
                    let left = &self.command_line[..self.command_line_cursor];
                    if let Some((i, _)) = left.char_indices().nth_back(count - 1) {
                        self.command_line_cursor = i;
                    } else {
                        self.command_line_cursor = 0;
                    }
                }
                CursorMove::Right { count } => {
                    let right = &self.command_line[self.command_line_cursor..];
                    if let Some((i, _)) = right.char_indices().nth(count) {
                        self.command_line_cursor += i;
                    } else {
                        self.command_line_cursor = self.command_line.len();
                    }
                }
                _ => return EventResult::Propagate,
            },
        }
        self.clamp_cursor();
        emit_event(
            Event {
                kind: EventKind("render".to_string()),
                data: Value::Null,
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
        EventResult::Handled
    }
    #[subscribe(priority = priority::DEFAULT)]
    fn on_edit(&mut self, data: &Value) -> EventResult {
        let Ok(edit) = EditAction::try_from(data.clone()) else {
            return EventResult::Propagate;
        };
        let mut cursor = self.cursor;
        match edit {
            EditAction::InsertText(text) => {
                emit_buffer_op(&BufferOp::Insert {
                    buffer_id: self.buffer_id,
                    line_idx: cursor.row,
                    byte_col_idx: cursor.byte_col,
                    text: text.clone(),
                    lock_token: self.key_holder,
                });
                cursor.byte_col += text.len();
            }
            EditAction::DeleteCharForward => {
                let line = self.line(cursor.row).unwrap();
                let right_one = &line[cursor.byte_col..].chars().next();
                if let Some(c) = right_one {
                    let next_size = c.len_utf8();
                    emit_buffer_op(&BufferOp::Remove {
                        buffer_id: self.buffer_id,
                        start_line_idx: cursor.row,
                        start_byte_col_idx: cursor.byte_col,
                        end_line_idx: cursor.row,
                        end_byte_col_idx: cursor.byte_col + next_size,
                        lock_token: self.key_holder,
                    });
                } else {
                    if let Some(next_line) = self.line(cursor.row + 1)
                        && let Some(first) = next_line.chars().next()
                    {
                        emit_buffer_op(&BufferOp::Remove {
                            buffer_id: self.buffer_id,
                            start_line_idx: cursor.row + 1,
                            start_byte_col_idx: 0,
                            end_line_idx: cursor.row + 1,
                            end_byte_col_idx: first.len_utf8(),
                            lock_token: self.key_holder,
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
                    emit_buffer_op(&BufferOp::Remove {
                        buffer_id: self.buffer_id,
                        start_line_idx: cursor.row,
                        start_byte_col_idx: cursor.byte_col - prev_size,
                        end_line_idx: cursor.row,
                        end_byte_col_idx: cursor.byte_col,
                        lock_token: self.key_holder,
                    });
                    cursor.byte_col -= prev_size;
                } else {
                    if let Some(prev_line) = self.line(cursor.row - 1)
                        && let Some(last) = prev_line.chars().next_back()
                    {
                        emit_buffer_op(&BufferOp::Remove {
                            buffer_id: self.buffer_id,
                            start_line_idx: cursor.row - 1,
                            start_byte_col_idx: prev_line.len() - last.len_utf8(),
                            end_line_idx: cursor.row - 1,
                            end_byte_col_idx: prev_line.len(),
                            lock_token: self.key_holder,
                        });
                        cursor.row -= 1;
                        cursor.byte_col = prev_line.len() - last.len_utf8();
                    }
                }
            }
            EditAction::NewLine => {
                emit_buffer_op(&BufferOp::Insert {
                    buffer_id: self.buffer_id,
                    line_idx: cursor.row,
                    byte_col_idx: cursor.byte_col,
                    text: "\n".to_string(),
                    lock_token: self.key_holder,
                });
                cursor.row += 1;
                cursor.byte_col = 0;
            }
            EditAction::Undo => {
                emit_buffer_op(&BufferOp::Undo(self.buffer_id));
            }
            EditAction::Redo => {
                emit_buffer_op(&BufferOp::Redo(self.buffer_id));
            }
            _ => return EventResult::Propagate,
        }
        self.cursor = cursor;
        emit_event(
            Event {
                kind: EventKind("render".to_string()),
                data: Value::Null,
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
        EventResult::Handled
    }
    #[subscribe(priority = priority::DEFAULT)]
    fn on_command_line_action(&mut self, data: &Value) -> EventResult {
        let Ok(cmd_action) = CommandLineAction::try_from(data.clone()) else {
            return EventResult::Propagate;
        };
        let command_line = &mut self.command_line;
        let cursor = self.command_line_cursor;
        match cmd_action {
            CommandLineAction::InsertText(text) => {
                command_line.insert_str(cursor, &text);
                self.command_line_cursor += text.len();
            }
            CommandLineAction::DeleteCharForward => {
                if command_line.len() < cursor {
                    return EventResult::Propagate;
                }
                let next_size = command_line[cursor..].chars().next().unwrap().len_utf8();
                command_line.drain(cursor..cursor + next_size);
            }
            CommandLineAction::DeleteCharBackward => {
                if cursor == 0 {
                    return EventResult::Propagate;
                }
                let prev_size = command_line[..cursor]
                    .chars()
                    .next_back()
                    .unwrap()
                    .len_utf8();
                command_line.drain(cursor - prev_size..cursor);
                self.command_line_cursor -= prev_size;
            }
            CommandLineAction::Execute => {
                let parsed = command_line
                    .split_whitespace()
                    .map(String::from)
                    .collect::<Vec<_>>();
                if !parsed.is_empty() {
                    execute_command(&parsed[0], &parsed[1..]);
                }
                self.command_line_cursor = 0;
                command_line.clear();
                self.mode = Mode::Normal;
            }
            CommandLineAction::Clear => {
                self.command_line_cursor = 0;
                command_line.clear();
            }
        }
        emit_event(
            Event {
                kind: EventKind("render".to_string()),
                data: Value::Null,
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
        EventResult::Handled
    }
    #[subscribe(priority = priority::DEFAULT)]
    fn on_switch_buffer(&mut self, data: &Value) -> EventResult {
        let Ok(buffer_id) = BufferId::try_from(data.clone()) else {
            return EventResult::Propagate;
        };
        self.buffer_id = buffer_id;
        self.cursor = CursorState::default();
        self.clamp_cursor();
        emit_event(
            Event {
                kind: EventKind("render".to_string()),
                data: Value::Null,
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
        EventResult::Handled
    }
    #[subscribe(priority = priority::DEFAULT)]
    fn on_buffer_changed(&mut self, data: &Value) -> EventResult {
        let Ok(_buffer_change) = BufferChange::try_from(data.clone()) else {
            return EventResult::Propagate;
        };
        self.clamp_cursor();
        EventResult::Handled
    }
    #[service(name = "render_pane")]
    fn render_pane(&mut self, args: &[Value]) -> Result<Value, String> {
        use clm_editor_tui::*;
        let pane_id: PaneId = get_arg(args, 0)?;
        let w: u16 = get_arg(args, 1)?;
        let h: u16 = get_arg(args, 2)?;
        Ok(self.render((w, h)).unwrap().into())
    }
    #[service]
    fn mode(&self, _args: &[Value]) -> Result<Value, String> {
        Ok(self.mode.into())
    }
    #[service]
    fn cursor(&self, _args: &[Value]) -> Result<Value, String> {
        Ok(self.cursor.into())
    }
    #[service]
    fn command_line(&self, _args: &[Value]) -> Result<Value, String> {
        Ok(self.command_line.clone().into())
    }
    #[service]
    fn command_line_cursor(&self, _args: &[Value]) -> Result<Value, String> {
        Ok(self.command_line_cursor.into())
    }
    #[service]
    fn buffer_id(&self, _args: &[Value]) -> Result<Value, String> {
        Ok(self.buffer_id.into())
    }
}
impl ModalPlugin {
    fn render(&mut self, size: (u16, u16)) -> anyhow::Result<Vec<clm_editor_tui::DrawCommand>> {
        use clm_editor_tui::*;
        use unicode_width::UnicodeWidthStr;

        let size = (size.0 as usize, size.1 as usize);
        let mut commands = vec![];

        let mode = self.mode;
        let cursor = self.cursor;
        let buffer_id = self.buffer_id;
        let view_offset = &mut self.view_offset;
        let cursor_line: String =
            query_service_anyhow("buffer.line", &[buffer_id.into(), cursor.row.into()])
                .unwrap()
                .try_into()
                .unwrap();

        // オフセットの計算
        {
            if cursor.row < view_offset.1 {
                view_offset.1 = cursor.row;
            }
            if view_offset.1 + size.1 <= cursor.row {
                view_offset.1 = cursor.row - (size.1 - 1);
            }
            let display_col_l = cursor_line[..cursor.byte_col].width();
            let display_col_r = display_col_l + cursor_line[cursor.byte_col..].width();
            if display_col_l <= view_offset.0 {
                view_offset.0 = display_col_l;
            }
            if view_offset.0 + size.0 <= display_col_r {
                view_offset.0 = display_col_r - size.0;
            }
        }

        // バッファーの表示
        let mut cell_grid = vec![];
        for row in 0..size.1 {
            let line: Option<String> = query_service_anyhow(
                "buffer.line",
                &[buffer_id.into(), (view_offset.1 + row).into()],
            )
            .unwrap()
            .try_into()
            .unwrap();
            if let Some(line) = line {
                cell_grid.push(trim_display_range(
                    &line,
                    view_offset.0,
                    view_offset.0 + size.0,
                ));
            } else {
                break;
            }
        }
        commands.push(DrawCommand::CellGrid(cell_grid));
        // ステータスラインの設定
        /*execute!(stdout(), MoveTo(0, size.1 - 1))?;
        match mode {
            Mode::Normal => execute!(stdout(), Print("-- NORMAL --"),)?,
            Mode::Insert => execute!(stdout(), Print("-- INSERT --"))?,
            Mode::Command => execute!(stdout(), Print("-- COMMAND -- :"), Print(&command_line))?,
        }*/
        // カーソルの設定
        match mode {
            Mode::Normal | Mode::Insert => {
                let x = cursor_line[..cursor.byte_col].width();
                commands.push(DrawCommand::SetCursor {
                    position: (
                        (x - view_offset.0) as u16,
                        (cursor.row - view_offset.1) as u16,
                    ),
                    style: match mode {
                        Mode::Normal => CursorStyle::SteadyBlock,
                        Mode::Insert => CursorStyle::SteadyBar,
                        _ => unreachable!(),
                    },
                });
            }
            _ => {}
        }
        Ok(commands)
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
                        data: Value::Null,
                    },
                    DispatchDescriptor::Broadcast,
                )]
            }),
        );
        register_command(
            "w",
            Box::new(|_| {
                vec![(
                    Event {
                        kind: EventKind("buffer_op".to_string()),
                        data: BufferOp::Save(BufferId(
                            query_service("modal.buffer_id", &[])
                                .unwrap()
                                .try_into()
                                .unwrap(),
                        ))
                        .into(),
                    },
                    DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
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
                            data: BufferOp::Save(BufferId(
                                query_service("modal.buffer_id", &[])
                                    .unwrap()
                                    .try_into()
                                    .unwrap(),
                            ))
                            .into(),
                        },
                        DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
                    ),
                    (
                        Event {
                            kind: EventKind("quit".to_string()),
                            data: Value::Null,
                        },
                        DispatchDescriptor::Broadcast,
                    ),
                ]
            }),
        );
        register_command(
            "e",
            Box::new(|args| {
                let path = if let Some(path) = args.first() {
                    path.clone()
                } else {
                    let current_id: usize = query_service("modal.buffer_id", &[])
                        .unwrap()
                        .try_into()
                        .unwrap();
                    let file_path: Option<String> =
                        query_service("buffer.file_path", &[current_id.into()])
                            .unwrap()
                            .try_into()
                            .unwrap();
                    let Some(file_path) = file_path else {
                        return vec![];
                    };
                    file_path
                };

                let buffer_id = query_service("buffer.open", &[path.into()]).unwrap();
                vec![(
                    Event {
                        kind: EventKind("switch_buffer".to_string()),
                        data: buffer_id,
                    },
                    DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
                )]
            }),
        );
    }
}

fn query_service_anyhow(name: &str, args: &[Value]) -> anyhow::Result<Value> {
    query_service(name, args).map_err(anyhow::Error::msg)
}

fn trim_display_range(line: &str, range_l: usize, range_r: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut width = 0;
    let mut result = String::new();
    for c in line.chars() {
        let l = width;
        let w = c.width().unwrap_or(0);
        let r = l + w;
        width += w;
        if r <= range_l {
            continue;
        }
        if range_r <= l {
            break;
        }
        if l < range_l || range_r < r {
            for i in l..r {
                if range_l <= i && i < range_r {
                    result.push(' ');
                }
            }
        } else {
            if c != '\n' {
                result.push(c);
            }
        }
    }
    result
}
