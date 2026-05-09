use redis::{Commands, Connection};
use serde_json::Value;
use std::collections::HashMap;

/// SCAN keys and fetch TYPE + TTL for each
pub fn scan_key_infos(
    keys: &[String],
    conn: &mut Connection,
) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
    let mut result = Vec::with_capacity(keys.len());
    for k in keys {
        let key_type: String = redis::cmd("TYPE")
            .arg(k)
            .query(conn)
            .unwrap_or_else(|_| "unknown".into());
        let ttl: i64 = redis::cmd("TTL").arg(k).query(conn).unwrap_or(-2);
        result.push(serde_json::json!({ "key": k, "type": key_type, "ttl": ttl }));
    }
    Ok(result)
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
