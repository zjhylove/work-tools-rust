# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

- **[2026-03-03]** 全新的项目主 README.md
- **[2026-03-03]** 代码清理优化指南 (docs/CLEANUP_OPTIMIZATION_GUIDE.md)
- **[2026-03-03]** 清理总结文档 (docs/fixes/2026-03-03-cleanup-summary.md)

### Changed

- **[2026-03-03]** 增强 .gitignore 规则
- **[2026-03-03]** 清理冗余文件和过时文档

### Removed

- **[2026-03-03]** 删除 Solid.js 迁移备份文件
- **[2026-03-03]** 删除过时的测试脚本 (旧架构)
- **[2026-03-03]** 删除过时的文档

### Fixed

- **[2026-03-03]** 密码管理器 UI/UX 改进
- **[2026-03-03]** 导入/导出功能优化
- **[2026-03-03]** DOM 移除错误修复
- **[2026-03-03]** Toast 通知优化
- **[2026-03-03]** 注册 open_url 命令

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
