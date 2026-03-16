use crate::api::WeComApiClient;
use crate::crypto::decrypt_file;
use crate::error::AiBotError;
use crate::logger::DefaultLogger;
use crate::message_handler::MessageHandler;
use crate::types::api::{
  ReplyFeedback,
  ReplyMsgItem,
  SendMediaMsgBody,
  SendMsgBody,
  StreamReplyBody,
  StreamReplyBodyStream,
  StreamWithTemplateCardReplyBody,
  TemplateCard,
  TemplateCardReplyBody,
  UpdateTemplateCardBody,
  UploadMediaFinishResult,
  UploadMediaOptions,
  WelcomeReplyBody,
  WsCmd,
  WsFrame,
  WsFrameHeaders,
};
use crate::types::common::Logger;
use crate::types::config::WSClientOptions;
use crate::utils::generate_req_id;
use crate::ws::WsConnectionManager;
use base64::engine::general_purpose::STANDARD as BASE64_STD;
use base64::Engine;
use futures::stream::{self, StreamExt};
use serde_json::Value;
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub struct WSClient {
  inner: Arc<WSClientInner>,
}

struct WSClientInner {
  api_client: WeComApiClient,
  ws_manager: WsConnectionManager,
  message_handler: MessageHandler,
  logger: Arc<dyn Logger>,
  events: RwLock<EventHandlers>,
}

#[derive(Clone)]
struct WSClientOptionsResolved {
  bot_id: String,
  secret: String,
  reconnect_interval: u64,
  max_reconnect_attempts: i64,
  heartbeat_interval: u64,
  request_timeout: u64,
  ws_url: Option<String>,
}

#[derive(Default)]
struct EventHandlers {
  message: Vec<Arc<dyn Fn(WsFrame<crate::types::message::BaseMessage>) + Send + Sync>>,
  message_text: Vec<Arc<dyn Fn(WsFrame<crate::types::message::TextMessage>) + Send + Sync>>,
  message_image: Vec<Arc<dyn Fn(WsFrame<crate::types::message::ImageMessage>) + Send + Sync>>,
  message_mixed: Vec<Arc<dyn Fn(WsFrame<crate::types::message::MixedMessage>) + Send + Sync>>,
  message_voice: Vec<Arc<dyn Fn(WsFrame<crate::types::message::VoiceMessage>) + Send + Sync>>,
  message_file: Vec<Arc<dyn Fn(WsFrame<crate::types::message::FileMessage>) + Send + Sync>>,
  event: Vec<Arc<dyn Fn(WsFrame<crate::types::event::EventMessage>) + Send + Sync>>,
  event_enter_chat: Vec<Arc<dyn Fn(WsFrame<crate::types::event::EventMessage>) + Send + Sync>>,
  event_template_card_event: Vec<Arc<dyn Fn(WsFrame<crate::types::event::EventMessage>) + Send + Sync>>,
  event_feedback_event: Vec<Arc<dyn Fn(WsFrame<crate::types::event::EventMessage>) + Send + Sync>>,
  connected: Vec<Arc<dyn Fn() + Send + Sync>>,
  authenticated: Vec<Arc<dyn Fn() + Send + Sync>>,
  disconnected: Vec<Arc<dyn Fn(String) + Send + Sync>>,
  reconnecting: Vec<Arc<dyn Fn(u32) + Send + Sync>>,
  error: Vec<Arc<dyn Fn(String) + Send + Sync>>,
}

