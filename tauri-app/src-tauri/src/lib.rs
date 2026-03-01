pub mod plugin_manager;
mod config;
mod commands;
mod crypto;

use anyhow::Result;
use plugin_manager::PluginManager;
use crypto::PasswordEncryptor;
use std::sync::Arc;
use tauri::Manager;

/// 初始化日志系统
fn init_logging() -> Result<()> {
    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;
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
    let plugin_manager = Arc::new(
        PluginManager::new().expect("无法创建插件管理器")
    );

    // 尝试加载已保存的加密配置
    let crypto_config = if let Ok(config) = config::load_plugin_config("password-manager") {
        let master_password = config.get("master_password")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let salt = config.get("salt")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        crypto::CryptoConfig {
            master_password,
            salt,
        }
    } else {
        crypto::CryptoConfig::default()
    };

    // 创建密码加密器
    let password_encryptor = Arc::new(std::sync::Mutex::new(
        PasswordEncryptor::new(crypto_config)
    ));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
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
            // 设置密码加密器状态
            app.manage(password_encryptor);

            println!("Work Tools 应用启动成功");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_available_plugins,
            commands::get_installed_plugins,
            commands::install_plugin,
            commands::uninstall_plugin,
            commands::get_plugin_config,
            commands::set_plugin_config,
            commands::get_app_config,
            commands::set_app_config,
            commands::get_password_entries,
            commands::save_password_entry,
            commands::delete_password_entry,
            commands::clear_all_password_entries,
            commands::get_auth_entries,
            commands::save_auth_entry,
            commands::delete_auth_entry,
            commands::generate_totp,
            commands::generate_secret,
            commands::init_or_verify_master_password,
            commands::has_master_password,
            commands::get_crypto_config,
            commands::load_crypto_config,
            commands::encrypt_password,
            commands::decrypt_password,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
