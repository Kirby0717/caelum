use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use crate::editor::PluginContext;
use crate::event::data::EventData;
use crate::event::{
    DispatchDescriptor, Event, EventResult, Plugin, PluginId, PluginRegistrar,
    PropertyKey, RawEventHandler, SortKey, Subscription, SubscriptionId,
};
use crate::value::Value;

pub type Resolver = Box<dyn Fn(Option<&Value>) -> i64>;
pub type Command = Box<dyn Fn(&[String]) -> Vec<(Event, DispatchDescriptor)>>;
pub type RawServiceHandler = unsafe fn(*const (), &[Value]) -> Value;
pub type RawMutServiceHandler = unsafe fn(*mut (), &[Value]) -> Value;
#[derive(Debug, Clone, Copy)]
pub enum ServiceHandler {
    Immutable(RawServiceHandler),
    Mutable(RawMutServiceHandler),
}
#[derive(Debug, Clone, Copy)]
pub struct Service {
    pub plugin_id: PluginId,
    pub handler: ServiceHandler,
}

thread_local! {
    static PLUGINS: RefCell<Vec<RefCell<Box<dyn Plugin>>>> = const { RefCell::new(Vec::new()) };
    static EVENT_QUEUE: RefCell<VecDeque<(Event, DispatchDescriptor)>> = const { RefCell::new(VecDeque::new()) };
    static SUBSCRIPTIONS: RefCell<Vec<Option<Subscription>>> = const { RefCell::new(Vec::new()) };
    static RESOLVERS: RefCell<HashMap<SortKey, (PropertyKey, Resolver)>> = RefCell::new(HashMap::new());
    static COMMAND_QUEUE: RefCell<VecDeque<(String, Vec<String>)>> = const { RefCell::new(VecDeque::new()) };
    static COMMANDS: RefCell<HashMap<String, Command>> = RefCell::new(HashMap::new());
    static SERVICES: RefCell<HashMap<String, Service>> = RefCell::new(HashMap::new());
}

pub fn add_plugin(mut plugin: impl Plugin + 'static) -> PluginId {
    let id = PLUGINS.with_borrow(|plugins| plugins.len());
    let id = PluginId(id);
    let reg = PluginRegistrar { plugin_id: id };
    plugin.init(reg);
    PLUGINS.with_borrow_mut(|plugins| {
        plugins.push(RefCell::new(Box::new(plugin)))
    });
    id
}

pub fn emit_event(event: Event, descriptor: DispatchDescriptor) {
    EVENT_QUEUE.with_borrow_mut(|queue| queue.push_back((event, descriptor)));
}

pub(crate) fn subscribe(subscription: Subscription) -> SubscriptionId {
    let id = SUBSCRIPTIONS.with_borrow(|subscriptions| subscriptions.len());
    let id = SubscriptionId(id);
    SUBSCRIPTIONS.with_borrow_mut(|subscriptions| {
        subscriptions.push(Some(subscription))
    });
    id
}
pub fn unsubscribe(id: SubscriptionId) {
    SUBSCRIPTIONS
        .with_borrow_mut(|subscriptions| subscriptions.get_mut(id.0)?.take());
}

pub fn register_resolver(
    sort_key: SortKey,
    property_key: PropertyKey,
    resolver: Resolver,
) {
    RESOLVERS.with_borrow_mut(|resolvers| {
        resolvers.insert(sort_key, (property_key, resolver))
    });
}

pub fn register_command(name: &str, command: Command) {
    COMMANDS
        .with_borrow_mut(|commands| commands.insert(name.to_string(), command));
}
pub fn execute_command(name: &str, args: &[String]) {
    COMMAND_QUEUE.with_borrow_mut(|queue| {
        queue.push_back((name.to_string(), args.to_vec()));
    });
}

pub(crate) fn register_service(name: &str, service: Service) {
    SERVICES
        .with_borrow_mut(|services| services.insert(name.to_string(), service));
}
pub fn query_service(name: &str, args: &[Value]) -> Option<Value> {
    let service =
        SERVICES.with_borrow(|services| services.get(name).copied())?;
    PLUGINS.with_borrow(|plugins| {
        let plugin = plugins.get(service.plugin_id.0)?;
        match service.handler {
            ServiceHandler::Immutable(handler) => {
                let plugin = plugin.try_borrow().ok()?;
                Some(call_service_handler(handler, plugin.as_ref(), args))
            }
            ServiceHandler::Mutable(handler) => {
                let mut plugin = plugin.try_borrow_mut().ok()?;
                Some(call_mut_service_handler(handler, plugin.as_mut(), args))
            }
        }
    })
}

