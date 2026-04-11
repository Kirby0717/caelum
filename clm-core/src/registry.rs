use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use crate::editor::PluginContext;
use crate::event::{
    DispatchDescriptor, Event, EventPayload, EventResult, Plugin, PluginId,
    PropertyKey, SortKey, Subscription, SubscriptionId,
};
use crate::value::Value;

pub type Resolver = Box<dyn Fn(Option<&Value>) -> i64>;
pub type Command = Box<dyn Fn(&[String]) -> Vec<(Event, DispatchDescriptor)>>;
pub type Service = Box<dyn Fn(&[Value]) -> Value>;

thread_local! {
    static PLUGINS: RefCell<HashMap<PluginId, Box<dyn Plugin>>> = RefCell::new(HashMap::new());
    static NEXT_PLUGIN_ID: RefCell<usize> = const { RefCell::new(0) };
    static EVENT_QUEUE: RefCell<VecDeque<(Event, DispatchDescriptor)>> = const { RefCell::new(VecDeque::new()) };
    static SUBSCRIPTIONS: RefCell<HashMap<SubscriptionId, Subscription>> = RefCell::new(HashMap::new());
    static NEXT_SUBSCRIPTION_ID: RefCell<usize> = const { RefCell::new(0) };
    static RESOLVERS: RefCell<HashMap<SortKey, (PropertyKey, Resolver)>> = RefCell::new(HashMap::new());
    static COMMANDS: RefCell<HashMap<String, Command>> = RefCell::new(HashMap::new());
    static SERVICES: RefCell<HashMap<String, Service>> = RefCell::new(HashMap::new());
}

pub fn add_plugin(mut plugin: impl Plugin + 'static) -> PluginId {
    let id = NEXT_PLUGIN_ID.with(|next_id| {
        let id = *next_id.borrow();
        *next_id.borrow_mut() += 1;
        PluginId(id)
    });
    plugin.init(id);
    PLUGINS.with(|p| p.borrow_mut().insert(id, Box::new(plugin)));
    id
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
                let resolvers = resolvers.borrow();
                let mut subscriptions = subscriptions.borrow_mut();
                // 変換関数の解決
                let mut resolvers = descriptor
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
                            .iter_mut()
                            .map(|(key, resolver)| {
                                resolver(subscription.properties.get(key))
                            })
                            .collect::<Vec<_>>();
                        (key, subscription.plugin_id)
                    })
                    .collect::<Vec<_>>();
                // 降順ソート
                subscriptions.sort_by_cached_key(|(key, _)| {
                    key.iter()
                        .map(|k| std::cmp::Reverse(*k))
                        .collect::<Vec<_>>()
                });
                // 順番に配信する
                PLUGINS.with(|p| {
                    let mut plugins = p.borrow_mut();
                    for (_, id) in subscriptions {
                        if let Some(plugin) = plugins.get_mut(&id) {
                            match call_handler(plugin.as_mut(), &event, ctx) {
                                EventResult::Propagate => continue,
                                EventResult::Handled => break,
                            }
                        }
                    }
                });
            });
        });
    }
    // ブロードキャスト型
    else {
        SUBSCRIPTIONS.with(|s| {
            PLUGINS.with(|p| {
                let mut plugins = p.borrow_mut();
                for subscription in s.borrow_mut().values_mut() {
                    if subscription.kind != event.kind {
                        continue;
                    }
                    let Some(plugin) = plugins.get_mut(&subscription.plugin_id)
                    else {
                        continue;
                    };
                    call_handler(plugin.as_mut(), &event, ctx);
                }
            });
        })
    }
    true
}
fn call_handler(
    plugin: &mut dyn Plugin,
    event: &Event,
    ctx: &mut dyn PluginContext,
) -> EventResult {
    match &event.payload {
        EventPayload::KeyInput(key) => plugin.on_key_input(key, ctx),
        EventPayload::CursorMove(mv) => plugin.on_cursor_move(*mv, ctx),
        EventPayload::Mode(mode) => plugin.on_mode_change(*mode, ctx),
        EventPayload::EditAction(action) => plugin.on_edit_action(action, ctx),
        EventPayload::CommandLine(action) => {
            plugin.on_command_line(action, ctx)
        }
        EventPayload::Exit => plugin.on_exit(ctx),
        EventPayload::Custom(value) => {
            plugin.on_custom(&event.kind.0, value, ctx)
        }
    }
}

pub fn register_command(name: &str, command: Command) {
    COMMANDS.with(|c| c.borrow_mut().insert(name.to_string(), command));
}
pub fn execute_command(name: &str, args: &[String]) {
    COMMANDS.with(|c| {
        if let Some(command) = c.borrow_mut().get_mut(name) {
            let events = command(args);
            EVENT_QUEUE.with(|q| {
                let mut queue = q.borrow_mut();
                for (event, descriptor) in events {
                    queue.push_back((event, descriptor));
                }
            });
        }
    });
}

pub fn register_service(name: &str, service: Service) {
    SERVICES.with(|s| s.borrow_mut().insert(name.to_string(), service));
}
pub fn query_service(name: &str, args: &[Value]) -> Option<Value> {
    SERVICES.with(|s| s.borrow().get(name).map(|service| service(args)))
}
