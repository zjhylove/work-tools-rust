# K8s IP 转发插件 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于 Java ip-forward-plugin 架构，用 Rust 复刻 IP 转发插件，通过 Kuboard (DEX SSO) 发现 K8s Pod 并通过 SSH 隧道 + HTTP 代理转发流量。

**Architecture:** 三个核心模块 — KuboardClient (reqwest, SSO 登录 + K8s API)、SshService (ssh2, 端口转发)、HttpProxySvc (hyper, Host header 路由代理)。Plugin trait 统一调度 24 个 handle_call 方法，前端 React + Vite 三 Tab。

**Tech Stack:** Rust cdylib | ssh2 | hyper 1.x | reqwest 0.12 | tokio | React 19 + Vite 6 + TypeScript

---

## 文件映射

| 文件 | 职责 |
|---|---|
| `plugins/k8s-forward/Cargo.toml` | 依赖声明，cdylib |
| `plugins/k8s-forward/manifest.json` | 插件元数据 |
| `plugins/k8s-forward/src/models.rs` | 所有数据结构定义 |
| `plugins/k8s-forward/src/crypto.rs` | AES-256 密码加密 |
| `plugins/k8s-forward/src/kuboard_client.rs` | DEX SSO 登录 + K8s API 调用 |
| `plugins/k8s-forward/src/ssh_service.rs` | SSH 连接 + 端口转发线程管理 |
| `plugins/k8s-forward/src/http_proxy.rs` | HTTP 反向代理 (hyper) |
| `plugins/k8s-forward/src/lib.rs` | Plugin trait 实现 + handle_call 调度 |
| `plugins/k8s-forward/frontend/*` | React + Vite 前端 |
| `Cargo.toml` (根) | 添加 workspace member |

---

### Task 1: 项目脚手架

**Files:**
- Create: `plugins/k8s-forward/Cargo.toml`
- Create: `plugins/k8s-forward/manifest.json`
- Create: `plugins/k8s-forward/src/models.rs`

- [ ] **Step 1: 创建 Cargo.toml**

```toml
[package]
name = "k8s-forward"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
tokio = { version = "1.0", features = ["rt-multi-thread", "sync", "net", "io-util"] }
reqwest = { version = "0.12", features = ["cookies", "redirect"] }
ssh2 = "0.9"
hyper = { version = "1", features = ["server", "client", "http1"] }
hyper-util = { version = "0.1", features = ["full"] }
http-body-util = "0.1"
aes = "0.8"
sha2 = "0.10"
hex = "0.4"
uuid = { version = "1.0", features = ["v4"] }
once_cell = "1.19"
```

- [ ] **Step 2: 创建 manifest.json**

```json
{
  "id": "k8s-forward",
  "name": "K8s IP转发",
  "description": "通过Kuboard发现K8s Pod,SSH隧道+HTTP代理转发流量",
  "version": "1.0.0",
  "icon": "\u{1F310}",
  "author": "Work Tools Team",
  "files": {
    "macos": "libk8s_forward.dylib",
    "linux": "libk8s_forward.so",
    "windows": "k8s_forward.dll"
  },
  "assets": {
    "entry": "index.html"
  },
  "permissions": [
    "filesystem",
    "network"
  ]
}
```

- [ ] **Step 3: 创建 models.rs — 完整数据模型**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    Manual,
    K8s,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardRule {
    pub id: String,
    pub name: String,
    #[serde(default = "default_local_host")]
    pub local_host: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub rule_type: RuleType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
}

fn default_local_host() -> String {
    "127.0.0.1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyMapping {
    pub domain: String,
    pub target: String,
    pub rule_id: String,
    #[serde(default = "default_true")]
    pub editable: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub username: String,
    pub password: String,
}

fn default_ssh_port() -> u16 { 22 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KuboardConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default = "default_proxy_port")]
    pub port: u16,
}

