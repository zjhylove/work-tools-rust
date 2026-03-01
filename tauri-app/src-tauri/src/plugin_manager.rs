use anyhow::{Context, Result};
use libloading::{Library, Symbol};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use worktools_plugin_api::{Plugin, PluginCreateFn};
use worktools_shared_types::PluginInfo;

/// 已加载的插件
pub struct LoadedPlugin {
    pub info: PluginInfo,
    pub instance: Box<dyn Plugin>,
    /// 保存 Library 实例,防止被卸载
    _library: Library,
}

/// 插件管理器
pub struct PluginManager {
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
    plugin_dir: PathBuf,
}

impl PluginManager {
    /// 创建新的插件管理器
    pub fn new() -> Result<Self> {
        let user_dirs = directories::UserDirs::new()
            .ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;
        let plugin_dir = user_dirs.home_dir().join(".worktools/plugins");

        // 创建插件目录
        std::fs::create_dir_all(&plugin_dir)
            .context("创建插件目录失败")?;

        Ok(Self {
            plugins: RwLock::new(HashMap::new()),
            plugin_dir,
        })
    }

    /// 初始化插件管理器,扫描并加载所有插件
    pub async fn init(&self) -> Result<()> {
        tracing::info!("初始化插件管理器,插件目录: {:?}", self.plugin_dir);

        // 扫描插件目录
        let entries = std::fs::read_dir(&self.plugin_dir)
            .context("读取插件目录失败")?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // 查找动态库文件
            if path.is_dir() {
                if let Some(plugin_name) = path.file_name().and_then(|n| n.to_str()) {
                    // 根据平台查找动态库文件
                    let lib_name = if cfg!(target_os = "macos") {
                        format!("lib{}.dylib", plugin_name.replace('-', "_"))
                    } else if cfg!(target_os = "linux") {
                        format!("lib{}.so", plugin_name.replace('-', "_"))
                    } else if cfg!(target_os = "windows") {
                        format!("{}.dll", plugin_name.replace('-', "_"))
                    } else {
                        continue;
                    };

                    let lib_path = path.join(&lib_name);
                    if lib_path.exists() {
                        if let Err(e) = self.load_plugin(&lib_path).await {
                            tracing::warn!("加载插件失败 {:?}: {}", lib_path, e);
                        }
                    }
                }
            }
        }

        tracing::info!(
            "插件管理器初始化完成,成功加载 {} 个插件",
            self.plugins.read().await.len()
        );
        Ok(())
    }

    /// 加载插件动态库
    async fn load_plugin(&self, lib_path: &Path) -> Result<()> {
        tracing::info!("加载插件: {:?}", lib_path);

        unsafe {
            // 加载动态库
            let library = Library::new(lib_path)
                .context("加载动态库失败")?;

            // 获取 plugin_create 函数
            let create: Symbol<PluginCreateFn> = library.get(b"plugin_create")
                .context("未找到 plugin_create 导出函数")?;

            // 调用工厂函数创建插件实例
            let plugin_ptr = create();
            if plugin_ptr.is_null() {
                anyhow::bail!("plugin_create 返回空指针");
            }

            let mut plugin = Box::from_raw(plugin_ptr);

            // 初始化插件
            if let Err(e) = plugin.init() {
                anyhow::bail!("插件初始化失败: {}", e);
            }

            // 构建插件信息
            let info = PluginInfo {
                id: plugin.id().to_string(),
                name: plugin.name().to_string(),
                description: plugin.description().to_string(),
                version: plugin.version().to_string(),
                icon: plugin.icon().to_string(),
            };

            tracing::info!(
                "插件加载成功: {} (v{})",
                info.name,
                info.version
            );

            // 保存到已加载插件列表
            let mut plugins = self.plugins.write().await;
            plugins.insert(
                info.id.clone(),
                LoadedPlugin {
                    info,
                    instance: *plugin,
                    _library: library,
                },
            );
        }

        Ok(())
    }

    /// 获取所有已加载的插件
    pub async fn get_installed_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .read()
            .await
            .values()
            .map(|p| p.info.clone())
            .collect()
    }

    /// 根据 ID 获取插件信息
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<PluginInfo> {
        self.plugins
            .read()
            .await
            .get(plugin_id)
            .map(|p| p.info.clone())
    }

    /// 获取插件视图 HTML
    pub async fn get_plugin_view(&self, plugin_id: &str) -> Result<String> {
        let plugins = self.plugins.read().await;

        let plugin = plugins
            .get(plugin_id)
            .ok_or_else(|| anyhow::anyhow!("插件不存在: {}", plugin_id))?;

        Ok(plugin.instance.get_view())
    }

    /// 调用插件方法
    pub async fn call_plugin_method(
        &self,
        plugin_id: &str,
        method: &str,
        params: Value,
    ) -> Result<Value> {
        let mut plugins = self.plugins.write().await;

        let plugin = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| anyhow::anyhow!("插件不存在: {}", plugin_id))?;

        plugin
            .instance
            .handle_call(method, params)
            .map_err(|e| anyhow::anyhow!("插件方法调用失败: {}", e))
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new().expect("无法创建插件管理器")
    }
}
