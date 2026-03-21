# 数据库文档生成插件设计文档

> 创建日期: 2026-03-21
> 基于 Java 版本 db-doc-plugin 复刻优化

## 概述

将 Java 版本的数据库文档生成工具移植到 Rust/Tauri 平台,作为 Work Tools 的插件实现。

## 核心决策

| 决策项 | 方案 |
|--------|------|
| 数据库支持 | MySQL + PostgreSQL |
| 输出格式 | Word (DOCX) + Markdown + PDF |
| 连接管理 | 多配置管理,密码 AES 加密存储 |
| 表选择 | 从数据库加载表列表,支持搜索多选 |
| 模板定制 | 3 套预设模板 (简洁/详细/企业) |
| 技术方案 | 纯 Rust 后端,无外部依赖 |
| DOCX 生成 | Handlebars + OOXML 模板化 |

## 项目结构

```
plugins/db-doc/
├── Cargo.toml                    # Rust 依赖配置
├── manifest.json                 # 插件元数据
├── src/
│   ├── lib.rs                    # 插件入口 + Plugin trait 实现
│   ├── models/
│   │   ├── mod.rs
│   │   ├── column.rs             # 列信息模型
│   │   ├── table.rs              # 表信息模型
│   │   └── connection.rs         # 连接配置模型
│   ├── database/
│   │   ├── mod.rs
│   │   ├── extractor.rs          # DatabaseExtractor trait
│   │   ├── mysql.rs              # MySQL 元数据提取
│   │   └── postgresql.rs         # PostgreSQL 元数据提取
│   ├── exporter/
│   │   ├── mod.rs
│   │   ├── markdown.rs           # Markdown 导出器
│   │   ├── docx.rs               # Word 导出器
│   │   └── pdf.rs                # PDF 导出器
│   ├── storage.rs                # 数据存储 (连接配置、历史)
│   ├── crypto.rs                 # 密码加密 (复用密码管理器方案)
│   └── templates/
│       └── embedded/             # 内嵌模板文件
│           ├── simple/
│           ├── detailed/
│           └── enterprise/
├── assets/                       # 前端资源 (打包后)
│   ├── index.html
│   ├── main.js
│   └── styles.css
└── frontend/                     # React 前端源码
    ├── src/
    │   ├── App.tsx
    │   ├── components/
    │   │   ├── ConnectionManager.tsx
    │   │   ├── TableSelector.tsx
    │   │   ├── ExportSettings.tsx
    │   │   └── ExportProgress.tsx
    │   └── utils/
    └── package.json
```

## 核心数据模型

### 列信息 (ColumnInfo)

```rust
pub struct ColumnInfo {
    pub name: String,           // 字段名
    pub data_type: String,      // 数据类型 (VARCHAR, INT...)
    pub max_length: Option<u64>,// 最大长度
    pub is_nullable: bool,      // 是否允许 NULL
    pub is_primary_key: bool,   // 是否主键
    pub default_value: Option<String>, // 默认值
    pub comment: Option<String>,// 字段注释
    pub position: u32,          // 列位置
}
```

### 表信息 (TableInfo)

```rust
pub struct TableInfo {
    pub name: String,           // 表名
    pub schema: String,         // 所属 schema/数据库
    pub comment: Option<String>,// 表注释
    pub columns: Vec<ColumnInfo>,// 所有列
    pub indexes: Vec<IndexInfo>,// 索引信息
}

pub struct IndexInfo {
    pub name: String,           // 索引名
    pub columns: Vec<String>,   // 索引列
    pub is_unique: bool,        // 是否唯一索引
    pub is_primary: bool,       // 是否主键索引
}
```

### 连接配置 (ConnectionConfig)

```rust
pub struct ConnectionConfig {
    pub id: String,             // 配置 ID
    pub name: String,           // 配置名称 (如 "生产环境")
    pub db_type: DatabaseType,  // MySQL / PostgreSQL
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,       // 加密存储
    pub created_at: u64,
    pub last_used: Option<u64>,
}

pub enum DatabaseType {
    MySQL,
    PostgreSQL,
}
```