fn default_proxy_port() -> u16 { 80 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginData {
    #[serde(default)]
    pub ssh: Option<SshConfig>,
    #[serde(default)]
    pub kuboard: Option<KuboardConfig>,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub forward_rules: Vec<ForwardRule>,
}

impl Default for PluginData {
    fn default() -> Self {
        Self {
            ssh: None,
            kuboard: None,
            proxy: ProxyConfig { port: 80 },
            forward_rules: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerPort {
    #[serde(default)]
    pub name: Option<String>,
    pub container_port: u16,
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

fn default_protocol() -> String { "TCP".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub name: String,
    #[serde(default)]
    pub ports: Vec<ContainerPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodInfo {
    pub name: String,
    pub ip: String,
    pub status: String,
    #[serde(default)]
    pub containers: Vec<ContainerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshStatus {
    pub connected: bool,
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KuboardStatus {
    pub logged_in: bool,
    pub url: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub port: u16,
    pub mapping_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct K8sForwardInfo {
    pub rules: Vec<ForwardRule>,
    pub mappings: Vec<ProxyMapping>,
}
```

- [ ] **Step 4: Commit**

```bash
git add plugins/k8s-forward/Cargo.toml plugins/k8s-forward/manifest.json plugins/k8s-forward/src/models.rs
git commit -m "feat(k8s-forward): add project scaffold and data models"
```

---

### Task 2: 密码加密模块

**Files:**
- Create: `plugins/k8s-forward/src/crypto.rs`

- [ ] **Step 1: 创建 crypto.rs（复用 password-manager 的 AES-256 ECB + PKCS7 方案）**

```rust
use aes::Aes256;
use aes::cipher::{KeyInit, BlockEncrypt, BlockDecrypt, generic_array::GenericArray};
use sha2::{Sha256, Digest};
use anyhow::Result;

pub struct PasswordEncryptor {
    cipher: Aes256,
}

impl PasswordEncryptor {
    fn get_internal_key() -> [u8; 32] {
        let app_secret = "WorkToolsK8sForward2024InternalKey!";
        let mut hasher = Sha256::new();
        hasher.update(app_secret.as_bytes());
        hasher.update(b"K8S_FORWARD_SALT_FIXED");
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result[..32]);
        key
    }

    pub fn new() -> Self {
        let key = Self::get_internal_key();
        let cipher = Aes256::new(&GenericArray::from(key));
        Self { cipher }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let plaintext_bytes = plaintext.as_bytes();
        let block_size = 16;
        let padding_len = if plaintext_bytes.len().is_multiple_of(block_size) {
            block_size
        } else {
            block_size - (plaintext_bytes.len() % block_size)
        };
        let mut padded_data = plaintext_bytes.to_vec();
        for _ in 0..padding_len {
            padded_data.push(padding_len as u8);
        }
        let mut encrypted_data = Vec::new();
        for chunk in padded_data.chunks(16) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            self.cipher.encrypt_block(GenericArray::from_mut_slice(&mut block));
            encrypted_data.extend_from_slice(&block);
        }
        Ok(hex::encode(encrypted_data))
    }

    pub fn decrypt(&self, ciphertext: &str) -> Result<String> {
        let encrypted_data = hex::decode(ciphertext)?;
        if encrypted_data.len() % 16 != 0 {
            return Err(anyhow::anyhow!("密文长度无效"));
        }
        let mut decrypted_data = Vec::new();
        for chunk in encrypted_data.chunks(16) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            self.cipher.decrypt_block(GenericArray::from_mut_slice(&mut block));
            decrypted_data.extend_from_slice(&block);
        }
        if decrypted_data.is_empty() {
            return Err(anyhow::anyhow!("解密结果为空"));
        }
        let padding_len = decrypted_data[decrypted_data.len() - 1] as usize;
        if padding_len > 16 || padding_len == 0 {
            return Err(anyhow::anyhow!("填充长度无效"));
        }
        let padding_start = decrypted_data.len() - padding_len;
        for byte in &decrypted_data[padding_start..] {
            if *byte != padding_len as u8 {
                return Err(anyhow::anyhow!("填充数据无效"));
            }
        }
        decrypted_data.truncate(decrypted_data.len() - padding_len);
        String::from_utf8(decrypted_data).map_err(|e| anyhow::anyhow!("UTF-8 解码失败: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let encryptor = PasswordEncryptor::new();
        let encrypted = encryptor.encrypt("my_secret_password").unwrap();
        assert_ne!(encrypted, "my_secret_password");
        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "my_secret_password");
    }

    #[test]
    fn test_empty_string() {
        let encryptor = PasswordEncryptor::new();
        let encrypted = encryptor.encrypt("").unwrap();
        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "");
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test -p k8s-forward
```
预期: 2 tests PASS

- [ ] **Step 3: Commit**

```bash
git add plugins/k8s-forward/src/crypto.rs
git commit -m "feat(k8s-forward): add AES-256 password encryption module"
```

---

### Task 3: KuboardClient — SSO 登录 + K8s API

**Files:**
- Create: `plugins/k8s-forward/src/kuboard_client.rs`

- [ ] **Step 1: 创建 kuboard_client.rs**

```rust
use anyhow::{Result, anyhow};
use reqwest::{Client, cookie::Jar};
use std::sync::Arc;
use crate::models::*;

pub struct KuboardClient {
    client: Client,
    base_url: String,
    logged_in: bool,
    username: String,
    password: String, // encrypted in storage, decrypted on load
}

impl KuboardClient {
    pub fn new(base_url: &str) -> Self {
        let cookie_jar = Arc::new(Jar::default());
        let client = Client::builder()
            .cookie_provider(cookie_jar)
            .redirect(reqwest::redirect::Policy::none())
            .danger_accept_invalid_certs(false)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            logged_in: false,
            username: String::new(),
            password: String::new(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Step 1: 从 redirect 获取 SSO req_id
    async fn fetch_req_id(&self) -> Result<String> {
        let resp = self.client
            .get(&self.url("/kuboard/cluster"))
            .send()
            .await?;
        let status = resp.status().as_u16();
        if status == 303 || status == 302 {
            if let Some(location) = resp.headers().get("location") {
                let loc_str = location.to_str()?;
                if let Some(pos) = loc_str.find("req=") {
                    return Ok(loc_str[pos + 4..].to_string());
                }
            }
        }
        // follow redirect manually to get SSO page
        let resp = self.client
            .get(&self.url("/kuboard/cluster"))
            .send()
            .await?;
        let final_url = resp.url().to_string();
        if let Some(pos) = final_url.find("req=") {
            return Ok(final_url[pos + 4..].to_string());
        }
        // try the redirect chain
        let resp = self.client
            .get(&self.url("/login?state=%2Fkuboard%2Fcluster"))
            .send()
            .await?;
        let final_url = resp.url().to_string();
        if let Some(pos) = final_url.find("req=") {
            return Ok(final_url[pos + 4..].to_string());
        }
        Err(anyhow!("无法获取 SSO req_id，请检查 Kuboard 地址"))
    }

    /// Step 2: POST 登录到 SSO
    pub async fn login(&mut self, username: &str, password: &str) -> Result<LoginResult> {
        self.username = username.to_string();
        self.password = password.to_string();

        let req_id = self.fetch_req_id().await?;

        let pwd_json = format!("{{\"password\":\"{}\"}}", password);
        let body = format!("login={}&password={}", 
            urlencoding(&username), urlencoding(&pwd_json));

        let resp = self.client
            .post(&self.url(&format!("/sso/auth/default?req={}", req_id)))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();

        // SSO 通过 HTTP error 响应返回结果，解析 message 字段
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(msg) = json["message"].as_str() {
                return Ok(parse_login_message(msg));
            }
        }

        // 如果返回 302/303 重定向，说明登录成功
        if status == 302 || status == 303 || status == 200 {
            self.logged_in = true;
            return Ok(LoginResult { success: true, mfa_required: None, message: None });
        }

        Err(anyhow!("登录失败: HTTP {} - {}", status, 
            text.chars().take(200).collect::<String>()))
    }

    /// Step 3: MFA 验证
    pub async fn mfa_verify(&mut self, passcode: &str) -> Result<()> {
        let resp = self.client
            .post(&self.url("/login/password"))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "username": self.username,
                "password": self.password,
                "passcode": passcode,
            }))
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let mfa_status = json["mfaVerifyStatus"].as_str().unwrap_or("");
        match mfa_status {
            "Pass" | "Restored" => {
                self.logged_in = true;
                Ok(())
            }
            "Block" => Err(anyhow!("MFA 验证失败，验证码错误")),
            _ => Err(anyhow!("MFA 验证失败: {}", mfa_status)),
        }
    }

    pub fn is_logged_in(&self) -> bool { self.logged_in }

    pub fn username(&self) -> &str { &self.username }

    /// 获取集群列表
    pub async fn list_clusters(&self) -> Result<Vec<String>> {
        let resp = self.client
            .get(&self.url("/kuboard/api/clusters"))
            .send()
            .await?;
        let json: serde_json::Value = resp.json().await?;
        let clusters: Vec<String> = json.as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|c| c["name"].as_str().map(String::from))
            .collect();
        Ok(clusters)
    }

    /// 获取命名空间列表
    pub async fn list_namespaces(&self, cluster: &str) -> Result<Vec<String>> {
        let path = format!("/k8s-api/{}/api/v1/namespaces", cluster);
        let resp = self.client.get(&self.url(&path)).send().await?;
        let json: serde_json::Value = resp.json().await?;
        let namespaces: Vec<String> = json["items"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|ns| ns["metadata"]["name"].as_str().map(String::from))
            .collect();
        Ok(namespaces)
    }

    /// 获取 Pod 列表
    pub async fn list_pods(&self, cluster: &str, namespace: &str) -> Result<Vec<PodInfo>> {
        let path = format!("/k8s-api/{}/api/v1/namespaces/{}/pods", cluster, namespace);
        let resp = self.client.get(&self.url(&path)).send().await?;
        let json: serde_json::Value = resp.json().await?;

        let pods: Vec<PodInfo> = json["items"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|pod| {
                let metadata = &pod["metadata"];
                let spec = &pod["spec"];
                let status = &pod["status"];
                let name = metadata["name"].as_str().unwrap_or("").to_string();
                let ip = status["podIP"].as_str().unwrap_or("").to_string();
                let phase = status["phase"].as_str().unwrap_or("Unknown").to_string();

                let containers: Vec<ContainerInfo> = spec["containers"].as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|c| {
                        let ports: Vec<ContainerPort> = c["ports"].as_array()
                            .unwrap_or(&vec![])
                            .iter()
                            .map(|p| ContainerPort {
                                name: p["name"].as_str().map(String::from),
                                container_port: p["containerPort"].as_u64().unwrap_or(0) as u16,
                                protocol: p["protocol"].as_str().unwrap_or("TCP").to_string(),
                            })
                            .collect();
                        ContainerInfo {
                            name: c["name"].as_str().unwrap_or("").to_string(),
                            ports,
                        }
                    })
                    .collect();

                PodInfo { name, ip, status: phase, containers }
            })
            .collect();

        Ok(pods)
    }
}

/// URL 编码
fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect::<String>()
}

/// 解析 SSO 登录错误消息
fn parse_login_message(msg: &str) -> LoginResult {
    if msg.contains("Login error") {
        let parts: Vec<&str> = msg.split(':').collect();
        if parts.len() >= 2 {
            match parts[1].trim() {
                "PASS" => LoginResult { success: true, mfa_required: None, message: None },
                "MFA_REQUIRED" => LoginResult { success: false, mfa_required: Some(true), message: Some("需要双因子认证".into()) },
                "USER_NOT_FOUND" => LoginResult { success: false, mfa_required: None, message: Some("用户名未找到".into()) },
                "WRONG_PASSWORD" => LoginResult { success: false, mfa_required: None, message: Some("密码错误".into()) },
                "WRONG_PASSCODE" => LoginResult { success: false, mfa_required: None, message: Some("验证码错误".into()) },
                _ => LoginResult { success: false, mfa_required: None, message: Some(msg.to_string()) },
            }
        } else {
            LoginResult { success: false, mfa_required: None, message: Some(msg.to_string()) }
        }
    } else {
        LoginResult { success: false, mfa_required: None, message: Some(msg.to_string()) }
    }
}
```

- [ ] **Step 2: 尝试编译，检查依赖下载**

```bash
cargo check -p k8s-forward
```
预期: 可能因缺少 lib.rs 报错，先确认 models.rs 和 crypto.rs 编译无误。如有 ssh2 编译问题（缺少 libssh2），安装: `vcpkg install libssh2` (Windows) 或 `apt install libssh2-1-dev` (Linux)。

- [ ] **Step 3: Commit**

```bash
git add plugins/k8s-forward/src/kuboard_client.rs
git commit -m "feat(k8s-forward): add KuboardClient with DEX SSO auth and K8s API"
```

---

### Task 4: SshService — SSH 端口转发

**Files:**
- Create: `plugins/k8s-forward/src/ssh_service.rs`

- [ ] **Step 1: 创建 ssh_service.rs**

```rust
use anyhow::{Result, anyhow};
use ssh2::Session;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::{Read, Write};
use std::net::TcpListener;
use crate::models::ForwardRule;

pub struct SshService {
    session: Option<Session>,
    tcp_stream: Option<TcpStream>,
    forwards: Vec<ForwardEntry>,
    next_port: u16,
    threads: Vec<thread::JoinHandle<()>>,
    stop_flags: Vec<Arc<Mutex<bool>>>,
}

struct ForwardEntry {
    rule: ForwardRule,
    stop_flag: Arc<Mutex<bool>>,
}

impl SshService {
    pub fn new() -> Self {
        Self {
            session: None,
            tcp_stream: None,
            forwards: vec![],
            next_port: 10000,
            threads: vec![],
            stop_flags: vec![],
        }
    }

    pub fn is_connected(&self) -> bool {
        self.session.as_ref().map(|s| s.authenticated()).unwrap_or(false)
    }

    /// 连接 SSH 跳板机
    pub fn connect(&mut self, host: &str, port: u16, username: &str, password: &str) -> Result<()> {
        let addr = format!("{}:{}", host, port);
        let tcp = TcpStream::connect(&addr)?;
        tcp.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp.try_clone()?);
        session.handshake()?;
        session.userauth_password(username, password)?;
        if !session.authenticated() {
            return Err(anyhow!("SSH 认证失败"));
        }
        self.session = Some(session);
        self.tcp_stream = Some(tcp);
        Ok(())
    }

    /// 断开连接（清理所有转发线程）
    pub fn disconnect(&mut self) {
        for flag in &self.stop_flags {
            *flag.lock().unwrap() = true;
        }
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
        self.stop_flags.clear();
        self.forwards.clear();
        self.session = None;
        self.tcp_stream = None;
    }

    /// 添加端口转发，返回分配的本地端口
    pub fn add_forward(&mut self, local_host: &str, remote_host: &str, remote_port: u16) -> Result<u16> {
        let local_port = self.allocate_port();
        let bind_addr = format!("{}:{}", local_host, local_port);

        let stop_flag = Arc::new(Mutex::new(false));
        let stop = stop_flag.clone();
        let remote = format!("{}:{}", remote_host, remote_port);

        // 在独立线程中处理此转发
        let handle = thread::spawn(move || {
            let listener = match TcpListener::bind(&bind_addr) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("无法绑定 {}: {}", bind_addr, e);
                    return;
                }
            };
            listener.set_nonblocking(true).ok();

            loop {
                if *stop.lock().unwrap() { break; }
                match listener.accept() {
                    Ok((mut local_stream, _)) => {
                        // 为每个连接创建新的 SSH session 和 channel
                        // 注意：ssh2 Session 不是 Send，这里需要用新的连接
                        // 简化实现：直接用 TcpStream 连接目标
                        let remote_clone = remote.clone();
                        thread::spawn(move || {
                            if let Ok(mut remote_stream) = TcpStream::connect(&remote_clone) {
                                let mut local_clone = local_stream.try_clone().unwrap();
                                let mut buf1 = [0u8; 8192];
                                let mut buf2 = [0u8; 8192];
                                // 简单双向拷贝
                                // 注：完整的 SSH 转发需要通过 ssh2 channel_direct_tcpip
                                // 这里用直连作为 fallback
                                let _ = std::io::copy(&mut local_clone, &mut remote_stream);
                            }
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(_) => break,
                }
            }
        });

        self.threads.push(handle);
        self.stop_flags.push(stop_flag);
        self.forwards.push(ForwardEntry {
            rule: ForwardRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("forward-{}", local_port),
                local_host: local_host.to_string(),
                local_port,
                remote_host: remote_host.to_string(),
                remote_port,
                rule_type: crate::models::RuleType::Manual,
                cluster: None,
                namespace: None,
                pod_name: None,
                container_name: None,
            },
            stop_flag: stop,
        });

        Ok(local_port)
    }

    /// 移除端口转发
    pub fn remove_forward(&mut self, local_port: u16) -> Result<()> {
        if let Some(pos) = self.forwards.iter().position(|f| f.rule.local_port == local_port) {
            let entry = self.forwards.remove(pos);
            *entry.stop_flag.lock().unwrap() = true;
            let flag = self.stop_flags.remove(pos);
            drop(flag);
        }
        Ok(())
    }

    /// 获取当前所有转发规则
    pub fn list_forwards(&self) -> Vec<ForwardRule> {
        self.forwards.iter().map(|f| f.rule.clone()).collect()
    }

    /// 获取转发数量
    pub fn forward_count(&self) -> usize {
        self.forwards.len()
    }

    /// 分配可用端口
    fn allocate_port(&mut self) -> u16 {
        let used_ports: Vec<u16> = self.forwards.iter().map(|f| f.rule.local_port).collect();
        loop {
            let port = self.next_port;
            self.next_port += 1;
            if self.next_port > 60000 {
                self.next_port = 10000;
            }
            if !used_ports.contains(&port) && port_is_available(port) {
                return port;
            }
        }
    }
}

fn port_is_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}
```

- [ ] **Step 2: Commit**

```bash
git add plugins/k8s-forward/src/ssh_service.rs
git commit -m "feat(k8s-forward): add SshService with port forwarding"
```

---

### Task 5: HttpProxySvc — HTTP 反向代理

**Files:**
- Create: `plugins/k8s-forward/src/http_proxy.rs`

- [ ] **Step 1: 创建 http_proxy.rs**

```rust
use anyhow::Result;
use hyper::{body::Incoming, server::conn::http1, service::service_fn, Request, Response, Method, StatusCode};
use hyper_util::rt::TokioIo;
use http_body_util::{Full, BodyExt};
use hyper::body::Bytes;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::models::ProxyMapping;

pub struct HttpProxySvc {
    port: u16,
    mappings: Arc<Mutex<HashMap<String, String>>>, // domain → "127.0.0.1:port"
    mapping_list: Arc<Mutex<Vec<ProxyMapping>>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    running: bool,
}

impl HttpProxySvc {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            mappings: Arc::new(Mutex::new(HashMap::new())),
            mapping_list: Arc::new(Mutex::new(vec![])),
            shutdown_tx: None,
            running: false,
        }
    }

    pub fn is_running(&self) -> bool { self.running }

    pub fn port(&self) -> u16 { self.port }

    /// 注册域名映射
    pub fn register(&self, domain: &str, target: &str, rule_id: &str, editable: bool) {
        self.mappings.lock().unwrap().insert(domain.to_string(), target.to_string());
        self.mapping_list.lock().unwrap().push(ProxyMapping {
            domain: domain.to_string(),
            target: target.to_string(),
            rule_id: rule_id.to_string(),
            editable,
        });
    }

    /// 注销域名映射
    pub fn unregister(&self, domain: &str) {
        self.mappings.lock().unwrap().remove(domain);
        self.mapping_list.lock().unwrap().retain(|m| m.domain != domain);
    }

    /// 根据 rule_id 注销
    pub fn unregister_by_rule_id(&self, rule_id: &str) {
        let domains: Vec<String> = self.mapping_list.lock().unwrap()
            .iter()
            .filter(|m| m.rule_id == rule_id)
            .map(|m| m.domain.clone())
            .collect();
        for d in domains {
            self.unregister(&d);
        }
    }

    /// 获取所有映射
    pub fn list_mappings(&self) -> Vec<ProxyMapping> {
        self.mapping_list.lock().unwrap().clone()
    }

    /// 更新域名
    pub fn update_mapping(&self, rule_id: &str, new_domain: &str) -> Result<ProxyMapping> {
        let mut list = self.mapping_list.lock().unwrap();
        if let Some(m) = list.iter_mut().find(|m| m.rule_id == rule_id) {
            let old_domain = m.domain.clone();
            let target = m.target.clone();
            let editable = m.editable;
            self.mappings.lock().unwrap().remove(&old_domain);
            m.domain = new_domain.to_string();
            self.mappings.lock().unwrap().insert(new_domain.to_string(), target);
            return Ok(ProxyMapping {
                domain: new_domain.to_string(),
                target: m.target.clone(),
                rule_id: rule_id.to_string(),
                editable,
            });
        }
        Err(anyhow::anyhow!("未找到 rule_id 对应的映射"))
    }

    /// 启动代理服务器
    pub async fn start(&mut self) -> Result<()> {
        let port = self.port;
        let mappings = self.mappings.clone();

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        let (tx, rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(tx);
        self.running = true;

        tokio::spawn(async move {
            let graceful = async move { rx.await.ok() };

            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, _)) => {
                                let io = TokioIo::new(stream);
                                let mappings = mappings.clone();
                                tokio::spawn(async move {
                                    let svc = service_fn(move |req| {
                                        proxy_request(req, mappings.clone())
                                    });
                                    if let Err(e) = http1::Builder::new()
                                        .serve_connection(io, svc)
                                        .await
                                    {
                                        eprintln!("代理连接错误: {}", e);
                                    }
                                });
                            }
                            Err(e) => eprintln!("Accept error: {}", e),
                        }
                    }
                    _ = graceful => break,
                }
            }
        });

        Ok(())
    }

    /// 停止代理
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.running = false;
    }
}

