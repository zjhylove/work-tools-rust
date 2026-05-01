//! # 共享数据类型
//!
//! 这个 crate 定义了主程序和插件之间共享的数据结构。
//! 它被放在独立的 crate 中是为了避免循环依赖：
//! 主程序依赖 types，插件依赖 plugin-api，两者都可以同时依赖 types。
//!
//! ## Rust 知识点
//! - `derive`: 自动为类型生成 trait 实现
//!   - `Debug`: 调试输出 `{:?}`
//!   - `Clone`: 创建值的深拷贝 `.clone()`
//!   - `Serialize`: 序列化为 JSON 等格式（由 serde 提供）
//!   - `Deserialize`: 从 JSON 等格式反序列化（由 serde 提供）

use serde::{Deserialize, Serialize};

/// 插件元信息
///
/// 这是插件向外部暴露的基本信息的集合。
/// 通过 `#[derive(Serialize, Deserialize)]`，
/// 这个结构体可以自动在 JSON 和 Rust 结构体之间转换。
///
/// ## Rust 知识点: 结构体 (struct)
/// Rust 的 struct 类似于其他语言的 class/record，
/// 但只包含数据，不包含方法（方法通过 impl 块添加）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// 插件唯一标识符，例如 "password-manager"
    pub id: String,
    /// 插件显示名称，例如 "密码管理器"
    pub name: String,
    /// 版本号
    pub version: String,
    /// 功能描述
    pub description: String,
    /// 图标（emoji 字符）
    pub icon: String,
}
