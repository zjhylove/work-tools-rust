mod commands;
mod config;
pub mod plugin_manager;
mod plugin_package;
mod plugin_registry;
mod logger;

use anyhow::Result;
use plugin_manager::PluginManager;
use std::sync::Arc;
use tauri::Manager;

fn init_logging() -> Result<()> {
    logger::init_logging()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    if let Err(e) = init_logging() {
        eprintln!("初始化日志失败: {}", e);
    }

    // 创建插件管理器
    let plugin_manager = Arc::new(
        PluginManager::new()
            .inspect_err(|e| tracing::error!("创建插件管理器失败: {}", e))
            .expect("无法创建插件管理器"),
    );

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
                    tracing::error!("插件管理器初始化失败: {}", e);
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
            commands::open_folder_dialog,
            commands::write_file,
            commands::get_logs,
            commands::clear_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