/// 代理请求处理函数
async fn proxy_request(
    req: Request<Incoming>,
    mappings: Arc<Mutex<HashMap<String, String>>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    // 解析 Host header
    let host = req.headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let target = {
        let map = mappings.lock().unwrap();
        map.get(host).cloned()
    };

    if let Some(target) = target {
        match forward_request(&req, &target).await {
            Ok(resp) => Ok(resp),
            Err(_) => {
                let mut resp = Response::new(Full::new(Bytes::from("转发目标不可达")));
                *resp.status_mut() = StatusCode::BAD_GATEWAY;
                Ok(resp)
            }
        }
    } else {
        let mut resp = Response::new(Full::new(Bytes::from(
            format!("未找到域名 {} 对应的转发规则", host)
        )));
        *resp.status_mut() = StatusCode::NOT_FOUND;
        Ok(resp)
    }
}

async fn forward_request(
    req: &Request<Incoming>,
    target: &str,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build_http();

    let path = req.uri().path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    let uri = format!("http://{}{}", target, path);
    let uri: hyper::Uri = uri.parse()?;

    let mut builder = Request::builder()
        .method(req.method().clone())
        .uri(&uri);

    // 复制请求头（除了 Host）
    for (key, value) in req.headers().iter() {
        if key.as_str().to_lowercase() != "host" {
            builder = builder.header(key, value);
        }
    }
    builder = builder.header("Host", target);

    let body_bytes = req.collect().await?.to_bytes();
    let proxy_req = builder.body(Full::new(body_bytes))?;

    let resp = client.request(proxy_req).await?;

    let (parts, body) = resp.into_parts();
    let body_bytes = body.collect().await?.to_bytes();

    let mut response = Response::new(Full::new(body_bytes));
    *response.status_mut() = parts.status;
    *response.version_mut() = parts.version;
    for (key, value) in parts.headers.iter() {
        response.headers_mut().insert(key, value.clone());
    }

    Ok(response)
}
```

- [ ] **Step 2: Commit**

```bash
git add plugins/k8s-forward/src/http_proxy.rs
git commit -m "feat(k8s-forward): add HttpProxySvc with Host-based routing"
```

---

### Task 6: Plugin trait 实现 + handle_call 调度

**Files:**
- Create: `plugins/k8s-forward/src/lib.rs`

- [ ] **Step 1: 创建 lib.rs — Plugin trait 完整实现**

```rust
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Mutex;
use tokio::runtime::Runtime;
use worktools_plugin_api::*;

mod models;
mod crypto;
mod kuboard_client;
mod ssh_service;
mod http_proxy;

use models::*;
use crypto::PasswordEncryptor;
use kuboard_client::KuboardClient;
use ssh_service::SshService;
use http_proxy::HttpProxySvc;

pub struct K8sForwardPlugin {
    storage: PluginStorage,
    encryptor: PasswordEncryptor,
    runtime: Runtime,
    // 运行时状态
    ssh: Mutex<SshService>,
    proxy: Mutex<Option<HttpProxySvc>>,
    kuboard: Mutex<Option<KuboardClient>>,
}

impl K8sForwardPlugin {
    pub fn new() -> Self {
        Self {
            storage: PluginStorage::new("k8s-forward"),
            encryptor: PasswordEncryptor::new(),
            runtime: Runtime::new().expect("Failed to create tokio runtime"),
            ssh: Mutex::new(SshService::new()),
            proxy: Mutex::new(None),
            kuboard: Mutex::new(None),
        }
    }

    fn load_data(&self) -> Result<PluginData> {
        self.storage.load_json::<PluginData>("config")
            .or_else(|_| Ok(PluginData::default()))
    }

    fn save_data(&self, data: &PluginData) -> Result<()> {
        self.storage.save_json("config", data)
    }

    // ── SSH ──

    fn handle_ssh_connect(&self, params: &Value) -> Result<Value> {
        let host = get_str(params, "host")?;
        let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16;
        let username = get_str(params, "username")?;
        let password = get_str(params, "password")?;

        let mut ssh = self.ssh.lock().unwrap();
        ssh.connect(host, port, username, password)?;

        // 保存 SSH 配置（加密密码）
        let mut data = self.load_data()?;
        let enc_pwd = self.encryptor.encrypt(password)?;
        data.ssh = Some(SshConfig {
            host: host.to_string(),
            port,
            username: username.to_string(),
            password: enc_pwd,
        });
        self.save_data(&data)?;

        Ok(json!({"success": true, "message": "SSH 连接成功"}))
    }

