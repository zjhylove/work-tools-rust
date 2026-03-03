# Work Tools 安装包

## 📦 安装包列表

### 主应用
- **Work Tools_1.0.0_aarch64.dmg** (5.4 MB)
  - 主应用安装包 (适用于 Apple Silicon Mac)
  - 包含 Work Tools Platform 完整功能
  - 双击安装后拖拽到 Applications 文件夹

### 插件安装包

#### 1. 密码管理器 (password-manager.wtplugin.zip)
- **版本**: 1.0.0
- **大小**: 362 KB
- **功能**: 安全存储和管理密码
- **安装方式**:
  1. 启动 Work Tools 应用
  2. 点击左侧插件商店按钮 (🧩)
  3. 点击"导入插件"按钮
  4. 选择 `password-manager.wtplugin.zip` 文件
  5. 插件将自动安装并出现在侧边栏

#### 2. 双因素验证 (auth.wtplugin.zip)
- **版本**: 1.0.0
- **大小**: 358 KB
- **功能**: TOTP 双因素验证码生成 (支持 Google Authenticator)
- **安装方式**: 同密码管理器

## 🚀 快速开始

### 安装主应用
1. 双击 `Work Tools_1.0.0_aarch64.dmg`
2. 将 Work Tools 拖拽到 Applications 文件夹
3. 在 Launchpad 或 Applications 中启动 Work Tools

### 安装插件
1. 启动 Work Tools 应用
2. 点击左侧的插件商店图标 (🧩)
3. 点击"导入插件"按钮
4. 选择对应的 `.wtplugin.zip` 文件
5. 插件将自动安装到 `~/.worktools/plugins/` 目录

### 使用插件
安装完成后,插件图标会出现在左侧边栏,点击即可使用。

## 📋 系统要求

- **操作系统**: macOS 11.0+ (Big Sur 或更高版本)
- **架构**: Apple Silicon (M1/M2/M3)
- **内存**: 至少 4 GB RAM
- **磁盘空间**: 至少 50 MB 可用空间

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
└── registry.json           # 插件注册表
```

## ❓ 常见问题

### Q: 插件安装后没有显示?
A: 检查 `~/.worktools/plugins/` 目录,确认插件已正确解压。如果问题持续,重启应用。

### Q: Intel Mac 可以使用吗?
A: 当前版本仅支持 Apple Silicon。需要为 Intel Mac 重新编译。

### Q: 如何卸载插件?
A: 删除 `~/.worktools/plugins/<plugin-id>/` 目录并重启应用。

### Q: 数据会丢失吗?
A: 不会。插件数据存储在 `~/.worktools/history/plugins/` 目录,卸载插件不会删除数据。

## 📝 更新日志

### v1.0.0 (2026-03-03)
- ✨ 首次发布
- 🎉 支持密码管理器插件
- 🎉 支持双因素验证插件
- 🔧 修复编译错误
- 📦 优化插件包格式

## 📞 支持

- GitHub: https://github.com/your-repo
- 问题反馈: https://github.com/your-repo/issues

---

**Work Tools Platform** - 一个基于 Tauri + Rust 的可扩展工具平台
