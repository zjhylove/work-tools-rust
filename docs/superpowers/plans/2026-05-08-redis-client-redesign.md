# Redis Client 插件重构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 参考 Another Redis Desktop Manager 重构 redis-client 插件，增强连接管理、新增 SSH 隧道、数据查看器及批量操作。

**Architecture:** 后端拆分为 connection / ssh_tunnel / operations / lib 四个模块；前端从单文件 App.tsx 拆分为 15+ 组件 + 3 个 hooks + 工具函数，按 view 状态路由。

**Tech Stack:** Rust (ssh2 crate), React 18 + TypeScript + Vite, CSS 变量令牌

---

### Task 1: 创建 Rust 连接模型并添加 ssh2 依赖

**Files:**
- Modify: `plugins/redis-client/Cargo.toml`
- Create: `plugins/redis-client/src/connection.rs`

- [ ] **Step 1: 添加 ssh2 依赖**

```toml
# plugins/redis-client/Cargo.toml — 在 [dependencies] 末尾追加
ssh2 = "0.9"
```

- [ ] **Step 2: 创建 connection.rs 连接模型**

```rust
// plugins/redis-client/src/connection.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
    pub timeout_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SshAuth {
    #[serde(rename = "password")]
    Password { password_obfuscated: String },
    #[serde(rename = "key")]
    KeyPath { key_path: String, passphrase_obfuscated: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub seed_nodes: Vec<String>, // "host:port" 格式
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub host: String,
    pub port: u16,
    pub db: i64,
    #[serde(default)]
    pub password_obfuscated: String,
    #[serde(default)]
    pub ssh: Option<SshConfig>,
    #[serde(default)]
    pub cluster: Option<ClusterConfig>,
}

/// 连接模式（运行时决定走哪种路径）
pub enum ConnectionMode {
    Direct {
        host: String,
        port: u16,
        db: i64,
        password: Option<String>,
    },
    SshTunnel {
        ssh: SshConfig,
        remote_host: String,
        remote_port: u16,
        db: i64,
        password: Option<String>,
    },
    Cluster {
        seed_nodes: Vec<String>,
        password: Option<String>,
    },
}

impl ConnectionConfig {
    /// 构建用于建立 Redis 连接的 ConnectionMode
    pub fn to_connection_mode(&self, password: Option<String>) -> ConnectionMode {
        match (&self.ssh, &self.cluster) {
            (Some(ssh), _) => ConnectionMode::SshTunnel {
                ssh: ssh.clone(),
                remote_host: self.host.clone(),
                remote_port: self.port,
                db: self.db,
                password,
            },
            (_, Some(cluster)) => ConnectionMode::Cluster {
                seed_nodes: cluster.seed_nodes.clone(),
                password,
            },
            _ => ConnectionMode::Direct {
                host: self.host.clone(),
                port: self.port,
                db: self.db,
                password,
            },
        }
    }
}
```

- [ ] **Step 3: cargo check 验证编译**

```bash
cargo check -p redis-client
```

Expected: 编译通过（ssh2 crate 已存在于 lock 文件）

- [ ] **Step 4: 提交**

```bash
git add plugins/redis-client/Cargo.toml plugins/redis-client/src/connection.rs
git commit -m "feat(redis-client): add connection model and ssh2 dependency"
```

---

### Task 2: 实现 SSH 隧道模块

**Files:**
- Create: `plugins/redis-client/src/ssh_tunnel.rs`
- Reference: `plugins/k8s-forward/src/ssh_service.rs` (相同 ssh2 crate 用法)

- [ ] **Step 1: 创建 ssh_tunnel.rs**

```rust
// plugins/redis-client/src/ssh_tunnel.rs
use crate::connection::{SshAuth, SshConfig};
use ssh2::Session;
use std::net::TcpStream;
use std::time::Duration;

pub struct SshTunnel {
    session: Session,
    local_port: u16,
    _listener: std::net::TcpListener,
}

impl SshTunnel {
    /// 建立 SSH 连接并创建到目标 Redis 的本地端口转发
    /// 返回 local_port 供 redis crate 连接 localhost:<port>
    pub fn connect(config: &SshConfig, remote_host: &str, remote_port: u16) -> Result<u16, String> {
        let addr = format!("{}:{}", config.host, config.port);
        let tcp = TcpStream::connect(&addr)
            .map_err(|e| format!("SSH TCP 连接失败: {e}"))?;
        tcp.set_read_timeout(Some(Duration::from_secs(config.timeout_secs as u64)))
            .ok();

        let mut session = Session::new()
            .map_err(|e| format!("SSH session 创建失败: {e}"))?;
        session.set_tcp_stream(tcp);
        session.handshake()
            .map_err(|e| format!("SSH 握手失败: {e}"))?;

        match &config.auth {
            SshAuth::Password { password_obfuscated } => {
                let pass = super::hex::deobfuscate(password_obfuscated)
                    .unwrap_or_default();
                session.userauth_password(&config.username, &pass)
                    .map_err(|e| format!("SSH 密码认证失败: {e}"))?;
            }
            SshAuth::KeyPath { key_path, passphrase_obfuscated } => {
                let passphrase = passphrase_obfuscated.as_ref()
                    .and_then(|p| super::hex::deobfuscate(p));
                session.userauth_pubkey_file(
                    &config.username,
                    None,
                    std::path::Path::new(key_path),
                    passphrase.as_deref(),
                ).map_err(|e| format!("SSH 私钥认证失败: {e}"))?;
            }
        }

        if !session.authenticated() {
            return Err("SSH 认证未通过".into());
        }

        // 本地端口转发: localhost:0 → remote_host:remote_port
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("本地端口绑定失败: {e}"))?;
        let local_port = listener.local_addr()
            .map_err(|e| format!("获取本地端口失败: {e}"))?
            .port();

        // 转发线程
        let remote = format!("{}:{}", remote_host, remote_port);
        let session_clone = unsafe { session.sock() };
        let _thread = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    let mut channel = session.channel_direct_tcpip(
                        remote_host, remote_port, None,
                    );
                    if let Ok(mut channel) = channel {
                        let mut reader = stream.try_clone().unwrap();
                        let mut writer = stream;
                        // 双向转发 (简化版，完整版参考 k8s-forward)
                        std::io::copy(&mut reader, &mut channel).ok();
                        std::io::copy(&mut channel, &mut writer).ok();
                    }
                }
            }
        });

        Ok(local_port)
    }

    /// 验证 SSH 连通性（不建立转发，仅握手+认证）
    pub fn test_connect(config: &SshConfig) -> Result<(), String> {
        let addr = format!("{}:{}", config.host, config.port);
        let tcp = TcpStream::connect(&addr)
            .map_err(|e| format!("SSH TCP 连接失败: {e}"))?;
        tcp.set_read_timeout(Some(Duration::from_secs(config.timeout_secs as u64)))
            .ok();

        let mut session = Session::new()
            .map_err(|e| format!("SSH session 创建失败: {e}"))?;
        session.set_tcp_stream(tcp);
        session.handshake()
            .map_err(|e| format!("SSH 握手失败: {e}"))?;

        match &config.auth {
            SshAuth::Password { password_obfuscated } => {
                let pass = super::hex::deobfuscate(password_obfuscated)
                    .unwrap_or_default();
                session.userauth_password(&config.username, &pass)
                    .map_err(|e| format!("SSH 密码认证失败: {e}"))?;
            }
            SshAuth::KeyPath { key_path, passphrase_obfuscated } => {
                let passphrase = passphrase_obfuscated.as_ref()
                    .and_then(|p| super::hex::deobfuscate(p));
                session.userauth_pubkey_file(
                    &config.username,
                    None,
                    std::path::Path::new(key_path),
                    passphrase.as_deref(),
                ).map_err(|e| format!("SSH 私钥认证失败: {e}"))?;
            }
        }

        if !session.authenticated() {
            return Err("SSH 认证未通过".into());
        }
        Ok(())
    }
}
```

- [ ] **Step 2: cargo check**

```bash
cargo check -p redis-client
```

Expected: 编译通过

- [ ] **Step 3: 提交**

```bash
git add plugins/redis-client/src/ssh_tunnel.rs
git commit -m "feat(redis-client): implement SSH tunnel module"
```

---

### Task 3: 提取 hex.rs 为独立模块

**Files:**
- Modify: `plugins/redis-client/src/lib.rs` (移除 hex 模块代码)
- Create: `plugins/redis-client/src/hex.rs`

- [ ] **Step 1: 创建 hex.rs 并移动 hex 模块**

```rust
// plugins/redis-client/src/hex.rs
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
```

- [ ] **Step 2: 修改 lib.rs — 移除 hex mod，添加模块声明**

在 `lib.rs` 顶部：
```rust
use anyhow::Context;
use redis::{Client, Commands, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use worktools_plugin_api::storage::PluginStorage;
use worktools_plugin_api::Plugin;

mod hex;          // ← 新增
mod connection;   // ← 新增 (来自 Task1)
mod ssh_tunnel;   // ← 新增 (来自 Task2)
mod operations;   // ← 新增 (下一个 Task)

// 删除 XOR_KEY / obfuscate / deobfuscate 函数
// 删除末尾的 mod hex { ... } 整个块
```

将 lib.rs 中对 `obfuscate`、`deobfuscate` 的调用改为 `hex::obfuscate`、`hex::deobfuscate`：
```rust
// 原: obfuscate(password)
// 改: hex::obfuscate(password)

// 原: deobfuscate(&conn.password_obfuscated)
// 改: hex::deobfuscate(&conn.password_obfuscated)
```

- [ ] **Step 3: cargo check**

```bash
cargo check -p redis-client
```

Expected: 编译通过

- [ ] **Step 4: cargo test**

```bash
cargo test -p redis-client
```

Expected: 现有测试全部通过

- [ ] **Step 5: 提交**

```bash
git add plugins/redis-client/src/hex.rs plugins/redis-client/src/lib.rs
git commit -m "refactor(redis-client): extract hex module from lib.rs"
```

---

### Task 4: 创建 operations.rs 提取 Redis 操作逻辑

**Files:**
- Create: `plugins/redis-client/src/operations.rs`
- Modify: `plugins/redis-client/src/lib.rs` (handle_call 调用 operations 函数)

