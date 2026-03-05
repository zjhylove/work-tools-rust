# Work Tools Platform (Rust Edition)

> 基于 Tauri + Rust 的可扩展工具平台,采用完全解耦的动态库插件架构

## 🌟 项目特色

- **🔌 完全解耦架构** - 主程序零插件细节,插件完全独立
- **⚡ 同进程通信** - 插件与主程序在同进程,通过函数直接调用
- **📦 插件包分发** - ZIP 格式打包 (.wtplugin.zip),包含动态库和前端资源
- **🎨 现代化前端** - 使用 React 19 + TypeScript + Tailwind CSS
- **🔒 安全加密** - 基于 AES-256-GCM 的密码加密服务
- **🚀 跨平台支持** - 支持 macOS (Intel/Apple Silicon)、Windows、Linux

## 📋 目录

- [快速开始](#快速开始)
- [下载安装](#下载安装)
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

## 📥 下载安装

### macOS 安装包

**版本**: 1.0.0
**平台**: macOS Apple Silicon (M1/M2/M3)
**大小**: 5.4 MB
**下载**: [Work Tools_1.0.0_aarch64.dmg](release/Work Tools_1.0.0_aarch64.dmg)

#### 安装步骤

1. 下载 `Work Tools_1.0.0_aarch64.dmg`
2. 双击打开 DMG 文件
3. 将 "Work Tools.app" 拖到 "Applications" 文件夹
4. 从启动台启动应用

### 插件安装

#### 密码管理器

**版本**: 1.0.0
**大小**: 373 KB
**下载**: [password-manager.wtplugin.zip](release/password-manager.wtplugin.zip)

**功能**:
- 本地安全存储密码
- 导入/导出功能 (JSON 格式)
- 密码搜索和过滤
- URL 快速打开
- 剪贴板复制

#### 双因素验证

**版本**: 1.0.0
**大小**: 377 KB
**下载**: [auth.wtplugin.zip](release/auth.wtplugin.zip)

**功能**:
- TOTP (Time-based One-Time Password) 支持
- 6 位和 8 位验证码
- 自动刷新倒计时
- 二维码扫描导入
- 剪贴板复制

#### JSON 工具

**版本**: 1.0.0
**大小**: 642 KB
**位置**: [plugins/json-tools/](plugins/json-tools/)

**功能**:
- JSON 格式化和压缩
- JSON 转义和去转义
- 树形视图可视化
- 节点选择和删除
- 全展开/全折叠
- 实时 JSON 语法验证

#### 安装插件

**方式一: 通过插件商店** (推荐)

1. 启动 Work Tools 应用
2. 点击底部工具栏的 **🧩** 按钮
3. 选择对应的 `.wtplugin.zip` 文件导入

**方式二: 手动安装**

```bash
# 创建插件目录
mkdir -p ~/.worktools/plugins/password-manager
mkdir -p ~/.worktools/plugins/auth

# 解压插件包
unzip release/password-manager.wtplugin.zip -d ~/.worktools/plugins/password-manager/
unzip release/auth.wtplugin.zip -d ~/.worktools/plugins/auth/
```

---

## 🛠 技术栈

### 后端
- **Rust** 1.70+ - 系统编程语言
- **Tauri** 2.10.2 - 跨平台桌面应用框架
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
- **React 组件** - 前端界面
- **ZIP** - 插件包格式

---

## 📂 项目结构

```
work-tools-rust/
├── tauri-app/              # Tauri 主应用
│   ├── src/               # React 前端源码
│   │   ├── components/    # UI 组件
│   │   │   ├── ErrorBoundary.tsx      # 错误边界
│   │   │   ├── LogViewer.tsx          # 日志查看器
│   │   │   ├── PluginPlaceholder.tsx  # 插件通用加载器
│   │   │   ├── PluginStore.tsx        # 插件商店
│   │   │   ├── Dialog.css             # 对话框样式
│   │   │   └── PluginStore.css        # 插件商店样式
│   │   ├── utils/         # 工具函数
│   │   │   ├── logger.ts              # 日志工具
│   │   │   └── pluginBridge.ts        # 插件通信桥
│   │   ├── assets/        # 静态资源
│   │   ├── App.tsx        # 主应用 (13 KB, 完全解耦)
│   │   ├── App.css        # 全局样式
│   │   └── main-react.tsx # React 入口
│   └── src-tauri/         # Rust 后端
│       ├── src/
│       │   ├── plugin_manager.rs       # 插件管理器 (8 KB)
│       │   ├── plugin_package.rs       # 插件包解析 (6 KB)
│       │   ├── plugin_registry.rs      # 插件注册表 (7.5 KB)
│       │   ├── commands.rs             # Tauri 命令 (24 KB)
│       │   ├── crypto.rs               # 加密服务 (5 KB)
│       │   ├── config.rs               # 配置管理 (4 KB)
│       │   ├── lib.rs                  # 库入口
│       │   └── main.rs                 # 主函数
│       └── Cargo.toml
├── plugins/                # 插件项目
│   ├── password-manager/   # 密码管理器
│   │   ├── src/           # Rust 动态库源码
│   │   ├── assets/        # 前端资源 (构建后)
│   │   ├── frontend/      # React 前端源码
│   │   └── manifest.json  # 插件配置
│   └── auth-plugin/        # 双因素验证
│       ├── src/           # Rust 动态库源码
│       ├── assets/        # 前端资源 (构建后)
│       ├── frontend/      # React 前端源码
│       └── manifest.json  # 插件配置
├── shared/                 # 共享库
│   ├── types/             # 共享数据类型
│   └── plugin-api/        # 插件 API 定义
├── scripts/               # 构建脚本
│   ├── build-plugins.sh   # 插件打包脚本
│   └── check-env.sh       # 环境检查脚本
├── release/               # 发布产物
│   ├── Work Tools_1.0.0_aarch64.dmg     # macOS 安装包
│   ├── password-manager.wtplugin.zip    # 密码管理器插件
│   ├── auth.wtplugin.zip                # 双因素验证插件
│   └── BUILD_SUMMARY.md                 # 构建说明
├── docs/                  # 文档
│   ├── CLEANUP_OPTIMIZATION_GUIDE.md
│   └── fixes/             # 修复记录
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
./scripts/build-plugins.sh
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

#### 5. 创建前端

在 `frontend/` 目录创建 React 应用:

```typescript
// frontend/src/main.tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

```typescript
// frontend/src/App.tsx
import React, { useState, useEffect } from 'react';

declare global {
  interface Window {
    pluginAPI: {
      call: (pluginId: string, method: string, params: any) => Promise<any>;
    };
  }
}

export default function App() {
  const [data, setData] = useState<any>(null);

  useEffect(() => {
    // 调用插件后端方法
    window.pluginAPI.call('my-plugin', 'my_method', { param: 'World' })
      .then(result => setData(result));
  }, []);

  return (
    <div>
      <h1>My Plugin</h1>
      <pre>{JSON.stringify(data, null, 2)}</pre>
    </div>
  );
}
```

#### 6. 编译并打包

```bash
# 编译动态库
cargo build --release

# 使用打包脚本
cd ../..
./scripts/build-plugins.sh
```

---

## 🏗 构建发布

### 清理缓存

```bash
# 清理所有构建缓存
cargo clean
rm -rf tauri-app/dist/
```

### macOS 构建

```bash
cd tauri-app
npm run tauri build

# 构建产物:
# - target/release/bundle/macos/Work Tools.app
# - target/release/bundle/dmg/Work Tools_1.0.0_aarch64.dmg
```

**当前版本**:
- 架构: aarch64 (Apple Silicon M1/M2/M3)
- 大小: 5.4 MB
- 位置: `release/Work Tools_1.0.0_aarch64.dmg`

### Windows 构建

```powershell
cd tauri-app
npm run tauri build

# 构建产物:
# - src-tauri/target/release/bundle/msi/Work Tools_1.0.0_x64_en-US.msi
# - src-tauri/target/release/bundle/nsis/Work Tools_1.0.0_x64-setup.exe
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
# - src-tauri/target/release/bundle/deb/work-tools_1.0.0_amd64.deb
# - src-tauri/target/release/bundle/appimage/work-tools_1.0.0_amd64.AppImage
```

---

## 🎨 架构设计

### 完全解耦架构

**主程序 (App.tsx - 13 KB)**:
```
├── 插件加载器 (PluginPlaceholder)
├── 插件商店 (PluginStore)
├── 错误边界 (ErrorBoundary)
├── 日志查看器 (LogViewer)
└── 新增插件无需修改主程序
```

**插件 (独立包)**:
```
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

**优势**:
- ✅ 主程序 = 空壳框架,零插件细节
- ✅ 新增插件 = 打包上传,无需修改主程序
- ✅ 插件完全独立,自包含所有逻辑
- ✅ 可扩展性极强

### 数据流向

```
前端 (React)
  → window.pluginAPI.call(method, params)
  → Tauri: call_plugin_method command
  → PluginManager::call_plugin_method()
  → Plugin::handle_call() (同进程函数调用)
  → 业务逻辑
```

### 核心文件说明

#### 主应用

| 文件 | 大小 | 作用 |
|------|------|------|
| **App.tsx** | 13 KB | 主应用,完全解耦,零插件细节 |
| **PluginPlaceholder.tsx** | 8 KB | 通用插件加载器,动态加载任何插件 |
| **PluginStore.tsx** | 5.6 KB | 插件商店,管理插件导入导出 |
| **commands.rs** | 24 KB | Tauri 命令,前后端通信接口 |
| **plugin_manager.rs** | 8 KB | 插件管理器,动态库加载 |
| **plugin_package.rs** | 6 KB | 插件包解析,ZIP 解压 |
| **plugin_registry.rs** | 7.5 KB | 插件注册表,管理插件元数据 |

#### 插件

| 插件 | 动态库 | 前端 | 功能 |
|------|--------|------|------|
| **密码管理器** | libpassword_manager.dylib | React | 密码存储管理 |
| **双因素验证** | libauth_plugin.dylib | React | TOTP 验证码 |

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

- **[CLAUDE.md](CLAUDE.md)** - 项目详细说明 (给 AI 的指令)
- **[CHANGELOG.md](CHANGELOG.md)** - 项目变更日志
- **[release/BUILD_SUMMARY.md](release/BUILD_SUMMARY.md)** - 构建说明
- **[docs/CLEANUP_OPTIMIZATION_GUIDE.md](docs/CLEANUP_OPTIMIZATION_GUIDE.md)** - 清理优化指南

---

## 🔗 相关资源

- [Tauri 官方文档](https://tauri.app/)
- [React 文档](https://react.dev/)
- [Rust 官方文档](https://www.rust-lang.org/)
- [libloading 文档](https://docs.rs/libloading/)

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

## 📮 联系方式

如有问题或建议,欢迎提 Issue 或 Pull Request。

---

**当前版本**: 1.0.0
**最后更新**: 2026-03-04
**维护者**: Work Tools Team