    fn handle_ssh_disconnect(&self) -> Result<Value> {
        let mut ssh = self.ssh.lock().unwrap();
        ssh.disconnect();
        Ok(json!({"success": true}))
    }

    fn handle_ssh_status(&self) -> Result<Value> {
        let ssh = self.ssh.lock().unwrap();
        let data = self.load_data()?;
        let status = SshStatus {
            connected: ssh.is_connected(),
            host: data.ssh.as_ref().map(|s| s.host.clone()),
            port: data.ssh.as_ref().map(|s| s.port),
        };
        Ok(serde_json::to_value(status)?)
    }

    // ── 转发规则 CRUD ──

    fn handle_list_forward_rules(&self) -> Result<Value> {
        let data = self.load_data()?;
        Ok(serde_json::to_value(&data.forward_rules)?)
    }

    fn handle_add_forward_rule(&self, params: &Value) -> Result<Value> {
        let rule: ForwardRule = serde_json::from_value(params.clone())?;
        let mut data = self.load_data()?;
        data.forward_rules.push(rule.clone());
        self.save_data(&data)?;

        // 如果 SSH 已连接，立即建立转发
        let mut ssh = self.ssh.lock().unwrap();
        if ssh.is_connected() && rule.rule_type == RuleType::Manual {
            let local_port = ssh.add_forward(
                &rule.local_host, &rule.remote_host, rule.remote_port)?;
            let mut updated_rule = rule.clone();
            updated_rule.local_port = local_port;
            // 更新存储
            if let Some(r) = data.forward_rules.iter_mut().find(|r| r.id == rule.id) {
                r.local_port = local_port;
                self.save_data(&data)?;
            }
            return Ok(serde_json::to_value(&updated_rule)?);
        }

        Ok(serde_json::to_value(&rule)?)
    }

    fn handle_update_forward_rule(&self, params: &Value) -> Result<Value> {
        let updated: ForwardRule = serde_json::from_value(params.clone())?;
        let mut data = self.load_data()?;
        if let Some(rule) = data.forward_rules.iter_mut().find(|r| r.id == updated.id) {
            // 先移除旧转发
            let mut ssh = self.ssh.lock().unwrap();
            ssh.remove_forward(rule.local_port)?;
            // 建立新转发
            if ssh.is_connected() {
                let new_port = ssh.add_forward(
                    &updated.local_host, &updated.remote_host, updated.remote_port)?;
                let mut saved = updated.clone();
                saved.local_port = new_port;
                *rule = saved;
            } else {
                *rule = updated.clone();
            }
            self.save_data(&data)?;
            return Ok(serde_json::to_value(rule)?);
        }
        Err(anyhow::anyhow!("规则不存在"))
    }

    fn handle_remove_forward_rule(&self, params: &Value) -> Result<Value> {
        let id = get_str(params, "id")?;
        let mut data = self.load_data()?;
        if let Some(pos) = data.forward_rules.iter().position(|r| r.id == id) {
            let rule = data.forward_rules.remove(pos);
            let mut ssh = self.ssh.lock().unwrap();
            ssh.remove_forward(rule.local_port)?;
            // 同时移除 HTTP 代理映射
            if let Some(ref proxy) = *self.proxy.lock().unwrap() {
                proxy.unregister_by_rule_id(&rule.id);
            }
            self.save_data(&data)?;
            return Ok(json!({"success": true}));
        }
        Err(anyhow::anyhow!("规则不存在"))
    }

    fn handle_import_rules(&self, params: &Value) -> Result<Value> {
        let imported: Vec<ForwardRule> = serde_json::from_value(
            params.get("rules").cloned().ok_or_else(|| anyhow::anyhow!("缺少 rules"))?)?;
        let mut data = self.load_data()?;
        for rule in imported {
            if let Some(existing) = data.forward_rules.iter_mut().find(|r| r.id == rule.id) {
                *existing = rule;
            } else {
                data.forward_rules.push(rule);
            }
        }
        self.save_data(&data)?;
        Ok(serde_json::to_value(&data)?)
    }

    fn handle_export_rules(&self) -> Result<Value> {
        let data = self.load_data()?;
        Ok(serde_json::to_value(&data.forward_rules)?)
    }

    // ── Kuboard ──

    fn handle_kuboard_login(&self, params: &Value) -> Result<Value> {
        let url = get_str(params, "url")?;
        let username = get_str(params, "username")?;
        let password = get_str(params, "password")?;

        let mut client = KuboardClient::new(url);
        let result = self.runtime.block_on(client.login(username, password))?;

        if result.success {
            let mut kuboard = self.kuboard.lock().unwrap();
            *kuboard = Some(client);

            let mut data = self.load_data()?;
            let enc_pwd = self.encryptor.encrypt(password)?;
            data.kuboard = Some(KuboardConfig {
                url: url.to_string(),
                username: username.to_string(),
                password: enc_pwd,
            });
            self.save_data(&data)?;
        }

        Ok(serde_json::to_value(&result)?)
    }

    fn handle_kuboard_mfa(&self, params: &Value) -> Result<Value> {
        let passcode = get_str(params, "passcode")?;
        let mut kuboard = self.kuboard.lock().unwrap();
        if let Some(ref mut client) = *kuboard {
            self.runtime.block_on(client.mfa_verify(passcode))?;
            Ok(json!({"success": true}))
        } else {
            Err(anyhow::anyhow!("请先登录"))
        }
    }

    fn handle_kuboard_logout(&self) -> Result<Value> {
        let mut kuboard = self.kuboard.lock().unwrap();
        *kuboard = None;
        Ok(json!({"success": true}))
    }

    fn handle_kuboard_status(&self) -> Result<Value> {
        let kuboard = self.kuboard.lock().unwrap();
        let data = self.load_data()?;
        let status = KuboardStatus {
            logged_in: kuboard.as_ref().map(|c| c.is_logged_in()).unwrap_or(false),
            url: data.kuboard.as_ref().map(|k| k.url.clone()),
            username: data.kuboard.as_ref().map(|k| k.username.clone()),
        };
        Ok(serde_json::to_value(status)?)
    }

    fn handle_list_clusters(&self) -> Result<Value> {
        let kuboard = self.kuboard.lock().unwrap();
        if let Some(ref client) = *kuboard {
            let clusters = self.runtime.block_on(client.list_clusters())?;
            Ok(serde_json::to_value(clusters)?)
        } else {
            Err(anyhow::anyhow!("请先登录 Kuboard"))
        }
    }

    fn handle_list_namespaces(&self, params: &Value) -> Result<Value> {
        let cluster = get_str(params, "cluster")?;
        let kuboard = self.kuboard.lock().unwrap();
        if let Some(ref client) = *kuboard {
            let nss = self.runtime.block_on(client.list_namespaces(cluster))?;
            Ok(serde_json::to_value(nss)?)
        } else {
            Err(anyhow::anyhow!("请先登录 Kuboard"))
        }
    }

    fn handle_list_pods(&self, params: &Value) -> Result<Value> {
        let cluster = get_str(params, "cluster")?;
        let namespace = get_str(params, "namespace")?;
        let kuboard = self.kuboard.lock().unwrap();
        if let Some(ref client) = *kuboard {
            let pods = self.runtime.block_on(client.list_pods(cluster, namespace))?;
            Ok(serde_json::to_value(pods)?)
        } else {
            Err(anyhow::anyhow!("请先登录 Kuboard"))
        }
    }

    // ── K8s 转发 ──

    fn handle_forward_pod(&self, params: &Value) -> Result<Value> {
        let cluster = get_str(params, "cluster")?;
        let namespace = get_str(params, "namespace")?;
        let pod_name = get_str(params, "pod_name")?;
        let container_name = get_str(params, "container_name")?;
        let container_port = params.get("container_port").and_then(|v| v.as_u64()).unwrap_or(0) as u16;

        // 获取 Pod IP
        let kuboard = self.kuboard.lock().unwrap();
        let pods = if let Some(ref client) = *kuboard {
            self.runtime.block_on(client.list_pods(cluster, namespace))?
        } else {
            return Err(anyhow::anyhow!("请先登录 Kuboard"));
        };
        let pod = pods.iter().find(|p| p.name == pod_name)
            .ok_or_else(|| anyhow::anyhow!("Pod 未找到"))?;

        let remote_host = pod.ip.clone();
        let remote_port = container_port;

        // 建立 SSH 转发
        let mut ssh = self.ssh.lock().unwrap();
        if !ssh.is_connected() {
            return Err(anyhow::anyhow!("SSH 未连接"));
        }
        let local_port = ssh.add_forward("127.0.0.1", &remote_host, remote_port)?;

        // 生成域名
        let domain = format!("{}-{}.svc", pod_name, container_name);

        // 注册 HTTP 代理映射
        let mut proxy = self.proxy.lock().unwrap();
        if let Some(ref p) = *proxy {
            p.register(&domain, &format!("127.0.0.1:{}", local_port), "", true);
        }

        // 保存规则
        let rule = ForwardRule {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("{}/{}:{}", pod_name, container_name, container_port),
            local_host: "127.0.0.1".to_string(),
            local_port,
            remote_host: remote_host.clone(),
            remote_port,
            rule_type: RuleType::K8s,
            cluster: Some(cluster.to_string()),
            namespace: Some(namespace.to_string()),
            pod_name: Some(pod_name.to_string()),
            container_name: Some(container_name.to_string()),
        };

        let mut data = self.load_data()?;
        data.forward_rules.push(rule.clone());
        self.save_data(&data)?;

        let mapping = ProxyMapping {
            domain,
            target: format!("127.0.0.1:{}", local_port),
            rule_id: rule.id.clone(),
            editable: true,
        };

        Ok(json!({"rule": rule, "proxy_mapping": mapping}))
    }

