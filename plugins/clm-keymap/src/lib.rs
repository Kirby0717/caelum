use std::collections::HashMap;
use std::rc::{Rc, Weak};

use clm_plugin_api::core::*;
use clm_plugin_api::data::id::*;
use clm_plugin_api::data::input::*;
use clm_plugin_api::data::*;
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

fn table_to_event(mut table: toml::Table) -> Result<Event, ParseError> {
    let Some(kind) = table.remove("event") else {
        return Err(ParseError("this in not event".to_string()));
    };
    let toml::Value::String(kind) = kind else {
        return Err(ParseError("event kind should string".to_string()));
    };
    let kind = EventKind(kind);
    if let Some(data) = table.remove("data") {
        if !table.is_empty() {
            return Err(ParseError("should use the data key alone".to_string()));
        }
        Ok(Event {
            kind,
            data: toml_to_value(data),
        })
    } else {
        Ok(Event {
            kind,
            data: toml_to_value(toml::Value::Table(table)),
        })
    }
}
#[derive(Debug, Clone)]
struct ServiceQuery {
    name: String,
    args: Vec<Value>,
}
fn table_to_service_query(mut table: toml::Table) -> Result<ServiceQuery, ParseError> {
    let Some(name) = table.remove("service") else {
        return Err(ParseError("this in not service".to_string()));
    };
    let toml::Value::String(name) = name else {
        return Err(ParseError("service name should string".to_string()));
    };
    if let Some(args) = table.remove("args") {
        let args = if let toml::Value::Array(args) = args {
            args
        } else {
            vec![args]
        };
        if !table.is_empty() {
            return Err(ParseError("should use the args key alone".to_string()));
        }
        Ok(ServiceQuery {
            name,
            args: args.into_iter().map(toml_to_value).collect(),
        })
    } else {
        Ok(ServiceQuery {
            name,
            args: vec![toml_to_value(toml::Value::Table(table))],
        })
    }
}
#[derive(Debug, Clone)]
struct Command {
    name: String,
    args: Vec<String>,
}
fn table_to_command(mut table: toml::Table) -> Result<Command, ParseError> {
    let Some(name) = table.remove("command") else {
        return Err(ParseError("this in not command".to_string()));
    };
    let toml::Value::String(name) = name else {
        return Err(ParseError("command name should string".to_string()));
    };
    let Some(args) = table.remove("args") else {
        return Ok(Command { name, args: vec![] });
    };
    match args {
        toml::Value::String(arg) => Ok(Command {
            name,
            args: vec![arg],
        }),
        toml::Value::Array(args) => {
            if !args.iter().all(|arg| arg.is_str()) {
                return Err(ParseError("args should string array".to_string()));
            }
            Ok(Command {
                name,
                args: args
                    .into_iter()
                    .map(|arg| arg.as_str().unwrap().to_string())
                    .collect(),
            })
        }
        _ => Err(ParseError("args should string array or string".to_string())),
    }
}
#[derive(Debug, Clone)]
enum Binding {
    Event(Event),
    Service(ServiceQuery),
    Command(Command),
}
fn toml_to_binding(toml: toml::Value) -> Result<Binding, ParseError> {
    let toml::Value::Table(table) = toml else {
        return Err(ParseError(format!("invalid value: {toml}")));
    };
    let is_event = table.contains_key("event");
    let is_service = table.contains_key("service");
    let is_command = table.contains_key("command");
    match (is_event, is_service, is_command) {
        (true, false, false) => Ok(Binding::Event(table_to_event(table)?)),
        (false, true, false) => Ok(Binding::Service(table_to_service_query(table)?)),
        (false, false, true) => Ok(Binding::Command(table_to_command(table)?)),
        (false, false, false) => Err(ParseError("no binding name".to_string())),
        _ => Err(ParseError("conflict binding name".to_string())),
    }
}
type Bindings = Vec<Binding>;
fn toml_to_bindings(toml: toml::Value) -> Result<Bindings, ParseError> {
    match toml {
        toml::Value::Array(sequence) => Ok(sequence
            .into_iter()
            .map(toml_to_binding)
            .collect::<Result<Vec<_>, _>>()?),
        _ => Ok(vec![toml_to_binding(toml)?]),
    }
}

#[derive(Debug, Default)]
struct TrieNode {
    pub binding: Option<Vec<Binding>>,
    pub children: HashMap<KeyPattern, Rc<TrieNode>>,
}
impl TrieNode {
    fn from_toml(value: toml::Value) -> Result<Rc<Self>, ParseError> {
        let mut root = Rc::new(Self::default());
        if let toml::Value::Table(table) = value {
            for (key_sequence, bindings) in table {
                let key_sequence = parse_key_sequence(&key_sequence)?;
                let bindings = toml_to_bindings(bindings)?;
                root.add_bindings(&key_sequence, bindings);
            }
            Ok(root)
        } else {
            Err(ParseError("keymap is not table".to_string()))
        }
    }
    fn add_bindings(self: &mut Rc<Self>, key_sequence: &[KeyPattern], bindings: Vec<Binding>) {
        let this = Rc::get_mut(self).unwrap();
        if let Some(key) = key_sequence.first() {
            this.children
                .entry(key.clone())
                .or_insert_with(|| Rc::new(Self::default()))
                .add_bindings(&key_sequence[1..], bindings);
        } else {
            this.binding = Some(bindings);
        }
    }
}

