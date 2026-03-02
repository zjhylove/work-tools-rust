use crate::config::{load_app_config, save_app_config, load_plugin_config, save_plugin_config};
use crate::plugin_manager::PluginManager;
use crate::plugin_package::{PluginPackage, PluginManifest};
use crate::plugin_registry::{PluginRegistry, InstalledPlugin};
use crate::crypto::PasswordEncryptor;
use serde_json::Value;
use std::fs;
use std::sync::Arc;
use tauri::State;

/// 插件管理器状态
pub type PluginManagerState = Arc<PluginManager>;

/// 密码加密器状态 (使用 Arc<Mutex<>> 以支持跨线程共享)
pub type CryptoState = Arc<std::sync::Mutex<PasswordEncryptor>>;

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

/// 获取所有已安装插件
#[tauri::command]
pub async fn get_installed_plugins(
    manager: State<'_, PluginManagerState>,
) -> Result<Vec<worktools_shared_types::PluginInfo>, String> {
    Ok(manager.get_installed_plugins().await)
}

/// 获取插件视图 (返回 HTML 字符串)
#[tauri::command]
pub async fn get_plugin_view(
    plugin_id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<String, String> {
    manager.get_plugin_view(&plugin_id).await.map_err(|e| e.to_string())
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

/// 获取所有密码条目 (自动解密) - 调用插件
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

    // 自动解密所有密码
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
                // 解密失败,可能是旧版本的明文密码,直接使用原始值
                decrypted_entries.push(DecryptedPasswordEntry {
                    id: entry.id,
                    url: entry.url,
                    service: entry.service,
                    username: entry.username,
                    password: entry.password.clone(),
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

/// ============= 加密辅助命令 (用于调试和迁移) =============

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

/// ============= 插件商店命令 =============

/// 导入插件包
#[tauri::command]
pub async fn import_plugin_package(
    file_path: String,
    manager: State<'_, PluginManagerState>,
) -> Result<String, String> {
    // 1. 加载并验证插件包
    let pkg = PluginPackage::from_zip(std::path::Path::new(&file_path))
        .map_err(|e| format!("加载插件包失败: {}", e))?;

    pkg.validate()
        .map_err(|e| format!("插件包验证失败: {}", e))?;

    // 2. 创建插件目录
    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| "无法找到用户主目录".to_string())?;
    let plugin_dir = user_dirs.home_dir()
        .join(".worktools/plugins")
        .join(&pkg.manifest.id);

    // 3. 安装插件
    pkg.install(&plugin_dir)
        .map_err(|e| format!("安装插件失败: {}", e))?;

    // 4. 获取动态库和资源路径
    let library_path = pkg.get_library_path(&plugin_dir)
        .map_err(|e| format!("获取动态库路径失败: {}", e))?;

    let assets_dir = pkg.get_assets_dir(&plugin_dir);

    // 5. 注册到插件注册表
    let mut registry = PluginRegistry::new()
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    let installed_plugin = InstalledPlugin {
        id: pkg.manifest.id.clone(),
        name: pkg.manifest.name.clone(),
        description: pkg.manifest.description.clone(),
        version: pkg.manifest.version.clone(),
        icon: pkg.manifest.icon.clone(),
        author: pkg.manifest.author.clone(),
        homepage: pkg.manifest.homepage.clone(),
        installed_at: chrono::Utc::now(),
        enabled: true,
        assets_path: assets_dir.clone(),
        library_path: library_path.clone(),
    };

    registry.register(installed_plugin)
        .map_err(|e| format!("注册插件失败: {}", e))?;

    // 6. 重新加载插件管理器
    manager.init().await
        .map_err(|e| format!("重新加载插件管理器失败: {}", e))?;

    Ok(format!("插件 {} 安装成功", pkg.manifest.name))
}

/// 获取所有可用插件 (已安装 + 可安装)
#[tauri::command]
pub async fn get_available_plugins() -> Result<Vec<PluginManifest>, String> {
    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| "无法找到用户主目录".to_string())?;

    let plugins_dir = user_dirs.home_dir()
        .join(".worktools/plugins");

    let mut plugins = Vec::new();

    if plugins_dir.exists() {
        let entries = fs::read_dir(&plugins_dir)
            .map_err(|e| format!("读取插件目录失败: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                let manifest_path = path.join("manifest.json");
                if manifest_path.exists() {
                    let content = fs::read_to_string(&manifest_path)
                        .map_err(|e| format!("读取 manifest.json 失败: {}", e))?;

                    let manifest: PluginManifest = serde_json::from_str(&content)
                        .map_err(|e| format!("解析 manifest.json 失败: {}", e))?;

                    plugins.push(manifest);
                }
            }
        }
    }

    Ok(plugins)
}

/// 获取已安装插件列表 (从注册表)
#[tauri::command]
pub async fn get_installed_plugins_from_registry() -> Result<Vec<InstalledPlugin>, String> {
    let registry = PluginRegistry::new()
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    Ok(registry.get_installed())
}

/// 安装插件 (如果插件包已解压到插件目录)
#[tauri::command]
pub async fn install_plugin(
    plugin_id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<String, String> {
    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| "无法找到用户主目录".to_string())?;

    let plugin_dir = user_dirs.home_dir()
        .join(".worktools/plugins")
        .join(&plugin_id);

    let manifest_path = plugin_dir.join("manifest.json");
    if !manifest_path.exists() {
        return Err("插件未找到".to_string());
    }

    // 读取 manifest
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("读取 manifest.json 失败: {}", e))?;

    let manifest: PluginManifest = serde_json::from_str(&content)
        .map_err(|e| format!("解析 manifest.json 失败: {}", e))?;

    // 获取动态库路径
    let lib_name = manifest.files.macos.as_ref()
        .or_else(|| manifest.files.linux.as_ref())
        .or_else(|| manifest.files.windows.as_ref())
        .ok_or_else(|| "未找到动态库配置".to_string())?;

    let library_path = plugin_dir.join(lib_name);
    let assets_dir = plugin_dir.join("assets");

    // 注册到注册表
    let mut registry = PluginRegistry::new()
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    let installed_plugin = InstalledPlugin {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        description: manifest.description.clone(),
        version: manifest.version.clone(),
        icon: manifest.icon.clone(),
        author: manifest.author.clone(),
        homepage: manifest.homepage.clone(),
        installed_at: chrono::Utc::now(),
        enabled: true,
        assets_path: assets_dir,
        library_path,
    };

    registry.register(installed_plugin)
        .map_err(|e| format!("注册插件失败: {}", e))?;

    // 重新加载插件管理器
    manager.init().await
        .map_err(|e| format!("重新加载插件管理器失败: {}", e))?;

    Ok(format!("插件 {} 安装成功", manifest.name))
}

/// 卸载插件
#[tauri::command]
pub async fn uninstall_plugin(
    plugin_id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<String, String> {
    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| "无法找到用户主目录".to_string())?;

    let plugins_base_dir = user_dirs.home_dir().join(".worktools/plugins");

    // 首先尝试直接删除 plugin_id 对应的目录
    let plugin_dir = plugins_base_dir.join(&plugin_id);

    let mut deleted_dir = false;
    if plugin_dir.exists() {
        fs::remove_dir_all(&plugin_dir)
            .map_err(|e| format!("删除插件目录失败: {}", e))?;
        deleted_dir = true;
        tracing::info!("删除插件目录: {:?}", plugin_dir);
    } else {
        // 如果标准路径不存在,扫描所有子目录查找匹配的 manifest.json
        if plugins_base_dir.exists() {
            let entries = fs::read_dir(&plugins_base_dir)
                .map_err(|e| format!("读取插件目录失败: {}", e))?;

            for entry in entries {
                let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
                let path = entry.path();

                if path.is_dir() {
                    let manifest_path = path.join("manifest.json");
                    if manifest_path.exists() {
                        // 读取 manifest.json 检查 ID 是否匹配
                        if let Ok(content) = fs::read_to_string(&manifest_path) {
                            if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
                                if manifest.get("id")
                                    .and_then(|v| v.as_str())
                                    .map(|id| id == plugin_id)
                                    .unwrap_or(false)
                                {
                                    // 找到匹配的插件目录,删除它
                                    fs::remove_dir_all(&path)
                                        .map_err(|e| format!("删除插件目录失败: {}", e))?;
                                    deleted_dir = true;
                                    tracing::info!("删除插件目录(扫描找到): {:?}", path);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !deleted_dir {
        tracing::warn!("未找到插件 {} 的目录", plugin_id);
    }

    // 从注册表移除
    let mut registry = PluginRegistry::new()
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    registry.unregister(&plugin_id)
        .map_err(|e| format!("从注册表移除插件失败: {}", e))?;

    // 重新加载插件管理器
    manager.init().await
        .map_err(|e| format!("重新加载插件管理器失败: {}", e))?;

    Ok(format!("插件 {} 卸载成功", plugin_id))
}

/// 获取插件前端资源 URL
#[tauri::command]
pub async fn get_plugin_assets_url(plugin_id: String) -> Result<String, String> {
    let registry = PluginRegistry::new()
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    let plugin = registry.get(&plugin_id)
        .ok_or_else(|| format!("插件未安装: {}", plugin_id))?;

    Ok(plugin.assets_path.to_string_lossy().to_string())
}

/// 读取插件前端资源内容
#[tauri::command]
pub async fn read_plugin_asset(plugin_id: String, asset_path: String) -> Result<String, String> {
    let registry = PluginRegistry::new()
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    let plugin = registry.get(&plugin_id)
        .ok_or_else(|| format!("插件未安装: {}", plugin_id))?;

    // 构建完整的文件路径
    let full_path = plugin.assets_path.join(&asset_path);

    // 读取文件内容
    let content = std::fs::read_to_string(&full_path)
        .map_err(|e| format!("读取资源文件失败: {}", e))?;

    Ok(content)
}

