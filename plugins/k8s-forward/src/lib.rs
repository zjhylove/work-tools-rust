use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Mutex;
use tokio::runtime::Runtime;
use worktools_plugin_api::*;
use worktools_plugin_api::storage::PluginStorage;

pub mod crypto;
pub mod http_proxy;
pub mod kuboard_client;
pub mod models;
pub mod ssh_service;

use models::*;
use crypto::PasswordEncryptor;
use kuboard_client::KuboardClient;
use ssh_service::SshService;
use http_proxy::HttpProxySvc;

pub struct K8sForwardPlugin {
    storage: PluginStorage,
    encryptor: PasswordEncryptor,
    runtime: Runtime,
    ssh: Mutex<SshService>,
    proxy: Mutex<Option<HttpProxySvc>>,
    kuboard: Mutex<Option<KuboardClient>>,
}

impl K8sForwardPlugin {
    pub fn new() -> Self {
        Self {
            storage: PluginStorage::new("k8s-forward", "k8s-forward.json"),
            encryptor: PasswordEncryptor::new(),
            runtime: Runtime::new().expect("Failed to create tokio runtime"),
            ssh: Mutex::new(SshService::new()),
            proxy: Mutex::new(None),
            kuboard: Mutex::new(None),
        }
    }

    fn load_data(&self) -> Result<PluginData> {
        self.storage.load_json::<PluginData>()
    }

    fn save_data(&self, data: &PluginData) -> Result<()> {
        self.storage.save_json(data)
    }

    // ── SSH ──

    fn handle_ssh_connect(&self, params: &Value) -> Result<Value> {
        let host = get_str(params, "host")?;
        let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16;
        let username = get_str(params, "username")?;
        let password = get_str(params, "password")?;

        let mut ssh = self.ssh.lock().unwrap();
        ssh.connect(host, port, username, password)?;

        let mut data = self.load_data()?;

        let mut restored = 0;
        for rule in data.forward_rules.iter_mut() {
            match ssh.add_forward(
                &rule.local_host, &rule.remote_host, rule.remote_port, rule.local_port) {
                Ok(assigned) => {
                    if rule.local_port == 0 {
                        rule.local_port = assigned;
                    }
                    restored += 1;
                }
                Err(e) => {
                    eprintln!("恢复转发规则失败 [{}]: {}", rule.name, e);
                }
            }
        }

        let enc_pwd = self.encryptor.encrypt(password)?;
        data.ssh = Some(SshConfig {
            host: host.to_string(),
            port,
            username: username.to_string(),
            password: enc_pwd,
        });
        self.save_data(&data)?;

        Ok(json!({"success": true, "message": format!("SSH 连接成功，已恢复 {} 条转发规则", restored)}))
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
        let mut rule: ForwardRule = serde_json::from_value(params.clone())?;
        let mut data = self.load_data()?;

        let mut ssh = self.ssh.lock().unwrap();
        if ssh.is_connected() && rule.rule_type == RuleType::Manual {
            let assigned = ssh.add_forward(
                &rule.local_host, &rule.remote_host, rule.remote_port, rule.local_port)?;
            if rule.local_port == 0 {
                rule.local_port = assigned;
            }
        }
        if rule.id.is_empty() {
            rule.id = uuid::Uuid::new_v4().to_string();
        }

        data.forward_rules.push(rule.clone());
        self.save_data(&data)?;
        Ok(serde_json::to_value(&rule)?)
    }

