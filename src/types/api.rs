use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// WebSocket 命令类型常量
pub struct WsCmd;

impl WsCmd {
  // ========== 开发者 → 企业微信 ==========
  /// 认证订阅
  pub const SUBSCRIBE: &str = "aibot_subscribe";
  /// 心跳
  pub const HEARTBEAT: &str = "ping";
  /// 回复消息
  pub const RESPONSE: &str = "aibot_respond_msg";
  /// 回复欢迎语
  pub const RESPONSE_WELCOME: &str = "aibot_respond_welcome_msg";
  /// 更新模板卡片
  pub const RESPONSE_UPDATE: &str = "aibot_respond_update_msg";
  /// 主动发送消息
  pub const SEND_MSG: &str = "aibot_send_msg";
  /// 上传临时素材 - 初始化
  pub const UPLOAD_MEDIA_INIT: &str = "aibot_upload_media_init";
  /// 上传临时素材 - 分片上传
  pub const UPLOAD_MEDIA_CHUNK: &str = "aibot_upload_media_chunk";
  /// 上传临时素材 - 完成上传
  pub const UPLOAD_MEDIA_FINISH: &str = "aibot_upload_media_finish";

  // ========== 企业微信 → 开发者 ==========
  /// 消息推送回调
  pub const CALLBACK: &str = "aibot_msg_callback";
  /// 事件推送回调
  pub const EVENT_CALLBACK: &str = "aibot_event_callback";
}

/// WebSocket 帧结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsFrame<T = Value> {
  /// 命令类型；认证/心跳响应时可能为空
  #[serde(default)]
  pub cmd: Option<String>,
  /// 请求头信息
  pub headers: HashMap<String, Value>,
  /// 消息体
  #[serde(default)]
  pub body: Option<T>,
  /// 响应错误码，认证/心跳响应时存在
  #[serde(default)]
  pub errcode: Option<i64>,
  /// 响应错误信息，认证/心跳响应时存在
  #[serde(default)]
  pub errmsg: Option<String>,
}

/// 仅包含 headers 的 WsFrame 子集，用于 reply / replyStream 等方法的参数类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsFrameHeaders {
  pub headers: HashMap<String, Value>,
}

impl<T> From<&WsFrame<T>> for WsFrameHeaders {
  fn from(frame: &WsFrame<T>) -> Self {
    Self { headers: frame.headers.clone() }
  }
}

// ========== 回复消息中的通用子结构 ==========

/// 回复消息中的图文混排子项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyMsgItem {
  /// 类型：image
  pub msgtype: String,
  /// 图片内容
  pub image: ReplyMsgItemImage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyMsgItemImage {
  /// Base64 编码的图片数据
  pub base64: String,
  /// 图片内容（base64编码前）的 MD5 值
  pub md5: String,
}

/// 回复消息中的反馈信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyFeedback {
  /// 反馈 ID，有效长度为 256 字节以内
  pub id: String,
}

// ========== 流式回复消息体 ==========

/// 流式回复消息体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamReplyBody {
  pub msgtype: String,
  pub stream: StreamReplyBodyStream,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamReplyBodyStream {
  pub id: String,
  #[serde(default)]
  pub finish: Option<bool>,
  #[serde(default)]
  pub content: Option<String>,
  #[serde(default)]
  pub msg_item: Option<Vec<ReplyMsgItem>>,
  #[serde(default)]
  pub feedback: Option<ReplyFeedback>,
}

// ========== 欢迎语回复消息体 ==========

/// 欢迎语回复消息体（文本类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomeTextReplyBody {
  pub msgtype: String,
  pub text: WelcomeTextContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomeTextContent {
  pub content: String,
}

/// 欢迎语回复消息体（模板卡片类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomeTemplateCardReplyBody {
  pub msgtype: String,
  pub template_card: TemplateCard,
}

/// 欢迎语回复消息体联合类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WelcomeReplyBody {
  Text(WelcomeTextReplyBody),
  TemplateCard(WelcomeTemplateCardReplyBody),
}

// ========== 模板卡片回复消息体 ==========

/// 模板卡片回复消息体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardReplyBody {
  pub msgtype: String,
  pub template_card: TemplateCard,
}

// ========== 流式消息 + 模板卡片组合回复消息体 ==========

