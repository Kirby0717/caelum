use std::collections::HashMap;

use clm_plugin_api::core::*;
use clm_plugin_api::data::id::*;
use clm_plugin_api::data::*;

#[derive(Debug)]
pub struct PaneState {
    pub buffer_id: BufferId,
    pub cursor: CursorState,
    pub view_offset: (usize, usize),
}
impl PaneState {
    fn new(buffer_id: BufferId) -> Self {
        Self {
            buffer_id,
            cursor: CursorState::default(),
            view_offset: (0, 0),
        }
    }
    fn len_lines(&self) -> usize {
        query_service("buffer.len_lines", &[self.buffer_id.into()])
            .unwrap()
            .try_into()
            .unwrap()
    }
    fn line(&self, row: usize) -> Option<String> {
        query_service("buffer.line", &[self.buffer_id.into(), row.into()])
            .unwrap()
            .try_into()
            .unwrap()
    }
    fn line_len_bytes(&self, row: usize) -> Option<usize> {
        query_service(
            "buffer.line_len_bytes",
            &[self.buffer_id.into(), row.into()],
        )
        .unwrap()
        .try_into()
        .unwrap()
    }
    fn clamp_cursor(&mut self, mode: Mode) {
        use unicode_width::UnicodeWidthChar;
        let mut cursor = self.cursor;
        let max_row = self.len_lines().saturating_sub(1);
        cursor.row = cursor.row.min(max_row);
        let line = self.line(cursor.row).unwrap();
        let max_col = match mode {
            Mode::Insert => line.len(),
            _ => line.len() - line.chars().next_back().and_then(char::width).unwrap_or(0),
        };
        cursor.byte_col = cursor.byte_col.min(max_col);
        self.cursor = cursor;
    }
    fn render(
        &mut self,
        mode: Mode,
        size: (u16, u16),
    ) -> Result<Vec<clm_tui_compositor::DrawCommand>, String> {
        use clm_tui_compositor::*;
        use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

        let size = (size.0 as usize, size.1 as usize);
        let mut commands = vec![];

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
            let display_col_r = display_col_l
                + cursor_line[cursor.byte_col..]
                    .chars()
                    .next()
                    .and_then(char::width)
                    .unwrap_or(0);
            if display_col_l <= view_offset.0 {
                view_offset.0 = display_col_l;
            }
            if view_offset.0 + size.0 < display_col_r {
                view_offset.0 = display_col_r - size.0;
            }
        }

