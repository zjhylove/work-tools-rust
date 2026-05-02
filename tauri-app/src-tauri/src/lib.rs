//! # Tauri 应用主库
//!
//! 这是整个应用的核心，负责：
//! 1. 初始化日志系统
//! 2. 创建插件管理器
//! 3. 配置 Tauri 应用（命令注册、插件加载、系统托盘）
//!
//! ## Rust 知识点: 模块系统
//! `mod xxx;` 声明一个模块。Rust 会在以下位置查找模块代码：
//! 1. `xxx.rs` 文件
//! 2. `xxx/mod.rs` 文件
//!
//! `pub mod` 使模块对外部可见，`mod` 则是私有的。
//!
//! ## Tauri 架构
//! Tauri 是一个用 Rust 后端 + Web 前端构建桌面应用的框架。
//! 前端通过 `#[tauri::command]` 标注的函数与后端通信。
//! 这些命令通过 IPC（进程间通信）自动暴露给前端 JavaScript。

mod commands;
mod config;
mod logger;
mod paths;
pub mod plugin_manager; // pub: 可能被外部 crate 引用
mod plugin_package;
mod plugin_registry;
mod tray;

use anyhow::Result;
use plugin_manager::PluginManager;
use std::sync::Arc;
use tauri::{Emitter, Manager};

/// 初始化日志系统
/// 将初始化逻辑单独封装，便于错误处理
fn init_logging() -> Result<()> {
    logger::init_logging()
}

/// 应用入口函数
///
/// `#[cfg_attr(mobile, tauri::mobile_entry_point)]`
/// - 在移动端编译时，生成移动端所需的入口点
/// - 在桌面端，这个属性不起作用
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志 — 尽早调用，确保后续代码都能输出日志
    if let Err(e) = init_logging() {
        eprintln!("初始化日志失败: {}", e);
        // 注意：这里用 eprintln! 而不是 tracing::error!，
        // 因为日志系统本身可能初始化失败
    }

    // ── 创建插件管理器 ──
    // `Arc::new(...)` 创建原子引用计数指针
    // Arc (Atomic Reference Counting) 允许多个所有者共享同一份数据
    // 在 Tauri 中，多个 handler 可能需要访问同一个 PluginManager
    let plugin_manager = Arc::new(
        PluginManager::new()
            // `inspect_err` 检查错误但不改变它 — 用于记录日志
            .inspect_err(|e| tracing::error!("创建插件管理器失败: {}", e))
            // `expect` 在 Result 为 Err 时 panic，用于不可恢复的错误
            // 如果插件管理器创建失败，应用无法正常工作
            .expect("无法创建插件管理器"),
    );

    // ── 构建 Tauri 应用 ──
    // `tauri::Builder::default()` 使用建造者模式配置应用
    tauri::Builder::default()
        // Tauri 插件：提供 opener、shell、dialog、fs 等系统能力
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let manager = plugin_manager.clone();
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = manager.init().await {
                    tracing::error!("插件管理器初始化失败: {}", e);
                    return;
                }
                // Let the frontend render the skeleton loading state briefly
                // before showing the window — avoids a white flash on startup.
                const SHOW_WINDOW_DELAY_MS: u64 = 300;
                // Must match const in App.tsx: EVENT_PLUGINS_READY
                const EVENT_PLUGINS_READY: &str = "plugins-ready";
                let _ = handle.emit(EVENT_PLUGINS_READY, ());
                tokio::time::sleep(std::time::Duration::from_millis(SHOW_WINDOW_DELAY_MS)).await;
                if let Some(w) = handle.get_webview_window("main") {
                    let _ = w.show();
                }
                tracing::info!("plugins-ready 事件已发射，窗口已显示");
            });

            // `app.manage()` 将数据注入 Tauri 的状态管理系统
            // 后续的命令函数可以通过 `State<Arc<PluginManager>>` 获取
            app.manage(plugin_manager);

            // 初始化系统托盘（失败不影响应用正常启动）
            tray::start_tray(app);

            println!("Work Tools 应用启动成功");
            Ok(())
        })
        // `invoke_handler` 注册所有 Tauri 命令
        // `tauri::generate_handler!` 宏自动生成命令路由表
        .invoke_handler(tauri::generate_handler![
            commands::get_installed_plugins,
            commands::call_plugin_method,
            commands::get_plugin_config,
            commands::set_plugin_config,
            // 插件商店命令
            commands::import_plugin_package,
            commands::get_available_plugins,
            commands::get_installed_plugins_from_registry,
            commands::install_plugin,
            commands::uninstall_plugin,
            commands::read_plugin_asset,
            commands::open_url,
            commands::open_folder_dialog,
            commands::open_file_dialog,
            commands::write_file,
            commands::get_logs,
            commands::clear_logs,
            commands::set_window_theme,
        ])
        // `include!` 在编译时将指定文件的内容内联到此处
        // `concat!` 在编译时拼接字符串
        // `env!("OUT_DIR")` 获取编译输出目录
        .run(include!(concat!(
            env!("OUT_DIR"),
            "/tauri-build-context.rs"
        )))
        .expect("error while running tauri application");
}
