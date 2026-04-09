use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::buffer::{Buffer, BufferId};
use crate::command::CommandRegistry;

#[derive(Debug, Clone, Copy, Default)]
pub struct CursorState {
    pub row: usize,
    pub col: usize,
}
#[derive(Debug, Clone, Copy, Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Command,
}

pub struct EditorState {
    //pub buffers: BufferRegistry,
    // まずは1つのバッファー
    pub buffer: Buffer,
    pub cursor: CursorState,
    pub mode: Mode,
    pub running: bool,
    pub command_line: String,
    pub commands: CommandRegistry,
}
impl EditorState {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(BufferId(0)),
            cursor: CursorState::default(),
            mode: Mode::default(),
            running: true,
            command_line: String::new(),
            commands: CommandRegistry::new(),
        }
    }
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        Ok(Self {
            buffer: Buffer::from_file(BufferId(0), path)?,
            cursor: CursorState::default(),
            mode: Mode::default(),
            running: true,
            command_line: String::new(),
            commands: CommandRegistry::new(),
        })
    }
    pub fn clamp_cursor(&mut self) {
        let max_row = self.buffer_len_lines().saturating_sub(1);
        self.cursor.row = self.cursor.row.min(max_row);
        let max_col = match self.mode {
            Mode::Insert => self.buffer_line_len_chars(self.cursor.row),
            _ => self
                .buffer_line_len_chars(self.cursor.row)
                .saturating_sub(1),
        };
        self.cursor.col = self.cursor.col.min(max_col);
    }
}
impl PluginContext for EditorState {
    fn buffer_len_lines(&self) -> usize {
        self.buffer.rope().len_lines()
    }
    fn buffer_line(&self, row: usize) -> Option<String> {
        self.buffer
            .rope()
            .get_line(row)
            .map(|line| line.chars().collect())
    }
    fn buffer_line_len_chars(&self, row: usize) -> usize {
        let line = self.buffer.rope().line(row);
        let len = line.len_chars();
        if len > 0 && line.char(len - 1) == '\n' {
            len - 1
        }
        else {
            len
        }
    }
    fn buffer_insert_char(&mut self, row: usize, col: usize, ch: char) {
        debug_assert!(row < self.buffer_len_lines());
        debug_assert!(col <= self.buffer_line_len_chars(row));
        let buffer = &mut self.buffer;
        let char_idx = buffer.rope().line_to_char(row) + col;
        buffer.rope_mut().insert_char(char_idx, ch);
        if row == self.cursor.row && col <= self.cursor.col {
            self.cursor.col += 1;
        }
    }
    fn buffer_insert_char_at_cursor(&mut self, ch: char) {
        let buffer = &mut self.buffer;
        let char_idx =
            buffer.rope().line_to_char(self.cursor.row) + self.cursor.col;
        buffer.rope_mut().insert_char(char_idx, ch);
        self.cursor.col += 1;
    }
    fn buffer_backspace(&mut self) {
        let char_idx =
            self.buffer.rope().line_to_char(self.cursor.row) + self.cursor.col;
        if char_idx == 0 {
            return;
        }
        let char_idx = char_idx - 1;
        if self.cursor.col == 0 {
            self.cursor.row = self.cursor.row.saturating_sub(1);
            self.cursor.col = self.buffer_line_len_chars(self.cursor.row);
        }
        else {
            self.cursor.col -= 1;
        }
        self.buffer.rope_mut().remove(char_idx..char_idx + 1);
    }
    fn buffer_remove_range(
        &mut self,
        row: usize,
        col_start: usize,
        col_end: usize,
    ) {
        let char_idx = self.buffer.rope().line_to_char(row);
        self.buffer
            .rope_mut()
            .remove(char_idx + col_start..char_idx + col_end);
        self.clamp_cursor();
    }
    fn cursor_position(&self) -> (usize, usize) {
        (self.cursor.row, self.cursor.col)
    }
    fn cursor_set_position(&mut self, row: usize, col: usize) {
        self.cursor = CursorState { row, col };
        self.clamp_cursor();
    }
    fn cursor_up(&mut self, count: usize) {
        self.cursor.row = self.cursor.row.saturating_sub(count);
        self.clamp_cursor();
    }
    fn cursor_down(&mut self, count: usize) {
        self.cursor.row += count;
        self.clamp_cursor();
    }
    fn cursor_left(&mut self, count: usize) {
        self.cursor.col = self.cursor.col.saturating_sub(count);
        self.clamp_cursor();
    }
    fn cursor_right(&mut self, count: usize) {
        self.cursor.col += count;
        self.clamp_cursor();
    }
    fn mode(&self) -> Mode {
        self.mode
    }
    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }
    fn command_add_char(&mut self, ch: char) {
        self.command_line.push(ch);
    }
    fn command_clear(&mut self) {
        self.command_line.clear();
    }
    fn command_backspace(&mut self) {
        self.command_line.pop();
    }
    fn command_execute(&mut self) {
        match self.command_line.as_str() {
            "w" => {}
            "q" => {
                self.running = false;
            }
            _ => {}
        }
        self.command_line.clear();
        self.mode = Mode::Normal;
    }
    fn quit(&mut self) {
        self.running = false;
    }
}

pub type SharedState = Rc<RefCell<EditorState>>;

pub trait Plugin {}
pub trait PluginContext {
    // バッファー
    fn buffer_len_lines(&self) -> usize;
    fn buffer_line(&self, row: usize) -> Option<String>;
    fn buffer_line_len_chars(&self, row: usize) -> usize;
    fn buffer_insert_char(&mut self, row: usize, col: usize, ch: char);
    fn buffer_insert_char_at_cursor(&mut self, ch: char);
    fn buffer_backspace(&mut self);
    fn buffer_remove_range(
        &mut self,
        row: usize,
        col_start: usize,
        col_end: usize,
    );
    // カーソル
    fn cursor_position(&self) -> (usize, usize);
    fn cursor_set_position(&mut self, row: usize, col: usize);
    fn cursor_up(&mut self, count: usize);
    fn cursor_down(&mut self, count: usize);
    fn cursor_left(&mut self, count: usize);
    fn cursor_right(&mut self, count: usize);
    // モード
    fn mode(&self) -> Mode;
    fn set_mode(&mut self, mode: Mode);
    // コマンド
    fn command_add_char(&mut self, ch: char);
    fn command_clear(&mut self);
    fn command_backspace(&mut self);
    fn command_execute(&mut self);
    // 制御
    fn quit(&mut self);
}
