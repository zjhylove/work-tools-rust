# Work Tools Platform (Rust Edition)

> 基于 Tauri 2.x + Rust 的可扩展工具平台，采用完全解耦的动态库插件架构

## 🌟 项目特色

- **🔌 完全解耦架构** - 主程序零插件细节，插件完全独立
- **⚡ 同进程通信** - 插件与主程序在同进程，通过函数直接调用
- **📦 插件包分发** - ZIP 格式打包 (.wtplugin.zip)，包含动态库和前端资源
- **🎨 现代化前端** - React 19 + TypeScript + Vite 6 + Tailwind CSS
- **🔒 安全加密** - AES-256-GCM 加密 + TOTP 双因素验证
- **🚀 跨平台支持** - macOS (Intel/Apple Silicon)、Windows、Linux
- **🖥️ 系统托盘** - 最小化到托盘、后台运行、窗口切换
- **📋 系统日志** - 三层 tracing 架构，文件持久化 + 前端实时查看

## 📋 目录

- [快速开始](#快速开始)
- [插件列表](#插件列表)
- [技术栈](#技术栈)
- [项目结构](#项目结构)
- [开发指南](#开发指南)
- [插件开发](#插件开发)
- [构建发布](#构建发布)
- [架构设计](#架构设计)
- [已知问题](#已知问题)
- [贡献指南](#贡献指南)

---

## 🚀 快速开始

### 环境要求

- **Rust**: 1.70+ (`rustc --version`)
- **Node.js**: 18+ (`node --version`)
- **系统依赖**:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio C++ Build Tools + WebView2 Runtime
  - **Linux**: `libwebkit2gtk-4.1-dev build-essential curl wget file`

### 开发模式

```bash
# 安装前端依赖
cd tauri-app && npm install

# 启动开发服务器 (前端 :1420 热重载 + 后端自动重编译)
npm run tauri dev
```

### 环境检查

```bash
bash scripts/check-env.sh    # macOS/Linux
.\scripts\check-env.ps1      # Windows PowerShell
```

---

## 🔧 插件列表

本项目包含 **8 个** 独立插件：

| 插件 | ID | 图标 | 功能 | 权限 |
|------|-----|------|------|------|
| **密码管理器** | password-manager | 🔐 | AES-256-GCM 加密存储密码，导入/导出/搜索 | filesystem, clipboard |
| **双因素验证** | auth | 🔢 | TOTP 动态验证码，6/8位，二维码导入 | clipboard |
| **JSON 工具** | json-tools | { } | JSON 格式化/压缩/转义，树形可视化编辑 | - |
| **文本比对** | text-diff | 📝 | 并排文本比对，Monaco Editor，字符级差异高亮 | filesystem, clipboard |
| **数据库文档** | db-doc | 📊 | 连接 MySQL/PostgreSQL，生成表结构文档 (Word/Markdown/PDF) | filesystem, network |
| **K8s IP转发** | k8s-forward | 🌐 | Kuboard 发现 Pod，SSH 隧道 + HTTP 代理转发 | filesystem, network |
| **数据库路由** | db-router | 🗄️ | 根据编号解析数据库和表路由 (Rhai 脚本) | filesystem |
| **对象存储** | object-storage | 📦 | 阿里云 OSS + 腾讯云 COS，文件浏览/上传/下载/搜索/删除 | network, filesystem |

---

## 🛠 技术栈

### 后端
- **Rust** 1.70+ - 系统编程语言
- **Tauri** 2.x - 跨平台桌面应用框架
- **libloading** 0.8 - 动态库加载
- **serde** - 序列化/反序列化
- **AES-256-GCM** - 密码加密
- **tracing** 0.1 - 三层日志架构

### 前端
- **React** 19 - UI 框架
- **TypeScript** 5.6 - 类型安全
- **Vite** 6 - 构建工具
- **Tailwind CSS** - 样式框架

### 插件架构
- **动态库** (.dylib/.so/.dll) - 后端逻辑
- **iframe srcdoc** - 前端隔离渲染
- **ZIP** - 插件包格式

---

## 📂 项目结构

```
work-tools-rust/
├── tauri-app/              # Tauri 主应用
│   ├── src/               # React 前端
│   │   ├── components/    # UI 组件
│   │   │   ├── ErrorBoundary.tsx       # 错误边界
│   │   │   ├── LogViewer.tsx           # 日志查看器
│   │   │   ├── PluginPlaceholder.tsx   # 插件通用加载器 (iframe)
│   │   │   └── PluginStore.tsx         # 插件商店
│   │   ├── App.tsx        # 主应用 (完全解耦)
│   │   └── main-react.tsx # React 入口
│   └── src-tauri/src/     # Rust 后端
│       ├── lib.rs              # 应用初始化
│       ├── commands.rs         # 16 个 Tauri 命令
│       ├── plugin_manager.rs   # 动态库加载、插件生命周期
│       ├── plugin_package.rs   # .wtplugin.zip 解析安装
│       ├── plugin_registry.rs  # 插件注册表管理
│       ├── logger.rs           # 日志系统 (tracing 三层架构)
│       ├── tray.rs             # 系统托盘管理
│       └── config.rs           # 插件配置持久化
├── plugins/                # 8 个插件
│   ├── password-manager/   # 密码管理器
│   ├── auth-plugin/        # 双因素验证 (TOTP)
│   ├── json-tools/         # JSON 工具
│   ├── text-diff/          # 文本比对 (Monaco Editor)
│   ├── db-doc/             # 数据库文档生成
│   ├── k8s-forward/        # K8s 端口转发
│   ├── db-router/          # 数据库路由解析
│   └── object-storage/     # 对象存储 (OSS + COS)
├── shared/
│   ├── types/             # 共享数据类型
│   └── plugin-api/        # Plugin trait + storage/error/tracing
├── scripts/               # 构建脚本
├── release/               # 发布产物
└── Cargo.toml             # Workspace 配置
```

---

## 💻 开发指南

### 常用命令

```bash
# Rust 检查 (首选，比 build 快得多)
cargo check                      # workspace 全部类型检查
cargo check -p password-manager  # 单个 crate

# 测试
cargo test                       # 全部测试
cargo test -p password-manager   # 单个插件测试
cargo test -p db-router -- test_execute  # 按名称过滤

# 代码质量
cargo fmt                        # 格式化
cargo clippy                     # lint

# 前端类型检查
cd tauri-app && npx tsc --noEmit

# 构建
cd tauri-app && npm run tauri build   # 生产构建
bash scripts/build-plugins.sh          # 一键编译打包所有插件
```

### 配置与数据

所有数据存储在 `~/.worktools/`:

```
~/.worktools/
├── plugins/                # 已安装的插件
├── history/plugins/        # 插件数据文件
├── config/                 # 应用配置
│   └── installed-plugins.json
├── logs/                   # 日志文件 (按天滚动)
└── registry.json           # 插件注册表
```

---

## 🔌 插件开发

### 插件包格式 (.wtplugin.zip)

```
my-plugin.wtplugin.zip
├── manifest.json          # 插件元数据
├── libmy_plugin.dylib     # 动态库 (macOS)
├── libmy_plugin.so        # 动态库 (Linux)
├── my_plugin.dll          # 动态库 (Windows)
└── assets/                # 前端资源
    ├── index.html
    ├── main.js
    └── styles.css
```

### 创建新插件步骤

#### 1. 创建项目

```bash
mkdir -p plugins/my-plugin/{src,assets,frontend/src}
cd plugins/my-plugin && cargo init --lib
```

#### 2. 配置 `Cargo.toml`

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
tracing = "0.1"
```

#### 3. 创建 `manifest.json`

```json
{
  "id": "my-plugin",
  "name": "我的插件",
  "description": "插件描述",
  "version": "1.0.0",
  "icon": "🔧",
  "author": "Your Name",
  "files": {
    "macos": "libmy_plugin.dylib",
    "linux": "libmy_plugin.so",
    "windows": "my_plugin.dll"
  },
  "assets": { "entry": "index.html" }
}
```

#### 4. 实现 Plugin trait

```rust
use worktools_plugin_api::Plugin;
use serde_json::Value;

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn id(&self) -> &str { "my-plugin" }
    fn name(&self) -> &str { "我的插件" }
    fn description(&self) -> &str { "插件描述" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "🔧" }

    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "my_method" => Ok(serde_json::json!({ "result": "ok" })),
            _ => Err("unknown method".into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(MyPlugin));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
```

#### 5. 前端开发

插件前端通过 `window.pluginAPI` 与后端通信：

```typescript
// 调用后端方法
const result = await window.pluginAPI.call('my-plugin', 'my_method', { param: 'value' });

// 读写插件配置
const config = await window.pluginAPI.get_plugin_config('my-plugin');
await window.pluginAPI.set_plugin_config('my-plugin', { key: 'value' });
```

#### 6. 编译打包

```bash
cargo build --release
bash scripts/build-plugins.sh
```

---

## 🏗 构建发布

### macOS

```bash
cd tauri-app && npm run tauri build
# 产物: target/release/bundle/dmg/Work Tools_*.dmg
```

### Windows

```powershell
cd tauri-app; npm run tauri build
# 产物: .msi 和 .exe 安装包
```

### Linux

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget \
    libssl-dev libayatana-appindicator3-dev librsvg2-dev
cd tauri-app && npm run tauri build
# 产物: .deb 和 .AppImage
```

---

## 🎨 架构设计

### 插件渲染机制

插件前端通过 **iframe srcdoc** 渲染：

1. `PluginPlaceholder` 读取已安装插件的 `index.html`、`main.js`、`styles.css`
2. 内联到 HTML 字符串注入 iframe 的 srcdoc
3. iframe 加载后注入 `window.pluginAPI` 对象

### 数据流

```
前端 iframe → window.pluginAPI.call(pluginId, method, params)
  → Tauri command: call_plugin_method
  → PluginManager::call_plugin_method()
  → Plugin::handle_call(method, params)
  → 返回 JSON 结果
```

### 日志系统

三层 `tracing_subscriber::registry()` 架构：

| 层 | 输出 | 用途 |
|---|---|---|
| fmt::layer (stdout) | 控制台 | 开发调试，带 ANSI 颜色 |
| fmt::layer (non_blocking_file) | `~/.worktools/logs/` 按天滚动 | 持久化，无颜色 |
| LogRingLayer | `LOG_RING` (Mutex<VecDeque>, 1000条) | 前端查询 |

### 完全解耦设计

- 主程序 = 空壳框架，零插件细节
- 新增插件 = 打包上传，无需修改主程序
- 插件完全独立，自包含所有逻辑
- 同进程函数调用，无 IPC 开销

---

## ❓ 已知问题

### 插件加载失败

**症状**: 插件未出现在侧边栏

**排查**:
```bash
# 检查插件目录结构
ls -la ~/.worktools/plugins/<plugin-id>/

# 检查 manifest.json
cat ~/.worktools/plugins/<plugin-id>/manifest.json

# 检查导出符号 (macOS)
nm -gU ~/.worktools/plugins/<plugin-id>/lib<name>.dylib | grep plugin_create
```

### 编译错误: Send + Sync 约束

确保 Plugin trait 定义包含 `Send + Sync`:
```rust
pub trait Plugin: Send + Sync { /* ... */ }
```

---

## 📚 文档

- **[CLAUDE.md](CLAUDE.md)** - 项目详细说明 (AI 指令)
- **[CHANGELOG.md](CHANGELOG.md)** - 变更日志
- **[release/BUILD_SUMMARY.md](release/BUILD_SUMMARY.md)** - 构建说明
- **[plugins/README-INSTALL.md](plugins/README-INSTALL.md)** - 插件安装说明
- **[scripts/README.md](scripts/README.md)** - 脚本使用说明

---

## 🤝 贡献指南

### Git 提交规范 (Conventional Commits)

- `feat` - 新功能 | `fix` - 修复 | `refactor` - 重构
- `style` - 样式 | `docs` - 文档 | `test` - 测试 | `chore` - 构建/工具

### 开发流程

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'feat: add AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request

---

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

---

**最后更新**: 2026-05-01
**维护者**: Work Tools Team
