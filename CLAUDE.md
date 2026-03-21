# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Work Tools Platform - 基于 Tauri + Rust 的可扩展工具平台,采用动态库插件架构。

**核心技术栈**:
- **后端**: Rust + Tauri 2.x
- **前端**: React 19 + TypeScript
- **插件架构**: 动态库加载 (libloading)
- **插件包**: ZIP 格式 (.wtplugin.zip) 包含动态库 + 前端资源
- **数据存储**: JSON 文件,存储在 `~/.worktools/`

## 工作空间结构

```
work-tools-rust/
├── tauri-app/              # Tauri 主应用
│   ├── src/               # React 前端源码
│   │   ├── components/    # UI 组件
│   │   ├── types/         # TypeScript 类型定义
│   │   └── utils/         # 工具函数
│   └── src-tauri/         # Rust 后端
│       ├── plugin_manager.rs    # 插件管理器
│       ├── plugin_package.rs    # 插件包解析
│       ├── commands.rs          # Tauri 命令
│       └── crypto.rs            # 密码加密服务
├── plugins/                # 插件项目
│   ├── password-manager/   # 密码管理器
│   ├── json-tools/         # JSON 工具
│   ├── auth-plugin/        # 双因素验证 (TOTP)
│   └── text-diff/          # 文本比对 (Monaco Editor)
├── shared/                 # 共享库
│   ├── types/             # 共享数据类型
│   └── plugin-api/        # 插件 API 定义
└── scripts/               # 构建脚本
```

## 常用命令

### 开发模式
```bash
cd tauri-app
npm run tauri dev    # 启动开发服务器
```

### 构建和测试
```bash
# 测试单个插件
cargo test -p password-manager

# 测试所有 workspace
cargo test

# 前端类型检查
cd tauri-app && npx tsc --noEmit

# Rust 格式化和 lint
cargo fmt && cargo clippy
```

### 生产构建
```bash
cd tauri-app
npm run tauri build

# macOS 构建产物:
# - src-tauri/target/release/bundle/macos/Work Tools.app
# - src-tauri/target/release/bundle/dmg/Work Tools_<version>_x64.dmg
```

### 插件编译和打包
```bash
# 一键构建所有插件
bash scripts/build-plugins.sh

# 手动编译单个插件
cd plugins/password-manager && cargo build --release
```

构建产物位于各插件目录: `plugins/<name>/<name>.wtplugin.zip`

## 架构关键概念

### 插件系统架构

**核心设计**: 插件编译为动态库,主程序通过 libloading 动态加载,同进程通信。

**插件包格式 (.wtplugin.zip)**:
```
my-plugin.wtplugin.zip
├── manifest.json          # 插件元数据
├── libmy_plugin.dylib     # macOS 动态库
├── libmy_plugin.so        # Linux 动态库
├── my_plugin.dll          # Windows 动态库
└── assets/                # 前端资源
    ├── index.html
    ├── main.js
    └── styles.css
```

**插件生命周期**:
1. 导入 `.wtplugin.zip` → 解压到 `~/.worktools/plugins/<plugin-id>/`
2. 加载动态库 → 获取 `plugin_create` 函数指针
3. 创建插件实例 → 调用 `init()` 初始化
4. 前端加载 `assets/index.html` → 通过 `window.pluginAPI.call()` 调用后端

**关键实现文件**:
- `shared/plugin-api/src/lib.rs` - Plugin trait 定义
- `tauri-app/src-tauri/src/plugin_manager.rs` - 动态库加载
- `tauri-app/src-tauri/src/plugin_package.rs` - ZIP 解析

### 数据流向

```
前端 (React)
  → window.pluginAPI.call(pluginId, method, params)
  → Tauri: call_plugin_method command
  → PluginManager::call_plugin_method()
  → Plugin::handle_call()
  → 返回结果
```

### 配置和数据存储

```
~/.worktools/
├── plugins/                # 已安装的插件
├── history/plugins/        # 插件数据文件
├── config/app.json         # 应用配置
└── registry.json           # 插件注册表
```

## 插件开发

### 创建新插件

1. 创建项目结构:
```bash
mkdir -p my-plugin/{src,assets}
cd my-plugin && cargo init --lib
```

2. 配置 `Cargo.toml`:
```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
```

3. 创建 `manifest.json` 并实现 Plugin trait

4. 编译并打包:
```bash
cargo build --release
zip -r my-plugin.wtplugin.zip manifest.json target/release/libmy_plugin.dylib assets/
```

### Plugin trait 实现

```rust
use worktools_plugin_api::Plugin;
use serde_json::Value;

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn id(&self) -> &str { "my-plugin" }
    fn name(&self) -> &str { "我的插件" }
    fn description(&self) -> &str { "描述" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "🔧" }

    fn get_view(&self) -> String {
        "<div>加载中...</div>".to_string()
    }

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

## 调试技巧

### 查看插件日志
```bash
cd tauri-app && npm run tauri dev
# 插件日志输出到 stderr
```

### 检查动态库
```bash
# macOS: 检查导出符号
nm -gU ~/.worktools/plugins/<plugin-id>/lib<name>.dylib | grep plugin_create

# 检查依赖
otool -L ~/.worktools/plugins/<plugin-id>/lib<name>.dylib
```

### 前端调试
开发模式下右键 → 检查元素,使用 Chrome DevTools

## 常见问题

### 插件加载失败
- 检查动态库文件名: `lib<name>.dylib` / `.so` / `.dll`
- 检查 manifest.json 配置
- 验证 `plugin_create` 导出函数存在

### 编译错误: Send + Sync
确保 Plugin trait 包含 `Send + Sync`:
```rust
pub trait Plugin: Send + Sync { ... }
```

## Git 提交规范

使用 Conventional Commits: `feat` / `fix` / `refactor` / `style` / `docs` / `test` / `chore`
