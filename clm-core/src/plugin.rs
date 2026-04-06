use crate::command::CommandRegistry;
use crate::editor::SharedState;
use crate::event::EventBus;

pub trait Plugin: Send {
    fn init(
        &mut self,
        state: SharedState,
        bus: &mut EventBus,
        commands: &mut CommandRegistry,
    );
}
