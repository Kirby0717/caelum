use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ropey::{Rope};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferId(pub usize);

#[derive(Debug)]
pub struct Buffer {
    rope: Rope,
    file_path: Option<PathBuf>,
    dirty: bool,
    id: BufferId,
}
impl Buffer {
    pub fn new(id: BufferId) -> Self {
        Self {
            rope: Rope::new(),
            file_path: None,
            dirty: false,
            id,
        }
    }
    pub fn from_file<P: AsRef<Path>>(
        id: BufferId,
        path: P,
    ) -> std::io::Result<Self> {
        let file_path = Some(path.as_ref().to_path_buf());
        let file = std::fs::File::open(path)?;
        Ok(Self {
            rope: Rope::from_reader(file)?,
            file_path,
            dirty: false,
            id,
        })
    }
    #[inline]
    pub fn rope(&self) -> &Rope {
        &self.rope
    }
}

#[derive(Debug)]
pub struct BufferRegistry {
    buffers: HashMap<BufferId, Buffer>,
    next_id: usize,
}
