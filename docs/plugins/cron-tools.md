# Cron 表达式工具（cron-tools）

> Cron 表达式解析、人类可读描述、下次执行时间预览、可视化构建

## 功能特性

- Cron 表达式实时解析与验证（5 字段标准格式：分 时 日 月 周）
- 自动生成中文人类可读描述（如"每5分钟"、"工作日上午9点"）
- 下次执行时间预览：展示接下来 5 次执行时间的时间线
- 快速模板：9 个常用 Cron 表达式一键填入
- 可视化构建器：下拉选择每个字段的值，自动生成表达式

## 使用方法

### 基本操作

1. 在顶部输入框中输入或修改 Cron 表达式
2. 输入停止 300ms 后自动解析，显示验证结果和中文描述
3. 下方时间线显示接下来 5 次执行时间
4. 使用"快速模板"区域的芯片按钮一键填入常用表达式
5. 展开"可视化构建"面板，通过下拉框逐字段选择值

### 配置项

无持久化配置。表达式格式固定为 5 字段标准 Cron：

```
┌──────── 分钟 (0-59)
│ ┌────── 小时 (0-23)
│ │ ┌──── 日 (1-31)
│ │ │ ┌── 月 (1-12)
│ │ │ │ ┌ 周 (0-6, 0=周日)
│ │ │ │ │
* * * * *
```

支持的特殊字符：`*`（每）、`/`（间隔）、`,`（列表）、`-`（范围）

## 技术实现

### 后端（Rust）

- **模块结构**：单文件 `lib.rs`，无子模块
- **核心结构**：`CronTools` 实现 `Plugin` trait

- **handle_call 方法列表**：

| 方法 | 参数 | 返回值 |
|---|---|---|
| `parse_cron` | `expr` (string) | `{ valid, description, error }` |
| `next_executions` | `expr` (string), `count?` (number, 默认 5, 最大 20) | `{ times: [rfc3339_string, ...] }` |
| `get_presets` | (无) | `{ presets: [{ label, expr }, ...] }` |

- **表达式转换**：5 字段标准 Cron 在传给 `cron` crate 前自动补齐为 7 字段格式（添加秒和年为 `0` 和 `*`）
- **描述生成**：`describe_cron()` 逐字段解析，生成中文描述
  - `*` -> "每X"
  - `*/N` -> "每X间隔N"
  - `A,B,C` -> "X的第A、B、C"
  - `A-B` -> "X从A到B"
- **数据存储**：无持久化存储

- **依赖库**：
  - `cron` 0.14 - Cron 表达式解析与迭代
  - `chrono` 0.4 - 日期时间处理
  - `serde_json` / `serde` - JSON 序列化
  - `anyhow` - 错误处理
  - `worktools-plugin-api` - 插件 trait 定义

### 前端（React + TypeScript）

- **组件结构**：单组件 `App.tsx`，无子组件
- **状态管理**：
  - `expr` - 当前表达式
  - `description` / `valid` - 解析结果
  - `execTimes` - 下次执行时间列表
  - `presets` - 快速模板列表
  - `fields` - 可视化构建器中 5 个字段的值
- **pluginAPI.call 调用列表**：
  - `cron-tools` / `parse_cron` -- 解析表达式
  - `cron-tools` / `next_executions` -- 获取下次执行时间
  - `cron-tools` / `get_presets` -- 加载快速模板（组件挂载时）
- **特殊依赖**：无第三方 UI 库
- **防抖处理**：使用 `setTimeout` 300ms 防抖，避免频繁调用后端

## 开发与调试

```bash
# Rust 后端
cargo check -p cron-tools
cargo test -p cron-tools

# 前端
cd plugins/cron-tools/frontend && npm run dev

# 完整构建
cd plugins/cron-tools/frontend && npm run build
cargo build --release -p cron-tools
```

## 已知限制

- 仅支持标准 5 字段 Cron 格式，不支持 6/7 字段扩展（如秒级、年级）
- 可视化构建器仅支持基础值选择，不支持 `*/N`、`A-B`、`A,B` 等复合表达式的构建
- 下次执行时间预览最多 20 条
- 无自定义 Cron 表达式保存功能
