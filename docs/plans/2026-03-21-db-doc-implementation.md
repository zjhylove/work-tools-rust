# db-doc 插件实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现数据库文档生成插件,支持 MySQL/PostgreSQL 连接,导出 Word/Markdown/PDF 格式文档。

**Architecture:** 纯 Rust 后端使用 sqlx 连接数据库、handlebars 模板渲染、quick-xml+zip 生成 DOCX、printpdf 生成 PDF。前端 React 组件管理连接配置、表选择、导出设置。

**Tech Stack:** Rust (sqlx, handlebars, quick-xml, zip, printpdf, aes), React 19, TypeScript

---

## Task 1: 项目初始化

**Files:**
- Create: `plugins/db-doc/Cargo.toml`
- Create: `plugins/db-doc/manifest.json`
- Create: `plugins/db-doc/src/lib.rs` (骨架)

**Step 1: 创建目录结构**

```bash
mkdir -p plugins/db-doc/src/{models,database,exporter,templates}
```

**Step 2: 创建 Cargo.toml**

```toml
[package]
name = "db-doc"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
# 插件 API
worktools-plugin-api = { path = "../../shared/plugin-api" }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 错误处理
anyhow = "1.0"

# 数据库驱动 (编译时检查)
sqlx = { version = "0.7", features = ["mysql", "postgres", "runtime-tokio-rustls", "chrono"] }

# 模板引擎
handlebars = "5.1"

# DOCX 生成
quick-xml = "0.31"
zip = "0.6"

# PDF 生成
printpdf = "0.6"

# 加密 (复用密码管理器方案)
aes = "0.8"
sha2 = "0.10"
hex = "0.4"

# 工具库
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4"] }
tokio = { version = "1.0", features = ["rt-multi-thread"] }
async-trait = "0.1"
once_cell = "1.19"
tracing = "0.1"
```

**Step 3: 创建 manifest.json**

```json
{
  "id": "db-doc",
  "name": "数据库文档",
  "description": "连接数据库,自动生成表结构文档 (Word/Markdown/PDF)",
  "version": "1.0.0",
  "icon": "📊",
  "author": "Work Tools Team",
  "homepage": "https://github.com/worktools/db-doc",
  "files": {
    "macos": "libdb_doc.dylib",
    "linux": "libdb_doc.so",
    "windows": "db_doc.dll"
  },
  "assets": {
    "entry": "index.html"
  },
  "permissions": [
    "filesystem",
    "network"
  ]
}
```

**Step 4: 创建 src/lib.rs 骨架**

```rust
use anyhow::Result;
use serde_json::Value;
use worktools_plugin_api::Plugin;

pub mod models;
pub mod database;
pub mod exporter;
pub mod storage;
pub mod crypto;

/// 数据库文档生成插件
pub struct DbDocPlugin;

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
        match method {
            _ => Err(format!("未知方法: {}", method).into()),
        }
    }
}

/// 插件工厂函数
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(DbDocPlugin));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
```

**Step 5: 添加到 workspace**

修改 `Cargo.toml` (根目录),在 `[workspace.members]` 中添加:
```toml
"plugins/db-doc",
```

**Step 6: 验证编译**

```bash
cargo build -p db-doc 2>&1
```

Expected: 编译成功 (可能有 unused warnings)

**Step 7: Commit**

```bash
git add plugins/db-doc/Cargo.toml plugins/db-doc/manifest.json plugins/db-doc/src/lib.rs Cargo.toml
git commit -m "feat(db-doc): initialize plugin project structure"
```

---

## Task 2: 数据模型定义

**Files:**
- Create: `plugins/db-doc/src/models/mod.rs`
- Create: `plugins/db-doc/src/models/column.rs`
- Create: `plugins/db-doc/src/models/table.rs`
- Create: `plugins/db-doc/src/models/connection.rs`

**Step 1: 创建 models/mod.rs**

```rust
mod column;
mod table;
mod connection;

pub use column::*;
pub use table::*;
pub use connection::*;
```

**Step 2: 创建 models/column.rs**

```rust
use serde::{Deserialize, Serialize};

/// 列信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    /// 字段名
    pub name: String,
    /// 数据类型 (VARCHAR, INT...)
    pub data_type: String,
    /// 最大长度
    pub max_length: Option<u64>,
    /// 是否允许 NULL
    pub is_nullable: bool,
    /// 是否主键
    pub is_primary_key: bool,
    /// 默认值
    pub default_value: Option<String>,
    /// 字段注释
    pub comment: Option<String>,
    /// 列位置 (从 1 开始)
    pub position: u32,
}

impl ColumnInfo {
    /// 创建新的列信息
    pub fn new(name: impl Into<String>, data_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            max_length: None,
            is_nullable: true,
            is_primary_key: false,
            default_value: None,
            comment: None,
            position: 0,
        }
    }
}
```

