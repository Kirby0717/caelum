/*

イベントの概要

イベントが配信される
それらはキューに入れられ順番に処理される
もしブロードキャスト型なら
    イベントを購読者全員に配信する
もし消費型なら
    購読者をソートする
    前から順番に配信し、消費されたところで終了


イベントの購読登録の概要

購読登録は関数呼び出しによって行う
登録時には欲しいイベントの種類と購読性質を渡す


購読性質の概要

購読性質とは配信時に購読者の性質を判別するために必要な情報のことである
例えばゲームを遊べるプラグインを作る時、
キー入力の購読はゲームのウィンドウがフォーカスされているときのみの方が良い
この時ウィンドウIDという情報を渡すことにより、キー入力でその情報を使った配信が可能になる


配信の概要

配信にはイベントと配信方法、配信性質を必要とする
配信は関数呼び出しで行われキューに入れられる


配信性質の概要

配信性質とは、購読者をどのような優先順位で並べて試していくかである
例えばキー入力は(フォーカス, 優先順位)というキーでソートされる。
配信性質は各要素に対して変換関数により整数へ変換され比較可能となる。


変換関数の概要

変換関数は各配信性質に対して定義される
関数はOption<購読性質> -> i32のような形をしている
変換関数も購読と同じように登録する
Rustで書くなら変換関数のレジスタは以下のようになる
HashMap<配信性質, Box<dyn Fn(Option<購読性質>) -> i32>>

*/

use std::any::Any;
use std::collections::{HashMap, VecDeque};

use crate::plugin::PluginContext;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventKind(pub String);
pub type EventPayload = Box<dyn Any>;
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SortKey(pub String);

// イベント
pub struct Event {
    pub kind: EventKind,
    pub payload: EventPayload,
}

pub struct DispatchDescriptor {
    // 配信方法 true: 消費型 false: ブロードキャスト型
    pub consumable: bool,
    // 配信性質
    pub sort_keys: Vec<SortKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginId(pub usize);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PropertyKey(pub String);
pub type SubscriptionProperty = Box<dyn Any>;

// 購読
pub struct Subscription {
    pub plugin_id: PluginId,
    pub kind: EventKind,
    // 購読性質
    pub properties: HashMap<PropertyKey, SubscriptionProperty>,

    pub handler: Box<dyn EventHandler>,
}

pub trait EventHandler {
    fn handle(
        &mut self,
        event: &Event,
        ctx: &mut dyn PluginContext,
    ) -> EventResult;
}
pub enum EventResult {
    Handled,
    Propagate,
}

pub type Resolver = Box<dyn Fn(Option<&SubscriptionProperty>) -> i32>;
#[derive(Default)]
pub struct EventBus {
    queue: VecDeque<(Event, DispatchDescriptor)>,
    subscriptions: HashMap<SubscriptionId, Subscription>,
    next_subscription_id: usize,
    resolvers: HashMap<SortKey, (PropertyKey, Resolver)>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }
    // 配信の予約
    pub fn emit(&mut self, event: Event, descriptor: DispatchDescriptor) {
        self.queue.push_back((event, descriptor));
    }
    // 購読登録
    pub fn subscribe(&mut self, subscription: Subscription) -> SubscriptionId {
        let id = SubscriptionId(self.next_subscription_id);
        self.next_subscription_id += 1;
        self.subscriptions.insert(id, subscription);
        id
    }
    // 購読解除
    pub fn unsubscribe(&mut self, id: SubscriptionId) {
        self.subscriptions.remove(&id);
    }
    // 変換関数の登録
    pub fn register_resolver(
        &mut self,
        sort_key: SortKey,
        property_key: PropertyKey,
        resolver: Resolver,
    ) {
        self.resolvers.insert(sort_key, (property_key, resolver));
    }
    // 配信
    pub fn dispatch_next(&mut self, ctx: &mut dyn PluginContext) -> bool {
        let Some((event, descriptor)) = self.queue.pop_front()
        else {
            return false;
        };
        // 消費型
        if descriptor.consumable {
            // 変換関数の解決
            let resolvers = descriptor
                .sort_keys
                .iter()
                // TODO: 警告ログ
                .filter_map(|key| self.resolvers.get(key))
                .collect::<Vec<_>>();
            // 購読者のフィルターと優先順位の計算
            let mut subscriptions = self
                .subscriptions
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
        }
        // ブロードキャスト型
        else {
            for subscription in self.subscriptions.values_mut() {
                if subscription.kind != event.kind {
                    continue;
                }
                let _ = subscription.handler.handle(&event, ctx);
            }
        }
        true
    }
}
