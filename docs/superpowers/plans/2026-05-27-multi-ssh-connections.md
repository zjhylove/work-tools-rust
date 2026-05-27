# Multi-SSH Connections Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend k8s-forward plugin to support multiple simultaneous SSH connections, each with its own set of port forward rules.

**Architecture:** Replace single `SshService` instance with `HashMap<String, SshService>`. Add `SshConnection` entity and associate `ForwardRule` with a specific connection via `ssh_connection_id`. Frontend adds connection management UI and connection selection to forward operations.

**Tech Stack:** Rust + ssh2 + serde + tokio | React + TypeScript

**Spec:** `docs/superpowers/specs/2026-05-27-multi-ssh-connections-design.md`

---

## File Structure

| Action | File | Responsibility |
|---|---|---|
| Modify | `plugins/k8s-forward/src/models.rs` | Add `SshConnection`, modify `PluginData` and `ForwardRule`, add migration logic |
| Modify | `plugins/k8s-forward/src/lib.rs` | Replace `ssh: Mutex<SshService>` with `ssh_connections: Mutex<HashMap<String, SshService>>`, rewrite all SSH/forward handlers |
| Modify | `plugins/k8s-forward/src/ssh_service.rs` | Add `ssh_connection_id` field to `ForwardRule` construction in `add_forward` |
| Modify | `plugins/k8s-forward/frontend/src/types.ts` | Add `SshConnection` interface, update `ForwardRule`, `SshStatus` |
| Modify | `plugins/k8s-forward/frontend/src/components/TabSshForward.tsx` | Connection list UI, SSH selection dropdown for rules |
| Modify | `plugins/k8s-forward/frontend/src/components/TabK8sForward.tsx` | Add SSH connection selector for Pod forwarding |
| Modify | `plugins/k8s-forward/frontend/src/components/TabHttpProxy.tsx` | No functional changes needed (proxy aggregates from all connections) |

---

### Task 1: Data Model — Add `SshConnection` and update `PluginData`

**Files:**
- Modify: `plugins/k8s-forward/src/models.rs`

- [ ] **Step 1: Add `SshConnection` struct and update `ForwardRule`**

Add after the `SshConfig` struct (line 68):

```rust
/// SSH 连接配置（支持多个连接）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConnection {
    pub id: String,
    pub name: String,
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub username: String,
    pub password: String, // 加密存储
}
```

Add to `ForwardRule` after `container_name` (line 41):

```rust
    #[serde(default)]
    pub ssh_connection_id: String,
```

- [ ] **Step 2: Update `PluginData` to support both old and new formats**

Replace `PluginData` struct and `Default` impl (lines 100-122) with:

```rust
/// 插件持久化数据结构（顶层）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginData {
    #[serde(default)]
    pub ssh_connections: Vec<SshConnection>,
    #[serde(default)]
    pub ssh: Option<SshConfig>, // legacy, kept for migration
    #[serde(default)]
    pub kuboard: Option<KuboardConfig>,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub forward_rules: Vec<ForwardRule>,
}

impl PluginData {
    /// Migrate legacy single-SSH format to multi-SSH format.
    /// Returns true if migration occurred.
    pub fn migrate_legacy(&mut self) -> bool {
        if !self.ssh_connections.is_empty() || self.ssh.is_none() {
            return false;
        }
        let old = self.ssh.take().unwrap();
        let conn_id = uuid::Uuid::new_v4().to_string();
        let conn = SshConnection {
            name: old.host.clone(),
            host: old.host,
            port: old.port,
            username: old.username,
            password: old.password,
            id: conn_id.clone(),
        };
        for rule in &mut self.forward_rules {
            if rule.ssh_connection_id.is_empty() {
                rule.ssh_connection_id = conn_id.clone();
            }
        }
        self.ssh_connections.push(conn);
        true
    }
}

impl Default for PluginData {
    fn default() -> Self {
        Self {
            ssh_connections: vec![],
            ssh: None,
            kuboard: None,
            proxy: ProxyConfig { port: 80 },
            forward_rules: vec![],
        }
    }
}
```

