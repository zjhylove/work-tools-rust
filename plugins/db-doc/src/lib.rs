use anyhow::Result;
use serde_json::Value;
use tokio::runtime::Runtime;
use worktools_plugin_api::Plugin;

pub mod models;
pub mod database;
pub mod exporter;
pub mod storage;
pub mod crypto;

use models::*;
use storage::DbDocStorage;
use database::{DatabaseExtractor, MySqlExtractor};

/// 数据库文档生成插件
pub struct DbDocPlugin {
    storage: DbDocStorage,
    runtime: Runtime,
}

impl DbDocPlugin {
    pub fn new() -> Self {
        Self {
            storage: DbDocStorage::new(),
            runtime: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }

    /// 保存连接配置
    fn handle_save_connection(&self, params: Value) -> Result<Value> {
        let mut config: ConnectionConfig = serde_json::from_value(params)?;
        config.id = uuid::Uuid::new_v4().to_string();
        config.created_at = chrono::Utc::now().timestamp() as u64;

        let saved = self.storage.save_connection(config)?;
        Ok(serde_json::to_value(saved)?)
    }

    /// 更新连接配置
    fn handle_update_connection(&self, params: Value) -> Result<Value> {
        let config: ConnectionConfig = serde_json::from_value(params)?;

        // 验证 ID 存在
        let id = config.id.clone();
        let existing = self.storage.list_connections()?
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| anyhow::anyhow!("连接配置不存在"))?;

        // 保留创建时间
        let mut config = config;
        config.created_at = existing.created_at;

        let saved = self.storage.save_connection(config)?;
        Ok(serde_json::to_value(saved)?)
    }

    /// 获取所有连接配置
    fn handle_list_connections(&self) -> Result<Value> {
        let connections = self.storage.list_connections()?;
        Ok(serde_json::to_value(connections)?)
    }

    /// 删除连接配置
    fn handle_delete_connection(&self, params: Value) -> Result<Value> {
        let id = params
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 id 参数"))?;

        self.storage.delete_connection(id)?;
        Ok(serde_json::json!({ "success": true }))
    }

    /// 测试连接
    fn handle_test_connection(&self, params: Value) -> Result<Value> {
        let config: ConnectionConfig = serde_json::from_value(params)?;

        let result = match config.db_type {
            DatabaseType::MySQL => {
                let extractor = MySqlExtractor;
                self.runtime.block_on(extractor.test_connection(&config))
            }
            DatabaseType::PostgreSQL => {
                // TODO: Phase 2 实现
                return Err(anyhow::anyhow!("PostgreSQL 支持即将推出"));
            }
        };

        match result {
            Ok(true) => Ok(serde_json::json!({ "success": true, "message": "连接成功" })),
            Ok(false) => Ok(serde_json::json!({ "success": false, "message": "连接失败" })),
            Err(e) => Ok(serde_json::json!({ "success": false, "message": e.to_string() })),
        }
    }

    /// 获取表列表
    fn handle_list_tables(&self, params: Value) -> Result<Value> {
        let connection_id = params
            .get("connection_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 connection_id 参数"))?;

        // 获取连接配置
        let connections = self.storage.list_connections()?;
        let config = connections
            .into_iter()
            .find(|c| c.id == connection_id)
            .ok_or_else(|| anyhow::anyhow!("连接配置不存在"))?;

        let tables = match config.db_type {
            DatabaseType::MySQL => {
                let extractor = MySqlExtractor;
                self.runtime.block_on(extractor.list_tables(&config))?
            }
            DatabaseType::PostgreSQL => {
                return Err(anyhow::anyhow!("PostgreSQL 支持即将推出"));
            }
        };

