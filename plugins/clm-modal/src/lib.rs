use std::collections::HashMap;

use clm_plugin_api::core::*;
use clm_plugin_api::data::id::*;
use clm_plugin_api::data::tui_layout::*;
use clm_plugin_api::data::*;

#[derive(Debug)]
pub struct PaneState {
    pub buffer_id: BufferId,
    pub cursor: BufferPosition,
    pub view_offset: (usize, usize),
}
impl PaneState {
    fn new(buffer_id: BufferId) -> Self {
        Self {
            buffer_id,
            cursor: BufferPosition {
                line_idx: 0,
                byte_col_idx: 0,
            },
            view_offset: (0, 0),
        }
    }
    fn len_lines(&self) -> usize {
        query_service("buffer.len_lines", &[self.buffer_id.into()])
            .unwrap()
            .try_into()
            .unwrap()
    }
    fn line(&self, line_idx: usize) -> Option<String> {
        query_service("buffer.line", &[self.buffer_id.into(), line_idx.into()])
            .unwrap()
            .try_into()
            .unwrap()
    }
    fn line_len_bytes(&self, line_idx: usize) -> Option<usize> {
        query_service(
            "buffer.line_len_bytes",
            &[self.buffer_id.into(), line_idx.into()],
        )
        .unwrap()
        .try_into()
        .unwrap()
    }
    fn clamp_cursor(&mut self, mode: Mode) {
        let mut cursor = self.cursor;
        let max_lines = self.len_lines().saturating_sub(1);
        cursor.line_idx = cursor.line_idx.min(max_lines);
        let line = self.line(cursor.line_idx).unwrap();
        let max_col = match mode {
            Mode::Insert => line.len(),
            _ => line.char_indices().last().map_or(0, |(idx, _)| idx),
        };
        cursor.byte_col_idx = cursor.byte_col_idx.min(max_col);
        self.cursor = cursor;
    }
    fn render(&mut self, mode: Mode, size: (u16, u16)) -> Result<Vec<DrawCommand>, String> {
        use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

        let size = (size.0 as usize, size.1 as usize);
        let mut commands = vec![];

        let cursor = self.cursor;
        let buffer_id = self.buffer_id;
        let view_offset = &mut self.view_offset;
        let cursor_line: String =
            query_service_anyhow("buffer.line", &[buffer_id.into(), cursor.line_idx.into()])
                .unwrap()
                .try_into()
                .unwrap();

        // オフセットの計算
        {
            if cursor.line_idx < view_offset.1 {
                view_offset.1 = cursor.line_idx;
            }
            if view_offset.1 + size.1 <= cursor.line_idx {
                view_offset.1 = cursor.line_idx - (size.1 - 1);
            }
            let display_col_l = cursor_line[..cursor.byte_col_idx].width();
            let display_col_r = display_col_l
                + cursor_line[cursor.byte_col_idx..]
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
        let lines: Vec<String> = query_service_anyhow(
            "buffer.lines",
            &[
                buffer_id.into(),
                view_offset.1.into(),
                (view_offset.1 + size.1).into(),
            ],
        )
        .unwrap()
        .try_into()
        .unwrap();
        for (row, line) in lines.into_iter().enumerate() {
            commands.push(DrawCommand::DrawString {
                position: (0, row as u16),
                text: trim_display_range(&line, view_offset.0, view_offset.0 + size.0),
            });
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
                let x = cursor_line[..cursor.byte_col_idx].width();
                commands.push(DrawCommand::SetCursor {
                    position: (
                        x.saturating_sub(view_offset.0) as u16,
                        cursor.line_idx.saturating_sub(view_offset.1) as u16,
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
    fn quit(&self) -> Result<(), String> {
        quit();
        Ok(())
    }
    #[service]
    fn set_mode(&mut self, mode: Mode) -> Result<(), String> {
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
        Ok(())
    }
    #[service]
    fn cursor_move(&mut self, mv: CursorMove) -> Result<(), String> {
        match self.mode {
            Mode::Normal | Mode::Insert => {
                let Some(state) = self.active_pane_state_mut() else {
                    return Err("no active pane".to_string());
                };
                match mv {
                    CursorMove::Up { count } => {
                        let line_idx = state.cursor.line_idx;
                        state.cursor.line_idx = line_idx.saturating_sub(count);
                    }
                    CursorMove::Down { count } => {
                        state.cursor.line_idx += count;
                    }
                    CursorMove::Left { count } => {
                        if count == 0 {
                            return Ok(());
                        }
                        let line = state.line(state.cursor.line_idx).unwrap();
                        let left = &line[..state.cursor.byte_col_idx];
                        if let Some((i, _)) = left.char_indices().nth_back(count - 1) {
                            state.cursor.byte_col_idx = i;
                        } else {
                            state.cursor.byte_col_idx = 0;
                        }
                    }
                    CursorMove::Right { count } => {
                        let line = state.line(state.cursor.line_idx).unwrap();
                        let right = &line[state.cursor.byte_col_idx..];
                        if let Some((i, _)) = right.char_indices().nth(count) {
                            state.cursor.byte_col_idx += i;
                        } else {
                            state.cursor.byte_col_idx = line.len();
                        }
                    }
                    _ => {}
                }
                self.clamp_cursor();
            }
            Mode::Command => match mv {
                CursorMove::Left { count } => {
                    if count == 0 {
                        return Ok(());
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
        Ok(())
    }
    #[service]
    fn edit(&mut self, edit: EditAction) -> Result<(), String> {
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
                        cursor.into(),
                        text.clone().into(),
                        key_holder.into(),
                    ],
                )?;
                cursor.byte_col_idx += text.len();
            }
            EditAction::DeleteCharForward => {
                let line = state.line(cursor.line_idx).unwrap();
                let right_one = &line[cursor.byte_col_idx..].chars().next();
                let mut end_position = cursor;
                if let Some(c) = right_one {
                    // 通常
                    end_position.byte_col_idx += c.len_utf8();
                } else {
                    // 行末
                    end_position = BufferPosition {
                        line_idx: cursor.line_idx + 1,
                        byte_col_idx: 0,
                    };
                }
                query_service(
                    "buffer.remove",
                    &[
                        state.buffer_id.into(),
                        cursor.into(),
                        end_position.into(),
                        key_holder.into(),
                    ],
                )?;
            }
            EditAction::DeleteCharBackward => {
                let line = state.line(cursor.line_idx).unwrap();
                let left_one = &line[..cursor.byte_col_idx].chars().next_back();
                let end_position = cursor;
                if let Some(c) = left_one {
                    // 通常
                    cursor.byte_col_idx = cursor.byte_col_idx.saturating_sub(c.len_utf8());
                } else {
                    // 行頭
                    if cursor.line_idx != 0 {
                        cursor.line_idx -= 1;
                        let prev_line_len_bytes = state.line_len_bytes(cursor.line_idx).unwrap();
                        cursor.byte_col_idx = prev_line_len_bytes;
                    }
                }
                query_service(
                    "buffer.remove",
                    &[
                        state.buffer_id.into(),
                        cursor.into(),
                        end_position.into(),
                        key_holder.into(),
                    ],
                )?;
            }
            EditAction::NewLine => {
                query_service(
                    "buffer.insert",
                    &[
                        state.buffer_id.into(),
                        cursor.into(),
                        "\n".to_string().into(),
                        key_holder.into(),
                    ],
                )?;
                cursor.line_idx += 1;
                cursor.byte_col_idx = 0;
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
        Ok(())
    }
    #[service]
    fn command_line_action(&mut self, cmd_action: CommandLineAction) -> Result<(), String> {
        let command_line = &mut self.command_line;
        let cursor = self.command_line_cursor;
        match cmd_action {
            CommandLineAction::InsertText(text) => {
                command_line.insert_str(cursor, &text);
                self.command_line_cursor += text.len();
            }
            CommandLineAction::DeleteCharForward => {
                if command_line.len() < cursor {
                    return Ok(());
                }
                let next_size = command_line[cursor..].chars().next().unwrap().len_utf8();
                command_line.drain(cursor..cursor + next_size);
            }
            CommandLineAction::DeleteCharBackward => {
                if cursor != 0 {
                    return Ok(());
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
        Ok(())
    }
    #[service]
    fn switch_buffer(&mut self, buffer_id: BufferId) -> Result<(), String> {
        let Some(state) = self.active_pane_state_mut() else {
            return Err("no active pane".to_string());
        };
        if state.buffer_id == buffer_id {
            return Ok(());
        }
        state.buffer_id = buffer_id;
        state.cursor = BufferPosition {
            line_idx: 0,
            byte_col_idx: 0,
        };
        self.clamp_cursor();
        emit_event(
            Event {
                kind: EventKind("request_redraw".to_string()),
                data: Value::Null,
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
        Ok(())
    }
    #[service]
    fn open_command(&mut self) -> Result<(), String> {
        emit_event(
            Event {
                kind: EventKind("open_float_window".to_string()),
                data: OpenFloatWindowConfig {
                    float_window_handler: "cmdline".to_string(),
                    pane_handler: "cmdline".to_string(),
                }
                .into(),
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
        Ok(())
    }
    #[service]
    fn attach_pane(&mut self, pane_id: PaneId, path: Option<String>) -> Result<(), String> {
        let buffer_id: BufferId = if let Some(path) = path {
            query_service("buffer.open", &[path.into()])?.try_into()?
        } else {
            query_service("buffer.create", &[])?.try_into()?
        };
        self.panes.insert(pane_id, PaneState::new(buffer_id));
        Ok(())
    }
    #[service]
    fn render_pane(
        &mut self,
        pane_id: PaneId,
        size: (u16, u16),
    ) -> Result<Vec<DrawCommand>, String> {
        self.panes
            .get_mut(&pane_id)
            .unwrap()
            .render(self.mode, size)
    }
    #[service]
    fn split_pane(&mut self, new_id: PaneId, source_id: PaneId) -> Result<(), String> {
        let Some(pane_state) = self.panes.get(&source_id) else {
            return Err(format!("modal don't have pane: {source_id:?}"));
        };
        self.panes
            .insert(new_id, PaneState::new(pane_state.buffer_id));
        Ok(())
    }
    #[service]
    fn pane_active(&mut self, pane_id: PaneId) -> Result<(), String> {
        assert!(self.active_pane.is_none());
        self.active_pane = Some(pane_id);
        Ok(())
    }
    #[service]
    fn pane_inactive(&mut self, pane_id: PaneId) -> Result<(), String> {
        assert_eq!(self.active_pane, Some(pane_id));
        if let Some(key) = self.key_holder {
            let state = self.active_pane_state().unwrap();
            if let Err(e) = query_service("buffer.unlock", &[state.buffer_id.into(), key.into()]) {
                panic!("fail buffer unlock: {e}");
            }
        }
        self.mode = Mode::Normal;
        self.active_pane = None;
        Ok(())
    }
    #[service]
    fn mode(&self) -> Result<Mode, String> {
        Ok(self.mode)
    }
    #[service]
    fn command_line(&self) -> Result<String, String> {
        Ok(self.command_line.clone())
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
