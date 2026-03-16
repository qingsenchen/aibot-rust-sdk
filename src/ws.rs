use crate::error::AiBotError;
use crate::types::api::{WsCmd, WsFrame};
use crate::types::common::Logger;
use crate::utils::generate_req_id;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::connect_async;

const DEFAULT_WS_URL: &str = "wss://openws.work.weixin.qq.com";

#[derive(Clone)]
pub struct WsConnectionManager {
  inner: Arc<Inner>,
}

struct Inner {
  logger: Arc<dyn Logger>,
  ws_url: String,
  heartbeat_interval: Duration,
  reconnect_base_delay: Duration,
  reconnect_max_delay: Duration,
  max_reconnect_attempts: i64,
  reply_ack_timeout: Duration,
  max_reply_queue_size: usize,

  bot_id: Mutex<String>,
  bot_secret: Mutex<String>,

  missed_pong_count: AtomicUsize,
  max_missed_pong: usize,

  is_manual_close: AtomicBool,
  is_connected: AtomicBool,
  started: AtomicBool,
  auth_ok: AtomicBool,

  sender: Mutex<Option<mpsc::UnboundedSender<Message>>>,

  pending_acks: Mutex<HashMap<String, oneshot::Sender<WsFrame<Value>>>>,
  reply_queues: Mutex<HashMap<String, VecDeque<ReplyQueueItem>>>,
  processing: Mutex<HashSet<String>>,

  callbacks: Mutex<WsCallbacks>,
}

struct ReplyQueueItem {
  frame: WsFrame<Value>,
  tx: oneshot::Sender<Result<WsFrame<Value>, AiBotError>>,
}

#[derive(Default)]
struct WsCallbacks {
  on_connected: Option<Arc<dyn Fn() + Send + Sync>>,
  on_authenticated: Option<Arc<dyn Fn() + Send + Sync>>,
  on_disconnected: Option<Arc<dyn Fn(String) + Send + Sync>>,
  on_reconnecting: Option<Arc<dyn Fn(u32) + Send + Sync>>,
  on_error: Option<Arc<dyn Fn(String) + Send + Sync>>,
  on_message: Option<Arc<dyn Fn(WsFrame<Value>) + Send + Sync>>,
}

impl WsConnectionManager {
  pub fn new(
    logger: Arc<dyn Logger>,
    heartbeat_interval_ms: u64,
    reconnect_base_delay_ms: u64,
    max_reconnect_attempts: i64,
    ws_url: Option<String>,
  ) -> Self {
    let ws_url = ws_url.unwrap_or_else(|| DEFAULT_WS_URL.to_string());
    Self {
      inner: Arc::new(Inner {
        logger,
        ws_url,
        heartbeat_interval: Duration::from_millis(heartbeat_interval_ms),
        reconnect_base_delay: Duration::from_millis(reconnect_base_delay_ms),
        reconnect_max_delay: Duration::from_millis(30_000),
        max_reconnect_attempts,
        reply_ack_timeout: Duration::from_millis(5000),
        max_reply_queue_size: 100,
        bot_id: Mutex::new(String::new()),
        bot_secret: Mutex::new(String::new()),
        missed_pong_count: AtomicUsize::new(0),
        max_missed_pong: 2,
        is_manual_close: AtomicBool::new(false),
        is_connected: AtomicBool::new(false),
        started: AtomicBool::new(false),
        auth_ok: AtomicBool::new(false),
        sender: Mutex::new(None),
        pending_acks: Mutex::new(HashMap::new()),
        reply_queues: Mutex::new(HashMap::new()),
        processing: Mutex::new(HashSet::new()),
        callbacks: Mutex::new(WsCallbacks::default()),
      }),
    }
  }

  pub fn set_credentials(&self, bot_id: String, bot_secret: String) {
    *self.inner.bot_id.lock().unwrap() = bot_id;
    *self.inner.bot_secret.lock().unwrap() = bot_secret;
  }

  pub fn set_on_connected<F>(&self, f: F)
  where
    F: Fn() + Send + Sync + 'static,
  {
    self.inner.callbacks.lock().unwrap().on_connected = Some(Arc::new(f));
  }

  pub fn set_on_authenticated<F>(&self, f: F)
  where
    F: Fn() + Send + Sync + 'static,
  {
    self.inner.callbacks.lock().unwrap().on_authenticated = Some(Arc::new(f));
  }

  pub fn set_on_disconnected<F>(&self, f: F)
  where
    F: Fn(String) + Send + Sync + 'static,
  {
    self.inner.callbacks.lock().unwrap().on_disconnected = Some(Arc::new(f));
  }

  pub fn set_on_reconnecting<F>(&self, f: F)
  where
    F: Fn(u32) + Send + Sync + 'static,
  {
    self.inner.callbacks.lock().unwrap().on_reconnecting = Some(Arc::new(f));
  }

