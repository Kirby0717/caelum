use std::collections::HashMap;
use std::path::{Path, PathBuf};

use clm_plugin_api::core::*;
use clm_plugin_api::priority;
use ropey::LineType::LF_CR;
use ropey::Rope;

#[derive(Debug)]
pub struct Buffer {
    history: Vec<Rope>,
    head: usize,
    file_path: Option<PathBuf>,
    dirty: bool,
    id: BufferId,
    lock_holder: Option<i64>,
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
    pub fn from_file<P: AsRef<Path>>(
        id: BufferId,
        path: P,
    ) -> std::io::Result<Self> {
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
            let file =
                std::fs::File::create(file_path).map_err(|e| e.to_string())?;
            let file = std::io::BufWriter::new(file);
            self.rope().write_to(file).map_err(|e| e.to_string())?;
            self.dirty = false;
            Ok(())
        }
        else {
            Err("no file name".to_string())
        }
    }
    pub fn rope(&self) -> &Rope {
        &self.history[self.head]
    }
    pub fn rope_mut(&mut self, key: Option<i64>) -> Option<&mut Rope> {
        if let Some(lock) = self.lock_holder {
            if key? != lock {
                return None;
            }
        }
        else {
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

    pub fn lock(&mut self) -> Result<i64, String> {
        if self.lock_holder.is_some() {
            return Err("buffer is locked".to_string());
        }
        let key = fastrand::i64(..);
        self.lock_holder = Some(key);
        self.history.truncate(self.head + 1);
        self.history.push(self.history[self.head].clone());
        self.head += 1;
        Ok(key)
    }
    pub fn is_locked(&self) -> bool {
        self.lock_holder.is_some()
    }
    pub fn unlock(&mut self, key: i64) -> Result<(), String> {
        let Some(lock) = self.lock_holder
        else {
            return Err("buffer is not locked".to_string());
        };
        if lock == key {
            self.lock_holder = None;
            Ok(())
        }
        else {
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
    pub fn get_buffer(&self, args: &[Value]) -> Result<&Buffer, Value> {
        let Some(Value::Int(id)) = args.first()
        else {
            return Err(Value::Error("arg error".to_string()));
        };
        let Some(buffer) = self.buffers.get(&BufferId(*id as usize))
        else {
            return Err(Value::Error("buffer not found".to_string()));
        };
        Ok(buffer)
    }
    #[inline(always)]
    pub fn get_buffer_mut(
        &mut self,
        args: &[Value],
    ) -> Result<&mut Buffer, Value> {
        let Some(Value::Int(id)) = args.first()
        else {
            return Err(Value::Error("arg error".to_string()));
        };
        let Some(buffer) = self.buffers.get_mut(&BufferId(*id as usize))
        else {
            return Err(Value::Error("buffer not found".to_string()));
        };
        Ok(buffer)
    }
}
#[clm_plugin_api::clm_handlers(name = "buffer")]
impl BufferPlugin {
    #[service]
    fn len_bytes(&self, args: &[Value]) -> Value {
        let buffer = match self.get_buffer(args) {
            Ok(buffer) => buffer,
            Err(err) => return err,
        };
        Value::Int(buffer.rope().len() as i64)
    }
    #[service]
    fn len_lines(&self, args: &[Value]) -> Value {
        let buffer = match self.get_buffer(args) {
            Ok(buffer) => buffer,
            Err(err) => return err,
        };
        Value::Int(buffer.rope().len_lines(LF_CR) as i64)
    }
    #[service]
    fn line(&self, args: &[Value]) -> Value {
        let buffer = match self.get_buffer(args) {
            Ok(buffer) => buffer,
            Err(err) => return err,
        };
        let Some(Value::Int(line_idx)) = args.get(1)
        else {
            return Value::Error("arg error".to_string());
        };
        if let Some(line) = buffer.rope().get_line(*line_idx as usize, LF_CR) {
            Value::Str(line.to_string())
        }
        else {
            Value::Null
        }
    }
    #[service]
    fn line_len_bytes(&self, args: &[Value]) -> Value {
        let buffer = match self.get_buffer(args) {
            Ok(buffer) => buffer,
            Err(err) => return err,
        };
        let Some(Value::Int(line_idx)) = args.get(1)
        else {
            return Value::Error("arg error".to_string());
        };
        if let Some(line) = buffer.rope().get_line(*line_idx as usize, LF_CR) {
            let mut len = line.len();
            if (*line_idx as usize) + 1 < buffer.rope().len_lines(LF_CR) {
                len -= 1;
            }
            Value::Int(len as i64)
        }
        else {
            Value::Null
        }
    }
    #[service]
    fn lines(&self, args: &[Value]) -> Value {
        let buffer = match self.get_buffer(args) {
            Ok(buffer) => buffer,
            Err(err) => return err,
        };
        let Some(Value::Int(start_line_idx)) = args.get(1)
        else {
            return Value::Error("arg error".to_string());
        };
        let Some(Value::Int(end_line_idx)) = args.get(2)
        else {
            return Value::Error("arg error".to_string());
        };
        let mut lines = vec![];
        for line_idx in *start_line_idx..*end_line_idx {
            if let Some(line) = buffer.rope().get_line(line_idx as usize, LF_CR)
            {
                lines.push(Value::Str(line.to_string()));
            }
            else {
                break;
            }
        }
        Value::List(lines)
    }
    #[service]
    fn pos_to_byte(&self, args: &[Value]) -> Value {
        let buffer = match self.get_buffer(args) {
            Ok(buffer) => buffer,
            Err(err) => return err,
        };
        let Some(Value::Int(line_idx)) = args.get(1)
        else {
            return Value::Error("arg error".to_string());
        };
        let Some(Value::Int(byte_col_idx)) = args.get(2)
        else {
            return Value::Error("arg error".to_string());
        };
        if buffer.rope().len_lines(LF_CR) < *line_idx as usize {
            return Value::Error("line_idx is out of range".to_string());
        }
        Value::Int(
            buffer.rope().line_to_byte_idx(*line_idx as usize, LF_CR) as i64
                + *byte_col_idx,
        )
    }
    #[service]
    fn byte_to_pos(&self, args: &[Value]) -> Value {
        let buffer = match self.get_buffer(args) {
            Ok(buffer) => buffer,
            Err(err) => return err,
        };
        let Some(Value::Int(byte_idx)) = args.get(1)
        else {
            return Value::Error("arg error".to_string());
        };
        if buffer.rope().len() < *byte_idx as usize {
            return Value::Error("byte_idx is out of range".to_string());
        }
        let line_idx =
            buffer.rope().byte_to_line_idx(*byte_idx as usize, LF_CR) as i64;
        let byte_col_idx = *byte_idx
            - buffer.rope().line_to_byte_idx(line_idx as usize, LF_CR) as i64;
        Value::Map(HashMap::from([
            ("line".to_string(), Value::Int(line_idx)),
            ("col".to_string(), Value::Int(byte_col_idx)),
        ]))
    }
    #[service]
    fn id_list(&self, _args: &[Value]) -> Value {
        Value::List(
            self.buffers
                .keys()
                .map(|id| Value::Int(id.0 as i64))
                .collect(),
        )
    }
    #[service]
    fn file_path(&self, args: &[Value]) -> Value {
        let buffer = match self.get_buffer(args) {
            Ok(buffer) => buffer,
            Err(err) => return err,
        };
        if let Some(file_path) = buffer.file_path() {
            Value::Str(file_path.to_string_lossy().to_string())
        }
        else {
            Value::Null
        }
    }
    #[subscribe(priority = priority::DEFAULT)]
    fn on_buffer_op(
        &mut self,
        data: &EventData,
        _ctx: &mut dyn PluginContext,
    ) -> EventResult {
        let EventData::BufferOp(buffer_op) = data
        else {
            return EventResult::Propagate;
        };
        match buffer_op {
            BufferOp::Insert {
                buffer_id,
                line_idx,
                byte_col_idx,
                text,
                key,
            } => {
                if let Some(buffer) = self.buffers.get_mut(buffer_id) {
                    let byte_idx =
                        buffer.rope().line_to_byte_idx(*line_idx, LF_CR)
                            + *byte_col_idx;
                    let Some(rope) = buffer.rope_mut(*key)
                    else {
                        return EventResult::Handled;
                    };
                    rope.insert(byte_idx, text);
                    emit_event(
                        Event {
                            kind: EventKind("buffer_changed".to_string()),
                            data: EventData::BufferChanged(
                                BufferChange::Insert {
                                    buffer_id: *buffer_id,
                                    start_line_idx: *line_idx,
                                    start_byte_col_idx: *byte_col_idx,
                                    end_line_idx: *line_idx,
                                    end_byte_col_idx: *byte_col_idx
                                        + text.len(),
                                },
                            ),
                        },
                        DispatchDescriptor::Broadcast,
                    );
                }
                else {
                    return EventResult::Propagate;
                }
            }
            BufferOp::Remove {
                buffer_id,
                start_line_idx,
                start_byte_col_idx,
                end_line_idx,
                end_byte_col_idx,
                key,
            } => {
                if let Some(buffer) = self.buffers.get_mut(buffer_id) {
                    let start_byte_idx =
                        buffer.rope().line_to_byte_idx(*start_line_idx, LF_CR)
                            + *start_byte_col_idx;
                    let end_byte_idx =
                        buffer.rope().line_to_byte_idx(*end_line_idx, LF_CR)
                            + *end_byte_col_idx;
                    let remove_text = buffer
                        .rope()
                        .slice(start_byte_idx..end_byte_idx)
                        .to_string();
                    let Some(rope) = buffer.rope_mut(*key)
                    else {
                        return EventResult::Handled;
                    };
                    rope.remove(start_byte_idx..end_byte_idx);
                    emit_event(
                        Event {
                            kind: EventKind("buffer_changed".to_string()),
                            data: EventData::BufferChanged(
                                BufferChange::Remove {
                                    buffer_id: *buffer_id,
                                    line_idx: *start_line_idx,
                                    byte_col_idx: *start_byte_col_idx,
                                    text: remove_text,
                                },
                            ),
                        },
                        DispatchDescriptor::Broadcast,
                    );
                }
                else {
                    return EventResult::Propagate;
                }
            }
            BufferOp::Undo(buffer_id) => {
                if let Some(buffer) = self.buffers.get_mut(buffer_id) {
                    buffer.undo();
                    emit_event(
                        Event {
                            kind: EventKind("buffer_changed".to_string()),
                            data: EventData::BufferChanged(
                                BufferChange::Reset(*buffer_id),
                            ),
                        },
                        DispatchDescriptor::Broadcast,
                    );
                }
                else {
                    return EventResult::Propagate;
                }
            }
            BufferOp::Redo(buffer_id) => {
                if let Some(buffer) = self.buffers.get_mut(buffer_id) {
                    buffer.redo();
                    emit_event(
                        Event {
                            kind: EventKind("buffer_changed".to_string()),
                            data: EventData::BufferChanged(
                                BufferChange::Reset(*buffer_id),
                            ),
                        },
                        DispatchDescriptor::Broadcast,
                    );
                }
                else {
                    return EventResult::Propagate;
                }
            }
            BufferOp::Close(buffer_id) => {
                if let Some(buffer) = self.buffers.get(buffer_id)
                    && buffer.is_locked()
                {
                    // TODO: エラー出力
                    return EventResult::Handled;
                }
                if let Some(_buffer) = self.buffers.remove(buffer_id) {
                    return EventResult::Handled;
                }
                else {
                    return EventResult::Propagate;
                }
            }
            BufferOp::Save(buffer_id) => {
                if let Some(buffer) = self.buffers.get_mut(buffer_id) {
                    let _ = buffer.save();
                    emit_event(
                        Event {
                            kind: EventKind("buffer_saved".to_string()),
                            data: EventData::BufferId(*buffer_id),
                        },
                        DispatchDescriptor::Broadcast,
                    );
                }
                else {
                    return EventResult::Propagate;
                }
            }
        }
        EventResult::Handled
    }
    #[service]
    fn lock(&mut self, args: &[Value]) -> Value {
        let buffer = match self.get_buffer_mut(args) {
            Ok(buffer) => buffer,
            Err(err) => return err,
        };
        buffer.lock().into()
    }
    #[service]
    fn is_locked(&self, args: &[Value]) -> Value {
        let buffer = match self.get_buffer(args) {
            Ok(buffer) => buffer,
            Err(err) => return err,
        };
        buffer.is_locked().into()
    }
    #[service]
    fn unlock(&mut self, args: &[Value]) -> Value {
        let buffer = match self.get_buffer_mut(args) {
            Ok(buffer) => buffer,
            Err(err) => return err,
        };
        let Some(Value::Int(key)) = args.get(1)
        else {
            return Value::Error("arg error".to_string());
        };
        buffer.unlock(*key).into()
    }
    #[service]
    fn create(&mut self, _args: &[Value]) -> Value {
        let id = BufferId(self.next_id);
        self.buffers.insert(id, Buffer::new(id));
        self.next_id += 1;
        Value::Int(id.0 as i64)
    }
    #[service]
    fn open(&mut self, args: &[Value]) -> Value {
        let Some(Value::Str(path)) = args.first()
        else {
            return Value::Error("arg error".to_string());
        };
        let id = BufferId(self.next_id);
        let buffer = match Buffer::from_file(id, path) {
            Ok(buffer) => buffer,
            Err(err) => {
                return Value::Error(err.to_string());
            }
        };
        self.buffers.insert(id, buffer);
        self.next_id += 1;
        Value::Int(id.0 as i64)
    }
}

impl Plugin for BufferPlugin {
    fn init(&mut self, reg: PluginRegistrar) {
        Self::register_service_and_subscribe(&reg);
    }
}
