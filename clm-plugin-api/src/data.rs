use clm_macros::ConvertValueInApi;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ConvertValueInApi)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Command,
}
impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Mode::Normal => "normal",
            Mode::Insert => "insert",
            Mode::Command => "command",
        };
        write!(f, "{s}")
    }
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, ConvertValueInApi)]
pub struct CursorState {
    pub row: usize,
    pub byte_col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ConvertValueInApi)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CursorMove {
    Up {
        #[serde(default = "one")]
        count: usize,
    },
    Down {
        #[serde(default = "one")]
        count: usize,
    },
    Left {
        #[serde(default = "one")]
        count: usize,
    },
    Right {
        #[serde(default = "one")]
        count: usize,
    },
    LineStart,
    LineEnd,
    FileTop,
    FileBottom,
    WordForward,
    WordBackward,
}
fn one() -> usize {
    1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ConvertValueInApi)]
pub struct BufferId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ConvertValueInApi)]
pub struct LockToken(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ConvertValueInApi)]
#[serde(rename_all = "snake_case")]
pub enum EditAction {
    InsertText(String),
    DeleteCharForward,
    DeleteCharBackward,
    DeleteWord,
    NewLine,
    NewLineBelow,
    NewLineAbove,
    Undo,
    Redo,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ConvertValueInApi)]
#[serde(rename_all = "snake_case")]
pub enum CommandLineAction {
    InsertText(String),
    DeleteCharForward,
    DeleteCharBackward,
    Execute,
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ConvertValueInApi)]
pub enum BufferChange {
    Insert {
        buffer_id: BufferId,
        start_line_idx: usize,
        start_byte_col_idx: usize,
        end_line_idx: usize,
        end_byte_col_idx: usize,
    },
    Remove {
        buffer_id: BufferId,
        line_idx: usize,
        byte_col_idx: usize,
        text: String,
    },
    Save(BufferId),
    Reset(BufferId),
}
