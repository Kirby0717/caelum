use std::io::stdout;

use clm_plugin_api::core::*;
use clm_plugin_api::{ConvertValue, priority};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ConvertValue)]
pub struct CursorState {
    position: (u16, u16),
    style: CursorStyle,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, ConvertValue,
)]
pub enum CursorStyle {
    #[default]
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
    DrawString {
        position: (u16, u16),
        text: String,
    },
    SetCursor {
        position: (u16, u16),
        style: CursorStyle,
    },
}

#[derive(Debug, Default)]
pub struct TuiDriverPlugin();
impl TuiDriverPlugin {
    pub fn new() -> Self {
        Self::default()
    }
}
#[clm_plugin_api::clm_handlers(name = "tui-driver")]
impl TuiDriverPlugin {
    #[subscribe(priority = priority::DEFAULT)]
    fn on_request_redraw(&mut self, _data: &Value) -> EventResult {
        let terminal_size = crossterm::terminal::size().unwrap();
        let commands: Vec<DrawCommand> =
            query_service("tui-compositor.build_frame", &[terminal_size.into()])
                .unwrap()
                .try_into()
                .unwrap();
        draw(commands).unwrap();
        EventResult::Handled
    }
}
impl Plugin for TuiDriverPlugin {
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
                    TuiEvent::Resize(..) => {
                        emit_event_async(
                            Event {
                                kind: EventKind("request_redraw".to_string()),
                                data: Value::Null,
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
                kind: EventKind("request_redraw".to_string()),
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

fn convert_key_event(
    key_event: crossterm::event::KeyEvent,
) -> clm_plugin_api::data::input::KeyEvent {
    use clm_plugin_api::data::input::*;
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

fn draw(commands: Vec<DrawCommand>) -> std::io::Result<()> {
    use std::io::stdout;

    use crossterm::cursor::{Hide, MoveTo, SetCursorStyle, Show};
    use crossterm::execute;
    use crossterm::style::Print;
    use crossterm::terminal::{Clear, ClearType};

    execute!(stdout(), Clear(ClearType::All))?;
    let mut cursor = None;
    for command in commands {
        match command {
            DrawCommand::DrawString { position, text } => {
                execute!(stdout(), MoveTo(position.0, position.1), Print(text),)?;
            }
            DrawCommand::SetCursor { position, style } => {
                cursor = Some((position, style));
            }
        }
    }
    if let Some((position, style)) = cursor {
        execute!(
            stdout(),
            Show,
            MoveTo(position.0, position.1),
            SetCursorStyle::from(style),
        )?;
    } else {
        execute!(stdout(), Hide)?;
    }
    Ok(())
}
