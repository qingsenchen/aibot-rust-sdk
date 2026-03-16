use crate::types::common::Logger;
use std::sync::Arc;

/// WSClient 配置选项
#[derive(Clone)]
pub struct WSClientOptions {
  /// 机器人 ID（在企业微信后台获取）
  pub bot_id: String,
  /// 机器人 Secret（在企业微信后台获取）
  pub secret: String,
  /// WebSocket 重连基础延迟（毫秒），实际延迟按指数退避递增，默认 1000
  pub reconnect_interval: Option<u64>,
  /// 最大重连次数，默认 10，设为 -1 表示无限重连
  pub max_reconnect_attempts: Option<i64>,
  /// 心跳间隔（毫秒），默认 30000
  pub heartbeat_interval: Option<u64>,
  /// 请求超时时间（毫秒），默认 10000
  pub request_timeout: Option<u64>,
  /// 自定义 WebSocket 连接地址，默认 wss://openws.work.weixin.qq.com
  pub ws_url: Option<String>,
  /// 自定义日志函数
  pub logger: Option<Arc<dyn Logger>>,
}
