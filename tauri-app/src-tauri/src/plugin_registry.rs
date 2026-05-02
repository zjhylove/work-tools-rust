//! # 插件注册表
//!
//! 管理已安装插件的持久化元数据。
//! 注册表是一个 JSON 文件，记录了每个已安装插件的：
//! - 基本信息（ID、名称、版本等）
//! - 安装时间和启用状态
//! - 动态库文件路径和前端资源路径
//!
//! ## 与 PluginManager 的区别
//! - **PluginRegistry**: 持久化的元数据（文件），记录"哪些插件已安装"
//! - **PluginManager**: 运行时的实例管理（内存），管理"哪些插件已加载"
//!
//! ## Rust 知识点
//! - `serde::Serialize/Deserialize`: 自动序列化，让结构体可以保存为 JSON
//! - `HashMap`: 键值对集合，O(1) 查找
//! - `#[serde(default)]`: 字段缺失时使用类型的 Default 值
//! - `#[serde(default = "fn")]`: 使用指定函数提供默认值

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// 已安装插件信息（持久化到 JSON 文件的结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    /// `#[serde(default)]`: 如果 JSON 中没有这个字段，使用 Option 的默认值 None
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    /// 安装时间，使用 UTC 时间避免时区问题
    pub installed_at: chrono::DateTime<chrono::Utc>,
    /// `#[serde(default = "default_enabled")]` 自定义默认值函数
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 前端资源目录的绝对路径
    pub assets_path: PathBuf,
    /// 动态库文件的绝对路径
    pub library_path: PathBuf,
}

/// 为 `enabled` 字段提供默认值
fn default_enabled() -> bool {
    true // 新安装的插件默认启用
}

/// 插件注册表 — 管理已安装插件的元数据
///
/// 底层是一个 JSON 文件 (`~/.worktools/config/installed-plugins.json`)，
/// 在内存中维护为 HashMap，每次修改后自动写入文件。
pub struct PluginRegistry {
    /// 注册表文件的路径
    registry_file: PathBuf,
    /// 已安装插件的内存映射
    installed: HashMap<String, InstalledPlugin>,
}

impl PluginRegistry {
    /// 创建或加载插件注册表（使用默认路径）
    pub fn new() -> Result<Self> {
        Self::with_path(Self::default_registry_path()?)
    }

    /// 使用指定路径创建注册表
    ///
    /// 如果文件已存在，从中加载；否则初始化为空的 HashMap。
    pub fn with_path(registry_file: PathBuf) -> Result<Self> {
        let installed = if registry_file.exists() {
            let content = fs::read_to_string(&registry_file).context("读取注册表文件失败")?;
            serde_json::from_str(&content).context("解析注册表文件失败")?
        } else {
            HashMap::new()
        };

        Ok(Self {
            registry_file,
            installed,
        })
    }

    /// 获取默认注册表文件路径: `~/.worktools/config/installed-plugins.json`
    fn default_registry_path() -> Result<PathBuf> {
        let config_dir = crate::paths::config_dir()?;
        fs::create_dir_all(&config_dir).context("创建配置目录失败")?;
        Ok(config_dir.join("installed-plugins.json"))
    }

    // ── 核心操作 ──

    /// 注册已安装的插件（添加或更新）
    /// `HashMap::insert` 会覆盖已存在的同 ID 条目
    pub fn register(&mut self, plugin: InstalledPlugin) -> Result<()> {
        tracing::info!("注册插件: {} ({})", plugin.name, plugin.id);

        self.installed.insert(plugin.id.clone(), plugin);
        self.save()?; // 立即持久化

        Ok(())
    }

    /// 注销（移除）插件
    /// `HashMap::remove` 返回被移除的值，但我们不需要它
    pub fn unregister(&mut self, plugin_id: &str) -> Result<()> {
        tracing::info!("注销插件: {}", plugin_id);

        self.installed.remove(plugin_id);
        self.save()?;

        Ok(())
    }

    // ── 查询操作 ──

