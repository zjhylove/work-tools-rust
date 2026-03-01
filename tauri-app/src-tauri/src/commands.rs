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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
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

/// 获取插件视图
#[tauri::command]
pub async fn get_plugin_view(
    plugin_id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<worktools_shared_types::ViewSchema, String> {
    let result = manager
        .call_plugin_method(&plugin_id, "get_view", serde_json::json!({}))
        .await
        .map_err(|e| e.to_string())?;

    serde_json::from_value(result)
        .map_err(|e| format!("解析视图失败: {}", e))
}

/// 初始化插件
#[tauri::command]
pub async fn init_plugin(
    plugin_id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<serde_json::Value, String> {
    manager
        .call_plugin_method(&plugin_id, "init", serde_json::json!({}))
        .await
        .map_err(|e| e.to_string())
}

/// 调用插件方法
#[tauri::command]
pub async fn call_plugin_method(
    plugin_id: String,
    method: String,
    params: serde_json::Value,
    manager: State<'_, PluginManagerState>,
) -> Result<serde_json::Value, String> {
    manager
        .call_plugin_method(&plugin_id, &method, params)
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

/// 获取所有密码条目 (解密版本) - 调用插件
#[tauri::command]
pub async fn get_password_entries(
    crypto_state: State<'_, CryptoState>,
    manager: State<'_, PluginManagerState>,
) -> Result<Vec<DecryptedPasswordEntry>, String> {
    // 调用插件的 list_passwords 方法
    let result = manager
        .call_plugin_method("password-manager", "list_passwords", serde_json::json!({}))
        .await
        .map_err(|e| format!("调用插件失败: {}", e))?;

    // 解析响应
    let entries_value = result.get("entries")
        .ok_or_else(|| "插件返回格式错误".to_string())?;

    let entries: Vec<PasswordEntry> = serde_json::from_value(entries_value.clone())
        .map_err(|e| format!("解析条目失败: {}", e))?;

    // 解密所有密码 (在 await 之前完成)
    let encryptor = crypto_state.lock()
        .map_err(|e| format!("获取加密器失败: {}", e))?;

    // 检查是否已验证主密码(cipher 是否存在)
    if !encryptor.has_cipher() {
        return Err("主密码验证失败".to_string());
    }

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
                // 解密失败,可能是旧版本的明文密码,直接使用原始值
                // 这样用户可以看到旧数据,重新保存后会使用新加密
                decrypted_entries.push(DecryptedPasswordEntry {
                    id: entry.id,
                    url: entry.url,
                    service: entry.service,
                    username: entry.username,
                    password: entry.password.clone(), // 使用原始密码(可能是明文)
                    created_at: entry.created_at,
                    updated_at: entry.updated_at,
                });
            }
        }
    }

    Ok(decrypted_entries)
}

/// 保存密码条目 (加密密码后存储) - 调用插件
#[tauri::command]
pub async fn save_password_entry(
    entry: DecryptedPasswordEntry,
    crypto_state: State<'_, CryptoState>,
    manager: State<'_, PluginManagerState>,
) -> Result<(), String> {
    // 加密密码 (在 await 之前完成)
    let encrypted_password = {
        let encryptor = crypto_state.lock()
            .map_err(|e| format!("获取加密器失败: {}", e))?;
        encryptor.encrypt_password(&entry.password)
            .map_err(|e| format!("加密密码失败: {}", e))?
    };

    // 判断是新增还是更新
    let params = if entry.id.is_empty() {
        // 新增
        serde_json::json!({
            "service": entry.service,
            "username": entry.username,
            "password": encrypted_password,
            "url": entry.url,
        })
    } else {
        // 更新
        serde_json::json!({
            "id": entry.id,
            "service": entry.service,
            "username": entry.username,
            "password": encrypted_password,
            "url": entry.url,
        })
    };

    // 调用插件方法
    let method = if entry.id.is_empty() { "add_password" } else { "update_password" };

    manager
        .call_plugin_method("password-manager", method, params)
        .await
        .map_err(|e| format!("调用插件失败: {}", e))?;

    Ok(())
}

