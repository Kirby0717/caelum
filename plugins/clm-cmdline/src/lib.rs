use clm_plugin_api::core::*;
use clm_plugin_api::data::id::*;
use clm_plugin_api::data::input::*;
use clm_plugin_api::data::tui_layout::*;
use clm_plugin_api::data::*;

#[derive(Debug, Default)]
pub struct CommandLinePlugin {
    buffer: String,
    cursor: usize,
    float_id: Option<FloatId>,
    pane_id: Option<PaneId>,
}
impl CommandLinePlugin {
    pub fn new() -> Self {
        Self::default()
    }
}
#[clm_plugin_api::clm_handlers(name = "cmdline")]
impl CommandLinePlugin {
    #[service]
    fn attach_float_window(&mut self, float_id: FloatId) -> Result<(), String> {
        assert!(self.float_id.is_none());
        self.float_id = Some(float_id);
        Ok(())
    }
    #[service]
    fn is_focusable(&self) -> Result<bool, String> {
        Ok(true)
    }
    #[service]
    fn float_window_rect(
        &self,
        float_id: FloatId,
        terminal_size: (u16, u16),
    ) -> Result<Rect, String> {
        assert!(self.float_id == Some(float_id));

        // 幅60%範囲30～80、高さ3
        let size = (((terminal_size.0 as f64 * 0.6) as u16).clamp(30, 80), 3);
        // 真ん中ちょっと上
        let offset = (
            terminal_size.0.saturating_sub(size.0) / 2,
            (terminal_size.1 / 6).max(2),
        );

        let rect = Rect { offset, size };
        Ok(rect)
    }
    #[service]
    fn resolve_layout(
        &self,
        node: LayoutNode,
        float_window_size: (u16, u16),
    ) -> Result<ResolvedLayout, String> {
        assert!(matches!(node, LayoutNode::Pane(pane_id) if pane_id == self.pane_id.unwrap()));

        let cmdline_rect = Rect {
            offset: (1, 1),
            size: (float_window_size.0.saturating_sub(2), 1),
        };

        let mut commands = vec![];
        for row in 0..float_window_size.1 {
            if row == 0 {
                commands.push(DrawCommand::DrawString {
                    position: (0, row),
                    text: "╔".to_string()
                        + &"═".repeat(float_window_size.0.saturating_sub(2) as usize)
                        + "╗",
                });
            } else if 2 <= row && row + 1 == float_window_size.1 {
                commands.push(DrawCommand::DrawString {
                    position: (0, row),
                    text: "╚".to_string()
                        + &"═".repeat(float_window_size.0.saturating_sub(2) as usize)
                        + "╝",
                });
            } else {
                commands.push(DrawCommand::DrawString {
                    position: (0, row),
                    text: "║".to_string()
                        + &" ".repeat(float_window_size.0.saturating_sub(2) as usize)
                        + "║",
                });
            }
        }

        Ok(ResolvedLayout {
            pane_rects: vec![(self.pane_id.unwrap(), cmdline_rect)],
            back_draw_commands: commands,
        })
    }
    #[service]
    fn attach_pane(&mut self, pane_id: PaneId) -> Result<(), String> {
        assert!(self.pane_id.is_none());
        self.pane_id = Some(pane_id);
        self.buffer.clear();
        Ok(())
    }
    #[service]
    fn pane_active(&self, pane_id: PaneId) -> Result<(), String> {
        assert!(self.pane_id == Some(pane_id));
        query_service("keymap.add_pane", &[pane_id.into(), "cmdline".into()])?;
        Ok(())
    }
    #[service]
    fn pane_inactive(&mut self, pane_id: PaneId) -> Result<(), String> {
        query_service("keymap.remove_pane", &[pane_id.into()])?;
        Ok(())
    }
    #[service]
    fn render_pane(&self, pane_id: PaneId) -> Result<Vec<DrawCommand>, String> {
        use unicode_width::UnicodeWidthStr;
        assert!(self.pane_id == Some(pane_id));
        let x = self.buffer[..self.cursor].width() as u16;
        let commands = vec![
            DrawCommand::DrawString {
                position: (0, 0),
                text: ":".to_string() + &self.buffer,
            },
            DrawCommand::SetCursor {
                position: (1 + x, 0),
                style: CursorStyle::SteadyBar,
            },
        ];
        Ok(commands)
    }
    #[service]
    fn mode(&self) -> Result<String, String> {
        Ok("command".to_string())
    }
    #[service]
    fn execute(&mut self) -> Result<(), String> {
        let parsed = self
            .buffer
            .split_whitespace()
            .map(String::from)
            .collect::<Vec<_>>();
        if !parsed.is_empty() {
            execute_command(&parsed[0], &parsed[1..]);
        }
        self.buffer.clear();
        self.cursor = 0;
        // TODO: ペインとフロートウィンドウの削除
        Ok(())
    }
    #[service]
    fn cursor_move(&mut self, mv: CursorMove) -> Result<(), String> {
        match mv {
            CursorMove::Left { count } => {
                if count == 0 {
                    return Ok(());
                }
                if let Some((i, _)) = self.buffer[..self.cursor]
                    .char_indices()
                    .nth_back(count - 1)
                {
                    self.cursor = i;
                } else {
                    self.cursor = 0;
                }
            }
            CursorMove::Right { count } => {
                if let Some((i, _)) = self.buffer[self.cursor..].char_indices().nth(count) {
                    self.cursor += i;
                } else {
                    self.cursor = self.buffer.len();
                }
            }
            _ => return Ok(()),
        }
        request_redraw();
        Ok(())
    }
    #[service]
    fn key_input(&mut self, key_event: KeyEvent) -> Result<(), String> {
        match &key_event.logical_key {
            LogicalKey::Character(c) => {
                self.buffer += c;
                self.cursor += c.len();
            }
            LogicalKey::Named(named) => match named {
                NamedKey::Backspace => {
                    if self.cursor != 0 {
                        self.cursor_move(CursorMove::Left { count: 1 })?;
                        self.buffer.remove(self.cursor);
                    }
                }
                NamedKey::Delete => {
                    if self.cursor < self.buffer.len() {
                        self.buffer.remove(self.cursor);
                    }
                }
                _ => return Ok(()),
            },
            _ => return Ok(()),
        }
        request_redraw();
        Ok(())
    }
}
impl Plugin for CommandLinePlugin {
    fn init(&mut self, reg: PluginRegistrar) {
        Self::register_service_and_subscribe(reg);
    }
}
