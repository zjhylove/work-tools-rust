//! # 密码加密模块
//!
//! 使用 AES-256 加密算法保护存储的密码。
//!
//! ## 加密方案
//! - **算法**: AES-256 (ECB 模式) + PKCS7 填充
//! - **密钥派生**: SHA-256 哈希固定种子 → 256 位密钥
//! - **输出编码**: Hex 十六进制字符串
//!
//! ## 安全性说明
//! 这是一个简化实现，使用硬编码种子派生密钥。对于生产环境，建议使用：
//! - 操作系统密钥库（Windows Credential Manager / macOS Keychain）
//! - 用户主密码 + PBKDF2/Argon2 密钥派生
//!
//! ## Rust 知识点
//! - `aes` crate: AES 块加密算法的纯 Rust 实现
//! - `GenericArray`: 编译时固定长度的数组（因为 AES 要求 16 字节的块）
//! - `sha2` crate: SHA-256 哈希算法
//! - `hex` crate: 二进制 ↔ 十六进制字符串转换

use aes::Aes256;
use aes::cipher::{KeyInit, BlockEncrypt, BlockDecrypt, generic_array::GenericArray};
use sha2::{Sha256, Digest};
use anyhow::Result;

/// 加密配置（简化版本，预留扩展）
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct CryptoConfig {
    // 预留扩展字段，如自定义 salt、迭代次数等
}

/// 密码加密器（使用固定密钥）
///
/// `Aes256` 是 AES-256 算法的实例，存储了密钥并可以重复用于加密/解密。
pub struct PasswordEncryptor {
    cipher: Aes256, // 包含 256 位密钥的 AES 实例
}

impl PasswordEncryptor {
    /// 生成固定的内部密钥（基于应用标识符）
    ///
    /// SHA-256("WorkToolsPasswordManager2024InternalKey" + "SALT_FIX_FOR_LOCAL_ENCRYPTION") → 256 位密钥
    ///
    /// ## Rust 知识点: 哈希计算
    /// `Sha256::new()` 创建哈希器
    /// `.update()` 追加数据（可以多次调用，等价于一次传入）
    /// `.finalize()` 完成计算，返回定长的哈希值
    fn get_internal_key() -> [u8; 32] {
        let app_secret = "WorkToolsPasswordManager2024InternalKey";
        let mut hasher = Sha256::new();
        hasher.update(app_secret.as_bytes());
        hasher.update(b"SALT_FIX_FOR_LOCAL_ENCRYPTION"); // `b"..."` 是字节串字面量
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        // `copy_from_slice` 从切片复制字节，要求长度精确匹配
        key.copy_from_slice(&result[..32]);
        key
    }

    /// 创建新的加密器实例（自动初始化密钥）
    pub fn new(_config: CryptoConfig) -> Self {
        let key = Self::get_internal_key();
        // `GenericArray::from(key)` 将 [u8; 32] 转换为 AES 要求的密钥类型
        let cipher = Aes256::new(&GenericArray::from(key));
        Self { cipher }
    }

    /// 使用指定 cipher 加密文本
    ///
    /// ## ECB 模式 + PKCS7 填充
    /// AES 是块加密，每个块 16 字节。
    /// - **PKCS7 填充**: 如果数据长度不是 16 的倍数，补充 N 个值为 N 的字节
    /// - **示例**: 数据 10 字节 → 补充 6 个 `0x06` → 16 字节块
    /// - **特例**: 数据刚好 16 字节 → 补充 16 个 `0x10`（解码时知道如何去除）
    ///
    /// **ECB 模式不安全**（相同的明文块产生相同的密文），生产环境应用 CBC 或 GCM。
    fn encrypt_with_cipher(cipher: &Aes256, plaintext: &str) -> Result<String> {
        let plaintext_bytes = plaintext.as_bytes();

        // 计算 PKCS7 填充长度
        let block_size = 16;
        let padding_len = if plaintext_bytes.len().is_multiple_of(block_size) {
            block_size // 整倍数时填充一整个块
        } else {
            block_size - (plaintext_bytes.len() % block_size)
        };

        // 构建填充后的数据
        let mut padded_data = plaintext_bytes.to_vec();
        // `padding_len as u8` 显式转换，Rust 不允许隐式类型转换
        for _ in 0..padding_len {
            padded_data.push(padding_len as u8);
        }

        // 按 16 字节分块加密
        let mut encrypted_data = Vec::new();
        for chunk in padded_data.chunks(16) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            // `from_mut_slice` 创建可变的 GenericArray 引用
            // 加密结果直接写入 block
            cipher.encrypt_block(GenericArray::from_mut_slice(&mut block));
            encrypted_data.extend_from_slice(&block);
        }

        // 转为十六进制字符串输出
        Ok(hex::encode(encrypted_data))
    }

    /// 使用指定 cipher 解密文本
    fn decrypt_with_cipher(cipher: &Aes256, ciphertext: &str) -> Result<String> {
        // 十六进制字符串 → 字节数组
        let encrypted_data = hex::decode(ciphertext)?;

        // 密文长度必须是 16 的倍数
        if encrypted_data.len() % 16 != 0 {
            return Err(anyhow::anyhow!("密文长度无效"));
        }

        // 按 16 字节分块解密
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

        // 最后一个字节的值 = 填充长度
        let padding_len = decrypted_data[decrypted_data.len() - 1] as usize;

        // 验证填充长度的合理性（PKCS7 规定填充长度在 1-16 之间）
        if padding_len > 16 || padding_len == 0 {
            return Err(anyhow::anyhow!("填充长度无效"));
        }

        // 验证所有填充字节的值都等于 padding_len（数据完整性检查）
        let padding_start = decrypted_data.len() - padding_len;
        for byte in &decrypted_data[padding_start..] {
            if *byte != padding_len as u8 {
                return Err(anyhow::anyhow!("填充数据无效"));
            }
        }

        // 截去填充部分，恢复原始明文
        let plaintext_len = decrypted_data.len() - padding_len;
        decrypted_data.truncate(plaintext_len);

        // 将字节转为 UTF-8 字符串
        String::from_utf8(decrypted_data)
            .map_err(|e| anyhow::anyhow!("UTF-8 解码失败: {}", e))
    }

    /// 加密密码（公开接口）
    pub fn encrypt_password(&self, password: &str) -> Result<String> {
        Self::encrypt_with_cipher(&self.cipher, password)
    }

    /// 解密密码（公开接口）
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

        // 加密
        let encrypted = encryptor.encrypt_password("mypassword").unwrap();
        // 密文应该不同于原文
        assert_ne!(encrypted, "mypassword");

        // 解密
        let decrypted = encryptor.decrypt_password(&encrypted).unwrap();
        // 解密后应该恢复原文
        assert_eq!(decrypted, "mypassword");
    }
}