    /// 获取所有已安装插件列表
    /// `cloned()` 从 `&InstalledPlugin` 创建 `InstalledPlugin` 的副本
    pub fn get_installed(&self) -> Vec<InstalledPlugin> {
        self.installed.values().cloned().collect()
    }

    /// 根据 ID 获取插件信息
    /// 返回 `Option` — 插件可能不存在
    pub fn get(&self, plugin_id: &str) -> Option<InstalledPlugin> {
        self.installed.get(plugin_id).cloned()
    }

    /// 检查插件是否已安装
    /// `contains_key` 比 `get().is_some()` 更高效（不需要克隆值）
    #[allow(dead_code)] // 保留为公共 API，前端可能未来使用
    pub fn is_installed(&self, plugin_id: &str) -> bool {
        self.installed.contains_key(plugin_id)
    }

    /// 检查插件是否已启用
    #[allow(dead_code)] // 保留为公共 API，前端可能未来使用
    pub fn is_enabled(&self, plugin_id: &str) -> bool {
        self.installed
            .get(plugin_id)
            .map(|p| p.enabled)
            .unwrap_or(false) // 不存在视为未启用
    }

    // ── 维护操作 ──

    /// 更新插件启用状态
    #[allow(dead_code)] // 保留接口，前端可能未来使用
    pub fn set_enabled(&mut self, plugin_id: &str, enabled: bool) -> Result<()> {
        if let Some(plugin) = self.installed.get_mut(plugin_id) {
            plugin.enabled = enabled;
            self.save()?;
            tracing::info!(
                "插件 {} 状态已设置为: {}",
                plugin_id,
                if enabled { "启用" } else { "禁用" }
            );
        } else {
            tracing::warn!("插件 {} 不存在于注册表中", plugin_id);
        }
        Ok(())
    }

    /// 验证已安装插件的文件是否仍然存在
    /// 用于清理"脏"状态：文件被手动删除但注册表还残留
    #[allow(dead_code)]
    pub fn verify_installations(&mut self) -> Result<()> {
        let mut to_remove = Vec::new();

        for (id, plugin) in &self.installed {
            // 检查动态库是否存在
            if !plugin.library_path.exists() {
                tracing::warn!("插件 {} 的动态库文件不存在，标记为待移除", id);
                to_remove.push(id.clone());
            // 检查前端资源目录是否存在
            } else if !plugin.assets_path.exists() {
                tracing::warn!("插件 {} 的前端资源目录不存在，标记为待移除", id);
                to_remove.push(id.clone());
            }
        }

        for id in to_remove {
            self.unregister(&id)?;
            tracing::info!("已从注册表中移除无效插件: {}", id);
        }

        Ok(())
    }

    // ── 私有辅助方法 ──

    /// 保存注册表到文件
    /// `to_string_pretty` 输出格式化的 JSON（带缩进），方便人工查看和调试
    fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.installed).context("序列化注册表失败")?;
        fs::write(&self.registry_file, content).context("写入注册表文件失败")?;
        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new().expect("无法创建插件注册表")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_register_and_unregister() {
        // 使用临时目录，测试结束自动清理
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("test-registry.json");

        // 手动构造而不是用 new()，因为我们要控制文件路径
        let mut registry = PluginRegistry {
            registry_file: registry_file.clone(),
            installed: HashMap::new(),
        };

        let plugin = InstalledPlugin {
            id: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            description: "Test".to_string(),
            version: "1.0.0".to_string(),
            icon: Some("🔧".to_string()),
            author: None,
            homepage: None,
            installed_at: chrono::Utc::now(),
            enabled: true,
            assets_path: temp_dir.path().join("assets"),
            library_path: temp_dir.path().join("lib.so"),
        };

        // 测试注册
        registry.register(plugin.clone()).unwrap();
        assert!(registry.is_installed("test-plugin"));

        // 测试获取
        let retrieved = registry.get("test-plugin").unwrap();
        assert_eq!(retrieved.id, "test-plugin");

        // 测试注销
        registry.unregister("test-plugin").unwrap();
        assert!(!registry.is_installed("test-plugin"));
    }
}
