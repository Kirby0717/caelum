use clm_macros::ConvertValueInApi;
use serde::{Deserialize, Serialize};

/// 物理キー（キーの位置、レイアウト非依存）
/// キーバインド用。TUIでは取得できない場合がある。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ConvertValueInApi)]
pub enum PhysicalKey {
    Code(KeyCode),
    Unknown,
}

/// キーの物理位置（USキーボード基準）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ConvertValueInApi)]
pub enum KeyCode {
    // 文字キー（物理位置）
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    // 数字キー
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    // 機能キー
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    // ナビゲーション
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    // 編集
    Backspace,
    Delete,
    Enter,
    Tab,
    Escape,
    Insert,
    Space,
    // 記号キー（物理位置）
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Backquote,
    Comma,
    Period,
    Slash,
    // 修飾キー
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    SuperLeft,
    SuperRight,
    // テンキー
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadEnter,
    NumpadDecimal,
    // その他
    CapsLock,
    NumLock,
    ScrollLock,
    PrintScreen,
    Pause,
}

/// 論理キー（レイアウト依存、ユーザの意図）
/// テキスト入力やコマンド判定用。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ConvertValueInApi)]
pub enum LogicalKey {
    /// 名前付きキー
    Named(NamedKey),
    /// 文字入力（レイアウト変換後。複数コードポイントの可能性あり）
    Character(String),
    /// Dead key（アクセント記号等の合成途中）
    Dead(Option<char>),
    /// 不明
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ConvertValueInApi)]
pub enum NamedKey {
    Enter,
    Tab,
    BackTab,
    Space,
    Backspace,
    Delete,
    Escape,
    Insert,
    PrintScreen,
    // ナビゲーション
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    // 機能キー
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    // ロック
    CapsLock,
    NumLock,
    ScrollLock,
}

/// 修飾キーの状態
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, ConvertValueInApi,
)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

/// キーの位置（同じ論理キーが複数箇所にある場合の区別）
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, ConvertValueInApi,
)]
pub enum KeyLocation {
    #[default]
    Standard,
    Left,
    Right,
    Numpad,
}

/// 押下状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ConvertValueInApi)]
pub enum ElementState {
    Pressed,
    Released,
}
impl ElementState {
    pub fn is_pressed(self) -> bool {
        matches!(self, Self::Pressed)
    }
}

/// キーイベント本体
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ConvertValueInApi)]
pub struct KeyEvent {
    /// 物理キー（TUIではUnknownの場合あり）
    pub physical_key: PhysicalKey,
    /// 論理キー（レイアウト変換後）
    pub logical_key: LogicalKey,
    /// テキスト入力結果（修飾キー等はNone）
    pub text: Option<String>,
    /// 修飾キー状態
    pub modifiers: Modifiers,
    /// キー位置
    pub location: KeyLocation,
    /// 押下/離し
    pub state: ElementState,
    /// リピートかどうか
    pub repeat: bool,
}
