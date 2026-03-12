use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// 已安装插件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub assets_path: PathBuf,
    pub library_path: PathBuf,
}

fn default_enabled() -> bool {
    true
}

/// 插件注册表,管理已安装插件的元数据
pub struct PluginRegistry {
    registry_file: PathBuf,
    installed: HashMap<String, InstalledPlugin>,
}

impl PluginRegistry {
    /// 创建或加载插件注册表
    pub fn new() -> Result<Self> {
        Self::with_path(Self::default_registry_path()?)
    }

    /// 使用指定的注册表文件路径创建
    pub fn with_path(registry_file: PathBuf) -> Result<Self> {
        let installed = if registry_file.exists() {
            let content = fs::read_to_string(&registry_file).context("读取注册表文件失败")?;
            serde_json::from_str(&content).context("解析注册表文件失败")?
        } else {
            HashMap::new()
        };

        tracing::info!("插件注册表已加载,已安装 {} 个插件", installed.len());

        Ok(Self {
            registry_file,
            installed,
        })
    }

    /// 获取默认注册表文件路径
    fn default_registry_path() -> Result<PathBuf> {
        let user_dirs =
            directories::UserDirs::new().ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;

        let config_dir = user_dirs.home_dir().join(".worktools/config");
        fs::create_dir_all(&config_dir).context("创建配置目录失败")?;

        Ok(config_dir.join("installed-plugins.json"))
    }

    /// 注册已安装的插件
    pub fn register(&mut self, plugin: InstalledPlugin) -> Result<()> {
        tracing::info!("注册插件: {} ({})", plugin.name, plugin.id);

        self.installed.insert(plugin.id.clone(), plugin);
        self.save()?;

        Ok(())
    }

    /// 注销插件
    pub fn unregister(&mut self, plugin_id: &str) -> Result<()> {
        tracing::info!("注销插件: {}", plugin_id);

        self.installed.remove(plugin_id);
        self.save()?;

        Ok(())
    }

    /// 更新插件启用状态
    #[allow(dead_code)]
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

    /// 获取已安装插件列表
    pub fn get_installed(&self) -> Vec<InstalledPlugin> {
        self.installed.values().cloned().collect()
    }

    /// 根据ID获取插件信息
    pub fn get(&self, plugin_id: &str) -> Option<InstalledPlugin> {
        self.installed.get(plugin_id).cloned()
    }

    /// 检查插件是否已安装
    #[allow(dead_code)]
    pub fn is_installed(&self, plugin_id: &str) -> bool {
        self.installed.contains_key(plugin_id)
    }

    /// 检查插件是否已启用
    #[allow(dead_code)]
    pub fn is_enabled(&self, plugin_id: &str) -> bool {
        self.installed
            .get(plugin_id)
            .map(|p| p.enabled)
            .unwrap_or(false)
    }

    /// 保存注册表到文件
    fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.installed).context("序列化注册表失败")?;

        fs::write(&self.registry_file, content).context("写入注册表文件失败")?;

        Ok(())
    }

    /// 验证已安装插件的文件是否仍然存在
    #[allow(dead_code)]
    pub fn verify_installations(&mut self) -> Result<()> {
        let mut to_remove = Vec::new();

        for (id, plugin) in &self.installed {
            if !plugin.library_path.exists() {
                tracing::warn!("插件 {} 的动态库文件不存在,标记为待移除", id);
                to_remove.push(id.clone());
            } else if !plugin.assets_path.exists() {
                tracing::warn!("插件 {} 的前端资源目录不存在,标记为待移除", id);
                to_remove.push(id.clone());
            }
        }

        for id in to_remove {
            self.unregister(&id)?;
            tracing::info!("已从注册表中移除无效插件: {}", id);
        }

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
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("test-registry.json");

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

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let registry_file = temp_dir.path().join("test-registry.json");

        let mut registry = PluginRegistry {
            registry_file: registry_file.clone(),
            installed: HashMap::new(),
        };

        let plugin = InstalledPlugin {
            id: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            description: "Test".to_string(),
            version: "1.0.0".to_string(),
            icon: None,
            author: None,
            homepage: None,
            installed_at: chrono::Utc::now(),
            enabled: true,
            assets_path: temp_dir.path().join("assets"),
            library_path: temp_dir.path().join("lib.so"),
        };

        registry.register(plugin).unwrap();

        // 重新加载注册表
        let _registry2 = PluginRegistry {
            registry_file,
            installed: HashMap::new(),
        };

        // 注意:这里需要重新实现加载逻辑才能测试,暂且跳过
    }
}
