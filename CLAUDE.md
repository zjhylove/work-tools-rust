# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Work Tools Platform - 基于 Tauri 2.x + Rust 的可扩展工具平台，采用动态库插件架构。插件编译为 cdylib，主程序通过 libloading 同进程加载。

**技术栈**: Rust + Tauri 2.x | React 19 + TypeScript + Vite 6 | libloading 动态库 | JSON 文件存储

## 常用命令

```bash
# 开发
cd tauri-app && npm run tauri dev          # 启动开发服务器 (前端 :1420)

# 测试
cargo test                                 # 全部 workspace 测试
cargo test -p password-manager             # 单个插件测试

# 前端检查
cd tauri-app && npx tsc --noEmit           # TypeScript 类型检查

# Rust 检查
cargo fmt && cargo clippy

# 构建
cd tauri-app && npm run tauri build        # 生产构建 (macOS: .app + .dmg)
bash scripts/build-plugins.sh              # 一键构建所有插件
cd plugins/<name> && cargo build --release  # 单个插件编译

# 插件打包产物位于: plugins/<name>/<name>.wtplugin.zip
```

## 架构

### 整体结构

```
work-tools-rust/
├── tauri-app/              # Tauri 主应用
│   ├── src/               # React 前端
│   └── src-tauri/src/     # Rust 后端
│       ├── lib.rs              # 应用初始化、Tauri builder
│       ├── commands.rs         # 12 个 Tauri 命令
│       ├── plugin_manager.rs   # 动态库加载、插件生命周期
│       ├── plugin_package.rs   # .wtplugin.zip 解析安装
│       ├── plugin_registry.rs  # 插件注册表管理
│       └── config.rs           # 插件配置持久化
├── plugins/                # 5 个插件 (各有独立 frontend/ + React/Vite)
│   ├── password-manager/   # 密码管理器 (AES 加密)
│   ├── json-tools/         # JSON 工具
│   ├── auth-plugin/        # 双因素验证 (TOTP)
│   ├── text-diff/          # 文本比对 (Monaco Editor)
│   └── db-doc/             # 数据库文档生成 (MySQL/PostgreSQL, Word/Markdown 导出)
├── shared/
│   ├── types/             # 共享数据类型
│   └── plugin-api/        # Plugin trait + storage/error 辅助模块
└── scripts/               # build-plugins.sh 等构建脚本
```

### 插件渲染机制

插件前端通过 **iframe srcdoc** 渲染，不是直接嵌入 React 组件：
1. `PluginPlaceholder` 组件读取已安装插件的 `index.html`、`main.js`、`styles.css`
2. 将它们内联到一个 HTML 字符串注入 iframe 的 srcdoc
3. iframe 加载后注入 `window.pluginAPI` 对象，提供：`call()`、`get_plugin_config()`、`set_plugin_config()`、`open_url()`、`open_folder_dialog()`

### 数据流

```
前端 iframe → window.pluginAPI.call(pluginId, method, params)
  → Tauri command: call_plugin_method
  → PluginManager::call_plugin_method()
  → Plugin::handle_call(method, params)
  → 返回 JSON 结果
```

### 插件系统关键设计

**Plugin trait** (`shared/plugin-api/src/lib.rs`):
- 必须实现: `id()`、`name()`、`description()`、`version()`、`icon()`、`get_view()`
- 可选覆盖: `init()`、`destroy()`、`handle_call()`、`get_assets_path()`
- 辅助模块: `PluginStorage` (JSON 文件持久化)、`PluginError` / `method_error!` / `param_error!` (错误处理)
- 插件必须编译为 `cdylib`，导出 `plugin_create()` 函数
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

根 `Cargo.toml` 使用 `exclude = ["tauri-app"]` 但同时将 `tauri-app/src-tauri` 列为 workspace member。`cargo test` 在根目录运行时会测试所有 workspace members 包括 tauri 后端。

## 插件开发要点

新建插件需要：
1. `Cargo.toml` 中 `crate-type = ["cdylib"]`，依赖 `worktools-plugin-api`
2. 实现 `Plugin` trait，导出 `#[no_mangle] pub extern "C" fn plugin_create()`
3. 创建 `manifest.json` 和 `assets/` 前端资源
4. 使用 `PluginStorage` 管理持久化数据，`PluginError` / 宏处理错误

### 前端插件开发

每个插件有独立的 `frontend/` 目录 (React + Vite)，构建后输出到 `assets/`。插件前端通过 `window.pluginAPI` 与后端通信。

## CI/CD

GitHub Actions (`.github/workflows/build.yml`): Tag push (`v*`) 触发多平台构建 — macOS (universal/intel/arm .dmg)、Windows (.msi)、Linux (.deb/.AppImage)，自动创建 GitHub Release。

## Git 提交规范

Conventional Commits: `feat` / `fix` / `refactor` / `style` / `docs` / `test` / `chore`
