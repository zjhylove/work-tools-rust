//! # 插件包管理
//!
//! 处理 `.wtplugin.zip` 格式的插件包：解析、验证、安装。
//!
//! ## 插件包格式
//! ```
//! plugin.zip
//! ├── manifest.json          # 插件元数据（必须）
//! ├── libplugin.dll/.so/.dylib # 动态库（按平台）
//! └── assets/                # 前端资源
//!     ├── index.html
//!     ├── main.js
//!     └── styles.css
//! ```
//!
//! ## Rust 知识点
//! - `zip` crate: 读取 ZIP 归档文件
//! - `Cursor`: 将字节数组包装为实现了 Read + Seek 的类型
//! - `cfg!`: 编译时条件判断，用于跨平台文件选择

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

/// 插件包结构
/// 包含已解析的 manifest 和原始的 ZIP 字节数据
/// 保留原始字节是因为后续需要多次读取（解压 + 验证）
pub struct PluginPackage {
    pub manifest: PluginManifest,
    /// 原始 ZIP 数据，保留以便多次解析
    archive_data: Vec<u8>,
}

/// 插件清单（manifest.json 的结构）
///
/// ## Rust 知识点: serde 属性
/// - `#[serde(default)]`: 反序列化时如果字段缺失，使用 Default::default()
/// - `#[serde(rename_all = "camelCase")]`: 支持 camelCase JSON 字段名
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,          // 唯一标识，如 "password-manager"
    pub name: String,        // 显示名称
    pub description: String, // 功能描述
    pub version: String,     // 版本号
    #[serde(default)]
    pub icon: Option<String>, // 图标
    #[serde(default)]
    pub author: Option<String>, // 作者
    #[serde(default)]
    pub homepage: Option<String>, // 项目主页
    #[serde(default)]
    pub min_app_version: Option<String>, // 最低应用版本要求
    #[serde(default)]
    pub license: Option<String>, // 许可证
    pub files: PlatformFiles, // 各平台的动态库文件配置
    pub assets: AssetsConfig, // 前端资源配置
    #[serde(default)]
    pub permissions: Vec<String>, // 权限列表
    #[serde(default)]
    pub screenshots: Vec<String>, // 截图列表
}

/// 各平台动态库文件配置
///
/// 因为不同平台的动态库文件名不同：
/// - Windows: `password_manager.dll`
/// - macOS: `libpassword_manager.dylib`
/// - Linux: `libpassword_manager.so`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformFiles {
    #[serde(default)]
    pub macos: Option<String>,
    #[serde(default)]
    pub linux: Option<String>,
    #[serde(default)]
    pub windows: Option<String>,
}

impl PluginManifest {
    /// 获取当前平台对应的动态库文件名
    /// `cfg!` 在编译时求值，其他平台的分支会被优化掉
    pub fn get_library_filename(&self) -> Option<&String> {
        if cfg!(target_os = "macos") {
            self.files.macos.as_ref()
        } else if cfg!(target_os = "linux") {
            self.files.linux.as_ref()
        } else if cfg!(target_os = "windows") {
            self.files.windows.as_ref()
        } else {
            None
        }
    }
}

/// 前端资源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetsConfig {
    /// 入口 HTML 文件名，例如 "index.html"
    pub entry: String,
    #[serde(default)]
    pub icon: Option<String>,
}

impl PluginPackage {
    /// 从 ZIP 文件路径加载插件包
    pub fn from_zip(zip_path: &Path) -> Result<Self> {
        let zip_data = std::fs::read(zip_path).context("读取插件包文件失败")?;
        Self::from_zip_bytes(&zip_data)
    }

    /// 从 ZIP 字节数据加载插件包
    ///
    /// ## Rust 知识点: Cursor
    /// `Cursor::new(data)` 将字节数组包装为一个实现了 `Read` + `Seek` trait 的类型。
    /// 这使得我们可以像操作文件一样操作内存中的数据。
    pub fn from_zip_bytes(data: &[u8]) -> Result<Self> {
        // `Cursor` 允许在内存中的字节数组上进行文件操作
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor).context("解析 ZIP 文件失败")?;

        // 从 ZIP 中读取 manifest.json
        // `by_name` 在 ZIP 条目中按文件名查找
        let manifest_file = archive
            .by_name("manifest.json")
            .context("插件包中未找到 manifest.json")?;