**Step 3: 创建 models/table.rs**

```rust
use serde::{Deserialize, Serialize};
use super::ColumnInfo;

/// 索引信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    /// 索引名
    pub name: String,
    /// 索引列
    pub columns: Vec<String>,
    /// 是否唯一索引
    pub is_unique: bool,
    /// 是否主键索引
    pub is_primary: bool,
}

impl IndexInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            is_unique: false,
            is_primary: false,
        }
    }
}

/// 表信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// 表名
    pub name: String,
    /// 所属 schema/数据库
    pub schema: String,
    /// 表注释
    pub comment: Option<String>,
    /// 所有列
    pub columns: Vec<ColumnInfo>,
    /// 索引信息
    pub indexes: Vec<IndexInfo>,
}

impl TableInfo {
    pub fn new(name: impl Into<String>, schema: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schema: schema.into(),
            comment: None,
            columns: Vec::new(),
            indexes: Vec::new(),
        }
    }

    /// 获取主键列
    pub fn primary_key_columns(&self) -> Vec<&ColumnInfo> {
        self.columns
            .iter()
            .filter(|c| c.is_primary_key)
            .collect()
    }
}
```

**Step 4: 创建 models/connection.rs**

```rust
use serde::{Deserialize, Serialize};

/// 数据库类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    MySQL,
    PostgreSQL,
}

impl DatabaseType {
    pub fn default_port(&self) -> u16 {
        match self {
            DatabaseType::MySQL => 3306,
            DatabaseType::PostgreSQL => 5432,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseType::MySQL => "mysql",
            DatabaseType::PostgreSQL => "postgresql",
        }
    }
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// 配置 ID
    pub id: String,
    /// 配置名称 (如 "生产环境")
    pub name: String,
    /// 数据库类型
    pub db_type: DatabaseType,
    /// 主机地址
    pub host: String,
    /// 端口
    pub port: u16,
    /// 数据库名
    pub database: String,
    /// 用户名
    pub username: String,
    /// 密码 (加密存储)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// 创建时间 (Unix 时间戳)
    pub created_at: u64,
    /// 最后使用时间
    pub last_used: Option<u64>,
}

impl ConnectionConfig {
    pub fn new(name: impl Into<String>, db_type: DatabaseType) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            db_type,
            host: "localhost".to_string(),
            port: db_type.default_port(),
            database: String::new(),
            username: "root".to_string(),
            password: None,
            created_at: now,
            last_used: None,
        }
    }

    /// 构建 JDBC URL (用于 sqlx)
    pub fn to_connection_string(&self) -> String {
        match self.db_type {
            DatabaseType::MySQL => {
                format!(
                    "mysql://{}:{}@{}:{}/{}",
                    self.username,
                    self.password.as_deref().unwrap_or(""),
                    self.host,
                    self.port,
                    self.database
                )
            }
            DatabaseType::PostgreSQL => {
                format!(
                    "postgres://{}:{}@{}:{}/{}",
                    self.username,
                    self.password.as_deref().unwrap_or(""),
                    self.host,
                    self.port,
                    self.database
                )
            }
        }
    }
}

/// 导出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Word,
    Markdown,
    Pdf,
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Word => "docx",
            ExportFormat::Markdown => "md",
            ExportFormat::Pdf => "pdf",
        }
    }
}

/// 模板风格
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TemplateStyle {
    Simple,      // 简洁版
    Detailed,    // 详细版
    Enterprise,  // 企业版
}

impl Default for TemplateStyle {
    fn default() -> Self {
        Self::Detailed
    }
}

/// 导出配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// 连接配置 ID
    pub connection_id: String,
    /// 选中的表
    pub tables: Vec<String>,
    /// 输出目录
    pub output_dir: String,
    /// 导出格式
    pub format: ExportFormat,
    /// 模板风格
    pub template: TemplateStyle,
}

/// 导出历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportHistory {
    /// 记录 ID
    pub id: String,
    /// 连接配置名称
    pub connection_name: String,
    /// 导出的表
    pub tables: Vec<String>,
    /// 导出格式
    pub format: ExportFormat,
    /// 模板风格
    pub template: TemplateStyle,
    /// 输出路径
    pub output_path: String,
    /// 导出时间 (ISO 8601)
    pub exported_at: String,
}
```

