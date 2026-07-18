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

    /// 异步版本的 new()，将同步 I/O 移到 spawn_blocking。
    /// 用于从 async 上下文调用时避免阻塞 tokio 运行时。
    pub async fn new_async() -> Result<Self> {
        let path = Self::default_registry_path()?;
        Self::with_path_async(path).await
    }

    /// 异步版本的 with_path()。
    pub async fn with_path_async(registry_file: PathBuf) -> Result<Self> {
        let rf = registry_file.clone();
        tokio::task::spawn_blocking(move || Self::with_path(rf))
            .await
            .map_err(|e| anyhow::anyhow!("注册表加载任务失败: {}", e))?
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
    ///
    /// 同步版本，适用于非 async 上下文（如测试）。
    /// async 上下文请使用 `register_async()`。
    #[allow(dead_code)] // 保留为同步 API（测试和未来同步调用方）
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

    /// 异步版本的 register()，将 sync I/O (save) 移到 spawn_blocking。
    /// 用于从 async 上下文调用时避免阻塞 tokio 运行时。
    pub async fn register_async(&mut self, plugin: InstalledPlugin) -> Result<()> {
        tracing::info!("注册插件: {} ({})", plugin.name, plugin.id);
        self.installed.insert(plugin.id.clone(), plugin);
        let registry_file = self.registry_file.clone();
        let content = serde_json::to_string_pretty(&self.installed).context("序列化注册表失败")?;
        tokio::task::spawn_blocking(move || {
            let tmp_path = registry_file.with_extension("tmp");
            fs::write(&tmp_path, &content).context("写入临时注册表文件失败")?;
            fs::rename(&tmp_path, registry_file).context("重命名注册表文件失败")?;
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("注册表写入任务失败: {}", e))?
    }

    pub async fn unregister_async(&mut self, plugin_id: &str) -> Result<()> {
        tracing::info!("注销插件: {}", plugin_id);
        self.installed.remove(plugin_id);
        let registry_file = self.registry_file.clone();
        let content = serde_json::to_string_pretty(&self.installed).context("序列化注册表失败")?;
        tokio::task::spawn_blocking(move || {
            let tmp_path = registry_file.with_extension("tmp");
            fs::write(&tmp_path, &content).context("写入临时注册表文件失败")?;
            fs::rename(&tmp_path, registry_file).context("重命名注册表文件失败")?;
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("注册表写入任务失败: {}", e))?
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
        // Write to temp file then rename for atomicity (prevents torn writes from concurrent access)
        let tmp_path = self.registry_file.with_extension("tmp");
        fs::write(&tmp_path, &content).context("写入临时注册表文件失败")?;
        fs::rename(&tmp_path, &self.registry_file).context("重命名注册表文件失败")?;
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

    /// Helper to create a test InstalledPlugin with minimal fields.
    fn make_test_plugin(id: &str, temp_dir: &std::path::Path) -> InstalledPlugin {
        InstalledPlugin {
            id: id.to_string(),
            name: format!("Test {}", id),
            description: "Test plugin".to_string(),
            version: "1.0.0".to_string(),
            icon: None,
            author: None,
            homepage: None,
            installed_at: chrono::Utc::now(),
            enabled: true,
            assets_path: temp_dir.join(id).join("assets"),
            library_path: temp_dir.join(id).join("lib.so"),
        }
    }

    #[test]
    fn test_register_persists_to_file() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("registry.json");

        let mut registry = PluginRegistry {
            registry_file: registry_file.clone(),
            installed: HashMap::new(),
        };
        registry.register(make_test_plugin("persist-test", temp_dir.path())).unwrap();

        // File should exist
        assert!(registry_file.exists());

        // Reload from file
        let reloaded = PluginRegistry::with_path(registry_file).unwrap();
        assert!(reloaded.is_installed("persist-test"));
        let p = reloaded.get("persist-test").unwrap();
        assert_eq!(p.name, "Test persist-test");
    }

    #[test]
    fn test_with_path_loads_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("existing.json");

        // Pre-write a valid registry file
        let data = serde_json::json!({
            "alpha": {
                "id": "alpha",
                "name": "Alpha Plugin",
                "description": "desc",
                "version": "2.0.0",
                "installed_at": "2026-01-01T00:00:00Z",
                "enabled": true,
                "assets_path": "/tmp/assets",
                "library_path": "/tmp/lib.so"
            }
        });
        std::fs::write(&registry_file, serde_json::to_string_pretty(&data).unwrap()).unwrap();

        let registry = PluginRegistry::with_path(registry_file).unwrap();
        assert!(registry.is_installed("alpha"));
        assert_eq!(registry.get("alpha").unwrap().version, "2.0.0");
    }

    #[test]
    fn test_with_path_empty_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("nonexistent.json");

        let registry = PluginRegistry::with_path(registry_file).unwrap();
        assert!(!registry.is_installed("anything"));
        assert!(registry.get_installed().is_empty());
    }

    #[test]
    fn test_get_installed_returns_all() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("multi.json");

        let mut registry = PluginRegistry {
            registry_file,
            installed: HashMap::new(),
        };
        registry.register(make_test_plugin("a", temp_dir.path())).unwrap();
        registry.register(make_test_plugin("b", temp_dir.path())).unwrap();
        registry.register(make_test_plugin("c", temp_dir.path())).unwrap();

        let all = registry.get_installed();
        assert_eq!(all.len(), 3);
        let ids: Vec<&str> = all.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
        assert!(ids.contains(&"c"));
    }

    #[test]
    fn test_set_enabled() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("toggle.json");

        let mut registry = PluginRegistry {
            registry_file,
            installed: HashMap::new(),
        };
        registry.register(make_test_plugin("toggle-test", temp_dir.path())).unwrap();
        assert!(registry.is_enabled("toggle-test"));

        registry.set_enabled("toggle-test", false).unwrap();
        assert!(!registry.is_enabled("toggle-test"));

        registry.set_enabled("toggle-test", true).unwrap();
        assert!(registry.is_enabled("toggle-test"));
    }

    #[test]
    fn test_set_enabled_nonexistent_is_noop() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("noop.json");

        let mut registry = PluginRegistry {
            registry_file,
            installed: HashMap::new(),
        };
        // Should not panic or error
        registry.set_enabled("ghost", false).unwrap();
    }

    #[test]
    fn test_verify_installations_removes_missing() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("verify.json");

        let mut registry = PluginRegistry {
            registry_file: registry_file.clone(),
            installed: HashMap::new(),
        };

        // Plugin with existing files
        let good_dir = temp_dir.path().join("good-plugin");
        std::fs::create_dir_all(good_dir.join("assets")).unwrap();
        std::fs::write(good_dir.join("lib.so"), "fake").unwrap();
        let good_plugin = InstalledPlugin {
            id: "good".to_string(),
            name: "Good".to_string(),
            description: "Good plugin".to_string(),
            version: "1.0.0".to_string(),
            icon: None,
            author: None,
            homepage: None,
            installed_at: chrono::Utc::now(),
            enabled: true,
            assets_path: good_dir.join("assets"),
            library_path: good_dir.join("lib.so"),
        };
        registry.register(good_plugin).unwrap();

        // Plugin with missing files
        registry.register(make_test_plugin("bad", temp_dir.path())).unwrap();

        assert_eq!(registry.installed.len(), 2);
        registry.verify_installations().unwrap();

        // Only "good" should remain
        assert_eq!(registry.installed.len(), 1);
        assert!(registry.is_installed("good"));
        assert!(!registry.is_installed("bad"));
    }

    #[test]
    fn test_get_returns_none_for_missing() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("empty.json");

        let registry = PluginRegistry {
            registry_file,
            installed: HashMap::new(),
        };
        assert!(registry.get("nonexistent").is_none());
        assert!(!registry.is_installed("nonexistent"));
        assert!(!registry.is_enabled("nonexistent"));
    }

    #[test]
    fn test_unregister_nonexistent_is_noop() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("noop-unreg.json");

        let mut registry = PluginRegistry {
            registry_file,
            installed: HashMap::new(),
        };
        // Should not error on unregistering nonexistent
        registry.unregister("ghost").unwrap();
    }

    #[tokio::test]
    async fn test_with_path_async_loads_file() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("async-test.json");

        let data = serde_json::json!({
            "beta": {
                "id": "beta",
                "name": "Beta Plugin",
                "description": "desc",
                "version": "3.0.0",
                "installed_at": "2026-06-01T00:00:00Z",
                "enabled": false,
                "assets_path": "/tmp/assets",
                "library_path": "/tmp/lib.so"
            }
        });
        std::fs::write(&registry_file, serde_json::to_string(&data).unwrap()).unwrap();

        let registry = PluginRegistry::with_path_async(registry_file).await.unwrap();
        assert!(registry.is_installed("beta"));
        assert_eq!(registry.get("beta").unwrap().version, "3.0.0");
        assert!(!registry.is_enabled("beta"));
    }

    #[tokio::test]
    async fn test_with_path_async_empty_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("nonexistent-async.json");

        let registry = PluginRegistry::with_path_async(registry_file).await.unwrap();
        assert!(registry.get_installed().is_empty());
    }

    #[tokio::test]
    async fn test_register_async_persists() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("reg-async.json");

        let mut registry = PluginRegistry {
            registry_file: registry_file.clone(),
            installed: HashMap::new(),
        };

        let plugin = make_test_plugin("async-reg", temp_dir.path());
        registry.register_async(plugin).await.unwrap();

        // Verify persistence: reload from file
        let reloaded = PluginRegistry::with_path(registry_file).unwrap();
        assert!(reloaded.is_installed("async-reg"));
        assert_eq!(reloaded.get("async-reg").unwrap().name, "Test async-reg");
    }

    #[tokio::test]
    async fn test_unregister_async_persists() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("unreg-async.json");

        let mut registry = PluginRegistry {
            registry_file: registry_file.clone(),
            installed: HashMap::new(),
        };

        registry.register(make_test_plugin("to-remove", temp_dir.path())).unwrap();
        registry.unregister_async("to-remove").await.unwrap();

        // Verify persistence: reload from file
        let reloaded = PluginRegistry::with_path(registry_file).unwrap();
        assert!(!reloaded.is_installed("to-remove"));
    }
}
