use anyhow::Result;
use serde::{Deserialize, Serialize};
use worktools_plugin_api::storage::PluginStorage;

use crate::models::ApiDocConfig;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ApiDocData {
    pub version: u32,
    pub last_config: Option<ApiDocConfig>,
}

impl ApiDocData {
    pub fn new() -> Self {
        Self {
            version: 1,
            last_config: None,
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
}
