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
    /// 获取当前平台的动态库文件扩展名
    fn get_platform_extension() -> &'static str {
        if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "linux") {
            "so"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "unknown"
        }
    }

    /// 获取当前平台的动态库前缀
    fn get_platform_prefix() -> &'static str {
        if cfg!(target_os = "windows") {
            ""
        } else {
            "lib"
        }
    }

    /// 从 manifest 读取当前平台的动态库文件名
    fn get_library_from_manifest(manifest: &serde_json::Value) -> Option<String> {
        let platform = if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            return None;
        };

        manifest
            .get("files")
            .and_then(|f| f.get(platform))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// 创建新的插件管理器
    pub fn new() -> Result<Self> {
        let user_dirs = directories::UserDirs::new()
            .ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;

        let plugin_dir = user_dirs.home_dir().join(".worktools/plugins");

        // 创建插件目录
        std::fs::create_dir_all(&plugin_dir)
            .inspect_err(|e| {
                tracing::error!(
                    plugin_dir = %plugin_dir.display(),
                    "创建插件目录失败: {}",
                    e
                );
            })
            .context("创建插件目录失败")?;

        Ok(Self {
            plugins: RwLock::new(HashMap::new()),
            plugin_dir,
        })
    }

    /// 初始化插件管理器,扫描并加载所有插件
    pub async fn init(&self) -> Result<()> {
        tracing::info!("初始化插件管理器,插件目录: {:?}", self.plugin_dir);

        // 清空已加载的插件列表
        self.plugins.write().await.clear();

        // 扫描插件目录
        let entries = std::fs::read_dir(&self.plugin_dir)
            .inspect_err(|e| {
                tracing::error!(
                    plugin_dir = %self.plugin_dir.display(),
                    "读取插件目录失败: {}",
                    e
                );
            })
            .context("读取插件目录失败")?;

        for entry in entries {
            let entry = entry
                .inspect_err(|e| {
                    tracing::error!("读取目录项失败: {}", e);
                })
                .context("读取目录项失败")?;

            let path = entry.path();

            // 查找动态库文件
            if path.is_dir() {
                // 尝试从 manifest.json 读取动态库文件名
                let manifest_path = path.join("manifest.json");
                let lib_path = if manifest_path.exists() {
                    // 读取 manifest.json 获取动态库文件名
                    std::fs::read_to_string(&manifest_path)
                        .ok()
                        .and_then(|content| {
                            serde_json::from_str::<serde_json::Value>(&content).ok()
                        })
                        .and_then(|manifest| Self::get_library_from_manifest(&manifest))
                        .map(|name| {
                            tracing::info!("从 manifest.json 读取动态库文件名: {}", name);
                            path.join(name)
                        })
                } else {
                    // 回退到旧的命名规则
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|plugin_name| {
                            let lib_name = format!(
                                "{}{}.{}",
                                Self::get_platform_prefix(),
                                plugin_name.replace('-', "_"),
                                Self::get_platform_extension()
                            );
                            path.join(lib_name)
                        })
                };

                let lib_path = match lib_path {
                    Some(path) => path,
                    None => continue,
                };

                if lib_path.exists() {
                    if let Err(e) = self.load_plugin(&lib_path).await {
                        tracing::warn!("加载插件失败 {:?}: {}", lib_path, e);
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
                .inspect_err(|e| {
                    tracing::error!(
                        lib_path = %lib_path.display(),
                        "加载动态库失败: {}",
                        e
                    );
                })
                .context("加载动态库失败")?;

            // 获取 plugin_create 函数
            let create: Symbol<PluginCreateFn> = library
                .get(b"plugin_create")
                .inspect_err(|e| {
                    tracing::error!(
                        lib_path = %lib_path.display(),
                        "未找到 plugin_create 导出函数: {}",
                        e
                    );
                })
                .context("未找到 plugin_create 导出函数")?;

            // 调用工厂函数创建插件实例
            let plugin_ptr = create();
            if plugin_ptr.is_null() {
                tracing::error!(
                    lib_path = %lib_path.display(),
                    "plugin_create 返回空指针"
                );
                anyhow::bail!("plugin_create 返回空指针");
            }

            let mut plugin = Box::from_raw(plugin_ptr);

            // 初始化插件
            if let Err(e) = plugin.init() {
                tracing::error!(
                    lib_path = %lib_path.display(),
                    "插件初始化失败: {}",
                    e
                );
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

            tracing::info!("插件加载成功: {} (v{})", info.name, info.version);

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
            .ok_or_else(|| {
                tracing::error!(plugin_id = %plugin_id, "插件不存在");
                anyhow::anyhow!("插件不存在: {}", plugin_id)
            })?;

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
            .ok_or_else(|| {
                tracing::error!(
                    plugin_id = %plugin_id,
                    method = %method,
                    "插件不存在"
                );
                anyhow::anyhow!("插件不存在: {}", plugin_id)
            })?;

        plugin
            .instance
            .handle_call(method, params)
            .inspect_err(|e| {
                tracing::error!(
                    plugin_id = %plugin_id,
                    method = %method,
                    "插件方法调用失败: {}",
                    e
                );
            })
            .map_err(|e| anyhow::anyhow!("插件方法调用失败: {}", e))
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new().expect("无法创建插件管理器")
    }
}
