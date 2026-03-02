use worktools_plugin_api::Plugin;
use serde_json::Value;

pub struct TestPlugin;

impl Plugin for TestPlugin {
    fn id(&self) -> &str {
        "test-plugin"
    }

    fn name(&self) -> &str {
        "测试插件"
    }

    fn description(&self) -> &str {
        "这是一个测试插件,用于验证插件商店功能"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn icon(&self) -> &str {
        "🧪"
    }

    fn get_view(&self) -> String {
        // 这个方法不会被使用,因为我们有独立的前端资源
        // 但为了兼容性,仍然提供一个简单的 HTML
        r#"<div style="padding: 20px;"><h2>测试插件</h2><p>传统 HTML 模式</p></div>"#.to_string()
    }

    fn handle_call(&mut self, _method: &str, _params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // 简单的测试方法
        Ok(serde_json::json!({
            "status": "success",
            "message": "Test plugin is working!"
        }))
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(TestPlugin));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}

