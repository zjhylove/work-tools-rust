//! # 插件管理器
//!
//! 这是插件系统最核心的模块，负责：
//! 1. 扫描插件目录，发现已安装的插件
//! 2. 动态加载插件库（.dll / .so / .dylib）
//! 3. 调用插件的生命周期方法（init / destroy）
//! 4. 将前端的方法调用路由到正确的插件
//!
//! ## 核心技术: 动态库加载 (libloading)
//!
//! Rust 通过 `libloading` crate 在运行时加载动态库，类似于：
//! - Windows: `LoadLibrary` + `GetProcAddress`
//! - Linux: `dlopen` + `dlsym`
//! - macOS: `dlopen` + `dlsym`
//!
//! ## 为什么用动态库而不是静态链接？
//! 1. **热插拔**: 可以在不重启应用的情况下安装/卸载插件
//! 2. **独立编译**: 插件可以独立编译，不需要重新编译主程序
//! 3. **隔离**: 插件崩溃不会影响主程序（理论上是同进程，但 Rust 的安全性有帮助）
//!
//! ## Rust 知识点
//! - `unsafe`: Rust 的"信任我"关键字。动态库操作本质上是 unsafe 的
//! - `libloading::Library`: 动态库的句柄，drop 时自动卸载
//! - `libloading::Symbol`: 动态库中导出的函数/变量的引用
//! - `tokio::sync::RwLock`: 异步读写锁，允许多个读或一个写
//! - `Box::from_raw`: 从原始指针重建 Box，恢复 Rust 的所有权语义
//! - `cfg!`: 编译时条件判断宏

use anyhow::{Context, Result};
use libloading::{Library, Symbol};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use worktools_plugin_api::{Plugin, PluginCreateFn};
use worktools_shared_types::PluginInfo;

use crate::plugin_package::PluginManifest;

/// 已加载的插件
///
/// ## Rust 知识点: 结构体字段
/// - `_library`: 前导下划线表示"这个字段不会被直接读取"
///   但它的存在很重要 — 当 `LoadedPlugin` 被 drop 时，`_library` 也会被 drop，
///   从而自动卸载动态库（RAII 模式）
/// - `instance`: `Box<dyn Plugin>` 是 trait 对象，存储实际的插件实例
pub struct LoadedPlugin {
    pub info: PluginInfo,
    pub instance: Box<dyn Plugin>,
    /// 保存 Library 实例，防止被释放
    /// 只要这个字段存在，动态库就会保持在内存中
    /// 当 LoadedPlugin 被从 HashMap 中移除时，Library 自动 drop，触发动态库卸载
    _library: Library,
}

/// 插件管理器
///
/// ## Rust 知识点: RwLock 的选择
/// `RwLock<HashMap<...>>` 而非 `Mutex<HashMap<...>>`:
/// - `RwLock`: 读操作（如列出插件）可以并发进行
/// - `Mutex`: 所有操作（包括读）都是互斥的
/// - 选择 RwLock 因为"列出插件"比"修改插件列表"频繁得多
pub struct PluginManager {
    /// 已加载的插件映射表：plugin_id → LoadedPlugin
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
    /// 插件目录路径
    plugin_dir: PathBuf,
}

impl PluginManager {
    // ── 平台适配 ──