    fn handle_update_forward_rule(&self, params: &Value) -> Result<Value> {
        let updated: ForwardRule = serde_json::from_value(params.clone())?;
        let mut data = self.load_data()?;
        if let Some(rule) = data.forward_rules.iter_mut().find(|r| r.id == updated.id) {
            let mut ssh = self.ssh.lock().unwrap();
            ssh.remove_forward(rule.local_port)?;
            if ssh.is_connected() {
                let assigned = ssh.add_forward(
                    &updated.local_host, &updated.remote_host, updated.remote_port, updated.local_port)?;
                let mut saved = updated.clone();
                if saved.local_port == 0 {
                    saved.local_port = assigned;
                }
                *rule = saved;
            } else {
                *rule = updated.clone();
            }
            let result = serde_json::to_value(&*rule)?;
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

        let mut ssh = self.ssh.lock().unwrap();
        if !ssh.is_connected() {
            return Err(anyhow::anyhow!("SSH 未连接"));
        }
        let local_port = ssh.add_forward("127.0.0.1", &remote_host, remote_port, 0)?;

        let domain = pod_name.to_string();
        let addr = format!("{}:{}", remote_host, remote_port);
        let rule_id = uuid::Uuid::new_v4().to_string();

        let proxy = self.proxy.lock().unwrap();
        if let Some(ref p) = *proxy {
            p.register(&domain, &format!("127.0.0.1:{}", local_port), &rule_id, false);
            p.register(&addr, &format!("127.0.0.1:{}", local_port), &rule_id, true);
        }

        let rule = ForwardRule {
            id: rule_id,
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
            domain: addr,
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

    fn handle_validate_k8s_forwards(&self) -> Result<Value> {
        let kuboard_guard = self.kuboard.lock().unwrap();
        let client = kuboard_guard.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Kuboard 未登录"))?;

        let mut data = self.load_data()?;

        let mut ns_map: std::collections::HashMap<(String, String), Vec<(usize, String)>> =
            std::collections::HashMap::new();
        for (i, r) in data.forward_rules.iter().enumerate() {
            if r.rule_type != RuleType::K8s { continue; }
            let cluster = r.cluster.clone().unwrap_or_default();
            let namespace = r.namespace.clone().unwrap_or_default();
            let pod_name = r.pod_name.clone().unwrap_or_default();
            ns_map.entry((cluster, namespace)).or_default().push((i, pod_name));
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
                        let valid = pods.iter().any(|p| p.name == *pod_name && p.status == "Running");
                        if !valid { to_remove.push(*idx); }
                    }
                }
                Err(_) => {
                    /* K8s API 不可达，跳过此命名空间的验证 */
                }
            }
        }

        to_remove.sort_unstable();
        to_remove.dedup();
        to_remove.reverse();

        let mut ssh = self.ssh.lock().unwrap();
        let proxy_guard = self.proxy.lock().unwrap();
        for idx in &to_remove {
            let rule = data.forward_rules.remove(*idx);
            let _ = ssh.remove_forward(rule.local_port);
            if let Some(ref proxy) = *proxy_guard {
                proxy.unregister_by_rule_id(&rule.id);
            }
        }
        self.save_data(&data)?;

        Ok(json!({"removed": to_remove.len()}))
    }

    // ── HTTP 代理 ──

    fn handle_proxy_start(&self, params: &Value) -> Result<Value> {
        let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
        let mut proxy = HttpProxySvc::new(port);

        let data = self.load_data()?;
        for rule in &data.forward_rules {
            if rule.rule_type == RuleType::K8s {
                let domain = rule.pod_name.as_deref().unwrap_or("");
                let addr = format!("{}:{}", rule.remote_host, rule.remote_port);
                proxy.register(domain,
                    &format!("127.0.0.1:{}", rule.local_port),
                    &rule.id, false);
                proxy.register(&addr,
                    &format!("127.0.0.1:{}", rule.local_port),
                    &rule.id, true);
            }
        }

        self.runtime.block_on(proxy.start())?;

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
            mapping_count: guard.as_ref().map(|p| {
                let ms = p.list_mappings();
                let mut ids = std::collections::HashSet::new();
                for m in &ms { ids.insert(m.rule_id.clone()); }
                ids.len()
            }).unwrap_or(0),
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
        let mut data = self.load_data()?;
        if let Some(ref ssh_cfg) = data.ssh {
            if let Ok(pwd) = self.encryptor.decrypt(&ssh_cfg.password) {
                data.ssh = Some(SshConfig { password: pwd, ..ssh_cfg.clone() });
            }
        }
        if let Some(ref kb_cfg) = data.kuboard {
            if let Ok(pwd) = self.encryptor.decrypt(&kb_cfg.password) {
                data.kuboard = Some(KuboardConfig { password: pwd, ..kb_cfg.clone() });
            }
        }
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
    fn icon(&self) -> &str { "\u{1F310}" }
    fn get_view(&self) -> String { "<div>插件前端资源加载中...</div>".to_string() }

    fn destroy(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 停止 HTTP 代理
        if let Some(ref mut proxy) = *self.proxy.lock().unwrap() {
            proxy.stop();
        }
        // 断开 SSH（停止所有转发线程 + join 等待所有 handler 线程结束）
        self.ssh.lock().unwrap().disconnect();
        Ok(())
    }

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

#[cfg(test)]
mod forwarding_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpStream, TcpListener};
    use std::sync::{Arc, Mutex, mpsc};
    use std::thread;
    use std::time::Duration;

