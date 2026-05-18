# 发布流程

本文档面向维护者，说明如何发布 Work Tools 新版本。

## 版本号规范

遵循 [Semantic Versioning (SemVer)](https://semver.org/)：`MAJOR.MINOR.PATCH`

- **MAJOR**：不兼容的 API 变更
- **MINOR**：向后兼容的功能新增
- **PATCH**：向后兼容的问题修复

版本号定义在根 `Cargo.toml` 的 `[workspace.package]` 中：

```toml
[workspace.package]
version = "1.0.0"
```

所有 workspace members 共享此版本号。发布前需同步更新以下位置：
1. `/Cargo.toml` -- `workspace.package.version`
2. `/tauri-app/src-tauri/tauri.conf.json` -- `version`
3. `/tauri-app/package.json` -- `version`（如存在）

## 发布前检查清单

### 1. 代码质量检查

```bash
# 运行全部 workspace 测试（2 shared + 13 plugins）
cargo test

# 运行 Clippy lint
cargo clippy -- -D warnings

# 格式化检查
cargo fmt --check

# TypeScript 类型检查
cd tauri-app && npx tsc --noEmit
```

确保以上命令全部通过，无错误和警告。

### 2. 功能验证

```bash
# 启动开发服务器进行手动测试
cd tauri-app && npm run tauri dev
```

验证要点：
- 应用正常启动，侧边栏显示所有内置插件
- 插件导入/卸载功能正常
- 浅色/暗色主题切换正常
- 各插件核心功能可正常使用

### 3. 构建验证

```bash
# Release 构建
cargo build --release

# 插件打包
bash scripts/build-plugins.sh

# Tauri 生产构建
cd tauri-app && npm run tauri build
```

## 更新 CHANGELOG

在项目根目录维护 `CHANGELOG.md`，记录每个版本的变更：

```markdown
## [1.x.x] - YYYY-MM-DD

### Added
- 新增 xxx 功能

### Changed
- 修改 xxx 行为

### Fixed
- 修复 xxx 问题

### Removed
- 移除 xxx
```

## 创建发布 Tag

确认所有检查通过后，创建版本 tag 并推送：

```bash
# 创建 tag
git tag v1.x.x

# 推送 tag 到远程
git push origin v1.x.x
```

Tag 命名规则：`v` 前缀 + 版本号，例如 `v1.0.0`、`v1.1.0`、`v2.0.0`。

## CI 自动构建

推送 tag 后，GitHub Actions（`.github/workflows/build.yml`）自动触发构建流程。

### 构建矩阵

| 平台 | Runner | Target | 产物 |
|------|--------|--------|------|
| macOS (Apple Silicon) | `macos-latest` | `aarch64-apple-darwin` | `.dmg` |
| macOS (Intel) | -- 见下方说明 -- | -- | -- |
| Windows | `windows-latest` | `x86_64-pc-windows-msvc` | `.msi` |
| Linux | `ubuntu-latest` | `x86_64-unknown-linux-gnu` | `.deb` + `.AppImage` |

### 构建步骤

1. **Checkout** -- 拉取代码
2. **Install Rust** -- 安装 Rust 工具链和指定 target
3. **Install Node.js** -- 安装 Node 22
4. **Install system deps** -- Linux 安装 webkit2gtk 等依赖
5. **Install frontend deps** -- `npm ci`（主应用 + 所有插件前端）
6. **Build Rust workspace** -- `cargo build --release`（编译所有插件 + Tauri 后端）
7. **Package plugins** -- 执行 `scripts/build-plugins.sh`（或 Windows 下的 PowerShell 版本）
8. **Bundle plugins** -- 将所有 `.wtplugin.zip` 打包为 `plugins-<platform>.zip`
9. **Build Tauri app** -- 使用 `tauri-apps/tauri-action` 构建安装包
10. **Upload artifacts** -- 上传构建产物，保留 14 天

### 插件包打包

`scripts/build-plugins.sh` 工作流程：

1. 检查构建环境（cargo, zip）
2. 编译所有 Rust 动态库（`cargo build --release`）
3. 扫描 `plugins/` 目录下的所有插件
4. 对每个插件：
   - 构建前端（如存在 `frontend/` 目录）
   - 从 `manifest.json` 读取动态库文件名
   - 打包 `manifest.json` + 动态库 + `assets/` 为 `.wtplugin.zip`
5. 输出统计信息

产物命名规则：`<plugin-id>-<platform>.wtplugin.zip`

平台标识：`macos-arm` / `macos-intel` / `linux` / `windows`

## GitHub Release 自动创建

构建全部完成后，`release` job 自动执行：

1. 下载所有平台的构建产物
2. 使用 `softprops/action-gh-release@v2` 创建 GitHub Release
3. 自动附加以下文件：
   - `**/*.dmg` -- macOS 安装包
   - `**/*.msi` -- Windows 安装包
   - `**/*.exe` -- Windows 可执行文件
   - `**/*.deb` -- Linux Debian 包
   - `**/*.AppImage` -- Linux AppImage
   - `**/plugins-*.zip` -- 插件包合集
4. Release 标题为 tag 名称（如 `v1.0.0`）
5. 自动生成 Release Notes（基于 commit 历史）
6. Release 默认为正式版（非 draft、非 prerelease）

## 发布后验证

### 1. 检查 GitHub Release

- 确认 Release 已创建，标题和 tag 正确
- 确认所有平台的安装包和插件包都已附加
- 验证 Release Notes 内容完整

### 2. 下载验证

从 GitHub Release 页面下载各平台安装包，执行以下检查：

- [ ] macOS: `.dmg` 可正常安装、启动、加载插件
- [ ] Windows: `.msi` 可正常安装、启动、加载插件
- [ ] Linux: `.deb` / `.AppImage` 可正常安装/运行、加载插件
- [ ] `plugins-<platform>.zip` 可正常解压、导入

### 3. 功能冒烟测试

- 应用启动正常
- 内置插件列表显示正确
- 主题切换正常
- 插件导入/卸载正常

## 手动触发构建

如果需要在不创建 tag 的情况下触发构建，可在 GitHub Actions 页面使用 `workflow_dispatch` 手动触发。此模式只构建产物，不会创建 Release。
