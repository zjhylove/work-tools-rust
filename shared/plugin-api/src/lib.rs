use serde_json::Value;

pub mod storage;
pub mod error;

pub use error::{PluginError, PluginResult};

/// 插件 Trait - 所有插件必须实现此接口
pub trait Plugin: Send + Sync {
    /// 插件唯一标识符
    fn id(&self) -> &str;

    /// 插件显示名称
    fn name(&self) -> &str;

    /// 插件描述
    fn description(&self) -> &str;

    /// 插件版本
    fn version(&self) -> &str;

    /// 插件图标 (emoji 或图标名称)
    fn icon(&self) -> &str;

    /// 获取插件 UI 的 HTML 内容
    fn get_view(&self) -> String;

    /// 插件初始化 (可选实现)
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// 插件销毁时的清理 (可选实现)
    fn destroy(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// 处理来自前端的方法调用 (可选实现)
    fn handle_call(
        &mut self,
        _method: &str,
        _params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err("method not implemented".into())
    }

    /// 获取插件前端资源路径 (相对于插件目录)
    /// 默认返回 "assets",插件可以自定义
    fn get_assets_path(&self) -> &str {
        "assets"
    }
}

/// 插件工厂函数类型定义
/// 动态库必须导出此签名的 `plugin_create` 函数
pub type PluginCreateFn = unsafe extern "C" fn() -> *mut Box<dyn Plugin>;
