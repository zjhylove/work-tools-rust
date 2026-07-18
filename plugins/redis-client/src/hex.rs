pub fn encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
    if !s.len().is_multiple_of(2) {
        return Err(());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

pub const XOR_KEY: &[u8] = b"worktools-redis-2026";

pub fn obfuscate(s: &str) -> String {
    let bytes: Vec<u8> = s
        .bytes()
        .zip(XOR_KEY.iter().cycle())
        .map(|(a, b)| a ^ b)
        .collect();
    encode(&bytes)
}

pub fn deobfuscate(s: &str) -> Option<String> {
    let bytes = decode(s).ok()?;
    let decoded: Vec<u8> = bytes
        .iter()
        .zip(XOR_KEY.iter().cycle())
        .map(|(a, b)| a ^ b)
        .collect();
    String::from_utf8(decoded).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_empty() {
        assert_eq!(encode(b""), "");
    }

    #[test]
    fn encode_hello() {
        assert_eq!(encode(b"hello"), "68656c6c6f");
    }

    #[test]
    fn decode_empty() {
        assert_eq!(decode(""), Ok(vec![]));
    }

    #[test]
    fn decode_hello() {
        assert_eq!(decode("68656c6c6f"), Ok(b"hello".to_vec()));
    }

    #[test]
    fn decode_invalid_length() {
        assert_eq!(decode("abc"), Err(()));
    }

    #[test]
    fn decode_invalid_hex() {
        assert_eq!(decode("zz"), Err(()));
    }

    #[test]
    fn obfuscate_deobfuscate_roundtrip() {
        let original = "password123!@#";
        let obs = obfuscate(original);
        assert_ne!(obs, original);
        assert_eq!(deobfuscate(&obs), Some(original.to_string()));
    }

    #[test]
    fn obfuscate_empty() {
        assert_eq!(obfuscate(""), "");
    }

    #[test]
    fn deobfuscate_invalid_hex() {
        assert_eq!(deobfuscate("gg"), None);
    }
}
