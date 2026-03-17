# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Work Tools Platform 的 Rust 重写版本 - 一个基于 Tauri + Rust 的可扩展工具平台,采用动态库插件架构。

**核心技术栈**:
- **后端**: Rust + Tauri 2.x
- **前端**: Solid.js + TypeScript + Tailwind CSS
- **插件架构**: 动态库加载 (libloading)
- **插件通信**: 同进程函数调用
- **插件包**: ZIP 格式 (.wtplugin.zip) 包含动态库 + 前端资源
- **数据存储**: JSON 文件,存储在 `~/.worktools/`

## 工作空间结构

这是一个 Cargo workspace,包含以下成员:

```
work-tools-rust/
├── tauri-app/              # Tauri 主应用 (Solid.js 前端 + Rust 后端)
│   ├── src/               # Solid.js 前端源码
│   │   ├── components/    # UI 组件
│   │   │   ├── ContentArea.tsx      # 通用插件容器
│   │   │   ├── PluginView.tsx       # 动态渲染器
│   │   │   └── UiFieldComponent.tsx # UI 组件库
│   │   └── App.tsx        # 主应用 (简化后仅 144 行)
│   └── src-tauri/         # Rust 后端
│       ├── plugin_manager.rs    # 插件管理器 (动态库加载)
│       ├── plugin_package.rs    # 插件包管理 (ZIP 解析)
│       ├── plugin_registry.rs   # 插件注册表
│       ├── commands.rs          # Tauri 命令定义
│       └── crypto.rs            # 密码加密服务
├── plugins/                # 插件项目 (动态库)
│   ├── password-manager/   # 密码管理器
│   ├── json-tools/         # JSON 工具
│   ├── auth-plugin/        # 双因素验证 (TOTP)
│   └── text-diff/          # 文本比对工具 (Monaco Editor)
├── shared/                 # 共享库
│   ├── types/             # 共享数据类型
│   └── plugin-api/        # 插件 API 定义 (Plugin trait)
├── scripts/               # 构建和环境检查脚本
└── docs/plans/            # 开发计划和规范
```

## 常用命令

### 跨平台构建支持

项目完全支持 macOS (Intel/Apple Silicon)、Windows 和 Linux 平台。

**快速环境检查**:
```bash
# macOS/Linux
./scripts/check-env.sh

# Windows PowerShell
.\scripts\check-env.ps1
```

### 开发模式
```bash
cd tauri-app
npm run tauri dev    # 启动开发服务器 (前端热重载 + 后端自动重编译)
```

### 构建测试
```bash
# 测试单个插件
cargo test -p password-manager

# 测试插件 RPC 协议
cargo test -p worktools-rpc-protocol

# 测试插件管理器
cd tauri-app/src-tauri
cargo run --bin test-plugins
```

### 生产构建

#### macOS 构建

```bash
cd tauri-app
npm run tauri build

# 当前架构会自动检测:
# - Intel Mac → x86_64 二进制
# - Apple Silicon M1/M2/M3 → aarch64 二进制

# 构建产物:
# - src-tauri/target/release/bundle/macos/Work Tools.app
# - src-tauri/target/release/bundle/dmg/Work Tools_<version>_x64.dmg
```

**创建通用二进制 (Universal Binary,支持 Intel + Apple Silicon)**:

```bash
# 添加 Intel target
rustup target add x86_64-apple-darwin

# 构建 Intel 版本
cd tauri-app/src-tauri
cargo build --target x86_64-apple-darwin --release

# 构建 Apple Silicon 版本
cargo build --target aarch64-apple-darwin --release

# 合并为通用二进制
lipo -create -output target/release/Work-Tools \
    target/x86_64-apple-darwin/release/Work-Tools \
    target/aarch64-apple-darwin/release/Work-Tools
```

#### Windows 构建

```powershell
cd tauri-app
npm run tauri build

# 构建产物:
# - src-tauri/target/release/bundle/msi/Work Tools_<version>_x64_en-US.msi
# - src-tauri/target/release/bundle/nsis/Work Tools_<version>_x64-setup.exe
```

**前置要求**:
- Visual Studio C++ Build Tools
- WebView2 Runtime (Windows 10/11 通常已预装)

#### Linux 构建

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

#### CI/CD 自动化构建

