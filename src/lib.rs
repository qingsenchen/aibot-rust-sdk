//! WeCom AiBot Rust SDK (WebSocket long-connection)

pub mod api;
pub mod client;
pub mod crypto;
pub mod error;
pub mod logger;
pub mod message_handler;
pub mod types;
pub mod utils;
pub mod ws;

pub use client::WSClient;
pub use client::{ReplyStreamWithCardOptions, VideoOptions};
pub use api::WeComApiClient;
pub use ws::WsConnectionManager;
pub use message_handler::MessageHandler;
pub use crypto::decrypt_file;
pub use logger::DefaultLogger;
pub use utils::{generate_random_string, generate_req_id};
pub use error::AiBotError;

pub use types::api::{
  WsCmd,
  WsFrame,
  WsFrameHeaders,
  StreamReplyBody,
  ReplyMsgItem,
  ReplyFeedback,
  ReplyMsgItemImage,
  WelcomeTextReplyBody,
  WelcomeTextContent,
  WelcomeTemplateCardReplyBody,
  WelcomeReplyBody,
  TemplateCardMainTitle,
  TemplateCardButton,
  TemplateCardSource,
  TemplateCardActionMenu,
  TemplateCardEmphasisContent,
  TemplateCardQuoteArea,
  TemplateCardHorizontalContent,
  TemplateCardJumpAction,
  TemplateCardAction,
  TemplateCardVerticalContent,
  TemplateCardImage,
  TemplateCardImageTextArea,
  TemplateCardSubmitButton,
  TemplateCardSelectionItem,
  TemplateCardSelectionOption,
  TemplateCardCheckbox,
  TemplateCard,
  TemplateCardReplyBody,
  StreamWithTemplateCardReplyBody,
  UpdateTemplateCardBody,
  SendMarkdownMsgBody,
  SendMarkdownContent,
  SendTemplateCardMsgBody,
  SendMsgBody,
  SendMediaMsgBody,
  MediaContent,
  VideoMediaContent,
  WeComMediaType,
  UploadMediaOptions,
  UploadMediaFinishResult,
  UploadMediaInitBody,
  UploadMediaInitResult,
  UploadMediaChunkBody,
  UploadMediaFinishBody,
};

pub use types::message::{
  MessageType,
  BaseMessage,
  TextMessage,
  ImageMessage,
  MixedMessage,
  VoiceMessage,
  FileMessage,
  MessageFrom,
  TextContent,
  ImageContent,
  MixedContent,
  MixedMsgItem,
  VoiceContent,
  FileContent,
  QuoteContent,
  ReplyOptions,
  SendTextParams,
  SendMarkdownParams,
};

pub use types::event::{
  EventType,
  EventFrom,
  EnterChatEvent,
  TemplateCardEventData,
  FeedbackEventData,
  EventContent,
  EventMessage,
  EventMessageWith,
  WSClientEventMap,
};

pub use types::config::WSClientOptions;
pub use types::common::Logger;