/// 流式消息 + 模板卡片组合回复消息体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamWithTemplateCardReplyBody {
  pub msgtype: String,
  pub stream: StreamReplyBodyStream,
  #[serde(default)]
  pub template_card: Option<TemplateCard>,
}

// ========== 更新模板卡片消息体 ==========

/// 更新模板卡片消息体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTemplateCardBody {
  pub response_type: String,
  #[serde(default)]
  pub userids: Option<Vec<String>>,
  pub template_card: TemplateCard,
}

// ========== 模板卡片结构体及子结构体 ==========

/// 卡片类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateCardType {
  TextNotice,
  NewsNotice,
  ButtonInteraction,
  VoteInteraction,
  MultipleInteraction,
}

/// 卡片来源样式信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardSource {
  #[serde(default)]
  pub icon_url: Option<String>,
  #[serde(default)]
  pub desc: Option<String>,
  #[serde(default)]
  pub desc_color: Option<i32>,
}

/// 卡片右上角更多操作按钮
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardActionMenu {
  pub desc: String,
  pub action_list: Vec<TemplateCardActionMenuItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardActionMenuItem {
  pub text: String,
  pub key: String,
}

/// 模板卡片主标题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardMainTitle {
  #[serde(default)]
  pub title: Option<String>,
  #[serde(default)]
  pub desc: Option<String>,
}

/// 关键数据样式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardEmphasisContent {
  #[serde(default)]
  pub title: Option<String>,
  #[serde(default)]
  pub desc: Option<String>,
}

/// 引用文献样式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardQuoteArea {
  #[serde(default)]
  pub r#type: Option<i32>,
  #[serde(default)]
  pub url: Option<String>,
  #[serde(default)]
  pub appid: Option<String>,
  #[serde(default)]
  pub pagepath: Option<String>,
  #[serde(default)]
  pub title: Option<String>,
  #[serde(default)]
  pub quote_text: Option<String>,
}

/// 二级标题+文本列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardHorizontalContent {
  #[serde(default)]
  pub r#type: Option<i32>,
  pub keyname: String,
  #[serde(default)]
  pub value: Option<String>,
  #[serde(default)]
  pub url: Option<String>,
  #[serde(default)]
  pub userid: Option<String>,
}

/// 跳转指引样式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardJumpAction {
  #[serde(default)]
  pub r#type: Option<i32>,
  pub title: String,
  #[serde(default)]
  pub url: Option<String>,
  #[serde(default)]
  pub appid: Option<String>,
  #[serde(default)]
  pub pagepath: Option<String>,
  #[serde(default)]
  pub question: Option<String>,
}

/// 整体卡片的点击跳转事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardAction {
  pub r#type: i32,
  #[serde(default)]
  pub url: Option<String>,
  #[serde(default)]
  pub appid: Option<String>,
  #[serde(default)]
  pub pagepath: Option<String>,
}

/// 卡片二级垂直内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardVerticalContent {
  pub title: String,
  #[serde(default)]
  pub desc: Option<String>,
}

/// 图片样式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardImage {
  pub url: String,
  #[serde(default)]
  pub aspect_ratio: Option<f64>,
}

/// 左图右文样式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardImageTextArea {
  #[serde(default)]
  pub r#type: Option<i32>,
  #[serde(default)]
  pub url: Option<String>,
  #[serde(default)]
  pub appid: Option<String>,
  #[serde(default)]
  pub pagepath: Option<String>,
  #[serde(default)]
  pub title: Option<String>,
  #[serde(default)]
  pub desc: Option<String>,
  pub image_url: String,
}

/// 提交按钮样式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardSubmitButton {
  pub text: String,
  pub key: String,
}

/// 下拉式选择器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardSelectionItem {
  pub question_key: String,
  #[serde(default)]
  pub title: Option<String>,
  #[serde(default)]
  pub disable: Option<bool>,
  #[serde(default)]
  pub selected_id: Option<String>,
  pub option_list: Vec<TemplateCardSelectionOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardSelectionOption {
  pub id: String,
  pub text: String,
}

/// 模板卡片按钮
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardButton {
  pub text: String,
  #[serde(default)]
  pub style: Option<i32>,
  pub key: String,
}

