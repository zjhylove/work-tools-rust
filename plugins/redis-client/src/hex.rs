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
