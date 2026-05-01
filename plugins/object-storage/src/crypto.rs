//! # 对象存储插件的密钥加密
//!
//! 使用 XOR + Base64 方案保护云服务商的 Access Key / Secret Key。
//!
//! ## 与 AES 加密的区别
//! 对象存储插件使用更简单的 XOR 加密（而非 AES），原因是：
//! 1. XOR 实现简洁（无需依赖 aes/sha2 crate）
//! 2. 对于密钥这种短文本（< 100 字符），XOR 提供足够的混淆
//! 3. Base64 编码确保密文是可打印的 ASCII 文本
//!
//! ## 安全性说明
//! XOR 加密不是密码学安全的（已知明文攻击可恢复密钥）。
//! 对于生产环境，建议使用 AES-256-GCM 或依赖操作系统密钥库。
//!
//! ## Rust 知识点
//! - `^`: 按位异或运算符
//! - `bytes()`: 将字符串转为字节迭代器
//! - `enumerate()`: 迭代时获取索引
//! - `%`: 取模运算（使密钥循环使用）
//! - `base64` crate: Base64 编解码

use base64::Engine;

/// XOR 密钥（循环使用）
/// `b"..."` 是字节串字面量，类型为 `&[u8; N]`
const XOR_KEY: &[u8] = b"wt-obj-storage-2024-secure-key-v1";

/// 加密：纯文本 → XOR + Base64
///
/// ## 算法步骤
/// 1. 对每个字节循环异或 XOR_KEY 的对应字节
/// 2. 将结果用 Base64 编码
///
/// ## Rust 知识点: enumerate 和索引
/// `for (i, byte) in plain.bytes().enumerate()`:
/// - `bytes()` 返回迭代器，每次产生一个 `u8`
/// - `enumerate()` 包装迭代器，额外提供索引（从 0 开始）
/// - `i % XOR_KEY.len()` 使密钥循环使用
pub fn encrypt(plain: &str) -> String {
    let mut result = Vec::with_capacity(plain.len()); // 预分配内存
    for (i, byte) in plain.bytes().enumerate() {
        result.push(byte ^ XOR_KEY[i % XOR_KEY.len()]);
    }
    // Base64 编码（STANDARD = 标准 RFC 4648 Base64）
    base64::engine::general_purpose::STANDARD.encode(&result)
}

/// 解密：Base64 + XOR → 纯文本
///
/// 解密失败时返回原始字符串（优雅降级）。
///
/// ## Rust 知识点: String::from_utf8_lossy
/// 将字节数组转为字符串，遇到无效 UTF-8 时使用 � (U+FFFD) 替换。
/// 这不是严格正确的（应该返回错误），但在解密场景下简化了错误处理。
pub fn decrypt(encoded: &str) -> String {
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded) {
        let mut result = Vec::with_capacity(bytes.len());
        for (i, byte) in bytes.iter().enumerate() {
            result.push(byte ^ XOR_KEY[i % XOR_KEY.len()]);
        }
        String::from_utf8_lossy(&result).to_string()
    } else {
        // Base64 解码失败说明可能不是加密的值，直接返回原文
        encoded.to_string()
    }
}
