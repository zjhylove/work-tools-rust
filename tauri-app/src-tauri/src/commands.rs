//! # Tauri 命令
//!
//! 这个文件定义了所有前端可调用的后端函数。
//! 每个标记了 `#[tauri::command]` 的函数都会自动暴露给前端 JavaScript。
//!
//! ## Rust 知识点
//! - `#[tauri::command]`: Tauri 的过程宏（procedural macro），自动生成 IPC 处理代码
//! - `State<'_, T>`: Tauri 的依赖注入 — 从应用状态中提取类型为 T 的值
//! - `async fn`: 异步函数，返回 `impl Future`，由 Tauri 的异步运行时执行
//! - `Result<T, String>`: Tauri 要求的返回类型，错误必须是 String
//!
//! ## 数据流
//! ```
//! 前端 JavaScript (iframe)
//!   → window.pluginAPI.call(pluginId, method, params)
//!   → Tauri IPC (invoke)
//!   → #[tauri::command] fn call_plugin_method(...)
//!   → PluginManager::call_plugin_method(...)
//!   → Plugin::handle_call(method, params)
//!   → 返回 JSON
//! ```

use crate::config::{load_plugin_config, save_plugin_config};
use crate::logger::{LogEntry, LOG_RING};
use crate::plugin_manager::PluginManager;
use crate::plugin_package::{PluginManifest, PluginPackage};
use crate::plugin_registry::{InstalledPlugin, PluginRegistry};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::sync::Arc;
use tauri::{Manager, State};

/// 插件管理器状态的类型别名
/// `State<'_, PluginManagerState>` 比 `State<'_, Arc<PluginManager>>` 更简洁
pub type PluginManagerState = Arc<PluginManager>;

/// 获取所有已安装插件
///
/// ## Rust 知识点: #[tauri::command]
/// 这个属性宏自动：
/// 1. 生成序列化/反序列化代码（参数和返回值通过 JSON 传递）
/// 2. 将函数注册到 Tauri 的 IPC 路由表中
/// 3. 处理异步执行
#[tauri::command]
pub async fn get_installed_plugins(
    manager: State<'_, PluginManagerState>,
) -> Result<Vec<worktools_shared_types::PluginInfo>, String> {
    Ok(manager.get_installed_plugins().await)
}

/// 调用插件方法
///
/// 这是插件系统最核心的 API — 前端通过此函数调用任何插件的方法。
///
/// ## Rust 知识点: 属性宏中的命名参数
/// `tracing::error!(plugin_id = %plugin_id, ...)` 使用 `%` 前缀表示 Display 格式。
/// `?` 前缀表示 Debug 格式。这是 tracing 的结构化日志语法。
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
        // `inspect_err` 在 Result 为 Err 时执行闭包，但不改变 Result
        // 这比 match 或 if let 更简洁，用于副作用（如记录日志）
        .inspect_err(|e| {
            tracing::error!(
                plugin_id = %plugin_id,
                method = %method,
                "调用插件方法失败: {}",
                e
            )
        })
        // 将 anyhow::Error 转为 String（Tauri 命令要求错误类型为 String）
        .map_err(|e| e.to_string())
}

/// 获取插件配置
/// 从 JSON 文件中读取插件的持久化配置
#[tauri::command]
pub async fn get_plugin_config(plugin_id: String) -> Result<Value, String> {
    load_plugin_config(&plugin_id)
        .inspect_err(|e| {
            tracing::error!(plugin_id = %plugin_id, "读取插件配置失败: {}", e)
        })
        .map_err(|e| e.to_string())
}

