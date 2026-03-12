use anyhow::{Context, Result};
use std::path::PathBuf;

/// 获取历史记录目录
pub fn get_history_dir() -> Result<PathBuf> {
    let user_dirs =
        directories::UserDirs::new().ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;
    Ok(user_dirs.home_dir().join(".worktools/history"))
}

/// 加载插件配置
pub fn load_plugin_config(plugin_id: &str) -> Result<serde_json::Value> {
    let history_dir = get_history_dir()?;
    let config_path = history_dir.join(format!("plugins/{}.json", plugin_id));

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).context("读取插件配置失败")?;
        let config: serde_json::Value =
            serde_json::from_str(&content).context("解析插件配置失败")?;
        Ok(config)
    } else {
        Ok(serde_json::json!({}))
    }
}

/// 保存插件配置
pub fn save_plugin_config(plugin_id: &str, config: &serde_json::Value) -> Result<()> {
    let history_dir = get_history_dir()?;
    let plugins_dir = history_dir.join("plugins");
    std::fs::create_dir_all(&plugins_dir).context("创建插件配置目录失败")?;

    let config_path = plugins_dir.join(format!("{}.json", plugin_id));
    let content = serde_json::to_string_pretty(config).context("序列化插件配置失败")?;

    std::fs::write(&config_path, content).context("写入插件配置失败")?;

    Ok(())
}
