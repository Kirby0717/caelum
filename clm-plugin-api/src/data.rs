use clm_macros::ConvertValueInApi;
use serde::{Deserialize, Serialize};

pub mod id;
pub mod input;
pub mod tui_layout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ConvertValueInApi)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Normal,
    Insert,
}
impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Mode::Normal => "normal",
            Mode::Insert => "insert",
        };
        write!(f, "{s}")
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    ConvertValueInApi,
)]
pub struct BufferPosition {
    pub line_idx: usize,
    pub byte_col_idx: usize,
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
pub enum BufferChange {
    Insert {
        buffer_id: id::BufferId,
        start_position: BufferPosition,
        end_position: BufferPosition,
    },
    Remove {
        buffer_id: id::BufferId,
        position: BufferPosition,
        text: String,
    },
    Save(id::BufferId),
    Reset(id::BufferId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ConvertValueInApi)]
pub struct OpenFloatWindowConfig {
    pub float_window_handler: String,
    pub pane_handler: String,
}
