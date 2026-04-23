use std::collections::HashMap;

use clm_plugin_api::core::*;
use clm_plugin_api::data::*;
use clm_plugin_api::input::*;
use clm_plugin_api::priority;

#[derive(Debug)]
#[allow(unused)]
struct ParseError(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum KeyPatternKey {
    Char(char),
    Named(NamedKey),
}
fn parse_key_pattern_named_key(input: &str) -> Result<NamedKey, ParseError> {
    Ok(match input {
        "enter" => NamedKey::Enter,
        "tab" => NamedKey::Tab,
        "backtab" => NamedKey::BackTab,
        "space" => NamedKey::Space,
        "backspace" => NamedKey::Backspace,
        "delete" => NamedKey::Delete,
        "escape" => NamedKey::Escape,
        "insert" => NamedKey::Insert,
        "printscreen" => NamedKey::PrintScreen,
        "up" => NamedKey::ArrowUp,
        "down" => NamedKey::ArrowDown,
        "left" => NamedKey::ArrowLeft,
        "right" => NamedKey::ArrowRight,
        "home" => NamedKey::Home,
        "end" => NamedKey::End,
        "pageup" => NamedKey::PageUp,
        "pagedown" => NamedKey::PageDown,
        "f1" => NamedKey::F1,
        "f2" => NamedKey::F2,
        "f3" => NamedKey::F3,
        "f4" => NamedKey::F4,
        "f5" => NamedKey::F5,
        "f6" => NamedKey::F6,
        "f7" => NamedKey::F7,
        "f8" => NamedKey::F8,
        "f9" => NamedKey::F9,
        "f10" => NamedKey::F10,
        "f11" => NamedKey::F11,
        "f12" => NamedKey::F12,
        "capslock" => NamedKey::CapsLock,
        "numlock" => NamedKey::NumLock,
        "scrolllock" => NamedKey::ScrollLock,
        _ => return Err(ParseError(format!("unknown named key: {input}"))),
    })
}
fn parse_key_pattern_key(input: &str) -> Result<KeyPatternKey, ParseError> {
    if input.chars().count() == 1 {
        let c = input.chars().next().unwrap();
        if c.is_ascii_uppercase() {
            let lower = c.to_ascii_lowercase();
            return Err(ParseError(format!(
                "\"{c}\" should be written as \"S-{lower}\""
            )));
        }
        Ok(KeyPatternKey::Char(c))
    } else {
        Ok(KeyPatternKey::Named(parse_key_pattern_named_key(input)?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct KeyPattern {
    pub key: KeyPatternKey,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}
impl KeyPattern {
    fn from_key_event(event: &KeyEvent) -> Option<Self> {
        let key = match &event.logical_key {
            LogicalKey::Character(s) => KeyPatternKey::Char(s.chars().next()?.to_ascii_lowercase()),
            LogicalKey::Named(n) => KeyPatternKey::Named(*n),
            _ => return None,
        };
        Some(KeyPattern {
            key,
            shift: event.modifiers.shift,
            ctrl: event.modifiers.ctrl,
            alt: event.modifiers.alt,
        })
    }
}
fn parse_key_pattern(input: &str) -> Result<KeyPattern, ParseError> {
    let mut iter = input.split('-');

    let Some(key) = iter.next_back() else {
        return Err(ParseError("no key pattern".to_string()));
    };
    let key = parse_key_pattern_key(key)?;

    let mut shift = false;
    let mut ctrl = false;
    let mut alt = false;
    for modifier in iter {
        match modifier {
            "S" => shift = true,
            "C" => ctrl = true,
            "A" => alt = true,
            _ => return Err(ParseError(format!("invalid modifier: {modifier}"))),
        }
    }

    Ok(KeyPattern {
        key,
        shift,
        ctrl,
        alt,
    })
}

type KeySequence = Vec<KeyPattern>;
fn parse_key_sequence(input: &str) -> Result<KeySequence, ParseError> {
    input.split(' ').map(parse_key_pattern).collect()
}

fn toml_to_event(toml: toml::Value) -> Result<Event, ParseError> {
    let toml::Value::Table(mut event_table) = toml else {
        return Err(ParseError("event should table".to_string()));
    };
    let Some(kind) = event_table.remove("kind") else {
        return Err(ParseError("event should have kind".to_string()));
    };
    let toml::Value::String(kind) = kind else {
        return Err(ParseError("event kind should string".to_string()));
    };
    let kind = EventKind(kind);
    if let Some(data) = event_table.remove("data") {
        if !event_table.is_empty() {
            return Err(ParseError("use the data key alone".to_string()));
        }
        Ok(Event {
            kind,
            data: toml_to_value(data),
        })
    } else {
        Ok(Event {
            kind,
            data: toml_to_value(toml::Value::Table(event_table)),
        })
    }
}

#[derive(Debug)]
enum Binding {
    Event(Vec<Event>),
    Command { name: String, args: Vec<String> },
}
impl Binding {
    fn from_toml(toml: toml::Value) -> Result<Self, ParseError> {
        match toml {
            toml::Value::Array(events) => Ok(Binding::Event(
                events
                    .into_iter()
                    .map(toml_to_event)
                    .collect::<Result<_, _>>()?,
            )),
            toml::Value::Table(mut table) => {
                if let Some(command) = table.remove("command") {
                    let toml::Value::String(name) = command else {
                        return Err(ParseError("command name should string".to_string()));
                    };
                    let mut args = vec![];
                    if let Some(args_value) = table.remove("args") {
                        match args_value {
                            toml::Value::Array(args_array) => {
                                for arg in args_array {
                                    let toml::Value::String(arg) = arg else {
                                        return Err(ParseError("arg should string".to_string()));
                                    };
                                    args.push(arg);
                                }
                            }
                            _ => {
                                return Err(ParseError("args should array of string".to_string()));
                            }
                        }
                    }
                    if !table.is_empty() {
                        return Err(ParseError(
                            "command action should have only command/args".to_string(),
                        ));
                    }
                    Ok(Binding::Command { name, args })
                } else {
                    Ok(Binding::Event(vec![toml_to_event(toml::Value::Table(
                        table,
                    ))?]))
                }
            }
            _ => Err(ParseError(format!("invalid value: {toml}"))),
        }
    }
}

#[derive(Debug, Default)]
struct TrieNode {
    pub binding: Option<Binding>,
    pub children: HashMap<KeyPattern, TrieNode>,
}
impl TrieNode {
    fn from_toml(value: toml::Value) -> Result<Self, ParseError> {
        let mut root = Self::default();
        if let toml::Value::Table(table) = value {
            for (key_sequence, binding) in table {
                let key_sequence = parse_key_sequence(&key_sequence)?;
                let binding = Binding::from_toml(binding)?;
                root.add_binding(&key_sequence, binding);
            }
            Ok(root)
        } else {
            Err(ParseError("keymap is not table".to_string()))
        }
    }
    fn add_binding(&mut self, key_sequence: &[KeyPattern], binding: Binding) {
        if let Some(key) = key_sequence.first() {
            self.children
                .entry(key.clone())
                .or_default()
                .add_binding(&key_sequence[1..], binding);
        } else {
            self.binding = Some(binding);
        }
    }
}

#[derive(Debug, Default)]
struct Keymap {
    pub modes: HashMap<String, TrieNode>,
}
impl Keymap {
    fn from_toml(table: toml::Table) -> Result<Self, ParseError> {
        Ok(Self {
            modes: table
                .into_iter()
                .map(|(mode, keymap)| Ok((mode, TrieNode::from_toml(keymap)?)))
                .collect::<Result<_, ParseError>>()?,
        })
    }
    fn lookup(&self, mode: &str, key: &KeyPattern) -> Option<&TrieNode> {
        self.modes.get(mode)?.children.get(key)
    }
}

fn toml_to_value(toml: toml::Value) -> Value {
    match toml {
        toml::Value::String(s) => Value::Str(s),
        toml::Value::Integer(i) => Value::Int(i),
        toml::Value::Float(f) => Value::Float(f),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(date) => Value::Str(date.to_string()),
        toml::Value::Array(v) => Value::List(v.into_iter().map(toml_to_value).collect()),
        toml::Value::Table(t) => Value::Map(
            t.into_iter()
                .map(|(key, v)| (key, toml_to_value(v)))
                .collect(),
        ),
    }
}

#[derive(Debug, Default)]
pub struct KeymapPlugin {
    keymap: Keymap,
}
impl KeymapPlugin {
    pub fn new() -> Self {
        let config_path = "./keymap.toml";
        let config_file = std::fs::read_to_string(config_path).unwrap();
        let keymap = Keymap::from_toml(toml::from_str(&config_file).unwrap()).unwrap();
        Self { keymap }
    }
}
#[clm_plugin_api::clm_handlers(name = "keymap")]
impl KeymapPlugin {
    #[subscribe(kind = "key_input",priority = priority::DEFAULT - 1)]
    fn on_key_input_editing(&mut self, data: &Value, _ctx: &mut dyn PluginContext) -> EventResult {
        let Ok(key_event) = KeyEvent::try_from(data.clone()) else {
            return EventResult::Propagate;
        };
        let mode = query_service("modal.mode", &[])
            .ok()
            .and_then(|mode| mode.try_into().ok())
            .unwrap_or(Mode::Normal);
        match mode {
            Mode::Normal => return EventResult::Propagate,
            Mode::Insert => match &key_event.logical_key {
                LogicalKey::Character(c) => {
                    emit_edit(&EditAction::InsertText(c.clone()));
                }
                LogicalKey::Named(named) => match named {
                    NamedKey::Enter => {
                        emit_edit(&EditAction::NewLine);
                    }
                    NamedKey::Backspace => {
                        emit_edit(&EditAction::DeleteCharBackward);
                    }
                    NamedKey::Delete => {
                        emit_edit(&EditAction::DeleteCharForward);
                    }
                    _ => return EventResult::Propagate,
                },
                _ => return EventResult::Propagate,
            },
            Mode::Command => match &key_event.logical_key {
                LogicalKey::Character(c) => {
                    emit_command_line(&CommandLineAction::InsertText(c.clone()));
                }
                LogicalKey::Named(named) => match named {
                    NamedKey::Enter => {
                        emit_command_line(&CommandLineAction::Execute);
                    }
                    NamedKey::Escape => {
                        emit_command_line(&CommandLineAction::Clear);
                        emit_set_mode(Mode::Normal);
                    }
                    NamedKey::Backspace => {
                        emit_command_line(&CommandLineAction::DeleteCharBackward);
                    }
                    NamedKey::Delete => {
                        emit_command_line(&CommandLineAction::DeleteCharForward);
                    }
                    _ => return EventResult::Propagate,
                },
                _ => return EventResult::Propagate,
            },
        }
        EventResult::Handled
    }
    #[subscribe(priority = priority::DEFAULT)]
    fn on_key_input(&mut self, data: &Value, _ctx: &mut dyn PluginContext) -> EventResult {
        let Ok(key_event) = KeyEvent::try_from(data.clone()) else {
            return EventResult::Propagate;
        };
        let Some(key) = KeyPattern::from_key_event(&key_event) else {
            return EventResult::Propagate;
        };
        let mode = query_service("modal.mode", &[])
            .ok()
            .and_then(|mode| mode.try_into().ok())
            .unwrap_or(Mode::Normal);
        if key_event.state.is_pressed() {
            let mode = match mode {
                Mode::Normal => "normal",
                Mode::Insert => "insert",
                Mode::Command => "command",
            };
            // TODO: 単一キーだけで無くシーケンスに対応する
            // 子が無ければ実行
            // 入力されたキーが子に無ければ実行＆キーをシーケンスに追加
            // escでコマンド解除＆実行
            if let Some(trie) = self.keymap.lookup(mode, &key)
                && let Some(binding) = &trie.binding
            {
                match binding {
                    Binding::Event(events) => {
                        for event in events {
                            emit_event(
                                event.clone(),
                                DispatchDescriptor::Consumable(vec![SortKey(
                                    "priority".to_string(),
                                )]),
                            );
                        }
                    }
                    Binding::Command { name, args } => {
                        execute_command(name, args);
                    }
                }
            } else {
                return EventResult::Propagate;
            }
        }
        EventResult::Handled
    }
}
impl Plugin for KeymapPlugin {
    fn init(&mut self, reg: PluginRegistrar) {
        Self::register_service_and_subscribe(&reg);
    }
}
