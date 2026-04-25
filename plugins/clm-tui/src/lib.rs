use std::io::stdout;

use clm_plugin_api::core::*;
use clm_plugin_api::data::*;
use clm_plugin_api::priority;
use crossterm::cursor::{MoveTo, SetCursorStyle};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

#[derive(Debug, Default)]
pub struct TuiPlugin {
    view_offset: (usize, usize),
}
impl TuiPlugin {
    pub fn new() -> Self {
        Self::default()
    }
}
#[clm_plugin_api::clm_handlers(name = "tui")]
impl TuiPlugin {
    #[subscribe(priority = priority::DEFAULT)]
    fn on_render(&mut self, _data: &Value) -> EventResult {
        render(&mut self.view_offset).unwrap();
        EventResult::Handled
    }
}

impl Plugin for TuiPlugin {
    fn init(&mut self, reg: clm_plugin_api::core::PluginRegistrar) {
        Self::register_service_and_subscribe(&reg);

        spawn_async(async {
            use crossterm::event::{Event as TuiEvent, read};

            loop {
                let Ok(event) = read() else {
                    emit_event_async(
                        Event {
                            kind: EventKind("quit".to_string()),
                            data: Value::Null,
                        },
                        DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
                    );
                    return;
                };
                match event {
                    TuiEvent::Key(key_event) => {
                        emit_event_async(
                            Event {
                                kind: EventKind("key_input".to_string()),
                                data: convert_key_event(key_event).into(),
                            },
                            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
                        );
                    }
                    _ => {}
                }
            }
        });

        enable_raw_mode().unwrap();
        execute!(stdout(), EnterAlternateScreen).unwrap();

        emit_event(
            Event {
                kind: EventKind("render".to_string()),
                data: Value::Null,
            },
            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
        );
    }
    fn uninit(&mut self) {
        disable_raw_mode().unwrap();
        execute!(stdout(), LeaveAlternateScreen).unwrap();
    }
}

