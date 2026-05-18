# Work Tools Platform

> 基于 Tauri 2.x + Rust 的可扩展桌面工具平台，采用动态库插件架构，插件热加载、同进程零 IPC 开销

[![Rust](https://img.shields.io/badge/Rust-1.70+-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Vite](https://img.shields.io/badge/Vite-6-646CFF?logo=vite&logoColor=white)](https://vite.dev/)

## Feature Highlights

- **Dynamic Library Plugin Architecture** -- plugins compiled as cdylib, loaded via libloading in-process, zero IPC overhead
- **Hot-loadable Plugin Packages** -- .wtplugin.zip format with manifest, dynamic library, and frontend assets; install without rebuilding the host
- **13 Built-in Plugins** -- password manager, Redis client, database router, object storage, K8s port forwarding, and more
- **Dual Theme System** -- light/dark themes driven by CSS design tokens, propagated to plugin iframes via postMessage
- **Three-layer Logging** -- console output, daily-rotating file logs, and an in-memory ring buffer for frontend log viewer
- **Cross-platform** -- macOS (Intel / Apple Silicon), Windows, and Linux builds via GitHub Actions CI/CD

## Quick Start

```bash
# 1. Clone the repository
git clone https://github.com/zjliaosun/work-tools-rust.git
cd work-tools-rust

# 2. Verify Rust compilation
cargo check

# 3. Install frontend dependencies and launch dev server
cd tauri-app && npm install && npm run tauri dev
```

The dev server starts the frontend on `http://localhost:1420` with hot reload, and automatically recompiles the Rust backend on changes.

## Plugin List

| Icon | Plugin | Description |
|------|--------|-------------|
| 🔐 | **password-manager** | AES-256-GCM encrypted credential storage with search, import/export, and clipboard support |
| 🔧 | **json-tools** | JSON formatting, minification, escaping, and tree-view visual editor |
| 🔑 | **auth-plugin** | TOTP two-factor authentication codes (6/8 digit), QR code import |
| 📝 | **text-diff** | Side-by-side text comparison with Monaco Editor and character-level diff highlighting |
| 📊 | **db-doc** | Connect to MySQL/PostgreSQL and generate table structure documentation (Word/Markdown/PDF) |
| 🔌 | **k8s-forward** | Kubernetes Pod discovery via Kuboard, SSH tunnel + HTTP proxy port forwarding |
| 🔀 | **db-router** | Parse database and table routes from IDs using Rhai scripts |
| 📦 | **object-storage** | Alibaba Cloud OSS + Tencent Cloud COS file browsing, upload, download, search, and delete |
| ⏰ | **timestamp-converter** | Unix timestamp conversion with multi-timezone support and batch processing |
| ⏱️ | **cron-tools** | Cron expression parsing, validation, and execution schedule visualization |
| 🔴 | **redis-client** | Redis key browsing and multi-type value operations |
| 📄 | **api-doc** | API documentation generation from Spring Boot JAR analysis |

## Documentation

Full documentation is available in the [docs/](docs/) directory and [CLAUDE.md](CLAUDE.md).

## Build & Release

```bash
# Production build (Tauri host application)
cd tauri-app && npm run tauri build

# Build all plugins and package as .wtplugin.zip
bash scripts/build-plugins.sh

# Single plugin release build
cargo build --release -p password-manager
```

Tag pushes (`v*`) trigger GitHub Actions multi-platform builds: macOS (.dmg), Windows (.msi), Linux (.deb / .AppImage), with plugin packages bundled per platform.

## Screenshots

<!-- TODO: Add application screenshots here -->
<!-- Recommended: main window with sidebar, plugin in action, theme switching demo -->

## License

Licensed under the [Apache License 2.0](LICENSE).

Copyright 2024-2026 Work Tools Contributors.
