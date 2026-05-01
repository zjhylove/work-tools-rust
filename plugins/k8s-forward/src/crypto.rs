//! # K8s 转发插件的密码加密模块
//!
//! 与 password-manager 使用相同的 AES-256 ECB + PKCS7 方案。
//! 不同之处在于使用不同的固定种子（确保各插件密钥独立）。
//!
//! ## Rust 知识点
//! 同一个加密算法在不同插件中有独立的实现，原因是：
//! 1. 各插件使用不同的密钥种子（安全隔离）
//! 2. 插件之间不应有代码依赖
//! 3. 每种实现可以独立演进而互不影响

use aes::Aes256;
use aes::cipher::{KeyInit, BlockEncrypt, BlockDecrypt, generic_array::GenericArray};
use sha2::{Sha256, Digest};
use anyhow::Result;

/// 密码加密器（K8s 转发专用密钥）
pub struct PasswordEncryptor {
    cipher: Aes256, // AES-256 实例（包含密钥）
}

impl PasswordEncryptor {
    /// 生成 K8s 转发专用的内部密钥
    fn get_internal_key() -> [u8; 32] {
        let app_secret = "WorkToolsK8sForward2024InternalKey!";
        let mut hasher = Sha256::new();
        hasher.update(app_secret.as_bytes());
        hasher.update(b"K8S_FORWARD_SALT_FIXED");
        let result = hasher.finalize();
        let mut key = [0u8; 32]; // 256 位 = 32 字节
        key.copy_from_slice(&result[..32]);
        key
    }

    pub fn new() -> Self {
        let key = Self::get_internal_key();
        let cipher = Aes256::new(&GenericArray::from(key));
        Self { cipher }
    }

    /// 加密纯文本 → Hex 编码的密文
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let plaintext_bytes = plaintext.as_bytes();
        let block_size = 16;

        // PKCS7 填充
        let padding_len = if plaintext_bytes.len().is_multiple_of(block_size) {
            block_size
        } else {
            block_size - (plaintext_bytes.len() % block_size)
        };
        let mut padded_data = plaintext_bytes.to_vec();
        for _ in 0..padding_len {
            padded_data.push(padding_len as u8);
        }

        // 逐块加密
        let mut encrypted_data = Vec::new();
        for chunk in padded_data.chunks(16) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            self.cipher.encrypt_block(GenericArray::from_mut_slice(&mut block));
            encrypted_data.extend_from_slice(&block);
        }

        Ok(hex::encode(encrypted_data))
    }

    /// 解密 Hex 密文 → 纯文本
    pub fn decrypt(&self, ciphertext: &str) -> Result<String> {
        let encrypted_data = hex::decode(ciphertext)?;

        if encrypted_data.len() % 16 != 0 {
            return Err(anyhow::anyhow!("密文长度无效"));
        }

        // 逐块解密
        let mut decrypted_data = Vec::new();
        for chunk in encrypted_data.chunks(16) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            self.cipher.decrypt_block(GenericArray::from_mut_slice(&mut block));
            decrypted_data.extend_from_slice(&block);
        }

        // 移除 PKCS7 填充
        if decrypted_data.is_empty() {
            return Err(anyhow::anyhow!("解密结果为空"));
        }
        let padding_len = decrypted_data[decrypted_data.len() - 1] as usize;
        if padding_len > 16 || padding_len == 0 {
            return Err(anyhow::anyhow!("填充长度无效"));
        }
        decrypted_data.truncate(decrypted_data.len() - padding_len);

        String::from_utf8(decrypted_data).map_err(|e| anyhow::anyhow!("UTF-8 解码失败: {}", e))
    }
}
