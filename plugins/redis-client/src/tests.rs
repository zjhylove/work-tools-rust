use super::*;
use serde_json::json;

const TEST_PREFIX: &str = "__wt_test__";

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// 从 PluginStorage 中读取已保存连接的密码（自动反混淆）
fn load_saved_password() -> String {
    let storage = PluginStorage::new("redis-client", "redis-client.json");
    let saved: Vec<connection::ConnectionConfig> = storage.load_json().unwrap_or_default();
    saved
        .first()
        .and_then(|c| {
            if c.password_obfuscated.is_empty() {
                None
            } else {
                crate::hex::deobfuscate(&c.password_obfuscated)
            }
        })
        .unwrap_or_default()
}

/// 构造连接参数
/// 优先级: 环境变量 REDIS_TEST_PASSWORD > 已保存连接中的密码 > 空
///
/// SSH 隧道模式: 设置 REDIS_USE_SSH=1 启用，通过插件内建 SSH 隧道连接 Redis。
/// 环境变量:
///   REDIS_SSH_HOST (默认 10.73.64.28)
///   REDIS_SSH_PORT (默认 10022)
///   REDIS_SSH_USER (默认 lbscheck)
///   REDIS_SSH_PASSWORD
fn test_connect_params() -> serde_json::Value {
    let password = std::env::var("REDIS_TEST_PASSWORD")
        .ok()
        .unwrap_or_else(load_saved_password);

    let use_ssh = std::env::var("REDIS_USE_SSH").map(|v| v == "1").unwrap_or(false);

    let remote_host = env_or("REDIS_TEST_HOST", "10.73.70.213");
    let remote_port = env_or("REDIS_TEST_PORT", "6379").parse::<u16>().unwrap_or(6379);

    if use_ssh {
        let ssh_password = env_or("REDIS_SSH_PASSWORD", "");
        json!({
            "host": remote_host,
            "port": remote_port,
            "db": env_or("REDIS_TEST_DB", "0").parse::<i64>().unwrap_or(0),
            "password": password,
            "ssh": {
                "host": env_or("REDIS_SSH_HOST", "10.73.64.28"),
                "port": env_or("REDIS_SSH_PORT", "10022").parse::<u16>().unwrap_or(10022),
                "username": env_or("REDIS_SSH_USER", "lbscheck"),
                "auth": {
                    "type": "password",
                    "password_obfuscated": crate::hex::obfuscate(&ssh_password),
                },
                "timeout_secs": 10,
            }
        })
    } else {
        json!({
            "host": env_or("REDIS_TEST_HOST", "127.0.0.1"),
            "port": remote_port,
            "db": env_or("REDIS_TEST_DB", "0").parse::<i64>().unwrap_or(0),
            "password": password,
        })
    }
}

fn call(plugin: &mut RedisClientPlugin, method: &str, params: Value) -> Value {
    plugin
        .handle_call(method, params)
        .unwrap_or_else(|e| panic!("{method} 失败: {e}"))
}

fn mkplugin() -> RedisClientPlugin {
    let mut p = RedisClientPlugin::new();
    call(&mut p, "connect", test_connect_params());
    // 清理残留测试数据
    let mut conn = p.get_conn().unwrap();
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg(format!("{{{TEST_PREFIX}}}:*"))
        .query(&mut conn)
        .unwrap_or_default();
    for k in &keys {
        let _: () = conn.del(k).unwrap();
    }
    p
}

/// 使用 hash tag 确保集群模式下所有测试 key 落在同一 slot
fn tk(name: &str) -> String {
    format!("{{{TEST_PREFIX}}}:{name}")
}

// ═══════════════════════════════════════════════════════
// 连接管理
// ═══════════════════════════════════════════════════════

#[test]
fn test_connect_and_disconnect() {
    let mut p = RedisClientPlugin::new();
    let params = test_connect_params();

    let r = call(&mut p, "connect", params.clone());
    assert!(r["ok"].as_bool().unwrap());

    let info = call(&mut p, "get_connection_info", json!({}));
    assert!(info["connected"].as_bool().unwrap());

    call(&mut p, "disconnect", json!({}));
    let info = call(&mut p, "get_connection_info", json!({}));
    assert!(!info["connected"].as_bool().unwrap());
}

