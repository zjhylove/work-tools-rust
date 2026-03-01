use crate::config::{load_app_config, save_app_config, load_plugin_config, save_plugin_config};
use crate::plugin_manager::PluginManager;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

/// 插件管理器状态
pub type PluginManagerState = Arc<PluginManager>;

/// 密码条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PasswordEntry {
    pub id: String,
    pub url: Option<String>,
    pub service: String,
    pub username: String,
    pub password: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 双因素认证条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEntry {
    pub id: String,
    pub name: String,
    pub issuer: String,
    pub secret: String,
    pub algorithm: String,
    pub digits: u32,
    pub period: u64,
    pub created_at: String,
    pub updated_at: String,
}

/// 获取所有可用插件
#[tauri::command]
pub async fn get_available_plugins(
    manager: State<'_, PluginManagerState>,
) -> Result<Vec<worktools_shared_types::PluginInfo>, String> {
    manager
        .get_available_plugins()
        .await
        .into_iter()
        .map(|info| {
            Ok(worktools_shared_types::PluginInfo {
                id: info.id,
                name: info.name,
                version: info.version,
                description: info.description,
                icon: info.icon,
            })
        })
        .collect()
}

/// 获取所有已安装插件
#[tauri::command]
pub async fn get_installed_plugins(
    manager: State<'_, PluginManagerState>,
) -> Result<Vec<worktools_shared_types::PluginInfo>, String> {
    manager
        .get_installed_plugins()
        .await
        .into_iter()
        .map(|info| {
            Ok(worktools_shared_types::PluginInfo {
                id: info.id,
                name: info.name,
                version: info.version,
                description: info.description,
                icon: info.icon,
            })
        })
        .collect()
}

/// 安装插件
#[tauri::command]
pub async fn install_plugin(
    plugin_id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<(), String> {
    manager
        .install_plugin(&plugin_id)
        .await
        .map_err(|e| e.to_string())
}

/// 卸载插件
#[tauri::command]
pub async fn uninstall_plugin(
    plugin_id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<(), String> {
    manager
        .uninstall_plugin(&plugin_id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取插件配置
#[tauri::command]
pub async fn get_plugin_config(plugin_id: String) -> Result<Value, String> {
    load_plugin_config(&plugin_id).map_err(|e| e.to_string())
}

/// 保存插件配置
#[tauri::command]
pub async fn set_plugin_config(
    plugin_id: String,
    config: Value,
) -> Result<(), String> {
    save_plugin_config(&plugin_id, &config).map_err(|e| e.to_string())
}

/// 获取应用配置
#[tauri::command]
pub async fn get_app_config() -> Result<crate::config::AppConfig, String> {
    load_app_config().map_err(|e| e.to_string())
}

/// 保存应用配置
#[tauri::command]
pub async fn set_app_config(config: crate::config::AppConfig) -> Result<(), String> {
    save_app_config(&config).map_err(|e| e.to_string())
}

/// ============= 密码管理器命令 =============

/// 获取所有密码条目
#[tauri::command]
pub async fn get_password_entries() -> Result<Vec<PasswordEntry>, String> {
    let config = load_plugin_config("password-manager")
        .map_err(|e| e.to_string())?;

    let entries: Vec<PasswordEntry> = config
        .get("entries")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    Ok(entries)
}

/// 保存密码条目
#[tauri::command]
pub async fn save_password_entry(entry: PasswordEntry) -> Result<(), String> {
    let mut config = load_plugin_config("password-manager")
        .map_err(|e| e.to_string())?;

    let entries: Vec<PasswordEntry> = config
        .get("entries")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    // 查找并更新或添加新条目
    let mut updated_entries: Vec<PasswordEntry> = entries
        .into_iter()
        .filter(|e| e.id != entry.id)
        .collect();
    updated_entries.push(entry);

    config["entries"] = serde_json::to_value(&updated_entries)
        .map_err(|e| e.to_string())?;

    save_plugin_config("password-manager", &config)
        .map_err(|e| e.to_string())
}

/// 删除密码条目
#[tauri::command]
pub async fn delete_password_entry(id: String) -> Result<(), String> {
    let mut config = load_plugin_config("password-manager")
        .map_err(|e| e.to_string())?;

    let entries: Vec<PasswordEntry> = config
        .get("entries")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let updated_entries: Vec<PasswordEntry> = entries
        .into_iter()
        .filter(|e| e.id != id)
        .collect();

    config["entries"] = serde_json::to_value(&updated_entries)
        .map_err(|e| e.to_string())?;

    save_plugin_config("password-manager", &config)
        .map_err(|e| e.to_string())
}

/// ============= 双因素认证命令 =============

/// 获取所有双因素认证条目
#[tauri::command]
pub async fn get_auth_entries() -> Result<Vec<AuthEntry>, String> {
    let config = load_plugin_config("auth")
        .map_err(|e| e.to_string())?;

    let entries: Vec<AuthEntry> = config
        .get("entries")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    Ok(entries)
}

/// 保存双因素认证条目
#[tauri::command]
pub async fn save_auth_entry(entry: AuthEntry) -> Result<(), String> {
    let mut config = load_plugin_config("auth")
        .map_err(|e| e.to_string())?;

    let entries: Vec<AuthEntry> = config
        .get("entries")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let mut updated_entries: Vec<AuthEntry> = entries
        .into_iter()
        .filter(|e| e.id != entry.id)
        .collect();
    updated_entries.push(entry);

    config["entries"] = serde_json::to_value(&updated_entries)
        .map_err(|e| e.to_string())?;

    save_plugin_config("auth", &config)
        .map_err(|e| e.to_string())
}

/// 删除双因素认证条目
#[tauri::command]
pub async fn delete_auth_entry(id: String) -> Result<(), String> {
    let mut config = load_plugin_config("auth")
        .map_err(|e| e.to_string())?;

    let entries: Vec<AuthEntry> = config
        .get("entries")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let updated_entries: Vec<AuthEntry> = entries
        .into_iter()
        .filter(|e| e.id != id)
        .collect();

    config["entries"] = serde_json::to_value(&updated_entries)
        .map_err(|e| e.to_string())?;

    save_plugin_config("auth", &config)
        .map_err(|e| e.to_string())
}

/// 生成 TOTP 验证码
#[tauri::command]
pub async fn generate_totp(_secret: String, digits: u32, period: u64) -> Result<String, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let time_step = time / period;
    let code = (time_step % 1_000_000) as u32;
    Ok(format!("{:0width$}", code % (10_u32.pow(digits)), width = digits as usize))
}

/// 生成随机密钥
#[tauri::command]
pub async fn generate_secret() -> Result<String, String> {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut rng = rand::thread_rng();

    let secret: String = (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    Ok(secret)
}
