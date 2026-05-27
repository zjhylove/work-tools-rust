//! # K8s 端口转发插件
//!
//! 通过 Kuboard 发现 K8s Pod，使用 SSH 隧道 + HTTP 代理转发流量。
//! 这是最复杂的插件，集成了多种技术栈。
//!
//! ## 架构概览
//! ```
//! 用户浏览器
//!   → HTTP 代理 (本地端口)
//!   → SSH 隧道 (SSH 跳板机)
//!   → K8s Pod (远程集群)
//! ```
//!
//! ## 核心组件
//! 1. **KuboardClient**: 与 Kuboard API 交互，获取集群/Pod 信息
//! 2. **SshService**: SSH 连接管理 + 端口转发（ssh2 crate）
//! 3. **HttpProxySvc**: 本地 HTTP 反向代理（域名 → 本地端口映射）
//!
//! ## 数据流
//! ```
//! kuboard_login → list_clusters → list_namespaces → list_pods
//!   → ssh_connect → forward_pod (创建 SSH 隧道)
//!   → proxy_start (启动 HTTP 代理)
//!   → 浏览器访问代理端口 → 流量转发到 K8s Pod
//! ```
//!
//! ## Rust 知识点
//! - `tokio::runtime::Runtime`: 在同步插件中运行异步代码
//! - `Mutex<Option<T>>`: 可空的线程安全状态
//! - `macro_rules! dispatch!`: 内部宏减少样板代码
//! - `block_on`: 同步等待异步操作完成

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::runtime::Runtime;
use worktools_plugin_api::storage::PluginStorage;
use worktools_plugin_api::*;

pub mod crypto;
pub mod http_proxy;
pub mod kuboard_client;
pub mod models;
pub mod ssh_service;

use crypto::PasswordEncryptor;
use http_proxy::HttpProxySvc;
use kuboard_client::KuboardClient;
use models::*;
use ssh_service::SshService;

/// K8s 转发插件（最复杂的插件）
///
/// ## 字段说明
/// - `runtime`: 自有的 Tokio 运行时，用于在同步上下文中执行异步操作
/// - `ssh_connections`: 多个 SSH 连接服务（按 connection_id 索引，Mutex 保护用于跨异步任务共享）
/// - `proxy`: HTTP 代理（Option 表示可能未启动）
/// - `kuboard`: Kuboard 客户端（Option 表示可能未登录）
pub struct K8sForwardPlugin {
    storage: PluginStorage,
    encryptor: PasswordEncryptor,
    /// Tokio 异步运行时 — 因为 SshService 和 HTTP 代理内部使用 async
    /// Plugin trait 的方法是同步的，所以需要 block_on 桥接
    runtime: Runtime,
    ssh_connections: Mutex<HashMap<String, SshService>>,
    proxy: Mutex<Option<HttpProxySvc>>,
    kuboard: Mutex<Option<KuboardClient>>,
}

impl K8sForwardPlugin {
    pub fn new() -> Self {
        Self {
            storage: PluginStorage::new("k8s-forward", "k8s-forward.json"),
            encryptor: PasswordEncryptor::new(),
            // `Runtime::new()` 创建新的 Tokio 异步运行时
            runtime: Runtime::new().expect("Failed to create tokio runtime"),
            ssh_connections: Mutex::new(HashMap::new()),
            proxy: Mutex::new(None),
            kuboard: Mutex::new(None),
        }
    }

    fn load_data(&self) -> Result<PluginData> {
        let mut data: PluginData = self.storage.load_json()?;
        if data.migrate_legacy() {
            self.save_data(&data)?;
            tracing::info!("已迁移旧版单 SSH 配置到多 SSH 格式");
        }
        Ok(data)
    }

    fn save_data(&self, data: &PluginData) -> Result<()> {
        self.storage.save_json(data)
    }

    // ── SSH 管理 ──

