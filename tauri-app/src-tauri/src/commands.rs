use crate::config::{load_plugin_config, save_plugin_config};
use crate::logger::{LogEntry, LOG_RING};
use crate::plugin_manager::PluginManager;
use crate::plugin_package::{PluginManifest, PluginPackage};
use crate::plugin_registry::{InstalledPlugin, PluginRegistry};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::sync::Arc;
use tauri::State;

/// 插件管理器状态
pub type PluginManagerState = Arc<PluginManager>;

/// 获取所有已安装插件
#[tauri::command]
pub async fn get_installed_plugins(
    manager: State<'_, PluginManagerState>,
) -> Result<Vec<worktools_shared_types::PluginInfo>, String> {
    Ok(manager.get_installed_plugins().await)
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
        .inspect_err(|e| {
            tracing::error!(
                plugin_id = %plugin_id,
                method = %method,
                "调用插件方法失败: {}",
                e
            )
        })
        .map_err(|e| e.to_string())
}

/// 获取插件配置
#[tauri::command]
pub async fn get_plugin_config(plugin_id: String) -> Result<Value, String> {
    load_plugin_config(&plugin_id)
        .inspect_err(|e| {
            tracing::error!(plugin_id = %plugin_id, "读取插件配置失败: {}", e)
        })
        .map_err(|e| e.to_string())
}

/// 保存插件配置
#[tauri::command]
pub async fn set_plugin_config(plugin_id: String, config: Value) -> Result<(), String> {
    save_plugin_config(&plugin_id, &config)
        .inspect_err(|e| {
            tracing::error!(
                plugin_id = %plugin_id,
                config = ?config,
                "保存插件配置失败: {}",
                e
            )
        })
        .map_err(|e| e.to_string())
}

/// ============= 插件商店命令 =============

/// 导入插件包
#[tauri::command]
pub async fn import_plugin_package(
    file_path: String,
    manager: State<'_, PluginManagerState>,
) -> Result<String, String> {
    tracing::info!(file_path = %file_path, "开始导入插件包");

    // 1. 加载并验证插件包
    let pkg = PluginPackage::from_zip(std::path::Path::new(&file_path))
        .inspect_err(|e| {
            tracing::error!(
                file_path = %file_path,
                "加载插件包失败: {}",
                e
            )
        })
        .map_err(|e| format!("加载插件包失败: {}", e))?;

    pkg.validate()
        .inspect_err(|e| {
            tracing::error!(
                plugin_id = %pkg.manifest.id,
                "验证插件包失败: {}",
                e
            )
        })
        .map_err(|e| format!("插件包验证失败: {}", e))?;

    // 2. 创建插件目录
    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| {
            tracing::error!("获取用户主目录失败: 目录 API 不可用");
            "无法找到用户主目录".to_string()
        })?;

    let plugin_dir = user_dirs
        .home_dir()
        .join(".worktools/plugins")
        .join(&pkg.manifest.id);

    tracing::info!(plugin_dir = %plugin_dir.display(), "目标插件目录");

    // 3. 安装插件
    pkg.install(&plugin_dir)
        .inspect_err(|e| {
            tracing::error!(
                plugin_id = %pkg.manifest.id,
                plugin_dir = %plugin_dir.display(),
                "安装插件失败: {}",
                e
            )
        })
        .map_err(|e| format!("安装插件失败: {}", e))?;

    // 4. 获取动态库和资源路径
    let library_path = pkg
        .get_library_path(&plugin_dir)
        .inspect_err(|e| {
            tracing::error!(
                plugin_id = %pkg.manifest.id,
                "获取动态库路径失败: {}",
                e
            )
        })
        .map_err(|e| format!("获取动态库路径失败: {}", e))?;

    let assets_dir = pkg.get_assets_dir(&plugin_dir);

    // 5. 注册到插件注册表
    let mut registry = PluginRegistry::new()
        .inspect_err(|e| {
            tracing::error!("打开插件注册表失败: {}", e);
        })
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

    registry
        .register(installed_plugin)
        .inspect_err(|e| {
            tracing::error!(
                plugin_id = %pkg.manifest.id,
                "注册插件到注册表失败: {}",
                e
            )
        })
        .map_err(|e| format!("注册插件失败: {}", e))?;

    // 6. 重新加载插件管理器
    manager
        .init()
        .await
        .inspect_err(|e| {
            tracing::error!("重新加载插件管理器失败: {}", e);
        })
        .map_err(|e| format!("重新加载插件管理器失败: {}", e))?;

    tracing::info!(plugin_id = %pkg.manifest.id, "插件导入成功");

    Ok(format!("插件 {} 安装成功", pkg.manifest.name))
}

