use anyhow::Context;
use chrono::Local;
use cron::Schedule;
use serde_json::Value;
use std::str::FromStr;
use worktools_plugin_api::Plugin;

pub struct CronTools;

#[derive(Debug, Clone, Copy, PartialEq)]
enum CronFormat {
    Standard5,
    WithSeconds6,
    Quartz7,
}

const FIELDS_5: [(&str, &str); 5] = [
    ("minute", "分钟"),
    ("hour", "小时"),
    ("day_of_month", "日"),
    ("month", "月"),
    ("day_of_week", "周"),
];

const FIELDS_6: [(&str, &str); 6] = [
    ("second", "秒"),
    ("minute", "分钟"),
    ("hour", "小时"),
    ("day_of_month", "日"),
    ("month", "月"),
    ("day_of_week", "周"),
];

const FIELDS_7: [(&str, &str); 7] = [
    ("second", "秒"),
    ("minute", "分钟"),
    ("hour", "小时"),
    ("day_of_month", "日"),
    ("month", "月"),
    ("day_of_week", "周"),
    ("year", "年"),
];

fn describe_field(value: &str, field_name: &str) -> String {
    if value == "*" || value == "?" {
        return format!("每{}", field_name);
    }
    if value.contains('/') {
        let parts: Vec<&str> = value.split('/').collect();
        if parts.len() == 2 {
            let base = if parts[0] == "*" {
                "每".to_string()
            } else {
                format!("从第{}", parts[0])
            };
            return format!(
                "{}{}{}执行",
                base,
                field_name,
                match parts[1] {
                    "1" => "".to_string(),
                    n => format!("间隔{}", n),
                }
            );
        }
    }
    if value.contains(',') {
        let nums: Vec<&str> = value.split(',').collect();
        return format!("{}的第{}", field_name, nums.join("、"));
    }
    if value.contains('-') {
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() == 2 {
            return format!("{}从{}到{}", field_name, parts[0], parts[1]);
        }
    }
    format!("{}为{}", field_name, value)
}

fn detect_format(expr: &str) -> Option<CronFormat> {
    match expr.split_whitespace().count() {
        5 => Some(CronFormat::Standard5),
        6 => Some(CronFormat::WithSeconds6),
        7 => Some(CronFormat::Quartz7),
        _ => None,
    }
}

fn normalize_to_7_field(expr: &str) -> Option<String> {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    match fields.len() {
        5 => Some(format!("0 {} *", expr)),
        6 => Some(format!("{} *", expr)),
        7 => Some(expr.to_string()),
        _ => None,
    }
}

fn format_name(fmt: CronFormat) -> &'static str {
    match fmt {
        CronFormat::Standard5 => "5段 (Unix)",
        CronFormat::WithSeconds6 => "6段 (含秒)",
        CronFormat::Quartz7 => "7段 (Quartz)",
    }
}

fn describe_cron(expr: &str) -> String {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    let field_defs: &[(&str, &str)] = match fields.len() {
        5 => &FIELDS_5,
        6 => &FIELDS_6,
        7 => &FIELDS_7,
        _ => return "无效的 cron 表达式".to_string(),
    };
    let parts: Vec<String> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| describe_field(f, field_defs[i].1))
        .collect();
    parts.join("，")
}