- [ ] **Step 1: 创建 operations.rs**

```rust
// plugins/redis-client/src/operations.rs
use redis::Connection;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Context;

/// SCAN keys 并获取 TYPE + TTL
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
        let ttl: i64 = redis::cmd("TTL")
            .arg(k)
            .query(conn)
            .unwrap_or(-2);
        result.push(serde_json::json!({ "key": k, "type": key_type, "ttl": ttl }));
    }
    Ok(result)
}

/// 在值内搜索（String/Hash/List/Set/ZSet 通用）
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
        _ => Err(format!("不支持对 {key_type} 类型搜索").into()),
    }
}

/// 尝试检测值是否为 JSON 字符串
pub fn is_json(s: &str) -> bool {
    let trimmed = s.trim();
    (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
}

/// HEX dump 前 N 字节（通过 GETRANGE 读取）
pub fn hex_dump(
    conn: &mut Connection,
    key: &str,
    max_bytes: usize,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let limit = max_bytes.min(4096);
    let bytes: Vec<u8> = redis::cmd("GETRANGE")
        .arg(key)
        .arg(0)
        .arg(limit - 1)
        .query(conn)?;

    let hex_str = super::hex::encode(&bytes);
    Ok(serde_json::json!({ "hex": hex_str, "length": bytes.len() }))
}
```

- [ ] **Step 2: cargo check**

```bash
cargo check -p redis-client
```

Expected: 编译通过 (operations.rs 仅被 lib.rs 声明 mod，尚未调用)

- [ ] **Step 3: 提交**

```bash
git add plugins/redis-client/src/operations.rs
git commit -m "feat(redis-client): add operations module with search and hex_dump"
```

---

### Task 5: 重构 lib.rs — 使用新的连接模型和操作方法

**Files:**
- Modify: `plugins/redis-client/src/lib.rs` (大幅重构)

- [ ] **Step 1: 重写 RedisClientPlugin 结构体和 handle_call**

关键变更：
1. `SavedConnection` 替换为 `ConnectionConfig`（来自 connection.rs）
2. `handle_call` 中的 `connect` 支持 SSH 和 Cluster 模式
3. 新增 `test_connection`、`delete_keys`、`search_value`、`hex_dump` 方法
4. `save_connection` 支持 upsert（有 id 则更新）

```rust
use anyhow::Context;
use redis::{Client, Commands, Connection};
use serde_json::Value;
use worktools_plugin_api::storage::PluginStorage;
use worktools_plugin_api::Plugin;

mod hex;
mod connection;
mod ssh_tunnel;
mod operations;

use connection::{ConnectionConfig, ConnectionMode, SshAuth, SshConfig, ClusterConfig};

pub struct RedisClientPlugin {
    client: Option<Client>,
    current_config: Option<ConnectionConfig>,
    storage: PluginStorage,
    saved_connections: Vec<ConnectionConfig>,
    connections_loaded: bool,
    // SSH 隧道保持存活的关键引用
    #[allow(dead_code)]
    ssh_tunnel_local_port: Option<u16>,
}

impl RedisClientPlugin {
    fn new() -> Self {
        Self {
            client: None,
            current_config: None,
            storage: PluginStorage::new("redis-client", "redis-client.json"),
            saved_connections: Vec::new(),
            connections_loaded: false,
            ssh_tunnel_local_port: None,
        }
    }

    fn ensure_connections_loaded(&mut self) {
        if self.connections_loaded { return; }
        self.saved_connections = self
            .storage
            .load_json::<Vec<ConnectionConfig>>()
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

    /// 构建 redis URL，处理直连/SSH 转发两种路径
    fn make_redis_url(
        host: &str,
        port: u16,
        db: i64,
        password: Option<&str>,
    ) -> String {
        match password {
            Some(p) if !p.is_empty() => format!("redis://:{}@{}:{}/{}", p, host, port, db),
            _ => format!("redis://{}:{}/{}", host, port, db),
        }
    }

    fn connect_client(
        conn_cfg: &ConnectionConfig,
        password: Option<String>,
    ) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
        let mode = conn_cfg.to_connection_mode(password);
        match mode {
            ConnectionMode::Direct { host, port, db, password } => {
                let url = Self::make_redis_url(&host, port, db, password.as_deref());
                Client::open(url.as_str()).map_err(|e| e.into())
            }
            ConnectionMode::SshTunnel { ssh, remote_host, remote_port, db, password } => {
                let local_port = ssh_tunnel::SshTunnel::connect(
                    &ssh, &remote_host, remote_port,
                ).map_err(|e| format!("SSH 隧道建立失败: {e}"))?;
                let url = Self::make_redis_url("127.0.0.1", local_port, db, password.as_deref());
                Client::open(url.as_str()).map_err(|e| e.into())
            }
            ConnectionMode::Cluster { seed_nodes, password } => {
                let node_urls: Vec<String> = seed_nodes.iter().map(|n| {
                    match &password {
                        Some(p) if !p.is_empty() => format!("redis://:{p}@{n}"),
                        _ => format!("redis://{n}"),
                    }
                }).collect();
                Client::open(node_urls[0].as_str()).map_err(|e| e.into())
            }
        }
    }
}
```

- [ ] **Step 2: 重写 handle_call — 连接管理方法**

在 `impl Plugin for RedisClientPlugin` 的 `handle_call` 中：

```rust
"connect" => {
    let conn_id = params.get("id")
        .and_then(|v| v.as_str())
        .ok_or("缺少连接 ID")?;

    // 从已保存连接中找到配置
    self.ensure_connections_loaded();
    let conn_cfg = self
        .saved_connections
        .iter()
        .find(|c| c.id == conn_id)
        .cloned()
        .ok_or("连接配置不存在")?;

    // 解析密码
    let password = if conn_cfg.password_obfuscated.is_empty() {
        params.get("password").and_then(|v| v.as_str()).map(|s| s.to_string())
    } else {
        hex::deobfuscate(&conn_cfg.password_obfuscated)
    };

    let client = Self::connect_client(&conn_cfg, password)?;
    let _: String = redis::cmd("PING").query(&mut client.get_connection()?)?;

    self.client = Some(client);
    self.current_config = Some(conn_cfg.clone());

    tracing::info!(id = %conn_cfg.id, name = %conn_cfg.name, "Redis 连接成功");
    Ok(serde_json::json!({
        "ok": true, "id": conn_cfg.id, "name": conn_cfg.name,
        "host": conn_cfg.host, "port": conn_cfg.port, "db": conn_cfg.db,
        "color": conn_cfg.color,
    }))
}

"test_connection" => {
    let host = params.get("host").and_then(|v| v.as_str()).unwrap_or("127.0.0.1");
    let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(6379) as u16;
    let password = params.get("password").and_then(|v| v.as_str());
    let ssh_config = params.get("ssh").cloned();
    let cluster_config = params.get("cluster").cloned();

    let conn_cfg = ConnectionConfig {
        id: String::new(), name: String::new(), color: None,
        host: host.to_string(), port,
        db: params.get("db").and_then(|v| v.as_i64()).unwrap_or(0),
        password_obfuscated: password.map(hex::obfuscate).unwrap_or_default(),
        ssh: ssh_config.map(|v| serde_json::from_value(v)).transpose()?,
        cluster: cluster_config.map(|v| serde_json::from_value(v)).transpose()?,
    };

    let password = params.get("password").and_then(|v| v.as_str()).map(|s| s.to_string());
    let client = Self::connect_client(&conn_cfg, password)?;
    let _: String = redis::cmd("PING").query(&mut client.get_connection()?)?;
    Ok(serde_json::json!({ "ok": true }))
}

"save_connection" => {
    self.ensure_connections_loaded();
    let id = params.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let name = params.get("name").and_then(|v| v.as_str()).ok_or("缺少 name")?;
    let host = params.get("host").and_then(|v| v.as_str()).unwrap_or("127.0.0.1");
    let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(6379) as u16;
    let db = params.get("db").and_then(|v| v.as_i64()).unwrap_or(0);
    let color = params.get("color").and_then(|v| v.as_str()).map(|s| s.to_string());
    let password = params.get("password").and_then(|v| v.as_str()).unwrap_or("");
    let ssh = params.get("ssh").cloned().map(|v| serde_json::from_value(v)).transpose()?;
    let cluster = params.get("cluster").cloned().map(|v| serde_json::from_value(v)).transpose()?;

    let cfg = ConnectionConfig {
        id: id.clone(),
        name: name.to_string(),
        color,
        host: host.to_string(),
        port,
        db,
        password_obfuscated: if password.is_empty() { String::new() } else { hex::obfuscate(password) },
        ssh,
        cluster,
    };

    // upsert
    self.saved_connections.retain(|c| c.id != id);
    self.saved_connections.push(cfg);
    self.persist_connections()?;
    Ok(serde_json::json!({ "ok": true, "id": id }))
}

"get_connection_info" => {
    if let Some(ref cfg) = self.current_config {
        Ok(serde_json::json!({
            "connected": true,
            "id": cfg.id, "name": cfg.name,
            "host": cfg.host, "port": cfg.port, "db": cfg.db,
            "color": cfg.color,
        }))
    } else {
        Ok(serde_json::json!({ "connected": false }))
    }
}
```

- [ ] **Step 3: 重写 handle_call — Key 和数据操作**

