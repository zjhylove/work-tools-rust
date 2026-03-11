use anyhow::Result;
use serde_json::Value;
use worktools_plugin_api::Plugin;

/// 文本比对插件
pub struct TextDiff;

impl Plugin for TextDiff {
    fn id(&self) -> &str {
        "text-diff"
    }

    fn name(&self) -> &str {
        "文本比对"
    }

    fn description(&self) -> &str {
        "实时文本比对工具,支持差异高亮、文件导入导出、差异导航"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn icon(&self) -> &str {
        "🔍"
    }

    fn get_view(&self) -> String {
        // 插件已迁移到使用独立前端资源 (assets/index.html)
        // 此方法仅作为向后兼容的占位符
        "<div>插件前端资源加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        _params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            _ => Err(format!("未知方法: {}", method).into()),
        }
    }
}

/// 插件工厂函数 - 导出给动态库加载器
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(TextDiff));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
