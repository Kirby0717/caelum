use crate::editor::Mode;
use crate::input::KeyEvent;
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum EventData {
    None,
    Key(KeyEvent),
    Motion(CursorMove),
    Mode(Mode),
    BufferOp(BufferOp),
    Edit(EditAction),
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferOp {
    Insert { char_idx: usize, text: String },
    Remove((usize, usize)),
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
