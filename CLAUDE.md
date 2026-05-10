# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Work Tools Platform - 基于 Tauri 2.x + Rust 的可扩展工具平台，采用动态库插件架构。插件编译为 cdylib，主程序通过 libloading 同进程加载。

**技术栈**: Rust 1.70+ | Tauri 2.x | React 19 + TypeScript + Vite 6 | libloading 动态库 | JSON 文件存储

## 常用命令

```bash
# 开发
cd tauri-app && npm run tauri dev          # 启动开发服务器 (前端 :1420)

# Rust 检查 (开发时首选 cargo check，比 build 快得多)
cargo check                                 # workspace 全部类型检查
cargo check -p password-manager             # 单个 crate 检查
cargo fmt                                   # 代码格式化
cargo clippy                                # 代码 lint

# 测试
cargo test                                  # 全部 workspace 测试
cargo test -p password-manager              # 单个插件全部测试
cargo test -p password-manager -- crypto    # 插件中单个模块测试
cargo test -p db-router -- test_execute     # 按测试名过滤

# 前端检查
cd tauri-app && npx tsc --noEmit            # TypeScript 类型检查

# 构建
cd tauri-app && npm run tauri build         # 生产构建 (Tauri 主应用)
bash scripts/build-plugins.sh               # 一键编译+打包所有插件
cargo build --release -p password-manager   # 单个插件编译
cd plugins/text-diff/frontend && npm run build  # 单个插件前端构建
```

## 架构

### 目录结构

```
work-tools-rust/
├── tauri-app/              # Tauri 主应用
│   ├── src/               # React 前端
│   └── src-tauri/src/     # Rust 后端
│       ├── lib.rs              # 应用初始化、Tauri builder
│       ├── commands.rs         # 21 个 Tauri 命令
│       ├── plugin_manager.rs   # 动态库加载、插件生命周期
│       ├── plugin_package.rs   # .wtplugin.zip 解析安装
│       ├── plugin_registry.rs  # 插件注册表管理
│       ├── logger.rs           # 日志系统 (tracing 三层架构)
│       ├── config.rs           # 插件配置持久化
│       ├── paths.rs            # 工作目录路径管理
│       └── tray.rs             # 系统托盘
├── plugins/                # 13 个插件 (各有独立 frontend/)
│   ├── password-manager/   # 密码管理器 (AES 加密)
│   ├── json-tools/         # JSON 工具
│   ├── auth-plugin/        # 双因素验证 (TOTP)
│   ├── text-diff/          # 文本比对 (Monaco Editor)
│   ├── db-doc/             # 数据库文档生成 (MySQL/PostgreSQL)
│   ├── k8s-forward/        # K8s 端口转发 (SSH 隧道 + HTTP 代理)
│   ├── db-router/          # 数据库路由 (Rhai 脚本解析)
│   ├── object-storage/     # 对象存储 (阿里云OSS + 腾讯云COS)
│   ├── timestamp-converter/ # Unix时间戳转换 (多时区/批量)
│   ├── cron-tools/         # Cron表达式解析/可视化
│   ├── redis-client/       # Redis 客户端 (Key/多类型操作)
│   ├── api-doc/            # API文档生成 (Spring Boot JAR解析)
│   └── ...
├── shared/
│   ├── types/             # 共享数据类型
│   └── plugin-api/        # Plugin trait + storage/error/tracing
└── scripts/               # 构建脚本
```

### 插件渲染机制

插件前端通过 **iframe srcdoc** 渲染：
1. `PluginPlaceholder` 读取已安装插件的 `index.html`、`main.js`、`styles.css`
2. 内联到 HTML 字符串注入 iframe 的 srcdoc
3. iframe 加载后注入 `window.pluginAPI` 对象，提供 `call()`、`get_plugin_config()`、`set_plugin_config()`、`open_url()`、`open_folder_dialog()`、`open_file_dialog()`、`write_file()`

### 主题系统

支持浅色/暗色双主题，通过 CSS 变量 + `data-theme` 属性驱动：

- `tauri-app/src/styles/tokens.css` — 完整的设计令牌 (`:root` 浅色 + `[data-theme="dark"]` 暗色)
- `App.tsx` 管理 `theme` state，持久化到 localStorage，设置 `<html data-theme>`
- 侧边栏底部 moon/sun 图标按钮切换主题
- Rust 命令 `set_window_theme` 同步原生窗口标题栏主题 (Windows 10 1809+)
- 插件 iframe 通过 `INJECTED_TOKENS` 接收令牌（含 `[data-theme="dark"]` 块），先注入插件 styles.css 再注入令牌确保令牌优先级最高
- 切换时 `postMessage({ type: "theme", theme })` 通知所有已打开的 iframe 实时更新
- 插件 CSS **必须使用** `var(--xxx)` 令牌，禁止硬编码颜色值（否则暗色主题失效）

### 数据流

```
前端 iframe → window.pluginAPI.call(pluginId, method, params)
  → Tauri command: call_plugin_method
  → PluginManager::call_plugin_method()
  → Plugin::handle_call(method, params)
  → 返回 JSON 结果
```

### 日志系统

三层 `tracing_subscriber::registry()` 架构 (logger.rs):

