//! # Password Manager encryption
//!
//! Thin wrapper around `worktools_plugin_api::crypto`.
//! Uses AES-256-GCM for new encryptions, supports legacy ECB migration.

pub const SEED: &str = "WorkToolsPasswordManager2024InternalKeySALT_FIX_FOR_LOCAL_ENCRYPTION";

/// Encrypt a password using AES-256-GCM.
pub fn encrypt_password(password: &str) -> anyhow::Result<String> {
    worktools_plugin_api::crypto::encrypt_with_seed(SEED, password)
}

/// Decrypt a password. Tries GCM first, falls back to legacy ECB for migration.
pub fn decrypt_password(encrypted: &str) -> anyhow::Result<String> {
    // Try GCM first (modern format: nonce prefix + ciphertext + tag)
    match worktools_plugin_api::crypto::decrypt_with_seed(SEED, encrypted) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => {
            // Fall back to legacy ECB (no nonce prefix, just PKCS7-padded blocks)
            tracing::warn!("使用遗留 ECB 解密，建议重新保存以升级到 GCM");
            worktools_plugin_api::crypto::decrypt_ecb_with_seed(SEED, encrypted)
        }
    }
}
