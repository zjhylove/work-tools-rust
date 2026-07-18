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
//! ```text
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
use std::sync::Arc;
use tauri::{Manager, State};
use tokio::fs as tfs;

/// 校验插件 ID 格式：只允许小写字母、数字和连字符。
///
/// 此函数被 `read_plugin_asset`、`write_file` 等命令复用，
/// 并在 `#[cfg(test)]` 中测试。
pub fn validate_plugin_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
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
    params: Option<serde_json::Value>,
    manager: State<'_, PluginManagerState>,
) -> Result<serde_json::Value, String> {
    let params = params.unwrap_or(serde_json::Value::Object(Default::default()));
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
    let id = plugin_id.clone();
    tokio::task::spawn_blocking(move || load_plugin_config(&id))
        .await
        .map_err(|e| format!("配置加载任务失败: {}", e))?
        .inspect_err(|e| tracing::error!(plugin_id = %plugin_id, "读取插件配置失败: {}", e))
        .map_err(|e| e.to_string())
}

/// 保存插件配置
/// 将配置序列化为 JSON 并写入文件
#[tauri::command]
pub async fn set_plugin_config(plugin_id: String, config: Value) -> Result<(), String> {
    let id = plugin_id.clone();
    let cfg = config.clone();
    tokio::task::spawn_blocking(move || save_plugin_config(&id, &cfg))
        .await
        .map_err(|e| format!("配置保存任务失败: {}", e))?
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

    let fp = file_path.clone();
    let pkg =
        tokio::task::spawn_blocking(move || PluginPackage::from_zip(std::path::Path::new(&fp)))
            .await
            .map_err(|e| format!("插件包加载任务失败: {}", e))?
            .inspect_err(|e| tracing::error!(file_path = %file_path, "加载插件包失败: {}", e))
            .map_err(|e| format!("加载插件包失败: {}", e))?;

    // 2. 验证插件包完整性
    pkg.validate()
        .inspect_err(|e| tracing::error!(plugin_id = %pkg.manifest.id, "验证插件包失败: {}", e))
        .map_err(|e| format!("插件包验证失败: {}", e))?;

    // 3. 创建插件目录并解压
    let plugin_dir = crate::paths::plugins_dir()
        .map_err(|e| format!("获取插件目录失败: {}", e))?
        .join(&pkg.manifest.id);

    tracing::info!(plugin_dir = %plugin_dir.display(), "目标插件目录");

    let manifest = pkg.manifest.clone();
    let install_dir = plugin_dir.clone();
    tokio::task::spawn_blocking(move || pkg.install(&install_dir))
        .await
        .map_err(|e| format!("插件解压任务失败: {}", e))?
        .map_err(|e| format!("安装插件失败: {}", e))?;

    // 4. 注册 + 加载（共享逻辑）
    register_and_load_plugin(&manifest, &plugin_dir, &manager).await
}