### 导出配置 (ExportConfig)

```rust
pub struct ExportConfig {
    pub connection_id: String,  // 使用的连接
    pub tables: Vec<String>,    // 选中的表
    pub output_dir: String,     // 输出目录
    pub format: ExportFormat,   // 导出格式
    pub template: TemplateStyle,// 模板风格
}

pub enum ExportFormat {
    Word,
    Markdown,
    Pdf,
}

pub enum TemplateStyle {
    Simple,      // 简洁版: 仅表名+字段列表
    Detailed,    // 详细版: 包含索引、注释、默认值
    Enterprise,  // 企业版: 带封面、目录、版本信息
}
```

## 数据库元数据提取

### Extractor Trait

```rust
use async_trait::async_trait;

#[async_trait]
pub trait DatabaseExtractor: Send + Sync {
    /// 测试连接是否可用
    async fn test_connection(&self, config: &ConnectionConfig) -> Result<bool>;

    /// 获取所有表名列表
    async fn list_tables(&self, config: &ConnectionConfig) -> Result<Vec<String>>;

    /// 获取指定表的完整信息 (列、索引、注释)
    async fn get_table_info(&self, config: &ConnectionConfig, table_name: &str) -> Result<TableInfo>;

    /// 批量获取多张表的信息
    async fn get_tables_info(&self, config: &ConnectionConfig, tables: &[String]) -> Result<Vec<TableInfo>>;
}
```

### MySQL 实现

通过 `information_schema` 查询:
- `COLUMNS` 表: 获取列信息
- `TABLES` 表: 获取表注释
- `STATISTICS` 表: 获取索引信息

### PostgreSQL 实现

通过系统表查询:
- `information_schema.columns`
- `pg_catalog.pg_class` (表注释)
- `pg_catalog.pg_index` (索引信息)

## 导出器架构

### Exporter Trait

```rust
#[async_trait]
pub trait DocumentExporter: Send + Sync {
    /// 导出单张表的文档
    async fn export_table(&self, table: &TableInfo, output_path: &Path) -> Result<()>;

    /// 导出多张表到一个文件
    async fn export_tables(&self, tables: &[TableInfo], output_path: &Path) -> Result<()>;

    /// 获取文件扩展名
    fn file_extension(&self) -> &'static str;
}
```

### Markdown 导出器

使用 Handlebars 渲染 Markdown 模板,输出 `.md` 文件。

### DOCX 导出器

1. 加载预制 `.xml` 模板 (Word 2003 XML 格式)
2. Handlebars 替换变量 `{{table_name}}`, `{{#each columns}}`...
3. 将 XML 打包为 `.docx` (本质是 ZIP)

### PDF 导出器

使用 `printpdf` 库生成 PDF,或先将 Markdown 转换为 PDF。

## 插件 API 接口

```rust
impl Plugin for DbDocPlugin {
    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value> {
        match method {
            // ========== 连接管理 ==========
            "save_connection" => { ... },      // 保存连接配置
            "list_connections" => { ... },     // 获取所有连接配置
            "delete_connection" => { ... },    // 删除连接配置
            "test_connection" => { ... },      // 测试连接

            // ========== 表查询 ==========
            "list_tables" => { ... },          // 获取所有表名
            "preview_table" => { ... },        // 预览表信息

            // ========== 导出功能 ==========
            "export_docs" => { ... },          // 执行导出
            "get_export_progress" => { ... },  // 获取进度

            // ========== 模板管理 ==========
            "list_templates" => { ... },       // 获取模板列表
        }
    }
}
```

## 前端 UI 组件

