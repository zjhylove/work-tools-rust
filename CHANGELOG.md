# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

- **对象存储插件** — 支持阿里云 OSS 和腾讯云 COS，文件浏览/上传/下载/搜索/删除
- **K8s IP转发插件** — Kuboard DEX SSO 发现 Pod，SSH 隧道 + HTTP 代理转发，24 个 handle_call 方法，3 Tab 前端
- **数据库路由插件** — Rhai 脚本引擎解析数据库和表路由，丰富内置函数，双栏前端布局
- **数据库文档插件** — MySQL/PostgreSQL 表结构提取，Markdown/Word 导出，步骤导航，Toast 通知
- **文本比对插件** — Monaco Editor 并排比对，字符级差异高亮与统计
- **JSON 工具插件** — JSON 格式化/压缩/转义，树形可视化编辑
- **系统托盘功能** — 最小化到托盘、首次使用提示、双击窗口切换
- **系统日志** — tracing 三层架构，文件持久化按天滚动，前端实时查看，8 插件全覆盖
- **文件夹对话框 API** — 插件前端可调用 `open_folder_dialog()`
- **CI 插件构建** — GitHub Actions 自动编译插件并发布 `.wtplugin.zip`

### Changed

- **界面设计系统全面升级** — 统一 8 个插件的视觉风格
- **日志代码简化** — 优化性能，新增清理功能
- **托盘模块简化**
- **db-doc 重构** — 提取公共模式，统一导出流程
- **平台库查找逻辑** — 消除脆弱模式

### Fixed

- 修复应用图标、路径重复、托盘退出报错
- 修复代码质量审查发现的问题
- 修复插件图标冲突、auth 列表样式互换
- 修复 Windows 上插件卸载前删除目录的问题
- 修复 db-doc MySQL/PostgreSQL 连接超时（添加 3 秒超时）
- 修复 db-router 前端与后端 API 不匹配

### Removed

- db-doc PDF 导出（CJK 字体限制）
- db-doc Enterprise 模板变体
- 过时文档和计划文件

### Documentation

- 全部 Rust 源码添加中文学习注释
- CLAUDE.md 多次更新（命令、架构、插件数量）
- 多个插件设计和实现计划文档

---

## [1.0.0] - 2026-03-01

### Added

- **密码管理器插件**
  - 本地安全存储密码
  - 导入/导出功能 (JSON 格式)
  - 密码搜索和过滤
  - URL 快速打开
  - 剪贴板复制

- **双因素验证插件**
  - TOTP (Time-based One-Time Password) 支持
  - 6 位和 8 位验证码
  - 自动刷新倒计时
  - 二维码扫描导入

- **插件系统**
  - 动态库加载 (libloading)
  - 插件包格式 (.wtplugin.zip)
  - 插件商店 UI
  - 插件导入/导出

- **核心功能**
  - AES-256-GCM 密码加密
  - 插件配置管理
  - 系统日志查看器
  - 跨平台支持 (macOS/Windows/Linux)

### Technology Stack

- **前端**: React 19 + TypeScript + Tailwind CSS
- **后端**: Rust + Tauri 2.x
- **插件架构**: 动态库 (同进程通信)
- **构建工具**: Vite 6.0 + Cargo

### Documentation

- 完整的 CLAUDE.md (项目说明)
- 插件开发指南
- 架构设计文档
- 测试指南

---

## 发布流程

1. 更新 CHANGELOG.md
2. 更新版本号 (Cargo.toml, package.json)
3. 创建 git tag
4. 构建发布包
5. 发布到 GitHub Releases

---

**格式说明**:
- `Added` - 新增功能
- `Changed` - 功能变更
- `Deprecated` - 即将废弃的功能
- `Removed` - 已删除的功能
- `Fixed` - 问题修复
- `Security` - 安全相关修复
