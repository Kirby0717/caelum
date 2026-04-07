use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ropey::Rope;

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
    #[inline]
    pub fn rope_mut(&mut self) -> &mut Rope {
        self.dirty = true;
        &mut self.rope
    }

    #[inline]
    pub fn id(&self) -> BufferId {
        self.id
    }
    #[inline]
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

#[derive(Debug, Default)]
pub struct BufferRegistry {
    buffers: HashMap<BufferId, Buffer>,
    next_id: usize,
}
impl BufferRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn create(&mut self) -> BufferId {
        let id = BufferId(self.next_id);
        self.next_id += 1;
        self.buffers.insert(id, Buffer::new(id));
        id
    }
    pub fn open<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> std::io::Result<BufferId> {
        let id = BufferId(self.next_id);
        self.next_id += 1;
        self.buffers.insert(id, Buffer::from_file(id, path)?);
        Ok(id)
    }
    pub fn get(&self, id: BufferId) -> Option<&Buffer> {
        self.buffers.get(&id)
    }
    pub fn get_mut(&mut self, id: BufferId) -> Option<&mut Buffer> {
        self.buffers.get_mut(&id)
    }
    pub fn remove(&mut self, id: BufferId) {
        self.buffers.remove(&id);
    }
}