Also update `SshStatus` to include connection info (replace lines 177-189):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshStatus {
    pub connected: bool,
    pub host: Option<String>,
    pub port: Option<u16>,
    #[serde(default = "default_connection_state")]
    pub status: SshConnectionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconnect_info: Option<ReconnectInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_name: Option<String>,
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p k8s-forward 2>&1`
Expected: Compilation errors in `lib.rs` and `ssh_service.rs` because `ForwardRule` now requires `ssh_connection_id` and `PluginData` struct changed — this is expected, fix in Task 2.

- [ ] **Step 4: Commit**

```bash
git add plugins/k8s-forward/src/models.rs
git commit -m "feat(k8s-forward): add SshConnection model and update PluginData for multi-SSH"
```

---

### Task 2: Backend — Update `SshService` to accept `ssh_connection_id` in `add_forward`

**Files:**
- Modify: `plugins/k8s-forward/src/ssh_service.rs`

- [ ] **Step 1: Update `add_forward` to include `ssh_connection_id` in constructed `ForwardRule`**

In `ssh_service.rs`, the `add_forward` method (line 336) constructs a `ForwardRule` without `ssh_connection_id`. Add a parameter:

Change the `add_forward` signature to accept `ssh_connection_id`:

```rust
    pub fn add_forward(
        &mut self,
        local_host: &str,
        remote_host: &str,
        remote_port: u16,
        local_port: u16,
        ssh_connection_id: &str,
    ) -> Result<u16> {
```

And update the `ForwardRule` construction inside (around line 336):

```rust
        let rule = ForwardRule {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("forward-{}", local_port),
            local_host: local_host.to_string(),
            local_port,
            remote_host: rh_for_rule,
            remote_port,
            rule_type: RuleType::Manual,
            cluster: None,
            namespace: None,
            pod_name: None,
            container_name: None,
            ssh_connection_id: ssh_connection_id.to_string(),
        };
```

- [ ] **Step 2: Commit**

```bash
git add plugins/k8s-forward/src/ssh_service.rs
git commit -m "feat(k8s-forward): pass ssh_connection_id to SshService::add_forward"
```

---

### Task 3: Backend — Rewrite `K8sForwardPlugin` for multi-SSH

**Files:**
- Modify: `plugins/k8s-forward/src/lib.rs`

This is the largest task. All changes are in one file.

- [ ] **Step 1: Update struct and constructor**

Replace the `K8sForwardPlugin` struct (lines 59-68):

```rust
use std::collections::HashMap;

pub struct K8sForwardPlugin {
    storage: PluginStorage,
    encryptor: PasswordEncryptor,
    runtime: Runtime,
    ssh_connections: Mutex<HashMap<String, SshService>>,
    proxy: Mutex<Option<HttpProxySvc>>,
    kuboard: Mutex<Option<KuboardClient>>,
}
```

Update `new()` (lines 71-81):

```rust
    pub fn new() -> Self {
        Self {
            storage: PluginStorage::new("k8s-forward", "k8s-forward.json"),
            encryptor: PasswordEncryptor::new(),
            runtime: Runtime::new().expect("Failed to create tokio runtime"),
            ssh_connections: Mutex::new(HashMap::new()),
            proxy: Mutex::new(None),
            kuboard: Mutex::new(None),
        }
    }
```

- [ ] **Step 2: Update `load_data` to run migration**

Replace `load_data` (lines 83-85):

```rust
    fn load_data(&self) -> Result<PluginData> {
        let mut data: PluginData = self.storage.load_json()?;
        if data.migrate_legacy() {
            self.save_data(&data)?;
            tracing::info!("已迁移旧版单 SSH 配置到多 SSH 格式");
        }
        Ok(data)
    }
```

- [ ] **Step 3: Update `restore_forwards` to filter by `ssh_connection_id`**

Replace `restore_forwards` (lines 94-113):

```rust
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
```

- [ ] **Step 4: Rewrite `handle_ssh_connect`**

Replace `handle_ssh_connect` (lines 115-145):

```rust
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
```

- [ ] **Step 5: Rewrite `handle_ssh_disconnect`**

Replace `handle_ssh_disconnect` (lines 147-152):

```rust
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
```

- [ ] **Step 6: Rewrite `handle_ssh_reconnect`**

Replace `handle_ssh_reconnect` (lines 154-166):

```rust
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
```

- [ ] **Step 7: Rewrite `handle_ssh_status`**

Replace `handle_ssh_status` (lines 168-214). This now returns status for a specific connection, but also needs to check all connections for auto-reconnect:

```rust
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
```

- [ ] **Step 8: Add new SSH connection CRUD handlers**

Add these new methods after `handle_ssh_status`:

```rust
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
        data.ssh = None; // clear legacy
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

        // Remove connection config
        data.ssh_connections.retain(|c| c.id != id);
        data.ssh = None;

        // Count and remove associated forward rules
        let removed_rules: Vec<ForwardRule> = data.forward_rules.drain_filter(|r| r.ssh_connection_id == id).collect();
        let rule_count = removed_rules.len();

        // Disconnect if active
        let mut connections = self.ssh_connections.lock().unwrap();
        if let Some(mut ssh) = connections.remove(id) {
            ssh.disconnect();
        }

        // Remove associated proxy mappings
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
```

Note: `drain_filter` requires `Vec::drain_filter` — if not stable yet, use a manual approach:

```rust
        let removed_rules: Vec<ForwardRule> = data.forward_rules
            .iter()
            .filter(|r| r.ssh_connection_id == id)
            .cloned()
            .collect();
        data.forward_rules.retain(|r| r.ssh_connection_id != id);
```

- [ ] **Step 9: Update `handle_forward_pod` to use `ssh_connection_id`**

Replace `handle_forward_pod` (lines 277-344):

```rust
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

        let mut connections = self.ssh_connections.lock().unwrap();
        let ssh = connections.get_mut(connection_id)
            .ok_or_else(|| anyhow::anyhow!("SSH 连接不存在"))?;
        if !ssh.is_connected() {
            return Err(anyhow::anyhow!("SSH 未连接，请先连接所选的 SSH 服务器"));
        }
        let local_port = ssh.add_forward("127.0.0.1", &pod.ip, container_port, 0, connection_id)?;

        let domain = pod_name.to_string();
        let addr = format!("{}:{}", pod.ip, container_port);
        let rule_id = uuid::Uuid::new_v4().to_string();

        if let Some(ref p) = *self.proxy.lock().unwrap() {
            p.register(
                &domain,
                &format!("127.0.0.1:{}", local_port),
                &rule_id,
                false,
            );
            p.register(&addr, &format!("127.0.0.1:{}", local_port), &rule_id, true);
        }

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
```

- [ ] **Step 10: Update remaining handlers that use `self.ssh`**

Replace `handle_add_forward_rule` (lines 395-418):

```rust
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
```

Replace `handle_update_forward_rule` (lines 420-446):

```rust
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
```

Replace `handle_remove_forward_rule` (lines 448-461):

```rust
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
```

Replace `handle_import_rules` (lines 463-480):

```rust
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
```

Replace `handle_unforward_pod` (lines 527-540):

```rust
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
```

Replace `handle_validate_k8s_forwards` (lines 557-614):

```rust
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
```

Replace `handle_get_config` (lines 662-688):

```rust
    fn handle_get_config(&self) -> Result<Value> {
        let mut data = self.load_data()?;
        // Decrypt SSH connection passwords
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
        // Don't return legacy ssh field
        data.ssh = None;
        Ok(serde_json::to_value(data)?)
    }
```

- [ ] **Step 11: Update `destroy`**

Replace `destroy` (lines 720-728):

```rust
    fn destroy(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref mut proxy) = *self.proxy.lock().unwrap() {
            proxy.stop();
        }
        for (_, mut ssh) in self.ssh_connections.lock().unwrap().drain() {
            ssh.disconnect();
        }
        Ok(())
    }
```

- [ ] **Step 12: Update `handle_call` dispatch**

Replace the `match method` block (lines 742-772):

```rust
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
```

- [ ] **Step 13: Verify compilation**

Run: `cargo check -p k8s-forward 2>&1`
Expected: PASS (no errors). If `drain_filter` is not stable, use the manual `retain` + `cloned` approach from Step 8.

- [ ] **Step 14: Commit**

```bash
git add plugins/k8s-forward/src/lib.rs
git commit -m "feat(k8s-forward): rewrite backend for multi-SSH connection management"
```

---

### Task 4: Frontend — Update TypeScript types

**Files:**
- Modify: `plugins/k8s-forward/frontend/src/types.ts`

- [ ] **Step 1: Add `SshConnection` and update existing types**

Replace entire file content:

```typescript
declare global {
  interface Window {
    pluginAPI: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
      open_folder_dialog: (title?: string) => Promise<string | null>;
      write_file: (path: string, content: string) => Promise<void>;
    };
  }
}

