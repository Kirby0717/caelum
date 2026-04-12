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
#[derive(Debug, Clone, Copy)]
pub struct Service {
    pub plugin_id: PluginId,
    pub handler: RawServiceHandler,
}

thread_local! {
    static PLUGINS: RefCell<Vec<Option<Box<dyn Plugin>>>> = const { RefCell::new(Vec::new()) };
    static EVENT_QUEUE: RefCell<VecDeque<(Event, DispatchDescriptor)>> = const { RefCell::new(VecDeque::new()) };
    static SUBSCRIPTIONS: RefCell<Vec<Option<Subscription>>> = const { RefCell::new(Vec::new()) };
    static RESOLVERS: RefCell<HashMap<SortKey, (PropertyKey, Resolver)>> = RefCell::new(HashMap::new());
    static COMMANDS: RefCell<HashMap<String, Command>> = RefCell::new(HashMap::new());
    static SERVICES: RefCell<HashMap<String, Service>> = RefCell::new(HashMap::new());
}

pub fn add_plugin(mut plugin: impl Plugin + 'static) -> PluginId {
    let id = PLUGINS.with_borrow(|plugins| plugins.len());
    let id = PluginId(id);
    let reg = PluginRegistrar { plugin_id: id };
    plugin.init(reg);
    PLUGINS.with_borrow_mut(|plugins| plugins.push(Some(Box::new(plugin))));
    id
}

pub fn emit_event(event: Event, descriptor: DispatchDescriptor) {
    EVENT_QUEUE.with_borrow_mut(|queue| queue.push_back((event, descriptor)));
}
fn pop_event() -> Option<(Event, DispatchDescriptor)> {
    EVENT_QUEUE.with_borrow_mut(|queue| queue.pop_front())
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
pub fn dispatch_next(ctx: &mut dyn PluginContext) -> bool {
    let Some((event, descriptor)) = pop_event()
    else {
        return false;
    };
    // 消費型
    if descriptor.consumable {
        RESOLVERS.with_borrow(|resolvers| {
            SUBSCRIPTIONS.with_borrow_mut(|subscriptions| {
                // 変換関数の解決
                let mut resolvers = descriptor
                    .sort_keys
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
                        (key, (subscription.handler, subscription.plugin_id))
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
                    // プラグインの取り出し
                    let plugin = PLUGINS.with_borrow_mut(|plugins| {
                        plugins.get_mut(id.0).and_then(|slot| slot.take())
                    });
                    if let Some(mut plugin) = plugin {
                        // イベントハンドラーの実行
                        let result = call_event_handler(
                            handler,
                            plugin.as_mut(),
                            &event.data,
                            ctx,
                        );
                        // プラグインを戻す
                        PLUGINS.with_borrow_mut(|plugins| {
                            plugins[id.0] = Some(plugin)
                        });
                        // 結果に応じて終了
                        match result {
                            EventResult::Propagate => continue,
                            EventResult::Handled => break,
                        }
                    }
                }
            });
        });
    }
    // ブロードキャスト型
    else {
        SUBSCRIPTIONS.with_borrow_mut(|s| {
            for subscription in s.iter_mut().flatten() {
                if subscription.kind != event.kind {
                    continue;
                }
                let id = subscription.plugin_id;
                // プラグインの取り出し
                let plugin = PLUGINS.with_borrow_mut(|plugins| {
                    plugins.get_mut(id.0).and_then(|slot| slot.take())
                });
                if let Some(mut plugin) = plugin {
                    // イベントハンドラーの実行
                    call_event_handler(
                        subscription.handler,
                        plugin.as_mut(),
                        &event.data,
                        ctx,
                    );
                    // プラグインを戻す
                    PLUGINS.with_borrow_mut(|plugins| {
                        plugins[id.0] = Some(plugin)
                    });
                }
            }
        })
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

pub fn register_command(name: &str, command: Command) {
    COMMANDS
        .with_borrow_mut(|commands| commands.insert(name.to_string(), command));
}
pub fn execute_command(name: &str, args: &[String]) {
    COMMANDS.with_borrow_mut(|commands| {
        if let Some(command) = commands.get_mut(name) {
            let events = command(args);
            EVENT_QUEUE.with_borrow_mut(|queue| {
                for (event, descriptor) in events {
                    queue.push_back((event, descriptor));
                }
            });
        }
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
        let plugin = plugins.get(service.plugin_id.0)?.as_ref()?;
        Some(call_service_handler(service.handler, plugin.as_ref(), args))
    })
}
fn call_service_handler(
    handler: RawServiceHandler,
    plugin: &dyn Plugin,
    args: &[Value],
) -> Value {
    unsafe { handler(plugin as *const dyn Plugin as *const (), args) }
}