fn render(view_offset: &mut (usize, usize)) -> anyhow::Result<()> {
    use crossterm::terminal::{Clear, ClearType};
    use unicode_width::UnicodeWidthStr;

    execute!(stdout(), Clear(ClearType::All))?;
    let size = crossterm::terminal::size()?;
    let mode: Mode = query_service("modal.mode", &[])?.try_into().unwrap();
    let cursor: CursorState = query_service("modal.cursor", &[])?
        .try_into()
        .unwrap_or_default();
    let view_size = (size.0, size.1 - 1);
    let buffer_id: BufferId = query_service("modal.buffer_id", &[])?.try_into().unwrap();
    let command_line: String = query_service("modal.command_line", &[])?
        .try_into()
        .unwrap_or_default();

    // オフセットの計算
    {
        if cursor.row < view_offset.1 {
            view_offset.1 = cursor.row;
        }
        if view_offset.1 + view_size.1 as usize <= cursor.row {
            view_offset.1 = cursor.row - (view_size.1 as usize - 1);
        }
        let line: String = query_service("buffer.line", &[buffer_id.into(), cursor.row.into()])
            .unwrap()
            .try_into()
            .unwrap();
        let display_col_l = line[..cursor.byte_col].width();
        let display_col_r = display_col_l + line[cursor.byte_col..].width();
        if display_col_l < view_offset.0 {
            view_offset.0 = display_col_l;
        }
        if view_offset.0 + (view_size.0 as usize) < display_col_r {
            view_offset.0 = display_col_r - (view_size.0 as usize);
        }
    }

    // バッファーの表示
    for row in 0..view_size.1 {
        let line: Option<String> = query_service(
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
            let line: String = query_service("buffer.line", &[buffer_id.into(), cursor.row.into()])
                .unwrap()
                .try_into()
                .unwrap();
            let x = line[..cursor.byte_col].width();
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
            let cursor: usize = query_service("modal.command_line_cursor", &[])?
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

fn query_service(name: &str, args: &[Value]) -> anyhow::Result<Value> {
    clm_plugin_api::core::query_service(name, args).map_err(anyhow::Error::msg)
}

fn convert_key_event(key_event: crossterm::event::KeyEvent) -> clm_plugin_api::input::KeyEvent {
    use clm_plugin_api::input::*;
    use crossterm::event::{
        KeyCode as TuiKeyCode, KeyEventKind as TuiKeyState, KeyModifiers as TuiModifiers,
    };
    KeyEvent {
        physical_key: PhysicalKey::Unknown,
        logical_key: match key_event.code {
            TuiKeyCode::Backspace => LogicalKey::Named(NamedKey::Backspace),
            TuiKeyCode::Enter => LogicalKey::Named(NamedKey::Enter),
            TuiKeyCode::Left => LogicalKey::Named(NamedKey::ArrowLeft),
            TuiKeyCode::Right => LogicalKey::Named(NamedKey::ArrowRight),
            TuiKeyCode::Up => LogicalKey::Named(NamedKey::ArrowUp),
            TuiKeyCode::Down => LogicalKey::Named(NamedKey::ArrowDown),
            TuiKeyCode::Home => LogicalKey::Named(NamedKey::Home),
            TuiKeyCode::End => LogicalKey::Named(NamedKey::End),
            TuiKeyCode::PageUp => LogicalKey::Named(NamedKey::PageUp),
            TuiKeyCode::PageDown => LogicalKey::Named(NamedKey::PageDown),
            TuiKeyCode::Tab => LogicalKey::Named(NamedKey::Tab),
            TuiKeyCode::BackTab => LogicalKey::Named(NamedKey::BackTab),
            TuiKeyCode::Delete => LogicalKey::Named(NamedKey::Delete),
            TuiKeyCode::Insert => LogicalKey::Named(NamedKey::Insert),
            TuiKeyCode::F(n) => match n {
                1 => LogicalKey::Named(NamedKey::F1),
                2 => LogicalKey::Named(NamedKey::F2),
                3 => LogicalKey::Named(NamedKey::F3),
                4 => LogicalKey::Named(NamedKey::F4),
                5 => LogicalKey::Named(NamedKey::F5),
                6 => LogicalKey::Named(NamedKey::F6),
                7 => LogicalKey::Named(NamedKey::F7),
                8 => LogicalKey::Named(NamedKey::F8),
                9 => LogicalKey::Named(NamedKey::F9),
                10 => LogicalKey::Named(NamedKey::F10),
                11 => LogicalKey::Named(NamedKey::F11),
                12 => LogicalKey::Named(NamedKey::F12),
                _ => LogicalKey::Unknown,
            },
            TuiKeyCode::Char(c) => LogicalKey::Character(c.to_string()),
            TuiKeyCode::Null => LogicalKey::Unknown,
            TuiKeyCode::Esc => LogicalKey::Named(NamedKey::Escape),
            TuiKeyCode::CapsLock => LogicalKey::Named(NamedKey::CapsLock),
            TuiKeyCode::ScrollLock => LogicalKey::Named(NamedKey::ScrollLock),
            TuiKeyCode::NumLock => LogicalKey::Named(NamedKey::NumLock),
            TuiKeyCode::PrintScreen => LogicalKey::Named(NamedKey::PrintScreen),
            _ => LogicalKey::Unknown,
        },
        text: None,
        modifiers: Modifiers {
            shift: key_event.modifiers.contains(TuiModifiers::SHIFT),
            ctrl: key_event.modifiers.contains(TuiModifiers::CONTROL),
            alt: key_event.modifiers.contains(TuiModifiers::ALT),
            super_key: key_event.modifiers.contains(TuiModifiers::SUPER),
        },
        location: KeyLocation::Standard,
        state: match key_event.kind {
            TuiKeyState::Press => ElementState::Pressed,
            TuiKeyState::Release => ElementState::Released,
            TuiKeyState::Repeat => ElementState::Pressed,
        },
        repeat: matches!(key_event.kind, TuiKeyState::Release),
    }
}
