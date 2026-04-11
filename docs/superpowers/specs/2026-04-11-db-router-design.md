# 数据库路由插件 (db-router) 设计文档

## 概述

将 Java 版数据库路由插件迁移到 Rust Tauri 插件架构。核心功能：通过用户定义的 Rhai 脚本，将编号（code）解析为数据库名和关联表名列表。支持多张表共享同一分片算法。

## 插件结构

```
plugins/db-router/
├── Cargo.toml
├── manifest.json
├── src/
│   ├── lib.rs          # Plugin trait + handle_call
│   ├── model.rs        # RouteRule / RouteResult
│   └── engine.rs       # Rhai 脚本引擎
├── assets/
└── frontend/
    ├── package.json
    ├── vite.config.ts
    ├── tsconfig.json
    ├── index.html
    └── src/
        ├── main.tsx
        ├── App.tsx
        └── App.css
```

## 数据模型

### RouteRule

```rust
#[derive(Serialize, Deserialize, Clone)]
struct RouteRule {
    id: String,              // UUID
    name: String,            // 规则名称（必填）
    description: String,     // 规则描述
    code_length: u32,        // 编号长度，0=任意
    code_prefix: String,     // 逗号分隔的前缀列表，空=任意
    route_script: String,    // Rhai 脚本（必填）
    tables: Vec<String>,     // 关联表名前缀列表，如 ["t_order", "t_order_item"]
}
```

### RouteResult

```rust
#[derive(Serialize, Deserialize, Clone)]
struct RouteResult {
    database: String,        // 解析出的数据库名
    tables: Vec<String>,     // 关联表名前缀 + 脚本算出的 table_suffix 拼接
    code: String,            // 原始输入编号
    rule_name: String,       // 使用的规则名称
    parse_time: String,      // ISO 8601 时间戳
}
```

### RouteData（持久化结构）

```rust
#[derive(Serialize, Deserialize, Default)]
struct RouteData {
    rules: Vec<RouteRule>,
}
```

## Rhai 脚本引擎 (engine.rs)

### 契约

- 输入：`code` 变量（String）
- 输出：必须设置 `database`（String）和 `table_suffix`（String）
- 超时：5 秒

### 安全限制

- 禁用 `file_*`、`http_*` 模块
- 禁止加载外部模块
- Rhai Engine 使用默认安全配置

### 执行流程

1. 创建 Rhai Engine，禁用不安全模块
2. 注入 `code` 变量到 scope
3. 执行 `route_script`
4. 从 scope 读取 `database` 和 `table_suffix`
5. 将 `table_suffix` 与规则的 `tables` 列表中每个前缀拼接，生成完整表名列表

## handle_call 方法

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `list_rules` | `{}` | `RouteData` | 获取所有规则 |
| `save_rule` | `{ rule: RouteRule }` | `RouteData` | 创建/更新规则（id 为空则新建） |
| `delete_rule` | `{ id: String }` | `RouteData` | 删除规则 |
| `parse_route` | `{ code: String, rule_id: String }` | `RouteResult` | 执行路由解析 |
| `match_rules` | `{ code: String }` | `Vec<RouteRule>` | 按编号过滤匹配规则 |
| `import_rules` | `{ rules: Vec<RouteRule> }` | `RouteData` | 导入规则（合并，同名覆盖） |
| `export_rules` | `{}` | `Vec<RouteRule>` | 导出所有规则 |
| `get_templates` | `{}` | `Vec<RouteRule>` | 获取预设模板规则列表 |

### 规则匹配逻辑

对输入 code 过滤规则，两个条件 AND：
1. `code_length == 0` 或 `code.len() == code_length`
2. `code_prefix` 为空 或 `code` 以任一前缀开头

## 持久化

- 使用 `PluginStorage`，存储文件 `~/.worktools/history/plugins/db-router.json`
- 存储 `RouteData` 结构（包含 `rules` 数组）

## 前端设计

