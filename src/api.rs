use crate::types::common::Logger;
use reqwest::header::CONTENT_DISPOSITION;
use std::sync::Arc;
use url::Url;

/// 企业微信 API 客户端
/// 仅负责文件下载等 HTTP 辅助功能，消息收发均走 WebSocket 通道
pub struct WeComApiClient {
  http_client: reqwest::Client,
  logger: Arc<dyn Logger>,
}

impl WeComApiClient {
  pub fn new(logger: Arc<dyn Logger>, timeout_ms: u64) -> Self {
    let http_client = reqwest::Client::builder()
      .timeout(std::time::Duration::from_millis(timeout_ms))
      .build()
      .expect("failed to build reqwest client");
    Self { http_client, logger }
  }

  /// 下载文件（返回原始 Buffer 及文件名）
  pub async fn download_file_raw(&self, url: &str) -> Result<(Vec<u8>, Option<String>), String> {
    self.logger.info("Downloading file...");

    let response = self.http_client.get(url).send().await
      .map_err(|e| format!("File download failed: {}", e))?;

    let headers = response.headers().clone();
    let bytes = response.bytes().await
      .map_err(|e| format!("File download failed: {}", e))?;

    let filename = headers
      .get(CONTENT_DISPOSITION)
      .and_then(|v| v.to_str().ok())
      .and_then(parse_content_disposition_filename);

    self.logger.info("File downloaded successfully");
    Ok((bytes.to_vec(), filename))
  }
}

fn parse_content_disposition_filename(header: &str) -> Option<String> {
  let lower = header.to_lowercase();

  if let Some(idx) = lower.find("filename*=") {
    let value = &header[idx + "filename*=".len()..];
    let value = value.trim_start_matches(' ');
    if let Some(pos) = value.find("''") {
      let encoded = &value[pos + 2..];
      let encoded = encoded.split(';').next().unwrap_or("").trim();
      return percent_decode(encoded);
    }
  }

  if let Some(idx) = lower.find("filename=") {
    let value = &header[idx + "filename=".len()..];
    let value = value.trim_start_matches(' ');
    let value = value.trim_matches('"');
    let value = value.split(';').next().unwrap_or("").trim();
    return percent_decode(value);
  }

  None
}

fn percent_decode(value: &str) -> Option<String> {
  Url::parse(&format!("https://dummy/?v={}", value))
    .ok()
    .and_then(|url| url.query_pairs().find(|(k, _)| k == "v").map(|(_, v)| v.to_string()))
}