        // バッファーの表示
        for row in 0..size.1 {
            let line: Option<String> = query_service_anyhow(
                "buffer.line",
                &[buffer_id.into(), (view_offset.1 + row).into()],
            )
            .unwrap()
            .try_into()
            .unwrap();
            if let Some(line) = line {
                commands.push(DrawCommand::DrawString {
                    position: (0, row as u16),
                    text: trim_display_range(&line, view_offset.0, view_offset.0 + size.0),
                });
            } else {
                break;
            }
        }
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
#[derive(Debug)]
pub struct ModalPlugin {
    mode: Mode,
    command_line: String,
    command_line_cursor: usize,
    panes: HashMap<PaneId, PaneState>,
    active_pane: Option<PaneId>,
    key_holder: Option<LockToken>,
}
impl Default for ModalPlugin {
    fn default() -> Self {
        Self::new()
    }
}
impl ModalPlugin {
    pub fn new() -> Self {
        let mode = Mode::Normal;
        let command_line = String::new();
        Self {
            mode,
            command_line,
            command_line_cursor: 0,
            panes: HashMap::new(),
            active_pane: None,
            key_holder: None,
        }
    }
    fn active_pane_state(&self) -> Option<&PaneState> {
        self.panes.get(&self.active_pane?)
    }
    fn active_pane_state_mut(&mut self) -> Option<&mut PaneState> {
        self.panes.get_mut(&self.active_pane?)
    }
    fn clamp_cursor(&mut self) {
        let Some(active_state) = self.active_pane_state() else {
            return;
        };
        let changed_buffer_id = active_state.buffer_id;
        for state in self.panes.values_mut() {
            if state.buffer_id == changed_buffer_id {
                state.clamp_cursor(self.mode);
            }
        }
    }
}

#[clm_plugin_api::clm_handlers(name = "modal")]
impl ModalPlugin {
    #[service]
    fn quit(&mut self, _args: &[Value]) -> Result<Value, String> {
        quit();
        Ok(Value::Null)
    }
    #[service]
    fn set_mode(&mut self, args: &[Value]) -> Result<Value, String> {
        let mode: Mode = get_arg(args, 0)?;
        if let Some(state) = self.active_pane_state() {
            let buffer_id = state.buffer_id;
            if self.mode != Mode::Insert && mode == Mode::Insert {
                let key: LockToken =
                    query_service("buffer.lock", &[buffer_id.into()])?.try_into()?;
                self.key_holder = Some(key);
            }
            if self.mode == Mode::Insert && mode != Mode::Insert {
                let key = self.key_holder.unwrap();
                if let Err(e) = query_service("buffer.unlock", &[buffer_id.into(), key.into()]) {
                    panic!("fail buffer unlock: {e}");
                }
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
                data: mode.into(),
            },
            DispatchDescriptor::Broadcast,
        );
        emit_event(
            Event {
                kind: EventKind("request_redraw".to_string()),
                data: Value::Null,
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
        Ok(Value::Null)
    }
    #[service]
    fn cursor_move(&mut self, args: &[Value]) -> Result<Value, String> {
        let mv: CursorMove = get_arg(args, 0)?;
        match self.mode {
            Mode::Normal | Mode::Insert => {
                let Some(state) = self.active_pane_state_mut() else {
                    return Err("no active pane".to_string());
                };
                match mv {
                    CursorMove::Up { count } => {
                        let row = state.cursor.row;
                        state.cursor.row = row.saturating_sub(count);
                    }
                    CursorMove::Down { count } => {
                        state.cursor.row += count;
                    }
                    CursorMove::Left { count } => {
                        if count == 0 {
                            return Ok(Value::Null);
                        }
                        let line = state.line(state.cursor.row).unwrap();
                        let left = &line[..state.cursor.byte_col];
                        if let Some((i, _)) = left.char_indices().nth_back(count - 1) {
                            state.cursor.byte_col = i;
                        } else {
                            state.cursor.byte_col = 0;
                        }
                    }
                    CursorMove::Right { count } => {
                        let line = state.line(state.cursor.row).unwrap();
                        let right = &line[state.cursor.byte_col..];
                        if let Some((i, _)) = right.char_indices().nth(count) {
                            state.cursor.byte_col += i;
                        } else {
                            state.cursor.byte_col = line.len();
                        }
                    }
                    _ => {}
                }
                self.clamp_cursor();
            }
            Mode::Command => match mv {
                CursorMove::Left { count } => {
                    if count == 0 {
                        return Ok(Value::Null);
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
                _ => {}
            },
        }
        emit_event(
            Event {
                kind: EventKind("request_redraw".to_string()),
                data: Value::Null,
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
        Ok(Value::Null)
    }
    #[service]
    fn edit(&mut self, args: &[Value]) -> Result<Value, String> {
        let edit: EditAction = get_arg(args, 0)?;
        let key_holder = self.key_holder;
        let Some(state) = self.active_pane_state_mut() else {
            return Err("no active pane".to_string());
        };
        let mut cursor = state.cursor;
        match edit {
            EditAction::InsertText(text) => {
                query_service(
                    "buffer.insert",
                    &[
                        state.buffer_id.into(),
                        cursor.row.into(),
                        cursor.byte_col.into(),
                        text.clone().into(),
                        key_holder.into(),
                    ],
                )?;
                cursor.byte_col += text.len();
            }
            EditAction::DeleteCharForward => {
                let line = state.line(cursor.row).unwrap();
                let right_one = &line[cursor.byte_col..].chars().next();
                if let Some(c) = right_one {
                    // 通常
                    let next_size = c.len_utf8();
                    query_service(
                        "buffer.remove",
                        &[
                            state.buffer_id.into(),
                            cursor.row.into(),
                            cursor.byte_col.into(),
                            cursor.row.into(),
                            (cursor.byte_col + next_size).into(),
                            key_holder.into(),
                        ],
                    )?;
                } else {
                    // 行末
                    if cursor.row + 1 < state.len_lines() {
                        query_service(
                            "buffer.remove",
                            &[
                                state.buffer_id.into(),
                                cursor.row.into(),
                                cursor.byte_col.into(),
                                (cursor.row + 1).into(),
                                0_usize.into(),
                                key_holder.into(),
                            ],
                        )?;
                    }
                }
            }
            EditAction::DeleteCharBackward => {
                if cursor.row == 0 && cursor.byte_col == 0 {
                    return Ok(Value::Null);
                }

                let line = state.line(cursor.row).unwrap();
                let left_one = &line[..cursor.byte_col].chars().next_back();
                if let Some(c) = left_one {
                    // 通常
                    let prev_size = c.len_utf8();
                    query_service(
                        "buffer.remove",
                        &[
                            state.buffer_id.into(),
                            cursor.row.into(),
                            (cursor.byte_col - prev_size).into(),
                            cursor.row.into(),
                            cursor.byte_col.into(),
                            key_holder.into(),
                        ],
                    )?;
                    cursor.byte_col -= prev_size;
                } else {
                    // 行頭
                    if cursor.row != 0 {
                        let prev_line_len_bytes = state.line_len_bytes(cursor.row - 1).unwrap();
                        query_service(
                            "buffer.remove",
                            &[
                                state.buffer_id.into(),
                                (cursor.row - 1).into(),
                                prev_line_len_bytes.into(),
                                cursor.row.into(),
                                0_usize.into(),
                                key_holder.into(),
                            ],
                        )?;
                        cursor.row -= 1;
                        cursor.byte_col = prev_line_len_bytes;
                    }
                }
            }
            EditAction::NewLine => {
                query_service(
                    "buffer.insert",
                    &[
                        state.buffer_id.into(),
                        cursor.row.into(),
                        cursor.byte_col.into(),
                        "\n".to_string().into(),
                        key_holder.into(),
                    ],
                )?;
                cursor.row += 1;
                cursor.byte_col = 0;
            }
            EditAction::Undo => {
                query_service("buffer.undo", &[state.buffer_id.into()])?;
            }
            EditAction::Redo => {
                query_service("buffer.redo", &[state.buffer_id.into()])?;
            }
            _ => {}
        }
        state.cursor = cursor;
        self.clamp_cursor();
        emit_event(
            Event {
                kind: EventKind("request_redraw".to_string()),
                data: Value::Null,
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
        Ok(Value::Null)
    }
    #[service]
    fn command_line_action(&mut self, args: &[Value]) -> Result<Value, String> {
        let cmd_action: CommandLineAction = get_arg(args, 0)?;
        let command_line = &mut self.command_line;
        let cursor = self.command_line_cursor;
        match cmd_action {
            CommandLineAction::InsertText(text) => {
                command_line.insert_str(cursor, &text);
                self.command_line_cursor += text.len();
            }
            CommandLineAction::DeleteCharForward => {
                if command_line.len() < cursor {
                    return Ok(Value::Null);
                }
                let next_size = command_line[cursor..].chars().next().unwrap().len_utf8();
                command_line.drain(cursor..cursor + next_size);
            }
            CommandLineAction::DeleteCharBackward => {
                if cursor != 0 {
                    return Ok(Value::Null);
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
                kind: EventKind("request_redraw".to_string()),
                data: Value::Null,
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
        Ok(Value::Null)
    }
    #[service]
    fn switch_buffer(&mut self, args: &[Value]) -> Result<Value, String> {
        let Some(state) = self.active_pane_state_mut() else {
            return Err("no active pane".to_string());
        };
        let buffer_id: BufferId = get_arg(args, 0)?;
        if state.buffer_id == buffer_id {
            return Ok(Value::Null);
        }
        state.buffer_id = buffer_id;
        state.cursor = CursorState::default();
        self.clamp_cursor();
        emit_event(
            Event {
                kind: EventKind("request_redraw".to_string()),
                data: Value::Null,
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
        Ok(Value::Null)
    }
    #[service]
    fn attach_pane(&mut self, args: &[Value]) -> Result<Value, String> {
        let pane_id: PaneId = get_arg(args, 0)?;
        let path: Option<String> = get_arg(args, 1).unwrap_or_default();
        let buffer_id: BufferId = if let Some(path) = path {
            query_service("buffer.open", &[path.into()])?.try_into()?
        } else {
            query_service("buffer.create", &[])?.try_into()?
        };
        self.panes.insert(pane_id, PaneState::new(buffer_id));
        Ok(Value::Null)
    }
    #[service]
    fn render_pane(&mut self, args: &[Value]) -> Result<Value, String> {
        let pane_id: PaneId = get_arg(args, 0)?;
        let size: (u16, u16) = get_arg(args, 1)?;
        Ok(self
            .panes
            .get_mut(&pane_id)
            .unwrap()
            .render(self.mode, size)?
            .into())
    }
    #[service]
    fn split_pane(&mut self, args: &[Value]) -> Result<Value, String> {
        let new_id: PaneId = get_arg(args, 0)?;
        let source_id: PaneId = get_arg(args, 1)?;
        let Some(pane_state) = self.panes.get(&source_id) else {
            return Err(format!("modal don't have pane: {source_id:?}"));
        };
        self.panes
            .insert(new_id, PaneState::new(pane_state.buffer_id));
        Ok(Value::Null)
    }
    #[service]
    fn pane_active(&mut self, args: &[Value]) -> Result<Value, String> {
        assert!(self.active_pane.is_none());
        let pane_id: PaneId = get_arg(args, 0)?;
        self.active_pane = Some(pane_id);
        Ok(Value::Null)
    }
    #[service]
    fn pane_inactive(&mut self, args: &[Value]) -> Result<Value, String> {
        let pane_id: PaneId = get_arg(args, 0)?;
        assert_eq!(self.active_pane, Some(pane_id));
        if let Some(key) = self.key_holder {
            let state = self.active_pane_state().unwrap();
            if let Err(e) = query_service("buffer.unlock", &[state.buffer_id.into(), key.into()]) {
                panic!("fail buffer unlock: {e}");
            }
        }
        self.mode = Mode::Normal;
        self.active_pane = None;
        Ok(Value::Null)
    }
    #[service]
    fn mode(&self, _args: &[Value]) -> Result<Value, String> {
        Ok(self.mode.into())
    }
    #[service]
    fn command_line(&self, _args: &[Value]) -> Result<Value, String> {
        Ok(self.command_line.clone().into())
    }
    #[service]
    fn command_line_cursor(&self, _args: &[Value]) -> Result<Value, String> {
        Ok(self.command_line_cursor.into())
    }
}

impl Plugin for ModalPlugin {
    fn init(&mut self, reg: PluginRegistrar) {
        Self::register_service_and_subscribe(&reg);
        register_command(
            "q",
            Box::new(|_| {
                query_service("modal.quit", &[])?;
                Ok(())
            }),
        );
        register_command(
            "w",
            Box::new(|_| {
                let buffer_id = query_service("modal.buffer_id", &[])?;
                query_service("buffer.save", &[buffer_id])?;
                Ok(())
            }),
        );
        register_command(
            "x",
            Box::new(|_| {
                let buffer_id = query_service("modal.buffer_id", &[])?;
                query_service("buffer.save", &[buffer_id])?;
                query_service("modal.quit", &[])?;
                Ok(())
            }),
        );
        register_command(
            "e",
            Box::new(|args| {
                let path = if let Some(path) = args.first() {
                    path.clone()
                } else {
                    let current_id: usize = query_service("modal.buffer_id", &[])?.try_into()?;
                    let file_path: Option<String> =
                        query_service("buffer.file_path", &[current_id.into()])?.try_into()?;
                    let Some(file_path) = file_path else {
                        return Ok(());
                    };
                    file_path
                };

                let buffer_id = query_service("buffer.open", &[path.into()])?;
                query_service("modal.switch_buffer", &[buffer_id])?;
                Ok(())
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
