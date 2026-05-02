//! # 插件错误类型
//!
//! 定义了插件系统中可能出现的各种错误类型，以及辅助宏。
//!
//! ## Rust 知识点
//! - `enum`: Rust 的枚举是代数数据类型（ADT），每个变体可以携带数据
//! - `derive(Debug)`: 自动生成 Debug trait 实现，用于调试输出
//! - `impl fmt::Display`: 实现 Display trait，控制用户可读的错误信息
//! - `impl std::error::Error`: 实现 Error trait，使该类型可以被 `?` 操作符传播
//! - `impl From<T>`: 实现类型转换，使 `?` 操作符能自动将一种错误转为另一种

use std::fmt;

/// 插件错误类型
///
/// ## Rust 知识点: 枚举变体携带数据
/// Rust 的枚举每个变体都可以携带不同类型的数据，这使得错误处理非常灵活：
/// - `NotFound(String)`: 携带插件 ID 字符串
/// - `MethodCallFailed { method, message }`: 命名字段，携带方法名和错误消息
#[derive(Debug)]
pub enum PluginError {
    /// 插件未找到
    NotFound(String),
    /// 插件加载失败
    LoadFailed(String),
    /// 插件初始化失败
    InitializationFailed(String),
    /// 插件方法调用失败
    MethodCallFailed { method: String, message: String },
    /// 数据存储失败
    StorageFailed(String),
    /// 序列化/反序列化失败
    SerializationFailed(String),
    /// 参数错误
    InvalidParameter(String),
    /// 未实现的方法
    MethodNotImplemented(String),
    /// 其他错误
    Other(String),
}

/// 实现 Display trait，定义错误的用户可读描述
///
/// ## Rust 知识点: Display trait
/// `Display` 决定了类型在 `println!("{}", x)` 或 `format!("{}", x)` 中的输出。
/// 它与 `Debug`（`{:?}`）不同：
/// - `Display`: 给用户看的，简洁清晰
/// - `Debug`: 给开发者看的，包含更多内部细节
///
/// `Formatter<'_>` 中的 `'_` 是生命周期省略，表示 formatter 的引用的生命周期。
impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `match` 模式匹配，Rust 的核心控制流结构
        // 编译器会检查是否覆盖了所有变体（exhaustive matching）
        match self {
            // `Self` 是当前 impl 块类型的别名 = PluginError
            Self::NotFound(id) => write!(f, "插件未找到: {}", id),
            Self::LoadFailed(msg) => write!(f, "插件加载失败: {}", msg),
            Self::InitializationFailed(msg) => write!(f, "插件初始化失败: {}", msg),
            Self::MethodCallFailed { method, message } => {
                write!(f, "插件方法调用失败 [{}]: {}", method, message)
            }
            Self::StorageFailed(msg) => write!(f, "数据存储失败: {}", msg),
            Self::SerializationFailed(msg) => write!(f, "序列化失败: {}", msg),
            Self::InvalidParameter(msg) => write!(f, "参数错误: {}", msg),
            Self::MethodNotImplemented(method) => write!(f, "方法未实现: {}", method),
            Self::Other(msg) => write!(f, "插件错误: {}", msg),
        }
    }
}

/// 实现 Error trait，使 PluginError 可以被 `?` 操作符传播
///
/// ## Rust 知识点: Error trait
/// 空实现（没有方法体）是因为 Error trait 的所有方法都有默认实现。
/// 只要实现了 Display + Debug，就可以自动获取 Error 的完整功能。
/// 这使得 `anyhow::Error` 等库能够包装 PluginError。
impl std::error::Error for PluginError {}

/// 将 String 转换为插件错误
///
/// ## Rust 知识点: From trait 与 ? 操作符
/// `impl From<String> for PluginError` 意味着：
/// 在任何期望 `PluginError` 的地方，可以直接使用 `String` — 编译器会自动转换。
///
/// 更重要的是 `?` 操作符的自动转换：
/// ```ignore
/// fn foo() -> Result<(), PluginError> {
///     let s: String = some_func()?; // 如果 some_func 返回 Result<T, String>
/// }
/// ```
/// `?` 会自动调用 `From::from()` 将 `String` 转为 `PluginError`。
impl From<String> for PluginError {
    fn from(msg: String) -> Self {
        Self::Other(msg)
    }
}