```
┌─────────────────────────────────────────────────────────────────┐
│  数据库文档生成                                                  │
├─────────────────────────────────────────────────────────────────┤
│  ┌─── 连接配置 ────────────────────────────────────────────┐   │
│  │  [下拉选择已保存的连接 ▼]    [+ 新建] [编辑] [删除]     │   │
│  │  主机/端口/数据库/用户名/密码           [测试连接]     │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─── 表选择 ──────────────────────────────────────────────┐   │
│  │  搜索: [输入表名筛选...]   [全选] [取消全选]            │   │
│  │  可滚动的多选列表,显示表名、注释、字段数                │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─── 导出设置 ────────────────────────────────────────────┐   │
│  │  输出格式: Word / Markdown / PDF                        │   │
│  │  模板风格: 简洁 / 详细 / 企业                            │   │
│  │  输出目录: [选择...]                                    │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  [开始导出]  +  进度条展示                                      │
└─────────────────────────────────────────────────────────────────┘
```

**主要组件**:
- `ConnectionManager`: 连接配置管理
- `TableSelector`: 可搜索的多选列表
- `ExportSettings`: 格式/模板/目录选择
- `ExportProgress`: 进度条 + 状态显示

## 数据存储

### 存储位置

沿用 `PluginStorage`,数据存储在 `~/.worktools/history/plugins/db-doc.json`

### 数据结构

```json
{
  "version": 1,
  "connections": [
    {
      "id": "conn_001",
      "name": "生产环境",
      "db_type": "mysql",
      "host": "192.168.1.100",
      "port": 3306,
      "database": "production",
      "username": "admin",
      "password": "AES加密后的hex字符串...",
      "created_at": 1711000000,
      "last_used": 1711500000
    }
  ],
  "export_history": [
    {
      "id": "export_001",
      "connection_id": "conn_001",
      "tables": ["users", "orders"],
      "format": "word",
      "template": "detailed",
      "output_path": "/Users/zj/Documents/db-docs/",
      "exported_at": "2024-03-21T15:30:00Z"
    }
  ]
}
```

### 密码加密

复用密码管理器的 `crypto.rs` 模块:
- AES-256 ECB 模式
- PKCS7 填充
- 固定密钥派生自应用标识符

## 依赖库

```toml
[dependencies]
# 插件 API
worktools-plugin-api = { path = "../../shared/plugin-api" }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 错误处理
anyhow = "1.0"

# 数据库驱动
sqlx = { version = "0.7", features = ["mysql", "postgres", "runtime-tokio-rustls", "chrono"] }

# 模板引擎
handlebars = "5.1"

# DOCX 生成
quick-xml = "0.31"
zip = "0.6"

# PDF 生成
printpdf = "0.6"

# 加密
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

## 实现计划

### Phase 1: 核心骨架 (P0)

1. 项目初始化
   - Cargo.toml 配置
   - manifest.json
   - 目录结构

2. Rust 后端核心
   - 数据模型定义
   - MySQL 元数据提取
   - Word 导出器 (单模板)
   - 存储层 (配置存储 + 密码加密)

3. 前端基础 UI
   - 连接配置表单
   - 表列表 (简单多选)
   - 导出按钮 + 基础进度

4. 集成测试
   - 连接 MySQL 测试
   - 导出单表/多表测试
   - 打包 .wtplugin.zip

### Phase 2: PostgreSQL + Markdown (P1)

1. PostgreSQL 元数据提取
2. Markdown 导出器
3. 前端优化

### Phase 3: PDF + 多模板 (P2)

1. PDF 导出器
2. 3 套模板实现
3. 模板选择 UI

### Phase 4: 增强 (P3)

1. 导出历史管理
2. 前端体验优化
3. 错误处理完善

## 相比 Java 版本的改进

| 方面 | Java 版本 | Rust 版本 |
|------|----------|-----------|
| 数据库支持 | 仅 MySQL | MySQL + PostgreSQL |
| 输出格式 | 仅 Word | Word + Markdown + PDF |
| 表选择 | 手动输入 | 可视化多选 |
| 连接管理 | 单次输入 | 多配置管理 |
| 模板 | 单一模板 | 3 套预设模板 |
| 依赖 | Spire.Doc (商业库) | 纯 Rust,无外部依赖 |
