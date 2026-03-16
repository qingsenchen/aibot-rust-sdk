use aes::Aes256;
use base64::engine::general_purpose::STANDARD as BASE64_STD;
use base64::Engine;
use cbc::Decryptor;
use cipher::{BlockDecryptMut, KeyIvInit, block_padding::NoPadding};

/// 使用 AES-256-CBC 解密文件
pub fn decrypt_file(encrypted: &[u8], aes_key_base64: &str) -> Result<Vec<u8>, String> {
  if encrypted.is_empty() {
    return Err("decrypt_file: encryptedBuffer is empty or not provided".to_string());
  }

  if aes_key_base64.trim().is_empty() {
    return Err("decrypt_file: aesKey must be a non-empty string".to_string());
  }

  let key = BASE64_STD.decode(aes_key_base64.as_bytes())
    .map_err(|e| format!("decrypt_file: invalid base64 aesKey: {}", e))?;

  if key.len() < 32 {
    return Err(format!("decrypt_file: aesKey length invalid: {}", key.len()));
  }

  let iv = &key[0..16];

  let mut buffer = encrypted.to_vec();
  let decrypted = Decryptor::<Aes256>::new_from_slices(&key, iv)
    .map_err(|e| format!("decrypt_file: cipher init failed: {}", e))?
    .decrypt_padded_mut::<NoPadding>(&mut buffer)
    .map_err(|e| format!("decrypt_file: Decryption failed - {}", e))?;

  if decrypted.is_empty() {
    return Err("decrypt_file: Decrypted data is empty".to_string());
  }

  // 手动去除 PKCS#7 填充（支持 32 字节 block）
  let pad_len = *decrypted.last().unwrap() as usize;
  if pad_len < 1 || pad_len > 32 || pad_len > decrypted.len() {
    return Err(format!("decrypt_file: Invalid PKCS#7 padding value: {}", pad_len));
  }

  let start = decrypted.len() - pad_len;
  for &b in &decrypted[start..] {
    if b as usize != pad_len {
      return Err("decrypt_file: Invalid PKCS#7 padding: padding bytes mismatch".to_string());
    }
  }

  Ok(decrypted[..start].to_vec())
}