/// 保存插件配置
/// 将配置序列化为 JSON 并写入文件
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
///
/// 完整的插件安装流程：
/// 1. 从 ZIP 文件加载插件包
/// 2. 验证插件包完整性
/// 3. 解压到插件目录
/// 4. 注册到插件注册表
/// 5. 重新加载插件管理器
///
/// ## Rust 知识点: `?` 操作符
/// 每个 `?` 都在做错误传播。如果 Result 是 Err，立即从当前函数返回。
/// 由于 Tauri 要求错误类型为 String，最后用 `.map_err(|e| format!(...))` 转换。
#[tauri::command]
pub async fn import_plugin_package(
    file_path: String,
    manager: State<'_, PluginManagerState>,
) -> Result<String, String> {
    tracing::info!(file_path = %file_path, "开始导入插件包");

    // 1. 从 ZIP 文件加载插件包
    let pkg = PluginPackage::from_zip(std::path::Path::new(&file_path))
        .inspect_err(|e| {
            tracing::error!(file_path = %file_path, "加载插件包失败: {}", e)
        })
        .map_err(|e| format!("加载插件包失败: {}", e))?;

    // 2. 验证插件包完整性
    pkg.validate()
        .inspect_err(|e| {
            tracing::error!(plugin_id = %pkg.manifest.id, "验证插件包失败: {}", e)
        })
        .map_err(|e| format!("插件包验证失败: {}", e))?;

    // 3. 创建插件目录并解压
    let plugin_dir = crate::paths::plugins_dir()
        .map_err(|e| format!("获取插件目录失败: {}", e))?
        .join(&pkg.manifest.id);

    tracing::info!(plugin_dir = %plugin_dir.display(), "目标插件目录");

    pkg.install(&plugin_dir)
        .map_err(|e| format!("安装插件失败: {}", e))?;

    // 4. 获取动态库路径和资源路径
    let library_path = pkg
        .get_library_path(&plugin_dir)
        .map_err(|e| format!("获取动态库路径失败: {}", e))?;

    let assets_dir = pkg.get_assets_dir(&plugin_dir);

    // 5. 注册到插件注册表（持久化元数据）
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
        installed_at: chrono::Utc::now(), // 记录安装时间
        enabled: true,                     // 默认启用
        assets_path: assets_dir.clone(),
        library_path: library_path.clone(),
    };

    registry.register(installed_plugin)
        .map_err(|e| format!("注册插件失败: {}", e))?;

    // 6. 重新加载插件管理器，使新插件生效
    manager.init().await
        .map_err(|e| format!("重新加载插件管理器失败: {}", e))?;

    tracing::info!(plugin_id = %pkg.manifest.id, "插件导入成功");

    Ok(format!("插件 {} 安装成功", pkg.manifest.name))
}