**Step 5: 更新 models/mod.rs 导出**

```rust
mod column;
mod table;
mod connection;

pub use column::ColumnInfo;
pub use table::{TableInfo, IndexInfo};
pub use connection::{
    DatabaseType, ConnectionConfig, ExportFormat, TemplateStyle, ExportConfig, ExportHistory
};
```

**Step 6: 验证编译**

```bash
cargo build -p db-doc 2>&1
```

Expected: 编译成功

**Step 7: Commit**

```bash
git add plugins/db-doc/src/models/
git commit -m "feat(db-doc): add data models (ColumnInfo, TableInfo, ConnectionConfig)"
```

---

## Task 3: 密码加密模块

**Files:**
- Create: `plugins/db-doc/src/crypto.rs`

**Step 1: 创建 crypto.rs (复用密码管理器方案)**

```rust
use aes::Aes256;
use aes::cipher::{KeyInit, BlockEncrypt, BlockDecrypt, generic_array::GenericArray};
use sha2::{Sha256, Digest};
use anyhow::Result;

/// 密码加密器 (AES-256 ECB + PKCS7)
pub struct PasswordEncryptor {
    cipher: Aes256,
}

impl PasswordEncryptor {
    /// 基于应用标识符生成固定密钥
    fn get_internal_key() -> [u8; 32] {
        let app_secret = "WorkToolsDbDocPlugin2024InternalKey";
        let mut hasher = Sha256::new();
        hasher.update(app_secret.as_bytes());
        hasher.update(b"SALT_DB_DOC_ENCRYPTION");
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result[..32]);
        key
    }

    /// 创建加密器实例
    pub fn new() -> Self {
        let key = Self::get_internal_key();
        let cipher = Aes256::new(&GenericArray::from(key));
        Self { cipher }
    }

    /// 加密密码
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let plaintext_bytes = plaintext.as_bytes();
        let block_size = 16;

        // PKCS7 填充
        let padding_len = if plaintext_bytes.len().is_multiple_of(block_size) {
            block_size
        } else {
            block_size - (plaintext_bytes.len() % block_size)
        };

        let mut padded_data = plaintext_bytes.to_vec();
        for _ in 0..padding_len {
            padded_data.push(padding_len as u8);
        }

        // 分块加密
        let mut encrypted_data = Vec::new();
        for chunk in padded_data.chunks(16) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            self.cipher.encrypt_block(GenericArray::from_mut_slice(&mut block));
            encrypted_data.extend_from_slice(&block);
        }

        Ok(hex::encode(encrypted_data))
    }

    /// 解密密码
    pub fn decrypt(&self, ciphertext: &str) -> Result<String> {
        let encrypted_data = hex::decode(ciphertext)?;

        if encrypted_data.len() % 16 != 0 {
            return Err(anyhow::anyhow!("密文长度无效"));
        }

        let mut decrypted_data = Vec::new();
        for chunk in encrypted_data.chunks(16) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            self.cipher.decrypt_block(GenericArray::from_mut_slice(&mut block));
            decrypted_data.extend_from_slice(&block);
        }

        // 移除 PKCS7 填充
        if decrypted_data.is_empty() {
            return Err(anyhow::anyhow!("解密结果为空"));
        }

        let padding_len = decrypted_data[decrypted_data.len() - 1] as usize;
        if padding_len > 16 || padding_len == 0 {
            return Err(anyhow::anyhow!("填充长度无效"));
        }

        let padding_start = decrypted_data.len() - padding_len;
        for byte in &decrypted_data[padding_start..] {
            if *byte != padding_len as u8 {
                return Err(anyhow::anyhow!("填充数据无效"));
            }
        }

        decrypted_data.truncate(decrypted_data.len() - padding_len);
        String::from_utf8(decrypted_data)
            .map_err(|e| anyhow::anyhow!("UTF-8 解码失败: {}", e))
    }
}

impl Default for PasswordEncryptor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let encryptor = PasswordEncryptor::new();
        let original = "my_secret_password_123";

        let encrypted = encryptor.encrypt(original).unwrap();
        assert_ne!(encrypted, original);

        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_encrypt_different_results() {
        let encryptor = PasswordEncryptor::new();

        // 相同密码加密结果相同 (ECB 模式特性)
        let encrypted1 = encryptor.encrypt("password").unwrap();
        let encrypted2 = encryptor.encrypt("password").unwrap();
        assert_eq!(encrypted1, encrypted2);
    }
}
```