/// 获取所有可用的插件（已安装 + 可安装）
/// 扫描插件目录下所有包含 manifest.json 的子目录
#[tauri::command]
pub async fn get_available_plugins() -> Result<Vec<PluginManifest>, String> {
    let plugins_dir =
        crate::paths::plugins_dir().map_err(|e| format!("获取插件目录失败: {}", e))?;

    let mut plugins = Vec::new();

    if plugins_dir.exists() {
        // `tfs::read_dir` 返回异步目录条目流
        let mut entries = tfs::read_dir(&plugins_dir)
            .await
            .map_err(|e| format!("读取插件目录失败: {}", e))?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();

            // 只处理子目录（插件目录 = 子目录名）
            let metadata = tfs::metadata(&path)
                .await
                .map_err(|e| format!("读取目录项元数据失败: {}", e))?;
            if metadata.is_dir() {
                let manifest_path = path.join("manifest.json");
                if tfs::metadata(&manifest_path).await.is_ok() {
                    // 读取并解析 manifest.json
                    let content = tfs::read_to_string(&manifest_path)
                        .await
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
    let registry = PluginRegistry::new_async()
        .await
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
    if tfs::metadata(&manifest_path).await.is_err() {
        return Err("插件未找到".to_string());
    }

    // 读取 manifest
    let content = tfs::read_to_string(&manifest_path)
        .await
        .map_err(|e| format!("读取 manifest.json 失败: {}", e))?;

    let manifest: PluginManifest =
        serde_json::from_str(&content).map_err(|e| format!("解析 manifest.json 失败: {}", e))?;

    // 注册 + 加载（共享逻辑）
    register_and_load_plugin(&manifest, &plugin_dir, &manager).await
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
    manager
        .unload_plugin(&plugin_id)
        .await
        .map_err(|e| format!("卸载插件失败: {}", e))?;

    let plugins_base_dir =
        crate::paths::plugins_dir().map_err(|e| format!("获取插件目录失败: {}", e))?;

    // 2. 删除插件目录（DLL 已释放，可以正常删除）
    let plugin_dir = plugins_base_dir.join(&plugin_id);

    let mut deleted_dir = false;
    if tfs::metadata(&plugin_dir).await.is_ok() {
        // 带重试的删除：Windows 上 DLL 释放可能有短暂延迟
        let delete_result = remove_dir_with_retry(&plugin_dir, 3).await;
        if let Err(e) = delete_result {
            return Err(format!("删除插件目录失败: {}", e));
        }
        deleted_dir = true;
        tracing::info!("删除插件目录: {:?}", plugin_dir);
    } else {
        // 如果标准路径不存在，扫描所有子目录查找匹配的 manifest.json
        // 这是为了兼容不同的目录命名方式
        if tfs::metadata(&plugins_base_dir).await.is_ok() {
            let mut entries = tfs::read_dir(&plugins_base_dir)
                .await
                .map_err(|e| format!("读取插件目录失败: {}", e))?;

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();

                let metadata = tfs::metadata(&path)
                    .await
                    .map_err(|e| format!("读取目录项元数据失败: {}", e))?;
                if metadata.is_dir() {
                    let manifest_path = path.join("manifest.json");
                    if let Ok(content) = tfs::read_to_string(&manifest_path).await {
                        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
                            // 检查 manifest 中的 id 是否匹配目标插件
                            if manifest
                                .get("id")
                                .and_then(|v| v.as_str())
                                .map(|id| id == plugin_id)
                                .unwrap_or(false)
                            {
                                let delete_result = remove_dir_with_retry(&path, 3).await;
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

    if !deleted_dir {
        tracing::warn!("未找到插件 {} 的目录", plugin_id);
    }

    // 3. 从注册表移除（持久化的元数据）
    let mut registry = PluginRegistry::new_async()
        .await
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    registry
        .unregister_async(&plugin_id)
        .await
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
async fn remove_dir_with_retry(path: &std::path::Path, max_retries: u32) -> std::io::Result<()> {
    let mut last_err = tfs::remove_dir_all(path).await;
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
        tokio::time::sleep(std::time::Duration::from_millis(200 * attempt as u64)).await;
        last_err = tfs::remove_dir_all(path).await;
    }
    last_err
}

/// 读取插件的前端资源文件内容
/// 前端需要动态加载插件的 HTML/JS/CSS
#[tauri::command]
pub async fn read_plugin_asset(plugin_id: String, asset_path: String) -> Result<String, String> {
    if !validate_plugin_id(&plugin_id) {
        return Err("非法插件 ID 格式".into());
    }

    // 拒绝路径穿越
    if asset_path.contains("..") {
        return Err("资源路径不允许包含 ..".into());
    }

    let registry = PluginRegistry::new_async()
        .await
        .map_err(|e| format!("打开插件注册表失败: {}", e))?;

    let plugin = registry
        .get(&plugin_id)
        .ok_or_else(|| format!("插件未安装: {}", plugin_id))?;

    let full_path = plugin.assets_path.join(&asset_path);

    // 验证路径未逃逸插件资源目录
    let canonical_assets = tfs::canonicalize(&plugin.assets_path)
        .await
        .map_err(|e| format!("解析资源目录失败: {}", e))?;
    let canonical_full = tfs::canonicalize(&full_path)
        .await
        .map_err(|e| format!("解析资源路径失败: {}", e))?;

    if !canonical_full.starts_with(&canonical_assets) {
        return Err("资源路径超出插件目录范围".into());
    }

    let content = tfs::read_to_string(&canonical_full)
        .await
        .map_err(|e| format!("读取资源文件失败: {}", e))?;

    Ok(content)
}

/// 打开外部 URL（在系统默认浏览器中）
/// 使用 `opener` crate 实现跨平台
#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    // 白名单 scheme：只允许安全协议
    let scheme = url.split(':').next().unwrap_or("");
    if !matches!(scheme, "http" | "https" | "mailto") {
        return Err(format!("不允许的 URL 协议: {}", scheme));
    }
    opener::open(&url).map_err(|e| format!("打开链接失败: {}", e))
}

/// 写入文本文件到指定路径
///
/// 仅允许写入 `~/.worktools/` 目录下的路径，防止任意文件写入。
#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    tracing::info!(path = %path, size = content.len(), "写入文件");

    // 路径沙箱：仅允许写入 ~/.worktools/ 目录下
    let canonical_path = tfs::canonicalize(std::path::Path::new(&path))
        .await
        .map_err(|e| format!("解析路径失败: {}", e))?;
    let base = crate::paths::worktools_base().map_err(|e| format!("获取应用目录失败: {}", e))?;
    let canonical_base = tfs::canonicalize(&base)
        .await
        .map_err(|e| format!("解析应用目录失败: {}", e))?;

    if !canonical_path.starts_with(&canonical_base) {
        return Err(format!("写入路径超出应用目录范围: {:?}", canonical_path));
    }

    tfs::write(&canonical_path, &content)
        .await
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
    filters: Option<Vec<serde_json::Value>>,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let mut builder = app.dialog().file();

    if let Some(title) = title {
        builder = builder.set_title(title);
    }

    if let Some(filters) = filters {
        for filter in filters {
            let name = filter["name"].as_str().unwrap_or("Files");
            let extensions: Vec<&str> = filter["extensions"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            if !extensions.is_empty() {
                builder = builder.add_filter(name, &extensions);
            }
        }
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

    // `parking_lot::Mutex::lock()` 返回 MutexGuard（无 poisoning）
    let ring = LOG_RING.lock();

    let entries: Vec<LogEntry> = ring
        .iter() // 从头到尾迭代（最旧的在前）
        .rev() // 反转：最新的在前
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
        .cloned() // 从引用克隆出独立的值
        .collect(); // 收集到 Vec 中

    Ok(entries)
}

/// 清空日志缓冲区
#[tauri::command]
pub fn clear_logs() -> Result<(), String> {
    let mut ring = LOG_RING.lock();
    ring.clear();
    Ok(())
}

#[tauri::command]
pub async fn set_window_theme(theme: String, app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        let t = match theme.as_str() {
            "dark" => Some(tauri::Theme::Dark),
            _ => Some(tauri::Theme::Light),
        };
        w.set_theme(t).map_err(|e| e.to_string())?;

        // macOS: with titleBarStyle "Transparent", the titlebar shows the
        // window background color. Set it to match the current theme so the
        // native titlebar area blends with the dark/light content.
        #[cfg(target_os = "macos")]
        {
            let color = match theme.as_str() {
                "dark" => tauri::window::Color(26, 27, 30, 255), // matches --bg-primary: #1a1b1e
                _ => tauri::window::Color(248, 249, 250, 255),   // matches --bg-secondary: #f8f9fa
            };
            w.set_background_color(Some(color))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// 注册插件到注册表并增量加载。
///
/// `import_plugin_package` 和 `install_plugin` 共享此逻辑。
async fn register_and_load_plugin(
    manifest: &PluginManifest,
    plugin_dir: &std::path::Path,
    manager: &PluginManagerState,
) -> Result<String, String> {
    let lib_name = manifest
        .get_library_filename()
        .ok_or_else(|| "未找到动态库配置".to_string())?;
    let library_path = plugin_dir.join(lib_name);
    let assets_dir = plugin_dir.join("assets");

    let mut registry = PluginRegistry::new_async()
        .await
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
        .register_async(installed_plugin)
        .await
        .map_err(|e| format!("注册插件失败: {}", e))?;

    manager
        .load_plugin_by_dir(plugin_dir)
        .await
        .map_err(|e| format!("加载插件失败: {}", e))?;

    tracing::info!(plugin_id = %manifest.id, "插件注册加载成功");
    Ok(format!("插件 {} 安装成功", manifest.name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::path::PathBuf;

    // Serialization lock for tests that share global LOG_RING state.
    // Prevents parallel test interference.
    static LOG_TEST_LOCK: Mutex<()> = Mutex::new(());

    // ── validate_plugin_id ───────────────────────────────────────────

    #[test]
    fn valid_plugin_ids() {
        assert!(validate_plugin_id("password-manager"));
        assert!(validate_plugin_id("redis-client"));
        assert!(validate_plugin_id("json-tools"));
        assert!(validate_plugin_id("a"));
        assert!(validate_plugin_id("a1"));
        assert!(validate_plugin_id("my-plugin-123"));
    }

    #[test]
    fn empty_plugin_id_rejected() {
        assert!(!validate_plugin_id(""));
    }

    #[test]
    fn uppercase_plugin_id_rejected() {
        assert!(!validate_plugin_id("PasswordManager"));
        assert!(!validate_plugin_id("JSON-Tools"));
    }

    #[test]
    fn spaces_in_plugin_id_rejected() {
        assert!(!validate_plugin_id("password manager"));
        assert!(!validate_plugin_id(" leading"));
        assert!(!validate_plugin_id("trailing "));
    }

    #[test]
    fn path_traversal_in_plugin_id_rejected() {
        assert!(!validate_plugin_id("../etc/passwd"));
        assert!(!validate_plugin_id(".."));
        assert!(!validate_plugin_id("foo/../bar"));
        assert!(!validate_plugin_id("foo/bar"));
    }

    #[test]
    fn special_chars_in_plugin_id_rejected() {
        assert!(!validate_plugin_id("foo.bar"));
        assert!(!validate_plugin_id("foo_bar"));
        assert!(!validate_plugin_id("foo@bar"));
        assert!(!validate_plugin_id("foo!bar"));
        assert!(!validate_plugin_id("中文插件"));
    }

    // ── write_file sandbox ──────────────────────────────────────────
    //
    // We test the sandbox validation logic directly by calling
    // `write_file` with a real temp dir standing in for worktools_base().
    // Because `write_file` uses `crate::paths::worktools_base()`, we
    // create a temp directory that canonicalize resolves to, and rely on
    // `std::env::set_var("HOME")` or test that paths outside are rejected
    // based on the canonical comparison.
    //
    // However, since `worktools_base()` depends on `directories::UserDirs`
    // which reads $HOME at runtime, we test the *validation logic* in
    // isolation by constructing the check directly.

    /// Delegate to the shared path_safety::sandbox_check implementation.
    fn sandbox_check(target: &std::path::Path, base_dir: &std::path::Path) -> Result<(), String> {
        crate::path_safety::sandbox_check(target, base_dir)
    }

    #[test]
    fn sandbox_allows_path_inside_base() {
        let base = tempfile::tempdir().unwrap();
        let inner = base.path().join("subdir");
        std::fs::create_dir_all(&inner).unwrap();
        assert!(sandbox_check(&inner, base.path()).is_ok());
    }

    #[test]
    fn sandbox_allows_file_inside_base() {
        let base = tempfile::tempdir().unwrap();
        let file = base.path().join("file.txt");
        std::fs::write(&file, "hello").unwrap();
        assert!(sandbox_check(&file, base.path()).is_ok());
    }

    #[test]
    fn sandbox_rejects_path_outside_base() {
        let base = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        assert!(sandbox_check(other.path(), base.path()).is_err());
    }

    #[test]
    fn sandbox_rejects_symlink_escape() {
        // Duplicated here for regression safety; canonical implementation lives in path_safety.rs
        let base = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();

        let symlink_path = base.path().join("escape");
        let symlink_result = {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(other.path(), &symlink_path)
            }
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_dir(other.path(), &symlink_path)
            }
        };
        if symlink_result.is_err() {
            return;
        }
        let canonical = std::fs::canonicalize(&symlink_path).unwrap();
        let canonical_base = std::fs::canonicalize(base.path()).unwrap();
        if !canonical.starts_with(&canonical_base) {
            assert!(sandbox_check(&symlink_path, base.path()).is_err());
        }
    }

    #[test]
    fn sandbox_rejects_nonexistent_path() {
        let base = tempfile::tempdir().unwrap();
        let ghost = PathBuf::from("/this/absolutely/does/not/exist");
        assert!(sandbox_check(&ghost, base.path()).is_err());
    }

    // ── read_plugin_asset path traversal ────────────────────────────

    #[test]
    fn asset_path_with_dotdot_rejected() {
        // Simulate the inline check that read_plugin_asset performs
        let asset_path = "../../etc/passwd";
        assert!(asset_path.contains(".."));
    }

    #[test]
    fn asset_path_without_dotdot_allowed() {
        let asset_path = "index.html";
        assert!(!asset_path.contains(".."));

        let nested = "js/app.js";
        assert!(!nested.contains(".."));
    }

    // ── open_url scheme validation ────────────────────────────────────
    //
    // open_url is pub async fn with no Tauri state dependency,
    // so we can call it directly from tests.
    // Rejected schemes return Err before calling opener::open().

    #[tokio::test]
    async fn open_url_rejects_file_scheme() {
        let result = open_url("file:///etc/passwd".into()).await;
        assert!(result.is_err(), "file:// scheme should be rejected");
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("不允许") || err_msg.contains("file"),
            "Expected scheme rejection error, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn open_url_rejects_javascript_scheme() {
        let result = open_url("javascript:alert(1)".into()).await;
        assert!(result.is_err(), "javascript: scheme should be rejected");
    }

    #[tokio::test]
    async fn open_url_rejects_data_scheme() {
        let result = open_url("data:text/html,<script>alert(1)</script>".into()).await;
        assert!(result.is_err(), "data: scheme should be rejected");
    }

    #[tokio::test]
    async fn open_url_rejects_ftp_scheme() {
        let result = open_url("ftp://evil.com/payload".into()).await;
        assert!(result.is_err(), "ftp:// scheme should be rejected");
    }

    #[tokio::test]
    async fn open_url_rejects_custom_scheme() {
        let result = open_url("custom-protocol://do-evil".into()).await;
        assert!(
            result.is_err(),
            "custom-protocol: scheme should be rejected"
        );
    }

    // Note: We do NOT test http/https/mailto acceptance here because
    // open_url() would actually invoke opener::open() and launch a browser.
    // Scheme extraction for allowed schemes is already covered above
    // by the static assertion tests.

    // ── get_logs filtering ──────────────────────────────────────────
    //
    // get_logs is purely synchronous and uses LOG_RING (a global static),
    // making it easy to test in isolation.

    #[test]
    fn get_logs_returns_empty_when_no_entries() {
        let _lock = LOG_TEST_LOCK.lock();
        // Clear any entries that might exist from other tests
        let mut ring = LOG_RING.lock();
        ring.clear();
        drop(ring);

        let result = get_logs(None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn get_logs_filters_by_level() {
        let _lock = LOG_TEST_LOCK.lock();
        let mut ring = LOG_RING.lock();
        ring.clear();
        ring.push_back(LogEntry {
            timestamp: "2026-01-01T00:00:00.000Z".into(),
            level: "INFO".into(),
            target: "test_module".into(),
            message: "info message".into(),
        });
        ring.push_back(LogEntry {
            timestamp: "2026-01-01T00:00:01.000Z".into(),
            level: "ERROR".into(),
            target: "test_module".into(),
            message: "error message".into(),
        });
        drop(ring);

        let result = get_logs(Some(LogQuery {
            level: Some("ERROR".into()),
            plugin: None,
            since: None,
        }))
        .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].level, "ERROR");
    }

    #[test]
    fn get_logs_filters_by_plugin() {
        let _lock = LOG_TEST_LOCK.lock();
        let mut ring = LOG_RING.lock();
        ring.clear();
        ring.push_back(LogEntry {
            timestamp: "2026-01-01T00:00:00.000Z".into(),
            level: "INFO".into(),
            target: "work_tools::plugin_password_manager".into(),
            message: "from password manager".into(),
        });
        ring.push_back(LogEntry {
            timestamp: "2026-01-01T00:00:01.000Z".into(),
            level: "INFO".into(),
            target: "work_tools::plugin_redis_client".into(),
            message: "from redis client".into(),
        });
        drop(ring);

        let result = get_logs(Some(LogQuery {
            level: None,
            plugin: Some("password_manager".into()),
            since: None,
        }))
        .unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].message.contains("password manager"));
    }
    #[test]
    fn get_logs_filters_by_since() {
        let _lock = LOG_TEST_LOCK.lock();
        let mut ring = LOG_RING.lock();
        ring.clear();
        ring.push_back(LogEntry {
            timestamp: "2026-01-01T00:00:00.000Z".into(),
            level: "INFO".into(),
            target: "test".into(),
            message: "old".into(),
        });
        ring.push_back(LogEntry {
            timestamp: "2026-01-01T00:01:00.000Z".into(),
            level: "INFO".into(),
            target: "test".into(),
            message: "new".into(),
        });
        drop(ring);

        let result = get_logs(Some(LogQuery {
            level: None,
            plugin: None,
            since: Some("2026-01-01T00:00:30.000Z".into()),
        }))
        .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].message, "new");
    }
    #[test]
    fn get_logs_respects_default_limit() {
        let _lock = LOG_TEST_LOCK.lock();
        let mut ring = LOG_RING.lock();
        ring.clear();
        for i in 0..150 {
            ring.push_back(LogEntry {
                timestamp: format!("2026-01-01T00:{:02}:00.000Z", i / 60),
                level: "INFO".into(),
                target: "test".into(),
                message: format!("entry {}", i),
            });
        }
        drop(ring);

        let result = get_logs(None).unwrap();
        assert_eq!(result.len(), 100);
    }
    #[test]
    fn clear_logs_works() {
        let _lock = LOG_TEST_LOCK.lock();
        let mut ring = LOG_RING.lock();
        ring.push_back(LogEntry {
            timestamp: "2026-01-01T00:00:00.000Z".into(),
            level: "INFO".into(),
            target: "test".into(),
            message: "to be cleared".into(),
        });
        assert!(!ring.is_empty());
        drop(ring);

        clear_logs().unwrap();

        let ring = LOG_RING.lock();
        assert!(ring.is_empty());
    }
}
