# Work Tools Platform (Rust Edition)

> 基于 Tauri + Rust 的可扩展工具平台,采用动态库插件架构

## 🌟 项目特色

- **🔌 动态库插件架构** - 插件编译为动态库,主程序通过 libloading 动态加载
- **⚡ 同进程通信** - 插件与主程序在同进程,通过函数直接调用,性能优异
- **🎨 现代化前端** - 使用 React 19 + TypeScript + Tailwind CSS
- **🔒 安全加密** - 基于 AES-256-GCM 的密码加密服务
- **📦 插件包格式** - ZIP 格式打包 (.wtplugin.zip),包含动态库和前端资源
- **🚀 跨平台支持** - 支持 macOS (Intel/Apple Silicon)、Windows、Linux

## 📋 目录

- [快速开始](#快速开始)
- [技术栈](#技术栈)
- [项目结构](#项目结构)
- [开发指南](#开发指南)
- [插件开发](#插件开发)
- [构建发布](#构建发布)
- [架构设计](#架构设计)
- [测试](#测试)
- [已知问题](#已知问题)
- [贡献指南](#贡献指南)

---

## 🚀 快速开始

### 环境要求

- **Rust**: 1.70+ (运行 `rustc --version` 检查)
- **Node.js**: 18+ (运行 `node --version` 检查)
- **系统依赖**:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio C++ Build Tools + WebView2 Runtime
  - **Linux**: `libwebkit2gtk-4.1-dev build-essential curl wget file`

### 快速环境检查

```bash
# macOS/Linux
./scripts/check-env.sh

# Windows PowerShell
.\scripts\check-env.ps1
```

### 安装依赖

```bash
# 安装前端依赖
cd tauri-app
npm install

# 构建工作空间
cargo build
```

### 开发模式

```bash
cd tauri-app
npm run tauri dev
```

这将启动开发服务器,前端热重载 + 后端自动重编译。

### 导入插件

1. 启动应用后,点击底部工具栏的 **🧩** 按钮
2. 选择插件包文件导入:
   - [release/password-manager.wtplugin.zip](release/password-manager.wtplugin.zip)
   - [release/auth.wtplugin.zip](release/auth.wtplugin.zip)

---

## 🛠 技术栈

### 后端
- **Rust** 1.70+ - 系统编程语言
- **Tauri** 2.x - 跨平台桌面应用框架
- **libloading** 0.8 - 动态库加载
- **serde** - 序列化/反序列化
- **AES-256-GCM** - 密码加密

### 前端
- **React** 19.2 - UI 框架
- **TypeScript** 5.6 - 类型安全
- **Vite** 6.0 - 构建工具
- **Tailwind CSS** - 样式框架

### 插件架构
- **动态库** (.dylib/.so/.dll) - 后端逻辑
- **HTML/CSS/JS** - 前端界面
- **ZIP** - 插件包格式

---

## 📂 项目结构

```
work-tools-rust/
├── tauri-app/              # Tauri 主应用
│   ├── src/               # React 前端源码
│   │   ├── components/    # UI 组件
│   │   │   ├── PasswordManager.tsx    # 密码管理器
│   │   │   ├── AuthPlugin.tsx         # 双因素验证
│   │   │   ├── PluginView.tsx         # 动态渲染器
│   │   │   ├── PluginStore.tsx        # 插件商店
│   │   │   └── Sidebar.tsx            # 侧边栏
│   │   ├── utils/         # 工具函数
│   │   │   ├── pluginRegistry.ts      # 插件注册表
│   │   │   ├── pluginBridge.ts        # 插件通信桥
│   │   │   └── logger.ts              # 日志工具
│   │   └── App.tsx        # 主应用入口
│   └── src-tauri/         # Rust 后端
│       ├── src/
│       │   ├── plugin_manager.rs    # 插件管理器
│       │   ├── plugin_package.rs    # 插件包解析
│       │   ├── plugin_registry.rs   # 插件注册表
│       │   ├── commands.rs          # Tauri 命令
│       │   ├── crypto.rs            # 加密服务
│       │   └── config.rs            # 配置管理
│       └── Cargo.toml
├── plugins/                # 插件项目
│   ├── password-manager/   # 密码管理器
│   │   ├── src/           # Rust 动态库源码
│   │   ├── assets/        # 前端资源
│   │   └── frontend/      # 前端构建源码
│   └── auth-plugin/        # 双因素验证
│       ├── src/
│       ├── assets/
│       └── frontend/
├── shared/                 # 共享库
│   ├── types/             # 共享数据类型
│   └── plugin-api/        # 插件 API 定义
├── scripts/               # 构建和环境检查脚本
├── docs/                  # 文档
│   ├── plans/             # 开发计划
│   ├── fixes/             # 修复记录
│   └── testing/           # 测试文档
├── release/               # 发布产物
│   ├── *.wtplugin.zip     # 插件包
│   └── *.dmg              # 安装包
└── Cargo.toml            # Workspace 配置
```

---

## 💻 开发指南

### 常用命令

#### 开发模式
```bash
cd tauri-app
npm run tauri dev    # 前端热重载 + 后端自动重编译
```

#### 测试
```bash
# 测试单个插件
cargo test -p password-manager

# 测试所有 workspace
cargo test

# 前端类型检查
cd tauri-app
npx tsc --noEmit
```

#### 构建
```bash
# 开发构建
cd tauri-app
npm run tauri build

# 构建插件
cd plugins
./build-all.sh
```

#### Lint 和格式化
```bash
# Rust 代码格式化
cargo fmt

# Rust 代码检查
cargo clippy

# 前端类型检查
cd tauri-app
npx tsc --noEmit
```

### 配置管理

所有数据存储在用户主目录下的 `~/.worktools/`:

```
~/.worktools/
├── plugins/                # 已安装的插件
│   ├── password-manager/
│   │   ├── manifest.json
│   │   ├── libpassword_manager.dylib
│   │   └── assets/
│   └── auth-plugin/
├── history/plugins/        # 插件数据文件
│   ├── password-manager.json
│   └── auth.json
├── config/                 # 应用配置
│   └── app.json
└── registry.json           # 插件注册表
```

---

## 🔌 插件开发

### 插件包格式

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

### 创建新插件

#### 1. 创建插件项目

```bash
mkdir -p plugins/my-plugin/{src,assets,frontend/src}
cd plugins/my-plugin
cargo init --lib
```

#### 2. 编辑 `Cargo.toml`

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
anyhow = "1.0"
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
  "homepage": "https://github.com/your/repo",
  "files": {
    "macos": "libmy_plugin.dylib",
    "linux": "libmy_plugin.so",
    "windows": "my_plugin.dll"
  },
  "assets": {
    "entry": "index.html"
  }
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

    fn get_view(&self) -> String {
        "<div>插件前端资源加载中...</div>".to_string()
    }

    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "my_method" => {
                let param = params.get("param").and_then(|v| v.as_str()).unwrap_or("");
                Ok(serde_json::json!({ "result": format!("Hello, {}", param) }))
            }
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

#### 5. 编译并打包

```bash
# 编译动态库
cargo build --release

# 打包为 .wtplugin.zip
zip -r my-plugin.wtplugin.zip \
  manifest.json \
  target/release/libmy_plugin.dylib \
  assets/

# 复制到 release 目录
cp my-plugin.wtplugin.zip ../../release/
```

#### 6. 测试插件

启动应用,通过插件商店导入 `my-plugin.wtplugin.zip`。

### 插件通信

前端通过 `window.pluginAPI.call()` 调用插件方法:

```typescript
// 调用插件方法
const result = await window.pluginAPI.call(
  pluginId,      // 插件 ID,如 "password-manager"
  method,        // 方法名,如 "list_passwords"
  params         // 参数对象,如 {}
);

// 读取插件配置
const config = await window.pluginAPI.get_plugin_config(pluginId);

// 保存插件配置
await window.pluginAPI.set_plugin_config(pluginId, config);
```

---

## 🏗 构建发布

### macOS 构建

```bash
cd tauri-app
npm run tauri build

# 构建产物:
# - src-tauri/target/release/bundle/macos/Work Tools.app
# - src-tauri/target/release/bundle/dmg/Work Tools_<version>_aarch64.dmg
```

**创建通用二进制 (Universal Binary,支持 Intel + Apple Silicon)**:

```bash
# 添加 Intel target
rustup target add x86_64-apple-darwin

# 构建 Intel 版本
cargo build --target x86_64-apple-darwin --release

# 构建 Apple Silicon 版本
cargo build --target aarch64-apple-darwin --release

# 合并为通用二进制
lipo -create -output target/release/Work-Tools \
    target/x86_64-apple-darwin/release/Work-Tools \
    target/aarch64-apple-darwin/release/Work-Tools
```

### Windows 构建

```powershell
cd tauri-app
npm run tauri build

# 构建产物:
# - src-tauri/target/release/bundle/msi/Work Tools_<version>_x64_en-US.msi
# - src-tauri/target/release/bundle/nsis/Work Tools_<version>_x64-setup.exe
```

### Linux 构建

```bash
# 安装依赖 (Ubuntu/Debian)
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget \
    libssl-dev libayatana-appindicator3-dev librsvg2-dev

# 构建
cd tauri-app
npm run tauri build

# 构建产物:
# - src-tauri/target/release/bundle/deb/work-tools_<version>_amd64.deb
# - src-tauri/target/release/bundle/appimage/work-tools_<version>_amd64.AppImage
```

---

## 🎨 架构设计

### 插件系统架构

```
主程序 (App.tsx)
├── 插件加载器 (动态导入)
├── 插件注册表 (运行时发现)
├── 通信桥梁 (统一接口)
└── 新增插件无需修改主程序

插件 (独立包)
├── 前端组件 (React + TypeScript)
│   ├── 界面逻辑
│   ├── 校验逻辑
│   └── 状态管理
├── 后端逻辑 (Rust 动态库)
│   ├── 业务逻辑
│   ├── 数据存储
│   └── 加密解密
└── 插件配置 (manifest.json)
```

### 数据流向

```
前端 (React)
  → window.pluginAPI.call(method, params)
  → Tauri: call_plugin_method command
  → PluginManager::call_plugin_method()
  → Plugin::handle_call() (同进程函数调用)
  → 业务逻辑
```

### 核心设计原则

1. **插件自治** - 插件包含自己的前端资源和后端逻辑
2. **通用渲染** - 主程序不包含插件特定业务逻辑
3. **完全解耦** - 新增插件无需修改主程序代码

---

## 🧪 测试

### 单元测试

```bash
# 测试单个插件
cargo test -p password-manager
cargo test -p auth-plugin

# 测试所有 workspace
cargo test
```

### 功能测试

```bash
# 测试所有已安装的插件
./scripts/test-all-plugins.sh

# 测试特定插件
./scripts/test-password-manager.sh
./scripts/test-auth-plugin.sh
```

### 前端类型检查

```bash
cd tauri-app
npx tsc --noEmit
```

---

## ❓ 已知问题

### 1. 插件加载失败

**问题**: 插件未出现在侧边栏

**原因**:
- 动态库文件名不正确 (必须是 lib<name>.dylib/.so/.dll)
- manifest.json 中的文件路径配置错误
- 缺少 plugin_create 导出函数

**解决**:
```bash
# 检查插件目录结构
ls -la ~/.worktools/plugins/<plugin-id>/

# 检查 manifest.json
cat ~/.worktools/plugins/<plugin-id>/manifest.json

# 检查导出符号 (macOS)
nm -gU ~/.worktools/plugins/<plugin-id>/lib<name>.dylib | grep plugin_create
```

### 2. 编译错误: Send + Sync 约束

**问题**: `the trait 'Send' is not implemented for 'dyn Plugin'`

**解决**: 确保 Plugin trait 定义包含 `Send + Sync`:
```rust
pub trait Plugin: Send + Sync {
    // ...
}
```

### 3. 点击无响应

**问题**: 点击插件菜单没有任何反应

**解决**: 在 onClick 事件中添加 `preventDefault()` 和 `stopPropagation()`

---

## 📚 文档

- **[CLAUDE.md](CLAUDE.md)** - 项目详细说明 (给 Claude Code 的指令)
- **[docs/plans/](docs/plans/)** - 开发计划和架构设计
- **[docs/fixes/](docs/fixes/)** - 问题修复记录
- **[docs/testing/](docs/testing/)** - 测试文档

---

## 🤝 贡献指南

### Git 提交规范

使用 Conventional Commits 格式:

- `feat`: 新功能
- `fix`: 修复 bug
- `refactor`: 重构
- `style`: 样式调整
- `docs`: 文档
- `test`: 测试
- `chore`: 构建/工具

### 开发流程

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'feat: add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request

---

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

---

## 🔗 相关资源

- [Tauri 官方文档](https://tauri.app/)
- [React 文档](https://react.dev/)
- [Rust 官方文档](https://www.rust-lang.org/)
- [libloading 文档](https://docs.rs/libloading/)

---

## 📮 联系方式

如有问题或建议,欢迎提 Issue 或 Pull Request。

---

**当前版本**: 1.0.0
**最后更新**: 2026-03-03
**维护者**: ZhengJun