  pub fn set_on_error<F>(&self, f: F)
  where
    F: Fn(String) + Send + Sync + 'static,
  {
    self.inner.callbacks.lock().unwrap().on_error = Some(Arc::new(f));
  }

  pub fn set_on_message<F>(&self, f: F)
  where
    F: Fn(WsFrame<Value>) + Send + Sync + 'static,
  {
    self.inner.callbacks.lock().unwrap().on_message = Some(Arc::new(f));
  }

  pub fn connect(&self) {
    if self.inner.started.swap(true, Ordering::SeqCst) {
      self.inner.logger.warn("Client already connected");
      return;
    }

    self.inner.logger.info("Establishing WebSocket connection...");
    let manager = self.clone();
    tokio::spawn(async move {
      manager.run_connection_loop().await;
    });
  }

  pub fn disconnect(&self) {
    self.inner.is_manual_close.store(true, Ordering::SeqCst);
    self.clear_pending_messages("Connection manually closed".to_string());

    if let Some(sender) = self.inner.sender.lock().unwrap().take() {
      let _ = sender.send(Message::Close(None));
    }

    self.inner.is_connected.store(false, Ordering::SeqCst);
    self.inner.logger.info("WebSocket connection manually closed");
  }

  pub fn is_connected(&self) -> bool {
    self.inner.is_connected.load(Ordering::SeqCst)
  }

  pub async fn send_reply(&self, req_id: String, body: Value, cmd: Option<&str>) -> Result<WsFrame<Value>, AiBotError> {
    let cmd = cmd.unwrap_or(WsCmd::RESPONSE);

    let frame = WsFrame {
      cmd: Some(cmd.to_string()),
      headers: {
        let mut h = HashMap::new();
        h.insert("req_id".to_string(), Value::String(req_id.clone()));
        h
      },
      body: Some(body),
      errcode: None,
      errmsg: None,
    };

    let (tx, rx) = oneshot::channel();

    {
      let mut queues = self.inner.reply_queues.lock().unwrap();
      let queue = queues.entry(req_id.clone()).or_insert_with(VecDeque::new);
      if queue.len() >= self.inner.max_reply_queue_size {
        return Err(AiBotError::Internal(format!(
          "Reply queue for reqId {} exceeds max size ({})",
          req_id, self.inner.max_reply_queue_size
        )));
      }

      queue.push_back(ReplyQueueItem { frame, tx });
    }

    {
      let mut processing = self.inner.processing.lock().unwrap();
      if !processing.contains(&req_id) {
        processing.insert(req_id.clone());
        let manager = self.clone();
        tokio::spawn(async move {
          manager.process_reply_queue(req_id).await;
        });
      }
    }

    rx.await.map_err(|_| AiBotError::Internal("Reply cancelled".to_string()))?
  }

