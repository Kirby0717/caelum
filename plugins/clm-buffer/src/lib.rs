use std::collections::HashMap;
use std::path::{Path, PathBuf};

use clm_plugin_api::core::*;
use clm_plugin_api::data::*;
use ropey::LineType::LF_CR;
use ropey::Rope;

#[derive(Debug)]
pub struct Buffer {
    history: Vec<Rope>,
    head: usize,
    file_path: Option<PathBuf>,
    dirty: bool,
    id: BufferId,
    lock_holder: Option<LockToken>,
}
impl Buffer {
    pub fn new(id: BufferId) -> Self {
        Self {
            history: vec![Rope::new()],
            head: 0,
            file_path: None,
            dirty: false,
            id,
            lock_holder: None,
        }
    }
    pub fn from_file<P: AsRef<Path>>(id: BufferId, path: P) -> std::io::Result<Self> {
        let file_path = Some(path.as_ref().to_path_buf());
        let file = std::fs::File::open(path)?;
        Ok(Self {
            history: vec![Rope::from_reader(file)?],
            head: 0,
            file_path,
            dirty: false,
            id,
            lock_holder: None,
        })
    }
    pub fn save(&mut self) -> Result<(), String> {
        if self.is_locked() {
            return Err("buffer is locked".to_string());
        }
        if let Some(file_path) = &self.file_path {
            let file = std::fs::File::create(file_path).map_err(|e| e.to_string())?;
            let file = std::io::BufWriter::new(file);
            self.rope().write_to(file).map_err(|e| e.to_string())?;
            self.dirty = false;
            Ok(())
        } else {
            Err("no file name".to_string())
        }
    }
    pub fn rope(&self) -> &Rope {
        &self.history[self.head]
    }
    pub fn rope_mut(&mut self, key: Option<LockToken>) -> Option<&mut Rope> {
        if let Some(lock) = self.lock_holder {
            if key? != lock {
                return None;
            }
        } else {
            self.history.truncate(self.head + 1);
            self.history.push(self.history[self.head].clone());
            self.head += 1;
        }
        self.dirty = true;
        Some(&mut self.history[self.head])
    }

    pub fn id(&self) -> BufferId {
        self.id
    }
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn lock(&mut self) -> Result<LockToken, String> {
        if self.lock_holder.is_some() {
            return Err("buffer is locked".to_string());
        }
        let token = LockToken(fastrand::u64(..));
        self.lock_holder = Some(token);
        self.history.truncate(self.head + 1);
        self.history.push(self.history[self.head].clone());
        self.head += 1;
        Ok(token)
    }
    pub fn is_locked(&self) -> bool {
        self.lock_holder.is_some()
    }
    pub fn unlock(&mut self, key: LockToken) -> Result<(), String> {
        let Some(lock) = self.lock_holder else {
            return Err("buffer is not locked".to_string());
        };
        if lock == key {
            self.lock_holder = None;
            Ok(())
        } else {
            Err("invalid key".to_string())
        }
    }

    pub fn undo(&mut self) {
        if self.head != 0 {
            self.head -= 1;
        }
    }
    pub fn redo(&mut self) {
        if self.head + 1 != self.history.len() {
            self.head += 1;
        }
    }
}

