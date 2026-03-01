use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use worktools_plugin_api::Plugin;
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
#[derive(Debug, Serialize, Deserialize)]
struct PasswordData {
    entries: Vec<PasswordEntry>,
}

impl Default for PasswordData {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

/// 密码管理器插件
pub struct PasswordManager;

impl PasswordManager {
    /// 获取数据文件路径
    fn get_data_file_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("无法获取用户主目录"))?;

        let mut data_dir = std::path::PathBuf::from(home);
        data_dir.push(".worktools/history/plugins");

        // 创建目录(如果不存在)
        std::fs::create_dir_all(&data_dir)?;

        data_dir.push("password-manager.json");
        Ok(data_dir)
    }

    /// 加载数据
    fn load_data() -> Result<PasswordData> {
        let data_path = Self::get_data_file_path()?;

        if !data_path.exists() {
            return Ok(PasswordData::default());
        }

        let file = File::open(&data_path)?;
        let data: PasswordData = serde_json::from_reader(file)?;
        Ok(data)
    }

    /// 保存数据
    fn save_data(data: &PasswordData) -> Result<()> {
        let data_path = Self::get_data_file_path()?;

        // 读取现有配置以保留 salt 和 validation_token
        let existing_config = if data_path.exists() {
            let file = File::open(&data_path)?;
            serde_json::from_reader::<_, Value>(file).ok()
        } else {
            None
        };

        // 使用临时文件模式确保原子性写入
        let temp_path = data_path.with_extension("tmp");
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)?;

        // 合并数据:保留 salt 和 validation_token,更新 entries
        let mut output = serde_json::to_value(data)?;
        if let Some(config) = existing_config {
            // 保留 salt 和 validation_token
            if let Some(salt) = config.get("salt") {
                output["salt"] = salt.clone();
            }
            if let Some(validation_token) = config.get("validation_token") {
                output["validation_token"] = validation_token.clone();
            }
        }

        serde_json::to_writer_pretty(&file, &output)?;
        file.sync_all()?;

        // 原子性替换文件
        std::fs::rename(&temp_path, &data_path)?;

        Ok(())
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
        // 返回简单的 HTML 界面
        r#"
        <div id="password-manager-app">
            <div class="password-manager-container">
                <div class="toolbar">
                    <button onclick="window.pluginAPI.call('list_passwords')">刷新列表</button>
                    <button onclick="window.pluginAPI.call('add_password_ui')">添加密码</button>
                </div>
                <div id="password-list">加载中...</div>
            </div>
        </div>
        <script>
            // 简单的列表加载示例
            window.pluginAPI.call('list_passwords').then(result => {
                console.log('Passwords:', result);
            });
        </script>
        "#.to_string()
    }

    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "list_passwords" => {
                let data = Self::load_data()?;
                let entries: Vec<Value> = data.entries.into_iter().map(|entry| {
                    serde_json::json!({
                        "id": entry.id,
                        "url": entry.url.as_ref().unwrap_or(&String::new()),
                        "service": entry.service,
                        "username": entry.username,
                        "password": entry.password,
                        "created_at": entry.created_at,
                        "updated_at": entry.updated_at.as_ref().unwrap_or(&String::new()),
                    })
                }).collect();
                Ok(serde_json::json!({ "entries": entries }))
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
                    "url": entry.url.as_ref().unwrap_or(&String::new()),
                    "service": entry.service,
                    "username": entry.username,
                    "password": entry.password,
                    "created_at": entry.created_at,
                    "updated_at": entry.updated_at.as_ref().unwrap_or(&String::new()),
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
                    "url": entry.url.as_ref().unwrap_or(&String::new()),
                    "service": entry.service,
                    "username": entry.username,
                    "password": entry.password,
                    "created_at": entry.created_at,
                    "updated_at": entry.updated_at.as_ref().unwrap_or(&String::new()),
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
            _ => Err(format!("未知方法: {}", method).into()),
        }
    }
}

/// 插件工厂函数 - 导出给动态库加载器
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(PasswordManager));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
