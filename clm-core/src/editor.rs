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

impl PluginContext for EditorState {
    fn quit(&mut self) {
        self.running = false;
    }
}
pub trait PluginContext {
    fn quit(&mut self);
}
