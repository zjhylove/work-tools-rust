//! # 插件 API 核心库
//!
//! 这个 crate 定义了整个插件系统的基础接口。
//! 所有插件都必须实现 `Plugin` trait，主程序通过这个 trait 与插件交互。
//!
//! ## 核心概念
//! - **Plugin trait**: 插件的统一接口，定义了插件必须提供的方法
//! - **PluginCreateFn**: 动态库导出的工厂函数签名，用于创建插件实例
//!
//! ## Rust 知识点
//! - `trait`: Rust 中的接口/协议，类似于其他语言的 interface
//! - `dyn Trait`: 动态分发，运行时才知道具体类型
//! - `Box<T>`: 堆上分配的值，用于存储大小不确定的类型
//! - `Send + Sync`: 标记 trait，表示类型可以安全地在线程间传递和共享

use serde_json::Value;

pub mod storage;
pub mod error;

// `pub use` 将子模块中的类型重新导出到当前模块，
// 这样外部只需 `use worktools_plugin_api::PluginError` 而不是 `use worktools_plugin_api::error::PluginError`
pub use error::{PluginError, PluginResult};

/// 插件 Trait —— 所有插件必须实现此接口
///
/// ## Rust 知识点: trait 作为接口
/// trait 是 Rust 中实现多态的主要方式之一。
/// 任何实现了这个 trait 的类型都可以被当作 `dyn Plugin` 使用。
///
/// ## Send + Sync 的含义
/// - `Send`: 该类型的所有权可以转移到另一个线程
/// - `Sync`: 该类型的引用 `&T` 可以在多个线程间共享
///   不加这两个标记，插件实例就无法在 Tauri 的异步运行时中使用
pub trait Plugin: Send + Sync {
    /// 插件唯一标识符，例如 "password-manager"
    /// 用于在系统中唯一标识一个插件
    fn id(&self) -> &str;

    /// 插件显示名称（中文），在前端 UI 中展示
    fn name(&self) -> &str;

    /// 插件描述，说明插件的功能
    fn description(&self) -> &str;

    /// 插件版本号，遵循语义化版本规范 (SemVer)
    fn version(&self) -> &str;

    /// 插件图标，可以是 emoji 或图标名称
    fn icon(&self) -> &str;

    /// 获取插件 UI 的 HTML 内容
    /// 返回一段 HTML 字符串，会被嵌入到前端的 iframe 中展示
    fn get_view(&self) -> String;

    // ── 以下方法都有默认实现（default implementation）──
    // 这意味着插件可以选择性地覆盖它们。
    // 这是 Rust trait 的强大特性：不强制实现所有方法。

    /// 插件初始化（可选实现）
    /// 当插件被加载时调用，用于执行一些启动时的设置工作
    ///
    /// ## Rust 知识点: 默认实现
    /// 方法体不是分号 `;` 而是 `{ Ok(()) }`，这提供了默认行为。
    /// 插件如果不需要初始化逻辑，就不需要覆盖这个方法。
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// 插件销毁时的清理（可选实现）
    /// 当插件被卸载时调用，用于释放资源（如网络连接、文件句柄等）
    fn destroy(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// 处理来自前端的方法调用（可选实现）
    ///
    /// ## 参数说明
    /// - `_method`: 方法名，前导下划线表示"此变量暂未使用"，编译器不会警告
    /// - `_params`: JSON 格式的参数，使用 serde_json::Value 类型
    ///
    /// ## 返回值
    /// - 成功时返回 JSON 值
    /// - 失败时返回错误，使用 `Box<dyn Error>` 可以包装任意类型的错误
    ///
    /// ## Rust 知识点: trait object (dyn Error)
    /// `Box<dyn std::error::Error>` 是"装箱的 trait 对象"：
    /// - `Box` 把数据放在堆上（因为 trait 对象大小在编译时未知）
    /// - `dyn Error` 表示"任何实现了 Error trait 的类型"
    /// - 这样函数可以返回不同类型的错误
    fn handle_call(
        &mut self,
        _method: &str,
        _params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // 默认实现：返回错误，表示该方法未实现
        // `into()` 将 &str 转换为 Box<dyn Error>
        Err("method not implemented".into())
    }

    /// 获取插件前端资源路径（相对于插件目录）
    /// 默认返回 "assets"，插件可以自定义（比如资源放在 "dist" 目录）
    fn get_assets_path(&self) -> &str {
        "assets"
    }
}

/// 插件工厂函数类型定义
///
/// ## Rust 知识点: 函数指针类型
/// `type PluginCreateFn = unsafe extern "C" fn() -> *mut Box<dyn Plugin>;`
/// 这定义了一个类型别名，表示一个函数指针：
/// - `unsafe extern "C"`: 使用 C 语言调用约定（ABI），这是动态库互操作的标准
/// - `fn()`: 函数不接受参数
/// - `-> *mut Box<dyn Plugin>`: 返回一个原始指针，指向堆上的 Box<dyn Plugin>
///
/// ## 为什么用原始指针？
/// 动态库（dll/so/dylib）的边界上必须使用 C 兼容的类型。
/// Rust 的 `Box` 不是 C 兼容的，所以用原始指针 `*mut` 来传递。
/// 调用方需要用 `unsafe { Box::from_raw(ptr) }` 重新获得 Rust 的所有权语义。
///
/// ## 为什么有两层 Box？
/// `Box<Box<dyn Plugin>>` 看起来多余，实际上是为了：
/// - 外层 Box: 提供固定大小的指针（fat pointer，包含数据指针 + vtable 指针）
/// - 内层 Box: 存储实际的插件数据（大小未知，必须在堆上）
/// 这样 `*mut Box<dyn Plugin>` 就是一个已知大小的指针，可以跨越 FFI 边界
pub type PluginCreateFn = unsafe extern "C" fn() -> *mut Box<dyn Plugin>;