/// 将 &str 转换为插件错误
/// 与上面的 String 实现类似，但用于 &str（字符串切片/借用）
impl From<&str> for PluginError {
    fn from(msg: &str) -> Self {
        // `to_string()` 将借用的 &str 转换为拥有的 String
        Self::Other(msg.to_string())
    }
}

/// 插件结果类型 —— 所有插件操作的标准返回类型
///
/// ## Rust 知识点: 类型别名
/// `pub type PluginResult<T> = Result<T, PluginError>;`
/// 这是类型别名，不是新类型。`PluginResult<T>` 和 `Result<T, PluginError>` 完全等价。
/// 这样写更简洁，而且可以统一修改错误类型（只需改这一处）。
pub type PluginResult<T> = Result<T, PluginError>;

/// 辅助宏：创建"方法调用失败"错误
///
/// ## Rust 知识点: 声明宏 (macro_rules!)
/// Rust 的宏在编译时展开，生成代码。宏名后面的 `!` 是区分宏和普通函数调用的标志。
///
/// `#[macro_export]` 使宏在 crate 外部也可用。
/// 这个宏有两个分支（通过模式匹配选择）：
/// 1. `($method:expr, $msg:expr)` — 两个简单的表达式
/// 2. `($method:expr, $fmt:expr, $($arg:tt)*)` — 支持格式化字符串，类似 `format!()`
///
/// `$crate` 是一个特殊变量，指向当前 crate 的根，确保宏在任何地方都能正确引用 `PluginError`。
#[macro_export]
macro_rules! method_error {
    // 分支1: 简单的字符串消息
    ($method:expr, $msg:expr) => {
        $crate::error::PluginError::MethodCallFailed {
            method: $method.to_string(),
            message: $msg.to_string(),
        }
    };
    // 分支2: 带格式化参数的消息（如 method_error!("my_method", "error: {}", code)）
    // `$($arg:tt)*` 匹配零个或多个 token，非常灵活
    ($method:expr, $fmt:expr, $($arg:tt)*) => {
        $crate::error::PluginError::MethodCallFailed {
            method: $method.to_string(),
            message: format!($fmt, $($arg)*),
        }
    };
}

/// 辅助宏：创建"参数错误"
/// 使用方式与上面的 method_error! 宏相同
#[macro_export]
macro_rules! param_error {
    ($msg:expr) => {
        $crate::error::PluginError::InvalidParameter($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::PluginError::InvalidParameter(format!($fmt, $($arg)*))
    };
}

/// 测试模块
///
/// ## Rust 知识点: #[cfg(test)] 条件编译
/// `#[cfg(test)]` 表示这个模块只在 `cargo test` 时编译。
/// 这样测试代码不会增加编译产物的体积。
///
/// `#[test]` 标记一个函数为测试函数，`cargo test` 会自动发现并运行它们。
#[cfg(test)]
mod tests {
    // `use super::*` 导入父模块的所有公共项
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PluginError::NotFound("test-plugin".to_string());
        // `format!("{}", err)` 调用 Display trait 的 fmt 方法
        assert_eq!(format!("{}", err), "插件未找到: test-plugin");

        let err = PluginError::MethodCallFailed {
            method: "test_method".to_string(),
            message: "test error".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "插件方法调用失败 [test_method]: test error"
        );
    }

    #[test]
    fn test_method_error_macro() {
        let err = method_error!("test_method", "test error");
        // `assert!` 检查条件为 true
        // `matches!` 宏检查值是否匹配某个模式
        assert!(matches!(
            err,
            PluginError::MethodCallFailed { method, message: _ }
            if method == "test_method"
        ));
    }

    #[test]
    fn test_param_error_macro() {
        let err = param_error!("missing parameter");
        assert!(matches!(
            err,
            PluginError::InvalidParameter(msg) if msg == "missing parameter"
        ));
    }
}
