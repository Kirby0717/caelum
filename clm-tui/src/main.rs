use std::cell::RefCell;
use std::io::stdout;
use std::rc::Rc;

use clm_core::editor::{CursorState, EditorState, Mode, SharedState};
use clm_core::event::{
    DispatchDescriptor, Event as ClmEvent, EventData, EventKind, PropertyKey,
    SortKey,
};
use clm_core::registry::{
    Resolver, add_plugin, dispatch_next, emit_event, query_service,
    register_resolver,
};
use clm_core::value::Value;
use crossterm::cursor::{MoveTo, SetCursorStyle};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode,
};
use unicode_width::UnicodeWidthChar;

fn main() -> anyhow::Result<()> {
    let file = "./Cargo.toml";
    let state = Rc::new(RefCell::new(EditorState::from_file(file)?));

    register_resolver(
        SortKey("priority".to_string()),
        PropertyKey("priority".to_string()),
        Box::new(|priority: Option<&Value>| {
            let Some(Value::Int(priority)) = priority
            else {
                return i64::MIN;
            };
            *priority
        }) as Resolver,
    );

    add_plugin(clm_modal::ModalPlugin::new());
    add_plugin(clm_motions::MotionPlugin::new());

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let mut size = crossterm::terminal::size()?;

    loop {
        use crossterm::event::{Event, read};
        match read()? {
            Event::Key(key_event) => {
                emit_event(
                    ClmEvent {
                        kind: EventKind("key_input".to_string()),
                        data: EventData::KeyInput(convert_key_event(key_event)),
                    },
                    DispatchDescriptor {
                        consumable: true,
                        sort_keys: vec![SortKey("priority".to_string())],
                    },
                );
            }
            Event::Resize(width, height) => {
                size = (width, height);
            }
            _ => {}
        }

        while dispatch_next(&mut *state.borrow_mut()) {}

        render(state.clone(), size)?;

        if !state.borrow().running {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn render(state: SharedState, size: (u16, u16)) -> anyhow::Result<()> {
    use crossterm::terminal::{Clear, ClearType};
    execute!(stdout(), Clear(ClearType::All))?;
    let state = state.borrow();
    let mode = query_service("modal.mode", &[])
        .and_then(|mode| mode.try_into().ok())
        .unwrap_or(Mode::Normal);
    let cursor: CursorState = query_service("modal.cursor", &[])
        .and_then(|cursor| cursor.try_into().ok())
        .unwrap_or_default();
    // バッファーの表示
    for row in 0..size.1 - 1 {
        if let Some(line) = state.buffer.rope().get_line(row as usize) {
            execute!(
                stdout(),
                MoveTo(0, row),
                Print(truncate_to_width(
                    &line.chars().collect::<String>(),
                    size.0 as usize
                ))
            )?;
        }
    }
    // ステータスラインの設定
    execute!(stdout(), MoveTo(0, size.1 - 1))?;
    match mode {
        Mode::Normal => execute!(stdout(), Print("-- NORMAL --"))?,
        Mode::Insert => execute!(stdout(), Print("-- INSERT --"))?,
        Mode::Command => execute!(
            stdout(),
            Print("-- COMMAND -- :"),
            Print(&state.command_line)
        )?,
    }
    // カーソルの設定
    match mode {
        Mode::Normal => {
            let x = state
                .buffer
                .rope()
                .line(cursor.row)
                .chars()
                .take(cursor.col)
                .map(|c| c.width().unwrap_or(0) as u16)
                .sum();
            execute!(
                stdout(),
                MoveTo(x, cursor.row as u16),
                SetCursorStyle::SteadyBlock
            )?;
        }
        Mode::Insert => {
            let x = state
                .buffer
                .rope()
                .line(cursor.row)
                .chars()
                .take(cursor.col)
                .map(|c| c.width().unwrap_or(0) as u16)
                .sum();
            execute!(
                stdout(),
                MoveTo(x, cursor.row as u16),
                SetCursorStyle::SteadyBar
            )?;
        }
        Mode::Command => {
            let x = state
                .command_line
                .chars()
                .map(|c| c.width().unwrap_or(0))
                .sum::<usize>()
                + "-- COMMAND -- :".len();
            execute!(
                stdout(),
                MoveTo(x as u16, size.1 - 1),
                SetCursorStyle::SteadyBar
            )?;
        }
    }
    Ok(())
}

fn truncate_to_width(line: &str, max_width: usize) -> &str {
    use unicode_width::UnicodeWidthChar;
    let mut width = 0;
    for (i, c) in line.char_indices() {
        let w = c.width().unwrap_or(0);
        if width + w > max_width {
            return &line[..i];
        }
        width += w;
    }
    line
}

fn convert_key_event(
    key_event: crossterm::event::KeyEvent,
) -> clm_plugin_api::input::KeyEvent {
    use clm_plugin_api::input::*;
    use crossterm::event::{
        KeyCode as TuiKeyCode, KeyEventKind as TuiKeyState,
        KeyModifiers as TuiModifiers,
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