```rust
// list_connections / delete_connection / get_saved_password 适配 ConnectionConfig
"list_connections" => {
    self.ensure_connections_loaded();
    let list: Vec<Value> = self.saved_connections.iter().map(|c| {
        serde_json::json!({
            "id": c.id, "name": c.name, "color": c.color,
            "host": c.host, "port": c.port, "db": c.db,
            "has_password": !c.password_obfuscated.is_empty(),
            "has_ssh": c.ssh.is_some(),
            "has_cluster": c.cluster.is_some(),
        })
    }).collect();
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
    let conn = self.saved_connections.iter().find(|c| c.id == id).ok_or("连接配置不存在")?;
    if conn.password_obfuscated.is_empty() {
        Ok(serde_json::json!({ "password": "" }))
    } else {
        let pass = hex::deobfuscate(&conn.password_obfuscated).unwrap_or_default();
        Ok(serde_json::json!({ "password": pass }))
    }
}

// 保持 "disconnect" / "scan_keys" / "get_key_info" 不变
// 新增 "delete_keys"（批量删除）
"delete_keys" => {
    let keys = params.get("keys").and_then(|v| v.as_array())
        .ok_or("缺少 keys 数组")?;
    let mut conn = self.get_conn()?;
    let mut deleted: i64 = 0;
    for k in keys {
        if let Some(key) = k.as_str() {
            let d: i32 = conn.del(key)?;
            deleted += d as i64;
        }
    }
    Ok(serde_json::json!({ "deleted": deleted }))
}

// 新增 "search_value" / "hex_dump"
"search_value" => {
    let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
    let query = params.get("query").and_then(|v| v.as_str()).ok_or("缺少 query")?;
    let key_type: String = {
        let k = params.get("key_type").and_then(|v| v.as_str());
        match k {
            Some(t) => t.to_string(),
            None => {
                let mut conn = self.get_conn()?;
                redis::cmd("TYPE").arg(key).query(&mut conn)?
            }
        }
    };
    let mut conn = self.get_conn()?;
    operations::search_value(&mut conn, key, &key_type, query)
}

"hex_dump" => {
    let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
    let max_bytes = params.get("max_bytes").and_then(|v| v.as_u64()).unwrap_or(1024) as usize;
    let mut conn = self.get_conn()?;
    operations::hex_dump(&mut conn, key, max_bytes)
}

// 原有方法（scan_keys / get_key_info / delete_key / rename_key / set_ttl
// get_string / set_string / get_hash / set_hash_field / del_hash_field
// get_list / lpush / rpush / lrem / get_set / sadd / srem
// get_zset / zadd / zrem）保持不变
```

- [ ] **Step 4: cargo check**

```bash
cargo check -p redis-client
```

Expected: 编译通过

- [ ] **Step 5: 提交**

```bash
git add plugins/redis-client/src/lib.rs
git commit -m "feat(redis-client): refactor lib.rs with new connection model and operations"
```

---

### Task 6: 更新测试适配新模型

**Files:**
- Modify: `plugins/redis-client/src/tests.rs`

- [ ] **Step 1: 更新 tests.rs 适配 ConnectionConfig**

```rust
// 替换 SavedConnection 引用为 ConnectionConfig
// test_saved_connections_lifecycle 改用 save_connection 新的参数格式

use connection::ConnectionConfig;

// test_saved_connections_lifecycle 方法:
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

// test_avoid_duplicate_on_reconnect: 删除此测试
// (新模型不通 connect 自动保存，改用 save_connection upsert)

// 新增 test_delete_keys:
#[test]
fn test_delete_keys_batch() {
    let mut p = mkplugin();
    let k1 = tk("batch1");
    let k2 = tk("batch2");

    call(&mut p, "set_string", json!({ "key": k1, "value": "a" }));
    call(&mut p, "set_string", json!({ "key": k2, "value": "b" }));

    let r = call(&mut p, "delete_keys", json!({ "keys": [k1, k2] }));
    assert_eq!(r["deleted"].as_i64().unwrap(), 2);
}

// 新增 test_hex_dump:
#[test]
fn test_hex_dump() {
    let mut p = mkplugin();
    let k = tk("hex");
    call(&mut p, "set_string", json!({ "key": k, "value": "hello" }));
    let r = call(&mut p, "hex_dump", json!({ "key": k, "max_bytes": 10 }));
    assert_eq!(r["hex"].as_str().unwrap(), "68656c6c6f");
    assert_eq!(r["length"].as_i64().unwrap(), 5);
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test -p redis-client
```

Expected: 全部通过（需要本地 Redis 实例）

- [ ] **Step 3: 提交**

```bash
git add plugins/redis-client/src/tests.rs
git commit -m "test(redis-client): update tests for new connection model and operations"
```

---

### Task 7: 前端基础设施 — types + utils

**Files:**
- Create: `plugins/redis-client/frontend/src/types.ts`
- Create: `plugins/redis-client/frontend/src/utils/tree.ts`
- Create: `plugins/redis-client/frontend/src/utils/json.ts`

- [ ] **Step 1: 创建 types.ts**

```typescript
// plugins/redis-client/frontend/src/types.ts

export interface KeyInfo {
  key: string;
  type: string;
  ttl: number;
}

export interface SavedConnection {
  id: string;
  name: string;
  color: string | null;
  host: string;
  port: number;
  db: number;
  has_password: boolean;
  has_ssh: boolean;
  has_cluster: boolean;
}

export interface ConnectionForm {
  name: string;
  color: string | null;
  host: string;
  port: number;
  db: number;
  password: string;
  ssh: SshForm | null;
  cluster: ClusterForm | null;
}

export interface SshForm {
  host: string;
  port: number;
  username: string;
  authType: 'password' | 'key';
  password: string;
  keyPath: string;
  keyPassphrase: string;
  timeoutSecs: number;
}

export interface ClusterForm {
  seedNodes: string;
}

export interface TreeNode {
  name: string;
  fullKey: string | null;
  keyInfo?: KeyInfo;
  children: TreeNode[];
}

export type AppView = 'connect' | 'workspace' | 'manager';

export interface ConnectionInfo {
  connected: boolean;
  id?: string;
  name?: string;
  host?: string;
  port?: number;
  db?: number;
  color?: string | null;
}
```

- [ ] **Step 2: 创建 utils/tree.ts**

```typescript
// plugins/redis-client/frontend/src/utils/tree.ts
import { KeyInfo, TreeNode } from '../types';

export function buildTree(keys: KeyInfo[]): TreeNode[] {
  const root: TreeNode = { name: '', fullKey: null, children: [] };
  for (const k of keys) {
    const parts = k.key.split(':');
    let node = root;
    for (let i = 0; i < parts.length; i++) {
      const isLast = i === parts.length - 1;
      let child = node.children.find(c => c.name === parts[i]);
      if (!child) {
        child = { name: parts[i], fullKey: isLast ? k.key : null, children: [] };
        if (isLast) child.keyInfo = k;
        node.children.push(child);
      } else if (isLast) {
        child.fullKey = k.key;
        child.keyInfo = k;
      }
      node = child;
    }
  }
  return root.children;
}
```

- [ ] **Step 3: 创建 utils/json.ts**

```typescript
// plugins/redis-client/frontend/src/utils/json.ts

export function isJson(str: string): boolean {
  const trimmed = str.trim();
  return (trimmed.startsWith('{') && trimmed.endsWith('}'))
      || (trimmed.startsWith('[') && trimmed.endsWith(']'));
}

export function formatJson(str: string): string {
  try {
    return JSON.stringify(JSON.parse(str), null, 2);
  } catch {
    return str;
  }
}

export function compressJson(str: string): string {
  try {
    return JSON.stringify(JSON.parse(str));
  } catch {
    return str;
  }
}
```

- [ ] **Step 4: TypeScript 类型检查**

```bash
cd plugins/redis-client/frontend && npx tsc --noEmit
```

Expected: 无错误

- [ ] **Step 5: 提交**

```bash
git add plugins/redis-client/frontend/src/types.ts plugins/redis-client/frontend/src/utils/
git commit -m "feat(redis-client): add frontend types and utility functions"
```

---

### Task 8: 前端 hooks

**Files:**
- Create: `plugins/redis-client/frontend/src/hooks/useConnection.ts`
- Create: `plugins/redis-client/frontend/src/hooks/useKeys.ts`
- Create: `plugins/redis-client/frontend/src/hooks/useKeyDetail.ts`

- [ ] **Step 1: 创建 useConnection.ts**

```typescript
// plugins/redis-client/frontend/src/hooks/useConnection.ts
import { useState, useCallback } from 'react';
import { ConnectionInfo } from '../types';

declare global {
  interface Window {
    pluginAPI?: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

export function useConnection() {
  const [connected, setConnected] = useState(false);
  const [connectionInfo, setConnectionInfo] = useState<ConnectionInfo>({ connected: false });
  const [error, setError] = useState<string | null>(null);

  const connect = useCallback(async (id: string, password?: string) => {
    setError(null);
    const r = await window.pluginAPI?.call('redis-client', 'connect', { id, password });
    if (r && (r as Record<string, unknown>).ok) {
      const info = r as ConnectionInfo;
      setConnected(true);
      setConnectionInfo(info);
      return true;
    }
    return false;
  }, []);

  const disconnect = useCallback(async () => {
    await window.pluginAPI?.call('redis-client', 'disconnect', {});
    setConnected(false);
    setConnectionInfo({ connected: false });
  }, []);

  const clearError = useCallback(() => setError(null), []);

  return { connected, connectionInfo, error, setError, connect, disconnect, clearError };
}
```

- [ ] **Step 2: 创建 useKeys.ts**

```typescript
// plugins/redis-client/frontend/src/hooks/useKeys.ts
import { useState, useCallback, useMemo } from 'react';
import { KeyInfo, TreeNode } from '../types';
import { buildTree } from '../utils/tree';

export function useKeys() {
  const [keys, setKeys] = useState<KeyInfo[]>([]);
  const [nextCursor, setNextCursor] = useState(0);
  const [scanLoading, setScanLoading] = useState(false);
  const [hasScanned, setHasScanned] = useState(false);
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());

  const tree = useMemo(() => buildTree(keys), [keys]);

  const togglePath = useCallback((path: string) => {
    setExpandedPaths(prev => {
      const next = new Set(prev);
      next.has(path) ? next.delete(path) : next.add(path);
      return next;
    });
  }, []);

  const scan = useCallback(async (pattern: string, append = false) => {
    setScanLoading(true);
    setHasScanned(false);
    const cursor = append ? nextCursor : 0;
    const r = await window.pluginAPI?.call('redis-client', 'scan_keys', { cursor, pattern, count: 200 });
    if (r && (r as Record<string, unknown>).keys) {
      const data = r as { keys: KeyInfo[]; cursor: number };
      setKeys(prev => append ? [...prev, ...data.keys] : data.keys);
      setNextCursor(data.cursor);
    }
    setHasScanned(true);
    setScanLoading(false);
  }, [nextCursor]);

  const deleteSelectedKeys = useCallback(async (selectedKeys: string[]) => {
    await window.pluginAPI?.call('redis-client', 'delete_keys', { keys: selectedKeys });
    setKeys(prev => prev.filter(k => !selectedKeys.includes(k.key)));
  }, []);

  return { keys, setKeys, tree, nextCursor, scanLoading, hasScanned, expandedPaths, togglePath, scan, deleteSelectedKeys };
}
```

