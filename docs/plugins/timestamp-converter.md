# 时间戳转换（timestamp-converter）

> Unix 时间戳与日期时间互相转换，支持多时区、批量处理

## 功能特性

- 实时显示当前时间戳和格式化时间，每秒自动刷新
- 时间戳转日期时间：支持秒（10 位）、毫秒（13 位）、微秒（16 位）三种精度
- 日期时间转时间戳：支持 ISO 8601、RFC 2822、`YYYY-MM-DD HH:MM:SS` 等多种格式输入
- 批量转换：多行输入，自动识别是时间戳还是日期时间，逐行转换
- 多时区支持：上海、东京、伦敦、纽约、洛杉矶、UTC 等 6 个常用时区
- 输出多种格式：ISO 8601、RFC 2822、本地时间、UTC 时间

## 使用方法

### 基本操作

1. 打开插件后，顶部实时显示当前时间和 Unix 时间戳
2. 通过顶部时区选择器切换目标时区
3. 使用三个 Tab 切换功能模式：
   - **时间戳 -> 日期**：输入时间戳数字，点击"转换"或在输入框按回车，获得多格式时间表示
   - **日期 -> 时间戳**：输入日期时间字符串，获得秒级、毫秒级、微秒级三种时间戳
   - **批量转换**：在文本框中每行输入一个值（时间戳或日期时间均可），点击"批量转换"获得结果表格

### 配置项

- **timezone**：目标时区，默认 `Asia/Shanghai`，可选 `Asia/Tokyo`、`Europe/London`、`America/New_York`、`America/Los_Angeles`、`UTC`

## 技术实现

### 后端（Rust）

- **模块结构**：单文件 `lib.rs`，无子模块
- **核心结构**：`TimestampConverter` 实现 `Plugin` trait

- **handle_call 方法列表**：

| 方法 | 参数 | 返回值 |
|---|---|---|
| `timestamp_to_datetime` | `ts` (string), `timezone?` (string) | `{ utc, datetime, timezone, format_iso, format_rfc2822 }` |
| `datetime_to_timestamp` | `datetime` (string), `timezone?` (string) | `{ ts_sec, ts_ms, ts_us }` |
| `current_time` | `timezone?` (string) | `{ ts_sec, ts_ms, utc, datetime, timezone, format_iso, format_rfc2822 }` |
| `batch_convert` | `items` (array of `{ value, direction }`), `timezone?` | `{ results: [...] }` |

- **时间戳解析逻辑**：
  - 10 位数字 -> 秒级
  - 13 位数字 -> 毫秒级，自动除以 1000
  - 16 位数字 -> 微秒级，自动除以 1,000,000
- **日期时间解析**：依次尝试 RFC 3339 -> RFC 2822 -> `YYYY-MM-DD HH:MM:SS`
- **默认时区**：`Asia/Shanghai`
- **数据存储**：无持久化存储

- **依赖库**：
  - `chrono` 0.4 (serde feature) - 日期时间处理
  - `chrono-tz` 0.10 - IANA 时区数据库支持
  - `serde_json` / `serde` - JSON 序列化
  - `anyhow` - 错误处理
  - `worktools-plugin-api` - 插件 trait 定义

### 前端（React + TypeScript）

- **组件结构**：单组件 `App.tsx`，无子组件
- **状态管理**：`useState` 管理 5 个核心状态（当前时间、时区、时间戳输入/结果、日期输入/结果、批量输入/结果）
- **pluginAPI.call 调用列表**：
  - `timestamp-converter` / `current_time` -- 每秒轮询更新时钟
  - `timestamp-converter` / `timestamp_to_datetime` -- 时间戳转日期
  - `timestamp-converter` / `datetime_to_timestamp` -- 日期转时间戳
  - `timestamp-converter` / `batch_convert` -- 批量转换
- **特殊依赖**：无第三方 UI 库
- **WorkTools 集成**：使用 `window.WorkTools.toast` 显示错误提示

## 开发与调试

```bash
# Rust 后端
cargo check -p timestamp-converter
cargo test -p timestamp-converter

# 前端
cd plugins/timestamp-converter/frontend && npm run dev

# 完整构建
cd plugins/timestamp-converter/frontend && npm run build
cargo build --release -p timestamp-converter
```

## 已知限制

- 时间戳解析仅支持 10/13/16 位三种长度，其他长度会报错
- 日期时间解析不支持 `YYYY-MM-DD` 纯日期格式（仅支持带时间的格式）
- 批量转换每行只能处理一个值
- 无历史记录保存功能
