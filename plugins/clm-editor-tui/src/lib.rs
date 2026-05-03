use core::range::RangeInclusive;

use clm_plugin_api::core::*;
use clm_plugin_api::{ConvertValue, priority};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ConvertValue)]
pub struct PaneId(pub usize);
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
    range: RangeInclusive<u16>,
}
impl Default for SizeConstraint {
    fn default() -> Self {
        Self::new(1.0, 0..u16::MAX)
    }
}
impl SizeConstraint {
    pub fn new(weight: f64, range: impl std::ops::RangeBounds<u16>) -> Self {
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
        SizeConstraint {
            weight,
            range: RangeInclusive { start, last },
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ConvertValue)]
pub enum CursorStyle {
    DefaultUserShape,
    BlinkingBlock,
    SteadyBlock,
    BlinkingUnderScore,
    SteadyUnderScore,
    BlinkingBar,
    SteadyBar,
}
impl From<CursorStyle> for crossterm::cursor::SetCursorStyle {
    fn from(value: CursorStyle) -> Self {
        use CursorStyle::*;
        match value {
            DefaultUserShape => Self::DefaultUserShape,
            BlinkingBlock => Self::BlinkingBlock,
            SteadyBlock => Self::SteadyBlock,
            BlinkingUnderScore => Self::BlinkingUnderScore,
            SteadyUnderScore => Self::SteadyUnderScore,
            BlinkingBar => Self::BlinkingBar,
            SteadyBar => Self::SteadyBar,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, ConvertValue)]
pub enum DrawCommand {
    CellGrid(Vec<String>),
    SetCursor {
        position: (u16, u16),
        style: CursorStyle,
    },
}

#[derive(Debug)]
pub struct EditorTuiPlugin {
    layout: LayoutNode,
    next_id: usize,
}
impl EditorTuiPlugin {
    pub fn new() -> (Self, PaneId) {
        let id = PaneId(0);
        (
            Self {
                layout: LayoutNode::Pane(id),
                next_id: 1,
            },
            id,
        )
    }
    pub fn get_next_id(&mut self) -> PaneId {
        let id = PaneId(self.next_id);
        self.next_id += 1;
        id
    }
    pub fn resolve_layout(&self, total_size: (u16, u16)) -> Vec<(PaneId, Rect)> {
        todo!()
    }
}
#[clm_plugin_api::clm_handlers(name = "editor-tui")]
impl EditorTuiPlugin {
    #[subscribe(priority = priority::DEFAULT)]
    fn on_render(&mut self, _data: &Value) -> EventResult {
        let Ok(terminal_size) = crossterm::terminal::size() else {
            return EventResult::Handled;
        };
        let pane_size = (terminal_size.0, terminal_size.1 - 1);
        let Ok(commands) = query_service(
            "render_pane",
            &[PaneId(0).into(), pane_size.0.into(), pane_size.1.into()],
        ) else {
            return EventResult::Handled;
        };
        let Ok(commands) = commands.try_into() else {
            return EventResult::Handled;
        };
        draw((0, 0), pane_size, commands).unwrap();
        EventResult::Handled
    }
}
impl Plugin for EditorTuiPlugin {
    fn init(&mut self, reg: clm_plugin_api::core::PluginRegistrar) {
        Self::register_service_and_subscribe(&reg);
    }
}

fn draw(
    offset: (u16, u16),
    pane_size: (u16, u16),
    commands: Vec<DrawCommand>,
) -> std::io::Result<()> {
    use std::io::stdout;

    use crossterm::cursor::{MoveTo, RestorePosition, SavePosition, SetCursorStyle};
    use crossterm::execute;
    use crossterm::style::Print;
    use crossterm::terminal::{Clear, ClearType};

    execute!(stdout(), Clear(ClearType::All))?;
    for command in commands {
        match command {
            DrawCommand::CellGrid(grid) => {
                execute!(stdout(), SavePosition)?;
                for (y, mut line) in grid.into_iter().enumerate() {
                    use unicode_width::UnicodeWidthStr;
                    while (pane_size.0 as usize) < line.width() {
                        line.pop();
                    }
                    execute!(stdout(), MoveTo(0, offset.1 + y as u16), Print(line))?;
                }
                execute!(stdout(), RestorePosition)?;
            }
            DrawCommand::SetCursor { position, style } => {
                execute!(
                    stdout(),
                    MoveTo(position.0, position.1),
                    SetCursorStyle::from(style)
                )?;
            }
        }
    }
    Ok(())
}