/// 选择题样式（投票选择）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardCheckbox {
  pub question_key: String,
  #[serde(default)]
  pub disable: Option<bool>,
  #[serde(default)]
  pub mode: Option<i32>,
  pub option_list: Vec<TemplateCardCheckboxOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCardCheckboxOption {
  pub id: String,
  pub text: String,
  #[serde(default)]
  pub is_checked: Option<bool>,
}

/// 模板卡片结构（通用类型，包含所有可能的字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCard {
  pub card_type: String,
  #[serde(default)]
  pub source: Option<TemplateCardSource>,
  #[serde(default)]
  pub action_menu: Option<TemplateCardActionMenu>,
  #[serde(default)]
  pub main_title: Option<TemplateCardMainTitle>,
  #[serde(default)]
  pub emphasis_content: Option<TemplateCardEmphasisContent>,
  #[serde(default)]
  pub quote_area: Option<TemplateCardQuoteArea>,
  #[serde(default)]
  pub sub_title_text: Option<String>,
  #[serde(default)]
  pub horizontal_content_list: Option<Vec<TemplateCardHorizontalContent>>,
  #[serde(default)]
  pub jump_list: Option<Vec<TemplateCardJumpAction>>,
  #[serde(default)]
  pub card_action: Option<TemplateCardAction>,
  #[serde(default)]
  pub card_image: Option<TemplateCardImage>,
  #[serde(default)]
  pub image_text_area: Option<TemplateCardImageTextArea>,
  #[serde(default)]
  pub vertical_content_list: Option<Vec<TemplateCardVerticalContent>>,
  #[serde(default)]
  pub button_selection: Option<TemplateCardSelectionItem>,
  #[serde(default)]
  pub button_list: Option<Vec<TemplateCardButton>>,
  #[serde(default)]
  pub checkbox: Option<TemplateCardCheckbox>,
  #[serde(default)]
  pub select_list: Option<Vec<TemplateCardSelectionItem>>,
  #[serde(default)]
  pub submit_button: Option<TemplateCardSubmitButton>,
  #[serde(default)]
  pub task_id: Option<String>,
  #[serde(default)]
  pub feedback: Option<ReplyFeedback>,
}

// ========== 主动发送消息体 ==========

/// 主动发送 Markdown 消息体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMarkdownMsgBody {
  pub msgtype: String,
  pub markdown: SendMarkdownContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMarkdownContent {
  pub content: String,
}

/// 主动发送模板卡片消息体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTemplateCardMsgBody {
  pub msgtype: String,
  pub template_card: TemplateCard,
}

/// 企业微信媒体类型
pub type WeComMediaType = String;

/// 媒体消息发送体（主动发送 + 被动回复共用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMediaMsgBody {
  pub msgtype: String,
  #[serde(default)]
  pub file: Option<MediaContent>,
  #[serde(default)]
  pub image: Option<MediaContent>,
  #[serde(default)]
  pub voice: Option<MediaContent>,
  #[serde(default)]
  pub video: Option<VideoMediaContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaContent {
  pub media_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMediaContent {
  pub media_id: String,
  #[serde(default)]
  pub title: Option<String>,
  #[serde(default)]
  pub description: Option<String>,
}

/// 主动发送消息体联合类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SendMsgBody {
  Markdown(SendMarkdownMsgBody),
  TemplateCard(SendTemplateCardMsgBody),
  Media(SendMediaMsgBody),
}

// ========== 上传临时素材相关类型 ==========

/// 上传素材初始化请求 body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMediaInitBody {
  pub r#type: String,
  pub filename: String,
  pub total_size: usize,
  pub total_chunks: usize,
  #[serde(default)]
  pub md5: Option<String>,
}

/// 上传素材初始化响应 body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMediaInitResult {
  pub upload_id: String,
}

/// 上传素材分片请求 body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMediaChunkBody {
  pub upload_id: String,
  pub chunk_index: usize,
  pub base64_data: String,
}

/// 完成上传请求 body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMediaFinishBody {
  pub upload_id: String,
}

/// 完成上传响应 body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMediaFinishResult {
  pub r#type: String,
  pub media_id: String,
  pub created_at: String,
}

/// uploadMedia 方法选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMediaOptions {
  pub r#type: String,
  pub filename: String,
}