| 层 | 输出 | 用途 |
|---|---|---|
| fmt::layer (stdout) | 控制台 | 开发调试，带 ANSI 颜色 |
| fmt::layer (non_blocking_file) | `~/.worktools/logs/` 按天滚动 | 持久化，无颜色，带 target |
| LogRingLayer | `LOG_RING` (Mutex<VecDeque>, 1000条) | 前端查询 |

- Tauri command `get_logs` 从 LOG_RING 读取，支持按 level/plugin/since 过滤，返回最近 100 条
- `get_logs` 通过 `iter().rev().filter().take(100)` 避免克隆全部
- `clear_logs` 清空环形缓冲
- winit/tao 的 WARN 被过滤到 ERROR 级别，消除事件循环噪音
- 所有 13 个插件接入 tracing，关键操作有 info/warn/error 日志

### 插件系统关键设计

**Plugin trait** (`shared/plugin-api/src/lib.rs`):
- 必须实现: `id()`、`name()`、`description()`、`version()`、`icon()`、`get_view()`
- 可选覆盖: `init()`、`destroy()`、`handle_call()`、`get_assets_path()`
- 辅助模块: `PluginStorage` (JSON 文件持久化)、`PluginError` / `method_error!` / `param_error!` (错误处理)
- 插件必须编译为 `cdylib`，导出 `#[no_mangle] pub extern "C" fn plugin_create()`
- trait bound: `Send + Sync`

**插件包格式 (.wtplugin.zip)**:
```
├── manifest.json          # 插件元数据
├── lib<name>.dylib/.so/.dll   # 动态库 (按平台)
└── assets/                # 前端资源
    ├── index.html
    ├── main.js
    └── styles.css
```

**安装路径**: `~/.worktools/plugins/<plugin-id>/`
**注册表**: `~/.worktools/config/installed-plugins.json`
**插件数据**: `~/.worktools/history/plugins/<plugin-id>.json`

### Workspace 注意事项

根 `Cargo.toml` 使用 `exclude = ["tauri-app"]` 但同时将 `tauri-app/src-tauri` 列为 workspace member。

- `tauri-app/` 被 exclude 是因为它包含前端项目 (package.json, node_modules 等)，不在 workspace 的管理范围内
- `tauri-app/src-tauri/` 作为 Rust crate 仍然是 workspace member
- `cargo test` 在根目录运行时会测试所有 15 个 workspace members（2 shared + 13 plugins）
- `cargo check/build` 只编译 workspace members（不包含 tauri-app 前端）

## 插件开发要点

新建插件需要：
1. `Cargo.toml` 中 `crate-type = ["cdylib"]`，依赖 `worktools-plugin-api` 和 `tracing = "0.1"`
2. 实现 `Plugin` trait，导出 `#[no_mangle] pub extern "C" fn plugin_create()`
3. 创建 `manifest.json` 和 `assets/` 前端资源
4. 使用 `PluginStorage` 管理持久化数据，`tracing::info!/warn!/error!` 记录关键操作

### 前端插件开发

每个插件有独立的 `frontend/` 目录 (React + Vite)，构建后输出到 `assets/`。插件前端通过 `window.pluginAPI` 与后端通信。**CSS 必须使用 `var(--xxx)` 设计令牌**（如 `var(--bg-primary)`、`var(--text-primary)`），禁止硬编码颜色，以兼容浅色/暗色双主题。

### 前端开发规范

**反馈提示**:
- 操作成功/失败 → `WorkTools.toast.success(msg)` / `.error(msg)` / `.info(msg)` / `.warning(msg)`
- 禁止自行实现 toast 或使用 alert()
- Toast 自动消失 3s，click 可提前关闭，支持多条同时显示

**表单校验**:
- 必须逐字段校验，失焦触发校验，输入时清除本字段错误
- 校验错误显示在本字段下方：`WorkTools.FieldError.show(inputEl, msg)`
- 提交前全量校验，有任一错误不提交
- 禁止用 toast 显示校验错误
- 禁止使用原生 `alert()` 或 `confirm()` 进行用户交互

**CSS 变量**:
- 所有颜色必须使用 `var(--xxx)` 设计令牌，禁止硬编码色值（如 `#c82333`、`#666`、`rgba(0,0,0,0.5)`）
- 按钮统一使用：`.wt-btn--primary` / `.wt-btn--secondary` / `.wt-btn--danger` / `.wt-btn--ghost`
- 模态框统一使用：`.wt-modal-overlay` / `.wt-modal` / `.wt-modal-header` / `.wt-modal-body` / `.wt-modal-footer`
- 空状态：`.wt-empty-state`
- 加载态：按钮内嵌 `.wt-spinner` + disabled 状态

**组件规范**:
- 删除/不可逆操作必须使用 `.wt-modal-*` 确认弹窗
- 提交/导出等异步操作按钮必须有 loading 态（`.wt-spinner` + disabled）
- 表单输入框使用 `.wt-form-input`，标签使用 `.wt-form-label`，容器使用 `.wt-form-group`

## CI/CD

GitHub Actions (`.github/workflows/build.yml`): Tag push (`v*`) 触发多平台构建 — macOS (universal/intel/arm .dmg)、Windows (.msi)、Linux (.deb/.AppImage)，各平台插件包合并为 `plugins-<platform>.zip`，自动创建 GitHub Release。

## Git 提交规范

Conventional Commits: `feat` / `fix` / `refactor` / `style` / `docs` / `test` / `chore`
