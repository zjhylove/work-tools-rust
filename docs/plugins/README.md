# Work Tools 插件目录

Work Tools Platform 内置 12 个插件，覆盖开发、运维、安全等场景。所有插件均采用 Rust 后端 + React 前端架构，通过动态库（cdylib）加载，iframe srcdoc 渲染。

---

## 插件列表

| 图标 | 插件名称 | plugin-id | 一句话描述 | 文档 |
|---|---|---|---|---|
| 🔐 | 密码管理器 | `password-manager` | 本地加密存储密码,支持导入导出 | -- |
| { } | JSON 工具 | `json-tools` | JSON 格式化、压缩、转义及树形可视化编辑 | -- |
| 🔢 | 双因素验证 | `auth` | TOTP 动态验证码生成与管理 | -- |
| 📝 | 文本比对 | `text-diff` | 并排文本比对,字符级差异高亮与统计 | -- |
| 📊 | 数据库文档 | `db-doc` | 连接数据库,自动生成表结构文档 (Word/Markdown/PDF) | -- |
| 🌐 | K8s IP 转发 | `k8s-forward` | 通过 Kuboard 发现 K8s Pod,SSH 隧道+HTTP 代理转发流量 | -- |
| 🗄 | 数据库路由 | `db-router` | 根据编号解析数据库和表路由规则，支持多表关联 | -- |
| 📦 | 对象存储 | `object-storage` | 管理阿里云 OSS 和腾讯云 COS，文件浏览/上传/下载/搜索/删除 | -- |
| ⏰ | 时间戳转换 | `timestamp-converter` | Unix 时间戳与日期时间互相转换，支持多时区、批量处理 | [文档](./timestamp-converter.md) |
| ⏱ | Cron 表达式工具 | `cron-tools` | Cron 表达式解析、人类可读描述、下次执行时间预览、可视化构建 | [文档](./cron-tools.md) |
| 🔴 | Redis 客户端 | `redis-client` | Redis 数据库管理工具，支持 Key 浏览、多数据类型操作、SSH 隧道 | [文档](./redis-client.md) |
| 📄 | API 文档生成 | `api-doc` | 解析 Spring Boot JAR 包,自动生成 API 接口文档 (Markdown/HTML) | [文档](./api-doc.md) |

---

## 分类

### 开发工具
- **JSON 工具** -- JSON 格式化与可视化编辑
- **文本比对** -- 代码/文本差异对比
- **时间戳转换** -- 时间戳与日期互转
- **Cron 表达式工具** -- Cron 表达式解析与构建

### 数据库
- **数据库文档** -- 自动生成表结构文档
- **数据库路由** -- 编号解析路由规则
- **Redis 客户端** -- Redis 数据管理

### 运维
- **K8s IP 转发** -- K8s Pod 端口转发
- **对象存储** -- OSS/COS 文件管理

### 安全
- **密码管理器** -- 加密密码存储
- **双因素验证** -- TOTP 验证码

### 文档
- **API 文档生成** -- Spring Boot API 文档自动生成

---

## 插件权限

| 权限 | 插件 |
|---|---|
| `filesystem` | password-manager, text-diff, db-doc, k8s-forward, db-router, object-storage, api-doc |
| `network` | db-doc, k8s-forward, object-storage, redis-client, api-doc |
| `clipboard` | password-manager, auth, text-diff |

## 通用架构

所有插件遵循统一的架构规范：

- **后端**：Rust，编译为 cdylib 动态库，实现 `Plugin` trait
- **前端**：React + TypeScript + Vite，构建后输出 `assets/` 目录
- **通信**：通过 `window.pluginAPI.call(pluginId, method, params)` 调用后端方法
- **主题**：CSS 必须使用 `var(--xxx)` 设计令牌，兼容浅色/暗色主题
- **打包**：`.wtplugin.zip` 格式（manifest.json + 动态库 + assets/）
- **安装路径**：`~/.worktools/plugins/<plugin-id>/`