        Ok(serde_json::to_value(tables)?)
    }

    /// 获取表详情
    fn handle_get_table_info(&self, params: Value) -> Result<Value> {
        let connection_id = params
            .get("connection_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 connection_id 参数"))?;
        let table_name = params
            .get("table_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 table_name 参数"))?;

        // 获取连接配置
        let connections = self.storage.list_connections()?;
        let config = connections
            .into_iter()
            .find(|c| c.id == connection_id)
            .ok_or_else(|| anyhow::anyhow!("连接配置不存在"))?;

        let table_info = match config.db_type {
            DatabaseType::MySQL => {
                let extractor = MySqlExtractor;
                self.runtime.block_on(extractor.get_table_info(&config, table_name))?
            }
            DatabaseType::PostgreSQL => {
                return Err(anyhow::anyhow!("PostgreSQL 支持即将推出"));
            }
        };

        Ok(serde_json::to_value(table_info)?)
    }

    /// 导出文档
    fn handle_export_docs(&self, params: Value) -> Result<Value> {
        let config: ExportConfig = serde_json::from_value(params)?;

        // 获取连接配置
        let connections = self.storage.list_connections()?;
        let conn_config = connections
            .into_iter()
            .find(|c| c.id == config.connection_id)
            .ok_or_else(|| anyhow::anyhow!("连接配置不存在"))?;

        // 获取表信息
        let tables_info = match conn_config.db_type {
            DatabaseType::MySQL => {
                let extractor = MySqlExtractor;
                self.runtime
                    .block_on(extractor.get_tables_info(&conn_config, &config.tables))?
            }
            DatabaseType::PostgreSQL => {
                return Err(anyhow::anyhow!("PostgreSQL 支持即将推出"));
            }
        };

        // 导出文档
        let output_path = std::path::PathBuf::from(&config.output_dir);
        std::fs::create_dir_all(&output_path)?;

        let mut exported_files = Vec::new();

        match config.format {
            ExportFormat::Markdown => {
                let exporter = exporter::MarkdownExporter::new(config.template);

                for table in &tables_info {
                    let file_name = format!("{}.md", table.name);
                    let file_path = output_path.join(&file_name);
                    exporter.export_table(table, &file_path)?;
                    exported_files.push(file_name);
                }
            }
            ExportFormat::Word => {
                // TODO: Phase 1 后续实现
                return Err(anyhow::anyhow!("Word 导出即将实现"));
            }
            ExportFormat::Pdf => {
                // TODO: Phase 3 实现
                return Err(anyhow::anyhow!("PDF 导出即将实现"));
            }
        }

        // 保存导出历史
        let history = ExportHistory {
            id: uuid::Uuid::new_v4().to_string(),
            connection_name: conn_config.name,
            tables: config.tables.clone(),
            format: config.format,
            template: config.template,
            output_path: config.output_dir.clone(),
            exported_at: chrono::Utc::now().to_rfc3339(),
        };
        self.storage.add_export_history(history)?;

        Ok(serde_json::json!({
            "success": true,
            "files": exported_files,
            "count": exported_files.len()
        }))
    }

    /// 获取导出历史
    fn handle_get_export_history(&self) -> Result<Value> {
        let data = self.storage.load()?;
        Ok(serde_json::to_value(data.export_history)?)
    }
}

impl Default for DbDocPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DbDocPlugin {
    fn id(&self) -> &str {
        "db-doc"
    }

    fn name(&self) -> &str {
        "数据库文档"
    }

    fn description(&self) -> &str {
        "连接数据库,自动生成表结构文档"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn icon(&self) -> &str {
        "📊"
    }

    fn get_view(&self) -> String {
        "<div>数据库文档生成器加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let result = match method {
            // 连接管理
            "save_connection" => self.handle_save_connection(params),
            "update_connection" => self.handle_update_connection(params),
            "list_connections" => self.handle_list_connections(),
            "delete_connection" => self.handle_delete_connection(params),
            "test_connection" => self.handle_test_connection(params),

            // 表查询
            "list_tables" => self.handle_list_tables(params),
            "get_table_info" => self.handle_get_table_info(params),

            // 导出
            "export_docs" => self.handle_export_docs(params),
            "get_export_history" => self.handle_get_export_history(),

            _ => Err(anyhow::anyhow!("未知方法: {}", method)),
        };

        result.map_err(|e| e.into())
    }
}

/// 插件工厂函数
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(DbDocPlugin::new()));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
