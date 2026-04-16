use crate::editor::Mode;
use crate::input::KeyEvent;
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum EventData {
    None,
    Key(KeyEvent),
    Motion(CursorMove),
    Mode(Mode),
    Edit(EditAction),
    BufferOp(BufferOp),
    CommandLine(CommandLineAction),
    Custom(Value),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorMove {
    Up(usize),
    Down(usize),
    Left(usize),
    Right(usize),
    LineStart,
    LineEnd,
    FileTop,
    FileBottom,
    WordForward,
    WordBackward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferOp {
    Insert {
        buffer_id: BufferId,
        line_idx: usize,
        byte_col_idx: usize,
        text: String,
    },
    Remove {
        buffer_id: BufferId,
        start_line_idx: usize,
        start_byte_col_idx: usize,
        end_line_idx: usize,
        end_byte_col_idx: usize,
    },
    Close(BufferId),
    Save(BufferId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditAction {
    InsertChar(char),
    InsertText(String),
    DeleteCharForward,
    DeleteCharBackward,
    DeleteWord,
    NewLine,
    NewLineBelow,
    NewLineAbove,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineAction {
    AddChar(char),
    Backspace,
    Execute,
    Clear,
}
