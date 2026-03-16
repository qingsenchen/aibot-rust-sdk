use rand::{RngCore, rngs::OsRng};
use hex::encode;

/// 生成随机字符串
pub fn generate_random_string(length: usize) -> String {
  let mut bytes = vec![0u8; (length + 1) / 2];
  OsRng.fill_bytes(&mut bytes);
  let mut s = encode(bytes);
  s.truncate(length);
  s
}

/// 生成唯一请求 ID
/// 格式：`{prefix}_{timestamp}_{random}`
pub fn generate_req_id(prefix: &str) -> String {
  let timestamp = chrono::Utc::now().timestamp_millis();
  let random = generate_random_string(8);
  format!("{}_{}_{}", prefix, timestamp, random)
}