#[derive(Debug, Default)]
struct Keymap {
    pub modes: HashMap<String, Rc<TrieNode>>,
    pub root: Option<Weak<TrieNode>>,
    pub head: Option<Weak<TrieNode>>,
}
impl Keymap {
    fn new(toml_table: toml::Table) -> Result<Self, ParseError> {
        let modes = toml_table
            .into_iter()
            .map(|(mode, keymap)| Ok((mode, TrieNode::from_toml(keymap)?)))
            .collect::<Result<HashMap<_, _>, ParseError>>()?;
        let root = modes.get(&Mode::default().to_string()).map(Rc::downgrade);
        let head = root.clone();
        Ok(Self { modes, root, head })
    }
    fn change_mode(&mut self, mode: &str) {
        self.root = self.modes.get(mode).map(Rc::downgrade);
        self.head = self.root.clone();
    }
    fn feed(&mut self, key: &KeyPattern) -> Option<Vec<Binding>> {
        let head = self.head.take()?.upgrade().unwrap();
        match head.children.get(key) {
            Some(child) => {
                if child.children.is_empty()
                    && let Some(binding) = &child.binding
                {
                    self.head = self.root.clone();
                    Some(binding.clone())
                } else {
                    self.head = Some(Rc::downgrade(child));
                    None
                }
            }
            None => {
                self.head = self.root.clone();
                head.binding.clone()
            }
        }
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

#[derive(Debug)]
pub struct KeymapPlugin {
    keymap: Keymap,
    panes: HashMap<PaneId, String>,
}
impl Default for KeymapPlugin {
    fn default() -> Self {
        Self::new()
    }
}
impl KeymapPlugin {
    pub fn new() -> Self {
        let config_path = "./keymap.toml";
        let config_file = std::fs::read_to_string(config_path).unwrap();
        let keymap = Keymap::new(toml::from_str(&config_file).unwrap()).unwrap();
        Self {
            keymap,
            panes: HashMap::new(),
        }
    }
}
#[clm_plugin_api::clm_handlers(name = "keymap")]
impl KeymapPlugin {
    #[subscribe(priority = priority::DEFAULT)]
    fn on_key_input(&mut self, key_event: KeyEvent) -> EventResult {
        let Some(key) = KeyPattern::from_key_event(&key_event) else {
            return EventResult::Propagate;
        };
        if key_event.state.is_pressed() {
            let Some(bindings) = self.keymap.feed(&key) else {
                return EventResult::Handled;
            };
            let focus: PaneId = query_service("tui-compositor.focus_pane", &[])
                .unwrap()
                .try_into()
                .unwrap();
            let Some(handler) = self.panes.get(&focus) else {
                return EventResult::Handled;
            };
            for binding in bindings {
                match binding {
                    Binding::Event(event) => {
                        emit_event(
                            event,
                            DispatchDescriptor::Consumable(vec![SortKey("priority".to_string())]),
                        );
                    }
                    Binding::Service(ServiceQuery { name, args }) => {
                        query_service(&format!("{handler}.{name}"), &args).unwrap();
                    }
                    Binding::Command(command) => {
                        execute_command(&command.name, &command.args);
                    }
                }
            }
        }
        EventResult::Handled
    }
    #[service]
    fn add_pane(&mut self, pane_id: PaneId, handler: String) -> Result<(), String> {
        self.panes.insert(pane_id, handler);
        Ok(())
    }
    #[service]
    fn remove_pane(&mut self, pane_id: PaneId) -> Result<(), String> {
        self.panes.remove(&pane_id);
        Ok(())
    }
    #[subscribe(priority = priority::DEFAULT)]
    fn on_mode_changed(&mut self, data: &Value) -> EventResult {
        let Ok(mode) = Mode::try_from(data.clone()) else {
            return EventResult::Propagate;
        };
        self.keymap.change_mode(&mode.to_string());
        EventResult::Handled
    }
}
impl Plugin for KeymapPlugin {
    fn init(&mut self, reg: PluginRegistrar) {
        Self::register_service_and_subscribe(&reg);
        reg.subscribe(
            "key_input",
            HashMap::from([
                (
                    PropertyKey("priority".to_string()),
                    priority::DEFAULT.into(),
                ),
                (PropertyKey("pane_id".to_string()), PaneId(0).into()),
            ]),
            Self::ON_KEY_INPUT,
        );
    }
}

pub fn query_set_mode(mode: Mode) {
    let _ = query_service("modal.set_mode", &[mode.into()]);
}
pub fn query_cursor_move(cursor_move: CursorMove) {
    let _ = query_service("modal.cursor_move", &[cursor_move.into()]);
}
pub fn query_edit(edit: EditAction) {
    let _ = query_service("modal.edit", &[edit.into()]);
}
pub fn query_command_line(cmd_action: CommandLineAction) {
    let _ = query_service("modal.command_line_action", &[cmd_action.into()]);
}
