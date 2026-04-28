use aes::Aes256;
use aes::cipher::{KeyInit, BlockEncrypt, BlockDecrypt, generic_array::GenericArray};
use sha2::{Sha256, Digest};
use anyhow::Result;

pub struct PasswordEncryptor {
    cipher: Aes256,
}

impl PasswordEncryptor {
    fn get_internal_key() -> [u8; 32] {
        let app_secret = "WorkToolsK8sForward2024InternalKey!";
        let mut hasher = Sha256::new();
        hasher.update(app_secret.as_bytes());
        hasher.update(b"K8S_FORWARD_SALT_FIXED");
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result[..32]);
        key
    }

    pub fn new() -> Self {
        let key = Self::get_internal_key();
        let cipher = Aes256::new(&GenericArray::from(key));
        Self { cipher }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let plaintext_bytes = plaintext.as_bytes();
        let block_size = 16;
        let padding_len = if plaintext_bytes.len().is_multiple_of(block_size) {
            block_size
        } else {
            block_size - (plaintext_bytes.len() % block_size)
        };
        let mut padded_data = plaintext_bytes.to_vec();
        for _ in 0..padding_len {
            padded_data.push(padding_len as u8);
        }
        let mut encrypted_data = Vec::new();
        for chunk in padded_data.chunks(16) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            self.cipher.encrypt_block(GenericArray::from_mut_slice(&mut block));
            encrypted_data.extend_from_slice(&block);
        }
        Ok(hex::encode(encrypted_data))
    }

    pub fn decrypt(&self, ciphertext: &str) -> Result<String> {
        let encrypted_data = hex::decode(ciphertext)?;
        if encrypted_data.len() % 16 != 0 {
            return Err(anyhow::anyhow!("密文长度无效"));
        }
        let mut decrypted_data = Vec::new();
        for chunk in encrypted_data.chunks(16) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            self.cipher.decrypt_block(GenericArray::from_mut_slice(&mut block));
            decrypted_data.extend_from_slice(&block);
        }
        if decrypted_data.is_empty() {
            return Err(anyhow::anyhow!("解密结果为空"));
        }
        let padding_len = decrypted_data[decrypted_data.len() - 1] as usize;
        if padding_len > 16 || padding_len == 0 {
            return Err(anyhow::anyhow!("填充长度无效"));
        }
        let padding_start = decrypted_data.len() - padding_len;
        for byte in &decrypted_data[padding_start..] {
            if *byte != padding_len as u8 {
                return Err(anyhow::anyhow!("填充数据无效"));
            }
        }
        decrypted_data.truncate(decrypted_data.len() - padding_len);
        String::from_utf8(decrypted_data).map_err(|e| anyhow::anyhow!("UTF-8 解码失败: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let encryptor = PasswordEncryptor::new();
        let encrypted = encryptor.encrypt("my_secret_password").unwrap();
        assert_ne!(encrypted, "my_secret_password");
        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "my_secret_password");
    }

    #[test]
    fn test_empty_string() {
        let encryptor = PasswordEncryptor::new();
        let encrypted = encryptor.encrypt("").unwrap();
        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "");
    }
}
