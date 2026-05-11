use clm_plugin_api::core::*;
use clm_plugin_api::data::id::*;
use clm_plugin_api::data::tui_layout::*;
use clm_tui_compositor::DrawCommand;

#[derive(Debug, Default)]
pub struct CommandLinePlugin {
    buffer: String,
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
    fn attach_float_window(&mut self, args: &[Value]) -> Result<Value, String> {
        let float_id = get_arg(args, 0)?;
        assert!(self.float_id.is_none());
        self.float_id = Some(float_id);
        Ok(Value::Null)
    }
    #[service]
    fn is_focusable(&mut self, _args: &[Value]) -> Result<Value, String> {
        Ok(true.into())
    }
    #[service]
    fn float_window_rect(&mut self, args: &[Value]) -> Result<Value, String> {
        let terminal_size: (u16, u16) = get_arg(args, 1)?;

        // 幅60%範囲30～80、高さ3
        let size = (
            ((terminal_size.0 as f64 * 0.6) as u16)
                .clamp(30, 80)
                .min(terminal_size.0.saturating_sub(2)),
            3.min(terminal_size.1),
        );
        // 真ん中ちょっと上
        let offset = (
            terminal_size.0.saturating_sub(size.0) / 2,
            (terminal_size.1 / 6)
                .max(2)
                .min(terminal_size.1.saturating_sub(size.1)),
        );

        let rect = Rect { offset, size };
        Ok(rect.into())
    }
    #[service]
    fn resolve_layout(&mut self, args: &[Value]) -> Result<Value, String> {
        let node: LayoutNode = get_arg(args, 0)?;
        let float_window_rect: Rect = get_arg(args, 1)?;
        assert!(matches!(node, LayoutNode::Pane(pane_id) if pane_id == self.pane_id.unwrap()));

        let mut cmdline_rect = float_window_rect;
        cmdline_rect.offset.0 += 1;
        cmdline_rect.offset.1 += 1;
        cmdline_rect.size.0 = cmdline_rect.size.0.saturating_sub(2);
        cmdline_rect.size.1 = cmdline_rect.size.1.saturating_sub(2);

        let mut commands = vec![];
        for row in 0..float_window_rect.size.1 {
            if row == 0 {
                commands.push(DrawCommand::DrawString {
                    position: (0, row),
                    text: "╔".to_string()
                        + &"═".repeat(float_window_rect.size.0.saturating_sub(2) as usize)
                        + "╗",
                });
            } else if row + 1 == float_window_rect.size.1 {
                commands.push(DrawCommand::DrawString {
                    position: (0, row),
                    text: "╚".to_string()
                        + &"═".repeat(float_window_rect.size.0.saturating_sub(2) as usize)
                        + "╝",
                });
            } else {
                commands.push(DrawCommand::DrawString {
                    position: (0, row),
                    text: "║".to_string()
                        + &" ".repeat(float_window_rect.size.0.saturating_sub(2) as usize)
                        + "║",
                });
            }
        }

        Ok((vec![(self.pane_id.unwrap(), cmdline_rect)], commands).into())
    }
    #[service]
    fn attach_pane(&mut self, args: &[Value]) -> Result<Value, String> {
        let pane_id = get_arg(args, 0)?;
        assert!(self.pane_id.is_none());
        self.pane_id = Some(pane_id);
        self.buffer.clear();
        Ok(Value::Null)
    }
    #[service]
    fn pane_active(&mut self, _args: &[Value]) -> Result<Value, String> {
        Ok(Value::Null)
    }
    #[service]
    fn render_pane(&mut self, _args: &[Value]) -> Result<Value, String> {
        let commands = vec![DrawCommand::DrawString {
            position: (0, 0),
            text: ":ここに入力されたコマンドが入るよ！！！".to_string(),
        }];

        Ok(commands.into())
    }
}
impl Plugin for CommandLinePlugin {
    fn init(&mut self, reg: clm_plugin_api::core::PluginRegistrar) {
        Self::register_service_and_subscribe(&reg);
    }
}
