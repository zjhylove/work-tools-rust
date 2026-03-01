use crate::config::{load_app_config, save_app_config, load_plugin_config, save_plugin_config};
use crate::plugin_manager::PluginManager;
use crate::crypto::{PasswordEncryptor, CryptoConfig};
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

/// 插件管理器状态
pub type PluginManagerState = Arc<PluginManager>;

/// 密码加密器状态 (使用 Arc<Mutex<>> 以支持跨线程共享)
pub type CryptoState = Arc<std::sync::Mutex<PasswordEncryptor>>;

/// 辅助函数: 从配置中加载密码条目列表
fn load_password_entries_from_config(config: &Value) -> Vec<PasswordEntry> {
    config
        .get("entries")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// 辅助函数: 将密码条目列表保存到配置
fn save_password_entries_to_config(
    entries: &[PasswordEntry],
    config: &mut Value,
) -> Result<(), String> {
    config["entries"] = serde_json::to_value(entries)
        .map_err(|e| format!("序列化条目失败: {}", e))?;
    Ok(())
}


/// 密码条目 (加密版本,存储在磁盘上)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PasswordEntry {
    pub id: String,
    pub url: Option<String>,
    pub service: String,
    pub username: String,
    /// 存储的是加密后的密码
    pub password: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 密码条目 (解密版本,返回给前端)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecryptedPasswordEntry {
    pub id: String,
    pub url: Option<String>,
    pub service: String,
    pub username: String,
    /// 解密后的明文密码
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

/// 获取所有密码条目 (解密版本)
#[tauri::command]
pub async fn get_password_entries(
    crypto_state: State<'_, CryptoState>,
) -> Result<Vec<DecryptedPasswordEntry>, String> {
    let config = load_plugin_config("password-manager")
        .map_err(|e| e.to_string())?;

    let entries: Vec<PasswordEntry> = config
        .get("entries")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    // 解密所有密码
    let encryptor = crypto_state.lock()
        .map_err(|e| format!("获取加密器失败: {}", e))?;

    let mut decrypted_entries = Vec::new();
    for entry in entries {
        match encryptor.decrypt_password(&entry.password) {
            Ok(decrypted_password) => {
                decrypted_entries.push(DecryptedPasswordEntry {
                    id: entry.id,
                    url: entry.url,
                    service: entry.service,
                    username: entry.username,
                    password: decrypted_password,
                    created_at: entry.created_at,
                    updated_at: entry.updated_at,
                });
            }
            Err(_e) => {
                // 旧格式的数据解密失败,跳过该条目
                continue;
            }
        }
    }

    Ok(decrypted_entries)
}

/// 保存密码条目 (加密密码后存储)
#[tauri::command]
pub async fn save_password_entry(
    entry: DecryptedPasswordEntry,
    crypto_state: State<'_, CryptoState>,
) -> Result<(), String> {
    // 加密密码
    let encryptor = crypto_state.lock()
        .map_err(|e| format!("获取加密器失败: {}", e))?;

    let encrypted_password = encryptor.encrypt_password(&entry.password)
        .map_err(|e| format!("加密密码失败: {}", e))?;

    // 创建加密后的条目用于存储
    let encrypted_entry = PasswordEntry {
        id: entry.id,
        url: entry.url,
        service: entry.service,
        username: entry.username,
        password: encrypted_password,
        created_at: entry.created_at,
        updated_at: entry.updated_at,
    };

    let mut config = load_plugin_config("password-manager")
        .map_err(|e| e.to_string())?;

    // 使用辅助函数加载条目
    let mut entries = load_password_entries_from_config(&config);

    // 查找并更新或添加新条目 (使用 filter)
    entries.retain(|e| e.id != encrypted_entry.id);
    entries.push(encrypted_entry);

    // 使用辅助函数保存条目
    save_password_entries_to_config(&entries, &mut config)?;

    // 保存加密配置 (master_password 和 salt)
    let crypto_config = encryptor.get_config();
    config["master_password"] = serde_json::to_value(&crypto_config.master_password)
        .map_err(|e| e.to_string())?;
    config["salt"] = serde_json::to_value(&crypto_config.salt)
        .map_err(|e| e.to_string())?;

    save_plugin_config("password-manager", &config)
        .map_err(|e| e.to_string())
}

/// 删除密码条目
#[tauri::command]
pub async fn delete_password_entry(id: String) -> Result<(), String> {
    let mut config = load_plugin_config("password-manager")
        .map_err(|e| e.to_string())?;

    // 使用辅助函数加载和过滤条目
    let mut entries = load_password_entries_from_config(&config);
    entries.retain(|e| e.id != id);

    // 使用辅助函数保存
    save_password_entries_to_config(&entries, &mut config)?;

    save_plugin_config("password-manager", &config)
        .map_err(|e| e.to_string())
}

/// 清空所有密码条目 (用于重置)
#[tauri::command]
pub async fn clear_all_password_entries() -> Result<(), String> {
    let mut config = load_plugin_config("password-manager")
        .map_err(|e| e.to_string())?;

    save_password_entries_to_config(&[], &mut config)?;

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

/// ============= 主密码管理命令 =============

/// 初始化或验证主密码
#[tauri::command]
pub async fn init_or_verify_master_password(
    password: String,
    crypto_state: State<'_, CryptoState>,
) -> Result<bool, String> {
    let mut encryptor = crypto_state.lock()
        .map_err(|e| format!("获取加密器失败: {}", e))?;

    encryptor.init_or_verify_master_password(&password)
        .map_err(|e| format!("主密码验证失败: {}", e))
}

/// 检查是否已设置主密码
#[tauri::command]
pub async fn has_master_password(
    crypto_state: State<'_, CryptoState>,
) -> Result<bool, String> {
    let encryptor = crypto_state.lock()
        .map_err(|e| format!("获取加密器失败: {}", e))?;

    Ok(encryptor.has_master_password())
}

/// 获取加密配置 (用于持久化)
#[tauri::command]
pub async fn get_crypto_config(
    crypto_state: State<'_, CryptoState>,
) -> Result<CryptoConfig, String> {
    let encryptor = crypto_state.lock()
        .map_err(|e| format!("获取加密器失败: {}", e))?;

    Ok(encryptor.get_config())
}

/// 从配置加载加密器
#[tauri::command]
pub async fn load_crypto_config(
    config: CryptoConfig,
    crypto_state: State<'_, CryptoState>,
) -> Result<(), String> {
    let mut encryptor = crypto_state.lock()
        .map_err(|e| format!("获取加密器失败: {}", e))?;

    // 创建新的加密器实例
    let new_encryptor = PasswordEncryptor::new(config);
    *encryptor = new_encryptor;

    Ok(())
}

/// 加密密码
#[tauri::command]
pub async fn encrypt_password(
    password: String,
    crypto_state: State<'_, CryptoState>,
) -> Result<String, String> {
    let encryptor = crypto_state.lock()
        .map_err(|e| format!("获取加密器失败: {}", e))?;

    encryptor.encrypt_password(&password)
        .map_err(|e| format!("加密失败: {}", e))
}

/// 解密密码
#[tauri::command]
pub async fn decrypt_password(
    encrypted_password: String,
    crypto_state: State<'_, CryptoState>,
) -> Result<String, String> {
    let encryptor = crypto_state.lock()
        .map_err(|e| format!("获取加密器失败: {}", e))?;

    encryptor.decrypt_password(&encrypted_password)
        .map_err(|e| format!("解密失败: {}", e))
}

/// 导出密码数据为 JSON 字符串
#[tauri::command]
pub async fn export_passwords() -> Result<String, String> {
    let config = load_plugin_config("password-manager")
        .map_err(|e| e.to_string())?;

    // 返回完整的配置 JSON (包括 master_password, salt 和 entries)
    let json_string = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化失败: {}", e))?;

    Ok(json_string)
}

/// 从 JSON 字符串导入密码数据
#[tauri::command]
pub async fn import_passwords(json_data: String) -> Result<(), String> {
    // 解析 JSON 数据
    let imported_config: serde_json::Value = serde_json::from_str(&json_data)
        .map_err(|e| format!("解析 JSON 失败: {}", e))?;

    // 获取当前的加密配置
    let mut current_config = load_plugin_config("password-manager")
        .map_err(|e| e.to_string())?;

    // 使用辅助函数加载条目
    let current_entries = load_password_entries_from_config(&current_config);
    let imported_entries = load_password_entries_from_config(&imported_config);

    // 使用 HashSet 优化 ID 查找,合并条目避免 ID 重复
    let existing_ids: std::collections::HashSet<String> = current_entries
        .iter()
        .map(|e| e.id.clone())
        .collect();

    let merged_entries: Vec<PasswordEntry> = current_entries
        .into_iter()
        .chain(imported_entries.into_iter().filter(|e| !existing_ids.contains(&e.id)))
        .collect();

    // 使用辅助函数保存
    save_password_entries_to_config(&merged_entries, &mut current_config)?;

    // 保存配置
    save_plugin_config("password-manager", &current_config)
        .map_err(|e| e.to_string())?;

    Ok(())
}
