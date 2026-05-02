//! # 插件数据存储
//!
//! 提供基于 JSON 文件的持久化存储，供所有插件使用。
//!
//! ## 核心设计
//! - 数据存储在 `~/.worktools/history/plugins/` 目录下
//! - 使用原子写入（先写临时文件，再 rename）防止文件损坏
//! - 支持保留字段合并（preserve fields）
//!
//! ## Rust 知识点
//! - `anyhow::Result`: 简化的错误处理，适合应用层代码
//! - `serde::Serialize/Deserialize`: 自动序列化/反序列化
//! - `where` 子句: 对泛型参数添加约束
//! - 原子文件操作: 通过写临时文件 + rename 实现

use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

/// 插件数据存储辅助工具
///
/// 每个插件通过 `PluginStorage::new("plugin-id", "data.json")` 创建自己的存储实例。
/// 不同插件使用不同的文件名，数据完全隔离。
pub struct PluginStorage {
    /// 插件 ID，用于日志标识
    plugin_id: String,
    /// 数据文件名，例如 "password-manager.json"
    data_filename: String,
}

impl PluginStorage {
    /// 创建新的插件存储实例
    ///
    /// ## 参数
    /// - `plugin_id`: 插件唯一标识符
    /// - `data_filename`: 数据文件名，建议使用 `<plugin-id>.json` 格式
    pub fn new(plugin_id: &str, data_filename: &str) -> Self {
        Self {
            // `to_string()` 将借用的 &str 转换为拥有的 String
            // 这样存储实例拥有自己的数据副本，不受参数生命周期限制
            plugin_id: plugin_id.to_string(),
            data_filename: data_filename.to_string(),
        }
    }

    /// 获取数据文件路径（使用 ~/.worktools/history/plugins/）
    ///
    /// ## Rust 知识点: Result<PathBuf>
    /// 返回 `Result` 因为可能找不到用户主目录（极少数情况）。
    /// `PathBuf` 是 `String` 的路径版：可变的、拥有的路径。
    pub fn get_data_path(&self) -> Result<PathBuf> {
        // `directories::UserDirs::new()` 是跨平台获取用户目录的方式
        // Windows: C:\Users\<用户名>\
        // macOS: /Users/<用户名>/
        // Linux: /home/<用户名>/
        let user_dirs =
            directories::UserDirs::new().ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;
        // `?` 操作符：如果返回 None，立即将错误向上传播

        let mut data_dir = user_dirs.home_dir().join(".worktools/history/plugins");

        // `create_dir_all` 类似 `mkdir -p`，递归创建所有需要的目录
        std::fs::create_dir_all(&data_dir)
            // `.context()` 为错误添加额外上下文信息，帮助定位问题
            .context("创建数据目录失败")?;

        // `push` 将文件名追加到路径末尾，例如变成 `/home/user/.worktools/history/plugins/data.json`
        data_dir.push(&self.data_filename);
        Ok(data_dir)
    }

    /// 获取替代数据文件路径（使用系统数据目录）
    /// 当主路径不可用时作为 fallback
    ///
    /// Windows: C:\Users\<用户>\AppData\Local\worktools\data\
    /// macOS: /Users/<用户>/Library/Application Support/worktools/data/
    /// Linux: /home/<用户>/.local/share/worktools/data/
    pub fn get_alternative_data_path(&self) -> Result<PathBuf> {
        let mut data_dir =
            dirs::data_local_dir().ok_or_else(|| anyhow::anyhow!("无法获取数据目录"))?;
        data_dir.push("worktools");
        data_dir.push("data");

        std::fs::create_dir_all(&data_dir)?;

        data_dir.push(&self.data_filename);
        Ok(data_dir)
    }

    /// 加载 JSON 数据
    ///
    /// ## Rust 知识点: 泛型约束
    /// ```ignore
    /// T: for<'de> serde::Deserialize<'de> + Default
    /// ```
    /// - `T: Deserialize<'de>`: T 必须可以被反序列化
    /// - `for<'de>`: 高阶生命周期（HRTB），表示"对任意生命周期都满足"
    /// - `+ Default`: T 必须有默认值（当文件不存在时返回默认值）
    pub fn load_json<T>(&self) -> Result<T>
    where
        T: for<'de> serde::Deserialize<'de> + Default,
    {
        let data_path = self.get_data_path()?;

        // 如果文件不存在，返回 T 的默认值（例如空 Vec、空字符串）
        if !data_path.exists() {
            return Ok(T::default());
        }

        let file = File::open(&data_path).context("打开数据文件失败")?;
        // `serde_json::from_reader` 直接从文件句柄解析 JSON，不需要先读到字符串
        let data: T = serde_json::from_reader(file).context("解析数据文件失败")?;
        Ok(data)
    }

