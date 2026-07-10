//! # K8s Forward encryption
//!
//! Thin wrapper around `worktools_plugin_api::crypto`.
//! Uses AES-256-GCM for new encryptions, supports legacy ECB migration.

pub const SEED: &str = "WorkToolsK8sForward2024InternalKey!K8S_FORWARD_SALT_FIXED";

/// Encrypt a password using AES-256-GCM.
pub fn encrypt(password: &str) -> anyhow::Result<String> {
    worktools_plugin_api::crypto::encrypt_with_seed(SEED, password)
}

/// Decrypt a password. Tries GCM first, falls back to legacy ECB.
pub fn decrypt(encrypted: &str) -> anyhow::Result<String> {
    match worktools_plugin_api::crypto::decrypt_with_seed(SEED, encrypted) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => {
            tracing::warn!("使用遗留 ECB 解密，建议重新保存以升级到 GCM");
            worktools_plugin_api::crypto::decrypt_ecb_with_seed(SEED, encrypted)
        }
    }
}
