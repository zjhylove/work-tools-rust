use async_trait::async_trait;
use anyhow::Result;
use sqlx::mysql::{MySqlPoolOptions, MySqlRow};
use sqlx::{Row, Pool, MySql};
use crate::models::{ConnectionConfig, TableInfo, ColumnInfo, IndexInfo};
use super::DatabaseExtractor;

/// MySQL 元数据提取器
pub struct MySqlExtractor;

impl MySqlExtractor {
    /// 创建数据库连接池
    async fn create_pool(config: &ConnectionConfig) -> Result<Pool<MySql>> {
        let url = config.to_connection_string();
        let pool = MySqlPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        Ok(pool)
    }

    /// 查询表注释
    async fn get_table_comment(
        &self,
        pool: &Pool<MySql>,
        schema: &str,
        table_name: &str,
    ) -> Result<Option<String>> {
        let sql = r#"
            SELECT TABLE_COMMENT
            FROM information_schema.TABLES
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
        "#;

        let row: Option<MySqlRow> = sqlx::query(sql)
            .bind(schema)
            .bind(table_name)
            .fetch_optional(pool)
            .await?;

        Ok(row.and_then(|r| r.try_get::<String, _>("TABLE_COMMENT").ok()))
    }

    /// 查询列信息
    async fn get_columns(
        &self,
        pool: &Pool<MySql>,
        schema: &str,
        table_name: &str,
    ) -> Result<Vec<ColumnInfo>> {
        let sql = r#"
            SELECT
                COLUMN_NAME,
                DATA_TYPE,
                CHARACTER_MAXIMUM_LENGTH,
                IS_NULLABLE,
                COLUMN_KEY,
                COLUMN_DEFAULT,
                COLUMN_COMMENT,
                ORDINAL_POSITION
            FROM information_schema.COLUMNS
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
            ORDER BY ORDINAL_POSITION
        "#;

        let rows = sqlx::query(sql)
            .bind(schema)
            .bind(table_name)
            .fetch_all(pool)
            .await?;

        let columns = rows
            .into_iter()
            .map(|row| {
                let nullable: String = row.try_get("IS_NULLABLE").unwrap_or_default();
                let column_key: String = row.try_get("COLUMN_KEY").unwrap_or_default();

                ColumnInfo {
                    name: row.try_get("COLUMN_NAME").unwrap_or_default(),
                    data_type: row.try_get("DATA_TYPE").unwrap_or_default(),
                    max_length: row.try_get("CHARACTER_MAXIMUM_LENGTH").ok(),
                    is_nullable: nullable == "YES",
                    is_primary_key: column_key == "PRI",
                    default_value: row.try_get("COLUMN_DEFAULT").ok(),
                    comment: row.try_get("COLUMN_COMMENT").ok(),
                    position: row.try_get("ORDINAL_POSITION").unwrap_or(0),
                }
            })
            .collect();

        Ok(columns)
    }

    /// 查询索引信息
    async fn get_indexes(
        &self,
        pool: &Pool<MySql>,
        schema: &str,
        table_name: &str,
    ) -> Result<Vec<IndexInfo>> {
        let sql = r#"
            SELECT
                INDEX_NAME,
                COLUMN_NAME,
                NON_UNIQUE
            FROM information_schema.STATISTICS
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
            ORDER BY INDEX_NAME, SEQ_IN_INDEX
        "#;

        let rows = sqlx::query(sql)
            .bind(schema)
            .bind(table_name)
            .fetch_all(pool)
            .await?;

        // 按索引名分组
        let mut index_map: std::collections::HashMap<String, IndexInfo> =
            std::collections::HashMap::new();

        for row in rows {
            let index_name: String = row.try_get("INDEX_NAME").unwrap_or_default();
            let column_name: String = row.try_get("COLUMN_NAME").unwrap_or_default();
            let non_unique: i64 = row.try_get("NON_UNIQUE").unwrap_or(1);

            let is_primary = index_name == "PRIMARY";

            let entry = index_map.entry(index_name.clone()).or_insert(IndexInfo {
                name: index_name,
                columns: Vec::new(),
                is_unique: non_unique == 0,
                is_primary,
            });

            entry.columns.push(column_name);
        }

        Ok(index_map.into_values().collect())
    }
}

#[async_trait]
impl DatabaseExtractor for MySqlExtractor {
    async fn test_connection(&self, config: &ConnectionConfig) -> Result<bool> {
        let pool = Self::create_pool(config).await?;
        sqlx::query("SELECT 1").fetch_one(&pool).await?;
        Ok(true)
    }

    async fn list_tables(&self, config: &ConnectionConfig) -> Result<Vec<String>> {
        let pool = Self::create_pool(config).await?;

        let sql = r#"
            SELECT TABLE_NAME
            FROM information_schema.TABLES
            WHERE TABLE_SCHEMA = ? AND TABLE_TYPE = 'BASE TABLE'
            ORDER BY TABLE_NAME
        "#;

        let rows = sqlx::query(sql)
            .bind(&config.database)
            .fetch_all(&pool)
            .await?;

        let tables = rows
            .into_iter()
            .filter_map(|row| row.try_get("TABLE_NAME").ok())
            .collect();

        Ok(tables)
    }

    async fn get_table_info(
        &self,
        config: &ConnectionConfig,
        table_name: &str,
    ) -> Result<TableInfo> {
        let pool = Self::create_pool(config).await?;
        let schema = &config.database;

        let comment = self.get_table_comment(&pool, schema, table_name).await?;
        let columns = self.get_columns(&pool, schema, table_name).await?;
        let indexes = self.get_indexes(&pool, schema, table_name).await?;

        Ok(TableInfo {
            name: table_name.to_string(),
            schema: schema.to_string(),
            comment,
            columns,
            indexes,
        })
    }
}
