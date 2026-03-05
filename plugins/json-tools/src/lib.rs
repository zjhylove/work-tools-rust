use serde_json::Value;
use worktools_plugin_api::Plugin;
use anyhow::Result;

pub struct JsonTools;

impl Plugin for JsonTools {
    fn id(&self) -> &str { "json-tools" }
    fn name(&self) -> &str { "JSON 工具" }
    fn description(&self) -> &str { "JSON 格式化、编辑和可视化工具" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "{ }" }
    fn get_view(&self) -> String {
        "<div>插件前端资源加载中...</div>".to_string()
    }

    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "format_json" => {
                let json_str = params.get("json")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

                let parsed: Value = serde_json::from_str(json_str)?;
                let formatted = serde_json::to_string_pretty(&parsed)?;
                Ok(serde_json::json!({ "result": formatted }))
            }
            "minify_json" => {
                let json_str = params.get("json")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

                let parsed: Value = serde_json::from_str(json_str)?;
                let minified = serde_json::to_string(&parsed)?;
                Ok(serde_json::json!({ "result": minified }))
            }
            "escape_json" => {
                let json_str = params.get("json")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

                let escaped = json_str.replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t");
                Ok(serde_json::json!({ "result": escaped }))
            }
            "unescape_json" => {
                let json_str = params.get("json")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

                let unescaped = json_str.replace("\\n", "\n")
                    .replace("\\r", "\r")
                    .replace("\\t", "\t")
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\");
                Ok(serde_json::json!({ "result": unescaped }))
            }
            "validate_json" => {
                let json_str = params.get("json")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

                match serde_json::from_str::<Value>(json_str) {
                    Ok(_) => Ok(serde_json::json!({ "valid": true, "error": null })),
                    Err(e) => {
                        let error_msg = e.to_string();
                        Ok(serde_json::json!({
                            "valid": false,
                            "error": error_msg
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
