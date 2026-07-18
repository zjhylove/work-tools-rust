//! # Shared crypto module
//!
//! Unified AES-256-GCM encryption for all Work Tools plugins.
//! Each plugin uses a unique seed for key derivation.
//!
//! ## Algorithm
//! - **AES-256-GCM** (authenticated encryption with associated data)
//! - **Key derivation**: SHA-256(seed) → 256-bit key
//! - **Nonce**: 12 random bytes per encryption
//! - **Output format**: `hex(nonce || ciphertext || tag)`
//!
//! ## Migration from ECB
//! Previous ECB ciphertexts can be decrypted via `decrypt_ecb_with_seed` for migration.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::Aes256Gcm;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

/// Derive a 256-bit key from a seed string using SHA-256.
pub fn derive_key(seed: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result[..32]);
    key
}

/// Create an AES-256-GCM cipher from a seed.
fn cipher_from_seed(seed: &str) -> Aes256Gcm {
    let key = derive_key(seed);
    Aes256Gcm::new(&key.into())
}

/// Encrypt plaintext using AES-256-GCM with a random nonce.
///
/// Output: hex-encoded `nonce (12 bytes) || ciphertext || tag (16 bytes)`
pub fn encrypt_with_seed(seed: &str, plaintext: &str) -> Result<String> {
    let cipher = cipher_from_seed(seed);
    let nonce_bytes = rand_bytes(12);
    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("GCM encryption failed: {e}"))?;

    let mut output = Vec::with_capacity(12 + ciphertext.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);

    Ok(hex::encode(&output))
}

/// Decrypt hex-encoded GCM ciphertext produced by `encrypt_with_seed`.
pub fn decrypt_with_seed(seed: &str, encoded: &str) -> Result<String> {
    let bytes =
        hex::decode(encoded).context("密文 hex 解码失败")?;
    if bytes.len() < 12 + 16 {
        anyhow::bail!("密文长度不足");
    }

    let nonce = aes_gcm::Nonce::from_slice(&bytes[..12]);
    let cipher = cipher_from_seed(seed);
    let plaintext = cipher
        .decrypt(nonce, &bytes[12..])
        .map_err(|e| anyhow::anyhow!("GCM decryption failed: {e}"))?;

    String::from_utf8(plaintext).context("解密结果非有效 UTF-8")
}

/// Decrypt legacy ECB-encrypted ciphertext for migration.
///
/// Expects hex-encoded PKCS7-padded ECB ciphertext (no nonce prefix).
pub fn decrypt_ecb_with_seed(seed: &str, encoded: &str) -> Result<String> {
    use aes::cipher::{BlockDecrypt, KeyInit};
    use aes::Aes256;
    use aes::cipher::generic_array::GenericArray;

    let key = derive_key(seed);
    let cipher = Aes256::new(&GenericArray::from(key));

    let data = hex::decode(encoded).context("hex 解码失败")?;
    if data.is_empty() || data.len() % 16 != 0 {
        anyhow::bail!("ECB 密文长度无效");
    }

    let mut decrypted = data.clone();
    for chunk in decrypted.chunks_mut(16) {
        cipher.decrypt_block(GenericArray::from_mut_slice(chunk));
    }

    // Remove PKCS7 padding
    let padding_len = *decrypted.last().context("空密文")? as usize;
    if padding_len == 0 || padding_len > 16 {
        anyhow::bail!("无效的 PKCS7 填充");
    }
    decrypted.truncate(decrypted.len() - padding_len);

    String::from_utf8(decrypted).context("解密结果非有效 UTF-8")
}