/// 获取所有可用的插件（已安装 + 可安装）
/// 扫描插件目录下所有包含 manifest.json 的子目录
#[tauri::command]
pub async fn get_available_plugins() -> Result<Vec<PluginManifest>, String> {
    let plugins_dir = crate::paths::plugins_dir()
        .map_err(|e| format!("获取插件目录失败: {}", e))?;

    let mut plugins = Vec::new();

    if plugins_dir.exists() {
        // `fs::read_dir` 返回目录条目迭代器
        let entries = fs::read_dir(&plugins_dir)
            .map_err(|e| format!("读取插件目录失败: {}", e))?;

        for entry in entries {
            // `entry?` 传播读取单个条目的错误
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;

            let path = entry.path();

            // 只处理子目录（插件目录 = 子目录名）
            if path.is_dir() {
                let manifest_path = path.join("manifest.json");
                if manifest_path.exists() {
                    // 读取并解析 manifest.json
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

/// 获取已安装插件列表（从注册表文件中读取）
#[tauri::command]
pub async fn get_installed_plugins_from_registry() -> Result<Vec<InstalledPlugin>, String> {
    let registry = PluginRegistry::new()
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    Ok(registry.get_installed())
}

/// 安装插件（如果插件包已手动解压到插件目录）
/// 直接读取 manifest.json 并注册
#[tauri::command]
pub async fn install_plugin(
    plugin_id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<String, String> {
    tracing::info!(plugin_id = %plugin_id, "开始安装插件");

    let plugin_dir = crate::paths::plugins_dir()
        .map_err(|e| format!("获取插件目录失败: {}", e))?
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

    // 获取当前平台对应的动态库文件名
    let lib_name = manifest
        .get_library_filename()
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

    tracing::info!(plugin_id = %manifest.id, "插件安装成功");

    Ok(format!("插件 {} 安装成功", manifest.name))
}

/// 卸载插件
///
/// 重要：Windows 上必须先卸载 DLL（释放文件锁），然后才能删除文件。
/// 顺序必须是：① 卸载 DLL → ② 删除文件 → ③ 从注册表移除
#[tauri::command]
pub async fn uninstall_plugin(
    plugin_id: String,
    manager: State<'_, PluginManagerState>,
) -> Result<String, String> {
    tracing::info!(plugin_id = %plugin_id, "开始卸载插件");

    // 1. 先从内存中卸载插件，释放 DLL 文件锁
    //    Windows 上，被加载的 DLL 文件无法删除，必须先释放
    manager.unload_plugin(&plugin_id).await
        .map_err(|e| format!("卸载插件失败: {}", e))?;

    let plugins_base_dir = crate::paths::plugins_dir()
        .map_err(|e| format!("获取插件目录失败: {}", e))?;

    // 2. 删除插件目录（DLL 已释放，可以正常删除）
    let plugin_dir = plugins_base_dir.join(&plugin_id);

    let mut deleted_dir = false;
    if plugin_dir.exists() {
        // 带重试的删除：Windows 上 DLL 释放可能有短暂延迟
        let delete_result = remove_dir_with_retry(&plugin_dir, 3);
        if let Err(e) = delete_result {
            return Err(format!("删除插件目录失败: {}", e));
        }
        deleted_dir = true;
        tracing::info!("删除插件目录: {:?}", plugin_dir);
    } else {
        // 如果标准路径不存在，扫描所有子目录查找匹配的 manifest.json
        // 这是为了兼容不同的目录命名方式
        if plugins_base_dir.exists() {
            let entries = fs::read_dir(&plugins_base_dir)
                .map_err(|e| format!("读取插件目录失败: {}", e))?;

            for entry in entries {
                let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
                let path = entry.path();

                if path.is_dir() {
                    let manifest_path = path.join("manifest.json");
                    if manifest_path.exists() {
                        if let Ok(content) = fs::read_to_string(&manifest_path) {
                            if let Ok(manifest) =
                                serde_json::from_str::<serde_json::Value>(&content)
                            {
                                // 检查 manifest 中的 id 是否匹配目标插件
                                if manifest
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .map(|id| id == plugin_id)
                                    .unwrap_or(false)
                                {
                                    let delete_result = remove_dir_with_retry(&path, 3);
                                    if let Err(e) = delete_result {
                                        return Err(format!("删除插件目录失败: {}", e));
                                    }
                                    deleted_dir = true;
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

    // 3. 从注册表移除（持久化的元数据）
    let mut registry = PluginRegistry::new()
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    registry.unregister(&plugin_id)
        .map_err(|e| format!("从注册表移除插件失败: {}", e))?;

    tracing::info!(plugin_id = %plugin_id, "插件卸载成功");

    Ok(format!("插件 {} 卸载成功", plugin_id))
}

/// 带重试的目录删除
///
/// ## 为什么需要重试？
/// Windows 上，即使调用了 `FreeLibrary` 释放 DLL，操作系统也可能有短暂的文件锁残留。
/// 重试机制使用递增延迟（200ms × 尝试次数），给操作系统时间完成清理。
///
/// ## Rust 知识点: 循环与错误处理
/// `for attempt in 1..=max_retries` — `1..=3` 表示包含 3 的范围（1, 2, 3）。
/// `match` 用于对 Result 进行模式匹配。
fn remove_dir_with_retry(path: &std::path::Path, max_retries: u32) -> std::io::Result<()> {
    let mut last_err = fs::remove_dir_all(path);
    for attempt in 1..=max_retries {
        match &last_err {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    attempt,
                    "删除目录失败，重试中...: {}",
                    e
                );
            }
        }
        // 递增延迟：第1次 200ms，第2次 400ms，第3次 600ms
        std::thread::sleep(std::time::Duration::from_millis(200 * attempt as u64));
        last_err = fs::remove_dir_all(path);
    }
    last_err
}

/// 读取插件的前端资源文件内容
/// 前端需要动态加载插件的 HTML/JS/CSS
#[tauri::command]
pub async fn read_plugin_asset(plugin_id: String, asset_path: String) -> Result<String, String> {
    let registry = PluginRegistry::new()
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    let plugin = registry
        .get(&plugin_id)
        .ok_or_else(|| format!("插件未安装: {}", plugin_id))?;

    // 构建完整路径：插件资源目录 + 相对路径
    let full_path = plugin.assets_path.join(&asset_path);

    // 读取文件内容（以 UTF-8 字符串形式返回）
    let content = std::fs::read_to_string(&full_path)
        .map_err(|e| format!("读取资源文件失败: {}", e))?;

    Ok(content)
}

/// 打开外部 URL（在系统默认浏览器中）
/// 使用 `opener` crate 实现跨平台
#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    opener::open(&url)
        .map_err(|e| format!("打开链接失败: {}", e))
}

/// 写入文本文件到指定路径
///
/// ## Rust 知识点: 方法链
/// `.inspect_err(...)` 和 `.map_err(...)` 可以链式调用，
/// 分别用于"观察错误"和"转换错误"。
#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    tracing::info!(path = %path, size = content.len(), "写入文件");
    fs::write(&path, &content)
        .map_err(|e| format!("写入文件失败: {}", e))
}

/// 打开文件夹选择对话框
/// 使用 Tauri 的 dialog 插件
#[tauri::command]
pub async fn open_folder_dialog(
    title: Option<String>,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    // `use` 可以在函数内部导入 trait（用于调用 trait 方法）
    use tauri_plugin_dialog::DialogExt;

    let mut builder = app.dialog().file();

    if let Some(title) = title {
        builder = builder.set_title(title);
    }

    // `blocking_pick_folder` 是同步阻塞调用，Tauri 会在后台线程执行
    let folder_path = builder.blocking_pick_folder();

    Ok(folder_path.map(|p| p.to_string()))
}

/// 打开文件选择对话框
#[tauri::command]
pub async fn open_file_dialog(
    title: Option<String>,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let mut builder = app.dialog().file();

    if let Some(title) = title {
        builder = builder.set_title(title);
    }

    let file_path = builder.blocking_pick_file();

    Ok(file_path.map(|p| p.to_string()))
}

// ── 日志查询 ──

/// 日志查询参数
/// `#[derive(Deserialize)]` 使 Tauri 能自动从前端传来的 JSON 中解析这些字段
/// 所有字段都是 `Option`，表示可选
#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub level: Option<String>,  // 按日志级别过滤 (INFO, WARN, ERROR...)
    pub plugin: Option<String>, // 按插件名过滤
    pub since: Option<String>,  // 按时间过滤 (RFC 3339 格式)
}

/// 获取日志
///
/// ## 实现要点
/// - `LOG_RING` 是一个全局的环形缓冲区（VecDeque），最多 1000 条
/// - 使用迭代器的 `.rev().filter().take()` 链，高效且惰性
/// - 限制最多返回 100 条（DEFAULT_LIMIT）
///
/// ## Rust 知识点: 迭代器组合子
/// `.rev()` — 从后往前遍历（最新的日志在前面）
/// `.filter()` — 按条件筛选
/// `.take(n)` — 只取前 n 个
/// `.cloned()` — 克隆每个元素（从 &LogEntry 转为 LogEntry）
/// `.collect()` — 收集到 Vec 中
///
/// 这些方法都是"零成本抽象"——编译后与手写循环性能相同。
#[tauri::command]
pub fn get_logs(query: Option<LogQuery>) -> Result<Vec<LogEntry>, String> {
    const DEFAULT_LIMIT: usize = 100;

    // `Mutex::lock()` 获取互斥锁
    // 如果锁被其他线程持有（panic 导致中毒），返回 Err
    let ring = LOG_RING.lock().map_err(|e| format!("Lock error: {}", e))?;

    let entries: Vec<LogEntry> = ring
        .iter()        // 从头到尾迭代（最旧的在前）
        .rev()         // 反转：最新的在前
        .filter(|e| match &query {
            Some(q) => {
                // 按日志级别过滤
                if let Some(ref lvl) = q.level {
                    if e.level != *lvl {
                        return false;
                    }
                }
                // 按插件名过滤（target 中包含插件名）
                if let Some(ref plugin) = q.plugin {
                    if !e.target.to_lowercase().contains(&plugin.to_lowercase()) {
                        return false;
                    }
                }
                // 按时间过滤（只返回 since 之后的日志）
                if let Some(ref since_str) = q.since {
                    if let Ok(since_dt) = chrono::DateTime::parse_from_rfc3339(since_str) {
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&e.timestamp) {
                            if dt <= since_dt {
                                return false;
                            }
                        }
                    }
                }
                true // 通过所有过滤条件
            }
            None => true, // 没有查询条件，全部通过
        })
        .take(DEFAULT_LIMIT) // 限制返回数量
        .cloned()             // 从引用克隆出独立的值
        .collect();           // 收集到 Vec 中

    Ok(entries)
}

/// 清空日志缓冲区
#[tauri::command]
pub fn clear_logs() -> Result<(), String> {
    let mut ring = LOG_RING.lock().map_err(|e| format!("Lock error: {}", e))?;
    ring.clear();
    Ok(())
}

/// 设置窗口主题 (light / dark)
#[tauri::command]
pub async fn set_window_theme(
    theme: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let tauri_theme = match theme.as_str() {
        "dark" => Some(tauri::Theme::Dark),
        "light" => Some(tauri::Theme::Light),
        _ => None,
    };
    if let Some(window) = app.get_webview_window("main") {
        window.set_theme(tauri_theme).map_err(|e| e.to_string())?;
    }
    Ok(())
}
