//! # K8s 转发数据模型
//!
//! 定义了 K8s 端口转发插件所有的数据结构。
//!
//! ## Rust 知识点: serde 序列化控制
//! - `#[serde(default)]`: 反序列化时缺失字段使用 Default 值
//! - `#[serde(default = "fn_name")]`: 使用自定义函数提供默认值
//! - `#[serde(skip_serializing_if = "Option::is_none")]`: None 时不序列化该字段
//! - `#[serde(rename_all = "...")]`: 批量重命名字段

use serde::{Deserialize, Serialize};

/// 转发规则类型
/// `#[derive(PartialEq, Eq)]` 使枚举可以比较相等性（== / !=）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleType {
    Manual, // 手动创建的转发规则
    K8s,    // 通过 Kuboard 自动创建的 K8s 转发规则
}

/// 端口转发规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardRule {
    pub id: String,
    pub name: String,
    #[serde(default = "default_local_host")] // 默认 "127.0.0.1"
    pub local_host: String,
    pub local_port: u16,     // 本地监听端口
    pub remote_host: String, // 远程目标主机
    pub remote_port: u16,    // 远程目标端口
    pub rule_type: RuleType,
    // 以下字段仅在 K8s 类型时有值
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

/// HTTP 代理映射（域名 → 本地端口）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyMapping {
    pub domain: String,  // 请求的域名
    pub target: String,  // 转发目标地址（如 "127.0.0.1:10001"）
    pub rule_id: String, // 关联的转发规则 ID
    #[serde(default = "default_true")]
    pub editable: bool, // 是否可编辑（Pod 地址为 true，别名为 false）
}

fn default_true() -> bool {
    true
}

/// SSH 连接配置（加密存储密码）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16, // 默认为 SSH 标准端口 22
    pub username: String,
    pub password: String, // 加密后的密码
}

fn default_ssh_port() -> u16 {
    22
}

/// Kuboard 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KuboardConfig {
    pub url: String,
    pub username: String,
    pub password: String, // 加密后的密码
}

/// HTTP 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default = "default_proxy_port")]
    pub port: u16, // 默认为 80
}

fn default_proxy_port() -> u16 {
    80
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self { port: 80 }
    }
}

/// 插件持久化数据结构（顶层）
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

// ── K8s 实体 ──

/// 容器端口
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerPort {
    #[serde(default)]
    pub name: Option<String>,
    pub container_port: u16,
    #[serde(default = "default_protocol")]
    pub protocol: String, // TCP / UDP
}

fn default_protocol() -> String {
    "TCP".to_string()
}

/// 容器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub name: String,
    #[serde(default)]
    pub ports: Vec<ContainerPort>,
}

/// K8s Pod 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodInfo {
    pub name: String,
    pub ip: String,     // Pod 的集群内 IP
    pub status: String, // Running / Pending / Failed 等
    #[serde(default)]
    pub containers: Vec<ContainerInfo>,
}

// ── 状态类型 ──

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

/// Kuboard 登录结果
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_required: Option<bool>, // 是否需要 MFA 验证
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// K8s 转发信息汇总
#[derive(Debug, Serialize, Deserialize)]
pub struct K8sForwardInfo {
    pub rules: Vec<ForwardRule>,
    pub mappings: Vec<ProxyMapping>,
}
