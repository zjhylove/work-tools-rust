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
