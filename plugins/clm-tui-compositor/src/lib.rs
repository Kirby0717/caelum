use std::collections::HashMap;
use std::ops::RangeBounds;
use std::range::RangeInclusive;

use clm_plugin_api::core::*;
use clm_plugin_api::data::id::*;
use clm_plugin_api::data::input::*;
use clm_plugin_api::priority;
pub use clm_tui_driver::{CursorStyle, DrawCommand};

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
    weight: f64,
    range: (RangeInclusive<u16>, RangeInclusive<u16>),
}
impl Default for SizeConstraint {
    fn default() -> Self {
        Self::new(1.0, (.., ..))
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
    pub fn new(weight: f64, range: (impl RangeBounds<u16>, impl RangeBounds<u16>)) -> Self {
        SizeConstraint {
            weight: weight.max(0.0),
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
    fn resolve_layout(&self, offset: (u16, u16), total_size: (u16, u16)) -> Vec<(PaneId, Rect)> {
        match self {
            Self::Pane(id) => vec![(
                *id,
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
                    todo!()
                }
                Direction::Vertical => {
                    todo!()
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
    fn vsplit(&mut self, _args: &[Value]) -> Result<Value, String> {
        let new_id = self.get_next_id();
        let handler = &self.pane_handlers[&self.focus];
        if query_service(
            &format!("{handler}.split_pane"),
            &[new_id.into(), self.focus.into()],
        )
        .is_ok()
        {
            todo!("ペインの分割、もし新しい方にフォーカスするならその通知");
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
            "vs",
            //"vsplit",
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
