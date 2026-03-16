/// 通用基础类型定义

/// 日志接口
pub trait Logger: Send + Sync {
  fn debug(&self, message: &str);
  fn info(&self, message: &str);
  fn warn(&self, message: &str);
  fn error(&self, message: &str);
}