impl WSClient {
  pub fn new(options: WSClientOptions) -> Self {
    let logger = options.logger.unwrap_or_else(|| Arc::new(DefaultLogger::default()));
    let resolved = WSClientOptionsResolved {
      bot_id: options.bot_id,
      secret: options.secret,
      reconnect_interval: options.reconnect_interval.unwrap_or(1000),
      max_reconnect_attempts: options.max_reconnect_attempts.unwrap_or(10),
      heartbeat_interval: options.heartbeat_interval.unwrap_or(30000),
      request_timeout: options.request_timeout.unwrap_or(10000),
      ws_url: options.ws_url,
    };

    let api_client = WeComApiClient::new(logger.clone(), resolved.request_timeout);
    let ws_manager = WsConnectionManager::new(
      logger.clone(),
      resolved.heartbeat_interval,
      resolved.reconnect_interval,
      resolved.max_reconnect_attempts,
      resolved.ws_url.clone(),
    );
    ws_manager.set_credentials(resolved.bot_id.clone(), resolved.secret.clone());

    let message_handler = MessageHandler::new(logger.clone());

    let client = WSClient {
      inner: Arc::new(WSClientInner {
        api_client,
        ws_manager,
        message_handler,
        logger,
        events: RwLock::new(EventHandlers::default()),
      }),
    };

    client.setup_ws_events();
    client
  }

  fn setup_ws_events(&self) {
    let client = self.clone();
    self.inner.ws_manager.set_on_connected(move || {
      client.emit_connected();
    });

    let client = self.clone();
    self.inner.ws_manager.set_on_authenticated(move || {
      client.inner.logger.info("Authenticated");
      client.emit_authenticated();
    });

    let client = self.clone();
    self.inner.ws_manager.set_on_disconnected(move |reason| {
      client.emit_disconnected(reason);
    });

    let client = self.clone();
    self.inner.ws_manager.set_on_reconnecting(move |attempt| {
      client.emit_reconnecting(attempt);
    });

    let client = self.clone();
    self.inner.ws_manager.set_on_error(move |err| {
      client.emit_error(err);
    });

    let client = self.clone();
    self.inner.ws_manager.set_on_message(move |frame| {
      client.inner.message_handler.handle_frame(frame, &client);
    });
  }

  pub fn connect(&self) {
    self.inner.ws_manager.connect();
  }

  pub fn disconnect(&self) {
    self.inner.ws_manager.disconnect();
  }

  pub fn is_connected(&self) -> bool {
    self.inner.ws_manager.is_connected()
  }

  pub fn api(&self) -> &WeComApiClient {
    &self.inner.api_client
  }