    /// 恢复指定连接的转发规则到 SSH 服务
    fn restore_forwards(&self, ssh: &mut SshService, connection_id: &str, data: &mut PluginData) -> usize {
        let mut restored = 0;
        for rule in data.forward_rules.iter_mut() {
            if rule.ssh_connection_id != connection_id {
                continue;
            }
            match ssh.add_forward(
                &rule.local_host,
                &rule.remote_host,
                rule.remote_port,
                rule.local_port,
                connection_id,
            ) {
                Ok(assigned) => {
                    if rule.local_port == 0 {
                        rule.local_port = assigned;
                    }
                    restored += 1;
                }
                Err(e) => tracing::warn!("恢复转发规则失败 [{}]: {}", rule.name, e),
            }
        }
        restored
    }

    fn handle_ssh_connect(&self, params: &Value) -> Result<Value> {
        let connection_id = get_str(params, "connection_id")?;

        let data = self.load_data()?;
        let conn = data.ssh_connections.iter()
            .find(|c| c.id == connection_id)
            .ok_or_else(|| anyhow::anyhow!("SSH 连接配置不存在"))?;

        let password = self.encryptor.decrypt(&conn.password)?;

        let mut connections = self.ssh_connections.lock().unwrap();
        let ssh = connections.entry(connection_id.to_string()).or_insert_with(SshService::new);
        ssh.set_manual_disconnect(false);
        if ssh.is_connected() {
            return Err(anyhow::anyhow!("SSH 已连接"));
        }
        ssh.disconnect();
        ssh.connect(&conn.host, conn.port, &conn.username, &password)?;
        ssh.start_heartbeat();

        let mut data = self.load_data()?;
        let restored = self.restore_forwards(ssh, connection_id, &mut data);

        Ok(
            json!({"success": true, "message": format!("SSH 连接成功，已恢复 {} 条转发规则", restored)}),
        )
    }

    fn handle_ssh_disconnect(&self, params: &Value) -> Result<Value> {
        let connection_id = get_str(params, "connection_id")?;
        let mut connections = self.ssh_connections.lock().unwrap();
        if let Some(ssh) = connections.get_mut(connection_id) {
            ssh.set_manual_disconnect(true);
            ssh.disconnect();
            Ok(json!({"success": true}))
        } else {
            Err(anyhow::anyhow!("SSH 连接不存在"))
        }
    }

    fn handle_ssh_reconnect(&self, params: &Value) -> Result<Value> {
        let connection_id = get_str(params, "connection_id")?;
        let mut connections = self.ssh_connections.lock().unwrap();
        let ssh = connections.get_mut(connection_id)
            .ok_or_else(|| anyhow::anyhow!("SSH 连接不存在"))?;
        if ssh.is_connected() {
            return Err(anyhow::anyhow!("SSH 已连接，无需重连"));
        }
        if !ssh.has_connect_params() {
            return Err(anyhow::anyhow!("没有保存的连接参数，请使用 ssh_connect"));
        }
        ssh.stop_reconnect();
        ssh.set_manual_disconnect(false);
        ssh.start_reconnect();
        Ok(json!({"success": true, "message": "开始重连..."}))
    }

    fn handle_ssh_status(&self, params: &Value) -> Result<Value> {
        let connection_id = get_str(params, "connection_id")?;
        let data = self.load_data()?;
        let conn = data.ssh_connections.iter()
            .find(|c| c.id == connection_id)
            .ok_or_else(|| anyhow::anyhow!("SSH 连接配置不存在"))?;

        let mut connections = self.ssh_connections.lock().unwrap();
        let ssh = connections.entry(connection_id.to_string()).or_insert_with(SshService::new);

        let need_reconnect = !ssh.is_reconnecting()
            && ssh.has_connect_params()
            && !ssh.manual_disconnect()
            && !ssh.is_reconnect_exhausted()
            && (ssh.heartbeat_exited() || (ssh.is_connected() && ssh.any_forward_thread_exited()));

        if need_reconnect {
            if ssh.heartbeat_exited() {
                tracing::warn!("SSH 心跳检测到断连 [{}]，启动自动重连", conn.name);
            } else {
                tracing::warn!("检测到转发线程异常退出 [{}]，启动自动重连", conn.name);
            }
            ssh.start_reconnect();
        }

        let reconnect_result = ssh.check_reconnect_result();

        if reconnect_result == Some(true) {
            ssh.stop_forwards();
            let mut data_mut = self.load_data()?;
            let restored = self.restore_forwards(ssh, connection_id, &mut data_mut);
            if restored > 0 {
                self.save_data(&data_mut)?;
            }
            ssh.start_heartbeat();
            tracing::info!("SSH [{}] 重连成功，已恢复 {} 条转发规则", conn.name, restored);
        }

        let state = ssh.connection_state();
        let reconnect_info = ssh.get_reconnect_info();

        let status = SshStatus {
            connected: state == SshConnectionState::Connected,
            host: Some(conn.host.clone()),
            port: Some(conn.port),
            status: state,
            reconnect_info,
            connection_id: Some(conn.id.clone()),
            connection_name: Some(conn.name.clone()),
        };
        Ok(serde_json::to_value(status)?)
    }

