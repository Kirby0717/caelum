pub enum KeyCode {
    Char(char),
    Enter,
    Backspace,
    Escape,
    Tab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    F(u8),
}

pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

pub enum KeyState {
    Press,
    Release,
    Repeat,
}

pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub state: KeyState,
}
