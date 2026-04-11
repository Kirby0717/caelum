use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::buffer::{Buffer, BufferId};
use crate::registry::{emit_event, execute_command};
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
    pub command_line: String,
}
impl EditorState {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(BufferId(0)),
            running: true,
            command_line: String::new(),
        }
    }
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        Ok(Self {
            buffer: Buffer::from_file(BufferId(0), path)?,
            running: true,
            command_line: String::new(),
        })
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
    }
    /*fn buffer_insert_char_at_cursor(&mut self, ch: char) {
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
    }*/
    fn buffer_remove(&mut self, char_range: (usize, usize)) {
        self.buffer.rope_mut().remove(char_range.0..char_range.1);
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
        execute_command(&self.command_line, &[]);
        self.command_line.clear();
        emit_event(
            crate::event::Event {
                kind: crate::event::EventKind("set_mode".to_string()),
                data: crate::event::EventData::Mode(Mode::Normal),
            },
            crate::event::DispatchDescriptor {
                consumable: true,
                sort_keys: vec![crate::event::SortKey("priority".to_string())],
            },
        );
    }
    fn quit(&mut self) {
        self.running = false;
    }
}

pub type SharedState = Rc<RefCell<EditorState>>;

pub trait PluginContext {
    // バッファー
    fn buffer_len_lines(&self) -> usize;
    fn buffer_line(&self, row: usize) -> Option<String>;
    fn buffer_line_len_chars(&self, row: usize) -> usize;
    fn buffer_insert_char(&mut self, row: usize, col: usize, ch: char);
    fn buffer_remove(&mut self, char_range: (usize, usize));
    /*
    fn buffer_insert_char_at_cursor(&mut self, ch: char);
    fn buffer_backspace(&mut self);
    */
    // コマンド
    fn command_add_char(&mut self, ch: char);
    fn command_clear(&mut self);
    fn command_backspace(&mut self);
    fn command_execute(&mut self);
    // 制御
    fn quit(&mut self);
}