        // `from_reader` 直接从 ZIP 条目流中反序列化 JSON
        let manifest: PluginManifest =
            serde_json::from_reader(manifest_file).context("解析 manifest.json 失败")?;

        Ok(Self {
            manifest,
            archive_data: data.to_vec(), // 保存原始数据供后续使用
        })
    }

    /// 安装插件到指定目录
    /// 将 ZIP 中的所有文件解压到 `plugin_dir`
    ///
    /// ## Rust 知识点: 泛型方法
    /// `ZipArchive::new(cursor)` — cursor 的类型由参数推断。
    /// 也可以写成 `ZipArchive::new::<Cursor<&[u8]>>(cursor)` 但通常不需要。
    pub fn install(&self, plugin_dir: &Path) -> Result<()> {
        tracing::info!("安装插件到: {:?}", plugin_dir);

        let cursor = Cursor::new(&self.archive_data);
        let mut archive = ZipArchive::new(cursor)?;

        // 确保目标目录存在
        std::fs::create_dir_all(plugin_dir).context("创建插件目录失败")?;

        // `extract` 递归解压所有文件到目标目录
        archive.extract(plugin_dir).context("解压插件包失败")?;

        tracing::info!("插件 {} 安装成功", self.manifest.id);
        Ok(())
    }

    /// 获取当前平台对应的动态库文件名
    pub fn get_library_filename(&self) -> Option<&String> {
        if cfg!(target_os = "macos") {
            self.manifest.files.macos.as_ref()
        } else if cfg!(target_os = "linux") {
            self.manifest.files.linux.as_ref()
        } else if cfg!(target_os = "windows") {
            self.manifest.files.windows.as_ref()
        } else {
            None
        }
    }

    /// 获取动态库的完整路径（插件目录 + 库文件名）
    pub fn get_library_path(&self, plugin_dir: &Path) -> Result<PathBuf> {
        let lib_name = self
            .get_library_filename()
            .ok_or_else(|| anyhow::anyhow!("当前平台不受支持"))?;

        Ok(plugin_dir.join(lib_name))
    }

    /// 获取前端资源目录路径
    pub fn get_assets_dir(&self, plugin_dir: &Path) -> PathBuf {
        plugin_dir.join("assets")
    }

    /// 验证插件包完整性
    ///
    /// 检查项：
    /// 1. 插件 ID 不能为空
    /// 2. 插件 ID 只能包含小写字母、数字和连字符（安全约束）
    /// 3. 必须配置当前平台的动态库文件
    /// 4. ZIP 包中必须包含 manifest.json
    /// 5. ZIP 包中必须包含声明的动态库文件
    /// 6. ZIP 包中必须包含前端入口文件
    pub fn validate(&self) -> Result<()> {
        // 检查 ID 非空
        if self.manifest.id.is_empty() {
            anyhow::bail!("插件 ID 不能为空");
        }

        // 检查 ID 格式：只允许小写字母、数字、连字符
        // `all()` 迭代器方法：检查所有元素是否满足条件
        if !self
            .manifest
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            anyhow::bail!("插件 ID 只能包含小写字母、数字和连字符");
        }

        // 检查动态库文件配置
        let lib_name = match self.get_library_filename() {
            Some(name) => name,
            None => anyhow::bail!("未配置当前平台的动态库文件"),
        };

        // 重新打开 ZIP 检查文件列表
        let cursor = Cursor::new(&self.archive_data);
        let archive = ZipArchive::new(cursor)?;

        let mut manifest_found = false;
        let mut library_found = false;
        let mut assets_entry_found = false;

        // `archive.file_names()` 返回 ZIP 中所有文件名
        for file_name in archive.file_names() {
            if file_name == "manifest.json" {
                manifest_found = true;
            }
            if file_name.ends_with(lib_name) && !file_name.contains("assets/") {
                library_found = true;
            }
            if file_name == format!("assets/{}", self.manifest.assets.entry) {
                assets_entry_found = true;
            }
        }

        if !manifest_found {
            anyhow::bail!("插件包缺少 manifest.json");
        }
        if !library_found {
            anyhow::bail!("插件包缺少动态库文件: {}", lib_name);
        }
        if !assets_entry_found {
            anyhow::bail!(
                "插件包缺少前端入口文件: assets/{}",
                self.manifest.assets.entry
            );
        }

        tracing::info!("插件包 {} 验证通过", self.manifest.id);
        Ok(())
    }
}
