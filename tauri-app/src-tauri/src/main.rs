//! # 应用入口点
//!
//! 这是程序的入口文件。在 Rust 中，`main.rs` 包含 `fn main()` 函数。
//!
//! ## Rust 知识点: 条件编译属性
//! `#![...]` 是内部属性（inner attribute），作用于整个 crate。
//! `#[...]` 是外部属性（outer attribute），作用于下一个项。
//!
//! `#[cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`
//! - `cfg_attr(条件, 属性)`: 条件编译属性
//! - `not(debug_assertions)`: 在 release 模式下为 true
//! - `windows_subsystem = "windows"`: Windows 平台不显示控制台窗口
//!   在 debug 模式下保留控制台（方便查看日志），在 release 模式隐藏

// 在 Windows release 模式下不显示控制台窗口，不要删除此行！
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 将实际逻辑放在 lib.rs 中，main.rs 只负责调用
    // 这是 Rust 应用的常见模式：
    // - main.rs: 最小化的入口
    // - lib.rs: 实际的应用逻辑（可被测试和文档测试引用）
    work_tools_lib::run()
}
