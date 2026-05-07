use anyhow::Result;
use serde::{Deserialize, Serialize};
use worktools_plugin_api::storage::PluginStorage;

use crate::models::{ApiDocConfig, ExportHistory};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ApiDocData {
    pub version: u32,
    pub last_config: Option<ApiDocConfig>,
    pub export_history: Vec<ExportHistory>,
}

impl ApiDocData {
    pub fn new() -> Self {
        Self {
            version: 1,
            last_config: None,
            export_history: Vec::new(),
        }
    }
}

pub struct ApiDocStorage {
    storage: PluginStorage,
}

impl Default for ApiDocStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiDocStorage {
    pub fn new() -> Self {
        Self {
            storage: PluginStorage::new("api-doc", "api-doc.json"),
        }
    }

    pub fn save_config(&self, config: &ApiDocConfig) -> Result<()> {
        let mut data: ApiDocData = self.storage.load_json()?;
        data.last_config = Some(config.clone());
        self.storage.save_json(&data)
    }

    pub fn load_config(&self) -> Result<Option<ApiDocConfig>> {
        let data: ApiDocData = self.storage.load_json()?;
        Ok(data.last_config)
    }

    pub fn add_export_history(&self, history: ExportHistory) -> Result<()> {
        let mut data: ApiDocData = self.storage.load_json()?;
        data.export_history.push(history);
        if data.export_history.len() > 50 {
            data.export_history.remove(0);
        }
        self.storage.save_json(&data)
    }

    pub fn get_export_history(&self) -> Result<Vec<ExportHistory>> {
        let data: ApiDocData = self.storage.load_json()?;
        Ok(data.export_history)
    }
}