/// 删除密码条目 - 调用插件
#[tauri::command]
pub async fn delete_password_entry(
    id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<(), String> {
    manager
        .call_plugin_method("password-manager", "delete_password", serde_json::json!({ "id": id }))
        .await
        .map_err(|e| format!("调用插件失败: {}", e))?;

    Ok(())
}

/// 清空所有密码条目 (用于重置)
#[tauri::command]
pub async fn clear_all_password_entries(
    manager: State<'_, PluginManagerState>,
) -> Result<(), String> {
    manager
        .call_plugin_method("password-manager", "clear_all_passwords", serde_json::json!({}))
        .await
        .map_err(|e| format!("调用插件失败: {}", e))?;

    Ok(())
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

    let result = encryptor.init_or_verify_master_password(&password)
        .map_err(|e| format!("主密码验证失败: {}", e))?;

    // 验证成功后,保存 salt 和 validation_token 到磁盘
    // 这样应用重启后可以验证主密码,但不存储主密码本身
    if result {
        let config = encryptor.get_config();

        // 保存到 password-manager 的配置文件
        let mut plugin_config = load_plugin_config("password-manager")
            .unwrap_or_else(|_| serde_json::json!({}));

        // 保存 salt 和 validation_token
        plugin_config["salt"] = serde_json::to_value(&config.salt)
            .map_err(|e| format!("序列化盐值失败: {}", e))?;
        plugin_config["validation_token"] = serde_json::to_value(&config.validation_token)
            .map_err(|e| format!("序列化验证令牌失败: {}", e))?;

        // 移除 master_password(如果存在旧数据)
        plugin_config.as_object_mut()
            .map(|obj| obj.remove("master_password"));

        save_plugin_config("password-manager", &plugin_config)
            .map_err(|e| format!("保存加密配置失败: {}", e))?;
    }

    Ok(result)
}

/// 检查是否已设置主密码
#[tauri::command]
pub async fn has_master_password(
    crypto_state: State<'_, CryptoState>,
) -> Result<bool, String> {
    let encryptor = crypto_state.lock()
        .map_err(|e| format!("获取加密器失败: {}", e))?;

    // 检查是否有 salt 存在(说明已经设置过主密码)
    let has_salt = encryptor.get_config().salt.is_some();

    // 或者检查配置文件
    let has_salt_in_config = if let Ok(config) = load_plugin_config("password-manager") {
        config.get("salt").is_some()
    } else {
        false
    };

    Ok(has_salt || has_salt_in_config)
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

/// 导出密码数据为 JSON 字符串 - 调用插件
#[tauri::command]
pub async fn export_passwords(
    manager: State<'_, PluginManagerState>,
) -> Result<String, String> {
    let result = manager
        .call_plugin_method("password-manager", "export_passwords", serde_json::json!({}))
        .await
        .map_err(|e| format!("调用插件失败: {}", e))?;

    let data = result.get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "插件返回格式错误".to_string())?;

    Ok(data.to_string())
}

/// 从 JSON 字符串导入密码数据 - 调用插件
#[tauri::command]
pub async fn import_passwords(
    json_data: String,
    manager: State<'_, PluginManagerState>,
) -> Result<(), String> {
    manager
        .call_plugin_method("password-manager", "import_passwords", serde_json::json!({ "data": json_data }))
        .await
        .map_err(|e| format!("调用插件失败: {}", e))?;

    Ok(())
}

/// ============= Auth Plugin 命令 (通过插件 Manager 调用) =============

/// 获取所有双因素认证条目
#[tauri::command]
pub async fn list_auth_entries(
    manager: State<'_, PluginManagerState>,
) -> Result<Vec<AuthEntry>, String> {
    let manager = manager.inner();
    let result = manager
        .call_plugin_method("auth", "list_entries", serde_json::json!({}))
        .await
        .map_err(|e| format!("调用插件失败: {}", e))?;

    let entries = result
        .get("entries")
        .and_then(|v: &serde_json::Value| serde_json::from_value::<Vec<AuthEntry>>(v.clone()).ok())
        .ok_or_else(|| format!("解析插件响应失败"))?;

    Ok(entries)
}

/// 添加双因素认证条目
#[tauri::command]
pub async fn add_auth_entry(
    entry: AuthEntry,
    manager: State<'_, PluginManagerState>,
) -> Result<AuthEntry, String> {
    let manager = manager.inner();
    let result = manager
        .call_plugin_method("auth", "add_entry", serde_json::to_value(entry).unwrap())
        .await
        .map_err(|e| format!("调用插件失败: {}", e))?;

    let added_entry = serde_json::from_value(result)
        .map_err(|e| format!("解析插件响应失败: {}", e))?;

    Ok(added_entry)
}

/// 更新双因素认证条目
#[tauri::command]
pub async fn update_auth_entry(
    entry: AuthEntry,
    manager: State<'_, PluginManagerState>,
) -> Result<AuthEntry, String> {
    let manager = manager.inner();
    let result = manager
        .call_plugin_method("auth", "update_entry", serde_json::to_value(entry).unwrap())
        .await
        .map_err(|e| format!("调用插件失败: {}", e))?;

    let updated_entry = serde_json::from_value(result)
        .map_err(|e| format!("解析插件响应失败: {}", e))?;

    Ok(updated_entry)
}

/// 删除双因素认证条目
#[tauri::command]
pub async fn delete_auth_entry_plugin(
    id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<(), String> {
    let manager = manager.inner();
    manager
        .call_plugin_method("auth", "delete_entry", serde_json::json!({ "id": id }))
        .await
        .map_err(|e| format!("调用插件失败: {}", e))?;

    Ok(())
}

/// 通过插件生成 TOTP 验证码
#[tauri::command]
pub async fn generate_totp_code(
    secret: String,
    digits: u32,
    period: u64,
    manager: State<'_, PluginManagerState>,
) -> Result<String, String> {
    let manager = manager.inner();
    let result = manager
        .call_plugin_method(
            "auth",
            "generate_totp",
            serde_json::json!({
                "secret": secret,
                "digits": digits,
                "period": period
            }),
        )
        .await
        .map_err(|e| format!("调用插件失败: {}", e))?;

    let code = result
        .get("code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("解析验证码失败"))?;

    Ok(code.to_string())
}