/// 获取所有可用插件 (已安装 + 可安装)
#[tauri::command]
pub async fn get_available_plugins() -> Result<Vec<PluginManifest>, String> {
    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| {
            tracing::error!("获取用户主目录失败: 目录 API 不可用");
            "无法找到用户主目录".to_string()
        })?;

    let plugins_dir = user_dirs.home_dir().join(".worktools/plugins");

    let mut plugins = Vec::new();

    if plugins_dir.exists() {
        let entries = fs::read_dir(&plugins_dir)
            .inspect_err(|e| {
                tracing::error!(
                    plugins_dir = %plugins_dir.display(),
                    "读取插件目录失败: {}",
                    e
                )
            })
            .map_err(|e| format!("读取插件目录失败: {}", e))?;

        for entry in entries {
            let entry = entry
                .inspect_err(|e| {
                    tracing::error!("读取目录项失败: {}", e);
                })
                .map_err(|e| format!("读取目录项失败: {}", e))?;

            let path = entry.path();

            if path.is_dir() {
                let manifest_path = path.join("manifest.json");
                if manifest_path.exists() {
                    let content = fs::read_to_string(&manifest_path)
                        .inspect_err(|e| {
                            tracing::error!(
                                manifest_path = %manifest_path.display(),
                                "读取 manifest.json 失败: {}",
                                e
                            )
                        })
                        .map_err(|e| format!("读取 manifest.json 失败: {}", e))?;

                    let manifest: PluginManifest = serde_json::from_str(&content)
                        .inspect_err(|e| {
                            tracing::error!(
                                manifest_path = %manifest_path.display(),
                                "解析 manifest.json 失败: {}",
                                e
                            )
                        })
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
        .inspect_err(|e| {
            tracing::error!("打开插件注册表失败: {}", e);
        })
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    Ok(registry.get_installed())
}

/// 安装插件 (如果插件包已解压到插件目录)
#[tauri::command]
pub async fn install_plugin(
    plugin_id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<String, String> {
    tracing::info!(plugin_id = %plugin_id, "开始安装插件");

    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| {
            tracing::error!("获取用户主目录失败: 目录 API 不可用");
            "无法找到用户主目录".to_string()
        })?;

    let plugin_dir = user_dirs
        .home_dir()
        .join(".worktools/plugins")
        .join(&plugin_id);

    let manifest_path = plugin_dir.join("manifest.json");
    if !manifest_path.exists() {
        tracing::error!(
            plugin_id = %plugin_id,
            plugin_dir = %plugin_dir.display(),
            "插件 manifest.json 不存在"
        );
        return Err("插件未找到".to_string());
    }

    // 读取 manifest
    let content = fs::read_to_string(&manifest_path)
        .inspect_err(|e| {
            tracing::error!(
                manifest_path = %manifest_path.display(),
                "读取 manifest.json 失败: {}",
                e
            )
        })
        .map_err(|e| format!("读取 manifest.json 失败: {}", e))?;

    let manifest: PluginManifest = serde_json::from_str(&content)
        .inspect_err(|e| {
            tracing::error!(
                manifest_path = %manifest_path.display(),
                "解析 manifest.json 失败: {}",
                e
            )
        })
        .map_err(|e| format!("解析 manifest.json 失败: {}", e))?;

    // 获取动态库路径
    let lib_name = manifest
        .get_library_filename()
        .ok_or_else(|| "未找到动态库配置".to_string())?;

    let library_path = plugin_dir.join(lib_name);
    let assets_dir = plugin_dir.join("assets");

    // 注册到注册表
    let mut registry = PluginRegistry::new()
        .inspect_err(|e| {
            tracing::error!("打开插件注册表失败: {}", e);
        })
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

    registry
        .register(installed_plugin)
        .inspect_err(|e| {
            tracing::error!(
                plugin_id = %manifest.id,
                "注册插件到注册表失败: {}",
                e
            )
        })
        .map_err(|e| format!("注册插件失败: {}", e))?;

    // 重新加载插件管理器
    manager
        .init()
        .await
        .inspect_err(|e| {
            tracing::error!("重新加载插件管理器失败: {}", e);
        })
        .map_err(|e| format!("重新加载插件管理器失败: {}", e))?;

    tracing::info!(plugin_id = %manifest.id, "插件安装成功");

    Ok(format!("插件 {} 安装成功", manifest.name))
}

/// 卸载插件
#[tauri::command]
pub async fn uninstall_plugin(
    plugin_id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<String, String> {
    tracing::info!(plugin_id = %plugin_id, "开始卸载插件");

    // 1. 先从内存中卸载插件, 释放 DLL 文件锁 (Windows 必须先释放才能删除)
    manager
        .unload_plugin(&plugin_id)
        .await
        .inspect_err(|e| {
            tracing::error!(
                plugin_id = %plugin_id,
                "从内存卸载插件失败: {}",
                e
            )
        })
        .map_err(|e| format!("卸载插件失败: {}", e))?;

    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| {
            tracing::error!("获取用户主目录失败: 目录 API 不可用");
            "无法找到用户主目录".to_string()
        })?;

    let plugins_base_dir = user_dirs.home_dir().join(".worktools/plugins");

    // 2. 删除插件目录 (DLL 已释放, Windows 上可以正常删除)
    let plugin_dir = plugins_base_dir.join(&plugin_id);

    let mut deleted_dir = false;
    if plugin_dir.exists() {
        // Windows 上 DLL 释放可能有短暂延迟, 加入重试
        let delete_result = remove_dir_with_retry(&plugin_dir, 3);
        if let Err(e) = delete_result {
            tracing::error!(
                plugin_dir = %plugin_dir.display(),
                "删除插件目录失败: {}",
                e
            );
            return Err(format!("删除插件目录失败: {}", e));
        }
        deleted_dir = true;
        tracing::info!("删除插件目录: {:?}", plugin_dir);
    } else {
        // 如果标准路径不存在,扫描所有子目录查找匹配的 manifest.json
        if plugins_base_dir.exists() {
            let entries = fs::read_dir(&plugins_base_dir)
                .inspect_err(|e| {
                    tracing::error!(
                        plugins_dir = %plugins_base_dir.display(),
                        "读取插件目录失败: {}",
                        e
                    )
                })
                .map_err(|e| format!("读取插件目录失败: {}", e))?;

            for entry in entries {
                let entry = entry
                    .inspect_err(|e| {
                        tracing::error!("读取目录项失败: {}", e);
                    })
                    .map_err(|e| format!("读取目录项失败: {}", e))?;
                let path = entry.path();

                if path.is_dir() {
                    let manifest_path = path.join("manifest.json");
                    if manifest_path.exists() {
                        // 读取 manifest.json 检查 ID 是否匹配
                        if let Ok(content) = fs::read_to_string(&manifest_path) {
                            if let Ok(manifest) =
                                serde_json::from_str::<serde_json::Value>(&content)
                            {
                                if manifest
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .map(|id| id == plugin_id)
                                    .unwrap_or(false)
                                {
                                    // 找到匹配的插件目录,删除它
                                    let delete_result = remove_dir_with_retry(&path, 3);
                                    if let Err(e) = delete_result {
                                        tracing::error!(
                                            plugin_dir = %path.display(),
                                            "删除插件目录失败: {}",
                                            e
                                        );
                                        return Err(format!("删除插件目录失败: {}", e));
                                    }
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

    // 3. 从注册表移除
    let mut registry = PluginRegistry::new()
        .inspect_err(|e| {
            tracing::error!("打开插件注册表失败: {}", e);
        })
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    registry
        .unregister(&plugin_id)
        .inspect_err(|e| {
            tracing::error!(
                plugin_id = %plugin_id,
                "从注册表移除插件失败: {}",
                e
            )
        })
        .map_err(|e| format!("从注册表移除插件失败: {}", e))?;

    tracing::info!(plugin_id = %plugin_id, "插件卸载成功");

    Ok(format!("插件 {} 卸载成功", plugin_id))
}

/// 带重试的目录删除 (Windows DLL 释放可能有短暂延迟)
fn remove_dir_with_retry(path: &std::path::Path, max_retries: u32) -> std::io::Result<()> {
    let mut last_err = fs::remove_dir_all(path);
    for attempt in 1..=max_retries {
        match &last_err {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    attempt,
                    "删除目录失败, 重试中...: {}",
                    e
                );
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200 * attempt as u64));
        last_err = fs::remove_dir_all(path);
    }
    last_err
}

/// 读取插件前端资源内容
#[tauri::command]
pub async fn read_plugin_asset(plugin_id: String, asset_path: String) -> Result<String, String> {
    let registry = PluginRegistry::new()
        .inspect_err(|e| {
            tracing::error!("打开插件注册表失败: {}", e);
        })
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    let plugin = registry
        .get(&plugin_id)
        .ok_or_else(|| {
            tracing::error!(
                plugin_id = %plugin_id,
                "插件未安装"
            );
            format!("插件未安装: {}", plugin_id)
        })?;

    // 构建完整的文件路径
    let full_path = plugin.assets_path.join(&asset_path);

    // 读取文件内容
    let content = std::fs::read_to_string(&full_path)
        .inspect_err(|e| {
            tracing::error!(
                asset_path = %asset_path,
                full_path = %full_path.display(),
                "读取资源文件失败: {}",
                e
            )
        })
        .map_err(|e| format!("读取资源文件失败: {}", e))?;

    Ok(content)
}

/// 打开外部 URL
#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    // 使用 opener crate 打开 URL (跨平台)
    opener::open(&url)
        .inspect_err(|e| {
            tracing::error!(url = %url, "打开 URL 失败: {}", e);
        })
        .map_err(|e| format!("打开链接失败: {}", e))
}

/// 写入文本文件到指定路径
#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    tracing::info!(path = %path, size = content.len(), "写入文件");
    fs::write(&path, &content)
        .inspect_err(|e| {
            tracing::error!(path = %path, "写入文件失败: {}", e);
        })
        .map_err(|e| format!("写入文件失败: {}", e))
}

/// 打开文件夹选择对话框
#[tauri::command]
pub async fn open_folder_dialog(
    title: Option<String>,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let mut builder = app.dialog().file();

    if let Some(title) = title {
        builder = builder.set_title(title);
    }

    let folder_path = builder.blocking_pick_folder();

    Ok(folder_path.map(|p| p.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub level: Option<String>,
    pub plugin: Option<String>,
    pub since: Option<String>,
}

#[tauri::command]
pub fn get_logs(query: Option<LogQuery>) -> Result<Vec<LogEntry>, String> {
    const DEFAULT_LIMIT: usize = 100;

    let ring = LOG_RING.lock().map_err(|e| format!("Lock error: {}", e))?;

    let entries: Vec<LogEntry> = ring
        .iter()
        .rev()
        .filter(|e| match &query {
            Some(q) => {
                if let Some(ref lvl) = q.level {
                    if e.level != *lvl {
                        return false;
                    }
                }
                if let Some(ref plugin) = q.plugin {
                    if !e.target.to_lowercase().contains(&plugin.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref since_str) = q.since {
                    if let Ok(since_dt) = chrono::DateTime::parse_from_rfc3339(since_str) {
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&e.timestamp) {
                            if dt <= since_dt {
                                return false;
                            }
                        }
                    }
                }
                true
            }
            None => true,
        })
        .take(DEFAULT_LIMIT)
        .cloned()
        .collect();

    Ok(entries)
}

#[tauri::command]
pub fn clear_logs() -> Result<(), String> {
    let mut ring = LOG_RING.lock().map_err(|e| format!("Lock error: {}", e))?;
    ring.clear();
    Ok(())
}
