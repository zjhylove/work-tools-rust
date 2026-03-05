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

    fn handle_call(&mut self, method: &str, _params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(JsonTools));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
