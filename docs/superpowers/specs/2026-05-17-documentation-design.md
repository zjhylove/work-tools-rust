# Documentation Site Design

Date: 2026-05-17

## Background

Work Tools Platform 准备开源，当前文档不足以支撑外部贡献者和用户上手。需要补齐完整文档体系。

## Audience

- 个人开发者（自己日后回顾 + AI 辅助开发）
- 开源社区（外部贡献者和终端用户）

## Language

中文 + 英文技术术语（与 CLAUDE.md 风格一致）。

## Current State

| 已有 | 状态 |
|---|---|
| `CLAUDE.md` | 完善，面向 AI |
| `CHANGELOG.md` | 完善 |
| `scripts/README.md` | 完善 |
| `scripts/README-BUILD.md` | 完善 |
| `scripts/QUICKREF.md` | 完善 |
| `docs/superpowers/` | 若干设计/计划文档 |
| 根目录 `README.md` | **不存在** |
| `LICENSE` | **不存在** |
| `CONTRIBUTING.md` | **不存在** |
| `tauri-app/README.md` | 过时模板内容 |
| 插件文档 × 13 | **不存在** |
| 用户手册 | **不存在** |
| 架构文档（面向人类） | **不存在** |
| 插件开发教程 | **不存在** |

## Decision

- Format: 纯 Markdown 放 `docs/` 目录，GitHub 原生渲染
- Structure: 方案 A — 扁平 `docs/` 结构
- License: Apache 2.0
- Plugin docs: 全部 13 个插件各一份完整文档
- Plugin doc template: 使用 + 技术实现合一

## File Structure

### Root Files

| File | Action | Content |
|---|---|---|
| `README.md` | **新建** | 项目首页：名称、描述、功能特性列表、技术栈 badge、快速开始（3 步）、截图/GIF、文档链接 |
| `LICENSE` | **新建** | Apache 2.0 全文 |
| `tauri-app/README.md` | **更新** | 替换过时的 "Tauri + Solid + Typescript" 模板内容，改为简短说明指向根 README |

### docs/ Directory

```
docs/
├── README.md                    # 文档导航索引
├── getting-started.md           # 开发环境搭建 + 首次运行
├── architecture.md              # 架构设计（面向人类开发者）
├── plugin-development.md        # 插件开发 step-by-step 教程
├── plugin-api-reference.md      # Plugin trait 完整 API 参考
├── user-guide.md                # 用户手册（安装、插件管理、主题、日志）
├── contributing.md              # 贡献指南（代码规范、PR 流程、commit 规范）
├── release-process.md           # 版本发布流程
├── design-token-reference.md    # CSS 设计令牌速查表
├── plugins/                     # 13 个插件各一份文档
│   ├── password-manager.md
│   ├── json-tools.md
│   ├── auth-plugin.md
│   ├── text-diff.md
│   ├── db-doc.md
│   ├── k8s-forward.md
│   ├── db-router.md
│   ├── object-storage.md
│   ├── timestamp-converter.md
│   ├── cron-tools.md
│   ├── redis-client.md
│   └── api-doc.md
└── shared/                      # 共享库文档
    ├── types.md
    └── plugin-api.md
```

### Document Descriptions

| Document | Reader | Core Content |
|---|---|---|
| `docs/README.md` | 所有人 | 文档目录导航，链接到各文档 |
| `docs/getting-started.md` | 新贡献者 | 环境要求（Rust/Node/Tauri）、clone、cargo check、tauri dev、常见问题 |
| `docs/architecture.md` | 开发者 | 目录结构、插件加载机制、iframe srcdoc 渲染、数据流、日志系统、主题系统。来源：从 CLAUDE.md 迁移并扩展 |
| `docs/plugin-development.md` | 插件开发者 | 从零创建一个完整插件的 step-by-step 教程 |
| `docs/plugin-api-reference.md` | 插件开发者 | Plugin trait 每个方法签名、参数、返回值、示例代码 |
| `docs/user-guide.md` | 终端用户 | 下载安装、插件导入/卸载、主题切换、日志查看 |
| `docs/contributing.md` | 贡献者 | 代码风格、commit 规范、PR 模板、测试要求 |
| `docs/release-process.md` | 维护者 | 版本号更新、CHANGELOG、tag、CI 触发、发布检查清单 |
| `docs/design-token-reference.md` | 前端开发者 | 所有 CSS 变量令牌列表（名称、语义、浅色值、暗色值）、用法示例 |
| `docs/plugins/*.md` | 所有人 | 每个插件的功能说明 + 使用方法 + 技术实现 + 配置项 |
| `docs/shared/*.md` | 开发者 | 共享 crate 的类型定义、工具函数说明 |

## Plugin Document Template

每个 `docs/plugins/*.md` 统一结构：

```markdown
# {插件中文名}（{plugin-id}）

> 一句话描述

## 功能特性
- 功能点列表

## 使用方法
### 基本操作
（截图 + 步骤说明）
### 配置项
（可配置参数说明）

## 技术实现
### 后端（Rust）
- 模块结构 + 核心方法
- handle_call 方法列表及参数/返回值
- 数据存储方式
- 依赖的外部库
### 前端（React + TypeScript）
- 组件结构
- pluginAPI.call 调用列表
- 特殊依赖

## 开发与调试
（cargo check/test, npm run dev 命令）

## 已知限制
（如有）
```

## Implementation Phases

### Phase 1 — P0（开源最低门槛，7 个文件）

1. 根目录 `README.md`
2. `LICENSE`（Apache 2.0）
3. `docs/README.md`
4. `docs/getting-started.md`
5. `docs/architecture.md`
6. `docs/contributing.md`
7. `tauri-app/README.md`（更新）

### Phase 2 — P1（插件生态核心，16 个文件）

1. `docs/plugin-development.md`
2. `docs/plugin-api-reference.md`
3. `docs/design-token-reference.md`
4. `docs/plugins/password-manager.md`
5. `docs/plugins/json-tools.md`
6. `docs/plugins/auth-plugin.md`
7. `docs/plugins/text-diff.md`
8. `docs/plugins/db-doc.md`
9. `docs/plugins/k8s-forward.md`
10. `docs/plugins/db-router.md`
11. `docs/plugins/object-storage.md`
12. `docs/plugins/timestamp-converter.md`
13. `docs/plugins/cron-tools.md`
14. `docs/plugins/redis-client.md`
15. `docs/plugins/api-doc.md`
16. `docs/plugins/README.md`（插件文档索引）

### Phase 3 — P2（补全，4 个文件）

1. `docs/user-guide.md`
2. `docs/release-process.md`
3. `docs/shared/types.md`
4. `docs/shared/plugin-api.md`

## Content Source Strategy

- `CLAUDE.md` 中已有的架构描述 → 迁移到 `docs/architecture.md`，CLAUDE.md 保留精简版 + 指向 docs/
- 每个插件的 `manifest.json` + `src/lib.rs` + `frontend/src/` → 提取信息生成文档
- `scripts/` 下的文档保持原位不动