    /// 最小化测试：不用 SshService，直接用 ssh2 测试 channel_direct_tcpip
    #[test]
    fn test_ssh_channel_direct() {
        let encryptor = PasswordEncryptor::new();
        let pwd = encryptor.decrypt("0bab0259a6236b6a70eb2c9afdc3bc22").unwrap();

        // Connect SSH
        let tcp = TcpStream::connect("10.73.64.28:10022").unwrap();
        let mut session = ssh2::Session::new().unwrap();
        session.set_tcp_stream(tcp);
        session.handshake().unwrap();
        session.userauth_password("lbscheck", &pwd).unwrap();
        assert!(session.authenticated());
        println!("[1] SSH connected");

        // Create direct-tcpip channel to redis
        let mut channel = session.channel_direct_tcpip("10.73.70.213", 6379, None)
            .expect("Failed to create channel to 10.73.70.213:6379");
        println!("[2] Channel created to 10.73.70.213:6379");

        // Send PING
        channel.write_all(b"PING\r\n").unwrap();
        println!("[3] Sent PING");

        // Read response
        let mut buf = [0u8; 1024];
        let n = channel.read(&mut buf).unwrap();
        let response = String::from_utf8_lossy(&buf[..n]);
        println!("[4] Response: {}", response.trim());
        assert!(response.contains("PONG"), "Expected PONG, got: {}", response);

        channel.send_eof().unwrap();
        println!("[5] Test passed - SSH direct channel works!");
    }

    /// 模拟 SshService listener 线程的完整行为
    #[test]
    fn test_listener_with_ssh_channel() {
        let encryptor = PasswordEncryptor::new();
        let pwd = encryptor.decrypt("0bab0259a6236b6a70eb2c9afdc3bc22").unwrap();

        // 连接 SSH
        let tcp = TcpStream::connect("10.73.64.28:10022").unwrap();
        let mut session = ssh2::Session::new().unwrap();
        session.set_tcp_stream(tcp);
        session.handshake().unwrap();
        session.userauth_password("lbscheck", &pwd).unwrap();
        let session = Arc::new(Mutex::new(session));
        println!("[1] SSH connected");

        // 先做一次 channel 测试（模拟 add_forward 的测试 channel）
        {
            let s = session.lock().unwrap();
            let _ch = s.channel_direct_tcpip("10.73.70.213", 6379, None).unwrap();
            drop(_ch);
            println!("[2] Test channel OK");
        }

        // 模拟 listener 线程
        let stop = Arc::new(Mutex::new(false));
        let stop_clone = stop.clone();
        let session_clone = session.clone();
        let (ready_tx, ready_rx) = mpsc::channel();

        thread::spawn(move || {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            println!("[listener] Bound to port {}", port);
            ready_tx.send(port).unwrap();
            listener.set_nonblocking(true).unwrap();

            loop {
                if *stop_clone.lock().unwrap() {
                    println!("[listener] Stop flag set, exiting");
                    return;
                }
                let stream = match listener.accept() {
                    Ok((s, _)) => s,
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    Err(e) => {
                        println!("[listener] Accept error: {}", e);
                        return;
                    }
                };
                println!("[listener] Connection accepted");

                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut loc_read = stream.try_clone().unwrap();
                let mut loc_write = stream;

                let s = session_clone.lock().unwrap();
                let mut channel = match s.channel_direct_tcpip("10.73.70.213", 6379, None) {
                    Ok(c) => c,
                    Err(e) => { println!("[listener] Channel error: {}", e); continue; }
                };
                println!("[listener] Channel created");

                let _ = std::io::copy(&mut loc_read, &mut channel);
                println!("[listener] Copy loc->ch done");
                let _ = channel.send_eof();
                println!("[listener] EOF sent");
                let _ = std::io::copy(&mut channel, &mut loc_write);
                println!("[listener] Copy ch->loc done");
            }
        });

        // 等待 listener 就绪
        let port = ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        println!("[3] Listener ready on port {}", port);

        // 连接并发数据
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        stream.write_all(b"PING\r\n").unwrap();
        println!("[4] Sent PING");

        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf).unwrap();
        let response = String::from_utf8_lossy(&buf[..n]);
        println!("[5] Response: {}", response.trim());
        assert!(response.contains("PONG") || response.contains("NOAUTH"));

