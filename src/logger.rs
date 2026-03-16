use chrono::Utc;
use crate::types::common::Logger;

/// 默认日志实现
/// 带有日志级别和时间戳的控制台日志
pub struct DefaultLogger {
  prefix: String,
}

impl Default for DefaultLogger {
  fn default() -> Self {
    Self::new("AiBotSDK")
  }
}

impl DefaultLogger {
  pub fn new(prefix: &str) -> Self {
    Self { prefix: prefix.to_string() }
  }

  fn format_time(&self) -> String {
    Utc::now().to_rfc3339()
  }

  fn format(&self, level: &str, message: &str) -> String {
    format!("[{}] [{}] [{}] {}", self.format_time(), self.prefix, level, message)
  }
}

impl Logger for DefaultLogger {
  fn debug(&self, message: &str) {
    println!("{}", self.format("DEBUG", message));
  }

  fn info(&self, message: &str) {
    println!("{}", self.format("INFO", message));
  }

  fn warn(&self, message: &str) {
    eprintln!("{}", self.format("WARN", message));
  }

  fn error(&self, message: &str) {
    eprintln!("{}", self.format("ERROR", message));
  }
}
