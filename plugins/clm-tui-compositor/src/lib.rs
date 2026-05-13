// TODO: フロートウィンドウの実装
mod utility;

use std::collections::HashMap;

use clm_plugin_api::core::*;
use clm_plugin_api::data::id::*;
use clm_plugin_api::data::tui_layout::*;
use clm_plugin_api::data::*;
use clm_plugin_api::priority;
pub use clm_tui_driver::{CursorStyle, DrawCommand};

fn resolve_layout(
    node: &LayoutNodeWithSizeConstraint,
    Rect { mut offset, size }: Rect,
) -> (Vec<(PaneId, Rect)>, Vec<DrawCommand>) {
    match node {
        LayoutNodeWithSizeConstraint::Pane((pane_id, _)) => {
            (vec![(*pane_id, Rect { offset, size })], vec![])
        }
        LayoutNodeWithSizeConstraint::Split {
            direction,
            children,
        } => match direction {
            Direction::Horizontal => {
                let weights: Vec<_> = children
                    .iter()
                    .map(|(size_constraint, _)| (size_constraint.weight.0, size_constraint.range.0))
                    .collect();
                let pane_len = children.len() as u16;
                if size.0 + 1 < pane_len {
                    // セパレーターがかけないサイズ
                    let mut commands = vec![];
                    // セパレータだけ描画
                    for row in 0..size.1 {
                        commands.push(DrawCommand::DrawString {
                            position: (0, row),
                            text: "│".repeat(size.0 as usize),
                        });
                    }
                    (vec![], commands)
                } else {
                    let width = utility::distribute(&weights, size.0 + 1 - pane_len);
                    let mut rects = vec![];
                    let mut commands = vec![];
                    for (((_, node), wideth), i) in children.iter().zip(width).zip(1..) {
                        let size = (wideth, size.1);
                        let (node_rects, node_commands) =
                            resolve_layout(node, Rect { offset, size });
                        rects.extend(node_rects);
                        commands.extend(translate_and_clip(
                            node_commands,
                            Rect { offset, size },
                            false,
                        ));
                        if i != pane_len {
                            // セパレータの描画
                            for row in 0..size.1 {
                                commands.push(DrawCommand::DrawString {
                                    position: (offset.0 + wideth, row),
                                    text: "│".to_string(),
                                });
                            }
                        }
                        offset.0 += wideth + 1;
                    }
                    (rects, commands)
                }
            }
            Direction::Vertical => {
                let weights: Vec<_> = children
                    .iter()
                    .map(|(size_constraint, _)| (size_constraint.weight.1, size_constraint.range.1))
                    .collect();
                let height = utility::distribute(&weights, size.1);
                let mut rects = vec![];
                let mut commands = vec![];
                for ((_, node), height) in children.iter().zip(height) {
                    let (node_rects, node_commands) = resolve_layout(
                        node,
                        Rect {
                            offset,
                            size: (size.0, height),
                        },
                    );
                    rects.extend(node_rects);
                    commands.extend(node_commands);
                    offset.1 += height;
                }
                (rects, commands)
            }
        },
    }
}

#[derive(Debug)]
struct FloatWindow {
    root: LayoutNode,
    handler: String,
}