**Step 2: 运行测试**

```bash
cargo test -p db-doc crypto 2>&1
```

Expected: 2 tests passed

**Step 3: Commit**

```bash
git add plugins/db-doc/src/crypto.rs
git commit -m "feat(db-doc): add password encryption module (AES-256)"
```

---

## Task 4: 数据存储模块

**Files:**
- Create: `plugins/db-doc/src/storage.rs`

**Step 1: 创建 storage.rs**

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use worktools_plugin_api::storage::PluginStorage;
use crate::models::{ConnectionConfig, ExportHistory};
use crate::crypto::PasswordEncryptor;
use once_cell::sync::Lazy;

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
```

**Step 2: 验证编译**

```bash
cargo build -p db-doc 2>&1
```

Expected: 编译成功

**Step 3: Commit**

```bash
git add plugins/db-doc/src/storage.rs
git commit -m "feat(db-doc): add storage module with encrypted password support"
```

---

## Task 5: 数据库提取器 Trait

**Files:**
- Create: `plugins/db-doc/src/database/mod.rs`
- Create: `plugins/db-doc/src/database/extractor.rs`

**Step 1: 创建 database/mod.rs**

```rust
mod extractor;
mod mysql;

pub use extractor::*;
pub use mysql::MySqlExtractor;
```

**Step 2: 创建 database/extractor.rs**

```rust
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
```

**Step 3: 更新 lib.rs 添加 async 支持**

在 `lib.rs` 顶部添加:
```rust
pub mod models;
pub mod database;
pub mod exporter;
pub mod storage;
pub mod crypto;

use std::sync::Arc;
use tokio::runtime::Runtime;
```

**Step 4: 验证编译**

```bash
cargo build -p db-doc 2>&1
```

Expected: 编译成功

**Step 5: Commit**

```bash
git add plugins/db-doc/src/database/
git commit -m "feat(db-doc): add DatabaseExtractor trait"
```

---

## Task 6: MySQL 元数据提取器

**Files:**
- Create: `plugins/db-doc/src/database/mysql.rs`

**Step 1: 创建 database/mysql.rs**

```rust
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
```

**Step 2: 验证编译**

```bash
cargo build -p db-doc 2>&1
```

Expected: 编译成功 (可能需要下载 sqlx 依赖)

**Step 3: Commit**

```bash
git add plugins/db-doc/src/database/mysql.rs
git commit -m "feat(db-doc): add MySQL metadata extractor"
```

---

## Task 7: Markdown 导出器

**Files:**
- Create: `plugins/db-doc/src/exporter/mod.rs`
- Create: `plugins/db-doc/src/exporter/markdown.rs`

**Step 1: 创建 exporter/mod.rs**

```rust
mod markdown;

pub use markdown::MarkdownExporter;
```

**Step 2: 创建 exporter/markdown.rs**

```rust
use anyhow::Result;
use std::path::Path;
use crate::models::{TableInfo, TemplateStyle};

/// Markdown 文档导出器
pub struct MarkdownExporter {
    template_style: TemplateStyle,
}

impl MarkdownExporter {
    pub fn new(template_style: TemplateStyle) -> Self {
        Self { template_style }
    }

    /// 导出单张表到 Markdown 文件
    pub fn export_table(&self, table: &TableInfo, output_path: &Path) -> Result<()> {
        let content = self.render_table(table);
        std::fs::write(output_path, content)?;
        Ok(())
    }

    /// 导出多张表到一个 Markdown 文件
    pub fn export_tables(&self, tables: &[TableInfo], output_path: &Path) -> Result<()> {
        let mut content = String::new();
        content.push_str("# 数据库文档\n\n");

        for (i, table) in tables.iter().enumerate() {
            if i > 0 {
                content.push_str("\n---\n\n");
            }
            content.push_str(&self.render_table(table));
        }

        std::fs::write(output_path, content)?;
        Ok(())
    }

    /// 渲染单张表
    fn render_table(&self, table: &TableInfo) -> String {
        match self.template_style {
            TemplateStyle::Simple => self.render_simple(table),
            TemplateStyle::Detailed | TemplateStyle::Enterprise => self.render_detailed(table),
        }
    }

