use anyhow::Context;
use chrono::Local;
use cron::Schedule;
use serde_json::Value;
use std::str::FromStr;
use worktools_plugin_api::Plugin;

pub struct CronTools;

const STANDARD_FIELDS: [(&str, &str); 5] = [
    ("minute", "分钟"),
    ("hour", "小时"),
    ("day_of_month", "日"),
    ("month", "月"),
    ("day_of_week", "周"),
];

fn describe_field(value: &str, field_name: &str) -> String {
    if value == "*" {
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

/// 将 5 字段标准 cron 转为 cron crate 要求的 7 字段格式
fn to_7_field(expr: &str) -> String {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() == 7 {
        return expr.to_string();
    }
    format!("0 {} *", expr)
}

fn describe_cron(expr: &str) -> String {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return "无效的 cron 表达式（需要5个字段）".to_string();
    }
    let parts: Vec<String> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| describe_field(f, STANDARD_FIELDS[i].1))
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

                if expr.split_whitespace().count() != 5 {
                    return Ok(serde_json::json!({
                        "valid": false,
                        "description": "无效的 cron 表达式（需要5个字段）",
                        "error": "表达式需要5个空格分隔的字段"
                    }));
                }

                match Schedule::from_str(&to_7_field(expr)) {
                    Ok(_) => Ok(serde_json::json!({
                        "valid": true,
                        "description": describe_cron(expr),
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

                let schedule = Schedule::from_str(&to_7_field(expr.trim())).context("cron 表达式解析失败")?;

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
