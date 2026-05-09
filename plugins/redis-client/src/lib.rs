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

    fn persist_connections(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.storage.save_json(&self.saved_connections)?;
        Ok(())
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
                        let pw = params
                            .get("password")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                if cfg.password_obfuscated.is_empty() {
                                    None
                                } else {
                                    hex::deobfuscate(&cfg.password_obfuscated)
                                }
                            });
                        (cfg, pw)
                    } else {
                        let host = params
                            .get("host")
                            .and_then(|v| v.as_str())
                            .unwrap_or("127.0.0.1");
                        let port =
                            params.get("port").and_then(|v| v.as_u64()).unwrap_or(6379) as u16;
                        let db = params.get("db").and_then(|v| v.as_i64()).unwrap_or(0);
                        let password = params
                            .get("password")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string());
                        let cfg = connection::ConnectionConfig {
                            id: String::new(),
                            name: format!("{host}:{port}"),
                            color: None,
                            host: host.to_string(),
                            port,
                            db,
                            password_obfuscated: String::new(),
                            ssh: None,
                            cluster: None,
                        };
                        (cfg, password)
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
                let id = params.get("id").and_then(|v| v.as_str()).ok_or("缺少 id")?;
                self.ensure_connections_loaded();
                let cfg = self
                    .saved_connections
                    .iter()
                    .find(|c| c.id == id)
                    .ok_or("连接配置不存在")?;

                // Test SSH first if configured
                if let Some(ssh) = &cfg.ssh {
                    ssh_tunnel::SshTunnel::test_connect(ssh)
                        .map_err(|e| format!("SSH 连接测试失败: {e}"))?;
                }

                // Test Redis connectivity
                let password = params
                    .get("password")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        if cfg.password_obfuscated.is_empty() {
                            None
                        } else {
                            hex::deobfuscate(&cfg.password_obfuscated)
                        }
                    });

                let mut temp_tunnel: Option<ssh_tunnel::SshTunnel> = None;
                let client = Self::connect_client(cfg, password, &mut temp_tunnel)?;
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
                let id = params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 name")?;
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

                let ssh: Option<connection::SshConfig> = params
                    .get("ssh")
                    .and_then(|v| if v.is_null() { None } else { serde_json::from_value(v.clone()).ok() });
                let cluster: Option<connection::ClusterConfig> = params
                    .get("cluster")
                    .and_then(|v| if v.is_null() { None } else { serde_json::from_value(v.clone()).ok() });

                let conn_cfg = connection::ConnectionConfig {
                    id: id.clone(),
                    name: name.to_string(),
                    color,
                    host: host.to_string(),
                    port,
                    db,
                    password_obfuscated,
                    ssh,
                    cluster,
                };

                // Upsert
                if let Some(existing) = self.saved_connections.iter_mut().find(|c| c.id == id) {
                    *existing = conn_cfg;
                } else {
                    self.saved_connections.push(conn_cfg);
                }
                self.persist_connections()?;
                Ok(serde_json::json!({ "ok": true, "id": id }))
            }

            "list_connections" => {
                self.ensure_connections_loaded();
                let list: Vec<Value> = self
                    .saved_connections
                    .iter()
                    .map(|c| {
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
                        })
                    })
                    .collect();
                Ok(serde_json::json!({ "connections": list }))
            }

            "delete_connection" => {
                self.ensure_connections_loaded();
                let id = params.get("id").and_then(|v| v.as_str()).ok_or("缺少 id")?;
                self.saved_connections.retain(|c| c.id != id);
                self.persist_connections()?;
                Ok(serde_json::json!({ "ok": true }))
            }

            "get_saved_password" => {
                self.ensure_connections_loaded();
                let id = params.get("id").and_then(|v| v.as_str()).ok_or("缺少 id")?;
                let conn = self
                    .saved_connections
                    .iter()
                    .find(|c| c.id == id)
                    .ok_or("连接配置不存在")?;
                if conn.password_obfuscated.is_empty() {
                    Ok(serde_json::json!({ "password": "" }))
                } else {
                    let pass = hex::deobfuscate(&conn.password_obfuscated).unwrap_or_default();
                    Ok(serde_json::json!({ "password": pass }))
                }
            }

            // ── Key 操作 ──
            "scan_keys" => {
                let mut conn = self.get_conn()?;
                let cursor: u64 =
                    params.get("cursor").and_then(|v| v.as_u64()).unwrap_or(0);
                let pattern = params
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*");
                let count: usize =
                    params.get("count").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

                let (next_cursor, raw_keys): (u64, Vec<Vec<u8>>) = redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(pattern)
                    .arg("COUNT")
                    .arg(count)
                    .query(&mut conn)?;

                let keys: Vec<String> = raw_keys
                    .iter()
                    .map(|b| String::from_utf8_lossy(b).to_string())
                    .take(20) // 硬限制防止阻塞命令线程过久
                    .collect();

                let key_infos: Vec<Value> = operations::scan_key_infos(&keys, &mut conn)?;

                Ok(serde_json::json!({ "cursor": next_cursor, "keys": key_infos }))
            }

            "get_key_info" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
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
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let mut conn = self.get_conn()?;
                let deleted: i32 = conn.del(key)?;
                Ok(serde_json::json!({ "deleted": deleted }))
            }

            "delete_keys" => {
                let keys = params
                    .get("keys")
                    .and_then(|v| v.as_array())
                    .ok_or("缺少 keys")?;
                let mut conn = self.get_conn()?;
                let mut cmd = redis::cmd("DEL");
                for k in keys.iter().filter_map(|v| v.as_str()) {
                    cmd.arg(k);
                }
                let count: i32 = cmd.query(&mut conn)?;
                Ok(serde_json::json!({ "deleted": count }))
            }

            "rename_key" => {
                let old = params
                    .get("old")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 old")?;
                let new = params
                    .get("new")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 new")?;
                let mut conn = self.get_conn()?;
                redis::cmd("RENAME")
                    .arg(old)
                    .arg(new)
                    .query::<()>(&mut conn)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            "set_ttl" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
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
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let mut conn = self.get_conn()?;
                let value: String = conn.get(key)?;
                Ok(serde_json::json!({ "value": value }))
            }

            "set_string" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let value = params
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 value")?;
                let mut conn = self.get_conn()?;
                let _: () = conn.set(key, value)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            // ── Hash ──
            "get_hash" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let mut conn = self.get_conn()?;
                let fields: HashMap<String, String> = conn.hgetall(key)?;
                Ok(serde_json::json!({ "fields": fields }))
            }

            "set_hash_field" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let field = params
                    .get("field")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 field")?;
                let value = params
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 value")?;
                let mut conn = self.get_conn()?;
                let _: () = conn.hset(key, field, value)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            "del_hash_field" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let field = params
                    .get("field")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 field")?;
                let mut conn = self.get_conn()?;
                let _: () = conn.hdel(key, field)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            // ── List ──
            "get_list" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let start: isize =
                    params.get("start").and_then(|v| v.as_i64()).unwrap_or(0) as isize;
                let stop: isize =
                    params.get("stop").and_then(|v| v.as_i64()).unwrap_or(-1) as isize;
                let mut conn = self.get_conn()?;
                let items: Vec<String> = conn.lrange(key, start, stop)?;
                Ok(serde_json::json!({ "items": items }))
            }

            "lpush" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let value = params
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 value")?;
                let mut conn = self.get_conn()?;
                let len: i32 = conn.lpush(key, value)?;
                Ok(serde_json::json!({ "length": len }))
            }

            "rpush" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let value = params
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 value")?;
                let mut conn = self.get_conn()?;
                let len: i32 = conn.rpush(key, value)?;
                Ok(serde_json::json!({ "length": len }))
            }

            "lrem" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
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
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let mut conn = self.get_conn()?;
                let members: Vec<String> = conn.smembers(key)?;
                Ok(serde_json::json!({ "members": members }))
            }

            "sadd" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let member = params
                    .get("member")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 member")?;
                let mut conn = self.get_conn()?;
                let added: i32 = conn.sadd(key, member)?;
                Ok(serde_json::json!({ "added": added }))
            }

            "srem" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let member = params
                    .get("member")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 member")?;
                let mut conn = self.get_conn()?;
                let removed: i32 = conn.srem(key, member)?;
                Ok(serde_json::json!({ "removed": removed }))
            }

            // ── ZSet ──
            "get_zset" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let mut conn = self.get_conn()?;
                let members: Vec<(String, f64)> = conn.zrange_withscores(key, 0, -1)?;
                let result: Vec<Value> = members
                    .into_iter()
                    .map(|(m, s)| serde_json::json!({ "member": m, "score": s }))
                    .collect();
                Ok(serde_json::json!({ "members": result }))
            }

            "zadd" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let score = params
                    .get("score")
                    .and_then(|v| v.as_f64())
                    .ok_or("缺少 score")?;
                let member = params
                    .get("member")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 member")?;
                let mut conn = self.get_conn()?;
                let added: i32 = conn.zadd(key, member, score)?;
                Ok(serde_json::json!({ "added": added }))
            }

            "zrem" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let member = params
                    .get("member")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 member")?;
                let mut conn = self.get_conn()?;
                let removed: i32 = conn.zrem(key, member)?;
                Ok(serde_json::json!({ "removed": removed }))
            }

            // ── Operations ──
            "search_value" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
                let key_type = params
                    .get("key_type")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key_type")?;
                let query = params
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 query")?;
                let mut conn = self.get_conn()?;
                operations::search_value(&mut conn, key, key_type, query)
            }

            "hex_dump" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 key")?;
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

#[cfg(test)]
mod tests;
