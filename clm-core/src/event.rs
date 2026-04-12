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

pub mod data;

use std::collections::HashMap;

use crate::editor::PluginContext;
use crate::event::data::EventData;
use crate::value::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventKind(pub String);
pub trait Plugin {
    fn init(&mut self, plugin_id: PluginId);
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SortKey(pub String);

// イベント
#[derive(Clone)]
pub struct Event {
    pub kind: EventKind,
    pub data: EventData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub type RawHandler =
    unsafe fn(*mut (), &EventData, &mut dyn PluginContext) -> EventResult;
// 購読
pub struct Subscription {
    pub plugin_id: PluginId,
    pub kind: EventKind,
    // 購読性質
    pub properties: HashMap<PropertyKey, Value>,
    pub handler: RawHandler,
}

pub type EventHandler =
    Box<dyn Fn(&Event, &mut dyn PluginContext) -> EventResult>;
pub enum EventResult {
    Handled,
    Propagate,
}