#[test]
fn test_saved_connections_lifecycle() {
    let mut p = mkplugin();
    let params = test_connect_params();
    let host = params["host"].as_str().unwrap();

    let before = call(&mut p, "list_connections", json!({}));
    let before_count = before["connections"].as_array().unwrap().len();

    // 保存
    call(&mut p, "save_connection", json!({
        "name": "test-conn",
        "host": host,
        "port": params["port"],
        "db": params["db"],
        "password": "s3cret!"
    }));

    let list = call(&mut p, "list_connections", json!({}));
    let conns = list["connections"].as_array().unwrap();
    assert_eq!(conns.len(), before_count + 1);
    let saved = conns.iter().find(|c| c["name"] == "test-conn").unwrap();
    assert!(saved["has_password"].as_bool().unwrap());

    // 读回密码
    let pw = call(&mut p, "get_saved_password", json!({ "id": saved["id"] }));
    assert_eq!(pw["password"].as_str().unwrap(), "s3cret!");

    // 删除
    call(&mut p, "delete_connection", json!({ "id": saved["id"] }));
    let list = call(&mut p, "list_connections", json!({}));
    let conns = list["connections"].as_array().unwrap();
    assert!(!conns.iter().any(|c| c["name"] == "test-conn"));
}

// ═══════════════════════════════════════════════════════
// String
// ═══════════════════════════════════════════════════════

#[test]
fn test_string_ops() {
    let mut p = mkplugin();
    let k = tk("str");

    call(&mut p, "set_string", json!({ "key": k, "value": "hello world" }));

    let r = call(&mut p, "get_string", json!({ "key": k }));
    assert_eq!(r["value"].as_str().unwrap(), "hello world");

    // 覆盖
    call(&mut p, "set_string", json!({ "key": k, "value": "updated" }));
    let r = call(&mut p, "get_string", json!({ "key": k }));
    assert_eq!(r["value"].as_str().unwrap(), "updated");
}

// ═══════════════════════════════════════════════════════
// Hash
// ═══════════════════════════════════════════════════════

#[test]
fn test_hash_ops() {
    let mut p = mkplugin();
    let k = tk("hash");

    call(&mut p, "set_hash_field", json!({ "key": k, "field": "name", "value": "Alice" }));
    call(&mut p, "set_hash_field", json!({ "key": k, "field": "age", "value": "30" }));
    call(&mut p, "set_hash_field", json!({ "key": k, "field": "city", "value": "Shanghai" }));

    let r = call(&mut p, "get_hash", json!({ "key": k }));
    let fields = r["fields"].as_object().unwrap();
    assert_eq!(fields["name"], "Alice");
    assert_eq!(fields["age"], "30");
    assert_eq!(fields.len(), 3);

    call(&mut p, "del_hash_field", json!({ "key": k, "field": "age" }));
    let r = call(&mut p, "get_hash", json!({ "key": k }));
    let fields = r["fields"].as_object().unwrap();
    assert_eq!(fields.len(), 2);
    assert!(fields.contains_key("name"));
    assert!(!fields.contains_key("age"));
}

// ═══════════════════════════════════════════════════════
// List
// ═══════════════════════════════════════════════════════

