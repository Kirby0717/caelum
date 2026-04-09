use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;

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
    static SERVICES: RefCell<HashMap<String, Box<dyn Any>>> = RefCell::new(HashMap::new());
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

pub fn register_service<T: Any + 'static>(name: &str, value: T) {
    SERVICES.with(|s| s.borrow_mut().insert(name.to_string(), Box::new(value)));
}
pub fn with_service<T: 'static, R>(
    name: &str,
    f: impl FnOnce(&T) -> R,
) -> Option<R> {
    SERVICES.with(|s| s.borrow().get(name)?.downcast_ref::<T>().map(f))
}
