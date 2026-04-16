use std::cell::RefCell;
use std::rc::Rc;

use crate::value::Value;

#[derive(Debug, Clone, Copy, Default)]
pub struct CursorState {
    pub row: usize,
    pub byte_col: usize,
}
impl From<CursorState> for Value {
    fn from(value: CursorState) -> Self {
        Self::Map(
            vec![
                ("row".to_string(), Value::Int(value.row as i64)),
                ("byte_col".to_string(), Value::Int(value.byte_col as i64)),
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
        let Some(Value::Int(byte_col)) = cursor.get("byte_col")
        else {
            return Err(());
        };
        Ok(CursorState {
            row: *row as usize,
            byte_col: *byte_col as usize,
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
    pub running: bool,
}
impl EditorState {
    pub fn new() -> Self {
        Self { running: true }
    }
}
impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedState = Rc<RefCell<EditorState>>;

impl PluginContext for EditorState {
    fn quit(&mut self) {
        self.running = false;
    }
}
pub trait PluginContext {
    // 制御
    fn quit(&mut self);
}
