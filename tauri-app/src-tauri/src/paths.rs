//! # 应用路径管理
//!
//! 集中管理所有应用目录，遵循各平台的约定。
//!
//! ## 目录结构
//! ```text
//! ~/.worktools/              # 应用根目录
//! ├── plugins/               # 已安装插件
//! ├── config/                # 配置文件 (注册表等)
//! ├── logs/                  # 日志文件
//! └── history/               # 历史数据
//!     └── plugins/           # 插件持久化数据
//! ```
//!
//! ## Rust 知识点
//! - `directories` crate: 跨平台获取标准用户目录
//! - `PathBuf`: 拥有的路径类型（相对于 `&Path` 的借用）
//! - `anyhow::Result`: 灵活的错误类型，适用于应用代码
//! - `ok_or_else`: 将 Option 转换为 Result，惰性求值

use anyhow::Result;
use std::path::PathBuf;
use std::sync::LazyLock;

static BASE_DIR: LazyLock<Result<PathBuf>> = LazyLock::new(|| {
    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;
    Ok(user_dirs.home_dir().join(".worktools"))
});

/// 获取应用基础目录: `~/.worktools`
///
/// ## 平台差异
/// - Windows: `C:\Users\<用户名>\.worktools`
/// - macOS: `/Users/<用户名>/.worktools`
/// - Linux: `/home/<用户名>/.worktools`
///
/// 使用 `LazyLock` 缓存结果，首次调用时初始化，后续直接返回缓存值。
pub fn worktools_base() -> Result<PathBuf> {
    BASE_DIR
        .as_ref()
        .map(|p| p.clone())
        .map_err(|e| anyhow::anyhow!("{e}"))
}

/// 插件目录: `~/.worktools/plugins`
/// 存放已安装插件的动态库和前端资源
pub fn plugins_dir() -> Result<PathBuf> {
    Ok(worktools_base()?.join("plugins"))
}

/// 配置目录: `~/.worktools/config`
/// 存放插件注册表等应用级配置文件
pub fn config_dir() -> Result<PathBuf> {
    Ok(worktools_base()?.join("config"))
}

/// 日志目录: `~/.worktools/logs`
/// 存放按天滚动的日志文件
pub fn logs_dir() -> Result<PathBuf> {
    Ok(worktools_base()?.join("logs"))
}

/// 历史数据目录: `~/.worktools/history`
/// 存放插件持久化数据（密码、配置等）
pub fn history_dir() -> Result<PathBuf> {
    Ok(worktools_base()?.join("history"))
}
