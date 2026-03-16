use crate::types::api::{WsCmd, WsFrame};
use crate::types::common::Logger;
use crate::types::event::{EventMessage};
use crate::types::message::{
  BaseMessage,
  TextMessage,
  ImageMessage,
  MixedMessage,
  VoiceMessage,
  FileMessage,
};
use crate::client::WSClient;
use serde_json::Value;
use std::sync::Arc;

/// 消息处理器
/// 负责解析 WebSocket 帧并分发为具体的消息事件和事件回调
pub struct MessageHandler {
  logger: Arc<dyn Logger>,
}

impl MessageHandler {
  pub fn new(logger: Arc<dyn Logger>) -> Self {
    Self { logger }
  }

  pub fn handle_frame(&self, frame: WsFrame<Value>, emitter: &WSClient) {
    let body = match frame.body.clone() {
      Some(b) => b,
      None => {
        self.logger.warn("Received invalid message format: body is empty");
        return;
      }
    };

    if !body.get("msgtype").is_some() {
      self.logger.warn("Received invalid message format: missing msgtype");
      return;
    }

    if frame.cmd.as_deref() == Some(WsCmd::EVENT_CALLBACK) {
      self.handle_event_callback(frame, emitter);
      return;
    }

    self.handle_message_callback(frame, emitter);
  }

  fn handle_message_callback(&self, frame: WsFrame<Value>, emitter: &WSClient) {
    let body_value = frame.body.clone().unwrap_or(Value::Null);
    let base: BaseMessage = match serde_json::from_value(body_value.clone()) {
      Ok(b) => b,
      Err(e) => {
        self.logger.error(&format!("Failed to parse BaseMessage: {}", e));
        return;
      }
    };

    emitter.emit_message(make_typed_frame(&frame, base.clone()));

    match base.msgtype.as_str() {
      "text" => {
        if let Ok(typed) = serde_json::from_value::<TextMessage>(body_value) {
          emitter.emit_message_text(make_typed_frame(&frame, typed));
        }
      }
      "image" => {
        if let Ok(typed) = serde_json::from_value::<ImageMessage>(body_value) {
          emitter.emit_message_image(make_typed_frame(&frame, typed));
        }
      }
      "mixed" => {
        if let Ok(typed) = serde_json::from_value::<MixedMessage>(body_value) {
          emitter.emit_message_mixed(make_typed_frame(&frame, typed));
        }
      }
      "voice" => {
        if let Ok(typed) = serde_json::from_value::<VoiceMessage>(body_value) {
          emitter.emit_message_voice(make_typed_frame(&frame, typed));
        }
      }
      "file" => {
        if let Ok(typed) = serde_json::from_value::<FileMessage>(body_value) {
          emitter.emit_message_file(make_typed_frame(&frame, typed));
        }
      }
      other => {
        self.logger.debug(&format!("Received unhandled message type: {}", other));
      }
    }
  }

  fn handle_event_callback(&self, frame: WsFrame<Value>, emitter: &WSClient) {
    let body_value = frame.body.clone().unwrap_or(Value::Null);
    let event: EventMessage = match serde_json::from_value(body_value.clone()) {
      Ok(e) => e,
      Err(err) => {
        self.logger.error(&format!("Failed to parse EventMessage: {}", err));
        return;
      }
    };

    emitter.emit_event(make_typed_frame(&frame, event.clone()));

    if let Some(eventtype) = body_value
      .get("event")
      .and_then(|v| v.get("eventtype"))
      .and_then(|v| v.as_str())
    {
      match eventtype {
        "enter_chat" => emitter.emit_event_enter_chat(make_typed_frame(&frame, event)),
        "template_card_event" => emitter.emit_event_template_card_event(make_typed_frame(&frame, event)),
        "feedback_event" => emitter.emit_event_feedback_event(make_typed_frame(&frame, event)),
        _ => {
          self.logger.debug(&format!("Received unknown event type: {}", eventtype));
        }
      }
    } else {
      self.logger.debug("Received event callback without eventtype");
    }
  }
}

fn make_typed_frame<T: Clone>(frame: &WsFrame<Value>, body: T) -> WsFrame<T> {
  WsFrame {
    cmd: frame.cmd.clone(),
    headers: frame.headers.clone(),
    body: Some(body),
    errcode: frame.errcode,
    errmsg: frame.errmsg.clone(),
  }
}
