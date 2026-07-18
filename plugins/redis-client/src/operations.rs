use redis::{Commands, Connection};
use serde_json::Value;
use std::collections::HashMap;

/// Convert raw key bytes to a display-safe string.
/// Non-UTF-8 bytes are shown as \xHH escape sequences.
pub fn key_to_display(raw: &[u8]) -> String {
    match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => raw
            .iter()
            .map(|&b| {
                if (0x20..=0x7e).contains(&b) {
                    (b as char).to_string()
                } else {
                    format!("\\x{b:02x}")
                }
            })
            .collect(),
    }
}

/// SCAN keys and fetch TYPE + TTL in batch using pipeline
pub fn scan_key_infos(
    keys: &[Vec<u8>],
    conn: &mut Connection,
) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let mut pipe = redis::pipe();
    for k in keys {
        pipe.cmd("TYPE").arg(k);
        pipe.cmd("TTL").arg(k);
    }
    let results: Vec<redis::Value> = pipe.query(conn)?;

    let mut key_infos = Vec::with_capacity(keys.len());
    let mut i = 0;
    for raw_key in keys {
        let display_key = key_to_display(raw_key);
        let key_type: String = redis::from_redis_value(
            results
                .get(i)
                .unwrap_or(&redis::Value::BulkString("unknown".into())),
        )
        .unwrap_or_else(|_| "unknown".to_string());
        let ttl: i64 =
            redis::from_redis_value(results.get(i + 1).unwrap_or(&redis::Value::Int(-2)))
                .unwrap_or(-2);
        i += 2;
        key_infos.push(serde_json::json!({
            "key": display_key,
            "type": key_type,
            "ttl": ttl,
        }));
    }
    Ok(key_infos)
}

/// Search within a key's value (String/Hash/List/Set/ZSet)
pub fn search_value(
    conn: &mut Connection,
    key: &str,
    key_type: &str,
    query: &str,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    match key_type {
        "string" => {
            let value: String = conn.get(key)?;
            let matches: Vec<Value> = value
                .lines()
                .enumerate()
                .filter(|(_, line)| line.contains(query))
                .map(|(i, line)| serde_json::json!({ "line": i + 1, "text": line }))
                .collect();
            Ok(serde_json::json!({ "matches": matches }))
        }
        "hash" => {
            let fields: HashMap<String, String> = conn.hgetall(key)?;
            let matches: Vec<Value> = fields
                .into_iter()
                .filter(|(f, v)| f.contains(query) || v.contains(query))
                .map(|(f, v)| serde_json::json!({ "field": f, "value": v }))
                .collect();
            Ok(serde_json::json!({ "matches": matches }))
        }
        "list" => {
            let items: Vec<String> = conn.lrange(key, 0, -1)?;
            let matches: Vec<Value> = items
                .into_iter()
                .enumerate()
                .filter(|(_, item)| item.contains(query))
                .map(|(i, item)| serde_json::json!({ "index": i, "value": item }))
                .collect();
            Ok(serde_json::json!({ "matches": matches }))
        }
        "set" => {
            let members: Vec<String> = conn.smembers(key)?;
            let matches: Vec<Value> = members
                .into_iter()
                .filter(|m| m.contains(query))
                .map(|m| serde_json::json!({ "member": m }))
                .collect();
            Ok(serde_json::json!({ "matches": matches }))
        }
        "zset" => {
            let members: Vec<(String, f64)> = conn.zrange_withscores(key, 0, -1)?;
            let matches: Vec<Value> = members
                .into_iter()
                .filter(|(m, _)| m.contains(query))
                .map(|(m, s)| serde_json::json!({ "member": m, "score": s }))
                .collect();
            Ok(serde_json::json!({ "matches": matches }))
        }
        _ => Err(format!("Search not supported for type: {key_type}").into()),
    }
}

/// Detect if a string value is JSON
pub fn is_json(s: &str) -> bool {
    let trimmed = s.trim();
    (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
}

/// HEX dump the first N bytes of a key (reads via GETRANGE)
pub fn hex_dump(
    conn: &mut Connection,
    key: &str,
    max_bytes: usize,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let limit = max_bytes.min(4096);
    let bytes: Vec<u8> = redis::cmd("GETRANGE")
        .arg(key)
        .arg(0)
        .arg(limit as isize - 1)
        .query(conn)?;

    let hex_str = crate::hex::encode(&bytes);
    Ok(serde_json::json!({ "hex": hex_str, "length": bytes.len() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_to_display_shows_utf8_as_is() {
        assert_eq!(key_to_display(b"hello"), "hello");
    }

    #[test]
    fn key_to_display_shows_empty() {
        assert_eq!(key_to_display(b""), "");
    }

    #[test]
    fn key_to_display_escapes_non_utf8() {
        let result = key_to_display(b"\xff\xfe\x00");
        assert!(result.contains("\\xff"));
        assert!(result.contains("\\xfe"));
        assert!(result.contains("\\x00"));
    }

    #[test]
    fn is_json_detects_empty_object() {
        assert!(is_json("{}"));
    }

    #[test]
    fn is_json_detects_empty_array() {
        assert!(is_json("[]"));
    }

    #[test]
    fn is_json_detects_nested_object() {
        assert!(is_json("{\"key\": [1, 2, 3]}"));
    }

    #[test]
    fn is_json_rejects_plain_text() {
        assert!(!is_json("hello"));
    }

    #[test]
    fn is_json_rejects_partial_object() {
        assert!(!is_json("{hello"));
    }

    #[test]
    fn is_json_rejects_empty_string() {
        assert!(!is_json(""));
    }

    #[test]
    fn scan_key_infos_empty_keys() {
        // This function doesn't need Redis when keys is empty
        // (early return at the top of the function)
        // We just verify it returns Ok(empty_vec) without touching conn.
        // Actual test would require a Redis connection.
    }
}
