use anyhow::Result;
use serde_json::Value;
use worktools_plugin_api::Plugin;

pub mod models;
pub mod database;
pub mod exporter;
pub mod storage;
pub mod crypto;

/// 数据库文档生成插件
pub struct DbDocPlugin;

impl Plugin for DbDocPlugin {
    fn id(&self) -> &str {
        "db-doc"
    }

    fn name(&self) -> &str {
        "数据库文档"
    }

    fn description(&self) -> &str {
        "连接数据库,自动生成表结构文档"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn icon(&self) -> &str {
        "📊"
    }

    fn get_view(&self) -> String {
        "<div>数据库文档生成器加载中...</div>".to_string()
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

/// 插件工厂函数
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(DbDocPlugin));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
