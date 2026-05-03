use std::io::stdout;

use clm_plugin_api::core::*;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

#[derive(Debug, Default)]
pub struct TuiPlugin();
impl TuiPlugin {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Plugin for TuiPlugin {
    fn init(&mut self, _reg: clm_plugin_api::core::PluginRegistrar) {
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
