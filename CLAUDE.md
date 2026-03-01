# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Work Tools Platform 的 Rust 重写版本 - 一个基于 Tauri + Rust 的可扩展工具平台,采用插件化架构。

**核心技术栈**:
- **后端**: Rust + Tauri 2.x
- **前端**: Solid.js + TypeScript + Tailwind CSS
- **插件通信**: JSON-RPC 2.0 over stdin/stdout (独立进程通信)
- **数据存储**: JSON 文件,存储在 `~/.worktools/`

## 工作空间结构

这是一个 Cargo workspace,包含以下成员:

```
work-tools-rust/
├── tauri-app/              # Tauri 主应用 (Solid.js 前端 + Rust 后端)
├── plugins/                # 插件项目 (独立可执行文件)
│   ├── password-manager/   # 密码管理器
│   └── auth-plugin/        # 双因素验证 (TOTP)
├── shared/                 # 共享库
│   ├── types/             # 共享数据类型 (PluginInfo, UiField, JSON-RPC 类型)
│   └── rpc-protocol/      # JSON-RPC 协议实现 (RpcServer)
└── docs/plans/            # 开发计划和规范
```

## 常用命令

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
```bash
# 编译主应用
cd tauri-app
npm run tauri build

# 编译单个插件
cd plugins/password-manager
cargo build --release

# 安装插件到用户目录
mkdir -p ~/.worktools/plugins/password-manager
cp target/release/password-manager ~/.worktools/plugins/password-manager/
```

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

**核心设计**: 插件是独立的可执行文件,通过 stdin/stdout 使用 JSON-RPC 2.0 协议与主应用通信。

**插件生命周期**:
1. 主应用扫描 `~/.worktools/plugins/` 目录
2. 启动插件进程并发送 `get_info` 请求获取元信息
3. 插件通过 JSON-RPC 暴露方法: `get_info`, `get_view`, `init`, `destroy`, `heartbeat`
4. 前端通过 `get_view` 获取 UI Schema 并动态渲染界面

**关键实现文件**:
- [tauri-app/src-tauri/src/plugin_manager.rs](tauri-app/src-tauri/src/plugin_manager.rs) - 插件管理器 (PluginManager)
- [shared/rpc-protocol/src/lib.rs](shared/rpc-protocol/src/lib.rs) - JSON-RPC 协议实现 (RpcServer)

**重要约束**: 插件的日志必须输出到 stderr,保持 stdout 纯净用于 JSON-RPC 响应:
```rust
tracing_subscriber::fmt()
    .with_writer(std::io::stderr)  // 关键配置
    .init();
```

### UI Schema 动态渲染

插件通过 `get_view` 方法返回 UI Schema,前端动态渲染组件:
- **UiField 枚举**: [shared/types/src/lib.rs](shared/types/src/lib.rs#L14-L56) 定义了支持的 UI 组件类型 (Input, Number, Table, Button, Checkbox, Select)
- **前端渲染器**: [tauri-app/src/components/UiRenderer.tsx](tauri-app/src/components/UiRenderer.tsx)

### 数据流向

```
前端 (Solid.js) → Tauri Commands → PluginManager → 插件进程 (stdin/stdout) → RpcServer → 业务逻辑
```

**Tauri Commands 定义**: [tauri-app/src-tauri/src/commands.rs](tauri-app/src-tauri/src/commands.rs)

### 配置管理

所有数据存储在用户主目录下的 `~/.worktools/`:
- `plugins/` - 插件可执行文件
- `data/` - 数据文件 (passwords.json, auth.json)
- `config/` - 配置文件

配置管理实现: [tauri-app/src-tauri/src/config.rs](tauri-app/src-tauri/src/config.rs)

## 插件开发规范

### 创建新插件

1. 创建插件项目:
```bash
mkdir -p plugins/my-plugin/src
cd plugins/my-plugin
cargo init --bin
```

2. 编辑 `Cargo.toml`,添加依赖:
```toml
[dependencies]
worktools-shared-types = { path = "../../shared/types" }
worktools-rpc-protocol = { path = "../../shared/rpc-protocol" }
serde_json = "1.0"
anyhow = "1.0"
```

3. 实现插件主函数 ([参考 password-manager](plugins/password-manager/src/main.rs)):
```rust
use worktools_rpc_protocol::RpcServer;

fn main() -> anyhow::Result<()> {
    let mut rpc_server = RpcServer::new();

    // 必须实现的 RPC 方法
    rpc_server.register_handler("get_info", |_params| {
        Ok(serde_json::json!({
            "id": "my-plugin",
            "name": "我的插件",
            "version": "1.0.0",
            "description": "...",
            "icon": "🔧"
        }))
    });

    rpc_server.register_handler("get_view", |_params| {
        // 返回 UI Schema
        Ok(serde_json::to_value(ViewSchema { ... })?)
    });

    // 从 stdin 读取请求并处理
    // ... (参考 password-manager 实现)
}
```

4. 编译并安装:
```bash
cargo build --release
mkdir -p ~/.worktools/plugins/my-plugin
cp target/release/my-plugin ~/.worktools/plugins/my-plugin/
```

## 已知问题和解决方案

### 1. 插件加载失败
**问题**: 只显示部分插件或无法加载插件
**原因**: 插件日志输出到 stdout,污染 JSON-RPC 响应
**解决**: 确保所有 tracing 日志重定向到 stderr (见上方插件系统架构)

### 2. 点击无响应
**问题**: 点击插件菜单没有任何反应
**原因**: Solid.js 事件冒泡和默认行为干扰
**解决**: 在 onClick 事件中添加 `preventDefault()` 和 `stopPropagation()`

### 3. UI 配色和交互
- 配色方案: 侧边栏 `#1e1e1e`, 内容区 `#f5f5f5`, 主色调 `#0078d4`
- 所有 onClick 事件需要添加事件阻止和防止文本选择 (`user-select: none`)

详细设计规范: [docs/plans/README.md](docs/plans/README.md#L186-L203)

## Git 提交规范

使用 Conventional Commits 格式:
- `feat`: 新功能
- `fix`: 修复 bug
- `refactor`: 重构
- `style`: 样式调整
- `docs`: 文档
- `test`: 测试
- `chore`: 构建/工具

## 参考资源

- [Tauri 官方文档](https://tauri.app/)
- [Solid.js 文档](https://www.solidjs.com/)
- [JSON-RPC 2.0 规范](https://www.jsonrpc.org/specification)
- [开发计划和规范](docs/plans/README.md)
