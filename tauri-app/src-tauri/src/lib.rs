mod commands;
mod config;
mod crypto;
pub mod plugin_manager;
mod plugin_package;
mod plugin_registry;

use anyhow::Result;
use plugin_manager::PluginManager;
use std::sync::Arc;
use tauri::Manager;

/// 初始化日志系统
fn init_logging() -> Result<()> {
    let user_dirs =
        directories::UserDirs::new().ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;
    let log_dir = user_dirs.home_dir().join(".worktools/logs");

    std::fs::create_dir_all(&log_dir)?;

    // 简化日志:只输出到控制台
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    if let Err(e) = init_logging() {
        eprintln!("初始化日志失败: {}", e);
    }

    // 创建插件管理器
    let plugin_manager = Arc::new(PluginManager::new().expect("无法创建插件管理器"));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // 初始化插件管理器
            let manager = plugin_manager.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = manager.init().await {
                    eprintln!("插件管理器初始化失败: {}", e);
                }
            });

            // 设置插件管理器状态
            app.manage(plugin_manager);

            println!("Work Tools 应用启动成功");
            Ok(())
        })
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
