use crate::command::CommandRegistry;
use crate::editor::SharedState;
use crate::event::EventBus;
use crate::mode::Mode;

pub trait Plugin {
    fn init(
        &mut self,
        state: SharedState,
        bus: &mut EventBus,
        commands: &mut CommandRegistry,
    );
}

pub trait PluginContext {
    // バッファー
    fn buffer_len_lines(&self) -> usize;
    fn buffer_line(&self, row: usize) -> Option<String>;
    fn buffer_line_len_chars(&self, row: usize) -> usize;
    fn buffer_insert_char(&mut self, row: usize, col: usize, ch: char);
    fn buffer_insert_char_at_cursor(&mut self, ch: char);
    fn buffer_backspace(&mut self);
    fn buffer_remove_range(
        &mut self,
        row: usize,
        col_start: usize,
        col_end: usize,
    );
    // カーソル
    fn cursor_position(&self) -> (usize, usize);
    fn cursor_set_position(&mut self, row: usize, col: usize);
    fn cursor_up(&mut self, count: usize);
    fn cursor_down(&mut self, count: usize);
    fn cursor_left(&mut self, count: usize);
    fn cursor_right(&mut self, count: usize);
    // モード
    fn mode(&self) -> Mode;
    fn set_mode(&mut self, mode: Mode);
    // コマンド
    fn command_add_char(&mut self, ch: char);
    fn command_clear(&mut self);
    fn command_backspace(&mut self);
    fn command_execute(&mut self);
    // 制御
    fn quit(&mut self);
}