  pub async fn reply(&self, frame: &WsFrameHeaders, body: Value, cmd: Option<&str>) -> Result<WsFrame<Value>, AiBotError> {
    let req_id = frame.headers.get("req_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    self.inner.ws_manager.send_reply(req_id, body, cmd).await
  }

  pub async fn reply_stream(
    &self,
    frame: &WsFrameHeaders,
    stream_id: &str,
    content: &str,
    finish: bool,
    msg_item: Option<Vec<ReplyMsgItem>>,
    feedback: Option<ReplyFeedback>,
  ) -> Result<WsFrame<Value>, AiBotError> {
    let mut stream = StreamReplyBodyStream {
      id: stream_id.to_string(),
      finish: Some(finish),
      content: Some(content.to_string()),
      msg_item: None,
      feedback: None,
    };

    if finish {
      if let Some(items) = msg_item {
        if !items.is_empty() {
          stream.msg_item = Some(items);
        }
      }
    }

    if let Some(feedback) = feedback {
      stream.feedback = Some(feedback);
    }

    let body = StreamReplyBody {
      msgtype: "stream".to_string(),
      stream,
    };

    let body_value = serde_json::to_value(body).map_err(|e| AiBotError::Internal(e.to_string()))?;
    self.reply(frame, body_value, None).await
  }

  pub async fn reply_welcome(&self, frame: &WsFrameHeaders, body: WelcomeReplyBody) -> Result<WsFrame<Value>, AiBotError> {
    let body_value = serde_json::to_value(body).map_err(|e| AiBotError::Internal(e.to_string()))?;
    self.reply(frame, body_value, Some(WsCmd::RESPONSE_WELCOME)).await
  }

  pub async fn reply_template_card(&self, frame: &WsFrameHeaders, template_card: TemplateCard, feedback: Option<ReplyFeedback>) -> Result<WsFrame<Value>, AiBotError> {
    let mut card = template_card;
    if let Some(feedback) = feedback {
      card.feedback = Some(feedback);
    }

    let body = TemplateCardReplyBody {
      msgtype: "template_card".to_string(),
      template_card: card,
    };

    let body_value = serde_json::to_value(body).map_err(|e| AiBotError::Internal(e.to_string()))?;
    self.reply(frame, body_value, None).await
  }

  pub async fn reply_stream_with_card(
    &self,
    frame: &WsFrameHeaders,
    stream_id: &str,
    content: &str,
    finish: bool,
    options: Option<ReplyStreamWithCardOptions>,
  ) -> Result<WsFrame<Value>, AiBotError> {
    let mut stream = StreamReplyBodyStream {
      id: stream_id.to_string(),
      finish: Some(finish),
      content: Some(content.to_string()),
      msg_item: None,
      feedback: None,
    };

    if finish {
      if let Some(msg_item) = options.as_ref().and_then(|o| o.msg_item.clone()) {
        if !msg_item.is_empty() {
          stream.msg_item = Some(msg_item);
        }
      }
    }

    if let Some(feedback) = options.as_ref().and_then(|o| o.stream_feedback.clone()) {
      stream.feedback = Some(feedback);
    }

    let mut body = StreamWithTemplateCardReplyBody {
      msgtype: "stream_with_template_card".to_string(),
      stream,
      template_card: None,
    };

    if let Some(template_card) = options.as_ref().and_then(|o| o.template_card.clone()) {
      body.template_card = Some(if let Some(card_feedback) = options.as_ref().and_then(|o| o.card_feedback.clone()) {
        let mut c = template_card.clone();
        c.feedback = Some(card_feedback);
        c
      } else {
        template_card
      });
    }

    let body_value = serde_json::to_value(body).map_err(|e| AiBotError::Internal(e.to_string()))?;
    self.reply(frame, body_value, None).await
  }

  pub async fn update_template_card(&self, frame: &WsFrameHeaders, template_card: TemplateCard, userids: Option<Vec<String>>) -> Result<WsFrame<Value>, AiBotError> {
    let body = UpdateTemplateCardBody {
      response_type: "update_template_card".to_string(),
      userids,
      template_card,
    };
    let body_value = serde_json::to_value(body).map_err(|e| AiBotError::Internal(e.to_string()))?;
    self.reply(frame, body_value, Some(WsCmd::RESPONSE_UPDATE)).await
  }

  pub async fn send_message(&self, chatid: &str, body: SendMsgBody) -> Result<WsFrame<Value>, AiBotError> {
    let req_id = generate_req_id(WsCmd::SEND_MSG);
    let mut body_value = serde_json::to_value(body).map_err(|e| AiBotError::Internal(e.to_string()))?;

    if let Value::Object(ref mut map) = body_value {
      map.insert("chatid".to_string(), Value::String(chatid.to_string()));
    }

    self.inner.ws_manager.send_reply(req_id, body_value, Some(WsCmd::SEND_MSG)).await
  }

  pub async fn upload_media(&self, file_buffer: &[u8], options: UploadMediaOptions) -> Result<UploadMediaFinishResult, AiBotError> {
    let file_buffer = Arc::new(file_buffer.to_vec());
    let total_size = file_buffer.len();
    if total_size == 0 {
      return Err(AiBotError::UploadFailed("File buffer is empty".to_string()));
    }
    let chunk_size = 512 * 1024;
    let total_chunks = (total_size + chunk_size - 1) / chunk_size;

    if total_chunks > 100 {
      return Err(AiBotError::UploadFailed(format!(
        "File too large: {} chunks exceeds maximum of 100 chunks (max ~50MB)",
        total_chunks
      )));
    }

    let md5_hex = format!("{:x}", md5::compute(&*file_buffer));

    self.inner.logger.info(&format!(
      "Uploading media: type={}, filename={}, size={}, chunks={}",
      options.r#type, options.filename, total_size, total_chunks
    ));

    let init_req_id = generate_req_id(WsCmd::UPLOAD_MEDIA_INIT);
    let init_body = serde_json::json!({
      "type": options.r#type,
      "filename": options.filename,
      "total_size": total_size,
      "total_chunks": total_chunks,
      "md5": md5_hex,
    });

    let init_result = self.inner.ws_manager.send_reply(init_req_id, init_body, Some(WsCmd::UPLOAD_MEDIA_INIT)).await?;
    let upload_id = init_result
      .body
      .and_then(|v| v.get("upload_id").cloned())
      .and_then(|v| v.as_str().map(|s| s.to_string()))
      .ok_or_else(|| AiBotError::UploadFailed("Upload init failed: no upload_id returned".to_string()))?;

    let max_chunk_retries = 2u32;
    let max_concurrency = if total_chunks <= 4 { total_chunks } else if total_chunks <= 10 { 3 } else { 2 };

    let ws_manager = self.inner.ws_manager.clone();
    let logger = self.inner.logger.clone();
    let upload_id_shared = upload_id.clone();
    let file_buffer_shared = file_buffer.clone();

    let upload_chunk = move |chunk_index: usize| {
      let ws_manager = ws_manager.clone();
      let logger = logger.clone();
      let upload_id = upload_id_shared.clone();
      let file_buffer = file_buffer_shared.clone();

      async move {
        let start = chunk_index * chunk_size;
        let end = std::cmp::min(start + chunk_size, total_size);
        let chunk = &file_buffer[start..end];
        let base64_data = BASE64_STD.encode(chunk);

        let mut last_error: Option<AiBotError> = None;
        for attempt in 0..=max_chunk_retries {
          let chunk_req_id = generate_req_id(WsCmd::UPLOAD_MEDIA_CHUNK);
          let body = serde_json::json!({
            "upload_id": upload_id,
            "chunk_index": chunk_index,
            "base64_data": base64_data,
          });

          match ws_manager.send_reply(chunk_req_id, body, Some(WsCmd::UPLOAD_MEDIA_CHUNK)).await {
            Ok(_) => return Ok(()),
            Err(err) => {
              last_error = Some(err);
              if attempt < max_chunk_retries {
                let delay = Duration::from_millis(500 * (attempt as u64 + 1));
                logger.warn(&format!(
                  "Chunk {} upload failed (attempt {}/{}), retrying in {:?}",
                  chunk_index,
                  attempt + 1,
                  max_chunk_retries + 1,
                  delay
                ));
                tokio::time::sleep(delay).await;
              }
            }
          }
        }

        Err(last_error.unwrap_or_else(|| AiBotError::UploadFailed("Unknown chunk error".to_string())))
      }
    };

    if total_chunks <= 1 {
      upload_chunk(0).await?;
    } else {
      let results = stream::iter(0..total_chunks)
        .map(upload_chunk)
        .buffer_unordered(max_concurrency)
        .collect::<Vec<_>>()
        .await;

      let mut errors = vec![];
      for res in results {
        if let Err(e) = res { errors.push(e); }
      }

      if !errors.is_empty() {
        return Err(AiBotError::UploadFailed(format!(
          "Upload failed: {} chunk(s) failed. First error: {}",
          errors.len(), errors[0]
        )));
      }
    }

    self.inner.logger.info("All chunks uploaded, finishing...");

    let finish_req_id = generate_req_id(WsCmd::UPLOAD_MEDIA_FINISH);
    let finish_body = serde_json::json!({ "upload_id": upload_id });
    let finish_result = self.inner.ws_manager.send_reply(finish_req_id, finish_body, Some(WsCmd::UPLOAD_MEDIA_FINISH)).await?;

    let media_id = finish_result
      .body
      .as_ref()
      .and_then(|v| v.get("media_id"))
      .and_then(|v| v.as_str().map(|s| s.to_string()))
      .ok_or_else(|| AiBotError::UploadFailed("Upload finish failed: no media_id returned".to_string()))?;

    let r#type = finish_result
      .body
      .as_ref()
      .and_then(|v| v.get("type"))
      .and_then(|v| v.as_str())
      .map(|s| s.to_string())
      .unwrap_or_else(|| "".to_string());

    let created_at = finish_result
      .body
      .as_ref()
      .and_then(|v| v.get("created_at"))
      .and_then(|v| v.as_str())
      .map(|s| s.to_string())
      .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    Ok(UploadMediaFinishResult { r#type, media_id, created_at })
  }

  pub async fn reply_media(&self, frame: &WsFrameHeaders, media_type: &str, media_id: &str, video_options: Option<VideoOptions>) -> Result<WsFrame<Value>, AiBotError> {
    let body = build_media_body(media_type, media_id, video_options);
    let body_value = serde_json::to_value(body).map_err(|e| AiBotError::Internal(e.to_string()))?;
    self.reply(frame, body_value, None).await
  }

  pub async fn send_media_message(&self, chatid: &str, media_type: &str, media_id: &str, video_options: Option<VideoOptions>) -> Result<WsFrame<Value>, AiBotError> {
    let body = build_media_body(media_type, media_id, video_options);
    let msg = SendMsgBody::Media(body);
    self.send_message(chatid, msg).await
  }

  pub async fn download_file(&self, url: &str, aes_key: Option<&str>) -> Result<(Vec<u8>, Option<String>), AiBotError> {
    self.inner.logger.info("Downloading and decrypting file...");
    let (encrypted, filename) = self.inner.api_client.download_file_raw(url).await
      .map_err(AiBotError::HttpError)?;

    if let Some(key) = aes_key {
      let decrypted = decrypt_file(&encrypted, key).map_err(AiBotError::CryptoError)?;
      Ok((decrypted, filename))
    } else {
      self.inner.logger.warn("No aesKey provided, returning raw file data");
      Ok((encrypted, filename))
    }
  }

  // Event registration helpers
  pub fn on_message<F>(&self, f: F)
  where
    F: Fn(WsFrame<crate::types::message::BaseMessage>) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().message.push(Arc::new(f));
  }

  pub fn on_message_text<F>(&self, f: F)
  where
    F: Fn(WsFrame<crate::types::message::TextMessage>) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().message_text.push(Arc::new(f));
  }

  pub fn on_message_image<F>(&self, f: F)
  where
    F: Fn(WsFrame<crate::types::message::ImageMessage>) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().message_image.push(Arc::new(f));
  }