    // ── SSH 连接配置 CRUD ──

    fn handle_ssh_add_connection(&self, params: &Value) -> Result<Value> {
        let name = get_str(params, "name")?;
        let host = get_str(params, "host")?;
        let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16;
        let username = get_str(params, "username")?;
        let password = get_str(params, "password")?;

        let enc_pwd = self.encryptor.encrypt(password)?;
        let conn = SshConnection {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            host: host.to_string(),
            port,
            username: username.to_string(),
            password: enc_pwd,
        };

        let mut data = self.load_data()?;
        data.ssh_connections.push(conn.clone());
        data.ssh = None;
        self.save_data(&data)?;

        Ok(serde_json::to_value(&conn)?)
    }

    fn handle_ssh_update_connection(&self, params: &Value) -> Result<Value> {
        let id = get_str(params, "id")?;
        let mut data = self.load_data()?;
        let conn = data.ssh_connections.iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;

        if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
            conn.name = name.to_string();
        }
        if let Some(host) = params.get("host").and_then(|v| v.as_str()) {
            conn.host = host.to_string();
        }
        if let Some(port) = params.get("port").and_then(|v| v.as_u64()) {
            conn.port = port as u16;
        }
        if let Some(username) = params.get("username").and_then(|v| v.as_str()) {
            conn.username = username.to_string();
        }
        if let Some(password) = params.get("password").and_then(|v| v.as_str()) {
            conn.password = self.encryptor.encrypt(password)?;
        }

