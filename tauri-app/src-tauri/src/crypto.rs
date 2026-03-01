use aes::Aes256;
use aes::cipher::{KeyInit, BlockEncrypt, BlockDecrypt, generic_array::GenericArray};
use sha2::{Sha256, Digest};
use anyhow::Result;

/// 加密配置（简化版本，不再需要 salt 和 validation_token）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CryptoConfig {
    // 预留扩展字段
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {}
    }
}

/// 密码加密器（使用固定密钥）
pub struct PasswordEncryptor {
    cipher: Aes256,
}

impl PasswordEncryptor {
    /// 固定的内部密钥（基于应用标识符生成）
    fn get_internal_key() -> [u8; 32] {
        // 使用固定的应用密钥（在实际应用中应该使用更安全的方式，如操作系统的密钥库）
        let app_secret = "WorkToolsPasswordManager2024InternalKey";
        let mut hasher = Sha256::new();
        hasher.update(app_secret.as_bytes());
        hasher.update(b"SALT_FIX_FOR_LOCAL_ENCRYPTION");
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result[..32]);
        key
    }

    /// 创建新的加密器实例（自动初始化）
    pub fn new(_config: CryptoConfig) -> Self {
        let key = Self::get_internal_key();
        let cipher = Aes256::new(&GenericArray::from(key));
        Self {
            cipher,
        }
    }


    /// 使用指定 cipher 加密文本
    fn encrypt_with_cipher(cipher: &Aes256, plaintext: &str) -> Result<String> {
        use aes::cipher::generic_array::GenericArray;

        let plaintext_bytes = plaintext.as_bytes();

        // PKCS7 风格填充
        let block_size = 16;
        let padding_len = if plaintext_bytes.len() % block_size == 0 {
            block_size // 如果正好是 16 的倍数,填充 16 个字节
        } else {
            block_size - (plaintext_bytes.len() % block_size)
        };

        let mut padded_data = plaintext_bytes.to_vec();

        // 填充 padding_len 个值为 padding_len 的字节
        for _ in 0..padding_len {
            padded_data.push(padding_len as u8);
        }

        // 按 16 字节分块加密 (AES ECB 模式)
        let mut encrypted_data = Vec::new();
        for chunk in padded_data.chunks(16) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            cipher.encrypt_block(GenericArray::from_mut_slice(&mut block));
            encrypted_data.extend_from_slice(&block);
        }

        Ok(hex::encode(encrypted_data))
    }

    /// 使用指定 cipher 解密文本
    fn decrypt_with_cipher(cipher: &Aes256, ciphertext: &str) -> Result<String> {
        use aes::cipher::generic_array::GenericArray;

        let encrypted_data = hex::decode(ciphertext)?;

        if encrypted_data.len() % 16 != 0 {
            return Err(anyhow::anyhow!("密文长度无效"));
        }

        let mut decrypted_data = Vec::new();
        for chunk in encrypted_data.chunks(16) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            cipher.decrypt_block(GenericArray::from_mut_slice(&mut block));
            decrypted_data.extend_from_slice(&block);
        }

        // 移除 PKCS7 填充
        if decrypted_data.is_empty() {
            return Err(anyhow::anyhow!("解密结果为空"));
        }

        let padding_len = decrypted_data[decrypted_data.len() - 1] as usize;

        // 验证填充长度
        if padding_len > 16 || padding_len == 0 {
            return Err(anyhow::anyhow!("填充长度无效"));
        }

        // 验证所有填充字节的值都等于 padding_len
        for i in (decrypted_data.len() - padding_len)..decrypted_data.len() {
            if decrypted_data[i] != padding_len as u8 {
                return Err(anyhow::anyhow!("填充数据无效"));
            }
        }

        let plaintext_len = decrypted_data.len() - padding_len;
        decrypted_data.truncate(plaintext_len);

        String::from_utf8(decrypted_data).map_err(|e| anyhow::anyhow!("UTF-8 解码失败: {}", e))
    }

    /// 加密密码
    pub fn encrypt_password(&self, password: &str) -> Result<String> {
        Self::encrypt_with_cipher(&self.cipher, password)
    }

    /// 解密密码
    pub fn decrypt_password(&self, encrypted_password: &str) -> Result<String> {
        Self::decrypt_with_cipher(&self.cipher, encrypted_password)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let config = CryptoConfig::default();
        let encryptor = PasswordEncryptor::new(config);

        // 加密测试密码
        let encrypted = encryptor.encrypt_password("mypassword").unwrap();
        assert_ne!(encrypted, "mypassword");

        // 解密测试密码
        let decrypted = encryptor.decrypt_password(&encrypted).unwrap();
        assert_eq!(decrypted, "mypassword");
    }
}
