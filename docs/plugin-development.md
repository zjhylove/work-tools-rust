# 插件开发教程

本教程将带你从零开始创建一个 Work Tools 插件。我们以 "hello-world" 插件为例，覆盖插件开发的完整流程。

## 目录

1. [创建项目结构](#1-创建项目结构)
2. [实现 Plugin trait](#2-实现-plugin-trait)
3. [导出 plugin_create 工厂函数](#3-导出-plugin_create-工厂函数)
4. [实现 handle_call 处理前端调用](#4-实现-handle_call-处理前端调用)
5. [使用 PluginStorage 持久化数据](#5-使用-pluginstorage-持久化数据)
6. [创建前端资源](#6-创建前端资源)
7. [使用 window.pluginAPI 与后端通信](#7-使用-windowpluginapi-与后端通信)
8. [创建 manifest.json](#8-创建-manifestjson)
9. [编译和测试](#9-编译和测试)
10. [打包 (.wtplugin.zip)](#10-打包-wtpluginzip)

---

## 1. 创建项目结构

在 `plugins/` 目录下创建插件目录，结构如下：

```
plugins/hello-world/
├── Cargo.toml          # Rust 项目配置
├── src/
│   └── lib.rs          # 插件后端逻辑
├── assets/             # 前端资源（打包后由构建脚本生成）
│   ├── index.html
│   ├── main.js
│   └── styles.css
├── frontend/           # 前端源码（React + Vite，可选）
│   ├── package.json
│   ├── vite.config.ts
│   ├── src/
│   │   ├── App.tsx
│   │   └── App.css
│   └── index.html
└── manifest.json       # 插件元数据
```

### Cargo.toml

```toml
[package]
name = "hello-world"
version = "1.0.0"
edition = "2021"

[lib]
# cdylib 表示编译为 C 动态库 (.dylib / .so / .dll)
# 这是插件系统通过 libloading 加载的前提条件
crate-type = ["cdylib"]

[dependencies]
# 插件 API 核心库，提供 Plugin trait、PluginStorage、PluginError
worktools-plugin-api = { path = "../../shared/plugin-api" }
# JSON 序列化/反序列化
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
# 错误处理
anyhow = "1.0"
# 结构化日志
tracing = "0.1"
```

要点：
- `crate-type = ["cdylib"]` 是必须的，否则编译产物无法被主程序动态加载。
- `worktools-plugin-api` 提供所有插件必须依赖的核心接口。
- 插件必须在根 `Cargo.toml` 的 workspace members 中注册。

### 注册到 Workspace

在根目录的 `Cargo.toml` 中，将新插件加入 workspace：

```toml
[workspace]
members = [
    # ... 其他 members ...
    "plugins/hello-world",
]
```

---

## 2. 实现 Plugin trait

`Plugin` trait 是所有插件必须实现的接口。创建 `src/lib.rs`：

```rust
use serde_json::Value;
use worktools_plugin_api::Plugin;

pub struct HelloWorldPlugin;

impl Plugin for HelloWorldPlugin {
    /// 唯一标识符，使用 kebab-case
    /// 用于注册表、存储路径、前端 API 调用
    fn id(&self) -> &str {
        "hello-world"
    }

    /// 显示名称，中文，在前端侧边栏展示
    fn name(&self) -> &str {
        "Hello World"
    }

    /// 功能描述
    fn description(&self) -> &str {
        "一个示例插件，展示插件开发的基本流程"
    }

    /// 语义化版本号
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// 图标，可以是 emoji 或图标名称
    fn icon(&self) -> &str {
        "👋"
    }

    /// 返回 HTML 内容，嵌入 iframe 展示
    /// 如果插件有 assets 目录，这里返回占位符即可
    /// 主程序会通过 iframe srcdoc 机制加载 assets 中的实际前端资源
    fn get_view(&self) -> String {
        "<div>插件前端资源加载中...</div>".to_string()
    }
}
```

六个必须实现的方法说明：

| 方法 | 返回类型 | 说明 |
|------|----------|------|
| `id()` | `&str` | 唯一标识符，全局唯一，kebab-case 格式 |
| `name()` | `&str` | 前端展示名称 |
| `description()` | `&str` | 功能描述 |
| `version()` | `&str` | SemVer 版本号 |
| `icon()` | `&str` | 侧边栏图标（emoji） |
| `get_view()` | `String` | HTML 视图，通常返回占位符 |

---

## 3. 导出 plugin_create 工厂函数

主程序通过 `libloading` 查找名为 `plugin_create` 的导出函数来创建插件实例。每个插件必须导出此函数：

```rust
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(HelloWorldPlugin));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
```

关键点：
- `#[no_mangle]` -- 防止编译器对函数名进行 name mangling，确保主程序可以通过 `"plugin_create"` 符号名找到此函数。
- `extern "C"` -- 使用 C ABI 调用约定，跨 FFI 边界的标准做法。
- 两层 `Box` 的原因：`Box<dyn Plugin>` 是 fat pointer（数据指针 + vtable 指针），无法直接作为原始指针传递。外层 `Box` 提供固定大小的指针，可以安全跨越 FFI 边界。
- `Box::leak()` -- 将堆上的所有权"泄漏"，返回原始指针。主程序侧通过 `Box::from_raw()` 重新接管所有权。

将这段代码放在 `src/lib.rs` 的末尾。

---

## 4. 实现 handle_call 处理前端调用

`handle_call` 是前后端通信的核心方法。前端通过 `window.pluginAPI.call()` 发起的请求最终会路由到这里：

```rust
impl Plugin for HelloWorldPlugin {
    // ... 前面 6 个必须方法 ...

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "greet" => {
                // 从参数中提取 name 字段
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 name 参数")?;

                Ok(serde_json::json!({
                    "message": format!("Hello, {}!", name)
                }))
            }

            "get_config" => {
                // 返回插件配置信息
                Ok(serde_json::json!({
                    "version": self.version(),
                    "description": self.description(),
                }))
            }

            // 未知方法返回错误
            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}
```

方法签名说明：

```rust
fn handle_call(
    &mut self,          // 可变引用，允许插件修改内部状态
    method: &str,       // 方法名，前端调用时传入
    params: Value,      // JSON 参数，serde_json::Value 类型
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>
```

最佳实践：
- 使用 `match method` 分发到不同的处理逻辑。
- 用 `params.get("key").and_then(|v| v.as_str())` 安全提取参数。
- 用 `serde_json::json!({})` 宏构建返回值。
- 未知方法返回 `Err("未知方法: {method}".into())`。
- 可以将复杂的处理逻辑拆分到独立的辅助方法中（参考 db-router 插件的做法）。

---

## 5. 使用 PluginStorage 持久化数据

`PluginStorage` 提供基于 JSON 文件的持久化存储，数据保存在 `~/.worktools/history/plugins/` 目录下。

### 基本用法

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use worktools_plugin_api::storage::PluginStorage;

/// 定义数据结构，derive 是必须的
#[derive(Debug, Serialize, Deserialize, Default)]
struct HelloData {
    greeting_count: u32,
    last_greeting: Option<String>,
}

impl HelloWorldPlugin {
    /// 创建存储实例
    /// 参数: (插件 ID, 数据文件名)
    fn storage() -> PluginStorage {
        PluginStorage::new("hello-world", "hello-world.json")
    }

    /// 读取数据
    fn load_data() -> Result<HelloData> {
        Self::storage().load_json()
        // 文件不存在时返回 HelloData::default()
    }

    /// 保存数据
    fn save_data(data: &HelloData) -> Result<()> {
        Self::storage().save_json(data)
        // 使用原子写入（先写临时文件再 rename），防止文件损坏
    }
}
```

### 在 handle_call 中使用

```rust
"greet" => {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("缺少 name 参数")?;

    // 读取并更新持久化数据
    let mut data = Self::load_data()?;
    data.greeting_count += 1;
    data.last_greeting = Some(name.to_string());
    Self::save_data(&data)?;

    Ok(serde_json::json!({
        "message": format!("Hello, {}!", name),
        "total_greetings": data.greeting_count,
    }))
}
```

### 保留字段写入

某些场景下需要保护已有字段不被覆盖（如加密 salt），使用 `save_json_preserving`：

```rust
// 第二个参数指定需要保留的字段名列表
// 这些字段会从现有文件中读取并保持不变
Self::storage().save_json_preserving(data, &["salt", "validation_token"])?;
```

---

## 6. 创建前端资源

前端资源放在 `assets/` 目录下，包含三个文件：

### assets/index.html

```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Hello World</title>
    <style>
      html, body { height: 100%; margin: 0; padding: 0; overflow: hidden; }
      #root { height: 100%; overflow: hidden; }
    </style>
    <script type="module" crossorigin src="./main.js"></script>
    <link rel="stylesheet" href="./styles.css">
  </head>
  <body>
    <div id="root"></div>
  </body>
</html>
```

### assets/styles.css

```css
/* 所有颜色必须使用 var(--xxx) 设计令牌，禁止硬编码色值 */
.hello-world {
  flex: 1;
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  padding: var(--space-xl);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: var(--font-sans);
}

.hello-world h1 {
  font-size: var(--font-size-2xl);
  margin: 0 0 var(--space-lg) 0;
}

.hello-world input {
  padding: var(--space-sm) var(--space-md);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: var(--font-size-md);
}

.hello-world input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-ring);
}

.result {
  margin-top: var(--space-lg);
  padding: var(--space-md) var(--space-lg);
  background: var(--success-light);
  border: 1px solid var(--success-border);
  border-radius: var(--radius-md);
  color: var(--success-text);
}
```

### 使用 React 前端项目（推荐）

大多数插件使用 React + Vite 构建前端。在 `frontend/` 目录下创建标准的 Vite + React 项目：

```
frontend/
├── package.json
├── vite.config.ts       # 配置输出到 ../assets/
├── tsconfig.json
├── index.html
└── src/
    ├── App.tsx          # 主组件
    ├── App.css          # 样式
    └── main.tsx         # 入口
```

`vite.config.ts` 配置输出到 `assets/`：

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: '../assets',
    emptyOutDir: true,
    rollupOptions: {
      output: {
        entryFileNames: 'main.js',
        assetFileNames: 'styles.[ext]',
      },
    },
  },
});
```

---

## 7. 使用 window.pluginAPI 与后端通信

主程序在 iframe 加载完成后，会注入 `window.pluginAPI` 对象，提供以下方法：

```typescript
interface PluginAPI {
  // 调用插件后端方法
  call(pluginId: string, method: string, params?: Record<string, unknown>): Promise<unknown>;

  // 获取插件配置
  get_plugin_config(): Promise<Record<string, unknown>>;

  // 设置插件配置
  set_plugin_config(config: Record<string, unknown>): Promise<void>;

  // 在系统浏览器中打开 URL
  open_url(url: string): Promise<void>;

  // 打开文件夹选择对话框
  open_folder_dialog(): Promise<string | null>;

  // 打开文件选择对话框
  open_file_dialog(filters?: { name: string; extensions: string[] }[]): Promise<string | null>;

  // 写入文件
  write_file(path: string, content: string): Promise<void>;
}
```

### TypeScript 类型声明

在 `frontend/src/App.tsx` 中声明全局类型：

```typescript
declare global {
  interface Window {
    pluginAPI?: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}
```

### 调用示例

```typescript
// 调用后端的 greet 方法
const result = await window.pluginAPI?.call('hello-world', 'greet', { name: 'World' });
// result: { message: "Hello, World!" }

// 获取预设列表
const presets = await window.pluginAPI?.call('hello-world', 'get_presets', {});
```

### 完整的 App.tsx 示例

```tsx
import { useState, useCallback } from 'react';
import './App.css';

declare global {
  interface Window {
    pluginAPI?: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

function App() {
  const [name, setName] = useState('');
  const [message, setMessage] = useState('');

  const handleGreet = useCallback(async () => {
    if (!name.trim() || !window.pluginAPI) return;
    try {
      const result = await window.pluginAPI.call('hello-world', 'greet', { name: name.trim() });
      if (result && typeof result === 'object' && 'message' in result) {
        setMessage((result as { message: string }).message);
      }
    } catch (err) {
      console.error('调用失败:', err);
    }
  }, [name]);

  return (
    <div className="hello-world">
      <h1>Hello World 插件</h1>
      <input
        type="text"
        value={name}
        onChange={e => setName(e.target.value)}
        placeholder="输入你的名字"
      />
      <button className="wt-btn--primary" onClick={handleGreet}>
        发送问候
      </button>
      {message && <div className="result">{message}</div>}
    </div>
  );
}

export default App;
```

### 前端开发规范

- **操作反馈**: 使用 `WorkTools.toast.success(msg)` / `.error(msg)` / `.info(msg)` / `.warning(msg)`，禁止自行实现 toast 或使用 `alert()`。
- **表单校验**: 逐字段校验，失焦触发，错误显示在本字段下方：`WorkTools.FieldError.show(inputEl, msg)`。禁止用 toast 显示校验错误。
- **按钮样式**: 统一使用 `.wt-btn--primary` / `.wt-btn--secondary` / `.wt-btn--danger` / `.wt-btn--ghost`。
- **模态框**: 删除等不可逆操作必须使用 `.wt-modal-*` 确认弹窗，禁止使用原生 `confirm()`。
- **加载态**: 提交等异步操作按钮必须有 loading 态（`.wt-spinner` + disabled）。
- **CSS 颜色**: 必须使用 `var(--xxx)` 设计令牌，禁止硬编码色值。

---

## 8. 创建 manifest.json

`manifest.json` 是插件的元数据文件，包含插件信息和平台相关的动态库文件名：

```json
{
  "id": "hello-world",
  "name": "Hello World",
  "description": "一个示例插件，展示插件开发的基本流程",
  "version": "1.0.0",
  "icon": "👋",
  "author": "Your Name",
  "files": {
    "macos": "libhello_world.dylib",
    "linux": "libhello_world.so",
    "windows": "hello_world.dll"
  },
  "assets": {
    "entry": "index.html"
  },
  "permissions": []
}
```

字段说明：
- `files` 中的文件名规则：
  - macOS/Linux: `lib` 前缀 + 插件名（连字符转下划线）+ 平台扩展名
  - Windows: 无前缀 + `.dll` 扩展名
  - 例如插件名为 `hello-world`，则 macOS 上为 `libhello_world.dylib`
- `assets.entry`: 前端入口文件，通常为 `index.html`
- `permissions`: 预留字段，当前为空数组

---

## 9. 编译和测试

### 编译检查

```bash
# 类型检查（比 build 快得多，开发时首选）
cargo check -p hello-world

# 完整编译（生成动态库）
cargo build --release -p hello-world

# 格式化和 lint
cargo fmt
cargo clippy -p hello-world
```

编译产物位置：
- macOS: `target/release/libhello_world.dylib`
- Linux: `target/release/libhello_world.so`
- Windows: `target/release/hello_world.dll`

### 编写测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use worktools_plugin_api::Plugin;

    #[test]
    fn test_plugin_metadata() {
        let plugin = HelloWorldPlugin;
        assert_eq!(plugin.id(), "hello-world");
        assert_eq!(plugin.version(), "1.0.0");
    }

    #[test]
    fn test_greet() {
        let mut plugin = HelloWorldPlugin;
        let result = plugin
            .handle_call("greet", serde_json::json!({ "name": "World" }))
            .unwrap();

        let obj = result.as_object().unwrap();
        assert_eq!(obj["message"], "Hello, World!");
    }

    #[test]
    fn test_unknown_method() {
        let mut plugin = HelloWorldPlugin;
        let result = plugin.handle_call("nonexistent", serde_json::json!({}));
        assert!(result.is_err());
    }
}
```

运行测试：

```bash
# 运行单个插件的所有测试
cargo test -p hello-world

# 按测试名过滤
cargo test -p hello-world -- test_greet
```

### 本地集成测试

将编译好的插件安装到本地进行测试：

1. 确保编译产物在 `target/release/` 中
2. 启动开发服务器：`cd tauri-app && npm run tauri dev`
3. 通过插件市场导入 `.wtplugin.zip` 文件，或手动将文件复制到 `~/.worktools/plugins/hello-world/`

---

## 10. 打包 (.wtplugin.zip)

### 使用构建脚本

一键编译并打包所有插件：

```bash
bash scripts/build-plugins.sh
```

此脚本会：
1. 编译所有插件的 Rust 动态库（`cargo build --release`）
2. 构建有 `frontend/` 目录的插件前端
3. 将 `manifest.json` + 动态库 + `assets/` 打包为 `.wtplugin.zip`

产物位置：`plugins/hello-world/hello-world-macos.wtplugin.zip`（平台名根据实际系统变化）。

### 手动打包

```bash
cd plugins/hello-world

# 确保已编译
cargo build --release -p hello-world

# 复制动态库到插件目录
cp ../../target/release/libhello_world.dylib .

# 打包
zip -r hello-world-macos.wtplugin.zip \
    manifest.json \
    libhello_world.dylib \
    assets/

# 清理
rm libhello_world.dylib
```

### 插件包结构

`.wtplugin.zip` 内部结构：

```
├── manifest.json              # 插件元数据
├── libhello_world.dylib       # 动态库（按平台）
└── assets/                    # 前端资源
    ├── index.html
    ├── main.js
    └── styles.css
```

安装后文件位于 `~/.worktools/plugins/hello-world/`。

---

## 完整代码参考

以下是 `src/lib.rs` 的完整示例：

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use worktools_plugin_api::storage::PluginStorage;
use worktools_plugin_api::Plugin;

/// 持久化数据结构
#[derive(Debug, Serialize, Deserialize, Default)]
struct HelloData {
    greeting_count: u32,
    last_greeting: Option<String>,
}

pub struct HelloWorldPlugin;

impl HelloWorldPlugin {
    fn storage() -> PluginStorage {
        PluginStorage::new("hello-world", "hello-world.json")
    }

    fn load_data() -> Result<HelloData> {
        Self::storage().load_json()
    }

    fn save_data(data: &HelloData) -> Result<()> {
        Self::storage().save_json(data)
    }
}

impl Plugin for HelloWorldPlugin {
    fn id(&self) -> &str { "hello-world" }
    fn name(&self) -> &str { "Hello World" }
    fn description(&self) -> &str { "一个示例插件，展示插件开发的基本流程" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "👋" }
    fn get_view(&self) -> String {
        "<div>插件前端资源加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "greet" => {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 name 参数")?;

                let mut data = Self::load_data()?;
                data.greeting_count += 1;
                data.last_greeting = Some(name.to_string());
                Self::save_data(&data)?;

                Ok(serde_json::json!({
                    "message": format!("Hello, {}!", name),
                    "total_greetings": data.greeting_count,
                }))
            }

            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(HelloWorldPlugin));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
```

---

## 下一步

- 阅读 [Plugin API Reference](./plugin-api-reference.md) 了解完整的接口文档。
- 阅读 [Design Token Reference](./design-token-reference.md) 了解前端样式变量。
- 参考现有插件源码：`plugins/cron-tools/`（无状态）、`plugins/db-router/`（有持久化）。