export interface SshConnection {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  password: string;
}

export interface ForwardRule {
  id: string;
  name: string;
  local_host: string;
  local_port: number;
  remote_host: string;
  remote_port: number;
  rule_type: "Manual" | "K8s";
  ssh_connection_id: string;
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

export type SshConnectionState = "Connected" | "Disconnected" | "Reconnecting";

export interface ReconnectInfo {
  retry_count: number;
  max_retries: number;
  next_retry_at: number;
}

export interface SshStatus {
  connected: boolean;
  host?: string;
  port?: number;
  status: SshConnectionState;
  reconnect_info?: ReconnectInfo;
  connection_id?: string;
  connection_name?: string;
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

- [ ] **Step 2: Commit**

```bash
git add plugins/k8s-forward/frontend/src/types.ts
git commit -m "feat(k8s-forward): add SshConnection type and update frontend types"
```

---

### Task 5: Frontend — Rewrite `TabSshForward` with connection management

**Files:**
- Modify: `plugins/k8s-forward/frontend/src/components/TabSshForward.tsx`

- [ ] **Step 1: Rewrite component with connection list and SSH-filtered rules**

Replace entire file content:

```tsx
import { useState, useEffect, useCallback } from "react";
import type { ForwardRule, SshConnection, SshStatus } from "../types";

declare global {
  interface Window { WorkTools: { toast: { success(m:string):void; error(m:string):void; info(m:string):void; warning(m:string):void }; FieldError: { show(el:HTMLElement, m:string):void; clear(el:HTMLElement):void; clearAll(f:HTMLElement):void } } }
}

const PLUGIN_ID = "k8s-forward";

export default function TabSshForward() {
  const [connections, setConnections] = useState<SshStatus[]>([]);
  const [rules, setRules] = useState<ForwardRule[]>([]);
  const [selectedConnId, setSelectedConnId] = useState<string>("");
  const [editingRule, setEditingRule] = useState<ForwardRule | null>(null);
  const [isNewRule, setIsNewRule] = useState(false);
  const [editingConn, setEditingConn] = useState<(SshConnection & { id?: string }) | null>(null);

  const call = useCallback(async (method: string, params?: unknown) => {
    return await window.pluginAPI.call(PLUGIN_ID, method, (params ?? {}) as Record<string, unknown>);
  }, []);

  const loadConnections = async () => {
    const list = await call("ssh_list_connections") as SshStatus[];
    setConnections(list);
  };

  const loadRules = async () => {
    const r = await call("list_forward_rules") as ForwardRule[];
    setRules(r.filter(r => r.rule_type === "Manual"));
  };

  useEffect(() => {
    Promise.allSettled([loadConnections(), loadRules()]).then(() => {});
  }, []);

  // Poll status for reconnecting connections
  useEffect(() => {
    const hasReconnecting = connections.some(c => c.status === "Reconnecting");
    if (!hasReconnecting) return;
    const timer = setInterval(async () => {
      const list = await call("ssh_list_connections") as SshStatus[];
      setConnections(list);
    }, 5000);
    return () => clearInterval(timer);
  }, [connections]);

  const handleConnect = async (connectionId: string) => {
    try {
      await call("ssh_connect", { connection_id: connectionId });
      window.WorkTools.toast.success("SSH 连接成功");
      loadConnections();
    } catch (e: unknown) { window.WorkTools.toast.error(`连接失败: ${e}`); }
  };

  const handleDisconnect = async (connectionId: string) => {
    try {
      await call("ssh_disconnect", { connection_id: connectionId });
      window.WorkTools.toast.info("SSH 已断开");
      loadConnections();
    } catch (e: unknown) { window.WorkTools.toast.error(`断开失败: ${e}`); }
  };

  const handleSaveConnection = async () => {
    if (!editingConn) return;
    try {
      if (editingConn.id) {
        await call("ssh_update_connection", editingConn);
        window.WorkTools.toast.success("连接已更新");
      } else {
        const result = await call("ssh_add_connection", editingConn) as SshConnection;
        window.WorkTools.toast.success("连接已添加");
      }
      setEditingConn(null);
      loadConnections();
    } catch (e: unknown) { window.WorkTools.toast.error(`保存失败: ${e}`); }
  };

  const handleRemoveConnection = async (id: string) => {
    try {
      const result = await call("ssh_remove_connection", { id }) as { removed_rules: number };
      if (result.removed_rules > 0) {
        window.WorkTools.toast.info(`已删除连接及 ${result.removed_rules} 条关联规则`);
      } else {
        window.WorkTools.toast.success("连接已删除");
      }
      if (selectedConnId === id) setSelectedConnId("");
      loadConnections();
      loadRules();
    } catch (e: unknown) { window.WorkTools.toast.error(`删除失败: ${e}`); }
  };

  const handleAddRule = () => {
    if (!selectedConnId && connections.length > 0) {
      window.WorkTools.toast.warning("请先选择一个 SSH 连接");
      return;
    }
    const rule: ForwardRule = {
      id: window.crypto.randomUUID(),
      name: `rule-${Date.now()}`,
      local_host: "127.0.0.1",
      local_port: 0,
      remote_host: "",
      remote_port: 0,
      rule_type: "Manual" as const,
      ssh_connection_id: selectedConnId,
    };
    setEditingRule(rule);
    setIsNewRule(true);
  };

  const handleSaveRule = async () => {
    if (!editingRule) return;
    try {
      if (isNewRule) {
        await call("add_forward_rule", editingRule);
      } else {
        await call("update_forward_rule", editingRule);
      }
      window.WorkTools.toast.success(isNewRule ? "规则已添加" : "已保存");
      setEditingRule(null);
      setIsNewRule(false);
      loadRules();
    } catch (e: unknown) { window.WorkTools.toast.error(`保存失败: ${e}`); }
  };

  const handleDeleteRule = async (id: string) => {
    try {
      await call("remove_forward_rule", { id });
      window.WorkTools.toast.success("已删除");
      loadRules();
    } catch (e: unknown) { window.WorkTools.toast.error(`删除失败: ${e}`); }
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
        window.WorkTools.toast.success(`已导入 ${arr.length} 条规则`);
        loadRules();
      } catch { window.WorkTools.toast.error("导入失败: 格式错误"); }
    };
    input.click();
  };

  const handleExport = async () => {
    try {
      const dir = await window.pluginAPI.open_folder_dialog("选择导出目录");
      if (!dir) return;
      const data = await call("export_rules") as ForwardRule[];
      const json = JSON.stringify(data.filter(r => r.rule_type === "Manual"), null, 2);
      const filename = `k8s-forward-rules-${new Date().toISOString().split("T")[0]}.json`;
      const filePath = `${dir.replace(/\\/g, "/")}/${filename}`;
      await window.pluginAPI.write_file(filePath, json);
      window.WorkTools.toast.success(`已导出到 ${filePath}`);
    } catch (e: unknown) { window.WorkTools.toast.error(`导出失败: ${e}`); }
  };

  const getConnectionName = (connId: string) => {
    const conn = connections.find(c => c.connection_id === connId);
    return conn?.connection_name || connId;
  };

  const filteredRules = selectedConnId
    ? rules.filter(r => r.ssh_connection_id === selectedConnId)
    : rules;

  return (
    <div>
      <div className="card">
        <div className="card-header" style={{display:"flex",justifyContent:"space-between",alignItems:"center"}}>
          <span>SSH 连接管理</span>
          <button className="btn btn-primary btn-sm" onClick={() => setEditingConn({ name: "", host: "", port: 22, username: "", password: "" })}>+ 添加 SSH</button>
        </div>
        {connections.length === 0 ? (
          <div style={{textAlign:"center",color:"var(--text-tertiary)",padding:20}}>暂无 SSH 连接，点击右上角添加</div>
        ) : (
          <table>
            <thead><tr><th>名称</th><th>地址</th><th>状态</th><th>操作</th></tr></thead>
            <tbody>
              {connections.map(c => (
                <tr key={c.connection_id}>
                  <td>{c.connection_name}</td>
                  <td><code>{c.host}:{c.port}</code></td>
                  <td>
                    <span className={`status-dot ${
                      c.status === "Connected" ? "online" :
                      c.status === "Reconnecting" ? "reconnecting" :
                      "offline"
                    }`}></span>
                    {c.status === "Connected" && "已连接"}
                    {c.status === "Reconnecting" && `重连中 (${c.reconnect_info?.retry_count ?? 0}/${c.reconnect_info?.max_retries ?? 10})`}
                    {c.status === "Disconnected" && "未连接"}
                  </td>
                  <td style={{whiteSpace:"nowrap"}}>
                    {c.status === "Connected" ? (
                      <button className="btn btn-danger btn-sm" onClick={() => handleDisconnect(c.connection_id!)}>断开</button>
                    ) : c.status === "Reconnecting" ? (
                      <button className="btn btn-secondary btn-sm" disabled>重连中...</button>
                    ) : (
                      <button className="btn btn-primary btn-sm" onClick={() => handleConnect(c.connection_id!)}>连接</button>
                    )}
                    <button className="btn btn-secondary btn-sm" style={{marginLeft:4}} onClick={() => setEditingConn({ id: c.connection_id, name: c.connection_name || "", host: c.host || "", port: c.port || 22, username: "", password: "" })}>编辑</button>
                    <button className="btn btn-danger btn-sm" style={{marginLeft:4}} onClick={() => handleRemoveConnection(c.connection_id!)}>删除</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="card">
        <div className="card-header" style={{display:"flex",justifyContent:"space-between",alignItems:"center"}}>
          <span style={{display:"flex",alignItems:"center",gap:8}}>
            转发规则
            <select value={selectedConnId} onChange={e => setSelectedConnId(e.target.value)} style={{fontSize:12,padding:"2px 4px"}}>
              <option value="">全部连接</option>
              {connections.map(c => (
                <option key={c.connection_id} value={c.connection_id}>{c.connection_name}</option>
              ))}
            </select>
          </span>
          <div style={{display:"flex",gap:8}}>
            <button className="btn btn-primary btn-sm" onClick={handleAddRule}>+ 添加规则</button>
            <button className="btn btn-secondary btn-sm" onClick={handleImport}>导入</button>
            <button className="btn btn-secondary btn-sm" onClick={handleExport}>导出</button>
          </div>
        </div>
        <table>
          <thead>
            <tr>
              <th>名称</th>
              <th>本地地址</th>
              <th>本地端口</th>
              <th>远程地址</th>
              <th>远程端口</th>
              {!selectedConnId && <th>SSH连接</th>}
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            {filteredRules.map(r => (
              <tr key={r.id}>
                <td>{r.name}</td>
                <td>{r.local_host}</td>
                <td>{r.local_port}</td>
                <td>{r.remote_host}</td>
                <td>{r.remote_port}</td>
                {!selectedConnId && <td>{getConnectionName(r.ssh_connection_id)}</td>}
                <td>
                  <button className="btn btn-secondary btn-sm" onClick={() => { setEditingRule(r); setIsNewRule(false); }} style={{marginRight:4}}>编辑</button>
                  <button className="btn btn-danger btn-sm" onClick={() => handleDeleteRule(r.id)}>删除</button>
                </td>
              </tr>
            ))}
            {filteredRules.length === 0 && <tr><td colSpan={selectedConnId ? 6 : 7} style={{textAlign:"center",color:"var(--text-tertiary)",padding:20}}>暂无规则</td></tr>}
          </tbody>
        </table>
      </div>

      {editingConn && (
        <div className="modal-overlay" onClick={() => setEditingConn(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>{editingConn.id ? "编辑 SSH 连接" : "添加 SSH 连接"}</h3>
            <div className="form-row">
              <div className="form-group"><label>名称</label><input value={editingConn.name} onChange={e => setEditingConn({...editingConn, name: e.target.value})} placeholder="如：生产环境" /></div>
              <div className="form-group"><label>主机地址</label><input value={editingConn.host} onChange={e => setEditingConn({...editingConn, host: e.target.value})} placeholder="10.73.x.x" /></div>
              <div className="form-group"><label>端口</label><input type="number" value={editingConn.port} onChange={e => setEditingConn({...editingConn, port: +e.target.value})} /></div>
            </div>
            <div className="form-row">
              <div className="form-group"><label>用户名</label><input value={editingConn.username} onChange={e => setEditingConn({...editingConn, username: e.target.value})} /></div>
              <div className="form-group"><label>密码</label><input type="password" value={editingConn.password} onChange={e => setEditingConn({...editingConn, password: e.target.value})} placeholder={editingConn.id ? "留空则不修改" : ""} /></div>
            </div>
            <div className="modal-actions">
              <button className="btn btn-secondary" onClick={() => setEditingConn(null)}>取消</button>
              <button className="btn btn-primary" onClick={handleSaveConnection}>保存</button>
            </div>
          </div>
        </div>
      )}

      {editingRule && (
        <div className="modal-overlay" onClick={() => setEditingRule(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>编辑规则</h3>
            <div className="form-row">
              <div className="form-group"><label>名称</label><input value={editingRule.name} onChange={e => setEditingRule({...editingRule, name: e.target.value})} /></div>
              <div className="form-group"><label>本地地址</label><input value={editingRule.local_host} onChange={e => setEditingRule({...editingRule, local_host: e.target.value})} /></div>
              <div className="form-group"><label>本地端口</label><input type="number" value={editingRule.local_port} onChange={e => setEditingRule({...editingRule, local_port: +e.target.value})} /></div>
              <div className="form-group"><label>远程地址</label><input value={editingRule.remote_host} onChange={e => setEditingRule({...editingRule, remote_host: e.target.value})} /></div>
              <div className="form-group"><label>远程端口</label><input type="number" value={editingRule.remote_port} onChange={e => setEditingRule({...editingRule, remote_port: +e.target.value})} /></div>
            </div>
            <div className="modal-actions">
              <button className="btn btn-secondary" onClick={() => { setEditingRule(null); setIsNewRule(false); }}>取消</button>
              <button className="btn btn-primary" onClick={handleSaveRule}>保存</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd plugins/k8s-forward/frontend && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add plugins/k8s-forward/frontend/src/components/TabSshForward.tsx
git commit -m "feat(k8s-forward): rewrite TabSshForward with multi-SSH connection management UI"
```

---

### Task 6: Frontend — Update `TabK8sForward` with SSH connection selector

**Files:**
- Modify: `plugins/k8s-forward/frontend/src/components/TabK8sForward.tsx`

- [ ] **Step 1: Add SSH connection selector and update forward/unforward calls**

The key changes:
1. Add `connections` state and `loadConnections` function
2. Add SSH selector dropdown before Pod list
3. Pass `ssh_connection_id` to `forward_pod` call
4. Load SSH status per selected connection for reconnect banners
5. Show connection name in forwards table

Add new state and loader after existing state declarations (around line 23):

```tsx
  const [connections, setConnections] = useState<SshStatus[]>([]);
  const [selectedSshId, setSelectedSshId] = useState<string>("");
```

Add connection loading in init and standalone function:

```tsx
  const loadConnections = async () => {
    try {
      const list = await call("ssh_list_connections") as SshStatus[];
      setConnections(list);
    } catch { /* ignore */ }
  };
```

Add `loadConnections()` to the init useEffect's Promise.allSettled:

```tsx
      const results = await Promise.allSettled([loadStatus(), loadForwards(), loadConnections()]);
```

Replace `handleForward` (around line 128) to pass `ssh_connection_id`:

```tsx
  const handleForward = async (podName: string, containerName: string, containerPort: number) => {
    if (!selectedSshId) {
      window.WorkTools.toast.warning("请先选择一个 SSH 连接");
      return;
    }
    try {
      await call("forward_pod", { cluster: selCluster, namespace: selNs, pod_name: podName, container_name: containerName, container_port: containerPort, ssh_connection_id: selectedSshId });
      window.WorkTools.toast.success(`已转发 ${podName}/${containerName}:${containerPort}`);
      loadForwards();
    } catch (e: unknown) { window.WorkTools.toast.error(`转发失败: ${e}`); }
  };
```

Add SSH connection selector before the Pod list card (before the `<div className="card">` that has "Pod 列表"):

```tsx
          <div className="card">
            <div className="card-header">SSH 连接选择</div>
            <div className="form-row">
              <div className="form-group">
                <label>通过 SSH 转发</label>
                <select value={selectedSshId} onChange={e => setSelectedSshId(e.target.value)}>
                  <option value="">-- 选择 SSH 连接 --</option>
                  {connections.filter(c => c.connected).map(c => (
                    <option key={c.connection_id} value={c.connection_id}>{c.connection_name} ({c.host}:{c.port})</option>
                  ))}
                </select>
              </div>
            </div>
          </div>
```

Update the forwards table header to include SSH connection column (in the "已转发列表" table):

```tsx
<thead><tr><th>Pod名称</th><th>Pod地址</th><th>本地端口</th><th>目标</th><th>SSH连接</th><th>操作</th></tr></thead>
```

And add the connection name cell in each row:

```tsx
<td>{getConnectionName(r.ssh_connection_id)}</td>
```

Add helper function:

```tsx
  const getConnectionName = (connId: string) => {
    const conn = connections.find(c => c.connection_id === connId);
    return conn?.connection_name || "-";
  };
```

Also update the import at the top to include `SshStatus`:

```tsx
import type { KuboardStatus, PodInfo, K8sForwardInfo, ForwardRule, ProxyMapping, LoginResult, SshStatus } from "../types";
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd plugins/k8s-forward/frontend && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add plugins/k8s-forward/frontend/src/components/TabK8sForward.tsx
git commit -m "feat(k8s-forward): add SSH connection selector to K8s Pod forwarding"
```

---

### Task 7: Build and verify

**Files:** None (verification only)

- [ ] **Step 1: Build the plugin**

Run: `cargo build -p k8s-forward 2>&1`
Expected: PASS

- [ ] **Step 2: Build the frontend**

Run: `cd plugins/k8s-forward/frontend && npm run build`
Expected: PASS

- [ ] **Step 3: Full workspace check**

Run: `cargo check 2>&1`
Expected: PASS

- [ ] **Step 4: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "fix(k8s-forward): address build issues from multi-SSH refactor"
```
