# Work Tools Platform - Rust Edition

一个基于 Tauri + Rust 的可扩展工具平台,从 Java 版本完全复刻。

## 项目概述

这是 Work Tools Platform 的 Rust 重写版本,采用现代化的技术栈实现相同的功能:

- ✅ **插件化架构**: 支持动态加载插件,无需重新编译主程序
- ✅ **现代化 UI**: 使用 Solid.js + Tailwind CSS 构建
- ✅ **高性能**: 利用 Rust 的性能优势,启动速度快,内存占用低
- ✅ **跨平台**: 支持 Windows、macOS、Linux

## 技术栈

| 层级 | 技术 |
|------|------|
| **GUI 框架** | Tauri 2.x |
| **前端** | Solid.js + TypeScript + Tailwind CSS |
| **后端** | Rust |
| **插件架构** | 独立进程 + JSON-RPC |
| **配置存储** | JSON 文件 |

## 项目结构

```
work-tools-rust/
├── tauri-app/              # Tauri 主应用
│   ├── src/               # Solid.js 前端
│   └── src-tauri/         # Rust 后端
├── plugins/               # 插件项目
│   └── password-manager/  # 密码管理器插件
├── shared/                # 共享类型定义
│   ├── types/            # 共享数据类型
│   └── rpc-protocol/     # JSON-RPC 协议
└── docs/                 # 文档
```

## 开发环境设置

### 跨平台支持

✅ **macOS** (Intel 和 Apple Silicon M1/M2/M3)
✅ **Windows** (x64)
✅ **Linux** (Ubuntu/Debian/Fedora)

### 环境检查

在开始之前,运行环境检查脚本:

```bash
# macOS/Linux
./scripts/check-env.sh

# Windows PowerShell
.\scripts\check-env.ps1
```

### 依赖

**所有平台**:
- Rust 1.70+
- Node.js 20+
- npm 或 yarn

**macOS**:
- Xcode Command Line Tools (`xcode-select --install`)

**Windows**:
- Visual Studio C++ Build Tools
- WebView2 Runtime (Windows 10/11 通常已预装)

**Linux** (Ubuntu/Debian):
```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget \
    libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

### 安装

```bash
# 1. 克隆项目
git clone <repository-url>
cd work-tools-rust

# 2. 安装前端依赖
cd tauri-app
npm install

# 3. 开发模式运行
npm run tauri dev
```

## 编译和发布

### 本地构建

#### macOS

```bash
cd tauri-app
npm run tauri build

# 当前架构会自动检测:
# - Intel Mac → x86_64 二进制 (.app 和 .dmg)
# - Apple Silicon → aarch64 二进制

# 构建产物:
# - target/release/bundle/macos/Work Tools.app
# - target/release/bundle/dmg/Work Tools_<version>_x64.dmg
```

**创建通用二进制 (Universal Binary)**:
```bash
# 添加 Intel target
rustup target add x86_64-apple-darwin

# 构建双架构版本
cd src-tauri
cargo build --target x86_64-apple-darwin --release
cargo build --target aarch64-apple-darwin --release

# 合并
lipo -create -output target/release/Work-Tools \
    target/x86_64-apple-darwin/release/Work-Tools \
    target/aarch64-apple-darwin/release/Work-Tools
```

#### Windows

```powershell
cd tauri-app
npm run tauri build

# 构建产物:
# - target/release/bundle/msi/Work Tools_<version>_x64_en-US.msi
# - target/release/bundle/nsis/Work Tools_<version>_x64-setup.exe
```

#### Linux

```bash
# 先安装依赖
sudo apt install libwebkit2gtk-4.1-dev build-essential \
    curl wget libssl-dev libayatana-appindicator3-dev librsvg2-dev

# 构建
cd tauri-app
npm run tauri build

# 构建产物:
# - target/release/bundle/deb/work-tools_<version>_amd64.deb
# - target/release/bundle/appimage/work-tools_<version>_amd64.AppImage
```

### CI/CD 自动化构建

使用 GitHub Actions 自动构建所有平台:

```bash
# 推送 tag 触发构建
git tag v1.0.0
git push origin v1.0.0
```

构建完成后,所有平台的安装包会自动上传到 GitHub Releases。

详见 [.github/workflows/build.yml](.github/workflows/build.yml)

### 编译插件

```bash
# 编译单个插件
cd plugins/password-manager
cargo build --release

# 安装插件
mkdir -p ~/.worktools/plugins/password-manager
cp target/release/password-manager ~/.worktools/plugins/password-manager/
```

## 插件开发

### 创建新插件

1. 创建插件项目:
```bash
mkdir -p plugins/my-plugin/src
cd plugins/my-plugin
cargo init --bin
```

2. 编辑 `Cargo.toml`:
```toml
[dependencies]
worktools-shared-types = { path = "../../shared/types" }
worktools-rpc-protocol = { path = "../../shared/rpc-protocol" }
serde_json = "1.0"
anyhow = "1.0"
```

3. 实现插件主函数:
```rust
use worktools_rpc_protocol::RpcServer;

fn main() -> anyhow::Result<()> {
    let mut rpc_server = RpcServer::new();

    // 注册 RPC 方法
    rpc_server.register_handler("get_info", |_params| {
        Ok(serde_json::json!({
            "id": "my-plugin",
            "name": "我的插件",
            "version": "1.0.0"
        }))
    });

    // 处理 stdin/stdout 通信...
    Ok(())
}
```

4. 编译并安装:
```bash
cargo build --release
mkdir -p ~/.worktools/plugins/my-plugin
cp target/release/my-plugin ~/.worktools/plugins/my-plugin/
```

## 配置文件

配置文件存储在 `~/.worktools/`:

```
~/.worktools/
├── config/
│   └── settings.json      # 应用配置
├── history/
│   └── plugins/           # 插件配置
└── logs/
    └── work-tools.log     # 日志文件
```

## 已实现功能

### 核心框架
- ✅ Tauri 应用框架
- ✅ 插件管理器
- ✅ JSON-RPC 通信层
- ✅ 配置管理系统
- ✅ 前端 UI 框架
- ✅ UI Schema 动态渲染

### 插件
- ✅ password-manager (密码管理器)

### 待开发插件
- ⏳ auth (认证工具)
- ⏳ db-doc (数据库文档)
- ⏳ ip-forward (IP 转发)
- ⏳ object-storage (对象存储)
- ⏳ ai-chat (AI 聊天)
- ⏳ api-doc (API 文档)
- ⏳ db-router (数据库路由)

## 与 Java 版本对比

| 指标 | Java 版本 | Rust 版本 |
|------|----------|----------|
| 启动时间 | ~3-5 秒 | ~0.5-1 秒 |
| 内存占用 | ~200-300 MB | ~50-100 MB |
| 安装包大小 | ~80 MB | ~10-15 MB |

## 开发路线图

- [x] 核心框架搭建
- [x] 第一个插件(password-manager)
- [ ] 插件市场功能
- [ ] 日志查看器
- [ ] 主题切换
- [ ] 所有插件迁移完成
- [ ] 性能优化
- [ ] 端到端测试

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request!
