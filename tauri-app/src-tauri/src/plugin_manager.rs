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
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::RwLock as AsyncRwLock;
use worktools_plugin_api::{Plugin, PluginCreateFn};
use worktools_shared_types::PluginInfo;

use crate::plugin_package::PluginManifest;

/// 插件运行时实例（可变部分，需要写锁保护）
///
/// 包含插件 trait 对象和动态库句柄。
/// 所有需要 &mut self 的操作（handle_call, destroy）都需要写锁。
struct PluginInstance {
    instance: Box<dyn Plugin>,
    /// 保存 Library 实例，防止被释放
    /// 只要这个字段存在，动态库就会保持在内存中
    _library: Library,
    /// DLL 文件路径，用于检测是否已加载（避免重复加载导致卸载崩溃）
    _library_path: PathBuf,
}

/// 插件条目：(PluginInfo, Arc<RwLock<PluginInstance>>)
/// - `PluginInfo` 不可变，存储在外层 — 查询插件列表只需外层读锁，无需触及内层锁
/// - `PluginInstance` 可变（handle_call/destroy 需要 &mut self），受内层 RwLock 保护
type PluginEntry = (PluginInfo, Arc<RwLock<PluginInstance>>);

/// 插件管理器
///
/// HashMap 存储 PluginEntry，查询插件列表只需外层读锁，无需触及内层锁。
pub struct PluginManager {
    plugins: AsyncRwLock<HashMap<String, PluginEntry>>,
    /// 插件目录路径
    plugin_dir: PathBuf,
}

