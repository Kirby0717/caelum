mod utility;

use std::collections::HashMap;
use std::ops::RangeBounds;
use std::range::RangeInclusive;

use clm_plugin_api::core::*;
use clm_plugin_api::data::id::*;
pub use clm_tui_driver::{CursorStyle, DrawCommand};

fn apportion(a: &[f64], l: u16) -> Vec<u16> {
    let n = a.len();
    let s = a.iter().sum::<f64>();
    let q = a.iter().map(|&a_i| l as f64 * a_i / s).collect::<Vec<_>>();

    let mut b = q.iter().map(|&q_i| q_i as u16).collect::<Vec<_>>();
    let t = b.iter().sum::<u16>();

    let rest = l - t;
    let mut d = (0..n).collect::<Vec<_>>();
    d.sort_by(|&i, &j| {
        let ri = q[i] - b[i] as f64;
        let rj = q[j] - b[j] as f64;
        rj.partial_cmp(&ri).unwrap()
    });
    for &i in &d[..rest as usize] {
        b[i] += 1;
    }
    b
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub offset: (u16, u16),
    pub size: (u16, u16),
}
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Horizontal,
    Vertical,
}
#[derive(Debug, Clone, Copy)]
pub struct SizeConstraint {
    weight: (f64, f64),
    range: (RangeInclusive<u16>, RangeInclusive<u16>),
}
impl Default for SizeConstraint {
    fn default() -> Self {
        Self::new((1.0, 1.0), (.., ..))
    }
}
fn range_bounds_into_range(range: impl std::ops::RangeBounds<u16>) -> RangeInclusive<u16> {
    use std::ops::Bound;
    let start = match range.start_bound() {
        Bound::Included(&n) => n,
        Bound::Excluded(&n) => n.checked_add(1).expect("start overflow"),
        Bound::Unbounded => u16::MIN,
    };
    let last = match range.end_bound() {
        Bound::Included(&n) => n,
        Bound::Excluded(&n) => n.checked_sub(1).expect("end underflow (empty range)"),
        Bound::Unbounded => u16::MAX,
    };

    RangeInclusive { start, last }
}
impl SizeConstraint {
    pub fn new(weight: (f64, f64), range: (impl RangeBounds<u16>, impl RangeBounds<u16>)) -> Self {
        SizeConstraint {
            weight: (
                weight.0.clamp(f64::EPSILON, f64::MAX),
                weight.1.clamp(f64::EPSILON, f64::MAX),
            ),
            range: (
                range_bounds_into_range(range.0),
                range_bounds_into_range(range.1),
            ),
        }
    }
}
#[derive(Debug)]
pub enum LayoutNode {
    Pane(PaneId),
    Split {
        direction: Direction,
        children: Vec<(SizeConstraint, LayoutNode)>,
    },
}
impl LayoutNode {
    fn resolve_layout(
        &self,
        mut offset: (u16, u16),
        total_size: (u16, u16),
    ) -> Vec<(PaneId, Rect)> {
        match self {
            Self::Pane(pane_id) => vec![(
                *pane_id,
                Rect {
                    offset,
                    size: total_size,
                },
            )],
            Self::Split {
                direction,
                children,
            } => match direction {
                Direction::Horizontal => {
                    let weights = children
                        .iter()
                        .map(|(size_constraint, _)| {
                            (size_constraint.weight.0, size_constraint.range.0)
                        })
                        .collect::<Vec<_>>();
                    let width = utility::distribute(&weights, total_size.0);
                    children
                        .iter()
                        .zip(width.iter())
                        .flat_map(|((_, node), &width)| {
                            let rects = node.resolve_layout(offset, (width, total_size.1));
                            offset.0 += width;
                            rects
                        })
                        .collect()
                }
                Direction::Vertical => {
                    let weights = children
                        .iter()
                        .map(|(size_constraint, _)| {
                            (size_constraint.weight.1, size_constraint.range.1)
                        })
                        .collect::<Vec<_>>();
                    let height = utility::distribute(&weights, total_size.1);
                    children
                        .iter()
                        .zip(height.iter())
                        .flat_map(|((_, node), &height)| {
                            let rects = node.resolve_layout(offset, (total_size.0, height));
                            offset.1 += height;
                            rects
                        })
                        .collect()
                }
            },
        }
    }
    fn split(&mut self, new_id: PaneId, source_id: PaneId) {
        match self {
            LayoutNode::Pane(pane_id) => {
                if source_id == *pane_id {
                    *self = LayoutNode::Split {
                        direction: Direction::Horizontal,
                        children: vec![
                            (SizeConstraint::default(), LayoutNode::Pane(*pane_id)),
                            (SizeConstraint::default(), LayoutNode::Pane(new_id)),
                        ],
                    };
                }
            }
            LayoutNode::Split {
                direction,
                children,
            } => match direction {
                Direction::Horizontal => {
                    let position = children
                        .iter()
                        .position(|(_, node)| matches!(node, LayoutNode::Pane(pane_id) if *pane_id == source_id));
                    if let Some(position) = position {
                        children.insert(
                            position + 1,
                            (SizeConstraint::default(), LayoutNode::Pane(new_id)),
                        );
                    } else {
                        for (_, node) in children {
                            node.split(new_id, source_id);
                        }
                    }
                }
                Direction::Vertical => {
                    for (_, node) in children {
                        node.split(new_id, source_id);
                    }
                }
            },
        }
    }
    fn vsplit(&mut self, new_id: PaneId, source_id: PaneId) {
        match self {
            LayoutNode::Pane(pane_id) => {
                if source_id == *pane_id {
                    *self = LayoutNode::Split {
                        direction: Direction::Vertical,
                        children: vec![
                            (SizeConstraint::default(), LayoutNode::Pane(*pane_id)),
                            (SizeConstraint::default(), LayoutNode::Pane(new_id)),
                        ],
                    };
                }
            }
            LayoutNode::Split {
                direction,
                children,
            } => match direction {
                Direction::Horizontal => {
                    for (_, node) in children {
                        node.vsplit(new_id, source_id);
                    }
                }
                Direction::Vertical => {
                    let position = children
                        .iter()
                        .position(|(_, node)| matches!(node, LayoutNode::Pane(pane_id) if *pane_id == source_id));
                    if let Some(position) = position {
                        children.insert(
                            position + 1,
                            (SizeConstraint::default(), LayoutNode::Pane(new_id)),
                        );
                    } else {
                        for (_, node) in children {
                            node.vsplit(new_id, source_id);
                        }
                    }
                }
            },
        }
    }
}

