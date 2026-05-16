use std::collections::HashMap;
use std::path::{Path, PathBuf};

use clm_plugin_api::core::*;
use clm_plugin_api::data::id::*;
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
        let file_path = Some(path.as_ref().canonicalize()?);
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

    pub fn check_line_idx(&self, line_idx: usize) -> Result<(), String> {
        if self.rope().len_lines(LF_CR) < line_idx {
            return Err("line_idx is out of range".to_string());
        }
        Ok(())
    }
    pub fn check_byte_idx(&self, byte_idx: usize) -> Result<(), String> {
        if self.rope().len() < byte_idx {
            return Err("byte_idx is out of range".to_string());
        }
        Ok(())
    }
    pub fn convert_position(&self, position: BufferPosition) -> Result<usize, String> {
        self.check_line_idx(position.line_idx)?;
        let byte_idx =
            self.rope().line_to_byte_idx(position.line_idx, LF_CR) + position.byte_col_idx;
        self.check_byte_idx(byte_idx)?;
        Ok(byte_idx)
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

fn trim_end_newline_code(mut s: String) -> String {
    let trimmed = s.trim_end_matches(['\r', '\n']);
    s.truncate(trimmed.len());
    s
}

#[derive(Debug)]
pub struct BufferPlugin {
    buffers: HashMap<BufferId, Buffer>,
    next_id: usize,
}
impl Default for BufferPlugin {
    fn default() -> Self {
        Self::new()
    }
}
impl BufferPlugin {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
            next_id: 0,
        }
    }

    #[inline(always)]
    pub fn get_buffer(&self, buffer_id: BufferId) -> Result<&Buffer, String> {
        let Some(buffer) = self.buffers.get(&buffer_id) else {
            return Err("buffer not found".to_string());
        };
        Ok(buffer)
    }
    #[inline(always)]
    pub fn get_buffer_mut(&mut self, buffer_id: BufferId) -> Result<&mut Buffer, String> {
        let Some(buffer) = self.buffers.get_mut(&buffer_id) else {
            return Err("buffer not found".to_string());
        };
        Ok(buffer)
    }
}
#[clm_plugin_api::clm_handlers(name = "buffer")]
impl BufferPlugin {
    #[service]
    fn len_bytes(&self, buffer_id: BufferId) -> Result<usize, String> {
        let buffer = self.get_buffer(buffer_id)?;
        Ok(buffer.rope().len())
    }
    #[service]
    fn len_lines(&self, buffer_id: BufferId) -> Result<usize, String> {
        let buffer = self.get_buffer(buffer_id)?;
        Ok(buffer.rope().len_lines(LF_CR))
    }
    #[service]
    fn line(&self, buffer_id: BufferId, line_idx: usize) -> Result<String, String> {
        let buffer = self.get_buffer(buffer_id)?;
        buffer.check_line_idx(line_idx)?;
        Ok(trim_end_newline_code(
            buffer.rope().line(line_idx, LF_CR).to_string(),
        ))
    }
    #[service]
    fn line_len_bytes(&self, buffer_id: BufferId, line_idx: usize) -> Result<usize, String> {
        let buffer = self.get_buffer(buffer_id)?;
        buffer.check_line_idx(line_idx)?;
        let line = buffer.rope().line(line_idx, LF_CR);
        Ok(
            if let Some(byte_idx) = line.trailing_line_break_idx(LF_CR) {
                byte_idx
            } else {
                line.len()
            },
        )
    }
    #[service]
    fn lines(
        &self,
        buffer_id: BufferId,
        start_line_idx: usize,
        end_line_idx: usize,
    ) -> Result<Vec<String>, String> {
        let buffer = self.get_buffer(buffer_id)?;
        let mut lines = vec![];
        for line_idx in start_line_idx..end_line_idx {
            if let Some(line) = buffer.rope().get_line(line_idx, LF_CR) {
                lines.push(trim_end_newline_code(line.to_string()));
            } else {
                break;
            }
        }
        Ok(lines)
    }
    #[service]
    fn pos_to_byte(&self, buffer_id: BufferId, position: BufferPosition) -> Result<usize, String> {
        self.get_buffer(buffer_id)?.convert_position(position)
    }
    #[service]
    fn byte_to_pos(&self, buffer_id: BufferId, byte_idx: usize) -> Result<BufferPosition, String> {
        let buffer = self.get_buffer(buffer_id)?;
        buffer.check_byte_idx(byte_idx)?;
        let line_idx = buffer.rope().byte_to_line_idx(byte_idx, LF_CR);
        let byte_col_idx = byte_idx - buffer.rope().line_to_byte_idx(line_idx, LF_CR);
        Ok(BufferPosition {
            line_idx,
            byte_col_idx,
        })
    }
    #[service]
    fn id_list(&self) -> Result<Vec<BufferId>, String> {
        Ok(self.buffers.keys().copied().collect())
    }
    #[service]
    fn file_path(&self, buffer_id: BufferId) -> Result<Option<&Path>, String> {
        let buffer = self.get_buffer(buffer_id)?;
        Ok(buffer.file_path())
    }
    #[service]
    fn insert(
        &mut self,
        buffer_id: BufferId,
        position: BufferPosition,
        text: String,
        lock_token: Option<LockToken>,
    ) -> Result<(), String> {
        let buffer = self.get_buffer_mut(buffer_id)?;
        let byte_idx = buffer.convert_position(position)?;
        let Some(rope) = buffer.rope_mut(lock_token) else {
            return Err("this buffer is locked".to_string());
        };
        rope.insert(byte_idx, &text);
        let mut end_position = position;
        end_position.byte_col_idx += text.len();

        emit_event(
            Event {
                kind: EventKind("buffer_changed".to_string()),
                data: BufferChange::Insert {
                    buffer_id,
                    start_position: position,
                    end_position,
                }
                .into(),
            },
            DispatchDescriptor::Broadcast,
        );
        Ok(())
    }
    #[service]
    fn remove(
        &mut self,
        buffer_id: BufferId,
        start_position: BufferPosition,
        end_position: BufferPosition,
        lock_token: Option<LockToken>,
    ) -> Result<String, String> {
        let buffer = self.get_buffer_mut(buffer_id)?;
        let start_byte_idx = buffer.convert_position(start_position)?;
        let end_byte_idx = buffer.convert_position(end_position)?;
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
                data: BufferChange::Remove {
                    buffer_id,
                    position: start_position,
                    text: remove_text.clone(),
                }
                .into(),
            },
            DispatchDescriptor::Broadcast,
        );
        Ok(remove_text)
    }
    #[service]
    fn undo(&mut self, buffer_id: BufferId) -> Result<(), String> {
        let buffer = self.get_buffer_mut(buffer_id)?;
        buffer.undo();

        emit_event(
            Event {
                kind: EventKind("buffer_changed".to_string()),
                data: BufferChange::Reset(buffer_id).into(),
            },
            DispatchDescriptor::Broadcast,
        );
        Ok(())
    }
    #[service]
    fn redo(&mut self, buffer_id: BufferId) -> Result<(), String> {
        let buffer = self.get_buffer_mut(buffer_id)?;
        buffer.redo();

        emit_event(
            Event {
                kind: EventKind("buffer_changed".to_string()),
                data: BufferChange::Reset(buffer_id).into(),
            },
            DispatchDescriptor::Broadcast,
        );
        Ok(())
    }
    #[service]
    fn close(&mut self, buffer_id: BufferId) -> Result<(), String> {
        let buffer = self.get_buffer(buffer_id)?;
        if buffer.is_locked() {
            return Err("this buffer is locked".to_string());
        }
        if let Some(_buffer) = self.buffers.remove(&buffer_id) {
            Ok(())
        } else {
            Err("buffer not found".to_string())
        }
    }
    #[service]
    fn save(&mut self, buffer_id: BufferId) -> Result<(), String> {
        let buffer = self.get_buffer_mut(buffer_id)?;
        buffer.save()?;
        emit_event(
            Event {
                kind: EventKind("buffer_saved".to_string()),
                data: buffer_id.into(),
            },
            DispatchDescriptor::Broadcast,
        );
        Ok(())
    }
    #[service]
    fn lock(&mut self, buffer_id: BufferId) -> Result<LockToken, String> {
        let buffer = self.get_buffer_mut(buffer_id)?;
        buffer.lock()
    }
    #[service]
    fn is_locked(&self, buffer_id: BufferId) -> Result<bool, String> {
        let buffer = self.get_buffer(buffer_id)?;
        Ok(buffer.is_locked())
    }
    #[service]
    fn unlock(&mut self, buffer_id: BufferId, key: LockToken) -> Result<(), String> {
        let buffer = self.get_buffer_mut(buffer_id)?;
        buffer.unlock(key)
    }
    #[service]
    fn create(&mut self) -> Result<BufferId, String> {
        let id = BufferId(self.next_id);
        self.next_id += 1;
        self.buffers.insert(id, Buffer::new(id));
        Ok(id)
    }
    #[service]
    fn open(&mut self, path: String) -> Result<BufferId, String> {
        let id = BufferId(self.next_id);
        self.next_id += 1;
        let full_path = PathBuf::from(path)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        for (id, buffer) in &self.buffers {
            if let Some(buffer_path) = buffer.file_path()
                && buffer_path == full_path
            {
                return Ok(*id);
            }
        }
        let buffer = Buffer::from_file(id, full_path).map_err(|e| e.to_string())?;
        self.buffers.insert(id, buffer);
        Ok(id)
    }
}

impl Plugin for BufferPlugin {
    fn init(&mut self, reg: PluginRegistrar) {
        Self::register_service_and_subscribe(reg);
    }
}
