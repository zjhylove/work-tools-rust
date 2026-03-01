use worktools_plugin_api::Plugin;
use serde_json::Value;

/// Auth Plugin - 双因素认证
pub struct AuthPlugin;

impl Plugin for AuthPlugin {
    fn id(&self) -> &str {
        "auth"
    }

    fn name(&self) -> &str {
        "双因素验证"
    }

    fn description(&self) -> &str {
        "TOTP 双因素认证"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn icon(&self) -> &str {
        "🔐"
    }

    fn get_view(&self) -> String {
        r#"
        <div id="auth-app">
            <h2>双因素验证</h2>
            <p>Auth Plugin (开发中)</p>
        </div>
        "#.to_string()
    }

    fn handle_call(&mut self, method: &str, _params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "list_entries" => Ok(serde_json::json!({ "entries": [] })),
            _ => Err(format!("未知方法: {}", method).into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(AuthPlugin));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