- [ ] **Step 3: 创建 useKeyDetail.ts**

```typescript
// plugins/redis-client/frontend/src/hooks/useKeyDetail.ts
import { useState, useCallback } from 'react';
import { KeyInfo } from '../types';

export function useKeyDetail() {
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [openTabs, setOpenTabs] = useState<KeyInfo[]>([]);
  const [keyDetail, setKeyDetail] = useState<Record<string, unknown> | null>(null);
  const [valueData, setValueData] = useState<unknown>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  const viewerMethods: Record<string, string> = {
    string: 'get_string',
    hash: 'get_hash',
    list: 'get_list',
    set: 'get_set',
    zset: 'get_zset',
  };

  const selectKey = useCallback(async (key: string) => {
    setSelectedKey(key);
    setDetailLoading(true);
    setValueData(null);

    try {
      const info = await window.pluginAPI?.call('redis-client', 'get_key_info', { key });
      setKeyDetail(info as Record<string, unknown>);

      const kType = (info as Record<string, string>).type;
      const method = viewerMethods[kType];
      if (method) {
        const v = await window.pluginAPI?.call('redis-client', method, { key });
        setValueData(v);
      }

      // 加入或更新 openTabs
      setOpenTabs(prev => {
        const exists = prev.find(t => t.key === key);
        if (exists) return prev;
        return [...prev, { key, type: kType, ttl: (info as Record<string, number>).ttl }];
      });
    } catch { /* handle in component */ }

    setDetailLoading(false);
  }, []);

  const closeTab = useCallback((key: string) => {
    setOpenTabs(prev => prev.filter(t => t.key !== key));
    if (selectedKey === key) {
      setSelectedKey(null);
      setKeyDetail(null);
      setValueData(null);
    }
  }, [selectedKey]);

  const refresh = useCallback(() => {
    if (selectedKey) selectKey(selectedKey);
  }, [selectedKey, selectKey]);

  return { selectedKey, setSelectedKey, openTabs, closeTab, keyDetail, valueData, detailLoading, selectKey, refresh };
}
```

- [ ] **Step 4: TypeScript 类型检查**

```bash
cd plugins/redis-client/frontend && npx tsc --noEmit
```

Expected: 无错误

- [ ] **Step 5: 提交**

```bash
git add plugins/redis-client/frontend/src/hooks/
git commit -m "feat(redis-client): add custom hooks for connection, keys, and key detail"
```

---

### Task 9: 连接管理组件（ConnectionBar + ConnectionManager + ConnectionEdit）

**Files:**
- Create: `plugins/redis-client/frontend/src/components/ConnectionBar.tsx`
- Create: `plugins/redis-client/frontend/src/components/ConnectionManager.tsx`
- Create: `plugins/redis-client/frontend/src/components/modals/ConnectionEdit.tsx`
- Create: `plugins/redis-client/frontend/src/components/modals/DeleteConfirm.tsx`

- [ ] **Step 1: 创建 ConnectionBar.tsx**

```tsx
// plugins/redis-client/frontend/src/components/ConnectionBar.tsx
import { SavedConnection } from '../types';

interface Props {
  savedConns: SavedConnection[];
  currentId: string | null;
  onConnect: (id: string) => void;
  onDisconnect: () => void;
  onManage: () => void;
}

const COLORS = ['#ef4444', '#f59e0b', '#10b981', '#3b82f6', '#8b5cf6', '#ec4899', '#06b6d4', '#6b7280'];

export function ConnectionBar({ savedConns, currentId, onConnect, onDisconnect, onManage }: Props) {
  const current = savedConns.find(c => c.id === currentId);

  return (
    <div className="connection-bar">
      <div className="connection-selector">
        <span className="status-dot" />
        <select
          value={currentId || ''}
          onChange={e => { if (e.target.value) onConnect(e.target.value); }}
        >
          <option value="" disabled>选择连接...</option>
          {savedConns.map(c => (
            <option key={c.id} value={c.id}>
              {COLORS[savedConns.indexOf(c) % COLORS.length] ? '' : ''} {c.name} ({c.host}:{c.port})
            </option>
          ))}
        </select>
      </div>
      <div className="connection-actions">
        <button onClick={onManage} title="管理连接">⚙</button>
        <button onClick={onDisconnect} title="断开连接">✕</button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 创建 ConnectionManager.tsx**

```tsx
// plugins/redis-client/frontend/src/components/ConnectionManager.tsx
import { useState } from 'react';
import { SavedConnection } from '../types';
import { ConnectionEdit } from './modals/ConnectionEdit';
import { DeleteConfirm } from './modals/DeleteConfirm';

interface Props {
  savedConns: SavedConnection[];
  onBack: () => void;
  onSave: () => void;
  onDelete: (id: string) => void;
  editId: string | null;
  onEditStart: (id: string | null) => void;
}

const COLORS = ['#ef4444', '#f59e0b', '#10b981', '#3b82f6', '#8b5cf6', '#ec4899', '#06b6d4', '#6b7280'];