    /// 保存 JSON 数据（使用原子写入）
    ///
    /// ## 为什么要原子写入？
    /// 如果直接写文件，在写入过程中程序崩溃会导致文件损坏。
    /// 原子写入的流程：
    /// 1. 将数据写入 `.tmp` 临时文件
    /// 2. 调用 `sync_all()` 确保数据已刷到磁盘
    /// 3. 用 `rename()` 原子性地替换原文件
    ///
    /// ## Rust 知识点: 文件选项
    /// `OpenOptions::new()` 是 Rust 精细控制文件打开行为的方式：
    /// - `.write(true)`: 以写模式打开
    /// - `.create(true)`: 如果文件不存在则创建
    /// - `.truncate(true)`: 如果文件已存在，清空其内容
    pub fn save_json<T>(&self, data: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        let data_path = self.get_data_path()?;

        // `with_extension("tmp")` 将 `.json` 替换为 `.tmp`
        let temp_path = data_path.with_extension("tmp");
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .context("创建临时文件失败")?;

        // `to_writer_pretty` 格式化输出 JSON（带缩进），方便人工查看
        serde_json::to_writer_pretty(&file, data).context("序列化数据失败")?;
        // `sync_all()` 确保操作系统将缓冲区数据写入磁盘
        file.sync_all().context("同步文件失败")?;

        // `rename` 是 POSIX 保证的原子操作：要么是新文件，要么是旧文件，不存在中间状态
        std::fs::rename(&temp_path, &data_path).context("替换数据文件失败")?;

        // `tracing::debug!` 是结构化日志，`plugin_id` 以字段形式记录
        tracing::debug!("插件 {} 数据已保存到: {:?}", self.plugin_id, data_path);
        Ok(())
    }

    /// 保存 JSON 数据并保留指定字段
    ///
    /// 这个方法用于某些场景下需要保护已有字段不被覆盖的情况。
    /// 例如：加密的 salt 和 validation_token 应该在保存其他数据时保持不变。
    ///
    /// ## 实现原理
    /// 1. 读取现有的 JSON 文件（如果存在）
    /// 2. 将新数据转为 serde_json::Value
    /// 3. 用现有数据覆盖输出中 `preserve_fields` 指定的字段
    /// 4. 写入合并后的结果
    ///
    /// ## Rust 知识点: 闭包链式调用
    /// `.and_then(|f| ...)` 只在 `Some` 时执行，`None` 直接穿透。
    /// 这是 Option 类型的一种常见操作模式。
    pub fn save_json_preserving<T>(&self, data: &T, preserve_fields: &[&str]) -> Result<()>
    where
        T: serde::Serialize,
    {
        let data_path = self.get_data_path()?;

        // 读取现有配置以保留指定字段
        let existing_config = if data_path.exists() {
            File::open(&data_path)
                .ok()
                // `serde_json::Value` 是通用的 JSON 值类型，可以是对象、数组、字符串等
                .and_then(|f| serde_json::from_reader::<_, serde_json::Value>(f).ok())
        } else {
            None
        };

        // 使用临时文件模式确保原子性写入
        let temp_path = data_path.with_extension("tmp");
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .context("创建临时文件失败")?;

        // `serde_json::to_value` 将实现了 Serialize 的类型转为通用的 Value 类型
        let mut output = serde_json::to_value(data)?;
        if let Some(config) = existing_config {
            for field in preserve_fields {
                // `get(field)` 获取 JSON 对象中的字段
                // `clone()` 深拷贝值（因为 Value 实现了 Clone）
                if let Some(value) = config.get(field) {
                    output[field] = value.clone();
                }
            }
        }

        serde_json::to_writer_pretty(&file, &output).context("序列化数据失败")?;
        file.sync_all().context("同步文件失败")?;

        std::fs::rename(&temp_path, &data_path).context("替换数据文件失败")?;

        tracing::debug!("插件 {} 数据已保存到: {:?}", self.plugin_id, data_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    /// 测试用的数据结构
    /// `derive(Default)` 自动生成 Default 实现
    #[derive(Debug, Serialize, Deserialize, Default)]
    struct TestData {
        // `Vec<String>` 是动态数组，可以增删元素
        entries: Vec<String>,
    }

    #[test]
    fn test_storage() {
        let storage = PluginStorage::new("test-plugin", "test.json");

        // 保存数据
        let data = TestData {
            // `vec!` 宏创建 Vec
            entries: vec!["hello".to_string(), "world".to_string()],
        };
        // `unwrap()` 在测试中可以直接使用：如果失败，测试会 panic
        storage.save_json(&data).unwrap();

        // 加载数据
        let loaded: TestData = storage.load_json().unwrap();
        assert_eq!(loaded.entries.len(), 2);

        // 清理测试产生的文件
        let path = storage.get_data_path().unwrap();
        std::fs::remove_file(path).ok();
    }
}