使用 GitHub Actions 自动构建所有平台版本:

```bash
# 创建 git tag 触发构建
git tag v1.0.0
git push origin v1.0.0

# 或在 GitHub Actions 页面手动触发
```

详见 [.github/workflows/build.yml](.github/workflows/build.yml)

### 插件编译和打包

**⚠️ 重要**: 项目已升级为自动发现构建系统,无需手动修改构建脚本!

#### 一键构建所有插件 (推荐)

```bash
# Linux/macOS
bash scripts/build-plugins.sh

# Windows PowerShell
.\scripts\build-plugins.ps1
```

**功能**:
- ✅ 自动扫描 `plugins/` 目录下的所有插件
- ✅ 读取 `manifest.json` 获取插件配置
- ✅ 构建前端 (如果有 `frontend/` 目录)
- ✅ 编译 Rust 动态库
- ✅ 打包生成 `.wtplugin.zip` 文件

**输出**: 所有插件包会生成在各自的插件目录中:
```
plugins/
├── password-manager/password-manager.wtplugin.zip
├── json-tools/json-tools.wtplugin.zip
├── auth-plugin/auth.wtplugin.zip
└── text-diff/text-diff.wtplugin.zip
```

#### 手动编译单个插件

```bash
cd plugins/password-manager
cargo build --release
```

#### 快速构建脚本

```bash
bash plugins/build-all.sh
```

#### 添加新插件

1. 在 `plugins/` 目录下创建新的插件目录
2. 创建 `manifest.json` 文件
3. 运行 `bash scripts/build-plugins.sh`

**就这么简单!** 脚本会自动发现并构建你的新插件。

详见 [插件构建脚本使用说明](scripts/README-BUILD.md)

### Lint 和格式化
```bash
# Rust 代码格式化
cargo fmt

# Rust 代码检查
cargo clippy

# 前端类型检查
cd tauri-app
npx tsc --noEmit
```

## 架构关键概念

### 插件系统架构

**核心设计**: 插件编译为动态库 (.dylib/.so/.dll),主程序通过 libloading 动态加载,同进程通信。

**插件包格式 (.wtplugin.zip)**:
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

**插件生命周期**:
1. 用户通过插件商店导入 `.wtplugin.zip` 文件
2. 主应用解压插件包到 `~/.worktools/plugins/<plugin-id>/`
3. 扫描并加载动态库文件 (libloading::Library)
4. 获取 `plugin_create` 函数指针并创建插件实例
5. 调用 `init()` 初始化插件
6. 前端加载插件的 `assets/index.html` 并渲染
7. 前端通过 `window.pluginAPI.call()` 调用插件方法

**关键实现文件**:
- [shared/plugin-api/src/lib.rs](shared/plugin-api/src/lib.rs) - Plugin trait 定义
- [tauri-app/src-tauri/src/plugin_manager.rs](tauri-app/src-tauri/src/plugin_manager.rs) - 插件管理器 (动态库加载)
- [tauri-app/src-tauri/src/plugin_package.rs](tauri-app/src-tauri/src/plugin_package.rs) - 插件包解析 (ZIP)
- [tauri-app/src-tauri/src/plugin_registry.rs](tauri-app/src-tauri/src/plugin_registry.rs) - 插件注册表
- [tauri-app/src/components/PluginView.tsx](tauri-app/src/components/PluginView.tsx) - 动态渲染组件

### 数据流向

```
前端 (Solid.js)
  → window.pluginAPI.call(method, params)
  → Tauri: call_plugin_method command
  → PluginManager::call_plugin_method()
  → Plugin::handle_call() (同进程函数调用)
  → 业务逻辑
```

**Tauri Commands 定义**: [tauri-app/src-tauri/src/commands.rs](tauri-app/src-tauri/src/commands.rs)

### 配置管理

所有数据存储在用户主目录下的 `~/.worktools/`:
```
~/.worktools/
├── plugins/                # 已安装的插件
│   ├── password-manager/
│   │   ├── manifest.json
│   │   ├── libpassword_manager.dylib
│   │   └── assets/
│   ├── json-tools/         # JSON 工具
│   └── auth-plugin/
│       ├── manifest.json
│       ├── libauth_plugin.dylib
│       └── assets/
├── history/plugins/        # 插件数据文件
│   ├── password-manager.json
│   └── auth.json
├── config/                 # 应用配置
│   └── app.json
└── registry.json           # 插件注册表
```

