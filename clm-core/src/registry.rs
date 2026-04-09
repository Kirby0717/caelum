use std::cell::RefCell;

use crate::command::Command;
use crate::event::{
    DispatchDescriptor, Event, PropertyKey, Resolver, SortKey, Subscription,
    SubscriptionId,
};

pub enum RegistryAction {
    EmitEvent(Event, DispatchDescriptor),
    Subscribe(Subscription),
    Unsubscribe(SubscriptionId),
    RegisterResolver(SortKey, PropertyKey, Resolver),
    RegisterCommand(String, Command),
}

thread_local! {
    static PENDING: RefCell<Vec<RegistryAction>> = const { RefCell::new(Vec::new()) };
}

pub fn push_action(action: RegistryAction) {
    PENDING.with(|q| q.borrow_mut().push(action));
}
pub fn drain_actions() -> Vec<RegistryAction> {
    PENDING.with(|q| std::mem::take(&mut *q.borrow_mut()))
}

pub fn emit_event(event: Event, descriptor: DispatchDescriptor) {
    push_action(RegistryAction::EmitEvent(event, descriptor));
}

pub fn register_command(name: &str, command: Command) {
    push_action(RegistryAction::RegisterCommand(name.to_string(), command));
}

pub fn subscribe(subscription: Subscription) {
    push_action(RegistryAction::Subscribe(subscription));
}