#[test]
fn test_list_ops() {
    let mut p = mkplugin();
    let k = tk("list");

    let r = call(&mut p, "lpush", json!({ "key": k, "value": "b" }));
    assert_eq!(r["length"].as_i64().unwrap(), 1);
    call(&mut p, "lpush", json!({ "key": k, "value": "a" }));
    call(&mut p, "rpush", json!({ "key": k, "value": "c" }));

    // 应该: a, b, c
    let r = call(&mut p, "get_list", json!({ "key": k }));
    let items: Vec<&str> = r["items"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(items, vec!["a", "b", "c"]);

    // 范围查询
    let r = call(&mut p, "get_list", json!({ "key": k, "start": 0, "stop": 1 }));
    let items: Vec<&str> = r["items"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(items.len(), 2);

    // 按索引删除
    call(&mut p, "lrem", json!({ "key": k, "index": 1 }));
    let r = call(&mut p, "get_list", json!({ "key": k }));
    let items: Vec<&str> = r["items"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(items, vec!["a", "c"]);
}

// ═══════════════════════════════════════════════════════
// Set
// ═══════════════════════════════════════════════════════

#[test]
fn test_set_ops() {
    let mut p = mkplugin();
    let k = tk("set");

    call(&mut p, "sadd", json!({ "key": k, "member": "apple" }));
    call(&mut p, "sadd", json!({ "key": k, "member": "banana" }));
    let r = call(&mut p, "sadd", json!({ "key": k, "member": "apple" }));
    // 重复成员不增加
    assert_eq!(r["added"].as_i64().unwrap(), 0);

    let r = call(&mut p, "get_set", json!({ "key": k }));
    let members = r["members"].as_array().unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.iter().any(|m| m.as_str().unwrap() == "apple"));
    assert!(members.iter().any(|m| m.as_str().unwrap() == "banana"));

    call(&mut p, "srem", json!({ "key": k, "member": "banana" }));
    let r = call(&mut p, "get_set", json!({ "key": k }));
    assert_eq!(r["members"].as_array().unwrap().len(), 1);
}

// ═══════════════════════════════════════════════════════
// ZSet (Sorted Set)
// ═══════════════════════════════════════════════════════

#[test]
fn test_zset_ops() {
    let mut p = mkplugin();
    let k = tk("zset");

    call(&mut p, "zadd", json!({ "key": k, "score": 100.0, "member": "gold" }));
    call(&mut p, "zadd", json!({ "key": k, "score": 50.0, "member": "silver" }));
    call(&mut p, "zadd", json!({ "key": k, "score": 10.0, "member": "bronze" }));

    let r = call(&mut p, "get_zset", json!({ "key": k }));
    let members = r["members"].as_array().unwrap();
    assert_eq!(members.len(), 3);
    // ZRANGE 按分数升序
    assert_eq!(members[0]["member"], "bronze");
    assert!((members[0]["score"].as_f64().unwrap() - 10.0).abs() < 0.01);
    assert_eq!(members[1]["member"], "silver");
    assert_eq!(members[2]["member"], "gold");
    assert!((members[2]["score"].as_f64().unwrap() - 100.0).abs() < 0.01);

    // 删除
    call(&mut p, "zrem", json!({ "key": k, "member": "bronze" }));
    let r = call(&mut p, "get_zset", json!({ "key": k }));
    assert_eq!(r["members"].as_array().unwrap().len(), 2);
}

// ═══════════════════════════════════════════════════════
// Key 操作
// ═══════════════════════════════════════════════════════

#[test]
fn test_key_info() {
    let mut p = mkplugin();
    let k = tk("info");

    call(&mut p, "set_string", json!({ "key": k, "value": "test" }));
    let r = call(&mut p, "get_key_info", json!({ "key": k }));
    assert_eq!(r["type"].as_str().unwrap(), "string");
    assert_eq!(r["ttl"].as_i64().unwrap(), -1); // 无过期
}

#[test]
fn test_delete_key() {
    let mut p = mkplugin();
    let k = tk("del");

    call(&mut p, "set_string", json!({ "key": k, "value": "x" }));
    let r = call(&mut p, "delete_key", json!({ "key": k }));
    assert_eq!(r["deleted"].as_i64().unwrap(), 1);

    // 再次删除应返回 0
    let r = call(&mut p, "delete_key", json!({ "key": k }));
    assert_eq!(r["deleted"].as_i64().unwrap(), 0);
}

#[test]
fn test_delete_keys_batch() {
    let mut p = mkplugin();
    let k1 = tk("batch1");
    let k2 = tk("batch2");
    let k3 = tk("batch3");

    call(&mut p, "set_string", json!({ "key": k1, "value": "v1" }));
    call(&mut p, "set_string", json!({ "key": k2, "value": "v2" }));
    call(&mut p, "set_string", json!({ "key": k3, "value": "v3" }));

    let r = call(&mut p, "delete_keys", json!({ "keys": [k1, k2, k3] }));
    assert_eq!(r["deleted"].as_i64().unwrap(), 3);

    // Verify they're gone
    let r = call(&mut p, "get_key_info", json!({ "key": k1 }));
    assert_eq!(r["type"].as_str().unwrap(), "none");
}

#[test]
fn test_rename_key() {
    let mut p = mkplugin();
    let old = tk("old");
    let new = tk("new");

    call(&mut p, "set_string", json!({ "key": old, "value": "renamed" }));
    call(&mut p, "rename_key", json!({ "old": old, "new": new }));

    // 旧 key 不存在
    let r = call(&mut p, "get_key_info", json!({ "key": old }));
    assert_eq!(r["type"].as_str().unwrap(), "none");

    // 新 key 有值
    let r = call(&mut p, "get_string", json!({ "key": new }));
    assert_eq!(r["value"].as_str().unwrap(), "renamed");
}

#[test]
fn test_set_ttl_and_scan() {
    let mut p = mkplugin();
    let k = tk("ttl");

    call(&mut p, "set_string", json!({ "key": k, "value": "expiring" }));
    call(&mut p, "set_ttl", json!({ "key": k, "seconds": 3600 }));

    let r = call(&mut p, "get_key_info", json!({ "key": k }));
    assert_eq!(r["type"].as_str().unwrap(), "string");
    assert!(r["ttl"].as_i64().unwrap() > 0);

    // scan 返回结构验证 (集群模式下 SCAN 结果取决于 hash slot 分布)
    let r = call(&mut p, "scan_keys", json!({ "pattern": tk("*"), "count": 100 }));
    assert!(r["keys"].is_array());
    assert!(r["cursor"].is_number());
}

// ═══════════════════════════════════════════════════════
// Operations
// ═══════════════════════════════════════════════════════

#[test]
fn test_hex_dump() {
    let mut p = mkplugin();
    let k = tk("hex");

    call(&mut p, "set_string", json!({ "key": k, "value": "hello" }));
    let r = call(&mut p, "hex_dump", json!({ "key": k, "max_bytes": 10 }));
    assert_eq!(r["hex"].as_str().unwrap(), "68656c6c6f");
    assert_eq!(r["length"].as_i64().unwrap(), 5);
}

// ═══════════════════════════════════════════════════════
// 边界 / 错误
// ═══════════════════════════════════════════════════════

#[test]
fn test_missing_required_param() {
    let mut p = mkplugin();
    let err = p.handle_call("set_string", json!({ "key": "x" })).unwrap_err();
    assert!(err.to_string().contains("缺少"));
}

#[test]
fn test_unknown_method() {
    let mut p = mkplugin();
    let err = p.handle_call("no_such_method", json!({})).unwrap_err();
    assert!(err.to_string().contains("未知方法"));
}

#[test]
fn test_connect_not_connected() {
    let mut p = RedisClientPlugin::new();
    let err = p.handle_call("scan_keys", json!({})).unwrap_err();
    assert!(err.to_string().contains("未连接"));
}

#[test]
fn test_empty_string_value() {
    let mut p = mkplugin();
    let k = tk("empty");
    call(&mut p, "set_string", json!({ "key": k, "value": "" }));
    let r = call(&mut p, "get_string", json!({ "key": k }));
    assert_eq!(r["value"].as_str().unwrap(), "");
}

#[test]
fn test_hash_empty() {
    let mut p = mkplugin();
    let k = tk("emptyhash");
    // 空 hash (无字段时 HGETALL 行为)
    call(&mut p, "set_hash_field", json!({ "key": k, "field": "f", "value": "v" }));
    call(&mut p, "del_hash_field", json!({ "key": k, "field": "f" }));
    let r = call(&mut p, "get_hash", json!({ "key": k }));
    assert!(r["fields"].as_object().unwrap().is_empty());
}

#[test]
fn test_key_info_missing_key() {
    let mut p = mkplugin();
    let r = call(&mut p, "get_key_info", json!({ "key": tk("nonexistent") }));
    // 不存在的 key: TYPE 返回 "none"
    assert_eq!(r["type"].as_str().unwrap(), "none");
    assert_eq!(r["ttl"].as_i64().unwrap(), -2);
}

#[test]
fn test_scan_empty_pattern() {
    let mut p = mkplugin();
    let r = call(&mut p, "scan_keys", json!({ "pattern": "__wt_test__:nonexistent_xyz", "count": 10 }));
    let keys = r["keys"].as_array().unwrap();
    assert!(keys.is_empty());
}
