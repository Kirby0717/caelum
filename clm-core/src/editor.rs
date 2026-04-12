use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::buffer::{Buffer, BufferId};
use crate::value::Value;

#[derive(Debug, Clone, Copy, Default)]
pub struct CursorState {
    pub row: usize,
    pub col: usize,
}
impl From<CursorState> for Value {
    fn from(value: CursorState) -> Self {
        Self::Map(
            vec![
                ("row".to_string(), Value::Int(value.row as i64)),
                ("col".to_string(), Value::Int(value.col as i64)),
            ]
            .into_iter()
            .collect(),
        )
    }
}
impl TryFrom<Value> for CursorState {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let Value::Map(cursor) = value
        else {
            return Err(());
        };
        let Some(Value::Int(row)) = cursor.get("row")
        else {
            return Err(());
        };
        let Some(Value::Int(col)) = cursor.get("col")
        else {
            return Err(());
        };
        Ok(CursorState {
            row: *row as usize,
            col: *col as usize,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Command,
}
impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Normal => write!(f, "normal"),
            Mode::Insert => write!(f, "insert"),
            Mode::Command => write!(f, "command"),
        }
    }
}
impl TryFrom<Value> for Mode {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let Value::Str(mode) = value
        else {
            return Err(());
        };
        Ok(match mode.as_str() {
            "normal" => Mode::Normal,
            "insert" => Mode::Insert,
            "command" => Mode::Command,
            _ => return Err(()),
        })
    }
}

pub struct EditorState {
    //pub buffers: BufferRegistry,
    // まずは1つのバッファー
    pub buffer: Buffer,
    pub running: bool,
}
impl EditorState {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(BufferId(0)),
            running: true,
        }
    }
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        Ok(Self {
            buffer: Buffer::from_file(BufferId(0), path)?,
            running: true,
        })
    }
}
impl PluginContext for EditorState {
    fn buffer_len_lines(&self) -> usize {
        self.buffer.rope().len_lines()
    }
    fn buffer_len_chars(&self) -> usize {
        self.buffer.rope().len_chars()
    }
    fn buffer_line(&self, line_idx: usize) -> Option<String> {
        self.buffer
            .rope()
            .get_line(line_idx)
            .map(|line| line.chars().collect())
    }
    fn buffer_line_len_chars(&self, line_idx: usize) -> usize {
        let line = self.buffer.rope().line(line_idx);
        let len = line.len_chars();
        if len > 0 && line.char(len - 1) == '\n' {
            len - 1
        }
        else {
            len
        }
    }
    fn buffer_line_to_char(&self, line_idx: usize) -> usize {
        self.buffer.rope().line_to_char(line_idx)
    }
    fn buffer_insert_char(&mut self, char_idx: usize, ch: char) {
        self.buffer.rope_mut().insert_char(char_idx, ch);
    }
    fn buffer_insert(&mut self, char_idx: usize, text: &str) {
        self.buffer.rope_mut().insert(char_idx, text);
    }
    fn buffer_remove(&mut self, char_range: (usize, usize)) {
        self.buffer.rope_mut().remove(char_range.0..char_range.1);
    }
    fn quit(&mut self) {
        self.running = false;
    }
}

pub type SharedState = Rc<RefCell<EditorState>>;

pub trait PluginContext {
    // バッファー
    fn buffer_len_lines(&self) -> usize;
    fn buffer_len_chars(&self) -> usize;
    fn buffer_line(&self, line_idx: usize) -> Option<String>;
    fn buffer_line_len_chars(&self, line_idx: usize) -> usize;
    fn buffer_line_to_char(&self, line_idx: usize) -> usize;
    fn buffer_insert_char(&mut self, char_idx: usize, ch: char);
    fn buffer_insert(&mut self, char_idx: usize, text: &str);
    fn buffer_remove(&mut self, char_range: (usize, usize));
    // 制御
    fn quit(&mut self);
}