  pub fn on_message_mixed<F>(&self, f: F)
  where
    F: Fn(WsFrame<crate::types::message::MixedMessage>) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().message_mixed.push(Arc::new(f));
  }

  pub fn on_message_voice<F>(&self, f: F)
  where
    F: Fn(WsFrame<crate::types::message::VoiceMessage>) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().message_voice.push(Arc::new(f));
  }

  pub fn on_message_file<F>(&self, f: F)
  where
    F: Fn(WsFrame<crate::types::message::FileMessage>) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().message_file.push(Arc::new(f));
  }

  pub fn on_event<F>(&self, f: F)
  where
    F: Fn(WsFrame<crate::types::event::EventMessage>) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().event.push(Arc::new(f));
  }

  pub fn on_event_enter_chat<F>(&self, f: F)
  where
    F: Fn(WsFrame<crate::types::event::EventMessage>) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().event_enter_chat.push(Arc::new(f));
  }

  pub fn on_event_template_card_event<F>(&self, f: F)
  where
    F: Fn(WsFrame<crate::types::event::EventMessage>) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().event_template_card_event.push(Arc::new(f));
  }

  pub fn on_event_feedback_event<F>(&self, f: F)
  where
    F: Fn(WsFrame<crate::types::event::EventMessage>) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().event_feedback_event.push(Arc::new(f));
  }

  pub fn on_connected<F>(&self, f: F)
  where
    F: Fn() + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().connected.push(Arc::new(f));
  }

  pub fn on_authenticated<F>(&self, f: F)
  where
    F: Fn() + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().authenticated.push(Arc::new(f));
  }

  pub fn on_disconnected<F>(&self, f: F)
  where
    F: Fn(String) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().disconnected.push(Arc::new(f));
  }

  pub fn on_reconnecting<F>(&self, f: F)
  where
    F: Fn(u32) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().reconnecting.push(Arc::new(f));
  }

  pub fn on_error<F>(&self, f: F)
  where
    F: Fn(String) + Send + Sync + 'static,
  {
    self.inner.events.write().unwrap().error.push(Arc::new(f));
  }

  // emitters
  pub(crate) fn emit_message(&self, frame: WsFrame<crate::types::message::BaseMessage>) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.message { cb(frame.clone()); }
  }

  pub(crate) fn emit_message_text(&self, frame: WsFrame<crate::types::message::TextMessage>) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.message_text { cb(frame.clone()); }
  }

  pub(crate) fn emit_message_image(&self, frame: WsFrame<crate::types::message::ImageMessage>) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.message_image { cb(frame.clone()); }
  }

  pub(crate) fn emit_message_mixed(&self, frame: WsFrame<crate::types::message::MixedMessage>) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.message_mixed { cb(frame.clone()); }
  }

  pub(crate) fn emit_message_voice(&self, frame: WsFrame<crate::types::message::VoiceMessage>) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.message_voice { cb(frame.clone()); }
  }

  pub(crate) fn emit_message_file(&self, frame: WsFrame<crate::types::message::FileMessage>) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.message_file { cb(frame.clone()); }
  }

  pub(crate) fn emit_event(&self, frame: WsFrame<crate::types::event::EventMessage>) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.event { cb(frame.clone()); }
  }

  pub(crate) fn emit_event_enter_chat(&self, frame: WsFrame<crate::types::event::EventMessage>) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.event_enter_chat { cb(frame.clone()); }
  }

  pub(crate) fn emit_event_template_card_event(&self, frame: WsFrame<crate::types::event::EventMessage>) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.event_template_card_event { cb(frame.clone()); }
  }

  pub(crate) fn emit_event_feedback_event(&self, frame: WsFrame<crate::types::event::EventMessage>) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.event_feedback_event { cb(frame.clone()); }
  }

  pub(crate) fn emit_connected(&self) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.connected { cb(); }
  }

  pub(crate) fn emit_authenticated(&self) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.authenticated { cb(); }
  }

  pub(crate) fn emit_disconnected(&self, reason: String) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.disconnected { cb(reason.clone()); }
  }

  pub(crate) fn emit_reconnecting(&self, attempt: u32) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.reconnecting { cb(attempt); }
  }

  pub(crate) fn emit_error(&self, err: String) {
    let handlers = self.inner.events.read().unwrap();
    for cb in &handlers.error { cb(err.clone()); }
  }
}