        *stop.lock().unwrap() = true;
        println!("[6] Test passed!");
    }

    /// 最简化测试：验证 listener 能绑定和 accept
    #[test]
    fn test_listener_bind_and_accept() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        println!("Listener on port {}", port);
        listener.set_nonblocking(true).unwrap();

        // Accept should return WouldBlock immediately
        match listener.accept() {
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                println!("accept WouldBlock - expected");
            }
            other => panic!("Unexpected accept result: {:?}", other),
        }

        // Connect from another thread
        let (tx, rx) = std::sync::mpsc::channel();
        let port2 = port;
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            let mut s = TcpStream::connect(format!("127.0.0.1:{}", port2)).unwrap();
            s.write_all(b"hello").unwrap();
            tx.send(()).unwrap();
        });

        // Accept the connection
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(s) => break s,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(e) => panic!("accept error: {}", e),
            }
        };
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
        stream.set_nonblocking(false).unwrap();

        let mut buf = [0u8; 10];
        let n = stream.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"hello");
        println!("Listener test passed - received: {:?}", &buf[..n]);
    }

    /// 端到端测试：SshService 本地端口转发
    #[test]
    fn test_ssh_forward_end_to_end() {
        let encryptor = PasswordEncryptor::new();
        let pwd = encryptor.decrypt("0bab0259a6236b6a70eb2c9afdc3bc22").unwrap();

        let mut ssh = SshService::new();
        ssh.connect("10.73.64.28", 10022, "lbscheck", &pwd).unwrap();
        assert!(ssh.is_connected());
        println!("[1] SSH connected");

        // 使用自动分配端口来测试完整流程
        let local_port = ssh.add_forward("127.0.0.1", "10.73.70.213", 6379, 0).unwrap();
        println!("[2] Forward added: localhost:{} -> 10.73.70.213:6379 (auto port)", local_port);

        // 轮询等待 listener 就绪
        println!("[3] Waiting for listener on port {}...", local_port);
        for i in 0..20 {
            thread::sleep(Duration::from_millis(500));
            match TcpStream::connect_timeout(
                &format!("127.0.0.1:{}", local_port).parse().unwrap(),
                Duration::from_millis(500),
            ) {
                Ok(s) => { println!("[3] Connected on attempt {}", i+1); drop(s); break; }
                Err(_) if i < 19 => continue,
                Err(e) => panic!("Listener never ready on port {} after 10s: {:?}", local_port, e),
            }
        }

        // 重新连接并发数据
        thread::sleep(Duration::from_millis(500));
        let mut stream = TcpStream::connect_timeout(
            &format!("127.0.0.1:{}", local_port).parse().unwrap(),
            Duration::from_secs(10),
        ).expect("Second connect failed");

        stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
        stream.write_all(b"PING\r\n").unwrap();
        println!("[4] Sent PING");

        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf).unwrap();
        let response = String::from_utf8_lossy(&buf[..n]);
        println!("[5] Response ({} bytes): {}", n, response.trim());
        assert!(response.contains("PONG") || response.contains("NOAUTH"), "Unexpected: {}", response);

        ssh.disconnect();
        println!("[6] Test passed!");
    }
}