#[derive(Debug, Default)]
pub struct BufferPlugin {
    buffers: HashMap<BufferId, Buffer>,
    next_id: usize,
}
impl BufferPlugin {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub fn get_buffer(&self, args: &[Value]) -> Result<&Buffer, String> {
        let id: BufferId = get_arg(args, 0)?;
        let Some(buffer) = self.buffers.get(&id) else {
            return Err("buffer not found".to_string());
        };
        Ok(buffer)
    }
    #[inline(always)]
    pub fn get_buffer_mut(&mut self, args: &[Value]) -> Result<&mut Buffer, String> {
        let id: BufferId = get_arg(args, 0)?;
        let Some(buffer) = self.buffers.get_mut(&id) else {
            return Err("buffer not found".to_string());
        };
        Ok(buffer)
    }
}
#[clm_plugin_api::clm_handlers(name = "buffer")]
impl BufferPlugin {
    #[service]
    fn len_bytes(&self, args: &[Value]) -> Result<Value, String> {
        let buffer = self.get_buffer(args)?;
        Ok(buffer.rope().len().into())
    }
    #[service]
    fn len_lines(&self, args: &[Value]) -> Result<Value, String> {
        let buffer = self.get_buffer(args)?;
        Ok(buffer.rope().len_lines(LF_CR).into())
    }
    #[service]
    fn line(&self, args: &[Value]) -> Result<Value, String> {
        let buffer = self.get_buffer(args)?;
        let line_idx: usize = get_arg(args, 1)?;
        Ok(buffer
            .rope()
            .get_line(line_idx, LF_CR)
            .map(|line| line.to_string())
            .into())
    }
    #[service]
    fn line_len_bytes(&self, args: &[Value]) -> Result<Value, String> {
        let buffer = self.get_buffer(args)?;
        let line_idx: usize = get_arg(args, 1)?;
        Ok(buffer
            .rope()
            .get_line(line_idx, LF_CR)
            .map(|line| line.len())
            .into())
    }
    #[service]
    fn lines(&self, args: &[Value]) -> Result<Value, String> {
        let buffer = self.get_buffer(args)?;
        let start_line_idx: usize = get_arg(args, 1)?;
        let end_line_idx: usize = get_arg(args, 2)?;
        let mut lines = vec![];
        for line_idx in start_line_idx..end_line_idx {
            if let Some(line) = buffer.rope().get_line(line_idx, LF_CR) {
                lines.push(Value::Str(line.to_string()));
            } else {
                break;
            }
        }
        Ok(lines.into())
    }
    #[service]
    fn pos_to_byte(&self, args: &[Value]) -> Result<Value, String> {
        let buffer = self.get_buffer(args)?;
        let line_idx: usize = get_arg(args, 1)?;
        let byte_col_idx: usize = get_arg(args, 2)?;
        if buffer.rope().len_lines(LF_CR) < line_idx {
            return Err("line_idx is out of range".to_string());
        }
        let byte_idx = buffer.rope().line_to_byte_idx(line_idx, LF_CR) + byte_col_idx;
        Ok(byte_idx.into())
    }
    #[service]
    fn byte_to_pos(&self, args: &[Value]) -> Result<Value, String> {
        let buffer = self.get_buffer(args)?;
        let byte_idx: usize = get_arg(args, 1)?;
        if buffer.rope().len() < byte_idx {
            return Err("byte_idx is out of range".to_string());
        }
        let line_idx = buffer.rope().byte_to_line_idx(byte_idx, LF_CR);
        let byte_col_idx = byte_idx - buffer.rope().line_to_byte_idx(line_idx, LF_CR);
        Ok(HashMap::from([
            ("line".to_string(), line_idx),
            ("col".to_string(), byte_col_idx),
        ])
        .into())
    }
    #[service]
    fn id_list(&self, _args: &[Value]) -> Result<Value, String> {
        Ok(self
            .buffers
            .keys()
            .copied()
            .map(Value::from)
            .collect::<Vec<_>>()
            .into())
    }
    #[service]
    fn file_path(&self, args: &[Value]) -> Result<Value, String> {
        let buffer = self.get_buffer(args)?;
        Ok(buffer
            .file_path()
            .map(|file_path| file_path.to_string_lossy().to_string())
            .into())
    }
    #[service]
    fn insert(&mut self, args: &[Value]) -> Result<Value, String> {
        let buffer_id: BufferId = get_arg(args, 0)?;
        let Some(buffer) = self.buffers.get_mut(&buffer_id) else {
            return Err("buffer not found".to_string());
        };
        let line_idx: usize = get_arg(args, 1)?;
        let byte_col_idx: usize = get_arg(args, 2)?;
        let text: String = get_arg(args, 3)?;
        let lock_token: Option<LockToken> = get_arg(args, 4)?;

        let byte_idx = buffer.rope().line_to_byte_idx(line_idx, LF_CR) + byte_col_idx;
        let Some(rope) = buffer.rope_mut(lock_token) else {
            return Err("this buffer is locked".to_string());
        };
        rope.insert(byte_idx, &text);
        emit_event(
            Event {
                kind: EventKind("buffer_changed".to_string()),
                data: to_value(&BufferChange::Insert {
                    buffer_id,
                    start_line_idx: line_idx,
                    start_byte_col_idx: byte_col_idx,
                    end_line_idx: line_idx,
                    end_byte_col_idx: byte_col_idx + text.len(),
                })
                .unwrap(),
            },
            DispatchDescriptor::Broadcast,
        );
        Ok(Value::Null)
    }
    #[service]
    fn remove(&mut self, args: &[Value]) -> Result<Value, String> {
        let buffer_id: BufferId = get_arg(args, 0)?;
        let Some(buffer) = self.buffers.get_mut(&buffer_id) else {
            return Err("buffer not found".to_string());
        };
        let start_line_idx: usize = get_arg(args, 1)?;
        let start_byte_col_idx: usize = get_arg(args, 2)?;
        let end_line_idx: usize = get_arg(args, 3)?;
        let end_byte_col_idx: usize = get_arg(args, 4)?;
        let lock_token: Option<LockToken> = get_arg(args, 5)?;

        let start_byte_idx =
            buffer.rope().line_to_byte_idx(start_line_idx, LF_CR) + start_byte_col_idx;
        let end_byte_idx = buffer.rope().line_to_byte_idx(end_line_idx, LF_CR) + end_byte_col_idx;
        let remove_text = buffer
            .rope()
            .slice(start_byte_idx..end_byte_idx)
            .to_string();
        let Some(rope) = buffer.rope_mut(lock_token) else {
            return Err("this buffer is locked".to_string());
        };
        rope.remove(start_byte_idx..end_byte_idx);
        emit_event(
            Event {
                kind: EventKind("buffer_changed".to_string()),
                data: to_value(&BufferChange::Remove {
                    buffer_id,
                    line_idx: start_line_idx,
                    byte_col_idx: start_byte_col_idx,
                    text: remove_text,
                })
                .unwrap(),
            },
            DispatchDescriptor::Broadcast,
        );
        Ok(Value::Null)
    }
    #[service]
    fn undo(&mut self, args: &[Value]) -> Result<Value, String> {
        let buffer_id: BufferId = get_arg(args, 0)?;
        let Some(buffer) = self.buffers.get_mut(&buffer_id) else {
            return Err("buffer not found".to_string());
        };

        buffer.undo();
        emit_event(
            Event {
                kind: EventKind("buffer_changed".to_string()),
                data: to_value(&BufferChange::Reset(buffer_id)).unwrap(),
            },
            DispatchDescriptor::Broadcast,
        );

        Ok(Value::Null)
    }
    #[service]
    fn redo(&mut self, args: &[Value]) -> Result<Value, String> {
        let buffer_id: BufferId = get_arg(args, 0)?;
        let Some(buffer) = self.buffers.get_mut(&buffer_id) else {
            return Err("buffer not found".to_string());
        };

        buffer.redo();
        emit_event(
            Event {
                kind: EventKind("buffer_changed".to_string()),
                data: to_value(&BufferChange::Reset(buffer_id)).unwrap(),
            },
            DispatchDescriptor::Broadcast,
        );

        Ok(Value::Null)
    }
    #[service]
    fn close(&mut self, args: &[Value]) -> Result<Value, String> {
        let buffer_id: BufferId = get_arg(args, 0)?;
        if let Some(buffer) = self.buffers.get(&buffer_id)
            && buffer.is_locked()
        {
            return Err("this buffer is locked".to_string());
        }
        if let Some(_buffer) = self.buffers.remove(&buffer_id) {
            Ok(Value::Null)
        } else {
            Err("buffer not found".to_string())
        }
    }
    #[service]
    fn save(&mut self, args: &[Value]) -> Result<Value, String> {
        let buffer_id: BufferId = get_arg(args, 0)?;
        let Some(buffer) = self.buffers.get_mut(&buffer_id) else {
            return Err("buffer not found".to_string());
        };
        buffer.save()?;
        emit_event(
            Event {
                kind: EventKind("buffer_saved".to_string()),
                data: to_value(&buffer_id).unwrap(),
            },
            DispatchDescriptor::Broadcast,
        );
        Ok(Value::Null)
    }
    #[service]
    fn lock(&mut self, args: &[Value]) -> Result<Value, String> {
        let buffer = self.get_buffer_mut(args)?;
        Ok(buffer.lock()?.into())
    }
    #[service]
    fn is_locked(&self, args: &[Value]) -> Result<Value, String> {
        let buffer = self.get_buffer(args)?;
        Ok(buffer.is_locked().into())
    }
    #[service]
    fn unlock(&mut self, args: &[Value]) -> Result<Value, String> {
        let buffer = self.get_buffer_mut(args)?;
        let key: LockToken = get_arg(args, 1)?;
        Ok(buffer.unlock(key)?.into())
    }
    #[service]
    fn create(&mut self, _args: &[Value]) -> Result<Value, String> {
        let id = BufferId(self.next_id);
        self.buffers.insert(id, Buffer::new(id));
        self.next_id += 1;
        Ok(id.into())
    }
    #[service]
    fn open(&mut self, args: &[Value]) -> Result<Value, String> {
        let path: String = get_arg(args, 0)?;
        let id = BufferId(self.next_id);
        let buffer = match Buffer::from_file(id, path) {
            Ok(buffer) => buffer,
            Err(err) => {
                return Err(err.to_string());
            }
        };
        self.buffers.insert(id, buffer);
        self.next_id += 1;
        Ok(id.into())
    }
}

impl Plugin for BufferPlugin {
    fn init(&mut self, reg: PluginRegistrar) {
        Self::register_service_and_subscribe(&reg);
    }
}