fn pop_event() -> Option<(Event, DispatchDescriptor)> {
    EVENT_QUEUE.with_borrow_mut(|queue| queue.pop_front())
}
fn pop_command() -> Option<(String, Vec<String>)> {
    COMMAND_QUEUE.with_borrow_mut(|queue| queue.pop_front())
}
pub fn dispatch_next(ctx: &mut dyn PluginContext) -> bool {
    let Some((event, descriptor)) = pop_event()
    else {
        return false;
    };
    match descriptor {
        // 消費型
        DispatchDescriptor::Consumable(sort_keys) => {
            RESOLVERS.with_borrow(|resolvers| {
                SUBSCRIPTIONS.with_borrow_mut(|subscriptions| {
                    // 変換関数の解決
                    let mut resolvers = sort_keys
                        .iter()
                        // TODO: 警告ログ
                        .filter_map(|key| resolvers.get(key))
                        .collect::<Vec<_>>();
                    // 購読者のフィルターと優先順位の計算
                    let mut subscriptions = subscriptions
                        .iter_mut()
                        .flatten()
                        .filter(|subscription| subscription.kind == event.kind)
                        .map(|subscription| {
                            let key = resolvers
                                .iter_mut()
                                .map(|(key, resolver)| {
                                    resolver(subscription.properties.get(key))
                                })
                                .collect::<Vec<_>>();
                            (
                                key,
                                (subscription.handler, subscription.plugin_id),
                            )
                        })
                        .collect::<Vec<_>>();
                    // 降順ソート
                    subscriptions.sort_by_cached_key(|(key, _)| {
                        key.iter()
                            .map(|k| std::cmp::Reverse(*k))
                            .collect::<Vec<_>>()
                    });
                    // 順番に配信する
                    for (_, (handler, id)) in subscriptions {
                        let result = PLUGINS.with_borrow(|plugins| {
                            let plugin = plugins.get(id.0).and_then(|plugin| {
                                plugin.try_borrow_mut().ok()
                            });
                            if let Some(mut plugin) = plugin {
                                // イベントハンドラーの実行
                                call_event_handler(
                                    handler,
                                    plugin.as_mut(),
                                    &event.data,
                                    ctx,
                                )
                            }
                            else {
                                EventResult::Propagate
                            }
                        });
                        match result {
                            EventResult::Propagate => continue,
                            EventResult::Handled => break,
                        }
                    }
                });
            });
        }
        // ブロードキャスト型
        DispatchDescriptor::Broadcast => {
            SUBSCRIPTIONS.with_borrow_mut(|s| {
                for subscription in s.iter_mut().flatten() {
                    if subscription.kind != event.kind {
                        continue;
                    }
                    let id = subscription.plugin_id;
                    // プラグインの取り出し
                    PLUGINS.with_borrow_mut(|plugins| {
                        let plugin = plugins
                            .get_mut(id.0)
                            .and_then(|plugin| plugin.try_borrow_mut().ok());
                        if let Some(mut plugin) = plugin {
                            // イベントハンドラーの実行
                            call_event_handler(
                                subscription.handler,
                                plugin.as_mut(),
                                &event.data,
                                ctx,
                            );
                        }
                    });
                }
            })
        }
    }

    // コマンドの実行
    while let Some((name, args)) = pop_command() {
        COMMANDS.with_borrow_mut(|commands| {
            if let Some(command) = commands.get_mut(&name) {
                let events = command(&args);
                EVENT_QUEUE.with_borrow_mut(|queue| {
                    for (event, descriptor) in events {
                        queue.push_back((event, descriptor));
                    }
                });
            }
        });
    }
    true
}

fn call_event_handler(
    handler: RawEventHandler,
    plugin: &mut dyn Plugin,
    data: &EventData,
    ctx: &mut dyn PluginContext,
) -> EventResult {
    unsafe { handler(plugin as *mut dyn Plugin as *mut (), data, ctx) }
}
fn call_service_handler(
    handler: RawServiceHandler,
    plugin: &dyn Plugin,
    args: &[Value],
) -> Value {
    unsafe { handler(plugin as *const dyn Plugin as *const (), args) }
}
fn call_mut_service_handler(
    handler: RawMutServiceHandler,
    plugin: &mut dyn Plugin,
    args: &[Value],
) -> Value {
    unsafe { handler(plugin as *mut dyn Plugin as *mut (), args) }
}
