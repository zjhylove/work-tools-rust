use serde::{Deserialize, Serialize};

/// 插件元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: String,
}