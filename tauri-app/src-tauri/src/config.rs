//! # 插件配置管理
//!
//! 提供插件配置的加载和保存功能。
//! 每个插件的配置存储为独立的 JSON 文件。
//!
//! ## Rust 知识点
//! - `serde_json::Value`: 通用的 JSON 值，可以表示任意 JSON
//! - `serde_json::json!({})`: 宏，在代码中直接写 JSON 字面量
//! - `Path::join`: 安全的路径拼接，自动处理分隔符

use anyhow::{Context, Result};
use std::path::PathBuf;

/// 获取历史记录目录
fn get_history_dir() -> anyhow::Result<PathBuf> {
    crate::paths::history_dir()
}

/// 加载插件配置
///
/// 配置存储在 `~/.worktools/history/plugins/<plugin_id>.json`
/// 如果文件不存在，返回空 JSON 对象 `{}`
///
/// ## Rust 知识点: serde_json::Value
/// `serde_json::Value` 是动态类型的 JSON 表示，适合处理结构不固定的数据。
/// 与之相对的是 `serde_json::from_str::<MyStruct>()` 用于结构固定的数据。
pub fn load_plugin_config(plugin_id: &str) -> Result<serde_json::Value> {
    let history_dir = get_history_dir()?;
    let config_path = history_dir.join(format!("plugins/{}.json", plugin_id));

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).context("读取插件配置失败")?;
        let config: serde_json::Value =
            serde_json::from_str(&content).context("解析插件配置失败")?;
        Ok(config)
    } else {
        // 返回空 JSON 对象 — 前端可以用它作为默认值
        Ok(serde_json::json!({}))
    }
}

/// 保存插件配置
///
/// 使用 `to_string_pretty` 输出格式化的 JSON（带缩进和换行），
/// 方便用户手动编辑和调试。
pub fn save_plugin_config(plugin_id: &str, config: &serde_json::Value) -> Result<()> {
    let history_dir = get_history_dir()?;
    let plugins_dir = history_dir.join("plugins");
    std::fs::create_dir_all(&plugins_dir).context("创建插件配置目录失败")?;

    let config_path = plugins_dir.join(format!("{}.json", plugin_id));
    let content = serde_json::to_string_pretty(config).context("序列化插件配置失败")?;

    std::fs::write(&config_path, content).context("写入插件配置失败")?;

    Ok(())
}
