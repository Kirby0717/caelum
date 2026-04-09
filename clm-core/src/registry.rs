use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use crate::editor::PluginContext;
use crate::event::{
    DispatchDescriptor, Event, EventResult, PropertyKey, SortKey, Subscription,
    SubscriptionId, SubscriptionProperty,
};

pub type Resolver = Box<dyn Fn(Option<&SubscriptionProperty>) -> i32>;
pub trait CommandHandler {
    fn handle(&mut self, args: &[String]) -> u32;
}
pub type Command = Box<dyn CommandHandler>;

thread_local! {
    static EVENT_QUEUE: RefCell<VecDeque<(Event, DispatchDescriptor)>> = const { RefCell::new(VecDeque::new()) };
    static SUBSCRIPTIONS: RefCell<HashMap<SubscriptionId, Subscription>> = RefCell::new(HashMap::new());
    static NEXT_SUBSCRIPTION_ID: RefCell<usize> = const { RefCell::new(0) };
    static RESOLVERS: RefCell<HashMap<SortKey, (PropertyKey, Resolver)>> = RefCell::new(HashMap::new());
    static COMMANDS: RefCell<HashMap<String, Command>> = RefCell::new(HashMap::new());
    static SERVICES: RefCell<HashMap<String, Box<dyn Any>>> = RefCell::new(HashMap::new());
}

pub fn emit_event(event: Event, descriptor: DispatchDescriptor) {
    EVENT_QUEUE.with(|q| q.borrow_mut().push_back((event, descriptor)));
}
fn pop_event() -> Option<(Event, DispatchDescriptor)> {
    EVENT_QUEUE.with(|q| q.borrow_mut().pop_front())
}

pub fn subscribe(subscription: Subscription) -> SubscriptionId {
    let id = NEXT_SUBSCRIPTION_ID.with(|next_id| {
        let id = *next_id.borrow();
        *next_id.borrow_mut() += 1;
        SubscriptionId(id)
    });
    SUBSCRIPTIONS.with(|s| s.borrow_mut().insert(id, subscription));
    id
}
pub fn unsubscribe(id: SubscriptionId) {
    SUBSCRIPTIONS.with(|s| s.borrow_mut().remove(&id));
}
pub fn register_resolver(
    sort_key: SortKey,
    property_key: PropertyKey,
    resolver: Resolver,
) {
    RESOLVERS
        .with(|r| r.borrow_mut().insert(sort_key, (property_key, resolver)));
}
pub fn dispatch_next(ctx: &mut dyn PluginContext) -> bool {
    let Some((event, descriptor)) = pop_event()
    else {
        return false;
    };
    // 消費型
    if descriptor.consumable {
        RESOLVERS.with(|resolvers| {
            SUBSCRIPTIONS.with(|subscriptions| {
                let resolvers = resolvers.borrow_mut();
                let mut subscriptions = subscriptions.borrow_mut();
                // 変換関数の解決
                let resolvers = descriptor
                    .sort_keys
                    .iter()
                    // TODO: 警告ログ
                    .filter_map(|key| resolvers.get(key))
                    .collect::<Vec<_>>();
                // 購読者のフィルターと優先順位の計算
                let mut subscriptions = subscriptions
                    .values_mut()
                    .filter(|subscription| subscription.kind == event.kind)
                    .map(|subscription| {
                        let key = resolvers
                            .iter()
                            .map(|(key, resolver)| {
                                resolver(subscription.properties.get(key))
                            })
                            .collect::<Vec<_>>();
                        (key, subscription)
                    })
                    .collect::<Vec<_>>();
                // 降順ソート
                subscriptions.sort_by_cached_key(|(key, _)| {
                    key.iter()
                        .map(|k| std::cmp::Reverse(*k))
                        .collect::<Vec<_>>()
                });
                // 順番に配信する
                for (_, subscription) in subscriptions {
                    if matches!(
                        subscription.handler.handle(&event, ctx),
                        EventResult::Handled
                    ) {
                        break;
                    }
                }
            });
        });
    }
    // ブロードキャスト型
    else {
        SUBSCRIPTIONS.with(|s| {
            for subscription in s.borrow_mut().values_mut() {
                if subscription.kind != event.kind {
                    continue;
                }
                let _ = subscription.handler.handle(&event, ctx);
            }
        })
    }
    true
}

pub fn register_command(name: &str, command: Command) {
    COMMANDS.with(|c| c.borrow_mut().insert(name.to_string(), command));
}
pub fn execute_command(name: &str, args: &[String]) -> Option<u32> {
    COMMANDS.with(|c| {
        c.borrow_mut()
            .get_mut(name)
            .map(|command| command.handle(args))
    })
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