  async fn run_connection_loop(&self) {
    let mut attempts: i64 = 0;

    loop {
      if self.inner.is_manual_close.load(Ordering::SeqCst) {
        break;
      }

      let url = self.inner.ws_url.clone();
      self.inner.logger.info(&format!("Connecting to WebSocket: {}...", url));

      match connect_async(&url).await {
        Ok((ws_stream, _)) => {
          attempts = 0;
          self.inner.is_connected.store(true, Ordering::SeqCst);
          self.inner.missed_pong_count.store(0, Ordering::SeqCst);
          self.inner.auth_ok.store(false, Ordering::SeqCst);

          let (mut write, mut read) = ws_stream.split();
          let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
          *self.inner.sender.lock().unwrap() = Some(tx);

          if let Some(cb) = &self.inner.callbacks.lock().unwrap().on_connected {
            cb();
          }

          // Send auth immediately after connection is established
          let _ = self.send_auth().await;

          let mut heartbeat = tokio::time::interval(self.inner.heartbeat_interval);

          loop {
            tokio::select! {
              _ = heartbeat.tick() => {
                if self.inner.auth_ok.load(Ordering::SeqCst) {
                  if self.inner.missed_pong_count.load(Ordering::SeqCst) >= self.inner.max_missed_pong {
                    self.inner.logger.warn("No heartbeat ack received, connection considered dead");
                    break;
                  }
                  self.inner.missed_pong_count.fetch_add(1, Ordering::SeqCst);
                  if let Err(e) = self.send_heartbeat().await {
                    self.inner.logger.error(&format!("Failed to send heartbeat: {}", e));
                  }
                }
              }
              Some(msg) = rx.recv() => {
                if let Err(e) = write.send(msg).await {
                  self.inner.logger.error(&format!("WebSocket send error: {}", e));
                  break;
                }
              }
              incoming = read.next() => {
                match incoming {
                  Some(Ok(Message::Text(text))) => {
                    self.handle_text_frame(&text).await;
                  }
                  Some(Ok(Message::Ping(payload))) => {
                    let _ = write.send(Message::Pong(payload)).await;
                  }
                  Some(Ok(Message::Close(frame))) => {
                    let reason = frame.map(|f| f.reason.to_string()).unwrap_or_else(|| "closed".to_string());
                    self.inner.logger.warn(&format!("WebSocket connection closed: {}", reason));
                    self.clear_pending_messages(format!("WebSocket connection closed ({})", reason));
                    if let Some(cb) = &self.inner.callbacks.lock().unwrap().on_disconnected {
                      cb(reason);
                    }
                    break;
                  }
                  Some(Ok(_)) => {}
                  Some(Err(e)) => {
                    self.inner.logger.error(&format!("WebSocket error: {}", e));
                    if let Some(cb) = &self.inner.callbacks.lock().unwrap().on_error {
                      cb(e.to_string());
                    }
                    break;
                  }
                  None => {
                    self.inner.logger.warn("WebSocket stream ended");
                    break;
                  }
                }
              }
            }
          }

          self.inner.is_connected.store(false, Ordering::SeqCst);
          *self.inner.sender.lock().unwrap() = None;
        }
        Err(e) => {
          self.inner.logger.error(&format!("Failed to create WebSocket connection: {}", e));
          if let Some(cb) = &self.inner.callbacks.lock().unwrap().on_error {
            cb(e.to_string());
          }
        }
      }

      if self.inner.is_manual_close.load(Ordering::SeqCst) {
        break;
      }

      if self.inner.max_reconnect_attempts != -1 && attempts >= self.inner.max_reconnect_attempts {
        self.inner.logger.error(&format!("Max reconnect attempts reached ({}), giving up", self.inner.max_reconnect_attempts));
        if let Some(cb) = &self.inner.callbacks.lock().unwrap().on_error {
          cb("Max reconnect attempts exceeded".to_string());
        }
        break;
      }

      attempts += 1;
      let delay = std::cmp::min(
        self.inner.reconnect_base_delay.mul_f64(2_f64.powi((attempts - 1) as i32)),
        self.inner.reconnect_max_delay,
      );

      self.inner.logger.info(&format!("Reconnecting in {:?} (attempt {})...", delay, attempts));
      if let Some(cb) = &self.inner.callbacks.lock().unwrap().on_reconnecting {
        cb(attempts as u32);
      }
      tokio::time::sleep(delay).await;
    }
  }

  async fn send_auth(&self) -> Result<(), AiBotError> {
    let bot_id = self.inner.bot_id.lock().unwrap().clone();
    let bot_secret = self.inner.bot_secret.lock().unwrap().clone();

    let frame = WsFrame {
      cmd: Some(WsCmd::SUBSCRIBE.to_string()),
      headers: {
        let mut h = HashMap::new();
        h.insert("req_id".to_string(), Value::String(generate_req_id(WsCmd::SUBSCRIBE)));
        h
      },
      body: Some(serde_json::json!({ "bot_id": bot_id, "secret": bot_secret })),
      errcode: None,
      errmsg: None,
    };

    self.send_frame(frame).await
  }

  async fn send_heartbeat(&self) -> Result<(), AiBotError> {
    let frame = WsFrame {
      cmd: Some(WsCmd::HEARTBEAT.to_string()),
      headers: {
        let mut h = HashMap::new();
        h.insert("req_id".to_string(), Value::String(generate_req_id(WsCmd::HEARTBEAT)));
        h
      },
      body: None,
      errcode: None,
      errmsg: None,
    };
    self.send_frame(frame).await
  }

  async fn send_frame(&self, frame: WsFrame<Value>) -> Result<(), AiBotError> {
    let sender = self.inner.sender.lock().unwrap().clone();
    if let Some(sender) = sender {
      let data = serde_json::to_string(&frame)
        .map_err(|e| AiBotError::WebSocketSendFailed(e.to_string()))?;
      sender.send(Message::Text(data))
        .map_err(|e| AiBotError::WebSocketSendFailed(e.to_string()))?;
      Ok(())
    } else {
      Err(AiBotError::WebSocketNotConnected)
    }
  }