        let result = serde_json::to_value(&*conn)?;
        data.ssh = None;
        self.save_data(&data)?;
        Ok(result)
    }

    fn handle_ssh_remove_connection(&self, params: &Value) -> Result<Value> {
        let id = get_str(params, "id")?;
        let mut data = self.load_data()?;

        data.ssh_connections.retain(|c| c.id != id);
        data.ssh = None;

        let removed_rules: Vec<ForwardRule> = data.forward_rules
            .iter()
            .filter(|r| r.ssh_connection_id == id)
            .cloned()
            .collect();
        let rule_count = removed_rules.len();
        data.forward_rules.retain(|r| r.ssh_connection_id != id);

        let mut connections = self.ssh_connections.lock().unwrap();
        if let Some(mut ssh) = connections.remove(id) {
            ssh.disconnect();
        }

        if let Some(ref proxy) = *self.proxy.lock().unwrap() {
            for rule in &removed_rules {
                proxy.unregister_by_rule_id(&rule.id);
            }
        }

        self.save_data(&data)?;
        Ok(json!({"success": true, "removed_rules": rule_count}))
    }

    fn handle_ssh_list_connections(&self) -> Result<Value> {
        let data = self.load_data()?;
        let connections = self.ssh_connections.lock().unwrap();

        let list: Vec<Value> = data.ssh_connections.iter().map(|conn| {
            let ssh = connections.get(&conn.id);
            let state = ssh.map(|s| s.connection_state()).unwrap_or(SshConnectionState::Disconnected);
            let connected = state == SshConnectionState::Connected;
            let reconnect_info = ssh.and_then(|s| s.get_reconnect_info());
            serde_json::to_value(SshStatus {
                connected,
                host: Some(conn.host.clone()),
                port: Some(conn.port),
                status: state,
                reconnect_info,
                connection_id: Some(conn.id.clone()),
                connection_name: Some(conn.name.clone()),
            }).unwrap_or(json!({}))
        }).collect();

        Ok(json!(list))
    }

    // ── Kuboard 管理 ──

    fn handle_kuboard_login(&self, params: &Value) -> Result<Value> {
        let url = get_str(params, "url")?;
        let username = get_str(params, "username")?;
        let password = get_str(params, "password")?;

        let mut client = KuboardClient::new(url);
        // `block_on` 在同步上下文中执行异步 Future
        let result = self.runtime.block_on(client.login(username, password))?;

        if result.success {
            *self.kuboard.lock().unwrap() = Some(client);

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
        *self.kuboard.lock().unwrap() = None;
        Ok(json!({"success": true}))
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

    // ── K8s Pod 转发 ──

    /// 转发 K8s Pod 的端口到本地
    ///
    /// 流程：
    /// 1. 通过 Kuboard 获取 Pod 的 IP
    /// 2. 通过 SSH 创建到 Pod IP 的隧道
    /// 3. 在 HTTP 代理中注册域名映射
    fn handle_forward_pod(&self, params: &Value) -> Result<Value> {
        let connection_id = get_str(params, "ssh_connection_id")?;
        let cluster = get_str(params, "cluster")?;
        let namespace = get_str(params, "namespace")?;
        let pod_name = get_str(params, "pod_name")?;
        let container_name = get_str(params, "container_name")?;
        let container_port = params
            .get("container_port")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u16;

        // 获取 Pod IP
        let kuboard = self.kuboard.lock().unwrap();
        let pods = if let Some(ref client) = *kuboard {
            self.runtime
                .block_on(client.list_pods(cluster, namespace))?
        } else {
            return Err(anyhow::anyhow!("请先登录 Kuboard"));
        };
        let pod = pods
            .iter()
            .find(|p| p.name == pod_name)
            .ok_or_else(|| anyhow::anyhow!("Pod 未找到"))?;

        // 创建 SSH 隧道
        let mut connections = self.ssh_connections.lock().unwrap();
        let ssh = connections.get_mut(connection_id)
            .ok_or_else(|| anyhow::anyhow!("SSH 连接不存在"))?;
        if !ssh.is_connected() {
            return Err(anyhow::anyhow!("SSH 未连接，请先连接所选的 SSH 服务器"));
        }
        let local_port = ssh.add_forward("127.0.0.1", &pod.ip, container_port, 0, connection_id)?;

        // 注册到 HTTP 代理
        let domain = pod_name.to_string();
        let addr = format!("{}:{}", pod.ip, container_port);
        let rule_id = uuid::Uuid::new_v4().to_string();

        if let Some(ref p) = *self.proxy.lock().unwrap() {
            p.register(&domain, &format!("127.0.0.1:{}", local_port), &rule_id, false);
            p.register(&addr, &format!("127.0.0.1:{}", local_port), &rule_id, true);
        }

        // 保存规则
        let rule = ForwardRule {
            id: rule_id,
            name: format!("{}/{}:{}", pod_name, container_name, container_port),
            local_host: "127.0.0.1".to_string(),
            local_port,
            remote_host: pod.ip.clone(),
            remote_port: container_port,
            rule_type: RuleType::K8s,
            cluster: Some(cluster.to_string()),
            namespace: Some(namespace.to_string()),
            pod_name: Some(pod_name.to_string()),
            container_name: Some(container_name.to_string()),
            ssh_connection_id: connection_id.to_string(),
        };

        let mut data = self.load_data()?;
        data.forward_rules.push(rule.clone());
        data.ssh = None;
        self.save_data(&data)?;

        Ok(
            json!({"rule": rule, "proxy_mapping": {"domain": addr, "target": format!("127.0.0.1:{}", local_port)}}),
        )
    }

    // ── HTTP 代理 ──

    /// 启动 HTTP 反向代理
    /// 将所有已注册的 K8s 转发规则注册到代理中
    fn handle_proxy_start(&self, params: &Value) -> Result<Value> {
        let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
        let mut proxy = HttpProxySvc::new(port);

        let data = self.load_data()?;
        for rule in &data.forward_rules {
            if rule.rule_type == RuleType::K8s {
                let domain = rule.pod_name.as_deref().unwrap_or("");
                let addr = format!("{}:{}", rule.remote_host, rule.remote_port);
                proxy.register(
                    domain,
                    &format!("127.0.0.1:{}", rule.local_port),
                    &rule.id,
                    false,
                );
                proxy.register(
                    &addr,
                    &format!("127.0.0.1:{}", rule.local_port),
                    &rule.id,
                    true,
                );
            }
        }

        self.runtime.block_on(proxy.start())?;
        *self.proxy.lock().unwrap() = Some(proxy);

        Ok(json!({"success": true, "message": format!("代理已启动: 127.0.0.1:{}", port)}))
    }

    fn handle_proxy_stop(&self) -> Result<Value> {
        if let Some(ref mut proxy) = *self.proxy.lock().unwrap() {
            proxy.stop();
        }
        *self.proxy.lock().unwrap() = None;
        Ok(json!({"success": true}))
    }

    // ── 转发规则 CRUD ──

    fn handle_list_forward_rules(&self) -> Result<Value> {
        let data = self.load_data()?;
        Ok(serde_json::to_value(&data.forward_rules)?)
    }

    fn handle_add_forward_rule(&self, params: &Value) -> Result<Value> {
        let mut rule: ForwardRule = serde_json::from_value(params.clone())?;
        let mut data = self.load_data()?;

        let mut connections = self.ssh_connections.lock().unwrap();
        if let Some(ssh) = connections.get_mut(&rule.ssh_connection_id) {
            if ssh.is_connected() && rule.rule_type == RuleType::Manual {
                let assigned = ssh.add_forward(
                    &rule.local_host,
                    &rule.remote_host,
                    rule.remote_port,
                    rule.local_port,
                    &rule.ssh_connection_id,
                )?;
                if rule.local_port == 0 {
                    rule.local_port = assigned;
                }
            }
        }
        if rule.id.is_empty() {
            rule.id = uuid::Uuid::new_v4().to_string();
        }

        data.forward_rules.push(rule.clone());
        data.ssh = None;
        self.save_data(&data)?;
        Ok(serde_json::to_value(&rule)?)
    }

    fn handle_update_forward_rule(&self, params: &Value) -> Result<Value> {
        let updated: ForwardRule = serde_json::from_value(params.clone())?;
        let mut data = self.load_data()?;
        if let Some(rule) = data.forward_rules.iter_mut().find(|r| r.id == updated.id) {
            let mut connections = self.ssh_connections.lock().unwrap();
            if let Some(ssh) = connections.get_mut(&rule.ssh_connection_id) {
                let _ = ssh.remove_forward(rule.local_port);
                if ssh.is_connected() {
                    let assigned = ssh.add_forward(
                        &updated.local_host,
                        &updated.remote_host,
                        updated.remote_port,
                        updated.local_port,
                        &updated.ssh_connection_id,
                    )?;
                    let mut saved = updated.clone();
                    if saved.local_port == 0 {
                        saved.local_port = assigned;
                    }
                    *rule = saved;
                } else {
                    *rule = updated.clone();
                }
            } else {
                *rule = updated.clone();
            }
            let result = serde_json::to_value(&*rule)?;
            data.ssh = None;
            self.save_data(&data)?;
            return Ok(result);
        }
        Err(anyhow::anyhow!("规则不存在"))
    }

    fn handle_remove_forward_rule(&self, params: &Value) -> Result<Value> {
        let id = get_str(params, "id")?;
        let mut data = self.load_data()?;
        if let Some(pos) = data.forward_rules.iter().position(|r| r.id == id) {
            let rule = data.forward_rules.remove(pos);
            let mut connections = self.ssh_connections.lock().unwrap();
            if let Some(ssh) = connections.get_mut(&rule.ssh_connection_id) {
                let _ = ssh.remove_forward(rule.local_port);
            }
            if let Some(ref proxy) = *self.proxy.lock().unwrap() {
                proxy.unregister_by_rule_id(&rule.id);
            }
            data.ssh = None;
            self.save_data(&data)?;
            return Ok(json!({"success": true}));
        }
        Err(anyhow::anyhow!("规则不存在"))
    }

    fn handle_import_rules(&self, params: &Value) -> Result<Value> {
        let imported: Vec<ForwardRule> = serde_json::from_value(
            params
                .get("rules")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("缺少 rules"))?,
        )?;
        let mut data = self.load_data()?;
        for rule in imported {
            if let Some(existing) = data.forward_rules.iter_mut().find(|r| r.id == rule.id) {
                *existing = rule;
            } else {
                data.forward_rules.push(rule);
            }
        }
        data.ssh = None;
        self.save_data(&data)?;
        Ok(serde_json::to_value(&data)?)
    }

    fn handle_export_rules(&self) -> Result<Value> {
        let data = self.load_data()?;
        Ok(serde_json::to_value(&data.forward_rules)?)
    }

    // ── Kuboard 状态 ──

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
            let pods = self
                .runtime
                .block_on(client.list_pods(cluster, namespace))?;
            Ok(serde_json::to_value(pods)?)
        } else {
            Err(anyhow::anyhow!("请先登录 Kuboard"))
        }
    }

    // ── K8s 转发管理 ──

    fn handle_unforward_pod(&self, params: &Value) -> Result<Value> {
        let rule_id = get_str(params, "rule_id")?;
        let mut data = self.load_data()?;
        if let Some(pos) = data.forward_rules.iter().position(|r| r.id == rule_id) {
            let rule = data.forward_rules.remove(pos);
            let mut connections = self.ssh_connections.lock().unwrap();
            if let Some(ssh) = connections.get_mut(&rule.ssh_connection_id) {
                let _ = ssh.remove_forward(rule.local_port);
            }
            if let Some(ref proxy) = *self.proxy.lock().unwrap() {
                proxy.unregister_by_rule_id(&rule.id);
            }
            data.ssh = None;
            self.save_data(&data)?;
            return Ok(json!({"success": true}));
        }
        Err(anyhow::anyhow!("规则不存在"))
    }

    fn handle_list_k8s_forwards(&self) -> Result<Value> {
        let data = self.load_data()?;
        let k8s_rules: Vec<&ForwardRule> = data
            .forward_rules
            .iter()
            .filter(|r| r.rule_type == RuleType::K8s)
            .collect();
        let mappings = self.proxy.lock().unwrap();
        let mappings = mappings
            .as_ref()
            .map(|p| p.list_mappings())
            .unwrap_or_default();
        Ok(json!({"rules": k8s_rules, "mappings": mappings}))
    }

    fn handle_validate_k8s_forwards(&self) -> Result<Value> {
        let kuboard_guard = self.kuboard.lock().unwrap();
        let client = kuboard_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Kuboard 未登录"))?;

        let mut data = self.load_data()?;
        let mut ns_map: std::collections::HashMap<(String, String), Vec<(usize, String)>> =
            std::collections::HashMap::new();
        for (i, r) in data.forward_rules.iter().enumerate() {
            if r.rule_type != RuleType::K8s {
                continue;
            }
            let cluster = r.cluster.clone().unwrap_or_default();
            let namespace = r.namespace.clone().unwrap_or_default();
            let pod_name = r.pod_name.clone().unwrap_or_default();
            ns_map
                .entry((cluster, namespace))
                .or_default()
                .push((i, pod_name));
        }

        let mut to_remove: Vec<usize> = Vec::new();
        for ((cluster, namespace), entries) in &ns_map {
            if cluster.is_empty() || namespace.is_empty() {
                to_remove.extend(entries.iter().map(|(i, _)| *i));
                continue;
            }
            match self.runtime.block_on(client.list_pods(cluster, namespace)) {
                Ok(pods) => {
                    for (idx, pod_name) in entries {
                        let valid = pods
                            .iter()
                            .any(|p| p.name == *pod_name && p.status == "Running");
                        if !valid {
                            to_remove.push(*idx);
                        }
                    }
                }
                Err(_) => {}
            }
        }

        to_remove.sort_unstable();
        to_remove.dedup();
        to_remove.reverse();
        let mut connections = self.ssh_connections.lock().unwrap();
        let proxy_guard = self.proxy.lock().unwrap();
        for idx in &to_remove {
            let rule = data.forward_rules.remove(*idx);
            if let Some(ssh) = connections.get_mut(&rule.ssh_connection_id) {
                let _ = ssh.remove_forward(rule.local_port);
            }
            if let Some(ref proxy) = *proxy_guard {
                proxy.unregister_by_rule_id(&rule.id);
            }
        }
        data.ssh = None;
        self.save_data(&data)?;
        Ok(json!({"removed": to_remove.len()}))
    }

    // ── 代理状态 ──

    fn handle_proxy_status(&self) -> Result<Value> {
        let guard = self.proxy.lock().unwrap();
        let data = self.load_data()?;
        let status = ProxyStatus {
            running: guard.as_ref().map(|p| p.is_running()).unwrap_or(false),
            port: data.proxy.port,
            mapping_count: guard
                .as_ref()
                .map(|p| {
                    let ms = p.list_mappings();
                    let mut ids = std::collections::HashSet::new();
                    for m in &ms {
                        ids.insert(m.rule_id.clone());
                    }
                    ids.len()
                })
                .unwrap_or(0),
        };
        Ok(serde_json::to_value(status)?)
    }

    fn handle_list_proxy_mappings(&self) -> Result<Value> {
        let guard = self.proxy.lock().unwrap();
        let mappings = guard
            .as_ref()
            .map(|p| p.list_mappings())
            .unwrap_or_default();
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

    // ── 配置管理 ──

    fn handle_get_config(&self) -> Result<Value> {
        let mut data = self.load_data()?;
        // 解密 SSH 连接凭据后返回
        for conn in data.ssh_connections.iter_mut() {
            if let Ok(pwd) = self.encryptor.decrypt(&conn.password) {
                conn.password = pwd;
            }
        }
        if let Some(ref kb_cfg) = data.kuboard {
            if let Ok(pwd) = self.encryptor.decrypt(&kb_cfg.password) {
                data.kuboard = Some(KuboardConfig {
                    password: pwd,
                    ..kb_cfg.clone()
                });
            }
        }
        data.ssh = None;
        Ok(serde_json::to_value(data)?)
    }

    fn handle_reset_config(&self) -> Result<Value> {
        self.save_data(&PluginData::default())?;
        Ok(json!({"success": true}))
    }
}