### 布局：双栏

```
┌──────────────────────────────────────────────────┐
│  🔍 数据库路由            [+ 新建] [导入] [导出]  │
├────────────────┬─────────────────────────────────┤
│  🔍搜索规则     │  输入编号                        │
│ ─────────────  │  ┌─────────────────────────┐    │
│  规则卡片1      │  │ ORD2024011500175632    │    │
│  标签+关联表数  │  └─────────────────────────┘    │
│  [▶][✏][🗑]    │                                 │
│ ─────────────  │  解析结果                        │
│  规则卡片2      │  ┌─────────────────────────┐    │
│  ...           │  │ db: db_order_2024       │    │
│                │  │ tables:                  │    │
│                │  │  - t_order_001          │    │
│                │  │  - t_order_item_001     │    │
│                │  │  - t_payment_001        │    │
│                │  │ 脚本预览...              │    │
│                │  └─────────────────────────┘    │
│                │                     [📋 复制结果] │
│                │                                 │
│  共 N 条规则    │                                 │
└────────────────┴─────────────────────────────────┘
```

### 左栏：规则管理

- 搜索框：按名称模糊搜索
- 规则卡片：名称、描述、长度/前缀标签、关联表数标签（如"关联 3 张表"）
- 每张卡片右侧：解析(▶)、编辑(✏)、删除(🗑)
- 输入编号时实时过滤匹配规则

### 右栏：解析工作台

- 编号输入框（等宽字体，高亮样式）
- 解析结果卡片：
  - 成功：绿色背景，显示 database + tables 列表
  - 失败：红色背景，显示错误信息
  - 未解析：灰色空状态提示
- 脚本预览区域
- 复制结果按钮

### 规则编辑弹窗 (Modal)

| 字段 | 控件 | 必填 | 说明 |
|------|------|------|------|
| 规则名称 | TextField | 是 | |
| 规则描述 | TextArea | 否 | |
| 编号长度 | TextField | 否 | 数字，0=任意 |
| 编号前缀 | TextField | 否 | 逗号分隔 |
| 关联表名 | TextArea | 否 | 每行一个表名前缀 |
| 解析脚本 | TextArea (等宽) | 是 | Rhai 脚本 |
| 从模板加载 | 下拉选择 | - | 选择预设模板自动填充脚本 |

### 交互逻辑

- 输入编号 → 左栏规则实时过滤
- 点击规则卡片 ▶ → 右栏执行解析并显示结果
- 新建/编辑规则 → 弹窗表单，支持从模板加载
- 导入 → `open_folder_dialog` 选择 JSON 文件
- 导出 → `open_folder_dialog` 选择保存路径
- 删除 → 确认弹窗

## 预设模板规则

### 1. 按位置截取

```rhai
let database = "db_order_" + code.substr(3, 4);
let table_suffix = "_" + code.substr(15, 3);
```

### 2. 取模分片

```rhai
let hash = code.hash();
let shard = (hash % 16).to_string();
let database = "db_order_" + shard;
let table_suffix = "_" + shard;
```

### 3. 日期分表

```rhai
let year = code.substr(3, 4);
let month = code.substr(7, 2);
let database = "db_log";
let table_suffix = "_" + year + "_" + month;
```

## 依赖

### Rust (Cargo.toml)

```toml
[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rhai = "1.19"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
```

### 前端 (package.json)

```json
{
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "^5.6.0",
    "vite": "^5.4.0"
  }
}
```

## manifest.json

```json
{
  "id": "db-router",
  "name": "数据库路由",
  "description": "根据编号解析数据库和表路由规则，支持多表关联",
  "version": "1.0.0",
  "icon": "🔍",
  "author": "Work Tools Team",
  "files": {
    "macos": "libdb_router.dylib",
    "linux": "libdb_router.so",
    "windows": "db_router.dll"
  },
  "assets": {
    "entry": "index.html"
  },
  "permissions": ["filesystem"]
}
```