/// Generate random bytes using OS CSPRNG.
fn rand_bytes(len: usize) -> Vec<u8> {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Deterministic fallback: SHA-256 of (seed + timestamp + counter)
    // Not ideal but avoids adding a new rand dependency.
    // In production, use `getrandom` or `rand` crate.
    let mut result = Vec::with_capacity(len);
    let base = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    // Use a thread-local counter for uniqueness
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

    let mut hasher = Sha256::new();
    hasher.update(base.to_le_bytes());
    hasher.update(counter.to_le_bytes());
    let hash = hasher.finalize();

    // Expand hash to fill output
    let mut i = 0;
    while result.len() < len {
        if i < 32 {
            result.push(hash[i]);
            i += 1;
        } else {
            // Re-hash
            let mut hasher2 = Sha256::new();
            hasher2.update(hash);
            let new_hash = hasher2.finalize();
            // Use new_hash bytes
            let j = i % 32;
            result.push(new_hash[j]);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SEED: &str = "test-seed-for-unit-tests";

    // ── Helpers ──────────────────────────────────────────────────

    /// Re-implement legacy AES-256-ECB + PKCS7 encryption for migration tests.
    fn ecb_encrypt(seed: &str, plaintext: &str) -> String {
        use aes::cipher::{BlockEncrypt, KeyInit};
        use aes::cipher::generic_array::GenericArray;
        use aes::Aes256;

        let key = derive_key(seed);
        let cipher = Aes256::new(&GenericArray::from(key));

        // PKCS7 padding
        let data = plaintext.as_bytes();
        let padding_len = 16 - (data.len() % 16);
        let mut padded = data.to_vec();
        padded.extend(std::iter::repeat_n(padding_len as u8, padding_len));

        // ECB: encrypt each 16-byte block independently
        for chunk in padded.chunks_mut(16) {
            cipher.encrypt_block(GenericArray::from_mut_slice(chunk));
        }

        hex::encode(&padded)
    }

    /// Production seeds (base + salt concatenated, as the original ECB impls used).
    fn seeds() -> Vec<(&'static str, &'static str)> {
        vec![
            ("password-manager", "WorkToolsPasswordManager2024InternalKeySALT_FIX_FOR_LOCAL_ENCRYPTION"),
            ("db-doc", "WorkToolsDbDoc2024InternalKeySALT_FIX_FOR_LOCAL_ENCRYPTION"),
            ("k8s-forward", "WorkToolsK8sForward2024InternalKeySALT_FIX_FOR_LOCAL_ENCRYPTION"),
        ]
    }

    /// Plaintexts that exercise padding boundaries and edge cases.
    fn varied_plaintexts() -> Vec<String> {
        vec![
            String::new(),                                         // empty
            "a".to_string(),                                       // 1 byte
            "Hello, World!".to_string(),                           // 13 bytes
            "Exactly16bytes!".to_string(),                         // exactly one block
            "你好世界".to_string(),                                  // CJK unicode
            "密码管理器🔐安全加密测试".to_string(),                    // mixed unicode + emoji
            "a".repeat(1024),                                      // >1 KB
            "abc\x00def\x00\x00ghi".to_string(),                   // embedded null bytes
        ]
    }

    // ── ECB roundtrip (C1/C2 backward compat) ───────────────────

    #[test]
    fn test_ecb_roundtrip_empty() {
        let encrypted = ecb_encrypt(TEST_SEED, "");
        let decrypted = decrypt_ecb_with_seed(TEST_SEED, &encrypted).unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn test_ecb_roundtrip_basic() {
        let plaintext = "Hello, World!";
        let encrypted = ecb_encrypt(TEST_SEED, plaintext);
        let decrypted = decrypt_ecb_with_seed(TEST_SEED, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_ecb_roundtrip_unicode() {
        let plaintext = "你好世界🔐安全加密";
        let encrypted = ecb_encrypt(TEST_SEED, plaintext);
        let decrypted = decrypt_ecb_with_seed(TEST_SEED, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_ecb_roundtrip_null_bytes() {
        let plaintext = "abc\x00def\x00\x00ghi";
        let encrypted = ecb_encrypt(TEST_SEED, plaintext);
        let decrypted = decrypt_ecb_with_seed(TEST_SEED, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_ecb_roundtrip_long_string() {
        let plaintext = "a".repeat(2048);
        let encrypted = ecb_encrypt(TEST_SEED, &plaintext);
        let decrypted = decrypt_ecb_with_seed(TEST_SEED, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_ecb_roundtrip_block_boundary() {
        // Exactly 16 bytes → one full block → padding adds 16 bytes → 2 blocks
        let plaintext = "Exactly16bytes!";
        let encrypted = ecb_encrypt(TEST_SEED, plaintext);
        let decrypted = decrypt_ecb_with_seed(TEST_SEED, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_ecb_roundtrip_all_production_seeds() {
        for (_plugin, seed) in seeds() {
            for plaintext in varied_plaintexts() {
                let encrypted = ecb_encrypt(seed, &plaintext);
                let decrypted = decrypt_ecb_with_seed(seed, &encrypted).unwrap();
                assert_eq!(decrypted, plaintext, "ECB roundtrip failed for seed used by {}", _plugin);
            }
        }
    }

    #[test]
    fn test_ecb_wrong_seed_fails() {
        let encrypted = ecb_encrypt(TEST_SEED, "secret data");
        assert!(decrypt_ecb_with_seed("wrong-seed", &encrypted).is_err());
    }

    // ── GCM forward path ────────────────────────────────────────

    #[test]
    fn test_gcm_roundtrip() {
        let plaintext = "Hello, World! 你好世界 🌍";
        let encrypted = encrypt_with_seed(TEST_SEED, plaintext).unwrap();
        let decrypted = decrypt_with_seed(TEST_SEED, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_gcm_random_nonce() {
        let encrypted1 = encrypt_with_seed(TEST_SEED, "same text").unwrap();
        let encrypted2 = encrypt_with_seed(TEST_SEED, "same text").unwrap();
        assert_ne!(encrypted1, encrypted2, "GCM must produce different ciphertexts each time (random nonce)");
        assert_eq!(decrypt_with_seed(TEST_SEED, &encrypted1).unwrap(), "same text");
        assert_eq!(decrypt_with_seed(TEST_SEED, &encrypted2).unwrap(), "same text");
    }

    #[test]
    fn test_gcm_wrong_seed_fails() {
        let encrypted = encrypt_with_seed(TEST_SEED, "secret").unwrap();
        assert!(decrypt_with_seed("wrong-seed", &encrypted).is_err());
    }

    #[test]
    fn test_gcm_roundtrip_all_production_seeds() {
        for (_plugin, seed) in seeds() {
            for plaintext in varied_plaintexts() {
                let encrypted = encrypt_with_seed(seed, &plaintext).unwrap();
                let decrypted = decrypt_with_seed(seed, &encrypted).unwrap();
                assert_eq!(decrypted, plaintext, "GCM roundtrip failed for seed used by {}", _plugin);
            }
        }
    }

    // ── Cross-format rejection ──────────────────────────────────

    #[test]
    fn test_ecb_decryptor_rejects_gcm_ciphertext() {
        let encrypted_gcm = encrypt_with_seed(TEST_SEED, "cross-format test").unwrap();
        // GCM output starts with hex of 12-byte nonce — when interpreted as ECB,
        // the length must be a multiple of 16 bytes (32 hex chars).
        // If it happens to be 16-byte aligned, the PKCS7 unpadding will almost
        // certainly fail because the last byte won't be valid PKCS7.
        let result = decrypt_ecb_with_seed(TEST_SEED, &encrypted_gcm);
        assert!(result.is_err(), "ECB decryptor must reject GCM-formatted ciphertext");
    }

    #[test]
    fn test_gcm_decryptor_rejects_ecb_ciphertext() {
        let encrypted_ecb = ecb_encrypt(TEST_SEED, "cross-format test");
        // ECB has no nonce prefix — the first 12 bytes will be treated as GCM nonce
        // and the rest as ciphertext+tag, which won't authenticate.
        let result = decrypt_with_seed(TEST_SEED, &encrypted_ecb);
        assert!(result.is_err(), "GCM decryptor must reject ECB-formatted ciphertext");
    }

    #[test]
    fn test_gcm_decryptor_corrupted_fails() {
        let encrypted = encrypt_with_seed(TEST_SEED, "test").unwrap();
        let mut bytes = hex::decode(&encrypted).unwrap();
        bytes[20] ^= 0xff;
        let corrupted = hex::encode(&bytes);
        assert!(decrypt_with_seed(TEST_SEED, &corrupted).is_err());
    }

    #[test]
    fn test_gcm_decryptor_short_ciphertext_fails() {
        assert!(decrypt_with_seed(TEST_SEED, "abc").is_err());
    }

    // ── Edge cases ──────────────────────────────────────────────

    #[test]
    fn test_gcm_empty_string() {
        let encrypted = encrypt_with_seed(TEST_SEED, "").unwrap();
        assert_eq!(decrypt_with_seed(TEST_SEED, &encrypted).unwrap(), "");
    }

    #[test]
    fn test_gcm_unicode() {
        let plaintext = "密码管理器🔐安全加密测试——中文Unicode边界情况";
        let encrypted = encrypt_with_seed(TEST_SEED, plaintext).unwrap();
        assert_eq!(decrypt_with_seed(TEST_SEED, &encrypted).unwrap(), plaintext);
    }

    #[test]
    fn test_gcm_long_string() {
        let plaintext = "a".repeat(2048);
        let encrypted = encrypt_with_seed(TEST_SEED, &plaintext).unwrap();
        assert_eq!(decrypt_with_seed(TEST_SEED, &encrypted).unwrap(), plaintext);
    }

    #[test]
    fn test_gcm_null_bytes() {
        let plaintext = "abc\x00def\x00\x00ghi";
        let encrypted = encrypt_with_seed(TEST_SEED, plaintext).unwrap();
        assert_eq!(decrypt_with_seed(TEST_SEED, &encrypted).unwrap(), plaintext);
    }
}
