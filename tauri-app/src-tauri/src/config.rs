use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub theme: String,
    pub window_state: WindowState,
    pub settings: GeneralSettings,
}

/// 窗口状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 800,
            x: 100,
            y: 100,
            is_maximized: false,
        }
    }
}

/// 通用设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub auto_start: bool,
    pub minimize_to_tray: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            auto_start: false,
            minimize_to_tray: true,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "light".to_string(),
            window_state: WindowState::default(),
            settings: GeneralSettings::default(),
        }
    }
}

/// 获取配置目录
pub fn get_config_dir() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "zjhy", "WorkTools")
        .ok_or_else(|| anyhow::anyhow!("无法找到配置目录"))?;
    Ok(proj_dirs.config_dir().join("worktools"))
}

/// 获取历史记录目录
pub fn get_history_dir() -> Result<PathBuf> {
    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;
    Ok(user_dirs.home_dir().join(".worktools/history"))
}

/// 加载应用配置
pub fn load_app_config() -> Result<AppConfig> {
    let config_dir = get_config_dir()?;
    let config_path = config_dir.join("config/settings.json");

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .context("读取配置文件失败")?;
        let config: AppConfig = serde_json::from_str(&content)
            .context("解析配置文件失败")?;
        Ok(config)
    } else {
        Ok(AppConfig::default())
    }
}

/// 保存应用配置
pub fn save_app_config(config: &AppConfig) -> Result<()> {
    let config_dir = get_config_dir()?;
    std::fs::create_dir_all(&config_dir)
        .context("创建配置目录失败")?;

    let config_path = config_dir.join("config/settings.json");
    let content = serde_json::to_string_pretty(config)
        .context("序列化配置失败")?;

    std::fs::write(&config_path, content)
        .context("写入配置文件失败")?;

    Ok(())
}

/// 加载插件配置
pub fn load_plugin_config(plugin_id: &str) -> Result<serde_json::Value> {
    let history_dir = get_history_dir()?;
    let config_path = history_dir.join(format!("plugins/{}.json", plugin_id));

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .context("读取插件配置失败")?;
        let config: serde_json::Value = serde_json::from_str(&content)
            .context("解析插件配置失败")?;
        Ok(config)
    } else {
        Ok(serde_json::json!({}))
    }
}

/// 保存插件配置
pub fn save_plugin_config(plugin_id: &str, config: &serde_json::Value) -> Result<()> {
    let history_dir = get_history_dir()?;
    let plugins_dir = history_dir.join("plugins");
    std::fs::create_dir_all(&plugins_dir)
        .context("创建插件配置目录失败")?;

    let config_path = plugins_dir.join(format!("{}.json", plugin_id));
    let content = serde_json::to_string_pretty(config)
        .context("序列化插件配置失败")?;

    std::fs::write(&config_path, content)
        .context("写入插件配置失败")?;

    Ok(())
}