export function ConnectionManager({ savedConns, onBack, onSave, onDelete, editId, onEditStart }: Props) {
  const [deleteId, setDeleteId] = useState<string | null>(null);

  return (
    <div className="connection-manager">
      <div className="manager-header">
        <button onClick={onBack}>← 返回</button>
        <h3>连接管理</h3>
        <button className="btn-primary" onClick={() => onEditStart(null)}>+ 新建</button>
      </div>
      <div className="manager-list">
        {savedConns.map((c, i) => (
          <div key={c.id} className="conn-card">
            <div className="conn-card-color" style={{ background: c.color || COLORS[i % COLORS.length] }} />
            <div className="conn-card-info">
              <div className="conn-card-name">{c.name}</div>
              <div className="conn-card-detail">{c.host}:{c.port} db{c.db}</div>
              {c.has_ssh && <span className="conn-badge">SSH</span>}
              {c.has_cluster && <span className="conn-badge">Cluster</span>}
            </div>
            <div className="conn-card-actions">
              <button onClick={() => onEditStart(c.id)}>编辑</button>
              <button className="btn-danger-text" onClick={() => setDeleteId(c.id)}>删除</button>
            </div>
          </div>
        ))}
      </div>

      {editId !== undefined && (
        <ConnectionEdit
          connId={editId}
          onClose={() => onEditStart(undefined!)}
          onSave={() => { onSave(); onEditStart(undefined!); }}
        />
      )}

      {deleteId && (
        <DeleteConfirm
          message={`确定删除连接？`}
          onConfirm={() => { onDelete(deleteId); setDeleteId(null); }}
          onCancel={() => setDeleteId(null)}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 3: 创建 ConnectionEdit.tsx**

```tsx
// plugins/redis-client/frontend/src/components/modals/ConnectionEdit.tsx
import { useState, useEffect } from 'react';
import { ConnectionForm, SshForm } from '../../types';

interface Props {
  connId: string | null;
  onClose: () => void;
  onSave: () => void;
}

const COLORS = ['#ef4444', '#f59e0b', '#10b981', '#3b82f6', '#8b5cf6', '#ec4899', '#06b6d4', '#6b7280'];

const defaultForm: ConnectionForm = {
  name: '', color: null, host: '127.0.0.1', port: 6379, db: 0, password: '',
  ssh: null, cluster: null,
};

export function ConnectionEdit({ connId, onClose, onSave }: Props) {
  const [form, setForm] = useState<ConnectionForm>(defaultForm);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (connId) {
      // 加载已有连接配置
      (async () => {
        const r = await window.pluginAPI?.call('redis-client', 'list_connections', {});
        const conns = (r as { connections: ConnectionForm[] }).connections;
        const c = conns.find(x => x.id === connId);
        if (c) setForm(c);
      })();
    }
  }, [connId]);

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      await window.pluginAPI?.call('redis-client', 'test_connection', {
        host: form.host, port: form.port, db: form.db, password: form.password,
        ssh: form.ssh, cluster: form.cluster,
      });
      setTestResult('连接成功');
    } catch (e) { setTestResult(`连接失败: ${e}`); }
    setTesting(false);
  };

  const handleSave = async () => {
    setSaving(true);
    await window.pluginAPI?.call('redis-client', 'save_connection', {
      id: connId || undefined,
      name: form.name, color: form.color, host: form.host, port: form.port, db: form.db,
      password: form.password,
      ssh: form.ssh ? {
        host: form.ssh.host, port: form.ssh.port, username: form.ssh.username,
        auth: form.ssh.authType === 'password'
          ? { type: 'password', password_obfuscated: form.ssh.password }
          : { type: 'key', key_path: form.ssh.keyPath, passphrase_obfuscated: form.ssh.keyPassphrase || null },
        timeout_secs: form.ssh.timeoutSecs,
      } : null,
      cluster: form.cluster ? { seed_nodes: form.cluster.seedNodes.split(',').map(s => s.trim()) } : null,
    });
    setSaving(false);
    onSave();
  };

  return (
    <div className="modal-overlay">
      <div className="modal-content">
        <div className="modal-header">
          <h3>{connId ? '编辑连接' : '新建连接'}</h3>
          <button onClick={onClose}>✕</button>
        </div>
        <div className="modal-body">
          <div className="form-group">
            <label>名称</label>
            <input value={form.name} onChange={e => setForm(p => ({ ...p, name: e.target.value }))} />
          </div>
          <div className="form-group">
            <label>颜色标记</label>
            <div className="color-options">
              {COLORS.map(c => (
                <span key={c} className={`color-dot ${form.color === c ? 'selected' : ''}`}
                  style={{ background: c }} onClick={() => setForm(p => ({ ...p, color: c }))} />
              ))}
            </div>
          </div>
          <div className="form-row">
            <div className="form-group flex-3"><label>Host</label>
              <input value={form.host} onChange={e => setForm(p => ({ ...p, host: e.target.value }))} /></div>
            <div className="form-group flex-1"><label>Port</label>
              <input type="number" value={form.port} onChange={e => setForm(p => ({ ...p, port: Number(e.target.value) }))} /></div>
          </div>
          <div className="form-group"><label>密码</label>
            <input type="password" value={form.password} onChange={e => setForm(p => ({ ...p, password: e.target.value }))} /></div>
          <div className="form-group"><label>DB</label>
            <input type="number" value={form.db} onChange={e => setForm(p => ({ ...p, db: Number(e.target.value) }))} /></div>

          <label className="checkbox-row">
            <input type="checkbox" checked={!!form.ssh} onChange={e => setForm(p => ({ ...p, ssh: e.target.checked ? { host: '', port: 22, username: '', authType: 'password', password: '', keyPath: '', keyPassphrase: '', timeoutSecs: 10 } : null }))} />
            通过 SSH 隧道连接
          </label>
          {form.ssh && (
            <div className="ssh-section">
              <div className="form-row">
                <div className="form-group flex-3"><label>SSH Host</label>
                  <input value={form.ssh.host} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, host: e.target.value } }))} /></div>
                <div className="form-group flex-1"><label>Port</label>
                  <input type="number" value={form.ssh.port} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, port: Number(e.target.value) } }))} /></div>
              </div>
              <div className="form-group"><label>用户名</label>
                <input value={form.ssh.username} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, username: e.target.value } }))} /></div>
              <div className="form-group"><label>认证方式</label>
                <select value={form.ssh.authType} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, authType: e.target.value as 'password' | 'key' } }))}>
                  <option value="password">密码</option>
                  <option value="key">私钥文件</option>
                </select>
              </div>
              {form.ssh.authType === 'password' ? (
                <div className="form-group"><label>SSH 密码</label>
                  <input type="password" value={form.ssh.password} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, password: e.target.value } }))} /></div>
              ) : (
                <>
                  <div className="form-group"><label>私钥路径</label>
                    <input value={form.ssh.keyPath} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, keyPath: e.target.value } }))} /></div>
                  <div className="form-group"><label>私钥密码（可选）</label>
                    <input type="password" value={form.ssh.keyPassphrase} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, keyPassphrase: e.target.value } }))} /></div>
                </>
              )}
            </div>
          )}

          <label className="checkbox-row">
            <input type="checkbox" checked={!!form.cluster} onChange={e => setForm(p => ({ ...p, cluster: e.target.checked ? { seedNodes: '' } : null }))} />
            Cluster 模式
          </label>
          {form.cluster && (
            <div className="form-group"><label>种子节点（逗号分隔 host:port）</label>
              <input value={form.cluster.seedNodes} onChange={e => setForm(p => ({ ...p, cluster: { seedNodes: e.target.value } }))}
                placeholder="host1:7000,host2:7001" />
            </div>
          )}
        </div>
        <div className="modal-footer">
          {testResult && <span className={testResult.includes('成功') ? 'text-success' : 'text-error'}>{testResult}</span>}
          <button onClick={handleTest} disabled={testing}>{testing ? '测试中…' : '测试连接'}</button>
          <button onClick={onClose}>取消</button>
          <button className="btn-primary" onClick={handleSave} disabled={saving}>{saving ? '保存中…' : '保存'}</button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: 创建 DeleteConfirm.tsx**

```tsx
// plugins/redis-client/frontend/src/components/modals/DeleteConfirm.tsx
interface Props {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function DeleteConfirm({ message, onConfirm, onCancel }: Props) {
  return (
    <div className="modal-overlay">
      <div className="modal-content modal-sm">
        <p>{message}</p>
        <div className="modal-footer">
          <button className="btn-danger" onClick={onConfirm}>确认删除</button>
          <button onClick={onCancel}>取消</button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 5: TypeScript 类型检查**

```bash
cd plugins/redis-client/frontend && npx tsc --noEmit
```

Expected: 无错误

- [ ] **Step 6: 提交**

```bash
git add plugins/redis-client/frontend/src/components/
git commit -m "feat(redis-client): add connection management components"
```

---

### Task 10: Key 面板组件（KeyPanel + KeyTree）

**Files:**
- Create: `plugins/redis-client/frontend/src/components/KeyPanel.tsx`
- Create: `plugins/redis-client/frontend/src/components/KeyTree.tsx`

- [ ] **Step 1: 创建 KeyTree.tsx**

```tsx
// plugins/redis-client/frontend/src/components/KeyTree.tsx
import { TreeNode } from '../types';

interface TreeItemProps {
  node: TreeNode;
  depth: number;
  selectedKey: string | null;
  expandedPaths: Set<string>;
  multiSelect: Set<string>;
  onToggle: (p: string) => void;
  onSelect: (k: string) => void;
  onMultiToggle: (k: string) => void;
}

function TreeItem({ node, depth, selectedKey, expandedPaths, multiSelect, onToggle, onSelect, onMultiToggle }: TreeItemProps) {
  const path = node.fullKey || node.name;
  const isFolder = node.fullKey === null;
  const isExpanded = expandedPaths.has(path);
  const isSelected = multiSelect.has(node.fullKey || '');

  if (isFolder) {
    return (
      <div className="tree-branch">
        <div className="tree-folder" style={{ paddingLeft: depth * 14 + 8 }} onClick={() => onToggle(path)}>
          <span className="tree-arrow">{isExpanded ? '▾' : '▸'}</span>
          <span className="tree-folder-name">{node.name}</span>
        </div>
        {isExpanded && node.children.map(child => (
          <TreeItem key={child.name} node={child} depth={depth + 1}
            selectedKey={selectedKey} expandedPaths={expandedPaths} multiSelect={multiSelect}
            onToggle={onToggle} onSelect={onSelect} onMultiToggle={onMultiToggle} />
        ))}
      </div>
    );
  }

  return (
    <div className={`tree-leaf ${selectedKey === node.fullKey ? 'selected' : ''}`}
      style={{ paddingLeft: depth * 14 + 24 }}>
      <input type="checkbox" className="tree-checkbox"
        checked={isSelected}
        onChange={() => node.fullKey && onMultiToggle(node.fullKey)}
        onClick={e => e.stopPropagation()} />
      <div className="tree-leaf-main" onClick={() => node.fullKey && onSelect(node.fullKey)}>
        {node.keyInfo && (
          <span className="key-type-badge" data-type={node.keyInfo.type}>{node.keyInfo.type}</span>
        )}
        <span className="tree-leaf-name">{node.name}</span>
        {node.keyInfo && node.keyInfo.ttl > 0 && (
          <span className="key-ttl">{node.keyInfo.ttl}s</span>
        )}
      </div>
    </div>
  );
}

interface KeyTreeProps {
  tree: TreeNode[];
  selectedKey: string | null;
  expandedPaths: Set<string>;
  multiSelect: Set<string>;
  onToggle: (p: string) => void;
  onSelect: (k: string) => void;
  onMultiToggle: (k: string) => void;
}

export function KeyTree({ tree, selectedKey, expandedPaths, multiSelect, onToggle, onSelect, onMultiToggle }: KeyTreeProps) {
  return (
    <div className="key-list">
      {tree.map(node => (
        <TreeItem key={node.name} node={node} depth={0}
          selectedKey={selectedKey} expandedPaths={expandedPaths} multiSelect={multiSelect}
          onToggle={onToggle} onSelect={onSelect} onMultiToggle={onMultiToggle} />
      ))}
    </div>
  );
}
```

- [ ] **Step 2: 创建 KeyPanel.tsx**

```tsx
// plugins/redis-client/frontend/src/components/KeyPanel.tsx
import { useState } from 'react';
import { TreeNode } from '../types';
import { KeyTree } from './KeyTree';

interface Props {
  tree: TreeNode[];
  selectedKey: string | null;
  expandedPaths: Set<string>;
  multiSelect: Set<string>;
  scanLoading: boolean;
  hasScanned: boolean;
  nextCursor: number;
  onToggle: (p: string) => void;
  onSelect: (k: string) => void;
  onMultiToggle: (k: string) => void;
  onScan: (pattern: string) => void;
  onLoadMore: () => void;
  onDeleteSelected: () => void;
}

export function KeyPanel({ tree, selectedKey, expandedPaths, multiSelect, scanLoading, hasScanned, nextCursor,
  onToggle, onSelect, onMultiToggle, onScan, onLoadMore, onDeleteSelected }: Props) {
  const [search, setSearch] = useState('*');

  return (
    <div className="key-panel">
      <div className="panel-header">
        <input type="text" value={search} onChange={e => setSearch(e.target.value)}
          placeholder="搜索 key (* 通配)" onKeyDown={e => e.key === 'Enter' && onScan(search)} />
        <button onClick={() => onScan(search)} disabled={scanLoading}>🔍</button>
        {multiSelect.size > 0 && (
          <button onClick={onDeleteSelected} title="删除选中">🗑</button>
        )}
      </div>

      {scanLoading && !tree.length ? (
        <div className="list-status"><span className="spinner" />扫描中…</div>
      ) : tree.length > 0 ? (
        <>
          <KeyTree tree={tree} selectedKey={selectedKey} expandedPaths={expandedPaths}
            multiSelect={multiSelect} onToggle={onToggle} onSelect={onSelect} onMultiToggle={onMultiToggle} />
          {nextCursor !== 0 && (
            <button className="btn-load-more" onClick={onLoadMore} disabled={scanLoading}>
              {scanLoading ? '加载中…' : '加载更多'}
            </button>
          )}
        </>
      ) : hasScanned ? (
        <div className="list-status">无匹配的 Key</div>
      ) : (
        <div className="list-status">输入 pattern 后搜索</div>
      )}
    </div>
  );
}
```

- [ ] **Step 3: TypeScript 类型检查**

```bash
cd plugins/redis-client/frontend && npx tsc --noEmit
```

Expected: 无错误

- [ ] **Step 4: 提交**

```bash
git add plugins/redis-client/frontend/src/components/KeyPanel.tsx plugins/redis-client/frontend/src/components/KeyTree.tsx
git commit -m "feat(redis-client): add KeyPanel and KeyTree components with multi-select"
```

---

### Task 11: 值查看器组件（DetailPanel + Viewers）

**Files:**
- Create: `plugins/redis-client/frontend/src/components/DetailPanel.tsx`
- Create: `plugins/redis-client/frontend/src/components/DetailToolbar.tsx`
- Create: `plugins/redis-client/frontend/src/components/viewers/StringViewer.tsx`
- Create: `plugins/redis-client/frontend/src/components/viewers/HashViewer.tsx`
- Create: `plugins/redis-client/frontend/src/components/viewers/ListViewer.tsx`
- Create: `plugins/redis-client/frontend/src/components/viewers/SetViewer.tsx`
- Create: `plugins/redis-client/frontend/src/components/viewers/ZSetViewer.tsx`
- Create: `plugins/redis-client/frontend/src/components/viewers/HexViewer.tsx`

- [ ] **Step 1: 创建 DetailToolbar.tsx**

```tsx
// plugins/redis-client/frontend/src/components/DetailToolbar.tsx
interface Props {
  viewerMode: 'text' | 'hex';
  showSearch: boolean;
  searchQuery: string;
  onViewerModeChange: (m: 'text' | 'hex') => void;
  onSearchChange: (q: string) => void;
  onSearchToggle: () => void;
}

export function DetailToolbar({ viewerMode, showSearch, searchQuery, onViewerModeChange, onSearchChange, onSearchToggle }: Props) {
  return (
    <div className="detail-toolbar">
      <div className="toolbar-left">
        <button className={viewerMode === 'text' ? 'active' : ''} onClick={() => onViewerModeChange('text')}>Text</button>
        <button className={viewerMode === 'hex' ? 'active' : ''} onClick={() => onViewerModeChange('hex')}>HEX</button>
      </div>
      <div className="toolbar-right">
        <button className={showSearch ? 'active' : ''} onClick={onSearchToggle}>🔍 搜索</button>
        {showSearch && (
          <input type="text" value={searchQuery} onChange={e => onSearchChange(e.target.value)}
            placeholder="搜索值…" autoFocus />
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 创建各 Viewer 组件**

```tsx
// StringViewer.tsx
import { useState, useEffect } from 'react';
import { isJson, formatJson, compressJson } from '../../utils/json';

interface Props {
  value: { value: string };
  selectedKey: string | null;
  onSave: (value: string) => void;
}

export function StringViewer({ value, selectedKey, onSave }: Props) {
  const [editing, setEditing] = useState('');
  const [formatted, setFormatted] = useState(false);

  useEffect(() => {
    setEditing(value.value);
    setFormatted(isJson(value.value));
  }, [value.value, selectedKey]);

  return (
    <div className="value-editor">
      <div className="viewer-actions">
        {isJson(value.value) && (
          <button onClick={() => {
            setEditing(prev => formatted ? compressJson(prev) : formatJson(prev));
            setFormatted(!formatted);
          }}>{formatted ? '压缩' : '格式化'}</button>
        )}
      </div>
      <textarea value={editing} onChange={e => setEditing(e.target.value)} rows={14} />
      <button className="btn-primary" onClick={() => onSave(editing)}>保存</button>
    </div>
  );
}
```

```tsx
// HashViewer.tsx
import { useState } from 'react';

interface Props {
  fields: Record<string, string>;
  selectedKey: string | null;
  onSetField: (field: string, value: string) => void;
  onDelField: (field: string) => void;
  searchQuery: string;
  multiSelect: Set<string>;
  onMultiToggle: (f: string) => void;
  onDeleteSelected: () => void;
}

export function HashViewer({ fields, selectedKey, onSetField, onDelField, searchQuery, multiSelect, onMultiToggle, onDeleteSelected }: Props) {
  const [newField, setNewField] = useState({ field: '', value: '' });
  const entries = Object.entries(fields).filter(([f, v]) =>
    !searchQuery || f.includes(searchQuery) || v.includes(searchQuery)
  );

  return (
    <div className="hash-editor">
      {multiSelect.size > 0 && (
        <div className="batch-bar">
          <span>已选 {multiSelect.size} 项</span>
          <button className="btn-danger" onClick={onDeleteSelected}>批量删除</button>
        </div>
      )}
      <table>
        <thead><tr><th /><th>Field</th><th>Value</th><th>操作</th></tr></thead>
        <tbody>
          {entries.map(([f, v]) => (
            <tr key={f}>
              <td><input type="checkbox" checked={multiSelect.has(f)}
                onChange={() => onMultiToggle(f)} /></td>
              <td><code>{f}</code></td>
              <td><code>{v}</code></td>
              <td><button onClick={() => onDelField(f)}>删除</button></td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="add-field">
        <input placeholder="field" value={newField.field} onChange={e => setNewField(p => ({ ...p, field: e.target.value }))} />
        <input placeholder="value" value={newField.value} onChange={e => setNewField(p => ({ ...p, value: e.target.value }))} />
        <button className="btn-primary" onClick={() => { onSetField(newField.field, newField.value); setNewField({ field: '', value: '' }); }}>
          添加</button>
      </div>
    </div>
  );
}
```

```tsx
// ListViewer.tsx — 展示有序列表，带搜索过滤
interface Props { items: string[]; searchQuery: string; }

export function ListViewer({ items, searchQuery }: Props) {
  const filtered = searchQuery ? items.filter(i => i.includes(searchQuery)) : items;
  return (
    <div className="list-editor">
      <ol>{filtered.map((item, i) => <li key={i}><code>{item}</code></li>)}</ol>
      {searchQuery && <div className="search-info">{filtered.length} / {items.length} 条匹配</div>}
    </div>
  );
}
```

```tsx
// SetViewer.tsx — 标签展示 + 搜索过滤
interface Props { members: string[]; searchQuery: string; }

export function SetViewer({ members, searchQuery }: Props) {
  const filtered = searchQuery ? members.filter(m => m.includes(searchQuery)) : members;
  return (
    <div className="set-editor">
      {filtered.map(m => <span key={m} className="member-tag">{m}</span>)}
      {searchQuery && <div className="search-info">{filtered.length} / {members.length} 条匹配</div>}
    </div>
  );
}
```

```tsx
// ZSetViewer.tsx — 表格展示 + 搜索过滤，按 score 排序
interface Props { members: Array<{ member: string; score: number }>; searchQuery: string; }

export function ZSetViewer({ members, searchQuery }: Props) {
  const filtered = searchQuery ? members.filter(m => m.member.includes(searchQuery)) : members;
  return (
    <div className="zset-editor">
      <table>
        <thead><tr><th>Member</th><th>Score</th></tr></thead>
        <tbody>
          {filtered.map(m => (
            <tr key={m.member}><td><code>{m.member}</code></td><td>{m.score}</td></tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

```tsx
// HexViewer.tsx — HEX 转储显示
import { useState, useEffect } from 'react';

interface Props { selectedKey: string | null; }

export function HexViewer({ selectedKey }: Props) {
  const [hex, setHex] = useState('');
  const [length, setLength] = useState(0);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!selectedKey) return;
    (async () => {
      setLoading(true);
      const r = await window.pluginAPI?.call('redis-client', 'hex_dump', { key: selectedKey, max_bytes: 1024 });
      const data = r as { hex: string; length: number };
      setHex(data.hex);
      setLength(data.length);
      setLoading(false);
    })();
  }, [selectedKey]);

  if (loading) return <div className="detail-loading"><span className="spinner" />加载中…</div>;

  // 格式化 HEX 输出: 每行 16 字节 + ASCII 预览
  const lines: string[] = [];
  for (let i = 0; i < hex.length; i += 32) {
    const hexPart = hex.slice(i, i + 32).match(/.{1,2}/g)?.join(' ') || '';
    const bytePart = hex.slice(i, i + 32).match(/.{1,2}/g)?.map(b => {
      const c = parseInt(b, 16);
      return c >= 32 && c <= 126 ? String.fromCharCode(c) : '.';
    }).join('') || '';
    lines.push(`${i / 2 | 0}| ${hexPart.padEnd(47)}|${bytePart}|`);
  }

  return (
    <div className="hex-viewer">
      <div className="hex-length">共 {length} 字节</div>
      <pre className="hex-dump">{lines.join('\n')}</pre>
    </div>
  );
}
```

- [ ] **Step 3: 创建 DetailPanel.tsx**

```tsx
// plugins/redis-client/frontend/src/components/DetailPanel.tsx
import { useState } from 'react';
import { DetailToolbar } from './DetailToolbar';
import { StringViewer } from './viewers/StringViewer';
import { HashViewer } from './viewers/HashViewer';
import { ListViewer } from './viewers/ListViewer';
import { SetViewer } from './viewers/SetViewer';
import { ZSetViewer } from './viewers/ZSetViewer';
import { HexViewer } from './viewers/HexViewer';

interface Props {
  selectedKey: string | null;
  keyDetail: Record<string, unknown> | null;
  valueData: unknown;
  detailLoading: boolean;
  onDeleteKey: () => void;
  onSaveString: (value: string) => void;
  onSetHashField: (field: string, value: string) => void;
  onDelHashField: (field: string) => void;
}

export function DetailPanel({ selectedKey, keyDetail, valueData, detailLoading,
  onDeleteKey, onSaveString, onSetHashField, onDelHashField }: Props) {
  const [viewerMode, setViewerMode] = useState<'text' | 'hex'>('text');
  const [showSearch, setShowSearch] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [hashMultiSelect, setHashMultiSelect] = useState<Set<string>>(new Set());

  if (detailLoading) return <div className="detail-loading"><span className="spinner" />加载中…</div>;
  if (!selectedKey || !keyDetail) return <div className="empty-detail">选择一个 Key 查看详情</div>;

  const kType = keyDetail.type as string;

  return (
    <div className="detail-panel">
      <div className="detail-header">
        <h4>{selectedKey}</h4>
        <span className="type-badge">{kType}</span>
        <span className="ttl-badge">TTL: {keyDetail.ttl as number}s</span>
        <button className="btn-danger" onClick={onDeleteKey}>删除</button>
      </div>

      <DetailToolbar viewerMode={viewerMode} showSearch={showSearch} searchQuery={searchQuery}
        onViewerModeChange={setViewerMode} onSearchChange={setSearchQuery} onSearchToggle={() => setShowSearch(!showSearch)} />

      {viewerMode === 'hex' ? (
        <HexViewer selectedKey={selectedKey} />
      ) : (
        <>
          {kType === 'string' && valueData && (
            <StringViewer value={valueData as { value: string }} selectedKey={selectedKey} onSave={onSaveString} />
          )}
          {kType === 'hash' && valueData && (
            <HashViewer fields={(valueData as { fields: Record<string, string> }).fields}
              selectedKey={selectedKey} onSetField={onSetHashField} onDelField={onDelHashField}
              searchQuery={searchQuery}
              multiSelect={hashMultiSelect} onMultiToggle={f => {
                setHashMultiSelect(prev => { const n = new Set(prev); n.has(f) ? n.delete(f) : n.add(f); return n; });
              }}
              onDeleteSelected={() => {
                hashMultiSelect.forEach(f => onDelHashField(f));
                setHashMultiSelect(new Set());
              }} />
          )}
          {kType === 'list' && valueData && (
            <ListViewer items={(valueData as { items: string[] }).items} searchQuery={searchQuery} />
          )}
          {kType === 'set' && valueData && (
            <SetViewer members={(valueData as { members: string[] }).members} searchQuery={searchQuery} />
          )}
          {kType === 'zset' && valueData && (
            <ZSetViewer members={(valueData as { members: Array<{ member: string; score: number }> }).members} searchQuery={searchQuery} />
          )}
        </>
      )}
    </div>
  );
}
```

- [ ] **Step 4: TypeScript 类型检查**

```bash
cd plugins/redis-client/frontend && npx tsc --noEmit
```

Expected: 无错误（可能需要少量类型修正）

- [ ] **Step 5: 提交**

```bash
git add plugins/redis-client/frontend/src/components/DetailPanel.tsx plugins/redis-client/frontend/src/components/DetailToolbar.tsx plugins/redis-client/frontend/src/components/viewers/
git commit -m "feat(redis-client): add DetailPanel and value viewer components"
```

---

### Task 12: App.tsx 顶层重构 + ConnectView + WorkspaceView

**Files:**
- Modify: `plugins/redis-client/frontend/src/App.tsx` (完全重写)
- Create: `plugins/redis-client/frontend/src/components/ConnectView.tsx`
- Create: `plugins/redis-client/frontend/src/components/WorkspaceView.tsx`

- [ ] **Step 1: 重写 App.tsx 为状态路由**

```tsx
// plugins/redis-client/frontend/src/App.tsx
import { useState, useCallback, useEffect } from 'react';
import { AppView, SavedConnection } from './types';
import { ConnectView } from './components/ConnectView';
import { WorkspaceView } from './components/WorkspaceView';
import { ConnectionManager } from './components/ConnectionManager';
import './App.css';

function App() {
  const [view, setView] = useState<AppView>('connect');
  const [savedConns, setSavedConns] = useState<SavedConnection[]>([]);
  const [editConnId, setEditConnId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [currentConnectionId, setCurrentConnectionId] = useState<string | null>(null);

  const loadSavedConns = useCallback(async () => {
    const r = await window.pluginAPI?.call('redis-client', 'list_connections', {});
    if (r && (r as Record<string, unknown>).connections) {
      setSavedConns((r as { connections: SavedConnection[] }).connections);
    }
  }, []);

  useEffect(() => { loadSavedConns(); }, [loadSavedConns]);

  const handleConnect = useCallback(async (id: string, password?: string) => {
    setError(null);
    try {
      await window.pluginAPI?.call('redis-client', 'connect', { id, password });
      setCurrentConnectionId(id);
      setView('workspace');
    } catch (e) { setError(String(e)); }
  }, []);

  const handleDisconnect = useCallback(async () => {
    await window.pluginAPI?.call('redis-client', 'disconnect', {});
    setCurrentConnectionId(null);
    setView('connect');
  }, []);

  const handleDeleteConn = useCallback(async (id: string) => {
    await window.pluginAPI?.call('redis-client', 'delete_connection', { id });
    loadSavedConns();
  }, [loadSavedConns]);

  switch (view) {
    case 'workspace':
      return (
        <WorkspaceView
          savedConns={savedConns}
          currentConnectionId={currentConnectionId}
          onDisconnect={handleDisconnect}
          onManage={() => setView('manager')}
          onConnect={handleConnect}
        />
      );
    case 'manager':
      return (
        <ConnectionManager
          savedConns={savedConns}
          onBack={() => setView('connect')}
          onSave={loadSavedConns}
          onDelete={handleDeleteConn}
          editId={editConnId}
          onEditStart={setEditConnId}
        />
      );
    default:
      return (
        <ConnectView
          savedConns={savedConns}
          onConnect={handleConnect}
          onManage={() => setView('manager')}
          onRefresh={loadSavedConns}
        />
      );
  }
}

export default App;
```

- [ ] **Step 2: 创建 ConnectView.tsx**

```tsx
// plugins/redis-client/frontend/src/components/ConnectView.tsx
import { useState } from 'react';
import { SavedConnection } from '../types';

interface Props {
  savedConns: SavedConnection[];
  onConnect: (id: string, password?: string) => void;
  onManage: () => void;
  onRefresh: () => void;
}

const COLORS = ['#ef4444', '#f59e0b', '#10b981', '#3b82f6', '#8b5cf6', '#ec4899', '#06b6d4', '#6b7280'];

export function ConnectView({ savedConns, onConnect, onManage, onRefresh }: Props) {
  const [passwordMap, setPasswordMap] = useState<Record<string, string>>({});

  return (
    <div className="connect-view">
      <div className="connect-header">
        <h3>Redis 连接</h3>
        <button onClick={onManage}>管理连接</button>
      </div>
      <div className="saved-connections">
        {savedConns.map((c, i) => (
          <div key={c.id} className="saved-conn-item">
            <div className="saved-conn-main" onClick={() => onConnect(c.id, passwordMap[c.id])}>
              <div className="conn-left">
                <span className="conn-color-dot" style={{ background: c.color || COLORS[i % COLORS.length] }} />
                <div>
                  <div className="conn-name">{c.name}</div>
                  <div className="conn-info">{c.host}:{c.port} db{c.db}</div>
                </div>
              </div>
              <div className="conn-tags">
                {c.has_ssh && <span className="conn-badge">SSH</span>}
                {c.has_cluster && <span className="conn-badge">Cluster</span>}
              </div>
            </div>
            {c.has_password && (
              <input type="password" placeholder="密码" className="conn-password"
                onChange={e => setPasswordMap(p => ({ ...p, [c.id]: e.target.value }))}
                onClick={e => e.stopPropagation()} />
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: 创建 WorkspaceView.tsx**

```tsx
// plugins/redis-client/frontend/src/components/WorkspaceView.tsx
import { useState, useCallback } from 'react';
import { SavedConnection, KeyInfo } from '../types';
import { ConnectionBar } from './ConnectionBar';
import { KeyPanel } from './KeyPanel';
import { DetailPanel } from './DetailPanel';
import { useKeys } from '../hooks/useKeys';
import { useKeyDetail } from '../hooks/useKeyDetail';

interface Props {
  savedConns: SavedConnection[];
  currentConnectionId: string | null;
  onDisconnect: () => void;
  onManage: () => void;
  onConnect: (id: string) => void;
}

export function WorkspaceView({ savedConns, currentConnectionId, onDisconnect, onManage, onConnect }: Props) {
  const { keys, tree, nextCursor, scanLoading, hasScanned, expandedPaths,
    togglePath, scan, deleteSelectedKeys } = useKeys();
  const { selectedKey, keyDetail, valueData, detailLoading, selectKey, refresh } = useKeyDetail();
  const [multiSelect, setMultiSelect] = useState<Set<string>>(new Set());
  const [pattern, setPattern] = useState('*');

  const handleScan = useCallback((p: string) => {
    setPattern(p);
    scan(p, false);
  }, [scan]);

  const handleLoadMore = useCallback(() => {
    if (nextCursor === 0 || scanLoading) return;
    scan(pattern, true);
  }, [nextCursor, scanLoading, scan, pattern]);

  const handleSaveString = useCallback(async (value: string) => {
    if (!selectedKey) return;
    await window.pluginAPI?.call('redis-client', 'set_string', { key: selectedKey, value });
    refresh();
  }, [selectedKey, refresh]);

  const handleSetHashField = useCallback(async (field: string, value: string) => {
    if (!selectedKey) return;
    await window.pluginAPI?.call('redis-client', 'set_hash_field', { key: selectedKey, field, value });
    refresh();
  }, [selectedKey, refresh]);

  const handleDelHashField = useCallback(async (field: string) => {
    if (!selectedKey) return;
    await window.pluginAPI?.call('redis-client', 'del_hash_field', { key: selectedKey, field });
    refresh();
  }, [selectedKey, refresh]);

  const handleDeleteKey = useCallback(async () => {
    if (!selectedKey) return;
    await window.pluginAPI?.call('redis-client', 'delete_key', { key: selectedKey });
    scan(pattern, false);
  }, [selectedKey, scan, pattern]);

  const handleDeleteSelected = useCallback(async () => {
    await deleteSelectedKeys(Array.from(multiSelect));
    setMultiSelect(new Set());
    scan(pattern, false);
  }, [multiSelect, deleteSelectedKeys, scan, pattern]);

  return (
    <div className="redis-client">
      <div className="main-layout">
        <div className="left-panel">
          <ConnectionBar savedConns={savedConns} currentId={currentConnectionId}
            onConnect={onConnect} onDisconnect={onDisconnect} onManage={onManage} />
          <KeyPanel tree={tree} selectedKey={selectedKey} expandedPaths={expandedPaths}
            multiSelect={multiSelect} scanLoading={scanLoading} hasScanned={hasScanned}
            nextCursor={nextCursor}
            onToggle={togglePath} onSelect={selectKey}
            onMultiToggle={k => {
              setMultiSelect(prev => { const n = new Set(prev); n.has(k) ? n.delete(k) : n.add(k); return n; });
            }}
            onScan={handleScan} onLoadMore={handleLoadMore}
            onDeleteSelected={handleDeleteSelected} />
        </div>
        <DetailPanel selectedKey={selectedKey} keyDetail={keyDetail} valueData={valueData}
          detailLoading={detailLoading}
          onDeleteKey={handleDeleteKey} onSaveString={handleSaveString}
          onSetHashField={handleSetHashField} onDelHashField={handleDelHashField} />
      </div>
    </div>
  );
}
```

- [ ] **Step 4: TypeScript 类型检查 + 构建**

```bash
cd plugins/redis-client/frontend && npx tsc --noEmit && npm run build
```

Expected: 无错误，构建成功

- [ ] **Step 5: 提交**

```bash
git add plugins/redis-client/frontend/src/App.tsx plugins/redis-client/frontend/src/components/ConnectView.tsx plugins/redis-client/frontend/src/components/WorkspaceView.tsx plugins/redis-client/frontend/src/assets/ --all
git commit -m "feat(redis-client): rewrite App with state routing and connect views"
```

---

### Task 13: 新 CSS 样式（App.css）

**Files:**
- Modify: `plugins/redis-client/frontend/src/App.css` (追加新组件样式，保留现有样式基础)

- [ ] **Step 1: 追加新组件 CSS**

在现有 `plugins/redis-client/frontend/src/App.css` 末尾追加：

```css
/* ═══ Connect View (new) ═══ */
.connect-view {
  width: 420px;
  max-width: 95%;
  margin: 24px auto;
}

.connect-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.connect-header h3 {
  margin: 0;
  font-family: var(--font-sans);
  font-size: var(--font-size-lg);
  font-weight: 700;
  color: var(--text-primary);
}

.connect-header button {
  padding: 6px 14px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
  cursor: pointer;
}

.connect-header button:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.saved-connections {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.conn-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.conn-color-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.conn-tags {
  display: flex;
  gap: 4px;
}

.conn-badge {
  font-family: var(--font-sans);
  font-size: 9px;
  font-weight: 600;
  padding: 2px 6px;
  border-radius: var(--radius-xs);
  background: var(--accent-light);
  color: var(--accent);
}

.conn-password {
  width: 100px;
  padding: 4px 8px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-xs);
  font-size: var(--font-size-sm);
  background: var(--bg-primary);
  color: var(--text-primary);
  margin: 4px 10px 8px;
}

/* ═══ Connection Bar ═══ */
.connection-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-secondary);
}

.connection-selector {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
}

.connection-selector .status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--success);
  flex-shrink: 0;
}

.connection-selector select {
  flex: 1;
  padding: 6px 8px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-family: var(--font-sans);
  font-size: var(--font-size-sm);
  background: var(--bg-primary);
  color: var(--text-primary);
}

.connection-actions {
  display: flex;
  gap: 4px;
}

.connection-actions button {
  padding: 4px 8px;
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-xs);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: var(--font-size-sm);
}

.connection-actions button:hover {
  border-color: var(--accent);
  color: var(--accent);
}

/* ═══ Left Panel Layout ═══ */
.main-layout {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.left-panel {
  width: 300px;
  min-width: 300px;
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
}

/* ═══ Multi-select ═══ */
.tree-checkbox {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  cursor: pointer;
}

.tree-leaf-main {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}

/* ═══ Detail Toolbar ═══ */
.detail-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid var(--border-light);
  margin-bottom: 12px;
}

.toolbar-left, .toolbar-right {
  display: flex;
  gap: 4px;
  align-items: center;
}

.toolbar-left button, .toolbar-right button {
  padding: 4px 10px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-xs);
  font-family: var(--font-sans);
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
  cursor: pointer;
}

.toolbar-left button.active, .toolbar-right button.active {
  background: var(--accent-light);
  border-color: var(--accent);
  color: var(--accent);
}

.toolbar-right input {
  padding: 4px 8px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-xs);
  font-family: var(--font-sans);
  font-size: var(--font-size-xs);
  background: var(--bg-primary);
  color: var(--text-primary);
  width: 140px;
}

/* ═══ HEX Viewer ═══ */
.hex-viewer {
  padding: 12px 0;
}

.hex-length {
  font-family: var(--font-sans);
  font-size: var(--font-size-xs);
  color: var(--text-tertiary);
  margin-bottom: 8px;
}

.hex-dump {
  font-family: var(--font-mono);
  font-size: var(--font-size-xs);
  line-height: 1.6;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  padding: 12px;
  overflow-x: auto;
  white-space: pre;
  color: var(--text-primary);
}

/* ═══ Viewer Actions ═══ */
.viewer-actions {
  display: flex;
  gap: 6px;
}

.viewer-actions button {
  padding: 3px 10px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-xs);
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
  cursor: pointer;
}

.viewer-actions button:hover {
  border-color: var(--accent);
  color: var(--accent);
}

/* ═══ Batch Bar ═══ */
.batch-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--accent-light);
  border: 1px solid var(--accent);
  border-radius: var(--radius-sm);
  margin-bottom: 10px;
  font-family: var(--font-sans);
  font-size: var(--font-size-sm);
  color: var(--accent);
}

/* ═══ Connection Manager ═══ */
.connection-manager {
  flex: 1;
  padding: 20px;
  overflow: auto;
  background: var(--bg-primary);
}

.manager-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 20px;
}

.manager-header h3 {
  margin: 0;
  font-family: var(--font-sans);
  font-size: var(--font-size-lg);
  font-weight: 700;
  color: var(--text-primary);
  flex: 1;
}

.manager-header button {
  padding: 6px 14px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  cursor: pointer;
}

.manager-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.conn-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  transition: border-color var(--transition-fast);
}

.conn-card:hover {
  border-color: var(--accent);
}

.conn-card-color {
  width: 32px;
  height: 32px;
  border-radius: var(--radius-md);
  flex-shrink: 0;
}

.conn-card-info {
  flex: 1;
}

.conn-card-name {
  font-family: var(--font-sans);
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 2px;
}

.conn-card-detail {
  font-family: var(--font-mono);
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
}

.conn-card-actions {
  display: flex;
  gap: 6px;
}

.conn-card-actions button {
  padding: 5px 12px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-xs);
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
  cursor: pointer;
}

.conn-card-actions button:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.btn-danger-text {
  color: var(--error) !important;
  border-color: var(--error-border) !important;
}

.btn-danger-text:hover {
  background: var(--error-light) !important;
}

/* ═══ Modal ═══ */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal-content {
  width: 480px;
  max-width: 90vw;
  max-height: 85vh;
  overflow: auto;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-xl);
}

.modal-sm { width: 360px; }

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-color);
}

