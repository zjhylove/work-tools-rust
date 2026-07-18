//! # 插件包管理
//!
//! 处理 `.wtplugin.zip` 格式的插件包：解析、验证、安装。
//!
//! ## 插件包格式
//! ```text
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

        // 逐条目解压，防止 ZIP Slip 路径穿越攻击
        let canonical_dir = plugin_dir
            .canonicalize()
            .context("无法解析插件目录绝对路径")?;

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .with_context(|| format!("读取 ZIP 条目 {} 失败", i))?;

            // enclosed_name() 拒绝以 /.. 或 .. 开头的条目
            let Some(enclosed) = entry.enclosed_name() else {
                anyhow::bail!(
                    "插件包包含非法路径条目 (ZIP Slip): {}",
                    entry.name()
                );
            };

            let target = canonical_dir.join(enclosed);

            // 验证解压目标仍在插件目录内
            if !target.starts_with(&canonical_dir) {
                anyhow::bail!(
                    "插件包包含路径穿越条目: {} -> {:?}",
                    entry.name(),
                    target
                );
            }

            if entry.is_dir() {
                std::fs::create_dir_all(&target)
                    .with_context(|| format!("创建目录 {:?} 失败", target))?;
            } else {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("创建父目录 {:?} 失败", parent))?;
                }
                let mut out = std::fs::File::create(&target)
                    .with_context(|| format!("创建文件 {:?} 失败", target))?;
                std::io::copy(&mut entry, &mut out)
                    .with_context(|| format!("写入文件 {:?} 失败", target))?;
            }
        }

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
    #[allow(dead_code)]
    pub fn get_library_path(&self, plugin_dir: &Path) -> Result<PathBuf> {
        let lib_name = self
            .get_library_filename()
            .ok_or_else(|| anyhow::anyhow!("当前平台不受支持"))?;

        Ok(plugin_dir.join(lib_name))
    }

    /// 获取前端资源目录路径
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Create a minimal valid ZIP with manifest.json + dummy lib + assets/index.html
    fn make_test_zip(dir: &TempDir, id: &str, extra_entry: Option<(&str, &[u8])>) -> Vec<u8> {
        let zip_path = dir.path().join("test.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());

        let manifest = serde_json::json!({
            "id": id,
            "name": "Test Plugin",
            "version": "1.0.0",
            "description": "A test plugin",
            "entry": "index.html",
            "files": {
                "windows": format!("{id}.dll")
            },
            "assets": { "entry": "index.html" }
        });
        zip.start_file("manifest.json", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(serde_json::to_string(&manifest).unwrap().as_bytes()).unwrap();

        let lib_name = format!("{id}.dll");
        zip.start_file(&lib_name, zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(b"dll_data").unwrap();

        zip.start_file("assets/index.html", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(b"<html></html>").unwrap();

        if let Some((name, data)) = extra_entry {
            zip.start_file(name, zip::write::FileOptions::<()>::default()).unwrap();
            zip.write_all(data).unwrap();
        }

        zip.finish().unwrap();
        std::fs::read(&zip_path).unwrap()
    }

    #[test]
    fn test_validate_normal_package() {
        let dir = TempDir::new().unwrap();
        let data = make_test_zip(&dir, "test-plugin", None);
        let pkg = PluginPackage::from_zip_bytes(&data).unwrap();
        assert!(pkg.validate().is_ok());
    }

    #[test]
    fn test_validate_missing_manifest() {
        let dir = TempDir::new().unwrap();
        let zip_path = dir.path().join("test.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        zip.start_file("dummy.txt", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(b"hello").unwrap();
        zip.finish().unwrap();
        let data = std::fs::read(&zip_path).unwrap();
        let result = PluginPackage::from_zip_bytes(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_id_uppercase() {
        let dir = TempDir::new().unwrap();
        let data = make_test_zip(&dir, "TestPlugin", None);
        let pkg = PluginPackage::from_zip_bytes(&data).unwrap();
        assert!(pkg.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_id_spaces() {
        let dir = TempDir::new().unwrap();
        // Manually create with bad ID
        let zip_path = dir.path().join("test.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        let manifest = serde_json::json!({
            "id": "has spaces",
            "name": "Bad Plugin",
            "version": "1.0.0",
            "description": "bad",
            "entry": "index.html",
            "files": { "windows": "has_spaces.dll" },
            "assets": { "entry": "index.html" }
        });
        zip.start_file("manifest.json", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(serde_json::to_string(&manifest).unwrap().as_bytes()).unwrap();
        zip.start_file("has_spaces.dll", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(b"dll").unwrap();
        zip.start_file("assets/index.html", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(b"<html></html>").unwrap();
        zip.finish().unwrap();
        let data = std::fs::read(&zip_path).unwrap();
        let pkg = PluginPackage::from_zip_bytes(&data).unwrap();
        assert!(pkg.validate().is_err());
    }

    #[test]
    fn test_install_zip_slip_rejected() {
        let dir = TempDir::new().unwrap();
        // Create a ZIP with a path traversal entry
        let zip_path = dir.path().join("test.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        let manifest = serde_json::json!({
            "id": "evil-plugin",
            "name": "Evil",
            "version": "1.0.0",
            "description": "bad",
            "entry": "index.html",
            "files": { "windows": "evil.dll" },
            "assets": { "entry": "index.html" }
        });
        zip.start_file("manifest.json", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(serde_json::to_string(&manifest).unwrap().as_bytes()).unwrap();
        zip.start_file("evil.dll", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(b"dll").unwrap();
        zip.start_file("assets/index.html", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(b"<html></html>").unwrap();
        // ZIP Slip entry: try to write outside plugin dir
        zip.start_file("../../etc/crontab", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(b"malicious").unwrap();
        zip.finish().unwrap();

        let data = std::fs::read(&zip_path).unwrap();
        let pkg = PluginPackage::from_zip_bytes(&data).unwrap();
        let install_dir = dir.path().join("plugins").join("evil-plugin");
        let result = pkg.install(&install_dir);
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("非法路径条目") || err_msg.contains("ZIP Slip"),
            "Expected ZIP Slip rejection message, got: {}",
            err_msg
        );
        assert!(
            !dir.path().join("etc").join("crontab").exists(),
            "File should not escape plugin dir"
        );
    }

    #[test]
    fn test_validate_rejects_path_traversal_id() {
        let dir = TempDir::new().unwrap();
        let zip_path = dir.path().join("test.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        let manifest = serde_json::json!({
            "id": "../evil",
            "name": "Evil",
            "version": "1.0.0",
            "description": "bad",
            "entry": "index.html",
            "files": { "windows": "../evil.dll" },
            "assets": { "entry": "index.html" }
        });
        zip.start_file("manifest.json", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(serde_json::to_string(&manifest).unwrap().as_bytes()).unwrap();
        zip.start_file("../evil.dll", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(b"dll").unwrap();
        zip.start_file("assets/index.html", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(b"<html></html>").unwrap();
        zip.finish().unwrap();

        let data = std::fs::read(&zip_path).unwrap();
        let pkg = PluginPackage::from_zip_bytes(&data).unwrap();
        let result = pkg.validate();
        assert!(result.is_err(), "validate() should reject path traversal ID '../evil'");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("小写字母") || err_msg.contains("非法") || err_msg.contains("连字符"),
            "Expected ID format error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_validate_rejects_dotdot_in_id() {
        let dir = TempDir::new().unwrap();
        let zip_path = dir.path().join("test.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        let manifest = serde_json::json!({
            "id": "foo..bar",
            "name": "Dot Dot",
            "version": "1.0.0",
            "description": "bad",
            "entry": "index.html",
            "files": { "windows": "foo..bar.dll" },
            "assets": { "entry": "index.html" }
        });
        zip.start_file("manifest.json", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(serde_json::to_string(&manifest).unwrap().as_bytes()).unwrap();
        zip.start_file("foo..bar.dll", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(b"dll").unwrap();
        zip.start_file("assets/index.html", zip::write::FileOptions::<()>::default()).unwrap();
        zip.write_all(b"<html></html>").unwrap();
        zip.finish().unwrap();

        let data = std::fs::read(&zip_path).unwrap();
        let pkg = PluginPackage::from_zip_bytes(&data).unwrap();
        let result = pkg.validate();
        // "foo..bar" contains '.' which is not lowercase/digit/hyphen
        assert!(result.is_err(), "validate() should reject ID with '..'");
    }

    #[test]
    fn test_install_zip_slip_rejects_deep_traversal() {
        let dir = TempDir::new().unwrap();
        let data = make_test_zip(&dir, "evil-plugin", Some(("./../../tmp/pwned", b"pwn")));
        let pkg = PluginPackage::from_zip_bytes(&data).unwrap();
        let install_dir = dir.path().join("plugins").join("evil-plugin");
        let result = pkg.install(&install_dir);
        assert!(result.is_err(), "Deep traversal entry should be rejected");
        assert!(
            !dir.path().join("tmp").join("pwned").exists(),
            "Traversal file should not exist outside plugin dir"
        );
    }

    #[test]
    fn test_install_normal_succeeds() {
        let dir = TempDir::new().unwrap();
        let data = make_test_zip(&dir, "normal-plugin", None);
        let pkg = PluginPackage::from_zip_bytes(&data).unwrap();
        let install_dir = dir.path().join("plugins").join("normal-plugin");
        pkg.install(&install_dir).unwrap();
        assert!(install_dir.join("manifest.json").exists());
        assert!(install_dir.join("assets/index.html").exists());
        assert!(install_dir.join("normal-plugin.dll").exists());
    }
}
