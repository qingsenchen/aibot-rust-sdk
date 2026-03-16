use serde::{Deserialize, Serialize};

use crate::types::api::WsFrame;
use crate::types::message::{
  BaseMessage,
  TextMessage,
  ImageMessage,
  MixedMessage,
  VoiceMessage,
  FileMessage,
};

/// 事件类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
  /// 进入会话事件：用户当天首次进入机器人单聊会话
  EnterChat,
  /// 模板卡片事件：用户点击模板卡片按钮
  TemplateCardEvent,
  /// 用户反馈事件：用户对机器人回复进行反馈
  FeedbackEvent,
}

/// 事件发送者信息（比 MessageFrom 多了 corpid 字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFrom {
  /// 事件触发者的 userid
  pub userid: String,
  /// 事件触发者的 corpid，企业内部机器人不返回
  #[serde(default)]
  pub corpid: Option<String>,
}

/// 进入会话事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterChatEvent {
  /// 事件类型
  pub eventtype: EventType,
}

/// 模板卡片事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardEventData {
  /// 事件类型
  pub eventtype: EventType,
  /// 用户点击的按钮 key
  #[serde(default)]
  pub event_key: Option<String>,
  /// 任务 ID
  #[serde(default)]
  pub task_id: Option<String>,
}

/// 用户反馈事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEventData {
  /// 事件类型
  pub eventtype: EventType,
}

/// 事件内容联合类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventContent {
  EnterChat(EnterChatEvent),
  TemplateCardEvent(TemplateCardEventData),
  FeedbackEvent(FeedbackEventData),
}

/// 事件回调消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMessage {
  /// 本次回调的唯一性标志，用于事件排重
  pub msgid: String,
  /// 事件产生的时间戳
  pub create_time: i64,
  /// 智能机器人 id
  pub aibotid: String,
  /// 会话 id，仅群聊类型时返回
  #[serde(default)]
  pub chatid: Option<String>,
  /// 会话类型：single 单聊, group 群聊
  #[serde(default)]
  pub chattype: Option<String>,
  /// 事件触发者信息
  pub from: EventFrom,
  /// 消息类型，事件回调固定为 event
  pub msgtype: String,
  /// 事件内容
  pub event: EventContent,
}

/// 带有特定事件类型的事件消息（Rust 版不区分泛型参数）
pub type EventMessageWith = EventMessage;

/// WSClient 事件映射类型（用于文档/兼容）
#[allow(dead_code)]
pub struct WSClientEventMap;

impl WSClientEventMap {
  pub fn _message(_data: WsFrame<BaseMessage>) {}
  pub fn _message_text(_data: WsFrame<TextMessage>) {}
  pub fn _message_image(_data: WsFrame<ImageMessage>) {}
  pub fn _message_mixed(_data: WsFrame<MixedMessage>) {}
  pub fn _message_voice(_data: WsFrame<VoiceMessage>) {}
  pub fn _message_file(_data: WsFrame<FileMessage>) {}
  pub fn _event(_data: WsFrame<EventMessage>) {}
}
