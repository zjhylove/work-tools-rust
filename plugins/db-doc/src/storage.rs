use crate::crypto::PasswordEncryptor;
use crate::models::{ConnectionConfig, ExportHistory};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use worktools_plugin_api::storage::PluginStorage;

/// 插件数据存储结构
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DbDocData {
    /// 数据版本
    pub version: u32,
    /// 保存的连接配置
    pub connections: Vec<ConnectionConfig>,
    /// 导出历史
    pub export_history: Vec<ExportHistory>,
}

impl DbDocData {
    pub fn new() -> Self {
        Self {
            version: 1,
            connections: Vec::new(),
            export_history: Vec::new(),
        }
    }
}

/// 全局加密器实例
static ENCRYPTOR: Lazy<PasswordEncryptor> = Lazy::new(PasswordEncryptor::new);

/// 数据存储管理器
pub struct DbDocStorage {
    storage: PluginStorage,
}

impl DbDocStorage {
    pub fn new() -> Self {
        Self {
            storage: PluginStorage::new("db-doc", "db-doc.json"),
        }
    }

    /// 加载数据
    pub fn load(&self) -> Result<DbDocData> {
        self.storage.load_json()
    }

    /// 保存数据
    pub fn save(&self, data: &DbDocData) -> Result<()> {
        self.storage.save_json(data)
    }

    /// 加密密码
    pub fn encrypt_password(&self, password: &str) -> Result<String> {
        ENCRYPTOR.encrypt(password)
    }

    /// 解密密码
    pub fn decrypt_password(&self, encrypted: &str) -> Result<String> {
        ENCRYPTOR.decrypt(encrypted)
    }

    /// 获取所有连接配置 (密码解密后)
    pub fn list_connections(&self) -> Result<Vec<ConnectionConfig>> {
        let data = self.load()?;
        let connections = data
            .connections
            .into_iter()
            .map(|mut conn| {
                if let Some(ref encrypted) = conn.password {
                    conn.password = self.decrypt_password(encrypted).ok();
                }
                conn
            })
            .collect();
        Ok(connections)
    }

    /// 保存连接配置 (密码加密后)
    pub fn save_connection(&self, mut config: ConnectionConfig) -> Result<ConnectionConfig> {
        let mut data = self.load()?;

        // 加密密码
        if let Some(ref password) = config.password {
            config.password = Some(self.encrypt_password(password)?);
        }

        // 更新或添加
        if let Some(pos) = data.connections.iter().position(|c| c.id == config.id) {
            data.connections[pos] = config.clone();
        } else {
            data.connections.push(config.clone());
        }

        self.save(&data)?;

        // 返回时解密密码
        if let Some(ref encrypted) = config.password {
            config.password = self.decrypt_password(encrypted).ok();
        }

        Ok(config)
    }

    /// 删除连接配置
    pub fn delete_connection(&self, id: &str) -> Result<()> {
        let mut data = self.load()?;
        data.connections.retain(|c| c.id != id);
        self.save(&data)
    }

    /// 添加导出历史
    pub fn add_export_history(&self, history: ExportHistory) -> Result<()> {
        let mut data = self.load()?;
        data.export_history.push(history);
        // 只保留最近 50 条记录
        if data.export_history.len() > 50 {
            data.export_history.remove(0);
        }
        self.save(&data)
    }
}

impl Default for DbDocStorage {
    fn default() -> Self {
        Self::new()
    }
}
