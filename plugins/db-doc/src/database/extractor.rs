use async_trait::async_trait;
use anyhow::Result;
use crate::models::{ConnectionConfig, TableInfo};

/// 数据库元数据提取器
#[async_trait]
pub trait DatabaseExtractor: Send + Sync {
    /// 测试连接是否可用
    async fn test_connection(&self, config: &ConnectionConfig) -> Result<bool>;

    /// 获取所有表名列表
    async fn list_tables(&self, config: &ConnectionConfig) -> Result<Vec<String>>;

    /// 获取指定表的完整信息 (列、索引、注释)
    async fn get_table_info(
        &self,
        config: &ConnectionConfig,
        table_name: &str,
    ) -> Result<TableInfo>;

    /// 批量获取多张表的信息
    async fn get_tables_info(
        &self,
        config: &ConnectionConfig,
        tables: &[String],
    ) -> Result<Vec<TableInfo>> {
        let mut results = Vec::new();
        for table in tables {
            match self.get_table_info(config, table).await {
                Ok(info) => results.push(info),
                Err(e) => {
                    tracing::warn!("获取表 {} 信息失败: {}", table, e);
                }
            }
        }
        Ok(results)
    }
}
