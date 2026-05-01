# Work Tools 安装包

## 📦 安装包列表

### 主应用

- **Work Tools_1.0.0_aarch64.dmg** (5.4 MB)
  - 主应用安装包 (适用于 Apple Silicon Mac)
  - 包含 Work Tools Platform 完整功能
  - 双击安装后拖拽到 Applications 文件夹

### 插件安装包

#### 1. 密码管理器 (password-manager.wtplugin.zip)
- **版本**: 1.0.0 | **大小**: 373 KB
- **功能**: AES-256-GCM 加密存储密码，导入/导出/搜索/过滤，URL 快速打开，剪贴板复制
- **权限**: filesystem, clipboard

#### 2. 双因素验证 (auth.wtplugin.zip)
- **版本**: 1.0.0 | **大小**: 377 KB
- **功能**: TOTP 动态验证码生成 (支持 Google Authenticator)，6/8 位验证码，自动刷新，二维码导入

## 🚀 快速开始

### 安装主应用
1. 双击 `Work Tools_1.0.0_aarch64.dmg`
2. 将 Work Tools 拖拽到 Applications 文件夹
3. 在 Launchpad 或 Applications 中启动 Work Tools

### 安装插件
1. 启动 Work Tools 应用
2. 点击底部工具栏的插件商店图标 (🧩)
3. 点击"导入插件"按钮
4. 选择对应的 `.wtplugin.zip` 文件
5. 插件将自动安装到 `~/.worktools/plugins/` 目录

### 使用插件
安装完成后，插件图标会出现在左侧边栏，点击即可使用。

## 📋 系统要求

- **操作系统**: macOS 11.0+ (Big Sur 或更高)
- **架构**: Apple Silicon (M1/M2/M3)
- **内存**: 至少 4 GB RAM
- **磁盘空间**: 至少 100 MB 可用空间

> **Windows/Linux**: 通过源码构建，见 [开发指南](../README.md#构建发布)。

## 🔧 技术细节

### 插件包格式 (.wtplugin.zip)
```
my-plugin.wtplugin.zip
├── manifest.json          # 插件元数据
├── libmy_plugin.dylib     # 动态库 (macOS)
└── assets/                # 前端资源
    ├── index.html
    ├── main.js
    └── styles.css
```

### 数据存储位置
```
~/.worktools/
├── plugins/                # 已安装的插件
├── history/plugins/        # 插件数据文件
├── config/                 # 应用配置
│   └── installed-plugins.json
├── logs/                   # 日志文件 (按天滚动)
└── registry.json           # 插件注册表
```

## ❓ 常见问题

### Q: 插件安装后没有显示?
A: 检查 `~/.worktools/plugins/` 目录，确认插件已正确解压。如果问题持续，重启应用。

### Q: Intel Mac 可以使用吗?
A: 当前 DMG 仅支持 Apple Silicon。Intel Mac 需通过源码构建。

### Q: 如何卸载插件?
A: 删除 `~/.worktools/plugins/<plugin-id>/` 目录并重启应用。

### Q: 数据会丢失吗?
A: 不会。插件数据存储在 `~/.worktools/history/plugins/` 目录，卸载插件不会删除数据。

### Q: 其他插件在哪里?
A: 本项目共有 8 个插件（密码管理器、双因素验证、JSON 工具、文本比对、数据库文档、K8s IP转发、数据库路由、对象存储）。部分插件可通过源码构建安装。

## 📝 更新日志

### Unreleased
- 新增对象存储、K8s IP转发、数据库路由、数据库文档、文本比对、JSON 工具插件
- 系统托盘功能、系统日志 (三层 tracing 架构)
- 界面设计系统全面升级
- 多项 bug 修复

### v1.0.0 (2026-03-03)
- 首次发布
- 密码管理器插件
- 双因素验证插件

## 📞 支持

- 问题反馈: 提交 Issue 或 Pull Request

---

**Work Tools Platform** - 基于 Tauri 2.x + Rust 的可扩展工具平台
