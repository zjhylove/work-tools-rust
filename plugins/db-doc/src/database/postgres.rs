use std::time::Duration;
use async_trait::async_trait;
use anyhow::Result;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{Row, Pool, Postgres};
use std::collections::{HashMap, HashSet};
use crate::models::{ConnectionConfig, TableInfo, ColumnInfo, IndexInfo};
use super::DatabaseExtractor;

/// PostgreSQL 元数据提取器
pub struct PostgresExtractor;

impl PostgresExtractor {
    /// 创建数据库连接池
    async fn create_pool(config: &ConnectionConfig) -> Result<Pool<Postgres>> {
        let url = config.to_connection_string();
        let pool = tokio::time::timeout(
            Duration::from_secs(3),
            PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(3))
                .connect(&url),
        )
        .await??;
        Ok(pool)
    }

    /// 获取 PostgreSQL schema 名称
    /// 用户在 "数据库名" 字段填的可能是实际数据库名，
    /// PostgreSQL 中表属于某个 schema（默认 public），这里用 current_schema() 获取
    async fn get_schema(pool: &Pool<Postgres>) -> Result<String> {
        let row: PgRow = sqlx::query("SELECT current_schema()")
            .fetch_one(pool)
            .await?;
        Ok(row.try_get::<String, _>(0).unwrap_or_else(|_| "public".to_string()))
    }

    /// 查询表注释 (PostgreSQL 用 pg_catalog.obj_description)
    async fn get_table_comment(
        &self,
        pool: &Pool<Postgres>,
        schema: &str,
        table_name: &str,
    ) -> Result<Option<String>> {
        let sql = r#"
            SELECT obj_description(c.oid) AS table_comment
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = $1 AND c.relname = $2
        "#;

        let row: Option<PgRow> = sqlx::query(sql)
            .bind(schema)
            .bind(table_name)
            .fetch_optional(pool)
            .await?;

        Ok(row.and_then(|r| r.try_get::<Option<String>, _>("table_comment").ok().flatten()))
    }

    /// 查询列信息
    async fn get_columns(
        &self,
        pool: &Pool<Postgres>,
        schema: &str,
        table_name: &str,
    ) -> Result<Vec<ColumnInfo>> {
        let sql = r#"
            SELECT
                c.column_name,
                c.data_type,
                c.character_maximum_length,
                c.is_nullable,
                c.column_default,
                c.ordinal_position,
                col_description(
                    (SELECT oid FROM pg_class WHERE relname = c.table_name
                     AND relnamespace = (SELECT oid FROM pg_namespace WHERE nspname = c.table_schema)),
                    c.ordinal_position::int
                ) AS column_comment
            FROM information_schema.columns c
            WHERE c.table_schema = $1 AND c.table_name = $2
            ORDER BY c.ordinal_position
        "#;

        let rows = sqlx::query(sql)
            .bind(schema)
            .bind(table_name)
            .fetch_all(pool)
            .await?;

        let columns = rows
            .into_iter()
            .map(|row| {
                let nullable: String = row.try_get("is_nullable").unwrap_or_default();

                ColumnInfo {
                    name: row.try_get("column_name").unwrap_or_default(),
                    data_type: row.try_get("data_type").unwrap_or_default(),
                    max_length: row.try_get::<i32, _>("character_maximum_length").ok().map(|v| v as u64),
                    is_nullable: nullable == "YES",
                    is_primary_key: false,
                    default_value: row.try_get("column_default").ok(),
                    comment: row.try_get::<Option<String>, _>("column_comment").ok().flatten(),
                    position: row.try_get::<i32, _>("ordinal_position").unwrap_or(0) as u32,
                }
            })
            .collect();

        Ok(columns)
    }

    /// 查询索引信息 (包括主键)
    async fn get_indexes(
        &self,
        pool: &Pool<Postgres>,
        schema: &str,
        table_name: &str,
    ) -> Result<Vec<IndexInfo>> {
        let sql = r#"
            SELECT
                i.relname AS index_name,
                a.attname AS column_name,
                ix.indisunique AS is_unique,
                ix.indisprimary AS is_primary
            FROM pg_index ix
            JOIN pg_class t ON t.oid = ix.indrelid
            JOIN pg_class i ON i.oid = ix.indexrelid
            JOIN pg_namespace n ON n.oid = t.relnamespace
            JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
            WHERE n.nspname = $1 AND t.relname = $2
            ORDER BY i.relname, a.attnum
        "#;

        let rows = sqlx::query(sql)
            .bind(schema)
            .bind(table_name)
            .fetch_all(pool)
            .await?;

        let mut index_map: HashMap<String, IndexInfo> = HashMap::new();

        for row in rows {
            let index_name: String = row.try_get("index_name").unwrap_or_default();
            let column_name: String = row.try_get("column_name").unwrap_or_default();
            let is_unique: bool = row.try_get("is_unique").unwrap_or(false);
            let is_primary: bool = row.try_get("is_primary").unwrap_or(false);

            let entry = index_map.entry(index_name.clone()).or_insert(IndexInfo {
                name: index_name,
                columns: Vec::new(),
                is_unique,
                is_primary,
            });

            entry.columns.push(column_name);
        }

        Ok(index_map.into_values().collect())
    }

    /// 从索引结果中提取主键列名
    fn collect_pk_columns(indexes: &[IndexInfo]) -> HashSet<&str> {
        indexes
            .iter()
            .filter(|idx| idx.is_primary)
            .flat_map(|idx| idx.columns.iter().map(|s| s.as_str()))
            .collect()
    }
}

#[async_trait]
impl DatabaseExtractor for PostgresExtractor {
    async fn test_connection(&self, config: &ConnectionConfig) -> Result<bool> {
        let pool = Self::create_pool(config).await?;
        sqlx::query("SELECT 1").fetch_one(&pool).await?;
        Ok(true)
    }

    async fn list_tables(&self, config: &ConnectionConfig) -> Result<Vec<String>> {
        let pool = Self::create_pool(config).await?;
        let schema = Self::get_schema(&pool).await?;

        let sql = r#"
            SELECT table_name
            FROM information_schema.tables
            WHERE table_schema = $1 AND table_type = 'BASE TABLE'
            ORDER BY table_name
        "#;

        let rows = sqlx::query(sql)
            .bind(&schema)
            .fetch_all(&pool)
            .await?;

        let tables = rows
            .into_iter()
            .filter_map(|row| row.try_get("table_name").ok())
            .collect();

        Ok(tables)
    }

    async fn get_table_info(
        &self,
        config: &ConnectionConfig,
        table_name: &str,
    ) -> Result<TableInfo> {
        let pool = Self::create_pool(config).await?;
        let schema = Self::get_schema(&pool).await?;

        let comment = self.get_table_comment(&pool, &schema, table_name).await?;
        let mut columns = self.get_columns(&pool, &schema, table_name).await?;
        let indexes = self.get_indexes(&pool, &schema, table_name).await?;

        let pk_columns = Self::collect_pk_columns(&indexes);
        for col in columns.iter_mut() {
            if pk_columns.contains(col.name.as_str()) {
                col.is_primary_key = true;
            }
        }

        Ok(TableInfo {
            name: table_name.to_string(),
            schema,
            comment,
            columns,
            indexes,
        })
    }
}
