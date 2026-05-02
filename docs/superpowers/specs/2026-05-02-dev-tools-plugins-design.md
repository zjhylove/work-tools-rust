# 开发者工具插件设计规格

**日期**: 2026-05-02
**状态**: 待审核

## 概述

新增 3 个独立 cdylib 插件：cron-tools、timestamp-converter、redis-client。均遵循现有 Plugin trait 模式，前端通过 iframe srcdoc 渲染。

---

## 1. cron-tools — Cron 表达式工具

### 1.1 功能

| 功能 | 说明 |
|------|------|
| 表达式 → 人类描述 | 如 `*/5 * * * *` → "每5分钟执行一次" |
| 表达式 → 下次执行时间 | 计算未来 N 次触发时间 |
| 可视化构建器 | 下拉选择器生成表达式 |
| 常用预设 | 每分钟、每小时、每天、工作日、每月初等 |
| 验证 | 检查表达式合法性 |

### 1.2 技术

- **后端**: `cron` crate 解析 + 调度计算
- **前端**: React 表单选择器 + 时间线预览
- **无持久化**，纯计算型
- 权限: 无需

### 1.3 Cargo.toml 依赖

```toml
[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tracing = "0.1"
cron = "0.14"
chrono = "0.4"
```

### 1.4 Rust handle_call 方法

```
parse_cron(expr, include_seconds?) → { description, valid, error? }
next_executions(expr, count?, start_time?) → { times: [string] }
get_presets() → { presets: [{ label, expr }] }
```

### 1.5 前端布局

```
Cron 表达式输入框  [解析]
├─ 人类可读描述
├─ 下次 N 次执行时间列表
├─ 可视化构建器 (折叠面板)
│  └─ 5个下拉选择器: 分/时/日/月/周
└─ 常用预设按钮组
```

---

## 2. timestamp-converter — 时间戳转换

### 2.1 功能

| 功能 | 说明 |
|------|------|
| 时间戳 → 日期 | 自动识别秒(10位)/毫秒(13位)/微秒(16位) |
| 日期 → 时间戳 | 解析 ISO 8601 / RFC 2822 / 常规格式 |
| 多时区 | UTC / 本地 / 指定 IANA 时区 |
| 当前时间 | 实时刷新 (1s 间隔) |
| 批量转换 | 多行输入批量处理 |

### 2.2 技术

- **后端**: `chrono` + `chrono-tz`
- **前端**: React 实时钟 + 转换表单
- **无持久化**
- 权限: 无需

### 2.3 Cargo.toml 依赖

```toml
[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }
chrono-tz = "0.10"
```

### 2.4 Rust handle_call 方法

```
timestamp_to_datetime(ts, timezone?) → { datetime, utc, format_iso, format_rfc2822 }
datetime_to_timestamp(datetime, timezone?) → { ts_sec, ts_ms, ts_us }
current_time(timezone?) → { ts_sec, ts_ms, datetime, utc }
batch_convert(items: [{value, direction}], timezone?) → { results: [...] }
```

### 2.5 前端布局

```
当前时间 (实时刷新)
─────────────────
[时间戳 → 日期] 输入框 + 时区选择 + 转换按钮
  → 多格式输出 (ISO 8601 / RFC 2822 / UTC)
─────────────────
[日期 → 时间戳] 输入框 + 时区选择 + 转换按钮
  → 秒 / 毫秒 / 微秒
─────────────────
[批量模式] 切换标签页
  → 多行输入 → 表格结果
```

---

## 3. redis-client — Redis 客户端

### 3.1 功能

| 功能 | 说明 |
|------|------|
| 连接管理 | 创建/断开连接，保存多组连接配置 |
| Key 浏览 | SCAN 遍历 + pattern 搜索，显示 type/ttl |
| String CRUD | GET/SET 带 JSON 格式化 |
| Hash CRUD | HGETALL 表格展示，字段增删改 |
| List 查看 | LRANGE 列表展示，支持 push/pop |
| Set/ZSet 查看 | SMEMBERS / ZRANGE with scores |
| Key 操作 | 删除/重命名/设置 TTL |