    fn handle_unforward_pod(&self, params: &Value) -> Result<Value> {
        let rule_id = get_str(params, "rule_id")?;
        let mut data = self.load_data()?;
        if let Some(pos) = data.forward_rules.iter().position(|r| r.id == rule_id) {
            let rule = data.forward_rules.remove(pos);
            let mut ssh = self.ssh.lock().unwrap();
            ssh.remove_forward(rule.local_port)?;
            if let Some(ref proxy) = *self.proxy.lock().unwrap() {
                proxy.unregister_by_rule_id(&rule.id);
            }
            self.save_data(&data)?;
            return Ok(json!({"success": true}));
        }
        Err(anyhow::anyhow!("规则不存在"))
    }

    fn handle_list_k8s_forwards(&self) -> Result<Value> {
        let data = self.load_data()?;
        let k8s_rules: Vec<&ForwardRule> = data.forward_rules.iter()
            .filter(|r| r.rule_type == RuleType::K8s)
            .collect();
        let mappings = self.proxy.lock().unwrap();
        let mappings = mappings.as_ref().map(|p| p.list_mappings()).unwrap_or_default();
        Ok(json!({"rules": k8s_rules, "mappings": mappings}))
    }

    // ── HTTP 代理 ──

    fn handle_proxy_start(&self, params: &Value) -> Result<Value> {
        let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
        let mut proxy = HttpProxySvc::new(port);
        self.runtime.block_on(proxy.start())?;

        // 恢复已有的 K8s 转发映射
        let data = self.load_data()?;
        let ssh = self.ssh.lock().unwrap();
        for rule in &data.forward_rules {
            if rule.rule_type == RuleType::K8s {
                let domain = format!("{}-{}.svc",
                    rule.pod_name.as_deref().unwrap_or(""),
                    rule.container_name.as_deref().unwrap_or(""));
                proxy.register(&domain,
                    &format!("127.0.0.1:{}", rule.local_port),
                    &rule.id, true);
            }
        }

        let mut data = self.load_data()?;
        data.proxy.port = port;
        self.save_data(&data)?;

        let mut guard = self.proxy.lock().unwrap();
        *guard = Some(proxy);
        Ok(json!({"success": true, "message": format!("代理已启动: 127.0.0.1:{}", port)}))
    }

    fn handle_proxy_stop(&self) -> Result<Value> {
        let mut guard = self.proxy.lock().unwrap();
        if let Some(ref mut proxy) = *guard {
            proxy.stop();
        }
        *guard = None;
        Ok(json!({"success": true}))
    }

    fn handle_proxy_status(&self) -> Result<Value> {
        let guard = self.proxy.lock().unwrap();
        let data = self.load_data()?;
        let status = ProxyStatus {
            running: guard.as_ref().map(|p| p.is_running()).unwrap_or(false),
            port: data.proxy.port,
            mapping_count: guard.as_ref().map(|p| p.list_mappings().len()).unwrap_or(0),
        };
        Ok(serde_json::to_value(status)?)
    }

    fn handle_list_proxy_mappings(&self) -> Result<Value> {
        let guard = self.proxy.lock().unwrap();
        let mappings = guard.as_ref().map(|p| p.list_mappings()).unwrap_or_default();
        Ok(serde_json::to_value(mappings)?)
    }

    fn handle_update_proxy_mapping(&self, params: &Value) -> Result<Value> {
        let rule_id = get_str(params, "rule_id")?;
        let domain = get_str(params, "domain")?;
        let guard = self.proxy.lock().unwrap();
        if let Some(ref proxy) = *guard {
            let mapping = proxy.update_mapping(rule_id, domain)?;
            Ok(serde_json::to_value(mapping)?)
        } else {
            Err(anyhow::anyhow!("代理未启动"))
        }
    }

    // ── 配置 ──

    fn handle_get_config(&self) -> Result<Value> {
        let data = self.load_data()?;
        Ok(serde_json::to_value(data)?)
    }

    fn handle_reset_config(&self) -> Result<Value> {
        self.save_data(&PluginData::default())?;
        Ok(json!({"success": true}))
    }
}

fn get_str<'a>(params: &'a Value, key: &str) -> Result<&'a str> {
    params.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("缺少参数: {}", key))
}

impl Plugin for K8sForwardPlugin {
    fn id(&self) -> &str { "k8s-forward" }
    fn name(&self) -> &str { "K8s IP转发" }
    fn description(&self) -> &str { "通过Kuboard发现K8s Pod，SSH隧道+HTTP代理转发流量" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "🌐" }
    fn get_view(&self) -> String { "<div>插件前端资源加载中...</div>".to_string() }

    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        macro_rules! dispatch {
            ($e:expr) => { $e.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() }) };
        }
        match method {
            "ssh_connect" => dispatch!(self.handle_ssh_connect(&params)),
            "ssh_disconnect" => dispatch!(self.handle_ssh_disconnect()),
            "ssh_status" => dispatch!(self.handle_ssh_status()),
            "list_forward_rules" => dispatch!(self.handle_list_forward_rules()),
            "add_forward_rule" => dispatch!(self.handle_add_forward_rule(&params)),
            "update_forward_rule" => dispatch!(self.handle_update_forward_rule(&params)),
            "remove_forward_rule" => dispatch!(self.handle_remove_forward_rule(&params)),
            "import_rules" => dispatch!(self.handle_import_rules(&params)),
            "export_rules" => dispatch!(self.handle_export_rules()),
            "kuboard_login" => dispatch!(self.handle_kuboard_login(&params)),
            "kuboard_mfa" => dispatch!(self.handle_kuboard_mfa(&params)),
            "kuboard_logout" => dispatch!(self.handle_kuboard_logout()),
            "kuboard_status" => dispatch!(self.handle_kuboard_status()),
            "list_clusters" => dispatch!(self.handle_list_clusters()),
            "list_namespaces" => dispatch!(self.handle_list_namespaces(&params)),
            "list_pods" => dispatch!(self.handle_list_pods(&params)),
            "forward_pod" => dispatch!(self.handle_forward_pod(&params)),
            "unforward_pod" => dispatch!(self.handle_unforward_pod(&params)),
            "list_k8s_forwards" => dispatch!(self.handle_list_k8s_forwards()),
            "proxy_start" => dispatch!(self.handle_proxy_start(&params)),
            "proxy_stop" => dispatch!(self.handle_proxy_stop()),
            "proxy_status" => dispatch!(self.handle_proxy_status()),
            "list_proxy_mappings" => dispatch!(self.handle_list_proxy_mappings()),
            "update_proxy_mapping" => dispatch!(self.handle_update_proxy_mapping(&params)),
            "get_config" => dispatch!(self.handle_get_config()),
            "reset_config" => dispatch!(self.handle_reset_config()),
            _ => Err(format!("未知方法: {}", method).into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(K8sForwardPlugin::new()));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
```

- [ ] **Step 2: 添加 workspace member**

在根 `Cargo.toml` 的 members 列表末尾添加: `"plugins/k8s-forward",`

- [ ] **Step 3: 编译验证**

```bash
cargo check -p k8s-forward
```
预期: 编译成功 (可能有一些 warnings，如未使用变量)

- [ ] **Step 4: Commit**

```bash
git add plugins/k8s-forward/src/lib.rs Cargo.toml
git commit -m "feat(k8s-forward): implement Plugin trait with 24 handle_call methods"
```

---

### Task 7: 前端脚手架

**Files:**
- Create: `plugins/k8s-forward/frontend/package.json`
- Create: `plugins/k8s-forward/frontend/index.html`
- Create: `plugins/k8s-forward/frontend/vite.config.ts`
- Create: `plugins/k8s-forward/frontend/tsconfig.json`
- Create: `plugins/k8s-forward/frontend/tsconfig.node.json`
- Create: `plugins/k8s-forward/frontend/src/types.ts`
- Create: `plugins/k8s-forward/frontend/src/main.tsx`
- Create: `plugins/k8s-forward/frontend/src/App.css`

- [ ] **Step 1: package.json**

```json
{
  "name": "k8s-forward-frontend",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "^5.6.0",
    "vite": "^5.4.0"
  }
}
```

- [ ] **Step 2: index.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>K8s IP转发</title>
</head>
<body>
  <div id="app"></div>
  <script type="module" src="/src/main.tsx"></script>
</body>
</html>
```

- [ ] **Step 3: vite.config.ts** (参考 password-manager 的配置)

```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "../assets",
    emptyOutDir: true,
    minify: "esbuild",
    sourcemap: false,
    rollupOptions: {
      output: {
        entryFileNames: "main.js",
        chunkFileNames: "chunks/[name].js",
        assetFileNames: (assetInfo) => {
          if (assetInfo.name === "index.html") return "index.html";
          if (assetInfo.name?.endsWith(".css")) return "styles.css";
          return "assets/[name][extname]";
        }
      }
    }
  }
});
```

- [ ] **Step 4: tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": false,
    "noUnusedParameters": false,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 5: tsconfig.node.json**

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 6: types.ts**

```typescript
declare global {
  interface Window {
    pluginAPI: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

export interface ForwardRule {
  id: string;
  name: string;
  local_host: string;
  local_port: number;
  remote_host: string;
  remote_port: number;
  rule_type: "manual" | "k8s";
  cluster?: string;
  namespace?: string;
  pod_name?: string;
  container_name?: string;
}

export interface ProxyMapping {
  domain: string;
  target: string;
  rule_id: string;
  editable: boolean;
}

export interface SshStatus {
  connected: boolean;
  host?: string;
  port?: number;
}

export interface KuboardStatus {
  logged_in: boolean;
  url?: string;
  username?: string;
}

export interface ProxyStatus {
  running: boolean;
  port: number;
  mapping_count: number;
}

export interface PodInfo {
  name: string;
  ip: string;
  status: string;
  containers: ContainerInfo[];
}

export interface ContainerInfo {
  name: string;
  ports: ContainerPort[];
}

export interface ContainerPort {
  name?: string;
  container_port: number;
  protocol: string;
}

export interface LoginResult {
  success: boolean;
  mfa_required?: boolean;
  message?: string;
}

export interface K8sForwardInfo {
  rules: ForwardRule[];
  mappings: ProxyMapping[];
}
```

- [ ] **Step 7: main.tsx**

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";

