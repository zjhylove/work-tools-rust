# 数据库文档生成器（db-doc）

> 连接 MySQL / PostgreSQL 数据库，自动提取表结构元数据，导出为 Markdown 或 HTML 文档。

## 功能特性

- 支持 MySQL 和 PostgreSQL 两种数据库类型
- 保存多个数据库连接配置，密码 AES 加密存储
- 一键测试数据库连接是否可用
- 批量提取表结构信息：列定义、索引、注释
- 表预览：选择表后实时加载并展示字段和索引详情
- 导出为 Markdown 或 HTML 格式
- 导出历史记录（最近 50 条）
- 支持按表名搜索、按前缀筛选、全选/反选

## 使用方法

### 基本操作

1. **新建连接**：在「连接管理」页面填写数据库类型、主机、端口、数据库名、用户名和密码，点击「保存」
2. **测试连接**：点击连接卡片上的「测试」按钮，验证连接配置是否正确
3. **选择表**：点击连接卡片进入「选择表 & 导出」页面，勾选需要导出的表
4. **预览结构**：点击「加载预览」查看选中表的字段和索引详情
5. **导出文档**：选择导出格式（Markdown / HTML），选择输出目录，点击「导出」

### 配置项

| 参数 | 说明 | 默认值 |
|------|------|--------|
| db_type | 数据库类型 | mysql |
| host | 主机地址 | localhost |
| port | 端口号 | MySQL 3306 / PostgreSQL 5432 |
| database | 数据库名 | -- |
| username | 用户名 | root |
| password | 密码（加密存储） | -- |

## 技术实现

### 后端（Rust）

**模块结构**：

```
src/
├── lib.rs              # 插件主入口，handle_call 方法分发
├── crypto.rs           # AES 密码加密/解密
├── storage.rs          # 数据持久化（连接配置 + 导出历史）
├── models/
│   ├── mod.rs          # 模型导出
│   ├── connection.rs   # ConnectionConfig, ExportConfig, ExportFormat
│   ├── column.rs       # ColumnInfo（字段名、类型、是否可空等）
│   └── table.rs        # TableInfo, IndexInfo
├── database/
│   ├── mod.rs          # 模块入口
│   ├── extractor.rs    # DatabaseExtractor trait（异步）
│   ├── mysql.rs        # MySQL 实现（sqlx）
│   └── postgres.rs     # PostgreSQL 实现（sqlx）
└── exporter/
    ├── mod.rs          # DocumentExporter trait
    ├── markdown.rs     # Markdown 导出器
    └── html.rs         # HTML 导出器
```

**handle_call 方法列表**：

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `save_connection` | `ConnectionConfig` | `ConnectionConfig` | 新建连接配置（自动生成 ID、加密密码） |
| `update_connection` | `ConnectionConfig`（含 id） | `ConnectionConfig` | 更新连接配置（保留创建时间） |
| `list_connections` | -- | `Vec<ConnectionConfig>` | 获取所有连接（密码解密后返回） |
| `delete_connection` | `{ id }` | `{ success: true }` | 删除连接 |
| `test_connection` | `ConnectionConfig` | `{ success, message }` | 测试数据库连接 |
| `list_tables` | `{ connection_id }` | `Vec<String>` | 获取指定连接的表名列表 |
| `get_table_info` | `{ connection_id, table_name }` | `TableInfo` | 获取表详情（列、索引、注释） |
| `export_docs` | `ExportConfig` | `{ success, files, count }` | 导出文档 |
| `get_export_history` | -- | `Vec<ExportHistory>` | 获取导出历史 |

**核心设计**：

- `with_extractor!` 宏根据 `db_type` 选择 MySQL 或 PostgreSQL extractor，避免大量 match 样板代码
- `DatabaseExtractor` trait 使用 `async_trait` 定义异步接口，`sqlx` 执行 SQL 查询提取元数据
- 自有 `tokio::runtime::Runtime` 在同步 Plugin trait 中桥接异步操作（`block_on`）
- 密码使用 AES 加密存储，读取时解密

**数据存储方式**：
- JSON 文件：`~/.worktools/history/plugins/db-doc.json`
- 存储内容：连接配置列表 + 导出历史
- 密码字段加密存储，读取时自动解密

**依赖的外部库**：

| 库 | 用途 |
|----|------|
| `sqlx` | MySQL / PostgreSQL 异步驱动（编译时 SQL 检查） |
| `tokio` | 异步运行时（`rt-multi-thread`） |
| `aes` + `sha2` | 密码 AES 加密 |
| `chrono` | 时间处理 |
| `uuid` | 生成连接 ID |
| `serde` / `serde_json` | 序列化 |
| `anyhow` | 错误处理 |
| `async-trait` | 异步 trait 支持 |

### 前端（React + TypeScript）

**组件结构**：

- `App` -- 主组件，管理连接列表和表选择两个视图
- `StepHeader` -- 步骤指示器（连接管理 -> 选择表 & 导出）
- `ConnectionView` -- 连接管理视图（左侧连接列表 + 右侧新建/编辑表单）
- `ConnectionForm` -- 连接配置表单（支持新建和编辑模式）
- `TablePreview` -- 表结构预览（字段表格 + 索引表格）

**pluginAPI.call 调用列表**：

| 调用方法 | 用途 |
|----------|------|
| `list_connections` | 加载连接列表 |
| `save_connection` | 新建连接 |
| `update_connection` | 更新连接 |
| `delete_connection` | 删除连接 |
| `test_connection` | 测试连接 |
| `list_tables` | 加载表名列表 |
| `get_table_info` | 加载表结构预览 |
| `export_docs` | 执行导出 |

**特殊依赖**：
- 无额外第三方依赖，仅使用 React 内置 hooks（`useState`, `useEffect`, `useMemo`）

## 开发与调试

```bash
# Rust 检查
cargo check -p db-doc

# 运行测试
cargo test -p db-doc

# 前端开发
cd plugins/db-doc/frontend && npm run dev

# 前端构建
cd plugins/db-doc/frontend && npm run build
```

## 已知限制

- 导出历史最多保留 50 条记录（FIFO）
- 批量获取表信息时，单张表失败不影响其他表，但会在日志中警告
- 不支持导出为 PDF 格式（仅 Markdown 和 HTML）
- 表结构提取不包含外键关系
