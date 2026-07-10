//! # Object Storage encryption
//!
//! Replaced XOR + Base64 pseudo-encryption with AES-256-GCM via shared crypto module.
//! Supports legacy XOR/Base64 migration.

pub const SEED: &str = "WorkToolsObjectStorage2024SecureKeyV1";

/// Encrypt credentials using AES-256-GCM.
pub fn encrypt(plain: &str) -> String {
    worktools_plugin_api::crypto::encrypt_with_seed(SEED, plain)
        .unwrap_or_else(|_| plain.to_string())
}

/// Decrypt credentials. Tries GCM first, falls back to legacy XOR.
pub fn decrypt(encoded: &str) -> String {
    // Try GCM first
    if let Ok(plaintext) = worktools_plugin_api::crypto::decrypt_with_seed(SEED, encoded) {
        return plaintext;
    }

    // Fall back to legacy XOR + Base64
    use base64::Engine;
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded) {
        const XOR_KEY: &[u8] = b"wt-obj-storage-2024-secure-key-v1";
        let result: Vec<u8> = bytes
            .iter()
            .enumerate()
            .map(|(i, byte)| byte ^ XOR_KEY[i % XOR_KEY.len()])
            .collect();
        String::from_utf8_lossy(&result).to_string()
    } else {
        encoded.to_string()
    }
}
