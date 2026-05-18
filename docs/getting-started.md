# 开发环境搭建 (Getting Started)

本文档介绍如何搭建 Work Tools Platform 的开发环境并完成首次运行。

## 环境要求

| 工具 | 最低版本 | 说明 |
|------|---------|------|
| Rust | 1.70+ | 推荐使用 `rustup` 安装 |
| Node.js | 22+ | 前端构建依赖 |
| Tauri CLI | 2.x | 通过 npm 安装 |
| 操作系统 | macOS / Linux / Windows | 三平台均支持 |

### macOS 额外依赖

```bash
xcode-select --install
```

### Linux 额外依赖 (Ubuntu/Debian)

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

### Windows 额外依赖

- [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (Windows 10 1803+ 通常已内置)

## 步骤 1: 克隆仓库

```bash
git clone https://github.com/your-org/work-tools-rust.git
cd work-tools-rust
```

## 步骤 2: 安装前端依赖

```bash
cd tauri-app
npm install
cd ..
```

## 步骤 3: 编译插件

一键编译所有插件的 Rust 动态库 + 前端资源并打包：

```bash
bash scripts/build-plugins.sh
```

该脚本会自动：
1. 检查构建环境 (cargo, zip)
2. 编译所有 Rust 动态库 (`cargo build --release`)
3. 扫描 `plugins/` 目录，读取每个插件的 `manifest.json`
4. 构建前端（如果存在 `frontend/` 目录）
5. 打包生成 `.wtplugin.zip` 文件

> Windows 用户使用 PowerShell: `.\scripts\build-plugins.ps1`

也可以单独编译某个插件：

```bash
cargo build --release -p password-manager
```

## 步骤 4: 启动开发服务器

```bash
cd tauri-app && npm run tauri dev
```

启动后：
- 前端开发服务器运行在 `http://localhost:1420`
- Rust 后端由 Tauri 管理自动编译和热重载
- 首次启动可能较慢，需要编译所有 Rust 依赖

## 常用开发命令

### Rust 开发

```bash
# 类型检查 -- 比 build 快得多，开发时首选
cargo check

# 检查单个 crate
cargo check -p password-manager

# 运行全部测试
cargo test

# 运行单个插件的测试
cargo test -p password-manager

# 按模块名过滤测试
cargo test -p password-manager -- crypto

# 按测试名过滤
cargo test -p db-router -- test_execute

# 代码格式化
cargo fmt

# Lint 检查
cargo clippy
```

### 前端开发

```bash
cd tauri-app

# TypeScript 类型检查
npx tsc --noEmit

# 生产构建
npm run tauri build
```

### 插件前端构建

```bash
# 单个插件前端构建
cd plugins/text-diff/frontend
npm install
npm run build
```

### 完整构建

```bash
# 构建 Tauri 主应用（生产模式）
cd tauri-app && npm run tauri build

# 编译+打包所有插件
bash scripts/build-plugins.sh
```

## 项目结构速览

```
work-tools-rust/
├── tauri-app/              # Tauri 主应用
│   ├── src/               # React 前端 (TypeScript + Vite)
│   └── src-tauri/src/     # Rust 后端
├── plugins/                # 插件目录（每个插件独立 crate）
│   └── <plugin-name>/
│       ├── src/           # Rust 后端
│       ├── frontend/      # React 前端（独立 Vite 项目）
│       ├── assets/        # 构建后的前端产物
│       └── manifest.json  # 插件元数据
├── shared/
│   ├── types/             # 共享数据类型 (PluginInfo 等)
│   └── plugin-api/        # Plugin trait + PluginStorage + 错误处理
└── scripts/               # 构建/打包脚本
```

## FAQ / 常见问题

### 1. `cargo check` 报错找不到某个 crate

确保在项目根目录运行。根 `Cargo.toml` 定义了整个 workspace，包含 `tauri-app/src-tauri`、所有插件和共享库。

```bash
# 确认 workspace members
cargo metadata --format-version 1 | jq '.workspace_members | length'
```

### 2. 插件加载失败 / 插件列表为空

检查以下几点：

1. **插件是否已编译**：确认 `target/release/` 下存在对应的动态库文件（`.dylib` / `.so` / `.dll`）
2. **插件是否已安装**：检查 `~/.worktools/plugins/` 下是否有插件目录
3. **manifest.json 是否存在**：已安装插件的目录内应有 `manifest.json`
4. **查看日志**：启动应用后在日志面板查看 `WARN` 级别信息

```bash
# 检查已安装的插件
ls ~/.worktools/plugins/

# 检查插件注册表
cat ~/.worktools/config/installed-plugins.json
```

### 3. 前端白屏或报错

1. 确认 `npm install` 已执行且无错误
2. 确认 Node.js 版本 >= 22
3. 尝试清除缓存重新安装：

```bash
cd tauri-app
rm -rf node_modules package-lock.json
npm install
```

### 4. 前端插件构建失败

```bash
# 进入插件前端目录
cd plugins/<plugin-name>/frontend

# 单独安装依赖并测试构建
npm install
npm run build
```

### 5. macOS 上出现 "无法打开动态库" 错误

macOS 可能阻止加载未签名的动态库。在终端执行：

```bash
# 移除隔离属性
xattr -cr ~/.worktools/plugins/
```

### 6. Windows 上 DLL 被锁定无法删除

这是 Windows 的文件锁机制，应用通过带重试的删除机制处理（最多 3 次，递增延迟）。卸载插件时应用会先释放 DLL 句柄再删除文件。如果仍然失败，重启应用后重试。

### 7. 动态库编译失败

```bash
# 确认 Rust 工具链
rustc --version
cargo --version

# 单独编译出错的插件查看完整错误
cargo build --release -p <plugin-name>
```