impl Clone for WSClient {
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}

#[derive(Clone)]
pub struct ReplyStreamWithCardOptions {
  pub msg_item: Option<Vec<ReplyMsgItem>>,
  pub stream_feedback: Option<ReplyFeedback>,
  pub template_card: Option<TemplateCard>,
  pub card_feedback: Option<ReplyFeedback>,
}

#[derive(Clone)]
pub struct VideoOptions {
  pub title: Option<String>,
  pub description: Option<String>,
}

fn build_media_body(media_type: &str, media_id: &str, video_options: Option<VideoOptions>) -> SendMediaMsgBody {
  let mut body = SendMediaMsgBody {
    msgtype: media_type.to_string(),
    file: None,
    image: None,
    voice: None,
    video: None,
  };

  match media_type {
    "file" => body.file = Some(crate::types::api::MediaContent { media_id: media_id.to_string() }),
    "image" => body.image = Some(crate::types::api::MediaContent { media_id: media_id.to_string() }),
    "voice" => body.voice = Some(crate::types::api::MediaContent { media_id: media_id.to_string() }),
    "video" => {
      body.video = Some(crate::types::api::VideoMediaContent {
        media_id: media_id.to_string(),
        title: video_options.as_ref().and_then(|o| o.title.clone()),
        description: video_options.as_ref().and_then(|o| o.description.clone()),
      });
    }
    _ => {}
  }

  body
}
