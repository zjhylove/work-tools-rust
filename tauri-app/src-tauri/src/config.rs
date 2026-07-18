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
    if !crate::commands::validate_plugin_id(plugin_id) {
        anyhow::bail!("非法插件 ID 格式: {}", plugin_id);
    }

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
    if !crate::commands::validate_plugin_id(plugin_id) {
        anyhow::bail!("非法插件 ID 格式: {}", plugin_id);
    }

    let history_dir = get_history_dir()?;
    let plugins_dir = history_dir.join("plugins");
    std::fs::create_dir_all(&plugins_dir).context("创建插件配置目录失败")?;

    let config_path = plugins_dir.join(format!("{}.json", plugin_id));
    let content = serde_json::to_string_pretty(config).context("序列化插件配置失败")?;

    std::fs::write(&config_path, content).context("写入插件配置失败")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_plugin_config_rejects_path_traversal_id() {
        let result = load_plugin_config("../../etc/passwd");
        assert!(
            result.is_err(),
            "load_plugin_config should reject '../../etc/passwd'"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("非法插件 ID") || err_msg.contains("非法"),
            "Expected ID validation error, got: {}",
            err_msg
        );
    }

    #[test]
    fn load_plugin_config_rejects_dot_only_id() {
        let result = load_plugin_config("..");
        assert!(result.is_err());
    }

    #[test]
    fn load_plugin_config_rejects_slash_in_id() {
        let result = load_plugin_config("foo/bar");
        assert!(result.is_err());
    }

    #[test]
    fn save_plugin_config_rejects_path_traversal_id() {
        let config = serde_json::json!({"key": "value"});
        let result = save_plugin_config("../../etc/passwd", &config);
        assert!(
            result.is_err(),
            "save_plugin_config should reject '../../etc/passwd'"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("非法插件 ID") || err_msg.contains("非法"),
            "Expected ID validation error, got: {}",
            err_msg
        );
    }

    #[test]
    fn save_plugin_config_rejects_dot_only_id() {
        let config = serde_json::json!({});
        let result = save_plugin_config("..", &config);
        assert!(result.is_err());
    }

    #[test]
    fn save_plugin_config_rejects_slash_in_id() {
        let config = serde_json::json!({});
        let result = save_plugin_config("foo/bar", &config);
        assert!(result.is_err());
    }

    #[test]
    fn load_plugin_config_valid_id_returns_empty_when_missing() {
        // "valid-id" passes validation but the file doesn't exist,
        // so it should succeed with empty JSON object (not an error).
        let result = load_plugin_config("valid-id");
        assert!(
            result.is_ok(),
            "Valid ID with no config file should return Ok, got: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), serde_json::json!({}));
    }

    #[test]
    fn load_plugin_config_rejects_uppercase_id() {
        let result = load_plugin_config("MyPlugin");
        assert!(result.is_err());
    }

    #[test]
    fn load_plugin_config_rejects_empty_id() {
        let result = load_plugin_config("");
        assert!(result.is_err());
    }

    #[test]
    fn save_and_load_plugin_config_round_trip() {
        let config = serde_json::json!({"theme": "dark", "fontSize": 14});
        save_plugin_config("round-trip-test", &config).unwrap();

        let loaded = load_plugin_config("round-trip-test").unwrap();
        assert_eq!(loaded["theme"], "dark");
        assert_eq!(loaded["fontSize"], 14);

        // Cleanup
        let history_dir = crate::paths::history_dir().unwrap();
        let config_path = history_dir.join("plugins/round-trip-test.json");
        let _ = std::fs::remove_file(config_path);
    }

    #[test]
    fn save_plugin_config_overwrites_existing() {
        let config1 = serde_json::json!({"v": 1});
        save_plugin_config("overwrite-test", &config1).unwrap();

        let config2 = serde_json::json!({"v": 2, "extra": true});
        save_plugin_config("overwrite-test", &config2).unwrap();

        let loaded = load_plugin_config("overwrite-test").unwrap();
        assert_eq!(loaded["v"], 2);
        assert_eq!(loaded["extra"], true);

        // Cleanup
        let history_dir = crate::paths::history_dir().unwrap();
        let config_path = history_dir.join("plugins/overwrite-test.json");
        let _ = std::fs::remove_file(config_path);
    }
}