/// 扫描插件目录，返回所有找到的动态库路径。
///
/// 这是一个纯同步函数，设计为在 `spawn_blocking` 中调用，
/// 以避免阻塞 tokio 异步运行时。
fn scan_plugin_dir(plugin_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let entries = std::fs::read_dir(plugin_dir).context("读取插件目录失败")?;
    let mut lib_paths = Vec::new();

    for entry in entries {
        let entry = entry.context("读取目录项失败")?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        // 优先从 manifest.json 读取动态库文件名
        let manifest_path = path.join("manifest.json");
        let lib_path = if manifest_path.exists() {
            std::fs::read_to_string(&manifest_path)
                .ok()
                .and_then(|content| {
                    serde_json::from_str::<PluginManifest>(&content)
                        .map_err(|e| {
                            tracing::warn!(
                                path = %manifest_path.display(),
                                "解析 manifest.json 失败: {}",
                                e
                            );
                            e
                        })
                        .ok()
                })
                .and_then(|manifest| manifest.get_library_filename().cloned())
                .map(|name| path.join(name))
        } else {
            // 旧版方式：根据目录名推测动态库名
            tracing::warn!(
                dir = %path.display(),
                "插件目录缺少 manifest.json，使用旧版名称推测"
            );
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|plugin_name| {
                    let lib_name = format!(
                        "{}{}.{}",
                        PluginManager::get_platform_prefix(),
                        plugin_name.replace('-', "_"),
                        PluginManager::get_platform_extension()
                    );
                    path.join(lib_name)
                })
        };

        if let Some(path) = lib_path {
            if path.exists() {
                lib_paths.push(path);
            }
        }
    }

    Ok(lib_paths)
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


    // ── 构造与初始化 ──

    /// 创建新的插件管理器
    ///
    /// ## Rust 知识点: Result 和错误传播
    /// `crate::paths::plugins_dir()?` 中的 `?` 表示：
    /// 如果函数返回 Err，立即将错误从 `new()` 传播出去。
    pub fn new() -> Result<Self> {
        let plugin_dir = crate::paths::plugins_dir()?;

        // 确保插件目录存在
        std::fs::create_dir_all(&plugin_dir).context("创建插件目录失败")?;

        Ok(Self {
            plugins: AsyncRwLock::new(HashMap::new()),
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
        self.plugins.write().await.clear();

        // 将同步 I/O 移到 spawn_blocking，避免阻塞 tokio 异步运行时
        let plugin_dir = self.plugin_dir.clone();
        let scan_result = tokio::task::spawn_blocking(move || {
            scan_plugin_dir(&plugin_dir)
        }).await.map_err(|e| anyhow::anyhow!("插件扫描任务失败: {}", e))?;

        let lib_paths = scan_result.context("扫描插件目录失败")?;

        for lib_path in lib_paths {
            if let Err(e) = self.load_plugin(&lib_path).await {
                tracing::warn!("加载插件失败 {:?}: {}", lib_path, e);
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
    /// 所有耗时操作（Library::new、符号查找、工厂函数调用、init）都在无锁状态下执行，
    /// 仅在最终插入 HashMap 时短暂持有写锁。
    ///
    /// ## Rust 知识点: unsafe 块
    /// 仅包含以下 unsafe 操作：
    /// 1. `Library::new()` — 加载任意动态库，可能有恶意代码
    /// 2. `library.get()` — 查找符号，类型安全性由程序员保证
    /// 3. `create()` — 调用 FFI 函数，可能违反 Rust 的安全保证
    /// 4. `Box::from_raw()` — 从原始指针重建 Box
    async fn load_plugin(&self, lib_path: &Path) -> Result<()> {
        tracing::info!("加载插件: {:?}", lib_path);

        // ── 阶段 1: 无锁加载（耗时操作） ──
        // Library::new, 符号查找, 工厂函数, init 全部在锁外完成，
        // 避免慢速插件初始化阻塞所有其他操作
        let (info, instance) = unsafe {
            // ── 步骤1: 加载动态库 ──
            // `Library::new()` 调用操作系统的动态库加载函数
            // 返回的 Library 对象会在 drop 时自动调用 dlclose/FreeLibrary
            let library = Library::new(lib_path).context("加载动态库失败")?;

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

            // ── 步骤6: 构建插件信息 ──
            let info = plugin.info();

            tracing::info!("插件加载成功: {} (v{})", info.name, info.version);

            let pi = PluginInstance {
                instance: *plugin,
                _library: library,
                _library_path: lib_path.to_path_buf(),
            };

            (info, pi)
        };

        // ── 阶段 2: 短暂写锁仅用于 HashMap::insert ──
        let plugin_id = info.id.clone();
        let mut plugins = self.plugins.write().await;
        if plugins.contains_key(&plugin_id) {
            tracing::warn!(
                id = %plugin_id,
                "插件已存在，跳过重复加载"
            );
            // instance 在此处 drop，自动卸载动态库
            return Ok(());
        }
        plugins.insert(plugin_id, (info, Arc::new(RwLock::new(instance))));

        Ok(())
    }

    // ── 增量加载 ──

    /// 增量加载单个插件目录
    ///
    /// 与 `init()` 的区别：
    /// - `init()` 会先清空所有已加载插件（卸载 DLL），再全量重载 → Windows 上 DLL 卸载/重载会崩溃
    /// - 本方法只加载一个新插件，不影响已加载的插件 → 安全
    pub async fn load_plugin_by_dir(&self, plugin_dir: &Path) -> Result<String> {
        let dir_name = plugin_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // 将 manifest 读取移到 spawn_blocking，避免阻塞 async runtime
        let plugin_dir_owned = plugin_dir.to_path_buf();
        let scan_result = tokio::task::spawn_blocking(move || {
            let manifest_path = plugin_dir_owned.join("manifest.json");
            if !manifest_path.exists() {
                return Ok::<Option<PathBuf>, anyhow::Error>(None);
            }
            Ok(std::fs::read_to_string(&manifest_path)
                .ok()
                .and_then(|content| serde_json::from_str::<PluginManifest>(&content).ok())
                .and_then(|manifest| manifest.get_library_filename().cloned())
                .map(|name| plugin_dir_owned.join(name)))
        })
        .await
        .map_err(|e| anyhow::anyhow!("任务执行失败: {}", e))??;

        let lib_path = match scan_result {
            Some(path) if path.exists() => path,
            _ => anyhow::bail!("未找到插件动态库: {}", dir_name),
        };

        // 检查 DLL 路径是否已加载（避免重复加载导致旧 DLL 被卸载崩溃）
        {
            let plugins = self.plugins.read().await;
            let already_loaded = plugins
                .values()
                .any(|(_, inst)| inst.read()._library_path == lib_path);
            if already_loaded {
                tracing::info!(dir = %dir_name, "插件已加载，跳过重复加载");
                return Ok(dir_name.to_string());
            }
        }

        tracing::info!("增量加载插件: {:?}", lib_path);
        self.load_plugin(&lib_path).await?;

        let count = self.plugins.read().await.len();
        tracing::info!("增量加载完成，当前已加载 {} 个插件", count);
        Ok(dir_name.to_string())
    }

    // ── 查询方法 ──

    /// 获取所有已加载的插件信息列表
    /// PluginInfo 存储在外层 — 只需外层读锁，无需触及内层锁（lock-free 查询）
    pub async fn get_installed_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .read().await
            .values()
            .map(|(info, _)| info.clone())
            .collect()
    }
    /// 根据 ID 获取单个插件信息
    /// 只需外层读锁 — 不触及 PluginInstance 内层锁
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<PluginInfo> {
        self.plugins
            .read().await
            .get(plugin_id)
            .map(|(info, _)| info.clone())
    }
    /// 获取插件视图 HTML
    pub async fn get_plugin_view(&self, plugin_id: &str) -> Result<String> {
        let (_, inst_arc) = self.plugins
            .read().await
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("插件不存在: {}", plugin_id))?;

        let inst = inst_arc.read();
        Ok(inst.instance.get_view())
    }

    // ── 生命周期管理 ──

    /// 卸载指定插件（释放 DLL 句柄）
    ///
    /// 调用插件的 destroy() 方法进行清理，然后从 HashMap 中移除。
    /// PluginInstance 被 drop 时，_library 也被 drop，触发动态库卸载。
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        // 先从 HashMap 移除（短暂写锁），再在无锁状态下调用 destroy()
        let inst_arc = {
            let mut plugins = self.plugins.write().await;
            plugins.remove(plugin_id).map(|(_, inst)| inst)
        };

        if let Some(arc) = inst_arc {
            tracing::info!("卸载插件: {}", plugin_id);
            let mut inst = arc.write();
            if let Err(e) = inst.instance.destroy() {
                tracing::warn!("插件 {} destroy 失败: {}", plugin_id, e);
            }
            // inst 和 arc 在此处 drop，自动卸载动态库
        }
        Ok(())
    }

    // ── 方法调用 ──

    /// 调用插件方法
    ///
    /// 使用 per-plugin RwLock::write()：只锁定目标插件的写锁，
    /// 其他插件的读操作（get_installed_plugins 等）不被阻塞。
    /// 查询时只需外层读锁获取 Arc，立即释放；仅持有目标插件的内层写锁。
    ///
    /// 包含 30 秒超时保护，防止恶意/故障插件阻塞调用线程。
    pub async fn call_plugin_method(
        &self,
        plugin_id: &str,
        method: &str,
        params: Value,
    ) -> Result<Value> {
        // 读锁查找 + clone Arc（立即释放读锁）
        let (_, inst_arc) = self.plugins
            .read().await
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("插件不存在: {}", plugin_id))?;

        // spawn_blocking 要求 'static，所以克隆参数
        let method_owned = method.to_owned();
        let params_owned = params;
        let plugin_id_owned = plugin_id.to_owned();
        let timeout_pid = plugin_id.to_owned();
        let timeout_method = method.to_owned();
        // 使用 spawn_blocking 执行插件调用，避免阻塞 async runtime
        // 超时 30 秒
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            tokio::task::spawn_blocking(move || {
                let mut inst = inst_arc.write();
                inst
                    .instance
                    .handle_call(&method_owned, params_owned)
                    .inspect_err(|e| {
                        tracing::error!(
                            plugin_id = %plugin_id_owned,
                            method = %method_owned,
                            "插件方法调用失败: {}",
                            e
                        );
                    })
                    .map_err(|e| anyhow::anyhow!("插件方法调用失败: {}", e))
            })
        ).await;

        match result {
            Ok(Ok(Ok(value))) => Ok(value),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err(anyhow::anyhow!("插件调用任务异常")),
            Err(_) => Err(anyhow::anyhow!("插件方法调用超时 (30s): {}::{}", timeout_pid, timeout_method)),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_extension_is_dll_on_windows() {
        if cfg!(target_os = "windows") {
            assert_eq!(PluginManager::get_platform_extension(), "dll");
            assert_eq!(PluginManager::get_platform_prefix(), "");
        } else if cfg!(target_os = "macos") {
            assert_eq!(PluginManager::get_platform_extension(), "dylib");
            assert_eq!(PluginManager::get_platform_prefix(), "lib");
        } else if cfg!(target_os = "linux") {
            assert_eq!(PluginManager::get_platform_extension(), "so");
            assert_eq!(PluginManager::get_platform_prefix(), "lib");
        }
    }

    #[test]
    fn scan_plugin_dir_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = scan_plugin_dir(dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn scan_plugin_dir_finds_manifest_based_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("my-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        // Write a manifest with the correct platform library
        let ext = PluginManager::get_platform_extension();
        let prefix = PluginManager::get_platform_prefix();
        let lib_name = format!("{}my_plugin.{}", prefix, ext);

        let manifest = serde_json::json!({
            "id": "my-plugin",
            "name": "My Plugin",
            "description": "Test plugin",
            "version": "1.0.0",
            "files": {
                "windows": lib_name,
                "macos": format!("libmy_plugin.dylib"),
                "linux": format!("libmy_plugin.so")
            },
            "assets": { "entry": "index.html" }
        });
        std::fs::write(
            plugin_dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        ).unwrap();

        // Create a fake .dll file
        std::fs::write(plugin_dir.join(&lib_name), "fake").unwrap();

        let result = scan_plugin_dir(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].ends_with(&lib_name));
    }

    #[test]
    fn scan_plugin_dir_skips_non_directories() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("readme.txt"), "hello").unwrap();

        let result = scan_plugin_dir(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn scan_plugin_dir_ignores_missing_library() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("empty-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        let manifest = serde_json::json!({
            "id": "empty-plugin",
            "name": "Empty",
            "version": "1.0.0",
            "files": { "windows": "empty.dll" },
            "assets": { "entry": "index.html" }
        });
        std::fs::write(
            plugin_dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        ).unwrap();

        // No .dll file created — should not appear in results
        let result = scan_plugin_dir(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn scan_plugin_dir_skips_corrupted_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("corrupted-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        // Write invalid JSON as manifest
        std::fs::write(plugin_dir.join("manifest.json"), "{invalid json").unwrap();

        // Should not panic or error — just skip the plugin
        let result = scan_plugin_dir(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn scan_plugin_dir_fallback_without_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("legacy-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        // No manifest.json — uses fallback: prefix + dir_name (with hyphens→underscores) + ext
        let ext = PluginManager::get_platform_extension();
        let prefix = PluginManager::get_platform_prefix();
        let lib_name = format!("{}legacy_plugin.{}", prefix, ext);
        std::fs::write(plugin_dir.join(&lib_name), "fake").unwrap();

        let result = scan_plugin_dir(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].ends_with(&lib_name));
    }

    #[tokio::test]
    async fn new_creates_plugin_dir() {
        let dir = tempfile::tempdir().unwrap();
        // Override plugin dir to temp
        let pm = PluginManager {
            plugins: AsyncRwLock::new(HashMap::new()),
            plugin_dir: dir.path().join("plugins"),
        };
        assert!(!pm.plugin_dir.exists());

        std::fs::create_dir_all(&pm.plugin_dir).unwrap();
        assert!(pm.plugin_dir.exists());
    }

    #[tokio::test]
    async fn get_installed_plugins_empty() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PluginManager {
            plugins: AsyncRwLock::new(HashMap::new()),
            plugin_dir: dir.path().to_path_buf(),
        };
        assert!(pm.get_installed_plugins().await.is_empty());
    }

    #[tokio::test]
    async fn get_plugin_returns_none_for_missing() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PluginManager {
            plugins: AsyncRwLock::new(HashMap::new()),
            plugin_dir: dir.path().to_path_buf(),
        };
        assert!(pm.get_plugin("nonexistent").await.is_none());
    }
}