.modal-header h3 {
  margin: 0;
  font-family: var(--font-sans);
  font-size: var(--font-size-lg);
  font-weight: 700;
  color: var(--text-primary);
}

.modal-header button {
  padding: 4px 8px;
  background: transparent;
  border: none;
  font-size: 16px;
  color: var(--text-tertiary);
  cursor: pointer;
}

.modal-body {
  padding: 20px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 8px;
  padding: 14px 20px;
  border-top: 1px solid var(--border-color);
}

.text-success { color: var(--success-text); font-size: var(--font-size-sm); }
.text-error { color: var(--error-text); font-size: var(--font-size-sm); }

/* ═══ Form Row ═══ */
.form-row {
  display: flex;
  gap: 10px;
}

.flex-1 { flex: 1; }
.flex-3 { flex: 3; }

.color-options {
  display: flex;
  gap: 8px;
}

.color-dot {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  cursor: pointer;
  border: 2px solid transparent;
  transition: border-color var(--transition-fast);
}

.color-dot.selected {
  border-color: var(--text-primary);
}

.checkbox-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: var(--font-sans);
  font-size: var(--font-size-base);
  color: var(--text-primary);
  padding: 10px 0;
  cursor: pointer;
}

.ssh-section {
  padding: 12px;
  margin: 8px 0;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
}

