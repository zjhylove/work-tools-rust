use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

/// 插件包结构
pub struct PluginPackage {
    pub manifest: PluginManifest,
    pub archive_data: Vec<u8>,
}

/// 插件清单元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
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
    #[serde(default)]
    pub min_app_version: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    pub files: PlatformFiles,
    pub assets: AssetsConfig,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,
}

/// 各平台动态库文件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformFiles {
    #[serde(default)]
    pub macos: Option<String>,
    #[serde(default)]
    pub linux: Option<String>,
    #[serde(default)]
    pub windows: Option<String>,
}

/// 前端资源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetsConfig {
    pub entry: String,
    #[serde(default)]
    pub icon: Option<String>,
}

impl PluginPackage {
    /// 从 ZIP 文件路径加载插件包
    pub fn from_zip(zip_path: &Path) -> Result<Self> {
        let zip_data = std::fs::read(zip_path)
            .context("读取插件包文件失败")?;
        Self::from_zip_bytes(&zip_data)
    }

    /// 从 ZIP 字节数据加载插件包
    pub fn from_zip_bytes(data: &[u8]) -> Result<Self> {
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor)
            .context("解析 ZIP 文件失败")?;

        // 读取并解析 manifest.json
        let manifest_file = archive.by_name("manifest.json")
            .context("插件包中未找到 manifest.json")?;

        let manifest: PluginManifest = serde_json::from_reader(manifest_file)
            .context("解析 manifest.json 失败")?;

        Ok(Self {
            manifest,
            archive_data: data.to_vec(),
        })
    }

    /// 安装插件到指定目录
    pub fn install(&self, plugin_dir: &Path) -> Result<()> {
        tracing::info!("安装插件到: {:?}", plugin_dir);

        let cursor = Cursor::new(&self.archive_data);
        let mut archive = ZipArchive::new(cursor)?;

        // 创建插件目录
        std::fs::create_dir_all(plugin_dir)
            .context("创建插件目录失败")?;

        // 解压所有文件
        archive.extract(plugin_dir)
            .context("解压插件包失败")?;

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

    /// 获取动态库的完整路径
    pub fn get_library_path(&self, plugin_dir: &Path) -> Result<PathBuf> {
        let lib_name = self.get_library_filename()
            .ok_or_else(|| anyhow::anyhow!("当前平台不受支持"))?;

        Ok(plugin_dir.join(lib_name))
    }

    /// 获取前端资源入口路径
    #[allow(dead_code)]
    pub fn get_assets_entry_path(&self, plugin_dir: &Path) -> PathBuf {
        plugin_dir.join("assets").join(&self.manifest.assets.entry)
    }

    /// 获取前端资源目录路径
    pub fn get_assets_dir(&self, plugin_dir: &Path) -> PathBuf {
        plugin_dir.join("assets")
    }

    /// 验证插件包完整性
    pub fn validate(&self) -> Result<()> {
        // 检查必需字段
        if self.manifest.id.is_empty() {
            anyhow::bail!("插件 ID 不能为空");
        }

        if !self.manifest.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            anyhow::bail!("插件 ID 只能包含小写字母、数字和连字符");
        }

        // 检查动态库文件配置
        let lib_name = self.get_library_filename()
            .ok_or_else(|| anyhow::anyhow!("未配置当前平台的动态库文件"))?;

        let cursor = Cursor::new(&self.archive_data);
        let archive = ZipArchive::new(cursor)?;

        let mut manifest_found = false;
        let mut library_found = false;
        let mut assets_entry_found = false;

        // 检查必需文件
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
            anyhow::bail!("插件包缺少前端入口文件: assets/{}", self.manifest.assets.entry);
        }

        tracing::info!("插件包 {} 验证通过", self.manifest.id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_id_validation() {
        // 有效的 ID
        let valid_json = r#"{
            "id": "test-plugin",
            "name": "Test",
            "description": "Test plugin",
            "version": "1.0.0",
            "files": {},
            "assets": {"entry": "index.html"}
        }"#;

        let manifest: PluginManifest = serde_json::from_str(valid_json).unwrap();
        assert_eq!(manifest.id, "test-plugin");
    }
}
