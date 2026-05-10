use anyhow::Context;
use redis::{Client, Commands, Connection};
use serde_json::Value;
use std::collections::HashMap;
use worktools_plugin_api::storage::PluginStorage;
use worktools_plugin_api::Plugin;

pub mod connection;
pub(crate) mod hex;
pub(crate) mod operations;
pub(crate) mod ssh_tunnel;

pub struct RedisClientPlugin {
    client: Option<Client>,
    current_config: Option<connection::ConnectionConfig>,
    storage: PluginStorage,
    saved_connections: Vec<connection::ConnectionConfig>,
    connections_loaded: bool,
    ssh_tunnel: Option<ssh_tunnel::SshTunnel>,
}

impl RedisClientPlugin {
    fn new() -> Self {
        Self {
            client: None,
            current_config: None,
            storage: PluginStorage::new("redis-client", "redis-client.json"),
            saved_connections: Vec::new(),
            connections_loaded: false,
            ssh_tunnel: None,
        }
    }

    fn ensure_connections_loaded(&mut self) {
        if self.connections_loaded {
            return;
        }
        self.saved_connections = self
            .storage
            .load_json::<Vec<connection::ConnectionConfig>>()
            .unwrap_or_default();
        self.connections_loaded = true;
    }

    fn get_conn(&self) -> Result<Connection, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .as_ref()
            .ok_or("未连接到 Redis")?
            .get_connection()
            .map_err(|e| e.into())
    }

    fn resolve_password(params: &Value, obfuscated: &str) -> Option<String> {
        params
            .get("password")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .or_else(|| {
                if obfuscated.is_empty() {
                    None
                } else {
                    hex::deobfuscate(obfuscated)
                }
            })
    }

    fn connect_client(
        conn_cfg: &connection::ConnectionConfig,
        password: Option<String>,
        ssh_tunnel: &mut Option<ssh_tunnel::SshTunnel>,
    ) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
        let mode = conn_cfg.to_connection_mode(password);
        match mode {
            connection::ConnectionMode::Direct {
                host,
                port,
                db,
                password,
            } => {
                let url = Self::make_redis_url(&host, port, db, password.as_deref());
                Client::open(url.as_str()).map_err(|e| e.into())
            }
            connection::ConnectionMode::SshTunnel {
                ssh,
                remote_host,
                remote_port,
                db,
                password,
            } => {
                let tunnel = ssh_tunnel::SshTunnel::connect(&ssh, &remote_host, remote_port)
                    .map_err(|e| format!("SSH tunnel failed: {e}"))?;
                let local_port = tunnel.local_port();
                let url =
                    Self::make_redis_url("127.0.0.1", local_port, db, password.as_deref());
                let client = Client::open(url.as_str()).map_err(|e| e.into());
                *ssh_tunnel = Some(tunnel);
                client
            }
            connection::ConnectionMode::Cluster { seed_nodes, password } => {
                let node_urls: Vec<String> = seed_nodes
                    .iter()
                    .map(|n| match &password {
                        Some(p) if !p.is_empty() => format!("redis://:{p}@{n}"),
                        _ => format!("redis://{n}"),
                    })
                    .collect();
                Client::open(node_urls[0].as_str()).map_err(|e| e.into())
            }
        }
    }

    fn make_redis_url(host: &str, port: u16, db: i64, password: Option<&str>) -> String {
        match password {
            Some(p) if !p.is_empty() => format!("redis://:{}@{}:{}/{}", p, host, port, db),
            _ => format!("redis://{}:{}/{}", host, port, db),
        }
    }
}

