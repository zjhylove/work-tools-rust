use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

/// 插件数据存储辅助工具
pub struct PluginStorage {
    plugin_id: String,
    data_filename: String,
}

impl PluginStorage {
    /// 创建新的插件存储实例
    pub fn new(plugin_id: &str, data_filename: &str) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            data_filename: data_filename.to_string(),
        }
    }

    /// 获取数据文件路径(使用 ~/.worktools/history/plugins/)
    pub fn get_data_path(&self) -> Result<PathBuf> {
        let user_dirs = directories::UserDirs::new()
            .ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;

        let mut data_dir = user_dirs.home_dir().join(".worktools/history/plugins");

        // 创建目录(如果不存在)
        std::fs::create_dir_all(&data_dir)
            .context("创建数据目录失败")?;

        data_dir.push(&self.data_filename);
        Ok(data_dir)
    }

    /// 获取替代数据文件路径(使用系统数据目录)
    pub fn get_alternative_data_path(&self) -> Result<PathBuf> {
        let mut data_dir = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("无法获取数据目录"))?;
        data_dir.push("worktools");
        data_dir.push("data");

        // 创建目录(如果不存在)
        std::fs::create_dir_all(&data_dir)?;

        data_dir.push(&self.data_filename);
        Ok(data_dir)
    }

    /// 加载 JSON 数据
    pub fn load_json<T>(&self) -> Result<T>
    where
        T: for<'de> serde::Deserialize<'de> + Default,
    {
        let data_path = self.get_data_path()?;

        if !data_path.exists() {
            return Ok(T::default());
        }

        let file = File::open(&data_path)
            .context("打开数据文件失败")?;
        let data: T = serde_json::from_reader(file)
            .context("解析数据文件失败")?;
        Ok(data)
    }

    /// 保存 JSON 数据(使用原子写入)
    pub fn save_json<T>(&self, data: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        let data_path = self.get_data_path()?;

        // 使用临时文件模式确保原子性写入
        let temp_path = data_path.with_extension("tmp");
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .context("创建临时文件失败")?;

        serde_json::to_writer_pretty(&file, data)
            .context("序列化数据失败")?;
        file.sync_all()
            .context("同步文件失败")?;

        // 原子性替换文件
        std::fs::rename(&temp_path, &data_path)
            .context("替换数据文件失败")?;

        tracing::debug!("插件 {} 数据已保存到: {:?}", self.plugin_id, data_path);
        Ok(())
    }

    /// 保存 JSON 数据并保留指定字段
    pub fn save_json_preserving<T>(
        &self,
        data: &T,
        preserve_fields: &[&str],
    ) -> Result<()>
    where
        T: serde::Serialize,
    {
        let data_path = self.get_data_path()?;

        // 读取现有配置以保留指定字段
        let existing_config = if data_path.exists() {
            File::open(&data_path)
                .ok()
                .and_then(|f| serde_json::from_reader::<_, serde_json::Value>(f).ok())
        } else {
            None
        };

        // 使用临时文件模式确保原子性写入
        let temp_path = data_path.with_extension("tmp");
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .context("创建临时文件失败")?;

        // 合并数据:保留指定字段,更新其他字段
        let mut output = serde_json::to_value(data)?;
        if let Some(config) = existing_config {
            for field in preserve_fields {
                if let Some(value) = config.get(field) {
                    output[field] = value.clone();
                }
            }
        }

        serde_json::to_writer_pretty(&file, &output)
            .context("序列化数据失败")?;
        file.sync_all()
            .context("同步文件失败")?;

        // 原子性替换文件
        std::fs::rename(&temp_path, &data_path)
            .context("替换数据文件失败")?;

        tracing::debug!("插件 {} 数据已保存到: {:?}", self.plugin_id, data_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Default)]
    struct TestData {
        entries: Vec<String>,
    }

    #[test]
    fn test_storage() {
        let storage = PluginStorage::new("test-plugin", "test.json");

        // 保存数据
        let data = TestData {
            entries: vec!["hello".to_string(), "world".to_string()],
        };
        storage.save_json(&data).unwrap();

        // 加载数据
        let loaded: TestData = storage.load_json().unwrap();
        assert_eq!(loaded.entries.len(), 2);

        // 清理
        let path = storage.get_data_path().unwrap();
        std::fs::remove_file(path).ok();
    }
}