  async fn handle_text_frame(&self, raw: &str) {
    let frame: WsFrame<Value> = match serde_json::from_str(raw) {
      Ok(f) => f,
      Err(e) => {
        self.inner.logger.error(&format!("Failed to parse WebSocket message: {}", e));
        return;
      }
    };

    let cmd = frame.cmd.clone().unwrap_or_default();
    let req_id = get_req_id(&frame.headers);

    if frame.cmd.as_deref() == Some(WsCmd::CALLBACK) || frame.cmd.as_deref() == Some(WsCmd::EVENT_CALLBACK) {
      self.inner.logger.info(&format!("[server -> plugin] cmd={}, reqId={:?}", cmd, req_id));
      if let Some(cb) = &self.inner.callbacks.lock().unwrap().on_message {
        cb(frame);
      }
      return;
    }

    if let Some(req_id) = req_id {
      if self.handle_reply_ack(&req_id, &frame) {
        return;
      }

      if req_id.starts_with(WsCmd::SUBSCRIBE) {
        if frame.errcode.unwrap_or(0) != 0 {
          let errmsg = frame.errmsg.clone().unwrap_or_else(|| "unknown".to_string());
          self.inner.logger.error(&format!("Authentication failed: errcode={:?}, errmsg={}", frame.errcode, errmsg));
          if let Some(cb) = &self.inner.callbacks.lock().unwrap().on_error {
            cb(format!("Authentication failed: {}", errmsg));
          }
          return;
        }
        self.inner.logger.info("Authentication successful");
        self.inner.missed_pong_count.store(0, Ordering::SeqCst);
        self.inner.auth_ok.store(true, Ordering::SeqCst);
        if let Some(cb) = &self.inner.callbacks.lock().unwrap().on_authenticated {
          cb();
        }
        return;
      }

      if req_id.starts_with(WsCmd::HEARTBEAT) {
        if frame.errcode.unwrap_or(0) != 0 {
          self.inner.logger.warn(&format!("Heartbeat ack error: errcode={:?}, errmsg={:?}", frame.errcode, frame.errmsg));
          return;
        }
        self.inner.missed_pong_count.store(0, Ordering::SeqCst);
        self.inner.logger.debug("Received heartbeat ack");
        return;
      }
    }

    self.inner.logger.warn(&format!("Received unknown frame (ignored): {}", raw));
  }

  fn handle_reply_ack(&self, req_id: &str, frame: &WsFrame<Value>) -> bool {
    let tx_opt = { self.inner.pending_acks.lock().unwrap().remove(req_id) };
    if let Some(tx) = tx_opt {
      let _ = tx.send(frame.clone());
      true
    } else {
      false
    }
  }

  async fn process_reply_queue(&self, req_id: String) {
    loop {
      let item_opt = {
        let mut queues = self.inner.reply_queues.lock().unwrap();
        match queues.get_mut(&req_id) {
          Some(queue) if !queue.is_empty() => Some(queue.pop_front().unwrap()),
          _ => {
            queues.remove(&req_id);
            None
          }
        }
      };

      let item = match item_opt {
        Some(i) => i,
        None => break,
      };

      let (ack_tx, ack_rx) = oneshot::channel();
      {
        let mut pending = self.inner.pending_acks.lock().unwrap();
        pending.insert(req_id.clone(), ack_tx);
      }

      if let Err(e) = self.send_frame(item.frame).await {
        let _ = item.tx.send(Err(e));
        self.inner.pending_acks.lock().unwrap().remove(&req_id);
        continue;
      }

      let ack = tokio::time::timeout(self.inner.reply_ack_timeout, ack_rx).await;
      match ack {
        Ok(Ok(frame)) => {
          if frame.errcode.unwrap_or(0) != 0 {
            let errcode = frame.errcode.unwrap_or(-1);
            let errmsg = frame.errmsg.unwrap_or_else(|| "unknown".to_string());
            let _ = item.tx.send(Err(AiBotError::ReplyAckError {
              req_id: req_id.clone(),
              errcode,
              errmsg,
            }));
          } else {
            let _ = item.tx.send(Ok(frame));
          }
        }
        Ok(Err(_)) => {
          let _ = item.tx.send(Err(AiBotError::Internal("Reply cancelled".to_string())));
        }
        Err(_) => {
          let _ = item.tx.send(Err(AiBotError::ReplyAckTimeout(
            self.inner.reply_ack_timeout.as_millis() as u64,
            req_id.clone(),
          )));
        }
      }
    }

    self.inner.processing.lock().unwrap().remove(&req_id);

  }

  fn clear_pending_messages(&self, reason: String) {
    let mut pending = self.inner.pending_acks.lock().unwrap();
    pending.clear();

    let mut queues = self.inner.reply_queues.lock().unwrap();
    for (_req_id, queue) in queues.iter_mut() {
      for item in queue.drain(..) {
        let _ = item.tx.send(Err(AiBotError::Internal(reason.clone())));
      }
    }
    queues.clear();

    self.inner.processing.lock().unwrap().clear();
  }
}

fn get_req_id(headers: &HashMap<String, Value>) -> Option<String> {
  headers.get("req_id").and_then(|v| v.as_str().map(|s| s.to_string()))
}
