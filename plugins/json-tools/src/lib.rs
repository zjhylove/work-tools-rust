//! # JSON 工具插件
//!
//! 提供 JSON 格式化、压缩、转义/反转义、验证等工具函数。
//! 这是最简单的插件实现，不涉及数据持久化。
//!
//! ## Rust 知识点
//! - `serde_json::Value`: 通用的 JSON 值类型
//! - `serde_json::from_str`: 解析 JSON 字符串 → Value
//! - `serde_json::to_string_pretty`: 格式化输出（带缩进）
//! - `serde_json::to_string`: 紧凑输出（无多余空格）

use anyhow::Result;
use serde_json::Value;
use worktools_plugin_api::Plugin;

/// JSON 工具插件（无状态结构体）
pub struct JsonTools;

impl Plugin for JsonTools {
    fn id(&self) -> &str {
        "json-tools"
    }
    fn name(&self) -> &str {
        "JSON 工具"
    }
    fn description(&self) -> &str {
        "JSON格式化、编辑工具"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn icon(&self) -> &str {
        "{ }"
    }
    fn get_view(&self) -> String {
        "<div>插件前端资源加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            // ── 格式化 JSON: 添加缩进和换行 ──
            "format_json" => {
                let json_str = params
                    .get("json")
                    .and_then(|v| v.as_str()) // Value → Option<&str>
                    .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

                // 先解析验证，然后格式化输出
                let parsed: Value = serde_json::from_str(json_str)?;
                let formatted = serde_json::to_string_pretty(&parsed)?;
                tracing::info!(size = json_str.len(), "格式化 JSON");
                Ok(serde_json::json!({ "result": formatted }))
            }

            // ── 压缩 JSON: 移除所有空白字符 ──
            "minify_json" => {
                let json_str = params
                    .get("json")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

                let parsed: Value = serde_json::from_str(json_str)?;
                // `to_string` 输出紧凑 JSON（无缩进、无多余空格）
                let minified = serde_json::to_string(&parsed)?;
                tracing::info!(size = json_str.len(), "压缩 JSON");
                Ok(serde_json::json!({ "result": minified }))
            }

            // ── 转义 JSON 字符串中的特殊字符 ──
            // 用于将 JSON 嵌入到其他上下文中（如 URL、HTML）
            "escape_json" => {
                let json_str = params
                    .get("json")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

                let escaped = json_str
                    .replace('\\', "\\\\") // 反斜杠（必须先转义，否则会干扰后续替换）
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t");
                Ok(serde_json::json!({ "result": escaped }))
            }

            // ── 反转义 JSON 字符串 ──
            "unescape_json" => {
                let json_str = params
                    .get("json")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

                let unescaped = json_str
                    .replace("\\n", "\n")
                    .replace("\\r", "\r")
                    .replace("\\t", "\t")
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\");
                Ok(serde_json::json!({ "result": unescaped }))
            }

            // ── 验证 JSON 字符串是否合法 ──
            "validate_json" => {
                let json_str = params
                    .get("json")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

                // `serde_json::from_str::<Value>` 解析并验证
                match serde_json::from_str::<Value>(json_str) {
                    Ok(_) => Ok(serde_json::json!({ "valid": true, "error": null })),
                    Err(e) => {
                        // 将解析错误信息返回给前端
                        Ok(serde_json::json!({
                            "valid": false,
                            "error": e.to_string()
                        }))
                    }
                }
            }

            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(JsonTools));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_plugin_info() {
        let plugin = JsonTools;
        assert_eq!(plugin.id(), "json-tools");
        assert_eq!(plugin.name(), "JSON 工具");
        assert_eq!(plugin.version(), "1.0.0");
        assert!(!plugin.icon().is_empty());
        assert!(!plugin.get_view().is_empty());
    }

    #[test]
    fn test_unknown_method_returns_error() {
        let mut plugin = JsonTools;
        let result = plugin.handle_call("nonexistent", json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_format_json() {
        let mut plugin = JsonTools;
        let result = plugin.handle_call("format_json", json!({"json": r#"{"a":1,"b":2}"#}));
        assert!(result.is_ok());
        let val = result.unwrap();
        let formatted = val.get("result").unwrap().as_str().unwrap();
        assert!(formatted.contains('\n'));
        assert!(formatted.contains("  "));
    }

    #[test]
    fn test_minify_json() {
        let mut plugin = JsonTools;
        let result = plugin.handle_call("minify_json", json!({"json": "{\n  \"a\": 1\n}"}));
        assert!(result.is_ok());
        let val = result.unwrap();
        assert_eq!(val.get("result").unwrap().as_str().unwrap(), r#"{"a":1}"#);
    }

    #[test]
    fn test_validate_json_valid() {
        let mut plugin = JsonTools;
        let result = plugin.handle_call("validate_json", json!({"json": r#"{"ok":true}"#}));
        assert!(result.is_ok());
        assert!(result.unwrap().get("valid").unwrap().as_bool().unwrap());
    }
    #[test]
    fn test_validate_json_invalid() {
        let mut plugin = JsonTools;
        let result = plugin.handle_call("validate_json", json!({"json": "{bad}"}));
        assert!(result.is_ok());
        let val = result.unwrap();
        assert!(!val.get("valid").unwrap().as_bool().unwrap());
        // Error message should be non-empty (exact wording depends on serde_json version)
        assert!(!val.get("error").unwrap().as_str().unwrap().is_empty());
    }

    #[test]
    fn test_missing_param_returns_error() {
        let mut plugin = JsonTools;
        let result = plugin.handle_call("format_json", json!({}));
        assert!(result.is_err());
    }
}