#[derive(Debug)]
pub struct TuiCompositorPlugin {
    layout: LayoutNode,
    pane_handlers: HashMap<PaneId, String>,
    focus: PaneId,
    next_id: usize,
}
impl TuiCompositorPlugin {
    pub fn new(handler: &str, args: &[Value]) -> Self {
        let id = PaneId(0);
        let mut attach_args = vec![id.into()];
        attach_args.extend_from_slice(args);
        query_service(&format!("{handler}.attach_pane"), &attach_args).unwrap();
        query_service(&format!("{handler}.pane_active"), &[id.into()]).unwrap();

        Self {
            layout: LayoutNode::Pane(id),
            pane_handlers: HashMap::from([(id, handler.to_string())]),
            focus: id,
            next_id: 1,
        }
    }
    pub fn get_next_id(&mut self) -> PaneId {
        let id = PaneId(self.next_id);
        self.next_id += 1;
        id
    }
    pub fn resolve_layout(&self, total_size: (u16, u16)) -> Vec<(PaneId, Rect)> {
        self.layout.resolve_layout((0, 0), total_size)
    }
}
#[clm_plugin_api::clm_handlers(name = "tui-compositor")]
impl TuiCompositorPlugin {
    #[service]
    fn build_frame(&self, args: &[Value]) -> Result<Value, String> {
        let terminal_size: (u16, u16) = get_arg(args, 0)?;
        let layout = self.resolve_layout(terminal_size);

        let mut commands = vec![];
        for (pane_id, rect) in layout {
            let handler = &self.pane_handlers[&pane_id];
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
        Ok(commands.into())
    }
    #[service]
    fn split(&mut self, _args: &[Value]) -> Result<Value, String> {
        let source_id = self.focus;
        let new_id = self.get_next_id();
        let handler = &self.pane_handlers[&source_id];
        if query_service(
            &format!("{handler}.split_pane"),
            &[new_id.into(), source_id.into()],
        )
        .is_ok()
        {
            self.pane_handlers.insert(new_id, handler.clone());
            self.layout.split(new_id, source_id);
        }
        Ok(Value::Null)
    }
    #[service]
    fn vsplit(&mut self, _args: &[Value]) -> Result<Value, String> {
        let source_id = self.focus;
        let new_id = self.get_next_id();
        let handler = &self.pane_handlers[&source_id];
        if query_service(
            &format!("{handler}.split_pane"),
            &[new_id.into(), source_id.into()],
        )
        .is_ok()
        {
            self.pane_handlers.insert(new_id, handler.clone());
            self.layout.vsplit(new_id, source_id);
        }
        Ok(Value::Null)
    }
    #[service]
    fn focus_pane(&self, _args: &[Value]) -> Result<Value, String> {
        Ok(self.focus.into())
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
