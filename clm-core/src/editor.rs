use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::buffer::BufferRegistry;
use crate::command::CommandRegistry;
use crate::cursor::CursorStatue;
use crate::mode::Mode;

pub struct EditorState {
    pub buffers: BufferRegistry,
    pub cursor: CursorStatue,
    pub mode: Mode,
    pub running: bool,
    pub command_line: String,
    pub commands: CommandRegistry,
}
impl EditorState {
    pub fn new<P: AsRef<Path>>(file: Option<P>) -> std::io::Result<Self> {
        let mut buffers = BufferRegistry::new();
        if let Some(path) = file {
            buffers.open(path)?;
        }
        Ok(Self {
            buffers,
            cursor: CursorStatue { position: 0 },
            mode: Mode::Normal,
            running: true,
            command_line: String::new(),
            commands: CommandRegistry::new(),
        })
    }
}

pub type SharedState = Rc<RefCell<EditorState>>;