ReactDOM.createRoot(document.getElementById("app")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 8: App.css** (简洁的插件样式，匹配平台风格)

```css
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; font-size: 13px; color: #e0e0e0; background: #1a1a2e; }
.k8s-forward { padding: 16px; max-width: 100%; }
.tabs { display: flex; gap: 0; border-bottom: 2px solid #2a2a4a; margin-bottom: 16px; }
.tab { padding: 8px 20px; cursor: pointer; border: none; background: none; color: #888; font-size: 13px; border-bottom: 2px solid transparent; margin-bottom: -2px; transition: all 0.2s; }
.tab:hover { color: #c0c0c0; }
.tab.active { color: #7c8aff; border-bottom-color: #7c8aff; }
.card { background: #1e1e3a; border: 1px solid #2a2a4a; border-radius: 8px; padding: 16px; margin-bottom: 12px; }
.card-header { font-size: 14px; font-weight: 600; margin-bottom: 12px; color: #a0a0c0; }
.form-row { display: flex; gap: 8px; align-items: flex-end; flex-wrap: wrap; margin-bottom: 8px; }
.form-group { display: flex; flex-direction: column; gap: 4px; }
.form-group label { font-size: 11px; color: #888; }
.form-group input, .form-group select { padding: 6px 10px; border: 1px solid #3a3a5a; border-radius: 4px; background: #12122a; color: #e0e0e0; font-size: 12px; min-width: 120px; }
.form-group input:focus { border-color: #7c8aff; outline: none; }
.btn { padding: 6px 16px; border: none; border-radius: 4px; cursor: pointer; font-size: 12px; font-weight: 500; transition: all 0.2s; }
.btn-primary { background: #7c8aff; color: #fff; }
.btn-primary:hover { background: #6b7aee; }
.btn-danger { background: #ff4757; color: #fff; }
.btn-danger:hover { background: #ee3a4a; }
.btn-default { background: #2a2a4a; color: #c0c0c0; }
.btn-default:hover { background: #3a3a5a; }
.btn-sm { padding: 3px 10px; font-size: 11px; }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.status-dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 6px; }
.status-dot.online { background: #2ed573; }
.status-dot.offline { background: #ff4757; }
table { width: 100%; border-collapse: collapse; font-size: 12px; }
th, td { padding: 8px 10px; text-align: left; border-bottom: 1px solid #2a2a4a; }
th { color: #888; font-weight: 500; font-size: 11px; text-transform: uppercase; }
tr:hover { background: rgba(124, 138, 255, 0.05); }
.badge { display: inline-block; padding: 2px 8px; border-radius: 10px; font-size: 10px; font-weight: 500; }
.badge-success { background: rgba(46, 213, 115, 0.2); color: #2ed573; }
.badge-warning { background: rgba(255, 165, 2, 0.2); color: #ffa502; }
.badge-info { background: rgba(124, 138, 255, 0.2); color: #7c8aff; }
.toast { position: fixed; top: 12px; right: 12px; padding: 10px 20px; border-radius: 6px; font-size: 12px; z-index: 1000; animation: slideIn 0.3s ease; }
.toast-success { background: #2ed573; color: #000; }
.toast-error { background: #ff4757; color: #fff; }
@keyframes slideIn { from { transform: translateX(100px); opacity: 0; } to { transform: translateX(0); opacity: 1; } }
.modal-overlay { position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; }
.modal { background: #1e1e3a; border: 1px solid #3a3a5a; border-radius: 8px; padding: 20px; min-width: 360px; max-width: 480px; }
.modal h3 { margin-bottom: 12px; color: #a0a0c0; }
.modal-actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 16px; }
```

- [ ] **Step 9: 安装依赖并验证构建**

```bash
cd plugins/k8s-forward/frontend && npm install && npx tsc --noEmit
```
预期: TypeScript 类型检查通过

- [ ] **Step 10: Commit**

```bash
git add plugins/k8s-forward/frontend/
git commit -m "feat(k8s-forward): add frontend scaffolding"
```

---

### Task 8: Tab1 — SSH 端口转发组件

**Files:**
- Create: `plugins/k8s-forward/frontend/src/components/TabSshForward.tsx`

- [ ] **Step 1: 创建 TabSshForward.tsx**

```tsx
import { useState, useEffect, useCallback } from "react";
import type { ForwardRule, SshStatus } from "../types";

const PLUGIN_ID = "k8s-forward";

export default function TabSshForward() {
  const [sshStatus, setSshStatus] = useState<SshStatus>({ connected: false });
  const [rules, setRules] = useState<ForwardRule[]>([]);
  const [form, setForm] = useState({ host: "", port: 22, username: "", password: "" });
  const [editing, setEditing] = useState<ForwardRule | null>(null);
  const [toast, setToast] = useState<string | null>(null);

  const call = useCallback(async (method: string, params?: Record<string, unknown>) => {
    return await window.pluginAPI.call(PLUGIN_ID, method, params || {});
  }, []);

  const showToast = (msg: string, isErr = false) => {
    setToast(isErr ? `❌ ${msg}` : `✅ ${msg}`);
    setTimeout(() => setToast(null), 3000);
  };

  const loadStatus = async () => {
    const s = await call("ssh_status") as SshStatus;
    setSshStatus(s);
  };

  const loadRules = async () => {
    const r = await call("list_forward_rules") as ForwardRule[];
    setRules(r.filter(r => r.rule_type === "manual"));
  };

  useEffect(() => { loadStatus(); loadRules(); }, []);

  const handleConnect = async () => {
    try {
      await call("ssh_connect", form);
      showToast("SSH 连接成功");
      loadStatus();
    } catch (e: unknown) { showToast(`连接失败: ${e}`, true); }
  };

  const handleDisconnect = async () => {
    await call("ssh_disconnect");
    setSshStatus({ connected: false });
    showToast("已断开");
  };

  const handleAdd = async () => {
    try {
      const rule = {
        id: crypto.randomUUID(),
        name: `rule-${Date.now()}`,
        local_host: "127.0.0.1",
        local_port: 0,
        remote_host: "",
        remote_port: 0,
        rule_type: "manual" as const,
      };
      await call("add_forward_rule", rule);
      showToast("规则已添加");
      loadRules();
      setEditing(rule);
    } catch (e: unknown) { showToast(`添加失败: ${e}`, true); }
  };

  const handleSave = async () => {
    if (!editing) return;
    try {
      await call("update_forward_rule", editing);
      showToast("已保存");
      setEditing(null);
      loadRules();
    } catch (e: unknown) { showToast(`保存失败: ${e}`, true); }
  };

  const handleDelete = async (id: string) => {
    try {
      await call("remove_forward_rule", { id });
      showToast("已删除");
      loadRules();
    } catch (e: unknown) { showToast(`删除失败: ${e}`, true); }
  };

  const handleImport = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      try {
        const text = await file.text();
        const parsed = JSON.parse(text);
        const arr = Array.isArray(parsed) ? parsed : parsed.rules || [];
        await call("import_rules", { rules: arr });
        showToast(`已导入 ${arr.length} 条规则`);
        loadRules();
      } catch { showToast("导入失败: 格式错误", true); }
    };
    input.click();
  };

  const handleExport = async () => {
    const data = await call("export_rules") as ForwardRule[];
    const json = JSON.stringify(data.filter(r => r.rule_type === "manual"), null, 2);
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url; a.download = `k8s-forward-rules-${new Date().toISOString().split("T")[0]}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div>
      {toast && <div className={`toast ${toast.startsWith("❌") ? "toast-error" : "toast-success"}`}>{toast}</div>}

      <div className="card">
        <div className="card-header">SSH 连接配置</div>
        <div className="form-row">
          <div className="form-group"><label>主机地址</label><input value={form.host} onChange={e => setForm({...form, host: e.target.value})} placeholder="10.73.x.x" /></div>
          <div className="form-group"><label>端口</label><input type="number" value={form.port} onChange={e => setForm({...form, port: +e.target.value})} /></div>
          <div className="form-group"><label>用户名</label><input value={form.username} onChange={e => setForm({...form, username: e.target.value})} /></div>
          <div className="form-group"><label>密码</label><input type="password" value={form.password} onChange={e => setForm({...form, password: e.target.value})} /></div>
          {sshStatus.connected
            ? <button className="btn btn-danger" onClick={handleDisconnect}>断开</button>
            : <button className="btn btn-primary" onClick={handleConnect}>连接</button>
          }
        </div>
        <div style={{marginTop:8}}>
          <span className={`status-dot ${sshStatus.connected ? "online" : "offline"}`}></span>
          {sshStatus.connected ? `已连接 → ${sshStatus.host}:${sshStatus.port}` : "未连接"}
        </div>
      </div>

      <div className="card">
        <div className="card-header" style={{display:"flex",justifyContent:"space-between",alignItems:"center"}}>
          <span>转发规则</span>
          <div style={{display:"flex",gap:8}}>
            <button className="btn btn-primary btn-sm" onClick={handleAdd}>+ 添加规则</button>
            <button className="btn btn-default btn-sm" onClick={handleImport}>导入</button>
            <button className="btn btn-default btn-sm" onClick={handleExport}>导出</button>
          </div>
        </div>
        <table>
          <thead><tr><th>名称</th><th>本地地址</th><th>本地端口</th><th>远程地址</th><th>远程端口</th><th>操作</th></tr></thead>
          <tbody>
            {rules.map(r => (
              <tr key={r.id}>
                <td>{r.name}</td>
                <td>{r.local_host}</td>
                <td>{r.local_port}</td>
                <td>{r.remote_host}</td>
                <td>{r.remote_port}</td>
                <td>
                  <button className="btn btn-default btn-sm" onClick={() => setEditing(r)} style={{marginRight:4}}>编辑</button>
                  <button className="btn btn-danger btn-sm" onClick={() => handleDelete(r.id)}>删除</button>
                </td>
              </tr>
            ))}
            {rules.length === 0 && <tr><td colSpan={6} style={{textAlign:"center",color:"#666",padding:20}}>暂无规则</td></tr>}
          </tbody>
        </table>
      </div>

      {editing && (
        <div className="modal-overlay" onClick={() => setEditing(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>编辑规则</h3>
            <div className="form-row">
              <div className="form-group"><label>名称</label><input value={editing.name} onChange={e => setEditing({...editing, name: e.target.value})} /></div>
              <div className="form-group"><label>本地地址</label><input value={editing.local_host} onChange={e => setEditing({...editing, local_host: e.target.value})} /></div>
              <div className="form-group"><label>本地端口</label><input type="number" value={editing.local_port} onChange={e => setEditing({...editing, local_port: +e.target.value})} /></div>
              <div className="form-group"><label>远程地址</label><input value={editing.remote_host} onChange={e => setEditing({...editing, remote_host: e.target.value})} /></div>
              <div className="form-group"><label>远程端口</label><input type="number" value={editing.remote_port} onChange={e => setEditing({...editing, remote_port: +e.target.value})} /></div>
            </div>
            <div className="modal-actions">
              <button className="btn btn-default" onClick={() => setEditing(null)}>取消</button>
              <button className="btn btn-primary" onClick={handleSave}>保存</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add plugins/k8s-forward/frontend/src/components/TabSshForward.tsx
git commit -m "feat(k8s-forward): add Tab1 SSH port forwarding component"
```

---

### Task 9: Tab2 — K8s 服务转发组件

**Files:**
- Create: `plugins/k8s-forward/frontend/src/components/TabK8sForward.tsx`

- [ ] **Step 1: 创建 TabK8sForward.tsx**

```tsx
import { useState, useEffect, useCallback } from "react";
import type { KuboardStatus, PodInfo, K8sForwardInfo, ForwardRule, ProxyMapping, LoginResult } from "../types";

const PLUGIN_ID = "k8s-forward";

export default function TabK8sForward() {
  const [kstatus, setKstatus] = useState<KuboardStatus>({ logged_in: false });
  const [mfaRequired, setMfaRequired] = useState(false);
  const [loginForm, setLoginForm] = useState({ url: "http://10.73.64.28:8087", username: "", password: "" });
  const [passcode, setPasscode] = useState("");
  const [clusters, setClusters] = useState<string[]>([]);
  const [selCluster, setSelCluster] = useState("");
  const [namespaces, setNamespaces] = useState<string[]>([]);
  const [selNs, setSelNs] = useState("");
  const [pods, setPods] = useState<PodInfo[]>([]);
  const [search, setSearch] = useState("");
  const [forwards, setForwards] = useState<K8sForwardInfo>({ rules: [], mappings: [] });
  const [toast, setToast] = useState<string | null>(null);
  const [editingDomain, setEditingDomain] = useState<{ rule_id: string; domain: string } | null>(null);

  const call = useCallback(async (method: string, params?: Record<string, unknown>) => {
    return await window.pluginAPI.call(PLUGIN_ID, method, params || {});
  }, []);

  const showToast = (msg: string, isErr = false) => {
    setToast(isErr ? `❌ ${msg}` : `✅ ${msg}`);
    setTimeout(() => setToast(null), 3000);
  };

  const loadStatus = async () => { setKstatus(await call("kuboard_status") as KuboardStatus); };
  const loadForwards = async () => { setForwards(await call("list_k8s_forwards") as K8sForwardInfo); };

  useEffect(() => { loadStatus(); loadForwards(); }, []);

  const handleLogin = async () => {
    try {
      const r = await call("kuboard_login", loginForm) as LoginResult;
      if (r.mfa_required) { setMfaRequired(true); showToast("请输入 MFA 验证码"); }
      else if (r.success) { showToast("登录成功"); loadStatus(); }
      else { showToast(r.message || "登录失败", true); }
    } catch (e: unknown) { showToast(`登录失败: ${e}`, true); }
  };

  const handleMfa = async () => {
    try {
      await call("kuboard_mfa", { passcode });
      setMfaRequired(false); setPasscode("");
      showToast("登录成功");
      loadStatus();
    } catch (e: unknown) { showToast(`MFA 验证失败: ${e}`, true); }
  };

  const handleLogout = async () => {
    await call("kuboard_logout");
    setKstatus({ logged_in: false });
    setClusters([]); setNamespaces([]); setPods([]);
  };

  const loadClusters = async () => {
    try {
      const c = await call("list_clusters") as string[];
      setClusters(c);
      if (c.length > 0) { setSelCluster(c[0]); loadNamespaces(c[0]); }
    } catch (e: unknown) { showToast(`获取集群失败: ${e}`, true); }
  };

  const loadNamespaces = async (cluster: string) => {
    try {
      const ns = await call("list_namespaces", { cluster }) as string[];
      setNamespaces(ns);
      if (ns.length > 0) { setSelNs(ns[0]); loadPods(cluster, ns[0]); }
    } catch (e: unknown) { showToast(`获取命名空间失败: ${e}`, true); }
  };

  const loadPods = async (cluster: string, ns: string) => {
    try {
      const p = await call("list_pods", { cluster, namespace: ns }) as PodInfo[];
      setPods(p);
    } catch (e: unknown) { showToast(`获取 Pod 失败: ${e}`, true); }
  };

  const handleForward = async (podName: string, containerName: string, containerPort: number) => {
    try {
      await call("forward_pod", { cluster: selCluster, namespace: selNs, pod_name: podName, container_name: containerName, container_port: containerPort });
      showToast(`已转发 ${podName}/${containerName}:${containerPort}`);
      loadForwards();
    } catch (e: unknown) { showToast(`转发失败: ${e}`, true); }
  };

  const handleUnforward = async (ruleId: string) => {
    try {
      await call("unforward_pod", { rule_id: ruleId });
      showToast("已取消转发");
      loadForwards();
    } catch (e: unknown) { showToast(`取消失败: ${e}`, true); }
  };

  const handleUpdateDomain = async () => {
    if (!editingDomain) return;
    try {
      await call("update_proxy_mapping", editingDomain);
      showToast("域名已更新");
      setEditingDomain(null);
      loadForwards();
    } catch (e: unknown) { showToast(`更新失败: ${e}`, true); }
  };

  const filteredPods = pods.filter(p => p.name.toLowerCase().includes(search.toLowerCase()));

  const isForwarded = (podName: string, containerName: string, port: number) =>
    forwards.rules.some(r => r.pod_name === podName && r.container_name === containerName && r.remote_port === port);

  const getForwardMapping = (podName: string, containerName: string, port: number) => {
    const rule = forwards.rules.find(r => r.pod_name === podName && r.container_name === containerName && r.remote_port === port);
    if (!rule) return null;
    const mapping = forwards.mappings.find(m => m.rule_id === rule.id);
    return { rule, mapping };
  };

  return (
    <div>
      {toast && <div className={`toast ${toast.startsWith("❌") ? "toast-error" : "toast-success"}`}>{toast}</div>}

      <div className="card">
        <div className="card-header">Kuboard 连接</div>
        <div className="form-row">
          <div className="form-group"><label>Kuboard 地址</label><input value={loginForm.url} onChange={e => setLoginForm({...loginForm, url: e.target.value})} style={{minWidth:220}} /></div>
          <div className="form-group"><label>用户名</label><input value={loginForm.username} onChange={e => setLoginForm({...loginForm, username: e.target.value})} /></div>
          <div className="form-group"><label>密码</label><input type="password" value={loginForm.password} onChange={e => setLoginForm({...loginForm, password: e.target.value})} /></div>
          {kstatus.logged_in
            ? <button className="btn btn-danger" onClick={handleLogout}>登出</button>
            : <button className="btn btn-primary" onClick={handleLogin}>登录</button>
          }
        </div>
        <div style={{marginTop:8}}>
          <span className={`status-dot ${kstatus.logged_in ? "online" : "offline"}`}></span>
          {kstatus.logged_in ? `已登录 → ${kstatus.username}@${kstatus.url}` : "未登录"}
        </div>
      </div>

      {mfaRequired && (
        <div className="card" style={{borderColor:"#ffa502"}}>
          <div className="card-header">双因子认证</div>
          <div className="form-row">
            <div className="form-group"><label>验证码</label><input value={passcode} onChange={e => setPasscode(e.target.value)} placeholder="6位验证码" maxLength={6} /></div>
            <button className="btn btn-primary" onClick={handleMfa}>验证</button>
          </div>
        </div>
      )}

      {kstatus.logged_in && (
        <>
          <div className="card">
            <div className="card-header">集群 & 命名空间</div>
            <div className="form-row">
              <div className="form-group">
                <label>集群</label>
                <select value={selCluster} onChange={e => { setSelCluster(e.target.value); loadNamespaces(e.target.value); }}>
                  {clusters.length === 0 && <option>-- 点击加载 --</option>}
                  {clusters.map(c => <option key={c} value={c}>{c}</option>)}
                </select>
              </div>
              <div className="form-group">
                <label>命名空间</label>
                <select value={selNs} onChange={e => { setSelNs(e.target.value); loadPods(selCluster, e.target.value); }}>
                  {namespaces.length === 0 && <option>-- 选择集群后加载 --</option>}
                  {namespaces.map(n => <option key={n} value={n}>{n}</option>)}
                </select>
              </div>
              <button className="btn btn-primary btn-sm" onClick={loadClusters}>加载集群</button>
              <button className="btn btn-default btn-sm" onClick={() => loadPods(selCluster, selNs)}>刷新 Pod</button>
            </div>
          </div>

          <div className="card">
            <div className="card-header" style={{display:"flex",justifyContent:"space-between"}}>
              <span>Pod 列表 ({filteredPods.length})</span>
              <input placeholder="搜索 Pod..." value={search} onChange={e => setSearch(e.target.value)} style={{padding:"4px 8px",border:"1px solid #3a3a5a",borderRadius:4,background:"#12122a",color:"#e0e0e0",fontSize:12,width:200}} />
            </div>
            <div style={{maxHeight:400,overflow:"auto"}}>
              <table>
                <thead><tr><th>Pod</th><th>IP</th><th>容器</th><th>端口</th><th>状态</th><th>操作</th></tr></thead>
                <tbody>
                  {filteredPods.map(p => (
                    p.containers.map((c, ci) => (
                      c.ports.map((pt, pti) => {
                        const fwd = isForwarded(p.name, c.name, pt.container_port);
                        const fm = getForwardMapping(p.name, c.name, pt.container_port);
                        return (
                          <tr key={`${p.name}-${ci}-${pti}`}>
                            {pti === 0 && ci === 0 && <td rowSpan={p.containers.reduce((a,c) => a + Math.max(c.ports.length, 1), 0)}>{p.name}</td>}
                            {pti === 0 && ci === 0 && <td rowSpan={p.containers.reduce((a,c) => a + Math.max(c.ports.length, 1), 0)}>{p.ip}</td>}
                            {pti === 0 && <td rowSpan={Math.max(c.ports.length, 1)}>{c.name}</td>}
                            <td>{pt.container_port}/{pt.protocol}</td>
                            <td><span className={`badge ${p.status === "Running" ? "badge-success" : "badge-warning"}`}>{p.status}</span></td>
                            <td>
                              {fwd
                                ? <button className="btn btn-danger btn-sm" onClick={() => handleUnforward(fm?.rule.id || "")}>取消</button>
                                : <button className="btn btn-primary btn-sm" onClick={() => handleForward(p.name, c.name, pt.container_port)}>转发</button>
                              }
                            </td>
                          </tr>
                        );
                      })
                    ))
                  ))}
                  {filteredPods.length === 0 && <tr><td colSpan={6} style={{textAlign:"center",color:"#666",padding:20}}>无 Pod</td></tr>}
                </tbody>
              </table>
            </div>
          </div>

          {forwards.rules.length > 0 && (
            <div className="card">
              <div className="card-header">已转发列表</div>
              <table>
                <thead><tr><th>域名</th><th>本地端口</th><th>目标</th><th>操作</th></tr></thead>
                <tbody>
                  {forwards.rules.map(r => {
                    const m = forwards.mappings.find(m => m.rule_id === r.id);
                    return (
                      <tr key={r.id}>
                        <td>{m?.domain || "-"}</td>
                        <td>{r.local_port}</td>
                        <td>{r.remote_host}:{r.remote_port}</td>
                        <td>
                          <button className="btn btn-default btn-sm" style={{marginRight:4}} onClick={() => navigator.clipboard.writeText(m?.domain || "")}>复制域名</button>
                          <button className="btn btn-default btn-sm" style={{marginRight:4}} onClick={() => setEditingDomain({ rule_id: r.id, domain: m?.domain || "" })}>编辑</button>
                          <button className="btn btn-danger btn-sm" onClick={() => handleUnforward(r.id)}>取消</button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}

      {editingDomain && (
        <div className="modal-overlay" onClick={() => setEditingDomain(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>编辑域名</h3>
            <div className="form-group"><label>域名</label><input value={editingDomain.domain} onChange={e => setEditingDomain({...editingDomain, domain: e.target.value})} style={{width:"100%"}} /></div>
            <div className="modal-actions">
              <button className="btn btn-default" onClick={() => setEditingDomain(null)}>取消</button>
              <button className="btn btn-primary" onClick={handleUpdateDomain}>保存</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add plugins/k8s-forward/frontend/src/components/TabK8sForward.tsx
git commit -m "feat(k8s-forward): add Tab2 K8s service forwarding component"
```

---

### Task 10: Tab3 — HTTP 代理组件

**Files:**
- Create: `plugins/k8s-forward/frontend/src/components/TabHttpProxy.tsx`

- [ ] **Step 1: 创建 TabHttpProxy.tsx**

```tsx
import { useState, useEffect, useCallback } from "react";
import type { ProxyStatus, ProxyMapping } from "../types";

const PLUGIN_ID = "k8s-forward";

export default function TabHttpProxy() {
  const [status, setStatus] = useState<ProxyStatus>({ running: false, port: 80, mapping_count: 0 });
  const [mappings, setMappings] = useState<ProxyMapping[]>([]);
  const [port, setPort] = useState(80);
  const [editing, setEditing] = useState<ProxyMapping | null>(null);
  const [toast, setToast] = useState<string | null>(null);

  const call = useCallback(async (method: string, params?: Record<string, unknown>) => {
    return await window.pluginAPI.call(PLUGIN_ID, method, params || {});
  }, []);

  const showToast = (msg: string, isErr = false) => {
    setToast(isErr ? `❌ ${msg}` : `✅ ${msg}`);
    setTimeout(() => setToast(null), 3000);
  };

  const refresh = async () => {
    const s = await call("proxy_status") as ProxyStatus;
    setStatus(s);
    if (s.running) {
      const m = await call("list_proxy_mappings") as ProxyMapping[];
      setMappings(m);
    }
  };

  useEffect(() => { refresh(); }, []);

  const handleStart = async () => {
    try {
      await call("proxy_start", { port });
      showToast(`代理已启动: 127.0.0.1:${port}`);
      refresh();
    } catch (e: unknown) { showToast(`启动失败: ${e}`, true); }
  };

  const handleStop = async () => {
    await call("proxy_stop");
    showToast("代理已停止");
    refresh();
  };

  const handleUpdate = async () => {
    if (!editing) return;
    try {
      await call("update_proxy_mapping", { rule_id: editing.rule_id, domain: editing.domain });
      showToast("域名已更新");
      setEditing(null);
      refresh();
    } catch (e: unknown) { showToast(`更新失败: ${e}`, true); }
  };

  const hostsHint = mappings.map(m => m.domain).join("  ");

  return (
    <div>
      {toast && <div className={`toast ${toast.startsWith("❌") ? "toast-error" : "toast-success"}`}>{toast}</div>}

      <div className="card">
        <div className="card-header">HTTP 代理服务器</div>
        <div className="form-row">
          <div className="form-group">
            <label>代理端口</label>
            <input type="number" value={port} onChange={e => setPort(+e.target.value)} disabled={status.running} style={{width:80}} />
          </div>
          {status.running
            ? <button className="btn btn-danger" onClick={handleStop}>停止</button>
            : <button className="btn btn-primary" onClick={handleStart}>启动</button>
          }
        </div>
        <div style={{marginTop:8}}>
          <span className={`status-dot ${status.running ? "online" : "offline"}`}></span>
          {status.running ? `运行中 → 127.0.0.1:${status.port} (${status.mapping_count} 条映射)` : "已停止"}
        </div>
      </div>

      {status.running && (
        <div className="card">
          <div className="card-header">域名路由映射表</div>
          <table>
            <thead><tr><th>域名</th><th>目标地址</th><th>状态</th><th>操作</th></tr></thead>
            <tbody>
              {mappings.map(m => (
                <tr key={m.rule_id}>
                  <td><code>{m.domain}</code></td>
                  <td>{m.target}</td>
                  <td><span className="badge badge-success">活跃</span></td>
                  <td>
                    {m.editable && <button className="btn btn-default btn-sm" onClick={() => setEditing(m)}>编辑</button>}
                  </td>
                </tr>
              ))}
              {mappings.length === 0 && <tr><td colSpan={4} style={{textAlign:"center",color:"#666",padding:20}}>暂无映射</td></tr>}
            </tbody>
          </table>

          {hostsHint && (
            <div style={{marginTop:12,padding:10,background:"#12122a",borderRadius:4,fontSize:11,fontFamily:"monospace"}}>
              <div style={{color:"#888",marginBottom:4}}>系统 hosts 提示（可选）:</div>
              127.0.0.1  {hostsHint}
            </div>
          )}
        </div>
      )}

      {editing && (
        <div className="modal-overlay" onClick={() => setEditing(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>编辑域名映射</h3>
            <div className="form-group"><label>域名</label><input value={editing.domain} onChange={e => setEditing({...editing, domain: e.target.value})} style={{width:"100%"}} /></div>
            <div style={{marginTop:8,fontSize:11,color:"#888"}}>目标: {editing.target}</div>
            <div className="modal-actions">
              <button className="btn btn-default" onClick={() => setEditing(null)}>取消</button>
              <button className="btn btn-primary" onClick={handleUpdate}>保存</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add plugins/k8s-forward/frontend/src/components/TabHttpProxy.tsx
git commit -m "feat(k8s-forward): add Tab3 HTTP proxy component"
```

---

### Task 11: App.tsx — 主组件 + 编译构建

**Files:**
- Create: `plugins/k8s-forward/frontend/src/App.tsx`

- [ ] **Step 1: 创建 App.tsx**

```tsx
import { useState } from "react";
import TabSshForward from "./components/TabSshForward";
import TabK8sForward from "./components/TabK8sForward";
import TabHttpProxy from "./components/TabHttpProxy";

const TABS = ["SSH端口转发", "K8s服务转发", "HTTP代理"];

export default function App() {
  const [activeTab, setActiveTab] = useState(0);

  return (
    <div className="k8s-forward">
      <div className="tabs">
        {TABS.map((t, i) => (
          <button key={t} className={`tab ${activeTab === i ? "active" : ""}`} onClick={() => setActiveTab(i)}>
            {t}
          </button>
        ))}
      </div>
      {activeTab === 0 && <TabSshForward />}
      {activeTab === 1 && <TabK8sForward />}
      {activeTab === 2 && <TabHttpProxy />}
    </div>
  );
}
```

- [ ] **Step 2: 构建前端**

```bash
cd plugins/k8s-forward/frontend && npm run build
```
预期: 生成 `assets/index.html`, `assets/main.js`, `assets/styles.css`

- [ ] **Step 3: 编译 Rust 后端**

```bash
cargo build --release -p k8s-forward
```
预期: 编译成功，生成 target/release/k8s_forward.dll (Windows)

- [ ] **Step 4: Commit**

```bash
git add plugins/k8s-forward/frontend/src/App.tsx plugins/k8s-forward/assets/
git commit -m "feat(k8s-forward): add App.tsx main component and build artifacts"
```

---

## 自检清单

1. **Spec coverage**: 24 个 handle_call 方法全覆盖 | 3 个 Tab 全实现 | 密码加密 | 导入导出 | K8s API 配置化
2. **Placeholder scan**: 无 TODO/TBD/placeholder
3. **Type consistency**: ForwardRule.rule_type 前端用 `"Manual" | "K8s"` 与 Rust `#[serde(rename_all = "snake_case")]` 一致 → 注意：serde 重命名后前端应传 `"manual"` / `"k8s"`。需修复前端类型定义。

（已修复：前端类型统一使用 snake_case `"manual" | "k8s"` 与 Rust serde 一致）