#[derive(Debug)]
pub struct TuiCompositorPlugin {
    main_window: LayoutNode,
    float_windows: HashMap<FloatId, FloatWindow>,
    panes: HashMap<PaneId, PaneEntry>,
    focus_window_stack: Vec<FloatId>,
    focus: PaneId,
    next_pane_id: usize,
    next_float_id: usize,
}
impl TuiCompositorPlugin {
    pub fn new(handler: &str, args: &[Value]) -> Self {
        let id = PaneId(0);
        let mut attach_args = vec![id.into()];
        attach_args.extend_from_slice(args);
        query_service(&format!("{handler}.attach_pane"), &attach_args).unwrap();
        query_service(&format!("{handler}.pane_active"), &[id.into()]).unwrap();

        Self {
            main_window: LayoutNode::Pane(id),
            float_windows: HashMap::new(),
            panes: HashMap::from([(
                id,
                PaneEntry {
                    parent: None,
                    handler: handler.to_string(),
                },
            )]),
            focus_window_stack: vec![],
            focus: id,
            next_pane_id: 1,
            next_float_id: 0,
        }
    }
    fn get_next_pane_id(&mut self) -> PaneId {
        let pane_id = PaneId(self.next_pane_id);
        self.next_pane_id += 1;
        pane_id
    }
    fn get_next_float_id(&mut self) -> FloatId {
        let float_id = FloatId(self.next_float_id);
        self.next_float_id += 1;
        float_id
    }
    fn split_pane(&mut self, direction: Direction) {
        let source_id = self.focus;
        let new_id = self.get_next_pane_id();
        let handler = self.panes[&source_id].handler.clone();
        if let Ok(parent) = query_service(
            &format!("{handler}.split_pane"),
            &[new_id.into(), source_id.into()],
        ) {
            let parent = parent.try_into().unwrap();
            self.panes.insert(new_id, PaneEntry { parent, handler });
            self.main_window.split(new_id, source_id, direction);
        }
    }
    fn get_all_size_constraints(
        &self,
        node: &LayoutNode,
    ) -> Result<Vec<(PaneId, SizeConstraint)>, String> {
        let mut size_constraints = vec![];
        for pane_id in node.pane_ids() {
            let size_constraint = match query_service(
                &format!("{}.size_constraint", self.panes[&pane_id].handler),
                &[pane_id.into()],
            ) {
                Ok(value) => SizeConstraint::try_from(value)?,
                Err(_) => SizeConstraint::default(),
            };
            size_constraints.push((pane_id, size_constraint));
        }
        Ok(size_constraints)
    }
}
#[clm_plugin_api::clm_handlers(name = "tui-compositor")]
impl TuiCompositorPlugin {
    #[service]
    fn build_frame(&self, args: &[Value]) -> Result<Value, String> {
        let terminal_size: (u16, u16) = get_arg(args, 0)?;
        let terminal_rect = Rect {
            offset: (0, 0),
            size: terminal_size,
        };

        let mut commands = vec![];

        // メイン画面
        // レイアウト解決
        let size_constraints = self.get_all_size_constraints(&self.main_window)?;
        let main_window = self.main_window.with_size_constraint(&size_constraints);
        let (rects, main_window_commands) = resolve_layout(&main_window, terminal_rect);
        commands.extend(translate_and_clip(
            main_window_commands,
            terminal_rect,
            false,
        ));
        // ペイン描画
        for (pane_id, mut rect) in rects {
            rect.clip(terminal_rect);
            let handler = &self.panes[&pane_id].handler;
            let pane_commands: Vec<DrawCommand> = query_service(
                &format!("{handler}.render_pane"),
                &[pane_id.into(), rect.size.into()],
            )?
            .try_into()?;
            commands.extend(translate_and_clip(
                pane_commands,
                rect,
                pane_id == self.focus,
            ));
        }

        // フロートウィンドウ
        for (id, FloatWindow { root, handler }) in self.float_windows.iter() {
            // 位置決め（絶対座標）
            let mut float_window_rect: Rect = query_service(
                &format!("{handler}.float_window_rect"),
                &[id.into(), terminal_size.into()],
            )?
            .try_into()?;
            float_window_rect.clip(terminal_rect);
            // レイアウト解決（フロートウィンドウ基準の相対座標）
            let size_constraints = self.get_all_size_constraints(root)?;
            let float_window_size = float_window_rect.size;
            let (rects, float_window_commands) = if let Ok(rects_and_commands) = query_service(
                &format!("{handler}.resolve_layout"),
                &[
                    root.into(),
                    float_window_size.into(),
                    (&size_constraints).into(),
                ],
            ) {
                rects_and_commands.try_into()?
            } else {
                let root = root.with_size_constraint(&size_constraints);
                resolve_layout(
                    &root,
                    Rect {
                        offset: (0, 0),
                        size: float_window_size,
                    },
                )
            };
            commands.extend(translate_and_clip(
                float_window_commands,
                float_window_rect,
                false,
            ));
            // ペイン描画
            for (pane_id, mut rect) in rects {
                rect.clip(Rect {
                    offset: (0, 0),
                    size: float_window_size,
                });
                let handler = &self.panes[&pane_id].handler;
                let pane_commands: Vec<DrawCommand> = query_service(
                    &format!("{handler}.render_pane"),
                    &[pane_id.into(), rect.size.into()],
                )?
                .try_into()?;
                rect.apply_offset(float_window_rect.offset);
                commands.extend(translate_and_clip(
                    pane_commands,
                    rect,
                    pane_id == self.focus,
                ));
            }
        }
        Ok(commands.into())
    }
    #[service]
    fn split(&mut self, _args: &[Value]) -> Result<Value, String> {
        self.split_pane(Direction::Vertical);
        Ok(Value::Null)
    }
    #[service]
    fn vsplit(&mut self, _args: &[Value]) -> Result<Value, String> {
        self.split_pane(Direction::Horizontal);
        Ok(Value::Null)
    }
    #[service]
    fn focus_pane(&self, _args: &[Value]) -> Result<Value, String> {
        Ok(self.focus.into())
    }
    #[subscribe(priority = priority::DEFAULT)]
    fn on_open_float_window(&mut self, data: &Value) -> EventResult {
        let Ok(OpenFloatWindowConfig {
            float_window_handler,
            pane_handler,
        }) = OpenFloatWindowConfig::try_from(data.clone())
        else {
            return EventResult::Propagate;
        };

        let float_id = self.get_next_float_id();
        let pane_id = self.get_next_pane_id();
        query_service(
            &format!("{float_window_handler}.attach_float_window"),
            &[float_id.into()],
        )
        .unwrap();
        self.float_windows.insert(
            float_id,
            FloatWindow {
                root: LayoutNode::Pane(pane_id),
                handler: float_window_handler.clone(),
            },
        );

        let parent = query_service(&format!("{pane_handler}.attach_pane"), &[pane_id.into()])
            .unwrap()
            .try_into()
            .unwrap();
        self.panes.insert(
            pane_id,
            PaneEntry {
                parent,
                handler: pane_handler.clone(),
            },
        );
        if let Ok(focusable) = query_service(&format!("{float_window_handler}.is_focusable"), &[])
            && let Ok(true) = bool::try_from(focusable)
        {
            self.focus_window_stack.push(float_id);
            query_service(&format!("{pane_handler}.pane_active"), &[pane_id.into()]).unwrap();
            self.focus = pane_id;
        }

        request_redraw();

        EventResult::Handled
    }
}
impl Plugin for TuiCompositorPlugin {
    fn init(&mut self, reg: clm_plugin_api::core::PluginRegistrar) {
        Self::register_service_and_subscribe(&reg);
        register_resolver(
            SortKey("focus_pane".to_string()),
            PropertyKey("pane_id".to_string()),
            Box::new(|pane_id: Option<&Value>| {
                let Some(pane_id) = pane_id else {
                    return i64::MIN;
                };
                let Ok(pane_id): Result<PaneId, _> = pane_id.clone().try_into() else {
                    return i64::MIN;
                };
                let Ok(focus_pane) = query_service("tui-compositor.focus_pane", &[]) else {
                    return i64::MIN;
                };
                let Ok(focus_pane): Result<PaneId, _> = focus_pane.try_into() else {
                    return i64::MIN;
                };
                if pane_id == focus_pane { 1 } else { 0 }
            }) as Resolver,
        );
        register_command(
            "split",
            Box::new(|_| {
                query_service("tui-compositor.split", &[])?;
                Ok(())
            }),
        );
        register_command(
            "vsplit",
            Box::new(|_| {
                query_service("tui-compositor.vsplit", &[])?;
                Ok(())
            }),
        );
    }
}

fn translate_and_clip(commands: Vec<DrawCommand>, rect: Rect, is_focus: bool) -> Vec<DrawCommand> {
    commands
        .into_iter()
        .filter_map(|mut command| {
            match &mut command {
                DrawCommand::DrawString { position, text } => {
                    if rect.size.1 <= position.1 {
                        return None;
                    }
                    use unicode_width::UnicodeWidthStr;
                    while rect.size.0 < position.0 + text.width() as u16 {
                        text.pop();
                    }
                    position.0 += rect.offset.0;
                    position.1 += rect.offset.1;
                }
                DrawCommand::SetCursor { position, .. } => {
                    if !is_focus {
                        return None;
                    }
                    position.0 += rect.offset.0;
                    position.1 += rect.offset.1;
                }
            }
            Some(command)
        })
        .collect()
}