    /// 简洁模板
    fn render_simple(&self, table: &TableInfo) -> String {
        let mut md = String::new();

        md.push_str(&format!("## {}\n\n", table.name));

        if let Some(ref comment) = table.comment {
            if !comment.is_empty() {
                md.push_str(&format!("> {}\n\n", comment));
            }
        }

        // 表格头
        md.push_str("| 字段 | 类型 | 说明 |\n");
        md.push_str("|------|------|------|\n");

        for col in &table.columns {
            let comment = col.comment.as_deref().unwrap_or("-");
            md.push_str(&format!(
                "| {} | {} | {} |\n",
                col.name,
                self.format_data_type(col),
                comment
            ));
        }

        md
    }

    /// 详细模板
    fn render_detailed(&self, table: &TableInfo) -> String {
        let mut md = String::new();

        md.push_str(&format!("## {}\n\n", table.name));

        // 表信息
        if let Some(ref comment) = table.comment {
            if !comment.is_empty() {
                md.push_str(&format!("**表注释**: {}\n\n", comment));
            }
        }

        md.push_str(&format!("**所属库**: {}\n\n", table.schema));

        // 字段表格
        md.push_str("### 字段列表\n\n");
        md.push_str("| 字段 | 类型 | 可空 | 主键 | 默认值 | 说明 |\n");
        md.push_str("|------|------|------|------|--------|------|\n");

        for col in &table.columns {
            let nullable = if col.is_nullable { "是" } else { "否" };
            let pk = if col.is_primary_key { "是" } else { "否" };
            let default = col.default_value.as_deref().unwrap_or("-");
            let comment = col.comment.as_deref().unwrap_or("-");

            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                col.name,
                self.format_data_type(col),
                nullable,
                pk,
                default,
                comment
            ));
        }

        // 索引信息
        if !table.indexes.is_empty() {
            md.push_str("\n### 索引列表\n\n");
            md.push_str("| 索引名 | 列 | 唯一 | 类型 |\n");
            md.push_str("|--------|-----|------|------|\n");

            for idx in &table.indexes {
                let unique = if idx.is_unique { "是" } else { "否" };
                let idx_type = if idx.is_primary { "主键" } else { "普通" };
                let columns = idx.columns.join(", ");

                md.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    idx.name, columns, unique, idx_type
                ));
            }
        }

        md
    }

    /// 格式化数据类型
    fn format_data_type(&self, col: &crate::models::ColumnInfo) -> String {
        if let Some(len) = col.max_length {
            format!("{}({})", col.data_type.to_uppercase(), len)
        } else {
            col.data_type.to_uppercase()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ColumnInfo;

    fn create_test_table() -> TableInfo {
        let mut table = TableInfo::new("users", "mydb");
        table.comment = Some("用户表".to_string());
        table.columns = vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "bigint".to_string(),
                max_length: None,
                is_nullable: false,
                is_primary_key: true,
                default_value: None,
                comment: Some("用户ID".to_string()),
                position: 1,
            },
            ColumnInfo {
                name: "username".to_string(),
                data_type: "varchar".to_string(),
                max_length: Some(255),
                is_nullable: false,
                is_primary_key: false,
                default_value: None,
                comment: Some("用户名".to_string()),
                position: 2,
            },
        ];
        table
    }

    #[test]
    fn test_render_simple() {
        let exporter = MarkdownExporter::new(TemplateStyle::Simple);
        let table = create_test_table();
        let md = exporter.render_table(&table);

        assert!(md.contains("## users"));
        assert!(md.contains("用户表"));
        assert!(md.contains("| id |"));
        assert!(md.contains("| username |"));
    }

    #[test]
    fn test_render_detailed() {
        let exporter = MarkdownExporter::new(TemplateStyle::Detailed);
        let table = create_test_table();
        let md = exporter.render_table(&table);

        assert!(md.contains("### 字段列表"));
        assert!(md.contains("BIGINT"));
        assert!(md.contains("VARCHAR(255)"));
    }
}
```

**Step 3: 运行测试**

```bash
cargo test -p db-doc exporter 2>&1
```

Expected: 2 tests passed

**Step 4: Commit**

```bash
git add plugins/db-doc/src/exporter/
git commit -m "feat(db-doc): add Markdown exporter with simple and detailed templates"
```

---

## Task 8: 插件 API 实现 - 连接管理

**Files:**
- Modify: `plugins/db-doc/src/lib.rs`

**Step 1: 更新 lib.rs 添加连接管理方法**

```rust
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
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
            "list_connections" => self.handle_list_connections(),
            "delete_connection" => self.handle_delete_connection(params),
            "test_connection" => self.handle_test_connection(params),

            // 表查询
            "list_tables" => self.handle_list_tables(params),

            // 导出
            "export_docs" => self.handle_export_docs(params),

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
```

**Step 2: 验证编译**

```bash
cargo build -p db-doc 2>&1
```

Expected: 编译成功

**Step 3: Commit**

```bash
git add plugins/db-doc/src/lib.rs
git commit -m "feat(db-doc): implement plugin API (connection management, table listing, export)"
```

---

## Task 9: 前端项目初始化

**Files:**
- Create: `plugins/db-doc/frontend/package.json`
- Create: `plugins/db-doc/frontend/tsconfig.json`
- Create: `plugins/db-doc/frontend/vite.config.ts`
- Create: `plugins/db-doc/frontend/index.html`
- Create: `plugins/db-doc/frontend/src/main.tsx`
- Create: `plugins/db-doc/frontend/src/App.tsx`

**Step 1: 创建 frontend/package.json**

```json
{
  "name": "db-doc-frontend",
  "private": true,
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build --outDir ../assets",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "^5.6.0",
    "vite": "^5.4.0"
  }
}
```

**Step 2: 创建 frontend/tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

**Step 3: 创建 frontend/tsconfig.node.json**

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true,
    "strict": true
  },
  "include": ["vite.config.ts"]
}
```