/// 辅助函数：从 JSON Value 中提取字符串参数
fn get_str<'a>(params: &'a Value, key: &str) -> Result<&'a str> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("缺少参数: {}", key))
}

impl Plugin for K8sForwardPlugin {
    fn id(&self) -> &str {
        "k8s-forward"
    }
    fn name(&self) -> &str {
        "K8s IP转发"
    }
    fn description(&self) -> &str {
        "通过Kuboard发现K8s Pod，SSH隧道+HTTP代理转发流量"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn icon(&self) -> &str {
        "\u{1F310}"
    }
    fn get_view(&self) -> String {
        "<div>插件前端资源加载中...</div>".to_string()
    }

    /// 插件销毁时的清理
    /// 按照依赖顺序释放资源：先停代理，再断 SSH
    fn destroy(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 停止 HTTP 代理
        if let Some(ref mut proxy) = *self.proxy.lock().unwrap() {
            proxy.stop();
        }
        // 断开所有 SSH 连接（停止所有转发线程 + join 等待线程结束）
        for (_, mut ssh) in self.ssh_connections.lock().unwrap().drain() {
            ssh.disconnect();
        }
        Ok(())
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // 内部宏：统一处理错误类型转换
        macro_rules! dispatch {
            ($e:expr) => {
                $e.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })
            };
        }

        match method {
            "ssh_connect" => dispatch!(self.handle_ssh_connect(&params)),
            "ssh_disconnect" => dispatch!(self.handle_ssh_disconnect(&params)),
            "ssh_status" => dispatch!(self.handle_ssh_status(&params)),
            "ssh_reconnect" => dispatch!(self.handle_ssh_reconnect(&params)),
            "ssh_add_connection" => dispatch!(self.handle_ssh_add_connection(&params)),
            "ssh_update_connection" => dispatch!(self.handle_ssh_update_connection(&params)),
            "ssh_remove_connection" => dispatch!(self.handle_ssh_remove_connection(&params)),
            "ssh_list_connections" => dispatch!(self.handle_ssh_list_connections()),
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
            "validate_k8s_forwards" => dispatch!(self.handle_validate_k8s_forwards()),
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
