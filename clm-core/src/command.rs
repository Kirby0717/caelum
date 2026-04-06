use std::collections::HashMap;

pub trait CommandHandler: Send {
    fn handle(&mut self, args: &[String]) -> u32;
}
pub type Command = Box<dyn CommandHandler>;
#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
}
impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, name: &str, command: Command) {
        self.commands.insert(name.to_string(), command);
    }
    pub fn execute(&mut self, name: &str, args: &[String]) -> Option<u32> {
        let command = self.commands.get_mut(name)?;
        Some(command.handle(args))
    }
    pub fn list(&self) -> Vec<&String> {
        self.commands.keys().collect()
    }
}