### 3.2 技术

- **后端**: `redis` crate + `PluginStorage` 持久化连接配置
- **前端**: 左右两栏布局 (连接面板 + 内容区)
- 权限: `network`
- **安全**: 密码不存储明文，用本地 key 混淆

### 3.3 Cargo.toml 依赖

```toml
[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tracing = "0.1"
redis = "0.27"
```

### 3.4 插件结构

```rust
pub struct RedisClient {
    client: Option<redis::Client>,    // Client is Send+Sync; get_connection() per operation
    current_config: Option<ConnectionConfig>,
    storage: PluginStorage,           // 持久化已保存连接
    saved_connections: Vec<SavedConnection>,
}
```

> `redis::Client` 是 `Send + Sync`，满足 Plugin trait bound。每个 handle_call 操作通过 `client.get_connection()` 获取新连接，避免连接状态问题和 Sync 约束冲突。

连接配置存储结构:
```json
// ~/.worktools/history/plugins/redis-client.json
{
  "saved_connections": [
    {
      "id": "uuid",
      "name": "dev",
      "host": "127.0.0.1",
      "port": 6379,
      "db": 0,
      "password_obfuscated": "..."
    }
  ]
}
```

### 3.5 Rust handle_call 方法

**连接:**
- `connect(host, port, db, password?)` → `{ ok }`
- `disconnect()` → `{ ok }`
- `save_connection(name, host, port, db, password?)` → `{ id }`
- `list_saved_connections()` → `{ connections }`
- `delete_saved_connection(id)` → `{ ok }`

**Key:**
- `scan_keys(cursor, pattern?)` → `{ cursor, keys: [{key, type, ttl}] }`
- `get_key_info(key)` → `{ key_type, ttl, length }`
- `delete_key(key)` / `rename_key(old, new)` / `set_ttl(key, seconds)`

**数据操作:**
- `get_string(key)` → `{ value }`
- `set_string(key, value)` → `{ ok }`
- `get_hash(key)` → `{ fields: {...} }`
- `set_hash_field(key, field, value)` / `del_hash_field(key, field)`
- `get_list(key, start?, stop?)` → `{ items: [...] }`
- `lpush(key, value)` / `rpush(key, value)` / `lrem(key, index)`
- `get_set(key)` → `{ members: [...] }`
- `sadd(key, member)` / `srem(key, member)`
- `get_zset(key)` → `{ members: [{member, score}] }`
- `zadd(key, score, member)` / `zrem(key, member)`

### 3.6 前端布局

```
┌──────────┬──────────────────────────────┐
│ 连接面板  │  内容区                        │
│          │                               │
│ Host: [] │  Key 搜索: [            🔍]   │
│ Port: [] │  ┌─ key1  [String]  TTL:-1 ┐ │
│ DB:   [] │  ├─ key2  [Hash]   TTL:60  │ │
│ Pass: [] │  ├─ key3  [List]   TTL:-1  │ │
│ [连接]   │  └─ ...                     │ │
│          │                               │
│ 已保存:   │  ── 选中 key 详情 ──          │
│ ├ dev    │  Type: String  TTL: 3600      │
│ ├ staging│  ┌───────────────────────┐    │
│ └ prod   │  │ value ...             │    │
│          │  └───────────────────────┘    │
│          │  [刷新] [保存] [删除]          │
└──────────┴──────────────────────────────┘
```

左侧面板宽 ~220px 固定宽度，右侧自适应。Key 列表支持分页 (SCAN 游标)，String 值自动检测 JSON 并语法高亮，Hash 用键值表格展示。

---

## 实施顺序

建议按复杂度递增: timestamp-converter → cron-tools → redis-client

1. **timestamp-converter**: 最简单，纯计算无状态，快速跑通流程
2. **cron-tools**: 纯计算但涉及 cron crate 和中文描述生成
3. **redis-client**: 最复杂，涉及连接管理、PluginStorage、网络操作、多种数据结构的 UI

每个插件实现后单独编译验证 (`cargo check -p <plugin>`) 再继续下一个。
