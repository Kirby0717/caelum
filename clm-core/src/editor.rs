use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::buffer::{Buffer, BufferId};
use crate::command::CommandRegistry;
use crate::cursor::CursorStatue;
use crate::mode::Mode;

pub struct EditorState {
    //pub buffers: BufferRegistry,
    // まずは1つのバッファー
    pub buffer: Buffer,
    pub cursor: CursorStatue,
    pub mode: Mode,
    pub running: bool,
    pub command_line: String,
    pub commands: CommandRegistry,
}
impl EditorState {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(BufferId(0)),
            cursor: CursorStatue::default(),
            mode: Mode::default(),
            running: true,
            command_line: String::new(),
            commands: CommandRegistry::new(),
        }
    }
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        Ok(Self {
            buffer: Buffer::from_file(BufferId(0), path)?,
            cursor: CursorStatue::default(),
            mode: Mode::default(),
            running: true,
            command_line: String::new(),
            commands: CommandRegistry::new(),
        })
    }

    pub fn line_count(&self) -> usize {
        self.buffer.rope().len_lines()
    }
    pub fn line_len(&self, row: usize) -> usize {
        let line = self.buffer.rope().line(row);
        let len = line.len_chars();
        if len > 0 && line.char(len - 1) == '\n' {
            len - 1
        }
        else {
            len
        }
    }
    pub fn clamp_cursor(&mut self) {
        let max_row = self.line_count().saturating_sub(1);
        self.cursor.row = self.cursor.row.min(max_row);
        let max_col = self.line_len(self.cursor.row).saturating_sub(1);
        self.cursor.col = self.cursor.col.min(max_col);
    }
}

pub type SharedState = Rc<RefCell<EditorState>>;