/* ═══ Search Info ═══ */
.search-info {
  font-family: var(--font-sans);
  font-size: var(--font-size-xs);
  color: var(--text-tertiary);
  margin-top: 8px;
}
```

- [ ] **Step 2: 构建验证**

```bash
cd plugins/redis-client/frontend && npm run build
```

Expected: 构建成功，输出到 assets/

- [ ] **Step 3: 提交**

```bash
git add plugins/redis-client/frontend/src/App.css
git commit -m "style(redis-client): add comprehensive CSS for redesigned components"
```

---

### Task 14: 端到端集成验证

- [ ] **Step 1: cargo clippy 全项目**

```bash
cargo clippy --all-targets -p redis-client
```

Expected: 无 error（允许有 warning）

- [ ] **Step 2: cargo test**

```bash
cargo test -p redis-client
```

需本地 Redis 实例在 127.0.0.1:6379 运行。

Expected: 测试全部通过

- [ ] **Step 3: 前端 TypeScript 检查 + 构建**

```bash
cd plugins/redis-client/frontend && npx tsc --noEmit && npm run build
```

Expected: 无类型错误，构建输出到 assets/

- [ ] **Step 4: 构建插件包**

```bash
cargo build --release -p redis-client
```

Expected: 编译成功，dll 输出到 target/release/

- [ ] **Step 5: 检查 assets 输出**

确认 `plugins/redis-client/assets/` 包含:
- `index.html`
- `main.js`
- `styles.css`

- [ ] **Step 6: 提交**

```bash
git add plugins/redis-client/assets/
git commit -m "chore(redis-client): finalize build artifacts"
```

---

### Task 15: 清理旧代码

**Files:**
- Modify: `plugins/redis-client/frontend/src/main.tsx` (保留不变)
- Modify: `plugins/redis-client/assets/index.html` (确认引用正确)

- [ ] **Step 1: 确认 index.html 引用路径正确**

```html
<!-- plugins/redis-client/assets/index.html -->
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Redis 客户端</title>
    <style>
      html, body { height: 100%; margin: 0; padding: 0; overflow: hidden; }
      #root { height: 100%; overflow: hidden; }
    </style>
    <script type="module" crossorigin src="./main.js"></script>
    <link rel="stylesheet" href="./styles.css">
  </head>
  <body>
    <div id="root"></div>
  </body>
</html>
```

- [ ] **Step 2: 删除旧的 assets 中的 main.js 和 styles.css**

构建已生成新版本，确认后不单独做此步 — vite build 会自动覆盖。

- [ ] **Step 3: 最终提交**

```bash
git add -A
git commit -m "chore(redis-client): cleanup and finalize plugin assets"
```
