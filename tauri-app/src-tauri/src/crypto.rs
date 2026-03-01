use aes::Aes256;
use aes::cipher::{KeyInit, BlockEncrypt, BlockDecrypt, generic_array::GenericArray};
use sha2::{Sha256, Digest};
use anyhow::Result;

/// 加密配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CryptoConfig {
    /// 加密后的主密码
    pub master_password: Option<String>,
    /// 盐值
    pub salt: Option<String>,
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            master_password: None,
            salt: None,
        }
    }
}

/// 密码加密器
pub struct PasswordEncryptor {
    cipher: Option<Aes256>,
    config: CryptoConfig,
}

impl PasswordEncryptor {
    /// 创建新的加密器实例
    pub fn new(config: CryptoConfig) -> Self {
        Self {
            cipher: None,
            config,
        }
    }

    /// 生成随机盐值
    pub fn generate_salt() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        let salt: String = (0..32)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        salt
    }

    /// 从密码和盐值生成 AES-256 密钥
    fn derive_key(password: &str, salt: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(salt.as_bytes());
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result[..32]);
        key
    }

    /// 初始化或验证主密码
    pub fn init_or_verify_master_password(&mut self, password: &str) -> Result<bool> {
        // 首次设置主密码
        if self.config.master_password.is_none() {
            let salt = Self::generate_salt();
            let key = Self::derive_key(password, &salt);

            // 使用 AES 加密主密码
            let cipher = Aes256::new(&GenericArray::from(key));
            let encrypted = Self::encrypt_with_cipher(&cipher, password)?;

            self.config.master_password = Some(encrypted);
            self.config.salt = Some(salt);
            self.cipher = Some(cipher);

            return Ok(true);
        }

        // 验证主密码
        let salt = self.config.salt.as_ref().ok_or_else(|| anyhow::anyhow!("盐值不存在"))?;
        let key = Self::derive_key(password, salt);
        let cipher = Aes256::new(&GenericArray::from(key));

        let stored_encrypted = self.config.master_password.as_ref().unwrap();

        // 尝试解密,如果失败说明密码错误
        let decrypted = match Self::decrypt_with_cipher(&cipher, stored_encrypted) {
            Ok(text) => text,
            Err(_) => return Ok(false), // 解密失败,密码错误
        };

        if password == decrypted {
            self.cipher = Some(cipher);
            Ok(true)
        } else {
            Ok(false)
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

    /// 加密密码 (必须在验证主密码后调用)
    pub fn encrypt_password(&self, password: &str) -> Result<String> {
        let cipher = self.cipher.as_ref().ok_or_else(|| anyhow::anyhow!("未验证主密码"))?;
        Self::encrypt_with_cipher(cipher, password)
    }

    /// 解密密码 (必须在验证主密码后调用)
    pub fn decrypt_password(&self, encrypted_password: &str) -> Result<String> {
        let cipher = self.cipher.as_ref().ok_or_else(|| anyhow::anyhow!("未验证主密码"))?;
        Self::decrypt_with_cipher(cipher, encrypted_password)
    }

    /// 获取加密配置 (用于持久化)
    pub fn get_config(&self) -> CryptoConfig {
        self.config.clone()
    }

    /// 检查是否已设置主密码
    pub fn has_master_password(&self) -> bool {
        self.config.master_password.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let config = CryptoConfig::default();
        let mut encryptor = PasswordEncryptor::new(config);

        // 设置主密码
        let result = encryptor.init_or_verify_master_password("test123");
        assert!(result.is_ok());
        assert!(result.unwrap());

        // 加密测试密码
        let encrypted = encryptor.encrypt_password("mypassword").unwrap();
        assert_ne!(encrypted, "mypassword");

        // 解密测试密码
        let decrypted = encryptor.decrypt_password(&encrypted).unwrap();
        assert_eq!(decrypted, "mypassword");
    }

    #[test]
    fn test_verify_master_password() {
        let config = CryptoConfig::default();
        let mut encryptor = PasswordEncryptor::new(config);

        // 设置主密码
        encryptor.init_or_verify_master_password("test123").unwrap();

        // 创建新的加密器实例并验证
        let config = encryptor.get_config();
        let mut encryptor2 = PasswordEncryptor::new(config);

        // 正确的密码
        let result = encryptor2.init_or_verify_master_password("test123").unwrap();
        assert!(result);

        // 错误的密码
        let result = encryptor2.init_or_verify_master_password("wrong").unwrap();
        assert!(!result);
    }
}