**Step 4: 创建 frontend/vite.config.ts**

```typescript
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: '../assets',
    emptyOutDir: true,
  },
})
```

**Step 5: 创建 frontend/index.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>数据库文档生成</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

**Step 6: 创建 frontend/src/main.tsx**

```typescript
import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
```

**Step 7: 创建 frontend/src/index.css**

```css
:root {
  font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
  line-height: 1.5;
  font-weight: 400;
  color: #213547;
  background-color: #ffffff;
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  min-height: 100vh;
  padding: 20px;
}

#root {
  max-width: 1200px;
  margin: 0 auto;
}
```

**Step 8: 创建 frontend/src/App.tsx (骨架)**

```typescript
import { useState } from 'react'
import './App.css'

// 声明 window.pluginAPI
declare global {
  interface Window {
    pluginAPI: {
      call: (method: string, params?: Record<string, unknown>) => Promise<unknown>
    }
  }
}

function App() {
  const [message, setMessage] = useState('数据库文档生成器')

  return (
    <div className="app">
      <h1>📊 {message}</h1>
      <p>前端框架已就绪,待实现完整 UI。</p>
    </div>
  )
}

export default App
```

**Step 9: 创建 frontend/src/App.css**

```css
.app {
  padding: 20px;
}

h1 {
  margin-bottom: 20px;
  color: #333;
}
```

**Step 10: 安装依赖并构建**

```bash
cd plugins/db-doc/frontend && npm install && npm run build
```

Expected: 构建成功,生成 assets/index.html, assets/main.js, assets/index.css

**Step 11: Commit**

```bash
git add plugins/db-doc/frontend/ plugins/db-doc/assets/
git commit -m "feat(db-doc): initialize React frontend with Vite"
```

---

## Task 10: 构建脚本与打包

**Files:**
- Modify: `scripts/build-plugins.sh`

**Step 1: 更新 build-plugins.sh 添加 db-doc**

在脚本的 PLUGINS 数组中添加 `db-doc`:

```bash
PLUGINS=("password-manager" "json-tools" "auth-plugin" "text-diff" "db-doc")
```

**Step 2: 测试插件编译**

```bash
cd plugins/db-doc && cargo build --release 2>&1 | tail -10
```

Expected: 编译成功,生成 `target/release/libdb_doc.dylib`

**Step 3: 测试打包**

```bash
cd plugins/db-doc
zip -r db-doc.wtplugin.zip manifest.json ../../target/release/libdb_doc.dylib assets/
```

**Step 4: Commit**

```bash
git add scripts/build-plugins.sh
git commit -m "feat(db-doc): add to build scripts"
```

---

## 执行选项

**计划已保存到 `docs/plans/2026-03-21-db-doc-implementation.md`。两种执行方式:**

**1. Subagent-Driven (当前会话)** - 我在当前会话中逐个任务派发子代理执行,每个任务完成后进行代码审查,快速迭代。

**2. Parallel Session (独立会话)** - 在新会话中打开 worktree 目录,使用 executing-plans 技能批量执行,有检查点。

**你选择哪种方式?**