    /// 获取当前平台的动态库文件扩展名
    ///
    /// ## Rust 知识点: cfg! 宏
    /// `cfg!(target_os = "macos")` 在编译时求值。
    /// 这是条件编译的运行时版本，生成的条件分支在编译后会被优化掉（dead code elimination）。
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
    /// - Linux/macOS: `lib` (例如 `libpassword_manager.so`)
    /// - Windows: 无前缀 (例如 `password_manager.dll`)
    fn get_platform_prefix() -> &'static str {
        if cfg!(target_os = "windows") {
            ""
        } else {
            "lib"
        }
    }

    /// 从 manifest 读取当前平台的动态库文件名
    fn get_library_from_manifest(manifest: &PluginManifest) -> Option<String> {
        manifest.get_library_filename().cloned()
    }

    // ── 构造与初始化 ──

    /// 创建新的插件管理器
    ///
    /// ## Rust 知识点: Result 和错误传播
    /// `crate::paths::plugins_dir()?` 中的 `?` 表示：
    /// 如果函数返回 Err，立即将错误从 `new()` 传播出去。
    pub fn new() -> Result<Self> {
        let plugin_dir = crate::paths::plugins_dir()?;

        // 确保插件目录存在
        std::fs::create_dir_all(&plugin_dir)
            .context("创建插件目录失败")?;

        Ok(Self {
            plugins: RwLock::new(HashMap::new()),
            plugin_dir,
        })
    }

    /// 初始化插件管理器：扫描并加载所有插件
    ///
    /// 流程：
    /// 1. 清空已加载的插件列表
    /// 2. 扫描插件目录下的每个子目录
    /// 3. 找到动态库文件并尝试加载
    /// 4. 加载失败的插件只记录警告，不影响其他插件
    pub async fn init(&self) -> Result<()> {
        tracing::info!("初始化插件管理器，插件目录: {:?}", self.plugin_dir);

        // 清空已加载的插件列表
        // `.write().await` 获取写锁，在异步上下文中等待
        self.plugins.write().await.clear();

        // 扫描插件目录
        let entries = std::fs::read_dir(&self.plugin_dir)
            .context("读取插件目录失败")?;

        for entry in entries {
            let entry = entry.context("读取目录项失败")?;
            let path = entry.path();

            // 只处理子目录
            if path.is_dir() {
                // 优先从 manifest.json 读取动态库文件名
                let manifest_path = path.join("manifest.json");
                let lib_path = if manifest_path.exists() {
                    // 新版方式：从 manifest.json 获取动态库名
                    std::fs::read_to_string(&manifest_path)
                        .ok()
                        .and_then(|content| {
                            serde_json::from_str::<PluginManifest>(&content).ok()
                        })
                        .and_then(|manifest| Self::get_library_from_manifest(&manifest))
                        .map(|name| path.join(name))
                } else {
                    // 旧版方式：根据目录名推测动态库名
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|plugin_name| {
                            let lib_name = format!(
                                "{}{}.{}",
                                Self::get_platform_prefix(),
                                plugin_name.replace('-', "_"), // 连字符转下划线
                                Self::get_platform_extension()
                            );
                            path.join(lib_name)
                        })
                };

                let lib_path = match lib_path {
                    Some(path) => path,
                    None => continue, // 跳过无法确定路径的
                };

                if lib_path.exists() {
                    // 加载失败只记录警告，不中断整个初始化
                    if let Err(e) = self.load_plugin(&lib_path).await {
                        tracing::warn!("加载插件失败 {:?}: {}", lib_path, e);
                    }
                }
            }
        }

        tracing::info!(
            "插件管理器初始化完成，成功加载 {} 个插件",
            self.plugins.read().await.len()
        );
        Ok(())
    }

    /// 加载单个插件动态库
    ///
    /// ## Rust 知识点: unsafe 块
    /// 整个函数体都在 `unsafe { ... }` 中，因为：
    /// 1. `Library::new()` — 加载任意动态库，可能有恶意代码
    /// 2. `library.get()` — 查找符号，类型安全性由程序员保证
    /// 3. `create()` — 调用 FFI 函数，可能违反 Rust 的安全保证
    /// 4. `Box::from_raw()` — 从原始指针重建 Box
    ///
    /// unsafe 不代表不安全，而是说"编译器无法验证，由程序员负责"。
    async fn load_plugin(&self, lib_path: &Path) -> Result<()> {
        tracing::info!("加载插件: {:?}", lib_path);

        unsafe {
            // ── 步骤1: 加载动态库 ──
            // `Library::new()` 调用操作系统的动态库加载函数
            // 返回的 Library 对象会在 drop 时自动调用 dlclose/FreeLibrary
            let library = Library::new(lib_path)
                .context("加载动态库失败")?;

            // ── 步骤2: 获取 plugin_create 函数指针 ──
            // `library.get(b"plugin_create")` 在动态库中查找名为 "plugin_create" 的符号
            // 泛型参数 `Symbol<PluginCreateFn>` 指定了函数签名为 `unsafe extern "C" fn() -> *mut Box<dyn Plugin>`
            let create: Symbol<PluginCreateFn> = library
                .get(b"plugin_create")
                .context("未找到 plugin_create 导出函数")?;

            // ── 步骤3: 调用工厂函数创建插件实例 ──
            // 返回原始指针（*mut Box<dyn Plugin>）
            let plugin_ptr = create();
            if plugin_ptr.is_null() {
                anyhow::bail!("plugin_create 返回空指针");
            }

            // ── 步骤4: 从原始指针重建 Box ──
            // `Box::from_raw(plugin_ptr)` 将原始指针转换回 Box，
            // 重新获得 Rust 的所有权和内存管理
            let mut plugin = Box::from_raw(plugin_ptr);

            // ── 步骤5: 初始化插件 ──
            // 调用插件的 init() 方法
            if let Err(e) = plugin.init() {
                anyhow::bail!("插件初始化失败: {}", e);
            }

            // ── 步骤6: 构建插件信息并保存 ──
            let info = PluginInfo {
                id: plugin.id().to_string(),
                name: plugin.name().to_string(),
                description: plugin.description().to_string(),
                version: plugin.version().to_string(),
                icon: plugin.icon().to_string(),
            };

            tracing::info!("插件加载成功: {} (v{})", info.name, info.version);

            // 将插件注册到 HashMap
            let mut plugins = self.plugins.write().await;
            plugins.insert(
                info.id.clone(),
                LoadedPlugin {
                    info,
                    instance: *plugin, // 解引用外层 Box，取回内层 Box<dyn Plugin>
                    _library: library, // Library 的 RAII guard
                },
            );
        }

        Ok(())
    }

    // ── 查询方法 ──

    /// 获取所有已加载的插件信息列表
    /// 只返回 PluginInfo（不包含实例），前端展示用
    pub async fn get_installed_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .read()           // 获取读锁
            .await            // 异步等待
            .values()         // 获取所有值（HashMap 的迭代器）
            .map(|p| p.info.clone()) // 克隆 PluginInfo
            .collect()        // 收集到 Vec
    }

    /// 根据 ID 获取单个插件信息
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<PluginInfo> {
        self.plugins
            .read()
            .await
            .get(plugin_id)   // HashMap::get 返回 Option<&V>
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

    // ── 生命周期管理 ──

    /// 卸载指定插件（释放 DLL 句柄）
    ///
    /// 调用插件的 destroy() 方法进行清理，然后从 HashMap 中移除。
    /// 当 LoadedPlugin 被 drop 时，_library 也被 drop，触发动态库卸载。
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(mut loaded) = plugins.remove(plugin_id) {
            tracing::info!("卸载插件: {}", plugin_id);
            // 调用插件的清理方法
            if let Err(e) = loaded.instance.destroy() {
                tracing::warn!("插件 {} destroy 失败: {}", plugin_id, e);
            }
            // loaded 离开作用域后被 drop
            // → loaded.instance 被 drop（释放插件实例）
            // → loaded._library 被 drop（卸载动态库）
            // → 操作系统释放 DLL 文件锁
        }
        Ok(())
    }

    // ── 方法调用 ──

    /// 调用插件方法
    ///
    /// 这是插件系统的核心数据通路：前端请求 → 路由到插件 → 执行 → 返回结果
    ///
    /// ## 为什么用 write() 而非 read()？
    /// `handle_call(&mut self, ...)` 需要 `&mut self`（可变引用）。
    /// 虽然大多数插件操作不需要修改自身，但 trait 定义使用了 `&mut self`
    /// 以支持插件可以修改内部状态。
    pub async fn call_plugin_method(
        &self,
        plugin_id: &str,
        method: &str,
        params: Value,
    ) -> Result<Value> {
        // 获取写锁（因为 handle_call 需要 &mut self）
        let mut plugins = self.plugins.write().await;

        let plugin = plugins
            .get_mut(plugin_id) // 可变引用访问
            .ok_or_else(|| anyhow::anyhow!("插件不存在: {}", plugin_id))?;

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

/// Default trait 实现 — 允许 PluginManager::default() 语法
///
/// ## Rust 知识点: Default trait
/// 实现了 Default 的类型可以用 `T::default()` 或 `Default::default()` 创建默认值。
/// 许多容器和框架依赖 Default 来初始化。
impl Default for PluginManager {
    fn default() -> Self {
        Self::new().expect("无法创建插件管理器")
    }
}
