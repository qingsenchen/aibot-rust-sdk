use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiBotError {
  #[error("WebSocket not connected")]
  WebSocketNotConnected,
  #[error("WebSocket send failed: {0}")]
  WebSocketSendFailed(String),
  #[error("Reply ack timeout ({0}ms) for reqId: {1}")]
  ReplyAckTimeout(u64, String),
  #[error("Reply ack error: reqId={req_id}, errcode={errcode}, errmsg={errmsg}")]
  ReplyAckError { req_id: String, errcode: i64, errmsg: String },
  #[error("Upload failed: {0}")]
  UploadFailed(String),
  #[error("Crypto error: {0}")]
  CryptoError(String),
  #[error("HTTP error: {0}")]
  HttpError(String),
  #[error("Internal error: {0}")]
  Internal(String),
}
