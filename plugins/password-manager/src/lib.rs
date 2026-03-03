use anyhow::Result;
use serde::{Deserialize, Serialize};
use worktools_plugin_api::{Plugin, storage::PluginStorage};
use serde_json::Value;

/// 密码条目 (加密版本)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    pub id: String,
    pub url: Option<String>,
    pub service: String,
    pub username: String,
    pub password: String, // 存储已加密的密码
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// 数据存储结构
#[derive(Debug, Default, Serialize, Deserialize)]
struct PasswordData {
    entries: Vec<PasswordEntry>,
}

/// 密码管理器插件
pub struct PasswordManager;

impl PasswordManager {
    /// 获取数据存储实例
    fn storage() -> PluginStorage {
        PluginStorage::new("password-manager", "password-manager.json")
    }

    /// 加载数据
    fn load_data() -> Result<PasswordData> {
        Self::storage().load_json()
    }

    /// 保存数据
    fn save_data(data: &PasswordData) -> Result<()> {
        Self::storage().save_json_preserving(data, &["salt", "validation_token"])
    }
}

impl Plugin for PasswordManager {
    fn id(&self) -> &str {
        "password-manager"
    }

    fn name(&self) -> &str {
        "密码管理器"
    }

    fn description(&self) -> &str {
        "本地安全存储和管理密码"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn icon(&self) -> &str {
        "🔐"
    }

    fn get_view(&self) -> String {
        // 插件已迁移到使用独立前端资源 (assets/index.html)
        // 此方法仅作为向后兼容的占位符
        "<div>插件前端资源加载中...</div>".to_string()
    }

    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "list_passwords" => {
                let data = Self::load_data()?;
                let entries: Vec<Value> = data.entries.into_iter().map(|entry| {
                    serde_json::json!({
                        "id": entry.id,
                        "url": entry.url.as_deref().unwrap_or_default(),
                        "service": entry.service,
                        "username": entry.username,
                        "password": entry.password,
                        "created_at": entry.created_at,
                        "updated_at": entry.updated_at.as_deref().unwrap_or_default(),
                    })
                }).collect();
                // 直接返回数组,而不是包装在对象中
                Ok(serde_json::to_value(entries)?)
            }
            "add_password" => {
                let service = params.get("service")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 service 参数"))?;

                let username = params.get("username")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 username 参数"))?;

                let password = params.get("password")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 password 参数"))?;

                let url = params.get("url").and_then(|v| v.as_str());

                let entry = PasswordEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    url: url.map(|s| s.to_string()),
                    service: service.to_string(),
                    username: username.to_string(),
                    password: password.to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: None,
                };

                let mut data = Self::load_data()?;
                data.entries.push(entry.clone());
                Self::save_data(&data)?;

                Ok(serde_json::json!({
                    "id": entry.id,
                    "url": entry.url.as_deref().unwrap_or_default(),
                    "service": entry.service,
                    "username": entry.username,
                    "password": entry.password,
                    "created_at": entry.created_at,
                    "updated_at": entry.updated_at.as_deref().unwrap_or_default(),
                }))
            }
            "update_password" => {
                let id = params.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 id 参数"))?;

                let service = params.get("service")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 service 参数"))?;

                let username = params.get("username")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 username 参数"))?;

                let password = params.get("password")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 password 参数"))?;

                let url = params.get("url").and_then(|v| v.as_str());

                let mut data = Self::load_data()?;
                let index = data.entries
                    .iter()
                    .position(|e| e.id == id)
                    .ok_or_else(|| anyhow::anyhow!("密码条目不存在"))?;

                let created_at = data.entries[index].created_at.clone();

                let entry = PasswordEntry {
                    id: id.to_string(),
                    url: url.map(|s| s.to_string()),
                    service: service.to_string(),
                    username: username.to_string(),
                    password: password.to_string(),
                    created_at,
                    updated_at: Some(chrono::Utc::now().to_rfc3339()),
                };

                data.entries[index] = entry.clone();
                Self::save_data(&data)?;

                Ok(serde_json::json!({
                    "id": entry.id,
                    "url": entry.url.as_deref().unwrap_or_default(),
                    "service": entry.service,
                    "username": entry.username,
                    "password": entry.password,
                    "created_at": entry.created_at,
                    "updated_at": entry.updated_at.as_deref().unwrap_or_default(),
                }))
            }
            "delete_password" => {
                let id = params.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 id 参数"))?;

                let mut data = Self::load_data()?;
                let index = data.entries
                    .iter()
                    .position(|e| e.id == id)
                    .ok_or_else(|| anyhow::anyhow!("密码条目不存在"))?;

                data.entries.remove(index);
                Self::save_data(&data)?;

                Ok(serde_json::json!({ "success": true }))
            }
            "clear_all_passwords" => {
                let mut data = Self::load_data()?;
                data.entries.clear();
                Self::save_data(&data)?;
                Ok(serde_json::json!({ "success": true }))
            }
            "export_passwords" => {
                let data = Self::load_data()?;
                let json = serde_json::to_string_pretty(&data)?;
                Ok(serde_json::json!({ "data": json }))
            }
            "import_passwords" => {
                let json_data = params.get("data")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 data 参数"))?;

                let imported_data: PasswordData = serde_json::from_str(json_data)?;
                let mut data = Self::load_data()?;

                for entry in imported_data.entries {
                    if !data.entries.iter().any(|e| e.id == entry.id) {
                        data.entries.push(entry);
                    }
                }

                Self::save_data(&data)?;
                Ok(serde_json::json!({ "success": true }))
            }
            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

/// 插件工厂函数 - 导出给动态库加载器
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(PasswordManager));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