配置管理实现: [tauri-app/src-tauri/src/config.rs](tauri-app/src-tauri/src/config.rs)

## 插件开发规范

### 创建新插件 (使用插件包格式)

#### 方式一: 完整插件包 (推荐)

1. 创建插件项目结构:
```bash
mkdir -p my-plugin/{src,assets}
cd my-plugin
cargo init --lib
```

2. 编辑 `Cargo.toml`:
```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
anyhow = "1.0"
```

3. 创建 `manifest.json`:
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

4. 实现 Plugin trait ([参考 password-manager](plugins/password-manager/src/lib.rs)):
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
        // 返回占位符,实际 HTML 在 assets/index.html 中
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

5. 创建前端资源 `assets/index.html`:
```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>我的插件</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <div id="app">
    <h1>我的插件</h1>
    <button onclick="callPlugin()">调用插件方法</button>
  </div>
  <script src="main.js"></script>
</body>
</html>
```

6. 创建 `assets/main.js`:
```javascript
async function callPlugin() {
  const result = await window.pluginAPI.call('my-plugin', 'my_method', {
    param: 'World'
  });
  console.log(result);
}
```

7. 编译并打包:
```bash
# 编译动态库
cargo build --release

# 打包为 .wtplugin.zip
zip -r my-plugin.wtplugin.zip \
  manifest.json \
  target/release/libmy_plugin.dylib \  # macOS
  assets/

# Linux 使用 .so, Windows 使用 .dll
```

8. 安装插件:
- 启动应用: `cd tauri-app && npm run tauri dev`
- 点击插件商店按钮 (🧩)
- 选择 `my-plugin.wtplugin.zip` 导入

#### 方式二: 仅动态库 (旧格式,向后兼容)

```bash
# 编译动态库
cargo build --release

# 直接安装到插件目录
mkdir -p ~/.worktools/plugins/my-plugin
cp target/release/libmy_plugin.dylib ~/.worktools/plugins/my-plugin/  # macOS
cp target/release/libmy_plugin.so ~/.worktools/plugins/my-plugin/     # Linux
cp target/release/my_plugin.dll ~/.worktools/plugins/my-plugin/       # Windows
```

## 已知问题和解决方案

### 1. 插件加载失败
**问题**: 插件未出现在侧边栏
**原因**:
- 动态库文件名不正确 (必须是 lib<name>.dylib/.so/.dll)
- manifest.json 中的文件路径配置错误
- 缺少 plugin_create 导出函数
- Plugin trait 未正确实现

**解决**:
```bash
# 检查插件目录结构
ls -la ~/.worktools/plugins/<plugin-id>/

# 检查 manifest.json
cat ~/.worktools/plugins/<plugin-id>/manifest.json

# 检查导出符号 (macOS)
nm -gU ~/.worktools/plugins/<plugin-id>/lib<name>.dylib | grep plugin_create

# 查看应用日志 (如果启用了日志)
cat ~/.worktools/logs/work-tools.log
```

### 2. 编译错误: Send + Sync 约束
**问题**: `the trait 'Send' is not implemented for 'dyn Plugin'`
**原因**: Plugin trait 需要 Send + Sync 约束以支持多线程

**解决**: 确保 Plugin trait 定义包含 `Send + Sync`:
```rust
pub trait Plugin: Send + Sync {
    // ...
}
```

### 3. 点击无响应
**问题**: 点击插件菜单没有任何反应
**原因**: Solid.js 事件冒泡和默认行为干扰
**解决**: 在 onClick 事件中添加 `preventDefault()` 和 `stopPropagation()`

### 4. UI 配色和交互
- 配色方案: 侧边栏 `#1e1e1e`, 内容区 `#f5f5f5`, 主色调 `#0078d4`
- 所有 onClick 事件需要添加事件阻止和防止文本选择 (`user-select: none`)

## Git 提交规范

使用 Conventional Commits 格式:
- `feat`: 新功能
- `fix`: 修复 bug
- `refactor`: 重构
- `style`: 样式调整
- `docs`: 文档
- `test`: 测试
- `chore`: 构建/工具