impl Plugin for RedisClientPlugin {
    fn id(&self) -> &str {
        "redis-client"
    }
    fn name(&self) -> &str {
        "Redis 客户端"
    }
    fn description(&self) -> &str {
        "Redis数据库管理工具"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn icon(&self) -> &str {
        "🔴"
    }
    fn get_view(&self) -> String {
        "<div>插件资源加载中...</div>".to_string()
    }

    fn destroy(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client = None;
        self.current_config = None;
        self.ssh_tunnel = None;
        tracing::info!("Redis 客户端已销毁");
        Ok(())
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            // ── 连接管理 ──
            "connect" => {
                self.client = None;
                self.ssh_tunnel = None;
                self.current_config = None;

                let (conn_cfg, password) =
                    if let Some(id) = params.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                        self.ensure_connections_loaded();
                        let cfg = self
                            .saved_connections
                            .iter()
                            .find(|c| c.id == id)
                            .ok_or("连接配置不存在")?
                            .clone();
                        let pw = Self::resolve_password(&params, &cfg.password_obfuscated);
                        (cfg, pw)
                    } else {
                        let host = params
                            .get("host")
                            .and_then(|v| v.as_str())
                            .unwrap_or("127.0.0.1");
                        let port =
                            params.get("port").and_then(|v| v.as_u64()).unwrap_or(6379) as u16;
                        let db = params.get("db").and_then(|v| v.as_i64()).unwrap_or(0);
                        let cfg = connection::ConnectionConfig {
                            id: String::new(),
                            name: format!("{host}:{port}"),
                            color: None,
                            host: host.to_string(),
                            port,
                            db,
                            password_obfuscated: String::new(),
                            ssh: parse_optional_struct(&params, "ssh"),
                            cluster: parse_optional_struct(&params, "cluster"),
                        };
                        (cfg, opt_str(&params, "password"))
                    };

                let client =
                    Self::connect_client(&conn_cfg, password, &mut self.ssh_tunnel)?;
                let _: String =
                    redis::cmd("PING").query(&mut client.get_connection().context("Redis 连接失败")?)?;

                self.client = Some(client);
                self.current_config = Some(conn_cfg.clone());

                tracing::info!(
                    host = %conn_cfg.host,
                    port = conn_cfg.port,
                    db = conn_cfg.db,
                    "Redis 连接成功"
                );
                Ok(serde_json::json!({ "ok": true, "host": conn_cfg.host, "port": conn_cfg.port, "db": conn_cfg.db }))
            }

            "disconnect" => {
                self.client = None;
                self.current_config = None;
                self.ssh_tunnel = None;
                Ok(serde_json::json!({ "ok": true }))
            }

            "test_connection" => {
                let cfg = if let Some(id) = opt_str(&params, "id") {
                    self.ensure_connections_loaded();
                    let cfg = self
                        .saved_connections
                        .iter()
                        .find(|c| c.id == id)
                        .ok_or("连接配置不存在")?;
                    let password = Self::resolve_password(&params, &cfg.password_obfuscated);
                    let mut temp_cfg = cfg.clone();
                    temp_cfg.id = id;
                    (temp_cfg, password)
                } else {
                    let host = params.get("host").and_then(|v| v.as_str()).unwrap_or("127.0.0.1").to_string();
                    let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(6379) as u16;
                    let db = params.get("db").and_then(|v| v.as_i64()).unwrap_or(0);
                    let password = opt_str(&params, "password");
                    let ssh: Option<connection::SshConfig> = parse_optional_struct(&params, "ssh");
                    let cluster: Option<connection::ClusterConfig> = parse_optional_struct(&params, "cluster");
                    let cfg = connection::ConnectionConfig {
                        id: String::new(),
                        name: String::new(),
                        color: None,
                        host,
                        port,
                        db,
                        password_obfuscated: String::new(),
                        ssh,
                        cluster,
                    };
                    (cfg, password)
                };

                let mut temp_tunnel: Option<ssh_tunnel::SshTunnel> = None;
                let client = Self::connect_client(&cfg.0, cfg.1, &mut temp_tunnel)?;
                let _: String =
                    redis::cmd("PING").query(&mut client.get_connection()?)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            "get_connection_info" => {
                if let Some(ref cfg) = self.current_config {
                    Ok(serde_json::json!({
                        "connected": true,
                        "id": cfg.id,
                        "name": cfg.name,
                        "color": cfg.color,
                        "host": cfg.host,
                        "port": cfg.port,
                        "db": cfg.db,
                    }))
                } else {
                    Ok(serde_json::json!({ "connected": false }))
                }
            }

            "save_connection" => {
                self.ensure_connections_loaded();
                let id = opt_str(&params, "id").unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let name = require_str(&params, "name")?;
                let host = params
                    .get("host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("127.0.0.1");
                let port =
                    params.get("port").and_then(|v| v.as_u64()).unwrap_or(6379) as u16;
                let db = params.get("db").and_then(|v| v.as_i64()).unwrap_or(0);
                let color = params
                    .get("color")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let password_obfuscated = match params.get("password").and_then(|v| v.as_str()) {
                    Some(p) if !p.is_empty() => hex::obfuscate(p),
                    _ => String::new(),
                };

                let mut conn_cfg = connection::ConnectionConfig {
                    id: id.clone(),
                    name: name.to_string(),
                    color,
                    host: host.to_string(),
                    port,
                    db,
                    password_obfuscated,
                    ssh: parse_optional_struct(&params, "ssh"),
                    cluster: parse_optional_struct(&params, "cluster"),
                };
                if let Some(ref mut ssh) = conn_cfg.ssh {
                    ssh.normalize();
                }

                if let Some(existing) = self.saved_connections.iter_mut().find(|c| c.id == id) {
                    *existing = conn_cfg;
                } else {
                    self.saved_connections.push(conn_cfg);
                }
                self.storage.save_json(&self.saved_connections)?;
                Ok(serde_json::json!({ "ok": true, "id": id }))
            }

            "list_connections" => {
                self.ensure_connections_loaded();
                let list: Vec<Value> = self
                    .saved_connections
                    .iter()
                    .map(|c| {
                        let ssh_info = c.ssh.as_ref().map(|s| {
                            let auth_type = match &s.auth {
                                connection::SshAuth::Password { .. } => "password",
                                connection::SshAuth::KeyPath { .. } => "key",
                            };
                            let has_auth = match &s.auth {
                                connection::SshAuth::Password { password_obfuscated } => !password_obfuscated.is_empty(),
                                connection::SshAuth::KeyPath { key_path, .. } => !key_path.is_empty(),
                            };
                            serde_json::json!({
                                "host": s.host,
                                "port": s.port,
                                "username": s.username,
                                "auth_type": auth_type,
                                "has_auth": has_auth,
                                "timeout_secs": s.timeout_secs,
                            })
                        });
                        let cluster_info = c.cluster.as_ref().map(|cl| {
                            serde_json::json!({
                                "seed_nodes": cl.seed_nodes.join(", "),
                            })
                        });
                        serde_json::json!({
                            "id": c.id,
                            "name": c.name,
                            "color": c.color,
                            "host": c.host,
                            "port": c.port,
                            "db": c.db,
                            "has_password": !c.password_obfuscated.is_empty(),
                            "has_ssh": c.ssh.is_some(),
                            "has_cluster": c.cluster.is_some(),
                            "ssh": ssh_info,
                            "cluster": cluster_info,
                        })
                    })
                    .collect();
                Ok(serde_json::json!({ "connections": list }))
            }

            "delete_connection" => {
                self.ensure_connections_loaded();
                let id = require_str(&params, "id")?;
                self.saved_connections.retain(|c| c.id != id);
                self.storage.save_json(&self.saved_connections)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            "get_saved_password" => {
                self.ensure_connections_loaded();
                let id = require_str(&params, "id")?;
                let conn = self
                    .saved_connections
                    .iter()
                    .find(|c| c.id == id)
                    .ok_or("连接配置不存在")?;
                let pass = if conn.password_obfuscated.is_empty() {
                    String::new()
                } else {
                    hex::deobfuscate(&conn.password_obfuscated).unwrap_or_default()
                };
                let ssh_password = conn.ssh.as_ref().and_then(|s| {
                    match &s.auth {
                        connection::SshAuth::Password { password_obfuscated } => {
                            if password_obfuscated.is_empty() {
                                None
                            } else {
                                hex::deobfuscate(password_obfuscated)
                            }
                        }
                        _ => None,
                    }
                }).unwrap_or_default();
                let ssh_key_passphrase = conn.ssh.as_ref().and_then(|s| {
                    match &s.auth {
                        connection::SshAuth::KeyPath { passphrase_obfuscated, .. } => {
                            passphrase_obfuscated.as_ref().and_then(|p| hex::deobfuscate(p))
                        }
                        _ => None,
                    }
                });
                Ok(serde_json::json!({
                    "password": pass,
                    "ssh_password": ssh_password,
                    "ssh_key_passphrase": ssh_key_passphrase,
                }))
            }

            // ── Key 操作 ──
            "scan_keys" => {
                let mut conn = self.get_conn()?;
                let cursor: u64 = params.get("cursor").and_then(|v| v.as_u64()).unwrap_or(0);
                let pattern = params.get("pattern").and_then(|v| v.as_str()).unwrap_or("*");
                let count: usize = params.get("count").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
                let max_keys = count.min(2000);

                let mut all_keys: Vec<Vec<u8>> = Vec::new();
                let mut current_cursor = cursor;
                let scan_batch = 1000u64;

                loop {
                    let (next_cursor, batch): (u64, Vec<Vec<u8>>) = redis::cmd("SCAN")
                        .arg(current_cursor)
                        .arg("MATCH")
                        .arg(pattern)
                        .arg("COUNT")
                        .arg(scan_batch)
                        .query(&mut conn)?;

                    all_keys.extend(batch);
                    current_cursor = next_cursor;

                    // Stop if we have enough keys or cursor wrapped back to 0
                    if all_keys.len() >= max_keys || current_cursor == 0 {
                        break;
                    }
                }

                all_keys.truncate(max_keys);
                let key_infos: Vec<Value> = operations::scan_key_infos(&all_keys, &mut conn)?;

                Ok(serde_json::json!({ "cursor": current_cursor, "keys": key_infos }))
            }

            "get_key_info" => {
                let key = require_str(&params, "key")?;
                let mut conn = self.get_conn()?;
                let key_type: String = redis::cmd("TYPE").arg(key).query(&mut conn)?;
                let ttl: i64 = redis::cmd("TTL").arg(key).query(&mut conn)?;
                let length: Option<usize> = match key_type.as_str() {
                    "string" => None,
                    "hash" => Some(redis::cmd("HLEN").arg(key).query(&mut conn)?),
                    "list" => Some(redis::cmd("LLEN").arg(key).query(&mut conn)?),
                    "set" => Some(redis::cmd("SCARD").arg(key).query(&mut conn)?),
                    "zset" => Some(redis::cmd("ZCARD").arg(key).query(&mut conn)?),
                    _ => None,
                };
                Ok(
                    serde_json::json!({ "key": key, "type": key_type, "ttl": ttl, "length": length }),
                )
            }

            "delete_key" => {
                let key = require_str(&params, "key")?;
                let mut conn = self.get_conn()?;
                let deleted: i32 = redis::cmd("UNLINK").arg(key).query(&mut conn)?;
                Ok(serde_json::json!({ "deleted": deleted }))
            }

            "delete_keys" => {
                let keys = params
                    .get("keys")
                    .and_then(|v| v.as_array())
                    .ok_or("缺少 keys")?;
                let mut conn = self.get_conn()?;
                let mut cmd = redis::cmd("UNLINK");
                for k in keys.iter().filter_map(|v| v.as_str()) {
                    cmd.arg(k);
                }
                let count: i32 = cmd.query(&mut conn)?;
                Ok(serde_json::json!({ "deleted": count }))
            }

            "rename_key" => {
                let old = require_str(&params, "old")?;
                let new = require_str(&params, "new")?;
                let mut conn = self.get_conn()?;
                redis::cmd("RENAME")
                    .arg(old)
                    .arg(new)
                    .query::<()>(&mut conn)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            "set_ttl" => {
                let key = require_str(&params, "key")?;
                let seconds = params
                    .get("seconds")
                    .and_then(|v| v.as_i64())
                    .ok_or("缺少 seconds")?;
                let mut conn = self.get_conn()?;
                let result: i32 = conn.expire(key, seconds)?;
                Ok(serde_json::json!({ "ok": result == 1 }))
            }

            // ── String ──
            "get_string" => {
                let key = require_str(&params, "key")?;
                let mut conn = self.get_conn()?;
                let value: String = conn.get(key)?;
                Ok(serde_json::json!({ "value": value, "is_json": operations::is_json(&value) }))
            }

            "set_string" => {
                let key = require_str(&params, "key")?;
                let value = require_str(&params, "value")?;
                let mut conn = self.get_conn()?;
                let _: () = conn.set(key, value)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            // ── Hash ──
            "get_hash" => {
                let key = require_str(&params, "key")?;
                let mut conn = self.get_conn()?;
                let fields: HashMap<String, String> = conn.hgetall(key)?;
                Ok(serde_json::json!({ "fields": fields }))
            }

            "set_hash_field" => {
                let key = require_str(&params, "key")?;
                let field = require_str(&params, "field")?;
                let value = require_str(&params, "value")?;
                let mut conn = self.get_conn()?;
                let _: () = conn.hset(key, field, value)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            "del_hash_field" => {
                let key = require_str(&params, "key")?;
                let field = require_str(&params, "field")?;
                let mut conn = self.get_conn()?;
                let _: () = conn.hdel(key, field)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            // ── List ──
            "get_list" => {
                let key = require_str(&params, "key")?;
                let start: isize =
                    params.get("start").and_then(|v| v.as_i64()).unwrap_or(0) as isize;
                let stop: isize =
                    params.get("stop").and_then(|v| v.as_i64()).unwrap_or(-1) as isize;
                let mut conn = self.get_conn()?;
                let items: Vec<String> = conn.lrange(key, start, stop)?;
                Ok(serde_json::json!({ "items": items }))
            }

            "lpush" => {
                let key = require_str(&params, "key")?;
                let value = require_str(&params, "value")?;
                let mut conn = self.get_conn()?;
                let len: i32 = conn.lpush(key, value)?;
                Ok(serde_json::json!({ "length": len }))
            }

            "rpush" => {
                let key = require_str(&params, "key")?;
                let value = require_str(&params, "value")?;
                let mut conn = self.get_conn()?;
                let len: i32 = conn.rpush(key, value)?;
                Ok(serde_json::json!({ "length": len }))
            }

            "lrem" => {
                let key = require_str(&params, "key")?;
                let index = params
                    .get("index")
                    .and_then(|v| v.as_i64())
                    .ok_or("缺少 index")?;
                let mut conn = self.get_conn()?;
                let value: String = conn.lindex(key, index as isize)?;
                let _: i32 = conn.lrem(key, 1, &value)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            // ── Set ──
            "get_set" => {
                let key = require_str(&params, "key")?;
                let mut conn = self.get_conn()?;
                let members: Vec<String> = conn.smembers(key)?;
                Ok(serde_json::json!({ "members": members }))
            }

            "sadd" => {
                let key = require_str(&params, "key")?;
                let member = require_str(&params, "member")?;
                let mut conn = self.get_conn()?;
                let added: i32 = conn.sadd(key, member)?;
                Ok(serde_json::json!({ "added": added }))
            }

            "srem" => {
                let key = require_str(&params, "key")?;
                let member = require_str(&params, "member")?;
                let mut conn = self.get_conn()?;
                let removed: i32 = conn.srem(key, member)?;
                Ok(serde_json::json!({ "removed": removed }))
            }

            // ── ZSet ──
            "get_zset" => {
                let key = require_str(&params, "key")?;
                let mut conn = self.get_conn()?;
                let members: Vec<(String, f64)> = conn.zrange_withscores(key, 0, -1)?;
                let result: Vec<Value> = members
                    .into_iter()
                    .map(|(m, s)| serde_json::json!({ "member": m, "score": s }))
                    .collect();
                Ok(serde_json::json!({ "members": result }))
            }

            "zadd" => {
                let key = require_str(&params, "key")?;
                let score = params
                    .get("score")
                    .and_then(|v| v.as_f64())
                    .ok_or("缺少 score")?;
                let member = require_str(&params, "member")?;
                let mut conn = self.get_conn()?;
                let added: i32 = conn.zadd(key, member, score)?;
                Ok(serde_json::json!({ "added": added }))
            }

            "zrem" => {
                let key = require_str(&params, "key")?;
                let member = require_str(&params, "member")?;
                let mut conn = self.get_conn()?;
                let removed: i32 = conn.zrem(key, member)?;
                Ok(serde_json::json!({ "removed": removed }))
            }

            // ── Operations ──
            "search_value" => {
                let key = require_str(&params, "key")?;
                let key_type = require_str(&params, "key_type")?;
                let query = require_str(&params, "query")?;
                let mut conn = self.get_conn()?;
                operations::search_value(&mut conn, key, key_type, query)
            }

            "hex_dump" => {
                let key = require_str(&params, "key")?;
                let max_bytes = params
                    .get("max_bytes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(256) as usize;
                let mut conn = self.get_conn()?;
                operations::hex_dump(&mut conn, key, max_bytes)
            }

            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(RedisClientPlugin::new()));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}

fn require_str<'a>(params: &'a Value, key: &str) -> Result<&'a str, Box<dyn std::error::Error + Send + Sync>> {
    params.get(key).and_then(|v| v.as_str()).ok_or(format!("缺少 {key}").into())
}

fn opt_str(params: &Value, key: &str) -> Option<String> {
    params.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string())
}

fn parse_optional_struct<T: serde::de::DeserializeOwned>(params: &Value, key: &str) -> Option<T> {
    params.get(key).and_then(|v| {
        if v.is_null() { None } else { serde_json::from_value(v.clone()).ok() }
    })
}

#[cfg(test)]
mod tests;
