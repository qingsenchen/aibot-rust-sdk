use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 消息类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
  /// 文本消息
  Text,
  /// 图片消息
  Image,
  /// 图文混排消息
  Mixed,
  /// 语音消息
  Voice,
  /// 文件消息
  File,
}

/// 消息发送者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageFrom {
  /// 操作者的 userid
  pub userid: String,
}

/// 文本结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
  /// 文本消息内容
  pub content: String,
}

/// 图片结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
  /// 图片的下载 url（五分钟内有效，已加密）
  pub url: String,
  /// 解密密钥，长连接模式下返回，每个下载链接的 aeskey 唯一
  #[serde(default)]
  pub aeskey: Option<String>,
}

/// 语音结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceContent {
  /// 语音转换成文本的内容
  pub content: String,
}

/// 文件结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
  /// 文件的下载 url（五分钟内有效，已加密）
  pub url: String,
  /// 解密密钥，长连接模式下返回，每个下载链接的 aeskey 唯一
  #[serde(default)]
  pub aeskey: Option<String>,
}

/// 图文混排子项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedMsgItem {
  /// 图文混排中的类型：text / image
  pub msgtype: String,
  /// 文本内容（msgtype 为 text 时存在）
  #[serde(default)]
  pub text: Option<TextContent>,
  /// 图片内容（msgtype 为 image 时存在）
  #[serde(default)]
  pub image: Option<ImageContent>,
}

/// 图文混排结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedContent {
  /// 图文混排消息项列表
  pub msg_item: Vec<MixedMsgItem>,
}

/// 引用结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteContent {
  /// 引用的类型：text / image / mixed / voice / file
  pub msgtype: String,
  /// 引用的文本内容
  #[serde(default)]
  pub text: Option<TextContent>,
  /// 引用的图片内容
  #[serde(default)]
  pub image: Option<ImageContent>,
  /// 引用的图文混排内容
  #[serde(default)]
  pub mixed: Option<MixedContent>,
  /// 引用的语音内容
  #[serde(default)]
  pub voice: Option<VoiceContent>,
  /// 引用的文件内容
  #[serde(default)]
  pub file: Option<FileContent>,
}

/// 基础消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseMessage {
  /// 本次回调的唯一性标志，用于事件排重
  pub msgid: String,
  /// 智能机器人 id
  pub aibotid: String,
  /// 会话 id，仅群聊类型时返回
  #[serde(default)]
  pub chatid: Option<String>,
  /// 会话类型：single 单聊, group 群聊
  pub chattype: String,
  /// 事件触发者信息
  pub from: MessageFrom,
  /// 事件产生的时间戳
  #[serde(default)]
  pub create_time: Option<i64>,
  /// 支持主动回复消息的临时 url
  #[serde(default)]
  pub response_url: Option<String>,
  /// 消息类型
  pub msgtype: String,
  /// 引用内容（若用户引用了其他消息则有该字段）
  #[serde(default)]
  pub quote: Option<QuoteContent>,
  /// 原始数据
  #[serde(flatten)]
  pub extra: HashMap<String, Value>,
}

/// 文本消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMessage {
  pub msgtype: String,
  pub msgid: String,
  pub aibotid: String,
  #[serde(default)]
  pub chatid: Option<String>,
  pub chattype: String,
  pub from: MessageFrom,
  #[serde(default)]
  pub create_time: Option<i64>,
  #[serde(default)]
  pub response_url: Option<String>,
  #[serde(default)]
  pub quote: Option<QuoteContent>,
  pub text: TextContent,
  #[serde(flatten)]
  pub extra: HashMap<String, Value>,
}

/// 图片消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMessage {
  pub msgtype: String,
  pub msgid: String,
  pub aibotid: String,
  #[serde(default)]
  pub chatid: Option<String>,
  pub chattype: String,
  pub from: MessageFrom,
  #[serde(default)]
  pub create_time: Option<i64>,
  #[serde(default)]
  pub response_url: Option<String>,
  #[serde(default)]
  pub quote: Option<QuoteContent>,
  pub image: ImageContent,
  #[serde(flatten)]
  pub extra: HashMap<String, Value>,
}

/// 图文混排消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedMessage {
  pub msgtype: String,
  pub msgid: String,
  pub aibotid: String,
  #[serde(default)]
  pub chatid: Option<String>,
  pub chattype: String,
  pub from: MessageFrom,
  #[serde(default)]
  pub create_time: Option<i64>,
  #[serde(default)]
  pub response_url: Option<String>,
  #[serde(default)]
  pub quote: Option<QuoteContent>,
  pub mixed: MixedContent,
  #[serde(flatten)]
  pub extra: HashMap<String, Value>,
}

/// 语音消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceMessage {
  pub msgtype: String,
  pub msgid: String,
  pub aibotid: String,
  #[serde(default)]
  pub chatid: Option<String>,
  pub chattype: String,
  pub from: MessageFrom,
  #[serde(default)]
  pub create_time: Option<i64>,
  #[serde(default)]
  pub response_url: Option<String>,
  #[serde(default)]
  pub quote: Option<QuoteContent>,
  pub voice: VoiceContent,
  #[serde(flatten)]
  pub extra: HashMap<String, Value>,
}

/// 文件消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMessage {
  pub msgtype: String,
  pub msgid: String,
  pub aibotid: String,
  #[serde(default)]
  pub chatid: Option<String>,
  pub chattype: String,
  pub from: MessageFrom,
  #[serde(default)]
  pub create_time: Option<i64>,
  #[serde(default)]
  pub response_url: Option<String>,
  #[serde(default)]
  pub quote: Option<QuoteContent>,
  pub file: FileContent,
  #[serde(flatten)]
  pub extra: HashMap<String, Value>,
}

/// 回复消息选项
#[derive(Debug, Clone)]
pub struct ReplyOptions {
  pub msgid: String,
  pub chatid: String,
}

/// 发送文本消息参数
#[derive(Debug, Clone)]
pub struct SendTextParams {
  pub msgid: String,
  pub chatid: String,
  pub content: String,
}

/// 发送 Markdown 消息参数
#[derive(Debug, Clone)]
pub struct SendMarkdownParams {
  pub msgid: String,
  pub chatid: String,
  pub content: String,
}