## 测试

### 单元测试
```bash
# 测试单个插件
cargo test -p password-manager
cargo test -p auth-plugin

# 测试所有 workspace
cargo test
```

### 插件功能测试
```bash
# 测试所有已安装的插件
./test-all-plugins.sh

# 测试特定插件
./test-password-manager.sh
./test-auth-plugin.sh
```

### 前端类型检查
```bash
cd tauri-app
npx tsc --noEmit
```

## 架构设计原则

### 插件自治
- 插件包含自己的前端资源 (HTML/CSS/JS)
- 主程序不包含插件特定业务逻辑
- 前端通过 `ContentArea` 组件统一加载插件视图

### 通用渲染架构
- [ContentArea.tsx](tauri-app/src/components/ContentArea.tsx): 插件容器,负责加载和初始化
- [PluginView.tsx](tauri-app/src/components/PluginView.tsx): 通用渲染器,遍历 UI Schema
- [UiFieldComponent.tsx](tauri-app/src/components/UiFieldComponent.tsx): UI 组件库,支持多种组件类型

### 数据流向
```
用户操作 → 前端事件处理
  → window.pluginAPI.call(method, params)
  → Tauri: call_plugin_method command
  → PluginManager::call_plugin_method()
  → Plugin::handle_call() (同进程函数调用)
  → 业务逻辑处理
  → 返回结果 → 前端更新视图
```

## 参考资源

- [Tauri 官方文档](https://tauri.app/)
- [Solid.js 文档](https://www.solidjs.com/)
- [libloading 文档](https://docs.rs/libloading/)
- [开发计划和规范](docs/plans/README.md)

## 前端开发注意事项

### Solid.js 特定模式
- 使用 `createEffect` 代替 `onMount` 支持响应式更新
- 所有 `onClick` 事件需要添加 `preventDefault()` 和 `stopPropagation()`
- 使用 CSS `user-select: none` 防止文本选择干扰

### 插件通信
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

## 插件安全最佳实践

### 密码加密
主应用提供基于 AES-256-GCM 的密码加密服务:
```rust
// 在 Tauri commands 中自动加密/解密
// 密码管理器插件存储加密后的密码
// 主应用在返回给前端前自动解密
```

### 数据隔离
- 每个插件有独立的数据文件 (~/.worktools/history/plugins/<plugin-id>.json)
- 插件无法访问其他插件的数据
- 插件无法访问系统文件系统 (除非通过 Tauri API 显式授权)

### 权限管理
manifest.json 中声明权限:
```json
{
  "permissions": [
    "network",     // 网络访问
    "filesystem",  // 文件系统访问
    "clipboard"    // 剪贴板访问
  ]
}
```

## 性能优化

### 插件加载优化
- 插件在应用启动时异步加载,不阻塞主界面
- 动态库使用 `RTLD_LAZY` 延迟绑定符号
- 已加载的插件缓存在内存中,避免重复加载

### 前端渲染优化
- 使用 Solid.js 的细粒度响应式系统
- 插件视图按需加载,仅在选中时渲染
- 大列表使用虚拟滚动 (如果需要)

## 调试技巧

### 查看插件日志
```bash
# 插件使用 tracing 输出日志到 stderr
# 可以通过 Tauri 的开发工具查看
cd tauri-app
npm run tauri dev
```

### 调试动态库加载
```bash
# macOS: 检查动态库依赖
otool -L ~/.worktools/plugins/<plugin-id>/lib<name>.dylib

# Linux: 检查动态库依赖
ldd ~/.worktools/plugins/<plugin-id>/lib<name>.so

# 检查导出符号
nm -gU ~/.worktools/plugins/<plugin-id>/lib<name>.dylib | grep plugin_create
```

### 浏览器开发者工具
在开发模式下,可以使用 Chrome DevTools 调试前端:
```bash
cd tauri-app
npm run tauri dev
# 在打开的窗口中右键 → 检查元素
```

## 已知限制

1. **插件热重载**: 当前不支持插件热重载,修改插件后需要重启应用
2. **前端资源**: 插件前端资源存储在本地,不支持 CDN 加载
3. **跨平台**: 需要为每个平台单独编译动态库
4. **版本兼容**: 插件与主程序的 API 兼容性需要手动维护
