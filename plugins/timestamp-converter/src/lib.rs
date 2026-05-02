use anyhow::Context;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::Value;
use worktools_plugin_api::Plugin;

pub struct TimestampConverter;

fn parse_timestamp(ts_str: &str) -> anyhow::Result<i64> {
    let ts_str = ts_str.trim();
    match ts_str.len() {
        10 => ts_str.parse::<i64>().context("时间戳解析失败"),
        13 => Ok(ts_str.parse::<i64>().context("时间戳解析失败")? / 1000),
        16 => Ok(ts_str.parse::<i64>().context("时间戳解析失败")? / 1_000_000),
        _ => Err(anyhow::anyhow!("无法识别时间戳格式，请输入10/13/16位数字")),
    }
}

fn parse_datetime_to_ts(dt_str: &str, tz: &Tz) -> Option<i64> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(dt_str) {
        return Some(dt.timestamp());
    }
    if let Ok(dt) = DateTime::parse_from_rfc2822(dt_str) {
        return Some(dt.timestamp());
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%d %H:%M:%S") {
        return tz.from_local_datetime(&naive).single().map(|d| d.timestamp());
    }
    None
}

fn parse_timezone(tz_str: Option<&str>) -> Tz {
    tz_str
        .and_then(|s| s.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::Asia::Shanghai)
}

fn format_datetimes(ts_sec: i64, tz: Tz) -> Value {
    let utc_dt = match Utc.timestamp_opt(ts_sec, 0) {
        chrono::LocalResult::Single(dt) => dt,
        _ => return serde_json::json!({ "error": "时间戳超出范围" }),
    };
    let local_dt = utc_dt.with_timezone(&tz);
    serde_json::json!({
        "utc": utc_dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        "datetime": local_dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        "timezone": tz.name(),
        "format_iso": local_dt.to_rfc3339(),
        "format_rfc2822": local_dt.to_rfc2822(),
    })
}

impl Plugin for TimestampConverter {
    fn id(&self) -> &str {
        "timestamp-converter"
    }
    fn name(&self) -> &str {
        "时间戳转换"
    }
    fn description(&self) -> &str {
        "Unix时间戳与日期时间互相转换，多时区、批量处理"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn icon(&self) -> &str {
        "⏰"
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
            "timestamp_to_datetime" => {
                let ts_str = params
                    .get("ts")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 ts 参数")?;
                let tz_str = params.get("timezone").and_then(|v| v.as_str());
                let ts_sec = parse_timestamp(ts_str)?;
                let tz = parse_timezone(tz_str);
                Ok(format_datetimes(ts_sec, tz))
            }

            "datetime_to_timestamp" => {
                let dt_str = params
                    .get("datetime")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 datetime 参数")?;
                let tz_str = params.get("timezone").and_then(|v| v.as_str());
                let tz = parse_timezone(tz_str);

                let ts_sec = parse_datetime_to_ts(dt_str, &tz)
                    .ok_or("无法解析日期格式，支持: ISO 8601 / RFC 2822 / YYYY-MM-DD HH:MM:SS / YYYY-MM-DD")?;

                Ok(serde_json::json!({
                    "ts_sec": ts_sec,
                    "ts_ms": ts_sec * 1000i64,
                    "ts_us": ts_sec * 1_000_000i64,
                }))
            }

            "current_time" => {
                let tz_str = params.get("timezone").and_then(|v| v.as_str());
                let tz = parse_timezone(tz_str);
                let now = Utc::now();
                let ts_sec = now.timestamp();
                let ts_ms = now.timestamp_millis();
                let mut result = format_datetimes(ts_sec, tz);
                if let Some(obj) = result.as_object_mut() {
                    obj.insert("ts_sec".into(), serde_json::json!(ts_sec));
                    obj.insert("ts_ms".into(), serde_json::json!(ts_ms));
                }
                Ok(result)
            }

            "batch_convert" => {
                let items = params
                    .get("items")
                    .and_then(|v| v.as_array())
                    .ok_or("缺少 items 数组")?;
                let tz_str = params.get("timezone").and_then(|v| v.as_str());
                let tz = parse_timezone(tz_str);

                let results: Vec<Value> = items.iter().map(|item| {
                    let value = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
                    let direction = item.get("direction").and_then(|v| v.as_str()).unwrap_or("to_datetime");

                    if direction == "to_datetime" {
                        match parse_timestamp(value) {
                            Ok(ts_sec) => format_datetimes(ts_sec, tz),
                            Err(e) => serde_json::json!({ "input": value, "error": e.to_string() }),
                        }
                    } else {
                        match parse_datetime_to_ts(value, &tz) {
                            Some(ts) => serde_json::json!({ "input": value, "ts_sec": ts, "ts_ms": ts * 1000i64 }),
                            None => serde_json::json!({ "input": value, "error": "无法解析" }),
                        }
                    }
                }).collect();

                Ok(serde_json::json!({ "results": results }))
            }

            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(TimestampConverter));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