impl Plugin for CronTools {
    fn id(&self) -> &str {
        "cron-tools"
    }
    fn name(&self) -> &str {
        "Cron 表达式"
    }
    fn description(&self) -> &str {
        "Cron表达式解析、人类可读描述、下次执行时间预览"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn icon(&self) -> &str {
        "⏱"
    }
    fn get_view(&self) -> String {
        "<div>插件资源加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "parse_cron" => {
                let expr = params
                    .get("expr")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 expr 参数")?;
                let expr = expr.trim();

                let fmt = match detect_format(expr) {
                    Some(f) => f,
                    None => {
                        return Ok(serde_json::json!({
                            "valid": false,
                            "description": "无效的 cron 表达式（需要 5、6 或 7 个字段）",
                            "error": "表达式需要 5、6 或 7 个空格分隔的字段"
                        }));
                    }
                };

                let normalized = normalize_to_7_field(expr).unwrap();

                match Schedule::from_str(&normalized) {
                    Ok(_) => Ok(serde_json::json!({
                        "valid": true,
                        "description": describe_cron(expr),
                        "format": format_name(fmt),
                        "error": null,
                    })),
                    Err(e) => Ok(serde_json::json!({
                        "valid": false,
                        "description": format!("无效表达式: {}", e),
                        "error": e.to_string(),
                    })),
                }
            }

            "next_executions" => {
                let expr = params
                    .get("expr")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 expr 参数")?;
                let count = params.get("count").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                let count = count.min(20);

                let normalized = normalize_to_7_field(expr.trim())
                    .ok_or("无效的 cron 表达式（需要 5、6 或 7 个字段）")?;
                let schedule = Schedule::from_str(&normalized).context("cron 表达式解析失败")?;

                let times: Vec<String> = schedule
                    .upcoming(Local)
                    .take(count)
                    .map(|dt| dt.to_rfc3339())
                    .collect();

                Ok(serde_json::json!({ "times": times }))
            }

            "get_presets" => Ok(serde_json::json!({
                "presets": [
                    { "label": "每分钟", "expr": "* * * * *" },
                    { "label": "每5分钟", "expr": "*/5 * * * *" },
                    { "label": "每15分钟", "expr": "*/15 * * * *" },
                    { "label": "每小时", "expr": "0 * * * *" },
                    { "label": "每天凌晨", "expr": "0 0 * * *" },
                    { "label": "每天上午9点", "expr": "0 9 * * *" },
                    { "label": "工作日上午9点", "expr": "0 9 * * 1-5" },
                    { "label": "每月1号凌晨", "expr": "0 0 1 * *" },
                    { "label": "每周一凌晨", "expr": "0 0 * * 1" },
                    { "label": "每30秒", "expr": "*/30 * * * * *" },
                    { "label": "整点每分钟", "expr": "0 * * * * *" },
                    { "label": "每天零点(Quartz)", "expr": "0 0 0 * * ? *" },
                    { "label": "每小时整点(Quartz)", "expr": "0 0 * * * ? *" },
                    { "label": "工作日9:30(Quartz)", "expr": "0 30 9 ? * MON-FRI *" },
                    { "label": "每月1号零点(Quartz)", "expr": "0 0 0 1 * ? *" },
                ]
            })),

            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(CronTools));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_plugin_info() {
        let plugin = CronTools;
        assert_eq!(plugin.id(), "cron-tools");
        assert_eq!(plugin.name(), "Cron 表达式");
        assert_eq!(plugin.version(), "1.0.0");
        assert!(!plugin.icon().is_empty());
        assert!(!plugin.get_view().is_empty());
    }

    #[test]
    fn test_unknown_method_returns_error() {
        let mut plugin = CronTools;
        let result = plugin.handle_call("nonexistent", json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_format() {
        assert_eq!(detect_format("* * * * *"), Some(CronFormat::Standard5));
        assert_eq!(detect_format("*/30 * * * * *"), Some(CronFormat::WithSeconds6));
        assert_eq!(detect_format("0 0 0 * * ? *"), Some(CronFormat::Quartz7));
        assert_eq!(detect_format("bad"), None);
        assert_eq!(detect_format("1 2 3 4 5 6 7 8"), None);
    }

    #[test]
    fn test_normalize_to_7_field() {
        // 5-field "a b c d e" → prepend "0 " to make 7 fields: "0 a b c d e *"
        assert_eq!(normalize_to_7_field("* * * * *").unwrap(), "0 * * * * * *");
        // 6-field "a b c d e f" → append " *" to make 7 fields
        assert_eq!(normalize_to_7_field("*/30 * * * * *").unwrap(), "*/30 * * * * * *");
        // 7-field stays the same
        assert_eq!(normalize_to_7_field("0 0 0 * * ? *").unwrap(), "0 0 0 * * ? *");
        assert!(normalize_to_7_field("bad").is_none());
    }

    #[test]
    fn test_format_name() {
        assert_eq!(format_name(CronFormat::Standard5), "5段 (Unix)");
        assert_eq!(format_name(CronFormat::WithSeconds6), "6段 (含秒)");
        assert_eq!(format_name(CronFormat::Quartz7), "7段 (Quartz)");
    }

    #[test]
    fn test_parse_cron_valid() {
        let mut plugin = CronTools;
        let result = plugin.handle_call("parse_cron", json!({"expr": "*/5 * * * *"}));
        assert!(result.is_ok());
        let val = result.unwrap();
        assert!(val.get("valid").unwrap().as_bool().unwrap());
        assert!(val.get("description").unwrap().as_str().unwrap().contains("每"));
    }

    #[test]
    fn test_parse_cron_invalid() {
        let mut plugin = CronTools;
        let result = plugin.handle_call("parse_cron", json!({"expr": "invalid"}));
        assert!(result.is_ok());
        let val = result.unwrap();
        assert!(!val.get("valid").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_next_executions() {
        let mut plugin = CronTools;
        let result = plugin.handle_call("next_executions", json!({"expr": "* * * * *", "count": 3}));
        assert!(result.is_ok());
        let val = result.unwrap();
        let times = val.get("times").unwrap().as_array().unwrap();
        assert_eq!(times.len(), 3);
    }

    #[test]
    fn test_get_presets() {
        let mut plugin = CronTools;
        let result = plugin.handle_call("get_presets", json!({}));
        assert!(result.is_ok());
        let val = result.unwrap();
        let presets = val.get("presets").unwrap().as_array().unwrap();
        assert!(!presets.is_empty());
    }
}
