use std::fmt;

/// 插件错误类型
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

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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

impl std::error::Error for PluginError {}

/// 将字符串错误转换为插件错误
impl From<String> for PluginError {
    fn from(msg: String) -> Self {
        Self::Other(msg)
    }
}

/// 将 &str 转换为插件错误
impl From<&str> for PluginError {
    fn from(msg: &str) -> Self {
        Self::Other(msg.to_string())
    }
}

/// 插件结果类型
pub type PluginResult<T> = Result<T, PluginError>;

/// 辅助宏:创建方法调用失败错误
#[macro_export]
macro_rules! method_error {
    ($method:expr, $msg:expr) => {
        $crate::error::PluginError::MethodCallFailed {
            method: $method.to_string(),
            message: $msg.to_string(),
        }
    };
    ($method:expr, $fmt:expr, $($arg:tt)*) => {
        $crate::error::PluginError::MethodCallFailed {
            method: $method.to_string(),
            message: format!($fmt, $($arg)*),
        }
    };
}

/// 辅助宏:创建参数错误
#[macro_export]
macro_rules! param_error {
    ($msg:expr) => {
        $crate::error::PluginError::InvalidParameter($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::PluginError::InvalidParameter(format!($fmt, $($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PluginError::NotFound("test-plugin".to_string());
        assert_eq!(format!("{}", err), "插件未找到: test-plugin");

        let err = PluginError::MethodCallFailed {
            method: "test_method".to_string(),
            message: "test error".to_string(),
        };
        assert_eq!(format!("{}", err), "插件方法调用失败 [test_method]: test error");
    }

    #[test]
    fn test_method_error_macro() {
        let err = method_error!("test_method", "test error");
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
