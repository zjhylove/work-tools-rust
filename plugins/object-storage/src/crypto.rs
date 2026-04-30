use base64::Engine;

const XOR_KEY: &[u8] = b"wt-obj-storage-2024-secure-key-v1";

pub fn encrypt(plain: &str) -> String {
    let mut result = Vec::with_capacity(plain.len());
    for (i, byte) in plain.bytes().enumerate() {
        result.push(byte ^ XOR_KEY[i % XOR_KEY.len()]);
    }
    base64::engine::general_purpose::STANDARD.encode(&result)
}

pub fn decrypt(encoded: &str) -> String {
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded) {
        let mut result = Vec::with_capacity(bytes.len());
        for (i, byte) in bytes.iter().enumerate() {
            result.push(byte ^ XOR_KEY[i % XOR_KEY.len()]);
        }
        String::from_utf8_lossy(&result).to_string()
    } else {
        encoded.to_string()
    }
}
