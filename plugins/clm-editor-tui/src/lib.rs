use clm_plugin_api::core::*;
use clm_plugin_api::data::*;
use clm_plugin_api::{ConvertValue, priority};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ConvertValue)]
pub struct PaneId(pub u32);
/*#[derive(Clone, Copy)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}
#[derive(Clone, Copy)]
pub enum Direction {
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy)]
pub enum SizeConstraint {
    Fill,
    Cells(u16),
    Ratio(u16, u16),
}
pub enum LayoutNode {
    Pane(PaneId),
    Split {
        direction: Direction,
        children: Vec<(SizeConstraint, LayoutNode)>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, ConvertValue)]
pub struct Cell {
    ch: char,
}*/
#[derive(Debug, Clone, Serialize, Deserialize, ConvertValue)]
pub enum DrawCommand {
    CellGrid(Vec<String>),
}

#[derive(Debug, Default)]
pub struct EditorTuiPlugin {
    view_offset: (usize, usize),
}
impl EditorTuiPlugin {
    pub fn new() -> Self {
        Self::default()
    }
}
#[clm_plugin_api::clm_handlers(name = "editor-tui")]
impl EditorTuiPlugin {
    #[subscribe(priority = priority::DEFAULT)]
    fn on_render(&mut self, _data: &Value) -> EventResult {
        /*let Ok(terminal_size) = crossterm::terminal::size() else {
            return EventResult::Handled;
        };
        let Ok(command) = query_service(
            "render_pane",
            &[
                PaneId(0).into(),
                terminal_size.0.into(),
                terminal_size.1.into(),
            ],
        ) else {
            return EventResult::Handled;
        };*/
        render(&mut self.view_offset).unwrap();
        //panic!("{command:?}");
        EventResult::Handled
    }
}
impl Plugin for EditorTuiPlugin {
    fn init(&mut self, reg: clm_plugin_api::core::PluginRegistrar) {
        Self::register_service_and_subscribe(&reg);
    }
}

fn query_service_anyhow(name: &str, args: &[Value]) -> anyhow::Result<Value> {
    query_service(name, args).map_err(anyhow::Error::msg)
}

fn render(view_offset: &mut (usize, usize)) -> anyhow::Result<()> {
    use std::io::stdout;

    use crossterm::cursor::{MoveTo, SetCursorStyle};
    use crossterm::execute;
    use crossterm::style::Print;
    use crossterm::terminal::{Clear, ClearType};
    use unicode_width::UnicodeWidthStr;

    execute!(stdout(), Clear(ClearType::All))?;
    let size = crossterm::terminal::size()?;
    let mode: Mode = query_service_anyhow("modal.mode", &[])?.try_into().unwrap();
    let cursor: CursorState = query_service_anyhow("modal.cursor", &[])?
        .try_into()
        .unwrap_or_default();
    let view_size = (size.0, size.1 - 1);
    let buffer_id: BufferId = query_service_anyhow("modal.buffer_id", &[])?
        .try_into()
        .unwrap();
    let command_line: String = query_service_anyhow("modal.command_line", &[])?
        .try_into()
        .unwrap_or_default();
    let cursor_line: String =
        query_service_anyhow("buffer.line", &[buffer_id.into(), cursor.row.into()])
            .unwrap()
            .try_into()
            .unwrap();

    // オフセットの計算
    {
        if cursor.row < view_offset.1 {
            view_offset.1 = cursor.row;
        }
        if view_offset.1 + view_size.1 as usize <= cursor.row {
            view_offset.1 = cursor.row - (view_size.1 as usize - 1);
        }
        let display_col_l = cursor_line[..cursor.byte_col].width();
        let display_col_r = display_col_l + cursor_line[cursor.byte_col..].width();
        if display_col_l <= view_offset.0 {
            view_offset.0 = display_col_l;
        }
        if view_offset.0 + (view_size.0 as usize) <= display_col_r {
            view_offset.0 = display_col_r - (view_size.0 as usize);
        }
    }

    // バッファーの表示
    for row in 0..view_size.1 {
        let line: Option<String> = query_service_anyhow(
            "buffer.line",
            &[buffer_id.into(), (view_offset.1 + row as usize).into()],
        )
        .unwrap()
        .try_into()
        .unwrap();
        if let Some(line) = line {
            execute!(
                stdout(),
                MoveTo(0, row),
                Print(trim_display_range(
                    &line,
                    view_offset.0,
                    view_offset.0 + view_size.0 as usize
                ))
            )?;
        } else {
            break;
        }
    }
    // ステータスラインの設定
    execute!(stdout(), MoveTo(0, size.1 - 1))?;
    match mode {
        Mode::Normal => execute!(stdout(), Print("-- NORMAL --"),)?,
        Mode::Insert => execute!(stdout(), Print("-- INSERT --"))?,
        Mode::Command => execute!(stdout(), Print("-- COMMAND -- :"), Print(&command_line))?,
    }
    // カーソルの設定
    match mode {
        Mode::Normal | Mode::Insert => {
            let x = cursor_line[..cursor.byte_col].width();
            execute!(
                stdout(),
                MoveTo(
                    (x - view_offset.0) as u16,
                    (cursor.row - view_offset.1) as u16
                ),
            )?;
            match mode {
                Mode::Normal => execute!(stdout(), SetCursorStyle::SteadyBlock)?,
                Mode::Insert => execute!(stdout(), SetCursorStyle::SteadyBar)?,
                _ => unreachable!(),
            }
        }
        Mode::Command => {
            let cursor: usize = query_service_anyhow("modal.command_line_cursor", &[])?
                .try_into()
                .unwrap_or_default();
            let x = "-- COMMAND -- :".width() + command_line[..cursor].width();

            execute!(
                stdout(),
                MoveTo(x as u16, size.1 - 1),
                SetCursorStyle::SteadyBar
            )?;
        }
    }
    Ok(())
}

fn trim_display_range(line: &str, range_l: usize, range_r: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut width = 0;
    let mut result = String::new();
    for c in line.chars() {
        let l = width;
        let w = c.width().unwrap_or(0);
        let r = l + w;
        width += w;
        if r <= range_l {
            continue;
        }
        if range_r <= l {
            break;
        }
        if l < range_l || range_r < r {
            for i in l..r {
                if range_l <= i && i < range_r {
                    result.push(' ');
                }
            }
        } else {
            if c != '\n' {
                result.push(c);
            }
        }
    }
    result
}
